//! Explicit exceptions to a disjointness, compiled away before any tool sees
//! them (#347).
//!
//! Issue #347 asked for inherited disjointness to become defeasible. What is
//! built instead is an `exceptFor` annotation on the reified disjointness and
//! a compile that rewrites the axiom into classical disjointness axioms. Every
//! satisfiability claim below is checked on the owl-dl TABLEAU and not only on
//! `defects`, because the point of compiling is that the tools cannot
//! disagree, and agreement is something only the reasoner can show.
//!
//! See `docs/decisions/0019-an-exception-is-compiled-and-not-inferred.md`.

use std::collections::BTreeSet;
use std::process::Command;
use std::sync::Arc;

use open_ontologies::defects::Defects;
use open_ontologies::exceptions::Exceptions;
use open_ontologies::graph::GraphStore;
use open_ontologies::tableaux::{Concept, DlReasoner, Verdict};
use serde_json::Value;

const X: &str = "https://example.org/x/";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const EXC: &str = "https://open-ontologies.org/exceptions#";

const PREFIXES: &str = r#"
@prefix ex:   <https://example.org/x/> .
@prefix owl:  <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix exc:  <https://open-ontologies.org/exceptions#> .
"#;

/// The issue's example, widened: a plain subclass on each side, a second
/// class under both sides that is NOT an exception, a subclass of the
/// exception, and an unannotated disjointness that must come through as it
/// went in.
const HIERARCHY: &str = r#"
ex:Vehicle a owl:Class . ex:Building a owl:Class . ex:Houseboat a owl:Class .
ex:Car a owl:Class ; rdfs:subClassOf ex:Vehicle .
ex:House a owl:Class ; rdfs:subClassOf ex:Building .
ex:FloatingRestaurant a owl:Class ; rdfs:subClassOf ex:Vehicle , ex:Building .
ex:Houseboat rdfs:subClassOf ex:Vehicle , ex:Building .
ex:HouseboatDeluxe a owl:Class ; rdfs:subClassOf ex:Houseboat .
ex:Animal a owl:Class . ex:Plant a owl:Class . ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
ex:Animal owl:disjointWith ex:Plant .
"#;

/// The annotated axiom, `Vehicle owl:disjointWith Building`, excepting `exceptions`.
fn annotated(exceptions: &str) -> String {
    format!(
        "ex:Vehicle owl:disjointWith ex:Building .\n\
         [] a owl:Axiom ; owl:annotatedSource ex:Vehicle ; owl:annotatedProperty owl:disjointWith ;\n\
            owl:annotatedTarget ex:Building ; exc:exceptFor {exceptions} .\n"
    )
}

fn issue_example() -> String {
    format!("{PREFIXES}{HIERARCHY}{}", annotated("ex:Houseboat"))
}

fn store(ttl: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(ttl, None).unwrap();
    g
}

fn compile(g: &Arc<GraphStore>) -> Value {
    Exceptions::compile(g).unwrap()
}

fn iri(local: &str) -> String {
    format!("<{X}{local}>")
}

fn pair(a: &str, b: &str) -> Value {
    serde_json::json!([iri(a), iri(b)])
}

fn names(v: &Value) -> BTreeSet<String> {
    v.as_array()
        .unwrap_or_else(|| panic!("not an array: {v}"))
        .iter()
        .map(|t| t.as_str().unwrap().trim_start_matches('<').trim_start_matches(X).trim_end_matches('>').to_string())
        .collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// The owl-dl tableau's verdict on a named class.
fn class_verdict(g: &Arc<GraphStore>, local: &str) -> Verdict {
    let r = DlReasoner::from_graph(g).unwrap();
    let id = r.named_class_id(&format!("{X}{local}")).unwrap_or_else(|| panic!("{local} is not a named class"));
    r.decide_satisfiable(&Concept::Atom(id))
}

/// The owl-dl tableau's verdict on the conjunction of two named classes.
fn conjunction_verdict(g: &Arc<GraphStore>, a: &str, b: &str) -> Verdict {
    let r = DlReasoner::from_graph(g).unwrap();
    let a = r.named_class_id(&format!("{X}{a}")).unwrap();
    let b = r.named_class_id(&format!("{X}{b}")).unwrap();
    let mut parts = vec![Concept::Atom(a), Concept::Atom(b)];
    parts.sort();
    r.decide_satisfiable(&Concept::And(parts))
}

fn defects(g: &Arc<GraphStore>) -> Value {
    serde_json::from_str(&Defects::check_with_limit(g, 0).unwrap()).unwrap()
}

/// Every defects row about `local`, of any kind.
fn rows_about(report: &Value, local: &str) -> Vec<Value> {
    let want = iri(local);
    report["defects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["class"].as_str() == Some(want.as_str()))
        .cloned()
        .collect()
}

fn triples(g: &GraphStore) -> BTreeSet<(String, String, String)> {
    g.all_triples().unwrap().into_iter().collect()
}

fn canonical(g: &GraphStore) -> BTreeSet<(String, String, String)> {
    triples(&g.canonicalize_blank_nodes().unwrap())
}

fn has_disjoint(g: &GraphStore, a: &str, b: &str) -> bool {
    triples(g).contains(&(iri(a), format!("<{OWL}disjointWith>"), iri(b)))
}

// ── The issue's example ──────────────────────────────────────────────────

#[test]
fn before_the_compile_the_tableau_finds_the_exception_unsatisfiable() {
    // The control. Without it every "now satisfiable" below could be a
    // fixture the tableau never found unsatisfiable in the first place.
    let g = store(&issue_example());
    assert_eq!(class_verdict(&g, "Houseboat"), Verdict::Unsatisfiable);
    assert_eq!(class_verdict(&g, "HouseboatDeluxe"), Verdict::Unsatisfiable);
    assert_eq!(class_verdict(&g, "FloatingRestaurant"), Verdict::Unsatisfiable);
}

#[test]
fn the_issue_example_compiles_and_the_tableau_agrees() {
    let g = store(&issue_example());
    let report = compile(&g);
    let axiom = &report["axioms"][0];
    assert_eq!(report["summary"]["annotated_axioms"], 1, "{report:#}");
    assert_eq!(axiom["status"], "compiled", "{report:#}");
    assert_eq!(axiom["source"], iri("Vehicle"));
    assert_eq!(axiom["target"], iri("Building"));
    assert_eq!(names(&axiom["exceptions_honoured"]), set(&["Houseboat"]));
    assert_eq!(axiom["exceptions_rejected"], serde_json::json!([]));
    assert_eq!(axiom["removed"], serde_json::json!([pair("Vehicle", "Building")]));
    let emitted: BTreeSet<String> =
        axiom["emitted"].as_array().unwrap().iter().map(|p| p.to_string()).collect();
    let expected: BTreeSet<String> = [
        pair("Car", "Building"),
        pair("FloatingRestaurant", "Building"),
        pair("FloatingRestaurant", "Vehicle"),
        pair("House", "Vehicle"),
    ]
    .iter()
    .map(|p| p.to_string())
    .collect();
    assert_eq!(emitted, expected, "{report:#}");
    // The honest cost: Vehicle and Building are exempt without being
    // exceptions, and the report says so.
    assert_eq!(
        names(&axiom["exempted_classes"]),
        set(&["Building", "Houseboat", "HouseboatDeluxe", "Vehicle"])
    );
    assert_eq!(names(&axiom["exempted_beyond_the_exceptions"]), set(&["Building", "Vehicle"]));
    assert_eq!(names(&axiom["still_under_both"]), set(&["FloatingRestaurant"]));
    assert!(report["cost"].as_str().unwrap().contains("no longer a contradiction"));

    // The tableau, on the compiled store.
    assert_eq!(class_verdict(&g, "Houseboat"), Verdict::Satisfiable);
    assert_eq!(class_verdict(&g, "HouseboatDeluxe"), Verdict::Satisfiable);
    assert_eq!(class_verdict(&g, "FloatingRestaurant"), Verdict::Unsatisfiable);
    assert_eq!(conjunction_verdict(&g, "Car", "Building"), Verdict::Unsatisfiable);
    assert_eq!(conjunction_verdict(&g, "House", "Vehicle"), Verdict::Unsatisfiable);
    assert_eq!(conjunction_verdict(&g, "Car", "House"), Verdict::Unsatisfiable);
    // The unannotated disjointness is untouched.
    assert_eq!(conjunction_verdict(&g, "Dog", "Plant"), Verdict::Unsatisfiable);
    // The cost, pinned: an individual that is only a Vehicle and a Building
    // is no longer a contradiction.
    assert_eq!(conjunction_verdict(&g, "Vehicle", "Building"), Verdict::Satisfiable);
}

#[test]
fn a_full_owl_dl_run_lists_only_the_class_that_is_not_an_exception() {
    let g = store(&issue_example());
    compile(&g);
    let run: Value = serde_json::from_str(
        &open_ontologies::reason::Reasoner::run(&g, "owl-dl", false).unwrap(),
    )
    .unwrap();
    let unsat: Vec<&str> =
        run["unsatisfiable_classes"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    assert_eq!(unsat.len(), 1, "{run:#}");
    assert!(unsat[0].contains("FloatingRestaurant"), "{run:#}");
}

#[test]
fn defects_after_the_compile_flags_the_class_that_is_not_an_exception() {
    let g = store(&issue_example());
    compile(&g);
    let report = defects(&g);
    assert!(rows_about(&report, "Houseboat").is_empty(), "{report:#}");
    assert!(rows_about(&report, "HouseboatDeluxe").is_empty(), "{report:#}");
    let fr = rows_about(&report, "FloatingRestaurant");
    assert!(!fr.is_empty(), "FloatingRestaurant must still be flagged: {report:#}");
    assert!(fr.iter().all(|r| r["severity"] == "error"), "{report:#}");
    assert_eq!(report["affected_classes"], 1, "{report:#}");
}

#[test]
fn every_other_triple_is_untouched() {
    let g = store(&issue_example());
    let before = triples(&g);
    compile(&g);
    let after = triples(&g);

    let dw = format!("<{OWL}disjointWith>");
    let cw = format!("<{EXC}compiledDisjointWith>");
    let ap = format!("<{OWL}annotatedProperty>");
    let gone: Vec<_> = before.difference(&after).cloned().collect();
    let new: Vec<_> = after.difference(&before).cloned().collect();

    // Removed: the axiom, and the reification's pointer at owl:disjointWith.
    assert_eq!(gone.len(), 2, "{gone:#?}");
    assert!(gone.contains(&(iri("Vehicle"), dw.clone(), iri("Building"))), "{gone:#?}");
    assert!(gone.iter().any(|(s, p, o)| s.starts_with("_:") && *p == ap && *o == dw), "{gone:#?}");

    // Added: the record, the reification's pointer at it, the two
    // declarations, and the four pairs.
    let ty = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>".to_string();
    let annotation_property = format!("<{OWL}AnnotationProperty>");
    let expected_ground: BTreeSet<(String, String, String)> = [
        (iri("Vehicle"), cw.clone(), iri("Building")),
        (format!("<{EXC}exceptFor>"), ty.clone(), annotation_property.clone()),
        (cw.clone(), ty, annotation_property),
        (iri("Car"), dw.clone(), iri("Building")),
        (iri("FloatingRestaurant"), dw.clone(), iri("Building")),
        (iri("FloatingRestaurant"), dw.clone(), iri("Vehicle")),
        (iri("House"), dw.clone(), iri("Vehicle")),
    ]
    .into_iter()
    .collect();
    let new_ground: BTreeSet<_> = new.iter().filter(|(s, _, _)| !s.starts_with("_:")).cloned().collect();
    assert_eq!(new_ground, expected_ground);
    let new_blank: Vec<_> = new.iter().filter(|(s, _, _)| s.starts_with("_:")).collect();
    assert_eq!(new_blank.len(), 1, "{new:#?}");
    assert_eq!((&new_blank[0].1, &new_blank[0].2), (&ap, &cw));
    // Nothing about the unannotated pair moved.
    assert!(has_disjoint(&g, "Animal", "Plant"));
}

// ── The shapes an annotation comes in ────────────────────────────────────

#[test]
fn several_exceptions_on_one_axiom_are_all_honoured() {
    let ttl = format!(
        "{PREFIXES}{HIERARCHY}\
         ex:MobileHome a owl:Class ; rdfs:subClassOf ex:Vehicle , ex:Building .\n{}",
        annotated("ex:Houseboat , ex:MobileHome")
    );
    let g = store(&ttl);
    let report = compile(&g);
    let axiom = &report["axioms"][0];
    assert_eq!(axiom["status"], "compiled", "{report:#}");
    assert_eq!(names(&axiom["exceptions_honoured"]), set(&["Houseboat", "MobileHome"]));
    assert_eq!(names(&axiom["still_under_both"]), set(&["FloatingRestaurant"]));
    assert_eq!(class_verdict(&g, "Houseboat"), Verdict::Satisfiable);
    assert_eq!(class_verdict(&g, "MobileHome"), Verdict::Satisfiable);
    assert_eq!(class_verdict(&g, "HouseboatDeluxe"), Verdict::Satisfiable);
    assert_eq!(class_verdict(&g, "FloatingRestaurant"), Verdict::Unsatisfiable);
    assert_eq!(conjunction_verdict(&g, "Car", "Building"), Verdict::Unsatisfiable);
    // Nothing licensed is below both exceptions, so they are kept apart.
    let emitted: Vec<String> =
        axiom["emitted"].as_array().unwrap().iter().map(|p| p.to_string()).collect();
    assert!(emitted.contains(&pair("Houseboat", "MobileHome").to_string()), "{report:#}");
    assert!(!emitted.contains(&pair("MobileHome", "Houseboat").to_string()), "paired once: {report:#}");
    assert_eq!(conjunction_verdict(&g, "Houseboat", "MobileHome"), Verdict::Unsatisfiable);
}

/// The coordinator's repro of the first version's gap. Every class here is
/// above or below an exception, so the first version exempted all of them,
/// emitted no pair at all, and `A1 ⊓ B2` became satisfiable: the axiom had
/// vanished. Under the reading (two classes stay compatible only when a
/// licensed class is below both) the intermediates are kept apart.
const TWO_EXCEPTIONS: &str = r#"
ex:X a owl:Class . ex:Y a owl:Class .
ex:A1 a owl:Class ; rdfs:subClassOf ex:X . ex:B1 a owl:Class ; rdfs:subClassOf ex:Y .
ex:A2 a owl:Class ; rdfs:subClassOf ex:X . ex:B2 a owl:Class ; rdfs:subClassOf ex:Y .
ex:E1 a owl:Class ; rdfs:subClassOf ex:A1 , ex:B1 .
ex:E2 a owl:Class ; rdfs:subClassOf ex:A2 , ex:B2 .
ex:X owl:disjointWith ex:Y .
[] a owl:Axiom ; owl:annotatedSource ex:X ; owl:annotatedProperty owl:disjointWith ; owl:annotatedTarget ex:Y ; exc:exceptFor ex:E1 , ex:E2 .
"#;

#[test]
fn two_exceptions_under_different_intermediates_keep_the_intermediates_apart() {
    let g = store(&format!("{PREFIXES}{TWO_EXCEPTIONS}"));
    let report = compile(&g);
    let axiom = &report["axioms"][0];
    assert_eq!(axiom["status"], "compiled", "{report:#}");
    // Exactly the two most general incompatible pairs; E1 ⊥ B2 and the like
    // are entailed by them and not written.
    assert_eq!(axiom["emitted"], serde_json::json!([pair("A1", "B2"), pair("A2", "B1")]), "{report:#}");
    assert_eq!(conjunction_verdict(&g, "A1", "B2"), Verdict::Unsatisfiable);
    assert_eq!(conjunction_verdict(&g, "A2", "B1"), Verdict::Unsatisfiable);
    assert_eq!(conjunction_verdict(&g, "E1", "E2"), Verdict::Unsatisfiable);
    assert_eq!(class_verdict(&g, "E1"), Verdict::Satisfiable);
    assert_eq!(class_verdict(&g, "E2"), Verdict::Satisfiable);
    assert_eq!(conjunction_verdict(&g, "A1", "B1"), Verdict::Satisfiable);
    assert_eq!(conjunction_verdict(&g, "A2", "B2"), Verdict::Satisfiable);
    // The documented cost, as on the issue example.
    assert_eq!(conjunction_verdict(&g, "X", "Y"), Verdict::Satisfiable);
    let again = compile(&g);
    assert_eq!(again["changed"], false, "{again:#}");
}

#[test]
fn a_subclass_of_an_exception_filed_under_a_second_class_stays_satisfiable() {
    // HouseboatDeluxe is below the exception and also under Luxury, a plain
    // Vehicle. Luxury has a licensed class below it, so it is compatible with
    // Building; pairing Luxury against all of Building would make the
    // exception's own subclass unsatisfiable.
    let ttl = format!(
        "{PREFIXES}{HIERARCHY}\
         ex:Luxury a owl:Class ; rdfs:subClassOf ex:Vehicle .\n\
         ex:HouseboatDeluxe rdfs:subClassOf ex:Luxury .\n{}",
        annotated("ex:Houseboat")
    );
    let g = store(&ttl);
    let report = compile(&g);
    assert_eq!(report["axioms"][0]["status"], "compiled", "{report:#}");
    assert!(!has_disjoint(&g, "Luxury", "Building"), "{report:#}");
    assert_eq!(class_verdict(&g, "HouseboatDeluxe"), Verdict::Satisfiable);
    assert_eq!(conjunction_verdict(&g, "Luxury", "Building"), Verdict::Satisfiable);
    assert_eq!(conjunction_verdict(&g, "Luxury", "House"), Verdict::Unsatisfiable);
    assert_eq!(class_verdict(&g, "FloatingRestaurant"), Verdict::Unsatisfiable);
}

#[test]
fn the_disjointness_written_the_other_way_round_is_still_compiled() {
    // The triple says Building ⊥ Vehicle, the annotation names Vehicle as
    // the source. Disjointness is symmetric, so the annotation is accepted.
    let ttl = format!(
        "{PREFIXES}{HIERARCHY}\
         ex:Building owl:disjointWith ex:Vehicle .\n\
         [] a owl:Axiom ; owl:annotatedSource ex:Vehicle ; owl:annotatedProperty owl:disjointWith ;\n\
            owl:annotatedTarget ex:Building ; exc:exceptFor ex:Houseboat .\n"
    );
    let g = store(&ttl);
    let report = compile(&g);
    let axiom = &report["axioms"][0];
    assert_eq!(axiom["status"], "compiled", "{report:#}");
    assert_eq!(axiom["removed"], serde_json::json!([pair("Building", "Vehicle")]), "{report:#}");
    assert!(!has_disjoint(&g, "Building", "Vehicle"));
    assert!(!has_disjoint(&g, "Vehicle", "Building"));
    assert_eq!(class_verdict(&g, "Houseboat"), Verdict::Satisfiable);
    assert_eq!(class_verdict(&g, "FloatingRestaurant"), Verdict::Unsatisfiable);
    assert_eq!(conjunction_verdict(&g, "Car", "Building"), Verdict::Unsatisfiable);
    assert_eq!(conjunction_verdict(&g, "House", "Vehicle"), Verdict::Unsatisfiable);
}

#[test]
fn an_exception_not_under_both_sides_is_rejected_by_name_and_not_used() {
    // Car is under Vehicle only. Honouring it would exempt Car's ancestors
    // and descendants for no conflict at all, and ignoring it in silence
    // would leave a declared intention with no trace.
    let ttl = format!("{PREFIXES}{HIERARCHY}{}", annotated("ex:Car"));
    let g = store(&ttl);
    let before = triples(&g);
    let report = compile(&g);
    let axiom = &report["axioms"][0];
    assert_eq!(axiom["status"], "not_compiled", "{report:#}");
    assert_eq!(axiom["reason"], "no_exception_is_under_both", "{report:#}");
    let rejected = &axiom["exceptions_rejected"][0];
    assert_eq!(rejected["exception"], iri("Car"));
    assert_eq!(rejected["reason"], "exception_not_under_both");
    assert_eq!(rejected["under_source"], true);
    assert_eq!(rejected["under_target"], false);
    assert_eq!(report["changed"], false, "{report:#}");
    assert_eq!(triples(&g), before, "a rejected exception changes nothing");
    assert_eq!(class_verdict(&g, "Houseboat"), Verdict::Unsatisfiable);
}

#[test]
fn a_rejected_exception_beside_an_honoured_one_is_reported_and_the_rest_compiles() {
    let ttl = format!("{PREFIXES}{HIERARCHY}{}", annotated("ex:Houseboat , ex:Car"));
    let g = store(&ttl);
    let report = compile(&g);
    let axiom = &report["axioms"][0];
    assert_eq!(axiom["status"], "compiled", "{report:#}");
    assert_eq!(names(&axiom["exceptions_honoured"]), set(&["Houseboat"]));
    assert_eq!(axiom["exceptions_rejected"][0]["exception"], iri("Car"));
    assert_eq!(report["summary"]["exceptions_rejected"], 1);
    // Car was not exempted by its rejected exception.
    assert_eq!(conjunction_verdict(&g, "Car", "Building"), Verdict::Unsatisfiable);
    assert_eq!(class_verdict(&g, "Houseboat"), Verdict::Satisfiable);
}

#[test]
fn a_class_under_a_side_through_an_intersection_keeps_its_disjointness() {
    // Seaplane is under Vehicle only through a defined class. Reading the
    // told hierarchy as rdfs:subClassOf alone would give it no pair, and
    // Seaplane ⊓ Building would become satisfiable with nothing said.
    let ttl = format!(
        "{PREFIXES}{HIERARCHY}\
         ex:Aircraft a owl:Class .\n\
         ex:Seaplane a owl:Class ; owl:equivalentClass [ a owl:Class ; owl:intersectionOf ( ex:Vehicle ex:Aircraft ) ] .\n{}",
        annotated("ex:Houseboat")
    );
    let g = store(&ttl);
    compile(&g);
    assert!(has_disjoint(&g, "Seaplane", "Building"));
    assert_eq!(conjunction_verdict(&g, "Seaplane", "Building"), Verdict::Unsatisfiable);
    assert_eq!(class_verdict(&g, "Houseboat"), Verdict::Satisfiable);
}

// ── Idempotence and re-runs ─────────────────────────────────────────────

#[test]
fn compiling_a_compiled_store_changes_nothing() {
    let g = store(&issue_example());
    compile(&g);
    let once = triples(&g);
    let again = compile(&g);
    assert_eq!(again["changed"], false, "{again:#}");
    assert_eq!(again["summary"]["inserted_triples"], 0, "{again:#}");
    assert_eq!(again["summary"]["removed"], 0, "{again:#}");
    assert_eq!(again["axioms"][0]["status"], "recompiled", "{again:#}");
    assert_eq!(triples(&g), once);
}

#[test]
fn compiling_a_compiled_file_changes_nothing() {
    // The same through a serialisation, which relabels every blank node, so
    // the two are compared after RDFC-1.0 canonicalisation.
    let g = store(&issue_example());
    compile(&g);
    let written = g.serialize("turtle").unwrap();
    let reread = store(&written);
    let first = canonical(&reread);
    let report = compile(&reread);
    assert_eq!(report["changed"], false, "{report:#}");
    assert_eq!(canonical(&reread), first);
    assert_eq!(class_verdict(&reread, "Houseboat"), Verdict::Satisfiable);
    assert_eq!(class_verdict(&reread, "FloatingRestaurant"), Verdict::Unsatisfiable);
}

#[test]
fn a_rerun_gives_a_new_subclass_its_pair() {
    let g = store(&issue_example());
    compile(&g);
    g.load_turtle(&format!("{PREFIXES} ex:Bus a owl:Class ; rdfs:subClassOf ex:Vehicle ."), None).unwrap();
    // Before the re-run the new class is the documented gap.
    assert_eq!(conjunction_verdict(&g, "Bus", "Building"), Verdict::Satisfiable);
    let report = compile(&g);
    assert_eq!(report["axioms"][0]["status"], "recompiled", "{report:#}");
    assert_eq!(report["summary"]["inserted_triples"], 1, "{report:#}");
    assert!(has_disjoint(&g, "Bus", "Building"));
    assert_eq!(conjunction_verdict(&g, "Bus", "Building"), Verdict::Unsatisfiable);
    assert_eq!(class_verdict(&g, "Houseboat"), Verdict::Satisfiable);
}

#[test]
fn a_record_whose_exception_left_one_side_restores_the_disjointness() {
    // After the compile, Houseboat stops being a Building. Its exception is
    // now under one side only, so it is rejected, and with no exception left
    // the disjointness holds in full again rather than staying switched off.
    let g = store(&issue_example());
    compile(&g);
    g.sparql_update(&format!(
        "DELETE DATA {{ {} <http://www.w3.org/2000/01/rdf-schema#subClassOf> {} }}",
        iri("Houseboat"),
        iri("Building")
    ))
    .unwrap();
    let report = compile(&g);
    assert_eq!(report["axioms"][0]["status"], "restored", "{report:#}");
    assert!(has_disjoint(&g, "Vehicle", "Building"));
    assert_eq!(conjunction_verdict(&g, "Vehicle", "Building"), Verdict::Unsatisfiable);
    let again = compile(&g);
    assert_eq!(again["changed"], false, "{again:#}");
}

#[test]
fn a_store_without_exceptions_is_left_exactly_as_it_was() {
    let g = store(&format!("{PREFIXES}{HIERARCHY} ex:Vehicle owl:disjointWith ex:Building ."));
    let before = triples(&g);
    let report = compile(&g);
    assert_eq!(report["changed"], false);
    assert_eq!(report["summary"]["annotated_axioms"], 0);
    assert_eq!(triples(&g), before);
}

#[test]
fn an_exception_in_a_named_graph_is_compiled_in_that_graph() {
    let body = format!("{HIERARCHY}{}", annotated("ex:Houseboat"));
    let trig = format!("{PREFIXES}\n<https://example.org/g/ontology> {{\n{body}\n}}\n");
    let g = Arc::new(GraphStore::new());
    g.load_content(&trig, oxigraph::io::RdfFormat::TriG).unwrap();
    let report = compile(&g);
    assert_eq!(report["axioms"][0]["status"], "compiled", "{report:#}");
    let in_graph = |a: &str, p: &str, b: &str| -> bool {
        let raw = g
            .sparql_select(&format!(
                "ASK {{ GRAPH <https://example.org/g/ontology> {{ {} {p} {} }} }}",
                iri(a),
                iri(b)
            ))
            .unwrap();
        serde_json::from_str::<Value>(&raw).unwrap()["result"] == true
    };
    let dw = format!("<{OWL}disjointWith>");
    assert!(in_graph("Car", &dw, "Building"));
    assert!(!in_graph("Vehicle", &dw, "Building"));
    assert!(in_graph("Vehicle", &format!("<{EXC}compiledDisjointWith>"), "Building"));
    // Nothing leaked into the default graph.
    let default: Value = serde_json::from_str(&g.sparql_select("SELECT * WHERE { ?s ?p ?o }").unwrap()).unwrap();
    assert_eq!(default["results"].as_array().unwrap().len(), 0, "{default:#}");
}

// ── defects on a store that has NOT been compiled ───────────────────────

#[test]
fn defects_marks_a_declared_exception_and_still_counts_it_an_error() {
    let g = store(&issue_example());
    let report = defects(&g);
    for class in ["Houseboat", "HouseboatDeluxe"] {
        let rows = rows_about(&report, class);
        assert_eq!(rows.len(), 1, "{class}: {report:#}");
        let row = &rows[0];
        assert_eq!(row["kind"], "inherited_disjoint", "{row:#}");
        assert_eq!(row["severity"], "error", "every reasoner still says unsatisfiable: {row:#}");
        assert_eq!(row["declared_exception"], true, "{row:#}");
        assert_eq!(row["exception"], iri("Houseboat"), "{row:#}");
        assert!(row["hint"].as_str().unwrap().contains("compile-exceptions"), "{row:#}");
    }
    let fr = rows_about(&report, "FloatingRestaurant");
    assert_eq!(fr.len(), 1, "{report:#}");
    assert!(fr[0].get("declared_exception").is_none(), "not an exception: {:#}", fr[0]);
    assert_eq!(report["declared_exceptions"], 2, "{report:#}");
    assert_eq!(report["severity_counts"]["error"], 3, "{report:#}");
    // And the tableau agrees with the error, which is the point of keeping it.
    assert_eq!(class_verdict(&g, "Houseboat"), Verdict::Unsatisfiable);
}

#[test]
fn defects_does_not_mark_a_class_whose_exception_would_be_rejected() {
    // WaterVehicle is under Vehicle only, so the compile would reject it,
    // and a hint pointing at the compile would send the reader to a command
    // that does not help.
    let ttl = format!(
        "{PREFIXES}{HIERARCHY}\
         ex:WaterVehicle a owl:Class ; rdfs:subClassOf ex:Vehicle .\n\
         ex:Pontoon a owl:Class ; rdfs:subClassOf ex:WaterVehicle , ex:Building .\n{}",
        annotated("ex:WaterVehicle")
    );
    let report = defects(&store(&ttl));
    let rows = rows_about(&report, "Pontoon");
    assert_eq!(rows.len(), 1, "{report:#}");
    assert!(rows[0].get("declared_exception").is_none(), "{:#}", rows[0]);
    assert_eq!(report["declared_exceptions"], 0);
}

#[test]
fn defects_without_any_annotation_is_unchanged() {
    let report = defects(&store(&format!("{PREFIXES}{HIERARCHY} ex:Vehicle owl:disjointWith ex:Building .")));
    assert_eq!(report["declared_exceptions"], 0);
    assert!(
        report["defects"].as_array().unwrap().iter().all(|r| r.get("declared_exception").is_none()),
        "{report:#}"
    );
}

// ── The surfaces ─────────────────────────────────────────────────────────

fn run(dir: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .arg("--data-dir")
        .arg(dir.join("data"))
        .args(args)
        .output()
        .expect("binary should run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn write(dir: &std::path::Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p.to_str().unwrap().to_string()
}

#[test]
fn the_cli_writes_the_compiled_file_and_reports_on_stdout() {
    let d = tempfile::tempdir().unwrap();
    let input = write(d.path(), "in.ttl", &issue_example());
    let out = d.path().join("out.ttl").to_str().unwrap().to_string();
    let (code, stdout, stderr) = run(d.path(), &["compile-exceptions", &input, "--out", &out]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    let report: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("{e}: {stdout}"));
    assert_eq!(report["axioms"][0]["status"], "compiled", "{report:#}");
    assert_eq!(report["written"], out.as_str());

    let compiled = store(&std::fs::read_to_string(&out).unwrap());
    assert_eq!(class_verdict(&compiled, "Houseboat"), Verdict::Satisfiable);
    assert_eq!(class_verdict(&compiled, "FloatingRestaurant"), Verdict::Unsatisfiable);

    // And the compiled file compiled again is unchanged.
    let (code, stdout, stderr) = run(d.path(), &["compile-exceptions", &out]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    let again: Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(again["changed"], false, "{again:#}");
}

#[test]
fn the_cli_without_out_is_a_dry_run() {
    let d = tempfile::tempdir().unwrap();
    let input = write(d.path(), "in.ttl", &issue_example());
    let (code, stdout, stderr) = run(d.path(), &["compile-exceptions", &input]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    let report: Value = serde_json::from_str(stdout.trim()).unwrap();
    assert!(report["written"].is_null(), "{report:#}");
    assert_eq!(std::fs::read_to_string(&input).unwrap(), issue_example());
}

#[test]
fn the_cli_fails_the_process_on_a_missing_file() {
    let d = tempfile::tempdir().unwrap();
    let missing = d.path().join("absent.ttl");
    let (code, stdout, _) = run(d.path(), &["compile-exceptions", missing.to_str().unwrap()]);
    assert_eq!(code, 1, "{stdout}");
    let report: Value = serde_json::from_str(stdout.trim()).unwrap();
    assert!(report.get("error").is_some(), "{report:#}");
}

#[test]
fn the_cli_renders_a_human_report() {
    let d = tempfile::tempdir().unwrap();
    let input = write(d.path(), "in.ttl", &issue_example());
    let (code, stdout, stderr) = run(d.path(), &["--human", "compile-exceptions", &input]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert!(stdout.contains("compiled"), "{stdout}");
    assert!(stdout.contains(&format!("{X}Houseboat")), "{stdout}");
    assert!(stdout.contains("still unsatisfiable"), "{stdout}");
}

#[test]
fn batch_compiles_a_file_and_the_batch_store() {
    let d = tempfile::tempdir().unwrap();
    let input = write(d.path(), "in.ttl", &issue_example());
    let out = d.path().join("batch-out.ttl").to_str().unwrap().to_string();
    let script = write(
        d.path(),
        "script.txt",
        &format!(
            "compile-exceptions {input} --out {out}\nload {input}\ncompile-exceptions\ndefects --limit 0\n"
        ),
    );
    let (code, stdout, stderr) = run(d.path(), &["batch", &script]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    let results: Vec<Value> =
        stdout.lines().filter(|l| !l.trim().is_empty()).map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(results[0]["result"]["axioms"][0]["status"], "compiled", "{:#}", results[0]);
    assert_eq!(results[0]["result"]["written"], out.as_str());
    assert!(std::path::Path::new(&out).exists());
    assert_eq!(results[2]["result"]["axioms"][0]["status"], "compiled", "{:#}", results[2]);
    let after = &results[3]["result"];
    assert!(rows_about(after, "Houseboat").is_empty(), "{after:#}");
    assert!(!rows_about(after, "FloatingRestaurant").is_empty(), "{after:#}");
}

#[tokio::test]
async fn onto_compile_exceptions_compiles_the_loaded_store_in_place() {
    use open_ontologies::inputs::OntoDefectsInput;
    use open_ontologies::server::OpenOntologiesServer;
    use open_ontologies::state::StateDb;
    use rmcp::handler::server::wrapper::Parameters;

    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    let graph = store(&issue_example());
    let server = OpenOntologiesServer::new_with_graph(db, graph.clone());

    let before: Value =
        serde_json::from_str(&server.onto_defects(Parameters(OntoDefectsInput { limit: Some(0) }))).unwrap();
    assert_eq!(before["declared_exceptions"], 2, "{before:#}");

    let report: Value = serde_json::from_str(&server.onto_compile_exceptions()).unwrap();
    assert_eq!(report["axioms"][0]["status"], "compiled", "{report:#}");

    let after: Value =
        serde_json::from_str(&server.onto_defects(Parameters(OntoDefectsInput { limit: Some(0) }))).unwrap();
    assert!(rows_about(&after, "Houseboat").is_empty(), "{after:#}");
    assert_eq!(class_verdict(&graph, "Houseboat"), Verdict::Satisfiable);
}

#[test]
fn the_human_rendering_names_the_cost() {
    let g = store(&issue_example());
    let text = open_ontologies::exceptions::render_human(&compile(&g));
    assert!(text.contains("1 compiled"), "{text}");
    assert!(text.contains("exempted without being an exception (the cost)"), "{text}");
    assert!(text.contains(&format!("{X}FloatingRestaurant")), "{text}");
}
