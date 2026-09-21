//! The verified SHACL compiler NAMES what it ignores, instead of discarding it.
//!
//! Issue #206. `ignoredParams` in `lean/Shacl/Compile.lean` lists seven SHACL
//! predicates that compile silently and leave no trace. Five of them carry no
//! conformance meaning and ignoring them is right. Two do not fit that
//! description:
//!
//! `sh:severity` exists to change what a result means to a reader. A shapes
//! graph marking a constraint `sh:Warning` and one marking it `sh:Violation`
//! produced byte-identical output, and `sh:resultSeverity` is never emitted.
//! `sh:message` is the author's explanation of the constraint, and a report
//! that blames a node without the sentence its author wrote for exactly that
//! case is less useful than the shapes graph it came from.
//!
//! The defect is not that they are ignored. It is that they were ignored in
//! SILENCE, in the one place a compiler whose stated rule is refuse rather
//! than guess discarded instead. A predicate in neither `knownParams` nor
//! `ignoredParams` is refused with a reason; a predicate in `ignoredParams`
//! vanished, and nothing in the output said so.
//!
//! So the report gains an `ignored` list of `(subject, predicate)` pairs, in
//! the spirit of `skipped_constraints` on the Rust path. The verdict does not
//! change, because none of these predicates ever affected it, and
//! `Shacl.validate_spec` is untouched. Steps 2 and 3 of the issue, carrying
//! the message and the severity through to each result, are deliberately not
//! here: they would put fields in a report that the theorem says nothing
//! about, and that is a separate decision from removing the silence.
//!
//! # The expectation is read out of the Lean source
//!
//! [`ignored_params_in_the_source`] scans `ignoredParams` in `Compile.lean`
//! and resolves each entry to its IRI. A predicate added to that list without
//! flowing through to the report turns
//! `every_ignored_parameter_reaches_the_report` red on the day it lands, which
//! a list of seven IRIs typed here could never do.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn compile_lean() -> String {
    std::fs::read_to_string(repo().join("lean").join("Shacl").join("Compile.lean"))
        .expect("read lean/Shacl/Compile.lean")
}

/// The IRIs of the predicates `ignoredParams` holds, resolved through the
/// `SH` vocabulary block in the same file.
fn ignored_params_in_the_source() -> BTreeSet<String> {
    let src = compile_lean();
    let start = src.find("def ignoredParams : List Term :=").expect(
        "`ignoredParams` is gone from Compile.lean. If it was renamed this scan reads \
         nothing and every assertion below is vacuous",
    );
    let body = &src[start..];
    let open = body.find('[').expect("the list opens");
    let close = body.find(']').expect("the list closes");
    let names: Vec<String> = body[open + 1..close]
        .split(',')
        .map(|s| s.trim().trim_start_matches("SH.").to_string())
        .filter(|s| !s.is_empty())
        .collect();
    assert!(
        names.len() >= 5,
        "the scan of ignoredParams found only {names:?}, so the spelling has changed and \
         this file is measuring nothing"
    );

    // Each name is a `def <name> : Term := iri "<local>"` in the vocabulary
    // block. Resolving through it rather than assuming the Lean identifier and
    // the SHACL local name coincide, because several of them do not: the
    // compiler spells `sh:class` as `klass` and `sh:not` as `notC`.
    let ns = "http://www.w3.org/ns/shacl#";
    names
        .iter()
        .map(|n| {
            let needle = format!("def {n} : Term := iri \"");
            let at = src
                .find(&needle)
                .unwrap_or_else(|| panic!("no vocabulary entry for SH.{n} in Compile.lean"));
            let rest = &src[at + needle.len()..];
            let end = rest.find('"').expect("the local name closes");
            format!("<{ns}{}>", &rest[..end])
        })
        .collect()
}

fn checker() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("OO_SHACL") {
        let p = PathBuf::from(p);
        return p.exists().then_some(p);
    }
    let built = repo().join("lean/.lake/build/bin/oo-shacl");
    built.exists().then_some(built)
}

fn skip() -> bool {
    common::skip_unless(
        checker().is_some(),
        "the verified SHACL evaluator (oo-shacl)",
        "run `lake build oo-shacl` in lean/, or point $OO_SHACL at a built one",
    )
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-shacl-ignored-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn validate(tag: &str, data: &str, shapes: &str) -> serde_json::Value {
    let d = dir(tag);
    let dp = d.join("data.nt");
    let sp = d.join("shapes.nt");
    std::fs::write(&dp, data).unwrap();
    std::fs::write(&sp, shapes).unwrap();
    let out = Command::new(checker().expect("checked by skip()"))
        .arg("validate")
        .arg(&dp)
        .arg(&sp)
        .output()
        .expect("run oo-shacl");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("oo-shacl printed no JSON ({e}): {text}"))
}

fn ignored_predicates(report: &serde_json::Value) -> BTreeSet<String> {
    report["ignored"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|p| p["predicate"].as_str().map(|s| s.to_string()))
        .collect()
}

const DATA: &str = concat!(
    "<http://ex.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://ex.org/Person> .\n",
    "<http://ex.org/alice> <http://ex.org/name> \"Alice\" .\n",
    "<http://ex.org/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://ex.org/Person> .\n",
);

/// A shapes graph using only predicates the compiler implements.
const PLAIN_SHAPES: &str = concat!(
    "<http://ex.org/S> <http://www.w3.org/ns/shacl#targetClass> <http://ex.org/Person> .\n",
    "<http://ex.org/S> <http://www.w3.org/ns/shacl#property> _:p .\n",
    "_:p <http://www.w3.org/ns/shacl#path> <http://ex.org/name> .\n",
    "_:p <http://www.w3.org/ns/shacl#minCount> \"1\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
);

#[test]
fn a_shapes_graph_that_ignores_nothing_lists_nothing() {
    if skip() {
        return;
    }
    let r = validate("plain", DATA, PLAIN_SHAPES);
    assert_eq!(r["conforms"], false, "bob has no name: {r}");
    assert!(
        ignored_predicates(&r).is_empty(),
        "a shapes graph using only implemented predicates must report an EMPTY ignored \
         list. A list that is never empty says nothing: {r}"
    );
}

#[test]
fn every_ignored_parameter_reaches_the_report() {
    if skip() {
        return;
    }
    let want = ignored_params_in_the_source();
    // One triple per ignored predicate, all on the shape node, with an object
    // that is never read. The compiler must accept every one of them, which is
    // what `ignoredParams` means, and must now name every one of them.
    let mut shapes = String::from(PLAIN_SHAPES);
    for (i, p) in want.iter().enumerate() {
        shapes.push_str(&format!("<http://ex.org/S> {p} \"ignored {i}\" .\n"));
    }
    let r = validate("all", DATA, &shapes);
    assert_eq!(
        r["status"], "verdict",
        "every predicate in ignoredParams must COMPILE, not be refused: {r}"
    );
    let got = ignored_predicates(&r);
    assert_eq!(
        got, want,
        "the report must name every parameter the compiler ignores. A predicate in \
         ignoredParams that never reaches the report is discarded in silence, which is \
         the defect this closes"
    );
}

#[test]
fn severity_changes_the_ignored_list_and_not_the_verdict() {
    if skip() {
        return;
    }
    // The pair the issue is really about. These two shapes graphs differ in one
    // triple, and that triple is the whole of what a reader would use to decide
    // whether a result blocks a pipeline or is logged.
    let warning = format!(
        "{PLAIN_SHAPES}<http://ex.org/S> <http://www.w3.org/ns/shacl#severity> \
         <http://www.w3.org/ns/shacl#Warning> .\n"
    );
    let violation = format!(
        "{PLAIN_SHAPES}<http://ex.org/S> <http://www.w3.org/ns/shacl#severity> \
         <http://www.w3.org/ns/shacl#Violation> .\n"
    );
    let a = validate("warn", DATA, &warning);
    let b = validate("viol", DATA, &violation);

    // Unchanged, which is the honest half: the theorem is about conformance and
    // severity is outside it. This must NOT start differing.
    assert_eq!(a["conforms"], b["conforms"], "severity must not move the verdict");
    assert_eq!(a["results"], b["results"], "nor any result field");
    assert_eq!(a["theorem"], "Shacl.validate_spec");

    // And the half that changed: both now SAY a severity was present and
    // dropped, where before the two reports were byte-identical.
    for (label, r) in [("warning", &a), ("violation", &b)] {
        assert!(
            ignored_predicates(r).contains("<http://www.w3.org/ns/shacl#severity>"),
            "the {label} graph carries sh:severity and the report must say it was \
             ignored: {r}"
        );
    }
    assert!(
        r_means(&a).contains("resultSeverity"),
        "and the report must say what ignoring it costs the reader: {a}"
    );
}

fn r_means(r: &serde_json::Value) -> String {
    r["ignored_means"].as_str().unwrap_or_default().to_string()
}

/// The pair names the SHAPE, not just the predicate, or a reader with a large
/// shapes graph learns that something somewhere was dropped.
#[test]
fn the_pair_names_the_shape_it_sat_on() {
    if skip() {
        return;
    }
    let shapes = format!(
        "{PLAIN_SHAPES}_:p <http://www.w3.org/ns/shacl#message> \"every person needs a name\" .\n"
    );
    let r = validate("subject", DATA, &shapes);
    let subjects: BTreeSet<String> = r["ignored"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["subject"].as_str().map(|s| s.to_string()))
        .collect();
    assert!(
        subjects.contains("_:p"),
        "the message sits on the property shape and the report must say which node \
         carried it: {r}"
    );
}

/// The Rust side echoes the checker's JSON rather than restating it, so the
/// field has to arrive through `onto_shacl`'s verified path without anybody
/// teaching it the name.
#[test]
fn the_field_reaches_the_rust_report_without_being_restated() {
    if skip() {
        return;
    }
    let src = std::fs::read_to_string(repo().join("src").join("shacl_verified.rs"))
        .expect("read src/shacl_verified.rs");
    assert!(
        !src.contains("ignored_means"),
        "the Rust side must not carry a copy of the checker's explanation. It echoes the \
         checker's report; a second copy here would be a string that can drift from the \
         one the checker prints"
    );

    let store = std::sync::Arc::new(open_ontologies::graph::GraphStore::new());
    store
        .load_file(&write_turtle_data())
        .expect("load the data");
    let shapes_ttl = concat!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
        "@prefix ex: <http://ex.org/> .\n",
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
        "ex:S a sh:NodeShape ; sh:targetClass ex:Person ; sh:severity sh:Warning ;\n",
        "  sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:message \"needs a name\" ] .\n",
    );
    let v = open_ontologies::shacl_verified::validate_verified(&store, shapes_ttl)
        .expect("the verified path runs");
    let listed = v["report"]["ignored"].as_array().cloned().unwrap_or_default();
    assert!(
        listed.len() >= 2,
        "sh:severity and sh:message are both in these shapes and both must reach the \
         Rust report: {v}"
    );
}

fn write_turtle_data() -> String {
    let d = dir("rust");
    let p = d.join("data.ttl");
    std::fs::write(
        &p,
        "@prefix ex: <http://ex.org/> .\nex:alice a ex:Person ; ex:name \"Alice\" .\nex:bob a ex:Person .\n",
    )
    .unwrap();
    p.display().to_string()
}

