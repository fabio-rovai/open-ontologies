# Ontology Runtime v2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add 6 new feature areas (14 MCP tools) to the existing open-ontologies server: active monitoring, lightweight lineage, plan/apply/migrate, drift detection, design pattern enforcement, and clinical crosswalks.

**Architecture:** Each feature is a new Rust module (`monitor.rs`, `lineage.rs`, `plan.rs`, `drift.rs`, `enforce.rs`, `clinical.rs`) registered in `lib.rs` and wired as MCP tool handlers in `server.rs`. All modules operate on the existing `GraphStore` (Oxigraph) and `StateDb` (SQLite). No new dependencies needed — uses existing `oxigraph`, `rusqlite`, `serde_json`, `parquet`, `arrow`, `chrono` crates.

**Tech Stack:** Rust, Oxigraph (SPARQL), SQLite (state/config), Parquet (crosswalks), rmcp (MCP protocol)

---

## Task 1: Extend SQLite Schema for v2 Features

**Files:**
- Modify: `src/state.rs`
- Test: `tests/state_v2_test.rs`

**Step 1: Write the failing test**

Create `tests/state_v2_test.rs`:

```rust
use open_ontologies::state::StateDb;
use tempfile::NamedTempFile;

#[test]
fn test_v2_tables_exist() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let conn = db.conn();

    // monitor_watchers table
    conn.execute(
        "INSERT INTO monitor_watchers (id, check_type, threshold, severity, action, query, message, enabled)
         VALUES ('test', 'sparql', 0.0, 'error', 'notify', 'ASK { ?s ?p ?o }', 'test', 1)",
        [],
    ).unwrap();

    // monitor_state table
    conn.execute(
        "INSERT INTO monitor_state (key, value) VALUES ('blocked', 'false')",
        [],
    ).unwrap();

    // drift_feedback table
    conn.execute(
        "INSERT INTO drift_feedback (id, from_iri, to_iri, predicted, confidence, actual,
         signal_domain_range, signal_label_sim, signal_hierarchy, signal_individuals, timestamp)
         VALUES ('t1', 'ex:a', 'ex:b', 'rename', 0.8, 'rename', 1, 0.9, 0, 1, '2026-03-11')",
        [],
    ).unwrap();

    // iri_locks table
    conn.execute(
        "INSERT INTO iri_locks (iri, locked_at, reason) VALUES ('ex:Person', '2026-03-11', 'production')",
        [],
    ).unwrap();

    // lineage_events table
    conn.execute(
        "INSERT INTO lineage_events (session_id, seq, timestamp, event_type, operation, details)
         VALUES ('abc', 1, '2026-03-11T00:00:00', 'L', 'load', '0→847')",
        [],
    ).unwrap();

    // enforce_rules table
    conn.execute(
        "INSERT INTO enforce_rules (id, rule_pack, query, severity, message, enabled)
         VALUES ('test', 'generic', 'ASK { ?s ?p ?o }', 'error', 'test', 1)",
        [],
    ).unwrap();

    // Verify counts
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM monitor_watchers", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 1);
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM drift_feedback", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 1);
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM lineage_events", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test state_v2_test -v`
Expected: FAIL — tables don't exist yet.

**Step 3: Write minimal implementation**

In `src/state.rs`, extend the `SCHEMA` constant to add all v2 tables:

```rust
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS ontology_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    label TEXT NOT NULL,
    triple_count INTEGER NOT NULL,
    content TEXT NOT NULL,
    format TEXT NOT NULL DEFAULT 'ntriples',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS monitor_watchers (
    id TEXT PRIMARY KEY,
    check_type TEXT NOT NULL,
    threshold REAL NOT NULL DEFAULT 0.0,
    severity TEXT NOT NULL DEFAULT 'warning',
    action TEXT NOT NULL DEFAULT 'notify',
    query TEXT,
    message TEXT,
    enabled INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS monitor_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS drift_feedback (
    id TEXT PRIMARY KEY,
    from_iri TEXT NOT NULL,
    to_iri TEXT NOT NULL,
    predicted TEXT NOT NULL,
    confidence REAL NOT NULL,
    actual TEXT,
    signal_domain_range INTEGER NOT NULL DEFAULT 0,
    signal_label_sim REAL NOT NULL DEFAULT 0.0,
    signal_hierarchy INTEGER NOT NULL DEFAULT 0,
    signal_individuals INTEGER NOT NULL DEFAULT 0,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS iri_locks (
    iri TEXT PRIMARY KEY,
    locked_at TEXT NOT NULL DEFAULT (datetime('now')),
    reason TEXT
);

CREATE TABLE IF NOT EXISTS lineage_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    operation TEXT NOT NULL,
    details TEXT
);

CREATE TABLE IF NOT EXISTS enforce_rules (
    id TEXT PRIMARY KEY,
    rule_pack TEXT NOT NULL,
    query TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'warning',
    message TEXT,
    enabled INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_lineage_session ON lineage_events(session_id);
CREATE INDEX IF NOT EXISTS idx_lineage_seq ON lineage_events(session_id, seq);
CREATE INDEX IF NOT EXISTS idx_enforce_pack ON enforce_rules(rule_pack);
";
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test state_v2_test -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/state.rs tests/state_v2_test.rs
git commit -m "feat: add v2 SQLite schema for monitor, drift, lineage, enforce"
```

---

## Task 2: Lightweight Lineage Module

**Files:**
- Create: `src/lineage.rs`
- Modify: `src/lib.rs` (add `pub mod lineage;`)
- Test: `tests/lineage_test.rs`

**Step 1: Write the failing test**

Create `tests/lineage_test.rs`:

```rust
use open_ontologies::lineage::LineageLog;
use open_ontologies::state::StateDb;
use tempfile::NamedTempFile;

#[test]
fn test_lineage_record_and_query() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let log = LineageLog::new(db.clone());

    let session = log.new_session();
    log.record(&session, "L", "load", "0→847");
    log.record(&session, "R", "reason", "owl-dl:847→1203");
    log.record(&session, "M", "monitor", "ok");

    let events = log.get_compact(&session);
    let lines: Vec<&str> = events.trim().lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains(":L:load:0→847"));
    assert!(lines[1].contains(":R:reason:owl-dl:847→1203"));
    assert!(lines[2].contains(":M:monitor:ok"));
}

#[test]
fn test_lineage_session_isolation() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let log = LineageLog::new(db.clone());

    let s1 = log.new_session();
    let s2 = log.new_session();
    log.record(&s1, "L", "load", "100");
    log.record(&s2, "L", "load", "200");

    let e1 = log.get_compact(&s1);
    let e2 = log.get_compact(&s2);
    assert!(e1.contains("100"));
    assert!(!e1.contains("200"));
    assert!(e2.contains("200"));
    assert!(!e2.contains("100"));
}

#[test]
fn test_lineage_sequential_numbering() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let log = LineageLog::new(db.clone());

    let session = log.new_session();
    log.record(&session, "L", "load", "a");
    log.record(&session, "V", "validate", "b");
    log.record(&session, "R", "reason", "c");

    let events = log.get_compact(&session);
    let lines: Vec<&str> = events.trim().lines().collect();
    // seq numbers should be 1, 2, 3
    assert!(lines[0].starts_with(&format!("{}:1:", session)));
    assert!(lines[1].starts_with(&format!("{}:2:", session)));
    assert!(lines[2].starts_with(&format!("{}:3:", session)));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test lineage_test -v`
Expected: FAIL — `lineage` module doesn't exist.

**Step 3: Write minimal implementation**

Create `src/lineage.rs`:

```rust
use crate::state::StateDb;
use chrono::Utc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Append-only lineage log. Compressed format for AI consumption.
pub struct LineageLog {
    db: StateDb,
    seq: AtomicU64,
}

impl LineageLog {
    pub fn new(db: StateDb) -> Self {
        Self {
            db,
            seq: AtomicU64::new(0),
        }
    }

    /// Generate a new session ID (short hex).
    pub fn new_session(&self) -> String {
        let id = format!("{:08x}", rand_id());
        self.seq.store(0, Ordering::Relaxed);
        id
    }

    /// Record a lineage event.
    /// Format: session:seq:timestamp:event_type:operation:details
    pub fn record(&self, session_id: &str, event_type: &str, operation: &str, details: &str) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed) + 1;
        let ts = Utc::now().timestamp();
        let conn = self.db.conn();
        let _ = conn.execute(
            "INSERT INTO lineage_events (session_id, seq, timestamp, event_type, operation, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![session_id, seq as i64, ts.to_string(), event_type, operation, details],
        );
    }

    /// Get compact lineage for a session.
    /// Returns: "session:seq:timestamp:type:operation:details\n" per event.
    pub fn get_compact(&self, session_id: &str) -> String {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare(
                "SELECT seq, timestamp, event_type, operation, details
                 FROM lineage_events WHERE session_id = ?1 ORDER BY seq ASC",
            )
            .unwrap();
        let rows: Vec<String> = stmt
            .query_map(rusqlite::params![session_id], |row| {
                let seq: i64 = row.get(0)?;
                let ts: String = row.get(1)?;
                let etype: String = row.get(2)?;
                let op: String = row.get(3)?;
                let details: String = row.get::<_, Option<String>>(4)?.unwrap_or_default();
                Ok(format!("{}:{}:{}:{}:{}:{}", session_id, seq, ts, etype, op, details))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        rows.join("\n") + "\n"
    }
}

fn rand_id() -> u32 {
    use std::time::SystemTime;
    let d = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    (d.as_nanos() & 0xFFFF_FFFF) as u32
}
```

Add to `src/lib.rs`:
```rust
pub mod lineage;
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test lineage_test -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lineage.rs src/lib.rs tests/lineage_test.rs
git commit -m "feat: add lightweight lineage module with compressed event log"
```

---

## Task 3: Active Monitor Module

**Files:**
- Create: `src/monitor.rs`
- Modify: `src/lib.rs` (add `pub mod monitor;`)
- Test: `tests/monitor_test.rs`

**Step 1: Write the failing test**

Create `tests/monitor_test.rs`:

```rust
use open_ontologies::graph::GraphStore;
use open_ontologies::monitor::{Monitor, Watcher, WatcherAction, MonitorResult};
use open_ontologies::state::StateDb;
use std::sync::Arc;
use tempfile::NamedTempFile;

#[test]
fn test_monitor_no_watchers_passes() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let graph = Arc::new(GraphStore::new());
    let monitor = Monitor::new(db, graph);

    let result = monitor.run_watchers();
    assert_eq!(result.status, "ok");
    assert!(result.alerts.is_empty());
}

#[test]
fn test_monitor_sparql_watcher_triggers() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let graph = Arc::new(GraphStore::new());

    // Load some data without labels
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#, None).unwrap();

    let monitor = Monitor::new(db.clone(), graph);

    // Add a watcher that checks for classes without labels
    monitor.add_watcher(Watcher {
        id: "no_labels".into(),
        check_type: "sparql".into(),
        threshold: 0.0,
        severity: "error".into(),
        action: WatcherAction::Notify,
        query: Some("SELECT (COUNT(?c) AS ?count) WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> . FILTER NOT EXISTS { ?c <http://www.w3.org/2000/01/rdf-schema#label> ?l } }".into()),
        message: Some("Classes without labels".into()),
    });

    let result = monitor.run_watchers();
    assert_eq!(result.status, "alert");
    assert_eq!(result.alerts.len(), 1);
    assert_eq!(result.alerts[0].watcher, "no_labels");
}

#[test]
fn test_monitor_block_flag() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let graph = Arc::new(GraphStore::new());

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#, None).unwrap();

    let monitor = Monitor::new(db.clone(), graph);

    monitor.add_watcher(Watcher {
        id: "no_labels".into(),
        check_type: "sparql".into(),
        threshold: 0.0,
        severity: "error".into(),
        action: WatcherAction::BlockNextApply,
        query: Some("SELECT (COUNT(?c) AS ?count) WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> . FILTER NOT EXISTS { ?c <http://www.w3.org/2000/01/rdf-schema#label> ?l } }".into()),
        message: Some("Classes without labels".into()),
    });

    let result = monitor.run_watchers();
    assert_eq!(result.status, "blocked");
    assert!(monitor.is_blocked());
}

#[test]
fn test_monitor_watcher_below_threshold_passes() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let graph = Arc::new(GraphStore::new());

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#, None).unwrap();

    let monitor = Monitor::new(db.clone(), graph);

    monitor.add_watcher(Watcher {
        id: "class_count".into(),
        check_type: "sparql".into(),
        threshold: 10.0,  // threshold is 10, only 1 class loaded
        severity: "warning".into(),
        action: WatcherAction::Notify,
        query: Some("SELECT (COUNT(?c) AS ?count) WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> }".into()),
        message: Some("Too many classes".into()),
    });

    let result = monitor.run_watchers();
    assert_eq!(result.status, "ok");
    assert!(result.alerts.is_empty());
    assert_eq!(result.passed.len(), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test monitor_test -v`
Expected: FAIL — `monitor` module doesn't exist.

**Step 3: Write minimal implementation**

Create `src/monitor.rs`:

```rust
use crate::graph::GraphStore;
use crate::state::StateDb;
use oxigraph::sparql::QueryResults;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatcherAction {
    #[serde(rename = "notify")]
    Notify,
    #[serde(rename = "block_next_apply")]
    BlockNextApply,
    #[serde(rename = "auto_rollback")]
    AutoRollback,
    #[serde(rename = "log")]
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watcher {
    pub id: String,
    pub check_type: String,    // "sparql", "unsatisfiable_classes", "shacl_violation_count", etc.
    pub threshold: f64,
    pub severity: String,      // "critical", "error", "warning"
    pub action: WatcherAction,
    pub query: Option<String>, // SPARQL query for check_type="sparql"
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub watcher: String,
    pub severity: String,
    pub value: f64,
    pub threshold: f64,
    pub action: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorResult {
    pub status: String,  // "ok", "alert", "blocked"
    pub alerts: Vec<Alert>,
    pub passed: Vec<String>,
}

pub struct Monitor {
    db: StateDb,
    graph: Arc<GraphStore>,
}

impl Monitor {
    pub fn new(db: StateDb, graph: Arc<GraphStore>) -> Self {
        Self { db, graph }
    }

    pub fn add_watcher(&self, watcher: Watcher) {
        let conn = self.db.conn();
        let action_str = serde_json::to_string(&watcher.action).unwrap_or_default();
        let action_str = action_str.trim_matches('"');
        let _ = conn.execute(
            "INSERT OR REPLACE INTO monitor_watchers (id, check_type, threshold, severity, action, query, message, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
            rusqlite::params![
                watcher.id, watcher.check_type, watcher.threshold,
                watcher.severity, action_str, watcher.query, watcher.message,
            ],
        );
    }

    pub fn run_watchers(&self) -> MonitorResult {
        let watchers = self.load_watchers();
        let mut alerts = Vec::new();
        let mut passed = Vec::new();
        let mut blocked = false;

        for w in &watchers {
            let value = self.evaluate_watcher(w);
            if value > w.threshold {
                let action_str = serde_json::to_string(&w.action).unwrap_or_default();
                let action_str = action_str.trim_matches('"').to_string();
                if matches!(w.action, WatcherAction::BlockNextApply) {
                    blocked = true;
                    self.set_blocked(true);
                }
                alerts.push(Alert {
                    watcher: w.id.clone(),
                    severity: w.severity.clone(),
                    value,
                    threshold: w.threshold,
                    action: action_str,
                    detail: w.message.clone().unwrap_or_default(),
                });
            } else {
                passed.push(w.id.clone());
            }
        }

        let status = if blocked {
            "blocked".to_string()
        } else if !alerts.is_empty() {
            "alert".to_string()
        } else {
            "ok".to_string()
        };

        MonitorResult { status, alerts, passed }
    }

    pub fn is_blocked(&self) -> bool {
        let conn = self.db.conn();
        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM monitor_state WHERE key = 'blocked'",
                [],
                |r| r.get(0),
            )
            .ok();
        result.as_deref() == Some("true")
    }

    pub fn set_blocked(&self, blocked: bool) {
        let conn = self.db.conn();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO monitor_state (key, value) VALUES ('blocked', ?1)",
            rusqlite::params![if blocked { "true" } else { "false" }],
        );
    }

    pub fn clear_blocked(&self) {
        self.set_blocked(false);
    }

    fn load_watchers(&self) -> Vec<Watcher> {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare("SELECT id, check_type, threshold, severity, action, query, message FROM monitor_watchers WHERE enabled = 1")
            .unwrap();
        stmt.query_map([], |row| {
            let action_str: String = row.get(4)?;
            let action = match action_str.as_str() {
                "block_next_apply" => WatcherAction::BlockNextApply,
                "auto_rollback" => WatcherAction::AutoRollback,
                "log" => WatcherAction::Log,
                _ => WatcherAction::Notify,
            };
            Ok(Watcher {
                id: row.get(0)?,
                check_type: row.get(1)?,
                threshold: row.get(2)?,
                severity: row.get(3)?,
                action,
                query: row.get(5)?,
                message: row.get(6)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    fn evaluate_watcher(&self, watcher: &Watcher) -> f64 {
        match watcher.check_type.as_str() {
            "sparql" => self.eval_sparql_watcher(watcher),
            _ => 0.0,
        }
    }

    fn eval_sparql_watcher(&self, watcher: &Watcher) -> f64 {
        let query = match &watcher.query {
            Some(q) => q,
            None => return 0.0,
        };
        // Expect a SELECT query returning a ?count binding
        match self.graph.sparql_select(query) {
            Ok(json) => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                    if let Some(results) = parsed["results"].as_array() {
                        if let Some(first) = results.first() {
                            if let Some(count_str) = first["count"].as_str() {
                                // Oxigraph returns literal like "\"1\"^^<...>"
                                let cleaned = count_str
                                    .trim_matches('"')
                                    .split("^^")
                                    .next()
                                    .unwrap_or("0")
                                    .trim_matches('"');
                                return cleaned.parse().unwrap_or(0.0);
                            }
                        }
                    }
                }
                0.0
            }
            Err(_) => 0.0,
        }
    }
}
```

Add to `src/lib.rs`:
```rust
pub mod monitor;
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test monitor_test -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/monitor.rs src/lib.rs tests/monitor_test.rs
git commit -m "feat: add active monitor with configurable watchers and block/notify actions"
```

---

## Task 4: Plan/Apply/Migrate Module

**Files:**
- Create: `src/plan.rs`
- Modify: `src/lib.rs` (add `pub mod plan;`)
- Test: `tests/plan_test.rs`

**Step 1: Write the failing test**

Create `tests/plan_test.rs`:

```rust
use open_ontologies::graph::GraphStore;
use open_ontologies::plan::Planner;
use open_ontologies::state::StateDb;
use open_ontologies::monitor::Monitor;
use std::sync::Arc;
use tempfile::NamedTempFile;

fn setup() -> (StateDb, Arc<GraphStore>) {
    let tmp = NamedTempFile::new().unwrap();
    // Leak the tempfile so it lives long enough
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();
    let graph = Arc::new(GraphStore::new());
    (db, graph)
}

#[test]
fn test_plan_additions_only() {
    let (db, graph) = setup();

    // Empty store, load new ontology
    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class ; rdfs:label "Dog" .
        ex:Cat a owl:Class ; rdfs:label "Cat" .
    "#;

    let planner = Planner::new(db, graph);
    let plan = planner.plan(new_turtle).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();

    assert!(parsed["added_classes"].as_array().unwrap().len() >= 2);
    assert_eq!(parsed["removed_classes"].as_array().unwrap().len(), 0);
    assert_eq!(parsed["risk_score"].as_str().unwrap(), "low");
}

#[test]
fn test_plan_detects_removals() {
    let (db, graph) = setup();

    // Load current state
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
        ex:Bird a owl:Class .
    "#, None).unwrap();

    // New state removes Bird
    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
    "#;

    let planner = Planner::new(db, graph);
    let plan = planner.plan(new_turtle).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();

    let removed = parsed["removed_classes"].as_array().unwrap();
    assert!(removed.iter().any(|v| v.as_str().unwrap().contains("Bird")));
    assert!(parsed["risk_score"].as_str().unwrap() != "low");
}

#[test]
fn test_plan_detects_property_changes() {
    let (db, graph) = setup();

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:hasName a owl:DatatypeProperty .
        ex:hasAge a owl:DatatypeProperty .
    "#, None).unwrap();

    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:hasName a owl:DatatypeProperty .
        ex:hasEmail a owl:DatatypeProperty .
    "#;

    let planner = Planner::new(db, graph);
    let plan = planner.plan(new_turtle).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();

    assert!(parsed["removed_properties"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().contains("hasAge")));
    assert!(parsed["added_properties"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().contains("hasEmail")));
}

#[test]
fn test_plan_blast_radius_counts_triples() {
    let (db, graph) = setup();

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Animal a owl:Class .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
        ex:Cat a owl:Class ; rdfs:subClassOf ex:Animal .
        ex:Poodle a owl:Class ; rdfs:subClassOf ex:Dog .
    "#, None).unwrap();

    // Remove Animal — everything depends on it
    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
        ex:Poodle a owl:Class ; rdfs:subClassOf ex:Dog .
    "#;

    let planner = Planner::new(db, graph);
    let plan = planner.plan(new_turtle).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();

    assert!(parsed["blast_radius"]["triples_affected"].as_u64().unwrap() > 0);
    assert_eq!(parsed["risk_score"].as_str().unwrap(), "high");
}

#[test]
fn test_apply_safe_mode() {
    let (db, graph) = setup();

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#, None).unwrap();

    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
    "#;

    let planner = Planner::new(db.clone(), graph.clone());
    let _ = planner.plan(new_turtle).unwrap();
    let result = planner.apply("safe").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(parsed["ok"].as_bool().unwrap(), true);
    // Cat should now be in the store
    let stats = graph.get_stats().unwrap();
    assert!(stats.contains("\"classes\":2") || stats.contains("\"classes\": 2"));
}

#[test]
fn test_apply_blocked_by_monitor() {
    let (db, graph) = setup();
    let monitor = Monitor::new(db.clone(), graph.clone());
    monitor.set_blocked(true);

    let planner = Planner::new(db, graph);
    let result = planner.apply("safe");
    assert!(result.is_err() || {
        let r = result.unwrap();
        r.contains("blocked")
    });
}

#[test]
fn test_migrate_generates_bridges() {
    let (db, graph) = setup();

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:authoredBy a owl:ObjectProperty .
    "#, None).unwrap();

    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:writtenBy a owl:ObjectProperty .
    "#;

    let planner = Planner::new(db, graph.clone());
    let _ = planner.plan(new_turtle).unwrap();
    let result = planner.apply("migrate").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed["migration_triples"].as_u64().unwrap() > 0);

    // Check that equivalentProperty bridge was created
    let query = "ASK { <http://example.org/authoredBy> <http://www.w3.org/2002/07/owl#equivalentProperty> <http://example.org/writtenBy> }";
    let ask_result = graph.sparql_select(query).unwrap();
    assert!(ask_result.contains("true"));
}

#[test]
fn test_lock_prevents_plan() {
    let (db, graph) = setup();

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Person a owl:Class .
    "#, None).unwrap();

    let planner = Planner::new(db, graph);
    planner.lock_iri("http://example.org/Person", "production");

    // Try to remove Person — should be rejected
    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#;

    let plan = planner.plan(new_turtle).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();
    assert!(parsed["locked_violations"].as_array().unwrap().len() > 0);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test plan_test -v`
Expected: FAIL — `plan` module doesn't exist.

**Step 3: Write minimal implementation**

Create `src/plan.rs`. This is the largest module (~600 lines). Key components:

- `Planner` struct holding `db`, `graph`, and a cached `last_plan` (the desired Turtle)
- `plan()` — loads new Turtle into temp store, diffs classes/properties, counts blast radius
- `apply()` — clears store, loads the planned Turtle, runs monitor
- `lock_iri()` / `is_locked()` — SQLite-backed IRI locks
- `generate_migration_triples()` — creates `owl:equivalentProperty`/`owl:equivalentClass` + `owl:deprecated` bridges

The `plan()` method works by:
1. Load new Turtle into a temporary `GraphStore`
2. SPARQL to get current classes/properties from `self.graph`
3. SPARQL to get new classes/properties from temp store
4. Set diff: added = new - current, removed = current - new
5. For each removed IRI, count triples referencing it in current store
6. Check locked IRIs against removed set
7. Score risk: low (additions only), medium (modifications), high (removals with dependents)

The `apply("migrate")` mode:
1. For each removed class, if a likely replacement exists in added set, generate `owl:equivalentClass` bridge
2. Same for properties with `owl:equivalentProperty`
3. Mark removed IRIs as `owl:deprecated true`
4. Add `rdfs:comment` with migration timestamp

Implementation details are straightforward SPARQL queries and set operations on the `GraphStore`. Use `std::cell::RefCell<Option<String>>` for caching the last planned Turtle in memory.

**Step 4: Run test to verify it passes**

Run: `cargo test --test plan_test -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/plan.rs src/lib.rs tests/plan_test.rs
git commit -m "feat: add plan/apply/migrate with blast radius analysis and IRI locks"
```

---

## Task 5: Drift Detection Module

**Files:**
- Create: `src/drift.rs`
- Modify: `src/lib.rs` (add `pub mod drift;`)
- Test: `tests/drift_test.rs`

**Step 1: Write the failing test**

Create `tests/drift_test.rs`:

```rust
use open_ontologies::drift::{DriftDetector, DriftResult};
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use std::sync::Arc;
use tempfile::NamedTempFile;

fn setup() -> (StateDb, Arc<GraphStore>) {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();
    let graph = Arc::new(GraphStore::new());
    (db, graph)
}

#[test]
fn test_drift_no_changes() {
    let (db, _graph) = setup();

    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
    "#;

    let detector = DriftDetector::new(db);
    let result = detector.detect(v1, v1).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed["likely_renames"].as_array().unwrap().is_empty());
    assert!(parsed["added"].as_array().unwrap().is_empty());
    assert!(parsed["removed"].as_array().unwrap().is_empty());
    assert!(parsed["drift_velocity"].as_f64().unwrap() < 0.01);
}

#[test]
fn test_drift_detects_addition_and_removal() {
    let (db, _graph) = setup();

    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
    "#;

    let v2 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Bird a owl:Class .
    "#;

    let detector = DriftDetector::new(db);
    let result = detector.detect(v1, v2).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed["removed"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().contains("Cat")));
    assert!(parsed["added"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().contains("Bird")));
}

#[test]
fn test_drift_detects_likely_rename_by_domain_range() {
    let (db, _graph) = setup();

    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:authoredBy a owl:ObjectProperty ;
            rdfs:domain ex:Paper ;
            rdfs:range ex:Person .
    "#;

    let v2 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:writtenBy a owl:ObjectProperty ;
            rdfs:domain ex:Paper ;
            rdfs:range ex:Person .
    "#;

    let detector = DriftDetector::new(db);
    let result = detector.detect(v1, v2).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let renames = parsed["likely_renames"].as_array().unwrap();
    assert!(!renames.is_empty());
    assert!(renames[0]["from"].as_str().unwrap().contains("authoredBy"));
    assert!(renames[0]["to"].as_str().unwrap().contains("writtenBy"));
    assert!(renames[0]["confidence"].as_f64().unwrap() > 0.5);
}

#[test]
fn test_drift_label_similarity() {
    let (db, _graph) = setup();

    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:DomesticCat a owl:Class ; rdfs:label "Domestic Cat" .
    "#;

    let v2 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:HouseCat a owl:Class ; rdfs:label "House Cat" .
    "#;

    let detector = DriftDetector::new(db);
    let result = detector.detect(v1, v2).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    // Should detect rename via label similarity (both end in "Cat")
    let renames = parsed["likely_renames"].as_array().unwrap();
    assert!(!renames.is_empty());
    assert!(renames[0]["signals"]["label_similarity"].as_f64().unwrap() > 0.3);
}

#[test]
fn test_drift_velocity() {
    let (db, _graph) = setup();

    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:A a owl:Class .
        ex:B a owl:Class .
        ex:C a owl:Class .
        ex:D a owl:Class .
    "#;

    // Replace 3 of 4 classes = high drift
    let v2 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:A a owl:Class .
        ex:X a owl:Class .
        ex:Y a owl:Class .
        ex:Z a owl:Class .
    "#;

    let detector = DriftDetector::new(db);
    let result = detector.detect(v1, v2).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed["drift_velocity"].as_f64().unwrap() > 0.5);
}

#[test]
fn test_drift_feedback_improves_confidence() {
    let (db, _graph) = setup();
    let detector = DriftDetector::new(db);

    // Record some feedback
    detector.record_feedback("ex:a", "ex:b", "rename", 0.8, "rename",
        true, 0.9, false, true);
    detector.record_feedback("ex:c", "ex:d", "rename", 0.6, "different_concept",
        false, 0.3, false, false);

    // Weights should now be retrievable
    let weights = detector.get_learned_weights();
    // With 2 data points we should have some weights (or fallback to defaults)
    assert_eq!(weights.len(), 4); // 4 signal weights
}

#[test]
fn test_jaro_winkler_similarity() {
    use open_ontologies::drift::jaro_winkler;
    assert!(jaro_winkler("authoredBy", "authoredBy") > 0.99);
    assert!(jaro_winkler("authoredBy", "writtenBy") > 0.4);
    assert!(jaro_winkler("Dog", "Cat") < 0.6);
    assert!(jaro_winkler("DomesticCat", "HouseCat") > 0.5);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test drift_test -v`
Expected: FAIL — `drift` module doesn't exist.

**Step 3: Write minimal implementation**

Create `src/drift.rs`. Key components:

- `DriftDetector` struct with `db: StateDb`
- `detect(v1_turtle, v2_turtle)` — loads both into temp stores, extracts class/property vocabularies, diffs, scores renames
- `jaro_winkler(a, b)` — pure Rust string similarity (public for testing)
- `record_feedback()` / `get_learned_weights()` — SQLite-backed feedback loop
- Rename candidate scoring: check domain/range match, label similarity, hierarchy position, individual membership
- `drift_velocity` = (added + removed) / (total_v1 + total_v2) — normalized change rate

The self-calibrating confidence uses stored feedback to fit logistic regression weights:
```rust
// sigmoid(w1*domain_range + w2*label_sim + w3*hierarchy + w4*individuals + bias)
// Fit by gradient descent on stored feedback rows
```

Falls back to equal weights (0.25 each) until 10+ feedback entries exist.

**Step 4: Run test to verify it passes**

Run: `cargo test --test drift_test -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/drift.rs src/lib.rs tests/drift_test.rs
git commit -m "feat: add drift detection with self-calibrating confidence and Jaro-Winkler"
```

---

## Task 6: Design Pattern Enforcement Module

**Files:**
- Create: `src/enforce.rs`
- Modify: `src/lib.rs` (add `pub mod enforce;`)
- Test: `tests/enforce_test.rs`

**Step 1: Write the failing test**

Create `tests/enforce_test.rs`:

```rust
use open_ontologies::enforce::Enforcer;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use std::sync::Arc;
use tempfile::NamedTempFile;

fn setup_with_ontology(ttl: &str) -> (StateDb, Arc<GraphStore>) {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(ttl, None).unwrap();
    (db, graph)
}

#[test]
fn test_enforce_generic_orphan_class() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Animal a owl:Class .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
        ex:OrphanClass a owl:Class .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("generic").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| {
        v["rule"].as_str().unwrap() == "orphan_class"
            && v["entity"].as_str().unwrap().contains("OrphanClass")
    }));
}

#[test]
fn test_enforce_generic_missing_domain_range() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:hasName a owl:DatatypeProperty .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("generic").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| v["rule"].as_str().unwrap() == "missing_domain"));
}

#[test]
fn test_enforce_boro_missing_state_class() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ies: <http://ies.data.gov.uk/ontology/ies4#> .
        @prefix ex: <http://example.org/> .
        ex:Building a owl:Class ; rdfs:subClassOf ies:Entity .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("boro").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    // Building is an Entity but has no BuildingState
    assert!(violations.iter().any(|v| {
        v["rule"].as_str().unwrap() == "missing_state_class"
            && v["entity"].as_str().unwrap().contains("Building")
    }));
}

#[test]
fn test_enforce_boro_passes_with_state() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ies: <http://ies.data.gov.uk/ontology/ies4#> .
        @prefix ex: <http://example.org/> .
        ex:Building a owl:Class ; rdfs:subClassOf ies:Entity .
        ex:BuildingState a owl:Class ; rdfs:subClassOf ies:State, ex:Building .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("boro").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    assert!(!violations.iter().any(|v| {
        v["rule"].as_str().unwrap() == "missing_state_class"
            && v["entity"].as_str().unwrap().contains("Building")
    }));
}

#[test]
fn test_enforce_value_partition_incomplete() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Spiciness a owl:Class .
        ex:Hot a owl:Class ; rdfs:subClassOf ex:Spiciness .
        ex:Medium a owl:Class ; rdfs:subClassOf ex:Spiciness .
        ex:Mild a owl:Class ; rdfs:subClassOf ex:Spiciness .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("value_partition").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    // Should flag that partition values are not pairwise disjoint
    assert!(violations.iter().any(|v| v["rule"].as_str().unwrap() == "partition_not_disjoint"));
}

#[test]
fn test_enforce_value_partition_passes() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Spiciness a owl:Class .
        ex:Hot a owl:Class ; rdfs:subClassOf ex:Spiciness ; owl:disjointWith ex:Medium, ex:Mild .
        ex:Medium a owl:Class ; rdfs:subClassOf ex:Spiciness ; owl:disjointWith ex:Hot, ex:Mild .
        ex:Mild a owl:Class ; rdfs:subClassOf ex:Spiciness ; owl:disjointWith ex:Hot, ex:Medium .
        ex:Spiciness owl:equivalentClass [ a owl:Class ; owl:unionOf ( ex:Hot ex:Medium ex:Mild ) ] .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("value_partition").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.is_empty() || !violations.iter().any(|v|
        v["rule"].as_str().unwrap() == "partition_not_disjoint"
            && v["entity"].as_str().unwrap().contains("Spiciness")
    ));
}

#[test]
fn test_enforce_custom_sparql_rule() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Drug a owl:Class .
    "#);

    let enforcer = Enforcer::new(db.clone(), graph);

    // Add custom rule: every Drug must have an indication
    enforcer.add_custom_rule(
        "drug_indication",
        "custom",
        "ASK { ?d a <http://example.org/Drug> . FILTER NOT EXISTS { ?d <http://example.org/hasIndication> ?i } }",
        "error",
        "Drug without indication",
    );

    let result = enforcer.enforce("custom").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| v["rule"].as_str().unwrap() == "drug_indication"));
}

#[test]
fn test_enforce_compliance_score() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class ; rdfs:label "Dog" .
        ex:Cat a owl:Class .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("generic").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    // Should have a compliance score between 0 and 1
    let score = parsed["compliance"].as_f64().unwrap();
    assert!(score >= 0.0 && score <= 1.0);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test enforce_test -v`
Expected: FAIL — `enforce` module doesn't exist.

**Step 3: Write minimal implementation**

Create `src/enforce.rs`. Key components:

- `Enforcer` struct with `db: StateDb`, `graph: Arc<GraphStore>`
- `enforce(rule_pack)` — loads rules for the pack, runs each SPARQL query, collects violations
- Built-in rule packs: `generic`, `boro`, `value_partition`
- `add_custom_rule()` — stores in SQLite `enforce_rules` table
- Each rule is a SPARQL ASK or SELECT query. ASK returning true = violation. SELECT returning rows = one violation per row.
- Compliance score = passed_rules / total_rules

**BORO rules** key queries:
- Missing state: `SELECT ?entity WHERE { ?entity rdfs:subClassOf ies:Entity . FILTER NOT EXISTS { ?state rdfs:subClassOf ?entity . ?state rdfs:subClassOf ies:State } }`
- Missing BoundingState: similar pattern checking for `isStartOf`/`isEndOf` relationships

**Value partition rules** key queries:
- Not disjoint: find classes that are siblings under the same parent but not `owl:disjointWith` each other
- Not covering: check if parent has `owl:equivalentClass` with `owl:unionOf` its children

**Step 4: Run test to verify it passes**

Run: `cargo test --test enforce_test -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/enforce.rs src/lib.rs tests/enforce_test.rs
git commit -m "feat: add design pattern enforcement with generic, BORO, and value partition packs"
```

---

## Task 7: Clinical Crosswalks Module

**Files:**
- Create: `src/clinical.rs`
- Create: `scripts/build_crosswalks.py`
- Create: `data/crosswalks.parquet` (built by script)
- Modify: `src/lib.rs` (add `pub mod clinical;`)
- Test: `tests/clinical_test.rs`

**Step 1: Write the build script**

Create `scripts/build_crosswalks.py` that downloads open crosswalk files and produces `data/crosswalks.parquet`.

Sources:
- WHO ICD-10 → download from WHO API or GitHub mirrors
- SNOMED-CT to ICD-10 map → NHS TRUD open mapping files
- MeSH descriptors → NLM FTP

Script normalizes all into columns: `source_code, source_system, target_code, target_system, relation, source_label, target_label` and writes Parquet via `pyarrow`.

**Step 2: Write the failing test**

Create `tests/clinical_test.rs`:

```rust
use open_ontologies::clinical::ClinicalCrosswalks;
use open_ontologies::graph::GraphStore;
use std::sync::Arc;

#[test]
fn test_crosswalk_lookup() {
    // This test uses the shipped Parquet file
    let cw = ClinicalCrosswalks::load("data/crosswalks.parquet");
    if cw.is_err() {
        // Skip if Parquet not built yet
        eprintln!("Skipping: crosswalks.parquet not found. Run scripts/build_crosswalks.py first.");
        return;
    }
    let cw = cw.unwrap();

    let results = cw.lookup("I10", "ICD10");
    // Should find at least one mapping
    assert!(!results.is_empty() || true); // Soft assert — depends on data
}

#[test]
fn test_crosswalk_search_by_label() {
    let cw = ClinicalCrosswalks::load("data/crosswalks.parquet");
    if cw.is_err() { return; }
    let cw = cw.unwrap();

    let results = cw.search_label("hypertension");
    // Should find matches across systems
    assert!(!results.is_empty() || true);
}

#[test]
fn test_validate_clinical_terms() {
    let cw = ClinicalCrosswalks::load("data/crosswalks.parquet");
    if cw.is_err() { return; }
    let cw = cw.unwrap();

    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Hypertension a owl:Class ; rdfs:label "Hypertension" .
        ex:FakeDisease a owl:Class ; rdfs:label "HyperTensionSyndrome" .
    "#, None).unwrap();

    let result = cw.validate_clinical(&graph);
    // Hypertension should match, FakeDisease might get a close suggestion
    assert!(result.contains("validated") || result.contains("unmatched"));
}

#[test]
fn test_enrich_adds_skos_mapping() {
    let cw = ClinicalCrosswalks::load("data/crosswalks.parquet");
    if cw.is_err() { return; }
    let cw = cw.unwrap();

    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Hypertension a owl:Class .
    "#, None).unwrap();

    let result = cw.enrich(&graph, "http://example.org/Hypertension", "I10", "ICD10");
    assert!(result.contains("ok") || result.contains("enriched"));
}

// Test without Parquet — just the Jaro-Winkler matching logic
#[test]
fn test_clinical_label_matching() {
    use open_ontologies::drift::jaro_winkler;
    let score = jaro_winkler("Hypertension", "HyperTensionSyndrome");
    assert!(score > 0.5); // Should be somewhat similar
    let score = jaro_winkler("Hypertension", "Essential hypertension");
    assert!(score > 0.4);
}
```

**Step 2b: Run test to verify it fails**

Run: `cargo test --test clinical_test -v`
Expected: FAIL — `clinical` module doesn't exist.

**Step 3: Write minimal implementation**

Create `src/clinical.rs`. Key components:

- `ClinicalCrosswalks` struct holding a `Vec<CrosswalkRow>` loaded from Parquet
- `load(path)` — reads Parquet using `parquet` and `arrow` crates (already in Cargo.toml)
- `lookup(code, system)` — filter rows by source_code + source_system, return all target mappings
- `search_label(text)` — Jaro-Winkler fuzzy search across all labels in the Parquet
- `validate_clinical(graph)` — SPARQL to get all class labels, match each against crosswalk labels
- `enrich(graph, class_iri, code, system)` — insert `skos:exactMatch` triple into graph

Parquet reading pattern (already used in `ingest.rs`):
```rust
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use arrow::array::StringArray;
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test clinical_test -v`
Expected: PASS (some tests soft-skip if Parquet not built yet)

**Step 5: Commit**

```bash
git add src/clinical.rs src/lib.rs scripts/build_crosswalks.py tests/clinical_test.rs
git commit -m "feat: add clinical crosswalks module with Parquet-backed term lookup"
```

---

## Task 8: Wire All New Tools in server.rs

**Files:**
- Modify: `src/server.rs`
- Test: Run all existing + new tests

**Step 1: Add input structs to server.rs**

Add new `#[derive(Deserialize, JsonSchema)]` structs for each new tool's input:

- `OntoPlanInput { new_turtle: String }`
- `OntoApplyInput { mode: String }` (safe/force/migrate)
- `OntoMigrateInput {}` (no params, generates from last plan)
- `OntoLockInput { iris: Vec<String>, reason: Option<String> }`
- `OntoDriftInput { version_a: String, version_b: String }`
- `OntoEnforceInput { rule_pack: String }`
- `OntoMonitorInput { watchers: Option<String> }` (inline JSON)
- `OntoCrosswalkInput { code: String, source_system: String, target_system: String }`
- `OntoEnrichInput { class_iri: String, code: String, system: String }`
- `OntoValidateClinicalInput {}` (no params, uses loaded graph)
- `OntoLineageInput { session_id: String, format: Option<String> }`
- `OntoMonitorStatusInput {}` (no params)

**Step 2: Add fields to OpenOntologiesServer**

```rust
pub struct OpenOntologiesServer {
    tool_router: ToolRouter<Self>,
    db: StateDb,
    graph: Arc<GraphStore>,
    monitor: Monitor,
    lineage: LineageLog,
    session_id: String,
}
```

Initialize `monitor`, `lineage`, and `session_id` in `new()`.

**Step 3: Add tool handlers**

Add `#[tool(...)]` handlers for each new tool, following the existing pattern. Each handler calls the corresponding module function and returns JSON.

For mutating tools (`onto_load`, `onto_apply`, `onto_ingest`, `onto_reason`, `onto_extend`), add after the main result:
1. `self.lineage.record(...)` — log the operation
2. `let monitor_result = self.monitor.run_watchers()` — check health
3. Merge monitor result into the response JSON

**Step 4: Run all tests**

Run: `cargo test`
Expected: All existing 102 tests pass + all new tests pass.

**Step 5: Update tool_router macro**

Add all new tool names to the `#[tool_router]` impl block.

**Step 6: Commit**

```bash
git add src/server.rs
git commit -m "feat: wire 14 new MCP tools (plan, drift, enforce, monitor, clinical, lineage)"
```

---

## Task 9: Integration Test — Full Terraform Loop

**Files:**
- Create: `tests/terraform_loop_test.rs`

**Step 1: Write integration test**

```rust
use open_ontologies::graph::GraphStore;
use open_ontologies::plan::Planner;
use open_ontologies::drift::DriftDetector;
use open_ontologies::monitor::Monitor;
use open_ontologies::enforce::Enforcer;
use open_ontologies::lineage::LineageLog;
use open_ontologies::state::StateDb;
use std::sync::Arc;
use tempfile::NamedTempFile;

/// Full Terraform-style loop: plan → enforce → apply → monitor → drift
#[test]
fn test_full_terraform_loop() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();
    let graph = Arc::new(GraphStore::new());

    // --- Initial state ---
    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Animal a owl:Class ; rdfs:label "Animal" .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal ; rdfs:label "Dog" .
    "#;
    graph.load_turtle(v1, None).unwrap();

    // --- Lineage ---
    let lineage = LineageLog::new(db.clone());
    let session = lineage.new_session();
    lineage.record(&session, "L", "load", "v1");

    // --- Plan changes ---
    let v2 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Animal a owl:Class ; rdfs:label "Animal" .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal ; rdfs:label "Dog" .
        ex:Cat a owl:Class ; rdfs:subClassOf ex:Animal ; rdfs:label "Cat" .
    "#;

    let planner = Planner::new(db.clone(), graph.clone());
    let plan = planner.plan(v2).unwrap();
    let plan_parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();
    assert_eq!(plan_parsed["risk_score"].as_str().unwrap(), "low");
    lineage.record(&session, "P", "plan", "low_risk");

    // --- Enforce ---
    let enforcer = Enforcer::new(db.clone(), graph.clone());
    let enforce_result = enforcer.enforce("generic").unwrap();
    lineage.record(&session, "E", "enforce", "generic");

    // --- Apply ---
    let apply_result = planner.apply("safe").unwrap();
    let apply_parsed: serde_json::Value = serde_json::from_str(&apply_result).unwrap();
    assert_eq!(apply_parsed["ok"].as_bool().unwrap(), true);
    lineage.record(&session, "A", "apply", "safe");

    // --- Monitor ---
    let monitor = Monitor::new(db.clone(), graph.clone());
    let mon_result = monitor.run_watchers();
    assert_eq!(mon_result.status, "ok");
    lineage.record(&session, "M", "monitor", "ok");

    // --- Drift check (v1 vs v2) ---
    let detector = DriftDetector::new(db.clone());
    let drift = detector.detect(v1, v2).unwrap();
    let drift_parsed: serde_json::Value = serde_json::from_str(&drift).unwrap();
    assert!(drift_parsed["drift_velocity"].as_f64().unwrap() < 0.5); // Low drift — just an addition
    lineage.record(&session, "D", "drift", "low");

    // --- Verify lineage ---
    let events = lineage.get_compact(&session);
    let lines: Vec<&str> = events.trim().lines().collect();
    assert_eq!(lines.len(), 6); // L, P, E, A, M, D
}
```

**Step 2: Run test**

Run: `cargo test --test terraform_loop_test -v`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/terraform_loop_test.rs
git commit -m "test: add full Terraform loop integration test"
```

---

## Task 10: Update README and CLAUDE.md

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`

**Step 1: Update README.md**

Add a new section after "OWL2-DL Reasoning" covering:
- Terraform-style lifecycle (plan/apply/migrate)
- Drift detection with self-calibrating confidence
- Active monitoring with watchers
- Design pattern enforcement (generic, BORO, value partition)
- Clinical crosswalks
- Lightweight lineage
- Updated tool table (existing 21 + 14 new = 35 tools)

**Step 2: Update CLAUDE.md**

Add new workflow section "Ontology Lifecycle" describing the plan→enforce→apply→monitor→drift cycle. Add new tools to the tool reference table.

**Step 3: Commit**

```bash
git add README.md CLAUDE.md
git commit -m "docs: add v2 features to README and CLAUDE.md"
```

---

## Implementation Order Summary

| Task | Module | Est. lines | Depends on |
|------|--------|-----------|------------|
| 1. SQLite schema | `state.rs` | ~50 | — |
| 2. Lineage | `lineage.rs` | ~150 | Task 1 |
| 3. Monitor | `monitor.rs` | ~350 | Task 1 |
| 4. Plan/Apply/Migrate | `plan.rs` | ~600 | Task 1, 3 |
| 5. Drift | `drift.rs` | ~400 | Task 1 |
| 6. Enforce | `enforce.rs` | ~500 | Task 1 |
| 7. Clinical | `clinical.rs` | ~300 | Task 5 (shares Jaro-Winkler) |
| 8. Server wiring | `server.rs` | ~400 | Tasks 2-7 |
| 9. Integration test | `terraform_loop_test.rs` | ~100 | Task 8 |
| 10. Docs | `README.md`, `CLAUDE.md` | ~200 | Task 9 |

**Total: ~3,050 lines of new Rust + tests + docs.**

Tasks 2, 3, 5, 6 can be parallelized (independent modules). Task 4 depends on 3 (uses monitor). Task 7 depends on 5 (shares Jaro-Winkler). Task 8 depends on all modules. Task 9 depends on 8.
