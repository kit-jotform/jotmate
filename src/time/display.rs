use std::io::Write;

use crate::time::compute::HOURS_DISPLAY_WIDTH;
use crate::tui::palette::{ansi_balance_color, ANSI_ACCENT, ANSI_MUTED, ANSI_RESET, SPINNER};

pub fn hide_cursor() {
    print!("\x1b[?25l");
    let _ = std::io::stdout().flush();
}

pub fn show_cursor() {
    print!("\x1b[?25h");
    let _ = std::io::stdout().flush();
}

pub fn print_progress(tick: usize, elapsed_secs: f64) {
    let spinner_ch = SPINNER[tick % SPINNER.len()];
    let pad = " ".repeat(HOURS_DISPLAY_WIDTH - 1);
    print!(
        "\r     {ANSI_MUTED}This Week:{ANSI_RESET} {ANSI_ACCENT}{spinner_ch}{ANSI_RESET}{pad}  {ANSI_MUTED}•{ANSI_RESET}  {ANSI_MUTED}Cumulative:{ANSI_RESET} {ANSI_ACCENT}{spinner_ch}{ANSI_RESET}{pad}  {ANSI_MUTED}•{ANSI_RESET}  {ANSI_MUTED}{elapsed_secs:.1}s{ANSI_RESET}  "
    );
    let _ = std::io::stdout().flush();
}

pub fn print_final(weekly: f64, cumulative: f64, elapsed_secs: f64) {
    let weekly_color = ansi_balance_color(weekly);
    let cum_color = ansi_balance_color(cumulative);
    let weekly_val = super::compute::format_hours_signed(weekly);
    let cum_val = super::compute::format_hours_signed(cumulative);
    let weekly_padded = format!("{weekly_val:<HOURS_DISPLAY_WIDTH$}");
    let cum_padded = format!("{cum_val:<HOURS_DISPLAY_WIDTH$}");
    println!(
        "\r     {ANSI_MUTED}This Week:{ANSI_RESET} {weekly_color}{weekly_padded}{ANSI_RESET}  {ANSI_MUTED}•{ANSI_RESET}  {ANSI_MUTED}Cumulative:{ANSI_RESET} {cum_color}{cum_padded}{ANSI_RESET}  {ANSI_MUTED}•{ANSI_RESET}  {ANSI_MUTED}{elapsed_secs:.1}s{ANSI_RESET}\x1b[K"
    );
    println!();
}
