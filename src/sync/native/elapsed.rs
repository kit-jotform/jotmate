use std::time::Instant;
use tokio::sync::mpsc;

use crate::tui::app::SyncUpdate;

/// Emit `Elapsed` updates for each tracked repo every 100ms.
/// Runs until aborted.
pub(super) async fn track_elapsed(
    tx: mpsc::UnboundedSender<SyncUpdate>,
    starts: Vec<(usize, Instant)>,
) {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        for &(idx, start) in &starts {
            let _ = tx.send(SyncUpdate::Elapsed(idx, start.elapsed().as_secs_f64()));
        }
    }
}
