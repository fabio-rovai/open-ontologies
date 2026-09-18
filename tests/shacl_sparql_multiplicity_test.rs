//! **A SPARQL constraint reports once per solution, and this pins which engine
//! is right about that.**
//!
//! Issue #169 records forty disagreements with pySHACL 0.40.1 on a private
//! corpus, thirty eight of them carrying `sh:SPARQLConstraintComponent`, with
//! this engine returning MORE results in every row. Twenty four of the forty
//! are consistent with duplication. The corpus cannot be published, so the
//! whole finding sat unreproducible and undecided.
//!
//! This is that class of disagreement in six triples, and it is decided rather
//! than merely recorded. Measured on 17 September 2026 against pySHACL 0.40.1
//! and rdflib, with the fixtures beside this file:
//!
//! | what was asked | answer |
//! | --- | --- |
//! | rdflib, `SELECT ?this WHERE { ?this ex:knows ?x }` | 4 solutions |
//! | this engine, the same query | 4 solutions |
//! | pySHACL, the shapes graph below | 2 validation results |
//! | this engine, the shapes graph below | 4 validation results |
//!
//! The two SPARQL engines AGREE on four. `ex:alice` has three `ex:knows`
//! edges and `ex:bob` has one, and SPARQL projection does not remove
//! duplicates: only `DISTINCT` and `REDUCED` do, per SPARQL 1.1 section 18.2.
//! So the disagreement is not about SPARQL at all. pySHACL's SHACL layer
//! collapses the three identical `ex:alice` solutions into one result, and
//! SHACL section 5.2.1 says a validation result is created for each solution.
//!
//! **This engine is therefore right and pySHACL under-reports**, which is the
//! opposite of what a reader would assume from a disagreement where we are the
//! outlier. It also means the twenty four duplication-consistent rows in #169
//! are very likely not defects here.
//!
//! The rows in #169 of the form "engine 1, pySHACL 0" are NOT explained by any
//! of this: nothing about multiplicity turns one result into none. Those stay
//! open and are the ones worth hunting.
//!
//! The reason a test is needed at all is that `src/shacl.rs` justified its
//! deliberate non-deduplication partly on the claim that "pyshacl emits those
//! too". On the corpus that claim was measured against it held, because the
//! extra solutions there bound something the report carried. For identical
//! solutions it does not hold, as the table above shows. The DECISION was
//! right and one of its stated reasons was not, which is worth pinning so that
//! nobody later "fixes" the count to match pySHACL and breaks conformance.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::path::PathBuf;
use std::sync::Arc;

fn fixture(name: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("shacl-sparql")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn report() -> serde_json::Value {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(&fixture("knows-data.ttl"), None).expect("data must parse");
    let json = ShaclValidator::validate(&graph, &fixture("knows-shapes.ttl")).expect("validate");
    serde_json::from_str(&json).expect("report is JSON")
}

/// Four solutions, four results. Three for `ex:alice` and one for `ex:bob`.
#[test]
fn a_sparql_constraint_reports_once_per_solution() {
    let r = report();
    assert_eq!(r["conforms"], serde_json::json!(false), "report: {r}");
    assert_eq!(
        r["violation_count"],
        serde_json::json!(4),
        "ex:alice has three ex:knows edges and ex:bob one, so the constraint query has \
         four solutions and SHACL 5.2.1 makes one result of each. Collapsing the three \
         identical ex:alice results would give 2, which is what pySHACL returns and what \
         this test exists to stop us copying: {r}"
    );
}

/// The multiplicity is per focus node, not smeared across the report. If a
/// future change deduplicated only partly, the total above could still be
/// right for the wrong reason.
#[test]
fn the_multiplicity_follows_the_focus_node_that_earned_it() {
    let r = report();
    let mut per_node: std::collections::BTreeMap<String, usize> = Default::default();
    for v in r["violations"].as_array().expect("violations is an array") {
        let f = v["focus_node"].as_str().expect("focus_node is a string").to_string();
        *per_node.entry(f).or_default() += 1;
    }
    assert_eq!(
        per_node.get("http://example.org/alice").copied(),
        Some(3),
        "ex:alice has three ex:knows edges: {per_node:?}"
    );
    assert_eq!(
        per_node.get("http://example.org/bob").copied(),
        Some(1),
        "ex:bob has one: {per_node:?}"
    );
    assert_eq!(per_node.len(), 2, "only the two ex:Person nodes are targets: {per_node:?}");
}

/// And the count is derived from the data rather than from this file. Add a
/// fourth `ex:knows` edge to `ex:alice` and the expectation moves with it, so
/// the numbers above cannot drift from the fixture they describe.
#[test]
fn the_expected_total_is_the_number_of_knows_edges_in_the_fixture() {
    let data = fixture("knows-data.ttl");
    // Both subjects are ex:Person, so every ex:knows edge is one solution.
    let edges = data.matches("ex:knows").count();
    let commas: usize = data
        .lines()
        .filter(|l| l.contains("ex:knows"))
        .map(|l| l.matches(',').count())
        .sum();
    let solutions = edges + commas;
    assert_eq!(
        solutions, 4,
        "the fixture should hold four ex:knows edges, counted as {edges} predicates plus \
         {commas} comma-separated objects"
    );
    assert_eq!(report()["violation_count"], serde_json::json!(solutions));
}
