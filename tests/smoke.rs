mod common;

use common::TestCtx;

#[test]
fn test_ctx_points_at_tempdir_not_home() {
    let tc = TestCtx::new();
    // Config file doesn't exist yet, but the path must live under /tmp-style
    // roots — nowhere near $HOME/.config/jotmate.
    let cfg = tc.config_file();
    let cfg_str = cfg.display().to_string();
    assert!(
        !cfg_str.contains("/.config/jotmate/config.toml")
            || cfg_str.contains("config/jotmate/config.toml"),
        "config path looks like the real user dir: {cfg_str}"
    );
    assert!(!cfg.exists());
}

#[test]
fn test_fake_keychain_seeded_via_with_entries() {
    let kc =
        common::FakeKeychain::with_entries(&[("session-cookie", "abc"), ("password", "hunter2")]);
    assert_eq!(kc.get_raw("session-cookie").as_deref(), Some("abc"));
    assert_eq!(kc.get_raw("password").as_deref(), Some("hunter2"));
}
