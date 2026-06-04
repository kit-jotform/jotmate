use chrono::NaiveDate;

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    AddingRepo(String),
    ConfirmDelete(String),
    ConfirmDeletePeriod(usize),
    ConfirmDeleteOffWeek(usize),
    SelectingTimezone(usize),
    EditingCpMonday(NaiveDate),
    EditingCpHours(usize),
    EditingOwMonday(NaiveDate),
    EditingField { field: TimeDoctorField, buf: String },
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TimeDoctorField {
    Email,
    Password,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CycleTarget {
    Timezone,
    CpMonday,
    CpHours,
    OwMonday,
}

#[derive(Clone, Copy)]
pub enum ToggleKind {
    UseCache,
    SkipForkSync,
    SkipRebase,
    SkipRdsSync,
    SmartSync,
    SkipCurrentWeek,
    UseTimeCache,
}

impl ToggleKind {
    pub fn is_sync(self) -> bool {
        matches!(
            self,
            ToggleKind::UseCache
                | ToggleKind::SkipForkSync
                | ToggleKind::SkipRebase
                | ToggleKind::SkipRdsSync
                | ToggleKind::SmartSync
        )
    }
}

pub const SECTION_RDS_SYNC: &str = "RDS Sync";
pub const SECTION_TIME_DOCTOR: &str = "Time Doctor";

#[derive(Clone)]
pub enum SettingRow {
    Separator(&'static str),
    Divider,
    Blank,
    SyncGeneralLink,
    ManageRepos,
    TdGeneralLink,
    TimeDoctorSettings,
    ContractPeriodsLink,
    OffWeeksLink,
    Back,
}

impl SettingRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            SettingRow::SyncGeneralLink
                | SettingRow::ManageRepos
                | SettingRow::TdGeneralLink
                | SettingRow::TimeDoctorSettings
                | SettingRow::ContractPeriodsLink
                | SettingRow::OffWeeksLink
                | SettingRow::Back
        )
    }
}

#[derive(Clone)]
pub enum GeneralToggleRow {
    Toggle {
        kind: ToggleKind,
        label: &'static str,
        hint: &'static str,
        on: bool,
        indent: bool,
        disabled: bool,
    },
    TimezoneSelector {
        value: String,
    },
    Blank,
    Separator,
    Back,
}

impl GeneralToggleRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            GeneralToggleRow::Toggle { .. }
                | GeneralToggleRow::TimezoneSelector { .. }
                | GeneralToggleRow::Back
        )
    }
}

#[derive(Clone)]
pub enum TimeSettingRow {
    EditField {
        field: TimeDoctorField,
        label: &'static str,
        value: String,
        masked: bool,
    },
    Password {
        is_set: bool,
    },
    Blank,
    Separator,
    Back,
}

impl TimeSettingRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            TimeSettingRow::EditField { .. }
                | TimeSettingRow::Password { .. }
                | TimeSettingRow::Back
        )
    }
}

#[derive(Clone)]
pub enum CpListRow {
    Period {
        index: usize,
        from: NaiveDate,
        weekly_hours: f64,
    },
    SectionTitle(&'static str),
    Blank,
    Separator,
    MondayField,
    HoursField,
    SavePeriod,
    Back,
}

impl CpListRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            CpListRow::Period { .. }
                | CpListRow::MondayField
                | CpListRow::HoursField
                | CpListRow::SavePeriod
                | CpListRow::Back
        )
    }
}

#[derive(Clone)]
pub enum OwListRow {
    OffWeek { index: usize, monday: NaiveDate },
    SectionTitle(&'static str),
    Blank,
    Separator,
    MondayField,
    SaveOffWeek,
    Back,
}

impl OwListRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            OwListRow::OffWeek { .. }
                | OwListRow::MondayField
                | OwListRow::SaveOffWeek
                | OwListRow::Back
        )
    }
}

#[derive(Clone)]
pub enum RepoManagerRow {
    Blank,
    Separator,
    RepoToggle {
        name: String,
        url: String,
        enabled: bool,
    },
    AddUrl,
    RemoveReposLink,
    Back,
}

impl RepoManagerRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            RepoManagerRow::RepoToggle { .. }
                | RepoManagerRow::AddUrl
                | RepoManagerRow::RemoveReposLink
                | RepoManagerRow::Back
        )
    }
}

#[derive(Clone)]
pub enum RemoveRepoRow {
    Blank,
    Separator,
    RepoDelete { name: String, url: String },
    Back,
}

impl RemoveRepoRow {
    pub fn is_interactive(&self) -> bool {
        matches!(self, RemoveRepoRow::RepoDelete { .. } | RemoveRepoRow::Back)
    }
}
