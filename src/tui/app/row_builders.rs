use crate::tui::rows::{
    CpListRow, GeneralToggleRow, RemoveRepoRow, RepoManagerRow, SettingRow, TimeDoctorField,
    TimeSettingRow, ToggleKind,
};

use super::constants::TIMEZONES;
use super::App;

impl App {
    pub fn settings_items(&self) -> Vec<SettingRow> {
        vec![
            SettingRow::Separator, // "RDS Sync"
            SettingRow::Blank,
            SettingRow::SyncGeneralLink,
            SettingRow::ManageRepos,
            SettingRow::Blank,
            SettingRow::Separator, // "Time Doctor"
            SettingRow::Blank,
            SettingRow::TdGeneralLink,
            SettingRow::TimeDoctorSettings,
            SettingRow::ContractPeriodsLink,
            SettingRow::Blank,
            SettingRow::Divider,
            SettingRow::Back,
        ]
    }

    pub fn sync_general_items(&self) -> Vec<GeneralToggleRow> {
        vec![
            GeneralToggleRow::Toggle {
                kind: ToggleKind::UseCache,
                label: "Use repo path cache",
                hint: "",
                on: self.use_cache,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipForkSync,
                label: "Fork sync",
                hint: "fetch+merge+push upstream",
                on: !self.skip_fork_sync,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipRebase,
                label: "Rebase",
                hint: "rebase branch after merge",
                on: !self.skip_rebase,
                indent: true,
                disabled: self.skip_fork_sync,
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipRdsSync,
                label: "RDS sync",
                hint: "./sync in each repo",
                on: !self.skip_rds_sync,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SmartSync,
                label: "Smart sync",
                hint: "skip if no changes",
                on: self.smart_sync,
                indent: true,
                disabled: self.skip_rds_sync,
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::Separator,
            GeneralToggleRow::Back,
        ]
    }

    pub fn td_general_items(&self) -> Vec<GeneralToggleRow> {
        vec![
            GeneralToggleRow::Toggle {
                kind: ToggleKind::UseTimeCache,
                label: "Use time cache",
                hint: "",
                on: self.td_use_time_cache,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipCurrentWeek,
                label: "Include current week",
                hint: "show incomplete week",
                on: !self.td_skip_current_week,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::ShowCumulative,
                label: "Show cumulative balance",
                hint: "running hour balance",
                on: self.td_show_cumulative,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::TimezoneSelector {
                value: TIMEZONES[self.td_timezone_idx].to_string(),
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::Separator,
            GeneralToggleRow::Back,
        ]
    }

    pub fn td_settings_items(&self) -> Vec<TimeSettingRow> {
        vec![
            TimeSettingRow::EditField {
                field: TimeDoctorField::Email,
                label: "Email",
                value: self.td_email.clone(),
                masked: false,
            },
            TimeSettingRow::Password {
                is_set: self.td_password_is_set,
            },
            TimeSettingRow::Blank,
            TimeSettingRow::Separator,
            TimeSettingRow::Back,
        ]
    }

    pub fn cp_list_items(&self) -> Vec<CpListRow> {
        let mut rows: Vec<CpListRow> = self
            .contract_periods
            .iter()
            .enumerate()
            .map(|(i, p)| CpListRow::Period {
                index: i,
                from: p.from,
                weekly_hours: p.weekly_hours,
            })
            .collect();
        rows.push(CpListRow::Blank);
        rows.push(CpListRow::Separator);
        rows.push(CpListRow::MondayField);
        rows.push(CpListRow::HoursField);
        rows.push(CpListRow::Blank);
        rows.push(CpListRow::SavePeriod);
        rows.push(CpListRow::Separator);
        rows.push(CpListRow::Blank);
        rows.push(CpListRow::Back);
        rows
    }

    pub fn repo_manager_items(&self) -> Vec<RepoManagerRow> {
        let mut rows: Vec<RepoManagerRow> = vec![];
        for r in &self.repos {
            rows.push(RepoManagerRow::RepoToggle {
                name: r.name.clone(),
                url: r.url.clone(),
                enabled: r.enabled,
            });
        }
        rows.push(RepoManagerRow::Blank);
        rows.push(RepoManagerRow::AddUrl);
        rows.push(RepoManagerRow::Blank);
        rows.push(RepoManagerRow::RemoveReposLink);
        rows.push(RepoManagerRow::Blank);
        rows.push(RepoManagerRow::Separator);
        rows.push(RepoManagerRow::Back);
        rows
    }

    pub fn remove_repo_items(&self) -> Vec<RemoveRepoRow> {
        let mut rows: Vec<RemoveRepoRow> = vec![];
        for r in &self.repos {
            rows.push(RemoveRepoRow::RepoDelete {
                name: r.name.clone(),
                url: r.url.clone(),
            });
        }
        rows.push(RemoveRepoRow::Blank);
        rows.push(RemoveRepoRow::Separator);
        rows.push(RemoveRepoRow::Back);
        rows
    }
}
