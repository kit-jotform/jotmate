use crate::time::compute::{format_hours, WeekRow};

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[90m";

pub fn print_results(rows: &[WeekRow]) {
    let separator = "=".repeat(110);
    let title = "WORK HOURS ANALYSIS";
    let pad = (110 - title.len()) / 2;
    println!("\n{separator}");
    println!("{}{title}", " ".repeat(pad));
    println!("{separator}\n");

    // Header
    println!(
        "{:<28}  {:<10}  {:<8}  {:<10}  {:<6}  {:<12}  {:<6}",
        "Week", "Worked", "Target", "Balance", "OK?", "Cumulative", "Cumul?"
    );
    println!("{}", "-".repeat(90));

    // Rows oldest-first for display
    let mut display_rows: Vec<&WeekRow> = rows.iter().collect();
    display_rows.sort_by_key(|r| r.monday);

    for row in &display_rows {
        let (bal_color, bal_icon) = if row.balance_hours >= 0.0 {
            (GREEN, "✅")
        } else {
            (RED, "❌")
        };
        let (cum_color, cum_icon) = if row.cumulative_hours >= 0.0 {
            (GREEN, "✅")
        } else {
            (RED, "❌")
        };
        let cache_marker = if row.from_cache {
            format!("{DIM}[cache]{RESET}")
        } else {
            String::new()
        };

        println!(
            "{:<28}  {:<10}  {:<8}  {bal_color}{:<10}{RESET}  {bal_icon}  {cum_color}{:<12}{RESET}  {cum_icon}  {cache_marker}",
            row.week_label,
            format_hours(row.worked_secs as f64 / 3600.0),
            format_hours(row.target_hours),
            format_hours(row.balance_hours),
            format_hours(row.cumulative_hours),
        );
    }

    println!("{}", "-".repeat(90));

    // Total balance = most recent row's cumulative
    if let Some(last) = rows.iter().max_by_key(|r| r.monday) {
        let total = last.cumulative_hours;
        let color = if total >= 0.0 { GREEN } else { RED };
        println!(
            "\n{BOLD}{color}Total Balance: {}{}{RESET}\n",
            if total >= 0.0 { "+" } else { "" },
            format_hours(total)
        );
    }
}
