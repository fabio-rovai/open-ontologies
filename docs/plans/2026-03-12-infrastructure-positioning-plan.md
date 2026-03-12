# Infrastructure Positioning Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform Open Ontologies from a Claude Code-only MCP server into production infrastructure — CLI, SQL→OWL import, real benchmarks, CI, README rewrite.

**Architecture:** Extend existing `clap` binary with one subcommand per MCP tool. Each subcommand initializes `StateDb` + `GraphStore`, calls the same library functions the MCP handlers use, prints JSON to stdout. New `schema.rs` module for database introspection. Benchmark harness in shell + Java + Python (no Rust recompilation).

**Tech Stack:** Rust (clap CLI), sqlx (postgres), Java (OWL API + HermiT + Pellet), Python (matplotlib for charts), GitHub Actions (CI), Docker (benchmark postgres).

---

### Task 1: Refactor main.rs for CLI subcommand structure

The current `main.rs` has only `Init` and `Serve` in the `Commands` enum. We need to add ~35 subcommands. To keep it clean, group them into sub-enums.

**Files:**
- Modify: `src/main.rs`

**Step 1: Write a CLI integration test**

Create `tests/cli_test.rs`:

```rust
use std::process::Command;

fn oo() -> Command {
    Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
}

#[test]
fn test_cli_help() {
    let out = oo().arg("--help").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("validate"));
    assert!(stdout.contains("query"));
    assert!(stdout.contains("import-schema"));
}

#[test]
fn test_cli_validate_inline_stdin() {
    let out = oo()
        .args(["validate", "-"])
        .stdin(std::process::Stdio::piped())
        .output()
        .unwrap();
    // Will fail until subcommand exists
    assert!(!out.status.success() || String::from_utf8_lossy(&out.stdout).contains("error"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test cli_test test_cli_help -- --nocapture`
Expected: FAIL — `validate` not in help output.

**Step 3: Restructure main.rs with all subcommands**

Replace the `Commands` enum in `src/main.rs`:

```rust
use clap::{Parser, Subcommand};
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
```

Add a shared setup helper and stub the `match` arms (just print `{"error":"not implemented"}` for now — we'll fill them in Tasks 2–6):

```rust
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test cli_test -- --nocapture`
Expected: PASS — help output contains `validate`, `query`, `import-schema`.

**Step 5: Commit**

```bash
git add src/main.rs tests/cli_test.rs
git commit -m "feat: restructure CLI with all subcommand definitions"
```

---

### Task 2: Implement core ontology CLI subcommands

10 subcommands: validate, load, save, clear, stats, query, diff, lint, convert, status. Each reads stdin or file, calls the library, prints JSON.

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/cli_test.rs`

**Step 1: Write CLI tests for core subcommands**

Add to `tests/cli_test.rs`:

```rust
#[test]
fn test_cli_validate_file() {
    let dir = tempfile::tempdir().unwrap();
    let ttl_path = dir.path().join("test.ttl");
    std::fs::write(&ttl_path, r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#).unwrap();

    let out = oo()
        .args(["validate", ttl_path.to_str().unwrap()])
        .output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("triples"));
}

#[test]
fn test_cli_validate_stdin() {
    use std::io::Write;
    let mut child = oo()
        .args(["validate", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn().unwrap();

    child.stdin.take().unwrap().write_all(b"@prefix ex: <http://example.org/> . ex:Dog a <http://www.w3.org/2002/07/owl#Class> .").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
}

#[test]
fn test_cli_stats_empty() {
    let out = oo().arg("stats").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("classes"));
}

#[test]
fn test_cli_clear() {
    let out = oo().arg("clear").output().unwrap();
    assert!(out.status.success());
}

#[test]
fn test_cli_status() {
    let out = oo().arg("status").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test cli_test -- --nocapture`
Expected: FAIL — subcommands return `not implemented`.

**Step 3: Implement core subcommand handlers in main.rs**

Fill in the `match` arms. Each one follows the same pattern: `setup()` → call library → `output_json()`.

For `validate`: detect `-` for stdin, read stdin to string, call `GraphStore::validate_turtle()` or `GraphStore::validate_file()`.

For `query`: detect `-` for stdin, read query string, call `graph.sparql_select()`.

For `load`/`save`/`clear`/`stats`/`diff`/`lint`/`convert`/`status`: direct library calls matching what `server.rs` does.

Key pattern (example for validate):
```rust
Commands::Validate { input } => {
    let (_db, graph) = setup(&cli.data_dir)?;
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
```

Follow this pattern for all 10 subcommands. Reference server.rs handlers for the exact library calls each needs.

**Step 4: Run tests to verify they pass**

Run: `cargo test --test cli_test -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add src/main.rs tests/cli_test.rs
git commit -m "feat: implement core ontology CLI subcommands (validate, load, save, clear, stats, query, diff, lint, convert, status)"
```

---

### Task 3: Implement remote + versioning CLI subcommands

6 subcommands: pull, push, import-owl, version, history, rollback.

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/cli_test.rs`

**Step 1: Write tests**

Add to `tests/cli_test.rs`:

```rust
#[test]
fn test_cli_history_empty() {
    let out = oo().arg("history").output().unwrap();
    assert!(out.status.success());
}

#[test]
fn test_cli_version_and_rollback() {
    // Load, version, rollback are stateful — each CLI invocation creates fresh state
    // This test verifies the subcommands accept args and return valid JSON
    let out = oo().args(["version", "test-v1"]).output().unwrap();
    assert!(out.status.success());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test cli_test test_cli_history -- --nocapture`
Expected: FAIL.

**Step 3: Implement handlers**

For `pull`/`push`: these are async (reqwest). The `#[tokio::main]` already exists. Call `GraphStore::fetch_url()` / `GraphStore::push_sparql()`.

For `version`/`history`/`rollback`: call `OntologyService::save_version()` / `list_versions()` / `rollback_version()`. Note: these need `OntologyService` which wraps `StateDb` + `GraphStore` — import and construct it.

For `import-owl`: call `graph.sparql_select()` to get imports, then `GraphStore::fetch_url()` + `graph.load_turtle()` in a loop (same as server.rs handler).

**Step 4: Run tests to verify they pass**

Run: `cargo test --test cli_test -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add src/main.rs tests/cli_test.rs
git commit -m "feat: implement remote + versioning CLI subcommands (pull, push, import-owl, version, history, rollback)"
```

---

### Task 4: Implement data pipeline CLI subcommands

5 subcommands: map, ingest, shacl, reason, extend.

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/cli_test.rs`

**Step 1: Write tests**

Add to `tests/cli_test.rs`:

```rust
#[test]
fn test_cli_reason_empty_store() {
    let out = oo().args(["reason", "--profile", "rdfs"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("inferred") || stdout.contains("triples"));
}

#[test]
fn test_cli_ingest_csv() {
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("data.csv");
    std::fs::write(&csv_path, "name,age\nAlice,30\nBob,25").unwrap();

    let out = oo()
        .args(["ingest", csv_path.to_str().unwrap()])
        .output().unwrap();
    assert!(out.status.success());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test cli_test test_cli_reason -- --nocapture`
Expected: FAIL.

**Step 3: Implement handlers**

For `ingest`: parse mapping from `--mapping` arg (file path to JSON), call `DataIngester::parse_file()`, `MappingConfig`, `rows_to_ntriples()`, `graph.load_ntriples()`.

For `reason`: call `Reasoner::run()` with profile.

For `shacl`: read shapes file, call `ShaclValidator::validate()`.

For `map`/`extend`: follow server.rs pattern.

**Step 4: Run tests to verify they pass**

Run: `cargo test --test cli_test -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add src/main.rs tests/cli_test.rs
git commit -m "feat: implement data pipeline CLI subcommands (map, ingest, shacl, reason, extend)"
```

---

### Task 5: Implement lifecycle + clinical CLI subcommands

11 subcommands: plan, apply, lock, drift, enforce, monitor, monitor-clear, lineage, crosswalk, enrich, validate-clinical.

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/cli_test.rs`

**Step 1: Write tests**

Add to `tests/cli_test.rs`:

```rust
#[test]
fn test_cli_enforce_generic() {
    let out = oo().args(["enforce", "generic"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("compliance") || stdout.contains("violations"));
}

#[test]
fn test_cli_plan() {
    let dir = tempfile::tempdir().unwrap();
    let ttl_path = dir.path().join("new.ttl");
    std::fs::write(&ttl_path, r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#).unwrap();

    let out = oo().args(["plan", ttl_path.to_str().unwrap()]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("risk_score") || stdout.contains("added"));
}

#[test]
fn test_cli_drift() {
    let dir = tempfile::tempdir().unwrap();
    let v1 = dir.path().join("v1.ttl");
    let v2 = dir.path().join("v2.ttl");
    std::fs::write(&v1, r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#).unwrap();
    std::fs::write(&v2, r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
    "#).unwrap();

    let out = oo().args(["drift", v1.to_str().unwrap(), v2.to_str().unwrap()]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("drift_velocity"));
}

#[test]
fn test_cli_lineage() {
    let out = oo().arg("lineage").output().unwrap();
    assert!(out.status.success());
}

#[test]
fn test_cli_monitor_clear() {
    let out = oo().arg("monitor-clear").output().unwrap();
    assert!(out.status.success());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test cli_test test_cli_enforce -- --nocapture`
Expected: FAIL.

**Step 3: Implement handlers**

For `plan`: read Turtle file, call `Planner::new()`, `.plan()`.
For `apply`: call `Planner::new()`, `.apply(mode)`.
For `lock`: call `Planner::new()`, `.lock_iri()` per IRI.
For `drift`: read both files, call `DriftDetector::new()`, `.detect()`.
For `enforce`: call `Enforcer::new()`, `.enforce(pack)`.
For `monitor`/`monitor-clear`: call `Monitor::new()`, `.run_watchers()` / `.clear_blocked()`.
For `lineage`: call `LineageLog::new()`, `.get_compact()`.
For `crosswalk`/`enrich`/`validate-clinical`: call `ClinicalCrosswalks::load()` methods.

**Step 4: Run tests to verify they pass**

Run: `cargo test --test cli_test -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add src/main.rs tests/cli_test.rs
git commit -m "feat: implement lifecycle + clinical CLI subcommands"
```

---

### Task 6: SQL schema import module

New module `src/schema.rs` — connects to postgres, introspects schema, generates OWL Turtle.

**Files:**
- Create: `src/schema.rs`
- Modify: `src/lib.rs` (add `pub mod schema;`)
- Modify: `Cargo.toml` (add `sqlx` dependency)
- Create: `tests/schema_test.rs`

**Step 1: Add sqlx dependency**

Add to `Cargo.toml`:
```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres"], optional = true }
```

Add a feature flag:
```toml
[features]
default = ["postgres"]
postgres = ["sqlx"]
```

**Step 2: Write unit tests (no live database)**

Create `tests/schema_test.rs`. Test the Turtle generation from a mock schema struct (no actual postgres connection needed for unit tests):

```rust
use open_ontologies::schema::{TableInfo, ColumnInfo, ForeignKey, SchemaIntrospector};

#[test]
fn test_generate_turtle_single_table() {
    let tables = vec![TableInfo {
        name: "users".into(),
        columns: vec![
            ColumnInfo { name: "id".into(), data_type: "integer".into(), is_nullable: false, is_primary_key: true },
            ColumnInfo { name: "name".into(), data_type: "varchar".into(), is_nullable: false, is_primary_key: false },
            ColumnInfo { name: "email".into(), data_type: "varchar".into(), is_nullable: true, is_primary_key: false },
        ],
        foreign_keys: vec![],
    }];

    let turtle = SchemaIntrospector::generate_turtle(&tables, "http://example.org/db/");
    assert!(turtle.contains("db:Users a owl:Class"));
    assert!(turtle.contains("db:users_name a owl:DatatypeProperty"));
    assert!(turtle.contains("xsd:string"));
    assert!(turtle.contains("owl:minCardinality"));  // NOT NULL → minCard 1
    assert!(turtle.contains("owl:FunctionalProperty"));  // PK
}

#[test]
fn test_generate_turtle_foreign_key() {
    let tables = vec![
        TableInfo {
            name: "users".into(),
            columns: vec![
                ColumnInfo { name: "id".into(), data_type: "integer".into(), is_nullable: false, is_primary_key: true },
            ],
            foreign_keys: vec![],
        },
        TableInfo {
            name: "orders".into(),
            columns: vec![
                ColumnInfo { name: "id".into(), data_type: "integer".into(), is_nullable: false, is_primary_key: true },
                ColumnInfo { name: "user_id".into(), data_type: "integer".into(), is_nullable: false, is_primary_key: false },
            ],
            foreign_keys: vec![ForeignKey {
                column: "user_id".into(),
                parent_table: "users".into(),
                parent_column: "id".into(),
            }],
        },
    ];

    let turtle = SchemaIntrospector::generate_turtle(&tables, "http://example.org/db/");
    assert!(turtle.contains("db:orders_user_id a owl:ObjectProperty"));
    assert!(turtle.contains("rdfs:range db:Users"));
}

#[test]
fn test_sql_type_to_xsd() {
    assert_eq!(SchemaIntrospector::sql_to_xsd("integer"), "xsd:integer");
    assert_eq!(SchemaIntrospector::sql_to_xsd("varchar"), "xsd:string");
    assert_eq!(SchemaIntrospector::sql_to_xsd("boolean"), "xsd:boolean");
    assert_eq!(SchemaIntrospector::sql_to_xsd("timestamp"), "xsd:dateTime");
    assert_eq!(SchemaIntrospector::sql_to_xsd("numeric"), "xsd:decimal");
    assert_eq!(SchemaIntrospector::sql_to_xsd("date"), "xsd:date");
    assert_eq!(SchemaIntrospector::sql_to_xsd("bytea"), "xsd:hexBinary");
    assert_eq!(SchemaIntrospector::sql_to_xsd("unknown_type"), "xsd:string");
}

#[test]
fn test_generate_turtle_unique_column() {
    let tables = vec![TableInfo {
        name: "users".into(),
        columns: vec![
            ColumnInfo { name: "id".into(), data_type: "integer".into(), is_nullable: false, is_primary_key: true },
            ColumnInfo { name: "email".into(), data_type: "varchar".into(), is_nullable: false, is_primary_key: false },
        ],
        foreign_keys: vec![],
    }];

    let turtle = SchemaIntrospector::generate_turtle(&tables, "http://example.org/db/");
    // NOT NULL columns get minCardinality 1
    assert!(turtle.contains("owl:minCardinality"));
}

#[test]
fn test_table_name_to_class() {
    assert_eq!(SchemaIntrospector::table_to_class("users"), "Users");
    assert_eq!(SchemaIntrospector::table_to_class("order_items"), "OrderItems");
    assert_eq!(SchemaIntrospector::table_to_class("product"), "Product");
}
```

**Step 3: Run tests to verify they fail**

Run: `cargo test --test schema_test -- --nocapture`
Expected: FAIL — module doesn't exist.

**Step 4: Implement schema.rs**

Create `src/schema.rs`:

```rust
/// Database schema introspection and OWL generation.

pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub foreign_keys: Vec<ForeignKey>,
}

pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
}

pub struct ForeignKey {
    pub column: String,
    pub parent_table: String,
    pub parent_column: String,
}

pub struct SchemaIntrospector;

impl SchemaIntrospector {
    /// Convert SQL type name to XSD datatype.
    pub fn sql_to_xsd(sql_type: &str) -> &'static str {
        match sql_type.to_lowercase().as_str() {
            "integer" | "int" | "bigint" | "smallint" | "int4" | "int8" | "int2" | "serial" | "bigserial" => "xsd:integer",
            "numeric" | "decimal" | "real" | "double precision" | "float4" | "float8" => "xsd:decimal",
            "boolean" | "bool" => "xsd:boolean",
            "date" => "xsd:date",
            "timestamp" | "timestamptz" | "timestamp without time zone" | "timestamp with time zone" => "xsd:dateTime",
            "bytea" | "blob" => "xsd:hexBinary",
            _ => "xsd:string",
        }
    }

    /// Convert snake_case table name to PascalCase class name.
    pub fn table_to_class(name: &str) -> String {
        name.split('_')
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().to_string() + &chars.collect::<String>(),
                }
            })
            .collect()
    }

    /// Generate OWL Turtle from introspected schema.
    pub fn generate_turtle(tables: &[TableInfo], base_iri: &str) -> String {
        let mut ttl = String::new();
        ttl.push_str(&format!("@prefix owl: <http://www.w3.org/2002/07/owl#> .\n"));
        ttl.push_str(&format!("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n"));
        ttl.push_str(&format!("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n"));
        ttl.push_str(&format!("@prefix db: <{}> .\n\n", base_iri));

        // Build FK lookup: (table, column) → parent_table
        let fk_map: std::collections::HashMap<(String, String), &ForeignKey> = tables.iter()
            .flat_map(|t| t.foreign_keys.iter().map(move |fk| ((t.name.clone(), fk.column.clone()), fk)))
            .collect();

        for table in tables {
            let class = Self::table_to_class(&table.name);
            ttl.push_str(&format!("db:{} a owl:Class ;\n    rdfs:label \"{}\" .\n\n", class, class));

            for col in &table.columns {
                let prop_name = format!("{}_{}", table.name, col.name);

                if let Some(fk) = fk_map.get(&(table.name.clone(), col.name.clone())) {
                    // Foreign key → ObjectProperty
                    let parent_class = Self::table_to_class(&fk.parent_table);
                    ttl.push_str(&format!("db:{} a owl:ObjectProperty ;\n", prop_name));
                    ttl.push_str(&format!("    rdfs:domain db:{} ;\n", class));
                    ttl.push_str(&format!("    rdfs:range db:{} ;\n", parent_class));
                    ttl.push_str(&format!("    rdfs:label \"{}\" .\n\n", col.name));
                } else {
                    // Regular column → DatatypeProperty
                    let xsd = Self::sql_to_xsd(&col.data_type);
                    if col.is_primary_key {
                        ttl.push_str(&format!("db:{} a owl:DatatypeProperty , owl:FunctionalProperty ;\n", prop_name));
                    } else {
                        ttl.push_str(&format!("db:{} a owl:DatatypeProperty ;\n", prop_name));
                    }
                    ttl.push_str(&format!("    rdfs:domain db:{} ;\n", class));
                    ttl.push_str(&format!("    rdfs:range {} ;\n", xsd));
                    ttl.push_str(&format!("    rdfs:label \"{}\" .\n\n", col.name));
                }

                // NOT NULL → cardinality restriction
                if !col.is_nullable {
                    ttl.push_str(&format!("db:{} rdfs:subClassOf [\n", class));
                    ttl.push_str(&format!("    a owl:Restriction ;\n"));
                    ttl.push_str(&format!("    owl:onProperty db:{} ;\n", prop_name));
                    ttl.push_str(&format!("    owl:minCardinality 1\n"));
                    ttl.push_str(&format!("] .\n\n"));
                }
            }
        }

        ttl
    }

    /// Connect to postgres, introspect schema, return TableInfo vec.
    #[cfg(feature = "postgres")]
    pub async fn introspect_postgres(connection_string: &str) -> anyhow::Result<Vec<TableInfo>> {
        use sqlx::postgres::PgPoolOptions;
        use sqlx::Row;

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(connection_string)
            .await?;

        // Get tables
        let table_rows = sqlx::query(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
        ).fetch_all(&pool).await?;

        let mut tables = Vec::new();

        for trow in &table_rows {
            let table_name: String = trow.get("table_name");

            // Get columns
            let col_rows = sqlx::query(
                "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_schema = 'public' AND table_name = $1 ORDER BY ordinal_position"
            ).bind(&table_name).fetch_all(&pool).await?;

            // Get primary keys
            let pk_rows = sqlx::query(
                "SELECT kcu.column_name FROM information_schema.table_constraints tc JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_name = $1"
            ).bind(&table_name).fetch_all(&pool).await?;

            let pk_cols: Vec<String> = pk_rows.iter().map(|r| r.get("column_name")).collect();

            let columns: Vec<ColumnInfo> = col_rows.iter().map(|r| {
                let name: String = r.get("column_name");
                let data_type: String = r.get("data_type");
                let nullable: String = r.get("is_nullable");
                ColumnInfo {
                    is_primary_key: pk_cols.contains(&name),
                    name,
                    data_type,
                    is_nullable: nullable == "YES",
                }
            }).collect();

            // Get foreign keys
            let fk_rows = sqlx::query(
                "SELECT kcu.column_name AS child_column, ccu.table_name AS parent_table, ccu.column_name AS parent_column FROM information_schema.table_constraints tc JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema JOIN information_schema.constraint_column_usage ccu ON tc.constraint_name = ccu.constraint_name WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_name = $1"
            ).bind(&table_name).fetch_all(&pool).await?;

            let foreign_keys: Vec<ForeignKey> = fk_rows.iter().map(|r| ForeignKey {
                column: r.get("child_column"),
                parent_table: r.get("parent_table"),
                parent_column: r.get("parent_column"),
            }).collect();

            tables.push(TableInfo { name: table_name, columns, foreign_keys });
        }

        pool.close().await;
        Ok(tables)
    }
}
```

Add `pub mod schema;` to `src/lib.rs`.

**Step 5: Run tests to verify they pass**

Run: `cargo test --test schema_test -- --nocapture`
Expected: PASS.

**Step 6: Wire import-schema subcommand in main.rs**

```rust
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
```

**Step 7: Commit**

```bash
git add src/schema.rs src/lib.rs src/main.rs Cargo.toml tests/schema_test.rs
git commit -m "feat: add SQL schema import — postgres introspection to OWL Turtle"
```

---

### Task 7: Wire import-schema as MCP tool

Expose `import-schema` via the MCP server too so Claude can use it interactively.

**Files:**
- Modify: `src/server.rs`

**Step 1: Add input struct and handler**

Add to server.rs:

```rust
#[derive(Deserialize, JsonSchema)]
pub struct OntoImportSchemaInput {
    /// Database connection string (e.g. postgres://user:pass@host/db)
    pub connection: String,
    /// Base IRI for generated classes (default: http://example.org/db/)
    pub base_iri: Option<String>,
}
```

Add `onto_import_schema` to the `#[tool_router]` macro and implement the handler following the same pattern as the CLI.

**Step 2: Run tests**

Run: `cargo test -- --nocapture`
Expected: PASS (all existing tests still pass).

**Step 3: Commit**

```bash
git add src/server.rs
git commit -m "feat: expose import-schema as MCP tool (onto_import_schema)"
```

---

### Task 8: Benchmark — Pizza correctness (HermiT / Pellet comparison)

No Rust compilation. Pure Java + Python + shell.

**Files:**
- Create: `benchmark/reasoner/run_pizza_correctness.sh`
- Create: `benchmark/reasoner/JavaReasoner.java`
- Create: `benchmark/reasoner/compare_results.py`
- Create: `benchmark/reasoner/README.md`

**Step 1: Create Java reasoner wrapper**

Create `benchmark/reasoner/JavaReasoner.java`:

```java
import org.semanticweb.owlapi.apibinding.OWLManager;
import org.semanticweb.owlapi.model.*;
import org.semanticweb.owlapi.reasoner.*;
import org.semanticweb.owlapi.reasoner.structural.StructuralReasonerFactory;
import java.io.*;
import java.util.*;

public class JavaReasoner {
    public static void main(String[] args) throws Exception {
        if (args.length < 3) {
            System.err.println("Usage: JavaReasoner <reasoner> <ontology.owl> <output.json>");
            System.exit(1);
        }
        String reasonerName = args[0];  // "hermit" or "pellet"
        String ontologyPath = args[1];
        String outputPath = args[2];

        OWLOntologyManager manager = OWLManager.createOWLOntologyManager();
        OWLOntology ontology = manager.loadOntologyFromOntologyDocument(new File(ontologyPath));

        OWLReasonerFactory factory;
        if (reasonerName.equals("hermit")) {
            factory = new org.semanticweb.HermiT.ReasonerFactory();
        } else if (reasonerName.equals("pellet")) {
            factory = com.clarkparsia.pellet.owlapi.PelletReasonerFactory.getInstance();
        } else {
            throw new IllegalArgumentException("Unknown reasoner: " + reasonerName);
        }

        long startTime = System.currentTimeMillis();
        OWLReasoner reasoner = factory.createReasoner(ontology);
        reasoner.precomputeInferences(InferenceType.CLASS_HIERARCHY);
        long elapsed = System.currentTimeMillis() - startTime;

        // Extract all subsumption pairs
        Set<OWLClass> classes = ontology.getClassesInSignature();
        List<String> subsumptions = new ArrayList<>();
        for (OWLClass cls : classes) {
            for (OWLClass sup : reasoner.getSuperClasses(cls, true).getFlattened()) {
                subsumptions.add(cls.getIRI().toString() + " -> " + sup.getIRI().toString());
            }
        }
        Collections.sort(subsumptions);

        // Write JSON output
        StringBuilder sb = new StringBuilder();
        sb.append("{\"reasoner\":\"").append(reasonerName).append("\",");
        sb.append("\"time_ms\":").append(elapsed).append(",");
        sb.append("\"classes\":").append(classes.size()).append(",");
        sb.append("\"subsumptions\":[");
        for (int i = 0; i < subsumptions.size(); i++) {
            if (i > 0) sb.append(",");
            sb.append("\"").append(subsumptions.get(i).replace("\"", "\\\"")).append("\"");
        }
        sb.append("]}");

        new FileWriter(outputPath).append(sb.toString()).close();
        System.out.println("Done: " + reasonerName + " in " + elapsed + "ms, " + subsumptions.size() + " subsumptions");
    }
}
```

**Step 2: Create shell orchestrator**

Create `benchmark/reasoner/run_pizza_correctness.sh`:

```bash
#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BENCHMARK_DIR="$SCRIPT_DIR/.."
PIZZA_OWL="$BENCHMARK_DIR/reference/pizza-reference.owl"
OO_BIN="${OO_BIN:-open-ontologies}"
RESULTS_DIR="$SCRIPT_DIR/results"

mkdir -p "$RESULTS_DIR"

echo "=== Pizza Ontology Correctness Benchmark ==="
echo ""

# 1. Open Ontologies
echo "Running Open Ontologies (owl-dl)..."
$OO_BIN load "$PIZZA_OWL"
$OO_BIN reason --profile owl-dl > "$RESULTS_DIR/oo_result.json"
echo "  Done."

# 2. HermiT
echo "Running HermiT..."
java -cp "$SCRIPT_DIR/lib/*" JavaReasoner hermit "$PIZZA_OWL" "$RESULTS_DIR/hermit_result.json"

# 3. Pellet
echo "Running Pellet..."
java -cp "$SCRIPT_DIR/lib/*" JavaReasoner pellet "$PIZZA_OWL" "$RESULTS_DIR/pellet_result.json"

# 4. Compare
echo ""
echo "Comparing results..."
python3 "$SCRIPT_DIR/compare_results.py" \
    "$RESULTS_DIR/hermit_result.json" \
    "$RESULTS_DIR/pellet_result.json" \
    "$RESULTS_DIR/oo_result.json"
```

**Step 3: Create comparison script**

Create `benchmark/reasoner/compare_results.py`:

```python
#!/usr/bin/env python3
"""Compare reasoner classification results for correctness."""
import json
import sys

def load_subsumptions(path):
    with open(path) as f:
        data = json.load(f)
    return set(data.get("subsumptions", [])), data.get("time_ms", 0), data.get("reasoner", "unknown")

def main():
    if len(sys.argv) < 4:
        print("Usage: compare_results.py <hermit.json> <pellet.json> <oo.json>")
        sys.exit(1)

    hermit_subs, hermit_ms, _ = load_subsumptions(sys.argv[1])
    pellet_subs, pellet_ms, _ = load_subsumptions(sys.argv[2])
    oo_subs, oo_ms, _ = load_subsumptions(sys.argv[3])

    # Compare OO against HermiT (reference)
    oo_vs_hermit_missing = hermit_subs - oo_subs
    oo_vs_hermit_extra = oo_subs - hermit_subs

    # Compare OO against Pellet
    oo_vs_pellet_missing = pellet_subs - oo_subs
    oo_vs_pellet_extra = oo_subs - pellet_subs

    print(f"{'Reasoner':<25} {'Time (ms)':<12} {'Subsumptions':<15}")
    print("-" * 52)
    print(f"{'HermiT':<25} {hermit_ms:<12} {len(hermit_subs):<15}")
    print(f"{'Pellet':<25} {pellet_ms:<12} {len(pellet_subs):<15}")
    print(f"{'Open Ontologies':<25} {oo_ms:<12} {len(oo_subs):<15}")
    print()

    if not oo_vs_hermit_missing and not oo_vs_hermit_extra:
        print("OO vs HermiT: EXACT MATCH")
    else:
        print(f"OO vs HermiT: {len(oo_vs_hermit_missing)} missing, {len(oo_vs_hermit_extra)} extra")
        for s in sorted(oo_vs_hermit_missing)[:10]:
            print(f"  MISSING: {s}")
        for s in sorted(oo_vs_hermit_extra)[:10]:
            print(f"  EXTRA:   {s}")

    if not oo_vs_pellet_missing and not oo_vs_pellet_extra:
        print("OO vs Pellet: EXACT MATCH")
    else:
        print(f"OO vs Pellet: {len(oo_vs_pellet_missing)} missing, {len(oo_vs_pellet_extra)} extra")

if __name__ == "__main__":
    main()
```

**Step 4: Create README with setup instructions**

Create `benchmark/reasoner/README.md` with instructions for downloading HermiT and Pellet jars.

**Step 5: Commit**

```bash
git add benchmark/reasoner/
git commit -m "feat: add Pizza correctness benchmark (HermiT / Pellet / Open Ontologies)"
```

---

### Task 9: Benchmark — LUBM performance

**Files:**
- Create: `benchmark/reasoner/run_lubm_performance.sh`
- Create: `benchmark/reasoner/generate_lubm.py`
- Create: `benchmark/reasoner/plot_results.py`

**Step 1: Create LUBM-style ontology generator**

Create `benchmark/reasoner/generate_lubm.py` — generates a university ontology at configurable scale with classes, properties, individuals, and OWL restrictions (someValuesFrom, cardinality, disjointWith):

```python
#!/usr/bin/env python3
"""Generate LUBM-style university ontology at configurable scale."""
import sys

def generate(num_axioms: int, output_path: str):
    # Generate university ontology with roughly num_axioms axioms
    # Include: class hierarchies, object properties, datatype properties,
    #          someValuesFrom restrictions, cardinality restrictions, disjoint classes
    ...  # Implementation generates valid OWL Turtle at requested scale

if __name__ == "__main__":
    for size in [1000, 5000, 10000, 50000]:
        generate(size, f"lubm_{size}.owl")
```

**Step 2: Create performance benchmark script**

Create `benchmark/reasoner/run_lubm_performance.sh`:

```bash
#!/bin/bash
set -e
SIZES="1000 5000 10000 50000"
# For each size: generate OWL, run all 3 reasoners, record time
# Output: results/lubm_results.json
```

**Step 3: Create chart plotter**

Create `benchmark/reasoner/plot_results.py`:

```python
#!/usr/bin/env python3
"""Plot LUBM benchmark results as comparison chart."""
import json
import matplotlib.pyplot as plt

# Load results, plot grouped bar chart: axiom count vs time for each reasoner
# Save to benchmark/reasoner/results/lubm_chart.png
```

**Step 4: Commit**

```bash
git add benchmark/reasoner/
git commit -m "feat: add LUBM performance benchmark with chart generation"
```

---

### Task 10: Benchmark — SQL→OWL end-to-end demo

**Files:**
- Create: `benchmark/demo/docker-compose.yml`
- Create: `benchmark/demo/seed.sql`
- Create: `benchmark/demo/run_demo.sh`

**Step 1: Create sample database schema**

Create `benchmark/demo/seed.sql`:

```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    price DECIMAL(10,2) NOT NULL,
    description TEXT
);

CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    product_id INTEGER NOT NULL REFERENCES products(id),
    quantity INTEGER NOT NULL DEFAULT 1,
    ordered_at TIMESTAMP DEFAULT NOW()
);

INSERT INTO users (name, email) VALUES
    ('Alice', 'alice@example.com'),
    ('Bob', 'bob@example.com');

INSERT INTO products (name, price) VALUES
    ('Widget', 9.99),
    ('Gadget', 24.99),
    ('Doohickey', 4.99);

INSERT INTO orders (user_id, product_id, quantity) VALUES
    (1, 1, 2), (1, 2, 1), (2, 3, 5);
```

**Step 2: Create Docker compose**

Create `benchmark/demo/docker-compose.yml`:

```yaml
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_DB: shop
      POSTGRES_USER: demo
      POSTGRES_PASSWORD: demo
    ports:
      - "5433:5432"
    volumes:
      - ./seed.sql:/docker-entrypoint-initdb.d/seed.sql
```

**Step 3: Create demo runner**

Create `benchmark/demo/run_demo.sh`:

```bash
#!/bin/bash
set -e
echo "=== SQL → OWL Demo ==="
echo ""

# Start postgres
docker compose up -d
sleep 3

OO="${OO_BIN:-open-ontologies}"
START=$(date +%s%N)

echo "Step 1: Import schema..."
$OO import-schema "postgres://demo:demo@localhost:5433/shop" --base-iri "http://shop.example.org/" --pretty

echo ""
echo "Step 2: Classify..."
$OO reason --profile owl-dl --pretty

echo ""
echo "Step 3: Query classes..."
$OO query "SELECT ?c ?label WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> . OPTIONAL { ?c <http://www.w3.org/2000/01/rdf-schema#label> ?label } }" --pretty

END=$(date +%s%N)
ELAPSED=$(( (END - START) / 1000000 ))
echo ""
echo "Total pipeline time: ${ELAPSED}ms"

# Cleanup
docker compose down
```

**Step 4: Commit**

```bash
git add benchmark/demo/
git commit -m "feat: add SQL→OWL end-to-end demo with Docker postgres"
```

---

### Task 11: CI pipeline — GitHub Actions

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/benchmark.yml`
- Create: `.github/workflows/release.yml`

**Step 1: Create CI workflow**

Create `.github/workflows/ci.yml`:

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - name: Build
        run: cargo build --all-targets
      - name: Test
        run: cargo test
      - name: Clippy
        run: cargo clippy -- -D warnings
      - name: Audit
        run: |
          cargo install cargo-audit
          cargo audit
```

**Step 2: Create benchmark workflow (manual trigger only)**

Create `.github/workflows/benchmark.yml`:

```yaml
name: Benchmark
on:
  workflow_dispatch:

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-java@v4
        with:
          distribution: 'temurin'
          java-version: '21'
      - uses: actions/setup-python@v5
        with:
          python-version: '3.12'
      - name: Install Python deps
        run: pip install matplotlib rdflib
      - name: Build Open Ontologies
        run: cargo build --release
      - name: Download reasoner jars
        run: |
          mkdir -p benchmark/reasoner/lib
          # Download HermiT and Pellet jars
          # (URLs to be confirmed at implementation time)
      - name: Run Pizza correctness
        run: |
          export OO_BIN=./target/release/open-ontologies
          cd benchmark/reasoner && bash run_pizza_correctness.sh
      - name: Run LUBM performance
        run: |
          export OO_BIN=./target/release/open-ontologies
          cd benchmark/reasoner && bash run_lubm_performance.sh
      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: benchmark/reasoner/results/
```

**Step 3: Create release workflow**

Create `.github/workflows/release.yml`:

```yaml
name: Release
on:
  push:
    tags: ['v*']

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: macos-13
            target: x86_64-apple-darwin
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      - name: Upload binary
        uses: actions/upload-artifact@v4
        with:
          name: open-ontologies-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/open-ontologies

  release:
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          files: open-ontologies-*/open-ontologies
```

**Step 4: Commit**

```bash
git add .github/
git commit -m "feat: add CI, benchmark, and release GitHub Actions workflows"
```

---

### Task 12: README rewrite

**Files:**
- Modify: `README.md`

**Step 1: Add badges after title**

Add after `# Open Ontologies`:

```markdown
[![CI](https://github.com/fabio-rovai/open-ontologies/actions/workflows/ci.yml/badge.svg)](https://github.com/fabio-rovai/open-ontologies/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
```

**Step 2: Add infrastructure pitch after the "Why not just ask Claude directly?" section**

Add new section:

```markdown
## The problem: AI generates knowledge, but can't guarantee consistency

LLMs can extract entities, build schemas, and generate ontologies. But they can't guarantee the result is logically consistent, structurally valid, or safe to deploy.

**Open Ontologies is the safety and automation layer.** Like Terraform validates infrastructure before applying it, Open Ontologies validates ontologies before they go live. Plan changes, detect drift, enforce design patterns, monitor health — with a full audit trail.

Position it in your stack:

| Layer | Tool | What it does |
|-------|------|--------------|
| Generation | Claude / GPT / LLaMA | Generates OWL/RDF from natural language |
| **Validation** | **Open Ontologies** | **Validates, classifies, enforces, monitors** |
| Storage | SPARQL endpoint / triplestore | Persists the production ontology |
| Consumption | Your app / API / pipeline | Queries the knowledge graph |
```

**Step 3: Add SQL→OWL demo section**

Add after infrastructure pitch:

```markdown
## Demo: Database → Ontology in 3 commands

```bash
# Import a PostgreSQL schema as OWL
open-ontologies import-schema postgres://demo:demo@localhost/shop

# Classify with native OWL2-DL reasoner
open-ontologies reason --profile owl-dl

# Query the result
open-ontologies query "SELECT ?c ?label WHERE { ?c a owl:Class . ?c rdfs:label ?label }"
```

**Step 4: Add CLI reference section**

Add before the benchmarks section — a compact subcommand reference table.

**Step 5: Rewrite benchmarks section**

Add new concise benchmark section with the HermiT/Pellet/OO comparison table (numbers TBD after running). Keep existing detailed benchmarks below as "Detailed Benchmark Methodology".

**Step 6: Update one-liner**

Change subtitle from "AI-native ontology engine" to:

```markdown
Terraform for Knowledge Graphs — validate, classify, and govern AI-generated ontologies.
```

**Step 7: Commit**

```bash
git add README.md
git commit -m "docs: rewrite README — infrastructure positioning, CLI reference, demo, benchmarks"
```

---

## Execution Summary

| Task | What | Rust compilation? |
|------|------|-------------------|
| 1 | CLI structure + subcommand definitions | Yes (once) |
| 2 | Core ontology CLI (10 subcommands) | Yes |
| 3 | Remote + versioning CLI (6 subcommands) | Yes |
| 4 | Data pipeline CLI (5 subcommands) | Yes |
| 5 | Lifecycle + clinical CLI (11 subcommands) | Yes |
| 6 | Schema import module + import-schema | Yes |
| 7 | Wire import-schema as MCP tool | Yes |
| 8 | Pizza correctness benchmark | No (Java + Python) |
| 9 | LUBM performance benchmark | No (Java + Python) |
| 10 | SQL→OWL demo | No (Docker + shell) |
| 11 | CI pipeline | No (YAML) |
| 12 | README rewrite | No (Markdown) |

Tasks 1–7 require Rust compilation. Tasks 8–12 do not.
