use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
};

use super::app::{App, InputMode, RemoveRepoRow, RepoManagerRow};
use super::draw::{
    back_item, draw_confirm_dialog, draw_screen_header, hint_confirm_cancel, hint_input_confirm,
    hint_select_back, sub_screen_layout, toggle_item,
};
use super::layout::LayoutEngine;
use super::palette::{C_ACCENT, C_DANGEROUS, C_MUTED, C_PRIMARY, C_TEXT};

pub fn draw_repo_manager(f: &mut ratatui::Frame, app: &App) {
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

pub fn draw_remove_repos(f: &mut ratatui::Frame, app: &App) {
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

    if let InputMode::ConfirmDelete(name) = &app.input_mode {
        draw_confirm_dialog(f, area, &format!("Delete \"{}\"?", name));
    }
}
