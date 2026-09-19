//! A named class carrying `owl:unionOf`, `owl:intersectionOf` or
//! `owl:complementOf` defines that class, and the reasoner used to drop it.
//!
//! `parse_class_expr` gated complex class expressions on `node.starts_with("_:")`,
//! so the constructors were read only out of blank nodes. A named IRI carrying
//! one came back as an opaque atom and its definition vanished, with nothing
//! recording that anything had been lost: `unmodelled_constructs` returned
//! empty, and the file header claimed `unionOf ✅`.
//!
//! That is the UNSOUND direction. An ontology inconsistent through such a
//! definition was reported consistent, and a class unsatisfiable through one was
//! reported satisfiable. A reasoner that misses an entailment is incomplete and
//! says so; one that reports a contradiction as fine is telling the user
//! something false.
//!
//! The OWL 2 Mapping to RDF Graphs makes the reading explicit: the pattern
//! `*:x rdf:type owl:Class . *:x owl:unionOf T(SEQ CE1 ... CEn)` with `*:x` an
//! IRI maps to `EquivalentClasses(*:x ObjectUnionOf(CE1 ... CEn))`. The IRI is
//! DEFINED by the expression, not merely related to it, which is why the
//! contradictions below are real ones.

use open_ontologies::graph::GraphStore;
use open_ontologies::tableaux::DlReasoner;
use std::sync::Arc;

const PREFIXES: &str = r#"
    @prefix : <http://ex.org/> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
"#;

fn unsatisfiable(ttl: &str) -> Vec<String> {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{ttl}"), None).unwrap();
    let out = DlReasoner::run(&store, false).unwrap();
    // `run` returns the JSON as a String, so parse it, not a re-serialisation
    // of the String, which is a JSON string and has no fields at all.
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    fn collect(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, x) in m {
                    if k == "unsatisfiable_classes"
                        && let Some(a) = x.as_array()
                    {
                        out.extend(a.iter().filter_map(|y| y.as_str().map(str::to_string)));
                    }
                    collect(x, out);
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| collect(x, out)),
            _ => {}
        }
    }
    let mut v = vec![];
    collect(&json, &mut v);
    v
}

/// `U ≡ A ⊔ B`, and `Bad` is in `U` and in `N`, which is disjoint from both.
/// Every branch of the union contradicts `N`, so `Bad` is empty.
#[test]
fn a_named_class_defined_by_a_union_is_a_definition() {
    let got = unsatisfiable(
        r#"
        :A a owl:Class . :B a owl:Class . :N a owl:Class .
        :A owl:disjointWith :N . :B owl:disjointWith :N .
        :U a owl:Class ; owl:unionOf ( :A :B ) .
        :Bad a owl:Class ; rdfs:subClassOf :U , :N .
        "#,
    );
    assert!(
        got.iter().any(|c| c.contains("Bad")),
        "a union definition on a NAMED class was dropped, so an unsatisfiable class \
         was reported satisfiable; unsatisfiable_classes = {got:?}"
    );
}

/// `I ≡ A ⊓ B`. Anything in `I` is in `A`, and `A` is disjoint from `N`, so
/// `Bad` is empty. This one needs the intersection read off the named class.
#[test]
fn a_named_class_defined_by_an_intersection_is_a_definition() {
    let got = unsatisfiable(
        r#"
        :A a owl:Class . :B a owl:Class . :N a owl:Class .
        :A owl:disjointWith :N .
        :I a owl:Class ; owl:intersectionOf ( :A :B ) .
        :Bad a owl:Class ; rdfs:subClassOf :I , :N .
        "#,
    );
    assert!(
        got.iter().any(|c| c.contains("Bad")),
        "an intersection definition on a NAMED class was dropped; \
         unsatisfiable_classes = {got:?}"
    );
}

/// `K ≡ ¬A`. `Bad` is in `A` and in `K`, which is a direct contradiction.
#[test]
fn a_named_class_defined_by_a_complement_is_a_definition() {
    let got = unsatisfiable(
        r#"
        :A a owl:Class .
        :K a owl:Class ; owl:complementOf :A .
        :Bad a owl:Class ; rdfs:subClassOf :A , :K .
        "#,
    );
    assert!(
        got.iter().any(|c| c.contains("Bad")),
        "a complement definition on a NAMED class was dropped; \
         unsatisfiable_classes = {got:?}"
    );
}

/// The definition works in the other direction too: a member of either disjunct
/// is a member of the union. Pins that the equivalence is bidirectional rather
/// than a one-way subclass axiom, which is what `owl:unionOf` on an IRI means
/// and is the half a `subClassOf`-shaped fix would silently lose.
#[test]
fn the_union_definition_runs_both_ways() {
    let got = unsatisfiable(
        r#"
        :A a owl:Class . :B a owl:Class . :N a owl:Class .
        :U a owl:Class ; owl:unionOf ( :A :B ) .
        :U owl:disjointWith :N .
        :Bad a owl:Class ; rdfs:subClassOf :A , :N .
        "#,
    );
    assert!(
        got.iter().any(|c| c.contains("Bad")),
        "A ⊑ U is the half that needs U ≡ A ⊔ B rather than U ⊑ A ⊔ B; \
         unsatisfiable_classes = {got:?}"
    );
}

/// A named class with no set constructor is untouched, and one whose
/// constructor is genuinely absent must not become `owl:Thing` or `owl:Nothing`
/// by accident. Both of those would be a fix that breaks more than it repairs.
#[test]
fn an_ordinary_named_class_is_not_disturbed() {
    let got = unsatisfiable(
        r#"
        :A a owl:Class . :N a owl:Class .
        :A owl:disjointWith :N .
        :Fine a owl:Class ; rdfs:subClassOf :A .
        :AlsoFine a owl:Class ; rdfs:subClassOf :N .
        "#,
    );
    assert!(
        got.is_empty(),
        "nothing here is contradictory and nothing should be reported: {got:?}"
    );
}

/// The extraction itself. Without this, every assertion above that expects an
/// EMPTY list passes whether or not `unsatisfiable_classes` is ever read, which
/// is a gate that cannot fail.
#[test]
fn the_helper_actually_sees_an_unsatisfiable_class() {
    let got = unsatisfiable(
        r#"
        :A a owl:Class . :N a owl:Class .
        :A owl:disjointWith :N .
        :Bad a owl:Class ; rdfs:subClassOf :A , :N .
        "#,
    );
    assert!(
        got.iter().any(|c| c.contains("Bad")),
        "the plainest possible contradiction must show up, or this file measures nothing: {got:?}"
    );
}

/// The IRI subjects that carry a set constructor, asked of the loaded graph
/// rather than of the file's syntax so the blank-node spelling cannot be
/// miscounted.
const NAMED_DEFINITION_QUERY: &str = "SELECT DISTINCT ?c WHERE { \
     ?c ?p ?o . \
     FILTER(?p = <http://www.w3.org/2002/07/owl#unionOf> \
         || ?p = <http://www.w3.org/2002/07/owl#intersectionOf> \
         || ?p = <http://www.w3.org/2002/07/owl#complementOf>) \
     FILTER(isIRI(?c)) }";

fn named_definition_count(store: &Arc<GraphStore>, what: &str) -> usize {
    let res = store.sparql_select(NAMED_DEFINITION_QUERY).unwrap();
    let json: serde_json::Value = serde_json::from_str(&res)
        .unwrap_or_else(|e| panic!("sparql_select did not return JSON for {what}: {e}\n{res}"));
    json["results"]
        .as_array()
        .unwrap_or_else(|| panic!("no results array for {what}: {json}"))
        .len()
}

#[test]
fn the_bundled_ontologies_do_not_use_the_named_definition_pattern() {
    // How often this fires on the ontologies that ship with the repository: never.
    //
    // Worth pinning as a number rather than leaving as an impression. Every set
    // constructor in `pizza-reference.owl`, all 86 of them, sits on a blank node,
    // and those were always read correctly. So the defect this file fixes is
    // LATENT in this repository: real, reachable, in the unsound direction, and
    // triggered by no benchmark here. If a future reference ontology starts using
    // the named-definition pattern, this count changes and the assertion below
    // should be updated rather than deleted, because the interesting thing is the
    // number and not the zero.

    // The zero below means nothing unless this query can see the pattern when
    // it IS there. A count that is always zero and a corpus that never uses the
    // construct look identical from the assertion.
    let control = Arc::new(GraphStore::new());
    control
        .load_turtle(
            &format!("{PREFIXES}:A a owl:Class . :B a owl:Class .\n\
                      :U a owl:Class ; owl:unionOf ( :A :B ) ."),
            None,
        )
        .unwrap();
    assert_eq!(
        named_definition_count(&control, "the control"),
        1,
        "the query cannot see a named definition, so the zeros below measure nothing"
    );

    use std::path::Path;
    let mut hits = vec![];
    for name in [
        "pizza-reference.owl",
        "ies-core.ttl",
        "ies4.ttl",
        "boro-building-handcrafted.ttl",
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("benchmark/reference")
            .join(name);
        if !path.exists() {
            continue;
        }
        let store = Arc::new(GraphStore::new());
        if store.load_file(path.to_str().unwrap()).is_err() {
            continue;
        }
        // A named class is DEFINED here only if an IRI subject carries one of
        // the three constructors. Asked of the loaded graph rather than of the
        // file's syntax, so the blank-node spelling cannot be miscounted.
        let n = named_definition_count(&store, name);
        if n > 0 {
            hits.push(format!("{name}: {n}"));
        }
    }
    assert!(
        hits.is_empty(),
        "a bundled ontology now uses the named-definition pattern, so this fix is no \
         longer latent here; update the count rather than removing the check: {hits:?}"
    );
}
