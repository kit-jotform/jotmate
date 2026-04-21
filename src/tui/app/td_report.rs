use chrono::NaiveDate;

use crate::config::TIMEDOCTOR_COMPANY_ID;
use crate::time::cache;
use crate::time::compute::{
    build_week_row_from_cache, compute_cumulative, get_week_start_monday, weeks_to_fetch,
};

use super::constants::TIMEZONES;
use super::screen::Screen;
use super::App;

// ── Time Doctor report state ─────────────────────────────────────────────────

pub enum TdReportState {
    Loading,
    /// Some weeks loaded from cache and shown immediately; uncached weeks are
    /// being fetched in the background. `pending` is the count of in-flight weeks.
    PartialReady {
        rows: Vec<crate::time::compute::WeekRow>,
        pending: usize,
        show_cumulative: bool,
        tick: u8,
    },
    Ready {
        rows: Vec<crate::time::compute::WeekRow>,
        show_cumulative: bool,
    },
    Error(String),
    NoCredentials(String),
    NoPeriods,
}

type FetchResult = std::result::Result<Vec<crate::time::compute::WeekRow>, String>;

fn map_err(e: anyhow::Error) -> String {
    match e.downcast_ref::<crate::error::AppError>() {
        Some(crate::error::AppError::TokenExpired) => {
            let _ = crate::time::auth::delete_token_from_keychain();
            "AUTH_FAILED:Session expired — please re-enter your password.".to_string()
        }
        Some(crate::error::AppError::AuthFailed(msg)) => {
            let _ = crate::time::auth::delete_token_from_keychain();
            format!("AUTH_FAILED:{msg}")
        }
        _ => "Could not connect to TimeDoctor. Check your internet connection and try again."
            .to_string(),
    }
}

async fn fetch_parallel(
    email: String,
    mondays: Vec<NaiveDate>,
    opts: crate::time::FetchOpts,
) -> FetchResult {
    crate::time::fetch_weeks_parallel(&email, mondays, opts)
        .await
        .map_err(map_err)
}

impl App {
    /// Navigate to the Time Doctor report screen.
    ///
    /// When cache is enabled:
    ///   - All weeks that are already cached are shown immediately as `PartialReady`.
    ///   - Only the uncached weeks (missing past weeks + current week) are fetched
    ///     in parallel in the background.
    ///
    /// When cache is disabled: all weeks are fetched in parallel from the start
    /// (screen stays `Loading` until done).
    pub fn launch_td_report(&mut self) {
        self.screen = Screen::TimeDoctorReport;
        self.td_report = TdReportState::Loading;
        self.td_report_scroll = 0;
        self.td_report_rx = None;

        if self.td.email.is_empty() || !self.td.password_is_set {
            self.td_report =
                TdReportState::NoCredentials("Email or password not configured.".to_string());
            return;
        }

        let Some(start_date) = self.td.contract_periods.first().map(|p| p.from) else {
            self.td_report = TdReportState::NoPeriods;
            return;
        };

        let opts = crate::time::FetchOpts {
            timezone: TIMEZONES[self.td.timezone_idx].to_string(),
            contract_periods: self.td.contract_periods.clone(),
            start_date,
            skip_current: self.td.skip_current_week,
            no_cache: !self.td.use_time_cache,
        };

        let all_mondays = weeks_to_fetch(start_date, self.td.skip_current_week);
        let today = chrono::Local::now().date_naive();
        let current_monday = get_week_start_monday(today);

        // Split mondays into cached (show now) and uncached (fetch in background).
        let mut cached_rows: Vec<crate::time::compute::WeekRow> = Vec::new();
        let mut uncached_mondays: Vec<NaiveDate> = Vec::new();

        if self.td.use_time_cache {
            for &monday in &all_mondays {
                let is_past = monday < current_monday;
                if is_past {
                    if let Some(stats) = cache::read_week_cache(TIMEDOCTOR_COMPANY_ID, monday) {
                        cached_rows.push(build_week_row_from_cache(
                            monday,
                            &stats,
                            &opts.contract_periods,
                        ));
                        continue;
                    }
                }
                uncached_mondays.push(monday);
            }
        } else {
            uncached_mondays = all_mondays;
        }

        // If we have cached rows to show immediately, go to PartialReady.
        if !cached_rows.is_empty() {
            let reset_from = crate::config::load()
                .ok()
                .and_then(|c| c.time.reset_cumulative_from_date);
            compute_cumulative(&mut cached_rows, reset_from);
            cached_rows.sort_by_key(|r| r.monday);
            self.td_report_scroll = cached_rows.len().saturating_sub(6);
            let pending = uncached_mondays.len();
            self.td_report = TdReportState::PartialReady {
                rows: cached_rows,
                pending,
                show_cumulative: self.td.show_cumulative,
                tick: 0,
            };
        }

        if uncached_mondays.is_empty() {
            // Nothing to fetch — finalize immediately from whatever we have.
            let (rows, show_cumulative) = match &self.td_report {
                TdReportState::PartialReady {
                    rows,
                    show_cumulative,
                    ..
                } => (rows.clone(), *show_cumulative),
                _ => (vec![], self.td.show_cumulative),
            };
            self.td_report_scroll = rows.len().saturating_sub(6);
            self.td_report = TdReportState::Ready {
                rows,
                show_cumulative,
            };
            return;
        }

        // Spawn background fetch for uncached weeks.
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.td_report_rx = Some(rx);
        let email = self.td.email.clone();
        tokio::spawn(async move {
            let _ = tx.send(fetch_parallel(email, uncached_mondays, opts).await);
        });
    }

    /// Poll for completed TD report results. Returns true if state changed.
    pub fn poll_td_report(&mut self) -> bool {
        if let Some(rx) = &mut self.td_report_rx {
            match rx.try_recv() {
                Ok(result) => {
                    self.td_report_rx = None;
                    self.apply_fetch_result(result);
                    return true;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
                Err(_) => {
                    self.td_report_rx = None;
                }
            }
        }

        // Tick spinner while partial results are showing.
        if let TdReportState::PartialReady { tick, .. } = &mut self.td_report {
            *tick = tick.wrapping_add(1);
            return true;
        }

        false
    }

    fn apply_fetch_result(&mut self, result: FetchResult) {
        let reset_from = crate::config::load()
            .ok()
            .and_then(|c| c.time.reset_cumulative_from_date);

        match result {
            Ok(new_rows) => {
                // Merge with any cached rows already in PartialReady.
                let mut rows = match &self.td_report {
                    TdReportState::PartialReady { rows, .. } => rows.clone(),
                    _ => vec![],
                };
                rows.extend(new_rows);
                compute_cumulative(&mut rows, reset_from);
                rows.sort_by_key(|r| r.monday);
                self.td_report_scroll = rows.len().saturating_sub(6);
                self.td_report = TdReportState::Ready {
                    rows,
                    show_cumulative: self.td.show_cumulative,
                };
            }
            Err(msg) if msg.starts_with("AUTH_FAILED:") => {
                self.td_report = TdReportState::NoCredentials(
                    msg.trim_start_matches("AUTH_FAILED:").to_string(),
                );
            }
            Err(msg) => {
                // On error: if we have cached rows, show them as Ready rather than Error.
                if let TdReportState::PartialReady {
                    rows,
                    show_cumulative,
                    ..
                } = &self.td_report
                {
                    let mut rows = rows.clone();
                    let show_cumulative = *show_cumulative;
                    compute_cumulative(&mut rows, reset_from);
                    rows.sort_by_key(|r| r.monday);
                    self.td_report_scroll = rows.len().saturating_sub(6);
                    self.td_report = TdReportState::Ready {
                        rows,
                        show_cumulative,
                    };
                } else {
                    self.td_report = TdReportState::Error(msg);
                }
            }
        }
    }

    /// Save password to keychain and update in-memory flag.
    /// Returns true if the password was saved successfully.
    pub fn set_td_password(&mut self, password: &str) -> bool {
        if password.is_empty() {
            return false;
        }
        // Delete old session token so a fresh login is triggered with the new password
        let _ = crate::time::auth::delete_token_from_keychain();
        match crate::time::auth::save_password_to_keychain(password) {
            Ok(()) => {
                self.td.password_is_set = true;
                true
            }
            Err(_) => false,
        }
    }
}
