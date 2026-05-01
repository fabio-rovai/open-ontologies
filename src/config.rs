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
    pub webhook: WebhookConfig,
    pub http: HttpConfig,
    pub monitor: MonitorConfig,
    pub reasoner: ReasonerConfig,
    pub feedback: FeedbackConfig,
    pub imports: ImportsConfig,
    pub repo: RepoConfig,
    pub socket: SocketConfig,
    pub logging: LoggingConfig,
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
    ///
    /// Accepts either canonical name `idle_ttl_secs` or the more descriptive
    /// alias `unload_timeout_secs` — both populate the same field.
    #[serde(alias = "unload_timeout_secs")]
    pub idle_ttl_secs: u64,
    /// How often the background evictor checks idle entries (seconds).
    pub evictor_interval_secs: u64,
    /// When true, every read tool checks the source file's mtime/sha and
    /// recompiles if it changed. Off by default for predictability.
    pub auto_refresh: bool,
    /// Number of bytes from the head of a source ontology file that are
    /// hashed (sha256) to form the cache fingerprint tie-breaker. Larger
    /// values reduce collision probability for very large dumps where many
    /// files share an identical leading region (e.g. `@prefix` headers in
    /// big N-Quads exports). Default 64 KiB.
    pub hash_prefix_bytes: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: "~/.open-ontologies/cache".into(),
            idle_ttl_secs: 0,
            evictor_interval_secs: 30,
            auto_refresh: false,
            hash_prefix_bytes: 64 * 1024,
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

// ─── New section configs ────────────────────────────────────────────────

/// `[webhook]` — outbound HTTP for governance / monitor alerts.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct WebhookConfig {
    /// HTTP timeout (seconds) for governance / monitor webhook deliveries.
    /// Override at runtime with `OPEN_ONTOLOGIES_WEBHOOK_REQUEST_TIMEOUT_SECS`.
    pub request_timeout_secs: u64,
}
impl Default for WebhookConfig {
    fn default() -> Self { Self { request_timeout_secs: 10 } }
}

/// `[http]` — Streamable HTTP transport (`serve-http`).
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct HttpConfig {
    /// Bind host. Default `127.0.0.1`. CLI `--host` and the
    /// `OPEN_ONTOLOGIES_HTTP_HOST` env var take precedence.
    pub host: String,
    /// Bind port. Default `8080`. CLI `--port` and the
    /// `OPEN_ONTOLOGIES_HTTP_PORT` env var take precedence.
    pub port: u16,
    /// Optional bearer token for authentication. Empty string ⇒ disabled.
    /// CLI `--token` and `OPEN_ONTOLOGIES_TOKEN` env var take precedence.
    pub token: String,
    /// Whether the rmcp `StreamableHttpServer` keeps per-session state.
    /// Default `true` (matches existing behaviour).
    pub stateful_mode: bool,
    /// Per-request HTTP timeout in seconds. `0` ⇒ no explicit cap (rmcp
    /// default). Override with `OPEN_ONTOLOGIES_HTTP_REQUEST_TIMEOUT_SECS`.
    pub request_timeout_secs: u64,
    /// HTTP keep-alive timeout in seconds. `0` ⇒ no explicit cap.
    pub keep_alive_secs: u64,
}
impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
            token: String::new(),
            stateful_mode: true,
            request_timeout_secs: 0,
            keep_alive_secs: 0,
        }
    }
}

/// `[monitor]` — continuous watcher loop.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct MonitorConfig {
    /// When true, start the background monitor loop on `serve` / `serve-http`.
    /// Equivalent to passing `--watch`. Default `false`.
    pub enabled: bool,
    /// Interval in seconds between monitor sweeps. Default `30`.
    /// Override with `OPEN_ONTOLOGIES_MONITOR_INTERVAL_SECS`.
    #[serde(alias = "watch_interval_secs")]
    pub interval_secs: u64,
}
impl Default for MonitorConfig {
    fn default() -> Self { Self { enabled: false, interval_secs: 30 } }
}

/// `[reasoner]` — RDFS / OWL-RL fixpoint and DL tableaux limits.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ReasonerConfig {
    /// Tableaux DL reasoner: max recursion depth before giving up. Default 100.
    pub tableaux_max_depth: usize,
    /// Tableaux DL reasoner: max nodes in a model before giving up.
    /// Default 10 000. Increase for very large ontologies; raising this past
    /// ~100 000 typically indicates the ontology is unsatisfiable in
    /// pathological ways and the reasoner should bail out anyway.
    pub tableaux_max_nodes: usize,
    /// RDFS / OWL-RL fixpoint guard. Maximum number of expansion iterations
    /// before the reasoner returns the partial closure. Default 64.
    pub max_iterations: usize,
}
impl Default for ReasonerConfig {
    fn default() -> Self {
        Self { tableaux_max_depth: 100, tableaux_max_nodes: 10_000, max_iterations: 64 }
    }
}

/// `[feedback]` — lint / enforce self-calibration thresholds.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct FeedbackConfig {
    /// Number of consecutive dismissals after which a lint/enforce issue
    /// is fully suppressed for that (tool, rule_id, entity) tuple. Default 3.
    pub suppress_threshold: i64,
    /// Number of consecutive dismissals after which a lint/enforce issue
    /// is downgraded one severity level. Must be `< suppress_threshold` to
    /// take effect. Default 2.
    pub downgrade_threshold: i64,
}
impl Default for FeedbackConfig {
    fn default() -> Self { Self { suppress_threshold: 3, downgrade_threshold: 2 } }
}

/// `[imports]` — `owl:imports` resolution policy.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ImportsConfig {
    /// Maximum transitive depth to follow when resolving owl:imports. The
    /// `onto_import` tool's `max_depth` parameter (when supplied) overrides
    /// this. Default 3.
    pub max_depth: usize,
    /// HTTP timeout in seconds for fetching each remote import. Set to `0`
    /// to disable the explicit per-call timeout and use reqwest's default
    /// (which itself has no timeout). Override with
    /// `OPEN_ONTOLOGIES_IMPORTS_REQUEST_TIMEOUT_SECS`.
    pub request_timeout_secs: u64,
    /// When false, remote (`http(s)://`) imports are refused — useful in
    /// air-gapped or sandboxed deployments. Default `true`.
    pub follow_remote: bool,
}
impl Default for ImportsConfig {
    fn default() -> Self {
        Self { max_depth: 3, request_timeout_secs: 30, follow_remote: true }
    }
}

/// `[repo]` — on-disk ontology repository tools.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RepoConfig {
    /// Default `limit` for `onto_repo_list` when the caller doesn't supply
    /// one. Default 1000.
    pub default_list_limit: usize,
}
impl Default for RepoConfig {
    fn default() -> Self { Self { default_list_limit: 1000 } }
}

/// `[socket]` — Unix domain socket adapter for Tardygrada fact grounding.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct SocketConfig {
    /// When true, `serve` / `serve-http` will additionally start the unix
    /// socket adapter. Currently the dedicated `serve-unix` subcommand reads
    /// these defaults. Default `false`.
    pub enabled: bool,
    /// Default socket path for `serve-unix`. CLI `--socket` overrides.
    pub path: Option<String>,
    /// Default ontology files to preload on `serve-unix` startup.
    /// CLI `--file` (repeatable) overrides.
    pub preload_files: Vec<String>,
}

/// `[logging]` — tracing subscriber configuration.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct LoggingConfig {
    /// Default log directives (e.g. `info`, `debug`,
    /// `open_ontologies=debug,reqwest=warn`). The `RUST_LOG` env var, when
    /// set, takes precedence over this value.
    pub level: String,
    /// Output format: `compact` (default), `pretty`, or `json`.
    pub format: String,
    /// Optional path to write logs to. When unset, logs go to stderr.
    pub file: Option<String>,
}
impl Default for LoggingConfig {
    fn default() -> Self {
        Self { level: "info".into(), format: "compact".into(), file: None }
    }
}

// ─── Env-override resolvers for the most operationally critical fields ──

/// Resolve the webhook request timeout. Precedence:
/// `OPEN_ONTOLOGIES_WEBHOOK_REQUEST_TIMEOUT_SECS` > config > default.
pub fn resolve_webhook_timeout_secs(cfg: &WebhookConfig) -> u64 {
    parse_env_u64("OPEN_ONTOLOGIES_WEBHOOK_REQUEST_TIMEOUT_SECS")
        .unwrap_or(cfg.request_timeout_secs)
}

/// Resolve the imports request timeout. Precedence:
/// `OPEN_ONTOLOGIES_IMPORTS_REQUEST_TIMEOUT_SECS` > config > default.
pub fn resolve_imports_timeout_secs(cfg: &ImportsConfig) -> u64 {
    parse_env_u64("OPEN_ONTOLOGIES_IMPORTS_REQUEST_TIMEOUT_SECS")
        .unwrap_or(cfg.request_timeout_secs)
}

/// Resolve the monitor sweep interval. Precedence:
/// `OPEN_ONTOLOGIES_MONITOR_INTERVAL_SECS` > config > default.
pub fn resolve_monitor_interval_secs(cfg: &MonitorConfig) -> u64 {
    parse_env_u64("OPEN_ONTOLOGIES_MONITOR_INTERVAL_SECS")
        .unwrap_or(cfg.interval_secs)
}

/// Resolve the HTTP bind host. Precedence:
/// `OPEN_ONTOLOGIES_HTTP_HOST` > config > default.
pub fn resolve_http_host(cfg: &HttpConfig) -> String {
    std::env::var("OPEN_ONTOLOGIES_HTTP_HOST")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| cfg.host.clone())
}

/// Resolve the HTTP bind port. Precedence:
/// `OPEN_ONTOLOGIES_HTTP_PORT` > config > default.
pub fn resolve_http_port(cfg: &HttpConfig) -> u16 {
    std::env::var("OPEN_ONTOLOGIES_HTTP_PORT")
        .ok()
        .and_then(|v| v.trim().parse::<u16>().ok())
        .unwrap_or(cfg.port)
}

/// Resolve the HTTP bearer token. Precedence:
/// `OPEN_ONTOLOGIES_TOKEN` > config (if non-empty) > `None`.
pub fn resolve_http_token(cfg: &HttpConfig) -> Option<String> {
    std::env::var("OPEN_ONTOLOGIES_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| Some(cfg.token.clone()).filter(|v| !v.trim().is_empty()))
}

/// Resolve the logging directives. Precedence: `RUST_LOG` > config > default.
pub fn resolve_logging_level(cfg: &LoggingConfig) -> String {
    std::env::var("RUST_LOG")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| cfg.level.clone())
}

fn parse_env_u64(var: &str) -> Option<u64> {
    std::env::var(var)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
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

    #[test]
    fn unload_timeout_alias_for_idle_ttl() {
        let toml_src = r#"
            [cache]
            unload_timeout_secs = 120
            hash_prefix_bytes = 131072
        "#;
        let cfg: Config = toml::from_str(toml_src).expect("parse");
        assert_eq!(cfg.cache.idle_ttl_secs, 120);
        assert_eq!(cfg.cache.hash_prefix_bytes, 131072);
    }

    #[test]
    fn new_sections_parse_with_defaults() {
        let toml_src = r#"
            [webhook]
            request_timeout_secs = 5

            [http]
            host = "0.0.0.0"
            port = 9000
            stateful_mode = false

            [monitor]
            enabled = true
            interval_secs = 15

            [reasoner]
            tableaux_max_depth = 200
            tableaux_max_nodes = 50000
            max_iterations = 128

            [feedback]
            suppress_threshold = 5
            downgrade_threshold = 3

            [imports]
            max_depth = 5
            request_timeout_secs = 60
            follow_remote = false

            [repo]
            default_list_limit = 250

            [socket]
            enabled = true
            path = "/tmp/foo.sock"
            preload_files = ["a.ttl", "b.ttl"]

            [logging]
            level = "debug"
            format = "json"
        "#;
        let cfg: Config = toml::from_str(toml_src).expect("parse");
        assert_eq!(cfg.webhook.request_timeout_secs, 5);
        assert_eq!(cfg.http.host, "0.0.0.0");
        assert_eq!(cfg.http.port, 9000);
        assert!(!cfg.http.stateful_mode);
        assert!(cfg.monitor.enabled);
        assert_eq!(cfg.monitor.interval_secs, 15);
        assert_eq!(cfg.reasoner.tableaux_max_depth, 200);
        assert_eq!(cfg.reasoner.tableaux_max_nodes, 50_000);
        assert_eq!(cfg.reasoner.max_iterations, 128);
        assert_eq!(cfg.feedback.suppress_threshold, 5);
        assert_eq!(cfg.feedback.downgrade_threshold, 3);
        assert_eq!(cfg.imports.max_depth, 5);
        assert_eq!(cfg.imports.request_timeout_secs, 60);
        assert!(!cfg.imports.follow_remote);
        assert_eq!(cfg.repo.default_list_limit, 250);
        assert!(cfg.socket.enabled);
        assert_eq!(cfg.socket.path.as_deref(), Some("/tmp/foo.sock"));
        assert_eq!(cfg.socket.preload_files, vec!["a.ttl", "b.ttl"]);
        assert_eq!(cfg.logging.level, "debug");
        assert_eq!(cfg.logging.format, "json");
    }

    #[test]
    fn new_sections_default_when_absent() {
        let cfg: Config = toml::from_str("").expect("parse empty");
        assert_eq!(cfg.webhook.request_timeout_secs, 10);
        assert_eq!(cfg.http.host, "127.0.0.1");
        assert_eq!(cfg.http.port, 8080);
        assert!(cfg.http.stateful_mode);
        assert!(!cfg.monitor.enabled);
        assert_eq!(cfg.monitor.interval_secs, 30);
        assert_eq!(cfg.reasoner.tableaux_max_depth, 100);
        assert_eq!(cfg.reasoner.tableaux_max_nodes, 10_000);
        assert_eq!(cfg.reasoner.max_iterations, 64);
        assert_eq!(cfg.feedback.suppress_threshold, 3);
        assert_eq!(cfg.feedback.downgrade_threshold, 2);
        assert_eq!(cfg.imports.max_depth, 3);
        assert_eq!(cfg.imports.request_timeout_secs, 30);
        assert!(cfg.imports.follow_remote);
        assert_eq!(cfg.repo.default_list_limit, 1000);
        assert!(!cfg.socket.enabled);
        assert!(cfg.socket.preload_files.is_empty());
        assert_eq!(cfg.logging.level, "info");
        assert_eq!(cfg.logging.format, "compact");
        assert_eq!(cfg.cache.hash_prefix_bytes, 64 * 1024);
    }
}
