//! The Turtle validation report against the W3C suite's expected reports (#318).
//!
//! `tests/w3c_shacl_conformance_test.rs` compares the JSON report on the
//! verdict and the SET of (focus node, source shape, component) triples, and
//! says why it ignores `sh:resultPath`, `sh:value`, `sh:resultSeverity` and
//! multiplicity: comparing them needs graph isomorphism. This file is that
//! comparison. For every `core/` case the baseline records as PASS, the engine
//! writes `report_format: "turtle"`, the graph is parsed back, and both it and
//! the suite's `mf:result` are projected onto
//!
//!   the report's `sh:conforms` and `sh:result`, and per result
//!   `sh:focusNode`, `sh:resultPath` (with the blank-node path structure under
//!   it), `sh:value`, `sh:sourceShape`, `sh:sourceConstraintComponent`,
//!   `sh:resultSeverity` and `sh:resultMessage`,
//!
//! canonicalised with RDFC-1.0 (`src/canon.rs`) and compared as strings. Every
//! result is its own blank node, so a result emitted twice is two results and
//! multiplicity IS compared here.
//!
//! One rule comes from the suite rather than from this file. `sh:resultMessage`
//! is free text that the suite only pins where the expected report names it
//! (`core/misc/message-001` says so in its own comment: the harness "needs to
//! preserve all sh:resultMessage triples that are mentioned in the 'expected'
//! results graph"). So an actual message is kept when that same literal is an
//! expected `sh:resultMessage` in the test, and dropped otherwise. An expected
//! message the engine does not produce still fails.
//!
//! `rdf:type` on the report and on each result is not compared: it is not on
//! the ticket's predicate list and the writer always emits both.
//!
//! The gate is a floor on the number of matching cases, measured and
//! committed. It may rise; lowering it is a deliberate, reviewable edit.

mod common;

use open_ontologies::canon::{self, HashAlgorithm};
use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use open_ontologies::shacl_report::ReportFormat;
use open_ontologies::temporal::ScopeRequest;
use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::model::{GraphName, NamedNode, NamedOrBlankNode, Quad, Term, Triple};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Measured on 24 Oct 2026, suite commit 94d8bc2: 48 of the 56 core/ cases the
/// conformance baseline records as PASS produce a report that matches the
/// expected one on every compared predicate. 49 since #333 (28 Oct 2026, same
/// suite commit), when `core/property/or-datatypes-001` started reporting all
/// three of its `sh:or` violations. 51 since #335 (28 Oct 2026, same suite
/// commit), when the oxigraph pin started keeping `"4.0"^^xsd:decimal` as
/// written and `core/node/maxExclusive-001` and `minExclusive-001` matched.
/// The other five are named below.
const MATCHED_FLOOR: usize = 51;

/// The PASS cases whose full report differs, each with the measured cause.
/// None is a defect of the report writer: each graph says what the JSON
/// already said. They are the gaps the set comparison in
/// `w3c_shacl_conformance_test.rs` cannot see, which is why this file exists.
/// A case not listed here that stops matching fails by name.
const KNOWN_DIFFERENCES: [(&str, &str); 5] = [
    (
        "core/node/class-003",
        "ex:John is selected by sh:targetClass AND sh:targetNode and reported once per target \
         declaration, the documented non-deduplication in src/shacl.rs: 6 results for 5",
    ),
    (
        "core/targets/targetSubjectsOf-002",
        "same cause: InvalidInstance1 is selected by sh:targetClass and sh:targetSubjectsOf and \
         reported twice, 3 results for 2",
    ),
    (
        "core/property/lessThan-001",
        "the query selects DISTINCT ?focus, so results carry no sh:value; projecting it changes \
         the result count, which the JSON contract fixes",
    ),
    (
        "core/property/lessThanOrEquals-001",
        "same cause, and visible in the count: two failing values on one focus node give one \
         result where SHACL gives two",
    ),
    ("core/property/node-002", "sh:node selects DISTINCT ?focus, so no sh:value"),
];

const SUITE_BASE: &str = "https://w3c.github.io/data-shapes/data-shapes-test-suite/tests/";
const SH: &str = "http://www.w3.org/ns/shacl#";
const MF: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#";
const SHT: &str = "http://www.w3.org/ns/shacl-test#";

const RESULT_PREDICATES: [&str; 7] = [
    "focusNode",
    "resultPath",
    "value",
    "sourceShape",
    "sourceConstraintComponent",
    "resultSeverity",
    "resultMessage",
];

fn suite_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("w3c-shacl")
}

fn base_of(path: &Path) -> String {
    let rel = path.strip_prefix(suite_root()).unwrap().to_string_lossy().replace('\\', "/");
    format!("{SUITE_BASE}{rel}")
}

fn path_of(iri: &str) -> PathBuf {
    suite_root().join(iri.strip_prefix(SUITE_BASE).expect("graph inside the vendored suite"))
}

fn parse(text: &str, format: RdfFormat, base: Option<&str>) -> Vec<Triple> {
    let mut parser = RdfParser::from_format(format);
    if let Some(b) = base {
        parser = parser.with_base_iri(b).unwrap();
    }
    parser
        .for_reader(Cursor::new(text.as_bytes()))
        .map(|q| Triple::from(q.unwrap()))
        .collect()
}

/// Index of a triple set by subject, for walking out from the report node.
struct Graph {
    by_subject: HashMap<NamedOrBlankNode, Vec<Triple>>,
}

impl Graph {
    fn new(triples: Vec<Triple>) -> Self {
        let mut by_subject: HashMap<NamedOrBlankNode, Vec<Triple>> = HashMap::new();
        for t in triples {
            by_subject.entry(t.subject.clone()).or_default().push(t);
        }
        Self { by_subject }
    }

    fn objects(&self, s: &NamedOrBlankNode, p: &str) -> Vec<Term> {
        self.by_subject
            .get(s)
            .map(|ts| ts.iter().filter(|t| t.predicate.as_str() == p).map(|t| t.object.clone()).collect())
            .unwrap_or_default()
    }

    /// Every triple reachable from `node` through blank-node objects: the
    /// structure of a property path such as `[ sh:inversePath ex:p ]`.
    fn closure(&self, node: &Term, out: &mut Vec<Triple>, seen: &mut HashSet<NamedOrBlankNode>) {
        let Term::BlankNode(b) = node else { return };
        let s = NamedOrBlankNode::from(b.clone());
        if !seen.insert(s.clone()) {
            return;
        }
        for t in self.by_subject.get(&s).into_iter().flatten() {
            out.push(t.clone());
            self.closure(&t.object, out, seen);
        }
    }
}

fn as_subject(t: &Term) -> Option<NamedOrBlankNode> {
    match t {
        Term::NamedNode(n) => Some(n.clone().into()),
        Term::BlankNode(b) => Some(b.clone().into()),
        _ => None,
    }
}

/// The compared projection of the report rooted at `report`. `keep_message`
/// decides which `sh:resultMessage` objects survive (see the module docs).
fn project(g: &Graph, report: &NamedOrBlankNode, keep_message: &dyn Fn(&Term) -> bool) -> Vec<Triple> {
    let mut out = Vec::new();
    for c in g.objects(report, &format!("{SH}conforms")) {
        out.push(Triple::new(report.clone(), NamedNode::new_unchecked(format!("{SH}conforms")), c));
    }
    for r in g.objects(report, &format!("{SH}result")) {
        let Some(rs) = as_subject(&r) else { continue };
        out.push(Triple::new(report.clone(), NamedNode::new_unchecked(format!("{SH}result")), r.clone()));
        for p in RESULT_PREDICATES {
            let iri = format!("{SH}{p}");
            for o in g.objects(&rs, &iri) {
                if p == "resultMessage" && !keep_message(&o) {
                    continue;
                }
                if p == "resultPath" {
                    g.closure(&o, &mut out, &mut HashSet::new());
                }
                out.push(Triple::new(rs.clone(), NamedNode::new_unchecked(iri.clone()), o));
            }
        }
    }
    out
}

fn canonical(triples: Vec<Triple>) -> String {
    let quads: Vec<Quad> = triples.into_iter().map(|t| t.in_graph(GraphName::DefaultGraph)).collect();
    canon::canonical_nquads(&quads, canon::DEFAULT_CALL_BUDGET, HashAlgorithm::Sha256).unwrap()
}

/// The core/ cases the conformance baseline records as PASS.
fn pass_cases() -> Vec<String> {
    let text = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("w3c_shacl_baseline.json"),
    )
    .unwrap();
    let baseline: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut out: Vec<String> = baseline["outcomes"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(name, outcome)| name.starts_with("core/") && *outcome == "PASS")
        .map(|(name, _)| name.clone())
        .collect();
    out.sort();
    out
}

enum Verdict {
    Match,
    Differs { missing: Vec<String>, extra: Vec<String> },
}

fn run_case(name: &str) -> Verdict {
    // Every core/ PASS case is a single file holding its own manifest entry.
    assert!(!name.contains("::"), "{name}: multi-entry manifests are not handled here");
    let file = suite_root().join(format!("{name}.ttl"));
    let base = base_of(&file);
    let manifest = Graph::new(parse(&std::fs::read_to_string(&file).unwrap(), RdfFormat::Turtle, Some(&base)));

    let entry = manifest
        .by_subject
        .iter()
        .find(|(_, ts)| {
            ts.iter().any(|t| {
                t.predicate.as_str().ends_with("#type")
                    && t.object == Term::from(NamedNode::new_unchecked(format!("{SHT}Validate")))
            })
        })
        .map(|(s, _)| s.clone())
        .unwrap_or_else(|| panic!("{name}: no sht:Validate entry"));
    let action = as_subject(&manifest.objects(&entry, &format!("{MF}action"))[0]).unwrap();
    let iri = |p: &str| match &manifest.objects(&action, &format!("{SHT}{p}"))[0] {
        Term::NamedNode(n) => n.as_str().to_string(),
        other => panic!("{name}: {p} is {other}"),
    };
    let (data_path, shapes_path) = (path_of(&iri("dataGraph")), path_of(&iri("shapesGraph")));
    let expected_root = as_subject(&manifest.objects(&entry, &format!("{MF}result"))[0]).unwrap();

    // Loaded exactly as the conformance harness loads them: data with its own
    // base, shapes through a store and out as absolute N-Triples, because the
    // public entry point takes no base IRI.
    let data = Arc::new(GraphStore::new());
    data.load_turtle(&std::fs::read_to_string(&data_path).unwrap(), Some(&base_of(&data_path))).unwrap();
    let shapes = GraphStore::new();
    shapes
        .load_turtle(&std::fs::read_to_string(&shapes_path).unwrap(), Some(&base_of(&shapes_path)))
        .unwrap();
    let ttl = ShaclValidator::report_scoped(&data, &shapes.serialize("ntriples").unwrap(), &ScopeRequest::Unscoped)
        .unwrap()
        .render(ReportFormat::Turtle)
        .unwrap();
    let actual = Graph::new(parse(&ttl, RdfFormat::Turtle, None));
    let actual_root = actual
        .by_subject
        .iter()
        .find(|(_, ts)| ts.iter().any(|t| t.object.to_string() == format!("<{SH}ValidationReport>")))
        .map(|(s, _)| s.clone())
        .unwrap_or_else(|| panic!("{name}: the Turtle has no sh:ValidationReport:\n{ttl}"));

    let expected_messages: HashSet<Term> = manifest
        .by_subject
        .values()
        .flatten()
        .filter(|t| t.predicate.as_str() == format!("{SH}resultMessage"))
        .map(|t| t.object.clone())
        .collect();
    let want = canonical(project(&manifest, &expected_root, &|_| true));
    let got = canonical(project(&actual, &actual_root, &|m| expected_messages.contains(m)));
    if want == got {
        return Verdict::Match;
    }
    let w: BTreeSet<&str> = want.lines().collect();
    let g: BTreeSet<&str> = got.lines().collect();
    Verdict::Differs {
        missing: w.difference(&g).map(|s| s.to_string()).collect(),
        extra: g.difference(&w).map(|s| s.to_string()).collect(),
    }
}

#[test]
fn turtle_reports_match_the_w3c_expected_reports() {
    if common::skip_unless(
        suite_root().join("manifest.ttl").is_file(),
        "the vendored W3C SHACL test suite at tests/w3c-shacl/",
        "see tests/w3c-shacl/README.md",
    ) {
        return;
    }
    let cases = pass_cases();
    assert!(!cases.is_empty(), "the baseline lists no core/ PASS case");
    let mut matched = 0;
    let mut differing: Vec<String> = Vec::new();
    for name in &cases {
        match run_case(name) {
            Verdict::Match => matched += 1,
            Verdict::Differs { missing, extra } => {
                // Canonical blank-node labels are not stable across two
                // different graphs, so a line can differ only by its label;
                // both sides are printed whole rather than summarised.
                differing.push(format!(
                    "{name}\n    expected, not produced:\n      {}\n    produced, not expected:\n      {}",
                    missing.join("\n      "),
                    extra.join("\n      ")
                ));
            }
        }
    }
    eprintln!(
        "W3C full-report match: {matched} of {} core/ PASS cases ({} differ)",
        cases.len(),
        differing.len()
    );
    for d in &differing {
        eprintln!("  {d}");
    }
    let known: HashSet<&str> = KNOWN_DIFFERENCES.iter().map(|(n, _)| *n).collect();
    let regressed: Vec<&String> =
        differing.iter().filter(|d| !known.contains(d.lines().next().unwrap_or_default())).collect();
    assert!(
        regressed.is_empty(),
        "these cases no longer match the expected report and are not known differences:\n{}",
        regressed.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
    );
    assert!(
        matched >= MATCHED_FLOOR,
        "{matched} of {} core/ PASS cases match the expected report, below the floor of \
         {MATCHED_FLOOR}:\n{}",
        cases.len(),
        differing.join("\n")
    );
}

#[test]
fn the_comparison_can_fail() {
    // A gate that cannot fail measures nothing. Dropping one result from the
    // engine's graph must change the canonical form.
    let name = "core/property/datatype-001";
    let file = suite_root().join(format!("{name}.ttl"));
    if !file.is_file() {
        return;
    }
    let base = base_of(&file);
    let manifest = Graph::new(parse(&std::fs::read_to_string(&file).unwrap(), RdfFormat::Turtle, Some(&base)));
    let root = manifest
        .by_subject
        .iter()
        .find(|(_, ts)| ts.iter().any(|t| t.object.to_string() == format!("<{SH}ValidationReport>")))
        .map(|(s, _)| s.clone())
        .unwrap();
    let full = project(&manifest, &root, &|_| true);
    let first_result = full
        .iter()
        .find(|t| t.predicate.as_str() == format!("{SH}result"))
        .map(|t| t.object.clone())
        .unwrap();
    let fewer: Vec<Triple> = full.iter().filter(|t| t.object != first_result).cloned().collect();
    assert_ne!(canonical(full), canonical(fewer));
}
