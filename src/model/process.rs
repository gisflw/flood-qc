use serde::Serialize;
use std::{
    fs::{self, File},
    io::{Read, Seek},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
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

#[derive(Clone, Copy)]
pub(crate) enum EngineStream {
    Stdout,
    Stderr,
}

pub(crate) type EngineLogObserver<'a> = dyn FnMut(EngineStream, &[u8]) + 'a;

struct LogReaders {
    files: [(Option<File>, EngineStream); 2],
    buffer: [u8; 16 * 1024],
}

impl LogReaders {
    fn open(stdout: &Path, stderr: &Path) -> Self {
        Self {
            files: [
                (File::open(stdout).ok(), EngineStream::Stdout),
                (File::open(stderr).ok(), EngineStream::Stderr),
            ],
            buffer: [0; 16 * 1024],
        }
    }

    fn read(&mut self, observer: &mut EngineLogObserver<'_>, finished: bool) {
        for (reader, stream) in &mut self.files {
            let Some(file) = reader else { continue };
            let limit = if finished {
                // Capture EOF once, even if an engine descendant still has the file open.
                let Ok(metadata) = file.metadata() else {
                    continue;
                };
                let Ok(position) = file.stream_position() else {
                    continue;
                };
                metadata.len().saturating_sub(position)
            } else {
                self.buffer.len() as u64
            };
            let mut input = file.take(limit);
            loop {
                match input.read(&mut self.buffer) {
                    Ok(0) => break,
                    Ok(size) => observer(*stream, &self.buffer[..size]),
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        *reader = None;
                        break;
                    }
                }
            }
        }
    }
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

fn run_engine_inner(
    plan: &EnginePlan,
    cancelled: &AtomicBool,
    mut observer: Option<&mut EngineLogObserver<'_>>,
) -> Result<ProcessOutcome, String> {
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
    let stdout_path = plan.logs.join("stdout.log");
    let stderr_path = plan.logs.join("stderr.log");
    let stdout = File::create(&stdout_path).map_err(|error| error.to_string())?;
    let stderr = File::create(&stderr_path).map_err(|error| error.to_string())?;
    let mut logs = observer
        .as_ref()
        .map(|_| LogReaders::open(&stdout_path, &stderr_path));
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
    let child = Command::new(&plan.executable)
        .current_dir(&plan.cwd)
        .env_clear()
        .env("MGB_INPUT_DIR", &plan.input)
        .env("MGB_OUTPUT_DIR", &plan.output)
        .env("OMP_NUM_THREADS", plan.omp_threads.to_string())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn();
    let mut was_cancelled = false;
    let status = (|| {
        Ok(match child {
            Err(error) => Err(error),
            Ok(mut child) => loop {
                match child.try_wait() {
                    Ok(Some(status)) => break Ok(status),
                    Ok(None) if cancelled.load(Ordering::Acquire) => {
                        was_cancelled = true;
                        if let Err(error) = child.kill() {
                            match child.try_wait() {
                                Ok(Some(status)) => break Ok(status),
                                _ => {
                                    let waited = child.wait();
                                    return Err(format!(
                                        "failed to stop engine: {error}; wait: {waited:?}"
                                    ));
                                }
                            }
                        }
                        break child.wait();
                    }
                    Ok(None) => {
                        if let (Some(logs), Some(observer)) = (&mut logs, observer.as_deref_mut()) {
                            logs.read(observer, false);
                        }
                        std::thread::sleep(Duration::from_millis(25));
                    }
                    Err(error) => {
                        let killed = child.kill();
                        let waited = child.wait();
                        return Err(format!(
                            "failed to poll engine: {error}; kill: {killed:?}; wait: {waited:?}"
                        ));
                    }
                }
            },
        })
    })();
    let finished_at = local_now();
    let elapsed_seconds = start.elapsed().as_secs_f64();
    if let (Some(logs), Some(observer)) = (logs.as_mut(), observer) {
        logs.read(observer, true);
    }
    let status = status?;
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
        finished_at,
        elapsed_seconds,
        exit_code: status
            .as_ref()
            .ok()
            .and_then(std::process::ExitStatus::code),
        signal,
        error: match status {
            Err(error) if was_cancelled => Some(super::ModelError::Io(format!(
                "failed to wait for cancelled engine: {error}"
            ))),
            Ok(_) if was_cancelled => Some(super::ModelError::Io("engine cancelled".into())),
            Ok(status) if status.success() => None,
            Ok(status) => Some(super::ModelError::NonzeroExit(format!(
                "engine exited {status}"
            ))),
            Err(error) => Some(super::ModelError::Spawn(format!(
                "engine spawn failed: {error}"
            ))),
        },
    };
    write_outcome(&plan.logs.join("execution.toml"), &outcome)?;
    Ok(outcome)
}

fn write_outcome(path: &Path, outcome: &ProcessOutcome) -> Result<(), String> {
    crate::store::write_draft(path, outcome)
}

pub(crate) fn run_engine(
    plan: &EnginePlan,
    cancelled: &AtomicBool,
    observer: Option<&mut EngineLogObserver<'_>>,
) -> Result<ProcessOutcome, super::ModelError> {
    run_engine_inner(plan, cancelled, observer).map_err(super::ModelError::Io)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::Ordering;

    #[test]
    fn cancellation_reaps_sleeping_engine() {
        fs::create_dir_all("runs").unwrap();
        let dir = tempfile::Builder::new()
            .prefix("engine-cancel-")
            .tempdir_in("runs")
            .unwrap();
        let root = dir.path().canonicalize().unwrap();
        let executable = root.join("engine");
        fs::write(
            &executable,
            "#!/bin/sh\necho $$ > \"$MGB_OUTPUT_DIR/pid\"\nexec /bin/sleep 30\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        let plan = EnginePlan {
            executable,
            cwd: root.clone(),
            input: root.clone(),
            output: root.join("output"),
            logs: root.join("logs"),
            omp_threads: 1,
            time_zone: "UTC".into(),
        };
        let cancelled = AtomicBool::new(false);
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| run_engine(&plan, &cancelled, None).unwrap());
            let pid_path = plan.output.join("pid");
            for _ in 0..200 {
                if pid_path.exists() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let pid = fs::read_to_string(pid_path).unwrap();
            cancelled.store(true, Ordering::Release);
            let outcome = worker.join().unwrap();
            assert!(outcome.error.unwrap().to_string().contains("cancelled"));
            #[cfg(target_os = "linux")]
            {
                let reaped = !Path::new("/proc").join(pid.trim()).exists();
                assert!(reaped);
            }
        });
    }
}
