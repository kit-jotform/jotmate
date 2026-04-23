pub mod cache;
pub mod discover;
pub mod native;

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::cli::SyncArgs;
use crate::config::UpstreamRepo;

pub async fn run(mut args: SyncArgs) -> Result<()> {
    // Conflict checks
    if args.sync_all && args.only.is_some() {
        anyhow::bail!("--sync-all and --only are mutually exclusive");
    }
    if args.rds_only && args.skip_rds_sync {
        anyhow::bail!("--rds-only and --skip-rds-sync are mutually exclusive");
    }

    // --rds-only implies --skip-fork-sync
    if args.rds_only {
        args.skip_fork_sync = true;
    }

    let config = crate::config::load()?;

    // Merge config defaults into args (CLI flags override config)
    if config.sync.skip_fork_sync && !args.skip_fork_sync {
        args.skip_fork_sync = true;
    }
    if config.sync.skip_rebase && !args.skip_rebase {
        args.skip_rebase = true;
    }
    if config.sync.skip_rds_sync && !args.skip_rds_sync {
        args.skip_rds_sync = true;
    }
    if !config.sync.smart_sync && !args.no_smart_sync {
        args.no_smart_sync = true;
    }

    // Resolve which repos to sync
    let all_repos = &config.sync.upstream_repos;
    let repos_to_sync: Vec<&UpstreamRepo> = if args.sync_all {
        all_repos.iter().collect()
    } else if let Some(ref names) = args.only {
        let mut selected = Vec::new();
        for name in names {
            match all_repos.iter().find(|r| &r.name == name) {
                Some(r) => selected.push(r),
                None => {
                    let valid: Vec<&str> = all_repos.iter().map(|r| r.name.as_str()).collect();
                    anyhow::bail!("Unknown repo '{}'. Valid names: {}", name, valid.join(", "));
                }
            }
        }
        selected
    } else {
        all_repos.iter().filter(|r| r.enabled).collect()
    };

    if repos_to_sync.is_empty() {
        anyhow::bail!(
            "No repos selected for sync. Enable at least one in Settings → Repos, or pass --sync-all / --only <name>."
        );
    }

    let use_cache = config.sync.use_cache && !args.no_cache;
    let paths = resolve_repo_paths(repos_to_sync.as_slice(), use_cache)?;

    let repo_list: Vec<(String, PathBuf)> = repos_to_sync
        .iter()
        .filter_map(|r| paths.get(&r.name).map(|p| (r.name.clone(), p.clone())))
        .collect();
    if repo_list.is_empty() {
        anyhow::bail!(
            "Could not locate any enabled repos on disk. Ensure they are cloned under your home directory."
        );
    }
    let opts = native::SyncOpts {
        skip_fork_sync: args.skip_fork_sync,
        skip_git_fetch: args.skip_fetch,
        skip_rebase: args.skip_rebase,
        skip_rds_sync: args.skip_rds_sync,
        smart_sync: !args.no_smart_sync,
    };
    native::run_headless(repo_list, opts).await;
    Ok(())
}

fn resolve_repo_paths(
    repos: &[&UpstreamRepo],
    use_cache: bool,
) -> Result<HashMap<String, PathBuf>> {
    let names: Vec<&str> = repos.iter().map(|r| r.name.as_str()).collect();

    if use_cache {
        if let Some(cached) = cache::load() {
            if cache::is_valid(&cached, &names) {
                return Ok(cached.paths);
            }
            eprintln!("Cached repo paths are invalid, rediscovering...");
            cache::invalidate();
        }
    }

    let all_repos: Vec<UpstreamRepo> = repos.iter().map(|r| (*r).clone()).collect();
    let discovered = discover::discover_and_cache(&all_repos)?;
    Ok(discovered.paths)
}
