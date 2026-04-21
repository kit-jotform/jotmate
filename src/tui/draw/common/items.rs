use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::ListItem,
};

use crate::tui::palette::{C_ACCENT, C_DANGEROUS, C_MUTED, C_PRIMARY, C_SUCCESS, C_TEXT};

use super::{HINT_CYCLE_VALUE, SEPARATOR_WIDTH};

pub(in crate::tui::draw) fn blank_item() -> ListItem<'static> {
    ListItem::new(Line::raw(""))
}

pub(in crate::tui::draw) fn divider_item() -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("─".repeat(SEPARATOR_WIDTH), Style::default().fg(C_MUTED)),
    ]))
}

pub(in crate::tui::draw) const FIELD_LABEL_W: usize = 18;

/// Narrower label column for the timezone selector — the inline editing hint
/// (↑↓ change • ↵ confirm • ⌫ cancel) needs the extra room to fit in UI_WIDTH.
pub(in crate::tui::draw) const FIELD_LABEL_W_TZ: usize = 13;

/// Build a muted section-separator list item: `"── {label} ────────────"`.
/// The total visible width is `SEPARATOR_WIDTH` chars.
pub(in crate::tui::draw) fn separator_item(label: &str) -> ListItem<'static> {
    // "── " + label + " " = prefix; fill the rest with "─"
    let prefix_chars = 3 + label.chars().count() + 1;
    let dashes = "─".repeat(SEPARATOR_WIDTH.saturating_sub(prefix_chars));
    let text = format!("── {} {}", label, dashes);
    ListItem::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(text, Style::default().fg(C_MUTED)),
    ]))
}

pub(in crate::tui::draw) fn back_item(is_sel: bool) -> ListItem<'static> {
    let style = if is_sel {
        Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(C_MUTED)
    };
    ListItem::new(Line::from(vec![
        Span::styled(
            if is_sel { "▸ " } else { "  " },
            Style::default().fg(C_PRIMARY),
        ),
        Span::styled("← Back", style),
    ]))
}

pub(in crate::tui::draw) fn toggle_item(
    is_sel: bool,
    on: bool,
    label: String,
    indent: bool,
    disabled: bool,
) -> ListItem<'static> {
    let prefix = if indent { "    " } else { "" };
    let badge = if on { "[ON ]  " } else { "[OFF]  " };
    if disabled {
        let arrow = if is_sel { "▸ " } else { "  " };
        ListItem::new(Line::from(vec![
            Span::styled(arrow, Style::default().fg(C_MUTED)),
            Span::styled(prefix, Style::default().fg(C_MUTED)),
            Span::styled(badge, Style::default().fg(C_MUTED)),
            Span::styled(label, Style::default().fg(C_MUTED)),
        ]))
    } else if is_sel {
        ListItem::new(Line::from(vec![
            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
            Span::styled(prefix, Style::default()),
            Span::styled(
                badge,
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                label,
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
        ]))
    } else {
        let badge_color = if on { C_SUCCESS } else { C_MUTED };
        ListItem::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(prefix, Style::default()),
            Span::styled(
                badge,
                Style::default()
                    .fg(badge_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(label, Style::default().fg(C_TEXT)),
        ]))
    }
}

/// Visual state for an inline label+value row (timezone selector, date field,
/// password, etc.).
///
/// - `Normal`   — not focused: label muted/text, value muted/text.
/// - `Selected` — focused but not editing: bold in C_ACCENT with `▸` prefix.
/// - `Editing`  — focused and editing: same as Selected, but the value renders
///   as `< value >` followed by the "↑↓ change • ↵ confirm" hint.
pub(in crate::tui::draw) enum FieldState {
    Normal,
    Selected,
    Editing,
}

pub(in crate::tui::draw) fn field_state(is_sel: bool, editing: bool) -> FieldState {
    if editing {
        FieldState::Editing
    } else if is_sel {
        FieldState::Selected
    } else {
        FieldState::Normal
    }
}

pub(in crate::tui::draw) fn inline_field_item(
    label: &str,
    value: &str,
    state: FieldState,
    label_w: usize,
) -> ListItem<'static> {
    let label_padded = format!("{:<width$}", label, width = label_w);
    let mut spans = focused_prefix(matches!(state, FieldState::Selected | FieldState::Editing));
    let accent_bold = Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD);
    match state {
        FieldState::Editing => {
            spans.push(Span::styled(label_padded, accent_bold));
            spans.push(Span::styled(
                format!("< {value} >"),
                Style::default().fg(C_ACCENT),
            ));
            spans.push(Span::styled(HINT_CYCLE_VALUE, Style::default().fg(C_MUTED)));
        }
        FieldState::Selected => {
            spans.push(Span::styled(label_padded, accent_bold));
            spans.push(Span::styled(value.to_string(), accent_bold));
        }
        FieldState::Normal => {
            spans.push(Span::styled(label_padded, Style::default().fg(C_TEXT)));
            spans.push(Span::styled(value.to_string(), Style::default().fg(C_TEXT)));
        }
    }
    ListItem::new(Line::from(spans))
}

fn focused_prefix(is_sel: bool) -> Vec<Span<'static>> {
    if is_sel {
        vec![Span::styled("▸ ", Style::default().fg(C_PRIMARY))]
    } else {
        vec![Span::raw("  ")]
    }
}

pub(in crate::tui::draw) fn del_item(is_sel: bool, detail: String) -> ListItem<'static> {
    if is_sel {
        ListItem::new(Line::from(vec![
            Span::styled("▸ ", Style::default().fg(C_DANGEROUS)),
            Span::styled(
                "[del]",
                Style::default()
                    .fg(C_DANGEROUS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(detail, Style::default().fg(C_TEXT)),
        ]))
    } else {
        ListItem::new(Line::from(vec![
            Span::raw("  "),
            Span::styled("[del]", Style::default().fg(C_MUTED)),
            Span::styled(detail, Style::default().fg(C_TEXT)),
        ]))
    }
}

pub(in crate::tui::draw) fn link_item(is_sel: bool, label: &str) -> ListItem<'static> {
    link_item_styled(is_sel, label, C_TEXT)
}

pub(in crate::tui::draw) fn sub_link_item(is_sel: bool, label: &str) -> ListItem<'static> {
    link_item_styled(is_sel, label, C_MUTED)
}

fn link_item_styled(
    is_sel: bool,
    label: &str,
    unselected_fg: ratatui::style::Color,
) -> ListItem<'static> {
    let label_style = if is_sel {
        Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(unselected_fg)
    };
    ListItem::new(Line::from(vec![
        Span::styled(
            if is_sel { "▸ " } else { "  " },
            Style::default().fg(C_PRIMARY),
        ),
        Span::styled(label.to_string(), label_style),
    ]))
}
