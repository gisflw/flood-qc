use super::commands::{Command, CommandContext, CommandFinding, CommandResult, parse, schema};
use crate::app::{DraftConflict, DraftDocument, DraftVersion, Session, request};
use crate::store::ForecastCatalog;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_REVIEW: AtomicU64 = AtomicU64::new(1);

pub(super) const COMMAND: Command = Command {
    id: "draft.propose_edit",
    description: "Stage draft horizon or forecast asset changes for operator review. Copy the version from draft.read. Applying and saving are operator actions.",
    arguments: arguments_schema,
    run: stage,
};

fn arguments_schema() -> Value {
    let version = schema(
        json!({
            "session": {"type": "integer", "minimum": 1},
            "draft": {"type": "integer", "minimum": 1},
            "revision": {"type": "integer", "minimum": 1}
        }),
        &["session", "draft", "revision"],
    );
    let asset = schema(
        json!({
            "scenario_index": {"type": "integer", "minimum": 0},
            "asset_id": {"type": "string", "minLength": 1}
        }),
        &["scenario_index", "asset_id"],
    );
    schema(
        json!({
            "version": version,
            "observed_horizon_days": {"type": "integer", "minimum": 0, "maximum": u32::MAX},
            "forecast_horizon_days": {"type": "integer", "minimum": 1, "maximum": u32::MAX},
            "forecast_asset": asset
        }),
        &["version"],
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposalArgs {
    version: DraftVersion,
    #[serde(default, deserialize_with = "provided")]
    observed_horizon_days: Option<u32>,
    #[serde(default, deserialize_with = "provided")]
    forecast_horizon_days: Option<u32>,
    #[serde(default, deserialize_with = "provided")]
    forecast_asset: Option<ForecastAsset>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ForecastAsset {
    scenario_index: usize,
    asset_id: String,
}

fn provided<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "field", rename_all = "snake_case")]
pub enum DraftEdit {
    ObservedHorizonDays {
        current: Option<u32>,
        proposed: u32,
    },
    ForecastHorizonDays {
        current: Option<u32>,
        proposed: u32,
    },
    ForecastAsset {
        scenario_index: usize,
        scenario_id: Option<String>,
        current: Option<String>,
        proposed: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProposalReview {
    #[serde(skip)]
    id: u64,
    pub changes: Vec<DraftEdit>,
    pub readiness: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalOutcome {
    Applied,
    Rejected,
    NoPending,
    Stale,
    Busy,
    Invalid(String),
}

impl std::fmt::Display for ProposalOutcome {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Applied => {
                formatter.write_str("Applied to the working draft. Save to write the edit to disk.")
            }
            Self::Rejected => formatter.write_str("Draft proposal rejected."),
            Self::NoPending => formatter.write_str("No draft proposal is pending."),
            Self::Stale => formatter.write_str(
                "The working draft changed. Request a new proposal from the current draft.",
            ),
            Self::Busy => {
                formatter.write_str("A draft job is active. Retry Apply after it finishes.")
            }
            Self::Invalid(error) => write!(formatter, "Draft proposal is invalid. {error}"),
        }
    }
}

struct Candidate {
    version: DraftVersion,
    document: DraftDocument,
    review: ProposalReview,
}

#[derive(Default)]
pub(super) struct ProposalReviewState {
    pending: Option<(u64, Candidate)>,
}

impl ProposalReviewState {
    pub(super) fn current(&self) -> Option<ProposalReview> {
        self.pending
            .as_ref()
            .map(|(_, candidate)| candidate.review.clone())
    }

    fn check_available(&self) -> Result<(), CommandFinding> {
        if self.pending.is_some() {
            return Err(CommandFinding::new(
                "proposal_pending",
                "An operator must apply or reject the pending proposal first.",
            ));
        }
        Ok(())
    }

    fn stage(&mut self, turn: u64, candidate: Candidate) -> CommandResult {
        self.check_available()?;
        if candidate.review.changes.is_empty() {
            return Ok(json!({"status": "unchanged", "message": "There are no changes to apply."}));
        }
        let result = json!({"status": "staged", "review": candidate.review});
        self.pending = Some((turn, candidate));
        Ok(result)
    }

    pub(super) fn apply(
        &mut self,
        session: &Session,
        expected: &ProposalReview,
    ) -> ProposalOutcome {
        if !self.matches(expected) {
            return ProposalOutcome::NoPending;
        }
        let (turn, candidate) = self.pending.take().expect("review matched");
        match session.apply_draft_update(candidate.version, candidate.document.clone()) {
            Ok(_) => ProposalOutcome::Applied,
            Err(DraftConflict::Stale) => ProposalOutcome::Stale,
            Err(DraftConflict::JobPending) => {
                self.pending = Some((turn, candidate));
                ProposalOutcome::Busy
            }
            Err(DraftConflict::Invalid(error)) => ProposalOutcome::Invalid(error),
        }
    }

    pub(super) fn reject(&mut self, expected: &ProposalReview) -> ProposalOutcome {
        if self.matches(expected) {
            self.clear();
            ProposalOutcome::Rejected
        } else {
            ProposalOutcome::NoPending
        }
    }

    fn matches(&self, expected: &ProposalReview) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|(_, candidate)| candidate.review.id == expected.id)
    }

    pub(super) fn cancel_turn(&mut self, turn: u64) {
        if self
            .pending
            .as_ref()
            .is_some_and(|(origin, _)| *origin == turn)
        {
            self.clear();
        }
    }

    pub(super) fn clear(&mut self) {
        self.pending = None;
    }
}

fn stage(context: &CommandContext, input: Value) -> CommandResult {
    let args = parse(input)?;
    context.review(|state, _| state.check_available())?;
    let candidate = validate(context.session(), args)?;
    context.review(|state, turn| state.stage(turn, candidate))
}

fn validate(session: &Session, args: ProposalArgs) -> Result<Candidate, CommandFinding> {
    let snapshot = session
        .draft()
        .filter(|snapshot| snapshot.version == args.version)
        .ok_or_else(stale)?;
    let mut draft = snapshot.draft;
    let operation = &mut draft.document.operation;
    let mut changes = Vec::new();
    let mut readiness = Vec::new();
    if draft.document.schema_version != 2
        && (args.observed_horizon_days.is_some() || args.forecast_horizon_days.is_some())
    {
        return Err(invalid(
            "Day horizons require a version 2 draft. Explicit period bounds are preserved.",
        ));
    }
    if let Some(proposed) = args.observed_horizon_days
        && operation.observed_horizon_days != Some(proposed)
    {
        validate_horizon(operation.reference_time.as_deref(), proposed, false)?;
        changes.push(DraftEdit::ObservedHorizonDays {
            current: operation.observed_horizon_days,
            proposed,
        });
        readiness.push(
            "Accept observations for the new historical window before previewing or preparing."
                .into(),
        );
        operation.observed_horizon_days = Some(proposed);
    }
    if let Some(proposed) = args.forecast_horizon_days {
        if proposed == 0 {
            return Err(invalid("Forecast horizon must be at least one day."));
        }
        if operation.forecast_horizon_days != Some(proposed) {
            validate_horizon(operation.reference_time.as_deref(), proposed, true)?;
            changes.push(DraftEdit::ForecastHorizonDays {
                current: operation.forecast_horizon_days,
                proposed,
            });
            operation.forecast_horizon_days = Some(proposed);
        }
    }
    if let Some(asset) = args.forecast_asset
        && let Some((change, finding)) =
            change_forecast_asset(&mut draft.document, &draft.workspace, asset)?
    {
        changes.push(change);
        readiness.push("Accept the newly selected forecast before previewing or preparing.".into());
        readiness.extend(finding);
    }
    session
        .validate_draft_update(args.version, &draft.document)
        .map_err(|error| match error {
            DraftConflict::Stale => stale(),
            error => invalid(error.to_string()),
        })?;
    if !changes.is_empty()
        && let Err(error) = draft.preparation_document()
    {
        readiness.push(error);
    }
    Ok(Candidate {
        version: args.version,
        document: draft.document,
        review: ProposalReview {
            id: NEXT_REVIEW.fetch_add(1, Ordering::Relaxed),
            changes,
            readiness,
        },
    })
}

fn validate_horizon(
    reference: Option<&str>,
    days: u32,
    forecast: bool,
) -> Result<(), CommandFinding> {
    // An incomplete draft may lack a reference. Use the widest viable period
    // for the edited field until the operator supplies that anchor.
    let reference = reference.unwrap_or(if forecast {
        "0000-01-01T00:00:00"
    } else {
        "9999-12-31T00:00:00"
    });
    let mut period = json!({
        "reference_time": reference,
        "observed_horizon_days": if forecast { 0 } else { days },
        "forecast_horizon_days": if forecast { days } else { 1 }
    });
    request::expand_period(&mut period).map_err(|error| invalid(error.to_string()))?;
    let field = if forecast {
        "forecast_last_end"
    } else {
        "historical_first_end"
    };
    let endpoint = period[field]
        .as_str()
        .expect("expanded period includes endpoint");
    request::timestamp(field, endpoint)
        .map_err(|_| invalid("The proposed horizon exceeds the supported date range."))?;
    Ok(())
}

fn change_forecast_asset(
    document: &mut DraftDocument,
    workspace: &std::path::Path,
    change: ForecastAsset,
) -> Result<Option<(DraftEdit, Option<String>)>, CommandFinding> {
    let index = change.scenario_index;
    let operation = &mut document.operation;
    let scenario = operation
        .scenarios
        .get_mut(index)
        .ok_or_else(|| invalid("Forecast asset selection requires an existing scenario index."))?;
    if change.asset_id.trim().is_empty() {
        return Err(invalid("Forecast asset ID must be nonempty."));
    }
    if scenario.asset_id.as_ref() == Some(&change.asset_id) {
        return Ok(None);
    }
    if scenario.kind.as_deref() == Some("zero") {
        return Err(invalid(
            "A zero-rainfall scenario cannot select a forecast asset.",
        ));
    }
    let catalog = operation
        .forecasts
        .as_ref()
        .and_then(|choice| choice.catalog.as_deref())
        .ok_or_else(|| {
            invalid("Select a forecast catalog in the draft before proposing an asset.")
        })?;
    let catalog = ForecastCatalog::open_read_only(workspace, catalog)
        .map_err(|error| invalid(format!("Forecast catalog unavailable. {error}")))?;
    let registered = match document
        .instructions
        .corrections
        .get(index)
        .and_then(Option::as_ref)
    {
        Some(values) => {
            if values
                .iter()
                .any(|value| value.value.asset_id != change.asset_id)
            {
                return Err(invalid(
                    "The scenario has a draft correction for another asset.",
                ));
            }
            &[][..]
        }
        None => scenario.correction_ids.as_deref().unwrap_or(&[]),
    };
    catalog
        .load_corrections(&change.asset_id, registered)
        .map_err(|error| invalid(format!("Cannot select this forecast asset. {error}")))?;
    let finding = catalog
        .check_asset_available(&change.asset_id)
        .err()
        .map(|error| format!("Forecast data is unavailable. {error}"));
    let edit = DraftEdit::ForecastAsset {
        scenario_index: index,
        scenario_id: scenario.id.clone(),
        current: scenario.asset_id.replace(change.asset_id.clone()),
        proposed: change.asset_id,
    };
    Ok(Some((edit, finding)))
}

fn invalid(message: impl Into<String>) -> CommandFinding {
    CommandFinding::new("invalid_proposal", message)
}

fn stale() -> CommandFinding {
    CommandFinding::new("stale_draft", ProposalOutcome::Stale.to_string())
}

#[cfg(test)]
mod tests;
