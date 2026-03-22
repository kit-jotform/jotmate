use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};

use crate::error::AppError;

const KEYRING_SERVICE: &str = "jotmate-timedoctor";
const KEYRING_USERNAME: &str = "session-cookie";

pub fn load_token_from_keychain() -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USERNAME).ok()?;
    entry.get_password().ok()
}

pub fn save_token_to_keychain(cookie_string: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)
        .map_err(|e| AppError::Keyring(e.to_string()))?;
    entry
        .set_password(cookie_string)
        .map_err(|e| AppError::Keyring(e.to_string()))?;
    Ok(())
}

pub fn delete_token_from_keychain() -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)
        .map_err(|e| AppError::Keyring(e.to_string()))?;
    let _ = entry.delete_credential(); // Ignore "not found" errors
    Ok(())
}

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

    // Extract cookies from Set-Cookie headers
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
            // Take only the name=value part (before the first ';')
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

/// Gets a valid session token, prompting for password if needed.
/// On 401 during API calls, call this again after `delete_token_from_keychain()`.
pub async fn get_or_refresh_token(email: &str) -> Result<String> {
    if let Some(token) = load_token_from_keychain() {
        return Ok(token);
    }

    let password = prompt_password(email).await?;
    eprintln!("Authenticating...");
    let cookie = login(email, &password).await?;
    save_token_to_keychain(&cookie)?;
    eprintln!("Authenticated successfully.");
    Ok(cookie)
}
