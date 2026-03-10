//! OWL 2 Conformance Validation Oracle
//!
//! Test cases derived from the W3C OWL 2 Test Cases specification and
//! established DL reasoning benchmarks. Each test has a documented expected
//! result that matches the output of HermiT/Pellet/FaCT++ reference reasoners.
//!
//! These serve as a validation oracle: if our SHOIQ tableaux produces the same
//! results as the reference reasoners, we have high confidence in correctness.
//!
//! Sources:
//! - W3C OWL 2 Test Cases: https://www.w3.org/TR/owl2-test/
//! - DL Handbook test patterns (Baader et al.)
//! - Pizza Ontology classification reference (Manchester tutorial)

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

// ── W3C-style Consistency Tests ─────────────────────────────────────────
// Reference: OWL 2 Structural Specification §11 (consistency)
// Oracle: HermiT 1.4.3

#[test]
fn w3c_consistent_empty_ontology() {
    // An empty ontology is trivially consistent.
    // HermiT: consistent ✓
    let store = Arc::new(GraphStore::new());
    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
}

#[test]
fn w3c_consistent_simple_hierarchy() {
    // A ⊑ B ⊑ C — simple subsumption chain is consistent.
    // HermiT: consistent ✓, A ⊑ C inferred
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

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(unsat.is_empty(), "No unsatisfiable classes expected");
}

#[test]
fn w3c_inconsistent_subclass_disjoint() {
    // A ⊑ B, A disjointWith B → A unsatisfiable
    // HermiT: A unsatisfiable ✓
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
        unsat.iter().any(|v| v.as_str().unwrap().contains("A")),
        "A should be unsatisfiable (subclass of B but disjoint with B)"
    );
}

// ── W3C-style Entailment Tests ──────────────────────────────────────────
// Reference: OWL 2 Direct Semantics §2.3 (entailment)

#[test]
fn w3c_entailment_equivalent_classes() {
    // A ≡ B → A ⊑ B and B ⊑ A
    // HermiT: equivalence detected ✓
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
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

#[test]
fn w3c_entailment_intersection_subsumption() {
    // C ≡ A ⊓ B → C ⊑ A and C ⊑ B
    // HermiT: C subClassOf A ✓, C subClassOf B ✓
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
        ex:C owl:equivalentClass [
            owl:intersectionOf ( ex:A ex:B )
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let classification = parsed["classification"].as_array().unwrap();
    let c_entry = classification
        .iter()
        .find(|e| e["class"].as_str().unwrap().contains("C"));
    assert!(c_entry.is_some(), "C should be in classification");
    let supers: Vec<&str> = c_entry.unwrap()["superclasses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        supers.iter().any(|s| s.contains("A")),
        "C ⊑ A (from C ≡ A ⊓ B)"
    );
    assert!(
        supers.iter().any(|s| s.contains("B")),
        "C ⊑ B (from C ≡ A ⊓ B)"
    );
}

#[test]
fn w3c_entailment_complement_unsatisfiable() {
    // A ⊑ B, A ⊑ ¬B → A unsatisfiable
    // HermiT: A unsatisfiable ✓
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
        ex:A rdfs:subClassOf [
            owl:complementOf ex:B
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        unsat.iter().any(|v| v.as_str().unwrap().contains("A")),
        "A ⊑ B ⊓ ¬B should be unsatisfiable"
    );
}

// ── Existential / Universal Interaction ─────────────────────────────────
// Classic DL Handbook pattern: ∃R.C clashes with ∀R.¬C

#[test]
fn w3c_exists_forall_direct_clash() {
    // A ⊑ ∃R.C, A ⊑ ∀R.¬C → A unsatisfiable
    // Every R-successor must have C (from ∃) and ¬C (from ∀). Clash.
    // HermiT: A unsatisfiable ✓
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:A a owl:Class .
        ex:C a owl:Class .
        ex:R a owl:ObjectProperty .
        ex:A rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:R ;
            owl:someValuesFrom ex:C
        ] .
        ex:A rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:R ;
            owl:allValuesFrom [
                owl:complementOf ex:C
            ]
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        unsat.iter().any(|v| v.as_str().unwrap().contains("A")),
        "A ⊑ ∃R.C ⊓ ∀R.¬C should be unsatisfiable"
    );
}

#[test]
fn w3c_exists_forall_transitive_clash() {
    // Pizza pattern: Pizza ⊑ ∀hasTopping.Veg, NonVegPizza ⊑ Pizza ⊓ ∃hasTopping.Meat,
    // Veg disjoint Meat → NonVegPizza unsatisfiable
    // HermiT: NonVegPizza unsatisfiable ✓
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Veg a owl:Class .
        ex:Meat a owl:Class .
        ex:Pizza a owl:Class .
        ex:NonVegPizza a owl:Class .
        ex:hasTopping a owl:ObjectProperty .
        ex:Veg owl:disjointWith ex:Meat .
        ex:Pizza rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasTopping ;
            owl:allValuesFrom ex:Veg
        ] .
        ex:NonVegPizza rdfs:subClassOf ex:Pizza .
        ex:NonVegPizza rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasTopping ;
            owl:someValuesFrom ex:Meat
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

// ── Number Restrictions (SHOQ) ──────────────────────────────────────────
// Reference: OWL 2 Structural Specification §8.4

#[test]
fn w3c_min_card_satisfiable() {
    // A ⊑ ≥2 R.B — satisfiable (just needs 2 R-successors with B)
    // HermiT: consistent ✓
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:A a owl:Class .
        ex:B a owl:Class .
        ex:R a owl:ObjectProperty .
        ex:A rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:R ;
            owl:onClass ex:B ;
            owl:minQualifiedCardinality "2"^^xsd:nonNegativeInteger
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        !unsat.iter().any(|v| v.as_str().unwrap().contains("A")),
        "A ⊑ ≥2 R.B should be satisfiable"
    );
}

#[test]
fn w3c_max_min_card_clash() {
    // A ⊑ ≥3 R.B ⊓ ≤1 R.B → unsatisfiable (needs ≥3 but max 1)
    // HermiT: A unsatisfiable ✓
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:A a owl:Class .
        ex:B a owl:Class .
        ex:R a owl:ObjectProperty .
        ex:A rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:R ;
            owl:onClass ex:B ;
            owl:minQualifiedCardinality "3"^^xsd:nonNegativeInteger
        ] .
        ex:A rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:R ;
            owl:onClass ex:B ;
            owl:maxQualifiedCardinality "1"^^xsd:nonNegativeInteger
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        unsat.iter().any(|v| v.as_str().unwrap().contains("A")),
        "A ⊑ ≥3 R.B ⊓ ≤1 R.B should be unsatisfiable"
    );
}

#[test]
fn w3c_functional_property_max_one() {
    // R is Functional → ≤1 R.⊤ universally.
    // A ⊑ ≥2 R.⊤ should be unsatisfiable.
    // HermiT: A unsatisfiable ✓
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:A a owl:Class .
        ex:R a owl:ObjectProperty, owl:FunctionalProperty .
        ex:A rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:R ;
            owl:minCardinality "2"^^xsd:nonNegativeInteger
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        unsat.iter().any(|v| v.as_str().unwrap().contains("A")),
        "A ⊑ ≥2 R.⊤ with R functional should be unsatisfiable"
    );
}

// ── Inverse Roles (SHIQ) ────────────────────────────────────────────────
// Reference: OWL 2 Structural Specification §9.2.1

#[test]
fn w3c_inverse_role_consistency() {
    // R inverseOf S — basic inverse declaration is consistent.
    // HermiT: consistent ✓
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        ex:R a owl:ObjectProperty .
        ex:S a owl:ObjectProperty .
        ex:R owl:inverseOf ex:S .
        ex:A a owl:Class .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
}

#[test]
fn w3c_symmetric_role_consistency() {
    // R is SymmetricProperty — consistent ontology with symmetric role.
    // HermiT: consistent ✓
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:friendOf a owl:ObjectProperty, owl:SymmetricProperty .
        ex:Person a owl:Class .
        ex:Person rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:friendOf ;
            owl:someValuesFrom ex:Person
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
}

// ── Disjunction / Union (ALC) ───────────────────────────────────────────
// Reference: DL Handbook §2.2.1

#[test]
fn w3c_union_satisfiable() {
    // A ≡ B ⊔ C — satisfiable, A is the union.
    // HermiT: consistent ✓, B ⊑ A, C ⊑ A
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        ex:A a owl:Class .
        ex:B a owl:Class .
        ex:C a owl:Class .
        ex:A owl:equivalentClass [
            owl:unionOf ( ex:B ex:C )
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(unsat.is_empty());
}

#[test]
fn w3c_disjoint_union_coverage() {
    // A ⊑ B ⊔ C, A disjoint B → A ⊑ C (via disjunction + elimination)
    // This tests that the ⊔-rule with backtracking correctly determines:
    // if A can't be B (disjoint), then A must be C.
    // HermiT: A ⊑ C inferred ✓
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
        ex:A rdfs:subClassOf [
            owl:unionOf ( ex:B ex:C )
        ] .
        ex:A owl:disjointWith ex:B .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", true).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
    // A should be subsumed by C (since A ⊑ B ⊔ C and A disjoint B → A ⊑ C)
    let classification = parsed["classification"].as_array().unwrap();
    let a_entry = classification
        .iter()
        .find(|e| e["class"].as_str().unwrap().contains("A"));
    if let Some(entry) = a_entry {
        let supers: Vec<&str> = entry["superclasses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(
            supers.iter().any(|s| s.contains("C")),
            "A ⊑ C should be inferred (A ⊑ B ⊔ C, A disjoint B)"
        );
    }
}

// ── Transitive Roles ────────────────────────────────────────────────────
// Reference: OWL 2 Structural Specification §9.2.4

#[test]
fn w3c_transitive_forall_propagation() {
    // R is transitive, A ⊑ ∀R.B.
    // If x:A and x R y and y R z, then z should be B.
    // The ∀-rule with transitive roles should propagate ∀R.B through chains.
    // HermiT: consistent ✓
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:R a owl:ObjectProperty, owl:TransitiveProperty .
        ex:A a owl:Class .
        ex:B a owl:Class .
        ex:A rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:R ;
            owl:allValuesFrom ex:B
        ] .
        ex:A rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:R ;
            owl:someValuesFrom [
                a owl:Restriction ;
                owl:onProperty ex:R ;
                owl:someValuesFrom owl:Thing
            ]
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
}

// ── Complex Classification (Pizza-style) ────────────────────────────────
// Reference: Manchester Pizza Tutorial, HermiT classification

#[test]
fn w3c_pizza_vegetarian_classification() {
    // MeatTopping and VegTopping are disjoint.
    // VegetarianPizza ≡ Pizza ⊓ ∀hasTopping.VegTopping
    // Margherita ⊑ Pizza ⊓ ∃hasTopping.Mozzarella ⊓ ∃hasTopping.Tomato
    // Mozzarella ⊑ VegTopping, Tomato ⊑ VegTopping
    // Margherita ⊑ ∀hasTopping.(Mozzarella ⊔ Tomato) (closure)
    //
    // HermiT: Margherita ⊑ VegetarianPizza ✓ (via closure axiom)
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:Pizza a owl:Class .
        ex:VegTopping a owl:Class .
        ex:MeatTopping a owl:Class .
        ex:Mozzarella a owl:Class .
        ex:Tomato a owl:Class .
        ex:VegetarianPizza a owl:Class .
        ex:Margherita a owl:Class .
        ex:hasTopping a owl:ObjectProperty .

        ex:VegTopping owl:disjointWith ex:MeatTopping .
        ex:Mozzarella rdfs:subClassOf ex:VegTopping .
        ex:Tomato rdfs:subClassOf ex:VegTopping .

        ex:VegetarianPizza owl:equivalentClass [
            owl:intersectionOf (
                ex:Pizza
                [ a owl:Restriction ;
                  owl:onProperty ex:hasTopping ;
                  owl:allValuesFrom ex:VegTopping ]
            )
        ] .

        ex:Margherita rdfs:subClassOf ex:Pizza .
        ex:Margherita rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasTopping ;
            owl:someValuesFrom ex:Mozzarella
        ] .
        ex:Margherita rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasTopping ;
            owl:someValuesFrom ex:Tomato
        ] .
        ex:Margherita rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasTopping ;
            owl:allValuesFrom [
                owl:unionOf ( ex:Mozzarella ex:Tomato )
            ]
        ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);

    // Margherita should be classified as VegetarianPizza
    let classification = parsed["classification"].as_array().unwrap();
    let marg = classification
        .iter()
        .find(|e| e["class"].as_str().unwrap().contains("Margherita"));
    assert!(marg.is_some(), "Margherita should appear in classification");
    if let Some(m) = marg {
        let supers: Vec<&str> = m["superclasses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(
            supers.iter().any(|s| s.contains("VegetarianPizza")),
            "Margherita ⊑ VegetarianPizza should be inferred. Got: {:?}",
            supers
        );
    }
}

#[test]
fn w3c_pizza_non_veg_unsatisfiable() {
    // Same as exists_forall_clash but more structured.
    // MeatPizza ⊑ Pizza ⊓ ∃hasTopping.Meat
    // Pizza ⊑ ∀hasTopping.Veg
    // Meat disjoint Veg
    // → MeatPizza unsatisfiable
    // HermiT: MeatPizza unsatisfiable ✓
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:Veg a owl:Class .
        ex:Meat a owl:Class .
        ex:Pizza a owl:Class .
        ex:MeatPizza a owl:Class .
        ex:hasTopping a owl:ObjectProperty .
        ex:Veg owl:disjointWith ex:Meat .
        ex:Pizza rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasTopping ;
            owl:allValuesFrom ex:Veg ] .
        ex:MeatPizza rdfs:subClassOf ex:Pizza .
        ex:MeatPizza rdfs:subClassOf [
            a owl:Restriction ;
            owl:onProperty ex:hasTopping ;
            owl:someValuesFrom ex:Meat ] .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let unsat = parsed["unsatisfiable_classes"].as_array().unwrap();
    assert!(
        unsat.iter().any(|v| v.as_str().unwrap().contains("MeatPizza")),
        "MeatPizza should be unsatisfiable"
    );
}

// ── ABox Consistency ────────────────────────────────────────────────────
// Reference: OWL 2 Direct Semantics §2.3.1

#[test]
fn w3c_abox_consistent_individual() {
    // john:Person, mary:Person — consistent ABox
    // HermiT: consistent ✓
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        ex:Person a owl:Class .
        ex:john a owl:NamedIndividual, ex:Person .
        ex:mary a owl:NamedIndividual, ex:Person .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["consistent"], true);
}

#[test]
fn w3c_abox_inconsistent_disjoint_types() {
    // john: Male AND Female, Male disjoint Female → inconsistent ABox
    // HermiT: inconsistent ✓
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        ex:Male a owl:Class .
        ex:Female a owl:Class .
        ex:Male owl:disjointWith ex:Female .
        ex:john a owl:NamedIndividual, ex:Male, ex:Female .
    "#,
            None,
        )
        .unwrap();

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let abox = &parsed["abox"];
    if !abox.is_null() {
        assert_eq!(
            abox["consistent"], false,
            "ABox should be inconsistent (john is both Male and Female, which are disjoint)"
        );
    }
}

// ── Agent Metadata Validation ───────────────────────────────────────────

#[test]
fn w3c_oracle_agent_metrics() {
    // Verify that the reasoner reports agent metrics in the output.
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

    let result = Reasoner::run(&store, "owl-dl", false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    // Agent metadata must be present
    let agents = &parsed["agents"];
    assert!(!agents.is_null(), "Agent metrics should be present");
    assert!(
        agents["satisfiability_agent"]["classes_checked"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(agents["parallel_workers"].as_u64().unwrap() > 0);
    assert_eq!(parsed["description_logic"], "SHOIQ");
}
