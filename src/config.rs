use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub general: GeneralConfig,
    pub embeddings: EmbeddingsConfig,
    pub cache: CacheConfig,
    pub tools: ToolsConfig,
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

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct EmbeddingsConfig {
    /// Path to the ONNX model file. Default: ~/.open-ontologies/models/bge-small-en-v1.5.onnx
    pub model_path: Option<String>,
    /// Path to the tokenizer.json file. Default: ~/.open-ontologies/models/tokenizer.json
    pub tokenizer_path: Option<String>,
    /// URL to download the ONNX model from. Default: BGE-small-en-v1.5 from Hugging Face
    pub model_url: Option<String>,
    /// URL to download the tokenizer from. Default: BGE-small-en-v1.5 tokenizer from Hugging Face
    pub tokenizer_url: Option<String>,
    /// Filename for the downloaded model. Default: bge-small-en-v1.5.onnx
    pub model_name: Option<String>,
}

/// Configuration for the on-disk N-Triples compile cache and TTL eviction.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct CacheConfig {
    /// Master switch for the compile cache. When false, every load re-parses
    /// from source and no metadata is recorded.
    pub enabled: bool,
    /// Directory where N-Triples cache files are stored.
    pub dir: String,
    /// If > 0, the active ontology will be unloaded from memory after this
    /// many seconds without access. The cache file is preserved and reloaded
    /// automatically on the next query.
    pub idle_ttl_secs: u64,
    /// How often the background evictor checks idle entries (seconds).
    pub evictor_interval_secs: u64,
    /// When true, every read tool checks the source file's mtime/sha and
    /// recompiles if it changed. Off by default for predictability.
    pub auto_refresh: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: "~/.open-ontologies/cache".into(),
            idle_ttl_secs: 0,
            evictor_interval_secs: 30,
            auto_refresh: false,
        }
    }
}

/// Configuration for limiting which MCP tools are exposed.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ToolsConfig {
    /// "all" (default), "allow", or "deny".
    pub mode: String,
    /// Explicit tool names included by the filter.
    pub list: Vec<String>,
    /// Group names (e.g. "read_only") expanded into tool names.
    pub groups: Vec<String>,
}

/// Expand a leading `~` in a path to the user's home directory.
pub fn expand_tilde(path: &str) -> String {
    if (path.starts_with("~/") || path == "~")
        && let Some(home) = std::env::var_os("HOME") {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    path.to_string()
}
