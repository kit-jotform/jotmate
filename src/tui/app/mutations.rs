use crate::config::{ContractPeriod, UpstreamRepo};
use crate::tui::rows::{CpListRow, InputMode, RemoveRepoRow, ToggleKind};

use super::constants::{this_monday, TIMEZONES, WEEKLY_HOURS_OPTIONS};
use super::navigation::clamp_to_last_interactive;
use super::screen::Screen;
use super::App;

impl App {
    // ── Toggle + settings mutation ────────────────────────────────────────────

    pub fn toggle_by_kind(&mut self, kind: ToggleKind) {
        let flag = match kind {
            ToggleKind::UseCache => &mut self.use_cache,
            ToggleKind::SkipForkSync => &mut self.skip_fork_sync,
            ToggleKind::SkipRebase => &mut self.skip_rebase,
            ToggleKind::SkipRdsSync => &mut self.skip_rds_sync,
            ToggleKind::SkipGitFetch => &mut self.skip_git_fetch,
            ToggleKind::SkipDirtySync => &mut self.skip_dirty_sync,
            ToggleKind::SkipCurrentWeek => &mut self.td_skip_current_week,
            ToggleKind::UseTimeCache => &mut self.td_use_time_cache,
            ToggleKind::ShowCumulative => &mut self.td_show_cumulative,
        };
        *flag = !*flag;
        if kind.is_sync() {
            self.persist_settings();
        } else {
            self.persist_td_settings();
        }
    }

    /// Cancel an in-progress cycle edit by restoring the snapshot stored in the InputMode variant.
    pub fn cancel_cycle_edit(&mut self) {
        match self.input_mode.clone() {
            InputMode::SelectingTimezone(snapshot) => {
                self.td_timezone_idx = snapshot;
                self.persist_td_settings();
            }
            InputMode::EditingCpMonday(snapshot) => {
                self.add_cp_monday = snapshot;
            }
            InputMode::EditingCpHours(snapshot) => {
                self.add_cp_hours_idx = snapshot;
            }
            _ => {}
        }
        self.input_mode = InputMode::Normal;
    }

    pub fn cycle_timezone(&mut self, delta: i32) {
        let len = TIMEZONES.len();
        if delta > 0 {
            self.td_timezone_idx = (self.td_timezone_idx + 1) % len;
        } else {
            self.td_timezone_idx = (self.td_timezone_idx + len - 1) % len;
        }
        self.persist_td_settings();
    }

    pub fn toggle_repo(&mut self, name: &str) {
        if let Some(repo) = self.repos.iter_mut().find(|r| r.name == name) {
            repo.enabled = !repo.enabled;
            self.persist_settings();
        }
    }

    pub fn confirm_delete_repo(&mut self, name: String) {
        self.input_mode = InputMode::ConfirmDelete(name);
    }

    pub fn execute_delete_repo(&mut self, name: &str) {
        self.repos.retain(|r| r.name != name);
        self.persist_settings();
        self.input_mode = InputMode::Normal;
        let rows = self.remove_repo_items();
        let cur = self.selected_index(Screen::RemoveRepos);
        let clamped = clamp_to_last_interactive(&rows, cur, RemoveRepoRow::is_interactive);
        self.select(Screen::RemoveRepos, clamped);
    }

    pub fn confirm_delete_period(&mut self, index: usize) {
        self.input_mode = InputMode::ConfirmDeletePeriod(index);
    }

    pub fn execute_delete_period(&mut self, index: usize) {
        if index < self.contract_periods.len() {
            self.contract_periods.remove(index);
            self.persist_td_settings();
        }
        self.input_mode = InputMode::Normal;
        let rows = self.cp_list_items();
        let cur = self.selected_index(Screen::ContractPeriods);
        let clamped = clamp_to_last_interactive(&rows, cur, CpListRow::is_interactive);
        self.select(Screen::ContractPeriods, clamped);
    }

    pub fn save_new_contract_period(&mut self) {
        let entry = ContractPeriod {
            from: self.add_cp_monday,
            weekly_hours: WEEKLY_HOURS_OPTIONS[self.add_cp_hours_idx],
        };
        if let Some(existing) = self
            .contract_periods
            .iter_mut()
            .find(|p| p.from == entry.from)
        {
            existing.weekly_hours = entry.weekly_hours;
        } else {
            self.contract_periods.push(entry);
            self.contract_periods.sort_by_key(|p| p.from);
            // New period added — bump selection to stay on the same row
            let i = self.selected_index(Screen::ContractPeriods);
            self.select(Screen::ContractPeriods, i + 1);
        }
        self.persist_td_settings();
        // Reset add fields for next entry
        self.add_cp_monday = self
            .contract_periods
            .last()
            .map(|p| p.from)
            .unwrap_or_else(this_monday);
        self.add_cp_hours_idx = 1;
    }

    pub fn cycle_add_cp_monday(&mut self, delta: i32) {
        let days = if delta > 0 { 7 } else { -7 };
        self.add_cp_monday += chrono::Duration::days(days);
    }

    pub fn cycle_add_cp_hours(&mut self, delta: i32) {
        let len = WEEKLY_HOURS_OPTIONS.len();
        if delta > 0 {
            self.add_cp_hours_idx = (self.add_cp_hours_idx + 1) % len;
        } else {
            self.add_cp_hours_idx = (self.add_cp_hours_idx + len - 1) % len;
        }
    }

    // ── Repo management ───────────────────────────────────────────────────────

    /// Derive a short name from a URL (last path component, stripped of .git).
    fn name_from_url(url: &str) -> String {
        url.trim_end_matches('/')
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or(url)
            .to_string()
    }

    pub fn add_repo_from_input(&mut self, url: String) {
        let url = url.trim().to_string();
        if url.is_empty() {
            return;
        }
        let name = Self::name_from_url(&url);
        if !self.repos.iter().any(|r| r.url == url) {
            self.repos.push(UpstreamRepo {
                url,
                name,
                enabled: true,
            });
            self.persist_settings();
        }
    }
}
