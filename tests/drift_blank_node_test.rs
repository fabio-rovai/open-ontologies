// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

use open_ontologies::drift::DriftDetector;
use open_ontologies::state::StateDb;
use tempfile::NamedTempFile;

fn setup() -> StateDb {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    StateDb::open(&path).unwrap()
}

const PIZZA_WITH_RESTRICTION: &str = r#"
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix ex:   <http://example.org/> .

    ex:Pizza      a owl:Class .
    ex:Topping    a owl:Class .
    ex:hasTopping a owl:ObjectProperty .

    ex:MeatyPizza a owl:Class ;
        owl:equivalentClass [
            a owl:Class ;
            owl:intersectionOf (
                ex:Pizza
                [ a owl:Restriction ;
                  owl:onProperty     ex:hasTopping ;
                  owl:someValuesFrom ex:Topping ]
            )
        ] .
"#;

#[test]
fn drift_of_same_ontology_with_restrictions_returns_no_changes() {
    let db = setup();
    let detector = DriftDetector::new(db);

    let result = detector.detect(PIZZA_WITH_RESTRICTION, PIZZA_WITH_RESTRICTION).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let added    = parsed["added"].as_array().unwrap();
    let removed  = parsed["removed"].as_array().unwrap();
    let renames  = parsed["likely_renames"].as_array().unwrap();
    let velocity = parsed["drift_velocity"].as_f64().unwrap();

    assert!(
        added.is_empty() && removed.is_empty() && renames.is_empty() && velocity < 0.01,
        "expected zero drift between two snapshots of the same ontology; got \
         added={:?}, removed={:?}, renames={:?}, velocity={}",
        added, removed, renames, velocity
    );
}

#[test]
fn real_named_change_detected_alongside_canonicalised_bnodes() {
    // Original spirit of this test (PR #14 era): assert that bnode IRIs don't
    // pollute the named-vocabulary diff. Under RDFC 1.0 canonicalisation we
    // ALLOW bnodes in the diff — they carry deterministic `_:c14n<n>` IDs and
    // represent real semantic content (anonymous restriction classes). The
    // assertion is now twofold:
    //
    //   1. Any bnode IRIs that DO appear use the canonical `_:c14n` prefix
    //      (proving canonicalisation ran), NOT raw parser-generated IDs.
    //   2. The named class rename is still detected.
    let db = setup();
    let detector = DriftDetector::new(db);

    let v2 = r#"
        @prefix owl:  <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex:   <http://example.org/> .

        ex:Pizza      a owl:Class .
        ex:Topping    a owl:Class .
        ex:hasTopping a owl:ObjectProperty .
        ex:VeggiePizza a owl:Class ;
            owl:equivalentClass [
                a owl:Class ;
                owl:intersectionOf (
                    ex:Pizza
                    [ a owl:Restriction ;
                      owl:onProperty     ex:hasTopping ;
                      owl:someValuesFrom ex:Topping ]
                )
            ] .
    "#;

    let result = detector.detect(PIZZA_WITH_RESTRICTION, v2).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    for iri in parsed["added"].as_array().unwrap().iter()
        .chain(parsed["removed"].as_array().unwrap().iter())
    {
        let s = iri.as_str().unwrap();
        if s.starts_with("_:") {
            assert!(
                s.starts_with("_:c14n"),
                "blank-node IRI in diff lacks the canonical `_:c14n` prefix — \
                 canonicalisation may not have run on this snapshot: {}",
                s
            );
        }
    }

    assert!(
        parsed["removed"].as_array().unwrap().iter()
            .any(|v| v.as_str().unwrap().contains("MeatyPizza")),
        "real removal (MeatyPizza) should still be detected"
    );
    assert!(
        parsed["added"].as_array().unwrap().iter()
            .any(|v| v.as_str().unwrap().contains("VeggiePizza")),
        "real addition (VeggiePizza) should still be detected"
    );
}

#[test]
fn canonical_bnode_ids_are_stable_across_independent_reparses() {
    // The reparse-stability guarantee that PR #14's filter approximated by
    // exclusion. Under canonicalisation we get the same property AND keep
    // bnode content: two independent loads of the same Turtle string canonicalise
    // to the same `_:c14n<n>` identifiers, so `detect(x, x)` produces zero noise.
    let db = setup();
    let detector = DriftDetector::new(db);

    // Two structurally distinct ontologies (a wider variety of restriction
    // shapes), each reparsed twice and diffed against itself.
    for ttl in &[PIZZA_WITH_RESTRICTION, r#"
        @prefix owl:  <http://www.w3.org/2002/07/owl#> .
        @prefix ex:   <http://example.org/> .
        ex:Vehicle a owl:Class .
        ex:Engine  a owl:Class .
        ex:hasEngine a owl:ObjectProperty .
        ex:MotorisedVehicle a owl:Class ;
            owl:equivalentClass [
                a owl:Class ;
                owl:intersectionOf (
                    ex:Vehicle
                    [ a owl:Restriction ;
                      owl:onProperty ex:hasEngine ;
                      owl:minCardinality 1 ]
                )
            ] .
    "#] {
        let report = detector.detect(ttl, ttl).expect("detect");
        let parsed: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert!(parsed["added"].as_array().unwrap().is_empty());
        assert!(parsed["removed"].as_array().unwrap().is_empty());
        assert!(parsed["likely_renames"].as_array().unwrap().is_empty());
        assert!(parsed["drift_velocity"].as_f64().unwrap() < 0.01);
    }
}
