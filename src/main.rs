use clap::{Parser, Subcommand};
use rmcp::ServiceExt;
use std::sync::Arc;

use open_ontologies::config::{Config, expand_tilde};
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;

const DEFAULT_CONFIG: &str = r#"[general]
data_dir = "~/.open-ontologies"

# Optional on-disk ontology repository directories. When set, the
# `onto_repo_list` and `onto_repo_load` MCP tools enumerate and load
# RDF/OWL files (.ttl, .nt, .rdf, .owl, .nq, .trig, .jsonld) from these
# directories. Container-friendly: mount a host folder of TTL files.
# Override at runtime with the `OPEN_ONTOLOGIES_ONTOLOGY_DIRS` env var
# (':' separated on Unix, ';' on Windows; either accepted on both).
# ontology_dirs = ["./ttl_data"]

# [cache]
# Compile-cache: parsed ontologies are written to N-Triples files for fast reload.
# enabled = true
# dir = "~/.open-ontologies/cache"
# # Idle timeout in seconds before the active ontology is unloaded from memory.
# # Set to 0 to disable eviction. The on-disk cache file is always preserved
# # across evictions; the next query reloads it transparently.
# # `idle_ttl_secs` is the canonical name; `unload_timeout_secs` is an alias.
# idle_ttl_secs = 0
# # unload_timeout_secs = 0
# # How often the background evictor runs (seconds).
# evictor_interval_secs = 30
# # When true, every read tool checks the source file's mtime/sha and
# # recompiles if it changed.
# auto_refresh = false
# # Bytes from the head of each ontology file that are sha256-hashed for the
# # cache fingerprint tie-breaker. Increase for very large dumps.
# hash_prefix_bytes = 65536

# [storage]
# Backend for the main triple store.
#   mode = "memory"      # in-memory (default; lost on restart)
#   mode = "persistent"  # RocksDB at <data_dir>/triplestore; survives restarts
# Override at runtime with OPEN_ONTOLOGIES_STORAGE_MODE or `--storage-mode`.
# Note: only one server process can hold the persistent store open at a time.
# mode = "memory"

# [tools]
# Restrict which MCP tools are exposed by this server.
# mode = "all" | "allow" | "deny"
# list = ["onto_status", "onto_query", "onto_load"]
# # Groups: read_only, mutating, governance, remote, embeddings.
# groups = ["read_only"]
# mode = "all"

# [embeddings]
# Provider selects how text embeddings are computed. Override at runtime
# with OPEN_ONTOLOGIES_EMBEDDINGS_PROVIDER.
# provider = "local"   # or "openai" for any OpenAI-compatible API
#
# ── Local provider (provider = "local", default) ────────────────────────
# Paths to a local ONNX model and tokenizer (loaded at runtime).
# Default is the multilingual MiniLM model (see model_url below); labels in
# different natural languages embed into a shared space for cross-lingual
# alignment.
# model_path = "~/.open-ontologies/models/paraphrase-multilingual-MiniLM-L12-v2.onnx"
# tokenizer_path = "~/.open-ontologies/models/tokenizer.json"
#
# URLs used by `open-ontologies init` to download the model.
# Override these to use a different sentence-transformer model. To go back to
# the smaller English-only model set these to BAAI/bge-small-en-v1.5.
# The model must be exported to ONNX and use a Hugging Face tokenizer.json.
# model_url = "https://huggingface.co/Xenova/paraphrase-multilingual-MiniLM-L12-v2/resolve/main/onnx/model.onnx"
# tokenizer_url = "https://huggingface.co/Xenova/paraphrase-multilingual-MiniLM-L12-v2/resolve/main/tokenizer.json"
# model_name = "paraphrase-multilingual-MiniLM-L12-v2.onnx"
#
# [language]
# Preferred natural-language tags for the labels onto_align consults. Untagged
# literals are always kept. Empty (default) = keep ALL languages (multilingual
# matching, paired with the multilingual embedding model above). Override with
# OPEN_ONTOLOGIES_LANGUAGES=en,fr
# preferred = []
#
# ── OpenAI-compatible provider (provider = "openai") ────────────────────
# Works with the official OpenAI API, Azure OpenAI, Ollama, vLLM, LocalAI,
# LM Studio, Together, Mistral, and any other gateway that speaks the
# `POST {api_base}/embeddings` protocol. Each field can be overridden via
# environment variables:
#   OPEN_ONTOLOGIES_EMBEDDINGS_API_BASE
#   OPEN_ONTOLOGIES_EMBEDDINGS_API_KEY  (or OPENAI_API_KEY)
#   OPEN_ONTOLOGIES_EMBEDDINGS_MODEL
# api_base = "https://api.openai.com/v1"
# api_key = "sk-..."                # optional — env vars take precedence
# model = "text-embedding-3-small"  # any model your gateway serves
# dimensions = 1536                 # optional — only sent when set
# request_timeout_secs = 30

# [webhook]
# HTTP timeout (seconds) for governance / monitor webhook deliveries.
# Override at runtime with OPEN_ONTOLOGIES_WEBHOOK_REQUEST_TIMEOUT_SECS.
# request_timeout_secs = 10

# [http]
# Streamable HTTP transport (`open-ontologies serve-http`). CLI flags
# (--host/--port/--token) and env vars (OPEN_ONTOLOGIES_HTTP_HOST,
# OPEN_ONTOLOGIES_HTTP_PORT, OPEN_ONTOLOGIES_TOKEN) take precedence.
# host = "127.0.0.1"
# port = 8080
# token = ""             # empty disables auth
# stateful_mode = true   # rmcp StreamableHttpServer per-session state
# request_timeout_secs = 0   # 0 = rmcp default
# keep_alive_secs = 0        # 0 = rmcp default

# [monitor]
# Continuous watcher loop. CLI `--watch` / `--watch-interval` override.
# enabled = false
# interval_secs = 30

# [reasoner]
# Safety limits for the OWL-DL tableaux reasoner and RDFS / OWL-RL fixpoint.
# tableaux_max_depth = 100
# tableaux_max_nodes = 10000
# max_iterations = 64

# [feedback]
# Lint / enforce self-calibration thresholds (number of dismissals before
# downgrading then suppressing a (tool, rule_id, entity) triple).
# suppress_threshold = 3
# downgrade_threshold = 2

# [imports]
# `owl:imports` resolution policy used by `onto_import`.
# max_depth = 3
# request_timeout_secs = 30
# follow_remote = true   # set false in air-gapped / sandboxed deployments

# [repo]
# Defaults for the `onto_repo_list` tool.
# default_list_limit = 1000

# [socket]
# Defaults for the `serve-unix` subcommand. CLI `--socket` / `--file`
# override these.
# enabled = false
# path = "/tmp/tardygrada-ontology-complete.sock"
# preload_files = []

# [logging]
# Tracing subscriber configuration. RUST_LOG, when set, takes precedence.
# level = "info"
# format = "compact"     # "compact" | "pretty" | "json"
# # file = "/var/log/open-ontologies.log"
"#;

/// The default for `--data-dir`, named so the server arms can tell an explicit
/// `--data-dir` from an unset one and give the flag precedence over config.
const DEFAULT_DATA_DIR: &str = "~/.open-ontologies";

#[derive(Parser)]
#[command(
    name = "open-ontologies",
    about = "Terraform for Knowledge Graphs — AI-native ontology engine"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output results as JSON. This is the default; the flag is accepted so
    /// that scripts can state the format they depend on rather than assume it.
    #[arg(long, global = true)]
    json: bool,

    /// Render results as human-readable text instead of JSON. Overridden by
    /// --json or --pretty when both are given.
    #[arg(long, global = true)]
    human: bool,

    /// Pretty-print JSON output (implies --json)
    #[arg(long, global = true)]
    pretty: bool,

    /// Data directory (default: ~/.open-ontologies)
    #[arg(long, global = true, default_value = DEFAULT_DATA_DIR)]
    data_dir: String,

    /// Force local execution — ignore a running daemon even if one is detected
    #[arg(long, global = true)]
    no_connect: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize data directory, DB, and default config
    Init {
        #[arg(long, default_value = "~/.open-ontologies")]
        data_dir: String,
        /// Custom ONNX model URL (default: multilingual MiniLM-L12-v2 from Hugging Face)
        #[arg(long)]
        model_url: Option<String>,
        /// Custom tokenizer URL (default: multilingual MiniLM-L12-v2 tokenizer from Hugging Face)
        #[arg(long)]
        tokenizer_url: Option<String>,
        /// Filename for the downloaded ONNX model (default: paraphrase-multilingual-MiniLM-L12-v2.onnx)
        #[arg(long)]
        model_name: Option<String>,
    },
    /// Start the MCP server (stdio transport)
    Serve {
        #[arg(long, default_value = "~/.open-ontologies/config.toml")]
        config: String,
        /// Optional governance webhook URL (fires on every lineage event)
        #[arg(long, env = "GOVERNANCE_WEBHOOK")]
        governance_webhook: Option<String>,
        /// Enable continuous background monitoring (runs watchers on an interval).
        /// When omitted, falls back to `[monitor] enabled` from the config (default false).
        #[arg(long)]
        watch: bool,
        /// Interval in seconds between monitor sweeps. When omitted, falls back to
        /// `OPEN_ONTOLOGIES_MONITOR_INTERVAL_SECS` env var, then `[monitor] interval_secs`
        /// from the config (default 30).
        #[arg(long)]
        watch_interval: Option<u64>,
        /// Comma-separated list of tools (or `@group`) to allow. Mutually exclusive with --tools-deny.
        #[arg(long)]
        tools_allow: Option<String>,
        /// Comma-separated list of tools (or `@group`) to deny.
        #[arg(long)]
        tools_deny: Option<String>,
        /// Idle TTL in seconds before the active ontology is unloaded from memory. 0 disables eviction.
        #[arg(long)]
        idle_ttl_secs: Option<u64>,
        /// When set, every read tool checks the source file for changes and recompiles.
        #[arg(long)]
        auto_refresh: bool,
        /// Storage backend for the main triple store: `memory` (default,
        /// in-memory) or `persistent` (RocksDB at `<data_dir>/triplestore`).
        /// CLI > `OPEN_ONTOLOGIES_STORAGE_MODE` env > `[storage] mode`.
        #[arg(long)]
        storage_mode: Option<String>,
    },
    /// Start the MCP server (Streamable HTTP transport)
    ServeHttp {
        #[arg(long, default_value = "~/.open-ontologies/config.toml")]
        config: String,
        /// Host to bind to. CLI > `OPEN_ONTOLOGIES_HTTP_HOST` env > `[http] host`
        /// in config > `127.0.0.1`.
        #[arg(long)]
        host: Option<String>,
        /// Port to bind to. CLI > `OPEN_ONTOLOGIES_HTTP_PORT` env > `[http] port` >
        /// `8080`.
        #[arg(long)]
        port: Option<u16>,
        /// Optional bearer token for authentication. CLI > `OPEN_ONTOLOGIES_TOKEN`
        /// env > `[http] token` in config.
        #[arg(long, env = "OPEN_ONTOLOGIES_TOKEN")]
        token: Option<String>,
        /// Optional governance webhook URL (fires on every lineage event)
        #[arg(long, env = "GOVERNANCE_WEBHOOK")]
        governance_webhook: Option<String>,
        /// Enable continuous background monitoring (runs watchers on an interval).
        /// When omitted, falls back to `[monitor] enabled` from the config (default false).
        #[arg(long)]
        watch: bool,
        /// Interval in seconds between monitor sweeps. When omitted, falls back to
        /// `[monitor] interval_secs` from the config (default 30).
        #[arg(long)]
        watch_interval: Option<u64>,
        /// Comma-separated list of tools (or `@group`) to allow.
        #[arg(long)]
        tools_allow: Option<String>,
        /// Comma-separated list of tools (or `@group`) to deny.
        #[arg(long)]
        tools_deny: Option<String>,
        /// Idle TTL in seconds before the active ontology is unloaded from memory.
        #[arg(long)]
        idle_ttl_secs: Option<u64>,
        /// When set, every read tool checks the source file for changes and recompiles.
        #[arg(long)]
        auto_refresh: bool,
        /// Storage backend for the main triple store: `memory` (default,
        /// in-memory) or `persistent` (RocksDB at `<data_dir>/triplestore`).
        /// CLI > `OPEN_ONTOLOGIES_STORAGE_MODE` env > `[storage] mode`.
        #[arg(long)]
        storage_mode: Option<String>,
    },

    /// Start unix socket server for Tardygrada fact grounding
    #[cfg(unix)]
    ServeUnix {
        #[arg(long, default_value = "~/.open-ontologies/config.toml")]
        config: String,
        /// Path to the unix socket. CLI > `[socket] path` in config >
        /// `/tmp/tardygrada-ontology-complete.sock`.
        #[arg(long)]
        socket: Option<String>,
        /// Ontology files to load on startup. When omitted, falls back to
        /// `[socket] preload_files` from the config.
        #[arg(long = "file", num_args = 1..)]
        files: Vec<String>,
    },
    /// Unix socket transport is not available on Windows.
    #[cfg(windows)]
    ServeUnix {
        #[arg(long, default_value = "~/.open-ontologies/config.toml")]
        config: String,
        #[arg(long)]
        socket: Option<String>,
        #[arg(long = "file", num_args = 1..)]
        files: Vec<String>,
    },

    // ─── Batch ────────────────────────────────────────────────────
    /// Run a batch of commands from a file or stdin (one per line, or JSON array)
    Batch {
        /// Path to batch file (use - for stdin)
        #[arg(default_value = "-")]
        input: String,
        /// Stop on first error
        #[arg(long)]
        bail: bool,
    },

    // ─── Daemon ───────────────────────────────────────────────────
    /// Manage background daemon (persistent in-memory store via serve-http)
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    // ─── Core ontology ────────────────────────────────────────────
    /// Validate RDF/OWL syntax (file or stdin with -)
    Validate { input: String },
    /// Load RDF file into in-memory graph store
    Load { path: String },
    /// Save ontology to file
    Save {
        path: String,
        #[arg(long, default_value = "turtle")]
        format: String,
    },
    /// Clear in-memory store
    Clear,
    /// Show triple count, classes, properties, individuals
    Stats,
    /// Run SPARQL query (or stdin with -)
    Query { query: String },
    /// Compare two ontology files
    Diff { old_path: String, new_path: String },
    /// Lint: check for missing labels, domains, ranges
    Lint { input: String },
    /// Convert between RDF formats
    Convert {
        path: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        output: Option<String>,
    },
    /// Server health and loaded triple count
    Status,

    // ─── Remote ───────────────────────────────────────────────────
    /// Fetch ontology from URL or SPARQL endpoint
    Pull {
        url: String,
        #[arg(long)]
        sparql: bool,
        #[arg(long)]
        query: Option<String>,
    },
    /// Push ontology to SPARQL endpoint
    Push {
        endpoint: String,
        #[arg(long)]
        graph: Option<String>,
    },
    /// Browse and install standard ontologies from marketplace
    Marketplace {
        /// Action: "list" or "install"
        action: String,
        /// Ontology ID (for install)
        #[arg(long)]
        id: Option<String>,
        /// Filter by domain (for list)
        #[arg(long)]
        domain: Option<String>,
    },
    /// Resolve and load owl:imports chain
    ImportOwl {
        #[arg(long, default_value = "10")]
        max_depth: usize,
    },

    // ─── Versioning ───────────────────────────────────────────────
    /// Save a named snapshot
    Version { label: String },
    /// List saved version snapshots
    History,
    /// Restore a previous version
    Rollback { label: String },

    // ─── Data pipeline ────────────────────────────────────────────
    /// Generate mapping config from data file + ontology
    Map {
        data_path: String,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        save: Option<String>,
    },
    /// Ingest structured data into RDF
    Ingest {
        path: String,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        mapping: Option<String>,
        #[arg(long)]
        base_iri: Option<String>,
    },
    /// Validate against SHACL shapes
    Shacl { shapes: String },
    /// Closed-world vocab check: flag data terms not declared in the loaded ontology
    VocabCheck { data: String },
    /// Run inference (rdfs, owl-rl, owl-rl-ext, owl-dl)
    Reason {
        #[arg(long, default_value = "rdfs")]
        profile: String,
    },
    /// Full pipeline: ingest → SHACL → reason
    Extend {
        data_path: String,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        mapping: Option<String>,
        #[arg(long)]
        shapes: Option<String>,
        #[arg(long)]
        profile: Option<String>,
    },

    // ─── Lifecycle ────────────────────────────────────────────────
    /// Plan changes: diff current vs proposed Turtle
    Plan { file: String },
    /// Apply planned changes (safe or migrate)
    Apply {
        #[arg(default_value = "safe")]
        mode: String,
        /// Plan to apply, as printed by `plan`. Defaults to the most recent plan.
        #[arg(long)]
        plan_id: Option<String>,
    },
    /// Lock IRIs to prevent removal
    Lock {
        iris: Vec<String>,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Detect drift between two ontology versions
    Drift { file_a: String, file_b: String },
    /// Run design pattern enforcement
    Enforce {
        #[arg(default_value = "generic")]
        pack: String,
    },
    /// Run active SPARQL watchers
    Monitor,
    /// Clear monitor block state
    MonitorClear,
    /// View lineage trail
    Lineage {
        #[arg(long)]
        session: Option<String>,
    },

    // ─── Alignment ────────────────────────────────────────────────
    /// Detect alignment candidates between two ontologies
    Align {
        /// Source ontology file
        source: String,
        /// Target ontology file (if omitted, aligns against loaded store)
        target: Option<String>,
        /// Minimum confidence threshold (default 0.85)
        #[arg(long, default_value = "0.85")]
        min_confidence: f64,
        /// Dry run — show candidates without inserting triples
        #[arg(long)]
        dry_run: bool,
    },
    /// Accept or reject an alignment candidate
    AlignFeedback {
        /// Source class IRI
        #[arg(long)]
        source: String,
        /// Target class IRI
        #[arg(long)]
        target: String,
        /// Accept the candidate
        #[arg(long, conflicts_with = "reject")]
        accept: bool,
        /// Reject the candidate
        #[arg(long, conflicts_with = "accept")]
        reject: bool,
    },

    // ─── Feedback ────────────────────────────────────────────────
    /// Accept or dismiss a lint issue
    LintFeedback {
        /// Lint rule ID (e.g. "missing_label", "missing_comment")
        #[arg(long)]
        rule_id: String,
        /// Entity IRI that triggered the issue
        #[arg(long)]
        entity: String,
        /// Accept the issue as valid
        #[arg(long, default_value_t = false)]
        accept: bool,
        /// Dismiss/ignore the issue
        #[arg(long, default_value_t = false)]
        dismiss: bool,
    },
    /// Accept or dismiss an enforce violation
    EnforceFeedback {
        /// Enforce rule ID (e.g. "orphan_class", "missing_domain")
        #[arg(long)]
        rule_id: String,
        /// Entity IRI that triggered the violation
        #[arg(long)]
        entity: String,
        /// Accept the violation as valid
        #[arg(long, default_value_t = false)]
        accept: bool,
        /// Dismiss/override the violation
        #[arg(long, default_value_t = false)]
        dismiss: bool,
    },

    // ─── Clinical ─────────────────────────────────────────────────
    /// Look up clinical terminology crosswalk
    Crosswalk {
        code: String,
        #[arg(long)]
        system: String,
    },
    /// Add skos:exactMatch triple for clinical code
    Enrich {
        class_iri: String,
        code: String,
        #[arg(long)]
        system: String,
    },
    /// Validate class labels against clinical terminology
    ValidateClinical,

    // ─── Schema import ────────────────────────────────────────────
    /// Import database schema as OWL ontology (Postgres or DuckDB)
    ImportSchema {
        /// Connection string. Supported:
        ///   postgres://user:pass@host/db (requires --features postgres)
        ///   duckdb:///path/to/file.duckdb or *.duckdb file path (requires --features duckdb)
        ///   :memory: for an in-memory DuckDB database
        connection: String,
        #[arg(long, default_value = "http://example.org/db/")]
        base_iri: String,
    },
    /// Run a SQL query against a relational backbone (Postgres or DuckDB)
    /// and ingest the result rows into the triple store as RDF.
    SqlIngest {
        /// Connection string (see import-schema for forms)
        connection: String,
        /// SQL SELECT to execute. Use `-` to read from stdin.
        sql: String,
        /// Path to mapping JSON, or inline JSON when --inline-mapping is set.
        /// If omitted, an auto-mapping is generated from the column names.
        #[arg(long)]
        mapping: Option<String>,
        /// Treat the value of --mapping as inline JSON instead of a file path.
        #[arg(long)]
        inline_mapping: bool,
        /// Base IRI for generated instances
        #[arg(long, default_value = "http://example.org/data/")]
        base_iri: String,
    },
}

impl Commands {
    /// Serialize a proxy-able command into the structured form of `/api/batch`:
    /// `{"command": name, "args": [..]}`, where every argument is its own array
    /// element and reaches the daemon exactly as clap parsed it.
    ///
    /// The previous shape of this was a shell-ish line that the daemon
    /// re-tokenized, and the round trip lost things. `parse_lines` splits on
    /// newlines before it looks at quotes, so a multi-line SPARQL query — the
    /// normal kind — arrived torn across lines and failed as an unterminated
    /// quote, while the identical command run locally succeeded. The
    /// double-quote fallback in the quoting helper did not escape backslashes,
    /// so an argument holding both quote styles came out mangled. Neither can
    /// happen to an array element.
    ///
    /// Returns None for commands that must always run locally (server modes,
    /// daemon management, static file operations), and for any invocation
    /// carrying a flag the batch handler cannot honour, which is safer than
    /// proxying it and dropping the flag in silence.
    fn to_batch_command(&self) -> Option<serde_json::Value> {
        fn cmd(name: &str, args: Vec<String>) -> Option<serde_json::Value> {
            Some(serde_json::json!({"command": name, "args": args}))
        }
        match self {
            Commands::Load { path } => cmd("load", vec![absolutize(path)]),
            Commands::Save { path, format } => cmd(
                "save",
                vec![absolutize(path), "--format".into(), format.clone()],
            ),
            Commands::Clear => cmd("clear", vec![]),
            Commands::Stats => cmd("stats", vec![]),
            Commands::Query { query } => cmd("query", vec![query.clone()]),
            Commands::Lint { input } => cmd("lint", vec![absolutize(input)]),
            Commands::Reason { profile } => {
                cmd("reason", vec!["--profile".into(), profile.clone()])
            }
            Commands::Shacl { shapes } => cmd("shacl", vec![absolutize(shapes)]),
            Commands::Status => cmd("status", vec![]),
            Commands::Pull { url, sparql, query } => {
                let mut a = vec![url.clone()];
                if *sparql {
                    a.push("--sparql".into());
                }
                if let Some(q) = query {
                    a.push("--query".into());
                    a.push(q.clone());
                }
                cmd("pull", a)
            }
            Commands::Push { endpoint, graph } => {
                let mut a = vec![endpoint.clone()];
                if let Some(g) = graph {
                    a.push("--graph".into());
                    a.push(g.clone());
                }
                cmd("push", a)
            }
            Commands::Version { label } => cmd("version", vec![label.clone()]),
            Commands::History => cmd("history", vec![]),
            Commands::Rollback { label } => cmd("rollback", vec![label.clone()]),
            // `--format` has no arm in the batch ingester, so an invocation
            // carrying it runs locally rather than being proxied without it.
            Commands::Ingest { path, format, mapping, base_iri } => {
                if format.is_some() {
                    return None;
                }
                let mut a = vec![absolutize(path)];
                if let Some(m) = mapping {
                    a.push("--mapping".into());
                    a.push(absolutize(m));
                }
                if let Some(b) = base_iri {
                    a.push("--base-iri".into());
                    a.push(b.clone());
                }
                cmd("ingest", a)
            }
            Commands::Plan { file } => cmd("plan", vec![absolutize(file)]),
            Commands::Apply { mode, plan_id } => {
                let mut a = vec![mode.clone()];
                if let Some(p) = plan_id {
                    a.push("--plan-id".into());
                    a.push(p.clone());
                }
                cmd("apply", a)
            }
            Commands::Enforce { pack } => cmd("enforce", vec![pack.clone()]),
            Commands::Monitor => cmd("monitor", vec![]),
            Commands::MonitorClear => cmd("monitor-clear", vec![]),
            Commands::Drift { file_a, file_b } => {
                cmd("drift", vec![absolutize(file_a), absolutize(file_b)])
            }
            Commands::Lock { iris, reason } => {
                let mut a: Vec<String> = iris.clone();
                if let Some(r) = reason {
                    a.push("--reason".into());
                    a.push(r.clone());
                }
                cmd("lock", a)
            }
            Commands::Marketplace { action, id, domain } => {
                let mut a = vec![action.clone()];
                if let Some(i) = id {
                    a.push("--id".into());
                    a.push(i.clone());
                }
                if let Some(d) = domain {
                    a.push("--domain".into());
                    a.push(d.clone());
                }
                cmd("marketplace", a)
            }
            // Never proxied: server modes, daemon, init, validate, diff, convert,
            // import-owl, map, extend, align, feedback, lineage, clinical, schema ops.
            _ => None,
        }
    }
}

/// Make a path argument absolute against the caller's working directory before
/// it is proxied.
///
/// The daemon inherits whatever directory `daemon start` ran in and never learns
/// the caller's, so a relative path forwarded verbatim resolved against the wrong
/// place: `cd /data && onto load ./x.ttl` either failed or loaded a different
/// file, and `save ./out.ttl` wrote somewhere the caller was not looking. Not
/// `canonicalize`, which requires the path to exist and so cannot be used for an
/// output path that is about to be created.
/// Hand `input` to a running daemon, or report that there is none to hand it to.
///
/// `Some(exit_code)` means the daemon ran it and the caller should exit with
/// that code; `None` means run locally. Both proxy sites went through their own
/// copy of this and the copies had already drifted — one hardcoded `bail` to
/// false — so there is one copy now, and one place where the decision to proxy
/// is made.
async fn proxy_if_daemon(ctx: &ProxyCtx, input: &str, bail: bool) -> anyhow::Result<Option<i32>> {
    if ctx.no_connect {
        return Ok(None);
    }
    let Some(info) = open_ontologies::daemon::read_daemon_info(&ctx.data_dir) else {
        return Ok(None);
    };
    if !open_ontologies::daemon::is_daemon_alive(info.pid) {
        // Stale daemon.json — clean up silently and fall through to local.
        open_ontologies::daemon::remove_daemon_info(&ctx.data_dir);
        return Ok(None);
    }
    open_ontologies::connect::proxy_batch(&info, input, bail, json_mode(), ctx.pretty).await
}

/// The parts of `Cli` the proxy needs, taken before `cli.command` is matched by
/// value and its fields move out.
struct ProxyCtx {
    no_connect: bool,
    data_dir: String,
    pretty: bool,
}

fn absolutize(path: &str) -> String {
    let expanded = expand_tilde(path);
    let p = std::path::Path::new(&expanded);
    if p.is_absolute() {
        return expanded;
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(p).to_string_lossy().into_owned(),
        Err(_) => expanded,
    }
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the daemon (serve-http in background, persistent in-memory store)
    Start {
        /// Host to bind to (default: 127.0.0.1)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Port to bind to (default: 8080)
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Optional bearer token for authentication
        #[arg(long, env = "OPEN_ONTOLOGIES_TOKEN")]
        token: Option<String>,
    },
    /// Stop the running daemon
    Stop,
    /// Show daemon status (pid, url, alive/dead)
    Status,
}

fn setup(data_dir: &str) -> anyhow::Result<(StateDb, Arc<GraphStore>)> {
    let data_dir = expand_tilde(data_dir);
    let data_path = std::path::Path::new(&data_dir);
    std::fs::create_dir_all(data_path)?;
    let db_path = data_path.join("open-ontologies.db");
    let db = StateDb::open(&db_path)?;
    // One-shot CLI subcommands respect the same [storage] setting as the
    // server modes (loaded from `<data_dir>/config.toml` if present). This is
    // what makes `open-ontologies load foo.ttl` + `open-ontologies query ...`
    // share state when persistence is enabled.
    let cfg_path = data_path.join("config.toml");
    let storage_cfg = open_ontologies::config::Config::load(&cfg_path)
        .map(|c| c.storage)
        .unwrap_or_default();
    let graph = build_main_graph(&storage_cfg, data_path)?;
    Ok((db, graph))
}

/// False only when --human is passed without --json or --pretty; checked by
/// output_json/output_result.
static JSON_MODE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// The resolved output format. Defaults to JSON on the paths that read it
/// before `async_main` has set it, so no caller can fall into human-readable
/// output by accident.
fn json_mode() -> bool {
    JSON_MODE.get().copied().unwrap_or(true)
}
/// Whether the resolved storage backend keeps the graph in RAM only.
///
/// One-shot CLI subcommands each build their own store, so in this mode nothing
/// a command loads is visible to the next one.
fn is_memory_storage(data_dir: &str) -> bool {
    use open_ontologies::config::{resolve_storage_mode, StorageMode};
    let data_path = std::path::PathBuf::from(expand_tilde(data_dir));
    let storage_cfg = open_ontologies::config::Config::load(&data_path.join("config.toml"))
        .map(|c| c.storage)
        .unwrap_or_default();
    matches!(resolve_storage_mode(&storage_cfg), StorageMode::Memory)
}

/// Build the singleton main graph using the configured storage backend.
///
/// In-memory: returns an empty `GraphStore`.
/// Persistent: opens (or creates) a RocksDB-backed store at
/// `<data_dir>/triplestore`.
fn build_main_graph(
    cfg: &open_ontologies::config::StorageConfig,
    data_dir: &std::path::Path,
) -> anyhow::Result<Arc<GraphStore>> {
    use open_ontologies::config::{resolve_storage_mode, StorageMode};
    match resolve_storage_mode(cfg) {
        StorageMode::Memory => Ok(Arc::new(GraphStore::new())),
        StorageMode::Persistent => {
            let path = data_dir.join("triplestore");
            let store = GraphStore::open_persistent(&path)?;
            tracing::info!(
                "opened persistent triple store at {} ({} triples)",
                path.display(),
                store.triple_count()
            );
            Ok(Arc::new(store))
        }
    }
}

fn output_json(value: &serde_json::Value, pretty: bool) {
    if json_mode() || pretty {
        if pretty {
            println!("{}", serde_json::to_string_pretty(value).unwrap());
        } else {
            println!("{}", value);
        }
    } else {
        println!("{}", open_ontologies::output::render_human(value));
    }
}

/// Print a JSON string result, with optional pretty-printing.
/// Handles the common pattern of domain functions returning String results.
fn output_result(result: &str, pretty: bool) {
    if json_mode() || pretty {
        if pretty {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(result) {
                println!("{}", serde_json::to_string_pretty(&v).unwrap());
            } else {
                println!("{}", result);
            }
        } else {
            println!("{}", result);
        }
    } else {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(result) {
            println!("{}", open_ontologies::output::render_human(&v));
        } else {
            println!("{}", result);
        }
    }
}

/// Compose the effective cache configuration from `[cache]` in config + CLI overrides.
fn build_cache_config(
    cfg: &Config,
    idle_ttl_secs: Option<u64>,
    auto_refresh: bool,
) -> open_ontologies::config::CacheConfig {
    let mut cc = cfg.cache.clone();
    if let Some(ttl) = idle_ttl_secs {
        cc.idle_ttl_secs = ttl;
    }
    if auto_refresh {
        cc.auto_refresh = true;
    }
    cc
}

/// Initialise the tracing subscriber from `[logging]` config. `RUST_LOG`
/// (when set) takes precedence over `level`. Idempotent: re-invocations are
/// harmless because `try_init` returns `Err` after the first install.
fn init_tracing(cfg: &open_ontologies::config::LoggingConfig) {
    use tracing_subscriber::{EnvFilter, fmt};

    let level = open_ontologies::config::resolve_logging_level(cfg);
    let env_filter = EnvFilter::try_new(&level).unwrap_or_else(|_| EnvFilter::new("info"));

    // Output target: file when configured, else stderr (tracing default).
    let writer_file = cfg.file.as_deref().and_then(|p| {
        let path = open_ontologies::config::expand_tilde(p);
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok()
    });

    let format = cfg.format.trim().to_lowercase();
    // Build and install. Use `try_init` so calling this from multiple
    // subcommand handlers (or repeated test setups) is safe.
    let result = match (format.as_str(), writer_file) {
        ("json", Some(f)) => fmt()
            .with_env_filter(env_filter)
            .json()
            .with_writer(std::sync::Mutex::new(f))
            .try_init(),
        ("json", None) => fmt()
            .with_env_filter(env_filter)
            .json()
            .with_writer(std::io::stderr)
            .try_init(),
        ("pretty", Some(f)) => fmt()
            .with_env_filter(env_filter)
            .pretty()
            .with_writer(std::sync::Mutex::new(f))
            .try_init(),
        ("pretty", None) => fmt()
            .with_env_filter(env_filter)
            .pretty()
            .with_writer(std::io::stderr)
            .try_init(),
        (_, Some(f)) => fmt()
            .with_env_filter(env_filter)
            .compact()
            .with_writer(std::sync::Mutex::new(f))
            .try_init(),
        (_, None) => fmt()
            .with_env_filter(env_filter)
            .compact()
            .with_writer(std::io::stderr)
            .try_init(),
    };
    let _ = result;
}

/// Compose the effective tool filter from `[tools]` in config + CLI flags.
/// CLI `--tools-allow` / `--tools-deny` override `[tools]` when present.
fn build_tool_filter(
    cfg: &Config,
    cli_allow: Option<&str>,
    cli_deny: Option<&str>,
) -> anyhow::Result<open_ontologies::toolfilter::ToolFilter> {
    use open_ontologies::toolfilter::{Mode, ToolFilter, parse_csv};

    if cli_allow.is_some() && cli_deny.is_some() {
        anyhow::bail!("--tools-allow and --tools-deny are mutually exclusive");
    }
    if let Some(spec) = cli_allow {
        let (list, groups) = parse_csv(spec);
        return Ok(ToolFilter {
            mode: Mode::Allow,
            list,
            groups,
        });
    }
    if let Some(spec) = cli_deny {
        let (list, groups) = parse_csv(spec);
        return Ok(ToolFilter {
            mode: Mode::Deny,
            list,
            groups,
        });
    }
    // Fall back to config file.
    let mode = if cfg.tools.mode.is_empty() {
        Mode::All
    } else {
        Mode::parse(&cfg.tools.mode).map_err(|e| anyhow::anyhow!(e))?
    };
    Ok(ToolFilter {
        mode,
        list: cfg.tools.list.clone(),
        groups: cfg.tools.groups.clone(),
    })
}

/// Resolves when the process is asked to terminate, naming the signal.
///
/// Listens for ctrl-c on every platform and, on unix, for `SIGTERM` as well —
/// the latter is what `docker stop`, systemd and Kubernetes actually send, so
/// handling only ctrl-c would leave container shutdown unhandled. The unix arm
/// is behind `#[cfg(unix)]` so the `x86_64-pc-windows-msvc` target in the
/// release matrix still builds; there the future simply never resolves.
async fn shutdown_signal() -> &'static str {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            eprintln!("Failed to install ctrl-c handler: {e}");
            // Never resolve: a broken handler must not look like a shutdown request.
            std::future::pending::<()>().await
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                eprintln!("Failed to install SIGTERM handler: {e}");
                std::future::pending::<()>().await
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => "ctrl-c",
        _ = terminate => "SIGTERM",
    }
}

/// Exit code conventionally reported for a process ended by `sig`.
fn signal_exit_code(sig: &str) -> i32 {
    // 128 + signal number: SIGINT = 2, SIGTERM = 15.
    if sig == "ctrl-c" {
        130
    } else {
        143
    }
}

fn main() -> anyhow::Result<()> {
    // The root async future is polled on the calling thread. Windows gives
    // the main thread 1 MiB of stack (vs 8 MiB on Linux/macOS), which
    // overflows in debug builds, so run on a thread with an explicit 8 MiB.
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(async_main)?
        .join()
        .expect("main thread panicked")
}

#[tokio::main]
async fn async_main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Activate JSON mode when --json or --pretty is passed.
    JSON_MODE.set(cli.json || cli.pretty || !cli.human).ok();

    let proxy_ctx = ProxyCtx {
        no_connect: cli.no_connect,
        data_dir: cli.data_dir.clone(),
        pretty: cli.pretty,
    };

    // If a daemon is running and this command is proxy-able, route to it.
    if let Some(batch) = cli.command.to_batch_command() {
        let payload = serde_json::to_string(&vec![batch])?;
        if let Some(code) = proxy_if_daemon(&proxy_ctx, &payload, false).await? {
            std::process::exit(code);
        }
    }

    match cli.command {
        Commands::Init {
            data_dir,
            model_url: _model_url,
            tokenizer_url: _tokenizer_url,
            model_name: _model_name,
        } => {
            let data_dir = expand_tilde(&data_dir);
            let data_path = std::path::Path::new(&data_dir);

            std::fs::create_dir_all(data_path)?;
            println!("Created data directory: {data_dir}");

            let db_path = data_path.join("open-ontologies.db");
            let _db = StateDb::open(&db_path)?;
            println!("Initialized database: {}", db_path.display());

            let config_path = data_path.join("config.toml");
            if !config_path.exists() {
                std::fs::write(&config_path, DEFAULT_CONFIG)?;
                println!("Created default config: {}", config_path.display());
            } else {
                println!("Config already exists: {}", config_path.display());
            }

            #[cfg(feature = "embeddings")]
            {
                let models_dir = data_path.join("models");

                // CLI flags > config.toml > defaults
                let cfg = open_ontologies::config::Config::load(&config_path)
                    .map(|c| c.embeddings)
                    .unwrap_or_default();

                let provider = open_ontologies::config::resolve_embeddings_provider(&cfg);
                if provider == "openai"
                    || provider == "openai-compatible"
                    || provider == "remote"
                    || provider == "http"
                {
                    println!(
                        "Embeddings provider: {} — skipping local ONNX model download.",
                        provider
                    );
                    println!(
                        "  Model: {}",
                        open_ontologies::config::resolve_embeddings_model(&cfg)
                    );
                    println!(
                        "  API base: {}",
                        open_ontologies::config::resolve_embeddings_api_base(&cfg)
                    );
                    if open_ontologies::config::resolve_embeddings_api_key(&cfg).is_none() {
                        println!(
                            "  Note: no API key configured (set OPENAI_API_KEY, \
                             OPEN_ONTOLOGIES_EMBEDDINGS_API_KEY, or [embeddings].api_key \
                             in config.toml if your gateway requires auth)."
                        );
                    }
                } else {
                    std::fs::create_dir_all(&models_dir)?;

                    let onnx_url = _model_url
                        .as_deref()
                        .or(cfg.model_url.as_deref())
                        .unwrap_or(open_ontologies::embed::DEFAULT_MODEL_ONNX_URL);
                    let tok_url = _tokenizer_url
                        .as_deref()
                        .or(cfg.tokenizer_url.as_deref())
                        .unwrap_or(open_ontologies::embed::DEFAULT_MODEL_TOKENIZER_URL);
                    let onnx_filename = _model_name
                        .as_deref()
                        .or(cfg.model_name.as_deref())
                        .unwrap_or(open_ontologies::embed::DEFAULT_MODEL_FILENAME);

                    let model_path = models_dir.join(onnx_filename);
                    let tokenizer_path = models_dir.join("tokenizer.json");

                    if !model_path.exists() {
                        println!("Downloading embedding model from {}...", onnx_url);
                        open_ontologies::embed::download_model_file(onnx_url, &model_path).await?;
                        println!("  Model saved: {}", model_path.display());
                    } else {
                        println!("Embedding model already exists: {}", model_path.display());
                    }

                    if !tokenizer_path.exists() {
                        println!("Downloading tokenizer from {}...", tok_url);
                        open_ontologies::embed::download_model_file(tok_url, &tokenizer_path)
                            .await?;
                        println!("  Tokenizer saved: {}", tokenizer_path.display());
                    } else {
                        println!("Tokenizer already exists: {}", tokenizer_path.display());
                    }
                }
            }

            println!("\nOpen Ontologies initialized successfully!");
        }
        Commands::Serve {
            config: config_path,
            governance_webhook,
            watch,
            watch_interval,
            tools_allow,
            tools_deny,
            idle_ttl_secs,
            auto_refresh,
            storage_mode,
        } => {
            let config_path = expand_tilde(&config_path);
            let cfg = match Config::load(std::path::Path::new(&config_path)) {
                Ok(c) => c,
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("failed to read") {
                        Config::default()
                    } else {
                        return Err(e);
                    }
                }
            };
            // Initialise tracing (RUST_LOG > [logging] level > default).
            init_tracing(&cfg.logging);
            // Initialise runtime knobs (tableaux limits, fixpoint cap, hash
            // prefix, feedback thresholds, repo / imports / webhook).
            open_ontologies::runtime::init_from_config(&cfg);

            // Same precedence as the HTTP arm: an explicit --data-dir wins over
            // config rather than being ignored.
            let data_dir = if cli.data_dir != DEFAULT_DATA_DIR {
                expand_tilde(&cli.data_dir)
            } else {
                expand_tilde(&cfg.general.data_dir)
            };
            let data_path = std::path::Path::new(&data_dir);
            let db_path = data_path.join("open-ontologies.db");

            std::fs::create_dir_all(data_path)?;
            let db = StateDb::open(&db_path)?;

            let mut storage_cfg = cfg.storage.clone();
            if let Some(m) = storage_mode.as_deref() {
                storage_cfg.mode = m.to_string();
            }
            let graph = build_main_graph(&storage_cfg, data_path)?;

            // Monitor: CLI `--watch` forces enabled; otherwise fall back to
            // `[monitor] enabled`. CLI `--watch-interval` > env > `[monitor]
            // interval_secs` > default 30.
            let monitor_enabled = watch || cfg.monitor.enabled;
            let monitor_interval = watch_interval.unwrap_or_else(|| {
                open_ontologies::config::resolve_monitor_interval_secs(&cfg.monitor)
            });

            let _watch_handle = if monitor_enabled {
                let watch_db = StateDb::open(&db_path)?;
                Some(open_ontologies::monitor::start_background_loop(
                    watch_db,
                    graph.clone(),
                    std::time::Duration::from_secs(monitor_interval),
                ))
            } else {
                None
            };

            let cache_config = build_cache_config(&cfg, idle_ttl_secs, auto_refresh);
            let tool_filter =
                build_tool_filter(&cfg, tools_allow.as_deref(), tools_deny.as_deref())?;
            let ontology_dirs =
                open_ontologies::config::resolve_ontology_dirs(&cfg.general.ontology_dirs);
            for d in &ontology_dirs {
                if !d.exists() {
                    eprintln!(
                        "warning: ontology_dirs entry does not exist: {}",
                        d.display()
                    );
                }
            }
            let server = OpenOntologiesServer::new_with_repo_options(
                db,
                graph,
                governance_webhook,
                cfg.embeddings,
                cache_config,
                tool_filter,
                ontology_dirs,
            );
            let _evictor = open_ontologies::registry::spawn_evictor(server.registry());
            let service = server.serve(rmcp::transport::stdio()).await?;
            service.waiting().await?;
        }
        Commands::ServeHttp {
            config: config_path,
            host,
            port,
            token,
            governance_webhook,
            watch,
            watch_interval,
            tools_allow,
            tools_deny,
            idle_ttl_secs,
            auto_refresh,
            storage_mode,
        } => {
            use rmcp::transport::streamable_http_server::{
                StreamableHttpServerConfig, StreamableHttpService,
                session::local::LocalSessionManager,
            };
            use tokio_util::sync::CancellationToken;

            let config_path = expand_tilde(&config_path);
            let cfg = match Config::load(std::path::Path::new(&config_path)) {
                Ok(c) => c,
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("failed to read") {
                        Config::default()
                    } else {
                        return Err(e);
                    }
                }
            };
            init_tracing(&cfg.logging);
            open_ontologies::runtime::init_from_config(&cfg);

            // Resolve effective host / port / token / stateful_mode honouring
            // CLI > env > config > default precedence.
            let host =
                host.unwrap_or_else(|| open_ontologies::config::resolve_http_host(&cfg.http));
            let port =
                port.unwrap_or_else(|| open_ontologies::config::resolve_http_port(&cfg.http));
            // Clap reads `OPEN_ONTOLOGIES_TOKEN` into `token` automatically
            // (because of `env = "OPEN_ONTOLOGIES_TOKEN"`), so a non-`None`
            // value already encompasses CLI + env. Fall back to config when
            // neither is set.
            let token = token.or_else(|| open_ontologies::config::resolve_http_token(&cfg.http));

            // `--data-dir` used to be dropped here, so `--data-dir /custom daemon
            // start` wrote daemon.json into /custom while the daemon it started
            // served ~/.open-ontologies: proxied and local commands then saw
            // different stores and nothing said so. Invisible whenever both
            // resolve to the same directory, which is why it survived testing.
            // Precedence matches host/port/token above: CLI over config.
            let data_dir = if cli.data_dir != DEFAULT_DATA_DIR {
                expand_tilde(&cli.data_dir)
            } else {
                expand_tilde(&cfg.general.data_dir)
            };
            let data_path_owned = std::path::PathBuf::from(&data_dir);
            let db_path_owned = data_path_owned.join("open-ontologies.db");

            std::fs::create_dir_all(&data_path_owned)?;

            // Shared graph store — all MCP sessions (agent + frontend) see the same triples
            let mut storage_cfg = cfg.storage.clone();
            if let Some(m) = storage_mode.as_deref() {
                storage_cfg.mode = m.to_string();
            }
            let shared_graph = build_main_graph(&storage_cfg, &data_path_owned)?;

            // Shared StateDb for lineage REST endpoint
            let shared_db = StateDb::open(&db_path_owned)?;

            let monitor_enabled = watch || cfg.monitor.enabled;
            let monitor_interval = watch_interval.unwrap_or_else(|| {
                open_ontologies::config::resolve_monitor_interval_secs(&cfg.monitor)
            });

            let _watch_handle = if monitor_enabled {
                let watch_db = StateDb::open(&db_path_owned)?;
                Some(open_ontologies::monitor::start_background_loop(
                    watch_db,
                    shared_graph.clone(),
                    std::time::Duration::from_secs(monitor_interval),
                ))
            } else {
                None
            };

            let ct = CancellationToken::new();
            // This token is the SOLE shutdown trigger for the HTTP transport: it
            // is handed to StreamableHttpServerConfig just below and awaited by
            // `axum::serve(..).with_graceful_shutdown(..)` at the end of this arm.
            // Nothing else cancels it, so the task spawned here is what makes that
            // path reachable at all — without it the shutdown future pends forever
            // and the process is killed outright, with the state DB never flushed.
            tokio::spawn({
                let ct = ct.clone();
                async move {
                    let sig = shutdown_signal().await;
                    eprintln!("{sig} received — stopping HTTP server");
                    ct.cancel();
                    // Tokio installs its signal handlers process-wide and never
                    // removes them, so from here the default terminate
                    // disposition is gone for good. If graceful shutdown then
                    // stalls — a long synchronous /api/query or /api/load holds
                    // its connection open, and axum waits for in-flight
                    // requests — a second signal would be swallowed and only
                    // SIGKILL would end the process. Stay listening so the
                    // second one is decisive.
                    let sig = shutdown_signal().await;
                    eprintln!("Second {sig} while shutting down — forcing exit");
                    std::process::exit(signal_exit_code(sig));
                }
            });
            // rmcp >=1.4 marks StreamableHttpServerConfig #[non_exhaustive], so it
            // can no longer be built with a struct literal from this crate. Start
            // from Default and set the public fields we care about.
            #[allow(clippy::field_reassign_with_default)]
            let http_config = {
                let mut c = StreamableHttpServerConfig::default();
                c.stateful_mode = cfg.http.stateful_mode;
                c.cancellation_token = ct.clone();
                c
            };

            let shared_graph_for_service = shared_graph.clone();
            let gw_for_service = governance_webhook.clone();
            let embed_config = cfg.embeddings.clone();
            let cache_config = build_cache_config(&cfg, idle_ttl_secs, auto_refresh);
            let tool_filter =
                build_tool_filter(&cfg, tools_allow.as_deref(), tools_deny.as_deref())?;
            let ontology_dirs =
                open_ontologies::config::resolve_ontology_dirs(&cfg.general.ontology_dirs);
            for d in &ontology_dirs {
                if !d.exists() {
                    eprintln!(
                        "warning: ontology_dirs entry does not exist: {}",
                        d.display()
                    );
                }
            }
            // Spawn a single evictor backed by a registry over the shared graph.
            // Each per-session server constructs its own registry (active slot
            // is per-session anyway), but the shared one drives memory cleanup.
            {
                let evictor_db = StateDb::open(&db_path_owned)?;
                let shared_registry = Arc::new(open_ontologies::registry::OntologyRegistry::new(
                    shared_graph.clone(),
                    evictor_db,
                    cache_config.clone(),
                )?);
                let _evictor = open_ontologies::registry::spawn_evictor(shared_registry);
            }
            let cache_for_service = cache_config.clone();
            let filter_for_service = tool_filter.clone();
            let dirs_for_service = ontology_dirs.clone();
            let service: StreamableHttpService<_, LocalSessionManager> = StreamableHttpService::new(
                move || {
                    let db = StateDb::open(&db_path_owned).map_err(std::io::Error::other)?;
                    Ok(OpenOntologiesServer::new_with_repo_options(
                        db,
                        shared_graph_for_service.clone(),
                        gw_for_service.clone(),
                        embed_config.clone(),
                        cache_for_service.clone(),
                        filter_for_service.clone(),
                        dirs_for_service.clone(),
                    ))
                },
                Default::default(),
                http_config,
            );

            // Simple REST API — no MCP sessions, direct access to shared graph
            let sg_stats = shared_graph.clone();
            let sg_query = shared_graph.clone();
            let sg_update = shared_graph.clone();
            let sg_load = shared_graph.clone();
            let sg_save = shared_graph.clone();
            let sg_load_turtle = shared_graph.clone();
            let sg_batch = shared_graph.clone();
            let db_for_batch = shared_db.clone();
            let api = axum::Router::new()
                .route("/stats", axum::routing::get(move || {
                    let g = sg_stats.clone();
                    async move {
                        axum::Json(serde_json::from_str::<serde_json::Value>(
                            &g.get_stats().unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
                        ).unwrap_or_default())
                    }
                }))
                .route("/query", axum::routing::post(move |body: axum::Json<serde_json::Value>| {
                    let g = sg_query.clone();
                    async move {
                        let query = body.0["query"].as_str().unwrap_or("").to_string();
                        axum::Json(serde_json::from_str::<serde_json::Value>(
                            &g.sparql_select(&query).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
                        ).unwrap_or_default())
                    }
                }))
                .route("/update", axum::routing::post(move |body: axum::Json<serde_json::Value>| {
                    let g = sg_update.clone();
                    async move {
                        let query = body.0["query"].as_str().unwrap_or("").to_string();
                        axum::Json(serde_json::from_str::<serde_json::Value>(
                            &match g.sparql_update(&query) {
                                Ok(n)  => format!(r#"{{"ok":true,"affected":{}}}"#, n),
                                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                            }
                        ).unwrap_or_default())
                    }
                }))
                .route("/load", axum::routing::post(move |body: axum::Json<serde_json::Value>| {
                    let g = sg_load.clone();
                    async move {
                        let path = body.0["path"].as_str().unwrap_or("").to_string();
                        let path = open_ontologies::config::expand_tilde(&path);
                        axum::Json(serde_json::from_str::<serde_json::Value>(
                            &match g.load_file(&path) {
                                Ok(n)  => format!(r#"{{"ok":true,"triples_loaded":{}}}"#, n),
                                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                            }
                        ).unwrap_or_default())
                    }
                }))
                .route("/load-turtle", axum::routing::post(move |body: axum::Json<serde_json::Value>| {
                    let g = sg_load_turtle.clone();
                    async move {
                        let turtle = body.0["turtle"].as_str().unwrap_or("").to_string();
                        let base = body.0["base"].as_str().map(|s| s.to_string());
                        axum::Json(serde_json::from_str::<serde_json::Value>(
                            &match g.load_turtle(&turtle, base.as_deref()) {
                                Ok(n)  => format!(r#"{{"ok":true,"triples_loaded":{}}}"#, n),
                                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                            }
                        ).unwrap_or_default())
                    }
                }))
                .route("/save", axum::routing::post(move |body: axum::Json<serde_json::Value>| {
                    let g = sg_save.clone();
                    async move {
                        let path = body.0["path"].as_str().unwrap_or("~/.open-ontologies/studio-live.ttl").to_string();
                        let format = body.0["format"].as_str().unwrap_or("turtle").to_string();
                        let path = open_ontologies::config::expand_tilde(&path);
                        axum::Json(serde_json::from_str::<serde_json::Value>(
                            &match g.save_file(&path, &format) {
                                Ok(_)  => format!(r#"{{"ok":true,"path":"{}"}}"#, path),
                                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                            }
                        ).unwrap_or_default())
                    }
                }))
                .route("/batch", axum::routing::post(move |req: axum::extract::Request| {
                    let g = sg_batch.clone();
                    let db = db_for_batch.clone();
                    async move {
                        let bail = req.uri().query()
                            .map(|q| q.contains("bail=true"))
                            .unwrap_or(false);
                        let body = axum::body::to_bytes(req.into_body(), usize::MAX).await
                            .unwrap_or_default();
                        let input = String::from_utf8_lossy(&body).to_string();
                        // The handle opened at startup, not a fresh
                        // `StateDb::open` per request: this is the hot path the
                        // daemon exists to make fast, and it was paying a SQLite
                        // open on every call while the adjacent /lineage route
                        // already cloned the shared one.
                        let runner = open_ontologies::batch::BatchRunner::new(db, g, false);
                        let (results, _) = runner.run_collect(&input, bail).await;
                        (axum::http::StatusCode::OK, axum::Json(serde_json::json!(results)))
                    }
                }))
                .route("/lineage", axum::routing::get(move || {
                    let db = shared_db.clone();
                    async move {
                        let conn = db.conn();
                        let mut stmt = conn.prepare(
                            "SELECT session_id, seq, timestamp, event_type, operation, details \
                             FROM lineage_events ORDER BY CAST(timestamp AS INTEGER) ASC, seq ASC LIMIT 500"
                        ).unwrap();
                        let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
                            let session_id: String = row.get(0)?;
                            let seq: i64 = row.get(1)?;
                            let timestamp: String = row.get(2)?;
                            let event_type: String = row.get(3)?;
                            let operation: String = row.get(4)?;
                            let details: String = row.get::<_, Option<String>>(5)?.unwrap_or_default();
                            Ok(serde_json::json!({
                                "session": session_id,
                                "seq": seq,
                                "ts": timestamp,
                                "type": event_type,
                                "op": operation,
                                "details": details
                            }))
                        }).unwrap().filter_map(|r| r.ok()).collect();
                        axum::Json(serde_json::json!({ "events": rows }))
                    }
                }));

            let router = axum::Router::new()
                .nest("/api", api)
                .nest_service("/mcp", service);
            let router = if let Some(ref token) = token {
                let expected = format!("Bearer {}", token);
                router.layer(axum::middleware::from_fn(
                    move |req: axum::extract::Request, next: axum::middleware::Next| {
                        let expected = expected.clone();
                        async move {
                            let auth = req
                                .headers()
                                .get("authorization")
                                .and_then(|v| v.to_str().ok());
                            if auth == Some(&expected) {
                                next.run(req).await
                            } else {
                                axum::http::Response::builder()
                                    .status(401)
                                    .body(axum::body::Body::from("Unauthorized"))
                                    .unwrap()
                            }
                        }
                    },
                ))
            } else {
                router
            };
            // Liveness probe. Registered AFTER the bearer layer on purpose:
            // `Router::layer` only wraps the routes already present, so adding
            // /health here leaves it outside the auth middleware while /api and
            // /mcp stay behind it. Putting it inside the `if let` branch above,
            // or before the layer call, would make it require credentials in the
            // one deployment where an unauthenticated probe is the point.
            //
            // The body is deliberately limited to status and version: an
            // unauthenticated endpoint should not describe loaded state, which is
            // exactly why /api/stats is not the right thing to probe.
            let router = router.route(
                "/health",
                axum::routing::get(|| async {
                    axum::Json(serde_json::json!({
                        "status": "ok",
                        "version": env!("CARGO_PKG_VERSION"),
                    }))
                }),
            );
            let router = router.layer(tower_http::cors::CorsLayer::permissive());
            let addr = format!("{host}:{port}");
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            eprintln!("Open Ontologies MCP server listening on http://{addr}/mcp");
            if token.is_some() {
                eprintln!("  Authentication: bearer token required");
            }

            axum::serve(listener, router)
                .with_graceful_shutdown(async move { ct.cancelled_owned().await })
                .await?;
        }

        #[cfg(unix)]
        Commands::ServeUnix {
            config: config_path,
            socket,
            files,
        } => {
            let config_path = expand_tilde(&config_path);
            let cfg = match Config::load(std::path::Path::new(&config_path)) {
                Ok(c) => c,
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("failed to read") {
                        Config::default()
                    } else {
                        return Err(e);
                    }
                }
            };
            init_tracing(&cfg.logging);
            open_ontologies::runtime::init_from_config(&cfg);

            // CLI > [socket] path > legacy default
            let socket_path = socket
                .or_else(|| cfg.socket.path.clone())
                .unwrap_or_else(|| "/tmp/tardygrada-ontology-complete.sock".to_string());

            // CLI `--file` (when supplied) overrides `[socket] preload_files`.
            let preload: Vec<String> = if !files.is_empty() {
                files
            } else {
                cfg.socket.preload_files.clone()
            };

            let graph = Arc::new(GraphStore::new());
            for f in &preload {
                let path = open_ontologies::config::expand_tilde(f);
                match graph.load_file(&path) {
                    Ok(n) => eprintln!("Loaded {path}: {n} triples"),
                    Err(e) => {
                        eprintln!("Failed to load {path}: {e}");
                        std::process::exit(1);
                    }
                }
            }
            eprintln!("Graph has {} triples total", graph.triple_count());

            use tokio_util::sync::CancellationToken;
            // Same rationale as the ServeHttp arm: this token is the sole
            // shutdown trigger. Without the task below, the accept loop in
            // socket::serve_with_shutdown never exits, the process can only be
            // killed, and the socket file stays on disk after it is gone.
            let ct = CancellationToken::new();
            tokio::spawn({
                let ct = ct.clone();
                let socket_path = socket_path.clone();
                async move {
                    let sig = shutdown_signal().await;
                    eprintln!("{sig} received — closing socket");
                    ct.cancel();
                    // Same reasoning as the ServeHttp arm: tokio's handlers are
                    // installed for the process lifetime, so a second signal
                    // would be swallowed if the accept loop were slow to unwind.
                    // Unlink on the forced path too, so the escape hatch does
                    // not reintroduce the leaked socket this commit removes.
                    let sig = shutdown_signal().await;
                    eprintln!("Second {sig} while shutting down — forcing exit");
                    let _ = std::fs::remove_file(&socket_path);
                    std::process::exit(signal_exit_code(sig));
                }
            });
            open_ontologies::socket::serve_with_shutdown(&socket_path, graph, ct).await?;
        }
        #[cfg(windows)]
        Commands::ServeUnix { .. } => {
            eprintln!(
                "serve-unix is not available on Windows. Use `serve` or `serve-http` instead."
            );
            std::process::exit(1);
        }

        // ─── Batch ──────────────────────────────────────────────────
        Commands::Batch { input, bail } => {
            let batch_input = if input == "-" {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                buf
            } else {
                std::fs::read_to_string(&input)?
            };
            // Route to daemon if running.
            if let Some(code) = proxy_if_daemon(&proxy_ctx, &batch_input, bail).await? {
                std::process::exit(code);
            }
            let (db, graph) = setup(&cli.data_dir)?;
            let runner = open_ontologies::batch::BatchRunner::new(db, graph, cli.pretty);
            let exit_code = runner.run(&batch_input, bail).await;
            std::process::exit(exit_code);
        }

        // ─── Daemon ─────────────────────────────────────────────────
        Commands::Daemon { action } => {
            match action {
                DaemonAction::Start { host, port, token } => {
                    // Check if already running.
                    if let Some(info) = open_ontologies::daemon::read_daemon_info(&cli.data_dir)
                        && open_ontologies::daemon::is_daemon_alive(info.pid) {
                            output_json(&serde_json::json!({
                                "error": format!("daemon already running (pid {})", info.pid),
                                "url": info.url,
                            }), cli.pretty);
                            std::process::exit(1);
                        }
                    match open_ontologies::daemon::start_daemon(&cli.data_dir, &host, port, token) {
                        Ok(info) => output_json(&serde_json::json!({
                            "ok": true,
                            "pid": info.pid,
                            "url": info.url,
                        }), cli.pretty),
                        Err(e) => {
                            output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                            std::process::exit(1);
                        }
                    }
                }
                DaemonAction::Stop => {
                    match open_ontologies::daemon::stop_daemon(&cli.data_dir) {
                        Ok(()) => output_json(&serde_json::json!({"ok": true, "message": "daemon stopped"}), cli.pretty),
                        Err(e) => {
                            output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                            std::process::exit(1);
                        }
                    }
                }
                DaemonAction::Status => {
                    match open_ontologies::daemon::read_daemon_info(&cli.data_dir) {
                        None => output_json(&serde_json::json!({"alive": false, "message": "no daemon.json found"}), cli.pretty),
                        Some(info) => {
                            let alive = open_ontologies::daemon::is_daemon_alive(info.pid);
                            output_json(&serde_json::json!({
                                "alive": alive,
                                "pid": info.pid,
                                "url": info.url,
                            }), cli.pretty);
                        }
                    }
                }
            }
        }

        // ─── Core ontology ─────────────────────────────────────────
        Commands::Validate { input } => {
            let result = if input == "-" {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                GraphStore::validate_turtle(&buf)
            } else {
                GraphStore::validate_file(&input)
            };
            match result {
                Ok(counts) => output_json(
                    &serde_json::json!({
                        "ok": true,
                        "triples": counts.triples,
                        "statements": counts.statements,
                    }),
                    cli.pretty,
                ),
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::Load { path } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            match graph.load_file(&path) {
                Ok(count) => {
                    let mut report =
                        serde_json::json!({"ok": true, "triples_loaded": count, "path": path});
                    // In-memory is the default backend, so a one-shot `load`
                    // fills a store that dies with the process. Saying only
                    // "triples_loaded" here reads as durable, and the next
                    // command reporting zero reads as data loss.
                    if is_memory_storage(&cli.data_dir) {
                        report["warning"] = serde_json::Value::String(
                            "storage mode is 'memory', so this load did not persist: \
                             a following command starts from an empty store. Set \
                             [storage] mode = \"persistent\" or OPEN_ONTOLOGIES_STORAGE_MODE=persistent \
                             to chain CLI commands."
                                .to_string(),
                        );
                    }
                    output_json(&report, cli.pretty)
                }
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::Save { path, format } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            match graph.save_file(&path, &format) {
                Ok(_) => output_json(
                    &serde_json::json!({"ok": true, "path": path, "format": format}),
                    cli.pretty,
                ),
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::Clear => {
            let (_db, graph) = setup(&cli.data_dir)?;
            match graph.clear() {
                Ok(_) => output_json(
                    &serde_json::json!({"ok": true, "message": "Store cleared"}),
                    cli.pretty,
                ),
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::Stats => {
            let (_db, graph) = setup(&cli.data_dir)?;
            let stats_json = graph
                .get_stats()
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&stats_json, cli.pretty);
        }
        Commands::Query { query } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            let query_str = if query == "-" {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                buf
            } else {
                query
            };
            let result = graph
                .sparql_select(&query_str)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Diff { old_path, new_path } => {
            use open_ontologies::ontology::OntologyService;
            let old = std::fs::read_to_string(&old_path)?;
            let new = std::fs::read_to_string(&new_path)?;
            let result = OntologyService::diff(&old, &new)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Lint { input } => {
            use open_ontologies::ontology::OntologyService;
            let (db, _graph) = setup(&cli.data_dir)?;
            let raw = if input == "-" {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                buf
            } else {
                std::fs::read_to_string(&input)?
            };
            // lint reasons over Turtle. Sniff the serialisation first, exactly as
            // validate does, so an RDF/XML document is converted rather than
            // handed to a Turtle parser that can only fail on it.
            let content = GraphStore::content_as_turtle(&input, raw)
                .unwrap_or_else(|e| format!("# could not read as RDF: {e}\n"));
            let result = OntologyService::lint_with_feedback(&content, Some(&db))
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Convert { path, to, output } => {
            let store = GraphStore::new();
            match store.load_file(&path) {
                Ok(_) => match store.serialize(&to) {
                    Ok(content) => {
                        if let Some(out_path) = output {
                            std::fs::write(&out_path, &content)?;
                            output_json(
                                &serde_json::json!({"ok": true, "path": out_path, "format": to}),
                                cli.pretty,
                            );
                        } else {
                            println!("{}", content);
                        }
                    }
                    Err(e) => {
                        output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::Status => {
            let (_db, graph) = setup(&cli.data_dir)?;
            output_json(
                &serde_json::json!({
                    "status": "ok",
                    "version": env!("CARGO_PKG_VERSION"),
                    "triples_loaded": graph.triple_count(),
                }),
                cli.pretty,
            );
        }

        // ─── Remote ─────────────────────────────────────────────────
        Commands::Marketplace { action, id, domain } => {
            use open_ontologies::marketplace;
            match action.as_str() {
                "list" => {
                    let (items, community_error) =
                        marketplace::cli_list(domain.as_deref()).await;
                    output_json(
                        &serde_json::json!({
                            "count": items.len(),
                            "ontologies": items,
                            "community_registry_error": community_error,
                        }),
                        cli.pretty,
                    );
                }
                "install" => {
                    let id = id.as_deref().unwrap_or_else(|| {
                        eprintln!("Error: --id is required for install");
                        std::process::exit(1);
                    });
                    // Curated first; community packs can never shadow a curated ID.
                    let pack = match marketplace::cli_resolve(id).await {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("{}", e);
                            std::process::exit(1);
                        }
                    };
                    let (entry_id, entry_name, entry_url, entry_format) =
                        (pack.id, pack.name, pack.url, pack.format);
                    let (_db, graph) = setup(&cli.data_dir)?;
                    let content = GraphStore::fetch_url(&entry_url).await?;
                    match graph.load_content_with_base(&content, entry_format, Some(&entry_url)) {
                        Ok(count) => {
                            let stats = graph.get_stats().unwrap_or_default();
                            output_json(
                                &serde_json::json!({
                                    "ok": true,
                                    "installed": entry_id,
                                    "name": entry_name,
                                    "triples_loaded": count,
                                    "stats": serde_json::from_str::<serde_json::Value>(&stats).unwrap_or_default(),
                                }),
                                cli.pretty,
                            );
                        }
                        Err(e) => {
                            output_json(
                                &serde_json::json!({"error": format!("Parse error: {}", e)}),
                                cli.pretty,
                            );
                            std::process::exit(1);
                        }
                    }
                }
                _ => {
                    eprintln!("Unknown action: '{}'. Use 'list' or 'install'.", action);
                    std::process::exit(1);
                }
            }
        }
        Commands::Pull { url, sparql, query } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            let content = if sparql {
                let q = query
                    .as_deref()
                    .unwrap_or("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }");
                GraphStore::fetch_sparql(&url, q).await?
            } else {
                GraphStore::fetch_url(&url).await?
            };
            match graph.load_turtle(&content, None) {
                Ok(count) => output_json(
                    &serde_json::json!({"ok": true, "triples_loaded": count, "source": url}),
                    cli.pretty,
                ),
                Err(e) => {
                    output_json(
                        &serde_json::json!({"error": format!("Parse error: {}", e)}),
                        cli.pretty,
                    );
                    std::process::exit(1);
                }
            }
        }
        Commands::Push {
            endpoint,
            graph: graph_name,
        } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            let content = graph.serialize("ntriples")?;
            match GraphStore::push_sparql(&endpoint, &content).await {
                Ok(msg) => {
                    output_json(&serde_json::json!({"ok": true, "message": msg}), cli.pretty)
                }
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            }
            let _ = graph_name; // reserved for future named graph support
        }
        Commands::ImportOwl { max_depth } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            let mut imported = Vec::new();
            let mut to_import: Vec<String> = Vec::new();

            let query =
                "SELECT ?import WHERE { ?onto <http://www.w3.org/2002/07/owl#imports> ?import }";
            if let Ok(result) = graph.sparql_select(query)
                && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result)
                && let Some(results) = parsed["results"].as_array()
            {
                for row in results {
                    if let Some(uri) = row["import"].as_str() {
                        let uri = uri.trim_matches(|c| c == '<' || c == '>');
                        to_import.push(uri.to_string());
                    }
                }
            }

            let mut depth = 0;
            while !to_import.is_empty() && depth < max_depth {
                let batch = std::mem::take(&mut to_import);
                for url in batch {
                    if imported.contains(&url) {
                        continue;
                    }
                    match GraphStore::fetch_url(&url).await {
                        Ok(content) => {
                            if let Ok(count) = graph.load_turtle(&content, None) {
                                eprintln!("Imported {} ({} triples)", url, count);
                                imported.push(url);
                            }
                        }
                        Err(e) => eprintln!("Failed to import {}: {}", url, e),
                    }
                }
                depth += 1;
            }

            output_json(
                &serde_json::json!({"ok": true, "imported": imported.len(), "urls": imported}),
                cli.pretty,
            );
        }

        // ─── Versioning ────────────────────────────────────────────
        Commands::Version { label } => {
            use open_ontologies::ontology::OntologyService;
            let (db, graph) = setup(&cli.data_dir)?;
            let result = OntologyService::save_version(&db, &graph, &label)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::History => {
            use open_ontologies::ontology::OntologyService;
            let (db, _graph) = setup(&cli.data_dir)?;
            let result = OntologyService::list_versions(&db)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Rollback { label } => {
            use open_ontologies::ontology::OntologyService;
            let (db, graph) = setup(&cli.data_dir)?;
            let result = OntologyService::rollback_version(&db, &graph, &label)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }

        // ─── Data pipeline ──────────────────────────────────────────
        Commands::Map {
            data_path,
            format: _format,
            save,
        } => {
            use open_ontologies::ingest::DataIngester;
            use open_ontologies::mapping::MappingConfig;
            let (_db, graph) = setup(&cli.data_dir)?;

            let rows = DataIngester::parse_file(&data_path)?;
            let headers = DataIngester::extract_headers(&rows);

            let classes_query = r#"SELECT DISTINCT ?c WHERE { { ?c a <http://www.w3.org/2002/07/owl#Class> } UNION { ?c a <http://www.w3.org/2000/01/rdf-schema#Class> } }"#;
            let props_query = r#"SELECT DISTINCT ?p WHERE { { ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> } UNION { ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> } UNION { ?p a <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property> } }"#;

            let classes = graph.sparql_select(classes_query).unwrap_or_default();
            let props = graph.sparql_select(props_query).unwrap_or_default();

            let mapping = MappingConfig::from_headers(
                &headers,
                "http://example.org/data/",
                "http://example.org/data/Thing",
            );
            let mapping_json = serde_json::to_string_pretty(&mapping).unwrap_or_default();

            if let Some(save_path) = save {
                std::fs::write(&save_path, &mapping_json)?;
                output_json(
                    &serde_json::json!({"ok": true, "saved": save_path}),
                    cli.pretty,
                );
            } else {
                let extract_iris = |json: &str, var: &str| -> Vec<String> {
                    serde_json::from_str::<serde_json::Value>(json)
                        .ok()
                        .and_then(|v| v["results"].as_array().cloned())
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|r| {
                            r[var]
                                .as_str()
                                .map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string())
                        })
                        .collect()
                };
                output_json(
                    &serde_json::json!({
                        "data_fields": headers,
                        "ontology_classes": extract_iris(&classes, "c"),
                        "ontology_properties": extract_iris(&props, "p"),
                        "suggested_mapping": serde_json::from_str::<serde_json::Value>(&mapping_json).unwrap_or_default(),
                    }),
                    cli.pretty,
                );
            }
        }
        Commands::Ingest {
            path,
            format: _format,
            mapping,
            base_iri,
        } => {
            use open_ontologies::ingest::DataIngester;
            use open_ontologies::mapping::MappingConfig;
            let (_db, graph) = setup(&cli.data_dir)?;

            let base = base_iri.as_deref().unwrap_or("http://example.org/data/");
            let rows = DataIngester::parse_file(&path)?;

            if rows.is_empty() {
                output_json(
                    &serde_json::json!({"ok": true, "triples_loaded": 0, "warnings": ["No data rows found"]}),
                    cli.pretty,
                );
            } else {
                let mapping_config = if let Some(ref mapping_path) = mapping {
                    let content = std::fs::read_to_string(mapping_path)?;
                    serde_json::from_str::<MappingConfig>(&content)?
                } else {
                    let headers = DataIngester::extract_headers(&rows);
                    MappingConfig::from_headers(&headers, base, &format!("{}Thing", base))
                };

                let ntriples = mapping_config.rows_to_ntriples(&rows);
                match graph.load_ntriples(&ntriples) {
                    Ok(count) => output_json(
                        &serde_json::json!({"ok": true, "triples_loaded": count, "rows": rows.len()}),
                        cli.pretty,
                    ),
                    Err(e) => {
                        output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Shacl { shapes } => {
            use open_ontologies::shacl::ShaclValidator;
            let (_db, graph) = setup(&cli.data_dir)?;
            let shapes_content = std::fs::read_to_string(&shapes)?;
            let result = ShaclValidator::validate(&graph, &shapes_content)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::VocabCheck { data } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            let data_content = std::fs::read_to_string(&data)?;
            let result = open_ontologies::vocab_check::check_data_vocab(&graph, &data_content, &[])
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Reason { profile } => {
            use open_ontologies::reason::Reasoner;
            let (_db, graph) = setup(&cli.data_dir)?;
            let result = Reasoner::run(&graph, &profile, true)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Extend {
            data_path,
            format: _format,
            mapping,
            shapes,
            profile,
        } => {
            use open_ontologies::ingest::DataIngester;
            use open_ontologies::mapping::MappingConfig;
            use open_ontologies::reason::Reasoner;
            use open_ontologies::shacl::ShaclValidator;
            let (_db, graph) = setup(&cli.data_dir)?;

            let base_iri = "http://example.org/data/";

            // 1. Ingest
            let rows = DataIngester::parse_file(&data_path)?;
            let mapping_config = if let Some(ref mapping_path) = mapping {
                let content = std::fs::read_to_string(mapping_path)?;
                serde_json::from_str::<MappingConfig>(&content)?
            } else {
                let headers = DataIngester::extract_headers(&rows);
                MappingConfig::from_headers(&headers, base_iri, &format!("{}Thing", base_iri))
            };

            let ntriples = mapping_config.rows_to_ntriples(&rows);
            let triples_loaded = graph.load_ntriples(&ntriples)?;

            // 2. SHACL (optional)
            let shacl_result = if let Some(ref shapes_path) = shapes {
                let shapes_content = std::fs::read_to_string(shapes_path)?;
                Some(
                    ShaclValidator::validate(&graph, &shapes_content)
                        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e)),
                )
            } else {
                None
            };

            // 3. Reason (optional)
            let reason_result = profile.as_ref().map(|prof| {
                Reasoner::run(&graph, prof, true)
                    .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
            });

            output_json(
                &serde_json::json!({
                    "ok": true,
                    "triples_loaded": triples_loaded,
                    "rows": rows.len(),
                    "shacl": shacl_result.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
                    "reason": reason_result.and_then(|r| serde_json::from_str::<serde_json::Value>(&r).ok()),
                }),
                cli.pretty,
            );
        }

        // ─── Lifecycle ──────────────────────────────────────────────
        Commands::Plan { file } => {
            let (db, graph) = setup(&cli.data_dir)?;
            let turtle = std::fs::read_to_string(&file)?;
            let planner = open_ontologies::plan::Planner::new(db, graph);
            let result = planner
                .plan(&turtle)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Apply { mode, plan_id } => {
            let (db, graph) = setup(&cli.data_dir)?;
            let planner = open_ontologies::plan::Planner::new(db, graph);
            let result = planner
                .apply_plan(plan_id.as_deref(), &mode)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Lock { iris, reason } => {
            let (db, graph) = setup(&cli.data_dir)?;
            let planner = open_ontologies::plan::Planner::new(db, graph);
            let reason_str = reason.as_deref().unwrap_or("locked");
            for iri in &iris {
                planner.lock_iri(iri, reason_str);
            }
            output_json(
                &serde_json::json!({
                    "ok": true,
                    "locked": iris,
                    "reason": reason_str,
                }),
                cli.pretty,
            );
        }
        Commands::Drift { file_a, file_b } => {
            let (db, _graph) = setup(&cli.data_dir)?;
            let v1 = std::fs::read_to_string(&file_a)?;
            let v2 = std::fs::read_to_string(&file_b)?;
            let detector = open_ontologies::drift::DriftDetector::new(db);
            let result = detector
                .detect(&v1, &v2)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Enforce { pack } => {
            let (db, graph) = setup(&cli.data_dir)?;
            let enforcer = open_ontologies::enforce::Enforcer::new(db.clone(), graph);
            let result = enforcer
                .enforce_with_feedback(&pack, Some(&db))
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Monitor => {
            let (db, graph) = setup(&cli.data_dir)?;
            let monitor = open_ontologies::monitor::Monitor::new(db, graph);
            let result = monitor.run_watchers();
            let json = serde_json::to_string(&result)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&json, cli.pretty);
        }
        Commands::MonitorClear => {
            let (db, graph) = setup(&cli.data_dir)?;
            let monitor = open_ontologies::monitor::Monitor::new(db, graph);
            monitor.clear_blocked();
            output_json(
                &serde_json::json!({"ok": true, "message": "Monitor block cleared"}),
                cli.pretty,
            );
        }
        Commands::Lineage { session } => {
            let (db, _graph) = setup(&cli.data_dir)?;
            let lineage = open_ontologies::lineage::LineageLog::new(db);
            let session_id = session.unwrap_or_else(|| "current".to_string());
            let events = lineage.get_compact(&session_id);
            output_json(
                &serde_json::json!({
                    "session_id": session_id,
                    "events": events.trim(),
                }),
                cli.pretty,
            );
        }

        // ─── Clinical ──────────────────────────────────────────────
        Commands::Crosswalk { code, system } => {
            match open_ontologies::clinical::ClinicalCrosswalks::load("data/crosswalks.parquet") {
                Ok(cw) => {
                    let results = cw.lookup(&code, &system);
                    output_json(
                        &serde_json::json!({
                            "code": code,
                            "system": system,
                            "mappings": results.iter().map(|r| serde_json::json!({
                                "target_code": r.target_code,
                                "target_system": r.target_system,
                                "relation": r.relation,
                                "source_label": r.source_label,
                                "target_label": r.target_label,
                            })).collect::<Vec<_>>(),
                        }),
                        cli.pretty,
                    );
                }
                Err(e) => {
                    output_json(
                        &serde_json::json!({"error": format!("Crosswalks not loaded: {}", e)}),
                        cli.pretty,
                    );
                    std::process::exit(1);
                }
            }
        }
        Commands::Enrich {
            class_iri,
            code,
            system,
        } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            match open_ontologies::clinical::ClinicalCrosswalks::load("data/crosswalks.parquet") {
                Ok(cw) => {
                    let result = cw.enrich(&graph, &class_iri, &code, &system);
                    output_result(&result, cli.pretty);
                }
                Err(e) => {
                    output_json(
                        &serde_json::json!({"error": format!("Crosswalks not loaded: {}", e)}),
                        cli.pretty,
                    );
                    std::process::exit(1);
                }
            }
        }
        Commands::ValidateClinical => {
            let (_db, graph) = setup(&cli.data_dir)?;
            match open_ontologies::clinical::ClinicalCrosswalks::load("data/crosswalks.parquet") {
                Ok(cw) => output_result(&cw.validate_clinical(&graph), cli.pretty),
                Err(e) => {
                    output_json(
                        &serde_json::json!({"error": format!("Crosswalks not loaded: {}", e)}),
                        cli.pretty,
                    );
                    std::process::exit(1);
                }
            }
        }

        // ─── Schema import ─────────────────────────────────────────
        #[allow(unreachable_code, unused_variables)]
        Commands::ImportSchema {
            connection,
            base_iri,
        } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            let driver = match open_ontologies::sqlsource::detect_driver(&connection) {
                Ok(d) => d,
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            };

            let tables: Vec<open_ontologies::schema::TableInfo> = match driver {
                open_ontologies::sqlsource::SqlDriver::Postgres => {
                    #[cfg(feature = "postgres")]
                    {
                        open_ontologies::schema::SchemaIntrospector::introspect_postgres(
                            &connection,
                        )
                        .await?
                    }
                    #[cfg(not(feature = "postgres"))]
                    {
                        output_json(
                            &serde_json::json!({"error": "import-schema for postgres requires the 'postgres' feature (compile with --features postgres)"}),
                            cli.pretty,
                        );
                        std::process::exit(1);
                    }
                }
                open_ontologies::sqlsource::SqlDriver::DuckDb => {
                    #[cfg(feature = "duckdb")]
                    {
                        let target = open_ontologies::sqlsource::duckdb_target(&connection);
                        tokio::task::spawn_blocking(move || {
                            open_ontologies::schema::SchemaIntrospector::introspect_duckdb(&target)
                        })
                        .await??
                    }
                    #[cfg(not(feature = "duckdb"))]
                    {
                        output_json(
                            &serde_json::json!({"error": "import-schema for duckdb requires the 'duckdb' feature (compile with --features duckdb)"}),
                            cli.pretty,
                        );
                        std::process::exit(1);
                    }
                }
            };

            let turtle =
                open_ontologies::schema::SchemaIntrospector::generate_turtle(&tables, &base_iri);

            // Validate + load
            GraphStore::validate_turtle(&turtle)?;
            let count = graph.load_turtle(&turtle, Some(&base_iri))?;

            output_json(
                &serde_json::json!({
                    "ok": true,
                    "driver": driver.as_str(),
                    "tables": tables.len(),
                    "classes": tables.len(),
                    "triples": count,
                    "base_iri": base_iri,
                }),
                cli.pretty,
            );
        }
        Commands::SqlIngest {
            connection,
            sql,
            mapping,
            inline_mapping,
            base_iri,
        } => {
            use open_ontologies::ingest::DataIngester;
            use open_ontologies::mapping::MappingConfig;

            let (_db, graph) = setup(&cli.data_dir)?;

            // Allow stdin via `-`.
            let sql = if sql == "-" {
                use std::io::Read;
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            } else {
                sql
            };

            let driver = match open_ontologies::sqlsource::detect_driver(&connection) {
                Ok(d) => d,
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            };

            let rows = open_ontologies::sqlsource::query_rows(&connection, &sql).await?;

            if rows.is_empty() {
                output_json(
                    &serde_json::json!({
                        "ok": true,
                        "driver": driver.as_str(),
                        "triples_loaded": 0,
                        "rows_processed": 0,
                        "warnings": ["Query returned no rows"],
                    }),
                    cli.pretty,
                );
                return Ok(());
            }

            let mapping_cfg = if let Some(ref m) = mapping {
                if inline_mapping {
                    serde_json::from_str::<MappingConfig>(m)?
                } else {
                    let content = std::fs::read_to_string(m)?;
                    serde_json::from_str::<MappingConfig>(&content)?
                }
            } else {
                let headers = DataIngester::extract_headers(&rows);
                MappingConfig::from_headers(&headers, &base_iri, &format!("{}Thing", base_iri))
            };

            let ntriples = mapping_cfg.rows_to_ntriples(&rows);
            let count = graph.load_ntriples(&ntriples)?;

            output_json(
                &serde_json::json!({
                    "ok": true,
                    "driver": driver.as_str(),
                    "triples_loaded": count,
                    "rows_processed": rows.len(),
                    "mapping_fields": mapping_cfg.mappings.len(),
                }),
                cli.pretty,
            );
        }
        Commands::Align {
            source,
            target,
            min_confidence,
            dry_run,
        } => {
            let (db, graph) = setup(&cli.data_dir)?;
            let source_ttl = std::fs::read_to_string(&source)?;
            let target_ttl = match target {
                Some(ref t) => Some(std::fs::read_to_string(t)?),
                None => None,
            };
            let engine = open_ontologies::align::AlignmentEngine::new(db, graph);
            let result = engine
                .align(&source_ttl, target_ttl.as_deref(), min_confidence, dry_run)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::AlignFeedback {
            source,
            target,
            accept,
            reject,
        } => {
            let (db, graph) = setup(&cli.data_dir)?;
            let engine = open_ontologies::align::AlignmentEngine::new(db, graph);
            let accepted = accept || !reject;
            let result = engine
                .record_feedback(&source, &target, "user_feedback", accepted, None)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::LintFeedback {
            rule_id,
            entity,
            accept,
            dismiss,
        } => {
            let (db, _graph) = setup(&cli.data_dir)?;
            let accepted = accept || !dismiss;
            let result = open_ontologies::feedback::record_tool_feedback(
                &db, "lint", &rule_id, &entity, accepted,
            )
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::EnforceFeedback {
            rule_id,
            entity,
            accept,
            dismiss,
        } => {
            let (db, _graph) = setup(&cli.data_dir)?;
            let accepted = accept || !dismiss;
            let result = open_ontologies::feedback::record_tool_feedback(
                &db, "enforce", &rule_id, &entity, accepted,
            )
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
    }

    Ok(())
}

#[cfg(test)]
mod proxy_serialization_tests {
    use super::*;

    fn args_of(cmd: &Commands) -> (String, Vec<String>) {
        let v = cmd.to_batch_command().expect("command is proxy-able");
        let name = v["command"].as_str().unwrap().to_string();
        let args = v["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap().to_string())
            .collect();
        (name, args)
    }

    #[test]
    fn a_multi_line_query_stays_one_argument() {
        // The bug this replaced: the query was rendered into a command line and
        // the daemon re-tokenized it, but `parse_lines` splits on newlines before
        // it looks at quotes, so a multi-line query — the normal kind — arrived
        // torn across lines and failed as an unterminated quote, while the same
        // query run locally succeeded.
        let query = "SELECT ?s ?o\nWHERE {\n  ?s <http://example.org/p> ?o .\n}";
        let (name, args) = args_of(&Commands::Query { query: query.to_string() });
        assert_eq!(name, "query");
        assert_eq!(args, vec![query.to_string()]);
    }

    #[test]
    fn an_argument_carrying_both_quote_styles_survives_verbatim() {
        // The old double-quote fallback did not escape backslashes, so an
        // argument holding both quote styles came out mangled.
        let query = r#"SELECT ?s WHERE { ?s ?p "it's \"quoted\"" }"#;
        let (_, args) = args_of(&Commands::Query { query: query.to_string() });
        assert_eq!(args, vec![query.to_string()]);
    }

    #[test]
    fn a_relative_path_is_resolved_before_it_leaves_the_caller() {
        // The daemon inherits whatever directory `daemon start` ran in, so a
        // relative path forwarded verbatim resolved against the wrong place.
        let (name, args) = args_of(&Commands::Load { path: "./data/x.ttl".into() });
        assert_eq!(name, "load");
        let sent = std::path::Path::new(&args[0]);
        assert!(sent.is_absolute(), "sent: {}", args[0]);
        assert!(args[0].ends_with("x.ttl"), "sent: {}", args[0]);
        assert!(sent.starts_with(std::env::current_dir().unwrap()));
    }

    #[test]
    fn an_absolute_path_is_left_alone() {
        let abs = if cfg!(windows) { r"C:\onto\proposed.ttl" } else { "/onto/proposed.ttl" };
        let (_, args) = args_of(&Commands::Plan { file: abs.into() });
        assert_eq!(args, vec![abs.to_string()]);
    }

    #[test]
    fn push_serializes_under_the_name_the_batch_runner_answers_to() {
        // `push` was serialized by the proxy with no arm on the other side, so it
        // was the one proxy-able command that could not run while a daemon was up.
        let (name, args) = args_of(&Commands::Push {
            endpoint: "http://example.org/sparql".into(),
            graph: Some("g1".into()),
        });
        assert_eq!(name, "push");
        assert_eq!(args, vec!["http://example.org/sparql", "--graph", "g1"]);
    }

    #[test]
    fn a_flag_the_batch_handler_cannot_honour_keeps_the_command_local() {
        // The batch ingester has no `--format`, so proxying would have dropped it
        // in silence. Running locally honours it.
        let with_format = Commands::Ingest {
            path: "x.csv".into(),
            format: Some("csv".into()),
            mapping: None,
            base_iri: None,
        };
        assert!(with_format.to_batch_command().is_none());

        let without = Commands::Ingest {
            path: "x.csv".into(),
            format: None,
            mapping: None,
            base_iri: None,
        };
        assert!(without.to_batch_command().is_some());
    }

    /// One instance of every variant `to_batch_command` will proxy. If a new
    /// proxy-able command is added and not listed here it simply goes uncovered;
    /// nothing here passes falsely because of it.
    fn every_proxy_able_command() -> Vec<Commands> {
        vec![
            Commands::Load { path: "x.ttl".into() },
            Commands::Save { path: "x.ttl".into(), format: "turtle".into() },
            Commands::Clear,
            Commands::Stats,
            Commands::Query { query: "SELECT ?s WHERE { ?s ?p ?o }".into() },
            Commands::Lint { input: "x.ttl".into() },
            Commands::Reason { profile: "rdfs".into() },
            Commands::Shacl { shapes: "s.ttl".into() },
            Commands::Status,
            Commands::Pull { url: "http://example.org".into(), sparql: false, query: None },
            Commands::Push { endpoint: "http://example.org".into(), graph: None },
            Commands::Version { label: "v1".into() },
            Commands::History,
            Commands::Rollback { label: "v1".into() },
            Commands::Ingest { path: "x.csv".into(), format: None, mapping: None, base_iri: None },
            Commands::Plan { file: "p.ttl".into() },
            Commands::Apply { mode: "safe".into(), plan_id: None },
            Commands::Enforce { pack: "generic".into() },
            Commands::Monitor,
            Commands::MonitorClear,
            Commands::Drift { file_a: "a.ttl".into(), file_b: "b.ttl".into() },
            Commands::Lock { iris: vec!["http://example.org/A".into()], reason: None },
            Commands::Marketplace { action: "list".into(), id: None, domain: None },
        ]
    }

    /// The names on the two sides of the proxy are matched by string across an
    /// HTTP boundary, and nothing but this checks that they agree. `push` was
    /// serialized here with no arm in the batch runner, so it was the one
    /// proxy-able command that could not run while a daemon was up: it fell
    /// through to `unknown batch command` and exited 1.
    ///
    /// Every command is invoked with no arguments, which each arm rejects before
    /// doing anything — the network ones included — so what this asserts is that
    /// the arm exists, not what it does.
    #[tokio::test]
    async fn every_proxy_able_command_has_an_arm_in_the_batch_runner() {
        let dir = tempfile::tempdir().unwrap();
        let db = StateDb::open(&dir.path().join("state.db")).unwrap();
        let runner = open_ontologies::batch::BatchRunner::new(
            db,
            Arc::new(GraphStore::new()),
            false,
        );

        for cmd in every_proxy_able_command() {
            let Some(v) = cmd.to_batch_command() else {
                panic!("expected a proxy-able command");
            };
            let name = v["command"].as_str().unwrap().to_string();
            let payload = serde_json::to_string(&vec![
                serde_json::json!({"command": name, "args": []}),
            ])
            .unwrap();
            let (results, _) = runner.run_collect(&payload, false).await;
            let err = results[0]["result"]["error"].as_str().unwrap_or("");
            assert!(
                !err.contains("unknown batch command"),
                "`{name}` is proxied but the batch runner has no arm for it"
            );
        }
    }

    #[test]
    fn commands_that_must_run_locally_are_not_proxied() {
        assert!(Commands::Diff { old_path: "a.ttl".into(), new_path: "b.ttl".into() }
            .to_batch_command()
            .is_none());
    }
}
