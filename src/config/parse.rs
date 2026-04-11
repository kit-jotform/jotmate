use anyhow::{Context, Result};
use chrono::NaiveDate;

use super::types::ContractPeriod;

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
