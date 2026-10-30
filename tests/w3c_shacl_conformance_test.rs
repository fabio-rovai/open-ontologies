//! The official W3C SHACL test suite, run against this engine as a ratchet.
//!
//! # Why this exists
//!
//! On 13 Sep 2026 an adversarial audit found four SHACL defects by hand, one at
//! a time: `strip_quotes` truncating a typed literal mid-query, a blank-node
//! node shape splicing into SPARQL as a non-distinguished variable, a
//! `sh:deactivated true` honoured on shapes and ignored on a `sh:sparql`
//! constraint node, and `sh:prefixes` never resolved so a twice-bound prefix
//! was decided by store row order (`tests/shacl_audit_regressions_test.rs`).
//! Every one of those is a conformance gap against a published Recommendation,
//! and the Working Group publishes a machine-readable suite that enumerates
//! gaps of exactly that kind in seconds. This repository was not running it.
//! Finding conformance bugs by inspection when the standards body ships the
//! list is the expensive way to be wrong.
//!
//! # What this file measures, and what it does not
//!
//! The suite is manifest-driven. `tests/w3c-shacl/manifest.ttl` includes a tree
//! of manifests down to individual test files; each file carries one entry
//! typed `sht:Validate` with an `mf:action` naming a data graph and a shapes
//! graph, and an `mf:result` giving either an expected `sh:ValidationReport` or
//! the sentinel `sht:Failure`.
//!
//! The suite's own report (<https://w3c.github.io/data-shapes/data-shapes-test-suite/>)
//! defines two levels. **Partial compliance** is agreeing on the `sh:conforms`
//! boolean and nothing else. **Full compliance** is producing a report
//! isomorphic to the expected one over a fixed predicate list. This harness
//! sits between them, deliberately, and the line is:
//!
//! There are two columns, judged from the same engine answer and ratcheted
//! separately. The first is the one this harness has gated on since it was
//! written; the second is the next step towards full compliance (#319).
//!
//! THE GATED COLUMN compares
//!   * the `sh:conforms` verdict, whenever the engine gives one;
//!   * the SET of `(focus node, sh:sourceShape, sh:sourceConstraintComponent)`
//!     triples, one per validation result, on both sides. The engine emits
//!     `source_shape` and `source_constraint_component` on every violation
//!     (#131), so this is a real comparison and not a proxy.
//!
//! THE FULL-TUPLE COLUMN compares the same verdict and the MULTISET of
//! `(focus, sh:resultPath, sh:value, source shape, component,
//! sh:resultSeverity)`. Paths are compared structurally: `path_form` renders
//! a path subgraph canonically, which for SHACL's acyclic path language
//! decides exactly what an RDFC-1.0 isomorphism check would. Measured on
//! 2026-10-24 at suite commit 94d8bc2 it scores PASS 12 of 120 against the gated
//! column's 61. Of the 49 tests that pass the gated column and not this one,
//! 48 differ in `sh:value`, which the engine does not emit on the default path
//! (#318); one, `core/misc/severity-002`, also differs in severity, because the
//! engine keeps only the local name of `sh:severity` and so reports the custom
//! `ex:MySeverity` as if it were `sh:MySeverity`; and one,
//! `core/targets/targetSubjectsOf-002`, differs only in multiplicity. No test
//! differs in path.
//!
//! With #318 landed, default-path results carry `sh:value` and the column
//! scores PASS 51 (measured 2026-10-24, same suite commit). The ten tests that
//! still pass the gated column and not this one: `severity-002` (the local-name
//! severity above), `class-003` and `targetSubjectsOf-002` (a node selected by
//! two targets is reported twice), `lessThan-001`, `lessThanOrEquals-001` and
//! `node-002` (no `sh:value`: their queries select DISTINCT ?focus),
//! `or-datatypes-001` (an `sh:or` false negative), and `pre-binding-001`,
//! `-002` and `-007` under `sparql/`.
//!
//! With #333 fixed, `or-datatypes-001` reports all three of its violations
//! (an IRI value and an ill-typed `"none"^^xsd:boolean` had been dropped) and
//! the column scores PASS 52 (measured 2026-10-28, same suite commit).
//! With #334 fixed, the JSON `severity` carries any IRI outside the SHACL
//! namespace whole, `severity-002` passes, and the column scores PASS 53
//! (measured 2026-10-28, same suite commit). No test now differs in severity.
//!
//! Until #335 this column could not see one gap, by construction. It reads
//! the expected report through `load_graph`, an oxigraph `Store`, and the
//! store used to encode a literal of a known XSD datatype by value and give it
//! back canonically, exactly as the engine's own stores did. So
//! `maxExclusive-001`'s expected `sh:value 4.0` was read as
//! `"4"^^xsd:decimal`, agreed with the engine's equally canonicalised answer,
//! and scored PASS here while `tests/shacl_validation_report_w3c_test.rs`,
//! which parses without a store, held it as a difference. The pinned store
//! now keeps literals as written (oxigraph/oxigraph#2010), both sides carry
//! `"4.0"`, and the two files agree (measured 2026-10-28: this column
//! unchanged at 53, the report match up from 49 to 51 of 56).
//!
//! With #343 the engine no longer refuses `sh:datatype` naming an
//! integer-derived type, and `core/property/datatype-ill-formed` moves from
//! UNDETERMINED to PASS in both columns: the gated one scores PASS 62 and the
//! full-tuple one PASS 54, with no other test moving (measured 2026-10-30,
//! same suite commit).
//!
//! Severity is in the full-tuple column and NOT in the gated one, by
//! measurement rather than by principle. The old reason for leaving it out
//! ("the engine's `severity` field is not yet driven by `sh:severity`") is
//! false since #206: `src/shacl.rs` reads `sh:severity`. But adding it to the
//! gated key drops that column from 61 to 60 PASS (`severity-002` above), and
//! the gated column is a ratchet in a required check. The defect is held by
//! the full-tuple column's ratchet instead, where fixing it shows as a gain.
//! #334 fixed it, and it showed as that gain. Moving severity into the gated
//! key is a separate decision and is not made here.
//!
//! IGNORED by both columns, each for a stated reason
//!   * `sh:resultMessage`. The suite itself only compares messages that the
//!     expected graph names explicitly.
//!
//! IGNORED by the gated column only
//!   * `sh:resultPath`, `sh:value` and `sh:resultSeverity`, for the measured
//!     reason above: the gated column is the committed floor, and each of
//!     these is held by the full-tuple column instead.
//!   * MULTIPLICITY. The gated comparison is over sets. One test in the suite
//!     turns on multiplicity, `core/validation-reports/shared`, whose own
//!     comment records that the Working Group has not settled whether a nested
//!     shape reached twice is reported twice. `src/shacl.rs` documents a
//!     deliberate non-deduplication that inflates counts in the other
//!     direction. The full-tuple column compares multisets, as an isomorphism
//!     check on the report graph would, and names a multiplicity difference as
//!     such so it is not mistaken for a wrong field.
//!
//! Blank node IDENTITY. A blank-node property shape in `sh:sourceShape` is
//! keyed, in both columns, by the IRI of the node shape that holds it under
//! `sh:property` plus its `sh:path`: that identity survives two parses and two
//! engines, where a blank label survives neither. Anything else blank (a blank
//! focus node or value, a blank node shape, a property shape with no single
//! IRI parent) still canonicalises to the opaque token `_:`, so tests turning
//! on those are compared more loosely than the rest, and that is named here
//! rather than hidden. Before #319 every blank source shape collapsed to `_:`,
//! which made two different property shapes compare equal; measured on the
//! same run, the re-keying left the gated column at 61 PASS.
//!
//! The comparator is shown to say no by
//! `the_comparator_names_every_planted_mutation`, which mutates one PASS case's
//! expected report in each of the six fields in turn and requires the full
//! tuple to reject each copy by name.
//!
//! # The yardstick is calibrated, not asserted
//!
//! A low conformance score means nothing until you know whether the harness is
//! harsh. `tools/w3c_shacl_pyshacl_oracle.py` runs the same walk and the same
//! comparison with pyshacl 0.40.1 as the engine. On 2026-09-13, suite commit
//! 94d8bc2, pyshacl scored PASS 111 / FAIL 8 / ERROR 1 of 120 under this exact
//! lens, against this engine's PASS 32 / FAIL 26 / UNDETERMINED 62 / ERROR 0.
//! The comparison is therefore not the reason the number is low. Re-run that
//! script whenever this file's comparison changes; it is not in CI, because a
//! gate should not move when someone else's engine does.
//!
//! That figure is for the gated column as it stood before #319, with every
//! blank source shape collapsed to `_:`; the script still applies that lens.
//! Per-case arbitration of this engine against pyshacl on the W3C expected
//! reports, with severity and the re-keyed source shape in the key, is
//! `tools/shacl_differential.py --w3c tests/w3c-shacl`, which the
//! `shacl-differential` CI job runs.
//!
//! # Four buckets, because this engine has three answers
//!
//! PASS         the verdict and the result set both agree; or the test expects
//!              `sht:Failure` and `validate` returned an error, which is the
//!              suite's definition of passing such a test.
//! FAIL         the engine answered, and the answer is wrong. The serious
//!              bucket. Sub-labelled `verdict` (wrong `sh:conforms`) or
//!              `results` (right verdict, wrong result set).
//! UNDETERMINED `conforms: null`. Not a pass and not a failure: the engine
//!              declined to answer because a constraint was not evaluated
//!              (`skipped_constraints`) or no focus node matched
//!              (`unmatched_shapes`). That refusal is the whole design of this
//!              validator and it is counted on its own line. It is NOT credit.
//! ERROR        `validate` returned `Err`, or a graph did not load. Also its
//!              own line, because a crash and a wrong answer need different
//!              work.
//!
//! # The gate is a ratchet
//!
//! This engine does not pass the whole suite and will not tomorrow. A gate set
//! at "pass everything" would be red forever, which is the same as no gate; a
//! gate set at "run it and print" is the silent success this codebase keeps
//! finding in other people's repositories. So the committed baseline in
//! `tests/w3c_shacl_baseline.json` records today's answer per test, and the
//! gate is monotone:
//!
//!   1. the PASS count may not drop;
//!   2. the FAIL count may not rise;
//!   3. no test that passed in the baseline may stop passing, by name.
//!
//! Rule 3 is stronger than the counts and is here because a change that breaks
//! one test and fixes another leaves both counts flat. All three can only be
//! satisfied by improvement or by a deliberate, reviewable edit to the
//! baseline, which is a diff a reviewer can see. Re-cut it with
//! `OO_W3C_SHACL_UPDATE_BASELINE=1 cargo test --test w3c_shacl_conformance_test`.
//!
//! UNDETERMINED and ERROR are reported but not gated. Gating UNDETERMINED
//! downward would push against the engine's own design principle, and the work
//! that moves a test out of it lands as a PASS, which rule 1 already counts.
//!
//! # One workaround, which is itself a finding
//!
//! `ShaclValidator::validate` takes the shapes graph as inline Turtle and
//! parses it with `RdfParser::from_format(RdfFormat::Turtle)` and no base IRI.
//! Every file in this suite uses relative IRIs (`<>` for "this graph",
//! `<minLength-001>` for the entry), which is what any shapes graph published
//! at a URL looks like, and oxigraph rejects a relative IRI outright when no
//! base is set. Handing the raw file to `validate` therefore fails to parse for
//! 121 of 121 tests. The harness loads the shapes file into a `GraphStore` with
//! an explicit base and hands `validate` the resulting N-Triples, which is
//! valid Turtle with every IRI already absolute. That is a harness workaround
//! for an engine gap: the public SHACL entry point has no way to supply a base
//! IRI. Reported, not fixed, because this task is measurement.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::model::{NamedNode, NamedOrBlankNodeRef, Term};
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// The base every relative IRI in the suite resolves against.
///
/// A fixed https base rather than the local `file://` path: the resolved IRIs
/// end up inside `sh:sourceShape` on both the expected and the actual side, so
/// a base that varied with the checkout directory would make the comparison
/// machine-dependent. This one is the suite's published location.
const SUITE_BASE: &str = "https://w3c.github.io/data-shapes/data-shapes-test-suite/tests/";

/// The upstream commit `tests/w3c-shacl/` was taken from. Recorded in the
/// baseline so a suite refresh and an engine change are distinguishable in a
/// diff.
const SUITE_COMMIT: &str = "94d8bc2bd4fc4fdc6f2964d1ec4a892329e05f06";

/// Entries reachable from the root manifest at `SUITE_COMMIT`.
///
/// The tree carries 121 files with an `sht:Validate` entry and 120 of them are
/// reachable. `sparql/component/nodeValidator-001.ttl` is not named by
/// `sparql/component/manifest.ttl`, so no manifest-driven harness runs it,
/// including the Working Group's own. That is an upstream defect against the
/// suite's own rule that "all approved tests must be reachable from the root
/// manifest", not a gap in this walk: see `the_vendored_suite_is_whole`, which
/// names every unreachable file so a refresh that fixes it upstream is visible
/// here rather than silent.
const EXPECTED_ENTRIES: usize = 120;

/// Files carrying an `sht:Validate` entry that no manifest includes, at
/// `SUITE_COMMIT`. Named, not ignored.
const KNOWN_ORPHANS: [&str; 1] = ["sparql/component/nodeValidator-001.ttl"];

const PREFIXES: &str = r#"
PREFIX mf:   <http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#>
PREFIX sht:  <http://www.w3.org/ns/shacl-test#>
PREFIX sh:   <http://www.w3.org/ns/shacl#>
PREFIX rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
"#;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn suite_root() -> PathBuf {
    repo().join("tests").join("w3c-shacl")
}

fn baseline_path() -> PathBuf {
    repo().join("tests").join("w3c_shacl_baseline.json")
}

/// Loud skip when the vendored suite is not in the tree, fatal under
/// `OO_REQUIRE_FIXTURES=1`. The CI job sets that, so the gate cannot be
/// satisfied by the suite having gone missing.
fn skip() -> bool {
    common::skip_unless(
        suite_root().join("manifest.ttl").is_file(),
        "the vendored W3C SHACL test suite at tests/w3c-shacl/",
        "see tests/w3c-shacl/README.md for the refresh command; it is normally committed",
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Manifest parsing
// ─────────────────────────────────────────────────────────────────────────────

/// The gated column's key: what one validation result is compared on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ResultKey {
    focus: String,
    shape: String,
    component: String,
}

impl std::fmt::Display for ResultKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", short(&self.focus), short(&self.shape), short(&self.component))
    }
}

/// Everything the suite's full-compliance lens compares on one result, except
/// `sh:resultMessage` (see the module docs). The second, separately ratcheted
/// column compares a MULTISET of these.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FullKey {
    focus: String,
    path: String,
    value: String,
    shape: String,
    component: String,
    severity: String,
}

/// Field names in the order `FullKey::fields` returns them. A mismatch names
/// fields from this list, and the planted controls assert on these names.
const FULL_FIELDS: [&str; 6] = ["focus", "path", "value", "source shape", "component", "severity"];

impl FullKey {
    fn fields(&self) -> [&str; 6] {
        [&self.focus, &self.path, &self.value, &self.shape, &self.component, &self.severity]
    }
    fn gated(&self) -> ResultKey {
        ResultKey { focus: self.focus.clone(), shape: self.shape.clone(), component: self.component.clone() }
    }
}

impl std::fmt::Display for FullKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parts: Vec<String> = self.fields().iter().map(|x| short(x)).collect();
        write!(f, "({})", parts.join(", "))
    }
}

#[derive(Debug, Clone)]
enum Expectation {
    /// `mf:result sht:Failure`: the engine is required to report a failure
    /// rather than a verdict.
    Failure,
    Report {
        conforms: bool,
        /// One entry per expected `sh:result`, sorted, duplicates kept.
        results: Vec<FullKey>,
    },
}

#[derive(Debug, Clone)]
struct TestCase {
    /// `core/node/minLength-001`, stable across machines, used as the baseline key.
    name: String,
    label: String,
    status: String,
    data: PathBuf,
    shapes: PathBuf,
    expected: Expectation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    Pass,
    Fail(String),
    Undetermined(String),
    Error(String),
}

impl Outcome {
    fn bucket(&self) -> &'static str {
        match self {
            Outcome::Pass => "PASS",
            Outcome::Fail(_) => "FAIL",
            Outcome::Undetermined(_) => "UNDETERMINED",
            Outcome::Error(_) => "ERROR",
        }
    }
    fn detail(&self) -> &str {
        match self {
            Outcome::Pass => "",
            Outcome::Fail(d) | Outcome::Undetermined(d) | Outcome::Error(d) => d,
        }
    }
}

/// Last path segment or fragment of an IRI, for readable tables. Left alone if
/// it is not an IRI, so literals and the `_:` token survive unharmed.
fn short(iri: &str) -> String {
    if iri.starts_with('"') || iri.starts_with("_:") || iri.contains(' ') || !iri.contains(['#', '/']) {
        return iri.to_string();
    }
    iri.rsplit(['#', '/']).next().unwrap_or(iri).to_string()
}

/// Canonical comparison form for one RDF term printed in N-Triples style.
///
/// IRIs lose their angle brackets (the engine already strips them, so this
/// makes the two sides symmetric). Blank nodes collapse to `_:`; a blank
/// source shape is keyed by `shape_key` before it reaches here. Literals are left exactly as oxigraph printed them,
/// which is the N-Triples canonical form on both sides.
fn canon(term: &str) -> String {
    let t = term.trim();
    if t.starts_with("_:") {
        return "_:".to_string();
    }
    if let Some(inner) = t.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        return inner.to_string();
    }
    t.to_string()
}

const SH: &str = "http://www.w3.org/ns/shacl#";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/// Objects of `(s, p, ?)` in `store`, through the store API rather than SPARQL,
/// because a blank node cannot be named in a query and every interesting
/// subject here (a result, a property shape, a path) is one.
fn objects(store: &Store, s: &Term, p: &str) -> Vec<Term> {
    let subject = match s {
        Term::NamedNode(n) => NamedOrBlankNodeRef::from(n.as_ref()),
        Term::BlankNode(b) => NamedOrBlankNodeRef::from(b.as_ref()),
        _ => return Vec::new(),
    };
    let predicate = NamedNode::new_unchecked(p);
    store
        .quads_for_pattern(Some(subject), Some(predicate.as_ref()), None, None)
        .filter_map(Result::ok)
        .map(|q| q.object)
        .collect()
}

fn subjects(store: &Store, p: &str, o: &Term) -> Vec<Term> {
    let predicate = NamedNode::new_unchecked(p);
    store
        .quads_for_pattern(None, Some(predicate.as_ref()), Some(o.as_ref()), None)
        .filter_map(Result::ok)
        .map(|q| Term::from(q.subject))
        .collect()
}

fn one(store: &Store, s: &Term, p: &str) -> Option<Term> {
    objects(store, s, p).into_iter().next()
}

/// Canonical text for a SHACL property path, read structurally from the graph.
///
/// A well-formed SHACL path (spec 2.3.1) is a finite tree over a closed
/// vocabulary with no cycles, so rendering it recursively is a canonical form:
/// two path subgraphs are isomorphic exactly when their renderings are equal,
/// which is what RDFC-1.0 would decide for this sublanguage, without carrying a
/// canonicaliser in a test. The rendering matches what the engine prints in
/// `path`: a bare IRI for a predicate and `^IRI` for `sh:inversePath`.
fn path_form(store: &Store, t: &Term, depth: usize) -> String {
    if depth > 32 {
        return "(path too deep)".to_string();
    }
    let Term::BlankNode(_) = t else { return canon(&t.to_string()) };
    let sub = |x: &Term| path_form(store, x, depth + 1);
    let list = |head: &Term, sep: &str| -> String {
        let mut items = Vec::new();
        let mut cur = head.clone();
        while let Some(first) = one(store, &cur, &format!("{RDF}first")) {
            items.push(sub(&first));
            match one(store, &cur, &format!("{RDF}rest")) {
                Some(rest) => cur = rest,
                None => break,
            }
        }
        format!("({})", items.join(sep))
    };
    if let Some(x) = one(store, t, &format!("{SH}inversePath")) {
        return format!("^{}", sub(&x));
    }
    if let Some(x) = one(store, t, &format!("{SH}alternativePath")) {
        return list(&x, "|");
    }
    for (pred, op) in [("zeroOrMorePath", "*"), ("oneOrMorePath", "+"), ("zeroOrOnePath", "?")] {
        if let Some(x) = one(store, t, &format!("{SH}{pred}")) {
            return format!("({}){op}", sub(&x));
        }
    }
    if one(store, t, &format!("{RDF}first")).is_some() {
        return list(t, "/");
    }
    "_:".to_string()
}

/// The comparison key for a source shape.
///
/// An IRI is itself. A blank-node property shape is keyed by the IRI of the
/// node shape that holds it under `sh:property` plus its `sh:path`, which is
/// the identity a blank property shape actually has: two parses of one file
/// agree on it, and so do two engines. Anything that key cannot name (a blank
/// node shape, a property shape reached from two parents or from a blank one)
/// stays the opaque `_:`, and stays counted as looser than the rest.
fn shape_key(parent: &str, path: &str) -> String {
    format!("{parent} sh:property [sh:path {path}]")
}

fn expected_shape(store: &Store, shape: Option<Term>) -> String {
    let Some(shape) = shape else { return "(none)".to_string() };
    if !matches!(shape, Term::BlankNode(_)) {
        return canon(&shape.to_string());
    }
    let parents = subjects(store, &format!("{SH}property"), &shape);
    match (parents.as_slice(), one(store, &shape, &format!("{SH}path"))) {
        ([Term::NamedNode(parent)], Some(path)) => {
            shape_key(parent.as_str(), &path_form(store, &path, 0))
        }
        _ => "_:".to_string(),
    }
}

/// Severity as a full IRI. The engine reports the local name only (it keeps
/// what follows the last `#` of `sh:severity`), so a bare name is read as
/// being in the SHACL namespace. A custom severity outside it therefore does
/// not round-trip, and `core/misc/severity-002` is where that shows.
fn severity_iri(s: &str) -> String {
    if s.contains(':') { canon(s) } else { format!("{SH}{s}") }
}

/// One expected `sh:result`, read in full.
fn expected_result(store: &Store, res: &Term) -> FullKey {
    let get = |p: &str| one(store, res, &format!("{SH}{p}"));
    let term = |p: &str| get(p).map(|t| canon(&t.to_string())).unwrap_or_else(|| "(none)".to_string());
    FullKey {
        focus: term("focusNode"),
        path: get("resultPath").map(|p| path_form(store, &p, 0)).unwrap_or_else(|| "(none)".to_string()),
        value: term("value"),
        shape: expected_shape(store, get("sourceShape")),
        component: term("sourceConstraintComponent"),
        severity: term("resultSeverity"),
    }
}

/// One result as the engine reported it, in the same key space.
fn actual_result(v: &serde_json::Value) -> FullKey {
    let field = |k: &str| v[k].as_str().filter(|s| !s.is_empty());
    let path = field("result_path").or_else(|| field("path")).map(canon);
    let shape = match field("source_shape") {
        None => "(none)".to_string(),
        Some(s) if s.starts_with("_:") => match (field("node_shape"), path.as_deref()) {
            (Some(parent), Some(p)) if !parent.starts_with("_:") => shape_key(&canon(parent), p),
            _ => "_:".to_string(),
        },
        Some(s) => canon(s),
    };
    FullKey {
        focus: canon(field("focus_node").unwrap_or("(none)")),
        path: path.unwrap_or_else(|| "(none)".to_string()),
        value: field("value").map(canon).unwrap_or_else(|| "(none)".to_string()),
        shape,
        component: canon(field("source_constraint_component").unwrap_or("(none)")),
        severity: field("severity").map(severity_iri).unwrap_or_else(|| "(none)".to_string()),
    }
}

fn rows(store: &Store, query: &str) -> anyhow::Result<Vec<HashMap<String, String>>> {
    let mut out = Vec::new();
    if let QueryResults::Solutions(solutions) =
        SparqlEvaluator::new().parse_query(query)?.on_store(store).execute()?
    {
        let vars: Vec<String> = solutions.variables().iter().map(|v| v.as_str().to_string()).collect();
        for solution in solutions {
            let solution = solution?;
            let mut row = HashMap::new();
            for v in &vars {
                if let Some(t) = solution.get(v.as_str()) {
                    row.insert(v.clone(), t.to_string());
                }
            }
            out.push(row);
        }
    }
    Ok(out)
}

fn cell(row: &HashMap<String, String>, var: &str) -> String {
    row.get(var).cloned().unwrap_or_else(|| "(none)".to_string())
}

/// Path under `tests/w3c-shacl`, slash-separated, for a file in the suite.
fn rel(path: &Path) -> String {
    path.strip_prefix(suite_root())
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn base_of(path: &Path) -> String {
    format!("{SUITE_BASE}{}", rel(path))
}

/// Inverse of `base_of`: the local file an absolute suite IRI names.
fn path_of(iri: &str) -> Option<PathBuf> {
    iri.strip_prefix(SUITE_BASE).map(|r| suite_root().join(r))
}

/// Parse one Turtle file into its own store, with the file's own IRI as base.
fn load_graph(path: &Path) -> anyhow::Result<Store> {
    let text = std::fs::read_to_string(path)?;
    let store = Store::new()?;
    let parser = RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(base_of(path))?
        .for_reader(Cursor::new(text.as_bytes()));
    for quad in parser {
        store.insert(&quad?)?;
    }
    Ok(store)
}

/// Walk `mf:include` from the root manifest and collect every `sht:Validate`
/// entry. Entries before includes, includes in sorted order, so the run order
/// is fixed on every machine.
fn collect_tests() -> anyhow::Result<(Vec<TestCase>, BTreeSet<PathBuf>)> {
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    let mut out: Vec<TestCase> = Vec::new();
    walk(&suite_root().join("manifest.ttl"), &mut seen, &mut out)?;
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((out, seen))
}

/// Files under `tests/w3c-shacl/` that declare an `sht:Validate` entry and that
/// the include walk never reached, so nothing runs them. Returned rather than
/// asserted on, so `the_vendored_suite_is_whole` can name each one.
fn unreachable_tests(reached: &BTreeSet<PathBuf>) -> Vec<String> {
    fn visit(dir: &Path, reached: &BTreeSet<PathBuf>, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();
        for p in paths {
            if p.is_dir() {
                visit(&p, reached, out);
            } else if p.extension().is_some_and(|e| e == "ttl") {
                let canonical = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
                if reached.contains(&canonical) {
                    continue;
                }
                if std::fs::read_to_string(&p).is_ok_and(|t| t.contains("sht:Validate")) {
                    out.push(rel(&p));
                }
            }
        }
    }
    let mut out = Vec::new();
    visit(&suite_root(), reached, &mut out);
    out.sort();
    out
}

fn walk(manifest: &Path, seen: &mut BTreeSet<PathBuf>, out: &mut Vec<TestCase>) -> anyhow::Result<()> {
    let canonical = std::fs::canonicalize(manifest).unwrap_or_else(|_| manifest.to_path_buf());
    if !seen.insert(canonical) {
        return Ok(());
    }
    let store = load_graph(manifest)?;

    for entry in entries(&store)? {
        out.push(read_entry(&store, manifest, &entry)?);
    }

    let mut includes: Vec<String> =
        rows(&store, &format!("{PREFIXES} SELECT ?i WHERE {{ ?m mf:include ?i }}"))?
            .into_iter()
            .filter_map(|r| r.get("i").map(|s| canon(s)))
            .collect();
    includes.sort();
    includes.dedup();
    for inc in includes {
        match path_of(&inc) {
            Some(p) if p.is_file() => walk(&p, seen, out)?,
            // A network fetch here would defeat the point of vendoring, so an
            // include that escapes the tree is an error rather than a skip.
            _ => anyhow::bail!("{}: mf:include leaves the vendored suite: {inc}", rel(manifest)),
        }
    }
    Ok(())
}

fn entries(store: &Store) -> anyhow::Result<Vec<String>> {
    Ok(rows(
        store,
        &format!(
            "{PREFIXES}
             SELECT ?e WHERE {{ ?m mf:entries/rdf:rest*/rdf:first ?e . ?e a sht:Validate . }}"
        ),
    )?
    .into_iter()
    .filter_map(|r| r.get("e").cloned())
    .collect())
}

fn read_entry(store: &Store, manifest: &Path, entry: &str) -> anyhow::Result<TestCase> {
    let action = rows(
        store,
        &format!(
            "{PREFIXES}
             SELECT ?d ?s WHERE {{ {entry} mf:action ?a . ?a sht:dataGraph ?d ; sht:shapesGraph ?s . }}"
        ),
    )?;
    let Some(a) = action.first() else {
        anyhow::bail!("{}: entry {entry} has no mf:action naming both graphs", rel(manifest));
    };
    let data = path_of(&canon(&cell(a, "d")))
        .ok_or_else(|| anyhow::anyhow!("{}: sht:dataGraph outside the suite", rel(manifest)))?;
    let shapes = path_of(&canon(&cell(a, "s")))
        .ok_or_else(|| anyhow::anyhow!("{}: sht:shapesGraph outside the suite", rel(manifest)))?;

    let label = rows(store, &format!("{PREFIXES} SELECT ?l WHERE {{ {entry} rdfs:label ?l }}"))?
        .first()
        .map(|r| cell(r, "l").trim_matches('"').to_string())
        .unwrap_or_default();
    let status = rows(store, &format!("{PREFIXES} SELECT ?s WHERE {{ {entry} mf:status ?s }}"))?
        .first()
        .map(|r| short(&canon(&cell(r, "s"))))
        .unwrap_or_else(|| "unstated".to_string());

    // `sht:Failure` sits in object position as an IRI; an expected report is a
    // blank node carrying sh:conforms.
    let result_term = rows(store, &format!("{PREFIXES} SELECT ?r WHERE {{ {entry} mf:result ?r }}"))?
        .first()
        .map(|r| cell(r, "r"))
        .unwrap_or_else(|| "(none)".to_string());

    let expected = if canon(&result_term) == "http://www.w3.org/ns/shacl-test#Failure" {
        Expectation::Failure
    } else {
        let conforms = rows(
            store,
            &format!("{PREFIXES} SELECT ?c WHERE {{ {entry} mf:result ?r . ?r sh:conforms ?c }}"),
        )?
        .first()
        .map(|r| {
            let c = cell(r, "c");
            c.starts_with("\"true\"") || c == "true"
        })
        .ok_or_else(|| {
            anyhow::anyhow!("{}: entry {entry} has neither sh:conforms nor sht:Failure", rel(manifest))
        })?;

        let mut results = Vec::new();
        for row in rows(
            store,
            &format!("{PREFIXES} SELECT ?res WHERE {{ {entry} mf:result ?r . ?r sh:result ?res . }}"),
        )? {
            let res: Term = cell(&row, "res")
                .parse()
                .map_err(|e| anyhow::anyhow!("{}: unreadable sh:result node: {e}", rel(manifest)))?;
            results.push(expected_result(store, &res));
        }
        results.sort();
        Expectation::Report { conforms, results }
    };

    let mut name = rel(manifest);
    name.truncate(name.len().saturating_sub(".ttl".len()));
    let local = short(&canon(entry));
    if !name.ends_with(&local) {
        name = format!("{name}::{local}");
    }

    Ok(TestCase { name, label, status, data, shapes, expected })
}

// ─────────────────────────────────────────────────────────────────────────────
// Running one test
// ─────────────────────────────────────────────────────────────────────────────

/// What the engine answered for one test: its verdict and its results, or the
/// outcome that ends the test before any comparison. Run once per test and
/// judged once per column.
fn engine_answer(tc: &TestCase) -> Result<(bool, Vec<FullKey>), Outcome> {
    // Data graph: loaded with its own IRI as base, so `<>` and friends resolve
    // to the same terms the expected report names.
    let data = Arc::new(GraphStore::new());
    let data_text = match std::fs::read_to_string(&tc.data) {
        Ok(t) => t,
        Err(e) => return Err(Outcome::Error(format!("data graph unreadable: {e}"))),
    };
    if let Err(e) = data.load_turtle(&data_text, Some(&base_of(&tc.data))) {
        return Err(Outcome::Error(format!("data graph does not parse: {}", first_line(&e.to_string()))));
    }

    // Shapes graph: through a store and back out as N-Triples. See the module
    // docs; `validate` cannot be given a base IRI, and every file here uses
    // relative IRIs.
    let shapes_store = GraphStore::new();
    let shapes_text = match std::fs::read_to_string(&tc.shapes) {
        Ok(t) => t,
        Err(e) => return Err(Outcome::Error(format!("shapes graph unreadable: {e}"))),
    };
    if let Err(e) = shapes_store.load_turtle(&shapes_text, Some(&base_of(&tc.shapes))) {
        return Err(Outcome::Error(format!("shapes graph does not parse: {}", first_line(&e.to_string()))));
    }
    let shapes_nt = match shapes_store.serialize("ntriples") {
        Ok(s) => s,
        Err(e) => return Err(Outcome::Error(format!("shapes graph will not re-serialise: {e}"))),
    };

    let raw = match ShaclValidator::validate(&data, &shapes_nt) {
        Ok(r) => r,
        Err(e) => {
            // The suite's rule: a test whose mf:result is sht:Failure passes if
            // the validation reported a failure. An error IS that report.
            return Err(match tc.expected {
                Expectation::Failure => Outcome::Pass,
                _ => Outcome::Error(format!("validate returned Err: {}", first_line(&e.to_string()))),
            });
        }
    };
    let report: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => return Err(Outcome::Error(format!("report is not JSON: {e}"))),
    };

    // The third answer, before any comparison. An engine that declined to
    // answer has not passed and has not failed, whatever the expectation was.
    if report["conforms"].is_null() {
        return Err(Outcome::Undetermined(why_undetermined(&report)));
    }

    let Some(actual_conforms) = report["conforms"].as_bool() else {
        return Err(Outcome::Error(format!("conforms is neither bool nor null: {}", report["conforms"])));
    };

    let mut actual: Vec<FullKey> = report["violations"]
        .as_array()
        .map(|vs| vs.iter().map(actual_result).collect())
        .unwrap_or_default();
    actual.sort();
    Ok((actual_conforms, actual))
}

/// One column's outcome for one test.
fn judge(tc: &TestCase, column: Column, answer: &Result<(bool, Vec<FullKey>), Outcome>) -> Outcome {
    let (actual_conforms, actual) = match answer {
        Ok((c, a)) => (*c, a),
        Err(o) => return o.clone(),
    };
    match &tc.expected {
        Expectation::Failure => Outcome::Fail(format!(
            "verdict: the suite requires a reported failure for this shapes graph; \
             the engine returned conforms={actual_conforms}"
        )),
        Expectation::Report { conforms, results } => {
            if actual_conforms != *conforms {
                return Outcome::Fail(format!(
                    "verdict: expected conforms={conforms}, got {actual_conforms} \
                     ({} expected result(s), {} produced)",
                    results.len(),
                    actual.len()
                ));
            }
            match compare(column, results, actual) {
                None => Outcome::Pass,
                Some(why) => Outcome::Fail(format!("results: verdict agrees (conforms={conforms}) but {why}")),
            }
        }
    }
}

/// The two ratcheted columns. Both see the same report and the same verdict
/// and differ only in how a result set is compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    /// The SET of (focus, source shape, component). The original column.
    Gated,
    /// The MULTISET of all six `FULL_FIELDS`.
    FullTuple,
}

/// `None` when the result sets agree under `column`, otherwise a description
/// of the difference that, for the full tuple, names the differing fields.
fn compare(column: Column, expected: &[FullKey], actual: &[FullKey]) -> Option<String> {
    match column {
        Column::Gated => {
            let e: BTreeSet<ResultKey> = expected.iter().map(FullKey::gated).collect();
            let a: BTreeSet<ResultKey> = actual.iter().map(FullKey::gated).collect();
            if e == a {
                return None;
            }
            let missing: Vec<String> = e.difference(&a).map(|k| k.to_string()).collect();
            let extra: Vec<String> = a.difference(&e).map(|k| k.to_string()).collect();
            Some(format!(
                "the result set differs; {} missing {}, {} unexpected {}",
                missing.len(),
                preview(&missing),
                extra.len(),
                preview(&extra)
            ))
        }
        Column::FullTuple => {
            let (missing, extra) = multiset_difference(expected, actual);
            if missing.is_empty() && extra.is_empty() {
                return None;
            }
            // Name the fields: pair each missing result with the unexpected one
            // it is closest to and list where the two differ. A missing result
            // with nothing to pair against is a dropped or a duplicated result,
            // and is named as multiplicity.
            let mut fields: BTreeSet<&str> = BTreeSet::new();
            for m in &missing {
                match extra.iter().map(|x| differing_fields(m, x)).min_by_key(Vec::len) {
                    Some(d) => fields.extend(d),
                    None => {
                        fields.insert("multiplicity");
                    }
                }
            }
            if missing.is_empty() {
                fields.insert("multiplicity");
            }
            let shown = |v: &[&FullKey]| preview(&v.iter().map(|k| k.to_string()).collect::<Vec<_>>());
            Some(format!(
                "the full tuple differs in [{}]; {} missing {}, {} unexpected {}",
                fields.into_iter().collect::<Vec<_>>().join(", "),
                missing.len(),
                shown(&missing),
                extra.len(),
                shown(&extra)
            ))
        }
    }
}

fn multiset_difference<'a>(expected: &'a [FullKey], actual: &'a [FullKey]) -> (Vec<&'a FullKey>, Vec<&'a FullKey>) {
    let mut count: BTreeMap<&FullKey, i64> = BTreeMap::new();
    for k in expected {
        *count.entry(k).or_default() += 1;
    }
    for k in actual {
        *count.entry(k).or_default() -= 1;
    }
    let mut missing = Vec::new();
    let mut extra = Vec::new();
    for (k, n) in count {
        for _ in 0..n.max(0) {
            missing.push(k);
        }
        for _ in 0..(-n).max(0) {
            extra.push(k);
        }
    }
    (missing, extra)
}

fn differing_fields(a: &FullKey, b: &FullKey) -> Vec<&'static str> {
    FULL_FIELDS
        .iter()
        .zip(a.fields().iter().zip(b.fields().iter()))
        .filter(|(_, (x, y))| x != y)
        .map(|(name, _)| *name)
        .collect()
}

/// Which of the two routes to `conforms: null` this report took, and what it
/// names. Reported rather than collapsed, because "a constraint was not
/// implemented" and "no focus node matched" are different pieces of work.
fn why_undetermined(report: &serde_json::Value) -> String {
    if let Some(skipped) = report["skipped_constraints"].as_array() {
        let kinds: BTreeSet<String> = skipped
            .iter()
            .map(|s| {
                s["constraint"]
                    .as_str()
                    .or_else(|| s["reason"].as_str())
                    .or_else(|| s.as_str())
                    .unwrap_or("unnamed")
                    .to_string()
            })
            .collect();
        let named = if kinds.is_empty() {
            "unnamed".to_string()
        } else {
            kinds.into_iter().collect::<Vec<_>>().join(", ")
        };
        format!("skipped {} constraint(s): {named}", skipped.len())
    } else {
        let n = report["unmatched_shapes"].as_array().map(|a| a.len()).unwrap_or(0);
        format!("no focus node matched ({n} unmatched shape(s))")
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or(s).to_string()
}

fn preview(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let shown: Vec<&str> = items.iter().take(2).map(String::as_str).collect();
    if items.len() > 2 {
        format!("[{}, ...]", shown.join("; "))
    } else {
        format!("[{}]", shown.join("; "))
    }
}

/// Constraint components a test turns on, taken from the expected report. Used
/// to cluster failures so the next piece of work is chosen by evidence and not
/// by whichever bug someone tripped over last.
fn components_of(tc: &TestCase) -> BTreeSet<String> {
    match &tc.expected {
        Expectation::Failure => {
            BTreeSet::from(["(sht:Failure, SPARQL pre-binding)".to_string()])
        }
        Expectation::Report { results, .. } => {
            let s: BTreeSet<String> = results.iter().map(|r| short(&r.component)).collect();
            if s.is_empty() {
                // A conforming test names no component, so the feature under
                // test is the directory it lives in.
                BTreeSet::from([format!("(conforming test in {})", area(tc))])
            } else {
                s
            }
        }
    }
}

fn area(tc: &TestCase) -> String {
    tc.name.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_else(|| tc.name.clone())
}

// ─────────────────────────────────────────────────────────────────────────────
// The run, memoised so the tests below share one pass over the suite
// ─────────────────────────────────────────────────────────────────────────────

struct Run {
    cases: Vec<TestCase>,
    /// The gated column, per test.
    outcomes: Vec<Outcome>,
    /// The full-tuple column, per test, from the same engine answer.
    full_outcomes: Vec<Outcome>,
    /// Test files carrying an entry that no manifest includes.
    orphans: Vec<String>,
    counts: Counts,
    full_counts: Counts,
    /// Tests that agreed on `sh:conforms` whatever the result set did. The
    /// suite calls this partial compliance; it is the number this engine could
    /// report to the Working Group today, and it is not the gated number.
    partial: usize,
    /// Tests expecting `sht:Failure` that landed in UNDETERMINED. Under a
    /// lenient reading of "the validation reported a failure" these would be
    /// passes. Counted, never credited.
    failure_via_undetermined: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
struct Counts {
    total: usize,
    pass: usize,
    fail: usize,
    undetermined: usize,
    error: usize,
}

impl Counts {
    fn of(outcomes: &[Outcome]) -> Counts {
        let mut c = Counts { total: outcomes.len(), ..Default::default() };
        for o in outcomes {
            match o {
                Outcome::Pass => c.pass += 1,
                Outcome::Fail(_) => c.fail += 1,
                Outcome::Undetermined(_) => c.undetermined += 1,
                Outcome::Error(_) => c.error += 1,
            }
        }
        c
    }
}

/// One column's committed floor.
#[derive(Debug, Serialize, Deserialize)]
struct ColumnBaseline {
    counts: Counts,
    outcomes: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Baseline {
    /// For the reader of the file, ignored by the gate.
    note: String,
    suite_commit: String,
    counts: Counts,
    partial_compliance: usize,
    /// Per test, so a swap that leaves the counts flat is still a diff.
    outcomes: BTreeMap<String, String>,
    /// The second column, ratcheted by the same three rules on its own.
    full_tuple: ColumnBaseline,
}

fn run() -> &'static Run {
    static RUN: OnceLock<Run> = OnceLock::new();
    RUN.get_or_init(|| {
        let (cases, reached) = collect_tests().expect("walk the vendored W3C SHACL manifests");
        let orphans = unreachable_tests(&reached);
        let mut outcomes = Vec::with_capacity(cases.len());
        let mut full_outcomes = Vec::with_capacity(cases.len());
        let mut partial = 0usize;
        let mut failure_via_undetermined = 0usize;
        for tc in &cases {
            let answer = engine_answer(tc);
            let o = judge(tc, Column::Gated, &answer);
            match &o {
                Outcome::Pass => partial += 1,
                // "results:" means the verdict was right and only the result
                // set was wrong, which is partial compliance.
                Outcome::Fail(d) if d.starts_with("results:") => partial += 1,
                Outcome::Undetermined(_) if matches!(tc.expected, Expectation::Failure) => {
                    failure_via_undetermined += 1
                }
                _ => {}
            }
            full_outcomes.push(judge(tc, Column::FullTuple, &answer));
            outcomes.push(o);
        }
        let counts = Counts::of(&outcomes);
        let full_counts = Counts::of(&full_outcomes);
        Run { cases, outcomes, full_outcomes, orphans, counts, full_counts, partial, failure_via_undetermined }
    })
}

fn outcome_map(r: &Run, outcomes: &[Outcome]) -> BTreeMap<String, String> {
    r.cases
        .iter()
        .zip(outcomes.iter())
        .map(|(c, o)| (c.name.clone(), o.bucket().to_string()))
        .collect()
}

/// The three ratchet rules, in one place so the gate and the test that proves
/// the gate bites cannot drift apart.
fn ratchet_verdict(
    base: &Counts,
    base_outcomes: &BTreeMap<String, String>,
    now: &Counts,
    now_outcomes: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut broken = Vec::new();
    if now.pass < base.pass {
        broken.push(format!("PASS dropped from {} to {}", base.pass, now.pass));
    }
    if now.fail > base.fail {
        broken.push(format!("FAIL rose from {} to {}", base.fail, now.fail));
    }
    let regressed: Vec<&str> = base_outcomes
        .iter()
        .filter(|(n, b)| {
            b.as_str() == "PASS" && now_outcomes.get(*n).map(String::as_str) != Some("PASS")
        })
        .map(|(n, _)| n.as_str())
        .collect();
    if !regressed.is_empty() {
        broken.push(format!(
            "{} test(s) that passed in the baseline no longer pass: {}",
            regressed.len(),
            regressed.join(", ")
        ));
    }
    broken
}

fn pct(n: usize, d: usize) -> f64 {
    if d == 0 { 0.0 } else { 100.0 * n as f64 / d as f64 }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

/// The suite is present, complete and reachable from the root manifest.
///
/// Without this, a botched vendoring that dropped half the tree would show up
/// as an improvement: fewer tests, fewer failures, and the FAIL ratchet
/// satisfied by deletion.
#[test]
fn the_vendored_suite_is_whole() {
    if skip() {
        return;
    }
    let r = run();
    assert!(
        r.cases.len() >= EXPECTED_ENTRIES,
        "the manifest walk found only {} sht:Validate entries; upstream {SUITE_COMMIT} carries \
         {EXPECTED_ENTRIES}. A shrunken suite makes the ratchet meaningless.",
        r.cases.len()
    );
    let approved = r.cases.iter().filter(|c| c.status == "approved").count();
    let proposed = r.cases.iter().filter(|c| c.status == "proposed").count();
    eprintln!("suite: {} entries, {approved} approved, {proposed} proposed", r.cases.len());
    assert_eq!(
        approved + proposed,
        r.cases.len(),
        "every entry must carry mf:status sht:approved or sht:proposed; another status means a \
         test the Working Group rejected, and it must not be scored"
    );
    // Every referenced graph is in the tree, not on the network.
    for c in &r.cases {
        assert!(c.data.is_file(), "{}: data graph missing at {}", c.name, c.data.display());
        assert!(c.shapes.is_file(), "{}: shapes graph missing at {}", c.name, c.shapes.display());
    }
    let names: BTreeSet<&str> = r.cases.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names.len(),
        r.cases.len(),
        "two entries share a baseline key, so one would shadow the other in the ratchet"
    );

    // Every test file the walk did not reach, named. The suite's own rule is
    // that all approved tests are reachable from the root manifest; where that
    // is false upstream, saying so here is the difference between an excluded
    // test and a lost one.
    for orphan in &r.orphans {
        eprintln!("UNREACHABLE_TEST: {orphan} declares an sht:Validate entry that no manifest includes");
    }
    assert_eq!(
        r.orphans,
        KNOWN_ORPHANS.map(str::to_string).to_vec(),
        "the set of unreachable test files changed. If upstream fixed the include, drop it from \
         KNOWN_ORPHANS and raise EXPECTED_ENTRIES; if a new one appeared, a test just went dark."
    );
}

/// The whole report: buckets, clusters, and the ten worst tests.
///
/// Printing is the point, so run it with `-- --nocapture`.
#[test]
fn the_conformance_report() {
    if skip() {
        return;
    }
    let r = run();
    let c = r.counts;

    eprintln!("\n====== W3C SHACL conformance, open-ontologies ======");
    eprintln!("suite commit {SUITE_COMMIT} (2026-09-12)\n");
    eprintln!("  total         {:>4}", c.total);
    eprintln!("  PASS          {:>4}  ({:.1}%)", c.pass, pct(c.pass, c.total));
    eprintln!(
        "  FAIL          {:>4}  ({:.1}%)  <- wrong answer, the serious bucket",
        c.fail,
        pct(c.fail, c.total)
    );
    eprintln!(
        "  UNDETERMINED  {:>4}  ({:.1}%)  <- conforms:null, the engine declined to answer",
        c.undetermined,
        pct(c.undetermined, c.total)
    );
    eprintln!(
        "  ERROR         {:>4}  ({:.1}%)  <- crash or parse failure",
        c.error,
        pct(c.error, c.total)
    );
    eprintln!(
        "\n  partial compliance (sh:conforms agrees, result set ignored): {} of {} ({:.1}%)",
        r.partial,
        c.total,
        pct(r.partial, c.total)
    );
    eprintln!(
        "  sht:Failure tests answered with conforms:null rather than an error: {}",
        r.failure_via_undetermined
    );

    // Where the non-passes cluster, by SHACL constraint component and by area.
    let mut by_component: BTreeMap<String, [usize; 3]> = BTreeMap::new(); // fail, undet, error
    let mut by_area: BTreeMap<String, [usize; 4]> = BTreeMap::new(); // pass, fail, undet, error
    for (tc, o) in r.cases.iter().zip(r.outcomes.iter()) {
        let a = by_area.entry(area(tc)).or_insert([0; 4]);
        let slot = match o {
            Outcome::Pass => 0,
            Outcome::Fail(_) => 1,
            Outcome::Undetermined(_) => 2,
            Outcome::Error(_) => 3,
        };
        a[slot] += 1;
        if slot == 0 {
            continue;
        }
        for comp in components_of(tc) {
            by_component.entry(comp).or_insert([0; 3])[slot - 1] += 1;
        }
    }

    eprintln!("\n-- where the non-passes cluster, by constraint component --");
    eprintln!("  {:<46} {:>5} {:>6} {:>6}", "component", "FAIL", "UNDET", "ERROR");
    let mut ranked: Vec<(&String, &[usize; 3])> = by_component.iter().collect();
    ranked.sort_by(|x, y| {
        (y.1[0] + y.1[2], y.1[1], x.0).cmp(&(x.1[0] + x.1[2], x.1[1], y.0))
    });
    for (comp, n) in &ranked {
        eprintln!("  {:<46} {:>5} {:>6} {:>6}", comp, n[0], n[1], n[2]);
    }

    eprintln!("\n-- by suite area --");
    eprintln!("  {:<32} {:>5} {:>5} {:>6} {:>6}", "area", "PASS", "FAIL", "UNDET", "ERROR");
    for (a, n) in &by_area {
        eprintln!("  {:<32} {:>5} {:>5} {:>6} {:>6}", a, n[0], n[1], n[2], n[3]);
    }

    eprintln!("\n-- the ten worst individual failures (FAIL first, then ERROR) --");
    let mut worst: Vec<(&TestCase, &Outcome)> = r
        .cases
        .iter()
        .zip(r.outcomes.iter())
        .filter(|(_, o)| matches!(o, Outcome::Fail(_) | Outcome::Error(_)))
        .collect();
    worst.sort_by(|x, y| {
        (matches!(x.1, Outcome::Error(_)), &x.0.name).cmp(&(matches!(y.1, Outcome::Error(_)), &y.0.name))
    });
    for (tc, o) in worst.iter().take(10) {
        eprintln!("  {:<42} {}: {}", tc.name, o.bucket(), o.detail());
    }

    eprintln!("\n-- every test --");
    for (tc, o) in r.cases.iter().zip(r.outcomes.iter()) {
        eprintln!("  {:<12} {:<42} {} {}", o.bucket(), tc.name, tc.label, o.detail());
    }

    let f = r.full_counts;
    eprintln!(
        "\n-- full tuple (focus, path, value, source shape, component, severity; multiset) --"
    );
    eprintln!(
        "  PASS {} ({:.1}%)  FAIL {}  UNDETERMINED {}  ERROR {}",
        f.pass,
        pct(f.pass, f.total),
        f.fail,
        f.undetermined,
        f.error
    );
    let mut by_field: BTreeMap<&str, usize> = BTreeMap::new();
    for (tc, o) in r.cases.iter().zip(r.full_outcomes.iter()) {
        if let Outcome::Fail(d) = o
            && d.starts_with("results:")
        {
            for field in FULL_FIELDS.iter().copied().chain(["multiplicity"]) {
                let listed = d
                    .split_once("differs in [")
                    .and_then(|(_, rest)| rest.split_once(']'))
                    .is_some_and(|(inside, _)| inside.split(", ").any(|x| x == field));
                if listed {
                    *by_field.entry(field).or_default() += 1;
                }
            }
            eprintln!("  FAIL         {:<42} {}", tc.name, o.detail());
        }
    }
    for (field, n) in &by_field {
        eprintln!("  tests whose full tuple differs in {field:<13} {n}");
    }
    eprintln!("===================================================\n");

    assert_eq!(
        c.pass + c.fail + c.undetermined + c.error,
        c.total,
        "every test must land in exactly one bucket"
    );
}

/// The ratchet. See the module docs for why it is monotone rather than pass-all.
#[test]
fn the_conformance_ratchet_holds() {
    if skip() {
        return;
    }
    let r = run();
    let now = outcome_map(r, &r.outcomes);
    let now_full = outcome_map(r, &r.full_outcomes);

    if std::env::var("OO_W3C_SHACL_UPDATE_BASELINE").as_deref() == Ok("1") {
        let b = Baseline {
            note: "W3C SHACL test suite conformance, ratcheted in two columns. In each, PASS may \
                   not drop, FAIL may not rise, and no named PASS may regress. `counts` and \
                   `outcomes` are the (focus, source shape, component) set; `full_tuple` adds \
                   path, value and severity and compares multisets. Regenerate with \
                   OO_W3C_SHACL_UPDATE_BASELINE=1 cargo test --test w3c_shacl_conformance_test. \
                   Lowering any number here is a deliberate act and should say why."
                .to_string(),
            suite_commit: SUITE_COMMIT.to_string(),
            counts: r.counts,
            partial_compliance: r.partial,
            outcomes: now,
            full_tuple: ColumnBaseline { counts: r.full_counts, outcomes: now_full },
        };
        std::fs::write(baseline_path(), format!("{}\n", serde_json::to_string_pretty(&b).unwrap()))
            .expect("write the baseline");
        eprintln!("baseline rewritten at {}", baseline_path().display());
        return;
    }

    let text = std::fs::read_to_string(baseline_path()).unwrap_or_else(|e| {
        panic!(
            "no baseline at {}: {e}. Cut one with \
             OO_W3C_SHACL_UPDATE_BASELINE=1 cargo test --test w3c_shacl_conformance_test",
            baseline_path().display()
        )
    });
    let base: Baseline = serde_json::from_str(&text).expect("parse the baseline");

    if base.suite_commit != SUITE_COMMIT || base.counts.total != r.counts.total {
        // Not a failure on its own: upstream adds tests, and a refresh is a
        // deliberate act. Loud, because comparing an engine number against a
        // baseline cut on a different suite is comparing two things.
        eprintln!(
            "NOTE: baseline was cut on suite {} with {} tests; this run is suite {SUITE_COMMIT} \
             with {} tests. Re-cut the baseline.",
            base.suite_commit, base.counts.total, r.counts.total
        );
    }

    let columns = [
        ("set (focus, source shape, component)", &base.counts, &base.outcomes, &r.counts, &now, &r.outcomes),
        (
            "full tuple",
            &base.full_tuple.counts,
            &base.full_tuple.outcomes,
            &r.full_counts,
            &now_full,
            &r.full_outcomes,
        ),
    ];
    let mut failures = Vec::new();
    for (name, base_counts, base_outcomes, now_counts, now_outcomes, outcomes) in columns {
        let broken = ratchet_verdict(base_counts, base_outcomes, now_counts, now_outcomes);
        if !broken.is_empty() {
            // Name what each regressed test now does, so the failure is a
            // diagnosis and not a number.
            let detail: Vec<String> = r
                .cases
                .iter()
                .zip(outcomes.iter())
                .filter(|(c, o)| {
                    base_outcomes.get(&c.name).map(String::as_str) == Some("PASS")
                        && !matches!(o, Outcome::Pass)
                })
                .map(|(c, o)| format!("  {} -> {} ({})", c.name, o.bucket(), o.detail()))
                .collect();
            failures.push(format!(
                "column {name}:\n  {}\n{}\nbaseline {:?}\nnow      {:?}",
                broken.join("\n  "),
                detail.join("\n"),
                base_counts,
                now_counts
            ));
        }
        let gained = now_counts.pass as i64 - base_counts.pass as i64;
        eprintln!(
            "column {name}: PASS {} (baseline {}, {gained:+}), FAIL {} (baseline {}), \
             UNDETERMINED {}, ERROR {}",
            now_counts.pass,
            base_counts.pass,
            now_counts.fail,
            base_counts.fail,
            now_counts.undetermined,
            now_counts.error
        );
        if gained > 0 {
            eprintln!(
                "NOTE: {gained} more test(s) pass in column {name} than the baseline records. \
                 Re-cut it so the gate holds the new floor, or the gain can be lost again silently."
            );
        }
    }
    if !failures.is_empty() {
        panic!(
            "W3C SHACL conformance regressed:\n{}\n\n\
             If this is a deliberate, justified move, re-cut the baseline with \
             OO_W3C_SHACL_UPDATE_BASELINE=1 and say why in the commit message.",
            failures.join("\n\n")
        );
    }
    eprintln!("ratchet ok: partial compliance {} (baseline {})", r.partial, base.partial_compliance);
}

/// A gate that cannot fail is not a gate.
///
/// `ratchet_verdict` is the function the real gate calls, and it is driven here
/// against synthetic counts so all three rules are known to bite rather than
/// assumed to. Without this, a comparison inverted by a typo is a permanently
/// green job.
#[test]
fn the_ratchet_can_say_no() {
    let base = Counts { total: 121, pass: 40, fail: 50, undetermined: 25, error: 6 };
    let outcomes = |v: [(&str, &str); 3]| -> BTreeMap<String, String> {
        v.into_iter().map(|(k, s)| (k.to_string(), s.to_string())).collect()
    };
    let base_outcomes = outcomes([("a", "PASS"), ("b", "PASS"), ("c", "FAIL")]);

    // Holding steady passes.
    assert!(ratchet_verdict(&base, &base_outcomes, &base, &base_outcomes).is_empty());

    // Improving passes: one FAIL becomes a PASS.
    let better = Counts { pass: 41, fail: 49, ..base };
    let better_outcomes = outcomes([("a", "PASS"), ("b", "PASS"), ("c", "PASS")]);
    assert!(ratchet_verdict(&base, &base_outcomes, &better, &better_outcomes).is_empty());

    // Rule 1: fewer passes.
    let fewer = Counts { pass: 39, undetermined: 26, ..base };
    assert!(
        ratchet_verdict(&base, &base_outcomes, &fewer, &base_outcomes)
            .iter()
            .any(|m| m.contains("PASS dropped")),
        "a drop in PASS must be caught"
    );

    // Rule 2: more failures.
    let worse = Counts { fail: 51, undetermined: 24, ..base };
    assert!(
        ratchet_verdict(&base, &base_outcomes, &worse, &base_outcomes)
            .iter()
            .any(|m| m.contains("FAIL rose")),
        "a rise in FAIL must be caught"
    );

    // Rule 3: a swap that leaves both counts flat.
    let swapped = outcomes([("a", "PASS"), ("b", "FAIL"), ("c", "PASS")]);
    let msgs = ratchet_verdict(&base, &base_outcomes, &base, &swapped);
    assert!(
        msgs.iter().any(|m| m.contains("no longer pass") && m.contains('b')),
        "a test swapped out of PASS with both counts unchanged must be caught, got {msgs:?}"
    );
}

/// The W3C case whose expected report the planted controls mutate. Chosen
/// because it is a PASS in the gated column and each of its two expected
/// results carries all six fields, a plain IRI path and a typed-literal value,
/// so no mutation lands on an absent field.
const CONTROL_CASE: &str = "core/property/datatype-001";

/// The comparator can say no, and says which field.
///
/// `the_ratchet_can_say_no` proves the ratchet bites; this proves the thing it
/// ratchets can tell two reports apart. One PASS case's expected report is
/// mutated in exactly one field, six times over, and the full-tuple comparison
/// must reject each copy and name that field and no other. The gated column is
/// held to its own documented key at the same time: it must see the three
/// mutations it compares and must NOT see the three it does not, so the module
/// docs cannot drift from what the code compares. A dropped result and a
/// duplicated one are planted as well, because a multiset comparison that
/// quietly became a set comparison would pass the six field mutations.
#[test]
fn the_comparator_names_every_planted_mutation() {
    if skip() {
        return;
    }
    let r = run();
    let i = r
        .cases
        .iter()
        .position(|c| c.name == CONTROL_CASE)
        .unwrap_or_else(|| panic!("{CONTROL_CASE} is not in the suite"));
    assert_eq!(r.outcomes[i], Outcome::Pass, "{CONTROL_CASE} must be a PASS case to be the control");
    let Expectation::Report { results, .. } = &r.cases[i].expected else {
        panic!("{CONTROL_CASE} must carry an expected report")
    };
    let target = results
        .iter()
        .position(|k| k.fields().iter().all(|f| *f != "(none)" && *f != "_:"))
        .unwrap_or_else(|| panic!("{CONTROL_CASE} has no expected result with all six fields"));

    assert_eq!(compare(Column::FullTuple, results, results), None, "a report must equal itself");
    assert_eq!(compare(Column::Gated, results, results), None, "a report must equal itself");

    let gated_fields = ["focus", "source shape", "component"];
    for (n, name) in FULL_FIELDS.iter().enumerate() {
        let mut mutated = results.clone();
        let k = &mut mutated[target];
        let slot = match n {
            0 => &mut k.focus,
            1 => &mut k.path,
            2 => &mut k.value,
            3 => &mut k.shape,
            4 => &mut k.component,
            _ => &mut k.severity,
        };
        slot.push_str("-planted");
        mutated.sort();

        let why = compare(Column::FullTuple, results, &mutated)
            .unwrap_or_else(|| panic!("a planted change to {name} went unseen by the full tuple"));
        assert!(
            why.contains(&format!("differs in [{name}]")),
            "the full tuple must name exactly `{name}` for a change to {name}, got: {why}"
        );
        assert_eq!(
            compare(Column::Gated, results, &mutated).is_some(),
            gated_fields.contains(name),
            "the gated column's key is (focus, source shape, component); a change to {name} \
             must be seen by it exactly when {name} is in that key"
        );
    }

    let mut dropped = results.clone();
    dropped.remove(target);
    let why = compare(Column::FullTuple, results, &dropped).expect("a dropped result must be seen");
    assert!(why.contains("[multiplicity]"), "a dropped result must be named multiplicity, got: {why}");

    let mut doubled = results.clone();
    doubled.push(results[target].clone());
    doubled.sort();
    let why = compare(Column::FullTuple, results, &doubled).expect("a duplicated result must be seen");
    assert!(why.contains("[multiplicity]"), "a duplicated result must be named multiplicity, got: {why}");
    assert_eq!(
        compare(Column::Gated, results, &doubled),
        None,
        "the gated column compares sets, so a duplicate is invisible to it by design"
    );
}
