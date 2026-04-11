use super::constants::TIMEZONES;
use super::App;

impl App {
    pub fn persist_settings(&self) {
        self.mutate_and_save(|cfg| {
            cfg.sync.sync_all_by_default = self.sync_all;
            cfg.sync.use_cache = self.use_cache;
            cfg.sync.skip_fork_sync = self.skip_fork_sync;
            cfg.sync.skip_rebase = self.skip_rebase;
            cfg.sync.skip_rds_sync = self.skip_rds_sync;
            cfg.sync.skip_git_fetch = self.skip_git_fetch;
            cfg.sync.skip_dirty_sync = self.skip_dirty_sync;
            cfg.sync.upstream_repos = self.repos.clone();
        });
    }

    pub fn persist_td_settings(&self) {
        self.mutate_and_save(|cfg| {
            cfg.time.email = Some(self.td_email.clone()).filter(|e| !e.is_empty());
            cfg.time.timezone = Some(TIMEZONES[self.td_timezone_idx].to_string());
            // start_date auto-derived from first contract period
            cfg.time.start_date = self.contract_periods.first().map(|p| p.from);
            cfg.time.skip_current_week = self.td_skip_current_week;
            cfg.time.use_time_cache = self.td_use_time_cache;
            cfg.time.show_cumulative = self.td_show_cumulative;
            cfg.time.contract_periods =
                Some(self.contract_periods.clone()).filter(|c| !c.is_empty());
        });
    }

    /// Load the config, apply `mutate`, and save — ignoring errors at either end.
    /// This is the single place where in-memory state is flushed back to disk.
    fn mutate_and_save(&self, mutate: impl FnOnce(&mut crate::config::Config)) {
        if let Ok(mut config) = crate::config::load() {
            mutate(&mut config);
            let _ = crate::config::save(&config);
        }
    }
}
