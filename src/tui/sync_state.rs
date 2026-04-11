use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

// ── Fork sync status ──────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum ForkStatus {
    Pending,
    FetchingUpstream,
    CheckingDiff,
    UpToDate,
    Stashing,
    CheckingOut,
    Merging,
    PushingDefault,
    Rebasing,
    PushingBranch,
    Unstashing,
    Done,
    Skipped(String),
    Error(String),
}

impl ForkStatus {
    pub fn label(&self) -> &str {
        match self {
            ForkStatus::Pending => "waiting…",
            ForkStatus::FetchingUpstream => "fetching upstream…",
            ForkStatus::CheckingDiff => "checking diff…",
            ForkStatus::UpToDate => "up to date",
            ForkStatus::Stashing => "stashing…",
            ForkStatus::CheckingOut => "checking out…",
            ForkStatus::Merging => "merging…",
            ForkStatus::PushingDefault => "pushing default…",
            ForkStatus::Rebasing => "rebasing…",
            ForkStatus::PushingBranch => "pushing branch…",
            ForkStatus::Unstashing => "unstashing…",
            ForkStatus::Done => "done",
            ForkStatus::Skipped(_) => "skipped",
            ForkStatus::Error(_) => "error",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ForkStatus::Done | ForkStatus::UpToDate | ForkStatus::Skipped(_) | ForkStatus::Error(_)
        )
    }

    pub fn is_error(&self) -> bool {
        matches!(self, ForkStatus::Error(_))
    }
}

// ── RDS sync status ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum RdsStatus {
    Pending,
    Preparing,
    Pulling,
    Running,
    Done,
    Skipped(String),
    Error(String),
}

impl RdsStatus {
    pub fn label(&self) -> &str {
        match self {
            RdsStatus::Pending => "waiting…",
            RdsStatus::Preparing => "preparing…",
            RdsStatus::Pulling => "pulling…",
            RdsStatus::Running => "running ./sync…",
            RdsStatus::Done => "done",
            RdsStatus::Skipped(_) => "skipped",
            RdsStatus::Error(_) => "error",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            RdsStatus::Done | RdsStatus::Skipped(_) | RdsStatus::Error(_)
        )
    }

    pub fn is_error(&self) -> bool {
        matches!(self, RdsStatus::Error(_))
    }
}

// ── Per-repo sync state ───────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct RepoSyncState {
    pub name: String,
    pub path: PathBuf,
    pub fork_status: ForkStatus,
    pub rds_status: RdsStatus,
    pub started_at: Option<Instant>,
    pub elapsed_secs: f64,
}

impl RepoSyncState {
    pub fn is_complete(&self) -> bool {
        self.fork_status.is_terminal() && self.rds_status.is_terminal()
    }

    pub fn is_active(&self) -> bool {
        !self.fork_status.is_terminal() || !self.rds_status.is_terminal()
    }

    pub fn has_error(&self) -> bool {
        self.fork_status.is_error() || self.rds_status.is_error()
    }

    pub fn is_skipped(&self) -> bool {
        matches!(
            (&self.fork_status, &self.rds_status),
            (
                ForkStatus::Skipped(_) | ForkStatus::UpToDate,
                RdsStatus::Skipped(_)
            )
        )
    }
}

// ── Sync channel messages ─────────────────────────────────────────────────────

pub enum SyncUpdate {
    Fork(usize, ForkStatus),
    Rds(usize, RdsStatus),
    Elapsed(usize, f64),
}

// ── Sync screen state ─────────────────────────────────────────────────────────

pub struct SyncScreenState {
    pub repos: Vec<RepoSyncState>,
    pub tick: u8,
    pub sync_handle: Option<JoinHandle<()>>,
    pub update_rx: mpsc::UnboundedReceiver<SyncUpdate>,
}
