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

pub(super) async fn detect_default_branch(repo: &Path) -> Option<String> {
    if let Ok(s) = git(repo, &["symbolic-ref", "refs/remotes/upstream/HEAD"]).await {
        return Some(s.replace("refs/remotes/upstream/", ""));
    }
    let out = git(
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
