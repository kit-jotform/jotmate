use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
};

use crate::tui::app::{App, InputMode, OwListRow, Screen};
use crate::tui::palette::{C_ACCENT, C_MUTED, C_PRIMARY, C_WARN};

use super::{
    back_item, blank_item, del_item, divider_item, draw_confirm_dialog, draw_screen_header,
    field_state, fmt_date, hint_confirm_cancel, hint_navigate_action, inline_field_item,
    sub_screen_setup, FIELD_LABEL_W,
};

pub fn draw_off_weeks(f: &mut ratatui::Frame, app: &App) {
    let (area, engine, layout) = sub_screen_setup(f);

    let ow_rows = app.ow_list_items();
    let selected = app.selected_index(Screen::OffWeeks);
    let hint_spans = match &app.input_mode {
        InputMode::ConfirmDeleteOffWeek(_) => hint_confirm_cancel(),
        _ => {
            let action = match ow_rows.get(selected) {
                Some(OwListRow::OffWeek { .. }) => "delete",
                Some(OwListRow::SaveOffWeek) => "save",
                Some(OwListRow::MondayField) => "change",
                Some(OwListRow::Back) => "enter",
                _ => "select",
            };
            hint_navigate_action(action)
        }
    };

    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        "Off Weeks",
        hint_spans,
    );

    let items: Vec<ListItem> = ow_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                OwListRow::SectionTitle(title) => ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(*title, Style::default().fg(C_MUTED)),
                ])),
                OwListRow::Blank => blank_item(),
                OwListRow::Separator => divider_item(),
                OwListRow::Back => back_item(is_sel),
                OwListRow::SaveOffWeek => {
                    if is_sel {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
                            Span::styled(
                                "Save off week",
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled("Save off week", Style::default().fg(C_WARN)),
                        ]))
                    }
                }
                OwListRow::MondayField => {
                    let value = fmt_date(app.add_ow.monday);
                    let editing = matches!(app.input_mode, InputMode::EditingOwMonday(_));
                    inline_field_item(
                        "From Monday",
                        &value,
                        field_state(is_sel, editing),
                        FIELD_LABEL_W,
                    )
                }
                OwListRow::OffWeek { monday, .. } => {
                    let detail = format!("  {}", fmt_date(*monday));
                    del_item(is_sel, detail)
                }
            }
        })
        .collect();

    f.render_stateful_widget(
        List::new(items),
        layout.get("list"),
        &mut app.list_state(Screen::OffWeeks).clone(),
    );

    if let InputMode::ConfirmDeleteOffWeek(idx) = &app.input_mode {
        if let Some(monday) = app.td.off_weeks.get(*idx) {
            draw_confirm_dialog(f, area, &format!("Delete off week {}?", fmt_date(*monday)));
        }
    }
}
