use super::super::{Location, RunParameters, RunScenario, RunSetupRevision, Session, Variable};
use crate::analysis::{HourlyWindow, resolve_hourly_window};
use crate::store::{ObservationStore, read_mini_topology, read_result_timeline};
use chrono::{NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

const MAX_POINTS: usize = 1_000;
const MAX_MATCHES: usize = 100;

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
    pub(super) parameters: RunParameters,
    pub(super) revision: RunSetupRevision,
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

fn revision_schema() -> Value {
    object(
        serde_json::json!({
            "session":{"type":"integer","minimum":0},
            "revision":{"type":"integer","minimum":0}
        }),
        &["session", "revision"],
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
        id: "run.parameters",
        description: "Read the current run parameters, revision, and available forecast cycles",
        schema: empty,
        invoke: run_parameters,
    },
    Command {
        id: "run.propose_parameters",
        description: "Propose a reference, horizon or an ordered scenario selection for operator review",
        schema: || {
            object(
                serde_json::json!({
                    "revision": revision_schema(),
                    "reference_time": {"type":"string"},
                    "lookback_days": {"type":"integer","minimum":0},
                    "forecast_days": {"type":"integer","minimum":1},
                    "scenarios": {"type":"array","items":{"type":"object","description":"kind zero, raw with forecast, or corrected with pinned forecast and ordered corrections"}}
                }),
                &["revision"],
            )
        },
        invoke: propose_parameters,
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

fn run_parameters(session: &Session, arguments: Value) -> Result {
    parse::<Empty>(arguments)?;
    let Some(snapshot) = session.run_setup() else {
        return Ok(Output::Data(serde_json::json!({"parameters":null})));
    };
    let operation = &snapshot.operation;
    Ok(Output::Data(serde_json::json!({
        "revision": snapshot.revision,
        "reference_time": operation.reference(),
        "parameters": operation.parameters(),
        "scenarios": operation.scenarios().map_err(Finding::read)?,
        "available_forecasts": operation.available_forecasts().map_err(Finding::read)?,
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposedParameters {
    revision: RunSetupRevision,
    reference_time: Option<String>,
    lookback_days: Option<u32>,
    forecast_days: Option<u32>,
    scenarios: Option<Vec<RunScenario>>,
}

fn propose_parameters(session: &Session, arguments: Value) -> Result {
    let args = parse::<ProposedParameters>(arguments)?;
    let snapshot = session
        .run_setup()
        .filter(|snapshot| snapshot.revision == args.revision)
        .ok_or_else(|| {
            Finding::new(
                "stale_run_parameters",
                "Read the current run parameters first",
            )
        })?;
    let mut operation = snapshot.operation;
    let mut parameters = operation.parameters().clone();
    let mut changes = Vec::new();
    fn change<T: serde::Serialize + PartialEq>(
        changes: &mut Vec<FieldChange>,
        field: &'static str,
        current: &mut Option<T>,
        proposed: Option<T>,
    ) {
        if let Some(proposed) = proposed
            && current.as_ref() != Some(&proposed)
        {
            changes.push(FieldChange {
                field,
                scenario_index: None,
                scenario_id: None,
                current: serde_json::json!(current),
                proposed: serde_json::json!(proposed),
            });
            *current = Some(proposed);
        }
    }
    change(
        &mut changes,
        "reference_time",
        &mut parameters.reference_time,
        args.reference_time,
    );
    change(
        &mut changes,
        "lookback_days",
        &mut parameters.lookback_days,
        args.lookback_days,
    );
    change(
        &mut changes,
        "forecast_days",
        &mut parameters.forecast_days,
        args.forecast_days,
    );
    change(
        &mut changes,
        "scenarios",
        &mut parameters.scenarios,
        args.scenarios,
    );
    if changes.is_empty() {
        return Err(Finding::invalid("No changed run parameters"));
    }
    operation.set_parameters(parameters.clone());
    operation.check_parameters().map_err(Finding::invalid)?;
    let readiness = operation.validate();
    Ok(Output::Proposal(Box::new(Proposal {
        parameters,
        revision: args.revision,
        review: ProposalReview { changes, readiness },
    })))
}
