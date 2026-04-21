use std::path::Path;
use tokio::process::Command;

pub(super) async fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
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

pub(super) async fn git_ok(repo: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(["-C", &repo.to_string_lossy()])
        .args(args)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detect upstream default branch (main, master, or from symbolic ref).
pub(super) async fn detect_default_branch(repo: &Path) -> Option<String> {
    if git_ok(repo, &["rev-parse", "--verify", "upstream/main"]).await {
        return Some("main".to_string());
    }
    if git_ok(repo, &["rev-parse", "--verify", "upstream/master"]).await {
        return Some("master".to_string());
    }
    git(repo, &["symbolic-ref", "refs/remotes/upstream/HEAD"])
        .await
        .ok()
        .map(|s| s.replace("refs/remotes/upstream/", ""))
}
