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
    /// Directories that act as on-disk ontology repositories. The
    /// `onto_repo_list` and `onto_repo_load` MCP tools enumerate and load
    /// RDF files (.ttl, .nt, .rdf, .owl, .nq, .trig, .jsonld) from these
    /// directories. Useful for containerized deployments where a host
    /// directory of TTL files is mounted into the server.
    ///
    /// Accepts either a TOML array under the canonical name `ontology_dirs`
    /// or, for compatibility with the original design proposal, the alias
    /// `data_dirs`. Each entry has `~` expanded to the user's home.
    #[serde(alias = "data_dirs")]
    pub ontology_dirs: Vec<String>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            data_dir: "~/.open-ontologies".into(),
            ontology_dirs: Vec::new(),
        }
    }
}

/// Resolve the configured ontology repository directories.
///
/// Behavior:
///  - If the env var `OPEN_ONTOLOGIES_ONTOLOGY_DIRS` is set and non-empty,
///    its value (split on `:` on Unix, `;` on Windows, accepting either on
///    both for convenience) overrides the config entries.
///  - Each entry has `~` expanded.
///  - Empty strings are dropped.
///  - Duplicates (after canonicalization fallback to the expanded string)
///    are removed while preserving order.
pub fn resolve_ontology_dirs(cfg: &[String]) -> Vec<std::path::PathBuf> {
    let from_env = std::env::var("OPEN_ONTOLOGIES_ONTOLOGY_DIRS").ok();
    let raw: Vec<String> = match from_env {
        Some(v) if !v.trim().is_empty() => v
            .split(|c| c == ':' || c == ';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => cfg.iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
    };
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(raw.len());
    for entry in raw {
        let expanded = expand_tilde(&entry);
        let key = std::fs::canonicalize(&expanded)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| expanded.clone());
        if seen.insert(key) {
            out.push(std::path::PathBuf::from(expanded));
        }
    }
    out
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
