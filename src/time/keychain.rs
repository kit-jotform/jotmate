use anyhow::{Context, Result};
use std::process::Command;

use crate::error::AppError;

const KEYCHAIN_SERVICE: &str = "jotmate-timedoctor";
const KEY_SESSION: &str = "session-cookie";
const KEY_PASSWORD: &str = "password";

// Using the `security` CLI instead of the keyring crate's apple-native backend
// because the native SecKeychain API binds items to the calling binary's hash.
// Every `cargo build` produces a new binary → new hash → macOS re-prompts even
// after "Always Allow". Items written via the `security` CLI have no
// trusted-application ACL and never prompt.

/// macOS `security` exit code for `errSecItemNotFound`.
const SECURITY_EXIT_NOT_FOUND: i32 = 44;

/// `Ok(Some(_))` = found, `Ok(None)` = not in keychain, `Err(_)` = access
/// denied, CLI missing, or other system failure. The distinction matters so
/// we can re-prompt on "not found" but surface a real error instead of
/// silently falling through on "denied".
fn keychain_get(account: &str) -> Result<Option<String>> {
    let out = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            account,
            "-w", // print password only
        ])
        .stderr(std::process::Stdio::piped())
        .output()
        .context("security CLI not found")?;

    if out.status.success() {
        let s = String::from_utf8(out.stdout).context("keychain returned non-UTF8 data")?;
        let trimmed = s.trim().to_string();
        return Ok(if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        });
    }

    if out.status.code() == Some(SECURITY_EXIT_NOT_FOUND) {
        return Ok(None);
    }

    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let code = out
        .status
        .code()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "signal".to_string());
    anyhow::bail!(
        "keychain access failed (exit {}): {}",
        code,
        if stderr.is_empty() {
            "no details"
        } else {
            stderr.as_str()
        }
    )
}

fn keychain_set(account: &str, password: &str) -> Result<()> {
    // Delete first (ignore errors), then add fresh — this avoids
    // "SecKeychainItemModifyAttributesAndData" errors on update.
    let _ = keychain_delete(account);

    let status = Command::new("security")
        .args([
            "add-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            account,
            "-w",
            password,
            "-U", // update if already exists
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("security CLI not found")?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("security add-generic-password failed (exit {})", status)
    }
}

fn keychain_delete(account: &str) -> Result<()> {
    let status = Command::new("security")
        .args([
            "delete-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            account,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("security CLI not found")?;
    // Ignore "not found" (exit 44)
    let _ = status;
    Ok(())
}

pub fn load_token_from_keychain() -> Result<Option<String>> {
    keychain_get(KEY_SESSION)
}

pub fn save_token_to_keychain(cookie_string: &str) -> Result<()> {
    keychain_set(KEY_SESSION, cookie_string).map_err(|e| AppError::Keyring(e.to_string()).into())
}

pub fn delete_token_from_keychain() -> Result<()> {
    keychain_delete(KEY_SESSION).map_err(|e| AppError::Keyring(e.to_string()).into())
}

pub fn load_password_from_keychain() -> Result<Option<String>> {
    keychain_get(KEY_PASSWORD)
}

pub fn save_password_to_keychain(password: &str) -> Result<()> {
    keychain_set(KEY_PASSWORD, password).map_err(|e| AppError::Keyring(e.to_string()).into())
}

#[allow(dead_code)]
pub fn delete_password_from_keychain() -> Result<()> {
    keychain_delete(KEY_PASSWORD).map_err(|e| AppError::Keyring(e.to_string()).into())
}
