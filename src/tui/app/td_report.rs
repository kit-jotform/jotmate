use super::constants::TIMEZONES;
use super::screen::Screen;
use super::App;

// ── Time Doctor report state ─────────────────────────────────────────────────

pub enum TdReportState {
    Loading,
    Ready {
        rows: Vec<crate::time::compute::WeekRow>,
        show_cumulative: bool,
    },
    Error(String),
    NeedsReauth,
}

/// Run a Time Doctor report fetch and map `TokenExpired` to the `TOKEN_EXPIRED` sentinel
/// consumed by `poll_td_report`, so the TUI can drop into foreground re-auth.
async fn fetch_report_result(
    email: &str,
    opts: crate::time::FetchOpts,
) -> std::result::Result<Vec<crate::time::compute::WeekRow>, String> {
    crate::time::fetch_report(email, opts).await.map_err(|e| {
        let is_expired = e
            .downcast_ref::<crate::error::AppError>()
            .map(|ae| matches!(ae, crate::error::AppError::TokenExpired))
            .unwrap_or(false);
        if is_expired {
            let _ = crate::time::auth::delete_token_from_keychain();
            "TOKEN_EXPIRED".to_string()
        } else {
            e.to_string()
        }
    })
}

impl App {
    /// Navigate to the Time Doctor report screen and kick off a background fetch.
    pub fn launch_td_report(&mut self) {
        self.screen = Screen::TimeDoctorReport;
        self.td_report = TdReportState::Loading;
        self.td_report_scroll = 0;
        self.td_report_rx = None;

        let Some(start_date) = self.contract_periods.first().map(|p| p.from) else {
            self.td_report = TdReportState::Error("No contract periods configured".to_string());
            return;
        };
        let opts = crate::time::FetchOpts {
            timezone: TIMEZONES[self.td_timezone_idx].to_string(),
            contract_periods: self.contract_periods.clone(),
            start_date,
            skip_current: self.td_skip_current_week,
            no_cache: !self.td_use_time_cache,
        };

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.td_report_rx = Some(rx);
        let email = self.td_email.clone();
        tokio::spawn(async move {
            let _ = tx.send(fetch_report_result(&email, opts).await);
        });
    }

    /// Poll for a completed TD report result. Returns true if state changed.
    pub fn poll_td_report(&mut self) -> bool {
        let result = match &mut self.td_report_rx {
            Some(rx) => match rx.try_recv() {
                Ok(v) => v,
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => return false,
                Err(_) => return false,
            },
            None => return false,
        };
        self.td_report_rx = None;
        match result {
            Ok(mut rows) => {
                let reset_from = crate::config::load()
                    .ok()
                    .and_then(|c| c.time.reset_cumulative_from_date);
                crate::time::compute::compute_cumulative(&mut rows, reset_from);
                rows.reverse();
                self.td_report_scroll = rows.len().saturating_sub(6);
                self.td_report = TdReportState::Ready {
                    rows,
                    show_cumulative: self.td_show_cumulative,
                };
            }
            Err(msg) if msg == "TOKEN_EXPIRED" => {
                self.td_report = TdReportState::NeedsReauth;
            }
            Err(msg) => {
                self.td_report = TdReportState::Error(msg);
            }
        }
        true
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
                self.td_password_is_set = true;
                true
            }
            Err(_) => false,
        }
    }
}
