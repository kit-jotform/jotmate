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

fn keychain_get(account: &str) -> Option<String> {
    let out = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            account,
            "-w", // print password only
        ])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if out.status.success() {
        let s = String::from_utf8(out.stdout).ok()?;
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    } else {
        None
    }
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

pub fn load_token_from_keychain() -> Option<String> {
    keychain_get(KEY_SESSION)
}

pub fn save_token_to_keychain(cookie_string: &str) -> Result<()> {
    keychain_set(KEY_SESSION, cookie_string).map_err(|e| AppError::Keyring(e.to_string()).into())
}

pub fn delete_token_from_keychain() -> Result<()> {
    keychain_delete(KEY_SESSION).map_err(|e| AppError::Keyring(e.to_string()).into())
}

pub fn load_password_from_keychain() -> Option<String> {
    keychain_get(KEY_PASSWORD)
}

pub fn save_password_to_keychain(password: &str) -> Result<()> {
    keychain_set(KEY_PASSWORD, password).map_err(|e| AppError::Keyring(e.to_string()).into())
}

#[allow(dead_code)]
pub fn delete_password_from_keychain() -> Result<()> {
    keychain_delete(KEY_PASSWORD).map_err(|e| AppError::Keyring(e.to_string()).into())
}
