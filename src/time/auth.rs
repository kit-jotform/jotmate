use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use std::process::Command;

use crate::error::AppError;

const KEYCHAIN_SERVICE: &str = "jotmate-timedoctor";
const KEY_SESSION: &str = "session-cookie";
const KEY_PASSWORD: &str = "password";

// ── Keychain helpers (macOS `security` CLI) ───────────────────────────────────
//
// Using the `security` CLI instead of the keyring crate's apple-native backend
// because the native SecKeychain API binds items to the calling binary's hash.
// Every `cargo build` produces a new binary → new hash → macOS re-prompts even
// after "Always Allow". Items written via the `security` CLI have no
// trusted-application ACL and never prompt.

fn keychain_get(account: &str) -> Option<String> {
    let out = Command::new("security")
        .args([
            "find-generic-password",
            "-s", KEYCHAIN_SERVICE,
            "-a", account,
            "-w", // print password only
        ])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if out.status.success() {
        let s = String::from_utf8(out.stdout).ok()?;
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
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
            "-s", KEYCHAIN_SERVICE,
            "-a", account,
            "-w", password,
            "-U", // update if already exists
        ])
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
            "-s", KEYCHAIN_SERVICE,
            "-a", account,
        ])
        .stderr(std::process::Stdio::null())
        .status()
        .context("security CLI not found")?;
    // Ignore "not found" (exit 44)
    let _ = status;
    Ok(())
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn load_token_from_keychain() -> Option<String> {
    keychain_get(KEY_SESSION)
}

pub fn save_token_to_keychain(cookie_string: &str) -> Result<()> {
    keychain_set(KEY_SESSION, cookie_string)
        .map_err(|e| AppError::Keyring(e.to_string()).into())
}

pub fn delete_token_from_keychain() -> Result<()> {
    keychain_delete(KEY_SESSION)
        .map_err(|e| AppError::Keyring(e.to_string()).into())
}

pub fn load_password_from_keychain() -> Option<String> {
    keychain_get(KEY_PASSWORD)
}

pub fn save_password_to_keychain(password: &str) -> Result<()> {
    keychain_set(KEY_PASSWORD, password)
        .map_err(|e| AppError::Keyring(e.to_string()).into())
}

#[allow(dead_code)]
pub fn delete_password_from_keychain() -> Result<()> {
    keychain_delete(KEY_PASSWORD)
        .map_err(|e| AppError::Keyring(e.to_string()).into())
}

// ── Network ───────────────────────────────────────────────────────────────────

pub async fn login(email: &str, password: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://2.timedoctor.com"),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://2.timedoctor.com/"),
    );

    let body = serde_json::json!({
        "email": email,
        "password": password,
    });

    let resp = client
        .post("https://api2.timedoctor.com/api/2.0/auth/v2/login")
        .headers(headers)
        .json(&body)
        .send()
        .await
        .context("Failed to connect to TimeDoctor API")?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED
        || resp.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Err(AppError::AuthFailed("Invalid email or password".to_string()).into());
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::AuthFailed(format!("HTTP {status}: {text}")).into());
    }

    let cookie_string = extract_cookies(&resp)?;

    if !cookie_string.contains("__Host-accessToken") {
        return Err(AppError::AuthFailed("No access token received from login".to_string()).into());
    }

    Ok(cookie_string)
}

fn extract_cookies(resp: &reqwest::Response) -> Result<String> {
    let mut parts = Vec::new();
    for value in resp.headers().get_all("set-cookie") {
        if let Ok(v) = value.to_str() {
            if let Some(pair) = v.split(';').next() {
                parts.push(pair.trim().to_string());
            }
        }
    }
    if parts.is_empty() {
        anyhow::bail!("No Set-Cookie headers received");
    }
    Ok(parts.join("; "))
}

pub async fn prompt_password(email: &str) -> Result<String> {
    print!("Enter TimeDoctor password for {email}: ");
    let password = rpassword::read_password().context("Failed to read password")?;
    Ok(password)
}

pub async fn get_or_refresh_token(email: &str) -> Result<String> {
    if let Some(token) = load_token_from_keychain() {
        return Ok(token);
    }
    do_login(email).await
}

/// Delete the stale session token and re-authenticate from scratch.
pub async fn reauth(email: &str) -> Result<String> {
    let _ = delete_token_from_keychain();
    do_login(email).await
}

async fn do_login(email: &str) -> Result<String> {
    let password = if let Some(pw) = load_password_from_keychain() {
        pw
    } else {
        prompt_password(email).await?
    };

    let cookie = login(email, &password).await?;
    save_token_to_keychain(&cookie)?;
    Ok(cookie)
}
