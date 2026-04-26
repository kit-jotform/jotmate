use anyhow::{anyhow, Result};

pub const REPO: &str = "kit-jotform/Jotmate";
pub const BINARY: &str = "jotmate";

pub fn asset_url(version_tag: &str) -> Result<String> {
    let target = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        (os, arch) => {
            return Err(anyhow!(
                "unsupported platform: {os}/{arch} — no prebuilt release asset"
            ));
        }
    };
    Ok(format!(
        "https://github.com/{REPO}/releases/download/{version_tag}/{BINARY}-{version_tag}-{target}.tar.gz"
    ))
}
