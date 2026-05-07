use async_trait::async_trait;
use std::path::Path;
use tokio::process::Command;

#[derive(Clone, Debug)]
pub enum RdsError {
    IpDenied { detail: String },
    Other(String),
}

/// Abstraction over `git` + RDS `./sync` invocation so the sync pipeline can
/// be tested without spawning real processes.
#[async_trait]
pub trait GitExec: Send + Sync {
    /// `git -C <repo> <args...>` — Ok = trimmed stdout, Err = trimmed stderr or exit code.
    async fn git(&self, repo: &Path, args: &[&str]) -> Result<String, String>;

    /// Run repo-local `./sync`. Callers check `repo.join("sync")` existence
    /// first so "no script" stays distinguishable from "script failed".
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
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let combined = format!("{stderr}\n{stdout}");
        Err(classify_rds_stderr(
            &combined,
            output.status.code().unwrap_or(1),
        ))
    }
}

const IP_DENIED_MARKERS: &[&str] = &[
    "connection timed out",
    "connection refused",
    "no route to host",
    "network is unreachable",
    "connection reset by peer",
    "permission denied (publickey",
    "kex_exchange_identification",
    "host key verification failed",
    "ip not allowed",
    "ip is not allowed",
    "access denied",
    "not authorized",
    "403 forbidden",
    "forbidden",
];

pub(super) fn classify_rds_stderr(output: &str, exit_code: i32) -> RdsError {
    let lower = output.to_lowercase();
    if let Some(marker) = IP_DENIED_MARKERS.iter().find(|m| lower.contains(*m)) {
        let detail = output
            .lines()
            .find(|l| l.to_lowercase().contains(*marker))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(*marker)
            .to_string();
        return RdsError::IpDenied { detail };
    }
    let first = output
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("exit {exit_code}"));
    RdsError::Other(first)
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
