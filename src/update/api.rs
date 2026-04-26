use anyhow::{anyhow, Result};
use semver::Version;
use serde::Deserialize;
use std::sync::OnceLock;
use std::time::Duration;

use super::target::REPO;

const USER_AGENT: &str = "jotmate-self-update";

fn shared_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(10))
            .build()
            .expect("build reqwest client")
    })
}

#[derive(Debug, Clone, Deserialize)]
struct LatestRelease {
    tag_name: String,
}

#[derive(Debug, Clone)]
pub struct UpdateAvailable {
    pub tag: String,
    pub version: String,
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub async fn check_for_update() -> Result<Option<UpdateAvailable>> {
    let release = fetch_latest_release().await?;
    compare(&release.tag_name, current_version())
}

async fn fetch_latest_release() -> Result<LatestRelease> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = shared_client().get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "GitHub API returned status {} for {REPO}",
            resp.status()
        ));
    }
    Ok(resp.json::<LatestRelease>().await?)
}

fn compare(latest_tag: &str, current: &str) -> Result<Option<UpdateAvailable>> {
    let latest_bare = latest_tag.strip_prefix('v').unwrap_or(latest_tag);
    let latest_ver = Version::parse(latest_bare)
        .map_err(|e| anyhow!("malformed release tag {latest_tag}: {e}"))?;
    let current_ver =
        Version::parse(current).map_err(|e| anyhow!("malformed crate version {current}: {e}"))?;
    if latest_ver > current_ver {
        Ok(Some(UpdateAvailable {
            tag: latest_tag.to_string(),
            version: latest_bare.to_string(),
        }))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_tag_is_update() {
        let r = compare("v9.9.9", "1.0.0").unwrap();
        assert!(r.is_some());
        let a = r.unwrap();
        assert_eq!(a.tag, "v9.9.9");
        assert_eq!(a.version, "9.9.9");
    }

    #[test]
    fn same_tag_is_no_update() {
        assert!(compare("v1.0.0", "1.0.0").unwrap().is_none());
    }

    #[test]
    fn older_tag_is_no_update() {
        assert!(compare("v0.0.1", "1.0.0").unwrap().is_none());
    }

    #[test]
    fn tag_without_v_prefix_works() {
        assert!(compare("2.0.0", "1.0.0").unwrap().is_some());
    }
}
