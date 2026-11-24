use clap::{Parser, Subcommand, ValueEnum};
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
    Automation {
        #[command(subcommand)]
        command: AutomationCommand,
    },
    Report {
        #[command(subcommand)]
        command: ReportCommand,
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
enum ObservationCommand {
    Init {
        #[arg(long)]
        database: PathBuf,
        #[arg(long)]
        time_zone: String,
    },
    Upgrade {
        #[arg(long)]
        database: PathBuf,
    },
    Import {
        #[arg(long)]
        database: PathBuf,
        #[arg(long)]
        input: PathBuf,
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
    Upgrade {
        #[arg(long)]
        database: PathBuf,
    },
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

fn draft_action(
    command: DraftCommand,
    invocation: &std::path::Path,
    control: &mgb_ops::OperationControl,
) -> Result<serde_json::Value, String> {
    fn value<T: serde::Serialize>(data: T) -> Result<serde_json::Value, String> {
        serde_json::to_value(data).map_err(|e| e.to_string())
    }
    fn path(invocation: &std::path::Path, path: &std::path::Path) -> PathBuf {
        if path.is_absolute() {
            path.into()
        } else {
            invocation.join(path)
        }
    }
    match command {
        DraftCommand::New {
            workspace,
            output,
            reference,
            time_zone,
        } => {
            let mut draft = mgb_ops::Draft::new(workspace, invocation);
            if let (Some(reference), Some(zone)) = (reference, time_zone) {
                draft.initialize_anchor(&reference, &zone)?;
            }
            draft.save(&path(invocation, &output), true)?;
            value(draft.document)
        }
        DraftCommand::Open { workspace, draft } => {
            let draft = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            let mut result = serde_json::Map::new();
            result.insert("document".into(), value(draft.document)?);
            result.insert("findings".into(), value(draft.findings)?);
            Ok(result.into())
        }
        DraftCommand::Save {
            workspace,
            draft,
            output,
            omit_visualization,
        } => {
            let mut current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            current.save(&output, !omit_visualization)?;
            let saved = mgb_ops::Draft::open(current.workspace.clone(), invocation, &output)?;
            value(saved.document)
        }
        DraftCommand::ImportToml {
            workspace,
            request,
            output,
        } => {
            let mut draft = mgb_ops::Draft::import_toml(workspace, invocation, &request)?;
            draft.save(&path(invocation, &output), true)?;
            value(draft.document)
        }
        DraftCommand::ImportManifest {
            workspace,
            manifest,
            output,
        } => {
            let mut draft = mgb_ops::Draft::import_manifest(workspace, invocation, &manifest)?;
            draft.save(&path(invocation, &output), true)?;
            value(draft.document)
        }
        DraftCommand::Edit {
            workspace,
            draft,
            document,
        } => {
            let mut replacement = mgb_ops::Draft::open(workspace.clone(), invocation, &document)?;
            if replacement
                .findings
                .iter()
                .any(|v| v == "saved workspace differs from selected workspace")
            {
                return Err("saved workspace differs from selected workspace".into());
            }
            let mut current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            let existing = &current.document.operation;
            let proposed = &replacement.document.operation;
            if (existing.reference_time.is_none() || existing.time_zone.is_none())
                && let (Some(reference), Some(zone)) =
                    (&proposed.reference_time, &proposed.time_zone)
            {
                current.initialize_anchor(reference, zone)?;
                replacement.document.sources = Default::default();
            }
            current.document = replacement.document;
            current.save(&path(invocation, &draft), true)?;
            value(current.document)
        }
        DraftCommand::Validate {
            workspace,
            draft,
            action,
            scenario,
        } => {
            let draft = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            let action = match action.as_str() {
                "preparation" => mgb_ops::DraftAction::Preparation,
                "observations" => mgb_ops::DraftAction::ObservationPreview,
                "forecast" => mgb_ops::DraftAction::ForecastPreview {
                    scenario: scenario.ok_or("forecast validation requires --scenario")?,
                },
                _ => return Err("action must be preparation, observations or forecast".into()),
            };
            value(draft.validate(action))
        }
        DraftCommand::Refresh {
            workspace,
            draft,
            source,
            scenario,
        } => {
            let mut current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            match source.as_str() {
                "all" => current.refresh_inputs()?,
                "observations" => current.refresh_observations()?,
                "forecast" => current
                    .refresh_forecast(scenario.ok_or("forecast refresh requires --scenario")?)?,
                _ => return Err("source must be all, observations or forecast".into()),
            }
            current.save(&path(invocation, &draft), true)?;
            value(current.document)
        }
        DraftCommand::PreviewObservations { workspace, draft } => {
            value(mgb_ops::Draft::open(workspace, invocation, &draft)?.preview_observations()?)
        }
        DraftCommand::PreviewForecast {
            workspace,
            draft,
            scenario,
        } => {
            value(mgb_ops::Draft::open(workspace, invocation, &draft)?.preview_forecast(scenario)?)
        }
        DraftCommand::Materialize {
            workspace,
            draft,
            source,
            scenario,
        } => {
            let mut current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            match source.as_str() {
                "observations" => current.materialize_replacements()?,
                "forecast" => current.materialize_corrections(
                    scenario.ok_or("forecast materialization requires --scenario")?,
                )?,
                _ => return Err("source must be observations or forecast".into()),
            }
            current.save(&path(invocation, &draft), true)?;
            value(current.document)
        }
        DraftCommand::CopyRegisteredForEdit {
            workspace,
            draft,
            mapping,
        } => {
            let mut current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            let mapping: mgb_ops::RegistrationMapping = serde_json::from_slice(
                &std::fs::read(path(invocation, &mapping)).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            current.copy_registered_for_edit(&mapping)?;
            current.save(&path(invocation, &draft), true)?;
            value(current.document)
        }
        DraftCommand::Register {
            workspace,
            draft,
            selection,
        } => {
            let current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            let selection: mgb_ops::RegistrationSelection = serde_json::from_slice(
                &std::fs::read(path(invocation, &selection)).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            value(current.register_selected(&selection))
        }
        DraftCommand::Run { workspace, draft } => {
            let draft = mgb_ops::Draft::open(workspace, invocation, &draft)?;
            value(mgb_ops::run_draft(&draft, control).map_err(|e| e.to_string())?)
        }
        DraftCommand::Prepare {
            workspace,
            draft,
            output,
        } => value(
            mgb_ops::Draft::open(workspace, invocation, &draft)?
                .prepare_controlled(invocation.into(), output, control)
                .map_err(|e| e.to_string())?,
        ),
    }
}

#[derive(Subcommand)]
enum InundationCommand {
    /// Process an aligned HAND and catchments GeoTIFF pair for a retained scenario.
    Process {
        #[arg(long)]
        run: PathBuf,
        #[arg(long)]
        scenario: String,
        #[arg(long)]
        hand: PathBuf,
        #[arg(long)]
        catchments: PathBuf,
        #[arg(long)]
        replace: bool,
        #[arg(long)]
        json: bool,
    },
}
#[derive(Subcommand)]
enum OperationCommand {
    Prepare {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        output: PathBuf,
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
fn operation_result<T>(
    result: Result<T, mgb_ops::OperationError>,
    json: bool,
    out: &mut impl Write,
) -> Result<Option<T>, Box<dyn std::error::Error>> {
    match result {
        Ok(report) => Ok(Some(report)),
        Err(error) if json => {
            serde_json::to_writer(&mut *out, &error)?;
            writeln!(out)?;
            out.flush()?;
            Ok(None)
        }
        Err(error) => Err(Box::new(error)),
    }
}
fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let control = mgb_ops::OperationControl::default();
    if matches!(
        &cli.command,
        Command::Operation {
            command: OperationCommand::Prepare { .. } | OperationCommand::Execute { .. }
        } | Command::Draft {
            command: DraftCommand::Prepare { .. } | DraftCommand::Run { .. },
            ..
        } | Command::Run {
            command: RunCommand::Regenerate { .. } | RunCommand::Replay { .. },
        } | Command::Inundation { .. }
    ) {
        let signal_control = control.clone();
        ctrlc::set_handler(move || signal_control.cancel())?;
    }
    let stdout = io::stdout();
    let mut out = stdout.lock();
    match cli.command {
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
        Command::Observation { command } => {
            use mgb_ops::store::ObservationStore;
            let result = match command {
                ObservationCommand::Init {
                    database,
                    time_zone,
                } => {
                    ObservationStore::create(&database, &time_zone)?;
                    serde_json::to_value(BTreeMap::from([("database", database)]))?
                }
                ObservationCommand::Upgrade { database } => {
                    ObservationStore::open_writable(&database)?.upgrade_replacements()?;
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
                    scenario,
                    hand,
                    catchments,
                    replace,
                    json,
                },
        } => {
            use mgb_ops::app::{
                InundationProcessRequest, InundationProcessStatus, process_inundation,
            };
            let result = process_inundation(
                InundationProcessRequest {
                    run,
                    scenario,
                    hand,
                    catchments,
                    replace,
                },
                &control,
            );
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
                    let result = mgb_ops::app::replay_run(&workspace, &manifest, &control);
                    let Some(report) = operation_result(result, json, &mut out)? else {
                        return Ok(false);
                    };
                    let completed = matches!(report.status, mgb_ops::BatchStatus::Completed);
                    if json {
                        serde_json::to_writer(&mut out, &report)?;
                    } else {
                        write!(
                            out,
                            "Run {} ({:?})",
                            report.run_root.display(),
                            report.status
                        )?;
                    }
                    if !completed {
                        writeln!(out)?;
                        return Ok(false);
                    }
                }
                RunCommand::Regenerate {
                    workspace,
                    bundle,
                    engine,
                    json,
                } => {
                    let result =
                        mgb_ops::app::regenerate_run(&workspace, &bundle, &engine, &control);
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
                ReferenceCommand::Upgrade { database } => {
                    mgb_ops::upgrade_level_references(&database).map(|()| serde_json::Value::Null)
                }
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
                if value.is_null() {
                    writeln!(out, "reference schema upgraded")?;
                } else {
                    serde_json::to_writer(&mut out, &value)?;
                    writeln!(out)?;
                }
                Ok(true)
            }
        }
        Command::Draft { command, json: _ } => {
            let is_validation = matches!(&command, DraftCommand::Validate { .. });
            let is_registration = matches!(&command, DraftCommand::Register { .. });
            let result = draft_action(command, &std::env::current_dir()?, &control);
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
                OperationCommand::Prepare {
                    workspace,
                    request,
                    output,
                    json,
                },
        } => {
            let result = mgb_ops::prepare_controlled(
                mgb_ops::PreparationRequest {
                    workspace,
                    invocation_dir: std::env::current_dir()?,
                    request_file: request,
                    output,
                },
                &control,
            );
            let Some(report) = operation_result(result, json, &mut out)? else {
                return Ok(false);
            };
            if json {
                serde_json::to_writer(&mut out, &report)?;
                writeln!(out)?;
            } else {
                writeln!(out, "Prepared: {}", report.prepared.display())?;
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
            let result = mgb_ops::execute_prepared_controlled(
                mgb_ops::ExecutionRequest {
                    workspace,
                    invocation_dir: std::env::current_dir()?,
                    prepared,
                },
                &control,
            );
            let Some(report) = operation_result(result, json, &mut out)? else {
                return Ok(false);
            };
            if json {
                serde_json::to_writer(&mut out, &report)?;
                writeln!(out)?;
            } else {
                writeln!(
                    out,
                    "Run: {} ({:?})",
                    report.run_root.display(),
                    report.status
                )?;
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
    match run() {
        Ok(true) => (),
        Ok(false) => std::process::exit(1),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
