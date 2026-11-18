use async_trait::async_trait;
use jotmate::sync::native::fork::{sync_fork, ForkOpts, ForkResult};
use jotmate::sync::native::{run_tui, GitExec, RdsError, RdsStateCache, SubprocessGit, SyncOpts};
use jotmate::tui::app::{ForkStatus, SyncUpdate};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::sync::mpsc;

#[derive(Default)]
struct RecordingGit {
    calls: Mutex<Vec<(PathBuf, Vec<String>)>>,
    uploads: Mutex<Vec<PathBuf>>,
    overrides: Mutex<Vec<GitOverride>>,
}
struct GitOverride {
    repo: PathBuf,
    args: Vec<String>,
    result: Result<String, String>,
}
impl RecordingGit {
    fn override_git(&self, repo: &Path, args: &[&str], result: Result<&str, &str>) {
        self.overrides.lock().unwrap().push(GitOverride {
            repo: repo.to_path_buf(),
            args: args.iter().map(|s| s.to_string()).collect(),
            result: result.map(str::to_string).map_err(str::to_string),
        });
    }
}
#[async_trait]
impl GitExec for RecordingGit {
    async fn git(&self, repo: &Path, args: &[&str]) -> Result<String, String> {
        self.calls.lock().unwrap().push((
            repo.to_path_buf(),
            args.iter().map(|s| s.to_string()).collect(),
        ));
        if let Some(rule) = self.overrides.lock().unwrap().iter().find(|rule| {
            rule.repo == repo
                && args
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .starts_with(&rule.args)
        }) {
            return rule.result.clone();
        }
        SubprocessGit.git(repo, args).await
    }
    async fn run_rds_script(&self, repo: &Path) -> Result<(), RdsError> {
        self.uploads.lock().unwrap().push(repo.to_path_buf());
        Ok(())
    }
}
fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}
struct Fixture {
    _root: TempDir,
    root: PathBuf,
    upstream: PathBuf,
    repo: PathBuf,
    feature: PathBuf,
}
impl Fixture {
    fn new(origin_only: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let upstream = root.join("upstream");
        std::fs::create_dir(&upstream).unwrap();
        git(&upstream, &["init", "-b", "main"]);
        std::fs::write(upstream.join("base"), "base\n").unwrap();
        std::fs::write(upstream.join("sync"), "#!/bin/sh\nexit 0\n").unwrap();
        git(&upstream, &["add", "."]);
        git(&upstream, &["commit", "-m", "base"]);
        let origin = root.join("origin.git");
        let repo = root.join("repo");
        if origin_only {
            git(
                &root,
                &["clone", upstream.to_str().unwrap(), repo.to_str().unwrap()],
            );
        } else {
            git(
                &root,
                &[
                    "clone",
                    "--bare",
                    upstream.to_str().unwrap(),
                    origin.to_str().unwrap(),
                ],
            );
            git(
                &root,
                &["clone", origin.to_str().unwrap(), repo.to_str().unwrap()],
            );
            git(
                &repo,
                &["remote", "add", "upstream", upstream.to_str().unwrap()],
            );
            git(&repo, &["fetch", "upstream"]);
        }
        git(&repo, &["config", "user.name", "Test"]);
        git(&repo, &["config", "user.email", "test@example.com"]);
        git(&repo, &["config", "commit.gpgsign", "false"]);
        let feature = root.join("feature with spaces");
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "feature",
                feature.to_str().unwrap(),
            ],
        );
        std::fs::write(feature.join("feature"), "feature\n").unwrap();
        git(&feature, &["add", "."]);
        git(&feature, &["commit", "-m", "feature"]);
        Self {
            _root: temp,
            root,
            upstream,
            repo,
            feature,
        }
    }
    fn advance(&self, conflict: bool) {
        let name = if conflict { "feature" } else { "new-default" };
        std::fs::write(self.upstream.join(name), "upstream\n").unwrap();
        git(&self.upstream, &["add", "."]);
        git(&self.upstream, &["commit", "-m", "advance default"]);
    }
    async fn sync(
        &self,
        git: &RecordingGit,
        skip_rebase: bool,
        skip_worktree_rebase: bool,
    ) -> (ForkResult, Vec<SyncUpdate>) {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let result = sync_fork(
            git,
            0,
            &self.repo,
            &tx,
            &ForkOpts {
                skip_fork_sync: false,
                skip_git_fetch: false,
                skip_rebase,
                skip_worktree_rebase,
            },
        )
        .await;
        let mut statuses = Vec::new();
        while let Ok(update) = rx.try_recv() {
            statuses.push(update);
        }
        (result, statuses)
    }
    fn assert_rebased(&self) {
        assert_eq!(
            git(&self.feature, &["show", "HEAD:new-default"]),
            "upstream"
        );
        assert_eq!(git(&self.feature, &["show", "HEAD:feature"]), "feature");
    }
}
#[tokio::test]
async fn rebases_worktree_without_push_and_uploads_only_main_repo() {
    let f = Fixture::new(false);
    f.advance(false);
    let git = Arc::new(RecordingGit::default());
    let (tx, _rx) = mpsc::unbounded_channel();
    run_tui(
        git.clone(),
        vec![(0, f.repo.clone())],
        tx,
        SyncOpts {
            skip_fork_sync: false,
            skip_git_fetch: false,
            skip_rebase: false,
            skip_worktree_rebase: false,
            skip_rds_sync: false,
            smart_sync: false,
        },
        Arc::new(RdsStateCache::load(f.root.join("rds-state.json"))),
    )
    .await;
    f.assert_rebased();
    assert_eq!(*git.uploads.lock().unwrap(), vec![f.repo.clone()]);
    assert!(git
        .calls
        .lock()
        .unwrap()
        .iter()
        .all(|(p, args)| p != &f.feature
            || !matches!(args[0].as_str(), "push" | "fetch" | "checkout")));
    assert_eq!(
        git.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, args)| args[0] == "fetch")
            .count(),
        1
    );
}
#[tokio::test]
async fn origin_only_repo_also_rebases_worktrees() {
    let f = Fixture::new(true);
    f.advance(false);
    let (result, _) = f.sync(&RecordingGit::default(), false, false).await;
    assert_eq!(result, ForkResult::Updated);
    f.assert_rebased();
}
#[tokio::test]
async fn rebases_stale_worktree_even_when_main_is_already_current() {
    let f = Fixture::new(false);
    f.advance(false);
    git(&f.repo, &["fetch", "upstream"]);
    git(&f.repo, &["merge", "--ff-only", "upstream/main"]);
    let (result, _) = f.sync(&RecordingGit::default(), false, false).await;
    assert_eq!(result, ForkResult::Unchanged);
    f.assert_rebased();
}
#[tokio::test]
async fn skip_worktree_rebase_leaves_worktree_head_unchanged() {
    let f = Fixture::new(false);
    let before = git(&f.feature, &["rev-parse", "HEAD"]);
    f.advance(false);
    let (result, _) = f.sync(&RecordingGit::default(), false, true).await;
    assert_eq!(result, ForkResult::Updated);
    assert_eq!(git(&f.feature, &["rev-parse", "HEAD"]), before);
}
#[tokio::test]
async fn dirty_worktree_rebases_and_detached_worktree_is_skipped() {
    let f = Fixture::new(false);
    std::fs::write(f.feature.join("untracked"), "do not touch").unwrap();
    let detached = f.root.join("detached");
    git(
        &f.repo,
        &["worktree", "add", "--detach", detached.to_str().unwrap()],
    );
    let before = git(&detached, &["rev-parse", "HEAD"]);
    f.advance(false);
    let (result, updates) = f.sync(&RecordingGit::default(), false, false).await;
    assert_eq!(result, ForkResult::Updated);
    assert!(matches!(
        worktree_status(&updates, &f.feature),
        ForkStatus::Done
    ));
    assert!(
        matches!(worktree_status(&updates,&detached),ForkStatus::Skipped(s) if s=="detached HEAD")
    );
    f.assert_rebased();
    assert_eq!(git(&detached, &["rev-parse", "HEAD"]), before);
    assert_eq!(
        std::fs::read_to_string(f.feature.join("untracked")).unwrap(),
        "do not touch"
    );
}
#[tokio::test]
async fn conflict_is_aborted_and_other_worktrees_still_rebase() {
    let f = Fixture::new(false);
    let before = git(&f.feature, &["rev-parse", "HEAD"]);
    let clean = f.root.join("clean");
    git(
        &f.repo,
        &["worktree", "add", "-b", "clean", clean.to_str().unwrap()],
    );
    f.advance(true);
    let (result, updates) = f.sync(&RecordingGit::default(), false, false).await;
    assert_eq!(result, ForkResult::Updated);
    assert!(
        matches!(worktree_status(&updates,&f.feature),ForkStatus::Error(s) if s.contains("aborted"))
    );
    assert!(matches!(
        worktree_status(&updates, &clean),
        ForkStatus::Done
    ));
    assert!(updates
        .iter()
        .any(|u| matches!(u, SyncUpdate::Fork(_, ForkStatus::Done))));
    assert_eq!(git(&f.feature, &["rev-parse", "HEAD"]), before);
    assert!(git(&f.feature, &["status", "--porcelain"]).is_empty());
    assert_eq!(
        git(&clean, &["rev-parse", "HEAD"]),
        git(&f.repo, &["rev-parse", "main"])
    );
}
#[tokio::test]
async fn default_branch_checked_out_in_another_worktree_is_updated_there() {
    let f = Fixture::new(false);
    git(&f.repo, &["checkout", "-b", "primary-feature"]);
    let owner = f.root.join("default owner");
    git(
        &f.repo,
        &["worktree", "add", owner.to_str().unwrap(), "main"],
    );
    f.advance(false);
    let (result, _) = f.sync(&RecordingGit::default(), false, false).await;
    assert_eq!(result, ForkResult::Updated);
    f.assert_rebased();
    assert_eq!(
        git(&f.repo, &["branch", "--show-current"]),
        "primary-feature"
    );
    assert_eq!(git(&f.repo, &["show", "HEAD:new-default"]), "upstream");
}
#[tokio::test]
async fn locked_and_in_progress_worktrees_are_preserved() {
    let f = Fixture::new(false);
    git(&f.repo, &["worktree", "lock", f.feature.to_str().unwrap()]);
    let active = f.root.join("active");
    git(
        &f.repo,
        &["worktree", "add", "-b", "active", active.to_str().unwrap()],
    );
    let dir = git(&active, &["rev-parse", "--absolute-git-dir"]);
    std::fs::create_dir(Path::new(&dir).join("rebase-merge")).unwrap();
    f.advance(false);
    let (result, updates) = f.sync(&RecordingGit::default(), false, false).await;
    assert_eq!(result, ForkResult::Updated);
    assert!(
        matches!(worktree_status(&updates,&f.feature),ForkStatus::Skipped(s) if s.contains("locked"))
    );
    assert!(
        matches!(worktree_status(&updates,&active),ForkStatus::Skipped(s) if s.contains("operation in progress"))
    );
    assert!(Path::new(&dir).join("rebase-merge").exists());
}
#[tokio::test]
async fn newly_added_upstream_is_fetched_before_locating_default_worktree() {
    let f = Fixture::new(false);
    git(&f.repo, &["remote", "remove", "upstream"]);
    git(
        &f.repo,
        &["remote", "add", "upstream", f.upstream.to_str().unwrap()],
    );
    git(&f.repo, &["checkout", "-b", "primary-feature"]);
    let owner = f.root.join("default owner");
    git(
        &f.repo,
        &["worktree", "add", owner.to_str().unwrap(), "main"],
    );
    f.advance(false);
    let recorder = RecordingGit::default();
    let (result, _) = f.sync(&recorder, false, false).await;
    assert_eq!(result, ForkResult::Updated);
    f.assert_rebased();
    assert_eq!(
        recorder
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, args)| args[0] == "fetch")
            .count(),
        1
    );
}
#[tokio::test]
async fn dirty_default_worktree_is_not_stashed_or_modified() {
    let f = Fixture::new(false);
    git(&f.repo, &["checkout", "-b", "primary-feature"]);
    let owner = f.root.join("default owner");
    git(
        &f.repo,
        &["worktree", "add", owner.to_str().unwrap(), "main"],
    );
    std::fs::write(owner.join("base"), "in progress").unwrap();
    f.advance(false);
    let recorder = RecordingGit::default();
    let (result, _) = f.sync(&recorder, false, false).await;
    assert!(
        matches!(result,ForkResult::Error(s) if s.contains("dirty tree") && s.contains("default owner"))
    );
    assert_eq!(
        std::fs::read_to_string(owner.join("base")).unwrap(),
        "in progress"
    );
    assert!(recorder
        .calls
        .lock()
        .unwrap()
        .iter()
        .all(|(_, args)| !matches!(
            args[0].as_str(),
            "stash" | "rebase" | "checkout" | "merge" | "push"
        )));
}
#[tokio::test]
async fn current_branch_and_worktree_rebase_can_be_disabled_independently() {
    for default_in_worktree in [false, true] {
        for (skip_rebase, skip_worktree_rebase) in [(true, false), (false, true)] {
            let f = Fixture::new(false);
            git(&f.repo, &["checkout", "-b", "primary-feature"]);
            if default_in_worktree {
                let owner = f.root.join("default owner");
                git(
                    &f.repo,
                    &["worktree", "add", owner.to_str().unwrap(), "main"],
                );
            }
            let primary_before = git(&f.repo, &["rev-parse", "HEAD"]);
            let linked_before = git(&f.feature, &["rev-parse", "HEAD"]);
            f.advance(false);
            let (result, _) = f
                .sync(&RecordingGit::default(), skip_rebase, skip_worktree_rebase)
                .await;
            assert_eq!(result, ForkResult::Updated);
            assert_eq!(
                git(&f.repo, &["rev-parse", "HEAD"]) == primary_before,
                skip_rebase
            );
            assert_eq!(
                git(&f.feature, &["rev-parse", "HEAD"]) == linked_before,
                skip_worktree_rebase
            );
            if !skip_worktree_rebase {
                f.assert_rebased();
            }
        }
    }
}

fn worktree_status<'a>(updates: &'a [SyncUpdate], path: &Path) -> &'a ForkStatus {
    updates
        .iter()
        .rev()
        .find_map(|update| match update {
            SyncUpdate::Worktree(_, state) if state.path == path => Some(&state.status),
            _ => None,
        })
        .expect("worktree status should be reported separately")
}

#[tokio::test]
async fn dirty_rebase_restores_staged_unstaged_and_untracked_changes_and_preserves_other_stash() {
    let f = Fixture::new(false);
    std::fs::write(f.repo.join("base"), "unrelated stash\n").unwrap();
    git(&f.repo, &["stash", "push", "-m", "user stash"]);
    let previous_stash = git(&f.repo, &["stash", "list", "--format=%H"]);
    std::fs::write(f.feature.join("base"), "staged\n").unwrap();
    git(&f.feature, &["add", "base"]);
    std::fs::write(f.feature.join("base"), "staged\nunstaged\n").unwrap();
    std::fs::write(f.feature.join("untracked"), "untracked\n").unwrap();
    let before_status = git(&f.feature, &["status", "--porcelain"]);
    f.advance(false);
    let (result, statuses) = f.sync(&RecordingGit::default(), false, false).await;
    assert_eq!(result, ForkResult::Updated);
    assert!(matches!(
        worktree_status(&statuses, &f.feature),
        ForkStatus::Done
    ));
    f.assert_rebased();
    assert_eq!(git(&f.feature, &["status", "--porcelain"]), before_status);
    assert_eq!(git(&f.feature, &["show", ":base"]), "staged");
    assert_eq!(
        std::fs::read_to_string(f.feature.join("base")).unwrap(),
        "staged\nunstaged\n"
    );
    assert_eq!(
        std::fs::read_to_string(f.feature.join("untracked")).unwrap(),
        "untracked\n"
    );
    assert_eq!(
        git(&f.repo, &["stash", "list", "--format=%H"]),
        previous_stash
    );
}

#[tokio::test]
async fn rebase_conflict_aborts_and_restores_dirty_worktree() {
    let f = Fixture::new(false);
    let head = git(&f.feature, &["rev-parse", "HEAD"]);
    std::fs::write(f.feature.join("base"), "staged\n").unwrap();
    git(&f.feature, &["add", "base"]);
    std::fs::write(f.feature.join("base"), "staged\nunstaged\n").unwrap();
    std::fs::write(f.feature.join("untracked"), "keep me").unwrap();
    let status = git(&f.feature, &["status", "--porcelain"]);
    f.advance(true);
    let (_, updates) = f.sync(&RecordingGit::default(), false, false).await;
    assert!(
        matches!(worktree_status(&updates, &f.feature), ForkStatus::Error(s) if s.contains("aborted"))
    );
    assert_eq!(git(&f.feature, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&f.feature, &["status", "--porcelain"]), status);
    assert_eq!(git(&f.feature, &["show", ":base"]), "staged");
    assert_eq!(
        std::fs::read_to_string(f.feature.join("base")).unwrap(),
        "staged\nunstaged\n"
    );
    assert_eq!(
        std::fs::read_to_string(f.feature.join("untracked")).unwrap(),
        "keep me"
    );
    assert!(git(&f.repo, &["stash", "list"]).is_empty());
}

#[tokio::test]
async fn failed_or_noop_stash_never_rebases_or_restores_an_unrelated_stash() {
    for response in [Err("stash failed"), Ok("No local changes to save")] {
        let f = Fixture::new(false);
        std::fs::write(f.repo.join("base"), "user stash").unwrap();
        git(&f.repo, &["stash", "push", "-m", "user stash"]);
        let existing = git(&f.repo, &["stash", "list", "--format=%H"]);
        std::fs::write(f.feature.join("base"), "dirty").unwrap();
        let before = git(&f.feature, &["rev-parse", "HEAD"]);
        f.advance(false);
        let recorder = RecordingGit::default();
        recorder.override_git(&f.feature, &["stash", "push"], response);
        let (_, updates) = f.sync(&recorder, false, false).await;
        assert!(matches!(
            worktree_status(&updates, &f.feature),
            ForkStatus::Error(_)
        ));
        assert_eq!(git(&f.feature, &["rev-parse", "HEAD"]), before);
        assert_eq!(
            std::fs::read_to_string(f.feature.join("base")).unwrap(),
            "dirty"
        );
        assert_eq!(git(&f.repo, &["stash", "list", "--format=%H"]), existing);
        assert!(!recorder
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|(path, args)| path == &f.feature
                && (args[0] == "rebase"
                    || args.starts_with(&["stash".into(), "apply".into()])
                    || args.starts_with(&["stash".into(), "drop".into()]))));
    }
}

#[tokio::test]
async fn stash_restore_conflict_preserves_stash_for_recovery() {
    let f = Fixture::new(false);
    std::fs::write(f.feature.join("untracked"), "my untracked changes").unwrap();
    std::fs::write(f.upstream.join("untracked"), "upstream file").unwrap();
    f.advance(false);
    let (_, updates) = f.sync(&RecordingGit::default(), false, false).await;
    assert!(
        matches!(worktree_status(&updates, &f.feature), ForkStatus::Error(s) if s.contains("restore failed") && s.contains("kept in stash"))
    );
    assert_eq!(
        std::fs::read_to_string(f.feature.join("untracked")).unwrap(),
        "upstream file"
    );
    assert!(
        git(&f.feature, &["stash", "show", "--include-untracked", "-p"])
            .contains("my untracked changes")
    );
}

#[tokio::test]
async fn failed_abort_keeps_stash_without_applying_into_unfinished_rebase() {
    let f = Fixture::new(false);
    std::fs::write(f.feature.join("base"), "keep in stash").unwrap();
    f.advance(true);
    let recorder = RecordingGit::default();
    recorder.override_git(&f.feature, &["rebase", "--abort"], Err("abort failed"));
    let (_, updates) = f.sync(&recorder, false, false).await;
    assert!(
        matches!(worktree_status(&updates, &f.feature), ForkStatus::Error(s) if s.contains("abort failed") && s.contains("kept in stash"))
    );
    assert!(git(&f.feature, &["stash", "show", "-p"]).contains("keep in stash"));
    assert!(!recorder
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(path, args)| path == &f.feature
            && args.starts_with(&["stash".into(), "apply".into()])));
}

#[tokio::test]
async fn disabled_worktree_rebase_does_not_stash_dirty_changes() {
    let f = Fixture::new(false);
    std::fs::write(f.feature.join("base"), "do not stash").unwrap();
    f.advance(false);
    let recorder = RecordingGit::default();
    let (_, updates) = f.sync(&recorder, false, true).await;
    assert!(
        matches!(worktree_status(&updates, &f.feature), ForkStatus::Skipped(s) if s.contains("disabled"))
    );
    assert!(git(&f.repo, &["stash", "list"]).is_empty());
    assert!(!recorder
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(path, args)| path == &f.feature && args[0] == "stash"));
}
