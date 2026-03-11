use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use super::app::{App, Screen, MAIN_ITEMS};
use super::widgets::{IconWidget, LOGO, LOGO_SMALL};

// ── Palette ───────────────────────────────────────────────────────────────────

const C_PRIMARY: Color = Color::Rgb(124, 58, 237);  // purple
const C_ACCENT: Color = Color::Rgb(244, 114, 182);  // pink
const C_SUCCESS: Color = Color::Rgb(16, 185, 129);  // green
const C_MUTED: Color = Color::Rgb(107, 114, 128);   // gray
const C_TEXT: Color = Color::Rgb(229, 231, 235);    // near-white

pub fn draw(f: &mut ratatui::Frame, app: &App) {
    match app.screen {
        Screen::MainMenu => draw_main_menu(f, app),
        Screen::Settings => draw_settings(f, app),
    }
}

fn draw_main_menu(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    // Vertical: header block | tagline | divider | hint | menu
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // icon (7 rows) / logo (6 rows) — take max
            Constraint::Length(1), // tagline
            Constraint::Length(1), // divider
            Constraint::Length(1), // hint
            Constraint::Min(0),    // menu list
        ])
        .margin(1)
        .split(area);

    // Header row: icon (14 cols + 2 gap) | logo text
    let header_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14), // icon width
            Constraint::Length(2),  // gap
            Constraint::Min(0),     // logo text
        ])
        .split(rows[0]);

    // ── Icon ──
    f.render_widget(IconWidget, header_cols[0]);

    // ── Logo text (vertically centered: logo is 6 lines, area is 7) ──
    let logo_area = Rect { y: header_cols[2].y + 1, height: 6, ..header_cols[2] };
    let logo_lines: Vec<Line> = LOGO
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD))))
        .collect();
    f.render_widget(Paragraph::new(logo_lines), logo_area);

    // ── Tagline ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "The lazy engineer's Swiss Army knife",
            Style::default().fg(C_MUTED).add_modifier(Modifier::ITALIC),
        ))),
        rows[1],
    );

    // ── Divider ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─────────────────────────────────────────────────",
            Style::default().fg(C_MUTED),
        ))),
        rows[2],
    );

    // ── Hint ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "↑↓ navigate  ·  Enter select  ·  Esc/q exit",
            Style::default().fg(C_MUTED),
        ))),
        rows[3],
    );

    // ── Menu list ──
    let items: Vec<ListItem> = MAIN_ITEMS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let selected = app.main_state.selected() == Some(i);
            let style = if selected {
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(C_TEXT)
            };
            let prefix = if selected { "▸ " } else { "  " };
            ListItem::new(Line::from(Span::styled(format!("{prefix}{label}"), style)))
        })
        .collect();

    f.render_stateful_widget(List::new(items), rows[4], &mut app.main_state.clone());
}

fn draw_settings(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // small logo
            Constraint::Length(1), // "Settings" title
            Constraint::Length(1), // divider
            Constraint::Length(1), // hint
            Constraint::Min(0),    // list
        ])
        .margin(1)
        .split(area);

    // ── Small logo ──
    let logo_lines: Vec<Line> = LOGO_SMALL
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(C_PRIMARY).add_modifier(Modifier::BOLD))))
        .collect();
    f.render_widget(Paragraph::new(logo_lines), chunks[0]);

    // ── Title ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Settings",
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        ))),
        chunks[1],
    );

    // ── Divider ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─────────────────────────────────────────────────",
            Style::default().fg(C_MUTED),
        ))),
        chunks[2],
    );

    // ── Hint ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "←↑↓→ navigate  ·  Enter/Space toggle  ·  Esc/Backspace back",
            Style::default().fg(C_MUTED),
        ))),
        chunks[3],
    );

    // ── Settings list ──
    let setting_items = app.settings_items();
    let count = setting_items.len();
    let selected = app.settings_state.selected().unwrap_or(0);

    let items: Vec<ListItem> = setting_items
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if label.starts_with("──") {
                // separator row
                return ListItem::new(Line::from(Span::styled(
                    label.clone(),
                    Style::default().fg(C_MUTED),
                )));
            }
            let is_back = i == count - 1;
            let is_sel = selected == i;

            if is_sel {
                let prefix = "▸ ";
                if label.starts_with('[') {
                    let (badge, rest) = label.split_at(5);
                    return ListItem::new(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(C_PRIMARY)),
                        Span::styled(badge.to_string(), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                        Span::styled(rest.to_string(), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                    ]));
                }
                return ListItem::new(Line::from(Span::styled(
                    format!("{prefix}{label}"),
                    Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                )));
            }

            if is_back {
                return ListItem::new(Line::from(Span::styled(
                    format!("  {label}"),
                    Style::default().fg(C_MUTED),
                )));
            }

            // Normal toggle row — color the badge
            let on = label.starts_with("[ON");
            let badge_color = if on { C_SUCCESS } else { C_MUTED };
            let (badge, rest) = label.split_at(5);
            ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(badge.to_string(), Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                Span::styled(rest.to_string(), Style::default().fg(C_TEXT)),
            ]))
        })
        .collect();

    f.render_stateful_widget(List::new(items), chunks[4], &mut app.settings_state.clone());
}
