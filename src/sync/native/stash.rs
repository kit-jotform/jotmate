use std::path::Path;

use super::git::GitExec;

pub(super) async fn save(git: &dyn GitExec, repo: &Path) -> Result<String, String> {
    let before = git
        .git(repo, &["stash", "list", "--format=%H", "-1"])
        .await?;
    git.git(
        repo,
        &[
            "stash",
            "push",
            "--include-untracked",
            "-m",
            "Auto-stash before worktree rebase",
        ],
    )
    .await
    .map_err(|e| format!("stash push: {e}"))?;
    let saved = git
        .git(repo, &["stash", "list", "--format=%H", "-1"])
        .await
        .map_err(|e| format!("cannot identify saved changes; check git stash list: {e}"))?;
    if saved.is_empty() || saved == before {
        return Err("stash did not save local changes; rebase cancelled".into());
    }
    Ok(saved)
}

pub(super) async fn restore(git: &dyn GitExec, repo: &Path, saved: &str) -> Result<(), String> {
    git.git(repo, &["stash", "apply", "--index", saved])
        .await
        .map_err(|e| format!("restore failed; changes kept in stash {saved}: {e}"))?;
    let entries = git
        .git(repo, &["stash", "list", "--format=%gd%x00%H"])
        .await
        .map_err(|e| format!("changes restored; stash {saved} retained: {e}"))?;
    if let Some((reference, _)) = entries
        .lines()
        .filter_map(|line| line.split_once('\0'))
        .find(|(_, oid)| *oid == saved)
    {
        git.git(repo, &["stash", "drop", reference])
            .await
            .map_err(|e| format!("changes restored; stash {saved} retained: {e}"))?;
    }
    Ok(())
}
