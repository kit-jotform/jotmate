use chrono::NaiveDate;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, ListItem, Paragraph},
};

use super::app::{App, Screen};
use super::layout::{HAlign, LayoutEngine, RowMap, ScreenLayout, Widget, UI_WIDTH};
use super::palette::{C_ACCENT, C_DANGEROUS, C_MUTED, C_PRIMARY, C_SUCCESS, C_TEXT};
use super::widgets::LOGO_SMALL;

pub(super) use draw_main_menu::draw_main_menu;
pub(super) use draw_repos::{draw_remove_repos, draw_repo_manager};
pub(super) use draw_settings::{draw_general_toggles, draw_settings};
pub(super) use draw_td_report::draw_td_report;
pub(super) use draw_time::{draw_contract_periods, draw_td_settings};

use super::{draw_main_menu, draw_repos, draw_settings, draw_td_report, draw_time};

// ── Shared constants ──────────────────────────────────────────────────────────

pub(super) fn fmt_date(d: NaiveDate) -> String {
    d.format("%d-%m-%Y").to_string()
}

pub(super) const DIVIDER_WIDTH: u16 = 53;
pub(super) const HINT_CYCLE_VALUE: &str = "  ↑↓ change  •  ↵ confirm  •  ⌫ cancel";
/// Visible char width of a settings-screen section separator line (e.g. `"── RDS Sync ───…"`).
pub(super) const SEPARATOR_WIDTH: usize = 46;

// ── Shared hint builders ──────────────────────────────────────────────────────

pub(super) fn hint_muted(parts: &[&str]) -> Vec<Span<'static>> {
    parts
        .iter()
        .map(|s| Span::styled(s.to_string(), Style::default().fg(C_MUTED)))
        .collect()
}

/// Standard sub-screen layout: small logo, title bar, divider, then a fill area for lists.
pub(super) fn sub_screen_layout(area: Rect) -> RowMap {
    ScreenLayout::new()
        .row("logo", 3)
        .row("blank1", 1)
        .row("title", 1)
        .row("divider", 1)
        .row("blank2", 1)
        .row("list", 0)
        .margin(1)
        .split(area)
}

pub(super) fn hint_navigate_toggle() -> Vec<Span<'static>> {
    hint_muted(&[
        "↑↓",
        " navigate  •  ",
        "↵",
        " toggle  •  ",
        "⌫/Esc",
        " back",
    ])
}

pub(super) fn hint_select_back() -> Vec<Span<'static>> {
    hint_muted(&["↵", " select  •  ", "⌫/Esc", " back"])
}

pub(super) fn hint_confirm_cancel() -> Vec<Span<'static>> {
    hint_muted(&["↵/y", " confirm  •  ", "Esc/n", " cancel"])
}

pub(super) fn hint_input_confirm() -> Vec<Span<'static>> {
    hint_muted(&["↵", " confirm  •  ", "Esc", " cancel"])
}

// ── Shared list item helpers ───────────────────────────────────────────────────

/// Build a muted section-separator list item: `"── {label} ────────────"`.
/// The total visible width of the separator string is `SEPARATOR_WIDTH` chars.
pub(super) fn separator_item(label: &str) -> ListItem<'static> {
    // "── " + label + " " = prefix; fill the rest with "─"
    let prefix_chars = 3 + label.chars().count() + 1;
    let dashes = "─".repeat(SEPARATOR_WIDTH.saturating_sub(prefix_chars));
    let text = format!("── {} {}", label, dashes);
    ListItem::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(text, Style::default().fg(C_MUTED)),
    ]))
}

pub(super) fn back_item(is_sel: bool) -> ListItem<'static> {
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

pub(super) fn toggle_item(
    is_sel: bool,
    on: bool,
    label: String,
    indent: bool,
    disabled: bool,
) -> ListItem<'static> {
    let prefix = if indent { "    " } else { "" };
    let badge = if on { "[ON ] " } else { "[OFF] " };
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

/// Visual state for an inline label+value row (timezone selector, date field, password, etc.).
///
/// - `Normal`   — not focused: label muted/text, value muted/text.
/// - `Selected` — focused but not being edited: label+value bold in C_ACCENT with `▸` prefix.
/// - `Editing`  — focused and being edited: same as Selected, but the value renders as
///   `< value >` followed by the "↑↓ change • ↵ confirm" hint.
pub(super) enum FieldState {
    Normal,
    Selected,
    Editing,
}

/// Render one inline label+value row with consistent state visuals.
/// `label_w` is the fixed-width padding for the label column.
pub(super) fn inline_field_item(
    label: &str,
    value: &str,
    state: FieldState,
    label_w: usize,
) -> ListItem<'static> {
    let label_padded = format!("{:<width$}", label, width = label_w);
    match state {
        FieldState::Editing => ListItem::new(Line::from(vec![
            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
            Span::styled(
                label_padded,
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("< {value} >"), Style::default().fg(C_ACCENT)),
            Span::styled(HINT_CYCLE_VALUE, Style::default().fg(C_MUTED)),
        ])),
        FieldState::Selected => ListItem::new(Line::from(vec![
            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
            Span::styled(
                label_padded,
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                value.to_string(),
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
        ])),
        FieldState::Normal => ListItem::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(label_padded, Style::default().fg(C_TEXT)),
            Span::styled(value.to_string(), Style::default().fg(C_TEXT)),
        ])),
    }
}

pub(super) const FIELD_LABEL_W: usize = 18;
/// Narrower label column for the timezone selector — the inline editing hint
/// (↑↓ change • ↵ confirm • ⌫ cancel) needs the extra room to fit in UI_WIDTH.
pub(super) const FIELD_LABEL_W_TZ: usize = 13;

pub(super) fn link_item(is_sel: bool, label: &str) -> ListItem<'static> {
    link_item_styled(is_sel, label, C_TEXT)
}

/// Like `link_item` but renders the unselected label in `C_MUTED` — used for
/// sub-actions (e.g. "+ Add upstream URL", "→ Remove Repos") that should read as
/// secondary affordances until focused.
pub(super) fn sub_link_item(is_sel: bool, label: &str) -> ListItem<'static> {
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

pub(super) fn draw_screen_header(
    f: &mut ratatui::Frame,
    engine: &LayoutEngine,
    logo_area: Rect,
    title_area: Rect,
    divider_area: Rect,
    title: &str,
    hint_spans: Vec<Span<'static>>,
) {
    let logo_w = LOGO_SMALL[0].chars().count() as u16;
    let logo_lines: Vec<Line> = LOGO_SMALL
        .iter()
        .map(|l| {
            Line::from(Span::styled(
                *l,
                Style::default().fg(C_PRIMARY).add_modifier(Modifier::BOLD),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(logo_lines), engine.center(logo_w, logo_area));

    let title_row = engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), title_area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title,
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        ))),
        title_row,
    );
    f.render_widget(
        Paragraph::new(Line::from(hint_spans)).right_aligned(),
        title_row,
    );

    let divider = "─".repeat(UI_WIDTH as usize);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            divider,
            Style::default().fg(C_MUTED),
        ))),
        engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), divider_area),
    );
}

pub(super) fn draw_confirm_dialog(f: &mut ratatui::Frame, area: Rect, msg: &str) {
    let dialog_w = (msg.len() as u16 + 4).max(26);
    let dialog_h = 5u16;

    let x = area.x + area.width.saturating_sub(dialog_w) / 2;
    let y = area.y + area.height.saturating_sub(dialog_h) / 2;
    let dialog_area = Rect::new(x, y, dialog_w, dialog_h);

    f.render_widget(Clear, dialog_area);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_DANGEROUS))
            .title(Span::styled(" Confirm ", Style::default().fg(C_DANGEROUS))),
        dialog_area,
    );

    let inner = Rect::new(
        dialog_area.x + 1,
        dialog_area.y + 1,
        dialog_area.width - 2,
        dialog_area.height - 2,
    );

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(msg.to_string(), Style::default().fg(C_TEXT))),
            Line::raw(""),
            Line::from(vec![
                Span::styled(
                    " ↵/y ",
                    Style::default()
                        .fg(C_DANGEROUS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("delete  ", Style::default().fg(C_MUTED)),
                Span::styled(
                    " Esc/n ",
                    Style::default().fg(C_MUTED).add_modifier(Modifier::BOLD),
                ),
                Span::styled("cancel", Style::default().fg(C_MUTED)),
            ]),
        ]),
        inner,
    );
}

// ── Dispatcher ────────────────────────────────────────────────────────────────

pub fn draw(f: &mut ratatui::Frame, app: &App) {
    match app.screen {
        Screen::MainMenu => draw_main_menu(f, app),
        Screen::Settings => draw_settings(f, app),
        Screen::SyncGeneralSettings => draw_general_toggles(f, app, "RDS Sync", true),
        Screen::RepoManager => draw_repo_manager(f, app),
        Screen::RemoveRepos => draw_remove_repos(f, app),
        Screen::TdGeneralSettings => draw_general_toggles(f, app, "Time Doctor", false),
        Screen::TimeDoctorSettings => draw_td_settings(f, app),
        Screen::ContractPeriods => draw_contract_periods(f, app),
        Screen::TimeDoctorReport => draw_td_report(f, app),
        Screen::SyncProgress => super::sync_screen::draw_sync_progress(f, app),
    }
}
