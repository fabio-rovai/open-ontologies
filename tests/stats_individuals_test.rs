// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

use open_ontologies::graph::GraphStore;

#[test]
fn stats_do_not_count_anonymous_class_expressions_as_individuals() {
    let graph = GraphStore::new();
    graph
        .load_turtle(
            r#"
                @prefix ex: <https://example.org/> .
                @prefix owl: <http://www.w3.org/2002/07/owl#> .
                @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

                ex:Person a owl:Class ;
                    rdfs:subClassOf [ a owl:Restriction ;
                        owl:onProperty ex:knows ;
                        owl:someValuesFrom ex:Person ] .
                ex:Alice a owl:NamedIndividual, ex:Person .
            "#,
            None,
        )
        .unwrap();

    let stats: serde_json::Value = serde_json::from_str(&graph.get_stats().unwrap()).unwrap();
    assert_eq!(stats["individuals"], 1);
}
