use super::super::{
    Location, RunIdentity, RunSelection, SeriesQuery, SeriesSelection, Session, Variable, absolute,
    series,
};
use super::{Shared, proposal};
use crate::analysis::{HourlyWindow, resolve_hourly_window};
use crate::store::{
    ObservationStore, read_basin_geometry, read_result_timeline, read_station_results,
};
use chrono::{NaiveDateTime, Timelike};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(super) const MAX_POINTS: usize = 168;
const MAX_LOCATIONS: usize = 50;
const MAX_SCENARIOS: usize = 64;

#[derive(Debug, Serialize)]
pub(super) struct CommandFinding {
    pub(super) code: &'static str,
    pub(super) message: String,
}

impl CommandFinding {
    pub(super) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub(super) type CommandResult = Result<Value, CommandFinding>;

pub(super) struct CommandContext {
    session: Session,
    shared: Arc<Shared>,
    request_cancelled: Arc<AtomicBool>,
    turn: u64,
}

impl CommandContext {
    pub(super) fn new(
        session: Session,
        shared: Arc<Shared>,
        turn: u64,
        request_cancelled: Arc<AtomicBool>,
    ) -> Self {
        Self {
            session,
            shared,
            turn,
            request_cancelled,
        }
    }

    pub(super) fn review<T>(
        &self,
        action: impl FnOnce(&mut proposal::ProposalReviewState, u64) -> Result<T, CommandFinding>,
    ) -> Result<T, CommandFinding> {
        let mut state = self.shared.state.lock().unwrap();
        if state.turn != Some(self.turn) || self.request_cancelled.load(Ordering::Acquire) {
            return Err(CommandFinding::new(
                "cancelled",
                "The assistant tool request ended",
            ));
        }
        action(&mut state.review, self.turn)
    }

    pub(super) fn session(&self) -> &Session {
        &self.session
    }
}

pub(super) struct Command {
    pub(super) id: &'static str,
    pub(super) description: &'static str,
    pub(super) arguments: fn() -> Value,
    pub(super) run: fn(&CommandContext, Value) -> CommandResult,
}

impl Command {
    fn describe(&self) -> Value {
        json!({"id": self.id, "description": self.description, "arguments_schema": (self.arguments)()})
    }
}

static COMMANDS: &[Command] = &[
    Command {
        id: "workspace.inspect",
        description: "Inspect the selected workspace and report prerequisite findings and inspection scope.",
        arguments: empty_schema,
        run: |context, args| {
            parse::<Empty>(args)?;
            Ok(json!(context.session().workspace().inspect()))
        },
    },
    Command {
        id: "run.summary",
        description: "Identify the selected run and report its reference, scenarios and available retained results.",
        arguments: empty_schema,
        run: |context, args| {
            parse::<Empty>(args)?;
            run_summary(context.session())
        },
    },
    Command {
        id: "location.search",
        description: "Find station names or IDs and model mini-basin IDs in the selected workspace.",
        arguments: location_schema,
        run: |context, args| location_search(context.session(), parse(args)?),
    },
    Command {
        id: "series.query",
        description: "Query retained rain, flow or level at a station or mini-basin for one scenario and explicit local window. At most 168 points.",
        arguments: series_schema,
        run: |context, args| series_query(context.session(), parse(args)?),
    },
    Command {
        id: "draft.read",
        description: "Read the working draft identity, revision, periods and scenario asset selections.",
        arguments: empty_schema,
        run: |context, args| {
            parse::<Empty>(args)?;
            draft_read(context.session())
        },
    },
    proposal::COMMAND,
];

pub(super) fn schema(properties: Value, required: &[&str]) -> Value {
    json!({"type": "object", "properties": properties, "required": required, "additionalProperties": false})
}

fn empty_schema() -> Value {
    schema(json!({}), &[])
}

fn location_schema() -> Value {
    schema(
        json!({
            "query": {"type": "string", "description": "Case-insensitive station name or ID, or mini-basin ID substring."},
            "kind": {"enum": ["station", "mini"]},
            "limit": {"type": "integer", "minimum": 1, "maximum": MAX_LOCATIONS, "default": 20}
        }),
        &["query"],
    )
}

fn series_schema() -> Value {
    schema(
        json!({
            "location": {"oneOf": [
                schema(json!({"station": {"type": "string", "minLength": 1}}), &["station"]),
                schema(json!({"mini": {"type": "integer", "minimum": 1}}), &["mini"])
            ]},
            "scenario_id": {"type": "string", "pattern": "^[a-zA-Z0-9_-]+$"},
            "variable": {"enum": ["rain", "flow", "level"]},
            "start": {"type": "string", "description": "Local whole-hour time, YYYY-MM-DDTHH:MM:SS."},
            "end": {"type": "string", "description": "Local whole-hour time, YYYY-MM-DDTHH:MM:SS."},
            "expected_run": {"oneOf": [
                schema(json!({"Active": {"type": "string"}}), &["Active"]),
                schema(json!({"Saved": {"type": "string"}}), &["Saved"])
            ], "description": "Optional run identity copied from a previous result."}
        }),
        &["location", "scenario_id", "variable", "start", "end"],
    )
}

pub(super) fn tools() -> Value {
    json!([
        {"name": "search_commands", "description": "Search MGB Ops commands by name or purpose. Empty query lists all commands.",
         "inputSchema": schema(json!({"query": {"type": "string"}}), &["query"])},
        {"name": "run_command", "description": "Invoke a discovered MGB Ops command with its typed arguments.",
         "inputSchema": schema(json!({"id": {"type": "string"}, "arguments": {"type": "object"}}), &["id", "arguments"])}
    ])
}

pub(super) fn dispatch(context: &CommandContext, name: &str, args: Value) -> CommandResult {
    match name {
        "search_commands" => {
            let Search { query } = parse(args)?;
            let query = query.to_lowercase();
            let results: Vec<_> = COMMANDS
                .iter()
                .filter(|command| {
                    query.split_whitespace().all(|term| {
                        command.id.contains(term)
                            || command.description.to_lowercase().contains(term)
                    })
                })
                .map(Command::describe)
                .collect();
            Ok(json!({"commands": results}))
        }
        "run_command" => {
            let Invocation { id, arguments } = parse(args)?;
            let command = COMMANDS
                .iter()
                .find(|command| command.id == id)
                .ok_or_else(|| {
                    CommandFinding::new("unknown_command", "Search for a supported command ID")
                })?;
            (command.run)(context, arguments)
        }
        _ => Err(CommandFinding::new(
            "unknown_tool",
            "Use search_commands or run_command",
        )),
    }
}

pub(super) fn parse<T: DeserializeOwned>(args: Value) -> Result<T, CommandFinding> {
    serde_json::from_value(args)
        .map_err(|error| CommandFinding::new("invalid_arguments", error.to_string()))
}

fn present<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    query: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Invocation {
    id: String,
    arguments: Value,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum LocationKind {
    Mini,
    Station,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocationSearch {
    query: String,
    #[serde(default, deserialize_with = "present")]
    kind: Option<LocationKind>,
    #[serde(default = "default_location_limit")]
    limit: usize,
}

fn default_location_limit() -> usize {
    20
}

fn location_search(session: &Session, args: LocationSearch) -> CommandResult {
    if !(1..=MAX_LOCATIONS).contains(&args.limit) {
        return Err(CommandFinding::new(
            "invalid_arguments",
            format!("limit must be 1–{MAX_LOCATIONS}"),
        ));
    }
    let workspace = session.workspace();
    let query = args.query.trim().to_lowercase();
    let mut matches = Vec::new();
    let mut findings = Vec::new();
    if args.kind != Some(LocationKind::Mini) {
        match ObservationStore::open_read_only(&workspace.workspace().join("data/history.sqlite"))
            .and_then(|store| store.station_locations())
        {
            Ok(stations) => {
                matches.extend(stations.into_iter().map(|entry| entry.station).filter(|station| {
                    [&station.station_id, &station.station_code, &station.name].iter()
                        .any(|value| value.to_lowercase().contains(&query))
                }).map(|station| (LocationKind::Station, station.station_id.clone(),
                    json!({"location": Location::Station(station.station_id), "name": station.name}))));
            }
            Err(error) => findings.push(CommandFinding::new(
                "station_inventory_unavailable",
                error.to_string(),
            )),
        }
    }
    if args.kind != Some(LocationKind::Station) {
        let input = workspace.request().engine_input_dir.clone().map_or_else(
            || {
                workspace.read_config().map(|config| {
                    config
                        .engine
                        .and_then(|engine| engine.input_dir)
                        .unwrap_or_else(|| "mgb_runner/Input".into())
                        .into()
                })
            },
            Ok,
        );
        match input.and_then(|input| {
            read_basin_geometry(&absolute(workspace.workspace(), &input).join("MINI.gtp"))
        }) {
            Ok(geometry) => matches.extend(geometry.mini_ids.into_iter().filter_map(|id| {
                let key = id.to_string();
                key.contains(&query).then(|| {
                    (
                        LocationKind::Mini,
                        key,
                        json!({"location": Location::Mini(id)}),
                    )
                })
            })),
            Err(error) => findings.push(CommandFinding::new("mini_inventory_unavailable", error)),
        }
    }
    matches.sort_unstable_by(|(kind_a, id_a, _), (kind_b, id_b, _)| {
        kind_a.cmp(kind_b).then_with(|| match kind_a {
            LocationKind::Mini => id_a.len().cmp(&id_b.len()).then(id_a.cmp(id_b)),
            LocationKind::Station => id_a.cmp(id_b),
        })
    });
    let more = matches.len() > args.limit;
    let matches = matches
        .into_iter()
        .take(args.limit)
        .map(|(_, _, value)| value)
        .collect::<Vec<_>>();
    Ok(json!({"matches": matches, "more": more, "findings": findings}))
}

fn run_summary(session: &Session) -> CommandResult {
    let run = session
        .run()
        .map_summary(|data| {
            let root = &data.root;
            let manifest = &data.manifest;
            let more = manifest.scenarios.len() > MAX_SCENARIOS;
            let scenarios = manifest
                .scenarios
                .iter()
                .take(MAX_SCENARIOS)
                .map(|record| {
                    let available = data.scenarios.contains(&record.id);
                    let mut findings = data
                        .findings
                        .iter()
                        .filter(|finding| {
                            finding
                                .strip_prefix(&record.id)
                                .is_some_and(|rest| rest.starts_with(": "))
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    let station_results = available
                        && match read_station_results(
                            &root.join("results").join(&record.id).join("station.nc"),
                        ) {
                            Ok(_) => true,
                            Err(error) => {
                                findings.push(error.to_string());
                                false
                            }
                        };
                    let failure = record
                        .error
                        .as_ref()
                        .map(|error| json!({"stage": error.stage, "message": error.message}));
                    json!({
                        "id": record.id, "state": record.state, "basin_results": available,
                        "station_results": station_results, "findings": findings, "failure": failure
                    })
                })
                .collect::<Vec<_>>();
            let reference = if data.reference.is_empty() {
                manifest
                    .recipe
                    .pointer("/document/reference_time")
                    .and_then(Value::as_str)
            } else {
                Some(data.reference.as_str())
            };
            json!({"reference_time": reference, "time_zone": data.time_zone,
                "start": data.start, "end": data.end, "state": manifest.state,
                "scenarios": scenarios, "more_scenarios": more})
        })
        .map_err(|error| CommandFinding::new("run_unavailable", error))?
        .ok_or_else(no_run)?;
    let mut result = run.data;
    result["identity"] = json!(run.identity);
    Ok(result)
}

fn no_run() -> CommandFinding {
    CommandFinding::new(
        "no_selected_run",
        "The workspace has no selected completed run",
    )
}

fn stale_run() -> CommandFinding {
    CommandFinding::new(
        "stale_run",
        "The selected run changed. Request a new run summary before querying.",
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SeriesArguments {
    location: Location,
    scenario_id: String,
    variable: Variable,
    start: NaiveDateTime,
    end: NaiveDateTime,
    #[serde(default, deserialize_with = "present")]
    expected_run: Option<RunIdentity>,
}

fn series_query(session: &Session, args: SeriesArguments) -> CommandResult {
    if !series::valid_id(&args.scenario_id) {
        return Err(CommandFinding::new(
            "invalid_arguments",
            "Invalid scenario ID",
        ));
    }
    let valid_location = match &args.location {
        Location::Station(id) => !id.trim().is_empty(),
        Location::Mini(id) => *id > 0,
    };
    if !valid_location {
        return Err(CommandFinding::new(
            "invalid_arguments",
            "Select a station ID or a positive mini-basin ID",
        ));
    }
    if args.start >= args.end
        || [args.start, args.end]
            .iter()
            .any(|time| time.minute() != 0 || time.second() != 0 || time.nanosecond() != 0)
    {
        return Err(CommandFinding::new(
            "invalid_arguments",
            "Use ordered whole-hour window bounds",
        ));
    }
    let hours = usize::try_from((args.end - args.start).num_hours()).unwrap_or(usize::MAX);
    let hourly_points = hours.saturating_add(usize::from(!matches!(args.variable, Variable::Rain)));
    if hourly_points > MAX_POINTS {
        return Err(window_limit());
    }

    let expected = args.expected_run.as_ref();
    let snapshot = session.run();
    let matches_selection = match (expected, snapshot.selection()) {
        (None, _) | (Some(RunIdentity::Active(_)), RunSelection::Active) => true,
        (Some(RunIdentity::Saved(path)), RunSelection::Saved(saved)) => path == saved.root(),
        _ => false,
    };
    if !matches_selection {
        return Err(stale_run());
    }
    let had_expectation = expected.is_some();
    let result = snapshot
        .read(None, |root| {
            if expected.is_some_and(|expected| expected.root() != root) {
                return Ok(Err(stale_run()));
            }
            Ok(query_model(root, &args))
        })
        .map_err(|error| CommandFinding::new("run_unavailable", error))?;
    result.map_or_else(
        || {
            Err(if had_expectation {
                stale_run()
            } else {
                no_run()
            })
        },
        |run| {
            let report = run.data?;
            Ok(json!({"identity": run.identity, "report": report}))
        },
    )
}

fn window_limit() -> CommandFinding {
    CommandFinding::new(
        "series_limit",
        format!("Request a narrower window of at most {MAX_POINTS} points"),
    )
}

fn query_model(root: &std::path::Path, args: &SeriesArguments) -> CommandResult {
    let scenario_directory = root.join("results").join(&args.scenario_id);
    let timeline = read_result_timeline(&scenario_directory.join("basin.nc"))
        .map_err(|error| CommandFinding::new("scenario_unavailable", error.to_string()))?;
    let window = HourlyWindow::Explicit {
        start: args.start,
        end: args.end,
    };
    let bounds = resolve_hourly_window(&timeline.time_zone, &window)
        .map_err(|error| CommandFinding::new("invalid_series", error))?;
    let exceeds_limit = timeline
        .bounds
        .iter()
        .filter(|interval| series::model_interval_in_window(&args.variable, interval, &bounds))
        .nth(MAX_POINTS)
        .is_some();
    if exceeds_limit {
        return Err(window_limit());
    }
    let source = series::read_model(root, &args.scenario_id, Some(&timeline.time_zone), false)
        .map_err(|error| CommandFinding::new("scenario_unavailable", error))?;
    let request = SeriesQuery {
        location: args.location.clone(),
        time_zone: timeline.time_zone,
        window,
        selections: vec![SeriesSelection::Model {
            root: root.into(),
            scenario_id: args.scenario_id.clone(),
            variable: args.variable.clone(),
        }],
    };
    let report = series::query(&request, None, Some((root, &args.scenario_id, &source)))
        .map_err(|error| CommandFinding::new("invalid_series", error))?;
    if report
        .series
        .iter()
        .any(|series| series.points.len() > MAX_POINTS)
    {
        return Err(window_limit());
    }
    Ok(
        json!({"scenario_id": args.scenario_id, "series": report.series, "window": report.window,
        "location": report.location, "time_zone": report.time_zone, "findings": report.findings}),
    )
}

fn draft_read(session: &Session) -> CommandResult {
    let snapshot = session
        .draft()
        .ok_or_else(|| CommandFinding::new("no_open_draft", "No draft is open"))?;
    let document = &snapshot.draft.document;
    let operation = &document.operation;
    let more = operation.scenarios.len() > MAX_SCENARIOS;
    let scenarios = operation.scenarios.iter().take(MAX_SCENARIOS).enumerate()
        .map(|(index, scenario)| json!({"index": index, "id": scenario.id, "asset_id": scenario.asset_id}))
        .collect::<Vec<_>>();
    Ok(json!({
        "version": snapshot.version, "schema_version": document.schema_version,
        "reference_time": operation.reference_time, "time_zone": operation.time_zone,
        "period": {
            "observed_horizon_days": operation.observed_horizon_days, "forecast_horizon_days": operation.forecast_horizon_days,
            "historical_first_end": operation.historical_first_end, "historical_last_end": operation.historical_last_end,
            "forecast_first_end": operation.forecast_first_end, "forecast_last_end": operation.forecast_last_end,
            "step_seconds": operation.step_seconds
        },
        "scenarios": scenarios, "more_scenarios": more
    }))
}

#[cfg(test)]
mod tests;
