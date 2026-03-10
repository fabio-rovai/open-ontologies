use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

// ── Satisfiability ──────────────────────────────────────────────────────

#[test]
fn test_dl_simple_class_satisfiable() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        ex:Animal a owl:Class .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["profile_used"], "owl-dl");
    assert_eq!(parsed["algorithm"], "tableaux");
    assert_eq!(parsed["consistent"], true);
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(unsat.is_empty(), "No classes should be unsatisfiable");
}

#[test]
fn test_dl_unsatisfiable_class() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:A a owl:Class .
        ex:B a owl:Class .
        ex:A rdfs:subClassOf ex:B .
        ex:A owl:disjointWith ex:B .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        !unsat.is_empty(),
        "A should be unsatisfiable (subclass of B but disjoint with B)"
    );
}

// ── Subsumption ─────────────────────────────────────────────────────────

#[test]
fn test_dl_told_subsumption() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Cat a owl:Class .
        ex:Animal a owl:Class .
        ex:LivingThing a owl:Class .
        ex:Cat rdfs:subClassOf ex:Animal .
        ex:Animal rdfs:subClassOf ex:LivingThing .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", true).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);

    // Cat should be classified under both Animal and LivingThing
    let classification = parsed["classification"].as_array().unwrap();
    let cat_entry = classification
        .iter()
        .find(|e| {
            e["class"]
                .as_str()
                .unwrap()
                .contains("Cat")
        })
        .expect("Cat should be in classification");
    let supers: Vec<&str> = cat_entry["superclasses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        supers.iter().any(|s| s.contains("Animal")),
        "Cat should have Animal as superclass"
    );
    assert!(
        supers.iter().any(|s| s.contains("LivingThing")),
        "Cat should have LivingThing as superclass (transitive)"
    );
}

// ── Complement / Negation ───────────────────────────────────────────────

#[test]
fn test_dl_complement_reasoning() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Meat a owl:Class .
        ex:NonMeat a owl:Class .
        ex:NonMeat owl:equivalentClass _:comp .
        _:comp owl:complementOf ex:Meat .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
}

// ── Existential + Universal interaction ─────────────────────────────────

#[test]
fn test_dl_exists_forall_clash() {
    // If Pizza ⊑ ∀hasTopping.VegTopping and
    // NonVegPizza ⊑ Pizza ⊓ ∃hasTopping.MeatTopping and
    // MeatTopping disjointWith VegTopping
    // then NonVegPizza should be unsatisfiable
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:VegTopping a owl:Class .
        ex:MeatTopping a owl:Class .
        ex:Pizza a owl:Class .
        ex:NonVegPizza a owl:Class .
        ex:hasTopping a owl:ObjectProperty .

        ex:VegTopping owl:disjointWith ex:MeatTopping .

        ex:Pizza rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasTopping ;
            owl:allValuesFrom ex:VegTopping
        ] .

        ex:NonVegPizza rdfs:subClassOf ex:Pizza .
        ex:NonVegPizza rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasTopping ;
            owl:someValuesFrom ex:MeatTopping
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    let unsat_names: Vec<&str> = unsat.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(
        unsat_names.iter().any(|n| n.contains("NonVegPizza")),
        "NonVegPizza should be unsatisfiable: {:?}",
        unsat_names
    );
}

// ── Disjunction (unionOf) ───────────────────────────────────────────────

#[test]
fn test_dl_union_satisfiable() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:Cat a owl:Class .
        ex:Dog a owl:Class .
        ex:Pet a owl:Class .
        ex:Pet owl:equivalentClass [
            owl:unionOf ( ex:Cat ex:Dog )
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(unsat.is_empty(), "All classes should be satisfiable");
}

// ── Intersection (intersectionOf) ───────────────────────────────────────

#[test]
fn test_dl_intersection_subsumption() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:Male a owl:Class .
        ex:Parent a owl:Class .
        ex:Father a owl:Class .
        ex:Father owl:equivalentClass [
            owl:intersectionOf ( ex:Male ex:Parent )
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);

    // Father should be subclass of both Male and Parent
    let classification = parsed["classification"].as_array().unwrap();
    let father = classification
        .iter()
        .find(|e| e["class"].as_str().unwrap().contains("Father"));
    assert!(father.is_some(), "Father should be in classification");
    if let Some(f) = father {
        let supers: Vec<&str> = f["superclasses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(
            supers.iter().any(|s| s.contains("Male")),
            "Father ⊑ Male"
        );
        assert!(
            supers.iter().any(|s| s.contains("Parent")),
            "Father ⊑ Parent"
        );
    }
}

// ── Equivalence detection ───────────────────────────────────────────────

#[test]
fn test_dl_equivalence_detection() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:A a owl:Class .
        ex:B a owl:Class .
        ex:A owl:equivalentClass ex:B .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let equivs = parsed["equivalences"].as_array().unwrap();
    assert!(!equivs.is_empty(), "Should detect A ≡ B");
}

// ── Empty store ─────────────────────────────────────────────────────────

#[test]
fn test_dl_empty_store() {
    let store = Arc::new(GraphStore::new());
    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
    assert_eq!(parsed["profile_used"], "owl-dl");
    assert_eq!(parsed["dry_run"], true);
}

// ── Materialization ─────────────────────────────────────────────────────

#[test]
fn test_dl_materialize_subsumptions() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:A a owl:Class .
        ex:B a owl:Class .
        ex:C a owl:Class .
        ex:A rdfs:subClassOf ex:B .
        ex:B rdfs:subClassOf ex:C .
    "#,
            None,
        )
        .unwrap();

    let before = store.triple_count();
    let result = Reasoner::run(&store, "owl-dl", true).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    // A ⊑ C should be inferred and materialized
    assert!(
        store.triple_count() >= before,
        "Materialization should add triples"
    );

    // Verify A subClassOf C is in store
    let check = store
        .sparql_select(
            "ASK { <http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> }",
        )
        .unwrap();
    assert!(
        check.contains("true"),
        "A subClassOf C should be materialized: got {}",
        check
    );
}

// ── Dry run ─────────────────────────────────────────────────────────────

#[test]
fn test_dl_dry_run() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:A a owl:Class .
        ex:B a owl:Class .
        ex:A rdfs:subClassOf ex:B .
    "#,
            None,
        )
        .unwrap();

    let before = store.triple_count();
    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let _parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(_parsed["dry_run"], true);
    assert_eq!(store.triple_count(), before, "Dry run should not modify store");
}

// ── Transitive roles ────────────────────────────────────────────────────

#[test]
fn test_dl_transitive_role() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:partOf a owl:ObjectProperty, owl:TransitiveProperty .
        ex:Component a owl:Class .
        ex:Assembly a owl:Class .

        ex:Component rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:partOf ;
            owl:someValuesFrom ex:Assembly
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
}

// ── Existing RDFS/OWL-RL tests still pass ───────────────────────────────

#[test]
fn test_existing_profiles_unaffected() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Cat rdfs:subClassOf ex:Animal .
        ex:Tabby a ex:Cat .
    "#,
            None,
        )
        .unwrap();

    // RDFS still works
    let result = Reasoner::run(&store, "rdfs", true).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["profile_used"], "rdfs");
    assert!(parsed["inferred_count"].as_u64().unwrap() >= 1);
}
