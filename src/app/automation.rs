use super::{
    Draft, ExecutionReport, OperationControl, OperationError, OperationProgress, absolute,
    operation::{execute_locked, lock_workspace, run_draft_locked},
};
use crate::store::{automation_documents, write_automation_document};

mod email;
use self::email::{MailDelivery, deliver, valid_mailbox};
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationConfig {
    schema_version: u32,
    #[serde(default)]
    pub jobs: Vec<AutomationJob>,
    pub smtp: Option<SmtpConfiguration>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationJob {
    pub id: String,
    pub time_zone: Tz,
    pub hour: u32,
    #[serde(default)]
    pub minute: u32,
    pub steps: Vec<AutomationStep>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationStep {
    pub kind: AutomationStepKind,
    pub draft: Option<PathBuf>,
    pub prepared: Option<PathBuf>,
    #[serde(default)]
    pub rolling: bool,
    pub report: Option<AutomationReportConfiguration>,
    #[serde(default)]
    pub export: bool,
    #[serde(default)]
    pub email: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationReportConfiguration {
    pub profile: PathBuf,
    pub observations: PathBuf,
    pub geometry: PathBuf,
    pub map_geometry: Option<PathBuf>,
    pub scenario_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationStepKind {
    Prepare,
    Execute,
    RunAll,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmtpConfiguration {
    pub host: String,
    pub port: Option<u16>,
    pub tls: SmtpTls,
    pub username: Option<String>,
    pub password_env: Option<String>,
    pub from: String,
    pub to: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmtpTls {
    Implicit,
    Starttls,
}

impl SmtpConfiguration {
    fn validate(&self) -> Result<(), String> {
        let domain = self.host.strip_suffix('.').unwrap_or(&self.host);
        let valid_hostname = domain.len() <= 253
            && domain.split('.').all(|label| {
                label.len() <= 63
                    && label
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_alphanumeric)
                    && label
                        .as_bytes()
                        .last()
                        .is_some_and(u8::is_ascii_alphanumeric)
                    && label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            });
        if (!valid_hostname && self.host.parse::<std::net::IpAddr>().is_err())
            || self.port == Some(0)
            || !valid_mailbox(&self.from)
            || self.to.is_empty()
            || self.to.iter().any(|address| !valid_mailbox(address))
        {
            return Err("invalid SMTP relay, sender, or recipients".into());
        }
        match (&self.username, &self.password_env) {
            (None, None) => Ok(()),
            (Some(username), Some(variable))
                if !username.trim().is_empty()
                    && !variable.is_empty()
                    && variable.bytes().enumerate().all(|(index, ch)| {
                        ch == b'_' || ch.is_ascii_alphabetic() || (index > 0 && ch.is_ascii_digit())
                    }) =>
            {
                Ok(())
            }
            _ => Err("SMTP authentication requires username and password_env".into()),
        }
    }
}

fn valid_id(value: &str) -> bool {
    let id = value.as_bytes();
    !id.is_empty()
        && id.len() <= 64
        && id[0].is_ascii_alphanumeric()
        && id
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn workspace_root(workspace: &Path) -> Result<PathBuf, String> {
    let path = workspace
        .canonicalize()
        .map_err(|e| format!("{}: {e}", workspace.display()))?;
    if !path.is_dir() {
        return Err("automation workspace must be a directory".into());
    }
    Ok(path)
}

fn resolve_source(workspace: &Path, path: &Path, directory: bool) -> Result<PathBuf, String> {
    if path.to_string_lossy().trim().is_empty() {
        return Err("automation source path must be nonempty".into());
    }
    let path = absolute(workspace, path);
    let path = path
        .canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if (directory && !path.is_dir()) || (!directory && !path.is_file()) {
        return Err(format!(
            "wrong kind of automation source at {}",
            path.display()
        ));
    }
    Ok(path)
}

impl AutomationConfig {
    fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || (self.jobs.is_empty() && self.smtp.is_none()) {
            return Err("automation schema 1 requires jobs or SMTP configuration".into());
        }
        if let Some(smtp) = &self.smtp {
            smtp.validate()?;
        }
        let mut ids = HashSet::new();
        for job in &self.jobs {
            if !valid_id(&job.id)
                || !ids.insert(job.id.to_ascii_lowercase())
                || job.hour > 23
                || job.minute > 59
                || job.steps.is_empty()
            {
                return Err(format!("invalid or duplicate automation job {}", job.id));
            }
            for (index, step) in job.steps.iter().enumerate() {
                let post = step.report.is_some() || step.export || step.email;
                let valid = match step.kind {
                    AutomationStepKind::Prepare => {
                        step.draft.is_some() && step.prepared.is_none() && !post
                    }
                    AutomationStepKind::RunAll => step.draft.is_some() && step.prepared.is_none(),
                    AutomationStepKind::Execute => {
                        step.draft.is_none() && step.prepared.is_some() && !step.rolling
                    }
                };
                if !valid || (step.email && self.smtp.is_none()) {
                    return Err(format!("invalid job {} step {}", job.id, index + 1));
                }
                if let Some(report) = &step.report
                    && !valid_id(&report.scenario_id)
                {
                    return Err(format!(
                        "invalid report scenario for job {} step {}",
                        job.id,
                        index + 1
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn read_automation_config(workspace: &Path, path: &Path) -> Result<AutomationConfig, String> {
    let workspace = workspace_root(workspace)?;
    let path = resolve_source(&workspace, path, false)?;
    let contents = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    decode_automation_config(&workspace, &path, &contents)
}

pub fn save_automation_config(workspace: &Path, path: &Path, contents: &str) -> Result<(), String> {
    let workspace = workspace_root(workspace)?;
    let path = absolute(&workspace, path);
    decode_automation_config(&workspace, &path, contents)?;
    crate::store::write_atomic(&path, contents.as_bytes())
}

fn decode_automation_config(
    workspace: &Path,
    path: &Path,
    contents: &str,
) -> Result<AutomationConfig, String> {
    let mut config: AutomationConfig = toml::from_str(contents).map_err(|e: toml::de::Error| {
        // Avoid echoing config text, including any password pasted into an invalid field.
        let line = e.span().map_or(1, |span| {
            contents.as_bytes()[..span.start.min(contents.len())]
                .iter()
                .filter(|byte| **byte == b'\n')
                .count()
                + 1
        });
        format!(
            "invalid automation TOML in {} at line {line}",
            path.display()
        )
    })?;
    config.validate()?;
    for job in &mut config.jobs {
        for step in &mut job.steps {
            if let Some(path) = &mut step.draft {
                *path = resolve_source(workspace, path, false)?;
            }
            if let Some(path) = &mut step.prepared {
                *path = resolve_source(workspace, path, true)?;
                resolve_source(path, Path::new("manifest.json"), false)?;
            }
            if let Some(report) = &mut step.report {
                report.profile = resolve_source(workspace, &report.profile, false)?;
                report.observations = resolve_source(workspace, &report.observations, false)?;
                report.geometry = resolve_source(workspace, &report.geometry, false)?;
                if let Some(path) = &mut report.map_geometry {
                    *path = resolve_source(workspace, path, false)?;
                }
            }
        }
    }
    Ok(config)
}

#[derive(Clone, Debug, Default)]
pub struct AutomationControl {
    stopped: Arc<AtomicBool>,
    active: Arc<Mutex<Option<OperationControl>>>,
}

impl AutomationControl {
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        if let Some(operation) = self
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            operation.cancel();
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }

    pub fn progress(&self) -> Option<OperationProgress> {
        self.active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(OperationControl::progress)
    }

    fn begin(&self) -> OperationControl {
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        let operation = OperationControl::default();
        if self.is_stopped() {
            operation.cancel();
        }
        *active = Some(operation.clone());
        operation
    }

    fn end(&self) {
        *self.active.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationState {
    Running,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
    Busy,
    Missed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationStage {
    Prepare,
    Execute,
    RunAll,
    Report,
    Export,
    Email,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationStageStatus {
    pub step: usize,
    pub stage: AutomationStage,
    pub state: AutomationState,
    pub artifacts: Vec<PathBuf>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationJobStatus {
    pub schema_version: u32,
    pub job_id: String,
    pub slot: String,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub state: AutomationState,
    pub stages: Vec<AutomationStageStatus>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutomationPostAction {
    Report {
        profile: PathBuf,
        observations: PathBuf,
        geometry: PathBuf,
        map_geometry: Option<PathBuf>,
        scenario_id: String,
    },
    Export,
}

type PostAction<'a> =
    dyn FnMut(AutomationPostAction, &Path, &Path) -> Result<Vec<PathBuf>, String> + 'a;

fn automation_root(workspace: &Path) -> PathBuf {
    workspace.join("data/automation")
}

fn lease(workspace: &Path) -> Result<File, String> {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(workspace.join(".automation.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock().map_err(|e| match e {
        std::fs::TryLockError::WouldBlock => {
            "an automation runner already owns this workspace".into()
        }
        std::fs::TryLockError::Error(e) => e.to_string(),
    })?;
    Ok(lock)
}

pub fn automation_status(workspace: &Path) -> Result<Vec<AutomationJobStatus>, String> {
    let directory = automation_root(&workspace_root(workspace)?).join("status");
    let mut jobs: Vec<AutomationJobStatus> = automation_documents(&directory)?
        .into_iter()
        .map(|(_, status)| status)
        .collect();
    if jobs.iter().any(|job| job.schema_version != 1) {
        return Err("unsupported automation status schema".into());
    }
    jobs.sort_by_key(|job| std::cmp::Reverse(job.started_at));
    Ok(jobs)
}

fn recover(workspace: &Path, now: DateTime<Utc>) -> Result<(), String> {
    let directory = automation_root(workspace).join("status");
    for (path, mut job) in automation_documents::<AutomationJobStatus>(&directory)? {
        if job.schema_version != 1 {
            return Err("unsupported automation status schema".into());
        }
        if job.state == AutomationState::Running {
            job.state = AutomationState::Interrupted;
            job.error = Some("automation runner stopped before recording the outcome".into());
            job.finished_at = Some(now);
            if let Some(stage) = job
                .stages
                .last_mut()
                .filter(|stage| stage.state == AutomationState::Running)
            {
                stage.state = AutomationState::Interrupted;
                stage.error = job.error.clone();
            }
            write_automation_document(&path, &job)?;
        }
    }
    Ok(())
}

fn scheduled(job: &AutomationJob, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let date = now.with_timezone(&job.time_zone).date_naive();
    scheduled_on(job, date)
}

fn scheduled_on(job: &AutomationJob, date: NaiveDate) -> Option<DateTime<Utc>> {
    let time = date.and_hms_opt(job.hour, job.minute, 0)?;
    let local = job.time_zone.from_local_datetime(&time).single()?;
    Some(local.with_timezone(&Utc))
}

fn slot_key(job: &AutomationJob, when: DateTime<Utc>) -> String {
    when.with_timezone(&job.time_zone)
        .format("%Y%m%d-%H%M")
        .to_string()
}

fn slot_path(workspace: &Path, job: &AutomationJob, slot: &str) -> PathBuf {
    automation_root(workspace)
        .join("status")
        .join(format!("{}.{slot}.json", job.id))
}

fn draft_for_slot(
    workspace: &Path,
    job: &AutomationJob,
    step: &AutomationStep,
    local: Option<NaiveDateTime>,
) -> Result<Draft, String> {
    let path = step.draft.as_deref().ok_or("draft step requires a draft")?;
    let mut draft = Draft::open(workspace.to_path_buf(), workspace, path)?;
    if step.rolling {
        let local = local.ok_or("rolling draft needs a valid scheduled local time")?;
        let operation = &mut draft.document.operation;
        let zone = operation
            .time_zone
            .as_deref()
            .and_then(|time_zone| time_zone.parse::<Tz>().ok());
        if zone != Some(job.time_zone)
            || operation.observed_horizon_days.is_none()
            || operation.forecast_horizon_days.is_none()
        {
            return Err("rolling draft needs matching timezone and day-based horizons".into());
        }
        operation.reference_time = Some(local.format("%Y-%m-%dT%H:%M:%S").to_string());
        draft.refresh_inputs()?;
    }
    Ok(draft)
}

fn execute_step(
    workspace: &Path,
    job: &AutomationJob,
    step: &AutomationStep,
    local: Option<NaiveDateTime>,
    step_dir: &Path,
    control: &OperationControl,
    lock: &File,
) -> Result<PathBuf, OperationError> {
    let draft = || {
        draft_for_slot(workspace, job, step, local).map_err(|message| {
            OperationError::InvalidRequest {
                field: "draft".into(),
                message,
            }
        })
    };
    match step.kind {
        AutomationStepKind::Prepare => draft()?
            .prepare_controlled(workspace.into(), step_dir.join("prepared"), control)
            .map(|report| report.prepared),
        AutomationStepKind::RunAll => {
            run_draft_locked(&draft()?, control, workspace, lock).map(execution_root)
        }
        AutomationStepKind::Execute => execute_locked(
            workspace,
            step.prepared
                .as_deref()
                .ok_or_else(|| OperationError::InvalidRequest {
                    field: "prepared".into(),
                    message: "execute step requires a prepared directory".into(),
                })?,
            control,
            lock,
            false,
        )
        .map(execution_root),
    }
}

fn execution_root(report: ExecutionReport) -> PathBuf {
    report.run_root
}

fn begin_stage(
    status: &mut AutomationJobStatus,
    path: &Path,
    step: usize,
    stage: AutomationStage,
) -> Result<(), String> {
    status.stages.push(AutomationStageStatus {
        step,
        stage,
        state: AutomationState::Running,
        artifacts: Vec::new(),
        error: None,
    });
    write_automation_document(path, status)
}

fn finish_stage(
    status: &mut AutomationJobStatus,
    path: &Path,
    state: AutomationState,
    artifacts: Vec<PathBuf>,
    error: Option<String>,
) -> Result<(), String> {
    let stage = status
        .stages
        .last_mut()
        .ok_or("automation stage was not started")?;
    stage.state = state;
    stage.artifacts = artifacts;
    stage.error = error.clone();
    if state != AutomationState::Completed {
        status.state = state;
        status.error = error;
    }
    write_automation_document(path, status)
}

#[derive(Serialize)]
struct DeliveryRecord {
    schema_version: u32,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    state: AutomationState,
    error: Option<String>,
}

pub fn send_automation_email(
    workspace: &Path,
    smtp: &SmtpConfiguration,
    subject: &str,
    body: &str,
    attachments: &[PathBuf],
) -> Result<(), String> {
    smtp.validate()?;
    let workspace = workspace_root(workspace)?;
    let attachments: Vec<_> = attachments
        .iter()
        .map(|path| resolve_source(&workspace, path, false))
        .collect::<Result<_, _>>()?;
    let started_at = Utc::now();
    let filename = format!(
        "{}-{}.json",
        started_at.format("%Y%m%dT%H%M%S%.9f"),
        std::process::id()
    );
    let path = automation_root(&workspace).join("delivery").join(filename);
    let mut status = DeliveryRecord {
        schema_version: 1,
        started_at,
        finished_at: None,
        state: AutomationState::Running,
        error: None,
    };
    write_automation_document(&path, &status)?;
    let result = deliver(
        &MailDelivery {
            host: &smtp.host,
            port: smtp.port,
            starttls: smtp.tls == SmtpTls::Starttls,
            username: smtp.username.as_deref(),
            password_env: smtp.password_env.as_deref(),
        },
        &smtp.from,
        &smtp.to,
        subject,
        body,
        &attachments,
    );
    status.finished_at = Some(Utc::now());
    status.state = if result.is_ok() {
        AutomationState::Completed
    } else {
        AutomationState::Failed
    };
    status.error = result.as_ref().err().cloned();
    write_automation_document(&path, &status).map_err(|e| {
        format!(
            "could not record SMTP delivery outcome at {}: {e}",
            path.display()
        )
    })?;
    result
}

fn post_output(
    post: &mut PostAction<'_>,
    action: AutomationPostAction,
    run: &Path,
    path: &Path,
) -> Result<Vec<PathBuf>, String> {
    fs::create_dir(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let directory = path
        .canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let outputs = post(action, run, &directory)?;
    if outputs.is_empty() {
        return Err("post-stage produced no artifacts".into());
    }
    outputs
        .into_iter()
        .map(|output| {
            let path = absolute(&directory, &output)
                .canonicalize()
                .map_err(|e| format!("{}: {e}", output.display()))?;
            if !path.starts_with(&directory)
                || (!path.is_file() && !path.is_dir())
                || path == directory
            {
                return Err(format!(
                    "post-stage artifact must lie within {}",
                    directory.display()
                ));
            }
            Ok(path)
        })
        .collect()
}

struct Runner<'a> {
    workspace: &'a Path,
    smtp: Option<&'a SmtpConfiguration>,
    control: &'a AutomationControl,
    post: &'a mut PostAction<'a>,
    clock: &'a dyn Fn() -> DateTime<Utc>,
    previous_tick: Option<DateTime<Utc>>,
}

impl Runner<'_> {
    fn missed(
        &self,
        job: &AutomationJob,
        slot: String,
        when: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<AutomationJobStatus, String> {
        let path = slot_path(self.workspace, job, &slot);
        let status = AutomationJobStatus {
            schema_version: 1,
            job_id: job.id.clone(),
            slot,
            scheduled_at: Some(when),
            started_at: now,
            finished_at: Some(now),
            state: AutomationState::Missed,
            stages: Vec::new(),
            error: Some("scheduled minute elapsed while the runner was active".into()),
        };
        write_automation_document(&path, &status)?;
        Ok(status)
    }

    fn claim(
        &mut self,
        job: &AutomationJob,
        slot: String,
        scheduled_at: Option<DateTime<Utc>>,
        reference: Option<DateTime<Utc>>,
    ) -> Result<AutomationJobStatus, String> {
        let path = slot_path(self.workspace, job, &slot);
        if path.try_exists().map_err(|e| e.to_string())? {
            return Err(format!(
                "automation slot already claimed at {}",
                path.display()
            ));
        }
        let mut status = AutomationJobStatus {
            schema_version: 1,
            job_id: job.id.clone(),
            slot,
            scheduled_at,
            started_at: (self.clock)(),
            finished_at: None,
            state: AutomationState::Running,
            stages: Vec::new(),
            error: None,
        };
        write_automation_document(&path, &status)?;
        let lock = lock_workspace(self.workspace);
        match lock {
            Ok(lock) => {
                let local = reference.and_then(|time| {
                    let local = time
                        .with_timezone(&job.time_zone)
                        .naive_local()
                        .with_minute(0)?;
                    job.time_zone.from_local_datetime(&local).single()?;
                    Some(local)
                });
                self.run(job, &path, &lock, &mut status, local)?;
            }
            Err(OperationError::Busy { .. }) => {
                status.state = AutomationState::Busy;
                status.error = Some("another operation owns this workspace".into());
            }
            Err(error) => {
                status.state = AutomationState::Failed;
                status.error = Some(error.to_string());
            }
        }
        if status.state == AutomationState::Running {
            status.state = if self.control.is_stopped() {
                AutomationState::Cancelled
            } else {
                AutomationState::Completed
            };
        }
        status.finished_at = Some((self.clock)());
        write_automation_document(&path, &status)?;
        Ok(status)
    }

    fn run(
        &mut self,
        job: &AutomationJob,
        path: &Path,
        lock: &File,
        status: &mut AutomationJobStatus,
        local: Option<NaiveDateTime>,
    ) -> Result<(), String> {
        for (index, step) in job.steps.iter().enumerate() {
            if self.control.is_stopped() {
                break;
            }
            let number = index + 1;
            let stage = match step.kind {
                AutomationStepKind::Prepare => AutomationStage::Prepare,
                AutomationStepKind::Execute => AutomationStage::Execute,
                AutomationStepKind::RunAll => AutomationStage::RunAll,
            };
            begin_stage(status, path, number, stage)?;
            let step_dir = automation_root(self.workspace)
                .join("artifacts")
                .join(&job.id)
                .join(&status.slot)
                .join(number.to_string());
            if let Err(error) = fs::create_dir_all(&step_dir) {
                return finish_stage(
                    status,
                    path,
                    AutomationState::Failed,
                    vec![],
                    Some(error.to_string()),
                );
            }
            let control = self.control.begin();
            let result = execute_step(self.workspace, job, step, local, &step_dir, &control, lock);
            self.control.end();
            let artifact = match result {
                Ok(artifact) => {
                    finish_stage(
                        status,
                        path,
                        AutomationState::Completed,
                        vec![artifact.clone()],
                        None,
                    )?;
                    artifact
                }
                Err(OperationError::BatchFailed { report }) => {
                    let state = if self.control.is_stopped() {
                        AutomationState::Cancelled
                    } else {
                        AutomationState::Failed
                    };
                    let error = report
                        .error
                        .clone()
                        .or_else(|| Some("one or more scenarios failed".into()));
                    return finish_stage(status, path, state, vec![report.run_root], error);
                }
                Err(error) => {
                    let state = if matches!(error, OperationError::Cancelled { .. })
                        || self.control.is_stopped()
                    {
                        AutomationState::Cancelled
                    } else {
                        AutomationState::Failed
                    };
                    return finish_stage(status, path, state, vec![], Some(error.to_string()));
                }
            };

            let mut attachments = Vec::new();
            for (action, name, stage) in [
                (
                    step.report
                        .as_ref()
                        .map(|report| AutomationPostAction::Report {
                            profile: report.profile.clone(),
                            observations: report.observations.clone(),
                            geometry: report.geometry.clone(),
                            map_geometry: report.map_geometry.clone(),
                            scenario_id: report.scenario_id.clone(),
                        }),
                    "report",
                    AutomationStage::Report,
                ),
                (
                    step.export.then_some(AutomationPostAction::Export),
                    "export",
                    AutomationStage::Export,
                ),
            ] {
                let Some(action) = action.filter(|_| !self.control.is_stopped()) else {
                    continue;
                };
                begin_stage(status, path, number, stage)?;
                match post_output(self.post, action, &artifact, &step_dir.join(name)) {
                    Ok(outputs) => {
                        attachments.extend(outputs.iter().filter(|path| path.is_file()).cloned());
                        finish_stage(status, path, AutomationState::Completed, outputs, None)?;
                    }
                    Err(error) => {
                        return finish_stage(
                            status,
                            path,
                            AutomationState::Failed,
                            vec![],
                            Some(error),
                        );
                    }
                }
            }

            if step.email && !self.control.is_stopped() {
                begin_stage(status, path, number, AutomationStage::Email)?;
                let smtp = self.smtp.ok_or("email stage requires SMTP configuration")?;
                let subject = format!("MGB operation {} completed", job.id);
                let body = format!(
                    "Automation job {} completed step {number}.\nRun artifact: {}\n",
                    job.id,
                    artifact.display()
                );
                match send_automation_email(self.workspace, smtp, &subject, &body, &attachments) {
                    Ok(()) => finish_stage(status, path, AutomationState::Completed, vec![], None)?,
                    Err(error) => {
                        return finish_stage(
                            status,
                            path,
                            AutomationState::Failed,
                            vec![],
                            Some(error),
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn due(&mut self, jobs: &[AutomationJob]) -> Result<Vec<AutomationJobStatus>, String> {
        let tick = (self.clock)();
        let previous = self.previous_tick.replace(tick);
        let mut outcomes = Vec::new();
        for job in jobs {
            if self.control.is_stopped() {
                break;
            }
            let date = tick.with_timezone(&job.time_zone).date_naive();
            for date in date.pred_opt().into_iter().chain(std::iter::once(date)) {
                let Some(when) = scheduled_on(job, date).filter(|when| *when <= tick) else {
                    continue;
                };
                let due_at_tick = tick - when < Duration::minutes(1);
                let crossed = previous.is_some_and(|earlier| earlier <= when);
                if !due_at_tick && !crossed {
                    continue;
                }
                let now = (self.clock)();
                if now < when {
                    continue;
                }
                let slot = slot_key(job, when);
                if slot_path(self.workspace, job, &slot)
                    .try_exists()
                    .map_err(|e| e.to_string())?
                {
                    continue;
                }
                if now - when >= Duration::minutes(1) {
                    outcomes.push(self.missed(job, slot, when, now)?);
                } else {
                    outcomes.push(self.claim(job, slot, Some(when), Some(when))?);
                }
            }
        }
        Ok(outcomes)
    }
}

pub fn run_automation(
    workspace: &Path,
    config: &AutomationConfig,
    control: &AutomationControl,
    mut post: impl FnMut(AutomationPostAction, &Path, &Path) -> Result<Vec<PathBuf>, String>,
) -> Result<(), String> {
    let workspace = workspace_root(workspace)?;
    config.validate()?;
    if config.jobs.is_empty() {
        return Err("automation runner needs configured jobs".into());
    }
    let _lease = lease(&workspace)?;
    recover(&workspace, Utc::now())?;
    let mut runner = Runner {
        workspace: &workspace,
        smtp: config.smtp.as_ref(),
        control,
        post: &mut post,
        clock: &Utc::now,
        previous_tick: None,
    };
    while !control.is_stopped() {
        runner.due(&config.jobs)?;
        if !control.is_stopped() {
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }
    Ok(())
}

pub fn run_automation_due(
    workspace: &Path,
    config: &AutomationConfig,
    control: &AutomationControl,
    mut post: impl FnMut(AutomationPostAction, &Path, &Path) -> Result<Vec<PathBuf>, String>,
) -> Result<Vec<AutomationJobStatus>, String> {
    let workspace = workspace_root(workspace)?;
    config.validate()?;
    let _lease = lease(&workspace)?;
    recover(&workspace, Utc::now())?;
    Runner {
        workspace: &workspace,
        smtp: config.smtp.as_ref(),
        control,
        post: &mut post,
        clock: &Utc::now,
        previous_tick: None,
    }
    .due(&config.jobs)
}

pub fn run_automation_now(
    workspace: &Path,
    config: &AutomationConfig,
    job_id: &str,
    control: &AutomationControl,
    mut post: impl FnMut(AutomationPostAction, &Path, &Path) -> Result<Vec<PathBuf>, String>,
) -> Result<AutomationJobStatus, String> {
    let workspace = workspace_root(workspace)?;
    config.validate()?;
    let job = config
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .ok_or("unknown automation job")?;
    let now = Utc::now();
    let at = scheduled(job, now).filter(|when| *when <= now);
    if job.steps.iter().any(|step| step.rolling) && at.is_none() {
        return Err("rolling job needs today's scheduled local time to occur first".into());
    }
    let _lease = lease(&workspace)?;
    recover(&workspace, now)?;
    let slot = format!(
        "manual-{}-{}",
        now.format("%Y%m%dT%H%M%S%.9f"),
        std::process::id()
    );
    Runner {
        workspace: &workspace,
        smtp: config.smtp.as_ref(),
        control,
        post: &mut post,
        clock: &Utc::now,
        previous_tick: None,
    }
    .claim(job, slot, None, at)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(workspace: &Path) -> AutomationConfig {
        fs::write(workspace.join("draft.json"), "{}").unwrap();
        fs::write(
            workspace.join("automation.toml"),
            r#"
schema_version = 1
[[jobs]]
id = "rs"
time_zone = "America/Sao_Paulo"
hour = 7

[[jobs.steps]]
kind = "prepare"
draft = "draft.json"
"#,
        )
        .unwrap();
        read_automation_config(workspace, Path::new("automation.toml")).unwrap()
    }

    #[test]
    fn due_slots_are_once_only_across_locks_and_restart() {
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path();
        let config = load(root);
        assert_eq!(
            config.jobs[0].steps[0].draft.as_deref(),
            Some(root.join("draft.json").as_path())
        );
        let now: DateTime<Utc> = "2026-07-20T10:00:20Z".parse().unwrap();
        let control = AutomationControl::default();
        let lock = lease(root).unwrap();
        assert!(lease(root).is_err());
        let busy = lock_workspace(root).unwrap();
        let mut post = |_: AutomationPostAction,
                        _: &Path,
                        _: &Path|
         -> Result<Vec<PathBuf>, String> { panic!("unexpected post-stage") };
        let missed = now + Duration::minutes(1);
        let mut runner = Runner {
            workspace: root,
            smtp: None,
            control: &control,
            post: &mut post,
            clock: &|| missed,
            previous_tick: None,
        };
        let outcomes = runner.due(&config.jobs).unwrap();
        assert!(outcomes.is_empty());

        let clock = || now;
        runner.clock = &clock;
        let outcome = runner.due(&config.jobs).unwrap().pop().unwrap();
        assert_eq!(outcome.state, AutomationState::Busy);
        assert_eq!(outcome.slot, "20260720-0700");
        drop(busy);
        assert!(runner.due(&config.jobs).unwrap().is_empty());
        drop(lock);

        let now = now + Duration::days(1);
        let slot = scheduled(&config.jobs[0], now).unwrap();
        let path = slot_path(root, &config.jobs[0], "20260721-0700");
        let interrupted = AutomationJobStatus {
            schema_version: 1,
            job_id: "rs".into(),
            slot: "20260721-0700".into(),
            scheduled_at: Some(slot),
            started_at: now,
            finished_at: None,
            state: AutomationState::Running,
            stages: vec![AutomationStageStatus {
                step: 1,
                stage: AutomationStage::Prepare,
                state: AutomationState::Running,
                artifacts: Vec::new(),
                error: None,
            }],
            error: None,
        };
        write_automation_document(&path, &interrupted).unwrap();
        let _lease = lease(root).unwrap();
        recover(root, now + Duration::seconds(1)).unwrap();
        let mut resumed = Runner {
            workspace: root,
            smtp: None,
            control: &control,
            post: &mut post,
            clock: &|| now,
            previous_tick: None,
        };
        assert!(resumed.due(&config.jobs).unwrap().is_empty());
        let status = automation_status(root).unwrap();
        assert_eq!(status.len(), 2);
        assert_eq!(status[0].state, AutomationState::Interrupted);
        assert_eq!(status[0].stages[0].state, AutomationState::Interrupted);
    }

    #[test]
    fn schedule_skips_ambiguous_and_nonexistent_local_hours() {
        let workspace = tempfile::tempdir().unwrap();
        let mut job = load(workspace.path()).jobs.remove(0);
        job.time_zone = "America/New_York".parse().unwrap();
        job.hour = 1;
        let fall: DateTime<Utc> = "2026-11-01T05:00:00Z".parse().unwrap();
        assert!(scheduled(&job, fall).is_none());
        job.hour = 2;
        let spring: DateTime<Utc> = "2026-03-08T07:00:00Z".parse().unwrap();
        assert!(scheduled(&job, spring).is_none());
    }

    #[test]
    fn a_long_job_records_same_minute_peers_and_later_missed_slots() {
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path();
        let mut config = load(root);
        let mut peer = config.jobs[0].clone();
        peer.id = "peer".into();
        let mut later = config.jobs[0].clone();
        later.id = "later".into();
        later.minute = 1;
        config.jobs.extend([peer, later]);
        let start: DateTime<Utc> = "2026-07-20T10:00:20Z".parse().unwrap();
        let elapsed = start + Duration::minutes(2);
        let calls = std::cell::Cell::new(0);
        let clock = || {
            calls.set(calls.get() + 1);
            if calls.get() <= 3 { start } else { elapsed }
        };
        let control = AutomationControl::default();
        let _lease = lease(root).unwrap();
        let busy = lock_workspace(root).unwrap();
        let mut post = |_: AutomationPostAction,
                        _: &Path,
                        _: &Path|
         -> Result<Vec<PathBuf>, String> { panic!("unexpected post-stage") };
        let mut runner = Runner {
            workspace: root,
            smtp: None,
            control: &control,
            post: &mut post,
            clock: &clock,
            previous_tick: None,
        };
        let outcomes = runner.due(&config.jobs).unwrap();
        assert_eq!(outcomes.len(), 2);
        assert_eq!(outcomes[0].state, AutomationState::Busy);
        assert_eq!(outcomes[1].job_id, "peer");
        assert_eq!(outcomes[1].state, AutomationState::Missed);
        drop(busy);
        let outcomes = runner.due(&config.jobs).unwrap();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].job_id, "later");
        assert_eq!(outcomes[0].state, AutomationState::Missed);
        assert!(outcomes[0].stages.is_empty());
    }

    #[test]
    fn changing_operation_control_each_step_preserves_stop() {
        let control = AutomationControl::default();
        let first = control.begin();
        control.end();
        let second = control.begin();
        control.stop();
        let first_untouched = !first.is_cancelled();
        assert!(first_untouched);
        assert!(second.is_cancelled());
        assert!(control.begin().is_cancelled());
    }

    #[test]
    fn config_rejects_unknown_fields_and_inline_passwords() {
        let workspace = tempfile::tempdir().unwrap();
        let path = workspace.path().join("automation.toml");
        fs::write(
            &path,
            r#"
schema_version = 1
[smtp]
host="smtp.example.org"
tls="starttls"
from="sender@example.org"
to=["recipient@example.org"]
password="secret-that-must-not-appear"
"#,
        )
        .unwrap();
        let error = read_automation_config(workspace.path(), &path).unwrap_err();
        let redacted = !error.contains("secret-that-must-not-appear");
        assert!(redacted);
    }

    #[test]
    fn report_paths_resolve_at_workspace_and_minute_sets_the_slot() {
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path();
        let prepared = root.join("prepared");
        fs::create_dir(&prepared).unwrap();
        for name in [
            "profile.toml",
            "history.sqlite",
            "geometry.gpkg",
            "map.gpkg",
            "prepared/manifest.json",
        ] {
            fs::write(root.join(name), b"fixture").unwrap();
        }
        let config_path = root.join("automation.toml");
        fs::write(
            &config_path,
            r#"
schema_version = 1
[[jobs]]
id = "rs"
time_zone = "America/Sao_Paulo"
hour = 7
minute = 30
[[jobs.steps]]
kind = "execute"
prepared = "prepared"
[jobs.steps.report]
profile = "profile.toml"
observations = "history.sqlite"
geometry = "geometry.gpkg"
map_geometry = "map.gpkg"
scenario_id = "zero"
"#,
        )
        .unwrap();
        let config = read_automation_config(root, &config_path).unwrap();
        let job = &config.jobs[0];
        let report = job.steps[0].report.as_ref().unwrap();
        assert_eq!(report.profile, root.join("profile.toml"));
        assert_eq!(
            report.map_geometry.as_deref(),
            Some(root.join("map.gpkg").as_path())
        );
        assert_eq!(report.scenario_id, "zero");
        let now: DateTime<Utc> = "2026-07-20T10:30:45Z".parse().unwrap();
        let slot = scheduled(job, now).unwrap();
        assert_eq!(slot_key(job, slot), "20260720-0730");
    }
}
