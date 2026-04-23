//! Tests for the `KeychainStore` abstraction and the three-way
//! `Ok(Some)` / `Ok(None)` / `Err(_)` distinction that `auth::do_login`
//! relies on.
//!
//! Covered edge cases:
//!   B8    keychain empty → `get_*` returns Ok(None), triggers password prompt
//!   B9    keychain denied → `get_*` returns Err, propagates out of auth
//!   set_td_password / delete_token behavior through the App wrapper

mod common;

use common::{FakeKeychain, TestCtx};
use jotmate::ctx::Ctx;
use jotmate::time::keychain::KeychainStore;
use jotmate::tui::app::App;

fn app_from(tc: &TestCtx) -> App {
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
fn fake_keychain_empty_returns_ok_none() {
    let kc = FakeKeychain::new();
    assert!(matches!(kc.get_token(), Ok(None)));
    assert!(matches!(kc.get_password(), Ok(None)));
}

#[test]
fn fake_keychain_set_then_get_roundtrips() {
    let kc = FakeKeychain::new();
    kc.set_token("abc").unwrap();
    kc.set_password("hunter2").unwrap();
    assert_eq!(kc.get_token().unwrap().as_deref(), Some("abc"));
    assert_eq!(kc.get_password().unwrap().as_deref(), Some("hunter2"));
}

#[test]
fn fake_keychain_delete_removes_entry() {
    let kc = FakeKeychain::with_entries(&[("session-cookie", "x"), ("password", "y")]);
    kc.delete_token().unwrap();
    kc.delete_password().unwrap();
    assert!(matches!(kc.get_token(), Ok(None)));
    assert!(matches!(kc.get_password(), Ok(None)));
}

#[test]
fn fake_keychain_denied_returns_err() {
    // B9 — simulate access-denied.
    let kc = FakeKeychain::new();
    kc.set_get_error("user denied access");
    let err = kc.get_token().unwrap_err();
    assert!(err.to_string().contains("user denied access"));

    // set_* / delete_* must still succeed (they don't go through the read
    // path that's being denied).
    kc.set_token("ok").unwrap();
    // But a subsequent read still errors while the flag is set.
    assert!(kc.get_token().is_err());

    kc.clear_get_error();
    assert_eq!(kc.get_token().unwrap().as_deref(), Some("ok"));
}

#[test]
fn password_is_set_with_empty_keychain_returns_false() {
    // B8 — `App::password_is_set` returns false when the keychain has no entry.
    let tc = TestCtx::new();
    let app = app_from(&tc);
    assert!(!app.password_is_set());
}

#[test]
fn password_is_set_with_populated_keychain_returns_true() {
    let tc = TestCtx::new();
    tc.keychain.set_password("hunter2").unwrap();
    let app = app_from(&tc);
    assert!(app.password_is_set());
}

#[test]
fn password_is_set_under_denied_keychain_returns_false() {
    // B9 — `App::password_is_set` treats an Err from the keychain as "not
    // set" so the TUI still lets the user re-enter. The real error surfaces
    // from the save path.
    let tc = TestCtx::new();
    tc.keychain.set_password("hunter2").unwrap(); // present but inaccessible
    tc.keychain.set_get_error("Keychain access denied");
    let app = app_from(&tc);
    assert!(!app.password_is_set());
}

#[test]
fn set_td_password_stores_and_clears_session_token() {
    // Setting a new password must invalidate the old session cookie so the
    // next auth flow triggers a fresh login.
    let tc = TestCtx::new();
    tc.keychain.set_token("stale-cookie").unwrap();
    let mut app = app_from(&tc);
    assert!(app.set_td_password("new-pw"));
    // Token gone:
    assert!(matches!(tc.keychain.get_token(), Ok(None)));
    // Password stored:
    assert_eq!(
        tc.keychain.get_password().unwrap().as_deref(),
        Some("new-pw")
    );
}

#[test]
fn set_td_password_rejects_empty() {
    let tc = TestCtx::new();
    let mut app = app_from(&tc);
    assert!(!app.set_td_password(""));
    assert!(matches!(tc.keychain.get_password(), Ok(None)));
}
