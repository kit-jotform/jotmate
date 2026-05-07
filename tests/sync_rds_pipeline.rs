//! Sync RDS-pipeline tests driven by a scripted `FakeGit`.
//!
//! Covered edge cases:
//!   skip_rds_sync short-circuits (with or without ./sync script)
//!   smart_sync skips when fork unchanged + clean + ahead=0
//!   smart_sync proceeds when dirty tree
//!   smart_sync pulls when behind=N, ahead=0
//!   smart_sync pull failure reports via AlreadyReported path
//!   A14  no ./sync script in repo → Skipped
//!   A13  ./sync exits non-zero → Error with stderr
//!   IP-denied stderr → RdsStatus::IpDenied
//!   smart-sync recovery via cached HEAD SHA

mod common;

use common::{fake_repo_dir, FakeGit};
use jotmate::sync::native::fork::ForkResult;
use jotmate::sync::native::rds::{sync_rds, RdsOpts};
use jotmate::sync::native::RdsStateCache;
use jotmate::tui::app::{RdsStatus, SyncUpdate};
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::mpsc;

fn collect_rds_updates(rx: &mut mpsc::UnboundedReceiver<SyncUpdate>) -> Vec<RdsStatus> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let SyncUpdate::Rds(_, s) = msg {
            out.push(s);
        }
    }
    out
}

fn fresh_state() -> (TempDir, Arc<RdsStateCache>) {
    let td = TempDir::new().unwrap();
    let cache = Arc::new(RdsStateCache::load(td.path().join("rds_state.json")));
    (td, cache)
}

fn state_with(repo: &Path, sha: &str) -> (TempDir, Arc<RdsStateCache>) {
    let (td, cache) = fresh_state();
    cache.record_synced(repo, sha);
    (td, cache)
}

fn opts(skip_rds_sync: bool, smart_sync: bool, rds_state: Arc<RdsStateCache>) -> RdsOpts {
    RdsOpts {
        skip_rds_sync,
        smart_sync,
        rds_state,
    }
}

fn last_terminal(updates: &[RdsStatus]) -> &RdsStatus {
    updates
        .iter()
        .rev()
        .find(|s| s.is_terminal())
        .unwrap_or_else(|| panic!("no terminal status in {updates:?}"))
}

#[tokio::test]
async fn skip_rds_sync_short_circuits() {
    let git = FakeGit::new();
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(true, false, state),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Skipped(msg) => assert_eq!(msg, "--skip-rds-sync"),
        other => panic!("expected Skipped, got {other:?}"),
    }
    assert!(git.log().is_empty());
}

#[tokio::test]
async fn no_sync_script_is_skipped() {
    let git = FakeGit::new();
    let td = fake_repo_dir(false);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Updated,
        &tx,
        &opts(false, true, state),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Skipped(msg) => assert_eq!(msg, "no ./sync"),
        other => panic!("expected Skipped(no ./sync), got {other:?}"),
    }
}

#[tokio::test]
async fn smart_sync_skips_when_clean_and_ahead_zero_and_sha_matches_cache() {
    let git = FakeGit::new()
        .on(&["status"], Ok("# branch.head main\n# branch.ab +0 -0\n"))
        .on(&["rev-parse", "HEAD"], Ok("deadbeef"));
    let td = fake_repo_dir(true);
    let (_state_td, state) = state_with(td.path(), "deadbeef");
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true, state),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Skipped(msg) => assert_eq!(msg, "no changes"),
        other => panic!("expected Skipped(no changes), got {other:?}"),
    }
    assert_eq!(git.call_count(&["__rds_script__"]), 0);
}

#[tokio::test]
async fn smart_sync_proceeds_when_cache_missing() {
    let git = FakeGit::new()
        .on(&["status"], Ok("# branch.head main\n# branch.ab +0 -0\n"))
        .on(&["rev-parse", "HEAD"], Ok("deadbeef"));
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true, state),
    )
    .await;

    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}

#[tokio::test]
async fn smart_sync_proceeds_when_cached_sha_stale() {
    let git = FakeGit::new()
        .on(&["status"], Ok("# branch.head main\n# branch.ab +0 -0\n"))
        .on(&["rev-parse", "HEAD"], Ok("newer"));
    let td = fake_repo_dir(true);
    let (_state_td, state) = state_with(td.path(), "older");
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true, state),
    )
    .await;

    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}

#[tokio::test]
async fn successful_rds_sync_records_head_in_cache() {
    let git = FakeGit::new().on(&["rev-parse", "HEAD"], Ok("freshsha"));
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Updated,
        &tx,
        &opts(false, true, state.clone()),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
    assert_eq!(
        state.last_synced_sha(td.path()).as_deref(),
        Some("freshsha")
    );
}

#[tokio::test]
async fn rds_failure_does_not_record_cache() {
    let git = FakeGit::new();
    git.set_rds_error("build broke");
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Updated,
        &tx,
        &opts(false, true, state.clone()),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Error(msg) => assert_eq!(msg, "build broke"),
        other => panic!("expected Error, got {other:?}"),
    }
    assert_eq!(state.last_synced_sha(td.path()), None);
}

#[tokio::test]
async fn smart_sync_proceeds_when_dirty() {
    let git = FakeGit::new().on(
        &["status"],
        Ok("# branch.head main\n1 M. N... 100644 100644 100644 a a a README.md\n"),
    );
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true, state),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
    assert_eq!(git.call_count(&["__rds_script__"]), 1);
}

#[tokio::test]
async fn smart_sync_pulls_when_behind() {
    let git = FakeGit::new()
        .on(&["status"], Ok("# branch.head main\n# branch.ab +0 -3\n"))
        .on(&["pull", "--ff-only", "origin", "main"], Ok("Fast-forward"));
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true, state),
    )
    .await;

    assert!(git.was_called(&["pull", "--ff-only", "origin", "main"]));
    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}

#[tokio::test]
async fn smart_sync_pull_failure_reports_and_stops() {
    let git = FakeGit::new()
        .on(&["status"], Ok("# branch.head main\n# branch.ab +0 -3\n"))
        .on(
            &["pull", "--ff-only", "origin", "main"],
            Err("Aborting: not possible to fast-forward"),
        );
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true, state),
    )
    .await;

    assert_eq!(git.call_count(&["__rds_script__"]), 0);
    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Error(msg) => assert!(msg.starts_with("pull:"), "got: {msg}"),
        other => panic!("expected Error(pull:…), got {other:?}"),
    }
}

#[tokio::test]
async fn smart_sync_proceeds_when_ab_missing() {
    let git = FakeGit::new()
        .on(&["status"], Ok("# branch.head main\n"))
        .on(&["fetch", "origin", "main"], Ok(""));
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true, state),
    )
    .await;

    assert!(git.was_called(&["fetch", "origin", "main"]));
    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}

#[tokio::test]
async fn smart_sync_proceeds_on_detached_head() {
    let git = FakeGit::new().on(&["status"], Ok("# branch.head (detached)\n"));
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true, state),
    )
    .await;

    assert_eq!(git.call_count(&["pull"]), 0);
    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}

#[tokio::test]
async fn smart_sync_proceeds_when_branch_ahead_of_default() {
    let git = FakeGit::new()
        .on(
            &["status"],
            Ok("# branch.head feature\n# branch.ab +0 -0\n"),
        )
        .on(
            &["symbolic-ref", "refs/remotes/upstream/HEAD"],
            Ok("refs/remotes/upstream/master"),
        )
        .on(&["rev-list", "--count", "master..HEAD"], Ok("1"));
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true, state),
    )
    .await;

    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}

#[tokio::test]
async fn smart_sync_skips_when_on_default_branch_clean_and_sha_matches() {
    let git = FakeGit::new()
        .on(&["status"], Ok("# branch.head master\n# branch.ab +0 -0\n"))
        .on(
            &["symbolic-ref", "refs/remotes/upstream/HEAD"],
            Ok("refs/remotes/upstream/master"),
        )
        .on(&["rev-parse", "HEAD"], Ok("matchsha"));
    let td = fake_repo_dir(true);
    let (_state_td, state) = state_with(td.path(), "matchsha");
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true, state),
    )
    .await;

    assert_eq!(git.call_count(&["rev-list"]), 0);
    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Skipped(msg) => assert_eq!(msg, "no changes"),
        other => panic!("expected Skipped(no changes), got {other:?}"),
    }
    assert_eq!(git.call_count(&["__rds_script__"]), 0);
}

#[tokio::test]
async fn smart_sync_skips_when_branch_matches_default_commits_and_sha_matches() {
    let git = FakeGit::new()
        .on(
            &["status"],
            Ok("# branch.head feature\n# branch.ab +0 -0\n"),
        )
        .on(
            &["symbolic-ref", "refs/remotes/upstream/HEAD"],
            Ok("refs/remotes/upstream/master"),
        )
        .on(&["rev-list", "--count", "master..HEAD"], Ok("0"))
        .on(&["rev-parse", "HEAD"], Ok("matchsha"));
    let td = fake_repo_dir(true);
    let (_state_td, state) = state_with(td.path(), "matchsha");
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true, state),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Skipped(msg) => assert_eq!(msg, "no changes"),
        other => panic!("expected Skipped(no changes), got {other:?}"),
    }
    assert_eq!(git.call_count(&["__rds_script__"]), 0);
}

#[tokio::test]
async fn sync_script_failure_reports_error() {
    let git = FakeGit::new();
    git.set_rds_error("build step failed");
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Updated,
        &tx,
        &opts(false, true, state),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Error(msg) => assert_eq!(msg, "build step failed"),
        other => panic!("expected Error(build step failed), got {other:?}"),
    }
}

#[tokio::test]
async fn ip_denied_surfaces_as_ip_denied_status() {
    let git = FakeGit::new();
    git.set_rds_ip_denied("Permission denied (publickey).");
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Updated,
        &tx,
        &opts(false, true, state),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::IpDenied(detail) => {
            assert!(
                detail.contains("Permission denied"),
                "detail should preserve the matching line, got: {detail}"
            );
        }
        other => panic!("expected IpDenied, got {other:?}"),
    }
}

#[tokio::test]
async fn smart_sync_off_runs_script_unconditionally() {
    let git = FakeGit::new();
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, false, state),
    )
    .await;

    assert_eq!(git.call_count(&["status"]), 0);
    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}

#[tokio::test]
async fn fork_error_still_runs_script_when_smart_sync_enabled() {
    let git = FakeGit::new();
    let td = fake_repo_dir(true);
    let (_state_td, state) = fresh_state();
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Error("something".into()),
        &tx,
        &opts(false, true, state),
    )
    .await;

    assert_eq!(git.call_count(&["status"]), 0);
    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}
