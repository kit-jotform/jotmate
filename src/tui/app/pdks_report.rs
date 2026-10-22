use chrono::{Duration, NaiveDate};

use crate::config::{PDKS_DAILY_TARGET_HOURS, PDKS_WEEKLY_TARGET_HOURS};

use super::{constants::this_monday, App, Screen};

pub enum PdksReportState {
    Loading { tick: u8 },
    Ready(crate::pdks::Report),
    Error(String),
}

impl App {
    pub fn launch_pdks_report(&mut self) {
        self.screen = Screen::PdksReport;
        self.pdks_week = this_monday();
        self.pdks_target_hours = self.pdks_target_for_week(self.pdks_week);
        self.pdks_target_editing = false;
        self.reload_pdks_report();
    }

    pub fn shift_pdks_week(&mut self, weeks: i64) {
        self.pdks_week += Duration::days(weeks * 7);
        self.pdks_target_hours = self.pdks_target_for_week(self.pdks_week);
        self.pdks_target_editing = false;
        self.reload_pdks_report();
    }

    pub fn adjust_pdks_target(&mut self, steps: i8) {
        self.pdks_target_hours = (self.pdks_target_hours
            + f64::from(steps) * PDKS_DAILY_TARGET_HOURS)
            .clamp(0.0, PDKS_WEEKLY_TARGET_HOURS);
        self.pdks_weekly_targets
            .retain(|target| target.week != self.pdks_week);
        if (self.pdks_target_hours - PDKS_WEEKLY_TARGET_HOURS).abs() > f64::EPSILON {
            self.pdks_weekly_targets
                .push(crate::config::PdksWeeklyTarget {
                    week: self.pdks_week,
                    target_hours: self.pdks_target_hours,
                });
            self.pdks_weekly_targets.sort_by_key(|target| target.week);
        }
        self.persist_pdks_targets();

        if let PdksReportState::Ready(report) = &mut self.pdks_report {
            report.target_hours = self.pdks_target_hours;
        }
    }

    fn pdks_target_for_week(&self, week: NaiveDate) -> f64 {
        self.pdks_weekly_targets
            .iter()
            .find(|target| target.week == week)
            .map(|target| target.target_hours)
            .unwrap_or(PDKS_WEEKLY_TARGET_HOURS)
    }

    fn reload_pdks_report(&mut self) {
        self.pdks_report = PdksReportState::Loading { tick: 0 };
        self.pdks_report_rx = None;

        if let Err(error) = crate::pdks::ensure_session() {
            self.pdks_report = PdksReportState::Error(error.to_string());
            return;
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pdks_report_rx = Some(rx);
        let args = crate::cli::PdksArgs {
            week: Some(self.pdks_week),
            target_hours: self.pdks_target_hours,
            ..Default::default()
        };
        tokio::spawn(async move {
            let _ = tx.send(
                crate::pdks::fetch_report(args)
                    .await
                    .map_err(|e| e.to_string()),
            );
        });
    }

    pub fn poll_pdks_report(&mut self) -> bool {
        if let Some(rx) = &mut self.pdks_report_rx {
            match rx.try_recv() {
                Ok(Ok(report)) => {
                    self.pdks_report_rx = None;
                    self.pdks_report = PdksReportState::Ready(report);
                    return true;
                }
                Ok(Err(error)) => {
                    self.pdks_report_rx = None;
                    self.pdks_report = PdksReportState::Error(error);
                    return true;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
                Err(_) => {
                    self.pdks_report_rx = None;
                    self.pdks_report =
                        PdksReportState::Error("PDKS request did not complete.".into());
                    return true;
                }
            }
        }
        if let PdksReportState::Loading { tick } = &mut self.pdks_report {
            *tick = tick.wrapping_add(1);
            return true;
        }
        false
    }
}
