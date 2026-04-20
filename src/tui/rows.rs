use chrono::NaiveDate;

// ── Input mode (used inside RepoManager and TimeDoctorSettings) ──────────────

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    AddingRepo(String),         // buffer holds URL being typed
    ConfirmDelete(String),      // holds the repo name pending deletion
    ConfirmDeletePeriod(usize), // holds the period index pending deletion
    SelectingTimezone(usize),   // ↑↓ cycles timezone; stored value = snapshot for cancel
    EditingCpMonday(NaiveDate), // ↑↓ cycles monday date; stored value = snapshot for cancel
    EditingCpHours(usize),      // ↑↓ cycles hours option; stored value = snapshot for cancel
    EditingField {
        // editing a text field in TimeDoctorSettings
        field: TimeDoctorField,
        buf: String,
    },
}

/// Which Time Doctor field is being edited
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TimeDoctorField {
    Email,
    Password,
}

// ── Toggle kind ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum ToggleKind {
    UseCache,
    SkipForkSync,
    SkipRebase,
    SkipRdsSync,
    SmartSync,
    SkipCurrentWeek,
    UseTimeCache,
    ShowCumulative,
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

// ── Settings rows ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub enum SettingRow {
    Separator,
    Divider,
    Blank,
    SyncGeneralLink,
    ManageRepos,
    TdGeneralLink,
    TimeDoctorSettings,
    ContractPeriodsLink,
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
                | SettingRow::Back
        )
    }
}

// ── General toggle rows (shared by Sync General & TD General sub-screens) ────

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

// ── Time Doctor settings rows ─────────────────────────────────────────────────

#[derive(Clone)]
pub enum TimeSettingRow {
    /// Editable text field (email)
    EditField {
        field: TimeDoctorField,
        label: &'static str,
        value: String,
        masked: bool,
    },
    /// Password row — shows [set] / [not set] badge instead of value
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

// ── Contract period rows ──────────────────────────────────────────────────────

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

// ── Repo manager rows ─────────────────────────────────────────────────────────

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

// ── Remove repos rows ─────────────────────────────────────────────────────────

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
