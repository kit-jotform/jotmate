use anyhow::Result;
use chrono::NaiveDate;
use std::io::{self, Write};

use super::io::{config_path, save};
use super::parse::parse_contract_periods;
use super::types::Config;

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
