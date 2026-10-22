use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "jotmate", about = "Jotform developer productivity CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Sync git forks with upstream
    Sync(SyncArgs),
    /// Check TimeDoctor work-hour stats
    Time(TimeArgs),
    /// Check weekly work hours from Ar-Ge Portal PDKS records
    Pdks(PdksArgs),
    /// View or update your Jotmate profile
    Profile(ProfileArgs),
    /// Edit default flags and credentials
    Settings,
    /// Check for and install a newer release
    Update,
}

#[derive(Args, Clone, Debug, Default)]
pub struct ProfileArgs {
    /// Set developer status (true or false)
    #[arg(long)]
    pub developer: Option<bool>,
    /// Set office: hacettepe, izmir, odtu, istanbul
    #[arg(long)]
    pub office: Option<String>,
    /// Set part-time status (true or false)
    #[arg(long)]
    pub part_time: Option<bool>,
}

#[derive(Args, Clone, Debug)]
pub struct PdksArgs {
    #[command(subcommand)]
    pub command: Option<PdksCommand>,

    /// Any date in the week to report (YYYY-MM-DD). Defaults to the current week.
    #[arg(long)]
    pub week: Option<chrono::NaiveDate>,

    /// Portal company id
    #[arg(long, default_value_t = 21)]
    pub company_id: u64,

    /// Portal term id. Omit to select the term(s) that cover the requested week.
    #[arg(long)]
    pub period_id: Option<u64>,

    /// Portal employee id. Omit when the account can see only one employee.
    #[arg(long)]
    pub personnel_id: Option<u64>,

    /// Required weekly hours
    #[arg(long, default_value_t = crate::config::PDKS_WEEKLY_TARGET_HOURS)]
    pub target_hours: f64,
}

impl Default for PdksArgs {
    fn default() -> Self {
        Self {
            command: None,
            week: None,
            company_id: 21,
            period_id: None,
            personnel_id: None,
            target_hours: crate::config::PDKS_WEEKLY_TARGET_HOURS,
        }
    }
}

#[derive(Subcommand, Clone, Debug)]
pub enum PdksCommand {
    /// Open the secure portal sign-in window
    Login,
}

#[derive(Args, Clone, Debug, Default)]
pub struct SyncArgs {
    /// Only sync specific repos (comma-separated names, forces even if disabled in config)
    #[arg(long, value_delimiter = ',')]
    pub only: Option<Vec<String>>,

    /// Sync all repos including disabled ones, bypass smart sync
    #[arg(long)]
    pub sync_all: bool,

    /// Skip fork sync entirely; run only RDS sync
    #[arg(long)]
    pub rds_only: bool,

    /// Skip fork sync (fetch upstream + merge + push)
    #[arg(long)]
    pub skip_fork_sync: bool,

    /// Skip rebasing current branch after fork sync
    #[arg(long)]
    pub skip_rebase: bool,

    /// Skip running ./sync in each repo
    #[arg(long)]
    pub skip_rds_sync: bool,

    /// Disable smart sync: run RDS sync unconditionally (short: -S)
    #[arg(long, short = 'S')]
    pub no_smart_sync: bool,

    /// Ignore the repo path discovery cache for this run
    #[arg(long)]
    pub no_cache: bool,

    /// Skip git fetch upstream (use already-fetched upstream refs)
    #[arg(long)]
    pub skip_fetch: bool,
}

#[derive(Args, Clone, Debug, Default)]
pub struct TimeArgs {
    /// Skip reporting for the current (incomplete) week
    #[arg(long)]
    pub skip_current_week: bool,

    /// Bypass local week cache and re-fetch from API
    #[arg(long)]
    pub no_cache: bool,
}
