use std::io::Write;

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
const VALUE_WIDTH: usize = 8;

// ANSI color codes matching palette.rs indexed colors.
const GREEN: &str = "\x1b[92m";
const RED: &str = "\x1b[91m";
const CYAN: &str = "\x1b[96m";
const MUTED: &str = "\x1b[90m";
const RESET: &str = "\x1b[0m";

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
    let pad = " ".repeat(VALUE_WIDTH - 1);
    print!(
        "\r     {MUTED}This Week:{RESET} {CYAN}{spinner_ch}{RESET}{pad}  {MUTED}•{RESET}  {MUTED}Cumulative:{RESET} {CYAN}{spinner_ch}{RESET}{pad}  {MUTED}•{RESET}  {MUTED}{elapsed_secs:.1}s{RESET}  "
    );
    let _ = std::io::stdout().flush();
}

pub fn print_final(weekly: f64, cumulative: f64, elapsed_secs: f64) {
    let weekly_color = if weekly >= 0.0 { GREEN } else { RED };
    let cum_color = if cumulative >= 0.0 { GREEN } else { RED };
    let weekly_val = super::compute::format_hours_signed(weekly);
    let cum_val = super::compute::format_hours_signed(cumulative);
    let weekly_pad = " ".repeat(VALUE_WIDTH.saturating_sub(weekly_val.chars().count()));
    let cum_pad = " ".repeat(VALUE_WIDTH.saturating_sub(cum_val.chars().count()));
    println!(
        "\r     {MUTED}This Week:{RESET} {weekly_color}{weekly_val}{RESET}{weekly_pad}  {MUTED}•{RESET}  {MUTED}Cumulative:{RESET} {cum_color}{cum_val}{RESET}{cum_pad}  {MUTED}•{RESET}  {MUTED}{elapsed_secs:.1}s{RESET}  "
    );
    println!();
}
