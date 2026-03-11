use crate::state::StateDb;
use chrono::Utc;

/// Append-only lineage log. Compressed format for AI consumption.
pub struct LineageLog {
    db: StateDb,
}

impl LineageLog {
    pub fn new(db: StateDb) -> Self {
        Self { db }
    }

    /// Generate a new session ID (short hex).
    pub fn new_session(&self) -> String {
        format!("{:08x}", rand_id())
    }

    /// Record a lineage event.
    /// Format: session:seq:timestamp:event_type:operation:details
    pub fn record(&self, session_id: &str, event_type: &str, operation: &str, details: &str) {
        let conn = self.db.conn();
        // Get next seq for this session
        let seq: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM lineage_events WHERE session_id = ?1",
                rusqlite::params![session_id],
                |r| r.get(0),
            )
            .unwrap_or(1);
        let ts = Utc::now().timestamp();
        let _ = conn.execute(
            "INSERT INTO lineage_events (session_id, seq, timestamp, event_type, operation, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![session_id, seq, ts.to_string(), event_type, operation, details],
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
