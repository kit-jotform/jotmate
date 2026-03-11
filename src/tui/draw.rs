use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use super::app::{App, Screen};
use super::widgets::{IconWidget, LOGO, LOGO_SMALL};

// ── Palette ───────────────────────────────────────────────────────────────────

const C_LOGO: Color = Color::Indexed(189);   // lavender — original logo colour
const C_PRIMARY: Color = Color::LightMagenta;
const C_ACCENT: Color = Color::LightCyan;
const C_SELECT: Color = Color::Indexed(141); // medium purple — selection highlight
const C_SUCCESS: Color = Color::LightGreen;
const C_MUTED: Color = Color::DarkGray;
const C_TEXT: Color = Color::White;

// ── Main menu items: (name, description) ──────────────────────────────────────

const MAIN_ITEMS: &[(&str, &str)] = &[
    ("Sync",        "Sync repos to upstream"),
    ("Time Doctor", "Track your work hours"),
    ("Settings",    "Configure jotmate"),
    ("Exit",        ""),
];

const NAME_COL_W: u16 = 16; // fixed width for the name column

pub fn draw(f: &mut ratatui::Frame, app: &App) {
    match app.screen {
        Screen::MainMenu => draw_main_menu(f, app),
        Screen::Settings => draw_settings(f, app),
    }
}

fn draw_main_menu(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    // Layout: header | tagline | time-version | blank | divider | blank | select-header | menu | blank | hint
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // header (icon + logo)
            Constraint::Length(1), // tagline
            Constraint::Length(1), // time | version
            Constraint::Length(1), // blank
            Constraint::Length(1), // divider
            Constraint::Length(1), // blank
            Constraint::Length(1), // "SELECT TOOL" header
            Constraint::Length(4), // menu list (4 items)
            Constraint::Length(1), // blank
            Constraint::Length(1), // hint
        ])
        .margin(1)
        .split(area);

    // Header row: icon | gap | logo
    let header_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(rows[0]);

    // ── Icon ──
    f.render_widget(IconWidget, header_cols[0]);

    // ── Logo (lavender, vertically centred in 7-row area) ──
    let logo_area = Rect { y: header_cols[2].y + 1, height: 6, ..header_cols[2] };
    let logo_lines: Vec<Line> = LOGO
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(C_LOGO).add_modifier(Modifier::BOLD))))
        .collect();
    f.render_widget(Paragraph::new(logo_lines), logo_area);

    // Divider anchored at logo x, logo width
    let logo_x = header_cols[2].x;

    const DIV_W: u16 = 49;

    let centered = |row: Rect, text_len: u16| -> Rect {
        let pad = DIV_W.saturating_sub(text_len) / 2;
        Rect { x: logo_x + pad, width: DIV_W.min(text_len), ..row }
    };

    // ── Divider — fixed 49 chars ──
    let divider = "─".repeat(DIV_W as usize);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(divider.clone(), Style::default().fg(C_MUTED)))),
        Rect { x: logo_x, width: DIV_W, ..rows[4] },
    );

    // ── Tagline — centered within divider width ──
    let tagline = "The lazy engineer's Swiss Army knife";
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            tagline,
            Style::default().fg(C_MUTED).add_modifier(Modifier::ITALIC),
        ))),
        centered(rows[1], tagline.chars().count() as u16),
    );

    // ── Time | version — centered within divider width ──
    let now = Local::now().format("%H:%M").to_string();
    let version = env!("CARGO_PKG_VERSION");
    let time_str = format!("{}  |  v{}", now, version);
    let time_len = time_str.chars().count() as u16;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(now, Style::default().fg(C_MUTED)),
            Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("v{version}"), Style::default().fg(C_MUTED)),
        ])),
        centered(rows[2], time_len),
    );

    // rows[3] blank

    // rows[5] blank

    // ── "SELECT TOOL" header with keys ──
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("SELECT TOOL", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("  (←↓↑→ navigate  •  ↵ submit)", Style::default().fg(C_MUTED)),
        ])),
        rows[6],
    );

    // ── Menu list ──
    let items: Vec<ListItem> = MAIN_ITEMS
        .iter()
        .enumerate()
        .map(|(i, (name, desc))| {
            let selected = app.main_state.selected() == Some(i);
            if selected {
                let name_padded = format!("{:<width$}", name, width = NAME_COL_W as usize);
                let mut spans = vec![
                    Span::styled("▸ ", Style::default().fg(C_SELECT)),
                    Span::styled(name_padded, Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD)),
                ];
                if !desc.is_empty() {
                    spans.push(Span::styled("— ", Style::default().fg(C_SELECT)));
                    spans.push(Span::styled(*desc, Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD)));
                }
                ListItem::new(Line::from(spans))
            } else {
                let name_padded = format!("{:<width$}", name, width = NAME_COL_W as usize);
                let mut spans = vec![
                    Span::raw("  "),
                    Span::styled(name_padded, Style::default().fg(C_TEXT)),
                ];
                if !desc.is_empty() {
                    spans.push(Span::styled("— ", Style::default().fg(C_MUTED)));
                    spans.push(Span::styled(*desc, Style::default().fg(C_TEXT)));
                }
                ListItem::new(Line::from(spans))
            }
        })
        .collect();

    f.render_stateful_widget(List::new(items), rows[7], &mut app.main_state.clone());

    // rows[8] blank

    // ── Hint ──
    f.render_widget(
        Paragraph::new(Line::from(
            Span::styled("Esc exit", Style::default().fg(C_MUTED)),
        )),
        rows[9],
    );
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
