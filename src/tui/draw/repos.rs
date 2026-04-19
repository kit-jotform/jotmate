use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
};

use crate::tui::app::{App, InputMode, RemoveRepoRow, RepoManagerRow, Screen};
use crate::tui::layout::LayoutEngine;
use crate::tui::palette::{C_ACCENT, C_DANGEROUS, C_MUTED, C_TEXT};

use super::{
    back_item, draw_confirm_dialog, draw_screen_header, hint_confirm_cancel, hint_input_confirm,
    hint_muted, sub_link_item, sub_screen_layout, toggle_item, SEPARATOR_WIDTH,
};

pub fn draw_repo_manager(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let engine = LayoutEngine::new(area);
    let layout = sub_screen_layout(engine.clamp_area(area));

    let rm_rows = app.repo_manager_items();
    let selected = app.selected_index(Screen::RepoManager);

    let hint_spans = match &app.input_mode {
        InputMode::AddingRepo(_) => hint_input_confirm(),
        _ => {
            let action = match rm_rows.get(selected) {
                Some(RepoManagerRow::RepoToggle { .. }) => "toggle",
                Some(RepoManagerRow::RemoveReposLink) => "enter",
                Some(RepoManagerRow::AddUrl) => "select",
                _ => "enter",
            };
            hint_muted(&["↑↓", " navigate  •  ", "↵", &format!(" {action:<6}  •  "), "⌫/Esc", " back"])
        }
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

    let items: Vec<ListItem> = rm_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                RepoManagerRow::Blank => ListItem::new(Line::raw("")),

                RepoManagerRow::Separator => ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "─".repeat(SEPARATOR_WIDTH),
                        Style::default().fg(C_MUTED),
                    ),
                ])),

                RepoManagerRow::Back => back_item(is_sel),

                RepoManagerRow::RepoToggle { name, url, enabled } => {
                    toggle_item(is_sel, *enabled, format!("{name}  <{url}>"), false, false)
                }

                RepoManagerRow::RemoveReposLink => sub_link_item(is_sel, "→ Remove Repos"),

                RepoManagerRow::AddUrl => match &app.input_mode {
                    InputMode::AddingRepo(buf) => {
                        let display = format!("  URL: {buf}_");
                        ListItem::new(Line::from(Span::styled(
                            display,
                            Style::default().fg(C_ACCENT),
                        )))
                    }
                    _ => sub_link_item(is_sel, "+ Add upstream URL"),
                },
            }
        })
        .collect();

    f.render_stateful_widget(
        List::new(items),
        layout.get("list"),
        &mut app.list_state(Screen::RepoManager).clone(),
    );
}

pub fn draw_remove_repos(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let engine = LayoutEngine::new(area);
    let layout = sub_screen_layout(engine.clamp_area(area));

    let rr_rows = app.remove_repo_items();
    let rr_selected = app.selected_index(Screen::RemoveRepos);
    let hint_spans = match &app.input_mode {
        InputMode::ConfirmDelete(_) => hint_confirm_cancel(),
        _ => {
            let action = match rr_rows.get(rr_selected) {
                Some(RemoveRepoRow::RepoDelete { .. }) => "delete",
                _ => "enter",
            };
            hint_muted(&["↑↓", " navigate  •  ", "↵", &format!(" {action:<6}  •  "), "⌫/Esc", " back"])
        }
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

    let items: Vec<ListItem> = rr_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = rr_selected == i;
            match row {
                RemoveRepoRow::Blank => ListItem::new(Line::raw("")),
                RemoveRepoRow::Separator => ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "─".repeat(SEPARATOR_WIDTH),
                        Style::default().fg(C_MUTED),
                    ),
                ])),
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
        &mut app.list_state(Screen::RemoveRepos).clone(),
    );

    if let InputMode::ConfirmDelete(name) = &app.input_mode {
        draw_confirm_dialog(f, area, &format!("Delete \"{}\"?", name));
    }
}
