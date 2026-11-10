// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

#[test]
fn disjoint_data_ranges_do_not_collapse_to_unqualified_cardinality() {
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

            :value a owl:DatatypeProperty .
            :StringValued a owl:Class ;
                rdfs:subClassOf [ a owl:Restriction ;
                    owl:onProperty :value ;
                    owl:minQualifiedCardinality "1"^^xsd:nonNegativeInteger ;
                    owl:onDataRange xsd:string ] ;
                rdfs:subClassOf [ a owl:Restriction ;
                    owl:onProperty :value ;
                    owl:maxQualifiedCardinality "0"^^xsd:nonNegativeInteger ;
                    owl:onDataRange xsd:integer ] .
            "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["complete"], false);
    assert!(parsed["constructs_not_modelled"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["construct"] == "datatype restriction"));
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        !unsat
            .iter()
            .any(|v| v.as_str().unwrap_or_default().contains("StringValued")),
        "a string-valued data property can satisfy min 1 xsd:string and max 0 xsd:integer; got {parsed}"
    );
}
