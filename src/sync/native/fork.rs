use std::path::Path;
use tokio::sync::mpsc;

use crate::tui::app::{ForkStatus, SyncUpdate};

use super::git::{detect_default_branch, GitExec};

pub struct ForkOpts {
    pub skip_fork_sync: bool,
    pub skip_git_fetch: bool,
    pub skip_rebase: bool,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub enum ForkResult {
    Updated,
    Unchanged,
    Error(String),
}

pub async fn sync_fork(
    git: &dyn GitExec,
    idx: usize,
    repo: &Path,
    tx: &mpsc::UnboundedSender<SyncUpdate>,
    opts: &ForkOpts,
) -> ForkResult {
    if opts.skip_fork_sync {
        let _ = tx.send(SyncUpdate::Fork(
            idx,
            ForkStatus::Skipped("--skip-fork-sync".into()),
        ));
        return ForkResult::Unchanged;
    }

    // Catches stale ~/.cache/jotmate/repo_paths.json pointing at a moved repo.
    if !repo.join(".git").exists() {
        let msg = format!("not a git repository: {}", repo.display());
        let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Error(msg.clone())));
        return ForkResult::Error(msg);
    }

    let remotes = match git.git(repo, &["remote"]).await {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Error(e)));
            return ForkResult::Error("no remotes".into());
        }
    };
    if !remotes.lines().any(|l| l.trim() == "upstream") {
        let _ = tx.send(SyncUpdate::Fork(
            idx,
            ForkStatus::Skipped("no upstream".into()),
        ));
        return ForkResult::Unchanged;
    }

    let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::FetchingUpstream));
    if !opts.skip_git_fetch {
        if let Err(e) = git.git(repo, &["fetch", "upstream"]).await {
            let msg = extract_fetch_error(&e);
            let _ = tx.send(SyncUpdate::Fork(
                idx,
                ForkStatus::Error(format!("fetch: {msg}")),
            ));
            return ForkResult::Error(msg);
        }
    }

    let default_branch = match detect_default_branch(git, repo).await {
        Some(b) => b,
        None => {
            let _ = tx.send(SyncUpdate::Fork(
                idx,
                ForkStatus::Skipped("no default branch".into()),
            ));
            return ForkResult::Unchanged;
        }
    };

    let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::CheckingDiff));

    let local_commit = git
        .git(repo, &["rev-parse", &default_branch])
        .await
        .unwrap_or_default();
    let upstream_ref = format!("upstream/{default_branch}");
    let upstream_commit = git
        .git(repo, &["rev-parse", &upstream_ref])
        .await
        .unwrap_or_default();

    if upstream_commit.is_empty() {
        let _ = tx.send(SyncUpdate::Fork(
            idx,
            ForkStatus::Skipped("no upstream ref".into()),
        ));
        return ForkResult::Unchanged;
    }

    if local_commit == upstream_commit {
        let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::UpToDate));
        return ForkResult::Unchanged;
    }

    let current_branch = git
        .git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .unwrap_or_default();

    let dirty = git
        .git(repo, &["diff-index", "--quiet", "HEAD", "--"])
        .await
        .is_err();
    if dirty {
        let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Stashing));
        let _ = git
            .git(
                repo,
                &["stash", "push", "-m", "Auto-stash before fork sync"],
            )
            .await;
    }

    // Local helper that reports an error and, if dirty, pops the stash.
    // Returns a matching ForkResult::Error so callers can early-return.
    async fn fail(
        git: &dyn GitExec,
        idx: usize,
        repo: &Path,
        tx: &mpsc::UnboundedSender<SyncUpdate>,
        dirty: bool,
        stage: &str,
        err: String,
    ) -> ForkResult {
        if dirty {
            let _ = git.git(repo, &["stash", "pop"]).await;
        }
        let _ = tx.send(SyncUpdate::Fork(
            idx,
            ForkStatus::Error(format!("{stage}: {err}")),
        ));
        ForkResult::Error(err)
    }

    let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::CheckingOut));
    if let Err(e) = git.git(repo, &["checkout", &default_branch]).await {
        return fail(git, idx, repo, tx, dirty, "checkout", e).await;
    }

    let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Merging));
    if let Err(e) = git.git(repo, &["merge", &upstream_ref, "--no-edit"]).await {
        return fail(git, idx, repo, tx, dirty, "merge", e).await;
    }

    let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::PushingDefault));
    if let Err(e) = git.git(repo, &["push", "origin", &default_branch]).await {
        return fail(git, idx, repo, tx, dirty, "push", e).await;
    }

    if !current_branch.is_empty() && current_branch != default_branch {
        if let Err(e) = git.git(repo, &["checkout", &current_branch]).await {
            return fail(git, idx, repo, tx, dirty, "checkout back", e).await;
        }

        if !opts.skip_rebase {
            let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Rebasing));
            if git.git(repo, &["rebase", &default_branch]).await.is_err() {
                let _ = git.git(repo, &["rebase", "--abort"]).await;
                if dirty {
                    let _ = git.git(repo, &["stash", "pop"]).await;
                }
                let _ = tx.send(SyncUpdate::Fork(
                    idx,
                    ForkStatus::Error("rebase conflict".into()),
                ));
                return ForkResult::Error("rebase conflict".into());
            }

            let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::PushingBranch));
            if let Err(e) = git
                .git(
                    repo,
                    &["push", "--force-with-lease", "origin", &current_branch],
                )
                .await
            {
                return fail(git, idx, repo, tx, dirty, "push branch", e).await;
            }
        }
    }

    if dirty {
        let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Unstashing));
        if let Err(e) = git.git(repo, &["stash", "pop"]).await {
            let _ = tx.send(SyncUpdate::Fork(
                idx,
                ForkStatus::Error(format!(
                    "stash pop conflict — run `git stash list` in this repo: {e}"
                )),
            ));
            return ForkResult::Error("stash pop conflict".into());
        }
    }

    let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Done));
    ForkResult::Updated
}

/// `git fetch` writes progress ("From …", "* [new branch] …", "remote: …") to
/// stderr even on success. When it exits non-zero, pull out the actual error
/// line so we don't surface benign progress as the failure reason.
fn extract_fetch_error(stderr: &str) -> String {
    let is_progress = |line: &str| {
        let t = line.trim_start();
        t.starts_with("From ")
            || t.starts_with("remote:")
            || t.starts_with("* ")
            || t.starts_with("+ ")
            || t.starts_with("- ")
            || t.starts_with("= ")
            || t.starts_with("! ")
            || t.starts_with("Fetching ")
            || t.is_empty()
    };

    stderr
        .lines()
        .rev()
        .find(|l| {
            let t = l.trim_start();
            !is_progress(l) && !t.to_lowercase().starts_with("warning:")
        })
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| "fetch failed".to_string())
}
