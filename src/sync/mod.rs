pub mod cache;
pub mod discover;
pub mod native;

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::cli::SyncArgs;
use crate::config::{Config, UpstreamRepo};
use crate::ctx::Ctx;

/// `SyncArgs` + `Config` resolved into a runnable plan. Pure (no I/O).
#[derive(Debug)]
pub struct SyncPlan {
    pub repos: Vec<UpstreamRepo>,
    pub opts: native::SyncOpts,
    pub use_cache: bool,
}

/// Validate args, merge config defaults; user-facing error on bad combinations.
pub fn plan_sync(mut args: SyncArgs, config: &Config) -> Result<SyncPlan> {
    if args.sync_all && args.only.is_some() {
        anyhow::bail!("--sync-all and --only are mutually exclusive");
    }
    if args.rds_only && args.skip_rds_sync {
        anyhow::bail!("--rds-only and --skip-rds-sync are mutually exclusive");
    }

    if args.rds_only {
        args.skip_fork_sync = true;
    }

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

    let all_repos = &config.sync.upstream_repos;
    let repos: Vec<UpstreamRepo> = if args.sync_all {
        all_repos.clone()
    } else if let Some(ref names) = args.only {
        let mut selected = Vec::new();
        for name in names {
            match all_repos.iter().find(|r| &r.name == name) {
                Some(r) => selected.push(r.clone()),
                None => {
                    let valid: Vec<&str> = all_repos.iter().map(|r| r.name.as_str()).collect();
                    anyhow::bail!("Unknown repo '{}'. Valid names: {}", name, valid.join(", "));
                }
            }
        }
        selected
    } else {
        all_repos.iter().filter(|r| r.enabled).cloned().collect()
    };

    if repos.is_empty() {
        anyhow::bail!(
            "No repos selected for sync. Enable at least one in Settings → Repos, or pass --sync-all / --only <name>."
        );
    }

    let use_cache = config.sync.use_cache && !args.no_cache;
    let opts = native::SyncOpts {
        skip_fork_sync: args.skip_fork_sync,
        skip_git_fetch: args.skip_fetch,
        skip_rebase: args.skip_rebase,
        skip_rds_sync: args.skip_rds_sync,
        smart_sync: !args.no_smart_sync,
    };

    Ok(SyncPlan {
        repos,
        opts,
        use_cache,
    })
}

pub async fn run(ctx: &Ctx, args: SyncArgs) -> Result<()> {
    let config = crate::config::load(&ctx.paths)?;
    let plan = plan_sync(args, &config)?;

    let repo_refs: Vec<&UpstreamRepo> = plan.repos.iter().collect();
    let paths = resolve_repo_paths(&ctx.paths, &repo_refs, plan.use_cache)?;

    let repo_list: Vec<(String, PathBuf)> = plan
        .repos
        .iter()
        .filter_map(|r| paths.get(&r.name).map(|p| (r.name.clone(), p.clone())))
        .collect();
    if repo_list.is_empty() {
        anyhow::bail!(
            "Could not locate any enabled repos on disk. Ensure they are cloned under your home directory."
        );
    }
    let git: std::sync::Arc<dyn native::GitExec> = std::sync::Arc::new(native::SubprocessGit);
    native::run_headless(git, repo_list, plan.opts).await;
    Ok(())
}

fn resolve_repo_paths(
    paths: &crate::ctx::Paths,
    repos: &[&UpstreamRepo],
    use_cache: bool,
) -> Result<HashMap<String, PathBuf>> {
    let names: Vec<&str> = repos.iter().map(|r| r.name.as_str()).collect();

    if use_cache {
        if let Some(cached) = cache::load(paths) {
            if cache::is_valid(&cached, &names) {
                return Ok(cached.paths);
            }
            eprintln!("Cached repo paths are invalid, rediscovering...");
            cache::invalidate(paths);
        }
    }

    let all_repos: Vec<UpstreamRepo> = repos.iter().map(|r| (*r).clone()).collect();
    println!("Discovering git repositories (this may take a moment)...");
    let discovered = discover::discover_and_cache(paths, &all_repos)?;
    println!("All repositories located:");
    for (project, path) in &discovered.paths {
        println!("  {project}: {}", path.display());
    }
    Ok(discovered.paths)
}
