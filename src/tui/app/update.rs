use tokio::sync::mpsc;

use crate::tui::update_state::UpdateScreenState;
use crate::update::{run_update, UpdatePhase, UpdateUpdate};

use super::screen::Screen;
use super::App;

impl App {
    pub fn launch_update(&mut self) {
        let (tx, rx) = mpsc::unbounded_channel::<UpdateUpdate>();
        let handle = tokio::spawn(async move {
            run_update(tx).await;
        });
        self.update_state = Some(UpdateScreenState {
            phase: UpdatePhase::Checking,
            tick: 0,
            update_rx: rx,
            task_handle: Some(handle),
        });
        self.screen = Screen::UpdateProgress;
    }

    pub fn apply_update_event(&mut self, event: UpdateUpdate) {
        let UpdateUpdate::Phase(phase) = event;
        if matches!(&phase, UpdatePhase::Done(_) | UpdatePhase::UpToDate) {
            self.available_update = Some(None);
        }
        if let Some(state) = self.update_state.as_mut() {
            state.phase = phase;
        }
    }

    pub fn update_is_terminal(&self) -> bool {
        self.update_state
            .as_ref()
            .map(|s| s.phase.is_terminal())
            .unwrap_or(true)
    }

    pub fn cancel_update(&mut self) {
        if let Some(state) = self.update_state.take() {
            if let Some(handle) = state.task_handle {
                handle.abort();
            }
        }
    }
}
