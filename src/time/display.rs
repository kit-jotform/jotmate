use std::io::Write;

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

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
    let cum_part = format!("{CYAN}{spinner_ch}{RESET}     ");
    print!(
        "\r     {MUTED}Total Weekly:{RESET} {CYAN}{spinner_ch}{RESET}  {MUTED}•  Cumulative:{RESET} {cum_part}  {MUTED}•  {elapsed_secs:.1}s{RESET}  "
    );
    let _ = std::io::stdout().flush();
}

pub fn print_final(weekly: f64, cumulative: f64, elapsed_secs: f64) {
    let weekly_color = if weekly >= 0.0 { GREEN } else { RED };
    let cum_color = if cumulative >= 0.0 { GREEN } else { RED };
    let weekly_val = super::compute::format_hours_signed(weekly);
    let cum_val = super::compute::format_hours_signed(cumulative);
    println!(
        "\r     {MUTED}Total Weekly:{RESET} {weekly_color}{weekly_val}{RESET}  {MUTED}•  {RESET}{MUTED}Cumulative:{RESET} {cum_color}{cum_val}{RESET}  {MUTED}•  {elapsed_secs:.1}s{RESET}  "
    );
    println!();
}
