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
    NoCredentials(String),
    NoPeriods,
}

async fn fetch_report_result(
    email: &str,
    opts: crate::time::FetchOpts,
) -> std::result::Result<Vec<crate::time::compute::WeekRow>, String> {
    crate::time::fetch_report(email, opts).await.map_err(|e| {
        match e.downcast_ref::<crate::error::AppError>() {
            Some(crate::error::AppError::TokenExpired) => {
                let _ = crate::time::auth::delete_token_from_keychain();
                "AUTH_FAILED:Session expired — please re-enter your password.".to_string()
            }
            Some(crate::error::AppError::AuthFailed(msg)) => {
                let _ = crate::time::auth::delete_token_from_keychain();
                format!("AUTH_FAILED:{msg}")
            }
            _ => "Could not connect to TimeDoctor. Check your internet connection and try again.".to_string(),
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

        if self.td_email.is_empty() || !self.td_password_is_set {
            self.td_report = TdReportState::NoCredentials(
                "Email or password not configured.".to_string(),
            );
            return;
        }

        let Some(start_date) = self.contract_periods.first().map(|p| p.from) else {
            self.td_report = TdReportState::NoPeriods;
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
            Err(msg) if msg.starts_with("AUTH_FAILED:") => {
                let reason = msg.trim_start_matches("AUTH_FAILED:").to_string();
                self.td_report = TdReportState::NoCredentials(reason);
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
