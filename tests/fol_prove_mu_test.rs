//! Zhaozhou's 無: a question the ontology cannot be asked is returned unasked,
//! with the verdict `mu`, and no prover is consulted about it.
//!
//! Before this, `triple_as_axiom` accepted any IRI in class position
//! (`Concept::Atom` takes a string), the prover found a countermodel to a
//! symbol no axiom constrains, and the run filed it as "not entailed": the
//! file was reported as having said no to a question it never understood.
//! ies-core carries 43 `skos:Concept` terms that are never classes and the
//! first version of the front-page figure drew them as if they were.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use open_ontologies::graph::GraphStore;
use open_ontologies::tptp::{declared_classes, read_graph, types_of, unasked};
use open_ontologies::tstp::{prove_export, ProveOptions, Prover};

const IES: &str = "benchmark/reference/ies-core.ttl";
const IES_NS: &str = "http://purl.org/ies/core/v0/ont/";
const SUB: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

static NEXT: AtomicU64 = AtomicU64::new(0);

fn triples() -> Vec<(String, String, String)> {
    let ttl = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(IES)).expect("fixture");
    let g = GraphStore::new();
    g.load_turtle(&ttl, None).expect("turtle parses");
    g.all_triples().unwrap()
}

fn ies(local: &str) -> String {
    format!("<{IES_NS}{local}>")
}

/// The classifier alone, no prover: a `skos:Concept` in subject position.
#[test]
fn a_skos_concept_in_class_position_is_unasked() {
    let t = triples();
    let (declared, types) = (declared_classes(&t), types_of(&t));
    let read = read_graph(t);
    let u = unasked(&read, &declared, &types, &ies("Accent"), &format!("<{SUB}>"), &ies("Characteristic"))
        .expect("Accent is a skos:Concept, never a class: the question must be returned");
    assert_eq!(u.term, format!("{IES_NS}Accent"));
    assert_eq!(u.position, "subject");
    assert!(u.why.contains("skos"), "the refusal must say what the file DOES call it: {}", u.why);
    assert!(
        u.kind == "individual" || u.kind == "typed_not_a_class",
        "a concept is either an individual (skos:broader made it one) or typed and never a class: {}",
        u.kind
    );
}

/// A name the file has never used, in object position.
#[test]
fn an_undeclared_term_is_unasked_and_named_as_such() {
    let t = triples();
    let (declared, types) = (declared_classes(&t), types_of(&t));
    let read = read_graph(t);
    let u = unasked(&read, &declared, &types, &ies("SetOfSigns"), &format!("<{SUB}>"), &ies("NoSuchThing"))
        .expect("an unknown object is unasked");
    assert_eq!(u.kind, "undeclared");
    assert_eq!(u.position, "object");
}

/// The gate can pass: two declared classes are a question the file can be asked.
#[test]
fn a_question_between_two_classes_is_asked() {
    let t = triples();
    let (declared, types) = (declared_classes(&t), types_of(&t));
    let read = read_graph(t);
    assert!(
        unasked(&read, &declared, &types, &ies("ICalRepresentation"), &format!("<{SUB}>"), &ies("SetOfSigns")).is_none()
    );
    // And a class used only in the hierarchy, never declared owl:Class, still counts as a class.
    let used: BTreeSet<String> = open_ontologies::tptp::class_positions(&read.axioms);
    assert!(used.contains(&format!("{IES_NS}SetOfSigns")), "SetOfSigns is used as a class");
}

/// End to end through `prove_export`: the unasked goals are counted, carry
/// the verdict, and never reach the prover; the askable one still does.
#[test]
fn prove_export_returns_the_unasked_goals_and_asks_the_rest() {
    if !Prover::Vampire.available() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("oo-prove-mu-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).unwrap();
    let goals = dir.join("goals.tsv");
    std::fs::write(
        &goals,
        format!(
            "{}\t<{SUB}>\t{}\n{}\t<{SUB}>\t{}\n{}\t<{SUB}>\t{}\n",
            ies("Accent"), ies("Characteristic"),
            ies("SetOfSigns"), ies("NoSuchThing"),
            ies("ICalRepresentation"), ies("SetOfSigns"),
        ),
    )
    .unwrap();
    let ttl = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(IES)).unwrap();
    let g = std::sync::Arc::new(GraphStore::new());
    g.load_turtle(&ttl, None).unwrap();
    let opts = ProveOptions { prover: Prover::Vampire, timeout_secs: 20 };
    let out = prove_export(&g, &dir, &opts, Some(goals.as_path()), 0).expect("prove runs");
    let j: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(j["unasked"], 2, "{j}");
    assert_eq!(j["verdict_counts"]["mu"], 2, "{j}");
    let goals = j["goals"].as_array().unwrap();
    assert_eq!(goals.len(), 3);
    assert_eq!(goals[0]["report"]["verdict"], "mu");
    assert_eq!(goals[0]["report"]["term"], format!("{IES_NS}Accent"));
    assert_eq!(goals[1]["report"]["verdict"], "mu");
    assert_eq!(goals[1]["report"]["kind"], "undeclared");
    assert_ne!(goals[2]["report"]["verdict"], "mu", "a real question is still asked: {}", goals[2]);
    assert!(goals[2]["report"].get("szs_status").is_some(), "the prover ran on the askable goal");
    assert_eq!(j["not_asked"].as_array().unwrap().len(), 0, "unasked is its own list, not not_asked");
    assert_eq!(j["stop_the_line"], 0);
}
