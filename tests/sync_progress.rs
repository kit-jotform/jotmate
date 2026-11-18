mod common;

use common::TestCtx;
use jotmate::ctx::Ctx;
use jotmate::tui::app::{App, ForkStatus, RdsStatus, SyncUpdate, WorktreeSyncState};
use std::path::Path;

fn app(tc: &TestCtx) -> App {
    App::new(Ctx {
        paths: tc.ctx.paths.clone(),
        keychain: tc.ctx.keychain.clone(),
        http: tc.ctx.http.clone(),
    })
    .unwrap()
}

fn worktree(path: &Path, status: ForkStatus) -> SyncUpdate {
    SyncUpdate::Worktree(
        0,
        WorktreeSyncState {
            path: path.to_path_buf(),
            branch: Some("feature/linked".into()),
            status,
        },
    )
}

#[test]
fn worktree_progress_is_separate_and_completion_waits_for_restore() {
    let tc = TestCtx::new();
    let mut app = app(&tc);
    let path = tc.paths().config_file().with_file_name("linked");
    let _tx = app.start_sync(vec![("frontend".into(), path.with_file_name("main"))]);
    app.apply_sync_update(SyncUpdate::Fork(0, ForkStatus::Done));
    app.apply_sync_update(SyncUpdate::Rds(0, RdsStatus::Done));
    app.apply_sync_update(worktree(&path, ForkStatus::Pending));
    assert!(!app.sync_is_complete());
    for status in [
        ForkStatus::Stashing,
        ForkStatus::Rebasing,
        ForkStatus::Unstashing,
    ] {
        app.apply_sync_update(worktree(&path, status));
        assert!(!app.sync_is_complete());
        assert_eq!(app.sync_state.as_ref().unwrap().repos[0].worktrees.len(), 1);
    }
    app.apply_sync_update(worktree(&path, ForkStatus::Done));
    assert!(app.sync_is_complete());
    let repo = &app.sync_state.as_ref().unwrap().repos[0];
    assert!(matches!(repo.fork_status, ForkStatus::Done));
    assert_eq!(repo.row_count(), 3);
    assert!(!repo.has_error());
}

#[test]
fn worktree_error_and_skip_do_not_overwrite_successful_fork_sync() {
    let tc = TestCtx::new();
    let mut app = app(&tc);
    let path = tc.paths().config_file().with_file_name("linked");
    let _tx = app.start_sync(vec![("backend".into(), path.with_file_name("main"))]);
    app.apply_sync_update(SyncUpdate::Fork(0, ForkStatus::Done));
    app.apply_sync_update(SyncUpdate::Rds(
        0,
        RdsStatus::Skipped("--skip-rds-sync".into()),
    ));
    app.apply_sync_update(worktree(&path, ForkStatus::Skipped("locked".into())));
    let repo = &app.sync_state.as_ref().unwrap().repos[0];
    assert!(matches!(repo.fork_status, ForkStatus::Done));
    assert!(!repo.is_skipped());
    assert!(!repo.has_error());
    assert_eq!(repo.row_count(), 3);
    app.apply_sync_update(worktree(
        &path,
        ForkStatus::Error("restore failed; stash retained".into()),
    ));
    assert!(app.sync_is_complete());
    let repo = &app.sync_state.as_ref().unwrap().repos[0];
    assert!(repo.has_error());
    assert!(matches!(repo.fork_status, ForkStatus::Done));
}

#[test]
fn up_to_date_main_with_rebased_worktree_is_not_counted_as_skipped() {
    let tc = TestCtx::new();
    let mut app = app(&tc);
    let path = tc.paths().config_file().with_file_name("linked");
    let _tx = app.start_sync(vec![("frontend".into(), path.with_file_name("main"))]);
    app.apply_sync_update(SyncUpdate::Fork(0, ForkStatus::UpToDate));
    app.apply_sync_update(SyncUpdate::Rds(0, RdsStatus::Skipped("no changes".into())));
    app.apply_sync_update(worktree(&path, ForkStatus::Done));
    assert!(!app.sync_state.as_ref().unwrap().repos[0].is_skipped());
}
