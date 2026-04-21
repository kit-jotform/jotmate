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
    /// Edit default flags and credentials
    Settings,
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
