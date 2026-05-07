use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const CACHE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize, Default)]
struct RdsStateFile {
    version: u32,
    repos: HashMap<String, String>,
}

pub struct RdsStateCache {
    path: PathBuf,
    inner: Mutex<HashMap<String, String>>,
}

impl RdsStateCache {
    pub fn load(path: PathBuf) -> Self {
        let inner = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<RdsStateFile>(&s).ok())
            .filter(|f| f.version == CACHE_VERSION)
            .map(|f| f.repos)
            .unwrap_or_default();
        Self {
            path,
            inner: Mutex::new(inner),
        }
    }

    pub fn last_synced_sha(&self, repo: &Path) -> Option<String> {
        let key = repo.to_string_lossy().to_string();
        self.inner.lock().unwrap().get(&key).cloned()
    }

    pub fn record_synced(&self, repo: &Path, sha: &str) {
        let key = repo.to_string_lossy().to_string();
        self.inner
            .lock()
            .unwrap()
            .insert(key, sha.trim().to_string());
        self.save();
    }

    fn save(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let snapshot = self.inner.lock().unwrap().clone();
        let file = RdsStateFile {
            version: CACHE_VERSION,
            repos: snapshot,
        };
        if let Ok(s) = serde_json::to_string_pretty(&file) {
            let _ = std::fs::write(&self.path, s);
        }
    }
}
