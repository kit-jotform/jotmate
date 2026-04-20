use ratatui::{style::Style, text::Span};

use crate::tui::palette::C_MUTED;

pub(in crate::tui::draw) const HINT_CYCLE_VALUE: &str = "  ↑↓ change  •  ↵ confirm  •  ⌫ cancel";

pub(in crate::tui::draw) fn hint_muted(parts: &[&str]) -> Vec<Span<'static>> {
    parts
        .iter()
        .map(|s| Span::styled(s.to_string(), Style::default().fg(C_MUTED)))
        .collect()
}

/// Standard sub-screen footer hint: `↑↓ navigate • ↵ {action} • ⌫/Esc back`.
/// `action` is padded to 6 chars so the `•` separators align regardless of verb.
pub(in crate::tui::draw) fn hint_navigate_action(action: &str) -> Vec<Span<'static>> {
    hint_muted(&[
        "↑↓",
        " navigate  •  ",
        "↵",
        &format!(" {action:<6}  •  "),
        "⌫/Esc",
        " back",
    ])
}

pub(in crate::tui::draw) fn hint_confirm_cancel() -> Vec<Span<'static>> {
    hint_muted(&["↵/y", " confirm  •  ", "Esc/n", " cancel"])
}

pub(in crate::tui::draw) fn hint_input_confirm() -> Vec<Span<'static>> {
    hint_muted(&["↵", " confirm  •  ", "Esc", " cancel"])
}
