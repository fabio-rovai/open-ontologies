//! `sh:or` on a property shape: each value must satisfy at least one alternative.
//!
//! Before this, `sh:or` reached `skipped` and suppressed the verdict, so a shapes
//! graph using it could never return `conforms: true`. That mattered beyond the
//! feature gap: the NAPH heritage-aerial standard states its date-precision policy
//! (ADR-0009) as three `sh:datatype` alternatives — day, month or year — because
//! real archive holdings include year-only frames. Its own README and tutorials
//! document `conforms: true` as the expected output, while every run over the
//! shipped samples returned `conforms: null` with that one constraint unevaluated.
//! The single most domain-specific rule in the standard was the one not checked.
//!
//! Only the leaf form is evaluated: a list whose members each carry exactly one of
//! `sh:datatype`, `sh:class` or `sh:hasValue`. A member using anything else sends
//! the whole disjunction to `skipped`, because evaluating only the alternatives
//! that happened to be understood is not the disjunction that was written, and
//! would report a violation for a value the shape permits.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

fn validate(data: &str, shapes: &str) -> serde_json::Value {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(data, None).expect("data should parse");
    let out = ShaclValidator::validate(&g, shapes).expect("validation should run");
    serde_json::from_str(&out).expect("report should be JSON")
}

const DATE_SHAPE: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix : <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:PhotoShape a sh:NodeShape ; sh:targetClass :Photo ;
  sh:property [ sh:path :capturedOn ;
    sh:or ( [ sh:datatype xsd:date ] [ sh:datatype xsd:gYearMonth ] [ sh:datatype xsd:gYear ] ) ] .
"#;

/// The ADR-0009 case: all three precisions are admitted, and nothing is skipped.
#[test]
fn all_three_date_precisions_conform() {
    let data = r#"
@prefix : <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:day   a :Photo ; :capturedOn "1944-06-06"^^xsd:date .
:month a :Photo ; :capturedOn "1944-06"^^xsd:gYearMonth .
:year  a :Photo ; :capturedOn "1944"^^xsd:gYear .
"#;
    let r = validate(data, DATE_SHAPE);
    assert_eq!(r["violation_count"], 0, "report: {r}");
    assert_eq!(
        r["conforms"], true,
        "sh:or must now yield a verdict, not null: {r}"
    );
    let skipped_empty = r["skipped_constraints"]
        .as_array()
        .map(|a| a.is_empty())
        .unwrap_or(true); // the key is omitted entirely when nothing was skipped
    assert!(skipped_empty, "nothing should be skipped: {r}");
}

/// A value matching no alternative is a violation.
#[test]
fn value_matching_no_alternative_violates() {
    let data = r#"
@prefix : <http://ex.org/> .
:freetext a :Photo ; :capturedOn "summer 1944" .
"#;
    let r = validate(data, DATE_SHAPE);
    assert_eq!(r["violation_count"], 1, "report: {r}");
    assert_eq!(r["violations"][0]["constraint"], "or");
    assert_eq!(r["conforms"], false);
}

/// The bug this test exists for. A property shape is a blank node, and a
/// blank-node label written into a SPARQL query is a fresh variable rather than
/// a reference to that node. Looking `sh:or` up that way matched EVERY property
/// shape in the file, so a shape with no `sh:or` at all inherited another
/// shape's alternatives and reported violations on unrelated paths.
#[test]
fn or_does_not_leak_to_other_property_shapes() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix : <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:S a sh:NodeShape ; sh:targetClass :Photo ;
  sh:property [ sh:path :capturedOn ;
    sh:or ( [ sh:datatype xsd:date ] [ sh:datatype xsd:gYear ] ) ] ;
  sh:property [ sh:path :title ; sh:minCount 1 ] .
"#;
    let data = r#"
@prefix : <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:p a :Photo ; :capturedOn "1944"^^xsd:gYear ; :title "a plain string title" .
"#;
    let r = validate(data, shapes);
    assert_eq!(
        r["violation_count"], 0,
        ":title has no sh:or and must not be judged against :capturedOn's alternatives: {r}"
    );
}

/// A member form that is not implemented must skip the whole disjunction and
/// suppress the verdict, never silently narrow it.
#[test]
fn unsupported_member_form_skips_rather_than_narrowing() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix : <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:S a sh:NodeShape ; sh:targetClass :Photo ;
  sh:property [ sh:path :capturedOn ;
    sh:or ( [ sh:datatype xsd:date ] [ sh:minLength 4 ] ) ] .
"#;
    let data = r#"
@prefix : <http://ex.org/> .
:p a :Photo ; :capturedOn "1944" .
"#;
    let r = validate(data, shapes);
    let skipped = r["skipped_constraints"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        skipped.iter().any(|s| s["constraint"] == "sh:or"),
        "an unsupported member must send sh:or to skipped: {r}"
    );
    assert!(
        r["conforms"].is_null(),
        "a skipped constraint must suppress the verdict: {r}"
    );
    assert_eq!(
        r["violation_count"], 0,
        "a skipped disjunction must not report violations: {r}"
    );
}

/// `sh:in`: the value must be one of an enumerated list of terms.
#[test]
fn sh_in_enumerates_permitted_values() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix : <http://ex.org/> .
:S a sh:NodeShape ; sh:targetClass :T ;
  sh:property [ sh:path :status ; sh:in ( "ISSUED" "LAPSED" ) ] .
"#;
    let good = r#"
@prefix : <http://ex.org/> .
:a a :T ; :status "ISSUED" .
:b a :T ; :status "LAPSED" .
"#;
    let r = validate(good, shapes);
    assert_eq!(r["violation_count"], 0, "permitted values must pass: {r}");
    assert_eq!(r["conforms"], true, "{r}");

    let bad = r#"
@prefix : <http://ex.org/> .
:c a :T ; :status "RETIRED" .
"#;
    let r = validate(bad, shapes);
    assert_eq!(r["violation_count"], 1, "{r}");
    assert_eq!(r["violations"][0]["constraint"], "in");
}

/// `sh:nodeKind`: an identifier field typed as a literal where an IRI is required
/// is one of the commonest register defects, so this must be caught rather than skipped.
#[test]
fn sh_node_kind_separates_iris_from_literals() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix : <http://ex.org/> .
:S a sh:NodeShape ; sh:targetClass :T ;
  sh:property [ sh:path :ref ; sh:nodeKind sh:IRI ] .
"#;
    let r = validate(
        "@prefix : <http://ex.org/> .\n:a a :T ; :ref <http://ex.org/target> .\n",
        shapes,
    );
    assert_eq!(r["violation_count"], 0, "an IRI must pass: {r}");

    let r = validate(
        "@prefix : <http://ex.org/> .\n:b a :T ; :ref \"http://ex.org/target\" .\n",
        shapes,
    );
    assert_eq!(r["violation_count"], 1, "a literal must fail: {r}");
    assert_eq!(r["violations"][0]["constraint"], "nodeKind");
}

/// An unrecognised node kind must reach `skipped`, never pass silently.
#[test]
fn unrecognised_node_kind_is_skipped_not_passed() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix : <http://ex.org/> .
:S a sh:NodeShape ; sh:targetClass :T ;
  sh:property [ sh:path :ref ; sh:nodeKind :NotARealKind ] .
"#;
    let r = validate("@prefix : <http://ex.org/> .\n:a a :T ; :ref \"x\" .\n", shapes);
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(
        skipped.iter().any(|s| s["constraint"] == "sh:nodeKind"),
        "an unrecognised node kind must be reported: {r}"
    );
    assert!(r["conforms"].is_null(), "verdict must be suppressed: {r}");
}
