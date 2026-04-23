use anyhow::Result;
use chrono::NaiveDate;
use std::io::{self, Write};

use super::io::save;
use super::parse::parse_contract_periods;
use super::types::{Config, DEFAULT_TIMEZONE};
use crate::ctx::Paths;

pub fn ensure_time_credentials(paths: &Paths, config: &mut Config) -> Result<()> {
    let mut changed = false;

    if config
        .time
        .email
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        loop {
            let email = prompt("TimeDoctor email", None)?;
            if email.trim().is_empty() {
                eprintln!("Email cannot be empty.");
                continue;
            }
            config.time.email = Some(email);
            changed = true;
            break;
        }
    }

    if config.time.timezone.is_none() {
        let tz = prompt("Timezone", Some(DEFAULT_TIMEZONE))?;
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
        save(paths, config)?;
        println!("Configuration saved to {}", paths.config_file().display());
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
