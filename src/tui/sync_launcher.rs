//! Launches a background sync task for the currently-enabled upstream repos
//! and wires its progress channel into the `App`'s sync state.

use tokio::sync::mpsc;

use crate::tui::app::App;

pub(super) fn launch_sync(app: &mut App) {
    let Ok(config) = crate::config::load() else {
        return;
    };

    let enabled: Vec<_> = config
        .sync
        .upstream_repos
        .iter()
        .filter(|r| r.enabled)
        .collect();
    if enabled.is_empty() {
        return;
    }

    // Resolve repo paths from the cache. We can't discover inside the TUI
    // (fd is slow + prints), so bail if the cache is missing or invalid.
    let paths = {
        if !config.sync.use_cache {
            return;
        }
        let Some(cached) = crate::sync::cache::load() else {
            return;
        };
        let enabled_names: Vec<&str> = enabled.iter().map(|r| r.name.as_str()).collect();
        if !crate::sync::cache::is_valid(&cached, &enabled_names) {
            return;
        }
        cached.paths
    };

    let (tx, rx) = mpsc::unbounded_channel();
    app.start_sync(paths.clone(), rx);

    // Build repo list with indices matching sync_state.repos order
    let repo_list: Vec<(usize, std::path::PathBuf)> = app
        .sync_state
        .as_ref()
        .unwrap()
        .repos
        .iter()
        .enumerate()
        .map(|(idx, r)| (idx, r.path.clone()))
        .collect();

    let opts = crate::sync::native::SyncOpts {
        skip_fork_sync: app.skip_fork_sync,
        skip_git_fetch: false,
        skip_rebase: app.skip_rebase,
        skip_rds_sync: app.skip_rds_sync,
        smart_sync: app.smart_sync,
    };

    let handle = tokio::spawn(async move {
        crate::sync::native::run_tui(repo_list, tx, opts).await;
    });

    if let Some(state) = &mut app.sync_state {
        state.sync_handle = Some(handle);
    }
}
