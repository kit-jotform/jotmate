use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

use crate::cli::SyncArgs;

/// The sync script embedded at compile time.
const SYNC_SCRIPT: &str = include_str!("../../scripts/run-sync.sh");

/// Patches the GITHUB_BASE variable in the embedded script to point to the
/// discovered base directory where all repos live.
fn build_patched_script(github_base: &Path) -> String {
    let base_str = github_base.to_string_lossy();
    SYNC_SCRIPT
        .lines()
        .map(|line| {
            if line.starts_with("GITHUB_BASE=") {
                format!("GITHUB_BASE=\"{base_str}\"")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_flag_args(args: &SyncArgs) -> Vec<String> {
    let mut flags = Vec::new();
    if let Some(only) = &args.only {
        flags.push("--only".to_string());
        flags.push(only.join(","));
    }
    if args.sync_all {
        flags.push("--sync-all".to_string());
    }
    flags
}

/// Run the sync script in plain CLI mode (inheriting stdio).
pub fn run_cli(args: &SyncArgs, github_base: &Path) -> Result<()> {
    let patched = build_patched_script(github_base);

    let mut tmp =
        NamedTempFile::with_suffix(".sh").context("Failed to create temporary script file")?;
    tmp.write_all(patched.as_bytes())
        .context("Failed to write patched script")?;
    tmp.flush()?;

    // Make it executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tmp.as_file().metadata()?.permissions();
        perms.set_mode(0o755);
        tmp.as_file().set_permissions(perms)?;
    }

    let tmp_path = tmp.path().to_path_buf();
    // Keep the file alive for the duration of the command
    let _tmp_guard = tmp;

    let flags = build_flag_args(args);
    let status = std::process::Command::new("bash")
        .arg(&tmp_path)
        .args(&flags)
        .status()
        .context("Failed to execute sync script")?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        anyhow::bail!("Sync script exited with code {code}");
    }

    Ok(())
}
