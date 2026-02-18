use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

fn default_workspace_count() -> usize {
    10
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub monitor_config_path: String,
    #[serde(default = "default_workspace_count")]
    pub workspace_count: usize,
}

pub fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| {
        color_eyre::eyre::eyre!("Could not determine config directory")
    })?;
    Ok(base.join("xwlm").join("config.toml"))
}

pub fn load() -> Result<Option<AppConfig>> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let contents =
        fs::read_to_string(&path).wrap_err("Failed to read config file")?;
    let config: AppConfig =
        toml::from_str(&contents).wrap_err("Failed to parse config file")?;
    Ok(Some(config))
}

pub fn save(config: &AppConfig) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .wrap_err("Failed to create config directory")?;
    }
    let contents = toml::to_string_pretty(config)
        .wrap_err("Failed to serialize config")?;
    fs::write(&path, contents).wrap_err("Failed to write config file")?;
    Ok(())
}
