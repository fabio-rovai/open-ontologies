//! Adding an axiom either changes what the ontology already said, or it does
//! not, and the difference is invisible to every other number a plan reports.
//!
//! The pair that matters is `adding_a_domain_reclassifies_existing_individuals`
//! and `adding_a_disconnected_hierarchy_changes_nothing_over_the_old_names`.
//! They start from the SAME base and differ by two triples, the plan's shape
//! diff cannot tell them apart (both add a class or a triple and remove
//! nothing), and one of them silently retypes existing data.

use open_ontologies::conservativity as cx;
use open_ontologies::graph::GraphStore;
use std::path::PathBuf;
use std::sync::Arc;

const BASE: &str = r#"
@prefix owl:  <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex:   <http://ex.org/> .

ex:Person    a owl:Class .
ex:hasParent a owl:ObjectProperty .
ex:Alice     ex:hasParent ex:Bob .
"#;

const EX: &str = "http://ex.org/";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

fn loaded(ttl: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(ttl, None).unwrap_or_else(|e| panic!("fixture must parse: {e}\n{ttl}"));
    g
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-cx-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn opts(name: &str, mode: cx::ExtensionMode) -> cx::ConservativityOptions {
    cx::ConservativityOptions { mode, out: scratch(name), ..Default::default() }
}

fn check(name: &str, extension: &str, mode: cx::ExtensionMode) -> cx::ConservativityReport {
    cx::conservativity_check(&loaded(BASE), extension, &opts(name, mode))
        .expect("the check must run")
}

fn rendered(r: &cx::ConservativityReport) -> Vec<String> {
    r.new_consequences_over_old_signature
        .iter()
        .map(|c| format!("{} {} {}", c.triple.0, c.triple.1, c.triple.2))
        .collect()
}

// ───────────────────────────────────────────────────────────────────────────
// It changed something
// ───────────────────────────────────────────────────────────────────────────

/// One `rdfs:domain` triple. It adds no class, removes nothing, and retypes
/// every existing subject of `ex:hasParent`. A shape diff sees a single
/// addition; this sees the reclassification.
#[test]
fn adding_a_domain_reclassifies_existing_individuals() {
    let r = check(
        "domain",
        "@prefix ex: <http://ex.org/> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . \
         ex:hasParent rdfs:domain ex:Person .",
        cx::ExtensionMode::Delta,
    );
    assert_eq!(r.conservativity_verdict, cx::NOT_CONSERVATIVE, "{}", r.headline);
    assert_eq!(r.conservative_under_rule_table, Some(false));
    assert_eq!(r.exit_code, 1, "a finding is exit 1, not an error");

    let want = format!("<{EX}Alice> <{RDF_TYPE}> <{EX}Person>");
    assert!(
        rendered(&r).contains(&want),
        "the reclassification must be named, not summarised: {:#?}",
        rendered(&r)
    );

    // The row has to carry the derivation, or the user is being asked to
    // trust a verdict word.
    let row = r
        .new_consequences_over_old_signature
        .iter()
        .find(|c| format!("{} {} {}", c.triple.0, c.triple.1, c.triple.2) == want)
        .unwrap();
    assert_eq!(row.rule.as_deref(), Some("rdfs2"), "the rule that produced it: {row:?}");
    assert!(row.over_old_signature);
    assert!(
        row.premises_the_base_lacked
            .iter()
            .any(|p| p.1.contains("domain")),
        "the premise the base lacked is the domain axiom itself: {:#?}",
        row.premises_the_base_lacked
    );
}

/// A second, structurally different way to change the old vocabulary: a
/// subclass edge between two classes that both already existed.
#[test]
fn adding_a_subclass_edge_between_existing_classes_is_a_finding() {
    let base = format!("{BASE}\n@prefix ex2: <http://ex.org/> . ex2:Agent a <http://www.w3.org/2002/07/owl#Class> .");
    let r = cx::conservativity_check(
        &loaded(&base),
        "@prefix ex: <http://ex.org/> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . \
         ex:Person rdfs:subClassOf ex:Agent .",
        &opts("subclass", cx::ExtensionMode::Delta),
    )
    .unwrap();
    assert_eq!(r.conservativity_verdict, cx::NOT_CONSERVATIVE, "{}", r.headline);
    let want = format!(
        "<{EX}Person> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <{EX}Agent>"
    );
    assert!(rendered(&r).contains(&want), "{:#?}", rendered(&r));
}

// ───────────────────────────────────────────────────────────────────────────
// It changed nothing over the old names
// ───────────────────────────────────────────────────────────────────────────

/// The control. Two triples, both over names the base never used. The extension
/// adds plenty of NEW consequences, which is what adding a class does; none of
/// them is over the old signature, which is the whole question.
#[test]
fn adding_a_disconnected_hierarchy_changes_nothing_over_the_old_names() {
    let r = check(
        "disconnected",
        "@prefix ex: <http://ex.org/> . @prefix owl: <http://www.w3.org/2002/07/owl#> . \
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . \
         ex:Robot a owl:Class . ex:Android rdfs:subClassOf ex:Robot .",
        cx::ExtensionMode::Delta,
    );
    assert_eq!(r.conservativity_verdict, cx::CONSERVATIVE, "{}\n{:#?}", r.headline, rendered(&r));
    assert_eq!(r.conservative_under_rule_table, Some(true));
    assert_eq!(r.new_consequences_over_old_signature_total, 0);
    assert_eq!(r.exit_code, 0);
    // A clean verdict over an extension that derived nothing at all would be
    // vacuous, so the control has to have moved the closure.
    assert!(
        r.new_consequences_total > 0,
        "the extension must add SOME consequence, or 'none over the old signature' says nothing"
    );
    assert_eq!(
        r.signature_added_total, 2,
        "two new names: {:?}",
        r.signature_added
    );
}

// ───────────────────────────────────────────────────────────────────────────
// The honesty of the verdict
// ───────────────────────────────────────────────────────────────────────────

/// The field name is the whole point of the feature. A reader who sees
/// `conservative: true` will believe something nobody can compute.
#[test]
fn the_payload_never_carries_a_bare_conservative_flag() {
    let r = check(
        "wording",
        "@prefix ex: <http://ex.org/> . @prefix owl: <http://www.w3.org/2002/07/owl#> . \
         ex:Robot a owl:Class .",
        cx::ExtensionMode::Delta,
    );
    let json = serde_json::to_string(&r).unwrap();
    assert!(
        !json.contains("\"conservative\""),
        "no field may be called `conservative`, because conservativity in a description logic is \
         not what was computed"
    );
    assert!(json.contains("\"conservative_under_rule_table\""));
    assert!(json.contains("\"conservativity_verdict\""));
    // The fragment has to travel in the payload, not only in the docs, because
    // the docs are not what a dashboard renders.
    for phrase in ["UNDECIDABLE for ALCQIO", "undecidable already for EL", "rule_table"] {
        assert!(json.contains(phrase), "the payload must carry {phrase:?}");
    }
    assert!(json.contains("FINDING, not a failure"));
}

/// A change that REMOVES is not an extension, and the question does not apply
/// to it. Reporting it as conservative would be the dangerous answer, and
/// reporting it as non-conservative would be a false alarm, so it is neither.
#[test]
fn a_replacement_that_drops_a_base_triple_is_undecided_not_conservative() {
    let r = check(
        "removal",
        "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix ex: <http://ex.org/> . \
         ex:Person a owl:Class . ex:hasParent a owl:ObjectProperty .",
        cx::ExtensionMode::Replacement,
    );
    assert_eq!(r.conservativity_verdict, cx::UNDECIDED_NOT_AN_EXTENSION, "{}", r.headline);
    assert_eq!(r.conservative_under_rule_table, None, "undecided is null, never false");
    assert!(!r.is_an_extension);
    assert_eq!(r.base_triples_the_proposal_drops, 1, "{:?}", r.dropped_examples);
    assert_eq!(r.exit_code, 2);
    assert!(r.dropped_examples.iter().any(|d| d.contains("Alice")));
}

/// The same turtle, read the other way round. A delta says nothing about the
/// triples it does not mention, so the mode is asked for rather than guessed.
#[test]
fn the_same_turtle_read_as_a_delta_is_not_a_removal() {
    let ttl = "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix ex: <http://ex.org/> . \
               ex:Person a owl:Class . ex:hasParent a owl:ObjectProperty .";
    let as_delta = check("mode-delta", ttl, cx::ExtensionMode::Delta);
    let as_replacement = check("mode-replacement", ttl, cx::ExtensionMode::Replacement);
    assert!(as_delta.is_an_extension);
    assert_eq!(as_delta.base_triples_the_proposal_drops, 0);
    assert_eq!(as_delta.conservativity_verdict, cx::CONSERVATIVE);
    assert!(!as_replacement.is_an_extension);
    assert_ne!(as_delta.conservativity_verdict, as_replacement.conservativity_verdict);
}

#[test]
fn an_unknown_mode_is_refused_by_name() {
    let e = cx::ExtensionMode::parse("maybe").unwrap_err().to_string();
    assert!(e.contains("delta"), "{e}");
    assert!(e.contains("replacement"), "{e}");
}

// ───────────────────────────────────────────────────────────────────────────
// The engine's own gate, riding along
// ───────────────────────────────────────────────────────────────────────────

/// The base is a subset of the extended graph by construction and the rule
/// table is monotone, so nothing the base derives may be missing from the
/// extension's closure. That gate must be ARMED, not merely empty: an empty
/// violation list under a disarmed gate means "we did not look".
#[test]
fn the_monotonicity_gate_is_armed_and_finds_nothing() {
    let r = check(
        "monotonicity",
        "@prefix ex: <http://ex.org/> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . \
         ex:hasParent rdfs:domain ex:Person .",
        cx::ExtensionMode::Delta,
    );
    assert!(r.engine_soundness_violations.is_empty(), "{:?}", r.engine_soundness_violations);
    assert!(
        r.engine_soundness_gate_skipped.is_none(),
        "the gate must actually run, or the empty list proves nothing: {:?}",
        r.engine_soundness_gate_skipped
    );
}

/// Blank nodes on both sides used to be the reason the gate could not run.
/// Skolemising the base under its OWN prefix before the extension is merged in
/// is what keeps them apart: the same label on both sides must not become the
/// same IRI.
#[test]
fn blank_nodes_in_the_base_do_not_disarm_the_gate() {
    let base = "@prefix owl: <http://www.w3.org/2002/07/owl#> . \
                @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . \
                @prefix ex: <http://ex.org/> . \
                ex:Cheesy rdfs:subClassOf \
                  [ a owl:Restriction ; owl:onProperty ex:hasTopping ; \
                    owl:someValuesFrom ex:Cheese ] .";
    let r = cx::conservativity_check(
        &loaded(base),
        "@prefix owl: <http://www.w3.org/2002/07/owl#> . \
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . \
         @prefix ex: <http://ex.org/> . \
         ex:Deep rdfs:subClassOf \
           [ a owl:Restriction ; owl:onProperty ex:hasBase ; owl:someValuesFrom ex:Thin ] .",
        &opts("blanks", cx::ExtensionMode::Delta),
    )
    .unwrap();
    assert!(
        r.engine_soundness_gate_skipped.is_none(),
        "blank nodes on both sides must not disarm the gate: {:?}",
        r.engine_soundness_gate_skipped
    );
    assert!(r.engine_soundness_violations.is_empty());
    assert_eq!(r.conservativity_verdict, cx::CONSERVATIVE, "{}", r.headline);
}

// ───────────────────────────────────────────────────────────────────────────
// Where the feature actually lives
// ───────────────────────────────────────────────────────────────────────────

/// `onto_plan` is the home. A plan that did not run the check must SAY it did
/// not run it, because a missing block and a clean block read the same to a
/// dashboard.
#[test]
fn a_plan_reports_conservativity_or_reports_that_it_did_not_look() {
    use open_ontologies::plan::Planner;
    use open_ontologies::state::StateDb;

    let dir = scratch("plan");
    let db = StateDb::open(&dir.join("state.db")).expect("state db");
    let graph = loaded(BASE);
    let planner = Planner::new(db, graph);

    let proposal = format!(
        "{BASE}\n@prefix ex2: <http://ex.org/> . \
         @prefix rdfs2: <http://www.w3.org/2000/01/rdf-schema#> . \
         ex2:hasParent rdfs2:domain ex2:Person ."
    );

    let silent: serde_json::Value =
        serde_json::from_str(&planner.plan(&proposal).unwrap()).unwrap();
    assert_eq!(silent["conservativity"]["ran"], false);
    assert!(
        silent["conservativity"]["skipped"].as_str().unwrap().contains("not requested"),
        "a plan that did not look must say so: {}",
        silent["conservativity"]
    );

    let checked: serde_json::Value = serde_json::from_str(
        &planner
            .plan_checked(
                &proposal,
                Some(cx::ConservativityOptions {
                    mode: cx::ExtensionMode::Replacement,
                    out: scratch("plan-cx"),
                    ..Default::default()
                }),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(checked["conservativity"]["ran"], true);
    assert_eq!(
        checked["conservativity"]["conservativity_verdict"],
        cx::NOT_CONSERVATIVE,
        "{}",
        checked["conservativity"]["headline"]
    );
    // The shape diff sees nothing wrong, which is the entire reason the
    // semantic block had to be added.
    assert_eq!(checked["risk_score"], "low");
    assert_eq!(checked["removed_classes"].as_array().unwrap().len(), 0);
    assert_eq!(checked["added_classes"].as_array().unwrap().len(), 0);
}
