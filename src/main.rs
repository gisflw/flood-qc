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
fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    match cli.command {
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
