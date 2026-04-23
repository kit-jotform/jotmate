use ratatui::style::Color;

// One literal → both Color::Indexed and the matching ANSI escape.
macro_rules! color {
    ($name:ident, $ansi:ident, $idx:literal) => {
        pub const $name: Color = Color::Indexed($idx);
        pub const $ansi: &str = concat!("\x1b[38;5;", $idx, "m");
    };
}

color!(C_TEXT, ANSI_TEXT, 255);
color!(C_ACCENT, ANSI_ACCENT, 51);
color!(C_SUCCESS, ANSI_SUCCESS, 40);
color!(C_MUTED, ANSI_MUTED, 243);
color!(C_DANGEROUS, ANSI_DANGEROUS, 160);
color!(C_WARN, ANSI_WARN, 220);

pub const C_PRIMARY: Color = Color::Indexed(199);

pub const C_SELECT: Color = C_PRIMARY;
pub const C_LOGO: Color = C_TEXT;
pub const ANSI_RESET: &str = "\x1b[0m";

pub const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
