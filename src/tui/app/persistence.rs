use super::constants::TIMEZONES;
use super::App;

impl App {
    pub fn persist_settings(&self) {
        self.mutate_and_save(|cfg| {
            cfg.sync.use_cache = self.sync.use_cache;
            cfg.sync.skip_fork_sync = self.sync.skip_fork_sync;
            cfg.sync.skip_rebase = self.sync.skip_rebase;
            cfg.sync.skip_rds_sync = self.sync.skip_rds_sync;
            cfg.sync.smart_sync = self.sync.smart_sync;
            cfg.sync.upstream_repos = self.sync.repos.clone();
        });
    }

    pub fn persist_td_settings(&self) {
        self.mutate_and_save(|cfg| {
            cfg.time.email = Some(self.td.email.clone()).filter(|e| !e.is_empty());
            cfg.time.timezone = Some(TIMEZONES[self.td.timezone_idx].to_string());
            // start_date auto-derived from first contract period
            cfg.time.start_date = self.td.contract_periods.first().map(|p| p.from);
            cfg.time.skip_current_week = self.td.skip_current_week;
            cfg.time.use_time_cache = self.td.use_time_cache;
            cfg.time.show_cumulative = self.td.show_cumulative;
            cfg.time.contract_periods =
                Some(self.td.contract_periods.clone()).filter(|c| !c.is_empty());
        });
    }

    fn mutate_and_save(&self, mutate: impl FnOnce(&mut crate::config::Config)) {
        if let Ok(mut config) = crate::config::load() {
            mutate(&mut config);
            let _ = crate::config::save(&config);
        }
    }
}
