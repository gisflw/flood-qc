use super::{Session, SessionId, WorkspaceSelection, absolute};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunSelection {
    Active,
    Saved(SavedRun),
}

/// A validated independently retained run directory or bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedRun {
    root: PathBuf,
}

impl SavedRun {
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunIdentity {
    Active(PathBuf),
    Saved(PathBuf),
}

impl RunIdentity {
    pub fn root(&self) -> &Path {
        match self {
            Self::Active(root) | Self::Saved(root) => root,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RunRead<T> {
    pub identity: RunIdentity,
    pub data: T,
}

/// A run selection captured before a worker starts I/O.
#[derive(Clone, Debug)]
pub struct RunSnapshot {
    session: SessionId,
    revision: u64,
    selection: RunSelection,
    workspace: Arc<WorkspaceSelection>,
}

impl PartialEq for RunSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.session == other.session
            && self.revision == other.revision
            && self.selection == other.selection
    }
}

impl Eq for RunSnapshot {}

impl Session {
    pub fn run(&self) -> RunSnapshot {
        let state = self.state();
        RunSnapshot {
            session: self.id,
            revision: state.run_revision,
            selection: state.run.clone(),
            workspace: Arc::clone(&self.workspace),
        }
    }

    /// Commit a selection resolved from the current snapshot, without filesystem I/O.
    pub fn select_run(&self, candidate: RunSnapshot) -> Result<(), String> {
        let mut state = self.state();
        if self.id != candidate.session || state.run_revision != candidate.revision {
            return Err("Run selection has changed".into());
        }
        if state.run != candidate.selection {
            state.run = candidate.selection;
            state.run_revision += 1;
        }
        Ok(())
    }

    /// Detect late reads after a workspace or run selection has changed.
    pub fn is_current_run(&self, snapshot: &RunSnapshot) -> bool {
        *snapshot == self.run()
    }
}

impl RunSnapshot {
    pub fn selection(&self) -> &RunSelection {
        &self.selection
    }

    pub fn follow_active(mut self) -> Self {
        self.selection = RunSelection::Active;
        self
    }

    pub fn select_saved(mut self, path: &Path) -> Result<Self, String> {
        let path = absolute(&self.workspace.request().invocation_dir, path);
        let root = crate::store::saved_run_root(self.workspace.workspace(), &path)?;
        self.selection = RunSelection::Saved(SavedRun { root });
        Ok(self)
    }

    /// Resolve active identity under the guard and hold it through the artifact read.
    /// Readers supplied here use the resolved root directly.
    pub fn read<T>(
        &self,
        expected: Option<&RunIdentity>,
        read: impl FnOnce(&Path) -> Result<T, String>,
    ) -> Result<Option<RunRead<T>>, String> {
        let with_identity = |identity: RunIdentity| {
            if expected.is_some_and(|expected| expected != &identity) {
                return Err("Selected run changed while loading".into());
            }
            let data = read(identity.root())?;
            Ok(RunRead { identity, data })
        };
        let result = match &self.selection {
            RunSelection::Active => {
                crate::store::with_active_run(self.workspace.workspace(), |root| {
                    with_identity(RunIdentity::Active(root.to_path_buf()))
                })
            }
            RunSelection::Saved(saved) => {
                let root = crate::store::saved_run_root(self.workspace.workspace(), &saved.root)?;
                if root != saved.root {
                    return Err("Saved run directory has changed".into());
                }
                with_identity(RunIdentity::Saved(root)).map(Some)
            }
        }?;
        if result.is_none() && expected.is_some() {
            return Err("Selected run is no longer available".into());
        }
        Ok(result)
    }

    /// Export the exact selected generation or saved run to a retained bundle.
    pub fn export(
        &self,
        expected: &RunIdentity,
        preparation: Option<&Path>,
        destination: &Path,
    ) -> Result<super::super::RunBundleReport, String> {
        self.read(Some(expected), |root| {
            super::super::sharing::export_selected_run(
                self.workspace.workspace(),
                expected,
                root,
                preparation,
                destination,
            )
        })?
        .map(|loaded| loaded.data)
        .ok_or_else(|| "Selected run is no longer available".into())
    }

    pub fn summary(
        &self,
        expected: Option<&RunIdentity>,
    ) -> Result<Option<RunRead<SelectedRun>>, String> {
        self.read(expected, read_run)
    }
}

#[derive(Debug, Serialize)]
pub struct SelectedRun {
    pub root: PathBuf,
    pub manifest: crate::store::OperationManifest,
    pub day_horizons: Option<(u32, u32)>,
    pub scenarios: Vec<String>,
    pub findings: Vec<String>,
    pub reference: String,
    pub start: String,
    pub end: String,
    pub time_zone: String,
}
fn read_run(root: &Path) -> Result<SelectedRun, String> {
    let manifest = crate::store::read_retained_manifest(root)
        .map_err(|e| format!("{}: {e}", root.display()))?;
    if manifest.scenarios.iter().any(|s| {
        s.id.is_empty()
            || !s
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    }) {
        return Err("invalid scenario ID in retained manifest".into());
    }
    let mut scenarios = Vec::new();
    let mut findings = Vec::new();
    let mut first = None;
    for scenario in manifest.scenarios.iter().filter(|s| s.state == "completed") {
        match crate::store::read_result_series(
            &root.join("results").join(&scenario.id).join("basin.nc"),
        ) {
            Ok(basin) => {
                scenarios.push(scenario.id.clone());
                if first.is_none() {
                    first = Some(basin);
                }
            }
            Err(error) => findings.push(format!("{}: {error}", scenario.id)),
        }
    }
    let (reference, start, end, time_zone) = if let Some(basin) = first {
        let bounds = &basin.timeline.bounds;
        (
            basin
                .provenance
                .processing
                .get("reference_time")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .into(),
            bounds
                .first()
                .ok_or("empty result timeline")?
                .start
                .to_string()
                .replace(' ', "T"),
            bounds
                .last()
                .ok_or("empty result timeline")?
                .end
                .to_string()
                .replace(' ', "T"),
            basin.timeline.time_zone,
        )
    } else {
        (String::new(), String::new(), String::new(), String::new())
    };
    Ok(SelectedRun {
        root: root.to_path_buf(),
        day_horizons: super::super::retained_day_horizons(&manifest),
        manifest,
        scenarios,
        findings,
        reference,
        start,
        end,
        time_zone,
    })
}
