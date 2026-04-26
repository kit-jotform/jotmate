#[derive(Clone, Debug)]
pub struct MainMenuItem {
    pub kind: MainMenuKind,
    pub name: String,
    pub desc: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainMenuKind {
    Sync,
    TimeDoctor,
    Update,
    Settings,
    Exit,
}

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
    UpdateProgress,
}
