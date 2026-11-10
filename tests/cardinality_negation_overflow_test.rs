// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

use open_ontologies::tableaux::Concept;
use open_ontologies::{graph::GraphStore, reason::Reasoner};
use std::sync::Arc;

#[test]
fn negating_the_largest_representable_max_cardinality_does_not_wrap() {
    let max = Concept::MaxCard(1, u32::MAX, Box::new(Concept::Top));
    assert_eq!(
        max.negate(),
        Concept::MinCard(1, u32::MAX, Box::new(Concept::Top)),
        "the complement needs one more successor; saturating preserves an unrepresentable-large requirement instead of wrapping to zero"
    );
}

#[test]
fn reasoning_with_max_cardinality_stays_incomplete_instead_of_panicking() {
    open_ontologies::runtime::set_tableaux_test_timeout_ms(0);
    open_ontologies::runtime::set_classify_timeout_ms(0);
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
            @prefix : <http://example.org/> .
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
            :r a owl:ObjectProperty .
            :A a owl:Class ; owl:equivalentClass [ a owl:Restriction ;
                owl:onProperty :r ;
                owl:maxCardinality "4294967295"^^xsd:nonNegativeInteger ] .
            :B a owl:Class .
            "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["algorithm"], "tableaux");
    assert_eq!(
        parsed["complete"], false,
        "the minimum above the representable max threshold must reach a resource bound, not wrap to zero: {parsed}"
    );
}
