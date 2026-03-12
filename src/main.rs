use clap::{Parser, Subcommand};
use rmcp::ServiceExt;
use std::sync::Arc;

use open_ontologies::config::{expand_tilde, Config};
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;

const DEFAULT_CONFIG: &str = r#"[general]
data_dir = "~/.open-ontologies"
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
    },
    /// Start the MCP server
    Serve {
        #[arg(long, default_value = "~/.open-ontologies/config.toml")]
        config: String,
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { data_dir } => {
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

            println!("\nOpen Ontologies initialized successfully!");
        }
        Commands::Serve { config: config_path } => {
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

            let server = OpenOntologiesServer::new(db);
            let service = server.serve(rmcp::transport::stdio()).await?;
            service.waiting().await?;
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
            if cli.pretty {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&stats_json) {
                    println!("{}", serde_json::to_string_pretty(&v).unwrap());
                } else {
                    println!("{}", stats_json);
                }
            } else {
                println!("{}", stats_json);
            }
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
            if cli.pretty {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&result) {
                    println!("{}", serde_json::to_string_pretty(&v).unwrap());
                } else {
                    println!("{}", result);
                }
            } else {
                println!("{}", result);
            }
        }
        Commands::Diff { old_path, new_path } => {
            use open_ontologies::ontology::OntologyService;
            let old = std::fs::read_to_string(&old_path)?;
            let new = std::fs::read_to_string(&new_path)?;
            let result = OntologyService::diff(&old, &new).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            if cli.pretty {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&result) {
                    println!("{}", serde_json::to_string_pretty(&v).unwrap());
                } else {
                    println!("{}", result);
                }
            } else {
                println!("{}", result);
            }
        }
        Commands::Lint { input } => {
            use open_ontologies::ontology::OntologyService;
            let content = if input == "-" {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                buf
            } else {
                std::fs::read_to_string(&input)?
            };
            let result = OntologyService::lint(&content).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
            if cli.pretty {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&result) {
                    println!("{}", serde_json::to_string_pretty(&v).unwrap());
                } else {
                    println!("{}", result);
                }
            } else {
                println!("{}", result);
            }
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

        // ─── Stub remaining subcommands ───────────────────────────
        _ => {
            output_json(&serde_json::json!({"error": "not implemented"}), cli.pretty);
            std::process::exit(1);
        }
    }

    Ok(())
}
