//! Plans must outlive the `Planner` that computed them.
//!
//! Regression coverage for #91. Every real call site — the `plan`/`apply` CLI
//! subcommands, `batch`'s `exec_plan`/`exec_apply`, and the `onto_plan` /
//! `onto_apply` MCP handlers — constructs a *fresh* `Planner` per invocation.
//! While the plan lived in a `RefCell` on the instance, `apply` could never see
//! it, and `apply` was dead in every shipped interface.
//!
//! The pre-existing suites (`plan_test.rs`, `terraform_loop_test.rs`) all held
//! one `Planner` across both calls, so they exercised a lifetime shape no
//! caller has. These tests deliberately do not: each one crosses the same
//! boundary a real caller crosses.

use open_ontologies::graph::GraphStore;
use open_ontologies::plan::Planner;
use open_ontologies::state::StateDb;
use std::sync::Arc;

fn setup() -> (tempfile::TempDir, StateDb, Arc<GraphStore>) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    (tmp, db, Arc::new(GraphStore::new()))
}

const BASE: &str = r#"
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix ex: <https://example.org/> .
    ex:Persona a owl:Class ; rdfs:label "Persona" .
"#;

const PROPOSED: &str = r#"
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix ex: <https://example.org/> .
    ex:Persona a owl:Class ; rdfs:label "Persona" .
    ex:Organizacion a owl:Class ; rdfs:label "Organizacion" .
"#;

fn json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap()
}

#[test]
fn a_plan_survives_into_a_separately_constructed_planner() {
    let (_tmp, db, graph) = setup();
    graph.load_turtle(BASE, None).unwrap();

    // Two Planners, exactly as batch's exec_plan/exec_apply and the MCP
    // handlers build them: same db, same graph, different instances.
    let plan = json(&Planner::new(db.clone(), graph.clone()).plan(PROPOSED).unwrap());
    assert_eq!(plan["added_classes"].as_array().unwrap().len(), 1);

    let applied = Planner::new(db.clone(), graph.clone())
        .apply("safe")
        .expect("apply must see the plan a different Planner computed");

    let applied = json(&applied);
    assert_eq!(applied["ok"], true);
    assert_eq!(applied["added_classes"], 1);
}

#[test]
fn plan_returns_an_id_that_apply_can_target() {
    let (_tmp, db, graph) = setup();
    graph.load_turtle(BASE, None).unwrap();

    let plan = json(&Planner::new(db.clone(), graph.clone()).plan(PROPOSED).unwrap());
    let plan_id = plan["plan_id"].as_str().expect("plan() must return a plan_id").to_string();

    let applied = json(
        &Planner::new(db.clone(), graph.clone())
            .apply_plan(Some(&plan_id), "safe")
            .unwrap(),
    );
    assert_eq!(applied["ok"], true);
    assert_eq!(applied["plan_id"].as_str().unwrap(), plan_id);
}

#[test]
fn an_older_plan_is_still_reachable_by_id_after_a_newer_one() {
    let (_tmp, db, graph) = setup();
    graph.load_turtle(BASE, None).unwrap();

    let first = json(&Planner::new(db.clone(), graph.clone()).plan(PROPOSED).unwrap());
    let first_id = first["plan_id"].as_str().unwrap().to_string();

    // A second, different plan supersedes it as "latest".
    let second_turtle = format!("{PROPOSED}\n@prefix ex2: <https://example.org/x/> .\nex2:Tercera a <http://www.w3.org/2002/07/owl#Class> .");
    let second = json(&Planner::new(db.clone(), graph.clone()).plan(&second_turtle).unwrap());
    assert_ne!(second["plan_id"].as_str().unwrap(), first_id);

    // Targeting the older id applies the older plan, not the newer one.
    let applied = json(
        &Planner::new(db.clone(), graph.clone())
            .apply_plan(Some(&first_id), "safe")
            .unwrap(),
    );
    assert_eq!(applied["plan_id"].as_str().unwrap(), first_id);
    assert_eq!(applied["added_classes"], 1);
}

#[test]
fn the_most_recent_plan_is_the_one_applied_when_no_id_is_given() {
    let (_tmp, db, graph) = setup();
    graph.load_turtle(BASE, None).unwrap();

    Planner::new(db.clone(), graph.clone()).plan(PROPOSED).unwrap();
    let second_turtle = format!("{PROPOSED}\nex:Cuarta a <http://www.w3.org/2002/07/owl#Class> .");
    let second = json(&Planner::new(db.clone(), graph.clone()).plan(&second_turtle).unwrap());
    let second_id = second["plan_id"].as_str().unwrap().to_string();

    let applied = json(&Planner::new(db.clone(), graph.clone()).apply("safe").unwrap());
    assert_eq!(applied["plan_id"].as_str().unwrap(), second_id);
    assert_eq!(applied["added_classes"], 2);
}

#[test]
fn apply_with_no_plan_at_all_still_fails() {
    let (_tmp, db, graph) = setup();
    let err = Planner::new(db, graph).apply("safe").unwrap_err();
    assert!(
        err.to_string().contains("No plan found"),
        "unexpected error: {err}"
    );
}

#[test]
fn apply_with_an_unknown_plan_id_fails_rather_than_falling_back() {
    let (_tmp, db, graph) = setup();
    graph.load_turtle(BASE, None).unwrap();
    Planner::new(db.clone(), graph.clone()).plan(PROPOSED).unwrap();

    // A stored plan exists, but not this one. Silently applying the latest
    // instead would apply changes the caller never asked for.
    let err = Planner::new(db.clone(), graph.clone())
        .apply_plan(Some("does-not-exist"), "safe")
        .unwrap_err();
    assert!(err.to_string().contains("does-not-exist"), "unexpected error: {err}");
}

#[test]
fn a_plan_survives_reopening_the_state_database() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("state.db");
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(BASE, None).unwrap();

    let plan_id = {
        let db = StateDb::open(&db_path).unwrap();
        let plan = json(&Planner::new(db, graph.clone()).plan(PROPOSED).unwrap());
        plan["plan_id"].as_str().unwrap().to_string()
    };

    // A separate process is a separate StateDb over the same file.
    let db = StateDb::open(&db_path).unwrap();
    let applied = json(&Planner::new(db, graph.clone()).apply_plan(Some(&plan_id), "safe").unwrap());
    assert_eq!(applied["ok"], true);
}

#[test]
fn migrate_mode_also_reads_the_persisted_plan() {
    let (_tmp, db, graph) = setup();
    graph.load_turtle(BASE, None).unwrap();

    Planner::new(db.clone(), graph.clone()).plan(PROPOSED).unwrap();
    let applied = json(&Planner::new(db.clone(), graph.clone()).apply("migrate").unwrap());
    assert_eq!(applied["ok"], true);
    assert_eq!(applied["mode"], "migrate");
}
