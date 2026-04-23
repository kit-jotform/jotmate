//! Shared test fixtures: temp-dir-backed `Ctx`, in-memory keychain, and a
//! helper that wires a `mockito::Server` into the TimeDoctor HTTP endpoints.
//!
//! Every test should construct a `TestCtx` via `TestCtx::new()` and drop
//! the returned value when done; the `TempDir` cleans itself up. No test
//! should ever touch `~/.config/jotmate`, `~/.cache/jotmate`, or the real
//! macOS keychain.

#![allow(dead_code)] // shared helpers, not all used by every test file

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use jotmate::ctx::{Ctx, HttpBase, Paths};
use jotmate::time::keychain::KeychainStore;
use tempfile::TempDir;

/// In-memory `KeychainStore` for tests. Thread-safe, accepts pre-seeded
/// data, and can be configured to simulate access-denied errors.
pub struct FakeKeychain {
    inner: Mutex<HashMap<String, String>>,
    /// If set, all `get_*` calls return `Err` with this message, simulating
    /// an OS-level access-denied. `set_*` and `delete_*` are unaffected.
    get_error: Mutex<Option<String>>,
}

impl FakeKeychain {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(HashMap::new()),
            get_error: Mutex::new(None),
        })
    }

    pub fn with_entries(entries: &[(&str, &str)]) -> Arc<Self> {
        let kc = Self::new();
        for (k, v) in entries {
            kc.inner
                .lock()
                .unwrap()
                .insert((*k).to_string(), (*v).to_string());
        }
        kc
    }

    pub fn set_get_error(&self, msg: &str) {
        *self.get_error.lock().unwrap() = Some(msg.to_string());
    }

    pub fn clear_get_error(&self) {
        *self.get_error.lock().unwrap() = None;
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.lock().unwrap().contains_key(key)
    }

    pub fn get_raw(&self, key: &str) -> Option<String> {
        self.inner.lock().unwrap().get(key).cloned()
    }
}

impl KeychainStore for FakeKeychain {
    fn get_token(&self) -> Result<Option<String>> {
        if let Some(err) = self.get_error.lock().unwrap().as_ref() {
            anyhow::bail!("{err}");
        }
        Ok(self.inner.lock().unwrap().get("session-cookie").cloned())
    }

    fn set_token(&self, value: &str) -> Result<()> {
        self.inner
            .lock()
            .unwrap()
            .insert("session-cookie".to_string(), value.to_string());
        Ok(())
    }

    fn delete_token(&self) -> Result<()> {
        self.inner.lock().unwrap().remove("session-cookie");
        Ok(())
    }

    fn get_password(&self) -> Result<Option<String>> {
        if let Some(err) = self.get_error.lock().unwrap().as_ref() {
            anyhow::bail!("{err}");
        }
        Ok(self.inner.lock().unwrap().get("password").cloned())
    }

    fn set_password(&self, value: &str) -> Result<()> {
        self.inner
            .lock()
            .unwrap()
            .insert("password".to_string(), value.to_string());
        Ok(())
    }

    fn delete_password(&self) -> Result<()> {
        self.inner.lock().unwrap().remove("password");
        Ok(())
    }
}

/// A per-test context: owns the `TempDir`, exposes a ready-to-use `Ctx`
/// bound to it, and keeps a typed handle on the `FakeKeychain` so assertions
/// can inspect stored credentials.
pub struct TestCtx {
    /// Hold on to this so the temp dir isn't removed while the test runs.
    _tempdir: TempDir,
    pub ctx: Ctx,
    pub keychain: Arc<FakeKeychain>,
}

impl TestCtx {
    pub fn new() -> Self {
        let tempdir = TempDir::new().expect("failed to create tempdir");
        let root = tempdir.path().to_path_buf();
        Self::with_root(tempdir, &root, HttpBase::production())
    }

    /// Variant that points HTTP endpoints at a mockito server base URL.
    /// Pass a URL like `server.url()`; we append the standard path suffixes.
    pub fn with_mock_http(base_url: &str) -> Self {
        let tempdir = TempDir::new().expect("failed to create tempdir");
        let root = tempdir.path().to_path_buf();
        let http = HttpBase {
            login: format!("{base_url}/api/2.0/auth/v2/login"),
            stats: format!("{base_url}/api/1.1/stats/total"),
        };
        Self::with_root(tempdir, &root, http)
    }

    fn with_root(tempdir: TempDir, root: &Path, http: HttpBase) -> Self {
        let config_dir = root.join("config").join("jotmate");
        let cache_dir = root.join("cache").join("jotmate");
        let paths = Paths::new(config_dir, cache_dir);
        let keychain = FakeKeychain::new();
        let keychain_dyn: Arc<dyn KeychainStore> = keychain.clone();
        let ctx = Ctx {
            paths,
            keychain: keychain_dyn,
            http,
        };
        Self {
            _tempdir: tempdir,
            ctx,
            keychain,
        }
    }

    pub fn paths(&self) -> &Paths {
        &self.ctx.paths
    }

    pub fn config_file(&self) -> PathBuf {
        self.ctx.paths.config_file()
    }

    /// Write a raw TOML blob to the config path (creates parent dirs).
    pub fn write_config(&self, toml: &str) {
        let p = self.config_file();
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, toml).unwrap();
    }
}
