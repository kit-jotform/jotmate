use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::tui::app::{App, Screen};
use crate::tui::layout::{is_compact, HAlign, LayoutEngine, ScreenLayout, Widget, UI_WIDTH};
use crate::tui::palette::{C_ACCENT, C_LOGO, C_MUTED, C_SELECT, C_TEXT, C_WARN};
use crate::tui::widgets::{IconWidget, LOGO};

use super::DIVIDER_WIDTH;

const NAME_COL_W: u16 = 16;

const TAGLINES: &[&str] = &[
    "One command, rules all repos.",
    "Keep your RDS always in sync!",
    "How many hours are you behind?",
];
const TAGLINE_CYCLE_MS: u64 = 10000;
const TAGLINE_FADE_MS: u64 = 600;
const TAGLINE_GRAY_RAMP: &[u8] = &[232, 234, 236, 238, 240, 241, 242, 243];

fn current_tagline() -> (&'static str, Color) {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let phase = now_ms % TAGLINE_CYCLE_MS;
    let idx = ((now_ms / TAGLINE_CYCLE_MS) as usize) % TAGLINES.len();

    let (line_idx, alpha) = if phase < TAGLINE_FADE_MS / 2 {
        let out = phase as f32 / (TAGLINE_FADE_MS as f32 / 2.0);
        (
            (idx + TAGLINES.len() - 1) % TAGLINES.len(),
            (1.0 - out).max(0.0),
        )
    } else if phase < TAGLINE_FADE_MS {
        let inp = (phase - TAGLINE_FADE_MS / 2) as f32 / (TAGLINE_FADE_MS as f32 / 2.0);
        (idx, inp.min(1.0))
    } else {
        (idx, 1.0)
    };

    let last = TAGLINE_GRAY_RAMP.len() - 1;
    let ramp_idx = (alpha * last as f32).round() as usize;
    let color = Color::Indexed(TAGLINE_GRAY_RAMP[ramp_idx.min(last)]);
    (TAGLINES[line_idx], color)
}

pub fn draw_main_menu(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let engine = LayoutEngine::new(area);
    let compact = is_compact(area);
    let show_icon = !compact && area.width >= UI_WIDTH;

    let menu_height = app.main_menu_items().len() as u16;
    let header_h = if compact { 0 } else { 7 };
    let blank1_h = if compact { 0 } else { 1 };
    let tagline_h = if compact { 0 } else { 1 };
    let time_ver_h = if compact { 0 } else { 1 };
    let divider_h = if compact { 0 } else { 1 };
    let blank2_h = if compact { 0 } else { 1 };
    let rows = ScreenLayout::new()
        .row("header", header_h)
        .row("blank1", blank1_h)
        .row("tagline", tagline_h)
        .row("time_ver", time_ver_h)
        .row("divider", divider_h)
        .row("blank2", blank2_h)
        .row("sel_hdr", 1)
        .row("blank_sel", 1)
        .row("menu", menu_height)
        .row("blank3", 1)
        .row("hint", 1)
        .margin(1)
        .split(engine.clamp_area(area));

    let header_row = rows.get("header");
    if header_row.height > 0 {
        let logo_col = if show_icon {
            let header_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(14),
                    Constraint::Length(2),
                    Constraint::Min(0),
                ])
                .split(header_row);
            f.render_widget(IconWidget, header_cols[0]);
            header_cols[2]
        } else {
            header_row
        };

        let logo_area = Rect {
            y: logo_col.y + 1,
            height: 6,
            ..logo_col
        };
        let logo_lines: Vec<Line> = LOGO
            .iter()
            .map(|l| {
                Line::from(Span::styled(
                    *l,
                    Style::default().fg(C_LOGO).add_modifier(Modifier::BOLD),
                ))
            })
            .collect();
        f.render_widget(Paragraph::new(logo_lines), logo_area);
    }

    let divider_row = rows.get("divider");
    if divider_row.height > 0 {
        let divider = "─".repeat(DIVIDER_WIDTH as usize);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                divider,
                Style::default().fg(C_MUTED),
            ))),
            engine.center(DIVIDER_WIDTH, divider_row),
        );
    }

    let tagline_row = rows.get("tagline");
    if tagline_row.height > 0 {
        let (tagline, tagline_color) = current_tagline();
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                tagline,
                Style::default()
                    .fg(tagline_color)
                    .add_modifier(Modifier::ITALIC),
            ))),
            engine.center(tagline.chars().count() as u16, tagline_row),
        );
    }

    let time_ver_row = rows.get("time_ver");
    if time_ver_row.height > 0 {
        let now = Local::now().format("%H:%M").to_string();
        let version = env!("CARGO_PKG_VERSION");
        let time_str = format!("{}  |  v{}", now, version);
        let time_len = time_str.chars().count() as u16;
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(now, Style::default().fg(C_MUTED)),
                Span::styled("  |  ", Style::default().fg(C_MUTED)),
                Span::styled(format!("v{version}"), Style::default().fg(C_MUTED)),
            ])),
            engine.center(time_len, time_ver_row),
        );
    }

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "SELECT TOOL",
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (↓↑ navigate  •  ↵ submit)", Style::default().fg(C_MUTED)),
        ])),
        engine.place(&Widget::anon(UI_WIDTH, HAlign::Center), rows.get("sel_hdr")),
    );

    let menu_items = app.main_menu_items();
    let items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let selected = app.selected_index(Screen::MainMenu) == i;
            let name_padded = format!("{:<width$}", item.name, width = NAME_COL_W as usize);
            if selected {
                let mut spans = vec![
                    Span::styled("▸ ", Style::default().fg(C_SELECT)),
                    Span::styled(
                        name_padded,
                        Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD),
                    ),
                ];
                if !item.desc.is_empty() {
                    spans.push(Span::styled("— ", Style::default().fg(C_SELECT)));
                    spans.push(Span::styled(
                        item.desc.clone(),
                        Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD),
                    ));
                }
                ListItem::new(Line::from(spans))
            } else {
                let mut spans = vec![
                    Span::raw("  "),
                    Span::styled(name_padded, Style::default().fg(C_TEXT)),
                ];
                if !item.desc.is_empty() {
                    spans.push(Span::styled("— ", Style::default().fg(C_MUTED)));
                    spans.push(Span::styled(item.desc.clone(), Style::default().fg(C_TEXT)));
                }
                ListItem::new(Line::from(spans))
            }
        })
        .collect();

    f.render_stateful_widget(
        List::new(items),
        rows.get("menu"),
        &mut app.list_state(Screen::MainMenu).clone(),
    );

    let hint_line = if app.config_load_error.is_some() {
        Line::from(Span::styled(
            "⚠ config unreadable — using defaults; changes won't persist until fixed",
            Style::default().fg(C_WARN),
        ))
    } else {
        Line::from(Span::styled("q/Esc exit", Style::default().fg(C_MUTED)))
    };
    f.render_widget(Paragraph::new(hint_line), rows.get("hint"));
}
