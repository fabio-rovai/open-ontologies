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
