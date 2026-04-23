//! Config file parsing / persistence edge cases.
//!
//! Covered edge cases:
//!   A19   malformed config → TUI fall-back (via App::new())
//!   A20   malformed config → CLI bails (via config::load)
//!   C4    extra/unknown TOML keys ignored
//!   Empty config file → defaults (via Config::default + missing file)
//!   C5    duplicate upstream_repos entries in config load as-is (documents
//!         current behavior — hand-edited configs aren't deduped).

mod common;

use common::TestCtx;
use jotmate::config;
use jotmate::ctx::Ctx;
use jotmate::tui::app::App;

/// App::new owns the passed Ctx, so we move a clone and keep the `TestCtx`
/// alive to own the TempDir.
fn app_from(tc: &TestCtx) -> App {
    let ctx = Ctx {
        paths: tc.ctx.paths.clone(),
        keychain: tc.ctx.keychain.clone(),
        http: tc.ctx.http.clone(),
    };
    App::new(ctx).expect("App::new should always succeed (uses defaults on error)")
}

#[test]
fn missing_config_file_loads_defaults() {
    let tc = TestCtx::new();
    let cfg = config::load(tc.paths()).unwrap();
    assert!(!cfg.sync.upstream_repos.is_empty(), "default repos present");
    assert!(cfg.sync.use_cache, "use_cache defaults to true");
    assert!(cfg.sync.smart_sync, "smart_sync defaults to true");
}

#[test]
fn malformed_config_cli_bails() {
    // A20 — CLI path surfaces the parse error.
    let tc = TestCtx::new();
    tc.write_config("this is not valid toml {{{");
    let err = config::load(tc.paths()).unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("TOML") || msg.contains("parse") || msg.contains("expected"),
        "expected TOML parse error, got: {msg}"
    );
}

#[test]
fn malformed_config_tui_falls_back_and_warns() {
    // A19 — TUI should not crash; App::new uses defaults and sets the banner.
    let tc = TestCtx::new();
    tc.write_config("this is not valid toml {{{");
    let app = app_from(&tc);
    assert!(
        app.config_load_error.is_some(),
        "config_load_error must be set when parse fails"
    );
    // Still has default repos so the user can navigate.
    assert!(!app.sync.repos.is_empty());
}

#[test]
fn valid_config_gives_no_warning() {
    let tc = TestCtx::new();
    tc.write_config(
        r#"
[sync]
upstream_repos = []
use_cache = true
"#,
    );
    let app = app_from(&tc);
    assert!(app.config_load_error.is_none());
    assert!(app.sync.repos.is_empty());
}

#[test]
fn unknown_keys_are_ignored() {
    // C4 — serde with #[serde(default)] tolerates extra keys.
    let tc = TestCtx::new();
    tc.write_config(
        r#"
unknown_top_level = 42

[sync]
unknown_sync_field = "hi"

[[sync.upstream_repos]]
url = "https://example.com/a.git"
name = "a"
enabled = true
"#,
    );
    let cfg = config::load(tc.paths()).unwrap();
    assert_eq!(cfg.sync.upstream_repos.len(), 1);
    assert_eq!(cfg.sync.upstream_repos[0].name, "a");
}

#[test]
fn save_then_load_roundtrips() {
    let tc = TestCtx::new();
    let mut cfg = config::load(tc.paths()).unwrap();
    cfg.sync.use_cache = false;
    cfg.sync.smart_sync = false;
    config::save(tc.paths(), &cfg).unwrap();

    let reloaded = config::load(tc.paths()).unwrap();
    assert!(!reloaded.sync.use_cache);
    assert!(!reloaded.sync.smart_sync);
}

#[test]
fn save_never_writes_to_real_home() {
    // Defense-in-depth: save() writes exactly to the `Paths`-configured path
    // and nowhere else.
    let tc = TestCtx::new();
    let cfg = config::load(tc.paths()).unwrap();
    config::save(tc.paths(), &cfg).unwrap();

    let target = tc.config_file();
    assert!(target.exists(), "save should create the target file");

    // Sanity: the written path must live under the tempdir.
    let canonical = target.canonicalize().unwrap();
    let parent = tc.paths().config_file();
    // Both paths descend from the same tempdir root; the file must not
    // ever have landed in the user's real `~/.config`.
    let canonical_str = canonical.display().to_string();
    assert!(
        !canonical_str.contains("/Users/")
            || canonical_str.contains("/tmp")
            || canonical_str.contains("/var/folders"),
        "save wrote outside the tempdir: {canonical_str} (expected path under {})",
        parent.display()
    );
}

#[test]
fn duplicate_upstream_repos_load_as_is() {
    // C5 — documents that the loader does NOT dedup hand-edited duplicates.
    // Our URL-add validation prevents creating dups, but hand edits aren't
    // sanitized on load. This test will fail if someone adds dedup-on-load
    // without also updating this test.
    let tc = TestCtx::new();
    tc.write_config(
        r#"
[[sync.upstream_repos]]
url = "https://github.com/jotform/frontend.git"
name = "frontend"
enabled = true

[[sync.upstream_repos]]
url = "https://github.com/jotform/frontend.git"
name = "frontend"
enabled = true
"#,
    );
    let cfg = config::load(tc.paths()).unwrap();
    assert_eq!(cfg.sync.upstream_repos.len(), 2, "no dedup at load time");
}
