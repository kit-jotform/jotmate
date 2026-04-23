//! Composition root: filesystem roots, keychain, HTTP bases. Production builds
//! one `Ctx::production()`; tests swap in a temp-dir + fake keychain + mockito.
//! Prefer narrow leaf args (`&Paths`, `&dyn KeychainStore`) over `&Ctx`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::time::keychain::KeychainStore;

/// Production reads `dirs::config_dir()` / `dirs::cache_dir()`; tests pass a `TempDir`.
#[derive(Debug, Clone)]
pub struct Paths {
    config_dir: PathBuf,
    cache_dir: PathBuf,
}

impl Paths {
    #[allow(dead_code)] // used by integration tests
    pub fn new(config_dir: PathBuf, cache_dir: PathBuf) -> Self {
        Self {
            config_dir,
            cache_dir,
        }
    }

    pub fn production() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("jotmate");
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("~/.cache"))
            .join("jotmate");
        Self {
            config_dir,
            cache_dir,
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    #[allow(dead_code)] // used by integration tests
    pub fn cache_root(&self) -> &Path {
        &self.cache_dir
    }

    pub fn repo_paths_cache(&self) -> PathBuf {
        self.cache_dir.join("repo_paths.json")
    }

    pub fn time_cache_root(&self) -> PathBuf {
        self.cache_dir.join("time")
    }
}

/// TimeDoctor endpoint URLs; tests point these at mockito.
#[derive(Debug, Clone)]
pub struct HttpBase {
    pub login: String,
    pub stats: String,
}

impl HttpBase {
    pub fn production() -> Self {
        Self {
            login: "https://api2.timedoctor.com/api/2.0/auth/v2/login".to_string(),
            stats: "https://api2.timedoctor.com/api/1.1/stats/total".to_string(),
        }
    }
}

/// Top-level execution context; built once by `main.rs`, per-test in `tests/common`.
#[derive(Clone)]
pub struct Ctx {
    pub paths: Paths,
    pub keychain: Arc<dyn KeychainStore>,
    pub http: HttpBase,
}

impl Ctx {
    pub fn production() -> Self {
        Self {
            paths: Paths::production(),
            keychain: Arc::new(crate::time::keychain::SecurityCliKeychain),
            http: HttpBase::production(),
        }
    }
}
