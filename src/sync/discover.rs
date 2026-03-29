use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::cache::{self, RepoPathsCache};
use crate::config::UpstreamRepo;
use crate::error::AppError;

pub fn discover_all_git_repos() -> Result<Vec<PathBuf>> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;

    // Check fd is available
    let check = Command::new("fd").arg("--version").output();
    if check.is_err() || !check.unwrap().status.success() {
        return Err(AppError::FdNotFound.into());
    }

    let output = Command::new("fd")
        .args(["-H", "-t", "d", "^.git$", home.to_str().unwrap()])
        .output()
        .context("Failed to run fd")?;

    if !output.status.success() {
        anyhow::bail!(
            "fd exited with error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let repos: Vec<PathBuf> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let git_dir = PathBuf::from(line.trim());
            git_dir.parent().map(|p| p.to_path_buf())
        })
        .collect();

    Ok(repos)
}

fn get_remote_url(repo_path: &Path, remote: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", repo_path.to_str()?, "remote", "get-url", remote])
        .output()
        .ok()?;
    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !url.is_empty() {
            return Some(url);
        }
    }
    None
}

/// Returns the upstream URL for a repo, falling back to origin if no upstream remote exists.
pub fn get_upstream_url(repo_path: &Path) -> Option<String> {
    get_remote_url(repo_path, "upstream").or_else(|| get_remote_url(repo_path, "origin"))
}

/// Build a lookup map from (normalized URL → project name) for enabled repos,
/// including SSH variants of HTTPS URLs.
pub fn build_upstream_map(repos: &[UpstreamRepo]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for repo in repos.iter().filter(|r| r.enabled) {
        let url = repo.url.trim_end_matches('/').trim_end_matches(".git");
        // HTTPS form
        map.insert(format!("{}.git", url), repo.name.clone());
        map.insert(url.to_string(), repo.name.clone());

        // Derive SSH form from HTTPS: https://github.com/org/repo → git@github.com:org/repo
        if let Some(rest) = repo.url.strip_prefix("https://") {
            if let Some(slash) = rest.find('/') {
                let host = &rest[..slash];
                let path = rest[slash + 1..]
                    .trim_end_matches('/')
                    .trim_end_matches(".git");
                let ssh = format!("git@{}:{}", host, path);
                map.insert(format!("{}.git", ssh), repo.name.clone());
                map.insert(ssh, repo.name.clone());
            }
        }
    }
    map
}

pub fn match_repos_to_projects(
    repo_roots: &[PathBuf],
    upstream_repos: &[UpstreamRepo],
) -> Result<HashMap<String, PathBuf>> {
    let upstream_map = build_upstream_map(upstream_repos);
    let expected_names: Vec<&str> = upstream_repos
        .iter()
        .filter(|r| r.enabled)
        .map(|r| r.name.as_str())
        .collect();

    let mut found: HashMap<String, PathBuf> = HashMap::new();

    for repo_path in repo_roots {
        if let Some(url) = get_upstream_url(repo_path) {
            let url_trimmed = url.trim_end_matches('/');
            if let Some(name) = upstream_map.get(url_trimmed) {
                found
                    .entry(name.clone())
                    .or_insert_with(|| repo_path.clone());
            }
        }
        if found.len() == expected_names.len() {
            break;
        }
    }

    let missing: Vec<&str> = expected_names
        .iter()
        .filter(|n| !found.contains_key(**n))
        .copied()
        .collect();

    if !missing.is_empty() {
        anyhow::bail!(
            "Could not find local paths for: {}. \
             Ensure you have cloned these repos with an 'upstream' remote configured.",
            missing.join(", ")
        );
    }

    Ok(found)
}

pub fn discover_and_cache(upstream_repos: &[UpstreamRepo]) -> Result<RepoPathsCache> {
    println!("Discovering git repositories (this may take a moment)...");
    let repos = discover_all_git_repos()?;
    println!(
        "Found {} git repos, matching against known upstreams...",
        repos.len()
    );

    let paths = match_repos_to_projects(&repos, upstream_repos)?;

    println!("All repositories located:");
    for (project, path) in &paths {
        println!("  {project}: {}", path.display());
    }

    let cache = RepoPathsCache {
        version: 1,
        cached_at: Utc::now(),
        paths,
    };

    cache::save(&cache)?;
    Ok(cache)
}
