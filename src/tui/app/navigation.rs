use ratatui::widgets::ListState;

use crate::tui::rows::{
    CpListRow, GeneralToggleRow, RemoveRepoRow, RepoManagerRow, SettingRow, TimeSettingRow,
};

use super::screen::Screen;
use super::App;

pub(super) fn list_state_at(index: usize) -> ListState {
    let mut s = ListState::default();
    s.select(Some(index));
    s
}

fn first_interactive<T>(rows: &[T], is_interactive: impl Fn(&T) -> bool) -> usize {
    rows.iter().position(is_interactive).unwrap_or(0)
}

fn navigate_simple(len: usize, current: usize, delta: i32) -> usize {
    if len == 0 {
        return 0;
    }
    let last = len - 1;
    if delta < 0 {
        if current == 0 {
            last
        } else {
            current - 1
        }
    } else if current == last {
        0
    } else {
        current + 1
    }
}

fn navigate_rows<T>(
    rows: &[T],
    current: usize,
    delta: i32,
    is_interactive: impl Fn(&T) -> bool,
) -> usize {
    let len = rows.len();
    if len == 0 {
        return current;
    }
    let mut next = current;
    for _ in 0..len {
        next = navigate_simple(len, next, delta);
        if is_interactive(&rows[next]) {
            return next;
        }
    }
    current
}

pub(super) fn clamp_to_last_interactive<T>(
    rows: &[T],
    current: usize,
    is_interactive: impl Fn(&T) -> bool,
) -> usize {
    let last = rows.iter().rposition(is_interactive).unwrap_or(0);
    current.min(last)
}

/// Dispatches an expression across every list-screen's row-provider, so
/// `first_interactive` and `navigate_rows` share one match arm per screen.
/// Usage: `with_rows!(self, screen, rows, is_int => body; default_expr)`.
macro_rules! with_rows {
    ($self:ident, $screen:expr, $rows:ident, $is_int:ident => $body:expr ; $default:expr) => {
        match $screen {
            Screen::Settings => {
                let $rows = $self.settings_items();
                let $is_int = SettingRow::is_interactive;
                $body
            }
            Screen::SyncGeneralSettings => {
                let $rows = $self.sync_general_items();
                let $is_int = GeneralToggleRow::is_interactive;
                $body
            }
            Screen::TdGeneralSettings => {
                let $rows = $self.td_general_items();
                let $is_int = GeneralToggleRow::is_interactive;
                $body
            }
            Screen::RepoManager => {
                let $rows = $self.repo_manager_items();
                let $is_int = RepoManagerRow::is_interactive;
                $body
            }
            Screen::RemoveRepos => {
                let $rows = $self.remove_repo_items();
                let $is_int = RemoveRepoRow::is_interactive;
                $body
            }
            Screen::TimeDoctorSettings => {
                let $rows = $self.td_settings_items();
                let $is_int = TimeSettingRow::is_interactive;
                $body
            }
            Screen::ContractPeriods => {
                let $rows = $self.cp_list_items();
                let $is_int = CpListRow::is_interactive;
                $body
            }
            _ => $default,
        }
    };
}

impl App {
    pub fn list_state(&self, screen: Screen) -> &ListState {
        self.list_states
            .get(&screen)
            .unwrap_or_else(|| panic!("no ListState for screen {:?}", screen as u8))
    }

    pub fn list_state_mut(&mut self, screen: Screen) -> &mut ListState {
        self.list_states
            .get_mut(&screen)
            .unwrap_or_else(|| panic!("no ListState for screen {:?}", screen as u8))
    }

    pub fn selected_index(&self, screen: Screen) -> usize {
        self.list_state(screen).selected().unwrap_or(0)
    }

    pub fn select(&mut self, screen: Screen, i: usize) {
        self.list_state_mut(screen).select(Some(i));
    }

    pub fn select_first_interactive(&mut self, screen: Screen) {
        let i = with_rows!(self, screen, rows, is_int => first_interactive(&rows, is_int); 0);
        self.select(screen, i);
    }

    pub fn navigate_current(&mut self, delta: i32) {
        let screen = self.screen;
        let cur = self.selected_index(screen);
        let next = with_rows!(
            self, screen, rows, is_int => navigate_rows(&rows, cur, delta, is_int);
            match screen {
                Screen::MainMenu => navigate_simple(self.main_menu_items().len(), cur, delta),
                _ => cur,
            }
        );
        self.select(screen, next);
    }
}
