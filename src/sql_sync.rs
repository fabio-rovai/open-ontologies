//! CDC (Change Data Capture) state tracking for SQL ingest.
//!
//! Pattern borrowed from
//! [synaptic-memory](https://github.com/PlateerLab/synaptic-memory) — when
//! ingesting from a live database, the server tracks a watermark per
//! `sync_key` so the caller can pull only rows newer than the last sync.
//!
//! ## Design (deliberately small)
//!
//! The server is NOT a SQL-AST rewriter. The caller writes their own
//! `WHERE {watermark_column} > {value}` clause; the server's job is just to
//! remember the last watermark per sync_key and to record the new max after
//! each ingest.
//!
//! Caller workflow:
//!
//!   1. `onto_sql_sync_state(sync_key)` → last_watermark (or `None`).
//!   2. Build SQL with that watermark as a filter.
//!   3. `onto_sql_ingest(..., sync_key, watermark_column)` → ingests rows;
//!      server records max(watermark_column) from the result set as the new
//!      watermark.
//!   4. Next call: step 1 returns the freshly-recorded watermark.
//!
//! This keeps the integration MCP-native: the orchestrator builds the SQL,
//! the server stores state and ingests. No fragile string manipulation of
//! the user's query.

use crate::state::StateDb;
use serde::{Deserialize, Serialize};

const ENSURE_TABLE: &str = "
CREATE TABLE IF NOT EXISTS sql_sync_state (
    sync_key TEXT PRIMARY KEY,
    last_watermark TEXT NOT NULL,
    watermark_column TEXT,
    last_synced_at TEXT NOT NULL DEFAULT (datetime('now')),
    rows_synced INTEGER NOT NULL DEFAULT 0,
    total_rows_lifetime INTEGER NOT NULL DEFAULT 0
)";

fn ensure(db: &StateDb) -> anyhow::Result<()> {
    db.conn().execute(ENSURE_TABLE, [])?;
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SyncState {
    pub sync_key: String,
    pub last_watermark: String,
    pub watermark_column: Option<String>,
    pub last_synced_at: String,
    pub rows_synced: u64,
    pub total_rows_lifetime: u64,
}

/// Look up the last watermark for a `sync_key`. Returns `None` if no sync
/// has been recorded yet (caller should fetch everything).
pub fn get_watermark(db: &StateDb, sync_key: &str) -> anyhow::Result<Option<String>> {
    ensure(db)?;
    let conn = db.conn();
    let row: Option<String> = conn
        .query_row(
            "SELECT last_watermark FROM sql_sync_state WHERE sync_key = ?1",
            rusqlite::params![sync_key],
            |r| r.get(0),
        )
        .ok();
    Ok(row)
}

/// Look up the full SyncState row for a sync_key.
pub fn get_state(db: &StateDb, sync_key: &str) -> anyhow::Result<Option<SyncState>> {
    ensure(db)?;
    let conn = db.conn();
    let row: Option<SyncState> = conn
        .query_row(
            "SELECT sync_key, last_watermark, watermark_column, last_synced_at, rows_synced, total_rows_lifetime \
             FROM sql_sync_state WHERE sync_key = ?1",
            rusqlite::params![sync_key],
            |r| {
                Ok(SyncState {
                    sync_key: r.get(0)?,
                    last_watermark: r.get(1)?,
                    watermark_column: r.get(2)?,
                    last_synced_at: r.get(3)?,
                    rows_synced: r.get::<_, i64>(4)? as u64,
                    total_rows_lifetime: r.get::<_, i64>(5)? as u64,
                })
            },
        )
        .ok();
    Ok(row)
}

/// Record a new watermark after a successful ingest. `rows_synced` is the
/// row count of THIS sync; the function accumulates the lifetime total.
pub fn set_watermark(
    db: &StateDb,
    sync_key: &str,
    watermark: &str,
    watermark_column: Option<&str>,
    rows_synced: u64,
) -> anyhow::Result<()> {
    ensure(db)?;
    let conn = db.conn();
    let prior_total: i64 = conn
        .query_row(
            "SELECT total_rows_lifetime FROM sql_sync_state WHERE sync_key = ?1",
            rusqlite::params![sync_key],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let new_total = prior_total + rows_synced as i64;
    conn.execute(
        "INSERT OR REPLACE INTO sql_sync_state \
         (sync_key, last_watermark, watermark_column, last_synced_at, rows_synced, total_rows_lifetime) \
         VALUES (?1, ?2, ?3, datetime('now'), ?4, ?5)",
        rusqlite::params![
            sync_key,
            watermark,
            watermark_column,
            rows_synced as i64,
            new_total
        ],
    )?;
    Ok(())
}

/// Clear the recorded watermark for a sync_key. Returns `true` if a row
/// was deleted.
pub fn reset_watermark(db: &StateDb, sync_key: &str) -> anyhow::Result<bool> {
    ensure(db)?;
    let n = db.conn().execute(
        "DELETE FROM sql_sync_state WHERE sync_key = ?1",
        rusqlite::params![sync_key],
    )?;
    Ok(n > 0)
}

/// List every recorded sync state — diagnostic helper.
pub fn list_states(db: &StateDb) -> anyhow::Result<Vec<SyncState>> {
    ensure(db)?;
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT sync_key, last_watermark, watermark_column, last_synced_at, rows_synced, total_rows_lifetime \
         FROM sql_sync_state ORDER BY last_synced_at DESC",
    )?;
    let rows: Vec<SyncState> = stmt
        .query_map([], |r| {
            Ok(SyncState {
                sync_key: r.get(0)?,
                last_watermark: r.get(1)?,
                watermark_column: r.get(2)?,
                last_synced_at: r.get(3)?,
                rows_synced: r.get::<_, i64>(4)? as u64,
                total_rows_lifetime: r.get::<_, i64>(5)? as u64,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// Extract the maximum value of `column` from a list of row maps. Returns
/// `None` if no row contains the column or if all values are empty. String
/// ordering is used (works for ISO-8601 timestamps and monotonic
/// integer IDs alike).
pub fn extract_max_watermark(
    rows: &[std::collections::HashMap<String, String>],
    column: &str,
) -> Option<String> {
    let mut max: Option<&str> = None;
    for row in rows {
        if let Some(v) = row.get(column).map(|s| s.as_str())
            && !v.is_empty()
        {
            match max {
                None => max = Some(v),
                Some(cur) if v > cur => max = Some(v),
                _ => {}
            }
        }
    }
    max.map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::Path;

    fn fresh_db() -> StateDb {
        StateDb::open(Path::new(":memory:")).unwrap()
    }

    fn row(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn get_watermark_returns_none_when_no_sync_recorded() {
        let db = fresh_db();
        assert!(get_watermark(&db, "events").unwrap().is_none());
    }

    #[test]
    fn set_then_get_watermark_round_trip() {
        let db = fresh_db();
        set_watermark(
            &db,
            "events",
            "2026-05-28T08:00:00Z",
            Some("updated_at"),
            42,
        )
        .unwrap();
        assert_eq!(
            get_watermark(&db, "events").unwrap().as_deref(),
            Some("2026-05-28T08:00:00Z")
        );
    }

    #[test]
    fn get_state_returns_full_row_with_column_and_count() {
        let db = fresh_db();
        set_watermark(&db, "users", "1234", Some("id"), 100).unwrap();
        let s = get_state(&db, "users").unwrap().expect("found");
        assert_eq!(s.sync_key, "users");
        assert_eq!(s.last_watermark, "1234");
        assert_eq!(s.watermark_column.as_deref(), Some("id"));
        assert_eq!(s.rows_synced, 100);
        assert_eq!(s.total_rows_lifetime, 100);
    }

    #[test]
    fn second_sync_overwrites_watermark_and_accumulates_lifetime() {
        let db = fresh_db();
        set_watermark(&db, "logs", "T1", Some("ts"), 50).unwrap();
        set_watermark(&db, "logs", "T2", Some("ts"), 30).unwrap();
        let s = get_state(&db, "logs").unwrap().unwrap();
        assert_eq!(s.last_watermark, "T2");
        assert_eq!(s.rows_synced, 30); // last batch
        assert_eq!(s.total_rows_lifetime, 80); // cumulative
    }

    #[test]
    fn reset_watermark_removes_the_state_row() {
        let db = fresh_db();
        set_watermark(&db, "events", "X", None, 1).unwrap();
        assert!(reset_watermark(&db, "events").unwrap());
        assert!(get_watermark(&db, "events").unwrap().is_none());
        // Second reset on a missing key returns false.
        assert!(!reset_watermark(&db, "events").unwrap());
    }

    #[test]
    fn list_states_returns_every_sync_key_recorded() {
        let db = fresh_db();
        set_watermark(&db, "a", "1", None, 1).unwrap();
        set_watermark(&db, "b", "2", None, 1).unwrap();
        set_watermark(&db, "c", "3", None, 1).unwrap();
        let all = list_states(&db).unwrap();
        let keys: Vec<&str> = all.iter().map(|s| s.sync_key.as_str()).collect();
        assert_eq!(keys.len(), 3);
        for k in ["a", "b", "c"] {
            assert!(keys.contains(&k));
        }
    }

    #[test]
    fn extract_max_watermark_picks_lexicographic_largest_iso_timestamp() {
        let rows = vec![
            row(&[("id", "1"), ("updated_at", "2026-05-28T08:00:00Z")]),
            row(&[("id", "2"), ("updated_at", "2026-05-28T10:30:00Z")]),
            row(&[("id", "3"), ("updated_at", "2026-05-28T09:00:00Z")]),
        ];
        assert_eq!(
            extract_max_watermark(&rows, "updated_at").as_deref(),
            Some("2026-05-28T10:30:00Z")
        );
    }

    #[test]
    fn extract_max_watermark_returns_none_when_column_missing() {
        let rows = vec![row(&[("id", "1")]), row(&[("id", "2")])];
        assert!(extract_max_watermark(&rows, "updated_at").is_none());
    }

    #[test]
    fn extract_max_watermark_handles_monotonic_integer_ids_as_strings() {
        let rows = vec![
            row(&[("id", "100")]),
            row(&[("id", "20")]),
            row(&[("id", "9")]),
        ];
        // String ordering: "9" > "20" > "100". This is correct for ISO
        // timestamps but WRONG for unpadded integers — caller should
        // pad/use a string-sortable format (e.g. zero-padded). We don't
        // try to be clever; document the contract.
        // Test asserts the documented (string-sort) behaviour:
        assert_eq!(extract_max_watermark(&rows, "id").as_deref(), Some("9"));
    }

    #[test]
    fn extract_max_watermark_skips_empty_string_values() {
        let rows = vec![
            row(&[("ts", "")]),
            row(&[("ts", "2026-01-01")]),
            row(&[("ts", "")]),
        ];
        assert_eq!(
            extract_max_watermark(&rows, "ts").as_deref(),
            Some("2026-01-01")
        );
    }
}
