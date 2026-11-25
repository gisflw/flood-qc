use super::super::{DraftSnapshot, DraftVersion, Location, Session, Variable};
use crate::analysis::{HourlyWindow, resolve_hourly_window};
use crate::store::{ForecastCatalog, ObservationStore, read_mini_topology, read_result_timeline};
use chrono::{NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

const MAX_POINTS: usize = 1_000;
const MAX_MATCHES: usize = 100;
const MAX_SCENARIOS: usize = 100;

#[derive(Debug, Serialize)]
pub(super) struct Finding {
    code: &'static str,
    message: String,
}
impl Finding {
    pub(super) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid_arguments", message)
    }

    fn read(message: impl ToString) -> Self {
        Self::new("read_failed", message.to_string())
    }
}

pub(super) enum Output {
    Data(Value),
    Proposal(Box<Proposal>),
}
pub(super) type Result = std::result::Result<Output, Finding>;

#[derive(Clone, Serialize)]
pub struct ProposalReview {
    pub changes: Vec<FieldChange>,
    pub readiness: Vec<String>,
}

#[derive(Clone, Serialize)]
pub struct FieldChange {
    pub field: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_id: Option<String>,
    pub current: Value,
    pub proposed: Value,
}

pub(super) struct Proposal {
    pub(super) document: super::super::DraftDocument,
    pub(super) version: DraftVersion,
    pub(super) review: ProposalReview,
}

struct Command {
    id: &'static str,
    description: &'static str,
    schema: fn() -> Value,
    invoke: fn(&Session, Value) -> Result,
}

fn object(properties: Value, required: &[&str]) -> Value {
    serde_json::json!({"type":"object", "properties":properties, "required":required, "additionalProperties":false})
}

fn empty() -> Value {
    object(serde_json::json!({}), &[])
}

fn version_schema() -> Value {
    object(
        serde_json::json!({
            "session":{"type":"integer","minimum":0},
            "draft":{"type":"integer","minimum":0},
            "revision":{"type":"integer","minimum":0}
        }),
        &["session", "draft", "revision"],
    )
}

const COMMANDS: &[Command] = &[
    Command {
        id: "workspace.inspect",
        description: "Inspect the selected workspace and return prerequisite findings",
        schema: empty,
        invoke: inspect,
    },
    Command {
        id: "run.summary",
        description: "Summarize the selected run, reference time, scenarios and available results",
        schema: empty,
        invoke: run_summary,
    },
    Command {
        id: "location.search",
        description: "Find station names, station IDs or mini-basin IDs",
        schema: || {
            object(
                serde_json::json!({
                    "query":{"type":"string"}, "limit":{"type":"integer","minimum":1,"maximum":MAX_MATCHES}
                }),
                &["query"],
            )
        },
        invoke: location_search,
    },
    Command {
        id: "series.query",
        description: "Read retained rainfall, flow or level for one scenario and explicit local window",
        schema: || {
            object(
                serde_json::json!({
                    "location":{"oneOf":[
                        object(serde_json::json!({"station":{"type":"string","minLength":1}}), &["station"]),
                        object(serde_json::json!({"mini":{"type":"integer","minimum":1}}), &["mini"])
                    ]},
                    "scenario":{"type":"string","pattern":"^[a-zA-Z0-9_-]+$"},
                    "variable":{"enum":["rain","flow","level"]},
                    "start":{"type":"string","description":"Local YYYY-MM-DDTHH:MM:SS"},
                    "end":{"type":"string","description":"Local YYYY-MM-DDTHH:MM:SS"},
                    "expected_run":{"type":"string"}
                }),
                &["location", "scenario", "variable", "start", "end"],
            )
        },
        invoke: series_query,
    },
    Command {
        id: "draft.read",
        description: "Read the open working draft anchor, period, scenarios and version for review",
        schema: empty,
        invoke: draft_read,
    },
    Command {
        id: "draft.propose_edit",
        description: "Stage horizon or scenario forecast asset changes for operator review",
        schema: || {
            object(
                serde_json::json!({
                    "version":version_schema(),
                    "observed_horizon_days":{"type":"integer","minimum":0},
                    "forecast_horizon_days":{"type":"integer","minimum":1},
                    "forecast_assets":{"type":"array","items":object(serde_json::json!({
                        "scenario_index":{"type":"integer","minimum":0},
                        "asset_id":{"type":"string","minLength":1}
                    }), &["scenario_index","asset_id"])}
                }),
                &["version"],
            )
        },
        invoke: draft_edit,
    },
];

pub(super) fn search(query: &str) -> Value {
    let words = query
        .split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    let commands: Vec<_> = COMMANDS.iter().filter_map(|command| {
        let searchable = format!("{} {}", command.id, command.description).to_lowercase();
        words.iter().all(|word| searchable.contains(word))
            .then(|| serde_json::json!({"id":command.id,"description":command.description,"arguments_schema":(command.schema)()}))
    }).collect();
    serde_json::json!({"commands":commands})
}

pub(super) fn run(session: &Session, id: &str, arguments: Value) -> Result {
    let command = COMMANDS
        .iter()
        .find(|command| command.id == id)
        .ok_or_else(|| {
            Finding::new(
                "unknown_command",
                "Use search_commands to find a command ID",
            )
        })?;
    (command.invoke)(session, arguments)
}

fn parse<T: DeserializeOwned>(arguments: Value) -> std::result::Result<T, Finding> {
    serde_json::from_value(arguments).map_err(|error| Finding::invalid(error.to_string()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

fn inspect(session: &Session, arguments: Value) -> Result {
    parse::<Empty>(arguments)?;
    Ok(Output::Data(serde_json::json!(session.inspect())))
}

fn no_run() -> Finding {
    Finding::new("no_active_run", "The workspace has no active run")
}

fn run_summary(session: &Session, arguments: Value) -> Result {
    parse::<Empty>(arguments)?;
    let run = session.run_snapshot().read(None, |root, identity| {
        let summary = super::super::read_run_summary(root)?;
        let scenarios: Vec<_> = summary.manifest.scenarios.iter().map(|scenario| {
            let basin = summary.scenarios.contains(&scenario.id);
            let stations = basin && root.join("results").join(&scenario.id).join("station.nc").is_file();
            serde_json::json!({"id":scenario.id, "state":scenario.state, "basin_results":basin,
                "station_results":stations, "failure":scenario.error.as_ref().map(|failure| &failure.message)})
        }).collect();
        let reference = if summary.reference.is_empty() {
            summary.manifest.recipe.pointer("/document/reference_time").cloned().unwrap_or(Value::Null)
        } else {
            serde_json::json!(summary.reference)
        };
        Ok(serde_json::json!({"run":identity,"state":summary.manifest.state,"reference_time":reference,
            "scenarios":scenarios,"findings":summary.findings}))
    }).map_err(Finding::read)?.ok_or_else(no_run)?;
    Ok(Output::Data(run))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocationSearch {
    query: String,
    #[serde(default)]
    limit: Option<usize>,
}

fn location_search(session: &Session, arguments: Value) -> Result {
    let args: LocationSearch = parse(arguments)?;
    let limit = args.limit.unwrap_or(20);
    if !(1..=MAX_MATCHES).contains(&limit) {
        return Err(Finding::invalid(format!(
            "limit must be between 1 and {MAX_MATCHES}"
        )));
    }
    let needle = args.query.trim().to_lowercase();
    let mini_query = needle
        .strip_prefix("mini")
        .map_or(needle.as_str(), str::trim);
    let mut findings = Vec::new();
    let mut matches = Vec::new();
    let history = super::super::operation_database(session.workspace());
    match ObservationStore::open_read_only(&history).and_then(|store| store.station_locations()) {
        Ok(mut stations) => {
            stations.sort_by(|a, b| a.station.station_id.cmp(&b.station.station_id));
            matches.extend(
                stations
                    .into_iter()
                    .filter_map(|location| {
                        let station = location.station;
                        (station.station_id.to_lowercase().contains(&needle)
                            || station.name.to_lowercase().contains(&needle))
                        .then(|| {
                            serde_json::json!({"station":station.station_id,"name":station.name,
                        "mini":location.offset.map(|offset| offset.mini_id)})
                        })
                    })
                    .take(limit + 1),
            );
        }
        Err(error) => findings.push(Finding::read(error)),
    }
    // Inspection uses the same overrides and default input path as other session reads.
    let inspection = session.inspect();
    if let Some(path) = inspection
        .findings
        .iter()
        .find(|finding| finding.check == "engine.input.MINI.gtp")
        .and_then(|finding| finding.path.as_deref())
    {
        match read_mini_topology(path) {
            Ok((mut geometry, _)) => {
                geometry.mini_ids.sort_unstable();
                matches.extend(
                    geometry
                        .mini_ids
                        .into_iter()
                        .filter(|id| id.to_string().contains(mini_query))
                        .take((limit + 1).saturating_sub(matches.len()))
                        .map(|id| serde_json::json!({"mini":id})),
                );
            }
            Err(error) => findings.push(Finding::read(error)),
        }
    } else {
        findings.push(Finding::new(
            "mini_unavailable",
            "The selected workspace has no readable MINI topology path",
        ));
    }
    let more = matches.len() > limit;
    matches.truncate(limit);
    Ok(Output::Data(
        serde_json::json!({"matches":matches,"more":more,"findings":findings}),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SeriesArguments {
    location: Location,
    scenario: String,
    variable: Variable,
    start: NaiveDateTime,
    end: NaiveDateTime,
    #[serde(default)]
    expected_run: Option<String>,
}

fn too_many_points() -> Finding {
    Finding::new(
        "series_limit",
        format!("Use a narrower window with at most {MAX_POINTS} points"),
    )
}

fn series_query(session: &Session, arguments: Value) -> Result {
    use super::super::{SeriesQuery, SeriesSelection, series};
    let args: SeriesArguments = parse(arguments)?;
    if !series::valid_id(&args.scenario) {
        return Err(Finding::invalid("Invalid scenario ID"));
    }
    match &args.location {
        Location::Station(id) if id.trim().is_empty() => {
            return Err(Finding::invalid("Station ID must be nonempty"));
        }
        Location::Mini(id) if *id <= 0 => {
            return Err(Finding::invalid("MINI ID must be positive"));
        }
        _ => {}
    }
    if args.start >= args.end
        || [args.start, args.end]
            .iter()
            .any(|time| time.minute() != 0 || time.second() != 0 || time.nanosecond() != 0)
    {
        return Err(Finding::invalid("Use ordered whole-hour local bounds"));
    }
    let result = session
        .run_snapshot()
        .read(None, |root, identity| {
            if args
                .expected_run
                .as_deref()
                .is_some_and(|expected| expected != identity)
            {
                return Ok(Err(Finding::new(
                    "stale_run",
                    "The run changed. Query run.summary again.",
                )));
            }
            let hours = (args.end - args.start).num_hours();
            let inclusive = i64::from(!matches!(args.variable, Variable::Rain));
            if hours + inclusive > MAX_POINTS as i64 {
                return Ok(Err(too_many_points()));
            }
            let basin = root.join("results").join(&args.scenario).join("basin.nc");
            let timeline = read_result_timeline(&basin).map_err(|error| error.to_string())?;
            let window = HourlyWindow::Explicit {
                start: args.start,
                end: args.end,
            };
            if let Err(error) = resolve_hourly_window(&timeline.time_zone, &window) {
                return Ok(Err(Finding::invalid(error)));
            }
            let possible = timeline
                .bounds
                .iter()
                .filter(|interval| {
                    interval.end >= args.start
                        && interval.end <= args.end
                        && (!matches!(args.variable, Variable::Rain)
                            || interval.start >= args.start)
                })
                .take(MAX_POINTS + 1)
                .count();
            if possible > MAX_POINTS {
                return Ok(Err(too_many_points()));
            }
            let mini = matches!(args.location, Location::Mini(_));
        let level = matches!(args.variable, Variable::Level);
        let source = series::read_model(root, &args.scenario, Some(&timeline.time_zone), mini)?;
        let report = series::query(&SeriesQuery {
            location:args.location, time_zone: timeline.time_zone, window,
            selections: vec![SeriesSelection::Model { root:root.into(), scenario_id:args.scenario.clone(), variable:args.variable }],
        }, None, Some((root, &args.scenario, &source)))?;
        let level_reference = if level {
            report.series.first().map(|series| {
                if series.series_kind == "model_raw_level" { "raw_basin" } else { "station_offset" }
            })
        } else {
            None
        };
        Ok(Ok(serde_json::json!({
            "run":identity,"scenario":args.scenario,"level_reference":level_reference,"report":report
        })))
    }).map_err(Finding::read)?;
    match result {
        Some(report) => report.map(Output::Data),
        None if args.expected_run.is_some() => Err(Finding::new(
            "stale_run",
            "The expected run is no longer selected",
        )),
        None => Err(no_run()),
    }
}

fn draft_read(session: &Session, arguments: Value) -> Result {
    parse::<Empty>(arguments)?;
    let Some(snapshot) = session.draft() else {
        return Ok(Output::Data(serde_json::json!({"draft":null})));
    };
    let operation = &snapshot.draft.document.operation;
    let scenarios: Vec<_> = operation.scenarios.iter().take(MAX_SCENARIOS).enumerate()
        .map(|(index, scenario)| {
            serde_json::json!({"index":index,"id":scenario.id,"asset_id":scenario.asset_id})
        }).collect();
    Ok(Output::Data(serde_json::json!({
        "version":snapshot.version, "schema_version":snapshot.draft.document.schema_version,
        "reference_time":operation.reference_time,"time_zone":operation.time_zone,
        "period":{
            "observed_horizon_days":operation.observed_horizon_days,"forecast_horizon_days":operation.forecast_horizon_days,
            "step_seconds":operation.step_seconds,"historical_first_end":operation.historical_first_end,
            "historical_last_end":operation.historical_last_end,"forecast_first_end":operation.forecast_first_end,
            "forecast_last_end":operation.forecast_last_end
        },
        "scenarios":scenarios,"more_scenarios":operation.scenarios.len() > MAX_SCENARIOS
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftEdit {
    version: DraftVersion,
    #[serde(default, deserialize_with = "present")]
    observed_horizon_days: Option<u32>,
    #[serde(default, deserialize_with = "present")]
    forecast_horizon_days: Option<u32>,
    #[serde(default)]
    forecast_assets: Vec<AssetChange>,
}

fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> std::result::Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetChange {
    scenario_index: usize,
    asset_id: String,
}

fn check_asset(snapshot: &DraftSnapshot, change: &AssetChange) -> std::result::Result<(), Finding> {
    let operation = &snapshot.draft.document.operation;
    let path = operation
        .forecasts
        .as_ref()
        .and_then(|forecasts| forecasts.catalog.as_deref())
        .ok_or_else(|| Finding::invalid("Select a forecast catalog in the working draft"))?;
    let catalog = ForecastCatalog::open_read_only(&snapshot.draft.workspace, path)
        .map_err(|error| Finding::invalid(error.to_string()))?;
    catalog
        .check_asset_available(&change.asset_id)
        .map_err(|error| Finding::invalid(error.to_string()))?;
    let ids = operation.scenarios[change.scenario_index]
        .correction_ids
        .as_deref()
        .unwrap_or_default();
    let corrections = snapshot
        .draft
        .document
        .instructions
        .corrections
        .get(change.scenario_index)
        .and_then(Option::as_ref);
    if corrections.is_some_and(|values| {
        values
            .iter()
            .any(|item| item.value.asset_id != change.asset_id)
    }) {
        return Err(Finding::invalid(
            "The selected draft corrections belong to another asset",
        ));
    }
    catalog
        .load_corrections(&change.asset_id, ids)
        .map_err(|error| Finding::invalid(error.to_string()))?;
    Ok(())
}

fn draft_edit(session: &Session, arguments: Value) -> Result {
    let args: DraftEdit = parse(arguments)?;
    let snapshot = session
        .draft()
        .filter(|draft| draft.version == args.version)
        .ok_or_else(|| {
            Finding::new(
                "stale_draft",
                super::super::DraftConflict::Stale.to_string(),
            )
        })?;
    let mut document = snapshot.draft.document.clone();
    let operation = &mut document.operation;
    let mut changes = Vec::new();
    if let (Some(observed), Some(forecast)) = (
        args.observed_horizon_days
            .or(operation.observed_horizon_days),
        args.forecast_horizon_days
            .or(operation.forecast_horizon_days),
    ) && (args.observed_horizon_days.is_some() || args.forecast_horizon_days.is_some())
    {
        let mut period = serde_json::json!({
            "reference_time":operation.reference_time.as_deref().unwrap_or("2000-01-01T00:00:00"),
            "observed_horizon_days":observed,"forecast_horizon_days":forecast
        });
        super::super::request::expand_period(&mut period)
            .map_err(|error| Finding::invalid(error.to_string()))?;
    }
    for (field, current, proposed) in [
        (
            "observed_horizon_days",
            &mut operation.observed_horizon_days,
            args.observed_horizon_days,
        ),
        (
            "forecast_horizon_days",
            &mut operation.forecast_horizon_days,
            args.forecast_horizon_days,
        ),
    ] {
        if let Some(value) = proposed {
            if document.schema_version != 2 {
                return Err(Finding::invalid("Day horizons require a version 2 draft"));
            }
            if field == "forecast_horizon_days" && value == 0 {
                return Err(Finding::invalid("Forecast horizon must be positive"));
            }
            if *current != Some(value) {
                changes.push(FieldChange {
                    field,
                    scenario_index: None,
                    scenario_id: None,
                    current: serde_json::json!(current),
                    proposed: serde_json::json!(value),
                });
                *current = Some(value);
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    for change in args.forecast_assets {
        if !seen.insert(change.scenario_index) || change.asset_id.trim().is_empty() {
            return Err(Finding::invalid(
                "Use a nonempty asset ID and one change per scenario",
            ));
        }
        let scenario = operation
            .scenarios
            .get_mut(change.scenario_index)
            .ok_or_else(|| Finding::invalid("Scenario index is outside the reviewed draft"))?;
        if scenario.asset_id.as_deref() != Some(&change.asset_id) {
            check_asset(&snapshot, &change)?;
            changes.push(FieldChange {
                field: "asset_id",
                scenario_index: Some(change.scenario_index),
                scenario_id: scenario.id.clone(),
                current: serde_json::json!(scenario.asset_id),
                proposed: serde_json::json!(change.asset_id),
            });
            scenario.asset_id = Some(change.asset_id);
        }
    }
    if changes.is_empty() {
        return Ok(Output::Data(
            serde_json::json!({"status":"unchanged","message":"There are no changes to apply"}),
        ));
    }
    let candidate = snapshot
        .draft
        .validate_working_document(document)
        .map_err(Finding::invalid)?;
    let operation = &candidate.document.operation;
    let mut readiness = Vec::new();
    if operation.reference_time.is_none() || operation.time_zone.is_none() {
        readiness.push("The draft needs a reference time and timezone.".into());
    }
    if candidate.document.schema_version == 2
        && (operation.observed_horizon_days.is_none() || operation.forecast_horizon_days.is_none())
    {
        readiness.push("The draft needs both day horizons.".into());
    }
    if operation.scenarios.is_empty() {
        readiness.push("The draft needs at least one scenario.".into());
    }
    readiness.push("Accept selected inputs explicitly before using changed selections.".into());
    Ok(Output::Proposal(Box::new(Proposal {
        document: candidate.document,
        version: snapshot.version,
        review: ProposalReview { changes, readiness },
    })))
}
