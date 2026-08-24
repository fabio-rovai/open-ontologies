//! A plan belongs to whoever computed it.
//!
//! Persisting plans (#91) fixed `apply`, but put every caller's plans in one
//! table. In HTTP mode the MCP server shares a state db across sessions, so
//! "apply the most recent plan" could hand session A the changes session B was
//! still reviewing — a governance tool applying an ontology change nobody
//! approved. Plans are therefore scoped to an owner, and an explicit `plan_id`
//! remains the way to reach across one deliberately.

use open_ontologies::graph::GraphStore;
use open_ontologies::plan::Planner;
use open_ontologies::state::StateDb;
use std::sync::Arc;

fn setup() -> (tempfile::TempDir, StateDb) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    (tmp, db)
}

fn graph_with_base() -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(
        r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Persona a owl:Class .
    "#,
        None,
    )
    .unwrap();
    g
}

const PROPOSED_A: &str = r#"
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix ex: <http://example.org/> .
    ex:Persona a owl:Class .
    ex:Organizacion a owl:Class .
"#;

const PROPOSED_B: &str = r#"
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix ex: <http://example.org/> .
    ex:Persona a owl:Class .
    ex:Vehiculo a owl:Class .
    ex:Bicicleta a owl:Class .
"#;

fn json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap()
}

#[test]
fn one_session_does_not_apply_another_sessions_plan() {
    let (_t, db) = setup();
    let graph = graph_with_base();

    // Session B plans last, so "most recent overall" would be B's.
    let plan_a = json(
        &Planner::with_owner(db.clone(), graph.clone(), "session-a")
            .plan(PROPOSED_A)
            .unwrap(),
    );
    Planner::with_owner(db.clone(), graph.clone(), "session-b")
        .plan(PROPOSED_B)
        .unwrap();

    let applied = json(
        &Planner::with_owner(db.clone(), graph.clone(), "session-a")
            .apply("safe")
            .unwrap(),
    );
    assert_eq!(
        applied["plan_id"].as_str().unwrap(),
        plan_a["plan_id"].as_str().unwrap(),
        "session A applied a plan it did not compute"
    );
    assert_eq!(applied["added_classes"], 1);
}

#[test]
fn a_session_with_no_plan_of_its_own_is_told_so() {
    let (_t, db) = setup();
    let graph = graph_with_base();

    Planner::with_owner(db.clone(), graph.clone(), "session-a")
        .plan(PROPOSED_A)
        .unwrap();

    let err = Planner::with_owner(db.clone(), graph.clone(), "session-b")
        .apply("safe")
        .unwrap_err()
        .to_string();
    assert!(err.contains("No plan found"), "unexpected error: {err}");
    // Silence here would read as "nothing was ever planned", which is false and
    // would send someone hunting the wrong problem.
    assert!(
        err.contains("another session") || err.contains("other owners") || err.contains("1 plan"),
        "the error should say plans exist under a different owner: {err}"
    );
}

#[test]
fn an_explicit_plan_id_still_crosses_owners() {
    let (_t, db) = setup();
    let graph = graph_with_base();

    let plan_a = json(
        &Planner::with_owner(db.clone(), graph.clone(), "session-a")
            .plan(PROPOSED_A)
            .unwrap(),
    );
    let id = plan_a["plan_id"].as_str().unwrap();

    // Naming a plan is a deliberate act, and the audit trail records it.
    let applied = json(
        &Planner::with_owner(db.clone(), graph.clone(), "session-b")
            .apply_plan(Some(id), "safe")
            .unwrap(),
    );
    assert_eq!(applied["plan_id"].as_str().unwrap(), id);
}

#[test]
fn every_cli_invocation_shares_one_owner() {
    let (_t, db) = setup();
    let graph = graph_with_base();

    // `plan` and `apply` are separate processes with no session between them,
    // so the default owner has to be stable rather than per-process.
    let plan = json(&Planner::new(db.clone(), graph.clone()).plan(PROPOSED_A).unwrap());
    let applied = json(&Planner::new(db.clone(), graph.clone()).apply("safe").unwrap());
    assert_eq!(
        applied["plan_id"].as_str().unwrap(),
        plan["plan_id"].as_str().unwrap()
    );
}

#[test]
fn a_server_session_does_not_pick_up_a_cli_plan() {
    let (_t, db) = setup();
    let graph = graph_with_base();

    Planner::new(db.clone(), graph.clone()).plan(PROPOSED_A).unwrap();
    let err = Planner::with_owner(db.clone(), graph.clone(), "session-a")
        .apply("safe")
        .unwrap_err()
        .to_string();
    assert!(err.contains("No plan found"), "unexpected error: {err}");
}
