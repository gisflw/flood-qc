use serde::Serialize;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Instant,
};

pub(crate) struct EnginePlan {
    pub executable: PathBuf,
    pub cwd: PathBuf,
    pub input: PathBuf,
    pub output: PathBuf,
    pub logs: PathBuf,
    pub omp_threads: usize,
    pub time_zone: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct EngineEnvironment {
    #[serde(rename = "MGB_INPUT_DIR")]
    pub input: PathBuf,
    #[serde(rename = "MGB_OUTPUT_DIR")]
    pub output: PathBuf,
    #[serde(rename = "OMP_NUM_THREADS")]
    pub omp_threads: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ProcessOutcome {
    pub command: Vec<PathBuf>,
    pub environment: EngineEnvironment,
    pub executable: PathBuf,
    pub cwd: PathBuf,
    pub input: PathBuf,
    pub output: PathBuf,
    pub omp_threads: usize,
    pub started_at: String,
    pub finished_at: String,
    pub elapsed_seconds: f64,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub error: Option<super::ModelError>,
}

fn run_engine_inner(plan: &EnginePlan) -> Result<ProcessOutcome, String> {
    if plan.omp_threads == 0
        || !plan.executable.is_absolute()
        || !plan.cwd.is_absolute()
        || !plan.input.is_absolute()
        || !plan.output.is_absolute()
        || !plan.logs.is_absolute()
    {
        return Err("invalid engine plan".into());
    }
    fs::create_dir_all(&plan.logs).map_err(|error| error.to_string())?;
    fs::create_dir_all(&plan.output).map_err(|error| error.to_string())?;
    let stdout = File::create(plan.logs.join("stdout.log")).map_err(|error| error.to_string())?;
    let stderr = File::create(plan.logs.join("stderr.log")).map_err(|error| error.to_string())?;
    let zone = plan
        .time_zone
        .parse::<chrono_tz::Tz>()
        .map_err(|error| error.to_string())?;
    let local_now = || {
        chrono::Utc::now()
            .with_timezone(&zone)
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string()
    };
    let start = Instant::now();
    let started_at = local_now();
    let status = Command::new(&plan.executable)
        .current_dir(&plan.cwd)
        .env_clear()
        .env("MGB_INPUT_DIR", &plan.input)
        .env("MGB_OUTPUT_DIR", &plan.output)
        .env("OMP_NUM_THREADS", plan.omp_threads.to_string())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .status();
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        status.as_ref().ok().and_then(|value| value.signal())
    };
    #[cfg(not(unix))]
    let signal = None;
    let outcome = ProcessOutcome {
        command: vec![plan.executable.clone()],
        environment: EngineEnvironment {
            input: plan.input.clone(),
            output: plan.output.clone(),
            omp_threads: plan.omp_threads.to_string(),
        },
        executable: plan.executable.clone(),
        cwd: plan.cwd.clone(),
        input: plan.input.clone(),
        output: plan.output.clone(),
        omp_threads: plan.omp_threads,
        started_at,
        finished_at: local_now(),
        elapsed_seconds: start.elapsed().as_secs_f64(),
        exit_code: status
            .as_ref()
            .ok()
            .and_then(std::process::ExitStatus::code),
        signal,
        error: match status {
            Ok(status) if status.success() => None,
            Ok(status) => Some(super::ModelError::NonzeroExit(format!(
                "engine exited {status}"
            ))),
            Err(error) => Some(super::ModelError::Spawn(format!(
                "engine spawn failed: {error}"
            ))),
        },
    };
    write_outcome(&plan.logs.join("execution.json"), &outcome)?;
    Ok(outcome)
}

fn write_outcome(path: &Path, outcome: &ProcessOutcome) -> Result<(), String> {
    let file = File::create(path).map_err(|error| error.to_string())?;
    serde_json::to_writer_pretty(&file, outcome).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

pub(crate) fn run_engine(plan: &EnginePlan) -> Result<ProcessOutcome, super::ModelError> {
    run_engine_inner(plan).map_err(super::ModelError::Io)
}
