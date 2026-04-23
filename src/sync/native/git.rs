use async_trait::async_trait;
use std::path::Path;
use tokio::process::Command;

/// Abstraction over `git` + RDS `./sync` invocation so the sync pipeline can
/// be tested without spawning real processes.
#[async_trait]
pub trait GitExec: Send + Sync {
    /// `git -C <repo> <args...>` — Ok = trimmed stdout, Err = trimmed stderr or exit code.
    async fn git(&self, repo: &Path, args: &[&str]) -> Result<String, String>;

    /// Run repo-local `./sync`. Callers check `repo.join("sync")` existence
    /// first so "no script" stays distinguishable from "script failed".
    async fn run_rds_script(&self, repo: &Path) -> Result<(), String>;
}

pub struct SubprocessGit;

#[async_trait]
impl GitExec for SubprocessGit {
    async fn git(&self, repo: &Path, args: &[&str]) -> Result<String, String> {
        let output = Command::new("git")
            .args(["-C", &repo.to_string_lossy()])
            .args(args)
            .output()
            .await
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(if stderr.is_empty() {
                format!("exit code {}", output.status.code().unwrap_or(1))
            } else {
                stderr
            })
        }
    }

    async fn run_rds_script(&self, repo: &Path) -> Result<(), String> {
        let output = Command::new("./sync")
            .current_dir(repo)
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let msg = if stderr.is_empty() {
                format!("exit {}", output.status.code().unwrap_or(1))
            } else {
                stderr.lines().next().unwrap_or("failed").to_string()
            };
            Err(msg)
        }
    }
}

pub(super) async fn detect_default_branch(git: &dyn GitExec, repo: &Path) -> Option<String> {
    if let Ok(s) = git
        .git(repo, &["symbolic-ref", "refs/remotes/upstream/HEAD"])
        .await
    {
        return Some(s.replace("refs/remotes/upstream/", ""));
    }
    let out = git
        .git(
            repo,
            &[
                "for-each-ref",
                "--format=%(refname:short)",
                "refs/remotes/upstream/main",
                "refs/remotes/upstream/master",
            ],
        )
        .await
        .ok()?;
    for line in out.lines() {
        if let Some(name) = line.strip_prefix("upstream/") {
            return Some(name.to_string());
        }
    }
    None
}
