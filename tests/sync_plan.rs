//! Tests for `sync::plan_sync` — the pure arg + config → plan function that
//! `sync::run` wraps. Exercises all argument validation without spawning git.
//!
//! Covered edge cases:
//!   A1/A2  empty repo selection → bail
//!   A3     unknown --only name → bail with valid names
//!   A4     --sync-all + --only mutually exclusive
//!   A5     --rds-only + --skip-rds-sync mutually exclusive
//!   --rds-only implies --skip-fork-sync
//!   Config defaults merged into args (skip_fork_sync, smart_sync, etc.)
//!   --sync-all includes disabled repos
//!   plain default picks only enabled repos

use jotmate::cli::SyncArgs;
use jotmate::config::{Config, UpstreamRepo};
use jotmate::sync::plan_sync;

fn upstream(name: &str, enabled: bool) -> UpstreamRepo {
    UpstreamRepo {
        url: format!("https://github.com/test/{name}.git"),
        name: name.to_string(),
        enabled,
    }
}

fn config_with(repos: Vec<UpstreamRepo>) -> Config {
    let mut cfg = Config::default();
    cfg.sync.upstream_repos = repos;
    cfg
}

#[test]
fn sync_all_and_only_are_mutually_exclusive() {
    // A4
    let cfg = config_with(vec![upstream("a", true)]);
    let args = SyncArgs {
        sync_all: true,
        only: Some(vec!["a".into()]),
        ..Default::default()
    };
    let err = plan_sync(args, &cfg).unwrap_err();
    assert!(err.to_string().contains("mutually exclusive"));
}

#[test]
fn rds_only_and_skip_rds_sync_are_mutually_exclusive() {
    // A5
    let cfg = config_with(vec![upstream("a", true)]);
    let args = SyncArgs {
        rds_only: true,
        skip_rds_sync: true,
        ..Default::default()
    };
    let err = plan_sync(args, &cfg).unwrap_err();
    assert!(err.to_string().contains("mutually exclusive"));
}

#[test]
fn empty_selection_bails() {
    // A1/A2 — all repos disabled, no --only, no --sync-all.
    let cfg = config_with(vec![upstream("a", false), upstream("b", false)]);
    let args = SyncArgs::default();
    let err = plan_sync(args, &cfg).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("No repos selected"),
        "expected 'No repos selected', got: {msg}"
    );
}

#[test]
fn unknown_only_name_bails_with_valid_names() {
    // A3
    let cfg = config_with(vec![upstream("frontend", true), upstream("backend", true)]);
    let args = SyncArgs {
        only: Some(vec!["bogus".into()]),
        ..Default::default()
    };
    let err = plan_sync(args, &cfg).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Unknown repo 'bogus'"), "got: {msg}");
    assert!(msg.contains("frontend"), "got: {msg}");
    assert!(msg.contains("backend"), "got: {msg}");
}

#[test]
fn rds_only_implies_skip_fork_sync() {
    let cfg = config_with(vec![upstream("a", true)]);
    let args = SyncArgs {
        rds_only: true,
        ..Default::default()
    };
    let plan = plan_sync(args, &cfg).unwrap();
    assert!(plan.opts.skip_fork_sync);
}

#[test]
fn default_includes_only_enabled_repos() {
    let cfg = config_with(vec![
        upstream("a", true),
        upstream("b", false),
        upstream("c", true),
    ]);
    let plan = plan_sync(SyncArgs::default(), &cfg).unwrap();
    let names: Vec<&str> = plan.repos.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, &["a", "c"]);
}

#[test]
fn sync_all_includes_disabled_repos() {
    let cfg = config_with(vec![upstream("a", true), upstream("b", false)]);
    let args = SyncArgs {
        sync_all: true,
        ..Default::default()
    };
    let plan = plan_sync(args, &cfg).unwrap();
    let names: Vec<&str> = plan.repos.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, &["a", "b"]);
}

#[test]
fn only_picks_named_repos_even_if_disabled() {
    let cfg = config_with(vec![upstream("a", false), upstream("b", false)]);
    let args = SyncArgs {
        only: Some(vec!["a".into()]),
        ..Default::default()
    };
    let plan = plan_sync(args, &cfg).unwrap();
    let names: Vec<&str> = plan.repos.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, &["a"]);
}

#[test]
fn config_defaults_merge_into_args_skip_fork_sync() {
    let mut cfg = config_with(vec![upstream("a", true)]);
    cfg.sync.skip_fork_sync = true;
    let plan = plan_sync(SyncArgs::default(), &cfg).unwrap();
    assert!(plan.opts.skip_fork_sync);
}

#[test]
fn config_disabled_smart_sync_propagates() {
    let mut cfg = config_with(vec![upstream("a", true)]);
    cfg.sync.smart_sync = false;
    let plan = plan_sync(SyncArgs::default(), &cfg).unwrap();
    assert!(!plan.opts.smart_sync);
}

#[test]
fn cli_no_cache_flag_overrides_use_cache() {
    let mut cfg = config_with(vec![upstream("a", true)]);
    cfg.sync.use_cache = true;
    let args = SyncArgs {
        no_cache: true,
        ..Default::default()
    };
    let plan = plan_sync(args, &cfg).unwrap();
    assert!(!plan.use_cache);
}

#[test]
fn config_use_cache_false_survives_default_args() {
    let mut cfg = config_with(vec![upstream("a", true)]);
    cfg.sync.use_cache = false;
    let plan = plan_sync(SyncArgs::default(), &cfg).unwrap();
    assert!(!plan.use_cache);
}
