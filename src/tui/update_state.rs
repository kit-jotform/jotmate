use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::update::{UpdatePhase, UpdateUpdate};

pub struct UpdateScreenState {
    pub phase: UpdatePhase,
    pub tick: u8,
    pub update_rx: mpsc::UnboundedReceiver<UpdateUpdate>,
    pub task_handle: Option<JoinHandle<()>>,
}
