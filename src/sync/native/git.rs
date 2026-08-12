use async_trait::async_trait;
use std::path::Path;
use tokio::process::Command;

#[derive(Clone, Debug)]
pub enum RdsError {
    IpDenied { detail: String },
    Other(String),
}

/// Injectable `git` and `./sync` (real subprocess vs test doubles).
#[async_trait]
pub trait GitExec: Send + Sync {
    /// `git -C <repo> …` — success = trimmed stdout; failure = stderr or exit message.
    async fn git(&self, repo: &Path, args: &[&str]) -> Result<String, String>;

    /// `./sync` in the repo root. Distinguish "no script" via `repo.join("sync")` before calling.
    async fn run_rds_script(&self, repo: &Path) -> Result<(), RdsError>;
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

    async fn run_rds_script(&self, repo: &Path) -> Result<(), RdsError> {
        let output = Command::new("./sync")
            .current_dir(repo)
            .output()
            .await
            .map_err(|e| RdsError::Other(e.to_string()))?;
        // The repo ./sync scripts always exit 0 even when ssh/rsync fails, so we
        // must inspect output regardless of exit code.
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        );
        if let Some(detail) = detect_ip_denial(&combined) {
            return Err(RdsError::IpDenied { detail });
        }
        if output.status.success() {
            return Ok(());
        }
        let first = combined
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("exit {}", output.status.code().unwrap_or(1)));
        Err(RdsError::Other(first))
    }
}

const IP_DENIED_MARKERS: &[&str] = &[
    "unexpected end of file",
    "connection closed by",
    "rsync error:",
    "connection timed out",
    "connection refused",
    "permission denied (publickey",
];

pub fn detect_ip_denial(output: &str) -> Option<String> {
    let lower = output.to_lowercase();
    let marker = IP_DENIED_MARKERS.iter().find(|m| lower.contains(*m))?;
    let detail = output
        .lines()
        .find(|l| l.to_lowercase().contains(*marker))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(*marker)
        .to_string();
    Some(detail)
}

pub(super) async fn detect_default_branch(git: &dyn GitExec, repo: &Path) -> Option<String> {
    detect_default_branch_for_remote(git, repo, "upstream").await
}

pub(super) async fn detect_default_branch_for_remote(
    git: &dyn GitExec,
    repo: &Path,
    remote: &str,
) -> Option<String> {
    let head_ref = format!("refs/remotes/{remote}/HEAD");
    if let Ok(s) = git.git(repo, &["symbolic-ref", &head_ref]).await {
        let prefix = format!("refs/remotes/{remote}/");
        return Some(s.replace(&prefix, ""));
    }
    let main_ref = format!("refs/remotes/{remote}/main");
    let master_ref = format!("refs/remotes/{remote}/master");
    let out = git
        .git(
            repo,
            &[
                "for-each-ref",
                "--format=%(refname:short)",
                &main_ref,
                &master_ref,
            ],
        )
        .await
        .ok()?;
    let strip_prefix = format!("{remote}/");
    for line in out.lines() {
        if let Some(name) = line.strip_prefix(&strip_prefix) {
            return Some(name.to_string());
        }
    }
    None
}
