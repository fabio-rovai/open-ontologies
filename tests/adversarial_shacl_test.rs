//! **Conformance is not truth.** Data that passes a sound SHACL gate and lies.
//!
//! The dual of reward hacking on a verifier. Where a policy learns to satisfy a
//! reward model without doing the task, a data author writes a graph that
//! satisfies a shapes graph without the facts the shapes were written to
//! require. This engine is the place to run that experiment because its SHACL
//! Core evaluator is mechanised in Lean and `Shacl.validate_spec` proves the
//! decision procedure agrees with the specification in both directions, so an
//! attack here cannot be answered with "your validator is buggy". Some of what
//! follows IS a validator bug, and those are labelled as such and kept in their
//! own section; the rest are not, and those are the interesting ones.
//!
//! # Every attack test asserts two things
//!
//! One of them alone is a rhetorical claim.
//!
//!   1. what the gate answered, read out of the report;
//!   2. an oracle saying the data is false, where "false" means something
//!      checkable rather than something argued.
//!
//! The oracles, in the order they appear: `chrono::NaiveDate` refusing to read
//! a lexical form, the same evaluator over the same store through a target form
//! that does reach the check, a second shapes graph over the same store that
//! reports the violation, `Reasoner::derivation_graph` showing that the
//! satisfying triple was concluded by a named rule and never asserted, and
//! `Reasoner::run` finding a clash the Lean refutation checker holds a semantic
//! condition for.
//!
//! # Two buckets, and they are not blurred
//!
//! ENGINE DEFECTS are things this engine does that the specification and its
//! own Lean evaluator do not. They are bugs and belong on a fix list.
//!
//! STANDARD LIMITATIONS are cases where the validator is RIGHT and the data is
//! still false. They are findings about what a conformance verdict means, and
//! nothing in `src/shacl.rs` should change because of them.
//!
//! One candidate was tried and is filed as NEITHER, in its own section, because
//! calling it an attack would have been an overclaim: see
//! `owl_same_as_merges_a_cardinality_that_nothing_here_reports`.
//!
//! # The control
//!
//! An attack surface where every probe succeeds is not a measurement.
//! `a_datatype_the_store_cannot_preserve_yields_no_verdict_rather_than_a_pass`
//! is a probe the existing defence stops, and it is kept in this file rather
//! than in a passing-cases file so that it is read beside the failures. The
//! defence stops it in one direction only, and the test measures the other
//! rather than claiming both.
//!
//! # Where this runs
//!
//! Everywhere `cargo test` runs. It calls no external binary and prints no skip
//! marker, so `tests/ci_gate_coverage_test.rs` has nothing to say about it and
//! `docs/ci-gates.md` needs no row. The one measurement in this file that DID
//! need an external binary, the verified Lean evaluator's verdict on the
//! impossible date, was taken by hand and is recorded in
//! `docs/decisions/0018-conformance-is-not-truth.md` with the command, rather
//! than wired in here where it would have made the file skippable.

use chrono::NaiveDate;
use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{InferenceTarget, Reasoner, Spelled};
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

fn loaded(ttl: &str) -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    store
}

fn report(store: &Arc<GraphStore>, shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(store, shapes).unwrap()).unwrap()
}

const PREFIXES: &str = r#"
    @prefix ex: <http://ex.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
"#;

// ═══════════════════════════════════════════════════════════════════════════
// BUCKET ONE: ENGINE DEFECTS
// ═══════════════════════════════════════════════════════════════════════════

// ── E1. A day that does not exist passes sh:datatype xsd:date ──────────────
//
// BUCKET: engine defect, and NOT FIXED on this branch.
//
// SHACL 4.1.2 makes a literal whose lexical form is outside its datatype's
// lexical space a violation of `sh:datatype`. XSD 1.1 section 3.3.9 puts the
// day-of-month constraint IN the lexical space of `xsd:date`, so
// `"2026-02-31"^^xsd:date` is ill formed and must be reported.
//
// `ill_typed_test` in `src/shacl.rs` is the helper that implements the lexical
// test, and for `xsd:date` it is the regex
// `^-?[0-9]{4}-[0-9]{2}-[0-9]{2}(Z|[+-][0-9]{2}:[0-9]{2})?$`. That is
// positional and counts digits. `2026-02-31` matches it. So does `2026-13-01`.
// Both conform, on both evaluation paths, and this branch does not change that:
// closing it means implementing the XSD date lexical space in Rust, which the
// Lean development already does in `parseDateBody` and the Rust does not, and
// a regex written by hand to approximate it fails in the direction that reports
// violations on valid dates.
//
// The verified evaluator disagrees. Run by hand on 28 September 2026 against
// `lean/.lake/build/bin/oo-shacl validate` over exactly this data and these
// shapes, it returned `conforms: false` with two `DatatypeConstraintComponent`
// results, one per literal, under the theorem `Shacl.validate_spec`. The
// command and its output are in decision 0018. It is not asserted here because
// wiring it in would make this file skip wherever `lake` is absent.

const E1_DATA: &str = r#"
    ex:cert1 a ex:Certificate ; ex:expiresOn "2026-02-31"^^xsd:date .
    ex:cert2 a ex:Certificate ; ex:expiresOn "2026-13-01"^^xsd:date .
"#;

const E1_PROPERTY_SHAPES: &str = r#"
    ex:CertificateShape a sh:NodeShape ;
        sh:targetClass ex:Certificate ;
        sh:property [ sh:path ex:expiresOn ; sh:datatype xsd:date ; sh:minCount 1 ] .
"#;

/// The same constraint reached through `sh:targetObjectsOf`, which binds the
/// LITERAL as the focus node so the node-shape path evaluates it.
const E1_NODE_SHAPES: &str = r#"
    ex:ExpiryShape a sh:NodeShape ;
        sh:targetObjectsOf ex:expiresOn ;
        sh:datatype xsd:date .
"#;

/// A literal that is ill formed by SPELLING rather than by calendar. The
/// lexical test catches this one, which is how the test below can show that the
/// lexical test RAN and still let the impossible dates through.
const E1_MISSPELT_DATA: &str = r#"
    ex:cert3 a ex:Certificate ; ex:expiresOn "31/02/2026"^^xsd:date .
"#;

/// The literal has to survive the store before any verdict over it means
/// anything. Oxigraph normalises some typed literals: `"300"^^xsd:byte` comes
/// back as `"300"^^xsd:integer`, which is what
/// `datatype_is_indistinguishable_in_store` exists for and what the control at
/// the bottom of this file measures. A test that asserted a verdict without
/// asserting this would be blaming the validator for a store rewrite.
#[test]
fn the_impossible_date_reaches_the_store_with_its_datatype_intact() {
    let store = loaded(&format!("{PREFIXES}{E1_DATA}"));
    let json = store
        .sparql_select_union(
            r#"SELECT ?o WHERE { <http://ex.org/cert1> <http://ex.org/expiresOn> ?o }"#,
        )
        .unwrap();
    assert!(
        json.contains("2026-02-31") && json.contains("XMLSchema#date"),
        "the store rewrote the literal, so nothing below is a measurement of the validator: {json}"
    );
}

/// The gate says yes. The calendar says there is no such day.
///
/// This test records a DEFECT that is open. When the XSD date lexical space is
/// implemented in Rust it will fail, and the right response is to change the
/// expectations here and amend decision 0018, not to weaken it.
#[test]
fn a_day_that_does_not_exist_conforms_to_sh_datatype_on_both_paths() {
    let store = loaded(&format!("{PREFIXES}{E1_DATA}"));

    // Oracle. An independent reader of the lexical space, and not this engine.
    // 2026 is not a leap year, February has 28 days, and there is no month 13.
    assert!(
        NaiveDate::parse_from_str("2026-02-31", "%Y-%m-%d").is_err(),
        "31 February is not a day"
    );
    assert!(
        NaiveDate::parse_from_str("2026-13-01", "%Y-%m-%d").is_err(),
        "there is no thirteenth month"
    );
    // And the well-formed neighbour parses, so the oracle is discriminating
    // rather than refusing everything.
    assert!(NaiveDate::parse_from_str("2026-02-28", "%Y-%m-%d").is_ok());

    // The gate, through a property shape.
    let property_path = report(&store, &format!("{PREFIXES}{E1_PROPERTY_SHAPES}"));
    assert_eq!(
        property_path["conforms"],
        serde_json::json!(true),
        "defect record: an expiry that never arrives passes a date constraint: {property_path}"
    );
    assert_eq!(property_path["violation_count"], serde_json::json!(0));
    assert_eq!(property_path["focus_nodes"], serde_json::json!(2));

    // The gate, through a node shape, which is the path that has always run
    // the lexical test. Same answer, so this is not a divergence between two
    // paths: it is one lexical test that is too weak on both.
    let node_path = report(&store, &format!("{PREFIXES}{E1_NODE_SHAPES}"));
    assert_eq!(
        node_path["conforms"],
        serde_json::json!(true),
        "defect record: the node-shape path runs ill_typed_test and still passes: {node_path}"
    );
    assert_eq!(node_path["violation_count"], serde_json::json!(0));
}

/// The lexical test is not absent, it is an under-approximation, and that is a
/// different bug with a different fix. A literal ill formed by SPELLING is
/// caught on both paths; one ill formed only by the CALENDAR is caught on
/// neither. Without this the test above could be read as "no lexical test
/// runs", which was true of the property-shape path before this branch and is
/// not the finding.
#[test]
fn the_lexical_test_runs_and_is_a_strict_under_approximation_of_the_date_space() {
    let misspelt = loaded(&format!("{PREFIXES}{E1_MISSPELT_DATA}"));
    let caught = report(&misspelt, &format!("{PREFIXES}{E1_PROPERTY_SHAPES}"));
    assert_eq!(
        caught["conforms"],
        serde_json::json!(false),
        "a date written dd/mm/yyyy is outside the lexical space by spelling and is reported: \
         {caught}"
    );
    assert_eq!(caught["violation_count"], serde_json::json!(1));

    let impossible = loaded(&format!("{PREFIXES}{E1_DATA}"));
    let missed = report(&impossible, &format!("{PREFIXES}{E1_PROPERTY_SHAPES}"));
    assert_eq!(
        missed["conforms"],
        serde_json::json!(true),
        "and one outside it by calendar is not: {missed}"
    );
}

// ── E2. One evaluator gave two answers to one constraint ───────────────────
//
// BUCKET: engine defect, FIXED on this branch.
//
// `ill_typed_test` had exactly one call site, the node-shape path at
// `src/shacl.rs:475`. The property-shape `sh:datatype` query was
// `FILTER(DATATYPE(?val) != <dt>)` and nothing else. So which answer a shapes
// graph got to one constraint was decided by whether its author wrote the
// constraint under `sh:property`, which is where almost every real shapes
// graph writes it.
//
// Measured before the fix, over one store: `"aldi"^^xsd:integer` reported one
// violation through `sh:targetObjectsOf` and `conforms: true` through
// `sh:property`. This test is the regression gate on that.

const E2_DATA: &str = r#"
    ex:r1 a ex:Reading ; ex:level "aldi"^^xsd:integer .
"#;

const E2_PROPERTY_SHAPES: &str = r#"
    ex:ReadingShape a sh:NodeShape ;
        sh:targetClass ex:Reading ;
        sh:property [ sh:path ex:level ; sh:datatype xsd:integer ] .
"#;

const E2_NODE_SHAPES: &str = r#"
    ex:LevelShape a sh:NodeShape ;
        sh:targetObjectsOf ex:level ;
        sh:datatype xsd:integer .
"#;

#[test]
fn an_ill_formed_literal_violates_sh_datatype_wherever_the_constraint_is_written() {
    let store = loaded(&format!("{PREFIXES}{E2_DATA}"));

    let node_path = report(&store, &format!("{PREFIXES}{E2_NODE_SHAPES}"));
    let property_path = report(&store, &format!("{PREFIXES}{E2_PROPERTY_SHAPES}"));

    assert_eq!(
        node_path["conforms"],
        serde_json::json!(false),
        "the node-shape path has always reported this: {node_path}"
    );
    assert_eq!(
        property_path["conforms"],
        serde_json::json!(false),
        "and the property-shape path must give the same answer to the same constraint: \
         {property_path}"
    );
    assert_eq!(property_path["violation_count"], serde_json::json!(1));
    let violations = property_path["violations"].as_array().unwrap();
    assert_eq!(violations[0]["constraint"], "datatype");
    assert_eq!(
        violations[0]["source_constraint_component"],
        "http://www.w3.org/ns/shacl#DatatypeConstraintComponent"
    );
}

// ── E3. The engine's own conclusion satisfies the constraint asking for one ─
//
// BUCKET: engine defect, and NOT FIXED on this branch.
//
// `Reasoner::run_with_target` with `InferenceTarget::Inferred` writes into
// `https://open-ontologies.org/graph/inferred`, which its own documentation
// calls the place "where nothing downstream can mistake an inference for an
// assertion". `GraphStore::triples_in_scope` honours that under
// `ReadScope::AllGraphs`, citing TCB-8 in its own comment.
// `GraphStore::sparql_select_scoped` does not: for `AllGraphs` it reaches
// `sparql_select_union`, which is every graph in the store. Every data-side
// query the SHACL evaluator emits goes through that path.
//
// The fixture is a claimant writing a subclass axiom about itself. Nobody
// audits anything. `rdfs9` and then `cls-hv1` conclude the evidence triple, the
// validator reads it, and a shapes graph requiring evidence reports a pass.
//
// It is left unfixed here because narrowing the dataset a validation run reads
// changes the public answer for every store that has been materialised, and
// that is a decision about the report rather than a bug fix. Decision 0018
// states it as open and says what the fix would have to handle.

const E3_DATA: &str = r#"
    ex:VerifiedClaim owl:onProperty ex:verifiedBy ; owl:hasValue ex:AuditBoard .
    ex:Claim rdfs:subClassOf ex:VerifiedClaim .
    ex:claim1 a ex:Claim ; ex:statesThat "emissions fell by 18 percent" .
"#;

const E3_SHAPES: &str = r#"
    ex:ClaimShape a sh:NodeShape ;
        sh:targetClass ex:Claim ;
        sh:property [ sh:path ex:verifiedBy ; sh:minCount 1 ;
                      sh:message "every claim cites who verified it" ] .
"#;

/// The triple the shape was written to require.
fn evidence_triple() -> Spelled {
    (
        "<http://ex.org/claim1>".to_string(),
        "<http://ex.org/verifiedBy>".to_string(),
        "<http://ex.org/AuditBoard>".to_string(),
    )
}

#[test]
fn the_evidence_triple_was_concluded_by_a_named_rule_and_never_asserted() {
    let store = loaded(&format!("{PREFIXES}{E3_DATA}"));
    let dag = Reasoner::derivation_graph(&store, "owl-rl-ext").unwrap();
    let t = evidence_triple();

    assert!(dag.holds(&t), "cls-hv1 should reach this triple: {:?}", dag.instances);
    assert!(
        !dag.is_asserted(&t),
        "the whole finding is that nobody wrote this triple down"
    );
    assert!(
        dag.instances
            .iter()
            .any(|i| i.rule == "cls-hv1" && i.conclusion == t),
        "the rule responsible has to be nameable, or the finding is an anecdote: {:?}",
        dag.instances
    );
}

/// The oracle, stated over the same store before anything is materialised: the
/// assertions VIOLATE the shape. So the pass the next test records is a pass
/// over a graph whose asserted content fails.
#[test]
fn the_asserted_graph_alone_violates_the_evidence_constraint() {
    let store = loaded(&format!("{PREFIXES}{E3_DATA}"));
    let before = report(&store, &format!("{PREFIXES}{E3_SHAPES}"));
    assert_eq!(before["conforms"], serde_json::json!(false), "{before}");
    assert_eq!(before["violation_count"], serde_json::json!(1));

    // A second, independent reading of the same fact. Turtle carries no graph
    // name, so `GraphStore::serialize` drops the inference graph by design;
    // reloading its output is reloading what a person actually wrote.
    let materialised = loaded(&format!("{PREFIXES}{E3_DATA}"));
    Reasoner::run_with_target(&materialised, "owl-rl-ext", true, InferenceTarget::Inferred)
        .unwrap();
    let asserted_only = loaded(&materialised.serialize("turtle").unwrap());
    let oracle = report(&asserted_only, &format!("{PREFIXES}{E3_SHAPES}"));
    assert_eq!(oracle["conforms"], serde_json::json!(false), "{oracle}");
    assert_eq!(oracle["violation_count"], serde_json::json!(1));
}

/// Running the reasoner flips the verdict from false to true over a store whose
/// assertions did not change.
///
/// This test records a DEFECT that is open. When the validator stops reading
/// the inference graph it will fail, and the right response is to change the
/// expectations here and amend decision 0018.
#[test]
fn materialising_an_inference_flips_the_gate_from_violated_to_conforming() {
    let store = loaded(&format!("{PREFIXES}{E3_DATA}"));
    let before = report(&store, &format!("{PREFIXES}{E3_SHAPES}"));
    assert_eq!(before["conforms"], serde_json::json!(false));

    Reasoner::run_with_target(&store, "owl-rl-ext", true, InferenceTarget::Inferred).unwrap();
    assert!(
        store.materialised_inference_count().unwrap() > 0,
        "the fixture depends on the materialisation having happened"
    );

    let after = report(&store, &format!("{PREFIXES}{E3_SHAPES}"));
    assert_eq!(
        after["conforms"],
        serde_json::json!(true),
        "defect record: a validation run read the engine's own conclusions as data: {after}"
    );
    assert_eq!(after["violation_count"], serde_json::json!(0));

    // And nothing in the report says so. The scope block still describes the
    // whole store, and no field distinguishes a satisfied constraint from one
    // satisfied by a derived triple.
    assert_eq!(after["scope"]["selector"], "whole-store");
    assert!(
        after.get("inferred_triples_read").is_none(),
        "if this key ever exists the finding is closed and this test must be rewritten"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BUCKET TWO: STANDARD LIMITATIONS, WHERE THE VALIDATOR IS RIGHT
// ═══════════════════════════════════════════════════════════════════════════

// ── S1. A decorative shape buys a verdict for a vacuous one ────────────────
//
// BUCKET: standard limitation plus a report-sufficiency gap, and NOT a
// validator bug. SHACL says a shape that selects no focus node conforms, full
// stop, so the evaluator is doing exactly what the Recommendation asks. This
// engine goes FURTHER than the Recommendation and answers `conforms: null`
// when nothing at all matched, which is a good defence, and the gap is that
// the escalation is decided from a TOTAL: `nothing_matched` compares
// `focus_nodes_total` against zero, so one decorative shape that selects one
// node restores a `true` verdict for a shapes graph whose load-bearing shape
// selected none.
//
// `tests/shacl_vacuous_target_test.rs::a_partial_match_keeps_its_verdict_and_still_names_the_empty_shape`
// pins that on purpose and this branch does not change it. `conforms` keeps
// SHACL's meaning. What this branch adds is `focus_nodes_by_target`, so the
// per-shape number the evaluator already computed reaches the reader.

const S1_DATA: &str = r#"
    ex:facility1 a ex:Facility ; ex:permitNumber "EPR/AB1234CD" .
    ex:load1 a ex:Consignment ; ex:tonnes "12"^^xsd:decimal .
    ex:load2 a ex:Consignment ; ex:tonnes "7"^^xsd:decimal .
"#;

/// The load-bearing shape targets `ex:HazardousConsignment`, which nothing in
/// the data is. The facility shape is the decoration that carries the verdict.
const S1_SHAPES: &str = r#"
    ex:FacilityShape a sh:NodeShape ;
        sh:targetClass ex:Facility ;
        sh:property [ sh:path ex:permitNumber ; sh:minCount 1 ] .
    ex:ConsignmentShape a sh:NodeShape ;
        sh:targetClass ex:HazardousConsignment ;
        sh:property [ sh:path ex:disposalCertificate ; sh:minCount 1 ] .
"#;

/// The same constraint, targeting the class the data actually uses.
const S1_SHAPES_CORRECTED: &str = r#"
    ex:ConsignmentShape a sh:NodeShape ;
        sh:targetClass ex:Consignment ;
        sh:property [ sh:path ex:disposalCertificate ; sh:minCount 1 ] .
"#;

#[test]
fn a_decorative_shape_buys_a_true_verdict_for_a_vacuous_one() {
    let store = loaded(&format!("{PREFIXES}{S1_DATA}"));

    let gate = report(&store, &format!("{PREFIXES}{S1_SHAPES}"));
    assert_eq!(gate["conforms"], serde_json::json!(true));
    assert_eq!(gate["violation_count"], serde_json::json!(0));
    assert_eq!(gate["focus_nodes"], serde_json::json!(1));

    // The oracle. The rule the vacuous shape was written to enforce is violated
    // by two nodes, and the same evaluator over the same store says so as soon
    // as the target selects them. That is the sense in which "conforms: true"
    // above is a false report about this data: nothing certified the
    // consignments and both of them fail the certificate rule.
    let oracle = report(&store, &format!("{PREFIXES}{S1_SHAPES_CORRECTED}"));
    assert_eq!(oracle["conforms"], serde_json::json!(false));
    assert_eq!(oracle["violation_count"], serde_json::json!(2));
}

/// The defence slice that shipped on this branch. The number was already
/// computed per target and summed away; now it is in the report, so a consumer
/// can see that the shape it cared about checked nothing without having to
/// notice that a normally empty key is non-empty.
#[test]
fn the_report_gives_every_target_its_own_focus_node_count() {
    let store = loaded(&format!("{PREFIXES}{S1_DATA}"));
    let gate = report(&store, &format!("{PREFIXES}{S1_SHAPES}"));

    let rows = gate["focus_nodes_by_target"]
        .as_array()
        .expect("focus_nodes_by_target must be present on every report");
    assert_eq!(rows.len(), 2, "one row per target declaration: {gate}");

    let facility = rows
        .iter()
        .find(|r| r["shape"] == "http://ex.org/FacilityShape")
        .expect("the decorative shape must have a row");
    assert_eq!(facility["focus_nodes"], serde_json::json!(1));
    assert_eq!(facility["target_form"], "class");

    let consignment = rows
        .iter()
        .find(|r| r["shape"] == "http://ex.org/ConsignmentShape")
        .expect("the load-bearing shape must have a row");
    assert_eq!(
        consignment["focus_nodes"],
        serde_json::json!(0),
        "the shape that mattered checked nothing, and the report now says the number"
    );
    assert_eq!(consignment["target"], "http://ex.org/HazardousConsignment");

    // The sum is still the old field, so nothing was redefined under a
    // consumer, and `conforms` is untouched.
    let total: u64 = rows.iter().map(|r| r["focus_nodes"].as_u64().unwrap()).sum();
    assert_eq!(serde_json::json!(total), gate["focus_nodes"]);
    assert_eq!(gate["conforms"], serde_json::json!(true));
}

// ── S2. A graph that entails everything conforms ───────────────────────────
//
// BUCKET: standard limitation. SHACL is a validation language and carries no
// consistency obligation; neither evaluator is wrong here. The falsity is
// logical and is found by a different tool in the same repository.

const S2_DATA: &str = r#"
    ex:ent1 a ex:LegalEntity ; ex:lei "213800XJ1M4RSM3DQ21" .
    ex:ent2 a ex:LegalEntity ; ex:lei "529900T8BM49AURSDO55" .
    ex:ent1 owl:sameAs ex:ent2 .
    ex:ent1 owl:differentFrom ex:ent2 .
"#;

const S2_SHAPES: &str = r#"
    ex:LegalEntityShape a sh:NodeShape ;
        sh:targetClass ex:LegalEntity ;
        sh:property [ sh:path ex:lei ; sh:minCount 1 ; sh:maxCount 1 ] .
"#;

#[test]
fn a_graph_that_entails_everything_still_conforms() {
    let store = loaded(&format!("{PREFIXES}{S2_DATA}"));

    let gate = report(&store, &format!("{PREFIXES}{S2_SHAPES}"));
    assert_eq!(gate["conforms"], serde_json::json!(true));
    assert_eq!(gate["focus_nodes"], serde_json::json!(2));
    assert_eq!(gate["violation_count"], serde_json::json!(0));

    // The oracle. `eq-diff1` is one of the ten clash rules this engine detects
    // and one of the rules `OOCert.RefuteConditions` holds a semantic condition
    // for, so the finding is certifiable rather than an opinion with nothing
    // behind it. An inconsistent graph entails every sentence, including the
    // negation of anything the shapes graph was written to establish, so a
    // conformance verdict over it certifies nothing at all. That is what makes
    // the data false in a checkable sense: the falsity is not a claim about the
    // world, it is a property of the graph that a second tool here computes.
    let run: serde_json::Value =
        serde_json::from_str(&Reasoner::run(&store, "owl-rl", false).unwrap()).unwrap();
    assert_eq!(run["inconsistency"]["found"], serde_json::json!(true), "{run}");
    assert!(
        run["inconsistency"]["by_rule"]["eq-diff1"]
            .as_u64()
            .unwrap_or(0)
            >= 1,
        "{run}"
    );
    assert!(
        run["inconsistency"]["clashes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["rule"] == "eq-diff1" && c["certifiable"] == serde_json::json!(true)),
        "a clash nothing could check would be an engine opinion, not an oracle: {run}"
    );
}

// ── S3. The limit case, and the one where "false" cannot be asserted ───────
//
// BUCKET: standard limitation, and the fixture that motivates the defence
// rather than indicting anything. The claim is asserted flatly. Every
// constraint is satisfied, the graph is consistent, no target is vacuous and no
// literal is ill formed.
//
// NOTHING HERE CAN SAY THIS DATA IS FALSE, and this test does not pretend
// otherwise. What it asserts instead is a measured statement that does not
// need an oracle: the report the gate produces for this store is EQUAL, field
// by field, to the report it produces for the E3 store where the satisfying
// triple was the engine's own conclusion. Two graphs, one written by a person
// and one assembled by a claimant so that a rule would write it, and one
// report. That is the case for binding a constraint to a SOURCE rather than to
// a triple, and it is the strongest thing this file can say without claiming to
// know what is true in the world.

const S3_DATA: &str = r#"
    ex:claim1 a ex:Claim ; ex:statesThat "emissions fell by 18 percent" ;
        ex:verifiedBy ex:AuditBoard .
"#;

#[test]
fn a_flatly_asserted_claim_conforms_and_no_oracle_here_contradicts_it() {
    let store = loaded(&format!("{PREFIXES}{S3_DATA}"));

    let gate = report(&store, &format!("{PREFIXES}{E3_SHAPES}"));
    assert_eq!(gate["conforms"], serde_json::json!(true));
    assert_eq!(gate["focus_nodes"], serde_json::json!(1));

    let dag = Reasoner::derivation_graph(&store, "owl-rl-ext").unwrap();
    let t = evidence_triple();
    assert!(
        dag.is_asserted(&t),
        "the satisfying triple is an assertion here, unlike E3's"
    );
    assert!(
        !dag.instances.iter().any(|i| i.conclusion == t),
        "no rule concluded it, so there is no derivation to inspect and nothing to explain"
    );
    assert!(
        dag.clashes.is_empty(),
        "and the graph is consistent, so the S2 oracle has nothing to say either: {:?}",
        dag.clashes
    );
}

/// The measurement, rather than the rhetoric: one report for two graphs.
#[test]
fn the_report_cannot_tell_an_asserted_claim_from_a_laundered_one() {
    let honest = loaded(&format!("{PREFIXES}{S3_DATA}"));
    let laundered = loaded(&format!("{PREFIXES}{E3_DATA}"));
    Reasoner::run_with_target(&laundered, "owl-rl-ext", true, InferenceTarget::Inferred).unwrap();

    let a = report(&honest, &format!("{PREFIXES}{E3_SHAPES}"));
    let b = report(&laundered, &format!("{PREFIXES}{E3_SHAPES}"));

    assert_eq!(
        a, b,
        "if these ever differ the gate has gained a way to tell them apart, which is the \
         whole point of the defence in decision 0018\nasserted: {a}\nlaundered: {b}"
    );
    assert_eq!(a["conforms"], serde_json::json!(true));
}

// ═══════════════════════════════════════════════════════════════════════════
// NEITHER BUCKET: A CANDIDATE THAT IS A LIMITATION AND IS NOT AN EXPLOIT
// ═══════════════════════════════════════════════════════════════════════════

// Under `owl:sameAs` the two register entries are one entity holding two LEIs,
// which a register with a one-LEI-per-entity rule forbids. SHACL counts value
// nodes by term, so each node holds one and both conform. `lean/Shacl/Spec.lean`
// states `maxCount` over term-distinct value nodes, so the verified evaluator
// agrees, and it is right to: the Recommendation says nothing about
// `owl:sameAs`.
//
// This is NOT filed as a working attack, and the reason is that no oracle in
// this repository reports the merged cardinality either. `eq-rep-s`, `eq-rep-p`
// and `eq-rep-o` are absent from `RULES_EVALUATED`, so `owl:sameAs` propagates
// no triple, and `prp-fp` is absent too, so no functional-property clash is
// looked for. The checkable statement is therefore about COVERAGE and not about
// truth, and that is what this test asserts and where it stops. Calling it an
// exploit would mean claiming a falsity that nothing here can demonstrate.

const NEITHER_DATA: &str = r#"
    ex:ent1 a ex:LegalEntity ; ex:lei "213800XJ1M4RSM3DQ21" .
    ex:ent2 a ex:LegalEntity ; ex:lei "529900T8BM49AURSDO55" .
    ex:ent1 owl:sameAs ex:ent2 .
"#;

#[test]
fn owl_same_as_merges_a_cardinality_that_nothing_here_reports() {
    use open_ontologies::reason::{CLASH_RULES_NOT_DETECTED, RULES_EVALUATED};

    let store = loaded(&format!("{PREFIXES}{NEITHER_DATA}"));
    let gate = report(&store, &format!("{PREFIXES}{S2_SHAPES}"));
    assert_eq!(gate["conforms"], serde_json::json!(true));
    assert_eq!(gate["focus_nodes"], serde_json::json!(2));

    assert!(
        !RULES_EVALUATED.iter().any(|(r, _)| r.starts_with("eq-rep")),
        "if eq-rep is ever evaluated this fixture becomes a flipped verdict rather than a \
         coverage statement, and the section above it has to be rewritten"
    );
    assert!(
        !RULES_EVALUATED.iter().any(|(r, _)| *r == "prp-fp"),
        "same for prp-fp"
    );
    assert!(
        !CLASH_RULES_NOT_DETECTED.iter().any(|(r, _)| *r == "prp-fp"),
        "prp-fp is not even on the list of clash rules this engine knows it does not look for, \
         which is the gap worth recording"
    );

    // And so no tool here contradicts this data, which is why it is filed as a
    // limitation rather than as an attack.
    let run: serde_json::Value =
        serde_json::from_str(&Reasoner::run(&store, "owl-rl-ext", false).unwrap()).unwrap();
    assert!(
        run.get("inconsistency").is_none(),
        "nothing in this repository refutes this graph: {run}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// THE CONTROL
// ═══════════════════════════════════════════════════════════════════════════

/// `"300"^^xsd:byte` is outside the byte range and is the W3C suite's own
/// ill-formed case. Oxigraph does not preserve `xsd:byte`, so the literal
/// arrives as `"300"^^xsd:integer`, `datatype_is_indistinguishable_in_store`
/// routes the constraint to `skipped_constraints`, and the verdict is null.
///
/// A probe the existing defence stops, kept here rather than in a
/// passing-cases file so that this set is a measurement and not a list of
/// successes. It stops it in ONE direction and the test measures the other,
/// because the earlier wording of this test asserted that "no consumer can
/// read it as a pass" and that statement is false about this repository.
#[test]
fn a_datatype_the_store_cannot_preserve_yields_no_verdict_rather_than_a_pass() {
    let store = loaded(&format!(
        "{PREFIXES} ex:i a ex:Reading ; ex:level \"300\"^^xsd:byte .\n"
    ));
    let gate = report(
        &store,
        &format!(
            "{PREFIXES} ex:ReadingShape a sh:NodeShape ; sh:targetClass ex:Reading ; \
             sh:property [ sh:path ex:level ; sh:datatype xsd:byte ] .\n"
        ),
    );

    assert_eq!(
        gate["conforms"],
        serde_json::Value::Null,
        "an undetermined verdict is the honest answer to a constraint that cannot be decided \
         over a term the store rewrote: {gate}"
    );
    assert_eq!(gate["skipped_constraints"].as_array().unwrap().len(), 1);
    assert_eq!(gate["violation_count"], serde_json::json!(0));
    // The report says WHY, so the null is readable rather than merely absent.
    assert!(
        gate["skipped_constraints"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("does not preserve"),
        "{gate}"
    );

    // And here is the other half of the bound, which is the half the earlier
    // wording got wrong. A consumer reading for a PASS is protected, because
    // the verdict is not `true`. A consumer reading for a FAILURE is not.
    // `onto_extend` stops its pipeline on `parsed["conforms"] == false`, and
    // `serde_json::Value::Null` does not equal `Value::Bool(false)`, so this
    // probe walks through that gate exactly as a clean run would.
    //
    // Asserting `!(conforms == false)` here would prove nothing, because the
    // assertion above already pins the verdict to null and makes that
    // comparison a tautology. The claim that can actually be wrong is the one
    // about the SOURCE, so the source is what is read. If the gate is ever
    // widened to treat an undetermined verdict as a stop, this fails and the
    // finding in decision 0018 has to be amended rather than quietly outlived.
    let server = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/server.rs"))
        .expect("src/server.rs is readable from the crate root");
    assert!(
        server.contains(r#"parsed["conforms"] == false"#),
        "the onto_extend pipeline gate no longer reads for `false` alone, so the statement that \
         an undetermined verdict passes through it is stale and decision 0018 must be amended"
    );
}

// ---------------------------------------------------------------------------
// A regression the E2 fix exposed, and which was latent on main before it.
//
// `ill_typed_test` builds a SPARQL fragment `!REGEX(STR(?v), "<pattern>")`, and
// the pattern goes inside a SPARQL string literal. The only escapes legal there
// are \t \b \n \r \f \" \' and \\. Four of the six patterns carry `\.` to mean a
// literal dot, which is not on that list, so embedding one raw makes the whole
// query unparseable and the validator returns Err rather than a verdict.
//
// Nothing caught it because the two datatypes whose regex contains no backslash
// at all, integer and boolean, are the ones every existing fixture reached. The
// node-shape path has spliced this fragment in since it was written, so main
// carries the same defect; giving the property path the same splice is what
// made it reachable from an induced shape and turned it red.
// ---------------------------------------------------------------------------

/// `xsd:decimal`, whose lexical-space regex contains `\.` twice.
const BACKSLASH_DATA: &str = r#"
ex:receipt ex:total "12.50"^^xsd:decimal ; ex:when "2026-01-02T03:04:05"^^xsd:dateTime .
"#;

const BACKSLASH_PROPERTY_SHAPES: &str = r#"
ex:ReceiptShape a sh:NodeShape ;
  sh:targetNode ex:receipt ;
  sh:property [ sh:path ex:total ; sh:datatype xsd:decimal ] ;
  sh:property [ sh:path ex:when  ; sh:datatype xsd:dateTime ] .
"#;

const BACKSLASH_NODE_SHAPES: &str = r#"
ex:TotalShape a sh:NodeShape ;
  sh:targetObjectsOf ex:total ;
  sh:datatype xsd:decimal .
"#;

#[test]
fn a_datatype_whose_regex_contains_a_backslash_still_produces_a_verdict() {
    let store = loaded(&format!("{PREFIXES}{BACKSLASH_DATA}"));

    // The bug was not a wrong answer, it was NO answer: the query failed to
    // parse and `validate` returned Err, so `report` panicked before any
    // assertion about conformance could run. Both paths are checked because
    // the node path carried this latently and the property path is what
    // exposed it.
    let property_path = report(&store, &format!("{PREFIXES}{BACKSLASH_PROPERTY_SHAPES}"));
    let node_path = report(&store, &format!("{PREFIXES}{BACKSLASH_NODE_SHAPES}"));

    // These values are well typed, so the honest verdict is that they conform.
    assert_eq!(
        property_path["conforms"],
        serde_json::json!(true),
        "a well-typed decimal and dateTime conform through sh:property: {property_path}"
    );
    assert_eq!(
        node_path["conforms"],
        serde_json::json!(true),
        "and through a node shape: {node_path}"
    );

    // And the constraint is genuinely being evaluated rather than skipped,
    // which is the failure mode that would make the assertions above vacuous:
    // an ill-typed decimal through the same shape must be caught.
    let ill = loaded(&format!(
        "{PREFIXES}\nex:receipt ex:total \"twelve fifty\"^^xsd:decimal .\n"
    ));
    let caught = report(&ill, &format!("{PREFIXES}{BACKSLASH_PROPERTY_SHAPES}"));
    assert_eq!(
        caught["conforms"],
        serde_json::json!(false),
        "a decimal outside its lexical space is still a violation, so the regex ran \
         rather than being quietly dropped: {caught}"
    );
}
