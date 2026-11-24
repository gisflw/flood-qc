use super::{Draft, DraftDocument, InspectionReport, InspectionRequest, absolute, inspect};
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
    draft: Option<DraftSnapshot>,
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
pub struct DraftVersion {
    session: u64,
    draft: u64,
    revision: u64,
}

#[derive(Clone, Debug)]
pub struct DraftSnapshot {
    pub version: DraftVersion,
    pub draft: Draft,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftConflict {
    Stale,
    Busy,
}

impl std::fmt::Display for DraftConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Stale => {
                "The working draft changed. Request a new proposal from the current draft."
            }
            Self::Busy => "A draft job is running. Retry after it finishes.",
        })
    }
}
impl std::error::Error for DraftConflict {}

pub struct DraftJob {
    session: Session,
    pub snapshot: Option<DraftSnapshot>,
}

impl Session {
    pub fn new(mut settings: InspectionRequest) -> Self {
        settings.workspace = absolute(&settings.invocation_dir, &settings.workspace);
        settings.config = Some(settings.config.as_deref().map_or_else(
            || settings.workspace.join("config/workspace.toml"),
            |path| absolute(&settings.invocation_dir, path),
        ));
        Self {
            settings: Arc::new(settings),
            id: next_id(),
            state: Arc::new(Mutex::new(State {
                selection: RunSelection::Active,
                run_revision: 0,
                draft: None,
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

    pub fn open_draft(&self, path: &Path) -> Result<Draft, String> {
        Draft::open(self.workspace().into(), &self.settings.invocation_dir, path)
    }

    pub fn draft(&self) -> Option<DraftSnapshot> {
        self.state().draft.clone()
    }

    pub fn draft_version(&self) -> Option<DraftVersion> {
        self.state()
            .draft
            .as_ref()
            .map(|snapshot| snapshot.version.clone())
    }

    pub fn replace_draft(&self, draft: Draft) -> Result<(), DraftConflict> {
        let mut state = self.state();
        if state.job {
            return Err(DraftConflict::Busy);
        }
        state.draft = Some(self.new_draft(draft)?);
        Ok(())
    }

    fn new_draft(&self, draft: Draft) -> Result<DraftSnapshot, DraftConflict> {
        if draft.workspace != self.workspace() {
            return Err(DraftConflict::Stale);
        }
        Ok(DraftSnapshot {
            version: DraftVersion {
                session: self.id,
                draft: next_id(),
                revision: 0,
            },
            draft,
        })
    }

    pub fn update_draft(&self, snapshot: DraftSnapshot) -> Result<(), DraftConflict> {
        let mut state = self.state();
        state
            .draft
            .as_ref()
            .filter(|draft| draft.version == snapshot.version)
            .ok_or(DraftConflict::Stale)?;
        if state.job {
            return Err(DraftConflict::Busy);
        }
        let current = state.draft.as_mut().unwrap();
        if snapshot.draft.workspace != self.workspace() {
            return Err(DraftConflict::Stale);
        }
        if !current.draft.same_working_state(&snapshot.draft) {
            current.advance();
        }
        current.draft = snapshot.draft;
        Ok(())
    }

    pub fn edit_draft<T>(&self, edit: impl FnOnce(&mut Draft) -> T) -> Result<T, DraftConflict> {
        let mut state = self.state();
        if state.job {
            return Err(DraftConflict::Busy);
        }
        let current = state.draft.as_mut().ok_or(DraftConflict::Stale)?;
        let mut candidate = current.draft.clone();
        let result = edit(&mut candidate);
        if candidate.workspace != self.workspace() {
            return Err(DraftConflict::Stale);
        }
        if !current.draft.same_working_state(&candidate) {
            current.advance();
        }
        current.draft = candidate;
        Ok(result)
    }

    pub fn apply_draft(
        &self,
        version: &DraftVersion,
        mut document: DraftDocument,
    ) -> Result<(), DraftConflict> {
        let mut state = self.state();
        let current = state
            .draft
            .as_ref()
            .filter(|draft| draft.version == *version)
            .ok_or(DraftConflict::Stale)?;
        if state.job {
            return Err(DraftConflict::Busy);
        }
        document
            .visualization
            .clone_from(&current.draft.document.visualization);
        for (index, (previous, proposed)) in current
            .draft
            .document
            .operation
            .scenarios
            .iter()
            .zip(&document.operation.scenarios)
            .enumerate()
        {
            if previous.asset_id != proposed.asset_id {
                document
                    .sources
                    .accepted
                    .remove(&format!("forecast:{index}"));
            }
        }
        let candidate = current
            .draft
            .validate_working_document(document)
            .map_err(|_| DraftConflict::Stale)?;
        let current = state.draft.as_mut().unwrap();
        if !current.draft.same_working_state(&candidate) {
            current.draft = candidate;
            current.advance();
        }
        Ok(())
    }

    pub fn start_draft_job(&self) -> Result<DraftJob, DraftConflict> {
        let mut state = self.state();
        if state.job {
            return Err(DraftConflict::Busy);
        }
        state.job = true;
        Ok(DraftJob {
            session: self.clone(),
            snapshot: state.draft.clone(),
        })
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

impl DraftSnapshot {
    fn advance(&mut self) {
        self.version.revision += 1;
    }
}

impl DraftJob {
    pub fn commit(&self, draft: Draft, newly_opened: bool) -> Result<(), DraftConflict> {
        let mut state = self.session.state();
        if state.draft.as_ref().map(|draft| &draft.version)
            != self.snapshot.as_ref().map(|draft| &draft.version)
        {
            return Err(DraftConflict::Stale);
        }
        if newly_opened {
            state.draft = Some(self.session.new_draft(draft)?);
        } else {
            let current = state.draft.as_mut().ok_or(DraftConflict::Stale)?;
            if draft.workspace != self.session.workspace() {
                return Err(DraftConflict::Stale);
            }
            if !current.draft.same_working_state(&draft) {
                current.advance();
            }
            current.draft = draft;
        }
        Ok(())
    }
}
impl Drop for DraftJob {
    fn drop(&mut self) {
        self.session.state().job = false;
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
            let identity =
                super::draft::identity(&(self.session.id, self.revision, root.to_string_lossy()))?;
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
