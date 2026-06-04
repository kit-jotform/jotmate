use crate::tui::rows::{
    CpListRow, GeneralToggleRow, OwListRow, RemoveRepoRow, RepoManagerRow, SettingRow,
    TimeDoctorField, TimeSettingRow, ToggleKind,
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
            SettingRow::OffWeeksLink,
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
                on: self.sync.use_cache,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipForkSync,
                label: "Fork sync",
                hint: "fetch+merge+push upstream",
                on: !self.sync.skip_fork_sync,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipRebase,
                label: "Rebase",
                hint: "rebase branch after merge",
                on: !self.sync.skip_rebase,
                indent: true,
                disabled: self.sync.skip_fork_sync,
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipRdsSync,
                label: "RDS sync",
                hint: "./sync in each repo",
                on: !self.sync.skip_rds_sync,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SmartSync,
                label: "Smart sync",
                hint: "skip if no changes",
                on: self.sync.smart_sync,
                indent: true,
                disabled: self.sync.skip_rds_sync,
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
                on: self.td.use_time_cache,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipCurrentWeek,
                label: "Include current week",
                hint: "show incomplete week",
                on: !self.td.skip_current_week,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::TimezoneSelector {
                value: TIMEZONES[self.td.timezone_idx].to_string(),
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
                value: self.td.email.clone(),
                masked: false,
            },
            TimeSettingRow::Password {
                is_set: self.password_is_set(),
            },
            TimeSettingRow::Blank,
            TimeSettingRow::Separator,
            TimeSettingRow::Back,
        ]
    }

    pub fn cp_list_items(&self) -> Vec<CpListRow> {
        let periods = self
            .td
            .contract_periods
            .iter()
            .enumerate()
            .map(|(i, p)| CpListRow::Period {
                index: i,
                from: p.from,
                weekly_hours: p.weekly_hours,
            });
        let mut rows = vec![
            CpListRow::SectionTitle("Contract Periods"),
            CpListRow::Blank,
        ];
        rows.extend(periods);
        rows.extend([
            CpListRow::Blank,
            CpListRow::Separator,
            CpListRow::MondayField,
            CpListRow::HoursField,
            CpListRow::Blank,
            CpListRow::SavePeriod,
            CpListRow::Separator,
            CpListRow::Blank,
            CpListRow::Back,
        ]);
        rows
    }

    pub fn ow_list_items(&self) -> Vec<OwListRow> {
        let off_weeks =
            self.td
                .off_weeks
                .iter()
                .enumerate()
                .map(|(i, monday)| OwListRow::OffWeek {
                    index: i,
                    monday: *monday,
                });
        let mut rows = vec![OwListRow::SectionTitle("Off Weeks"), OwListRow::Blank];
        rows.extend(off_weeks);
        rows.extend([
            OwListRow::Blank,
            OwListRow::Separator,
            OwListRow::MondayField,
            OwListRow::Blank,
            OwListRow::SaveOffWeek,
            OwListRow::Separator,
            OwListRow::Blank,
            OwListRow::Back,
        ]);
        rows
    }

    pub fn repo_manager_items(&self) -> Vec<RepoManagerRow> {
        let mut rows: Vec<RepoManagerRow> = self
            .sync
            .repos
            .iter()
            .map(|r| RepoManagerRow::RepoToggle {
                name: r.name.clone(),
                url: r.url.clone(),
                enabled: r.enabled,
            })
            .collect();
        rows.extend([
            RepoManagerRow::Blank,
            RepoManagerRow::AddUrl,
            RepoManagerRow::Blank,
            RepoManagerRow::RemoveReposLink,
            RepoManagerRow::Blank,
            RepoManagerRow::Separator,
            RepoManagerRow::Back,
        ]);
        rows
    }

    pub fn remove_repo_items(&self) -> Vec<RemoveRepoRow> {
        let mut rows: Vec<RemoveRepoRow> = self
            .sync
            .repos
            .iter()
            .map(|r| RemoveRepoRow::RepoDelete {
                name: r.name.clone(),
                url: r.url.clone(),
            })
            .collect();
        rows.extend([
            RemoveRepoRow::Blank,
            RemoveRepoRow::Separator,
            RemoveRepoRow::Back,
        ]);
        rows
    }
}
