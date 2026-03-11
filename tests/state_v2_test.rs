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
