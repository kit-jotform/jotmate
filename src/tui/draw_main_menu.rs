use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use super::app::{App, Screen, MAIN_ITEMS};
use super::draw::DIVIDER_WIDTH;
use super::layout::{HAlign, LayoutEngine, ScreenLayout, Widget, UI_WIDTH};
use super::palette::{C_ACCENT, C_LOGO, C_MUTED, C_SELECT, C_TEXT};
use super::widgets::{IconWidget, LOGO};

const NAME_COL_W: u16 = 16;

pub fn draw_main_menu(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    let rows = ScreenLayout::new()
        .row("header", 7)
        .row("blank1", 1)
        .row("tagline", 1)
        .row("time_ver", 1)
        .row("divider", 1)
        .row("blank2", 1)
        .row("sel_hdr", 1)
        .row("blank_sel", 1)
        .row("menu", 4)
        .row("blank3", 1)
        .row("hint", 1)
        .margin(1)
        .split(area);

    let engine = LayoutEngine::new(area.x);

    // Header row: icon | gap | logo
    let header_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(rows.get("header"));

    // ── Icon ──
    f.render_widget(IconWidget, header_cols[0]);

    // ── Logo (vertically centred in 7-row area) ──
    let logo_area = Rect {
        y: header_cols[2].y + 1,
        height: 6,
        ..header_cols[2]
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

    // ── Divider ──
    let divider = "─".repeat(DIVIDER_WIDTH as usize);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            divider.clone(),
            Style::default().fg(C_MUTED),
        ))),
        engine.center(DIVIDER_WIDTH, rows.get("divider")),
    );

    // ── Tagline ──
    let tagline = "The lazy engineer's Swiss Army knife";
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            tagline,
            Style::default().fg(C_MUTED).add_modifier(Modifier::ITALIC),
        ))),
        engine.center(tagline.chars().count() as u16, rows.get("tagline")),
    );

    // ── Time | version ──
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
        engine.center(time_len, rows.get("time_ver")),
    );

    // ── "SELECT TOOL" header with keys ──
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "SELECT TOOL",
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  (↓↑ navigate  •  ↵ submit)",
                Style::default().fg(C_MUTED),
            ),
        ])),
        engine.place(&Widget::anon(UI_WIDTH, HAlign::Center), rows.get("sel_hdr")),
    );

    // ── Menu list ──
    let items: Vec<ListItem> = MAIN_ITEMS
        .iter()
        .enumerate()
        .map(|(i, (name, desc))| {
            let selected = app.selected_index(Screen::MainMenu) == i;
            if selected {
                let name_padded = format!("{:<width$}", name, width = NAME_COL_W as usize);
                let mut spans = vec![
                    Span::styled("▸ ", Style::default().fg(C_SELECT)),
                    Span::styled(
                        name_padded,
                        Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD),
                    ),
                ];
                if !desc.is_empty() {
                    spans.push(Span::styled("— ", Style::default().fg(C_SELECT)));
                    spans.push(Span::styled(
                        *desc,
                        Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD),
                    ));
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

    f.render_stateful_widget(
        List::new(items),
        rows.get("menu"),
        &mut app.list_state(Screen::MainMenu).clone(),
    );

    // ── Hint ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "q/Esc exit",
            Style::default().fg(C_MUTED),
        ))),
        rows.get("hint"),
    );
}
