//! `migrate` must pair each removal with a plausible replacement.
//!
//! The third finding on #91: `apply_migrate` paired *every* removed class with
//! `plan.added_classes.first()` and every removed property with
//! `added_properties.first()`. With more than one addition in a plan that
//! fabricates `owl:equivalentClass` bridges between terms that have nothing to
//! do with each other — and an ontology bridge asserting a falsehood is worse
//! than no bridge, because downstream reasoners believe it.

use open_ontologies::graph::GraphStore;
use open_ontologies::plan::Planner;
use open_ontologies::state::StateDb;
use std::sync::Arc;

fn setup() -> (tempfile::TempDir, StateDb, Arc<GraphStore>) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    (tmp, db, Arc::new(GraphStore::new()))
}

fn asks(graph: &GraphStore, subject: &str, pred: &str, object: &str) -> bool {
    let q = format!("ASK {{ <{subject}> <{pred}> <{object}> }}");
    graph.sparql_select(&q).unwrap().contains("true")
}

const EQ_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";

#[test]
fn migrate_does_not_bridge_a_removal_to_an_unrelated_addition() {
    let (_t, db, graph) = setup();
    graph
        .load_turtle(
            r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Automobile a owl:Class .
    "#,
            None,
        )
        .unwrap();

    // Two additions. `Motorcar` is the rename; `Wavelength` is unrelated and
    // happens to sort first, which is all the old heuristic looked at.
    let proposed = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Automobil a owl:Class .
        ex:Wavelength a owl:Class .
    "#;

    Planner::new(db.clone(), graph.clone()).plan(proposed).unwrap();
    Planner::new(db.clone(), graph.clone()).apply("migrate").unwrap();

    assert!(
        !asks(&graph, "http://example.org/Automobile", EQ_CLASS, "http://example.org/Wavelength"),
        "migrate asserted Automobile ≡ Wavelength"
    );
    assert!(
        asks(&graph, "http://example.org/Automobile", EQ_CLASS, "http://example.org/Automobil"),
        "migrate missed the actual rename"
    );
}

#[test]
fn migrate_pairs_each_removal_with_its_own_best_match() {
    let (_t, db, graph) = setup();
    graph
        .load_turtle(
            r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Automobile a owl:Class .
        ex:Bicycle a owl:Class .
    "#,
            None,
        )
        .unwrap();

    let proposed = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Automobil a owl:Class .
        ex:Bicicle a owl:Class .
    "#;

    Planner::new(db.clone(), graph.clone()).plan(proposed).unwrap();
    let applied: serde_json::Value = serde_json::from_str(
        &Planner::new(db.clone(), graph.clone()).apply("migrate").unwrap(),
    )
    .unwrap();

    assert!(asks(&graph, "http://example.org/Automobile", EQ_CLASS, "http://example.org/Automobil"));
    assert!(asks(&graph, "http://example.org/Bicycle", EQ_CLASS, "http://example.org/Bicicle"));
    // No cross-pairing.
    assert!(!asks(&graph, "http://example.org/Automobile", EQ_CLASS, "http://example.org/Bicicle"));
    assert!(!asks(&graph, "http://example.org/Bicycle", EQ_CLASS, "http://example.org/Automobil"));
    assert_eq!(applied["bridges_created"], 2);
}

#[test]
fn migrate_reports_removals_it_could_not_bridge() {
    let (_t, db, graph) = setup();
    graph
        .load_turtle(
            r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Automobile a owl:Class .
    "#,
            None,
        )
        .unwrap();

    // Nothing resembling a rename is on offer.
    let proposed = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Wavelength a owl:Class .
    "#;

    Planner::new(db.clone(), graph.clone()).plan(proposed).unwrap();
    let applied: serde_json::Value = serde_json::from_str(
        &Planner::new(db.clone(), graph.clone()).apply("migrate").unwrap(),
    )
    .unwrap();

    assert_eq!(applied["bridges_created"], 0);
    let unbridged: Vec<String> = applied["unbridged_removals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(unbridged, vec!["http://example.org/Automobile".to_string()]);
}

#[test]
fn migrate_never_reuses_one_addition_for_two_removals() {
    let (_t, db, graph) = setup();
    graph
        .load_turtle(
            r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Automobile a owl:Class .
        ex:Automobiles a owl:Class .
    "#,
            None,
        )
        .unwrap();

    // One plausible target for two similar removals.
    let proposed = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Automobil a owl:Class .
    "#;

    Planner::new(db.clone(), graph.clone()).plan(proposed).unwrap();
    let applied: serde_json::Value = serde_json::from_str(
        &Planner::new(db.clone(), graph.clone()).apply("migrate").unwrap(),
    )
    .unwrap();

    assert_eq!(
        applied["bridges_created"], 1,
        "one addition cannot be the replacement for two different removals: {applied}"
    );
}
