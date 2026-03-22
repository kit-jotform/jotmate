use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsResponse {
    pub data: Vec<StatsEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsEntry {
    #[serde(default)]
    pub total: u64, // seconds
    #[serde(default)]
    pub computer: u64,
    #[serde(default)]
    pub mobile: u64,
    #[serde(default)]
    pub manual: u64,
    #[serde(default)]
    pub offcomputer: u64,
}

pub async fn get_week_stats(
    client: &reqwest::Client,
    cookie_string: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    company_id: &str,
    timezone: &str,
) -> Result<StatsResponse> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Cookie",
        HeaderValue::from_str(cookie_string)
            .map_err(|_| AppError::AuthFailed("Invalid cookie string".to_string()))?,
    );
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://2.timedoctor.com"),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://2.timedoctor.com/"),
    );

    let resp = client
        .get("https://api2.timedoctor.com/api/1.1/stats/total")
        .headers(headers)
        .query(&[
            ("from", from.to_rfc3339()),
            ("to", to.to_rfc3339()),
            ("timezone", timezone.to_string()),
            ("user", String::new()),
            ("group-by", "company".to_string()),
            (
                "fields",
                "mobile,manual,offcomputer,computer,computerRatio,partial,total,paidBreak,unpaidBreak,paidLeave".to_string(),
            ),
            ("untracked", "1".to_string()),
            ("page", "0".to_string()),
            ("limit", "200".to_string()),
            ("company", company_id.to_string()),
        ])
        .send()
        .await?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(AppError::TokenExpired.into());
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Stats API error: HTTP {status}: {text}");
    }

    let stats: StatsResponse = resp.json().await?;
    Ok(stats)
}
