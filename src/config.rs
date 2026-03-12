use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub general: GeneralConfig,
}


impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        Ok(config)
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub data_dir: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            data_dir: "~/.open-ontologies".into(),
        }
    }
}

/// Expand a leading `~` in a path to the user's home directory.
pub fn expand_tilde(path: &str) -> String {
    if (path.starts_with("~/") || path == "~")
        && let Some(home) = std::env::var_os("HOME") {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    path.to_string()
}
