pub mod cache;
pub mod discover;
pub mod native;
pub mod runner;

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::cli::SyncArgs;
use crate::config::UpstreamRepo;
use cache::compute_github_base;

pub async fn run(mut args: SyncArgs) -> Result<()> {
    // Deprecated flag warnings
    if args.skip_dirty_sync {
        eprintln!("Warning: --skip-dirty-sync is deprecated and has no effect.");
    }

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
                    anyhow::bail!(
                        "Unknown repo '{}'. Valid names: {}",
                        name,
                        valid.join(", ")
                    );
                }
            }
        }
        selected
    } else {
        all_repos.iter().filter(|r| r.enabled).collect()
    };

    let use_cache = config.sync.use_cache && !args.no_cache;
    let paths = resolve_repo_paths(repos_to_sync.as_slice(), use_cache)?;
    let github_base = compute_github_base(&paths).ok_or_else(|| {
        anyhow::anyhow!(
            "Repositories do not share a common parent directory. \
             Please ensure all repos are cloned under the same directory."
        )
    })?;

    runner::run_cli(&args, &github_base)
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
