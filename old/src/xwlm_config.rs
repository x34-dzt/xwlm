use serde::Deserialize;
use serde::Serialize;
use std::{fs, io, path::PathBuf};
use thiserror::Error;

use crate::utils;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("invalid config path: {0}")]
    Path(#[from] utils::UtilsError),

    #[error("failed to read config at {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("failed to write config at {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("invalid toml in config: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("failed to serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub monitor_config_path: PathBuf,
    #[serde(default = "default_workspace_count")]
    pub workspace_count: usize,
}

pub fn load_config() -> Result<Config, ConfigError> {
    load_from_path("~/.config/xwlm/config.toml")
}

pub fn save_config(config: &Config) -> Result<(), ConfigError> {
    save_to_path("~/.config/xwlm/config.toml", config)
}

fn load_from_path(path: &str) -> Result<Config, ConfigError> {
    let expanded_path = utils::expand_tilde(path)?;
    let file_content =
        fs::read_to_string(expanded_path).map_err(|e| ConfigError::Read {
            path: path.to_string(),
            source: e,
        })?;

    let config = toml::from_str(&file_content)?;

    Ok(config)
}

fn save_to_path(path: &str, config: &Config) -> Result<(), ConfigError> {
    let expanded_path = utils::expand_tilde(path)?;

    if let Some(parent) = expanded_path.parent() {
        fs::create_dir_all(parent).map_err(|e| ConfigError::Write {
            path: parent.to_string_lossy().into(),
            source: e,
        })?;
    }

    let comment = "# This file is managed by xwlm. Do not edit manually.\n# The monitor_config_path should always point to a file that ONLY contains\n# monitor and workspace configurations. Any other settings in that file will be\n# overwritten when xwlm saves changes.\n\n";
    let toml_string = toml::to_string_pretty(config)?;
    let final_content = format!("{}{}", comment, toml_string);

    fs::write(&expanded_path, final_content).map_err(|e| {
        ConfigError::Write {
            path: expanded_path.to_string_lossy().into(),
            source: e,
        }
    })?;

    Ok(())
}

fn default_workspace_count() -> usize {
    10
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const TEST_PATH: &str = "~/.config/test-xwlm/config.toml";

    #[test]
    fn save_then_load_config_works() {
        let config = Config {
            monitor_config_path: PathBuf::from("/tmp/test.conf"),
            workspace_count: 5,
        };

        save_to_path(TEST_PATH, &config).unwrap();

        let loaded = load_from_path(TEST_PATH).unwrap();

        assert_eq!(loaded.workspace_count, config.workspace_count);

        assert_eq!(loaded.monitor_config_path, config.monitor_config_path);
    }

    #[test]
    fn load_fails_when_file_missing() {
        let path = "~/.config/test-xwlm/missing.toml";

        let result = load_from_path(path);

        assert!(result.is_err());
    }

    #[test]
    fn load_fails_on_invalid_toml() {
        let path = "~/.config/test-xwlm/bad.toml";

        let expanded = utils::expand_tilde(path).unwrap();

        if let Some(parent) = expanded.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        std::fs::write(&expanded, "not = = = toml").unwrap();

        let result = load_from_path(path);

        assert!(matches!(result, Err(ConfigError::Parse(_))));
    }
}
