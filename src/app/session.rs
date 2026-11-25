use super::{
    InspectionReport, InspectionRequest, OperationalRun, RunParameters, absolute, inspect,
};
use crate::store::{
    OperationManifest, is_managed_run_path, read_result_series, read_retained_manifest,
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone)]
pub struct Session {
    settings: Arc<InspectionRequest>,
    id: u64,
    state: Arc<Mutex<State>>,
}

struct State {
    selection: RunSelection,
    run_revision: u64,
    setup: Option<OperationalRun>,
    setup_revision: u64,
    job: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum RunSelection {
    #[default]
    Active,
    Saved(PathBuf),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunSetupRevision {
    session: u64,
    revision: u64,
}

#[derive(Clone, Debug)]
pub struct RunSetupSnapshot {
    pub revision: RunSetupRevision,
    pub operation: OperationalRun,
}

pub struct WorkspaceJob(Session);

impl Drop for WorkspaceJob {
    fn drop(&mut self) {
        self.0.state().job = false;
    }
}

impl Session {
    pub fn new(mut settings: InspectionRequest) -> Self {
        settings.workspace = absolute(&settings.invocation_dir, &settings.workspace);
        settings.config = Some(settings.config.as_deref().map_or_else(
            || super::workspace_config_path(&settings.workspace),
            |path| absolute(&settings.invocation_dir, path),
        ));
        Self {
            settings: Arc::new(settings),
            id: next_id(),
            state: Arc::new(Mutex::new(State {
                selection: RunSelection::Active,
                run_revision: 0,
                setup: None,
                setup_revision: 0,
                job: false,
            })),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn workspace(&self) -> &Path {
        &self.settings.workspace
    }

    pub fn config(&self) -> &Path {
        self.settings.config.as_deref().unwrap()
    }

    pub fn inspect(&self) -> InspectionReport {
        inspect((*self.settings).clone())
    }

    pub fn run_setup(&self) -> Option<RunSetupSnapshot> {
        let state = self.state();
        Some(RunSetupSnapshot {
            revision: RunSetupRevision {
                session: self.id,
                revision: state.setup_revision,
            },
            operation: state.setup.clone()?,
        })
    }

    pub fn open_run_setup(&self, operation: OperationalRun) {
        let mut state = self.state();
        state.setup = Some(operation);
        state.setup_revision += 1;
    }

    pub fn update_run_setup(
        &self,
        revision: &RunSetupRevision,
        parameters: RunParameters,
    ) -> Result<(), String> {
        let mut state = self.state();
        if state.job {
            return Err("An operation is running".into());
        }
        if revision.session != self.id || revision.revision != state.setup_revision {
            return Err("The run form changed. Review the current selections.".into());
        }
        let operation = state.setup.as_mut().ok_or("Open the run form first")?;
        if operation.parameters() != &parameters {
            operation.set_parameters(parameters);
            state.setup_revision += 1;
        }
        Ok(())
    }

    pub fn start_workspace_job(&self) -> Result<WorkspaceJob, String> {
        let mut state = self.state();
        if state.job {
            return Err("A job is already active in this workspace".into());
        }
        state.job = true;
        Ok(WorkspaceJob(self.clone()))
    }

    pub fn run_selection(&self) -> RunSelection {
        self.state().selection.clone()
    }

    pub fn select_run(&self, selection: RunSelection) -> Result<(), String> {
        let selection = match selection {
            RunSelection::Saved(path) => RunSelection::Saved(self.saved_path(&path)?),
            RunSelection::Active => RunSelection::Active,
        };
        let mut state = self.state();
        if state.selection != selection {
            state.selection = selection;
            state.run_revision += 1;
        }
        Ok(())
    }

    fn saved_path(&self, path: &Path) -> Result<PathBuf, String> {
        let path = absolute(&self.settings.invocation_dir, path);
        let resolved = path
            .canonicalize()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if is_managed_run_path(self.workspace(), &resolved) {
            return Err(
                "Select an independently saved run. Managed run generations follow the active run."
                    .into(),
            );
        }
        if !resolved.is_dir() {
            return Err("Select a saved run directory or bundle".into());
        }
        Ok(resolved)
    }

    pub fn run_snapshot(&self) -> RunSnapshot {
        let state = self.state();
        RunSnapshot {
            session: self.clone(),
            selection: state.selection.clone(),
            revision: state.run_revision,
        }
    }
}

#[derive(Clone)]
pub struct RunSnapshot {
    session: Session,
    selection: RunSelection,
    revision: u64,
}

impl PartialEq for RunSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.session.id == other.session.id && self.revision == other.revision
    }
}
impl Eq for RunSnapshot {}

impl RunSnapshot {
    pub fn read<T>(
        &self,
        expected_root: Option<&Path>,
        read: impl FnOnce(&Path, &str) -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        let resolved = |root: &Path| {
            if expected_root.is_some_and(|expected| root != expected) {
                return Err("Active run changed while loading".into());
            }
            let identity = format!("{}:{}:{}", self.session.id, self.revision, root.display());
            read(root, &identity)
        };
        match &self.selection {
            RunSelection::Active => super::with_active_run(self.session.workspace(), resolved),
            RunSelection::Saved(root) => {
                let checked = self.session.saved_path(root)?;
                if checked != *root {
                    return Err("Saved run directory was replaced by an alias".into());
                }
                resolved(root).map(Some)
            }
        }
    }
}

#[derive(Serialize)]
pub struct RunSummary {
    pub root: PathBuf,
    pub manifest: OperationManifest,
    pub day_horizons: Option<(u32, u32)>,
    pub scenarios: Vec<String>,
    pub findings: Vec<String>,
    pub reference: String,
    pub start: String,
    pub end: String,
    pub time_zone: String,
}

pub fn read_run_summary(root: &Path) -> Result<RunSummary, String> {
    let manifest =
        read_retained_manifest(root).map_err(|error| format!("{}: {error}", root.display()))?;
    if manifest
        .scenarios
        .iter()
        .any(|scenario| !super::series::valid_id(&scenario.id))
    {
        return Err("invalid scenario ID in retained manifest".into());
    }
    let mut result = RunSummary {
        root: root.into(),
        day_horizons: super::retained_day_horizons(&manifest),
        manifest,
        scenarios: Vec::new(),
        findings: Vec::new(),
        reference: String::new(),
        start: String::new(),
        end: String::new(),
        time_zone: String::new(),
    };
    for scenario in &result.manifest.scenarios {
        if scenario.state != "completed" {
            continue;
        }
        match read_result_series(&root.join("results").join(&scenario.id).join("basin.nc")) {
            Ok(basin) => {
                if result.scenarios.is_empty() {
                    let bounds = &basin.timeline.bounds;
                    result.reference = basin
                        .provenance
                        .processing
                        .get("reference_time")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .into();
                    result.start = bounds
                        .first()
                        .ok_or("empty result timeline")?
                        .start
                        .format("%Y-%m-%dT%H:%M:%S")
                        .to_string();
                    result.end = bounds
                        .last()
                        .ok_or("empty result timeline")?
                        .end
                        .format("%Y-%m-%dT%H:%M:%S")
                        .to_string();
                    result.time_zone = basin.timeline.time_zone;
                }
                result.scenarios.push(scenario.id.clone());
            }
            Err(error) => result.findings.push(format!("{}: {error}", scenario.id)),
        }
    }
    Ok(result)
}
