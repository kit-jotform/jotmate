pub const MAIN_ITEMS: &[(&str, &str)] = &[
    ("Sync", "Sync RDS to upstream"),
    ("Time Doctor", "Track your work hours"),
    ("Settings", "Configure jotmate"),
    ("Exit", ""),
];

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Screen {
    MainMenu,
    Settings,
    SyncGeneralSettings,
    RepoManager,
    RemoveRepos,
    TdGeneralSettings,
    TimeDoctorSettings,
    ContractPeriods,
    SyncProgress,
    TimeDoctorReport,
}
