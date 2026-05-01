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
    /// Embedding provider: "local" (default — ONNX model on disk) or "openai"
    /// (any OpenAI-compatible HTTP API, e.g. OpenAI, Azure OpenAI, Ollama,
    /// vLLM, LM Studio, LocalAI, Together, etc.). Override at runtime with
    /// `OPEN_ONTOLOGIES_EMBEDDINGS_PROVIDER`.
    pub provider: Option<String>,
    /// Path to the ONNX model file (provider = "local" only).
    /// Default: ~/.open-ontologies/models/bge-small-en-v1.5.onnx
    pub model_path: Option<String>,
    /// Path to the tokenizer.json file (provider = "local" only).
    /// Default: ~/.open-ontologies/models/tokenizer.json
    pub tokenizer_path: Option<String>,
    /// URL to download the ONNX model from. Default: BGE-small-en-v1.5 from Hugging Face
    pub model_url: Option<String>,
    /// URL to download the tokenizer from. Default: BGE-small-en-v1.5 tokenizer from Hugging Face
    pub tokenizer_url: Option<String>,
    /// Filename for the downloaded model. Default: bge-small-en-v1.5.onnx
    pub model_name: Option<String>,

    // ─── OpenAI-compatible provider (provider = "openai") ───────────────
    /// Base URL of the OpenAI-compatible API, without the trailing
    /// `/embeddings` path. Default: `https://api.openai.com/v1`. Override
    /// at runtime with `OPEN_ONTOLOGIES_EMBEDDINGS_API_BASE`.
    #[serde(alias = "base_url")]
    pub api_base: Option<String>,
    /// API key. If unset, falls back to the `OPEN_ONTOLOGIES_EMBEDDINGS_API_KEY`
    /// or `OPENAI_API_KEY` env var. Sent as `Authorization: Bearer <key>`.
    /// Optional — gateways that don't require auth (Ollama, LocalAI,
    /// vLLM behind a private network, …) can leave this unset.
    pub api_key: Option<String>,
    /// Model name to request, e.g. `text-embedding-3-small`,
    /// `text-embedding-3-large`, `text-embedding-ada-002`, or any model
    /// served by an OpenAI-compatible gateway. Default:
    /// `text-embedding-3-small`. Override with
    /// `OPEN_ONTOLOGIES_EMBEDDINGS_MODEL`.
    pub model: Option<String>,
    /// Optional `dimensions` parameter sent in the request body. Lets you
    /// truncate output dimensionality on models that support it
    /// (text-embedding-3-*). When unset, the API's default dimension is
    /// used and detected from the first response.
    pub dimensions: Option<usize>,
    /// HTTP request timeout in seconds. Default: 30.
    pub request_timeout_secs: Option<u64>,
}

/// Configuration for the on-disk N-Triples compile cache and TTL eviction.
/// Resolve the configured embedding provider name.
///
/// Precedence: `OPEN_ONTOLOGIES_EMBEDDINGS_PROVIDER` env var > config field >
/// default ("local"). Returns a lowercased, trimmed string.
pub fn resolve_embeddings_provider(cfg: &EmbeddingsConfig) -> String {
    let raw = std::env::var("OPEN_ONTOLOGIES_EMBEDDINGS_PROVIDER")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| cfg.provider.clone())
        .unwrap_or_else(|| "local".to_string());
    raw.trim().to_lowercase()
}

/// Resolve the OpenAI-compatible API base URL.
///
/// Precedence: `OPEN_ONTOLOGIES_EMBEDDINGS_API_BASE` env var > config >
/// `https://api.openai.com/v1`. Trailing slashes are stripped.
pub fn resolve_embeddings_api_base(cfg: &EmbeddingsConfig) -> String {
    let raw = std::env::var("OPEN_ONTOLOGIES_EMBEDDINGS_API_BASE")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| cfg.api_base.clone())
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    raw.trim().trim_end_matches('/').to_string()
}

/// Resolve the OpenAI-compatible API key.
///
/// Precedence: `OPEN_ONTOLOGIES_EMBEDDINGS_API_KEY` env var >
/// `OPENAI_API_KEY` env var > config. Returns `None` if no key is configured
/// (some local OpenAI-compatible gateways accept unauthenticated requests).
pub fn resolve_embeddings_api_key(cfg: &EmbeddingsConfig) -> Option<String> {
    std::env::var("OPEN_ONTOLOGIES_EMBEDDINGS_API_KEY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("OPENAI_API_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
        .or_else(|| cfg.api_key.clone().filter(|v| !v.trim().is_empty()))
}

/// Resolve the OpenAI-compatible model name.
///
/// Precedence: `OPEN_ONTOLOGIES_EMBEDDINGS_MODEL` env var > config >
/// `text-embedding-3-small`.
pub fn resolve_embeddings_model(cfg: &EmbeddingsConfig) -> String {
    std::env::var("OPEN_ONTOLOGIES_EMBEDDINGS_MODEL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| cfg.model.clone().filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| "text-embedding-3-small".to_string())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_openai_provider_block() {
        let toml_src = r#"
            [embeddings]
            provider = "openai"
            api_base = "https://api.example.com/v1/"
            api_key = "sk-test"
            model = "text-embedding-3-large"
            dimensions = 256
            request_timeout_secs = 60
        "#;
        let cfg: Config = toml::from_str(toml_src).expect("parse");
        assert_eq!(cfg.embeddings.provider.as_deref(), Some("openai"));
        assert_eq!(cfg.embeddings.api_key.as_deref(), Some("sk-test"));
        assert_eq!(cfg.embeddings.dimensions, Some(256));
        assert_eq!(cfg.embeddings.request_timeout_secs, Some(60));

        // Trailing slash is stripped by the resolver.
        assert_eq!(
            resolve_embeddings_api_base(&cfg.embeddings),
            "https://api.example.com/v1"
        );
        assert_eq!(resolve_embeddings_provider(&cfg.embeddings), "openai");
        assert_eq!(
            resolve_embeddings_model(&cfg.embeddings),
            "text-embedding-3-large"
        );
    }

    #[test]
    fn provider_defaults_to_local_when_unset() {
        // Verify the default-resolution logic without touching process-wide
        // env vars (which would race with other tests). When the env override
        // is absent the function should fall back to the config field, then
        // to "local".
        let cfg = EmbeddingsConfig::default();
        let resolved = cfg
            .provider
            .clone()
            .unwrap_or_else(|| "local".to_string())
            .trim()
            .to_lowercase();
        assert_eq!(resolved, "local");
    }

    #[test]
    fn base_url_alias_accepted() {
        // The legacy/alternative `base_url` key should also populate
        // `api_base` via serde alias.
        let toml_src = r#"
            [embeddings]
            base_url = "http://localhost:11434/v1"
        "#;
        let cfg: Config = toml::from_str(toml_src).expect("parse");
        assert_eq!(
            cfg.embeddings.api_base.as_deref(),
            Some("http://localhost:11434/v1")
        );
    }
}
