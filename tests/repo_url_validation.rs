//! Tests for `App::add_repo_from_input` URL handling.
//!
//! Covered edge cases:
//!   A21   cross-host name collision (derived name `api` already exists)
//!   A22   dedup between `.../frontend` and `.../frontend.git` forms
//!   A23   empty / whitespace URL rejected
//!   A24   URL with no path segment rejected

mod common;

use common::TestCtx;
use jotmate::ctx::Ctx;
use jotmate::tui::app::App;

fn empty_app(tc: &TestCtx) -> App {
    // Start from an empty repo list so existing-URL/name collisions come
    // entirely from what the test adds.
    tc.write_config(
        r#"
[sync]
upstream_repos = []
"#,
    );
    let ctx = Ctx {
        paths: tc.ctx.paths.clone(),
        keychain: tc.ctx.keychain.clone(),
        http: tc.ctx.http.clone(),
    };
    App::new(ctx).unwrap()
}

#[test]
fn accepts_fresh_url() {
    let tc = TestCtx::new();
    let mut app = empty_app(&tc);
    app.add_repo_from_input("https://github.com/jotform/frontend.git".into());
    assert_eq!(app.sync.repos.len(), 1);
    assert_eq!(app.sync.repos[0].name, "frontend");
}

#[test]
fn normalizes_trailing_dotgit() {
    let tc = TestCtx::new();
    let mut app = empty_app(&tc);
    app.add_repo_from_input("https://github.com/jotform/frontend.git".into());
    // A22 — second form with no .git must be rejected as duplicate.
    app.add_repo_from_input("https://github.com/jotform/frontend".into());
    assert_eq!(app.sync.repos.len(), 1);
}

#[test]
fn normalizes_trailing_slash() {
    let tc = TestCtx::new();
    let mut app = empty_app(&tc);
    app.add_repo_from_input("https://github.com/jotform/frontend".into());
    app.add_repo_from_input("https://github.com/jotform/frontend/".into());
    assert_eq!(app.sync.repos.len(), 1);
}

#[test]
fn rejects_cross_host_name_collision() {
    // A21 — both URLs derive name `frontend`; the second must be rejected.
    let tc = TestCtx::new();
    let mut app = empty_app(&tc);
    app.add_repo_from_input("https://github.com/jotform/frontend.git".into());
    app.add_repo_from_input("https://gitlab.com/other/frontend".into());
    assert_eq!(app.sync.repos.len(), 1);
    assert_eq!(app.sync.repos[0].name, "frontend");
}

#[test]
fn rejects_empty_string() {
    // A23 — empty URL → no-op.
    let tc = TestCtx::new();
    let mut app = empty_app(&tc);
    app.add_repo_from_input("".into());
    assert_eq!(app.sync.repos.len(), 0);
}

#[test]
fn rejects_whitespace_only() {
    let tc = TestCtx::new();
    let mut app = empty_app(&tc);
    app.add_repo_from_input("   ".into());
    app.add_repo_from_input("\t\n".into());
    assert_eq!(app.sync.repos.len(), 0);
}

#[test]
fn rejects_url_with_no_path_segment() {
    // A24 — derived name would be empty, reject.
    let tc = TestCtx::new();
    let mut app = empty_app(&tc);
    app.add_repo_from_input("https://github.com/".into());
    app.add_repo_from_input("https://github.com".into());
    assert_eq!(app.sync.repos.len(), 0);
}

#[test]
fn accepts_ssh_style_urls() {
    let tc = TestCtx::new();
    let mut app = empty_app(&tc);
    app.add_repo_from_input("git@github.com:jotform/vendors.git".into());
    assert_eq!(app.sync.repos.len(), 1);
    assert_eq!(app.sync.repos[0].name, "vendors");
}

#[test]
fn multiple_distinct_repos_all_accepted() {
    let tc = TestCtx::new();
    let mut app = empty_app(&tc);
    app.add_repo_from_input("https://github.com/jotform/frontend.git".into());
    app.add_repo_from_input("https://github.com/jotform/vendors.git".into());
    app.add_repo_from_input("https://github.com/jotform/backend.git".into());
    assert_eq!(app.sync.repos.len(), 3);
    let names: Vec<&str> = app.sync.repos.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, &["frontend", "vendors", "backend"]);
}
