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
    signals_json TEXT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_align_feedback_iris ON align_feedback(source_iri, target_iri);

CREATE TABLE IF NOT EXISTS support_verdicts (
    claim_id TEXT PRIMARY KEY,
    verdict TEXT NOT NULL,
    note TEXT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS tool_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tool TEXT NOT NULL,
    rule_id TEXT NOT NULL,
    entity TEXT NOT NULL,
    accepted INTEGER NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_tool_feedback ON tool_feedback(tool, rule_id, entity);

-- `model_fp` is a composite hash of the embedding configuration that produced
-- the vectors — see src/embed_fingerprint.rs. It is what catches a model or
-- provider swap that keeps the same dimension: `text_dim` still matches and the
-- bytes are unchanged, so nothing else notices. Nullable, because rows written
-- before this column existed have no answer, and unknown must stay
-- distinguishable from known-and-equal.
CREATE TABLE IF NOT EXISTS embeddings (
    iri TEXT PRIMARY KEY,
    text_vec BLOB NOT NULL,
    struct_vec BLOB NOT NULL,
    text_dim INTEGER NOT NULL,
    struct_dim INTEGER NOT NULL,
    model_fp TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Cached HNSW cosine index over the embeddings.text_vec column. Single-row
-- table keyed on `kind` (currently only the cosine variant) so future
-- index variants (Poincare, product) can coexist. `entries_hash` is a
-- fingerprint of the (iri, text_vec) set the index was built from; if it
-- changes we know the cached index is stale and must be rebuilt.
--
-- `model_fp` catches what `entries_hash` structurally cannot. With zero new
-- entities, a model swap leaves every stored vector old-model while every query
-- vector is new-model: the entry set is byte-identical, `entries_hash` is
-- unchanged, and the comparison is meaningless anyway. The two checks are kept
-- side by side because they detect different failures and both are cheap.
CREATE TABLE IF NOT EXISTS hnsw_index_cache (
    kind TEXT PRIMARY KEY,
    entries_hash BLOB NOT NULL,
    entry_count INTEGER NOT NULL,
    serialised BLOB NOT NULL,
    model_fp TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Compile cache for loaded ontology files. One row per ontology `name`.
-- See src/cache.rs for the validity policy.
CREATE TABLE IF NOT EXISTS ontology_cache (
    name TEXT PRIMARY KEY,
    source_path TEXT NOT NULL,
    source_mtime INTEGER NOT NULL,
    source_size INTEGER NOT NULL,
    source_sha TEXT NOT NULL,
    cache_path TEXT NOT NULL,
    triple_count INTEGER NOT NULL,
    compiled_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_access_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS plans (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_id TEXT NOT NULL UNIQUE,
    owner TEXT NOT NULL DEFAULT 'cli',
    new_turtle TEXT NOT NULL,
    added_classes TEXT NOT NULL,
    removed_classes TEXT NOT NULL,
    added_properties TEXT NOT NULL,
    removed_properties TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    applied_at TEXT,
    applied_mode TEXT
);
";

/// A column an upgrade adds to an existing table.
///
/// `CREATE TABLE IF NOT EXISTS` in [`SCHEMA`] already declares every column, so
/// a database created today needs none of these. They exist only for databases
/// created before the column was introduced.
struct ColumnAddition {
    table: &'static str,
    column: &'static str,
    ddl: &'static str,
}

/// One schema upgrade step. Applied inside a single transaction, after which
/// `PRAGMA user_version` is set to `version`.
struct Migration {
    version: i64,
    description: &'static str,
    columns: &'static [ColumnAddition],
}

/// Ordered schema migrations. Append only, never renumber: the version is
/// recorded in every `open-ontologies.db` in the field.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "monitor webhook delivery columns",
        columns: &[
            ColumnAddition {
                table: "monitor_watchers",
                column: "webhook_url",
                ddl: "ALTER TABLE monitor_watchers ADD COLUMN webhook_url TEXT",
            },
            ColumnAddition {
                table: "monitor_watchers",
                column: "webhook_headers",
                ddl: "ALTER TABLE monitor_watchers ADD COLUMN webhook_headers TEXT",
            },
        ],
    },
    Migration {
        version: 2,
        description: "align feedback signal payload",
        columns: &[ColumnAddition {
            table: "align_feedback",
            column: "signals_json",
            ddl: "ALTER TABLE align_feedback ADD COLUMN signals_json TEXT",
        }],
    },
    Migration {
        version: 3,
        description: "embedding model fingerprint on vectors and index cache",
        columns: &[
            ColumnAddition {
                table: "embeddings",
                column: "model_fp",
                ddl: "ALTER TABLE embeddings ADD COLUMN model_fp TEXT",
            },
            ColumnAddition {
                table: "hnsw_index_cache",
                column: "model_fp",
                ddl: "ALTER TABLE hnsw_index_cache ADD COLUMN model_fp TEXT",
            },
        ],
    },
];

/// Schema version a freshly-opened database is brought up to.
pub const SCHEMA_VERSION: i64 = 3;

/// Modelling-buffer vault: surrogate to original mappings, per session.
///
/// Declared alongside [`SCHEMA`] rather than as a migration because it is a new
/// table, and `CREATE TABLE IF NOT EXISTS` already handles both the fresh and
/// the upgrade case. [`MIGRATIONS`] exists for *columns added to tables that
/// already shipped*, which is a different problem.
///
/// This never leaves the machine. See
/// `docs/superpowers/specs/2026-08-09-modelling-buffer-design.md`.
const BUFFER_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS buffer_vault (
    session_id TEXT NOT NULL,
    surrogate TEXT NOT NULL,
    original TEXT NOT NULL,
    disposition TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (session_id, surrogate)
);
CREATE INDEX IF NOT EXISTS idx_buffer_vault_original
    ON buffer_vault(session_id, original);

CREATE TABLE IF NOT EXISTS buffer_sessions (
    session_id TEXT PRIMARY KEY,
    salt TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
";

/// Is `column` present on `table`?
///
/// This is the `PRAGMA table_info` probe that replaces the old
/// discard-the-error idiom. Asking whether the column is there lets a genuine
/// failure (locked file, I/O error, corrupt page) propagate, where
/// `let _ = execute_batch(...)` swallowed it alongside the expected
/// "duplicate column name".
fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Add `plans.owner` to a database created before the column existed.
///
/// Not a [`MIGRATIONS`] entry, by that list's own rule: it exists for *columns
/// added to tables that already shipped*, and `plans` has never appeared in a
/// release — it was added and then given an owner within the same unreleased
/// window. Taking a schema version for it would cost every in-flight branch a
/// renumber and every field database a version bump, to repair a table that
/// only exists on `main`. The `CREATE TABLE IF NOT EXISTS` in [`SCHEMA`]
/// handles every other case.
///
/// Adding the column rather than recreating the table keeps any plan already
/// computed against `main` usable, under the default owner.
fn add_plan_owner_column(conn: &Connection) -> Result<()> {
    if column_exists(conn, "plans", "owner")? {
        return Ok(());
    }
    if let Err(e) =
        conn.execute_batch("ALTER TABLE plans ADD COLUMN owner TEXT NOT NULL DEFAULT 'cli'")
    {
        // Another process opening the same database can add the column between
        // the check above and this statement, and SQLite then rejects ours as a
        // duplicate. That is the outcome we wanted, so only report the failure
        // when the column really is still missing.
        if !column_exists(conn, "plans", "owner").unwrap_or(false) {
            return Err(anyhow::anyhow!("failed adding plans.owner: {e}"));
        }
    }
    Ok(())
}

/// Bring the schema up to [`SCHEMA_VERSION`], recording progress in
/// `PRAGMA user_version`.
///
/// Two properties matter here, and the previous implementation had neither.
///
/// **Errors propagate.** Only the expected "column already exists" case is
/// tolerated, and it is handled by asking first rather than by discarding the
/// result, so everything else surfaces.
///
/// **Each column is applied independently, inside a transaction.** Databases
/// predating the tracker all report `user_version = 0` whether they have the
/// columns or not, so the version alone cannot decide what to do — and a
/// database can be *half* migrated, because the old code ran two `ALTER`s in
/// one batch where the first could commit and the second fail. Checking per
/// column heals that state instead of tripping over it: the applied column is
/// skipped, the missing one is added.
fn run_migrations(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current >= SCHEMA_VERSION {
        return Ok(());
    }

    for migration in MIGRATIONS.iter().filter(|m| m.version > current) {
        // IMMEDIATE, not the default DEFERRED. Each migration reads
        // (`column_exists`) before it writes, and a deferred transaction that
        // takes its read snapshot and then tries to upgrade to a write is the
        // one case SQLite refuses to retry: it returns SQLITE_BUSY straight
        // away rather than calling the busy handler, because waiting there can
        // deadlock. Two processes opening a cold database at once hit that
        // window and one of them exited 1 with "database is locked" in about
        // 2.5% of invocations. Taking the write lock at BEGIN is retried by the
        // busy handler like any other lock.
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        for col in migration.columns {
            if !column_exists(&tx, col.table, col.column)? {
                tx.execute_batch(col.ddl).map_err(|e| {
                    anyhow::anyhow!(
                        "migration {} ({}) failed adding {}.{}: {e}",
                        migration.version,
                        migration.description,
                        col.table,
                        col.column
                    )
                })?;
            }
        }
        // Inside the same transaction as the DDL: SQLite DDL is transactional,
        // so the version bump and the columns it describes commit together or
        // not at all. That is what makes a half-applied state unreachable.
        tx.pragma_update(None, "user_version", migration.version)?;
        tx.commit()?;
    }
    Ok(())
}

/// Put the database into WAL mode, tolerating other processes doing the same.
///
/// Converting a rollback-mode database to WAL takes an exclusive lock on the
/// file, and SQLite fails that conversion with SQLITE_BUSY without consulting
/// the busy handler, so the five second timeout rusqlite installs by default
/// never applies to it. Every subcommand opens this database on the way in, so
/// on a cold data directory any two of them starting at once both attempted the
/// conversion and the loser exited 1 with "database is locked": roughly 40% of
/// invocations under `xargs -P8`, and the cause of CI run 33334852611, where two
/// test binaries shelling out to the CLI raced each other.
///
/// Read the mode before writing it, so the common warm case takes no lock at
/// all, and retry the conversion itself with backoff.
fn enable_wal(conn: &Connection) -> Result<()> {
    const ATTEMPTS: u32 = 10;
    let mut delay = std::time::Duration::from_millis(20);
    for attempt in 0..ATTEMPTS {
        let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
        if mode.eq_ignore_ascii_case("wal") {
            return Ok(());
        }
        match conn.pragma_update(None, "journal_mode", "WAL") {
            Ok(()) => return Ok(()),
            // Only the contended case is retried. A read-only file, a corrupt
            // header or a database on a filesystem that cannot do WAL all fail
            // here on the first attempt, as they should.
            Err(e) if is_busy(&e) && attempt + 1 < ATTEMPTS => {
                std::thread::sleep(delay);
                delay = (delay * 2).min(std::time::Duration::from_millis(500));
            }
            Err(e) => return Err(e.into()),
        }
    }
    // Unreachable: the final attempt above returns Ok or Err, never falls
    // through. Reported as an error rather than a silent Ok so a future edit to
    // the loop cannot turn "gave up" into "succeeded".
    Err(anyhow::anyhow!(
        "could not switch the state database to WAL after {ATTEMPTS} attempts"
    ))
}

fn is_busy(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::DatabaseBusy
                || err.code == rusqlite::ErrorCode::DatabaseLocked
    )
}

/// Minimal SQLite state store for ontology versioning.
#[derive(Clone)]
pub struct StateDb {
    conn: Arc<Mutex<Connection>>,
}

impl StateDb {
    pub fn open(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        enable_wal(&conn)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(SCHEMA)?;
        conn.execute_batch(BUFFER_SCHEMA)?;
        add_plan_owner_column(&conn)?;
        run_migrations(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Schema version recorded in this database's `PRAGMA user_version`.
    pub fn schema_version(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}
