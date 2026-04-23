use anyhow::{Context, Result};
use std::sync::Arc;

use crate::ctx::HttpBase;
use crate::error::AppError;
use crate::time::keychain::KeychainStore;

/// Offload to the blocking pool — the `security` CLI is inherently blocking.
async fn blocking<F, T>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .expect("blocking task panicked")
}

pub async fn login(http: &HttpBase, email: &str, password: &str) -> Result<String> {
    let client = crate::time::api::shared_client();
    let headers = crate::time::api::td_headers();

    let body = serde_json::json!({
        "email": email,
        "password": password,
    });

    let resp = client
        .post(&http.login)
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
        return Err(AppError::Http(resp.error_for_status().unwrap_err()).into());
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

pub async fn get_or_refresh_token(
    keychain: Arc<dyn KeychainStore>,
    http: &HttpBase,
    email: &str,
) -> Result<String> {
    let kc = keychain.clone();
    match blocking(move || kc.get_token()).await {
        Ok(Some(token)) => return Ok(token),
        Ok(None) => {}
        Err(e) => return Err(e.context("Could not read session token from keychain")),
    }
    do_login(keychain, http, email).await
}

pub async fn reauth(
    keychain: Arc<dyn KeychainStore>,
    http: &HttpBase,
    email: &str,
) -> Result<String> {
    let kc = keychain.clone();
    let _ = blocking(move || kc.delete_token()).await;
    do_login(keychain, http, email).await
}

async fn do_login(
    keychain: Arc<dyn KeychainStore>,
    http: &HttpBase,
    email: &str,
) -> Result<String> {
    let kc = keychain.clone();
    let stored_password = blocking(move || kc.get_password())
        .await
        .context("Could not read saved password from keychain")?;
    let password = match stored_password {
        Some(pw) => pw,
        None => prompt_password(email).await?,
    };

    let cookie = login(http, email, &password).await?;
    let cookie_for_keychain = cookie.clone();
    let kc = keychain.clone();
    blocking(move || kc.set_token(&cookie_for_keychain)).await?;
    Ok(cookie)
}
