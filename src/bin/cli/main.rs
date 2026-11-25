mod assistant;
mod draft;
mod output;
mod progress;

use self::draft::draft_action;
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
    Draft {
        #[command(subcommand)]
        command: DraftCommand,
        #[arg(long, global = true)]
        json: bool,
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
    /// Replace the ordered edits for observations or a forecast cycle.
    Import {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        forecast: Option<String>,
    },
    Export {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        forecast: Option<String>,
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
        database: PathBuf,
        #[arg(long)]
        time_zone: String,
    },
    Import {
        #[arg(long)]
        database: PathBuf,
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
        database: PathBuf,
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
    workspace: PathBuf,
    #[arg(long)]
    request: Option<PathBuf>,
    #[arg(long)]
    reference_time: Option<String>,
    #[arg(long)]
    json: bool,
}

impl AcquisitionOptions {
    fn request(self) -> mgb_ops::app::AcquisitionRequest {
        mgb_ops::app::AcquisitionRequest {
            workspace: self.workspace,
            request_file: self.request.unwrap_or_else(|| "config.toml".into()),
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
        prepared: Option<PathBuf>,
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
    Replay {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Regenerate {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        engine: PathBuf,
        #[arg(long)]
        json: bool,
    },
}
#[derive(Subcommand)]
enum ReferenceCommand {
    Import {
        #[arg(long)]
        database: PathBuf,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        thresholds: PathBuf,
        #[arg(long)]
        history: PathBuf,
    },
}
#[derive(Subcommand)]
enum DraftCommand {
    New {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, requires = "time_zone")]
        reference: Option<String>,
        #[arg(long, requires = "reference")]
        time_zone: Option<String>,
    },
    Open {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        draft: PathBuf,
    },
    Save {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        draft: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        omit_visualization: bool,
    },
    ImportToml {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    ImportManifest {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Edit {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        draft: PathBuf,
        #[arg(long)]
        document: PathBuf,
    },
    Validate {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        draft: PathBuf,
        #[arg(long, default_value = "preparation")]
        action: String,
        #[arg(long)]
        scenario: Option<usize>,
    },
    Refresh {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        draft: PathBuf,
        #[arg(long, default_value = "all")]
        source: String,
        #[arg(long)]
        scenario: Option<usize>,
    },
    PreviewObservations {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        draft: PathBuf,
    },
    PreviewForecast {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        draft: PathBuf,
        #[arg(long)]
        scenario: usize,
    },
    Materialize {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        draft: PathBuf,
        #[arg(long)]
        source: String,
        #[arg(long)]
        scenario: Option<usize>,
    },
    CopyRegisteredForEdit {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        draft: PathBuf,
        #[arg(long)]
        mapping: PathBuf,
    },
    Register {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        draft: PathBuf,
        #[arg(long)]
        selection: PathBuf,
    },
    Run {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        draft: PathBuf,
    },
    Prepare {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        draft: PathBuf,
        #[arg(long)]
        output: PathBuf,
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
#[derive(Subcommand)]
enum OperationCommand {
    /// Acquire inputs when the recipe enables acquisition, then prepare and execute.
    RunAll {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        request: Option<PathBuf>,
        #[arg(long)]
        reference_time: Option<String>,
        #[arg(long = "scenario")]
        scenarios: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    Prepare {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        request: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        reference_time: Option<String>,
        #[arg(long = "scenario")]
        scenarios: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    Execute {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        prepared: PathBuf,
        #[arg(long)]
        json: bool,
    },
    MigrateCache {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        keep_run: String,
        #[arg(long)]
        inundation_donor: Option<String>,
        #[arg(long)]
        apply: bool,
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
        request: PathBuf,
    },
    Normalize {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    Example,
}
#[derive(Subcommand)]
enum WorkspaceCommand {
    /// Initialize the workspace database using config.toml.
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
                EditsCommand::Import {
                    workspace,
                    input,
                    forecast,
                } => mgb_ops::app::import_workspace_edits(&workspace, &input, forecast.as_deref())?,
                EditsCommand::Export {
                    workspace,
                    output,
                    forecast,
                } => {
                    mgb_ops::app::export_workspace_edits(&workspace, &output, forecast.as_deref())?
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
                ObservationCommand::Init {
                    database,
                    time_zone,
                } => {
                    ObservationStore::create(&database, &time_zone)?;
                    serde_json::to_value(BTreeMap::from([("database", database)]))?
                }
                ObservationCommand::Import { database, input } => {
                    let batch: mgb_ops::store::ObservationBatch = read_json(&input)?;
                    ObservationStore::open_writable(&database)?.ingest_batch(&batch)?;
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
                ObservationCommand::Inventory { database, input } => {
                    let stations: Vec<mgb_ops::store::Station> = read_json(&input)?;
                    ObservationStore::open_writable(&database)?
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
                    prepared,
                    output,
                } => {
                    let report = mgb_ops::app::export_run(&run, prepared.as_deref(), &output)?;
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
                RunCommand::Replay {
                    workspace,
                    manifest,
                    json,
                } => {
                    let result = progress::run(&mut out, json, |control| {
                        mgb_ops::app::replay_run(&workspace, &manifest, control)
                    })?;
                    let Some(report) = operation_result(result, json, &mut out)? else {
                        return Ok(false);
                    };
                    if json {
                        serde_json::to_writer(&mut out, &report)?;
                        writeln!(out)?;
                    }
                    return Ok(matches!(report.status, mgb_ops::BatchStatus::Completed));
                }
                RunCommand::Regenerate {
                    workspace,
                    bundle,
                    engine,
                    json,
                } => {
                    let result = progress::run(&mut out, true, |control| {
                        mgb_ops::app::regenerate_run(&workspace, &bundle, &engine, control)
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
                    database,
                    provider,
                    thresholds,
                    history,
                } => mgb_ops::import_level_references(&database, &provider, &thresholds, &history)
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
        Command::Draft { command, json: _ } => {
            let is_validation = matches!(&command, DraftCommand::Validate { .. });
            let is_registration = matches!(&command, DraftCommand::Register { .. });
            let invocation = std::env::current_dir()?;
            let result = match command {
                DraftCommand::Run { workspace, draft } => progress::run_draft(
                    &mut out,
                    || mgb_ops::Draft::open(workspace, &invocation, &draft),
                    |draft, control| mgb_ops::run_draft(&draft, control),
                )?,
                DraftCommand::Prepare {
                    workspace,
                    draft,
                    output,
                } => progress::run_draft(
                    &mut out,
                    || mgb_ops::Draft::open(workspace, &invocation, &draft),
                    |draft, control| draft.prepare_controlled(invocation.clone(), output, control),
                )?,
                command => draft_action(command, &invocation),
            };
            let ok = result.as_ref().is_ok_and(|v| {
                (!is_validation || v.as_array().is_some_and(Vec::is_empty))
                    && (!is_registration
                        || (v["observations"]["error"].is_null()
                            && v["forecasts"]["error"].is_null()))
            });
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
        }
        Command::Operation {
            command:
                OperationCommand::RunAll {
                    workspace,
                    request,
                    reference_time,
                    scenarios,
                    json,
                },
        } => {
            let report = progress::run_all(&mut out, json, |control| {
                let now = chrono::Utc::now();
                let mut operation = match request {
                    Some(path) => mgb_ops::app::OperationalRun::open(workspace, &path, now)?,
                    None => mgb_ops::app::OperationalRun::open_workspace(workspace, now)?,
                };
                if let Some(reference) = reference_time {
                    operation.set_reference(reference);
                }
                if !scenarios.is_empty() {
                    operation.set_scenarios(&scenarios)?;
                }
                if !operation.acquires_inputs() {
                    operation.accept_inputs()?;
                }
                Ok(operation.run_all_report(control))
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
            command:
                OperationCommand::Prepare {
                    workspace,
                    request,
                    output,
                    reference_time,
                    scenarios,
                    json,
                },
        } => {
            let invocation_dir = std::env::current_dir()?;
            if request.is_some() && (reference_time.is_some() || !scenarios.is_empty()) {
                return Err("--reference-time and --scenario use workspace settings".into());
            }
            let result = if let Some(request_file) = request {
                progress::run(&mut out, json, |control| {
                    mgb_ops::prepare_controlled(
                        mgb_ops::PreparationRequest {
                            workspace,
                            invocation_dir,
                            request_file,
                            output,
                        },
                        control,
                    )
                })?
            } else {
                let mut operation =
                    mgb_ops::app::OperationalRun::open_workspace(workspace, chrono::Utc::now())
                        .map_err(|message| mgb_ops::app::OperationError::InvalidRequest {
                            field: "workspace".into(),
                            message,
                        })?;
                if let Some(reference) = reference_time {
                    operation.set_reference(reference);
                }
                if !scenarios.is_empty() {
                    operation.set_scenarios(&scenarios).map_err(|message| {
                        mgb_ops::app::OperationError::InvalidRequest {
                            field: "scenarios".into(),
                            message,
                        }
                    })?;
                }
                progress::run(&mut out, json, |control| {
                    operation.prepare_saved(invocation_dir, output, control)
                })?
            };
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
                    control,
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
        Command::Operation {
            command:
                OperationCommand::MigrateCache {
                    workspace,
                    keep_run,
                    inundation_donor,
                    apply,
                    json,
                },
        } => {
            let report = mgb_ops::app::migrate_cache(
                &workspace,
                &keep_run,
                inundation_donor.as_deref(),
                apply,
            )?;
            if json {
                serde_json::to_writer(&mut out, &report)?;
                writeln!(out)?;
            } else {
                writeln!(out, "{}", serde_json::to_string_pretty(&report)?)?;
            }
            Ok(true)
        }
        Command::Forecast {
            command: ForecastCommand::Acquire(options),
        } => {
            let json = options.json;
            let result = progress::acquire(&mut out, json, |control| {
                mgb_ops::app::acquire_inputs(
                    &options.request(),
                    mgb_ops::app::AcquisitionKind::Forecasts,
                    control,
                )
            })?;
            acquisition_result(result, json, true, &mut out)
        }
        Command::Forecast {
            command: ForecastCommand::ImportCanonical { workspace, request },
        } => {
            let report = mgb_ops::app::import_canonical_forecast(&workspace, read_json(&request)?)?;
            serde_json::to_writer(&mut out, &report)?;
            writeln!(out)?;
            Ok(true)
        }
        Command::Forecast {
            command:
                ForecastCommand::Normalize {
                    workspace,
                    request,
                    json,
                },
        } => {
            let report = mgb_ops::normalize_forecast(mgb_ops::NormalizeForecastRequest {
                workspace,
                invocation_dir: std::env::current_dir()?,
                request_file: request,
            })?;
            if json {
                serde_json::to_writer(&mut out, &report)?;
                writeln!(out)?;
            } else {
                writeln!(out, "Asset: {}", report.asset_id)?;
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
            let database = mgb_ops::app::initialize_workspace(&workspace)?;
            writeln!(out, "{}", database.display())?;
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
