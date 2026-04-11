use crossterm::event::KeyCode;

use crate::tui::app::{App, InputMode, RemoveRepoRow, RepoManagerRow, Screen};

use super::helpers::{go_to, handle_list_nav};
use super::keys::is_activate;
use super::Action;

pub(super) fn handle_repo_manager(app: &mut App, code: KeyCode) -> Action {
    if let Some(a) = handle_list_nav(app, code, Screen::Settings) {
        return a;
    }
    if !is_activate(code) {
        return Action::Continue;
    }
    let rows = app.repo_manager_items();
    match rows.get(app.selected_index(Screen::RepoManager)) {
        Some(RepoManagerRow::Back) => app.screen = Screen::Settings,
        Some(RepoManagerRow::RepoToggle { name, .. }) => {
            let name = name.clone();
            app.toggle_repo(&name);
        }
        Some(RepoManagerRow::AddUrl) => app.input_mode = InputMode::AddingRepo(String::new()),
        Some(RepoManagerRow::RemoveReposLink) => go_to(app, Screen::RemoveRepos),
        _ => {}
    }
    Action::Continue
}

pub(super) fn handle_remove_repos(app: &mut App, code: KeyCode) -> Action {
    if let Some(a) = handle_list_nav(app, code, Screen::RepoManager) {
        return a;
    }
    if !is_activate(code) {
        return Action::Continue;
    }
    let rows = app.remove_repo_items();
    match rows.get(app.selected_index(Screen::RemoveRepos)) {
        Some(RemoveRepoRow::Back) => app.screen = Screen::RepoManager,
        Some(RemoveRepoRow::RepoDelete { name, .. }) => {
            let name = name.clone();
            app.confirm_delete_repo(name);
        }
        _ => {}
    }
    Action::Continue
}

pub(super) fn execute_pending_repo_delete(app: &mut App) {
    if let InputMode::ConfirmDelete(name) = app.input_mode.clone() {
        app.execute_delete_repo(&name);
    }
}

pub(super) fn apply_new_repo_url(app: &mut App, url: String) {
    app.add_repo_from_input(url);
}
