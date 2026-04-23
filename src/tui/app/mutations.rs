use crate::config::{ContractPeriod, UpstreamRepo};
use crate::tui::rows::{CpListRow, CycleTarget, InputMode, RemoveRepoRow, ToggleKind};

use super::constants::{this_monday, TIMEZONES, WEEKLY_HOURS_OPTIONS};
use super::navigation::clamp_to_last_interactive;
use super::screen::Screen;
use super::App;

impl App {
    // ── Toggle + settings mutation ────────────────────────────────────────────

    pub fn toggle_by_kind(&mut self, kind: ToggleKind) {
        let flag = match kind {
            ToggleKind::UseCache => &mut self.sync.use_cache,
            ToggleKind::SkipForkSync => &mut self.sync.skip_fork_sync,
            ToggleKind::SkipRebase => &mut self.sync.skip_rebase,
            ToggleKind::SkipRdsSync => &mut self.sync.skip_rds_sync,
            ToggleKind::SmartSync => &mut self.sync.smart_sync,
            ToggleKind::SkipCurrentWeek => &mut self.td.skip_current_week,
            ToggleKind::UseTimeCache => &mut self.td.use_time_cache,
        };
        *flag = !*flag;
        if kind.is_sync() {
            self.persist_settings();
        } else {
            self.persist_td_settings();
        }
    }

    pub fn enter_cycle(&mut self, target: CycleTarget) {
        self.input_mode = match target {
            CycleTarget::Timezone => InputMode::SelectingTimezone(self.td.timezone_idx),
            CycleTarget::CpMonday => InputMode::EditingCpMonday(self.add_cp.monday),
            CycleTarget::CpHours => InputMode::EditingCpHours(self.add_cp.hours_idx),
        };
    }

    pub fn cycle(&mut self, target: CycleTarget, delta: i32) {
        match target {
            CycleTarget::Timezone => {
                self.td.timezone_idx = cycle_idx(self.td.timezone_idx, delta, TIMEZONES.len());
                self.persist_td_settings();
            }
            CycleTarget::CpMonday => {
                let days = if delta > 0 { 7 } else { -7 };
                self.add_cp.monday += chrono::Duration::days(days);
            }
            CycleTarget::CpHours => {
                self.add_cp.hours_idx =
                    cycle_idx(self.add_cp.hours_idx, delta, WEEKLY_HOURS_OPTIONS.len());
            }
        }
    }

    pub fn cancel_cycle_edit(&mut self) {
        match self.input_mode.clone() {
            InputMode::SelectingTimezone(snapshot) => {
                self.td.timezone_idx = snapshot;
                self.persist_td_settings();
            }
            InputMode::EditingCpMonday(snapshot) => {
                self.add_cp.monday = snapshot;
            }
            InputMode::EditingCpHours(snapshot) => {
                self.add_cp.hours_idx = snapshot;
            }
            _ => {}
        }
        self.input_mode = InputMode::Normal;
    }

    pub fn toggle_repo(&mut self, name: &str) {
        if let Some(repo) = self.sync.repos.iter_mut().find(|r| r.name == name) {
            repo.enabled = !repo.enabled;
            self.persist_settings();
        }
    }

    pub fn confirm_delete_repo(&mut self, name: String) {
        self.input_mode = InputMode::ConfirmDelete(name);
    }

    pub fn execute_delete_repo(&mut self, name: &str) {
        self.sync.repos.retain(|r| r.name != name);
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
        if index < self.td.contract_periods.len() {
            self.td.contract_periods.remove(index);
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
            from: self.add_cp.monday,
            weekly_hours: WEEKLY_HOURS_OPTIONS[self.add_cp.hours_idx],
        };
        if let Some(existing) = self
            .td
            .contract_periods
            .iter_mut()
            .find(|p| p.from == entry.from)
        {
            existing.weekly_hours = entry.weekly_hours;
        } else {
            self.td.contract_periods.push(entry);
            self.td.contract_periods.sort_by_key(|p| p.from);
            // New period added — bump selection to stay on the same row
            let i = self.selected_index(Screen::ContractPeriods);
            self.select(Screen::ContractPeriods, i + 1);
        }
        self.persist_td_settings();
        // Reset add fields for next entry
        self.add_cp.monday = self
            .td
            .contract_periods
            .last()
            .map(|p| p.from)
            .unwrap_or_else(this_monday);
        self.add_cp.hours_idx = 1;
    }

    // ── Repo management ───────────────────────────────────────────────────────

    fn normalize_url(url: &str) -> String {
        url.trim()
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .to_string()
    }

    fn name_from_url(url: &str) -> String {
        Self::normalize_url(url)
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string()
    }

    pub fn add_repo_from_input(&mut self, url: String) {
        let normalized = Self::normalize_url(&url);
        if normalized.is_empty() {
            return;
        }
        let name = Self::name_from_url(&normalized);
        if name.is_empty() {
            return;
        }
        let url_exists = self
            .sync
            .repos
            .iter()
            .any(|r| Self::normalize_url(&r.url) == normalized);
        let name_exists = self.sync.repos.iter().any(|r| r.name == name);
        if url_exists || name_exists {
            return;
        }
        self.sync.repos.push(UpstreamRepo {
            url: normalized,
            name,
            enabled: true,
        });
        self.persist_settings();
    }
}

fn cycle_idx(idx: usize, delta: i32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if delta > 0 {
        (idx + 1) % len
    } else {
        (idx + len - 1) % len
    }
}
