//! Single source of truth for "can we start a sync?" — every failure path
//! routes through `App::fail_sync_setup` so the user always sees a
//! SyncProgress screen with a reason, never a silent no-op on the main menu.

use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::oneshot;

use crate::config::UpstreamRepo;
use crate::tui::app::App;
use crate::tui::sync_state::DiscoveryResult;

pub(super) fn launch_sync(app: &mut App) {
    let config = match crate::config::load() {
        Ok(c) => c,
        Err(e) => {
            app.fail_sync_setup(format!("Could not load config: {e}"));
            return;
        }
    };

    let enabled: Vec<UpstreamRepo> = config
        .sync
        .upstream_repos
        .iter()
        .filter(|r| r.enabled)
        .cloned()
        .collect();
    if enabled.is_empty() {
        app.fail_sync_setup(
            "No repos enabled for sync. Add or enable at least one in Settings → Repos.",
        );
        return;
    }

    let enabled_names: Vec<&str> = enabled.iter().map(|r| r.name.as_str()).collect();
    let cached_paths = if config.sync.use_cache {
        crate::sync::cache::load()
            .filter(|c| crate::sync::cache::is_valid(c, &enabled_names))
            .map(|c| c.paths)
    } else {
        None
    };

    match cached_paths {
        Some(paths) => start_real_sync(app, &enabled, paths),
        None => spawn_discovery(app, enabled),
    }
}

pub(super) fn promote_discovery_if_ready(app: &mut App) {
    let Some(result) = app.take_discovery_result() else {
        return;
    };
    match result {
        Ok((enabled, paths)) => start_real_sync(app, &enabled, paths),
        Err(msg) => app.fail_sync_setup(format!("Repo discovery failed: {msg}")),
    }
}

fn spawn_discovery(app: &mut App, enabled: Vec<UpstreamRepo>) {
    let (tx, rx) = oneshot::channel::<DiscoveryResult>();
    app.enter_sync_discovering_with(rx);

    tokio::task::spawn_blocking(move || {
        let result = crate::sync::discover::discover_and_cache_quiet(&enabled)
            .map(|cache| (enabled, cache.paths))
            .map_err(|e| e.to_string());
        let _ = tx.send(result);
    });
}

fn start_real_sync(app: &mut App, enabled: &[UpstreamRepo], paths: HashMap<String, PathBuf>) {
    // Preserve config order so the UI list is stable across runs.
    let ordered: Vec<(String, PathBuf)> = enabled
        .iter()
        .filter_map(|r| paths.get(&r.name).map(|p| (r.name.clone(), p.clone())))
        .collect();

    if ordered.is_empty() {
        app.fail_sync_setup(
            "Could not locate any enabled repos on disk. Check that they are cloned under your home directory.",
        );
        return;
    }

    let tx = app.start_sync(ordered.clone());

    let repo_list: Vec<(usize, PathBuf)> = ordered
        .into_iter()
        .enumerate()
        .map(|(idx, (_, path))| (idx, path))
        .collect();

    let opts = crate::sync::native::SyncOpts {
        skip_fork_sync: app.sync.skip_fork_sync,
        skip_git_fetch: false,
        skip_rebase: app.sync.skip_rebase,
        skip_rds_sync: app.sync.skip_rds_sync,
        smart_sync: app.sync.smart_sync,
    };

    let handle = tokio::spawn(async move {
        crate::sync::native::run_tui(repo_list, tx, opts).await;
    });

    if let Some(state) = &mut app.sync_state {
        state.sync_handle = Some(handle);
    }
}
