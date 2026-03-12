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

// ─── Remote + versioning tests ────────────────────────────────────

#[test]
fn test_cli_history_empty() {
    let out = oo().arg("history").output().unwrap();
    assert!(out.status.success());
}

#[test]
fn test_cli_version_and_rollback() {
    let out = oo().args(["version", "test-v1"]).output().unwrap();
    assert!(out.status.success());
}

// ─── Data pipeline tests ─────────────────────────────────────────

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

// ─── Lifecycle + clinical tests ──────────────────────────────────

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
