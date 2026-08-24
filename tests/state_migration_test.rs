//! Upgrade-in-place tests for the `StateDb` schema.
//!
//! These open **checked-in binary fixtures** under `tests/fixtures/state/`, not
//! databases produced by the code under test. That distinction is the whole
//! point: a fixture generated from the current schema would agree with a broken
//! migration by construction, and the bug this guards against is precisely a
//! migration that silently does not apply.
//!
//! They live under `tests/fixtures/` rather than `tests/data/` because
//! `.gitignore` ignores `data/` repo-wide. A fixture that is not committed is
//! not a fixture, and the failure would be silent on a fresh clone.
//!
//! Each test copies its fixture to a tempdir first. `StateDb::open` sets
//! `journal_mode = WAL`, which writes to the file, so opening a fixture in place
//! would mutate the committed artefact.

use std::path::{Path, PathBuf};

use open_ontologies::state::{StateDb, SCHEMA_VERSION};
use rusqlite::Connection;
use tempfile::TempDir;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/state")
        .join(name)
}

/// Copy a fixture into a fresh tempdir and return (dir, path). The dir must
/// stay alive for the lifetime of the database.
fn staged(name: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let dst = dir.path().join(name);
    std::fs::copy(fixture(name), &dst)
        .unwrap_or_else(|e| panic!("fixture {name} missing or unreadable: {e}"));
    (dir, dst)
}

fn columns(conn: &Connection, table: &str) -> Vec<String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .unwrap();
    stmt.query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// The baseline case: a database predating both migrations. It reports
/// `user_version = 0` and has neither column.
#[test]
fn upgrades_a_pre_migration_database_and_preserves_rows() {
    let (_dir, path) = staged("v0-pre-migrations.db");

    // Establish the starting state from the fixture itself, so this test fails
    // loudly if the fixture is ever regenerated into something else.
    {
        let raw = Connection::open(&path).unwrap();
        let v: i64 = raw.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 0, "fixture should start at user_version 0");
        let cols = columns(&raw, "monitor_watchers");
        assert!(!cols.contains(&"webhook_url".to_string()));
        assert!(!cols.contains(&"webhook_headers".to_string()));
        assert!(!columns(&raw, "align_feedback").contains(&"signals_json".to_string()));
    }

    let db = StateDb::open(&path).unwrap();
    assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);

    let conn = db.conn();
    let watchers = columns(&conn, "monitor_watchers");
    assert!(watchers.contains(&"webhook_url".to_string()));
    assert!(watchers.contains(&"webhook_headers".to_string()));
    assert!(columns(&conn, "align_feedback").contains(&"signals_json".to_string()));

    // Pre-existing rows survive, with their values intact and the new column
    // NULL rather than defaulted over the top of anything.
    let (id, threshold, message, hook): (String, f64, String, Option<String>) = conn
        .query_row(
            "SELECT id, threshold, message, webhook_url FROM monitor_watchers WHERE id = 'w-legacy'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(id, "w-legacy");
    assert_eq!(threshold, 42.0);
    assert_eq!(message, "pre-migration watcher");
    assert_eq!(hook, None);

    let (src, signals): (String, Option<String>) = conn
        .query_row(
            "SELECT source_iri, signals_json FROM align_feedback WHERE source_iri = 'http://ex.org/A'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(src, "http://ex.org/A");
    assert_eq!(signals, None);
}

/// The sharp case. The old code ran two `ALTER`s in one `execute_batch` and
/// discarded the result, so the first could commit while the second failed,
/// leaving a database with `webhook_url` but not `webhook_headers`. Re-running
/// migration 1 wholesale against that state fails on "duplicate column name",
/// which is why the check is per column rather than per migration.
#[test]
fn heals_a_half_applied_migration() {
    let (_dir, path) = staged("v0-half-migrated.db");

    {
        let raw = Connection::open(&path).unwrap();
        let cols = columns(&raw, "monitor_watchers");
        assert!(
            cols.contains(&"webhook_url".to_string()),
            "fixture should already have webhook_url"
        );
        assert!(
            !cols.contains(&"webhook_headers".to_string()),
            "fixture should be missing webhook_headers"
        );
    }

    let db = StateDb::open(&path).unwrap();
    assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);

    let conn = db.conn();
    let cols = columns(&conn, "monitor_watchers");
    assert!(cols.contains(&"webhook_url".to_string()));
    assert!(cols.contains(&"webhook_headers".to_string()));

    // The value already stored in the half-applied column is not clobbered by
    // the completing migration.
    let hook: String = conn
        .query_row(
            "SELECT webhook_url FROM monitor_watchers WHERE id = 'w-half'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hook, "https://hooks.example.org/x");
}

/// A database created today is already at the current version and needs no
/// upgrade on the next open.
#[test]
fn fresh_database_is_stamped_at_current_version() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fresh.db");

    let db = StateDb::open(&path).unwrap();
    assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
    drop(db);

    let reopened = StateDb::open(&path).unwrap();
    assert_eq!(reopened.schema_version().unwrap(), SCHEMA_VERSION);
}

/// Opening repeatedly is a no-op after the first upgrade, and does not disturb
/// data written between opens.
#[test]
fn reopening_an_upgraded_database_is_idempotent() {
    let (_dir, path) = staged("v0-pre-migrations.db");

    let db = StateDb::open(&path).unwrap();
    assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
    db.conn()
        .execute(
            "UPDATE monitor_watchers SET webhook_url = ?1 WHERE id = 'w-legacy'",
            ["https://hooks.example.org/after-upgrade"],
        )
        .unwrap();
    drop(db);

    let again = StateDb::open(&path).unwrap();
    assert_eq!(again.schema_version().unwrap(), SCHEMA_VERSION);
    let hook: String = again
        .conn()
        .query_row(
            "SELECT webhook_url FROM monitor_watchers WHERE id = 'w-legacy'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hook, "https://hooks.example.org/after-upgrade");
}

/// A real failure must surface rather than be swallowed. The old
/// `let _ = execute_batch(...)` discarded every error, not just the expected
/// duplicate-column one; pointing `open` at a file that is not a database is
/// the cheapest way to assert that errors now propagate.
#[test]
fn a_genuine_open_failure_is_an_error_not_a_silent_pass() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("not-a-database.db");
    std::fs::write(&path, b"this is definitely not a SQLite file").unwrap();

    assert!(
        StateDb::open(&path).is_err(),
        "opening a non-database must return Err, not succeed silently"
    );
}

/// Migration 3: the embedding model fingerprint columns.
///
/// The fixture is a hand-written v2 database — the DDL was copied from the
/// schema as it stood, not generated from the current `SCHEMA` constant, for
/// the reason in this file's header.
#[test]
fn adds_the_model_fingerprint_columns_and_keeps_the_rows() {
    let (_dir, path) = staged("v2-pre-model-fp.db");

    {
        let raw = Connection::open(&path).unwrap();
        let v: i64 = raw.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 2, "fixture should start at user_version 2");
        assert!(
            !columns(&raw, "embeddings").contains(&"model_fp".to_string()),
            "fixture should predate the column"
        );
        assert!(!columns(&raw, "hnsw_index_cache").contains(&"model_fp".to_string()));
    }

    let db = StateDb::open(&path).unwrap();
    assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);

    let conn = db.conn();
    assert!(columns(&conn, "embeddings").contains(&"model_fp".to_string()));
    assert!(columns(&conn, "hnsw_index_cache").contains(&"model_fp".to_string()));

    // The vectors survive, and their fingerprint is NULL rather than
    // back-filled with a guess. NULL is the honest answer — nothing recorded
    // which model produced them — and it is what makes the load path reject
    // them once instead of trusting them forever.
    let (iri, fp): (String, Option<String>) = conn
        .query_row(
            "SELECT iri, model_fp FROM embeddings WHERE iri = 'http://ex.org/Legacy'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(iri, "http://ex.org/Legacy");
    assert_eq!(fp, None);

    let (kind, count, fp): (String, i64, Option<String>) = conn
        .query_row(
            "SELECT kind, entry_count, model_fp FROM hnsw_index_cache WHERE kind = 'cosine'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(kind, "cosine");
    assert_eq!(count, 1);
    assert_eq!(fp, None);
}
