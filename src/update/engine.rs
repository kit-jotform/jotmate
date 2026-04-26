use anyhow::{anyhow, Context, Result};
use flate2::read::GzDecoder;
use std::fs;
use std::io::Cursor;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;
use tar::Archive;
use tokio::sync::mpsc;

use super::api::{check_for_update, UpdateAvailable};
use super::target::{asset_url, BINARY};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdatePhase {
    Checking,
    UpToDate,
    Downloading,
    Extracting,
    Replacing,
    Done(String),
    Failed(String),
}

impl UpdatePhase {
    pub fn label(&self) -> &'static str {
        match self {
            UpdatePhase::Checking => "checking for updates…",
            UpdatePhase::UpToDate => "up to date",
            UpdatePhase::Downloading => "downloading…",
            UpdatePhase::Extracting => "extracting…",
            UpdatePhase::Replacing => "installing…",
            UpdatePhase::Done(_) => "done",
            UpdatePhase::Failed(_) => "failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            UpdatePhase::UpToDate | UpdatePhase::Done(_) | UpdatePhase::Failed(_)
        )
    }
}

#[derive(Clone, Debug)]
pub enum UpdateUpdate {
    Phase(UpdatePhase),
}

pub async fn run_update(tx: mpsc::UnboundedSender<UpdateUpdate>) {
    send_phase(&tx, UpdatePhase::Checking);

    let available = match check_for_update().await {
        Ok(Some(a)) => a,
        Ok(None) => {
            send_phase(&tx, UpdatePhase::UpToDate);
            return;
        }
        Err(e) => {
            send_phase(
                &tx,
                UpdatePhase::Failed(format!("could not check for updates: {e}")),
            );
            return;
        }
    };

    if let Err(e) = perform_update(&available, &tx).await {
        send_phase(&tx, UpdatePhase::Failed(format!("{e:#}")));
    }
}

fn send_phase(tx: &mpsc::UnboundedSender<UpdateUpdate>, phase: UpdatePhase) {
    let _ = tx.send(UpdateUpdate::Phase(phase));
}

async fn perform_update(
    available: &UpdateAvailable,
    tx: &mpsc::UnboundedSender<UpdateUpdate>,
) -> Result<()> {
    send_phase(tx, UpdatePhase::Downloading);
    let bytes = download(&available.tag).await?;

    send_phase(tx, UpdatePhase::Extracting);
    let staged = tokio::task::spawn_blocking(move || extract_binary(&bytes))
        .await
        .context("extraction task panicked")??;

    send_phase(tx, UpdatePhase::Replacing);
    let target = current_exe_path()?;
    let staged_for_move = staged.clone();
    tokio::task::spawn_blocking(move || fs::rename(&staged_for_move, &target))
        .await
        .context("replace task panicked")?
        .with_context(|| format!("replacing running binary at {}", staged.display()))?;

    send_phase(tx, UpdatePhase::Done(available.version.clone()));
    Ok(())
}

async fn download(tag: &str) -> Result<Vec<u8>> {
    let url = asset_url(tag)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let resp = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("requesting {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "download failed: {url} returned status {}",
            resp.status()
        ));
    }
    Ok(resp.bytes().await?.to_vec())
}

fn extract_binary(bytes: &[u8]) -> Result<PathBuf> {
    let exe = current_exe_path()?;
    let parent = exe
        .parent()
        .ok_or_else(|| anyhow!("running binary has no parent dir: {}", exe.display()))?;
    let staging = parent.join(format!(".{BINARY}.update.tmp"));
    let _ = fs::remove_file(&staging);

    let mut archive = Archive::new(GzDecoder::new(Cursor::new(bytes)));
    for entry in archive.entries().context("reading tar entries")? {
        let mut entry = entry.context("reading tar entry")?;
        let is_target = entry
            .path()
            .ok()
            .and_then(|p| p.file_name().map(|n| n == BINARY))
            .unwrap_or(false);
        if is_target {
            entry
                .unpack(&staging)
                .with_context(|| format!("unpacking to {}", staging.display()))?;
            fs::set_permissions(&staging, fs::Permissions::from_mode(0o755))
                .with_context(|| format!("chmod +x {}", staging.display()))?;
            return Ok(staging);
        }
    }
    Err(anyhow!(
        "tarball did not contain expected binary `{BINARY}`"
    ))
}

// Canonicalize so we overwrite the real binary, not the `jf` symlink.
fn current_exe_path() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("locating running executable")?;
    fs::canonicalize(&exe).with_context(|| format!("canonicalizing {}", exe.display()))
}
