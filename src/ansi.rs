//! ANSI escape constants for headless (non-TUI) renderers.
//! Color indices match the palette constants in `tui/palette.rs`.

pub const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];

pub const ANSI_TEXT: &str = "\x1b[38;5;255m"; // C_TEXT      255 white
pub const ANSI_ACCENT: &str = "\x1b[38;5;51m"; // C_ACCENT     51 cyan
pub const ANSI_SUCCESS: &str = "\x1b[38;5;10m"; // C_SUCCESS    10 green
pub const ANSI_MUTED: &str = "\x1b[38;5;243m"; // C_MUTED     243 dark gray
pub const ANSI_DANGEROUS: &str = "\x1b[38;5;9m"; // C_DANGEROUS   9 red
pub const ANSI_WARN: &str = "\x1b[38;5;11m"; // C_WARN       11 yellow
pub const ANSI_RESET: &str = "\x1b[0m";
