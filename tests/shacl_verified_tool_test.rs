//! **The verified SHACL evaluator, reachable from a tool (session feedback).**
//!
//! `Shacl/` is a mechanised SHACL Core evaluator whose soundness is a
//! machine-checked theorem. Until `src/shacl_verified.rs` existed, a grep of
//! `src/` for `oo-shacl`, `ShaclMain` or `validate_spec` returned NOTHING: the
//! only caller in the repository was `tests/shacl_core_verified_test.rs`. The
//! strongest claim here was unreachable from its own interface, which made it a
//! lab result with a binary beside it rather than a feature.
//!
//! These tests are about reachability and about the one thing that must not go
//! wrong on the way: a refusal must never read as conformance.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl_verified::{find_checker, validate_verified};
use std::sync::Arc;

fn skip() -> bool {
    match find_checker() {
        Ok(_) => false,
        Err(why) => common::skip_unless(
            false,
            &why,
            "the verified SHACL tests need the oo-shacl binary and check nothing without it",
        ),
    }
}

const PREFIXES: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
"#;

fn run(data: &str, shapes: &str) -> serde_json::Value {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(&format!("{PREFIXES}{data}"), None).expect("data");
    validate_verified(&g, &format!("{PREFIXES}{shapes}")).expect("run")
}

/// The reachability claim itself: a caller with a store and shapes gets the
/// verified verdict and the name of the theorem covering it.
#[test]
fn a_conforming_graph_is_certified_and_names_its_theorem() {
    if skip() {
        return;
    }
    let v = run(
        "ex:alice a ex:Person ; ex:name \"Alice\" .",
        "ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
             sh:property [ sh:path ex:name ; sh:minCount 1 ] .",
    );
    assert_eq!(v["verified"], serde_json::json!(true), "{v}");
    assert_eq!(v["conforms"], serde_json::json!(true), "{v}");
    assert_eq!(
        v["theorem"],
        serde_json::json!("Shacl.validate_spec"),
        "a verified verdict has to name what covers it: {v}"
    );
}

/// And it can say no. A verdict layer that only ever agrees is decoration.
#[test]
fn a_violating_graph_is_certified_as_not_conforming() {
    if skip() {
        return;
    }
    let v = run(
        "ex:bob a ex:Person .",
        "ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
             sh:property [ sh:path ex:name ; sh:minCount 1 ] .",
    );
    assert_eq!(v["verified"], serde_json::json!(true), "{v}");
    assert_eq!(v["conforms"], serde_json::json!(false), "{v}");
    assert_eq!(v["theorem"], serde_json::json!("Shacl.validate_spec"), "{v}");
}

/// **The one that matters.**
///
/// `sh:sparql` is outside what the Lean development implements, and the whole
/// argument for this path is that it REFUSES rather than skipping. A refusal
/// must come back as `conforms: null` with a reason, never as `true`. If this
/// ever returns `true`, the verified path is claiming to have checked a
/// constraint it did not read, which is worse than the unverified path's
/// `skipped_constraints` list because it carries a theorem name.
#[test]
fn a_constraint_outside_the_development_is_undetermined_and_never_conforming() {
    if skip() {
        return;
    }
    let v = run(
        "ex:carol a ex:Person .",
        "ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
             sh:sparql [ sh:select \"SELECT $this WHERE { $this ex:x ?v }\" ] .",
    );
    assert_ne!(
        v["conforms"],
        serde_json::json!(true),
        "a constraint the evaluator cannot read came back as CONFORMING, carrying a theorem \
         name. That is the failure this whole path exists to prevent: {v}"
    );
}

/// A temporal scope cannot be honoured on this path, so asking for one must be
/// refused rather than dropped. Answering the unscoped question when a scoped
/// one was asked is a wrong answer, not a lenient one.
#[test]
fn a_scoped_request_is_refused_rather_than_silently_unscoped() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/server.rs"),
    )
    .expect("server.rs");
    let i = src
        .find("if input.verified.unwrap_or(false)")
        .expect("the verified branch");
    let window = &src[i..i + 1200];
    for arg in ["valid_at", "as_of", "all_versions"] {
        assert!(
            window.contains(arg),
            "the verified branch does not mention {arg}, so a scoped request would be \
             answered unscoped"
        );
    }
    assert!(
        window.contains("cannot be combined"),
        "the verified branch does not refuse a scoped request"
    );
}

/// Without the binary there is no verdict, and the absence says so rather than
/// producing one. A missing checker is not a conformance answer.
///
/// Exercised through `resolve_checker` rather than by setting `OO_SHACL`. The
/// environment is process-global and the harness runs tests in parallel, so the
/// first version of this test turned the two reachability tests above red by
/// pointing every other test at a path that does not exist.
#[test]
fn a_missing_checker_yields_no_verdict() {
    let e = open_ontologies::shacl_verified::resolve_checker(Some(
        "/nonexistent/oo-shacl".to_string(),
    ))
    .expect_err("a path that does not exist must not resolve to a checker");
    assert!(e.contains("does not exist"), "{e}");
}
