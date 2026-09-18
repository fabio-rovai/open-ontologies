//! Graph scope for the verdict-producing tools (#108).
//!
//! The defect: `Reasoner::run` read the store through `GraphStore::all_triples`,
//! which iterates every quad and drops the graph name, and `ShaclValidator::
//! validate` read it through `sparql_select_union`, which makes the default
//! graph the union of every graph. On a store that keeps one entity's versions
//! in one named graph each, both read every version at once and judged a state
//! that held at no instant. Neither had an argument that could change it.
//!
//! Why it needed its own gate rather than the ones already here: a certificate
//! over such a run is CORRECT. `oo-cert` verifies that each derivation follows
//! from the triples in `asserted.tsv`, and it does; `oo-refute` verifies that
//! the premises of a `cax-dw` clash are in `asserted.tsv`, and they are. The
//! graph in `asserted.tsv` is the thing that is wrong, and no checker can see
//! that far. The tests below that name a certificate are testing the one
//! failure this project's gates are blind to.
//!
//! Every test here fails on the tree before the fix. Four of them fail by
//! returning a confident wrong answer (`clash`, `conforms: false`, a written
//! `refutation.tsv`, a certificate with no scope); the rest fail because the
//! argument they pass did not exist.

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{InferenceTarget, Reasoner};
use open_ontologies::shacl::ShaclValidator;
use open_ontologies::temporal::{ScopeRequest, Temporal};
use oxigraph::io::RdfFormat;
use std::sync::Arc;

/// The module's own example, bi-temporal: one cell line, adherent until
/// 1 May 2026 and suspension from 1 May 2026. The two periods are half-open
/// and MEET, so they share no instant. At no moment was HEK293 both.
const TWO_VERSIONS: &str = r#"
@prefix :    <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

:g1 { :HEK293 a :AdherentCellLine . }
:g2 { :HEK293 a :SuspensionCellLine . }
{
  :AdherentCellLine   a owl:Class .
  :SuspensionCellLine a owl:Class ; owl:disjointWith :AdherentCellLine .

  :g1 t:validFrom  "2024-01-01"^^xsd:date ;
      t:validTo    "2026-05-01"^^xsd:date ;
      t:recordedAt "2024-01-05T09:00:00Z"^^xsd:dateTime .
  :g2 t:validFrom  "2026-05-01"^^xsd:date ;
      t:recordedAt "2026-05-02T09:00:00Z"^^xsd:dateTime .
}
"#;

/// The same store with the disagreement carried by a datatype property, so a
/// SHACL cardinality constraint can be written over it.
const TWO_VERSIONS_SHACL: &str = r#"
@prefix :    <https://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

:g1 { :HEK293 a :CellLine ; :growthMode "adherent" . }
:g2 { :HEK293 a :CellLine ; :growthMode "suspension" . }
{
  :g1 t:validFrom  "2024-01-01"^^xsd:date ;
      t:validTo    "2026-05-01"^^xsd:date ;
      t:recordedAt "2024-01-05T09:00:00Z"^^xsd:dateTime .
  :g2 t:validFrom  "2026-05-01"^^xsd:date ;
      t:recordedAt "2026-05-02T09:00:00Z"^^xsd:dateTime .
}
"#;

/// One growth mode at a time. True at every instant; false only of the union
/// of every version, which is a state the register never held.
const ONE_MODE: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix :   <https://example.org/> .

:CellLineShape a sh:NodeShape ;
  sh:targetClass :CellLine ;
  sh:property [ sh:path :growthMode ; sh:maxCount 1 ] .
"#;

/// No temporal vocabulary anywhere. Everything about a run over this store has
/// to be exactly what it was before #108.
const NO_TEMPORAL: &str = r#"
@prefix :    <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .

:g1 { :HEK293 a :CellLine ; :growthMode "adherent" . }
{ :CellLine a owl:Class . }
"#;

fn store(trig: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_content(trig, RdfFormat::TriG).expect("TriG parses");
    g
}

fn json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).expect("tool output is JSON")
}

fn at(valid_at: &str) -> ScopeRequest {
    ScopeRequest::Snapshot {
        valid_at: Some(valid_at.to_string()),
        as_of: None,
    }
}

/// A fresh directory per certificate, named for the test that asked for it, so
/// two tests running in parallel cannot read each other's files.
struct CertDir(std::path::PathBuf);

impl CertDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("oo-108-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Self(dir)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
    fn read(&self, file: &str) -> Option<String> {
        std::fs::read_to_string(self.0.join(file)).ok()
    }
}

impl Drop for CertDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ── The data, and what the temporal layer already said about it ─────────────

/// Quoted so nothing below can be read as a disagreement about the DATA. The
/// one tool that knew about valid time got this right before the fix and after
/// it; the two that produce verdicts did not.
#[test]
fn the_temporal_layer_agrees_the_two_versions_never_overlap() {
    let c = json(&Temporal::new(store(TWO_VERSIONS)).conflicts().unwrap());
    assert_eq!(
        c["contradiction_count"], 0,
        "the two periods meet and share no instant, so this is a correction, not a \
         contradiction: {c}"
    );
}

// ── The refusal ─────────────────────────────────────────────────────────────

#[test]
fn reasoning_over_temporal_data_with_no_snapshot_is_refused() {
    let e = Reasoner::run_full(
        &store(TWO_VERSIONS),
        "owl-rl",
        false,
        InferenceTarget::DefaultGraph,
        None,
    )
    .expect_err("a bi-temporal store with no instant named must not be answered");
    let msg = e.to_string();
    assert!(
        msg.contains("temporal scope not specified") && msg.contains("valid_at"),
        "the refusal has to name the way out, or it is a dead end: {msg}"
    );
}

#[test]
fn shacl_over_temporal_data_with_no_snapshot_is_refused() {
    let e = ShaclValidator::validate(&store(TWO_VERSIONS_SHACL), ONE_MODE)
        .expect_err("a bi-temporal store with no instant named must not be validated");
    assert!(
        e.to_string().contains("temporal scope not specified"),
        "{e}"
    );
}

#[test]
fn a_horn_run_over_temporal_data_with_no_snapshot_is_refused() {
    // The user-supplied-rules path reads the store the same way and writes the
    // same kind of certificate, so it gets the same gate. Refused BEFORE the
    // rule table is read, which is why a path that does not exist is fine here.
    let dir = CertDir::new("horn");
    let e = Reasoner::run_horn(
        &store(TWO_VERSIONS),
        std::path::Path::new("/nonexistent/rules.tsv"),
        dir.path(),
    )
    .expect_err("the Horn path must be gated too");
    assert!(
        e.to_string().contains("temporal scope not specified"),
        "the scope is resolved before the rule table is read: {e}"
    );
}

/// The second door. `onto_reason_incremental` reads the union through
/// `sparql_select_union` and materialises into the default graph, so leaving
/// it ungated would have left a way to get the old behaviour, written into the
/// store, by using a different tool. It has no snapshot form and does not
/// pretend to: the refusal names `onto_reason` for that.
#[test]
fn incremental_reasoning_over_temporal_data_with_no_snapshot_is_refused() {
    use open_ontologies::reason_incremental::{IncrementalReasoner, parse_ntriples};
    let delta = parse_ntriples(
        "<https://example.org/HEK293> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
         <https://example.org/AdherentCellLine> .",
    );
    let store = store(TWO_VERSIONS);
    let e = IncrementalReasoner::run(&store, &delta, true)
        .expect_err("the incremental path must not be a way around the gate");
    let msg = e.to_string();
    assert!(
        msg.contains("temporal scope not specified") && msg.contains("no snapshot form"),
        "{msg}"
    );
    // And it is not a dead end: the union is reachable by saying so.
    let out = json(
        &IncrementalReasoner::run_scoped(&store, &delta, false, true)
            .expect("all_versions says the union was meant"),
    );
    assert_eq!(out["ok"], serde_json::Value::Bool(true), "{out}");
}

#[test]
fn asking_for_one_instant_and_for_every_version_at_once_is_refused() {
    let e = ScopeRequest::from_args(Some("2026-01-01"), None, true)
        .expect_err("two different questions must not be resolved by precedence");
    assert!(e.to_string().contains("different questions"), "{e}");
}

/// The temporal layer reads validity from the DEFAULT graph. A description
/// sitting in a named graph is invisible to it, so every described graph would
/// read as timeless and be in scope at every instant: a confident answer over
/// a scope the store contradicts.
#[test]
fn a_temporal_description_out_of_reach_is_refused_rather_than_ignored() {
    let s = store(
        r#"
@prefix :    <https://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:g1   { :HEK293 a :CellLine . }
:meta { :g1 t:validFrom "2024-01-01"^^xsd:date . }
"#,
    );
    let e = ShaclValidator::validate_scoped(&s, ONE_MODE, &at("2025-01-01"))
        .expect_err("metadata the temporal layer cannot read must not be silently skipped");
    let msg = e.to_string();
    assert!(
        msg.contains("temporal description out of reach") && msg.contains("timeless"),
        "{msg}"
    );
}

// ── The scoped answer ───────────────────────────────────────────────────────

#[test]
fn a_snapshot_reasons_over_one_version_and_finds_no_clash() {
    for instant in ["2025-01-01", "2026-06-01"] {
        let out = json(
            &Reasoner::run_scoped(
                &store(TWO_VERSIONS),
                "owl-rl",
                false,
                InferenceTarget::DefaultGraph,
                None,
                &at(instant),
            )
            .unwrap(),
        );
        assert!(
            out.get("inconsistency").is_none(),
            "at {instant} exactly one version is in scope, so there is no disjointness clash \
             to find: {out}"
        );
        assert_eq!(out["scope"]["graph_count"], 1, "at {instant}: {out}");
    }
}

#[test]
fn a_snapshot_reads_the_version_that_was_true_then_and_no_other() {
    let early = json(
        &Reasoner::run_scoped(
            &store(TWO_VERSIONS),
            "owl-rl",
            false,
            InferenceTarget::DefaultGraph,
            None,
            &at("2025-01-01"),
        )
        .unwrap(),
    );
    let late = json(
        &Reasoner::run_scoped(
            &store(TWO_VERSIONS),
            "owl-rl",
            false,
            InferenceTarget::DefaultGraph,
            None,
            &at("2026-06-01"),
        )
        .unwrap(),
    );
    assert_eq!(early["scope"]["graphs"][0], "https://example.org/g1", "{early}");
    assert_eq!(late["scope"]["graphs"][0], "https://example.org/g2", "{late}");
}

#[test]
fn shacl_conforms_at_each_instant() {
    for instant in ["2025-01-01", "2026-06-01"] {
        let report = json(
            &ShaclValidator::validate_scoped(&store(TWO_VERSIONS_SHACL), ONE_MODE, &at(instant))
                .unwrap(),
        );
        assert_eq!(
            report["conforms"],
            serde_json::Value::Bool(true),
            "HEK293 had exactly one growth mode at {instant}: {report}"
        );
        assert_eq!(report["scope"]["selector"], "temporal-snapshot", "{report}");
    }
}

/// The union answer is not wrong to want; it was wrong to arrive at by
/// omission. Asked for by name it is available, and the report says which
/// question was answered.
#[test]
fn every_version_at_once_is_still_reachable_and_is_labelled() {
    let report = json(
        &ShaclValidator::validate_scoped(
            &store(TWO_VERSIONS_SHACL),
            ONE_MODE,
            &ScopeRequest::AllVersions,
        )
        .unwrap(),
    );
    assert_eq!(
        report["conforms"],
        serde_json::Value::Bool(false),
        "across every version there are two growth modes: {report}"
    );
    assert_eq!(
        report["scope"]["selector"], "whole-store-all-versions",
        "and the report has to say that is the question it answered: {report}"
    );
}

// ── The scope as evidence ───────────────────────────────────────────────────

#[test]
fn a_certificate_records_which_graphs_it_read() {
    let dir = CertDir::new("scope");
    let out = json(
        &Reasoner::run_scoped(
            &store(TWO_VERSIONS_SHACL),
            "rdfs",
            false,
            InferenceTarget::DefaultGraph,
            Some(dir.path()),
            &at("2025-01-01"),
        )
        .unwrap(),
    );
    assert_eq!(
        out["certificate"]["scope"]["selector"], "temporal-snapshot",
        "{out}"
    );
    let tsv = dir.read("scope.tsv").expect("scope.tsv sits beside asserted.tsv");
    assert!(tsv.starts_with("oo-scope/1\n"), "{tsv}");
    assert!(tsv.contains("graph\thttps://example.org/g1\n"), "{tsv}");
    assert!(
        !tsv.contains("graph\thttps://example.org/g2\n"),
        "g2 was out of scope at that instant and must not be listed as read: {tsv}"
    );
    assert!(tsv.contains("valid_at\t2025-01-01\n"), "{tsv}");
}

/// An unscoped run over a store with no versions records its scope too. The
/// key must not appear for the first time on the run where it matters, or a
/// reader has no baseline to compare against.
#[test]
fn an_unscoped_certificate_says_it_read_everything() {
    let dir = CertDir::new("whole");
    let out = json(
        &Reasoner::run_full(
            &store(NO_TEMPORAL),
            "rdfs",
            false,
            InferenceTarget::DefaultGraph,
            Some(dir.path()),
        )
        .unwrap(),
    );
    assert_eq!(out["certificate"]["scope"]["selector"], "whole-store", "{out}");
    let tsv = dir.read("scope.tsv").expect("scope.tsv is written");
    assert!(tsv.contains("graph\t*\n"), "{tsv}");
}

#[test]
fn a_store_with_no_temporal_vocabulary_is_unaffected() {
    let out = json(
        &Reasoner::run_full(
            &store(NO_TEMPORAL),
            "owl-rl",
            false,
            InferenceTarget::DefaultGraph,
            None,
        )
        .unwrap(),
    );
    assert_eq!(out["scope"]["selector"], "whole-store", "{out}");
    let report = json(&ShaclValidator::validate(&store(NO_TEMPORAL), ONE_MODE).unwrap());
    assert_eq!(report["conforms"], serde_json::Value::Bool(true), "{report}");
    assert_eq!(report["scope"]["selector"], "whole-store", "{report}");
}

// ── The sharp end: a valid certificate about the wrong graph ────────────────

/// `cax-dw` is the ONE clash rule `OOCert.RefuteConditions` has a semantic
/// condition for, so before the fix the engine wrote a `refutation.tsv` that
/// `oo-refute check` ACCEPTS: a machine-checked `unsatisfiable_under_
/// disjointness` over a union of versions that never held at once.
#[test]
fn no_machine_checkable_refutation_is_written_over_a_state_that_never_existed() {
    let dir = CertDir::new("refute");
    let out = json(
        &Reasoner::run_scoped(
            &store(TWO_VERSIONS),
            "owl-rl",
            false,
            InferenceTarget::DefaultGraph,
            Some(dir.path()),
            &at("2025-01-01"),
        )
        .unwrap(),
    );
    assert!(
        dir.read("refutation.tsv").is_none(),
        "at 2025-01-01 only the adherent version is in scope, so the graph is consistent and \
         there is nothing to refute. Report: {out}"
    );
    let asserted = dir.read("asserted.tsv").expect("asserted.tsv is written");
    let membership = "<https://example.org/HEK293>\t\
                      <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t\
                      <https://example.org/SuspensionCellLine>";
    assert!(
        !asserted.contains(membership),
        "the out-of-scope version's ABox must not be in the graph the certificate is \
         about:\n{asserted}"
    );
    // The DECLARATION `:SuspensionCellLine a owl:Class` and the disjointness
    // axiom ARE there, and belong there: they sit in the default graph, which
    // is in every snapshot's scope because the schema is not a version. What a
    // snapshot removes is the ABox that made the disjointness bite.
    assert!(asserted.contains("owl#disjointWith"), "{asserted}");
    // And the validity metadata is there too, because the default graph enters
    // wholesale. That is the cost of decision 3 on #108, stated in
    // docs/trusted-computing-base.md rather than hidden: these triples are in
    // the reasoner's closure and in SHACL's focus-node counts.
    assert!(asserted.contains("temporal#validFrom"), "{asserted}");
}

// ── Inferences are not assertions, across runs ──────────────────────────────

/// `docs/trusted-computing-base.md` hazard 5: `all_triples` reads every named
/// graph, so inferences an earlier run parked in the inference graph come back
/// as assertions in a later certified run, and `inference_graph: true` does not
/// stop it. An undescribed graph is also timeless, so the temporal scope would
/// have put it in at every instant. A scoped run drops it and says so.
#[test]
fn an_earlier_runs_inferences_are_not_read_back_as_assertions() {
    let s = store(
        r#"
@prefix :    <https://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:g1 { :HEK293 a :CellLine . }
<https://open-ontologies.org/graph/inferred> { :HEK293 a :SomethingDerived . }
{ :g1 t:validFrom "2024-01-01"^^xsd:date . }
"#,
    );
    let dir = CertDir::new("inferred");
    let out = json(
        &Reasoner::run_scoped(
            &s,
            "rdfs",
            false,
            InferenceTarget::DefaultGraph,
            Some(dir.path()),
            &at("2025-01-01"),
        )
        .unwrap(),
    );
    let asserted = dir.read("asserted.tsv").expect("asserted.tsv is written");
    assert!(
        !asserted.contains("SomethingDerived"),
        "an inference from an earlier run is not an assertion of this one:\n{asserted}"
    );
    let excluded = out["scope"]["excluded"].as_array().expect("excluded rows");
    assert!(
        excluded.iter().any(|r| r["reason"]
            .as_str()
            .is_some_and(|s| s.contains("materialised inferences"))),
        "the drop has to be recorded, not silent: {out}"
    );
}

// ── Materialisation under a scope ───────────────────────────────────────────

/// Both directions of the same rule. A snapshot's conclusions held at one
/// instant and would be read at all of them; an all-versions run's held at
/// none, and writing THOSE into the timeless default graph is the leak of #108
/// arriving through the exit instead of the entrance.
#[test]
fn no_run_over_a_versioned_store_materialises() {
    for (name, request) in [
        ("snapshot", at("2025-01-01")),
        ("all versions", ScopeRequest::AllVersions),
    ] {
        let e = Reasoner::run_scoped(
            &store(TWO_VERSIONS),
            "owl-rl",
            true,
            InferenceTarget::DefaultGraph,
            None,
            &request,
        )
        .unwrap_err();
        assert!(
            e.to_string()
                .contains("run over a versioned store does not materialise"),
            "{name}: {e}"
        );
    }
    // Including through the inference graph, which carries no validity
    // description and is therefore timeless and in scope at every instant.
    let e = Reasoner::run_scoped(
        &store(TWO_VERSIONS),
        "owl-rl",
        true,
        InferenceTarget::Inferred,
        None,
        &at("2025-01-01"),
    )
    .unwrap_err();
    assert!(
        e.to_string().contains("carrying no validity description"),
        "{e}"
    );
    // A store with no versions is untouched: it materialises as it always did.
    let plain = store(NO_TEMPORAL);
    let before = plain.triple_count();
    let out = json(
        &Reasoner::run_full(&plain, "rdfs", true, InferenceTarget::DefaultGraph, None).unwrap(),
    );
    assert!(out.get("dry_run").is_none(), "{out}");
    assert!(plain.triple_count() >= before, "{out}");
}

#[test]
fn a_scoped_run_refuses_the_owl_dl_path_rather_than_ignoring_the_snapshot() {
    let e = Reasoner::run_scoped(
        &store(TWO_VERSIONS),
        "owl-dl",
        false,
        InferenceTarget::DefaultGraph,
        None,
        &at("2025-01-01"),
    )
    .expect_err("the tableaux path has no scoped reader, so a snapshot would be ignored");
    assert!(e.to_string().contains("owl-dl"), "{e}");
}

// ── An empty scope is a non-answer, not a pass ──────────────────────────────

#[test]
fn a_snapshot_selecting_no_graph_does_not_conform_vacuously() {
    let report = json(
        &ShaclValidator::validate_scoped(&store(TWO_VERSIONS_SHACL), ONE_MODE, &at("2000-01-01"))
            .unwrap(),
    );
    assert_eq!(
        report["conforms"],
        serde_json::Value::Null,
        "before either version was valid there was no data to validate, and `true` would be \
         the same lie as reporting it for a constraint that never ran: {report}"
    );
    assert!(
        report["warning"]
            .as_str()
            .is_some_and(|w| w.contains("no graphs in scope")),
        "{report}"
    );
    assert_eq!(report["scope"]["graph_count"], 0, "{report}");
}

// ── The escape a scoped SPARQL constraint must not have ─────────────────────

/// `sh:sparql` runs a query the shapes author wrote, and a query can contain a
/// `GRAPH` block. Restricting the default graph alone would leave that door
/// open: the constraint would read the out-of-scope version and report a
/// violation that is wrong at the instant asked about.
#[test]
fn a_sparql_constraint_cannot_name_its_way_out_of_the_scope() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix :   <https://example.org/> .

:EscapeShape a sh:NodeShape ;
  sh:targetClass :CellLine ;
  sh:sparql [
    sh:message "read an out-of-scope graph" ;
    sh:select """
      SELECT $this WHERE {
        GRAPH <https://example.org/g2> { $this <https://example.org/growthMode> ?m }
      }
    """ ;
  ] .
"#;
    let report = json(
        &ShaclValidator::validate_scoped(&store(TWO_VERSIONS_SHACL), shapes, &at("2025-01-01"))
            .unwrap(),
    );
    assert_eq!(
        report["violation_count"], 0,
        "g2 is out of scope at 2025-01-01, so naming it explicitly must read nothing: {report}"
    );
}
