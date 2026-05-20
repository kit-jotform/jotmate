use anyhow::{Context, Result};
use std::process::Command;

use crate::error::AppError;

const KEYCHAIN_SERVICE: &str = "jotmate-timedoctor";
const KEY_SESSION: &str = "session-cookie";
const KEY_PASSWORD: &str = "password";

// `security` CLI instead of the keyring crate's apple-native backend: SecKeychain binds items to
// the calling binary's hash, so every `cargo build` re-prompts even after "Always Allow". Items
// written via the CLI have no trusted-application ACL and never prompt.

/// Abstracted so tests can substitute an in-memory fake.
pub trait KeychainStore: Send + Sync {
    /// `Ok(Some(_))` stored · `Ok(None)` absent · `Err` denied or unavailable.
    fn get_token(&self) -> Result<Option<String>>;
    fn set_token(&self, value: &str) -> Result<()>;
    fn delete_token(&self) -> Result<()>;

    fn get_password(&self) -> Result<Option<String>>;
    fn set_password(&self, value: &str) -> Result<()>;
    fn delete_password(&self) -> Result<()>;
}

pub struct SecurityCliKeychain;

/// `errSecItemNotFound`.
const SECURITY_EXIT_NOT_FOUND: i32 = 44;

impl SecurityCliKeychain {
    fn get(&self, account: &str) -> Result<Option<String>> {
        let out = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                KEYCHAIN_SERVICE,
                "-a",
                account,
                "-w",
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

    fn set(&self, account: &str, password: &str) -> Result<()> {
        // Delete-then-add avoids `SecKeychainItemModifyAttributesAndData` errors on update.
        let _ = self.delete(account);

        let status = Command::new("security")
            .args([
                "add-generic-password",
                "-s",
                KEYCHAIN_SERVICE,
                "-a",
                account,
                "-w",
                password,
                "-U",
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

    fn delete(&self, account: &str) -> Result<()> {
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
        let _ = status;
        Ok(())
    }
}

impl KeychainStore for SecurityCliKeychain {
    fn get_token(&self) -> Result<Option<String>> {
        self.get(KEY_SESSION)
    }

    fn set_token(&self, value: &str) -> Result<()> {
        self.set(KEY_SESSION, value)
            .map_err(|e| AppError::Keyring(e.to_string()).into())
    }

    fn delete_token(&self) -> Result<()> {
        self.delete(KEY_SESSION)
            .map_err(|e| AppError::Keyring(e.to_string()).into())
    }

    fn get_password(&self) -> Result<Option<String>> {
        self.get(KEY_PASSWORD)
    }

    fn set_password(&self, value: &str) -> Result<()> {
        self.set(KEY_PASSWORD, value)
            .map_err(|e| AppError::Keyring(e.to_string()).into())
    }

    fn delete_password(&self) -> Result<()> {
        self.delete(KEY_PASSWORD)
            .map_err(|e| AppError::Keyring(e.to_string()).into())
    }
}
