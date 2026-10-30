//! A plan asks whether the change is LOGICALLY safe, not only how big it is (#336).
//!
//! `risk_score` was derived from removal counts alone. A change that made a named
//! class unsatisfiable, or made the ABox inconsistent, removed nothing, touched no
//! existing triple, and scored `low`. Both checks that would have caught it worked
//! when asked (`defects` for the first, `reason --profile owl-dl` for the second);
//! the plan never asked them, and `defects` could not be asked through `batch` at
//! all, so a pipeline running `plan` and `apply` through `batch` promoted both.
//!
//! The two reproductions from the issue are R1 and R2 below. Each is run through
//! the library, the MCP handler, the CLI and `batch`, because those are four entry
//! points and the default has to be the same on all of them.
//!
//! The vocabulary is the repository's: a check that did not run, or ran out of
//! budget, reports `null` with the reason, never `true`, because `true` means "we
//! looked and found nothing".

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::inputs::OntoPlanInput;
use open_ontologies::plan::{PlanChecks, Planner};
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use rmcp::handler::server::wrapper::Parameters;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

/// The step budget is a process-wide setting, and one test here lowers it on
/// purpose. Every test that reasons holds this, so none of them runs under a
/// budget it did not ask for.
static BUDGET: Mutex<()> = Mutex::new(());

fn budget_lock() -> std::sync::MutexGuard<'static, ()> {
    BUDGET.lock().unwrap_or_else(|e| e.into_inner())
}

const BASE: &str = r#"
@prefix ex:   <http://ex.org/> .
@prefix owl:  <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
ex:Agent a owl:Class .
ex:Person a owl:Class ; rdfs:subClassOf ex:Agent .
ex:hasParent a owl:ObjectProperty ; rdfs:domain ex:Person ; rdfs:range ex:Person .
ex:alice a ex:Person .
ex:bob a ex:Person .
"#;

/// R1: Manager is a Person, every Person is an Agent, and Manager is disjoint
/// from Agent. Manager can have no instances.
fn r1() -> String {
    format!(
        "{BASE}\nex:Manager a owl:Class ; rdfs:subClassOf ex:Person .\n\
         ex:Manager owl:disjointWith ex:Agent .\n"
    )
}

/// R2: alice is a Person in the base, Machine is disjoint from Person, and the
/// change says alice is a Machine. No class is unsatisfiable; the ABox is
/// inconsistent.
fn r2() -> String {
    format!(
        "{BASE}\nex:Machine a owl:Class ; owl:disjointWith ex:Person .\n\
         ex:alice a ex:Machine .\n"
    )
}

/// The control: a new subclass and a new individual, nothing contradicted.
fn benign() -> String {
    format!(
        "{BASE}\nex:Employee a owl:Class ; rdfs:subClassOf ex:Person .\n\
         ex:carol a ex:Employee .\n"
    )
}

fn planner_over(base: &str) -> (tempfile::TempDir, Planner) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(base, None).unwrap();
    (tmp, Planner::new(db, graph))
}

fn plan(base: &str, proposed: &str) -> serde_json::Value {
    let _b = budget_lock();
    let (_tmp, planner) = planner_over(base);
    serde_json::from_str(&planner.plan(proposed).unwrap()).unwrap()
}

#[test]
fn r1_a_change_that_makes_a_class_unsatisfiable_is_high_risk() {
    let p = plan(BASE, &r1());
    assert_eq!(p["logic"]["new_defects"].as_u64(), Some(1), "{p:#}");
    let finding = &p["logic"]["new_defect_findings"][0];
    assert_eq!(finding["kind"], "disjoint_with_ancestor", "{p:#}");
    assert_eq!(finding["class"], "<http://ex.org/Manager>", "{p:#}");
    assert_eq!(
        p["risk_score"], "high",
        "a change that leaves a named class with no possible instance is not low risk: {p:#}"
    );
    assert!(
        p["risk_reasons"].as_array().is_some_and(|r| !r.is_empty()),
        "the plan says why the score is high: {p:#}"
    );
}

#[test]
fn r2_a_change_that_makes_the_abox_inconsistent_is_high_risk() {
    let p = plan(BASE, &r2());
    assert_eq!(p["logic"]["new_defects"].as_u64(), Some(0), "not a TBox self-defect: {p:#}");
    assert_eq!(p["logic"]["abox_consistent"], false, "{p:#}");
    assert_eq!(p["logic"]["abox_consistent_before"], true, "{p:#}");
    assert_eq!(p["risk_score"], "high", "{p:#}");
}

#[test]
fn a_benign_addition_stays_low_and_says_it_looked() {
    let p = plan(BASE, &benign());
    assert_eq!(p["logic"]["new_defects"].as_u64(), Some(0), "{p:#}");
    assert_eq!(p["logic"]["abox_consistent"], true, "{p:#}");
    assert_eq!(p["risk_score"], "low", "{p:#}");
}

#[test]
fn a_defect_the_store_already_had_is_not_new() {
    // The change did not cause it, so it must not make every later plan high.
    let broken_base = r1();
    let proposed = format!("{broken_base}\nex:Employee a owl:Class ; rdfs:subClassOf ex:Person .\n");
    let p = plan(&broken_base, &proposed);
    assert_eq!(p["logic"]["new_defects"].as_u64(), Some(0), "{p:#}");
    assert_eq!(p["risk_score"], "low", "{p:#}");
}

#[test]
fn an_inconsistency_the_store_already_had_is_not_a_flip() {
    let broken_base = r2();
    let proposed = format!("{broken_base}\nex:Employee a owl:Class ; rdfs:subClassOf ex:Person .\n");
    let p = plan(&broken_base, &proposed);
    assert_eq!(p["logic"]["abox_consistent"], false, "{p:#}");
    assert_eq!(p["logic"]["abox_consistent_before"], false, "{p:#}");
    assert_eq!(p["risk_score"], "low", "{p:#}");
}

#[test]
fn a_consistency_check_that_runs_out_of_budget_is_null_never_true() {
    let _b = budget_lock();
    let (_tmp, planner) = planner_over(BASE);
    // One step cannot finish any tableau with an individual in it.
    open_ontologies::runtime::set_tableaux_max_steps(1);
    let raw = planner.plan(&benign());
    open_ontologies::runtime::set_tableaux_max_steps(0);
    let p: serde_json::Value = serde_json::from_str(&raw.unwrap()).unwrap();
    assert!(
        p["logic"]["abox_consistent"].is_null(),
        "a check that gave up has not looked, so it must not say true: {p:#}"
    );
    assert!(
        p["logic"]["reason"].as_str().is_some_and(|r| r.contains("budget")),
        "and it says why: {p:#}"
    );
}

#[test]
fn a_construct_the_tableaux_does_not_model_makes_a_clean_answer_null() {
    // owl:sameAs is read by no tableau rule here, so finding no clash over the
    // remaining axioms is not a proof that the whole graph is consistent.
    let proposed = format!("{BASE}\nex:alice owl:sameAs ex:alicia .\n");
    let p = plan(BASE, &proposed);
    assert!(p["logic"]["abox_consistent"].is_null(), "{p:#}");
    assert!(
        p["logic"]["reason"].as_str().is_some_and(|r| r.contains("owl:sameAs")),
        "{p:#}"
    );
}

#[test]
fn the_logic_check_can_be_switched_off_and_then_says_so() {
    let _b = budget_lock();
    let (_tmp, planner) = planner_over(BASE);
    let raw = planner
        .plan_with(&r1(), PlanChecks { conservativity: None, logic: false })
        .unwrap();
    let p: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(p["logic"]["checked_by"], "not_checked", "{p:#}");
    assert!(p["logic"]["new_defects"].is_null(), "{p:#}");
    assert!(p["logic"]["abox_consistent"].is_null(), "{p:#}");
    assert!(p["logic"]["reason"].as_str().is_some_and(|r| !r.is_empty()), "{p:#}");
    // Off means the plan is back to shape alone, and R1 removes nothing.
    assert_eq!(p["risk_score"], "low", "{p:#}");
}

// ─── MCP ────────────────────────────────────────────────────────────────────

fn mcp_plan(proposed: &str, check_logic: Option<bool>) -> serde_json::Value {
    let _b = budget_lock();
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(BASE, None).unwrap();
    let server = OpenOntologiesServer::new_with_graph(db, graph);
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let out = rt.block_on(server.onto_plan(Parameters(OntoPlanInput {
        new_turtle: proposed.to_string(),
        check_conservativity: Some(false),
        conservativity_profile: None,
        conservativity_out_dir: None,
        check_logic,
    })));
    serde_json::from_str(&out).unwrap()
}

#[test]
fn onto_plan_runs_the_logic_check_by_default() {
    let p = mcp_plan(&r1(), None);
    assert_eq!(p["logic"]["new_defects"].as_u64(), Some(1), "{p:#}");
    assert_eq!(p["risk_score"], "high", "{p:#}");
}

#[test]
fn onto_plan_check_logic_false_opts_out() {
    let p = mcp_plan(&r2(), Some(false));
    assert_eq!(p["logic"]["checked_by"], "not_checked", "{p:#}");
    assert!(p["logic"]["abox_consistent"].is_null(), "{p:#}");
}

// ─── CLI and batch ──────────────────────────────────────────────────────────

fn run(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let out = {
        let _gate = common::exec_gate();
        Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .env("OPEN_ONTOLOGIES_STORAGE_MODE", "persistent")
            .arg("--data-dir")
            .arg(dir.join("data"))
            .arg("--no-connect")
            .args(args)
            .output()
            .expect("binary should run")
    };
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn write(dir: &Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p.to_str().unwrap().to_string()
}

fn store_dir() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("data")).unwrap();
    std::fs::write(
        d.path().join("data").join("config.toml"),
        "[storage]\nmode = 'persistent'\n",
    )
    .unwrap();
    d
}

#[test]
fn cli_plan_reports_r1_high_and_keeps_exit_0() {
    let d = store_dir();
    let base = write(d.path(), "base.ttl", BASE);
    let proposed = write(d.path(), "r1.ttl", &r1());
    let (code, out, err) = run(d.path(), &["load", &base]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = run(d.path(), &["plan", &proposed, "--no-conservativity"]);
    let p: serde_json::Value = serde_json::from_str(out.trim()).unwrap_or_else(|e| panic!("{e}: {out}{err}"));
    assert_eq!(p["risk_score"], "high", "{p:#}");
    assert_eq!(p["logic"]["new_defects"].as_u64(), Some(1), "{p:#}");
    // A plan is a report, not a gate: a high score from removals has always
    // exited 0, and a high score from logic does the same. The verdict is in
    // the output for the pipeline to read.
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn cli_plan_no_logic_check_skips_it() {
    let d = store_dir();
    let base = write(d.path(), "base.ttl", BASE);
    let proposed = write(d.path(), "r2.ttl", &r2());
    run(d.path(), &["load", &base]);
    let (code, out, err) = run(
        d.path(),
        &["plan", &proposed, "--no-conservativity", "--no-logic-check"],
    );
    assert_eq!(code, 0, "{out}{err}");
    let p: serde_json::Value = serde_json::from_str(out.trim()).unwrap_or_else(|e| panic!("{e}: {out}{err}"));
    assert_eq!(p["logic"]["checked_by"], "not_checked", "{p:#}");
}

fn batch(d: &Path, lines: &[String]) -> (i32, Vec<serde_json::Value>) {
    let script = write(d, "script.txt", &(lines.join("\n") + "\n"));
    let (code, out, err) = run(d, &["batch", &script]);
    let results = out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{e}: {l}\n{err}")))
        .collect();
    (code, results)
}

#[test]
fn batch_plan_reports_r2_high() {
    let d = store_dir();
    let base = write(d.path(), "base.ttl", BASE);
    let proposed = write(d.path(), "r2.ttl", &r2());
    let (_, results) = batch(
        d.path(),
        &[format!("load {base}"), format!("plan {proposed} --no-conservativity")],
    );
    let p = &results[1]["result"];
    assert_eq!(p["logic"]["abox_consistent"], false, "{p:#}");
    assert_eq!(p["risk_score"], "high", "{p:#}");
}

#[test]
fn batch_plan_no_logic_check_skips_it() {
    let d = store_dir();
    let base = write(d.path(), "base.ttl", BASE);
    let proposed = write(d.path(), "r2.ttl", &r2());
    let (_, results) = batch(
        d.path(),
        &[
            format!("load {base}"),
            format!("plan {proposed} --no-conservativity --no-logic-check"),
        ],
    );
    assert_eq!(results[1]["result"]["logic"]["checked_by"], "not_checked", "{:#}", results[1]);
}

#[test]
fn batch_runs_defects_on_a_file_and_on_the_store() {
    // `defects` had no batch arm, so this answered
    // {"error":"unknown batch command: 'defects'"} and exited 1.
    let d = store_dir();
    let r1_file = write(d.path(), "r1.ttl", &r1());
    let (code, results) = batch(
        d.path(),
        &[
            format!("defects {r1_file}"),
            format!("load {r1_file}"),
            "defects --limit 0".to_string(),
        ],
    );
    assert_eq!(code, 0, "{results:#?}");
    for r in [&results[0]["result"], &results[2]["result"]] {
        assert!(r.get("error").is_none(), "{r:#}");
        assert_eq!(r["severity_counts"]["error"].as_u64(), Some(1), "{r:#}");
        assert_eq!(r["affected_classes"].as_u64(), Some(1), "{r:#}");
    }
    assert_eq!(results[2]["result"]["limit_per_kind"].as_u64(), Some(0), "{:#}", results[2]);
}
