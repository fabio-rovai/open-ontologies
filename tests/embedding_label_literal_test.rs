use open_ontologies::graph::GraphStore;
use oxigraph::model::Term;

#[test]
fn union_term_query_keeps_label_lexical_value_and_language() {
    let graph = GraphStore::new();
    graph
        .load_turtle(
            r#"@prefix : <http://example.org/> .
               @prefix owl: <http://www.w3.org/2002/07/owl#> .
               @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
               :Canid a owl:Class ; rdfs:label "A \"quoted\" label"@en ."#,
            None,
        )
        .unwrap();

    let rows = graph
        .sparql_select_union_terms(
            r#"SELECT DISTINCT ?class ?label WHERE {
                ?class a <http://www.w3.org/2002/07/owl#Class> .
                OPTIONAL { ?class <http://www.w3.org/2000/01/rdf-schema#label> ?label }
                FILTER(isIRI(?class))
            }"#,
        )
        .unwrap();

    assert_eq!(rows.len(), 1);
    let label = match rows[0].get("label") {
        Some(Term::Literal(label)) => label,
        other => panic!("expected RDF literal, got {other:?}"),
    };
    assert_eq!(label.value(), "A \"quoted\" label");
    assert_eq!(label.language(), Some("en"));
}
