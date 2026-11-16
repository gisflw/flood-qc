use clap::{Parser, Subcommand};
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
        DraftCommand::New { workspace, output } => {
            let mut draft = mgb_ops::Draft::new(workspace, invocation);
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
            let replacement = mgb_ops::Draft::open(workspace.clone(), invocation, &document)?;
            if replacement
                .findings
                .iter()
                .any(|v| v == "saved workspace differs from selected workspace")
            {
                return Err("saved workspace differs from selected workspace".into());
            }
            let mut current = mgb_ops::Draft::open(workspace, invocation, &draft)?;
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
}
#[derive(Subcommand)]
enum ForecastCommand {
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
        }
    ) {
        let signal_control = control.clone();
        ctrlc::set_handler(move || signal_control.cancel())?;
    }
    let stdout = io::stdout();
    let mut out = stdout.lock();
    match cli.command {
        Command::Series { command, json } => {
            let result: Result<serde_json::Value, String> = (|| {
                let (path, destination) = match command {
                    SeriesCommand::Query { request } => (request, None),
                    SeriesCommand::Export { request, output } => (request, Some(output)),
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
