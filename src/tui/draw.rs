use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use ratatui::widgets::{Block, Borders, Clear};

use super::app::{
    AddCpFocus, App, CpListRow, InputMode, RepoManagerRow, Screen, SettingRow, TimeSettingRow,
    MAIN_ITEMS, WEEKLY_HOURS_OPTIONS,
};
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

fn link_item(is_sel: bool, label: &str) -> ListItem<'static> {
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
        Span::styled(label.to_string(), style),
    ]))
}

pub fn draw(f: &mut ratatui::Frame, app: &App) {
    match app.screen {
        Screen::MainMenu => draw_main_menu(f, app),
        Screen::Settings => draw_settings(f, app),
        Screen::RepoManager => draw_repo_manager(f, app),
        Screen::TimeDoctorSettings => draw_td_settings(f, app),
        Screen::ContractPeriods => draw_contract_periods(f, app),
        Screen::AddContractPeriod => draw_add_contract_period(f, app),
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

                SettingRow::Separator => {
                    let label = {
                        let rows = app.settings_items();
                        let sep_count = rows[..i]
                            .iter()
                            .filter(|r| matches!(r, SettingRow::Separator))
                            .count();
                        if sep_count == 0 {
                            "── RDS Sync ──────────────────────────────────"
                        } else {
                            "── Time Doctor ─────────────────────────────────"
                        }
                    };
                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(label, Style::default().fg(C_MUTED)),
                    ]))
                }

                SettingRow::Back => back_item(is_sel),

                SettingRow::ManageRepos => link_item(is_sel, "→ Manage upstream repos"),

                SettingRow::TimeDoctorSettings => link_item(is_sel, "→ Manage credentials"),

                SettingRow::ContractPeriodsLink => link_item(is_sel, "→ Manage contract periods"),

                SettingRow::TimezoneSelector { value } => {
                    let label_w = 18usize;
                    let label_padded = format!("{:<width$}", "Timezone", width = label_w);
                    if is_sel {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
                            Span::styled(
                                label_padded,
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                value.clone(),
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled("  (↑↓)", Style::default().fg(C_MUTED)),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(label_padded, Style::default().fg(C_TEXT)),
                            Span::styled(value.clone(), Style::default().fg(C_TEXT)),
                        ]))
                    }
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
        _ => vec![
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

                RepoManagerRow::RepoToggle { name, url, enabled } => {
                    toggle_item(is_sel, *enabled, format!("{name}  <{url}>"))
                }

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
        draw_confirm_dialog(f, area, &format!("Delete \"{}\"?", name));
    }
}

fn draw_td_settings(f: &mut ratatui::Frame, app: &App) {
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

    let editing = matches!(&app.input_mode, InputMode::EditingField { .. });
    let hint_spans: Vec<Span<'static>> = if editing {
        vec![
            Span::styled("↵", Style::default().fg(C_MUTED)),
            Span::styled(" save  •  ", Style::default().fg(C_MUTED)),
            Span::styled("Esc", Style::default().fg(C_MUTED)),
            Span::styled(" cancel", Style::default().fg(C_MUTED)),
        ]
    } else {
        vec![
            Span::styled("↵", Style::default().fg(C_MUTED)),
            Span::styled(" edit  •  ", Style::default().fg(C_MUTED)),
            Span::styled("⌫/Esc", Style::default().fg(C_MUTED)),
            Span::styled(" back", Style::default().fg(C_MUTED)),
        ]
    };

    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        "Manage Credentials",
        hint_spans,
    );

    let td_rows = app.td_settings_items();
    let selected = app.td_settings_state.selected().unwrap_or(0);

    let items: Vec<ListItem> = td_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                TimeSettingRow::Blank => ListItem::new(Line::raw("")),

                TimeSettingRow::Back => back_item(is_sel),

                TimeSettingRow::Password { is_set } => {
                    let badge = if *is_set { "[set]    " } else { "[not set]" };
                    let badge_color = if *is_set { C_SUCCESS } else { C_MUTED };
                    if is_sel {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
                            Span::styled(
                                badge,
                                Style::default().fg(badge_color).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                " Password",
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(badge, Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                            Span::styled(" Password", Style::default().fg(C_TEXT)),
                        ]))
                    }
                }

                TimeSettingRow::EditField { field, label, value, masked } => {
                    let active_buf = match &app.input_mode {
                        InputMode::EditingField { field: f, buf } if f == field => Some(buf.as_str()),
                        _ => None,
                    };

                    let display_value = if let Some(buf) = active_buf {
                        if *masked {
                            format!("{}_ ", "*".repeat(buf.len()))
                        } else {
                            format!("{buf}_ ")
                        }
                    } else if value.is_empty() {
                        "—".to_string()
                    } else if *masked {
                        "*".repeat(value.len())
                    } else {
                        value.clone()
                    };

                    let label_w = 18usize;
                    let label_padded = format!("{:<width$}", label, width = label_w);

                    if is_sel || active_buf.is_some() {
                        let value_style = if active_buf.is_some() {
                            Style::default().fg(C_ACCENT)
                        } else {
                            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)
                        };
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
                            Span::styled(label_padded, Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                            Span::styled(display_value, value_style),
                        ]))
                    } else {
                        let value_style = if value.is_empty() {
                            Style::default().fg(C_MUTED)
                        } else {
                            Style::default().fg(C_TEXT)
                        };
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(label_padded, Style::default().fg(C_TEXT)),
                            Span::styled(display_value, value_style),
                        ]))
                    }
                }
            }
        })
        .collect();

    f.render_stateful_widget(
        List::new(items),
        layout.get("list"),
        &mut app.td_settings_state.clone(),
    );
}

fn draw_contract_periods(f: &mut ratatui::Frame, app: &App) {
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
        InputMode::ConfirmDeletePeriod(_) => vec![
            Span::styled("↵/y", Style::default().fg(C_MUTED)),
            Span::styled(" confirm  •  ", Style::default().fg(C_MUTED)),
            Span::styled("Esc/n", Style::default().fg(C_MUTED)),
            Span::styled(" cancel", Style::default().fg(C_MUTED)),
        ],
        _ => vec![
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
        "Contract Periods",
        hint_spans,
    );

    let cp_rows = app.cp_list_items();
    let selected = app.cp_list_state.selected().unwrap_or(0);

    let items: Vec<ListItem> = cp_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                CpListRow::Blank => ListItem::new(Line::raw("")),
                CpListRow::Back => back_item(is_sel),
                CpListRow::AddPeriod => link_item(is_sel, "+ Add period"),
                CpListRow::Period { from, weekly_hours, .. } => {
                    let hours_display = if weekly_hours.fract() == 0.0 {
                        format!("{}h/week", *weekly_hours as u32)
                    } else {
                        format!("{weekly_hours}h/week")
                    };
                    let detail = format!("{}  {}", from, hours_display);
                    if is_sel {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_DANGEROUS)),
                            Span::styled(
                                "[del]",
                                Style::default().fg(C_DANGEROUS).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(format!("  {detail}"), Style::default().fg(C_TEXT)),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled("[del]", Style::default().fg(C_MUTED)),
                            Span::styled(format!("  {detail}"), Style::default().fg(C_TEXT)),
                        ]))
                    }
                }
            }
        })
        .collect();

    f.render_stateful_widget(
        List::new(items),
        layout.get("list"),
        &mut app.cp_list_state.clone(),
    );

    // ── Confirmation dialog overlay ──
    if let InputMode::ConfirmDeletePeriod(idx) = &app.input_mode {
        if let Some(p) = app.contract_periods.get(*idx) {
            draw_confirm_dialog(f, area, &format!("Delete period {}?", p.from));
        }
    }
}

fn draw_add_contract_period(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    let layout = ScreenLayout::new()
        .row("logo", 3)
        .row("blank1", 1)
        .row("title", 1)
        .row("divider", 1)
        .row("blank2", 1)
        .row("monday", 1)
        .row("hours", 1)
        .row("blank3", 1)
        .row("hint", 1)
        .row("fill", 0)
        .margin(1)
        .split(area);

    let engine = LayoutEngine::new(area.x);

    let hint_spans = vec![
        Span::styled("↑↓", Style::default().fg(C_MUTED)),
        Span::styled(" change  •  ", Style::default().fg(C_MUTED)),
        Span::styled("Tab", Style::default().fg(C_MUTED)),
        Span::styled(" switch  •  ", Style::default().fg(C_MUTED)),
        Span::styled("↵", Style::default().fg(C_MUTED)),
        Span::styled(" save  •  ", Style::default().fg(C_MUTED)),
        Span::styled("Esc", Style::default().fg(C_MUTED)),
        Span::styled(" cancel", Style::default().fg(C_MUTED)),
    ];

    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        "Add Contract Period",
        hint_spans,
    );

    let label_w = 18usize;
    let monday_focused = app.add_cp_focus == AddCpFocus::Monday;
    let hours_focused = app.add_cp_focus == AddCpFocus::Hours;

    // ── Monday row ──
    let monday_label = format!("{:<width$}", "From Monday", width = label_w);
    let monday_value = app.add_cp_monday.to_string();
    let monday_line = if monday_focused {
        Line::from(vec![
            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
            Span::styled(
                monday_label,
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                monday_value,
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (↑↓)", Style::default().fg(C_MUTED)),
        ])
    } else {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(monday_label, Style::default().fg(C_TEXT)),
            Span::styled(monday_value, Style::default().fg(C_TEXT)),
        ])
    };
    f.render_widget(
        Paragraph::new(monday_line),
        layout.get("monday"),
    );

    // ── Hours row ──
    let hours_label = format!("{:<width$}", "Weekly hours", width = label_w);
    let hours_val = WEEKLY_HOURS_OPTIONS[app.add_cp_hours_idx];
    let hours_display = if hours_val.fract() == 0.0 {
        format!("{}h", hours_val as u32)
    } else {
        format!("{hours_val}h")
    };
    let hours_line = if hours_focused {
        Line::from(vec![
            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
            Span::styled(
                hours_label,
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                hours_display,
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (↑↓)", Style::default().fg(C_MUTED)),
        ])
    } else {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(hours_label, Style::default().fg(C_TEXT)),
            Span::styled(hours_display, Style::default().fg(C_TEXT)),
        ])
    };
    f.render_widget(
        Paragraph::new(hours_line),
        layout.get("hours"),
    );
}

fn draw_confirm_dialog(f: &mut ratatui::Frame, area: Rect, msg: &str) {
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
                msg.to_string(),
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
