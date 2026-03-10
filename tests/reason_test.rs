use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

#[test]
fn test_rdfs_subclass_inference() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Cat rdfs:subClassOf ex:Animal .
        ex:Animal rdfs:subClassOf ex:LivingThing .
        ex:Tabby a ex:Cat .
    "#;
    store.load_turtle(ttl, None).unwrap();

    let result = Reasoner::run(&store, "rdfs", true).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let inferred = parsed["inferred_count"].as_u64().unwrap();
    // Tabby a Animal, Tabby a LivingThing, possibly Cat subClassOf LivingThing
    assert!(inferred >= 2, "Expected at least 2 inferred triples, got {}", inferred);

    // Verify Tabby is inferred as an Animal
    let check = store
        .sparql_select("ASK { <http://example.org/Tabby> a <http://example.org/Animal> }")
        .unwrap();
    assert!(check.contains("true"), "Tabby should be inferred as Animal");

    // Verify Tabby is inferred as a LivingThing
    let check2 = store
        .sparql_select("ASK { <http://example.org/Tabby> a <http://example.org/LivingThing> }")
        .unwrap();
    assert!(
        check2.contains("true"),
        "Tabby should be inferred as LivingThing"
    );
}

#[test]
fn test_rdfs_domain_inference() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:hasName rdfs:domain ex:Entity .
        ex:Alice ex:hasName "Alice" .
    "#;
    store.load_turtle(ttl, None).unwrap();
    Reasoner::run(&store, "rdfs", true).unwrap();
    let check = store
        .sparql_select("ASK { <http://example.org/Alice> a <http://example.org/Entity> }")
        .unwrap();
    assert!(
        check.contains("true"),
        "Alice should be inferred as Entity via domain"
    );
}

#[test]
fn test_rdfs_range_inference() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:livesIn rdfs:range ex:Place .
        ex:Alice ex:livesIn ex:London .
    "#;
    store.load_turtle(ttl, None).unwrap();
    Reasoner::run(&store, "rdfs", true).unwrap();
    let check = store
        .sparql_select("ASK { <http://example.org/London> a <http://example.org/Place> }")
        .unwrap();
    assert!(
        check.contains("true"),
        "London should be inferred as Place via range"
    );
}

#[test]
fn test_rdfs_subproperty_inference() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:hasFather rdfs:subPropertyOf ex:hasParent .
        ex:Bob ex:hasFather ex:John .
    "#;
    store.load_turtle(ttl, None).unwrap();
    Reasoner::run(&store, "rdfs", true).unwrap();
    let check = store
        .sparql_select(
            "ASK { <http://example.org/Bob> <http://example.org/hasParent> <http://example.org/John> }",
        )
        .unwrap();
    assert!(
        check.contains("true"),
        "Bob hasParent John should be inferred via subPropertyOf"
    );
}

#[test]
fn test_reason_no_materialize() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Cat rdfs:subClassOf ex:Animal .
        ex:Tabby a ex:Cat .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let before = store.triple_count();
    let result = Reasoner::run(&store, "rdfs", false).unwrap();
    assert_eq!(
        store.triple_count(),
        before,
        "Store should be unchanged after dry run"
    );
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(
        parsed["inferred_count"].as_u64().unwrap() >= 1,
        "Dry run should report at least 1 inferred triple"
    );
    assert_eq!(parsed["dry_run"], true, "Result should indicate dry_run");
}

#[test]
fn test_owl_transitive_property() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        ex:isAncestorOf a owl:TransitiveProperty .
        ex:Adam ex:isAncestorOf ex:Bob .
        ex:Bob ex:isAncestorOf ex:Carol .
    "#;
    store.load_turtle(ttl, None).unwrap();
    Reasoner::run(&store, "owl-rl", true).unwrap();
    let check = store
        .sparql_select(
            "ASK { <http://example.org/Adam> <http://example.org/isAncestorOf> <http://example.org/Carol> }",
        )
        .unwrap();
    assert!(
        check.contains("true"),
        "Adam isAncestorOf Carol should be inferred via transitivity"
    );
}

#[test]
fn test_owl_symmetric_property() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        ex:friendOf a owl:SymmetricProperty .
        ex:Alice ex:friendOf ex:Bob .
    "#;
    store.load_turtle(ttl, None).unwrap();
    Reasoner::run(&store, "owl-rl", true).unwrap();
    let check = store
        .sparql_select(
            "ASK { <http://example.org/Bob> <http://example.org/friendOf> <http://example.org/Alice> }",
        )
        .unwrap();
    assert!(
        check.contains("true"),
        "Bob friendOf Alice should be inferred via symmetry"
    );
}

#[test]
fn test_owl_inverse_property() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        ex:hasChild owl:inverseOf ex:hasParent .
        ex:Alice ex:hasChild ex:Bob .
    "#;
    store.load_turtle(ttl, None).unwrap();
    Reasoner::run(&store, "owl-rl", true).unwrap();
    let check = store
        .sparql_select(
            "ASK { <http://example.org/Bob> <http://example.org/hasParent> <http://example.org/Alice> }",
        )
        .unwrap();
    assert!(
        check.contains("true"),
        "Bob hasParent Alice should be inferred via inverseOf"
    );
}

#[test]
fn test_owl_sameas_symmetry() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        ex:NYC owl:sameAs ex:NewYork .
    "#;
    store.load_turtle(ttl, None).unwrap();
    Reasoner::run(&store, "owl-rl", true).unwrap();
    let check = store
        .sparql_select(
            "ASK { <http://example.org/NewYork> <http://www.w3.org/2002/07/owl#sameAs> <http://example.org/NYC> }",
        )
        .unwrap();
    assert!(
        check.contains("true"),
        "NewYork sameAs NYC should be inferred via symmetry"
    );
}

#[test]
fn test_fixpoint_convergence() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:A rdfs:subClassOf ex:B .
        ex:B rdfs:subClassOf ex:C .
        ex:C rdfs:subClassOf ex:D .
        ex:instance a ex:A .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let result = Reasoner::run(&store, "rdfs", true).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    // Should have converged in a small number of iterations
    let iterations = parsed["iterations"].as_u64().unwrap();
    assert!(
        iterations <= 5,
        "Should converge quickly, took {} iterations",
        iterations
    );

    // instance should be inferred as B, C, and D
    for class in &["B", "C", "D"] {
        let query = format!(
            "ASK {{ <http://example.org/instance> a <http://example.org/{}> }}",
            class
        );
        let check = store.sparql_select(&query).unwrap();
        assert!(
            check.contains("true"),
            "instance should be inferred as {}",
            class
        );
    }
}

#[test]
fn test_empty_store_no_errors() {
    let store = Arc::new(GraphStore::new());
    let result = Reasoner::run(&store, "rdfs", true).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["inferred_count"].as_u64().unwrap(), 0);
    assert_eq!(parsed["iterations"].as_u64().unwrap(), 1);
}

#[test]
fn test_unknown_profile_defaults_to_rdfs() {
    let store = Arc::new(GraphStore::new());
    let result = Reasoner::run(&store, "unknown-profile", true).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["profile_used"].as_str().unwrap(), "rdfs");
}
