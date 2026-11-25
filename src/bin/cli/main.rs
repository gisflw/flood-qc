mod acquisition;
mod assistant;
mod output;
mod progress;

use self::output::{acquisition_result, operation_result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Acquire standalone source files and normalize them for ingestion.
    Acquisition {
        #[command(subcommand)]
        command: acquisition::AcquisitionCommand,
    },
    /// Chat with an installed agent in an explicitly selected workspace.
    Assistant(assistant::Options),
    Automation {
        #[command(subcommand)]
        command: AutomationCommand,
    },
    Report {
        #[command(subcommand)]
        command: ReportCommand,
    },
    Edits {
        #[command(subcommand)]
        command: EditsCommand,
    },
    Observation {
        #[command(subcommand)]
        command: ObservationCommand,
    },
    Forcing {
        #[command(subcommand)]
        command: ForcingCommand,
    },
    Calibration {
        #[command(subcommand)]
        command: CalibrationCommand,
    },
    Run {
        #[command(subcommand)]
        command: RunCommand,
    },
    Inundation {
        #[command(subcommand)]
        command: InundationCommand,
    },
    Series {
        #[command(subcommand)]
        command: SeriesCommand,
        #[arg(long, global = true)]
        json: bool,
    },
    References {
        #[command(subcommand)]
        command: ReferenceCommand,
        #[arg(long, global = true)]
        json: bool,
    },
    Inputs {
        #[command(subcommand)]
        command: InputCommand,
    },
    Operation {
        #[command(subcommand)]
        command: OperationCommand,
    },
    Forecast {
        #[command(subcommand)]
        command: ForecastCommand,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
}
#[derive(Subcommand)]
enum SeriesCommand {
    Query {
        #[arg(long)]
        request: PathBuf,
    },
    Export {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Analyze {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    ExportBatch {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum ReportCommand {
    Generate {
        #[arg(long)]
        request: PathBuf,
    },
    ReturnPeriod {
        #[arg(long)]
        request: PathBuf,
    },
}

#[derive(Subcommand)]
enum AutomationCommand {
    Validate {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        config: PathBuf,
    },
    Status {
        #[arg(long)]
        workspace: PathBuf,
    },
    Serve {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        config: PathBuf,
    },
    RunNow {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        job: String,
    },
    Send {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        body_file: PathBuf,
        #[arg(long)]
        attachment: Vec<PathBuf>,
    },
}

fn automation_action(
    command: AutomationCommand,
    out: &mut impl Write,
) -> Result<bool, Box<dyn std::error::Error>> {
    use mgb_ops::app::{
        AutomationControl, AutomationState, automation_status, publish_automation_artifacts,
        read_automation_config, run_automation, run_automation_now, send_automation_email,
    };
    let control = AutomationControl::default();
    if matches!(
        command,
        AutomationCommand::Serve { .. } | AutomationCommand::RunNow { .. }
    ) {
        let signal = control.clone();
        ctrlc::set_handler(move || signal.stop())?;
    }
    let successful = match command {
        AutomationCommand::Validate { workspace, config } => {
            let config = read_automation_config(&workspace, &config)?;
            serde_json::to_writer(&mut *out, &BTreeMap::from([("jobs", config.jobs.len())]))?;
            true
        }
        AutomationCommand::Status { workspace } => {
            serde_json::to_writer(&mut *out, &automation_status(&workspace)?)?;
            true
        }
        AutomationCommand::Serve { workspace, config } => {
            let config = read_automation_config(&workspace, &config)?;
            writeln!(out, "Scheduler running")?;
            out.flush()?;
            run_automation(&workspace, &config, &control, publish_automation_artifacts)?;
            return Ok(true);
        }
        AutomationCommand::RunNow {
            workspace,
            config,
            job,
        } => {
            let config = read_automation_config(&workspace, &config)?;
            let status = run_automation_now(
                &workspace,
                &config,
                &job,
                &control,
                publish_automation_artifacts,
            )?;
            serde_json::to_writer(&mut *out, &status)?;
            status.state == AutomationState::Completed
        }
        AutomationCommand::Send {
            workspace,
            config,
            subject,
            body_file,
            attachment,
        } => {
            let config = read_automation_config(&workspace, &config)?;
            let smtp = config
                .smtp
                .as_ref()
                .ok_or("automation config requires [smtp]")?;
            let body = std::fs::read_to_string(workspace.join(body_file))?;
            send_automation_email(&workspace, smtp, &subject, &body, &attachment)?;
            writeln!(out, "Delivery accepted")?;
            return Ok(true);
        }
    };
    writeln!(out)?;
    Ok(successful)
}

#[derive(Subcommand)]
enum EditsCommand {
    /// Replace the ordered edits for observations.
    Import {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        input: PathBuf,
    },
    Export {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum ObservationCommand {
    /// Download ANA observations and import the normalized hourly values.
    Acquire(AcquisitionOptions),
    /// Import retained legacy-format ANA CSV files.
    ImportAnaCsv {
        #[command(flatten)]
        options: AcquisitionOptions,
        #[arg(long)]
        input: Vec<PathBuf>,
    },
    Init {
        #[arg(long)]
        assets: PathBuf,
        #[arg(long)]
        time_zone: String,
    },
    Import {
        #[arg(long)]
        assets: PathBuf,
        #[arg(long)]
        input: PathBuf,
    },
    ImportInventory {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        input: PathBuf,
    },
    ExportInventory {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Inventory {
        #[arg(long)]
        assets: PathBuf,
        #[arg(long)]
        input: PathBuf,
    },
    Availability {
        #[arg(long)]
        request: PathBuf,
    },
    SpatialQc {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        station: Option<String>,
        #[arg(long)]
        interpolation: PathBuf,
    },
}

#[derive(Args)]
struct AcquisitionOptions {
    #[arg(long)]
    force: bool,
    #[arg(long)]
    workspace: PathBuf,
    #[arg(long)]
    reference_time: Option<String>,
    #[arg(long)]
    json: bool,
}

impl AcquisitionOptions {
    fn request(self) -> mgb_ops::app::AcquisitionRequest {
        mgb_ops::app::AcquisitionRequest {
            workspace: self.workspace,
            reference_time: self.reference_time,
        }
    }
}

#[derive(ValueEnum, Clone, Copy)]
enum ForcingKind {
    Reservoirs,
    Wind,
}

#[derive(Subcommand)]
enum ForcingCommand {
    Import {
        #[arg(long)]
        kind: ForcingKind,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum CalibrationCommand {
    Geometry {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        edits: Option<PathBuf>,
        #[arg(long)]
        channel_table: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Soil {
        #[arg(long)]
        input: Option<PathBuf>,
        #[arg(long)]
        geometry: PathBuf,
        #[arg(long)]
        hrc: PathBuf,
        #[arg(long)]
        edits: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

fn read_json<T: serde::de::DeserializeOwned>(
    path: &std::path::Path,
) -> Result<T, Box<dyn std::error::Error>> {
    Ok(serde_json::from_reader(std::fs::File::open(path)?)?)
}

fn calibration_action(
    command: CalibrationCommand,
    out: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error>> {
    use mgb_ops::app::{CalibrationGeometry, GeometryEdit, SoilCalibration, SoilEdit};
    match command {
        CalibrationCommand::Geometry {
            input,
            edits,
            channel_table,
            output,
        } => {
            let mut geometry = CalibrationGeometry::read(&input)?;
            if let Some(path) = edits {
                for edit in read_json::<Vec<GeometryEdit>>(&path)? {
                    geometry.edit(edit)?;
                }
            }
            if let Some(path) = channel_table {
                geometry.apply_channel_table(&path)?;
            }
            if let Some(path) = output {
                geometry.save(&path)?;
            }
            serde_json::to_writer(&mut *out, geometry.rows())?;
        }
        CalibrationCommand::Soil {
            input,
            geometry,
            hrc,
            edits,
            output,
        } => {
            let geometry = CalibrationGeometry::read(&geometry)?;
            let mut soil = match input {
                Some(path) => SoilCalibration::read(&path, &geometry, &hrc)?,
                None => SoilCalibration::create(&geometry, &hrc)?,
            };
            if let Some(path) = edits {
                for edit in read_json::<Vec<SoilEdit>>(&path)? {
                    soil.edit(edit)?;
                }
            }
            if let Some(path) = output {
                soil.save(&path)?;
            }
            serde_json::to_writer(&mut *out, &(soil.classes(), soil.watersheds()))?;
        }
    }
    writeln!(out)?;
    Ok(())
}
#[derive(Subcommand)]
enum RunCommand {
    Export {
        #[arg(long)]
        run: PathBuf,
        #[arg(long)]
        inputs: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
    },
    Import {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Regenerate {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        engine: PathBuf,
        #[command(flatten)]
        policy: InputPolicy,
        #[arg(long)]
        json: bool,
    },
}
#[derive(Subcommand)]
enum ReferenceCommand {
    Import {
        #[arg(long)]
        assets: PathBuf,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        thresholds: PathBuf,
        #[arg(long)]
        history: PathBuf,
    },
}
#[derive(Subcommand)]
enum InundationCommand {
    /// Process an aligned HAND and catchments GeoTIFF pair for a retained scenario.
    Process {
        #[arg(long)]
        run: Option<PathBuf>,
        #[arg(long)]
        workspace: Option<PathBuf>,
        #[arg(long)]
        scenario: String,
        #[arg(long)]
        hand: Option<PathBuf>,
        #[arg(long)]
        catchments: Option<PathBuf>,
        #[arg(long)]
        replace: bool,
        #[arg(long)]
        json: bool,
    },
}
#[derive(Args)]
struct RunOptions {
    #[arg(long)]
    workspace: PathBuf,
    #[arg(long)]
    config: Option<PathBuf>,
    #[command(flatten)]
    input_policy: InputPolicy,
    #[arg(long)]
    force: bool,
    #[arg(long)]
    reference_time: Option<String>,
    #[arg(long)]
    lookback_days: Option<u32>,
    #[arg(long)]
    forecast_days: Option<u32>,
    /// Select zero, a forecast product/cycle, or a JSON scenario value. Repeat to select a set.
    #[arg(long = "scenario")]
    scenarios: Vec<mgb_ops::app::RunScenario>,
    #[arg(long)]
    forcing: Option<String>,
    #[arg(long)]
    calibration: Option<String>,
    #[arg(long)]
    geometry: Option<PathBuf>,
    #[arg(long)]
    hot_start: Option<String>,
    #[arg(long)]
    routing: Option<String>,
    #[arg(long)]
    json: bool,
}

fn parse_parameter<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, String> {
    serde_json::from_str(value).map_err(|error| error.to_string())
}

fn parse_routing(value: &str) -> Result<mgb_ops::app::RoutingMode, String> {
    match value {
        "inertial" => Ok(mgb_ops::app::RoutingMode::Inertial),
        "muskingum_cunge" => Ok(mgb_ops::app::RoutingMode::MuskingumCunge),
        _ => Err("Use inertial or muskingum_cunge".into()),
    }
}

impl RunOptions {
    fn session(self) -> Result<mgb_ops::app::Session, String> {
        let session = mgb_ops::app::Session::new(mgb_ops::InspectionRequest {
            workspace: self.workspace,
            invocation_dir: std::env::current_dir().map_err(|e| e.to_string())?,
            config: self.config,
            gpkg_path: None,
            engine_executable: None,
            engine_input_dir: None,
        });
        let setup = session
            .run_setup()
            .ok_or("Set valid workspace run defaults")?;
        session.update_run_setup(
            &setup.revision,
            mgb_ops::app::RunParameters {
                reference_time: self.reference_time,
                lookback_days: self.lookback_days,
                forecast_days: self.forecast_days,
                scenarios: (!self.scenarios.is_empty()).then_some(self.scenarios),
                forcing: self.forcing.as_deref().map(parse_parameter).transpose()?,
                calibration: self
                    .calibration
                    .as_deref()
                    .map(parse_parameter)
                    .transpose()?,
                geometry: self.geometry,
                hot_start: self.hot_start.as_deref().map(parse_parameter).transpose()?,
                routing: self.routing.as_deref().map(parse_routing).transpose()?,
            },
        )?;
        Ok(session)
    }
    fn operation(self) -> Result<mgb_ops::app::OperationalRun, String> {
        self.session()?
            .run_setup()
            .map(|s| s.operation)
            .ok_or_else(|| "Set valid run defaults".into())
    }
}

#[derive(Args, Clone, Default)]
struct InputPolicy {
    /// Save the previous unsaved inputs before replacing them.
    #[arg(long, conflicts_with = "discard_inputs")]
    save_inputs_to: Option<PathBuf>,
    /// Explicitly discard the previous inputs when replacing them.
    #[arg(long)]
    discard_inputs: bool,
}
impl InputPolicy {
    fn control(&self, control: &mgb_ops::OperationControl) -> mgb_ops::OperationControl {
        if let Some(destination) = &self.save_inputs_to {
            control
                .clone()
                .with_input_disposition(mgb_ops::app::InputDisposition::Save {
                    destination: destination.clone(),
                })
        } else if self.discard_inputs {
            control
                .clone()
                .with_input_disposition(mgb_ops::app::InputDisposition::Discard)
        } else {
            control.clone()
        }
    }
}
#[derive(Subcommand)]
enum InputCommand {
    /// Capture a self-contained input set from the current session selections.
    Capture(RunOptions),
    /// Validate and inspect an independently saved input set.
    Open {
        #[arg(long)]
        inputs: PathBuf,
    },
    /// Save the current input set independently of acquisition assets and outputs.
    Save {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Discard current inputs, preserving run manifests.
    Discard {
        #[arg(long)]
        workspace: PathBuf,
    },
    /// Regenerate and run using a matching installed executable.
    Execute {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        inputs: PathBuf,
        #[arg(long)]
        engine: PathBuf,
        #[command(flatten)]
        policy: InputPolicy,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum OperationCommand {
    /// Acquire configured inputs, prepare, execute and publish.
    RunAll(RunOptions),
    /// Freeze selected saved inputs to a new directory for later execution.
    Prepare {
        #[command(flatten)]
        options: RunOptions,
        #[arg(long)]
        output: PathBuf,
    },
    Execute {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        prepared: PathBuf,
        #[command(flatten)]
        policy: InputPolicy,
        #[arg(long)]
        json: bool,
    },
}
#[derive(Subcommand)]
enum ForecastCommand {
    /// Download and register the configured forecasts.
    Acquire(AcquisitionOptions),
    ImportCanonical {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        model: String,
        #[arg(long)]
        product: String,
        #[arg(long)]
        cycle: chrono::DateTime<chrono::Utc>,
    },
    Normalize {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        model: String,
        #[arg(long)]
        product: String,
        #[arg(long)]
        cycle: String,
        #[arg(long)]
        json: bool,
    },
}

fn parse_provider(value: &str) -> Result<mgb_ops::store::ForecastProvider, String> {
    match value {
        "ecmwf" => Ok(mgb_ops::store::ForecastProvider::Ecmwf),
        "noaa" => Ok(mgb_ops::store::ForecastProvider::Noaa),
        _ => Err("Use ecmwf or noaa".into()),
    }
}

#[derive(Subcommand)]
enum ConfigCommand {
    Example,
}
#[derive(Subcommand)]
enum WorkspaceCommand {
    /// Initialize the workspace assets using config.toml.
    Init {
        #[arg(long)]
        workspace: PathBuf,
    },
    Inspect {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        gpkg_path: Option<PathBuf>,
        #[arg(long)]
        engine_executable: Option<PathBuf>,
        #[arg(long)]
        engine_input_dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}
fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    if let Command::Assistant(options) = cli.command {
        return assistant::run(options);
    }
    let control = mgb_ops::OperationControl::default();
    if matches!(
        &cli.command,
        Command::Inundation { .. }
            | Command::Acquisition { .. }
            | Command::Observation {
                command: ObservationCommand::ImportAnaCsv { .. }
            }
    ) {
        let signal_control = control.clone();
        ctrlc::set_handler(move || signal_control.cancel())?;
    }
    let stdout = io::stdout();
    let mut out = stdout.lock();
    match cli.command {
        Command::Assistant(_) => unreachable!("handled interactive command"),
        Command::Acquisition { command } => acquisition::run(command, &control, &mut out),
        Command::Automation { command } => automation_action(command, &mut out),
        Command::Report { command } => {
            match command {
                ReportCommand::Generate { request } => serde_json::to_writer(
                    &mut out,
                    &mgb_ops::app::generate_bulletin(&read_json(&request)?)?,
                )?,
                ReportCommand::ReturnPeriod { request } => serde_json::to_writer(
                    &mut out,
                    &mgb_ops::app::export_return_period(&read_json(&request)?)?,
                )?,
            }
            writeln!(out)?;
            Ok(true)
        }
        Command::Edits { command } => {
            let count = match command {
                EditsCommand::Import { workspace, input } => {
                    mgb_ops::app::import_workspace_edits(&workspace, &input)?
                }
                EditsCommand::Export { workspace, output } => {
                    mgb_ops::app::export_workspace_edits(&workspace, &output)?
                }
            };
            writeln!(out, "{count} edits")?;
            Ok(true)
        }
        Command::Observation { command } => {
            use mgb_ops::store::ObservationStore;
            let result = match command {
                ObservationCommand::Acquire(options) => {
                    let json = options.json;
                    let result = progress::acquire(&mut out, json, |control| {
                        mgb_ops::app::acquire_inputs(
                            &options.request(),
                            mgb_ops::app::AcquisitionKind::Observations,
                            control,
                        )
                    })?;
                    return acquisition_result(result, json, true, &mut out);
                }
                ObservationCommand::ImportAnaCsv { options, input } => {
                    let json = options.json;
                    return acquisition_result(
                        mgb_ops::app::import_ana_csv(&options.request(), &input, &control),
                        json,
                        false,
                        &mut out,
                    );
                }
                ObservationCommand::Init { assets, time_zone } => {
                    ObservationStore::create(&assets, &time_zone)?;
                    serde_json::to_value(BTreeMap::from([("assets", assets)]))?
                }
                ObservationCommand::Import { assets, input } => {
                    let batch: mgb_ops::store::ObservationBatch = read_json(&input)?;
                    ObservationStore::open_writable(&assets)?.ingest_batch(&batch)?;
                    serde_json::to_value(BTreeMap::from([
                        ("stations", batch.stations.len()),
                        ("samples", batch.samples.len()),
                    ]))?
                }
                ObservationCommand::ImportInventory { workspace, input } => {
                    let count = mgb_ops::app::import_workspace_inventory(&workspace, &input)?;
                    serde_json::to_value(BTreeMap::from([("stations", count)]))?
                }
                ObservationCommand::ExportInventory { workspace, output } => {
                    let count = mgb_ops::app::export_workspace_inventory(&workspace, &output)?;
                    serde_json::to_value(BTreeMap::from([("stations", count)]))?
                }
                ObservationCommand::Inventory { assets, input } => {
                    let stations: Vec<mgb_ops::store::Station> = read_json(&input)?;
                    ObservationStore::open_writable(&assets)?
                        .update_station_inventory(&stations)?;
                    serde_json::to_value(BTreeMap::from([("stations", stations.len())]))?
                }
                ObservationCommand::Availability { request } => {
                    serde_json::to_value(mgb_ops::app::review_observations(&read_json(&request)?)?)?
                }
                ObservationCommand::SpatialQc {
                    request,
                    station,
                    interpolation,
                } => serde_json::to_value(mgb_ops::app::spatial_observation_qc(
                    &read_json(&request)?,
                    station.as_deref(),
                    &read_json(&interpolation)?,
                )?)?,
            };
            serde_json::to_writer(&mut out, &result)?;
            writeln!(out)?;
            Ok(true)
        }
        Command::Forcing {
            command:
                ForcingCommand::Import {
                    kind,
                    input,
                    output,
                },
        } => {
            let kind = match kind {
                ForcingKind::Reservoirs => mgb_ops::app::CanonicalForcingKind::Reservoirs,
                ForcingKind::Wind => mgb_ops::app::CanonicalForcingKind::Wind,
            };
            mgb_ops::app::save_canonical_forcing(&input, &output, kind)?;
            serde_json::to_writer(&mut out, &output)?;
            writeln!(out)?;
            Ok(true)
        }
        Command::Inundation {
            command:
                InundationCommand::Process {
                    run,
                    workspace,
                    scenario,
                    hand,
                    catchments,
                    replace,
                    json,
                },
        } => {
            use mgb_ops::app::{
                InundationProcessRequest, InundationProcessStatus, process_inundation,
                process_workspace_inundation,
            };
            let result = if let Some(workspace) = workspace {
                if hand.is_some() || catchments.is_some() {
                    return Err("--workspace uses the raster paths in config.toml".into());
                }
                process_workspace_inundation(&workspace, run, scenario, replace, &control)
            } else {
                process_inundation(
                    InundationProcessRequest {
                        run: run.ok_or("Select --workspace or --run")?,
                        scenario,
                        hand: hand.ok_or("Set --hand with an explicit run")?,
                        catchments: catchments.ok_or("Set --catchments with an explicit run")?,
                        replace,
                    },
                    &control,
                )
            };
            match result {
                Ok(report) if json => serde_json::to_writer(&mut out, &report)?,
                Ok(report) => {
                    let action = match report.status {
                        InundationProcessStatus::Created => "Created",
                        InundationProcessStatus::Reused => "Reused",
                        InundationProcessStatus::Replaced => "Replaced",
                    };
                    write!(out, "{action} {}", report.path.display())?;
                }
                Err(error) if json => {
                    serde_json::to_writer(
                        &mut out,
                        &BTreeMap::from([("error", error.to_string())]),
                    )?;
                    writeln!(out)?;
                    return Ok(false);
                }
                Err(error) => return Err(Box::new(error)),
            }
            writeln!(out)?;
            Ok(true)
        }
        Command::Calibration { command } => {
            calibration_action(command, &mut out)?;
            Ok(true)
        }
        Command::Run { command } => {
            match command {
                RunCommand::Export {
                    run,
                    inputs,
                    output,
                } => {
                    let report = mgb_ops::app::export_run(&run, inputs.as_deref(), &output)?;
                    serde_json::to_writer(&mut out, &report)?;
                }
                RunCommand::Import {
                    workspace,
                    bundle,
                    output,
                } => {
                    let report = mgb_ops::app::import_run_to(&workspace, &bundle, &output)?;
                    serde_json::to_writer(&mut out, &report)?;
                }
                RunCommand::Regenerate {
                    workspace,
                    bundle,
                    engine,
                    policy,
                    json,
                } => {
                    let result = progress::run(&mut out, true, |control| {
                        mgb_ops::app::regenerate_run(
                            &workspace,
                            &bundle,
                            &engine,
                            &policy.control(control),
                        )
                    })?;
                    let Some(report) = operation_result(result, json, &mut out)? else {
                        return Ok(false);
                    };
                    serde_json::to_writer(&mut out, &report)?;
                }
            }
            writeln!(out)?;
            Ok(true)
        }
        Command::Series { command, json } => {
            let result: Result<serde_json::Value, String> = (|| {
                let (path, destination) = match command {
                    SeriesCommand::Query { request } => (request, None),
                    SeriesCommand::Export { request, output } => (request, Some(output)),
                    SeriesCommand::Analyze { request, output } => {
                        let request = mgb_ops::store::read_draft(&request)?;
                        let report = mgb_ops::app::analyze_hydrograph(&request)?;
                        if let Some(path) = output {
                            mgb_ops::app::export_hydrograph(&report, &path)?;
                        }
                        return serde_json::to_value(report).map_err(|e| e.to_string());
                    }
                    SeriesCommand::ExportBatch { request, output } => {
                        let requests: Vec<mgb_ops::app::HydrographBatchItem> =
                            mgb_ops::store::read_draft(&request)?;
                        return serde_json::to_value(mgb_ops::app::export_hydrograph_batch(
                            &requests, &output,
                        )?)
                        .map_err(|e| e.to_string());
                    }
                };
                let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
                let request: mgb_ops::SeriesQuery =
                    serde_json::from_reader(file).map_err(|e| e.to_string())?;
                let report = match destination {
                    Some(path) => mgb_ops::export_series_csv(&request, &path),
                    None => mgb_ops::query_series(&request),
                }?;
                serde_json::to_value(report).map_err(|e| e.to_string())
            })();
            let ok = result.is_ok();
            if json {
                let mut value = serde_json::Map::new();
                value.insert("ok".into(), ok.into());
                match result {
                    Ok(result) => {
                        value.insert("result".into(), result);
                    }
                    Err(error) => {
                        value.insert("error".into(), error.into());
                    }
                }
                serde_json::to_writer(&mut out, &value)?;
                writeln!(out)?;
            } else {
                let value = result?;
                serde_json::to_writer(&mut out, &value)?;
                writeln!(out)?;
            }
            Ok(ok)
        }
        Command::References { command, json } => {
            let result = match command {
                ReferenceCommand::Import {
                    assets,
                    provider,
                    thresholds,
                    history,
                } => mgb_ops::import_level_references(&assets, &provider, &thresholds, &history)
                    .and_then(|report| serde_json::to_value(report).map_err(|e| e.to_string())),
            };
            if json {
                let ok = result.is_ok();
                let mut output = serde_json::Map::new();
                output.insert("ok".into(), ok.into());
                match result {
                    Ok(value) => {
                        output.insert("result".into(), value);
                    }
                    Err(error) => {
                        output.insert("error".into(), error.into());
                    }
                }
                serde_json::to_writer(&mut out, &output)?;
                writeln!(out)?;
                out.flush()?;
                Ok(ok)
            } else {
                let value = result?;
                serde_json::to_writer(&mut out, &value)?;
                writeln!(out)?;
                Ok(true)
            }
        }
        Command::Inputs { command } => {
            match command {
                InputCommand::Capture(options) => {
                    let json = options.json;
                    let policy = options.input_policy.clone();
                    let force = options.force;
                    let session = options.session()?;
                    let result = progress::run(&mut out, json, |control| {
                        mgb_ops::app::capture_workspace_inputs(
                            &session,
                            &policy.control(control).with_force_acquisition(force),
                        )
                    })?;
                    let manifest = result?;
                    serde_json::to_writer_pretty(&mut out, &manifest)?;
                }
                InputCommand::Open { inputs } => {
                    serde_json::to_writer_pretty(
                        &mut out,
                        &mgb_ops::store::read_input_set(&inputs)?,
                    )?;
                }
                InputCommand::Save { workspace, output } => {
                    serde_json::to_writer_pretty(
                        &mut out,
                        &mgb_ops::app::save_workspace_inputs(&workspace.canonicalize()?, &output)?,
                    )?;
                }
                InputCommand::Discard { workspace } => {
                    mgb_ops::app::discard_workspace_inputs(&workspace.canonicalize()?)?;
                }
                InputCommand::Execute {
                    workspace,
                    inputs,
                    engine,
                    policy,
                    json,
                } => {
                    let result = progress::run(&mut out, json, |control| {
                        mgb_ops::app::execute_input_set(
                            &workspace,
                            &inputs,
                            &engine,
                            &policy.control(control),
                        )
                    })?;
                    let Some(report) = operation_result(result, json, &mut out)? else {
                        return Ok(false);
                    };
                    serde_json::to_writer_pretty(&mut out, &report)?;
                }
            }
            writeln!(out)?;
            Ok(true)
        }
        Command::Operation {
            command: OperationCommand::RunAll(options),
        } => {
            let json = options.json;
            let report = progress::run_all(&mut out, json, |control| {
                let configured = options
                    .input_policy
                    .control(control)
                    .with_force_acquisition(options.force);
                options
                    .operation()
                    .map(|operation| operation.run_all_report(&configured))
            })?;
            let report = match report {
                Ok(report) => report,
                Err(error) if json => {
                    serde_json::to_writer(&mut out, &BTreeMap::from([("error", error)]))?;
                    writeln!(out)?;
                    return Ok(false);
                }
                Err(error) => return Err(error.into()),
            };
            if json {
                serde_json::to_writer(&mut out, &report)?;
                writeln!(out)?;
            }
            Ok(report.error.is_none())
        }
        Command::Operation {
            command: OperationCommand::Prepare { options, output },
        } => {
            let invocation_dir = std::env::current_dir()?;
            let json = options.json;
            let result = progress::run(&mut out, json, |control| {
                options
                    .operation()
                    .map_err(|message| mgb_ops::app::OperationError::InvalidRequest {
                        field: "workspace".into(),
                        message,
                    })
                    .and_then(|operation| operation.prepare_saved(invocation_dir, output, control))
            })?;
            let Some(report) = operation_result(result, json, &mut out)? else {
                return Ok(false);
            };
            if json {
                serde_json::to_writer(&mut out, &report)?;
                writeln!(out)?;
            }
            out.flush()?;
            Ok(true)
        }
        Command::Operation {
            command:
                OperationCommand::Execute {
                    workspace,
                    prepared,
                    policy,
                    json,
                },
        } => {
            let invocation_dir = std::env::current_dir()?;
            let result = progress::run(&mut out, json, |control| {
                mgb_ops::execute_prepared_controlled(
                    mgb_ops::ExecutionRequest {
                        workspace,
                        invocation_dir,
                        prepared,
                    },
                    &policy.control(control),
                )
            })?;
            let Some(report) = operation_result(result, json, &mut out)? else {
                return Ok(false);
            };
            if json {
                serde_json::to_writer(&mut out, &report)?;
                writeln!(out)?;
            }
            out.flush()?;
            Ok(matches!(report.status, mgb_ops::BatchStatus::Completed))
        }
        Command::Forecast {
            command: ForecastCommand::Acquire(options),
        } => {
            let json = options.json;
            let result = progress::acquire(&mut out, json, |control| {
                let configured = control.clone().with_force_acquisition(options.force);
                mgb_ops::app::acquire_inputs(
                    &options.request(),
                    mgb_ops::app::AcquisitionKind::Forecasts,
                    &configured,
                )
            })?;
            acquisition_result(result, json, true, &mut out)
        }
        Command::Forecast {
            command:
                ForecastCommand::ImportCanonical {
                    workspace,
                    source,
                    provider,
                    model,
                    product,
                    cycle,
                },
        } => {
            let report = mgb_ops::app::import_workspace_forecast(
                &workspace,
                std::env::current_dir()?.join(source),
                mgb_ops::store::CanonicalForecastMetadata {
                    provider,
                    model,
                    product,
                    cycle_utc: cycle,
                },
            )?;
            serde_json::to_writer(&mut out, &report)?;
            writeln!(out)?;
            Ok(true)
        }
        Command::Forecast {
            command:
                ForecastCommand::Normalize {
                    workspace,
                    source,
                    provider,
                    model,
                    product,
                    cycle,
                    json,
                },
        } => {
            let workspace = workspace.canonicalize()?;
            let source = std::env::current_dir()?.join(source);
            let provider = parse_provider(&provider)?;
            let label = format!(
                "{}/{model}/{product}@{cycle}",
                match provider {
                    mgb_ops::store::ForecastProvider::Ecmwf => "ecmwf",
                    mgb_ops::store::ForecastProvider::Noaa => "noaa",
                }
            );
            let report = mgb_ops::app::normalize_workspace_forecast(
                &workspace, source, provider, model, product, cycle,
            )?;
            if json {
                serde_json::to_writer(&mut out, &report)?;
                writeln!(out)?;
            } else {
                writeln!(out, "Forecast: {label}")?;
                writeln!(out, "Native artifact: {}", report.native_artifact.display())?;
            }
            out.flush()?;
            Ok(true)
        }
        Command::Config {
            command: ConfigCommand::Example,
        } => {
            write!(out, "{}", mgb_ops::example_config())?;
            out.flush()?;
            Ok(true)
        }
        Command::Workspace {
            command: WorkspaceCommand::Init { workspace },
        } => {
            let assets = mgb_ops::app::initialize_workspace(&workspace)?;
            writeln!(out, "{}", assets.display())?;
            Ok(true)
        }
        Command::Workspace {
            command:
                WorkspaceCommand::Inspect {
                    workspace,
                    config,
                    gpkg_path,
                    engine_executable,
                    engine_input_dir,
                    json,
                },
        } => {
            let report = mgb_ops::inspect(mgb_ops::InspectionRequest {
                workspace,
                invocation_dir: std::env::current_dir()?,
                config,
                gpkg_path,
                engine_executable,
                engine_input_dir,
            });
            if json {
                serde_json::to_writer(&mut out, &report)?;
                writeln!(out)?;
            } else {
                writeln!(out, "Workspace: {}", report.workspace.display())?;
                for f in &report.findings {
                    let path = f
                        .path
                        .as_ref()
                        .map_or_else(String::new, |p| format!(" [{}]", p.display()));
                    writeln!(
                        out,
                        "{}: {} ({}){}: {}",
                        f.check, f.status, f.code, path, f.message
                    )?;
                }
                writeln!(
                    out,
                    "Inspection {} (bounded prerequisite checks only).",
                    if report.ok { "passed" } else { "failed" }
                )?;
            }
            out.flush()?;
            Ok(report.ok)
        }
    }
}
fn main() {
    let mut arguments = std::env::args_os().skip(1);
    if arguments
        .next()
        .is_some_and(|arg| arg == "--assistant-mcp-stdio")
        && arguments.next().is_none()
    {
        if let Err(error) = mgb_ops::app::assistant_mcp_stdio() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    match run() {
        Ok(true) => (),
        Ok(false) => std::process::exit(1),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
