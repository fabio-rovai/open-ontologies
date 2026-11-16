// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! SHACL-SPARQL reads user data, including inferred facts, but not the ledger.

use open_ontologies::graph::{GraphStore, ReadScope};
use open_ontologies::reason::{InferenceTarget, Reasoner};
use open_ontologies::shacl::ShaclValidator;
use open_ontologies::temporal::ScopeRequest;
use open_ontologies::truth_maintenance::LEDGER_GRAPH;
use oxigraph::model::{NamedNode, Term};
use std::sync::Arc;

const DATA: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    ex:A rdfs:subClassOf ex:B .
    ex:x a ex:A ; ex:p ex:y .
"#;

const SHAPES: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:LedgerProbe a sh:NodeShape ; sh:targetClass ex:A ;
        sh:sparql [ sh:message "ledger is visible" ;
            sh:select """SELECT $this ?p WHERE {
                ?s ?p ?o .
                FILTER(STRSTARTS(STR(?p), "https://open-ontologies.org/provenance#"))
            }""" ] .
    ex:Control a sh:NodeShape ; sh:targetClass ex:B ;
        sh:sparql [ sh:message "control: ex:p is present" ;
            sh:select "SELECT $this WHERE { $this <http://example.org/p> ?o }" ] .
"#;

fn store(tracked: bool) -> Arc<GraphStore> {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(DATA, None).unwrap();
    Reasoner::run_scoped_with_tracking(
        &graph,
        "rdfs",
        true,
        InferenceTarget::DefaultGraph,
        None,
        &ScopeRequest::Unscoped,
        tracked,
    )
    .unwrap();
    assert_eq!(graph.ledger().unwrap().is_some(), tracked);
    graph
}

#[test]
fn tracking_does_not_add_shacl_sparql_violations() {
    for tracked in [false, true] {
        let graph = store(tracked);
        for request in [ScopeRequest::Unscoped, ScopeRequest::AllVersions] {
            let report: serde_json::Value = serde_json::from_str(
                &ShaclValidator::validate_scoped(&graph, SHAPES, &request).unwrap(),
            )
            .unwrap();
            assert_eq!(report["conforms"], false, "tracked={tracked}: {report}");
            assert_eq!(report["violation_count"], 1, "tracked={tracked}: {report}");
            assert_eq!(
                report["violations"][0]["message"],
                "control: ex:p is present"
            );
            assert_eq!(
                report["violations"][0]["focus_node"],
                "http://example.org/x"
            );
            assert!(report["skipped_constraints"].is_null(), "{report}");
        }
    }
}

#[test]
fn scoped_prebinding_hides_metadata_and_keeps_user_graph_boundaries() {
    let graph = store(true);
    graph
        .sparql_update(
            "INSERT DATA { GRAPH <http://example.org/source> {
                <http://example.org/x> <http://example.org/p> <http://example.org/z>
            } }",
        )
        .unwrap();
    let terms: Vec<Term> = ["http://example.org/x", "http://example.org/absent"]
        .into_iter()
        .map(|iri| NamedNode::new(iri).unwrap().into())
        .collect();
    let source = "http://example.org/source".to_string();
    let scopes = [
        (ReadScope::AllGraphs, vec!["y", "z"]),
        (
            ReadScope::Graphs {
                default_graph: true,
                named: vec![],
            },
            vec!["y"],
        ),
        (
            ReadScope::Graphs {
                default_graph: false,
                named: vec![source.clone()],
            },
            vec!["z"],
        ),
        (
            ReadScope::Graphs {
                default_graph: true,
                named: vec![source, LEDGER_GRAPH.into()],
            },
            vec!["y", "z"],
        ),
        (
            ReadScope::Graphs {
                default_graph: false,
                named: vec![LEDGER_GRAPH.into()],
            },
            vec![],
        ),
    ];
    let probes = [
        "SELECT $this WHERE { ?s ?p ?o FILTER(STRSTARTS(STR(?p), 'https://open-ontologies.org/provenance#')) }".to_string(),
        format!("SELECT $this WHERE {{ GRAPH <{LEDGER_GRAPH}> {{ ?s ?p ?o }} }}"),
        format!("SELECT $this WHERE {{ GRAPH ?g {{ ?s ?p ?o }} FILTER(?g = <{LEDGER_GRAPH}>) }}"),
    ];
    for (scope, expected) in scopes {
        let rows = graph
            .sparql_select_scoped_prebound(
                "SELECT $this ?o WHERE { $this <http://example.org/p> ?o }",
                "this",
                &terms,
                &scope,
            )
            .unwrap();
        assert_eq!(rows.len(), terms.len());
        assert!(
            rows[1].is_empty(),
            "the absent focus node must have no matches"
        );
        let mut actual: Vec<_> = rows[0].iter().map(|row| row["o"].clone()).collect();
        actual.sort();
        let expected: Vec<_> = expected
            .iter()
            .map(|name| format!("<http://example.org/{name}>"))
            .collect();
        assert_eq!(actual, expected, "scope={scope:?}");
        for row in &rows[0] {
            assert_eq!(row["this"], terms[0].to_string());
        }
        for query in &probes {
            let rows = graph
                .sparql_select_scoped_prebound(query, "this", &terms, &scope)
                .unwrap();
            assert!(
                rows.iter().all(Vec::is_empty),
                "scope={scope:?}, query={query}: {rows:?}"
            );
        }
    }
}
