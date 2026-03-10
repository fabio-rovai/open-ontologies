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
    let _parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

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

// ── Qualified Number Restrictions ────────────────────────────────────────

#[test]
fn test_dl_min_cardinality() {
    // Class with ≥2 hasChild.Person should be satisfiable
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        ex:Person a owl:Class .
        ex:hasChild a owl:ObjectProperty .

        ex:ParentOfTwo a owl:Class .
        ex:ParentOfTwo rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasChild ;
            owl:minQualifiedCardinality "2"^^xsd:nonNegativeInteger ;
            owl:onClass ex:Person
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        unsat.is_empty(),
        "ParentOfTwo (≥2 hasChild.Person) should be satisfiable"
    );
}

#[test]
fn test_dl_max_cardinality_ok() {
    // Class with ≤3 hasChild.Thing should be satisfiable
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        ex:hasChild a owl:ObjectProperty .

        ex:SmallFamily a owl:Class .
        ex:SmallFamily rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasChild ;
            owl:maxQualifiedCardinality "3"^^xsd:nonNegativeInteger ;
            owl:onClass owl:Thing
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        unsat.is_empty(),
        "SmallFamily (≤3 hasChild.Thing) should be satisfiable"
    );
}

#[test]
fn test_dl_max_cardinality_clash() {
    // MaxQualifiedCardinality: ≤0 hasChild.Person combined with ∃hasChild.Person
    // demands zero Person-typed children but also requires at least one — clash
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        ex:hasChild a owl:ObjectProperty .
        ex:Person a owl:Class .

        ex:Impossible a owl:Class .
        ex:Impossible rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasChild ;
            owl:maxQualifiedCardinality "0"^^xsd:nonNegativeInteger ;
            owl:onClass ex:Person
        ] .
        ex:Impossible rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasChild ;
            owl:someValuesFrom ex:Person
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
        unsat_names.iter().any(|n| n.contains("Impossible")),
        "Impossible (≤0 hasChild.Person ⊓ ∃hasChild.Person) should be unsatisfiable: {:?}",
        unsat_names
    );
}

#[test]
fn test_dl_exact_cardinality() {
    // owl:cardinality "2" should be satisfiable on its own
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        ex:hasWheel a owl:ObjectProperty .

        ex:Bicycle a owl:Class .
        ex:Bicycle rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasWheel ;
            owl:cardinality "2"^^xsd:nonNegativeInteger
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        unsat.is_empty(),
        "Bicycle (=2 hasWheel) should be satisfiable"
    );
}

#[test]
fn test_dl_functional_property() {
    // Functional property means ≤1, so a functional property
    // combined with ∀ restriction should be consistent and satisfiable
    // (functional constrains to at most one filler, ∀ constrains the filler)
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:hasMother a owl:ObjectProperty, owl:FunctionalProperty .
        ex:Person a owl:Class .
        ex:Female a owl:Class .

        ex:HasMother a owl:Class .
        ex:HasMother rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasMother ;
            owl:someValuesFrom ex:Person
        ] .
        ex:HasMother rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasMother ;
            owl:allValuesFrom ex:Female
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        unsat.is_empty(),
        "HasMother with functional property should be satisfiable: {:?}",
        unsat
    );
}

// ── Inverse Roles ────────────────────────────────────────────────────────

#[test]
fn test_dl_inverse_role() {
    // R inverseOf S: if hasChild inverseOf hasParent,
    // then ∃hasChild.Person and ∃hasParent.Person should both be satisfiable
    // and the ontology should be consistent
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:Person a owl:Class .
        ex:hasChild a owl:ObjectProperty .
        ex:hasParent a owl:ObjectProperty .
        ex:hasChild owl:inverseOf ex:hasParent .

        ex:Parent a owl:Class .
        ex:Parent rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasChild ;
            owl:someValuesFrom ex:Person
        ] .

        ex:Child a owl:Class .
        ex:Child rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasParent ;
            owl:someValuesFrom ex:Person
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        unsat.is_empty(),
        "All classes should be satisfiable with inverse roles"
    );
}

#[test]
fn test_dl_symmetric_role() {
    // SymmetricProperty should work as its own inverse
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:Person a owl:Class .
        ex:knows a owl:ObjectProperty, owl:SymmetricProperty .

        ex:Connected a owl:Class .
        ex:Connected rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:knows ;
            owl:someValuesFrom ex:Person
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        unsat.is_empty(),
        "All classes should be satisfiable with symmetric role"
    );
}

// ── ABox Reasoning ───────────────────────────────────────────────────────

#[test]
fn test_dl_abox_individual_types() {
    // Named individual with types, check consistency
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:Animal a owl:Class .
        ex:Cat a owl:Class .
        ex:Cat rdfs:subClassOf ex:Animal .

        ex:tom a owl:NamedIndividual, ex:Cat .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);

    // ABox section should be present with individual checked
    let abox = &parsed["abox"];
    assert!(!abox.is_null(), "ABox should be present when individuals exist");
    assert_eq!(abox["consistent"], true);
    assert!(
        abox["individuals_checked"].as_u64().unwrap() >= 1,
        "At least one individual should be checked"
    );
}

#[test]
fn test_dl_abox_inconsistent() {
    // Individual typed as two disjoint classes should be inconsistent
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:Cat a owl:Class .
        ex:Dog a owl:Class .
        ex:Cat owl:disjointWith ex:Dog .

        ex:impossible a owl:NamedIndividual, ex:Cat, ex:Dog .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    // ABox should report inconsistency
    let abox = &parsed["abox"];
    assert!(!abox.is_null(), "ABox should be present");
    assert_eq!(
        abox["consistent"], false,
        "Individual typed as disjoint classes should be inconsistent"
    );
}

// ── Agent Metadata ───────────────────────────────────────────────────────

#[test]
fn test_dl_agent_metadata() {
    // Run classification and verify the JSON output contains agent metrics
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:Animal a owl:Class .
        ex:Cat a owl:Class .
        ex:Dog a owl:Class .
        ex:Cat rdfs:subClassOf ex:Animal .
        ex:Dog rdfs:subClassOf ex:Animal .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);

    // Verify agent metadata structure
    let agents = &parsed["agents"];
    assert!(
        !agents.is_null(),
        "Output must contain 'agents' section"
    );

    // Satisfiability agent
    let sat_agent = &agents["satisfiability_agent"];
    assert!(
        sat_agent["classes_checked"].as_u64().is_some(),
        "satisfiability_agent should have classes_checked"
    );
    assert!(
        sat_agent["satisfiable_found"].as_u64().is_some(),
        "satisfiability_agent should have satisfiable_found"
    );
    assert!(
        sat_agent["time_ms"].as_u64().is_some(),
        "satisfiability_agent should have time_ms"
    );

    // Subsumption agent
    let sub_agent = &agents["subsumption_agent"];
    assert!(
        sub_agent["pairs_tested"].as_u64().is_some(),
        "subsumption_agent should have pairs_tested"
    );
    assert!(
        sub_agent["subsumptions_found"].as_u64().is_some(),
        "subsumption_agent should have subsumptions_found"
    );
    assert!(
        sub_agent["time_ms"].as_u64().is_some(),
        "subsumption_agent should have time_ms"
    );

    // Parallel workers
    assert!(
        agents["parallel_workers"].as_u64().unwrap() >= 1,
        "parallel_workers should be at least 1"
    );

    // Total time
    assert!(
        agents["total_time_ms"].as_u64().is_some(),
        "agents should have total_time_ms"
    );
}

// ── Complex Scenarios ────────────────────────────────────────────────────

#[test]
fn test_dl_pizza_classification() {
    // Mini pizza ontology with VegTopping, MeatTopping, hasTopping restrictions
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:Topping a owl:Class .
        ex:VegTopping a owl:Class .
        ex:MeatTopping a owl:Class .
        ex:VegTopping rdfs:subClassOf ex:Topping .
        ex:MeatTopping rdfs:subClassOf ex:Topping .
        ex:VegTopping owl:disjointWith ex:MeatTopping .

        ex:hasTopping a owl:ObjectProperty .

        ex:Pizza a owl:Class .
        ex:VegetarianPizza a owl:Class .
        ex:VegetarianPizza rdfs:subClassOf ex:Pizza .
        ex:VegetarianPizza rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasTopping ;
            owl:allValuesFrom ex:VegTopping
        ] .

        ex:MeatPizza a owl:Class .
        ex:MeatPizza rdfs:subClassOf ex:Pizza .
        ex:MeatPizza rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasTopping ;
            owl:someValuesFrom ex:MeatTopping
        ] .

        ex:MargheritaPizza a owl:Class .
        ex:MargheritaPizza rdfs:subClassOf ex:VegetarianPizza .
        ex:MargheritaPizza rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasTopping ;
            owl:someValuesFrom ex:VegTopping
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", true).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);

    // MargheritaPizza should be classified under VegetarianPizza and Pizza
    let classification = parsed["classification"].as_array().unwrap();
    let margherita = classification
        .iter()
        .find(|e| e["class"].as_str().unwrap().contains("MargheritaPizza"));
    assert!(margherita.is_some(), "MargheritaPizza should be in classification");
    if let Some(m) = margherita {
        let supers: Vec<&str> = m["superclasses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(
            supers.iter().any(|s| s.contains("VegetarianPizza")),
            "MargheritaPizza should be under VegetarianPizza: {:?}",
            supers
        );
        assert!(
            supers.iter().any(|s| s.contains("Pizza")),
            "MargheritaPizza should be under Pizza (transitive): {:?}",
            supers
        );
    }

    // VegTopping and MeatTopping should both be satisfiable
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    let unsat_names: Vec<&str> = unsat.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(
        !unsat_names.iter().any(|n| n.contains("VegTopping")),
        "VegTopping should be satisfiable"
    );
    assert!(
        !unsat_names.iter().any(|n| n.contains("MeatTopping")),
        "MeatTopping should be satisfiable"
    );
}

#[test]
fn test_dl_description_logic_field() {
    // Verify output contains "description_logic": "SHOIQ"
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        ex:A a owl:Class .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed["description_logic"], "SHOIQ",
        "Output must report description_logic as SHOIQ"
    );
    assert_eq!(parsed["algorithm"], "tableaux");
    assert_eq!(parsed["profile_used"], "owl-dl");
}
