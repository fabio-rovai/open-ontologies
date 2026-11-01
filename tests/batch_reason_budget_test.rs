//! A `reason` line in a batch takes the same budgets as the one-shot command.
//!
//! `batch` accepted `reason --phase-timeout-ms N --classify-timeout-ms M` and
//! dropped both flags without a word, so the run used the configured budgets
//! and its `budget` block reported those (#352). Batch is the one entry point
//! that can load and reason in one process, so it was also the one place a
//! caller could not bound the tableau, and nothing in the output said so.
//!
//! The end-to-end cases run the binary with `--no-connect`, so each batch owns
//! its process and none of them can move another test's reasoner clocks.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

use open_ontologies::batch::BatchRunner;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;

const ONTOLOGY: &str = r#"
@prefix :     <http://example.org/t#> .
@prefix owl:  <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
<http://example.org/t> a owl:Ontology .
:A a owl:Class ; rdfs:subClassOf :B .
:B a owl:Class .
:C a owl:Class .
"#;

/// Run `batch -` locally over `lines`, one envelope per line of output.
fn batch(dir: &Path, lines: &[&str]) -> Vec<serde_json::Value> {
    use std::io::Write as _;
    let mut child = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .arg("--no-connect")
        .arg("--data-dir")
        .arg(dir.join("data"))
        .args(["batch", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary should run");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(lines.join("\n").as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str(l).unwrap_or_else(|e| {
                panic!(
                    "batch printed a line that is not JSON ({e}): {l}\nstderr: {}",
                    String::from_utf8_lossy(&out.stderr)
                )
            })
        })
        .collect()
}

fn ontology(dir: &Path) -> String {
    let p = dir.join("t.ttl");
    std::fs::write(&p, ONTOLOGY).unwrap();
    p.to_str().unwrap().to_string()
}

#[test]
fn a_batch_reason_line_runs_at_the_budgets_it_names_and_only_that_line_does() {
    let d = tempfile::tempdir().unwrap();
    let load = format!("load {}", ontology(d.path()));
    let out = batch(
        d.path(),
        &[
            &load,
            "reason --profile owl-dl",
            "reason --profile owl-dl --phase-timeout-ms 7 --classify-timeout-ms 9",
            "reason --profile owl-dl",
        ],
    );
    assert_eq!(out.len(), 4, "{out:#?}");
    let budget = |i: usize| out[i]["result"]["budget"].clone();

    assert_eq!(budget(2)["phase_timeout_ms"], 7, "{:#}", out[2]);
    assert_eq!(budget(2)["classify_timeout_ms"], 9, "{:#}", out[2]);

    // The flags hold for their own line. The line after it runs at the budgets
    // the batch started with, which are whatever the one before it ran at.
    assert_ne!(budget(1)["phase_timeout_ms"], 7, "{:#}", out[1]);
    assert_eq!(budget(3)["phase_timeout_ms"], budget(1)["phase_timeout_ms"], "{out:#?}");
    assert_eq!(budget(3)["classify_timeout_ms"], budget(1)["classify_timeout_ms"], "{out:#?}");
}

#[test]
fn a_budget_flag_without_a_whole_number_is_an_error_and_not_a_dropped_flag() {
    let d = tempfile::tempdir().unwrap();
    let load = format!("load {}", ontology(d.path()));
    for line in [
        "reason --profile owl-dl --phase-timeout-ms soon",
        "reason --profile owl-dl --classify-timeout-ms -1",
        "reason --profile owl-dl --phase-timeout-ms",
    ] {
        let out = batch(d.path(), &[&load, line]);
        let err = out[1]["result"]["error"].as_str().unwrap_or_default();
        assert!(
            err.contains("--phase-timeout-ms") || err.contains("--classify-timeout-ms"),
            "`{line}` must fail and name its flag, not run at the default budget: {:#}",
            out[1]
        );
    }
}

/// The daemon serves every client from one process and fixes its budgets when
/// it starts, so a line that would move them is refused there, as the one-shot
/// `reason` is refused by `daemon_refuses`. Asked of the runner directly: no
/// clock is changed on the way to the refusal, so this cannot disturb anything
/// else in this binary.
#[tokio::test]
async fn the_daemon_refuses_a_batch_reason_line_that_sets_a_budget() {
    let d = tempfile::tempdir().unwrap();
    let db = StateDb::open(&d.path().join("state.db")).unwrap();
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ONTOLOGY, None).unwrap();
    let runner = BatchRunner::new(db, store, false).in_daemon(true);
    let (results, code) = runner
        .run_collect("reason --profile owl-dl --phase-timeout-ms 7", false)
        .await;
    let err = results[0]["result"]["error"].as_str().unwrap_or_default();
    assert!(err.contains("daemon") && err.contains("--no-connect"), "{results:#?}");
    assert_eq!(code, 1);

    // Without a budget flag the same daemon runs the line.
    let db = StateDb::open(&d.path().join("state2.db")).unwrap();
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ONTOLOGY, None).unwrap();
    let runner = BatchRunner::new(db, store, false).in_daemon(true);
    let (results, code) = runner.run_collect("reason --profile owl-dl", false).await;
    assert!(results[0]["result"].get("error").is_none(), "{results:#?}");
    assert_eq!(code, 0);
}
