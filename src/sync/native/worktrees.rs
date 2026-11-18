use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

use super::fork::{sync_checkout, ForkOpts, ForkResult};
use super::git::{detect_default_branch_for_remote, GitExec};
use super::stash;
use crate::tui::app::{ForkStatus, SyncUpdate, WorktreeSyncState};

#[derive(Default)]
struct Worktree {
    path: PathBuf,
    branch: Option<String>,
    unavailable: bool,
}

pub(super) async fn sync_fork(
    git: &dyn GitExec,
    idx: usize,
    repo: &Path,
    tx: &mpsc::UnboundedSender<SyncUpdate>,
    opts: &ForkOpts,
) -> ForkResult {
    let listing = match git
        .git(repo, &["worktree", "list", "--porcelain", "-z"])
        .await
    {
        Ok(listing) => listing,
        Err(e) => return report_error(idx, tx, format!("worktree list: {e}")),
    };
    let worktrees: Vec<Worktree> = listing
        .split("\0\0")
        .filter(|record| !record.is_empty())
        .map(|record| {
            let mut worktree = Worktree::default();
            for field in record.split('\0') {
                if let Some(path) = field.strip_prefix("worktree ") {
                    worktree.path = PathBuf::from(path);
                } else if let Some(branch) = field.strip_prefix("branch refs/heads/") {
                    worktree.branch = Some(branch.to_string());
                } else if field == "locked"
                    || field.starts_with("locked ")
                    || field == "prunable"
                    || field.starts_with("prunable ")
                {
                    worktree.unavailable = true;
                }
            }
            worktree
        })
        .collect();
    if worktrees.len() <= 1 {
        return sync_checkout(git, idx, repo, tx, opts).await;
    }
    let remote = match git.git(repo, &["remote"]).await {
        Ok(remotes) if remotes.lines().any(|r| r == "upstream") => "upstream",
        Ok(_) => "origin",
        Err(e) => return report_error(idx, tx, e),
    };
    let mut fork_opts = ForkOpts {
        skip_fork_sync: opts.skip_fork_sync,
        skip_git_fetch: opts.skip_git_fetch,
        skip_rebase: opts.skip_rebase,
        skip_worktree_rebase: opts.skip_worktree_rebase,
    };
    let mut default_branch = detect_default_branch_for_remote(git, repo, remote).await;
    // A newly added remote may have no local refs until its first fetch.
    if default_branch.is_none() && !opts.skip_git_fetch {
        let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::FetchingUpstream));
        if let Err(e) = git.git(repo, &["fetch", remote]).await {
            return report_error(idx, tx, format!("fetch: {e}"));
        }
        fork_opts.skip_git_fetch = true;
        default_branch = detect_default_branch_for_remote(git, repo, remote).await;
    }
    let Some(default_branch) = default_branch else {
        return sync_checkout(git, idx, repo, tx, &fork_opts).await;
    };
    // Update the default branch where it is checked out; Git forbids checking it out twice.
    let owner = worktrees
        .iter()
        .find(|w| w.branch.as_deref() == Some(default_branch.as_str()));
    if let Some(owner) = owner {
        if owner.unavailable {
            return report_error(
                idx,
                tx,
                format!(
                    "{}: default worktree is locked or prunable",
                    owner.path.display()
                ),
            );
        }
    }
    let sync_repo = owner.map(|w| w.path.as_path()).unwrap_or(repo);
    if sync_repo != repo {
        match skip_reason(git, sync_repo).await {
            Ok(None) => {}
            Ok(Some(reason)) => {
                return report_error(idx, tx, format!("{}: {reason}", sync_repo.display()))
            }
            Err(e) => return report_error(idx, tx, format!("{}: {e}", sync_repo.display())),
        }
    }
    let result = sync_checkout(git, idx, sync_repo, tx, &fork_opts).await;
    if matches!(result, ForkResult::Error(_)) {
        return result;
    }
    for worktree in &worktrees {
        if worktree.path == sync_repo {
            continue;
        }
        let update = |status| {
            let _ = tx.send(SyncUpdate::Worktree(
                idx,
                WorktreeSyncState {
                    path: worktree.path.clone(),
                    branch: worktree.branch.clone(),
                    status,
                },
            ));
        };
        let disabled = if worktree.path == repo {
            opts.skip_rebase
        } else {
            opts.skip_worktree_rebase
        };
        if disabled {
            update(ForkStatus::Skipped("rebase disabled".into()));
        } else if worktree.branch.is_none() {
            update(ForkStatus::Skipped("detached HEAD".into()));
        } else if worktree.branch.as_deref() == Some(default_branch.as_str()) {
            update(ForkStatus::Skipped("default branch".into()));
        } else if worktree.unavailable {
            update(ForkStatus::Skipped("locked or prunable".into()));
        } else {
            update(ForkStatus::Pending);
            let status = match rebase_worktree(git, &worktree.path, &default_branch, &update).await
            {
                Ok(status) => status,
                Err(e) => ForkStatus::Error(e),
            };
            update(status);
        }
    }
    // Worktree-only changes must not force another RDS upload of the main checkout.
    result
}

fn report_error(idx: usize, tx: &mpsc::UnboundedSender<SyncUpdate>, message: String) -> ForkResult {
    let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Error(message.clone())));
    ForkResult::Error(message)
}

async fn skip_reason(git: &dyn GitExec, repo: &Path) -> Result<Option<&'static str>, String> {
    let status = git.git(repo, &["status", "--porcelain"]).await?;
    if !status.is_empty() {
        return Ok(Some("dirty tree"));
    }
    if operation_in_progress(git, repo).await? {
        return Ok(Some("Git operation in progress"));
    }
    Ok(None)
}

async fn operation_in_progress(git: &dyn GitExec, repo: &Path) -> Result<bool, String> {
    let git_dir = PathBuf::from(git.git(repo, &["rev-parse", "--absolute-git-dir"]).await?);
    Ok([
        "rebase-merge",
        "rebase-apply",
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "sequencer",
    ]
    .iter()
    .any(|entry| git_dir.join(entry).exists()))
}

async fn rebase_worktree(
    git: &dyn GitExec,
    repo: &Path,
    default_branch: &str,
    update: &(impl Fn(ForkStatus) + Sync),
) -> Result<ForkStatus, String> {
    if operation_in_progress(git, repo).await? {
        return Ok(ForkStatus::Skipped("Git operation in progress".into()));
    }
    if git
        .git(
            repo,
            &["merge-base", "--is-ancestor", default_branch, "HEAD"],
        )
        .await
        .is_ok()
    {
        return Ok(ForkStatus::UpToDate);
    }
    let dirty = !git.git(repo, &["status", "--porcelain"]).await?.is_empty();
    let saved = if dirty {
        update(ForkStatus::Stashing);
        Some(stash::save(git, repo).await?)
    } else {
        None
    };
    update(ForkStatus::Rebasing);
    let rebase = git.git(repo, &["rebase", default_branch]).await;
    if let Err(e) = &rebase {
        if let Err(abort) = git.git(repo, &["rebase", "--abort"]).await {
            let recovery = saved
                .as_deref()
                .map(|oid| format!("; local changes kept in stash {oid}"))
                .unwrap_or_default();
            return Err(format!(
                "rebase failed: {e}; abort failed: {abort}{recovery}"
            ));
        }
    }
    if let Some(saved) = saved {
        update(ForkStatus::Unstashing);
        if let Err(restore) = stash::restore(git, repo, &saved).await {
            return Err(match rebase {
                Ok(_) => restore,
                Err(e) => format!("rebase failed and was aborted: {e}; {restore}"),
            });
        }
    }
    match rebase {
        Ok(_) => Ok(ForkStatus::Done),
        Err(e) => Err(format!("rebase failed and was aborted: {e}")),
    }
}
