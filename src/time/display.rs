use std::io::Write;

use crate::time::compute::HOURS_DISPLAY_WIDTH;
use crate::tui::palette::{
    ANSI_ACCENT, ANSI_DANGEROUS, ANSI_MUTED, ANSI_RESET, ANSI_SUCCESS, SPINNER,
};

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
    let weekly_color = if weekly >= 0.0 {
        ANSI_SUCCESS
    } else {
        ANSI_DANGEROUS
    };
    let cum_color = if cumulative >= 0.0 {
        ANSI_SUCCESS
    } else {
        ANSI_DANGEROUS
    };
    let weekly_val = super::compute::format_hours_signed(weekly);
    let cum_val = super::compute::format_hours_signed(cumulative);
    println!(
        "\r     {ANSI_MUTED}This Week:{ANSI_RESET} {weekly_color}{weekly_val}{ANSI_RESET}  {ANSI_MUTED}•{ANSI_RESET}  {ANSI_MUTED}Cumulative:{ANSI_RESET} {cum_color}{cum_val}{ANSI_RESET}  {ANSI_MUTED}•{ANSI_RESET}  {ANSI_MUTED}{elapsed_secs:.1}s{ANSI_RESET}\x1b[K"
    );
    println!();
}
