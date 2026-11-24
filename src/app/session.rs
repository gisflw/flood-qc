mod run;
pub use self::run::{RunIdentity, RunRead, RunSelection, RunSnapshot, SavedRun, SelectedRun};

use super::{Draft, DraftDocument, InspectionReport, InspectionRequest, WorkspaceConfig, absolute};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
};

/// Explicit selection shared by one-shot inspection and long-lived sessions.
#[derive(Clone, Debug)]
pub struct WorkspaceSelection {
    request: InspectionRequest,
    workspace: PathBuf,
    config: PathBuf,
}

impl WorkspaceSelection {
    pub fn new(request: InspectionRequest) -> Self {
        let workspace = absolute(&request.invocation_dir, &request.workspace);
        let config = request.config.as_ref().map_or_else(
            || workspace.join("config/workspace.toml"),
            |path| absolute(&request.invocation_dir, path),
        );
        Self {
            request,
            workspace,
            config,
        }
    }

    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    pub fn config(&self) -> &Path {
        &self.config
    }

    pub(super) fn request(&self) -> &InspectionRequest {
        &self.request
    }

    pub fn inspect(&self) -> InspectionReport {
        super::inspect_selection(self)
    }

    pub fn read_config(&self) -> Result<WorkspaceConfig, String> {
        super::read_workspace_config(&self.config)
            .map_err(|error| format!("{}: {error}", self.config.display()))
    }

    pub fn open_draft(&self, path: &Path) -> Result<Draft, String> {
        Draft::open(self.workspace.clone(), &self.request.invocation_dir, path)
    }

    pub fn new_draft(&self) -> Result<Draft, String> {
        self.read_config().map(|config| {
            let mut draft = Draft::new_with_workspace_config(
                self.workspace.clone(),
                &self.request.invocation_dir,
                Some(&config),
            );
            let engine = draft.document.operation.engine.get_or_insert_default();
            if let Some(path) = &self.request.engine_executable {
                engine.executable = Some(path.clone());
            }
            if let Some(path) = &self.request.engine_input_dir {
                engine.input_dir = Some(path.clone());
            }
            draft
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SessionId(u64);

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Identifies the exact working revision, including across draft and workspace opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DraftConflict {
    Stale,
    JobPending,
    Invalid(String),
}

impl std::fmt::Display for DraftConflict {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Stale => "The working draft has changed or is no longer open",
            Self::JobPending => "A draft job is already active in this session",
            Self::Invalid(message) => message,
        })
    }
}

impl std::error::Error for DraftConflict {}

struct State {
    draft: Option<DraftSnapshot>,
    revision: u64,
    job_pending: bool,
    run: RunSelection,
    run_revision: u64,
}

/// A process-local workspace session. Clones share the same session state.
#[derive(Clone)]
pub struct Session {
    id: SessionId,
    workspace: Arc<WorkspaceSelection>,
    state: Arc<Mutex<State>>,
}

impl Session {
    pub fn new(request: InspectionRequest) -> Self {
        Self {
            id: SessionId(next_id()),
            workspace: Arc::new(WorkspaceSelection::new(request)),
            state: Arc::new(Mutex::new(State {
                draft: None,
                revision: 0,
                job_pending: false,
                run: RunSelection::Active,
                run_revision: 0,
            })),
        }
    }

    pub fn id(&self) -> SessionId {
        self.id
    }

    pub fn workspace(&self) -> &WorkspaceSelection {
        &self.workspace
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().expect("session mutex poisoned")
    }

    pub fn draft(&self) -> Option<DraftSnapshot> {
        self.state().draft.clone()
    }

    fn install(&self, state: &mut State, draft: Draft) -> Result<DraftVersion, DraftConflict> {
        if draft.workspace != self.workspace.workspace() {
            return Err(DraftConflict::Invalid(
                "Draft was opened for another workspace".into(),
            ));
        }
        state.revision += 1;
        let version = DraftVersion {
            session: self.id.0,
            draft: next_id(),
            revision: state.revision,
        };
        state.draft = Some(DraftSnapshot { version, draft });
        Ok(version)
    }

    /// Installs a separately opened or created draft as a new working draft.
    pub fn set_draft(&self, draft: Draft) -> Result<DraftVersion, DraftConflict> {
        let mut state = self.state();
        if state.job_pending {
            return Err(DraftConflict::JobPending);
        }
        self.install(&mut state, draft)
    }

    fn current(state: &State, expected: DraftVersion) -> Result<&DraftSnapshot, DraftConflict> {
        state
            .draft
            .as_ref()
            .filter(|draft| draft.version == expected)
            .ok_or(DraftConflict::Stale)
    }

    fn replace(
        state: &mut State,
        expected: DraftVersion,
        draft: Draft,
    ) -> Result<DraftVersion, DraftConflict> {
        let old = Self::current(state, expected)?;
        if old.draft.workspace != draft.workspace {
            return Err(DraftConflict::Invalid(
                "Draft belongs to another workspace".into(),
            ));
        }
        let mut version = expected;
        if !old.draft.same_edit_state(&draft) {
            state.revision += 1;
            version.revision = state.revision;
        }
        state.draft = Some(DraftSnapshot { version, draft });
        Ok(version)
    }

    pub fn validate_draft_update(
        &self,
        expected: DraftVersion,
        proposed: &DraftDocument,
    ) -> Result<(), DraftConflict> {
        Self::current(&self.state(), expected)?
            .draft
            .validate_working_document(proposed, false)
            .map_err(DraftConflict::Invalid)
    }

    /// Validate and apply a proposed document without writes or workflow side effects.
    pub fn apply_draft_update(
        &self,
        expected: DraftVersion,
        mut proposed: DraftDocument,
    ) -> Result<DraftVersion, DraftConflict> {
        let mut state = self.state();
        let mut draft = Self::current(&state, expected)?.draft.clone();
        if state.job_pending {
            return Err(DraftConflict::JobPending);
        }
        draft
            .validate_working_document(&proposed, false)
            .map_err(DraftConflict::Invalid)?;
        proposed
            .visualization
            .clone_from(&draft.document.visualization);
        draft.document = proposed;
        Self::replace(&mut state, expected, draft)
    }

    /// Commit a local form edit against the snapshot the form displayed.
    pub fn commit_form_edit(
        &self,
        expected: DraftVersion,
        proposed: DraftDocument,
    ) -> Result<DraftVersion, DraftConflict> {
        let mut state = self.state();
        let mut draft = Self::current(&state, expected)?.draft.clone();
        if state.job_pending {
            return Err(DraftConflict::JobPending);
        }
        draft
            .validate_working_document(&proposed, true)
            .map_err(DraftConflict::Invalid)?;
        draft.document = proposed;
        Self::replace(&mut state, expected, draft)
    }

    pub fn discard_draft_changes(&self) -> Result<(), DraftConflict> {
        let mut state = self.state();
        if state.job_pending {
            return Err(DraftConflict::JobPending);
        }
        if let Some(mut snapshot) = state.draft.clone() {
            snapshot.draft.discard();
            Self::replace(&mut state, snapshot.version, snapshot.draft)?;
        }
        Ok(())
    }

    /// Atomically admit a job and capture its input, holding admission until completion.
    pub fn begin_draft_job(&self) -> Result<DraftJob, DraftConflict> {
        let mut state = self.state();
        if state.job_pending {
            return Err(DraftConflict::JobPending);
        }
        state.job_pending = true;
        Ok(DraftJob {
            session: self.clone(),
            input: state.draft.clone(),
        })
    }
}

pub struct DraftJob {
    session: Session,
    input: Option<DraftSnapshot>,
}

impl DraftJob {
    pub fn input(&self) -> Option<&DraftSnapshot> {
        self.input.as_ref()
    }

    /// Install a workflow result for the same draft (save, refresh, materialization).
    pub fn replace(self, session: &Session, draft: Draft) -> Result<DraftVersion, DraftConflict> {
        self.check_session(session)?;
        let version = self.input.as_ref().ok_or(DraftConflict::Stale)?.version;
        Session::replace(&mut session.state(), version, draft)
    }

    /// Install a draft opened or created by this job with a new identity.
    pub fn open(self, session: &Session, draft: Draft) -> Result<DraftVersion, DraftConflict> {
        self.check_session(session)?;
        session.install(&mut session.state(), draft)
    }

    fn check_session(&self, session: &Session) -> Result<(), DraftConflict> {
        if self.session.id != session.id {
            return Err(DraftConflict::Stale);
        }
        Ok(())
    }
}

impl Drop for DraftJob {
    fn drop(&mut self) {
        self.session.state().job_pending = false;
    }
}

#[cfg(test)]
mod tests;
