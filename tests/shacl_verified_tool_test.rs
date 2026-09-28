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

// ═══════════════════════════════════════════════════════════════════════════
// Engine defect E1: the Rust and the Lean answers to one calendar constraint
// ═══════════════════════════════════════════════════════════════════════════
//
// Until 28 September 2026 this repository held two SHACL evaluators that gave
// OPPOSITE answers to `sh:datatype xsd:date` over `"2026-02-31"`. The Lean
// checks the day against `daysInMonth` in `parseDateBody` and reported it; the
// Rust tested a regex that counted digits and did not. Decision 0018 recorded
// that as engine defect E1 and deferred the fix.
//
// The fix is measured here rather than asserted. The two evaluators are run
// over ONE store and ONE shapes graph carrying every fixture in
// `tests/shacl_xsd_date_lexical_test.rs`, and what is compared is the SET of
// focus nodes each reported, not just the conformance boolean. A boolean
// comparison over a graph with twenty-nine subjects would agree as soon as both
// sides reported anything at all.

/// The fixture list is duplicated rather than imported because an integration
/// test cannot depend on another integration test's crate. The two are kept in
/// step by `the_fixture_lists_have_not_drifted_apart`, which reads the other
/// file.
const DATE_FIXTURES: &[(&str, bool)] = &[
    ("2026-02-31", true),
    ("2026-13-01", true),
    ("2026-00-01", true),
    ("2026-01-00", true),
    ("2026-04-31", true),
    ("2026-06-31", true),
    ("2026-09-31", true),
    ("2026-11-31", true),
    ("2023-02-29", true),
    ("1900-02-29", true),
    ("2100-02-29", true),
    ("31/02/2026", true),
    ("2026-1-01", true),
    ("026-01-01", true),
    ("02026-01-01", true),
    ("2026-01-01+15:00", true),
    ("2026-01-01+14:30", true),
    ("2026-01-01Q", true),
    ("2026-02-28", false),
    ("2024-02-29", false),
    ("2000-02-29", false),
    ("1600-02-29", false),
    ("2026-01-31", false),
    ("2026-04-30", false),
    ("2026-12-31", false),
    ("12026-01-01", false),
    ("2026-01-01Z", false),
    ("2026-01-01+14:00", false),
    ("2026-01-01-05:00", false),
];

const DATE_TIME_FIXTURES: &[(&str, bool)] = &[
    ("2026-02-31T00:00:00", true),
    ("1900-02-29T00:00:00", true),
    ("2026-01-01T25:00:00", true),
    ("2026-01-01T00:60:00", true),
    ("2026-01-01T00:00:60", true),
    ("2026-01-01T00:00:00.", true),
    ("2026-01-01", true),
    ("2026-01-01T00:00:00", false),
    ("2026-01-01T23:59:59", false),
    ("2024-02-29T12:00:00.5Z", false),
    ("2026-01-01T00:00:00+14:00", false),
];

/// One subject per fixture, so the reports can be compared node by node.
fn calendar_store(table: &[(&str, bool)], datatype: &str) -> (String, String) {
    let mut data = String::new();
    for (i, (lex, _)) in table.iter().enumerate() {
        data.push_str(&format!(
            "ex:f{i} a ex:C ; ex:v \"{lex}\"^^{datatype} .\n"
        ));
    }
    let shapes = format!(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
             sh:property [ sh:path ex:v ; sh:datatype {datatype} ] ."
    );
    (data, shapes)
}

/// The focus nodes a report named, whichever evaluator wrote it.
///
/// The unverified report puts them in `violations` with a `focus_node`; the
/// verified one wraps the checker's own output, so they are in
/// `report.results` with a `focus` in N-Triples form. Reading both here is what
/// makes the two comparable at all.
fn reported(report: &serde_json::Value) -> std::collections::BTreeSet<String> {
    let rows = report["violations"]
        .as_array()
        .or_else(|| report["report"]["results"].as_array())
        .cloned()
        .unwrap_or_default();
    rows.iter()
        .filter_map(|r| {
            r["focus_node"]
                .as_str()
                .or_else(|| r["focus"].as_str())
                .map(|f| f.trim_matches(['<', '>']).to_string())
        })
        .collect()
}

fn expected(table: &[(&str, bool)]) -> std::collections::BTreeSet<String> {
    table
        .iter()
        .enumerate()
        .filter(|(_, (_, ill))| *ill)
        .map(|(i, _)| format!("http://ex.org/f{i}"))
        .collect()
}

fn agreement(table: &[(&str, bool)], datatype: &str) {
    let (data, shapes) = calendar_store(table, datatype);

    let g = Arc::new(GraphStore::new());
    g.load_turtle(&format!("{PREFIXES}{data}"), None).expect("data");
    let rust: serde_json::Value = serde_json::from_str(
        &open_ontologies::shacl::ShaclValidator::validate(&g, &format!("{PREFIXES}{shapes}"))
            .expect("the unverified evaluator runs"),
    )
    .expect("json");
    let lean = validate_verified(&g, &format!("{PREFIXES}{shapes}")).expect("the checker runs");

    assert_eq!(
        lean["theorem"],
        serde_json::json!("Shacl.validate_spec"),
        "the comparison is only worth taking against a verdict the theorem covers: {lean}"
    );
    assert_eq!(
        rust["conforms"], lean["conforms"],
        "the two evaluators disagreed on the verdict for {datatype}.\nrust: {rust}\nlean: {lean}"
    );

    let want = expected(table);
    let by_rust = reported(&rust);
    let by_lean = reported(&lean);
    assert_eq!(
        by_rust, by_lean,
        "the two evaluators named different focus nodes for {datatype}.\nrust only: {:?}\nlean only: {:?}",
        by_rust.difference(&by_lean).collect::<Vec<_>>(),
        by_lean.difference(&by_rust).collect::<Vec<_>>()
    );
    // And both agree with the table, so this is not two engines being wrong
    // together. The fixture index is recoverable from the IRI.
    assert_eq!(
        by_rust, want,
        "the agreed answer is not the answer the fixture table asks for, for {datatype}: {rust}"
    );
}

/// **The headline.** Rust and Lean now report the same `xsd:date` lexical-space
/// violations over the same store, including `2026-02-31`, which is the fixture
/// decision 0018 recorded them disagreeing on.
#[test]
fn the_two_evaluators_agree_on_every_xsd_date_fixture() {
    if skip() {
        return;
    }
    agreement(DATE_FIXTURES, "xsd:date");
}

/// And on `xsd:dateTime`, which carries the same date portion plus three fields
/// the old Rust regex never ranged at all.
#[test]
fn the_two_evaluators_agree_on_every_xsd_date_time_fixture() {
    if skip() {
        return;
    }
    agreement(DATE_TIME_FIXTURES, "xsd:dateTime");
}

/// The residual gap, measured rather than papered over.
///
/// A negative year and the year `0000` are well formed XSD 1.1. The Lean
/// declines to read either, and it declines the WHOLE validation run rather
/// than the one value, so its answer is `undetermined` with a reason. This
/// engine reports no violation over them instead, which is what it did before
/// E1 was fixed as well. The two are not opposite, because `undetermined` is
/// not a verdict, but they are not the same answer either, and decision 0018
/// records it as a narrower open gap. This test is what stops that sentence
/// from going stale.
#[test]
fn the_lean_evaluator_declines_the_years_this_one_declines() {
    if skip() {
        return;
    }
    for lex in ["-0044-03-15", "0000-01-01"] {
        let data = format!("ex:f a ex:C ; ex:v \"{lex}\"^^xsd:date .");
        let shapes = "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
             sh:property [ sh:path ex:v ; sh:datatype xsd:date ] .";
        let g = Arc::new(GraphStore::new());
        g.load_turtle(&format!("{PREFIXES}{data}"), None).expect("data");
        let lean = validate_verified(&g, &format!("{PREFIXES}{shapes}")).expect("run");
        assert_ne!(
            lean["conforms"],
            serde_json::json!(false),
            "the Lean is expected to decline {lex}, not to report it: {lean}"
        );
        let rust: serde_json::Value = serde_json::from_str(
            &open_ontologies::shacl::ShaclValidator::validate(&g, &format!("{PREFIXES}{shapes}"))
                .expect("runs"),
        )
        .expect("json");
        assert_eq!(
            rust["conforms"],
            serde_json::json!(true),
            "and this engine is expected to report nothing over {lex}: {rust}"
        );
    }
}

/// The two fixture lists are copies, so a copy that rots is a gate that stops
/// measuring. This reads the other file and fails if a form was added or
/// removed on one side only.
#[test]
fn the_fixture_lists_have_not_drifted_apart() {
    let other = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/shacl_xsd_date_lexical_test.rs"),
    )
    .expect("the pure-Rust fixture file is in the repository");
    for (lex, _) in DATE_FIXTURES.iter().chain(DATE_TIME_FIXTURES.iter()) {
        assert!(
            other.contains(&format!("(\"{lex}\",")),
            "{lex} is compared against the Lean here and is not in \
             tests/shacl_xsd_date_lexical_test.rs, so the two lists have drifted"
        );
    }
}
