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
    /// Only sync specific projects (comma-separated: Jotform3,vendors,core,backend,frontend)
    #[arg(long, value_delimiter = ',')]
    pub only: Option<Vec<String>>,

    /// Force sync all repos regardless of upstream diff
    #[arg(long)]
    pub sync_all: bool,
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
