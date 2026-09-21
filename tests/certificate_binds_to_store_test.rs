//! Issue #158: a certificate says which graph it is about, and the answer can
//! be wrong.
//!
//! `oo-cert` verifies that the derivations follow from the assertions listed in
//! the certificate. It cannot verify that those assertions are the ones in your
//! database, because nothing in `asserted.tsv` records where the triples came
//! from. A correct proof over the wrong graph therefore passes, and PR #157
//! reproduced exactly that on the temporal side.
//!
//! The run now records the digest of the assertions it used, and a holder of a
//! store can recompute it. This pins both directions: the store the certificate
//! came from matches, and a store with one triple added does not. The second
//! assertion is the one that matters, because a check that cannot fail is not a
//! check.

use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{asserted_bytes, asserted_digest, certificate_binds_to_store, Reasoner};
use open_ontologies::temporal::ScopeRequest;

const TTL: &str = r#"
    @prefix ex:   <http://example.org/> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    ex:Espresso rdfs:subClassOf ex:Coffee .
    ex:Coffee   rdfs:subClassOf ex:Drink .
    ex:myCup    a               ex:Espresso .
"#;

fn store(ttl: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(ttl, None).expect("turtle parses");
    g
}

fn scratch(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("oo-bind-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Reason over a store, then ask the certificate which store it is about.
#[test]
fn a_certificate_matches_the_store_it_came_from_and_not_another() {
    let dir = scratch("same");
    let g = store(TTL);
    let out = Reasoner::run_full(
        &g,
        "rdfs",
        false,
        open_ontologies::reason::InferenceTarget::DefaultGraph,
        Some(dir.as_path()),
    )
    .expect("reason");
    let report: serde_json::Value = serde_json::from_str(&out).unwrap();
    let recorded = report["certificate"]["asserted_sha256"]
        .as_str()
        .expect("the run records the digest of what it asserted");
    assert_eq!(recorded.len(), 64, "a sha256 is 64 hex characters: {recorded}");
    assert!(
        dir.join("asserted.sha256").exists(),
        "the digest is written beside the files, so a reader who has only the directory has it"
    );

    // The store it came from.
    let same = certificate_binds_to_store(&g, &dir, &ScopeRequest::Unscoped).expect("check runs");
    assert_eq!(same["matches"], true, "{same}");
    assert_eq!(same["recorded"], same["recomputed"]);
    assert!(same["means"].as_str().unwrap().contains("THIS selection"));

    // A store with one more triple is a different graph, and the check says so.
    // The extra triple must be one this run does NOT derive: `myCup a Drink` is
    // a conclusion of rdfs9 here, so a store carrying it is indistinguishable
    // from the same store after materialisation, and calling that a different
    // graph would be a guess.
    let other = store(&format!("{TTL}\nex:Tea a ex:Herbal .\n"));
    let differs =
        certificate_binds_to_store(&other, &dir, &ScopeRequest::Unscoped).expect("check runs");
    assert_eq!(
        differs["matches"], false,
        "a certificate from a different graph was accepted as this one: {differs}"
    );
    assert_eq!(differs["likely_cause"], "different_graph", "{differs}");
    assert!(
        differs["means"].as_str().unwrap().contains("DIFFERENT GRAPH"),
        "the report has to say which way it failed: {differs}"
    );
    assert_ne!(differs["recorded"], differs["recomputed"]);
}

/// The writer and the recomputation must build the same bytes, or the digest
/// compares two different canonicalisations and is worthless.
#[test]
fn the_writer_and_the_reader_build_the_same_bytes() {
    let dir = scratch("bytes");
    let g = store(TTL);
    Reasoner::run_full(
        &g,
        "rdfs",
        false,
        open_ontologies::reason::InferenceTarget::DefaultGraph,
        Some(dir.as_path()),
    )
    .expect("reason");
    let written = std::fs::read(dir.join("asserted.tsv")).expect("asserted.tsv");
    let (digest, n) = asserted_digest(&g, &ScopeRequest::Unscoped).expect("digest");
    assert_eq!(n, 3, "the fixture has three assertions");

    // Byte equality, not just digest equality: if these ever diverge, the
    // digest would still agree with itself and disagree with the file.
    let (scope, _) = open_ontologies::temporal::resolve(&g, &ScopeRequest::Unscoped).unwrap();
    let (triples, _) = g.triples_in_scope(&scope).unwrap();
    let rebuilt = asserted_bytes(&triples).expect("rebuild");
    assert_eq!(
        written, rebuilt,
        "the file the run wrote and the bytes the reader rebuilds are not the same"
    );
    let recorded = std::fs::read_to_string(dir.join("asserted.sha256")).unwrap();
    assert_eq!(recorded.trim(), digest);
}

/// Materialisation is not a wrong graph, and the report must tell them apart.
///
/// `reason` writes its inferences into the store by default. Re-reading that
/// store afterwards yields the assertions PLUS the derivations, so the digest
/// differs. A bare "no" would send a reader looking for a wrong graph they do
/// not have, so the check compares the two selections as sets and names the
/// cause. This is the case the first end-to-end run produced.
#[test]
fn a_materialised_store_is_named_as_such_and_not_called_a_different_graph() {
    let dir = scratch("materialised");
    let g = store(TTL);
    Reasoner::run_full(
        &g,
        "rdfs",
        true, // materialise, which is what the CLI does by default
        open_ontologies::reason::InferenceTarget::DefaultGraph,
        Some(dir.as_path()),
    )
    .expect("reason");
    let r = certificate_binds_to_store(&g, &dir, &ScopeRequest::Unscoped).expect("check runs");
    assert_eq!(r["matches"], false, "the store did change: {r}");
    assert_eq!(
        r["likely_cause"], "materialised_inferences",
        "a materialised store must not be reported as a different graph: {r}"
    );
    assert!(r["in_store_only"].as_u64().unwrap() > 0, "{r}");
    assert_eq!(r["in_certificate_only"], 0, "nothing the certificate lists is missing: {r}");
    assert!(r["means"].as_str().unwrap().contains("materialised its inferences"));
}

/// A certificate with no digest cannot be bound after the fact, and the error
/// says so rather than reporting a mismatch.
#[test]
fn a_certificate_without_a_digest_is_an_error_and_not_a_mismatch() {
    let dir = scratch("legacy");
    std::fs::write(dir.join("asserted.tsv"), "").unwrap();
    let g = store(TTL);
    let e = certificate_binds_to_store(&g, &dir, &ScopeRequest::Unscoped)
        .expect_err("a certificate with no digest cannot be checked");
    let msg = e.to_string();
    assert!(
        msg.contains("does not carry a digest"),
        "the error must explain that nothing can bind it after the fact: {msg}"
    );
}

/// Issue #159: the scope the certificate recorded is APPLIED, not trusted.
///
/// `scope.tsv` is written into every certificate directory, and before this
/// nothing read it, so a run could record a scope with no relation to the graph
/// it read. The check now applies the recorded scope to the store. A manifest
/// naming a graph the run never read selects different triples, and the digest
/// refuses it.
#[test]
fn a_recorded_scope_is_applied_and_a_forged_one_is_caught() {
    let dir = scratch("scope");
    let g = store(TTL);
    Reasoner::run_full(
        &g,
        "rdfs",
        false,
        open_ontologies::reason::InferenceTarget::DefaultGraph,
        Some(dir.as_path()),
    )
    .expect("reason");

    let honest = certificate_binds_to_store(&g, &dir, &ScopeRequest::Unscoped).expect("check");
    assert_eq!(honest["matches"], true, "{honest}");
    assert_eq!(
        honest["scope_source"], "certificate",
        "the recorded scope must be the one applied, or scope.tsv is still only attested: {honest}"
    );

    // Forge the manifest: claim the run read one named graph that does not exist.
    let manifest = std::fs::read_to_string(dir.join("scope.tsv")).expect("scope.tsv");
    assert!(manifest.contains("oo-scope/1"));
    std::fs::write(
        dir.join("scope.tsv"),
        manifest.replace("graph\t*", "default_graph\tfalse\ngraph\thttp://example.org/never-read"),
    )
    .unwrap();
    let forged = certificate_binds_to_store(&g, &dir, &ScopeRequest::Unscoped).expect("check");
    assert_eq!(
        forged["matches"], false,
        "a scope naming a graph the run never read reproduced the digest: {forged}"
    );
    assert_eq!(forged["scope_source"], "certificate");
    assert_eq!(forged["asserted_in_store"], 0, "the forged scope selects nothing: {forged}");
}

/// The parser reads what the writer wrote, including the whole-store form.
#[test]
fn the_scope_parser_round_trips_the_manifest_the_writer_produces() {
    use open_ontologies::reason::scope_from_manifest;
    let whole = "oo-scope/1\nselector\twhole-store\ndefault_graph\ttrue\ngraph\t*\n";
    assert!(matches!(
        scope_from_manifest(whole),
        Some(open_ontologies::graph::ReadScope::AllGraphs)
    ));
    let named = "oo-scope/1\ndefault_graph\tfalse\ngraph\thttp://example.org/g1\ngraph\thttp://example.org/g2\n";
    match scope_from_manifest(named) {
        Some(open_ontologies::graph::ReadScope::Graphs { default_graph, named }) => {
            assert!(!default_graph);
            assert_eq!(named.len(), 2);
        }
        other => panic!("{other:?}"),
    }
    // A manifest with no graph line is one this check cannot speak about.
    assert!(scope_from_manifest("oo-scope/1\nselector\twhole-store\n").is_none());
}
