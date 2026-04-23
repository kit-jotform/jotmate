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
use async_trait::async_trait;
use jotmate::ctx::{Ctx, HttpBase, Paths};
use jotmate::sync::native::GitExec;
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

// ─── FakeGit — programmable GitExec for sync-pipeline tests ────────────────

/// A scripted `GitExec` for tests. Each command matches against a vector of
/// rules; the first rule whose args-prefix matches wins. Call `log()` to
/// inspect the exact sequence of invocations for assertions.
///
/// Typical use:
/// ```ignore
/// let git = FakeGit::new()
///     .on(&["remote"], Ok("origin\nupstream\n"))
///     .on(&["fetch", "upstream"], Ok(""))
///     .on(&["symbolic-ref"], Ok("refs/remotes/upstream/main"))
///     .on(&["rev-parse", "main"], Ok("aaaa"))
///     .on(&["rev-parse", "upstream/main"], Ok("aaaa"));  // up-to-date
/// ```
pub struct FakeGit {
    rules: Mutex<Vec<Rule>>,
    rds_result: Mutex<Result<(), String>>,
    calls: Mutex<Vec<Vec<String>>>,
}

struct Rule {
    prefix: Vec<String>,
    response: Result<String, String>,
    /// If set, this rule can fire only once. Useful for scripting "stash
    /// pop fails the first time" while keeping a default afterwards.
    once: bool,
    consumed: bool,
}

impl FakeGit {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            rules: Mutex::new(Vec::new()),
            rds_result: Mutex::new(Ok(())),
            calls: Mutex::new(Vec::new()),
        })
    }

    /// Register a rule: when the `git` call's args start with `prefix`,
    /// return `response`. Rules are matched in registration order.
    pub fn on(self: Arc<Self>, prefix: &[&str], response: Result<&str, &str>) -> Arc<Self> {
        let rule = Rule {
            prefix: prefix.iter().map(|s| s.to_string()).collect(),
            response: response.map(|s| s.to_string()).map_err(|s| s.to_string()),
            once: false,
            consumed: false,
        };
        self.rules.lock().unwrap().push(rule);
        self
    }

    /// Register a rule that fires exactly once, then is ignored (so a
    /// later, broader rule can handle subsequent calls). Use for
    /// "stash pop fails the first time, succeeds the second" or similar.
    pub fn on_once(self: Arc<Self>, prefix: &[&str], response: Result<&str, &str>) -> Arc<Self> {
        let rule = Rule {
            prefix: prefix.iter().map(|s| s.to_string()).collect(),
            response: response.map(|s| s.to_string()).map_err(|s| s.to_string()),
            once: true,
            consumed: false,
        };
        self.rules.lock().unwrap().push(rule);
        self
    }

    pub fn set_rds_result(&self, result: Result<(), String>) {
        *self.rds_result.lock().unwrap() = result;
    }

    pub fn log(&self) -> Vec<Vec<String>> {
        self.calls.lock().unwrap().clone()
    }

    /// Returns true if any logged call's args started with `prefix`.
    pub fn was_called(&self, prefix: &[&str]) -> bool {
        let prefix: Vec<String> = prefix.iter().map(|s| s.to_string()).collect();
        self.calls
            .lock()
            .unwrap()
            .iter()
            .any(|args| args.len() >= prefix.len() && args[..prefix.len()] == prefix[..])
    }

    /// Count of calls whose args started with `prefix`.
    pub fn call_count(&self, prefix: &[&str]) -> usize {
        let prefix: Vec<String> = prefix.iter().map(|s| s.to_string()).collect();
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|args| args.len() >= prefix.len() && args[..prefix.len()] == prefix[..])
            .count()
    }
}

#[async_trait]
impl GitExec for FakeGit {
    async fn git(&self, _repo: &Path, args: &[&str]) -> std::result::Result<String, String> {
        let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        self.calls.lock().unwrap().push(args_owned.clone());

        let mut rules = self.rules.lock().unwrap();
        for rule in rules.iter_mut() {
            if rule.once && rule.consumed {
                continue;
            }
            if args_owned.len() < rule.prefix.len() {
                continue;
            }
            if args_owned[..rule.prefix.len()] == rule.prefix[..] {
                if rule.once {
                    rule.consumed = true;
                }
                return rule.response.clone();
            }
        }
        Err(format!(
            "FakeGit: no rule matched args: {}",
            args_owned.join(" ")
        ))
    }

    async fn run_rds_script(&self, _repo: &Path) -> std::result::Result<(), String> {
        self.calls
            .lock()
            .unwrap()
            .push(vec!["__rds_script__".to_string()]);
        self.rds_result.lock().unwrap().clone()
    }
}

/// Helper: create a tempdir that looks like a git repo (has a `.git/`
/// subdirectory) so `fork.rs`'s `.git` existence check passes. Optionally
/// create a `sync` executable so RDS-phase tests can exercise the script path.
pub fn fake_repo_dir(with_sync_script: bool) -> TempDir {
    let td = TempDir::new().unwrap();
    std::fs::create_dir_all(td.path().join(".git")).unwrap();
    if with_sync_script {
        let p = td.path().join("sync");
        std::fs::write(&p, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&p).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&p, perms).unwrap();
        }
    }
    td
}
