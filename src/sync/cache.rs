use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::ctx::Paths;

const CACHE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoPathsCache {
    pub version: u32,
    pub cached_at: DateTime<Utc>,
    pub paths: HashMap<String, PathBuf>,
}

pub fn load(paths: &Paths) -> Option<RepoPathsCache> {
    let path = paths.repo_paths_cache();
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    let cache: RepoPathsCache = serde_json::from_str(&content).ok()?;
    if cache.version != CACHE_VERSION {
        return None;
    }
    Some(cache)
}

pub fn save(paths: &Paths, cache: &RepoPathsCache) -> Result<()> {
    let path = paths.repo_paths_cache();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create cache directory {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(cache)?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write cache to {}", path.display()))?;
    Ok(())
}

pub fn is_valid(cache: &RepoPathsCache, expected_names: &[&str]) -> bool {
    for name in expected_names {
        match cache.paths.get(*name) {
            Some(p) if p.exists() => {}
            _ => return false,
        }
    }
    true
}

pub fn invalidate(paths: &Paths) {
    let _ = std::fs::remove_file(paths.repo_paths_cache());
}
