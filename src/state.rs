use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

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
    webhook_url TEXT,
    webhook_headers TEXT,
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

CREATE TABLE IF NOT EXISTS align_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_iri TEXT NOT NULL,
    target_iri TEXT NOT NULL,
    predicted_relation TEXT NOT NULL,
    accepted INTEGER NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_align_feedback_iris ON align_feedback(source_iri, target_iri);

CREATE TABLE IF NOT EXISTS tool_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tool TEXT NOT NULL,
    rule_id TEXT NOT NULL,
    entity TEXT NOT NULL,
    accepted INTEGER NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_tool_feedback ON tool_feedback(tool, rule_id, entity);

CREATE TABLE IF NOT EXISTS embeddings (
    iri TEXT PRIMARY KEY,
    text_vec BLOB NOT NULL,
    struct_vec BLOB NOT NULL,
    text_dim INTEGER NOT NULL,
    struct_dim INTEGER NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
";

/// Minimal SQLite state store for ontology versioning.
#[derive(Clone)]
pub struct StateDb {
    conn: Arc<Mutex<Connection>>,
}

impl StateDb {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(SCHEMA)?;
        // Safe migration: add webhook columns if upgrading from older schema
        let _ = conn.execute_batch(
            "ALTER TABLE monitor_watchers ADD COLUMN webhook_url TEXT;
             ALTER TABLE monitor_watchers ADD COLUMN webhook_headers TEXT;"
        );
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}
