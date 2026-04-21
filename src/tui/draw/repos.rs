use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{List, ListItem},
};

use crate::tui::app::{App, InputMode, RemoveRepoRow, RepoManagerRow, Screen};
use crate::tui::palette::C_ACCENT;

use super::{
    back_item, blank_item, del_item, divider_item, draw_confirm_dialog, draw_screen_header,
    hint_confirm_cancel, hint_input_confirm, hint_navigate_action, sub_link_item, sub_screen_setup,
    toggle_item,
};

pub fn draw_repo_manager(f: &mut ratatui::Frame, app: &App) {
    let (_area, engine, layout) = sub_screen_setup(f);

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
            hint_navigate_action(action)
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
                RepoManagerRow::Blank => blank_item(),

                RepoManagerRow::Separator => divider_item(),

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
    let (area, engine, layout) = sub_screen_setup(f);

    let rr_rows = app.remove_repo_items();
    let rr_selected = app.selected_index(Screen::RemoveRepos);
    let hint_spans = match &app.input_mode {
        InputMode::ConfirmDelete(_) => hint_confirm_cancel(),
        _ => {
            let action = match rr_rows.get(rr_selected) {
                Some(RemoveRepoRow::RepoDelete { .. }) => "delete",
                _ => "enter",
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
        "Remove Repos",
        hint_spans,
    );

    let items: Vec<ListItem> = rr_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = rr_selected == i;
            match row {
                RemoveRepoRow::Blank => blank_item(),
                RemoveRepoRow::Separator => divider_item(),
                RemoveRepoRow::Back => back_item(is_sel),
                RemoveRepoRow::RepoDelete { name, url } => {
                    del_item(is_sel, format!("  {name}  <{url}>"))
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
