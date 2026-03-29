use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use ratatui::widgets::{Block, Borders, Clear};

use super::app::{App, InputMode, RepoManagerRow, Screen, SettingRow, MAIN_ITEMS};
use super::layout::{HAlign, LayoutEngine, ScreenLayout, Widget, UI_WIDTH};
use super::widgets::{IconWidget, LOGO, LOGO_SMALL};

// ── Palette ───────────────────────────────────────────────────────────────────

const C_TEXT: Color = Color::Indexed(255);
const C_PRIMARY: Color = Color::Indexed(199); // medium purple — consistent across terminals
const C_ACCENT: Color = Color::Indexed(51);   // light cyan — consistent across terminals
const C_SELECT: Color = C_PRIMARY;
const C_SUCCESS: Color = Color::Indexed(10);    // bright green — consistent across terminals
const C_MUTED: Color = Color::Indexed(8);       // dark gray — consistent across terminals
const C_LOGO: Color = C_TEXT;
const C_DANGEROUS: Color = Color::Indexed(9);   // bright red — consistent across terminals

const NAME_COL_W: u16 = 16; // fixed width for the name column
const DIVIDER_WIDTH: u16 = 53;

// ── Shared list item helpers ───────────────────────────────────────────────────

fn back_item(is_sel: bool) -> ListItem<'static> {
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

fn toggle_item(is_sel: bool, on: bool, label: String) -> ListItem<'static> {
    let badge = if on { "[ON ] " } else { "[OFF] " };
    let badge_color = if on { C_SUCCESS } else { C_MUTED };
    if is_sel {
        ListItem::new(Line::from(vec![
            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
            Span::styled(
                badge,
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(label, Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        ]))
    } else {
        ListItem::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(badge, Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
            Span::styled(label, Style::default().fg(C_TEXT)),
        ]))
    }
}

pub fn draw(f: &mut ratatui::Frame, app: &App) {
    match app.screen {
        Screen::MainMenu => draw_main_menu(f, app),
        Screen::Settings => draw_settings(f, app),
        Screen::RepoManager => draw_repo_manager(f, app),
    }
}

fn draw_main_menu(f: &mut ratatui::Frame, app: &App) {
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

    // ── Logo (lavender, vertically centred in 7-row area) ──
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
                "  (←↓↑→ navigate  •  ↵ submit)",
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
            let selected = app.main_state.selected() == Some(i);
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
        &mut app.main_state.clone(),
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

fn draw_screen_header(
    f: &mut ratatui::Frame,
    engine: &LayoutEngine,
    logo_area: Rect,
    title_area: Rect,
    divider_area: Rect,
    title: &str,
    hint_spans: Vec<Span<'static>>,
) {
    // ── Small logo ──
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
    f.render_widget(
        Paragraph::new(logo_lines),
        engine.center(logo_w, logo_area),
    );

    // ── Title left, hint right ──
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

    // ── Divider ──
    let divider = "─".repeat(UI_WIDTH as usize);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            divider,
            Style::default().fg(C_MUTED),
        ))),
        engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), divider_area),
    );
}

fn draw_settings(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    let layout = ScreenLayout::new()
        .row("logo", 3)
        .row("blank1", 1)
        .row("title", 1)
        .row("divider", 1)
        .row("blank3", 1)
        .row("list", 0)
        .margin(1)
        .split(area);

    let engine = LayoutEngine::new(area.x);

    let hint_spans = vec![
        Span::styled("↑↓", Style::default().fg(C_MUTED)),
        Span::styled(" navigate  •  ", Style::default().fg(C_MUTED)),
        Span::styled("Space/↵", Style::default().fg(C_MUTED)),
        Span::styled(" toggle  •  ", Style::default().fg(C_MUTED)),
        Span::styled("⌫/Esc", Style::default().fg(C_MUTED)),
        Span::styled(" back", Style::default().fg(C_MUTED)),
    ];
    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        "Settings",
        hint_spans,
    );

    // ── Settings list ──
    let setting_rows = app.settings_items();
    let selected = app.settings_state.selected().unwrap_or(0);

    let items: Vec<ListItem> = setting_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                SettingRow::Blank => ListItem::new(Line::raw("")),

                SettingRow::Separator => ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "── Upstream Repositories ──────────────────────",
                        Style::default().fg(C_MUTED),
                    ),
                ])),

                SettingRow::Back => back_item(is_sel),

                SettingRow::ManageRepos => {
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
                        Span::styled("→ Manage upstream repos", style),
                    ]))
                }

                SettingRow::Toggle {
                    label, hint, on, ..
                } => {
                    let label_text = if hint.is_empty() {
                        label.to_string()
                    } else {
                        format!("{label}  ({hint})")
                    };
                    toggle_item(is_sel, *on, label_text)
                }

                SettingRow::RepoToggle { name, url, enabled } => {
                    toggle_item(is_sel, *enabled, format!("{name}  <{url}>"))
                }
            }
        })
        .collect();

    f.render_stateful_widget(
        List::new(items),
        layout.get("list"),
        &mut app.settings_state.clone(),
    );
}

fn draw_repo_manager(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    let layout = ScreenLayout::new()
        .row("logo", 3)
        .row("blank1", 1)
        .row("title", 1)
        .row("divider", 1)
        .row("blank2", 1)
        .row("list", 0)
        .margin(1)
        .split(area);

    let engine = LayoutEngine::new(area.x);

    let hint_spans: Vec<Span<'static>> = match &app.input_mode {
        InputMode::AddingRepo(_) => vec![
            Span::styled("↵", Style::default().fg(C_MUTED)),
            Span::styled(" confirm  •  ", Style::default().fg(C_MUTED)),
            Span::styled("Esc", Style::default().fg(C_MUTED)),
            Span::styled(" cancel", Style::default().fg(C_MUTED)),
        ],
        InputMode::ConfirmDelete(_) => vec![
            Span::styled("↵/y", Style::default().fg(C_MUTED)),
            Span::styled(" confirm  •  ", Style::default().fg(C_MUTED)),
            Span::styled("Esc/n", Style::default().fg(C_MUTED)),
            Span::styled(" cancel", Style::default().fg(C_MUTED)),
        ],
        InputMode::Normal => vec![
            Span::styled("↵", Style::default().fg(C_MUTED)),
            Span::styled(" select  •  ", Style::default().fg(C_MUTED)),
            Span::styled("⌫/Esc", Style::default().fg(C_MUTED)),
            Span::styled(" back", Style::default().fg(C_MUTED)),
        ],
    };
    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        "Manage Repos",
        hint_spans,
    );

    // ── Repo manager list ──
    let rm_rows = app.repo_manager_items();
    let selected = app.repo_manager_state.selected().unwrap_or(0);

    let items: Vec<ListItem> = rm_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                RepoManagerRow::Blank => ListItem::new(Line::raw("")),

                RepoManagerRow::Back => back_item(is_sel),

                RepoManagerRow::RepoDelete { name, url } => {
                    let detail = format!("  {name}  <{url}>");
                    if is_sel {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_DANGEROUS)),
                            Span::styled(
                                "[del]",
                                Style::default().fg(C_DANGEROUS).add_modifier(Modifier::BOLD),
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

                RepoManagerRow::AddUrl => match &app.input_mode {
                    InputMode::AddingRepo(buf) => {
                        let display = format!("  URL: {buf}_");
                        ListItem::new(Line::from(Span::styled(
                            display,
                            Style::default().fg(C_ACCENT),
                        )))
                    }
                    _ => {
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
                            Span::styled("+ Add upstream URL", style),
                        ]))
                    }
                },
            }
        })
        .collect();

    f.render_stateful_widget(
        List::new(items),
        layout.get("list"),
        &mut app.repo_manager_state.clone(),
    );

    // ── Confirmation dialog overlay ──
    if let InputMode::ConfirmDelete(name) = &app.input_mode {
        draw_confirm_delete(f, area, name);
    }
}

fn draw_confirm_delete(f: &mut ratatui::Frame, area: Rect, name: &str) {
    let msg = format!("Delete \"{}\"?", name);
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
            Line::from(Span::styled(
                msg,
                Style::default().fg(C_TEXT),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::styled(" ↵/y ", Style::default().fg(C_DANGEROUS).add_modifier(Modifier::BOLD)),
                Span::styled("delete  ", Style::default().fg(C_MUTED)),
                Span::styled(" Esc/n ", Style::default().fg(C_MUTED).add_modifier(Modifier::BOLD)),
                Span::styled("cancel", Style::default().fg(C_MUTED)),
            ]),
        ]),
        inner,
    );
}
