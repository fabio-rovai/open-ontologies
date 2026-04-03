use clap::{Parser, Subcommand};
use rmcp::ServiceExt;
use std::sync::Arc;

use open_ontologies::config::{expand_tilde, Config};
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;

const DEFAULT_CONFIG: &str = r#"[general]
data_dir = "~/.open-ontologies"

# [embeddings]
# model_path = "~/.open-ontologies/models/bge-small-en-v1.5.onnx"
# tokenizer_path = "~/.open-ontologies/models/tokenizer.json"
"#;

#[derive(Parser)]
#[command(name = "open-ontologies", about = "Terraform for Knowledge Graphs — AI-native ontology engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Pretty-print JSON output
    #[arg(long, global = true)]
    pretty: bool,

    /// Data directory (default: ~/.open-ontologies)
    #[arg(long, global = true, default_value = "~/.open-ontologies")]
    data_dir: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize data directory, DB, and default config
    Init {
        #[arg(long, default_value = "~/.open-ontologies")]
        data_dir: String,
        /// Custom ONNX model URL (default: BGE-small-en-v1.5 from Hugging Face)
        #[arg(long)]
        model_url: Option<String>,
        /// Custom tokenizer URL (default: BGE-small-en-v1.5 tokenizer from Hugging Face)
        #[arg(long)]
        tokenizer_url: Option<String>,
        /// Filename for the downloaded ONNX model (default: bge-small-en-v1.5.onnx)
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
    },
    /// Start the MCP server (Streamable HTTP transport)
    ServeHttp {
        #[arg(long, default_value = "~/.open-ontologies/config.toml")]
        config: String,
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Port to bind to
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Optional bearer token for authentication
        #[arg(long, env = "OPEN_ONTOLOGIES_TOKEN")]
        token: Option<String>,
        /// Optional governance webhook URL (fires on every lineage event)
        #[arg(long, env = "GOVERNANCE_WEBHOOK")]
        governance_webhook: Option<String>,
    },

    /// Start unix socket server for Tardygrada fact grounding
    ServeUnix {
        /// Path to the unix socket
        #[arg(long, default_value = "/tmp/tardygrada-ontology-complete.sock")]
        socket: String,
        /// Ontology files to load on startup
        #[arg(long = "file", num_args = 1..)]
        files: Vec<String>,
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
    Diff {
        old_path: String,
        new_path: String,
    },
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
    },
    /// Lock IRIs to prevent removal
    Lock {
        iris: Vec<String>,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Detect drift between two ontology versions
    Drift {
        file_a: String,
        file_b: String,
    },
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
    /// Import database schema as OWL ontology
    ImportSchema {
        /// Connection string (e.g. postgres://user:pass@host/db)
        connection: String,
        #[arg(long, default_value = "http://example.org/db/")]
        base_iri: String,
    },
}

fn setup(data_dir: &str) -> anyhow::Result<(StateDb, Arc<GraphStore>)> {
    let data_dir = expand_tilde(data_dir);
    let data_path = std::path::Path::new(&data_dir);
    std::fs::create_dir_all(data_path)?;
    let db_path = data_path.join("open-ontologies.db");
    let db = StateDb::open(&db_path)?;
    let graph = Arc::new(GraphStore::new());
    Ok((db, graph))
}

fn output_json(value: &serde_json::Value, pretty: bool) {
    if pretty {
        println!("{}", serde_json::to_string_pretty(value).unwrap());
    } else {
        println!("{}", value);
    }
}

/// Print a JSON string result, with optional pretty-printing.
/// Handles the common pattern of domain functions returning String results.
fn output_result(result: &str, pretty: bool) {
    if pretty {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(result) {
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
        } else {
            println!("{}", result);
        }
    } else {
        println!("{}", result);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { data_dir, model_url: _model_url, tokenizer_url: _tokenizer_url, model_name: _model_name } => {
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
                std::fs::create_dir_all(&models_dir)?;

                let onnx_url = _model_url.as_deref()
                    .unwrap_or(open_ontologies::embed::BGE_SMALL_ONNX_URL);
                let tok_url = _tokenizer_url.as_deref()
                    .unwrap_or(open_ontologies::embed::BGE_SMALL_TOKENIZER_URL);
                let onnx_filename = _model_name.as_deref()
                    .unwrap_or("bge-small-en-v1.5.onnx");

                let model_path = models_dir.join(onnx_filename);
                let tokenizer_path = models_dir.join("tokenizer.json");

                if !model_path.exists() {
                    println!("Downloading embedding model from {}...", onnx_url);
                    open_ontologies::embed::download_model_file(
                        onnx_url,
                        &model_path,
                    ).await?;
                    println!("  Model saved: {}", model_path.display());
                } else {
                    println!("Embedding model already exists: {}", model_path.display());
                }

                if !tokenizer_path.exists() {
                    println!("Downloading tokenizer from {}...", tok_url);
                    open_ontologies::embed::download_model_file(
                        tok_url,
                        &tokenizer_path,
                    ).await?;
                    println!("  Tokenizer saved: {}", tokenizer_path.display());
                } else {
                    println!("Tokenizer already exists: {}", tokenizer_path.display());
                }
            }

            println!("\nOpen Ontologies initialized successfully!");
        }
        Commands::Serve { config: config_path, governance_webhook } => {
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
            let data_dir = expand_tilde(&cfg.general.data_dir);
            let db_path = std::path::Path::new(&data_dir).join("open-ontologies.db");

            std::fs::create_dir_all(&data_dir)?;
            let db = StateDb::open(&db_path)?;

            let server = OpenOntologiesServer::new_with_full_options(db, Arc::new(GraphStore::new()), governance_webhook, cfg.embeddings);
            let service = server.serve(rmcp::transport::stdio()).await?;
            service.waiting().await?;
        }
        Commands::ServeHttp { config: config_path, host, port, token, governance_webhook } => {
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
            let data_dir = expand_tilde(&cfg.general.data_dir);
            let db_path_owned = std::path::Path::new(&data_dir).join("open-ontologies.db");

            std::fs::create_dir_all(&data_dir)?;

            // Shared graph store — all MCP sessions (agent + frontend) see the same triples
            let shared_graph = Arc::new(GraphStore::new());

            // Shared StateDb for lineage REST endpoint
            let shared_db = StateDb::open(&db_path_owned)?;

            let ct = CancellationToken::new();
            let http_config = StreamableHttpServerConfig {
                stateful_mode: true,
                cancellation_token: ct.clone(),
                ..Default::default()
            };

            let shared_graph_for_service = shared_graph.clone();
            let gw_for_service = governance_webhook.clone();
            let embed_config = cfg.embeddings.clone();
            let service: StreamableHttpService<_, LocalSessionManager> =
                StreamableHttpService::new(
                    move || {
                        let db = StateDb::open(&db_path_owned)
                            .map_err(std::io::Error::other)?;
                        Ok(OpenOntologiesServer::new_with_full_options(db, shared_graph_for_service.clone(), gw_for_service.clone(), embed_config.clone()))
                    },
                    Default::default(),
                    http_config,
                );

            // Simple REST API — no MCP sessions, direct access to shared graph
            let sg_stats  = shared_graph.clone();
            let sg_query  = shared_graph.clone();
            let sg_update = shared_graph.clone();
            let sg_load         = shared_graph.clone();
            let sg_save         = shared_graph.clone();
            let sg_load_turtle  = shared_graph.clone();
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
                router.layer(axum::middleware::from_fn(move |req: axum::extract::Request, next: axum::middleware::Next| {
                    let expected = expected.clone();
                    async move {
                        let auth = req.headers().get("authorization").and_then(|v| v.to_str().ok());
                        if auth == Some(&expected) {
                            next.run(req).await
                        } else {
                            axum::http::Response::builder()
                                .status(401)
                                .body(axum::body::Body::from("Unauthorized"))
                                .unwrap()
                        }
                    }
                }))
            } else {
                router
            };
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

        Commands::ServeUnix { socket, files } => {
            let graph = Arc::new(GraphStore::new());
            for f in &files {
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
            open_ontologies::socket::serve(&socket, graph).await?;
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
                Ok(count) => output_json(&serde_json::json!({"ok": true, "triples": count}), cli.pretty),
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::Load { path } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            match graph.load_file(&path) {
                Ok(count) => output_json(&serde_json::json!({"ok": true, "triples_loaded": count, "path": path}), cli.pretty),
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::Save { path, format } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            match graph.save_file(&path, &format) {
                Ok(_) => output_json(&serde_json::json!({"ok": true, "path": path, "format": format}), cli.pretty),
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::Clear => {
            let (_db, graph) = setup(&cli.data_dir)?;
            match graph.clear() {
                Ok(_) => output_json(&serde_json::json!({"ok": true, "message": "Store cleared"}), cli.pretty),
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::Stats => {
            let (_db, graph) = setup(&cli.data_dir)?;
            let stats_json = graph.get_stats().unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
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
            let result = graph.sparql_select(&query_str).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Diff { old_path, new_path } => {
            use open_ontologies::ontology::OntologyService;
            let old = std::fs::read_to_string(&old_path)?;
            let new = std::fs::read_to_string(&new_path)?;
            let result = OntologyService::diff(&old, &new).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Lint { input } => {
            use open_ontologies::ontology::OntologyService;
            let (db, _graph) = setup(&cli.data_dir)?;
            let content = if input == "-" {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                buf
            } else {
                std::fs::read_to_string(&input)?
            };
            let result = OntologyService::lint_with_feedback(&content, Some(&db)).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Convert { path, to, output } => {
            let store = GraphStore::new();
            match store.load_file(&path) {
                Ok(_) => {
                    match store.serialize(&to) {
                        Ok(content) => {
                            if let Some(out_path) = output {
                                std::fs::write(&out_path, &content)?;
                                output_json(&serde_json::json!({"ok": true, "path": out_path, "format": to}), cli.pretty);
                            } else {
                                println!("{}", content);
                            }
                        }
                        Err(e) => {
                            output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    output_json(&serde_json::json!({"error": e.to_string()}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::Status => {
            let (_db, graph) = setup(&cli.data_dir)?;
            output_json(&serde_json::json!({
                "status": "ok",
                "version": env!("CARGO_PKG_VERSION"),
                "triples_loaded": graph.triple_count(),
            }), cli.pretty);
        }

        // ─── Remote ─────────────────────────────────────────────────
        Commands::Marketplace { action, id, domain } => {
            use open_ontologies::marketplace;
            match action.as_str() {
                "list" => {
                    let entries = marketplace::list(domain.as_deref());
                    let items: Vec<serde_json::Value> = entries.iter().map(|e| {
                        serde_json::json!({
                            "id": e.id,
                            "name": e.name,
                            "description": e.description,
                            "domain": e.domain,
                            "format": marketplace::format_name(e.format),
                        })
                    }).collect();
                    output_json(&serde_json::json!({
                        "count": items.len(),
                        "ontologies": items,
                    }), cli.pretty);
                }
                "install" => {
                    let id = id.as_deref().unwrap_or_else(|| {
                        eprintln!("Error: --id is required for install");
                        std::process::exit(1);
                    });
                    let entry = match marketplace::find(id) {
                        Some(e) => e,
                        None => {
                            eprintln!("Unknown ontology ID: '{}'. Run 'marketplace list' to see available IDs.", id);
                            std::process::exit(1);
                        }
                    };
                    let (_db, graph) = setup(&cli.data_dir)?;
                    let content = GraphStore::fetch_url(entry.url).await?;
                    match graph.load_content_with_base(&content, entry.format, Some(entry.url)) {
                        Ok(count) => {
                            let stats = graph.get_stats().unwrap_or_default();
                            output_json(&serde_json::json!({
                                "ok": true,
                                "installed": entry.id,
                                "name": entry.name,
                                "triples_loaded": count,
                                "stats": serde_json::from_str::<serde_json::Value>(&stats).unwrap_or_default(),
                            }), cli.pretty);
                        }
                        Err(e) => {
                            output_json(&serde_json::json!({"error": format!("Parse error: {}", e)}), cli.pretty);
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
                let q = query.as_deref().unwrap_or("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }");
                GraphStore::fetch_sparql(&url, q).await?
            } else {
                GraphStore::fetch_url(&url).await?
            };
            match graph.load_turtle(&content, None) {
                Ok(count) => output_json(&serde_json::json!({"ok": true, "triples_loaded": count, "source": url}), cli.pretty),
                Err(e) => {
                    output_json(&serde_json::json!({"error": format!("Parse error: {}", e)}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::Push { endpoint, graph: graph_name } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            let content = graph.serialize("ntriples")?;
            match GraphStore::push_sparql(&endpoint, &content).await {
                Ok(msg) => output_json(&serde_json::json!({"ok": true, "message": msg}), cli.pretty),
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

            let query = "SELECT ?import WHERE { ?onto <http://www.w3.org/2002/07/owl#imports> ?import }";
            if let Ok(result) = graph.sparql_select(query)
                && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result)
                    && let Some(results) = parsed["results"].as_array() {
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
                    if imported.contains(&url) { continue; }
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

            output_json(&serde_json::json!({"ok": true, "imported": imported.len(), "urls": imported}), cli.pretty);
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
        Commands::Map { data_path, format: _format, save } => {
            use open_ontologies::ingest::DataIngester;
            use open_ontologies::mapping::MappingConfig;
            let (_db, graph) = setup(&cli.data_dir)?;

            let rows = DataIngester::parse_file(&data_path)?;
            let headers = DataIngester::extract_headers(&rows);

            let classes_query = r#"SELECT DISTINCT ?c WHERE { { ?c a <http://www.w3.org/2002/07/owl#Class> } UNION { ?c a <http://www.w3.org/2000/01/rdf-schema#Class> } }"#;
            let props_query = r#"SELECT DISTINCT ?p WHERE { { ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> } UNION { ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> } UNION { ?p a <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property> } }"#;

            let classes = graph.sparql_select(classes_query).unwrap_or_default();
            let props = graph.sparql_select(props_query).unwrap_or_default();

            let mapping = MappingConfig::from_headers(&headers, "http://example.org/data/", "http://example.org/data/Thing");
            let mapping_json = serde_json::to_string_pretty(&mapping).unwrap_or_default();

            if let Some(save_path) = save {
                std::fs::write(&save_path, &mapping_json)?;
                output_json(&serde_json::json!({"ok": true, "saved": save_path}), cli.pretty);
            } else {
                let extract_iris = |json: &str, var: &str| -> Vec<String> {
                    serde_json::from_str::<serde_json::Value>(json)
                        .ok()
                        .and_then(|v| v["results"].as_array().cloned())
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|r| r[var].as_str().map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string()))
                        .collect()
                };
                output_json(&serde_json::json!({
                    "data_fields": headers,
                    "ontology_classes": extract_iris(&classes, "c"),
                    "ontology_properties": extract_iris(&props, "p"),
                    "suggested_mapping": serde_json::from_str::<serde_json::Value>(&mapping_json).unwrap_or_default(),
                }), cli.pretty);
            }
        }
        Commands::Ingest { path, format: _format, mapping, base_iri } => {
            use open_ontologies::ingest::DataIngester;
            use open_ontologies::mapping::MappingConfig;
            let (_db, graph) = setup(&cli.data_dir)?;

            let base = base_iri.as_deref().unwrap_or("http://example.org/data/");
            let rows = DataIngester::parse_file(&path)?;

            if rows.is_empty() {
                output_json(&serde_json::json!({"ok": true, "triples_loaded": 0, "warnings": ["No data rows found"]}), cli.pretty);
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
                    Ok(count) => output_json(&serde_json::json!({"ok": true, "triples_loaded": count, "rows": rows.len()}), cli.pretty),
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
        Commands::Reason { profile } => {
            use open_ontologies::reason::Reasoner;
            let (_db, graph) = setup(&cli.data_dir)?;
            let result = Reasoner::run(&graph, &profile, true)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Extend { data_path, format: _format, mapping, shapes, profile } => {
            use open_ontologies::ingest::DataIngester;
            use open_ontologies::mapping::MappingConfig;
            use open_ontologies::shacl::ShaclValidator;
            use open_ontologies::reason::Reasoner;
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
                Some(ShaclValidator::validate(&graph, &shapes_content)
                    .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e)))
            } else {
                None
            };

            // 3. Reason (optional)
            let reason_result = profile.as_ref().map(|prof| Reasoner::run(&graph, prof, true)
                    .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e)));

            output_json(&serde_json::json!({
                "ok": true,
                "triples_loaded": triples_loaded,
                "rows": rows.len(),
                "shacl": shacl_result.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
                "reason": reason_result.and_then(|r| serde_json::from_str::<serde_json::Value>(&r).ok()),
            }), cli.pretty);
        }

        // ─── Lifecycle ──────────────────────────────────────────────
        Commands::Plan { file } => {
            let (db, graph) = setup(&cli.data_dir)?;
            let turtle = std::fs::read_to_string(&file)?;
            let planner = open_ontologies::plan::Planner::new(db, graph);
            let result = planner.plan(&turtle)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Apply { mode } => {
            let (db, graph) = setup(&cli.data_dir)?;
            let planner = open_ontologies::plan::Planner::new(db, graph);
            let result = planner.apply(&mode)
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
            output_json(&serde_json::json!({
                "ok": true,
                "locked": iris,
                "reason": reason_str,
            }), cli.pretty);
        }
        Commands::Drift { file_a, file_b } => {
            let (db, _graph) = setup(&cli.data_dir)?;
            let v1 = std::fs::read_to_string(&file_a)?;
            let v2 = std::fs::read_to_string(&file_b)?;
            let detector = open_ontologies::drift::DriftDetector::new(db);
            let result = detector.detect(&v1, &v2)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::Enforce { pack } => {
            let (db, graph) = setup(&cli.data_dir)?;
            let enforcer = open_ontologies::enforce::Enforcer::new(db.clone(), graph);
            let result = enforcer.enforce_with_feedback(&pack, Some(&db))
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
            output_json(&serde_json::json!({"ok": true, "message": "Monitor block cleared"}), cli.pretty);
        }
        Commands::Lineage { session } => {
            let (db, _graph) = setup(&cli.data_dir)?;
            let lineage = open_ontologies::lineage::LineageLog::new(db);
            let session_id = session.unwrap_or_else(|| "current".to_string());
            let events = lineage.get_compact(&session_id);
            output_json(&serde_json::json!({
                "session_id": session_id,
                "events": events.trim(),
            }), cli.pretty);
        }

        // ─── Clinical ──────────────────────────────────────────────
        Commands::Crosswalk { code, system } => {
            match open_ontologies::clinical::ClinicalCrosswalks::load("data/crosswalks.parquet") {
                Ok(cw) => {
                    let results = cw.lookup(&code, &system);
                    output_json(&serde_json::json!({
                        "code": code,
                        "system": system,
                        "mappings": results.iter().map(|r| serde_json::json!({
                            "target_code": r.target_code,
                            "target_system": r.target_system,
                            "relation": r.relation,
                            "source_label": r.source_label,
                            "target_label": r.target_label,
                        })).collect::<Vec<_>>(),
                    }), cli.pretty);
                }
                Err(e) => {
                    output_json(&serde_json::json!({"error": format!("Crosswalks not loaded: {}", e)}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::Enrich { class_iri, code, system } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            match open_ontologies::clinical::ClinicalCrosswalks::load("data/crosswalks.parquet") {
                Ok(cw) => {
                    let result = cw.enrich(&graph, &class_iri, &code, &system);
                    output_result(&result, cli.pretty);
                }
                Err(e) => {
                    output_json(&serde_json::json!({"error": format!("Crosswalks not loaded: {}", e)}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }
        Commands::ValidateClinical => {
            let (_db, graph) = setup(&cli.data_dir)?;
            match open_ontologies::clinical::ClinicalCrosswalks::load("data/crosswalks.parquet") {
                Ok(cw) => output_result(&cw.validate_clinical(&graph), cli.pretty),
                Err(e) => {
                    output_json(&serde_json::json!({"error": format!("Crosswalks not loaded: {}", e)}), cli.pretty);
                    std::process::exit(1);
                }
            }
        }

        // ─── Schema import ─────────────────────────────────────────
        #[cfg(feature = "postgres")]
        Commands::ImportSchema { connection, base_iri } => {
            let (_db, graph) = setup(&cli.data_dir)?;
            let tables = open_ontologies::schema::SchemaIntrospector::introspect_postgres(&connection).await?;
            let turtle = open_ontologies::schema::SchemaIntrospector::generate_turtle(&tables, &base_iri);

            // Validate + load
            GraphStore::validate_turtle(&turtle)?;
            let count = graph.load_turtle(&turtle, Some(&base_iri))?;

            output_json(&serde_json::json!({
                "ok": true,
                "tables": tables.len(),
                "classes": tables.len(),
                "triples": count,
                "base_iri": base_iri,
            }), cli.pretty);
        }
        #[cfg(not(feature = "postgres"))]
        Commands::ImportSchema { .. } => {
            output_json(&serde_json::json!({"error": "import-schema requires the 'postgres' feature (compile with --features postgres)"}), cli.pretty);
            std::process::exit(1);
        }
        Commands::Align { source, target, min_confidence, dry_run } => {
            let (db, graph) = setup(&cli.data_dir)?;
            let source_ttl = std::fs::read_to_string(&source)?;
            let target_ttl = match target {
                Some(ref t) => Some(std::fs::read_to_string(t)?),
                None => None,
            };
            let engine = open_ontologies::align::AlignmentEngine::new(db, graph);
            let result = engine.align(&source_ttl, target_ttl.as_deref(), min_confidence, dry_run)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::AlignFeedback { source, target, accept, reject } => {
            let (db, graph) = setup(&cli.data_dir)?;
            let engine = open_ontologies::align::AlignmentEngine::new(db, graph);
            let accepted = accept || !reject;
            let result = engine.record_feedback(&source, &target, "user_feedback", accepted)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::LintFeedback { rule_id, entity, accept, dismiss } => {
            let (db, _graph) = setup(&cli.data_dir)?;
            let accepted = accept || !dismiss;
            let result = open_ontologies::feedback::record_tool_feedback(&db, "lint", &rule_id, &entity, accepted)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
        Commands::EnforceFeedback { rule_id, entity, accept, dismiss } => {
            let (db, _graph) = setup(&cli.data_dir)?;
            let accepted = accept || !dismiss;
            let result = open_ontologies::feedback::record_tool_feedback(&db, "enforce", &rule_id, &entity, accepted)
                .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            output_result(&result, cli.pretty);
        }
    }

    Ok(())
}
