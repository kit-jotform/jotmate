use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};

use crate::error::AppError;
use crate::time::keychain::{
    delete_token_from_keychain, load_password_from_keychain, load_token_from_keychain,
    save_token_to_keychain,
};

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
