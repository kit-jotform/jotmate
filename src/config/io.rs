use anyhow::{Context, Result};

use super::types::Config;
use crate::ctx::Paths;

pub fn load(paths: &Paths) -> Result<Config> {
    let path = paths.config_file();
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config from {}", path.display()))?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

pub fn save(paths: &Paths, config: &Config) -> Result<()> {
    let path = paths.config_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory {}", parent.display()))?;
    }
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;
    Ok(())
}
