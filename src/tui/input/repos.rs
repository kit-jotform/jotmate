use std::ops::ControlFlow;

use crossterm::event::KeyCode;

use crate::tui::app::{App, InputMode, RemoveRepoRow, RepoManagerRow, Screen};

use super::helpers::{execute_if_confirmed, go_to, list_activate_row};
use super::Action;

pub(super) fn handle_repo_manager(app: &mut App, code: KeyCode) -> Action {
    let row = match list_activate_row(app, code, Screen::Settings, Screen::RepoManager, |a| {
        a.repo_manager_items()
    }) {
        ControlFlow::Break(a) => return a,
        ControlFlow::Continue(r) => r,
    };
    match row {
        Some(RepoManagerRow::Back) => app.screen = Screen::Settings,
        Some(RepoManagerRow::RepoToggle { name, .. }) => app.toggle_repo(&name),
        Some(RepoManagerRow::AddUrl) => app.input_mode = InputMode::AddingRepo(String::new()),
        Some(RepoManagerRow::RemoveReposLink) => go_to(app, Screen::RemoveRepos),
        _ => {}
    }
    Action::Continue
}

pub(super) fn handle_remove_repos(app: &mut App, code: KeyCode) -> Action {
    let row = match list_activate_row(app, code, Screen::RepoManager, Screen::RemoveRepos, |a| {
        a.remove_repo_items()
    }) {
        ControlFlow::Break(a) => return a,
        ControlFlow::Continue(r) => r,
    };
    match row {
        Some(RemoveRepoRow::Back) => app.screen = Screen::RepoManager,
        Some(RemoveRepoRow::RepoDelete { name, .. }) => app.confirm_delete_repo(name),
        _ => {}
    }
    Action::Continue
}

pub(super) fn execute_pending_repo_delete(app: &mut App) {
    execute_if_confirmed(
        app,
        |m| {
            if let InputMode::ConfirmDelete(n) = m {
                Some(n.clone())
            } else {
                None
            }
        },
        |a, name| a.execute_delete_repo(&name),
    );
}

pub(super) fn apply_new_repo_url(app: &mut App, url: String) {
    app.add_repo_from_input(url);
}
