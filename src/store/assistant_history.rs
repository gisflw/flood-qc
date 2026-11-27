use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatHistory {
    pub selected: Option<String>,
    pub chats: Vec<SavedChat>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedChat {
    pub id: String,
    pub title: String,
    pub agent: String,
    pub session_id: Option<String>,
    pub cwd: PathBuf,
    pub draft: String,
    pub carry_history: bool,
    pub turns: Vec<SavedTurn>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedTurn {
    pub prompt: String,
    pub answer: String,
    pub activity: Vec<SavedActivity>,
    pub interrupted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SavedActivity {
    Thought(String),
    Tool {
        id: Option<String>,
        title: Option<String>,
        status: Option<String>,
    },
}

#[derive(Clone)]
pub struct ChatHistoryStore {
    root: PathBuf,
}
impl ChatHistoryStore {
    pub fn open(app_data: &Path, workspace: &Path) -> Result<Self, String> {
        let workspace = fs::canonicalize(workspace).map_err(|e| e.to_string())?;
        let digest = Sha256::digest(workspace.as_os_str().as_encoded_bytes());
        let key = digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let root = app_data.join("assistant").join(key);
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
        Ok(Self { root })
    }
    pub fn load(&self) -> Result<ChatHistory, String> {
        let path = self.root.join("history.toml");
        if !path.exists() {
            return Ok(ChatHistory::default());
        }
        super::read_draft(&path)
    }
    pub fn save(&self, history: &ChatHistory) -> Result<(), String> {
        let bytes = super::draft::record_bytes(&self.root.join("history.toml"), history)?;
        let file = tempfile::Builder::new()
            .prefix(".history-")
            .tempfile_in(&self.root)
            .map_err(|e| e.to_string())?;
        use std::io::Write;
        let mut file = file;
        file.write_all(&bytes)
            .and_then(|()| file.as_file().sync_all())
            .map_err(|e| e.to_string())?;
        file.persist(self.root.join("history.toml"))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn new_cwd(&self) -> Result<PathBuf, String> {
        let dir = tempfile::Builder::new()
            .prefix("conversation-")
            .tempdir_in(&self.root)
            .map_err(|e| e.to_string())?;
        let path = dir.keep();
        fs::canonicalize(path).map_err(|e| e.to_string())
    }
    pub fn owns_cwd(&self, cwd: &Path) -> bool {
        cwd.parent() == Some(self.root.as_path())
            && cwd
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("conversation-"))
            && fs::canonicalize(cwd).is_ok_and(|canonical| canonical == cwd)
    }
    pub fn remove_cwd(&self, cwd: &Path) -> Result<(), String> {
        if cwd.parent() != Some(self.root.as_path())
            || !cwd
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("conversation-"))
        {
            return Err("Invalid conversation directory".into());
        }
        match fs::remove_dir_all(cwd) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_survives_restart_and_is_scoped_to_workspace() {
        std::fs::create_dir_all("runs").unwrap();
        let root = tempfile::tempdir_in("runs").unwrap();
        let a = root.path().join("a");
        let b = root.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let store = ChatHistoryStore::open(root.path(), &a).unwrap();
        let cwd = store.new_cwd().unwrap();
        let turns = vec![SavedTurn {
            prompt: "hello".into(),
            answer: "world".into(),
            ..SavedTurn::default()
        }];
        let history = ChatHistory {
            selected: Some("chat-1".into()),
            chats: vec![SavedChat {
                id: "chat-1".into(),
                title: "Review".into(),
                cwd,
                draft: "unfinished".into(),
                turns,
                ..SavedChat::default()
            }],
        };
        store.save(&history).unwrap();
        let reloaded = ChatHistoryStore::open(root.path(), &a)
            .unwrap()
            .load()
            .unwrap();
        assert_eq!(reloaded.selected.as_deref(), Some("chat-1"));
        assert_eq!(reloaded.chats[0].draft, "unfinished");
        assert_eq!(reloaded.chats[0].turns[0].answer, "world");
        assert!(
            ChatHistoryStore::open(root.path(), &b)
                .unwrap()
                .load()
                .unwrap()
                .chats
                .is_empty()
        );
    }
    #[test]
    fn save_error_does_not_prevent_a_later_retry() {
        fs::create_dir_all("runs").unwrap();
        let root = tempfile::tempdir_in("runs").unwrap();
        let workspace = root.path().join("workspace");
        fs::create_dir(&workspace).unwrap();
        let store = ChatHistoryStore::open(root.path(), &workspace).unwrap();
        fs::remove_dir_all(&store.root).unwrap();
        assert!(store.save(&ChatHistory::default()).is_err());
        fs::create_dir(&store.root).unwrap();
        store.save(&ChatHistory::default()).unwrap();
        assert!(store.load().unwrap().chats.is_empty());
    }
}
