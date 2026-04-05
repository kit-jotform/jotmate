use chrono::{Local, NaiveDate};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use ratatui::widgets::{Block, Borders, Clear};

use super::app::{
    App, CpListRow, GeneralToggleRow, InputMode, RemoveRepoRow, RepoManagerRow, Screen, SettingRow,
    TimeSettingRow, MAIN_ITEMS, WEEKLY_HOURS_OPTIONS,
};
use super::layout::{HAlign, LayoutEngine, RowMap, ScreenLayout, Widget, UI_WIDTH};
use super::widgets::{IconWidget, LOGO, LOGO_SMALL};

// ── Palette ───────────────────────────────────────────────────────────────────

const C_TEXT: Color = Color::Indexed(255);
const C_PRIMARY: Color = Color::Indexed(199); // medium purple — consistent across terminals
const C_ACCENT: Color = Color::Indexed(51); // light cyan — consistent across terminals
const C_SELECT: Color = C_PRIMARY;
const C_SUCCESS: Color = Color::Indexed(10); // bright green — consistent across terminals
const C_MUTED: Color = Color::Indexed(8); // dark gray — consistent across terminals
const C_LOGO: Color = C_TEXT;
const C_DANGEROUS: Color = Color::Indexed(9); // bright red — consistent across terminals

fn fmt_date(d: NaiveDate) -> String {
    d.format("%d-%m-%Y").to_string()
}

const NAME_COL_W: u16 = 16; // fixed width for the name column
const DIVIDER_WIDTH: u16 = 53;

// ── Shared hint builders ──────────────────────────────────────────────────────

fn hint_muted(parts: &[&str]) -> Vec<Span<'static>> {
    parts
        .iter()
        .map(|s| Span::styled(s.to_string(), Style::default().fg(C_MUTED)))
        .collect()
}

/// Standard sub-screen layout: small logo, title bar, divider, then a fill area for lists.
fn sub_screen_layout(area: Rect) -> RowMap {
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

fn hint_navigate_toggle() -> Vec<Span<'static>> {
    hint_muted(&[
        "↑↓",
        " navigate  •  ",
        "Space/↵",
        " toggle  •  ",
        "⌫/Esc",
        " back",
    ])
}

fn hint_select_back() -> Vec<Span<'static>> {
    hint_muted(&["↵", " select  •  ", "⌫/Esc", " back"])
}

fn hint_confirm_cancel() -> Vec<Span<'static>> {
    hint_muted(&["↵/y", " confirm  •  ", "Esc/n", " cancel"])
}

fn hint_input_confirm() -> Vec<Span<'static>> {
    hint_muted(&["↵", " confirm  •  ", "Esc", " cancel"])
}

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

fn toggle_item(
    is_sel: bool,
    on: bool,
    label: String,
    indent: bool,
    disabled: bool,
) -> ListItem<'static> {
    let prefix = if indent { "    " } else { "" };
    let badge = if on { "[ON ] " } else { "[OFF] " };
    if disabled {
        // Disabled items are fully muted
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

fn link_item(is_sel: bool, label: &str) -> ListItem<'static> {
    let style = if is_sel {
        Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(C_TEXT)
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
        Screen::SyncGeneralSettings => draw_general_toggles(f, app, "RDS Sync", true),
        Screen::RepoManager => draw_repo_manager(f, app),
        Screen::RemoveRepos => draw_remove_repos(f, app),
        Screen::TdGeneralSettings => draw_general_toggles(f, app, "Time Doctor", false),
        Screen::TimeDoctorSettings => draw_td_settings(f, app),
        Screen::ContractPeriods => draw_contract_periods(f, app),
        Screen::SyncProgress => super::sync_screen::draw_sync_progress(f, app),
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
    f.render_widget(Paragraph::new(logo_lines), engine.center(logo_w, logo_area));

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
    let layout = sub_screen_layout(area);
    let engine = LayoutEngine::new(area.x);

    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        "Settings",
        hint_navigate_toggle(),
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

                SettingRow::SyncGeneralLink => link_item(is_sel, "→ General settings"),

                SettingRow::ManageRepos => link_item(is_sel, "→ Manage upstream repos"),

                SettingRow::TdGeneralLink => link_item(is_sel, "→ General settings"),

                SettingRow::TimeDoctorSettings => link_item(is_sel, "→ Manage credentials"),

                SettingRow::ContractPeriodsLink => link_item(is_sel, "→ Manage contract periods"),
            }
        })
        .collect();

    f.render_stateful_widget(
        List::new(items),
        layout.get("list"),
        &mut app.settings_state.clone(),
    );
}

fn draw_general_toggles(f: &mut ratatui::Frame, app: &App, title: &str, is_sync: bool) {
    let area = f.area();
    let layout = sub_screen_layout(area);
    let engine = LayoutEngine::new(area.x);

    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        title,
        hint_navigate_toggle(),
    );

    let rows = if is_sync {
        app.sync_general_items()
    } else {
        app.td_general_items()
    };
    let state = if is_sync {
        &app.sync_general_state
    } else {
        &app.td_general_state
    };
    let selected = state.selected().unwrap_or(0);

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                GeneralToggleRow::Blank => ListItem::new(Line::raw("")),
                GeneralToggleRow::Back => back_item(is_sel),
                GeneralToggleRow::Toggle {
                    label,
                    hint,
                    on,
                    indent,
                    disabled,
                    ..
                } => {
                    let label_text = if hint.is_empty() {
                        label.to_string()
                    } else {
                        format!("{label}  ({hint})")
                    };
                    toggle_item(is_sel, *on, label_text, *indent, *disabled)
                }
                GeneralToggleRow::TimezoneSelector { value } => {
                    let label_w = 18usize;
                    let label_padded = format!("{:<width$}", "Timezone", width = label_w);
                    let selecting = matches!(app.input_mode, InputMode::SelectingTimezone);
                    if selecting {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
                            Span::styled(
                                label_padded,
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(format!("< {} >", value), Style::default().fg(C_ACCENT)),
                            Span::styled("  ↑↓ change  •  ↵ confirm", Style::default().fg(C_MUTED)),
                        ]))
                    } else if is_sel {
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
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(label_padded, Style::default().fg(C_TEXT)),
                            Span::styled(value.clone(), Style::default().fg(C_TEXT)),
                        ]))
                    }
                }
            }
        })
        .collect();

    f.render_stateful_widget(List::new(items), layout.get("list"), &mut state.clone());
}

fn draw_repo_manager(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let layout = sub_screen_layout(area);
    let engine = LayoutEngine::new(area.x);

    let hint_spans = match &app.input_mode {
        InputMode::AddingRepo(_) => hint_input_confirm(),
        _ => hint_select_back(),
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
                    toggle_item(is_sel, *enabled, format!("{name}  <{url}>"), false, false)
                }

                RepoManagerRow::RemoveReposLink => {
                    let style = if is_sel {
                        Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(C_MUTED)
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            if is_sel { "▸ " } else { "  " },
                            Style::default().fg(if is_sel { C_PRIMARY } else { C_MUTED }),
                        ),
                        Span::styled("→ Remove Repos", style),
                    ]))
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
}

fn draw_remove_repos(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let layout = sub_screen_layout(area);
    let engine = LayoutEngine::new(area.x);

    let hint_spans = match &app.input_mode {
        InputMode::ConfirmDelete(_) => hint_confirm_cancel(),
        _ => hint_select_back(),
    };
    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        "Remove Repos",
        hint_spans,
    );

    let rows = app.remove_repo_items();
    let selected = app.remove_repo_state.selected().unwrap_or(0);

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                RemoveRepoRow::Blank => ListItem::new(Line::raw("")),
                RemoveRepoRow::Back => back_item(is_sel),
                RemoveRepoRow::RepoDelete { name, url } => {
                    let detail = format!("  {name}  <{url}>");
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
            }
        })
        .collect();

    f.render_stateful_widget(
        List::new(items),
        layout.get("list"),
        &mut app.remove_repo_state.clone(),
    );

    // ── Confirmation dialog overlay ──
    if let InputMode::ConfirmDelete(name) = &app.input_mode {
        draw_confirm_dialog(f, area, &format!("Delete \"{}\"?", name));
    }
}

fn draw_td_settings(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let layout = sub_screen_layout(area);
    let engine = LayoutEngine::new(area.x);

    let hint_spans = if matches!(&app.input_mode, InputMode::EditingField { .. }) {
        hint_muted(&["↵", " save  •  ", "Esc", " cancel"])
    } else {
        hint_muted(&["↵", " edit  •  ", "⌫/Esc", " back"])
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
                                Style::default()
                                    .fg(badge_color)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                " Password",
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(
                                badge,
                                Style::default()
                                    .fg(badge_color)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(" Password", Style::default().fg(C_TEXT)),
                        ]))
                    }
                }

                TimeSettingRow::EditField {
                    field,
                    label,
                    value,
                    masked,
                } => {
                    let active_buf = match &app.input_mode {
                        InputMode::EditingField { field: f, buf } if f == field => {
                            Some(buf.as_str())
                        }
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
                            Span::styled(
                                label_padded,
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
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
    let layout = sub_screen_layout(area);
    let engine = LayoutEngine::new(area.x);

    let hint_spans = match &app.input_mode {
        InputMode::ConfirmDeletePeriod(_) => hint_confirm_cancel(),
        InputMode::EditingCpMonday | InputMode::EditingCpHours => {
            hint_muted(&["↑↓", " change  •  ", "↵", " confirm"])
        }
        _ => hint_select_back(),
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
    let label_w = 18usize;

    let items: Vec<ListItem> = cp_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                CpListRow::Blank => ListItem::new(Line::raw("")),
                CpListRow::Back => back_item(is_sel),
                CpListRow::SavePeriod => link_item(is_sel, "Save period"),
                CpListRow::MondayField => {
                    let label = format!("{:<width$}", "From Monday", width = label_w);
                    let value = fmt_date(app.add_cp_monday);
                    let editing = matches!(app.input_mode, InputMode::EditingCpMonday);
                    if editing {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
                            Span::styled(
                                label,
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(format!("< {value} >"), Style::default().fg(C_ACCENT)),
                            Span::styled("  ↑↓ change  •  ↵ confirm", Style::default().fg(C_MUTED)),
                        ]))
                    } else if is_sel {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
                            Span::styled(
                                label,
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                value,
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(label, Style::default().fg(C_TEXT)),
                            Span::styled(value, Style::default().fg(C_TEXT)),
                        ]))
                    }
                }
                CpListRow::HoursField => {
                    let label = format!("{:<width$}", "Weekly hours", width = label_w);
                    let hours_val = WEEKLY_HOURS_OPTIONS[app.add_cp_hours_idx];
                    let value = if hours_val.fract() == 0.0 {
                        format!("{}h", hours_val as u32)
                    } else {
                        format!("{hours_val}h")
                    };
                    let editing = matches!(app.input_mode, InputMode::EditingCpHours);
                    if editing {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
                            Span::styled(
                                label,
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(format!("< {value} >"), Style::default().fg(C_ACCENT)),
                            Span::styled("  ↑↓ change  •  ↵ confirm", Style::default().fg(C_MUTED)),
                        ]))
                    } else if is_sel {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
                            Span::styled(
                                label,
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                value,
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(label, Style::default().fg(C_TEXT)),
                            Span::styled(value, Style::default().fg(C_TEXT)),
                        ]))
                    }
                }
                CpListRow::Period {
                    from, weekly_hours, ..
                } => {
                    let hours_display = if weekly_hours.fract() == 0.0 {
                        format!("{}h/week", *weekly_hours as u32)
                    } else {
                        format!("{weekly_hours}h/week")
                    };
                    let detail = format!("{}  {}", fmt_date(*from), hours_display);
                    if is_sel {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_DANGEROUS)),
                            Span::styled(
                                "[del]",
                                Style::default()
                                    .fg(C_DANGEROUS)
                                    .add_modifier(Modifier::BOLD),
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
            draw_confirm_dialog(f, area, &format!("Delete period {}?", fmt_date(p.from)));
        }
    }
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
