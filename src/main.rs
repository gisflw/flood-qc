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
    let stdout = io::stdout();
    let mut out = stdout.lock();
    match cli.command {
        Command::Operation {
            command:
                OperationCommand::Prepare {
                    workspace,
                    request,
                    output,
                    json,
                },
        } => {
            let result = mgb_ops::prepare(mgb_ops::PreparationRequest {
                workspace,
                invocation_dir: std::env::current_dir()?,
                request_file: request,
                output,
            });
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
            let result = mgb_ops::execute_prepared(mgb_ops::ExecutionRequest {
                workspace,
                invocation_dir: std::env::current_dir()?,
                prepared,
            });
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
