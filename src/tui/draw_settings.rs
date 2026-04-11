use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
};

use super::app::{App, GeneralToggleRow, InputMode, SettingRow};
use super::draw::{
    back_item, draw_screen_header, hint_navigate_toggle, link_item, sub_screen_layout, toggle_item,
    HINT_CYCLE_VALUE,
};
use super::layout::LayoutEngine;
use super::palette::{C_ACCENT, C_MUTED, C_PRIMARY, C_TEXT};

pub fn draw_settings(f: &mut ratatui::Frame, app: &App) {
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

pub fn draw_general_toggles(f: &mut ratatui::Frame, app: &App, title: &str, is_sync: bool) {
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
                            Span::styled(HINT_CYCLE_VALUE, Style::default().fg(C_MUTED)),
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
