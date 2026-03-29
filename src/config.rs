use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub sync: SyncConfig,
    #[serde(default)]
    pub time: TimeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamRepo {
    pub url: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl UpstreamRepo {
    pub fn new(url: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            name: name.into(),
            enabled: true,
        }
    }
}

fn default_upstream_repos() -> Vec<UpstreamRepo> {
    vec![
        UpstreamRepo::new("https://github.com/jotform/frontend.git", "frontend"),
        UpstreamRepo::new("https://github.com/jotform/vendors.git", "vendors"),
        UpstreamRepo::new("https://github.com/jotform/backend.git", "backend"),
        UpstreamRepo::new("https://github.com/jotform/Jotform3.git", "Jotform3"),
        UpstreamRepo::new("https://github.com/jotform/core.git", "core"),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Upstream repositories to sync (URL + name + enabled flag)
    #[serde(default = "default_upstream_repos")]
    pub upstream_repos: Vec<UpstreamRepo>,
    /// Default projects to sync when --only is not passed
    pub default_only: Option<Vec<String>>,
    /// If true, run with --sync-all by default
    #[serde(default)]
    pub sync_all_by_default: bool,
    /// If true, use the repo path cache; if false, always rediscover
    #[serde(default = "default_true")]
    pub use_cache: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            upstream_repos: default_upstream_repos(),
            default_only: None,
            sync_all_by_default: false,
            use_cache: true,
        }
    }
}

/// TimeDoctor company ID — hardcoded, not user-configurable
pub const TIMEDOCTOR_COMPANY_ID: &str = "Xms4iFqBgQAEjLy2";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeConfig {
    pub email: Option<String>,
    pub timezone: Option<String>,
    pub start_date: Option<NaiveDate>,
    #[serde(default = "default_true")]
    pub skip_current_week: bool,
    pub contract_periods: Option<Vec<ContractPeriod>>,
    pub reset_cumulative_from_date: Option<NaiveDate>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractPeriod {
    pub from: NaiveDate,
    pub weekly_hours: f64,
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("jotmate")
        .join("config.toml")
}

pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config from {}", path.display()))?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory {}", parent.display()))?;
    }
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;
    Ok(())
}

/// Prompts for any missing TimeConfig fields interactively. Saves config before returning.
pub fn ensure_time_credentials(config: &mut Config) -> Result<()> {
    let mut changed = false;

    if config.time.email.is_none() {
        let email = prompt("TimeDoctor email", None)?;
        config.time.email = Some(email);
        changed = true;
    }

    if config.time.timezone.is_none() {
        let tz = prompt("Timezone", Some("Europe/Istanbul"))?;
        config.time.timezone = Some(tz);
        changed = true;
    }

    if config.time.start_date.is_none() {
        loop {
            let s = prompt("Start date (YYYY-MM-DD)", None)?;
            match NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                Ok(d) => {
                    config.time.start_date = Some(d);
                    changed = true;
                    break;
                }
                Err(_) => eprintln!("Invalid date format. Please use YYYY-MM-DD."),
            }
        }
    }

    if config.time.contract_periods.is_none() {
        println!("Enter contract periods (e.g. 2025-11-17:20,2026-02-02:28)");
        println!("Format: YYYY-MM-DD:HOURS[,YYYY-MM-DD:HOURS,...]");
        loop {
            let s = prompt("Contract periods", None)?;
            match parse_contract_periods(&s) {
                Ok(periods) => {
                    config.time.contract_periods = Some(periods);
                    changed = true;
                    break;
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
    }

    if changed {
        save(config)?;
        println!("Configuration saved to {}", config_path().display());
    }

    Ok(())
}

fn prompt(label: &str, default: Option<&str>) -> Result<String> {
    match default {
        Some(d) => print!("{label} [{d}]: "),
        None => print!("{label}: "),
    }
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        Ok(default.unwrap_or("").to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

pub fn parse_contract_periods(s: &str) -> Result<Vec<ContractPeriod>> {
    let mut periods = Vec::new();
    for entry in s.split(',') {
        let entry = entry.trim();
        let parts: Vec<&str> = entry.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!(
                "Invalid contract period '{}': expected YYYY-MM-DD:HOURS",
                entry
            );
        }
        let from = NaiveDate::parse_from_str(parts[0].trim(), "%Y-%m-%d")
            .with_context(|| format!("Invalid date '{}'", parts[0]))?;
        let weekly_hours: f64 = parts[1]
            .trim()
            .parse()
            .with_context(|| format!("Invalid hours '{}'", parts[1]))?;
        if weekly_hours < 0.0 {
            anyhow::bail!("Hours cannot be negative: {}", weekly_hours);
        }
        periods.push(ContractPeriod { from, weekly_hours });
    }
    if periods.is_empty() {
        anyhow::bail!("No contract periods provided");
    }
    periods.sort_by_key(|p| p.from);
    Ok(periods)
}
