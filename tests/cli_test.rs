use std::process::Command;

fn oo() -> Command {
    Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
}

/// Create an oo() command with an isolated temp data-dir to avoid SQLite lock
/// conflicts when tests run in parallel.
fn oo_isolated(dir: &tempfile::TempDir) -> Command {
    let mut cmd = oo();
    cmd.arg("--data-dir").arg(dir.path());
    cmd
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
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir).arg("stats").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("classes"));
}

#[test]
fn test_cli_clear() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir).arg("clear").output().unwrap();
    assert!(out.status.success());
}

#[test]
fn test_cli_status() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir).arg("status").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"));
}

// ─── Remote + versioning tests ────────────────────────────────────

#[test]
fn test_cli_history_empty() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir).arg("history").output().unwrap();
    assert!(out.status.success());
}

#[test]
fn test_cli_version_and_rollback() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir).args(["version", "test-v1"]).output().unwrap();
    assert!(out.status.success());
}

// ─── Data pipeline tests ─────────────────────────────────────────

#[test]
fn test_cli_reason_empty_store() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir).args(["reason", "--profile", "rdfs"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("inferred") || stdout.contains("triples"));
}

#[test]
fn test_cli_ingest_csv() {
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("data.csv");
    std::fs::write(&csv_path, "name,age\nAlice,30\nBob,25").unwrap();

    let out = oo_isolated(&dir)
        .args(["ingest", csv_path.to_str().unwrap()])
        .output().unwrap();
    assert!(out.status.success());
}

// ─── Lifecycle + clinical tests ──────────────────────────────────

#[test]
fn test_cli_enforce_generic() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir).args(["enforce", "generic"]).output().unwrap();
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

    let out = oo_isolated(&dir).args(["plan", ttl_path.to_str().unwrap()]).output().unwrap();
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
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir).arg("lineage").output().unwrap();
    assert!(out.status.success());
}

#[test]
fn test_cli_monitor_clear() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir).arg("monitor-clear").output().unwrap();
    assert!(out.status.success());
}

#[test]
fn test_cli_align_two_files() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.ttl");
    let target = dir.path().join("target.ttl");

    std::fs::write(&source, r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class ; rdfs:label "Dog" .
        ex:Cat a owl:Class ; rdfs:label "Cat" .
    "#).unwrap();

    std::fs::write(&target, r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix other: <http://other.org/> .
        other:Dog a owl:Class ; rdfs:label "Dog" .
        other:Feline a owl:Class ; rdfs:label "Cat" .
    "#).unwrap();

    let out = oo_isolated(&dir)
        .args(["align", source.to_str().unwrap(), target.to_str().unwrap(), "--min-confidence", "0.5", "--dry-run"])
        .output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("candidates"));
    assert!(stdout.contains("confidence"));
}

#[test]
fn test_cli_align_feedback() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir)
        .args(["align-feedback", "--source", "http://ex.org/Dog", "--target", "http://other.org/Canine", "--accept"])
        .output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"));
}

#[test]
fn test_cli_lint_feedback() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir)
        .args(["lint-feedback", "--rule-id", "missing_label", "--entity", "<http://example.org/Dog>", "--dismiss"])
        .output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"));
    assert!(stdout.contains("lint"));
}

#[test]
fn test_cli_enforce_feedback() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir)
        .args(["enforce-feedback", "--rule-id", "orphan_class", "--entity", "<http://example.org/Thing>", "--accept"])
        .output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"));
    assert!(stdout.contains("enforce"));
}

#[test]
fn test_cli_lint_suppression_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let ttl_path = dir.path().join("test.ttl");
    std::fs::write(&ttl_path, r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#).unwrap();

    // Lint should report issues initially
    let out = oo_isolated(&dir)
        .args(["lint", ttl_path.to_str().unwrap()])
        .output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();

    // Find the entity string for missing_label on Dog
    let issues = v["issues"].as_array().unwrap();
    let dog_issue = issues.iter().find(|i| {
        i["type"].as_str().unwrap_or("") == "missing_label" &&
        i["entity"].as_str().unwrap_or("").contains("example.org/Dog")
    });
    assert!(dog_issue.is_some(), "Should have missing_label for Dog");
    let entity_str = dog_issue.unwrap()["entity"].as_str().unwrap();

    // Dismiss 3 times using exact entity string from lint output
    for _ in 0..3 {
        let out = oo_isolated(&dir)
            .args(["lint-feedback", "--rule-id", "missing_label", "--entity", entity_str, "--dismiss"])
            .output().unwrap();
        assert!(out.status.success());
    }

    // Lint should now show suppressed_count > 0
    let out = oo_isolated(&dir)
        .args(["lint", ttl_path.to_str().unwrap()])
        .output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert!(v["suppressed_count"].as_u64().unwrap() > 0, "suppressed_count should be > 0 after 3 dismissals");
}

// ── #91: plan → apply across the real CLI surface ────────────────────────────
//
// The reporter's repro, run through the shipped binary rather than through a
// held `Planner`. `batch` is a single process; the two-process test covers the
// `plan` and `apply` subcommands, which build a `Planner` each.

fn write_plan_fixtures(dir: &tempfile::TempDir) -> (String, String) {
    let base = dir.path().join("base.ttl");
    let proposed = dir.path().join("proposed.ttl");
    std::fs::write(&base, r#"
        @prefix ex: <https://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Persona a owl:Class ; rdfs:label "Persona" .
    "#).unwrap();
    std::fs::write(&proposed, r#"
        @prefix ex: <https://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Persona a owl:Class ; rdfs:label "Persona" .
        ex:Organizacion a owl:Class ; rdfs:label "Organizacion" .
    "#).unwrap();
    (
        base.to_str().unwrap().to_string(),
        proposed.to_str().unwrap().to_string(),
    )
}

#[test]
fn test_cli_batch_plan_then_apply() {
    let dir = tempfile::tempdir().unwrap();
    let (base, proposed) = write_plan_fixtures(&dir);
    let batch = dir.path().join("batch.txt");
    // Absolute paths on purpose: batch lines are tokenised by
    // `split_command_line`, and a Windows path here is what caught
    // `shell_words::split` eating every separator.
    std::fs::write(
        &batch,
        format!("load {base}\nplan {proposed}\napply safe\nstats\n"),
    )
    .unwrap();

    let out = oo_isolated(&dir)
        .args(["batch", batch.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);

    let lines: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    let apply = lines.iter().find(|l| l["command"] == "apply").unwrap();
    assert!(
        apply["result"]["error"].is_null(),
        "apply failed inside a single batch process: {apply}"
    );
    assert_eq!(apply["result"]["ok"], true);

    // The applied class must actually be in the store afterwards.
    let stats = lines.iter().find(|l| l["command"] == "stats").unwrap();
    assert_eq!(
        stats["result"]["classes"], 2,
        "apply reported success but the store did not change: {stats}"
    );
}

#[test]
fn test_cli_plan_then_apply_separate_processes() {
    let dir = tempfile::tempdir().unwrap();
    let (_base, proposed) = write_plan_fixtures(&dir);

    let plan_out = oo_isolated(&dir)
        .args(["plan", &proposed])
        .output()
        .unwrap();
    assert!(plan_out.status.success());
    let plan: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&plan_out.stdout).trim()).unwrap();
    let plan_id = plan["plan_id"].as_str().expect("plan must emit a plan_id");

    // Fresh process, same data-dir: the plan is in the state db, not in memory.
    let apply_out = oo_isolated(&dir)
        .args(["apply", "--plan-id", plan_id])
        .output()
        .unwrap();
    let apply: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&apply_out.stdout).trim()).unwrap();
    assert!(
        apply["error"].is_null(),
        "apply could not find a plan from a previous process: {apply}"
    );
    assert_eq!(apply["ok"], true);
    assert_eq!(apply["plan_id"].as_str().unwrap(), plan_id);
}

#[test]
fn test_cli_apply_without_plan_reports_no_plan() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir).args(["apply", "safe"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No plan found"),
        "expected a no-plan error, got: {stdout}"
    );
}
