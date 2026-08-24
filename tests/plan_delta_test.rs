//! `plan` must see the ABox, and `apply` must write a delta rather than
//! clearing the store.
//!
//! The secondary findings on #91. `plan()` only ever queried `?c a owl:Class`
//! and `?p a owl:{Object,Datatype}Property`, so instance data was invisible to
//! it — while `apply()` did `graph.clear()` + `load_turtle(new_turtle)`, which
//! deleted that same invisible instance data wholesale. A plan that cannot
//! mention what an apply destroys is not a plan.

use open_ontologies::graph::GraphStore;
use open_ontologies::plan::Planner;
use open_ontologies::state::StateDb;
use std::sync::Arc;

fn setup() -> (tempfile::TempDir, StateDb, Arc<GraphStore>) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    (tmp, db, Arc::new(GraphStore::new()))
}

fn json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap()
}

fn strs(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

const WITH_TWO_INDIVIDUALS: &str = r#"
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix ex: <http://example.org/> .
    ex:Persona a owl:Class .
    ex:ana a ex:Persona .
    ex:beto a ex:Persona .
"#;

#[test]
fn plan_reports_added_individuals() {
    let (_t, db, graph) = setup();
    graph
        .load_turtle(
            r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Persona a owl:Class .
        ex:ana a ex:Persona .
    "#,
            None,
        )
        .unwrap();

    let plan = json(&Planner::new(db, graph).plan(WITH_TWO_INDIVIDUALS).unwrap());
    let added = strs(&plan["added_individuals"]);
    assert_eq!(added, vec!["http://example.org/beto".to_string()]);
    assert!(strs(&plan["removed_individuals"]).is_empty());
}

#[test]
fn plan_reports_individuals_an_apply_would_destroy() {
    let (_t, db, graph) = setup();
    graph.load_turtle(WITH_TWO_INDIVIDUALS, None).unwrap();

    // Proposed Turtle carries the TBox but drops the instance data — exactly
    // the case where the old plan output said "no changes" and apply then
    // deleted both individuals.
    let tbox_only = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Persona a owl:Class .
    "#;

    let plan = json(&Planner::new(db, graph).plan(tbox_only).unwrap());
    let mut removed = strs(&plan["removed_individuals"]);
    removed.sort();
    assert_eq!(
        removed,
        vec![
            "http://example.org/ana".to_string(),
            "http://example.org/beto".to_string()
        ]
    );
    assert_ne!(
        plan["risk_score"].as_str().unwrap(),
        "low",
        "dropping instance data is not a low-risk change: {plan}"
    );
}

#[test]
fn plan_reports_the_triple_level_delta() {
    let (_t, db, graph) = setup();
    graph
        .load_turtle(
            r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Persona a owl:Class .
        ex:ana a ex:Persona .
    "#,
            None,
        )
        .unwrap();

    let plan = json(&Planner::new(db, graph).plan(WITH_TWO_INDIVIDUALS).unwrap());
    assert_eq!(plan["triple_delta"]["insertions"], 1);
    assert_eq!(plan["triple_delta"]["deletions"], 0);
}

#[test]
fn apply_writes_a_delta_instead_of_clearing_the_store() {
    let (_t, db, graph) = setup();
    graph
        .load_turtle(
            r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Persona a owl:Class .
        ex:ana a ex:Persona .
    "#,
            None,
        )
        .unwrap();

    Planner::new(db.clone(), graph.clone())
        .plan(WITH_TWO_INDIVIDUALS)
        .unwrap();
    let applied = json(
        &Planner::new(db.clone(), graph.clone())
            .apply("safe")
            .unwrap(),
    );

    assert_eq!(applied["strategy"], "delta");
    assert_eq!(applied["triples_inserted"], 1);
    assert_eq!(applied["triples_deleted"], 0);

    // And the store really is the proposed state.
    let ask = "ASK { <http://example.org/beto> a <http://example.org/Persona> }";
    assert!(graph.sparql_select(ask).unwrap().contains("true"));
    assert_eq!(graph.triple_count(), 3);
}

#[test]
fn apply_deletes_individuals_absent_from_the_proposed_turtle() {
    let (_t, db, graph) = setup();
    graph.load_turtle(WITH_TWO_INDIVIDUALS, None).unwrap();

    let tbox_only = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Persona a owl:Class .
    "#;
    Planner::new(db.clone(), graph.clone()).plan(tbox_only).unwrap();
    let applied = json(&Planner::new(db.clone(), graph.clone()).apply("safe").unwrap());

    assert_eq!(applied["triples_deleted"], 2);
    assert_eq!(graph.triple_count(), 1);
}

#[test]
fn apply_falls_back_to_reload_when_blank_nodes_are_involved() {
    let (_t, db, graph) = setup();
    graph
        .load_turtle(
            r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Persona a owl:Class .
    "#,
            None,
        )
        .unwrap();

    // An anonymous restriction: blank-node labels are store-local, so a
    // triple-set difference over them is meaningless and DELETE DATA /
    // INSERT DATA cannot carry them at all.
    let with_restriction = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Persona a owl:Class .
        ex:Padre a owl:Class ; rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:tieneHijo ;
            owl:minCardinality 1
        ] .
    "#;

    Planner::new(db.clone(), graph.clone()).plan(with_restriction).unwrap();
    let applied = json(&Planner::new(db.clone(), graph.clone()).apply("safe").unwrap());

    assert_eq!(applied["strategy"], "reload");
    assert_eq!(applied["ok"], true);
    let ask = "ASK { <http://example.org/Padre> a <http://www.w3.org/2002/07/owl#Class> }";
    assert!(graph.sparql_select(ask).unwrap().contains("true"));
}
