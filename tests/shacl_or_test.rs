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

// ── #333: a datatype test that cannot be a type error ───────────────────────
//
// W3C `core/property/or-datatypes-001` expects three `sh:or` violations on one
// focus node and the engine reported one. Two causes, both in how "value V
// has datatype D" was spliced into SPARQL as `DATATYPE(?v) = <D>`:
//
//   * `DATATYPE()` of an IRI or blank node is a type error. Inside
//     `FILTER(!(a || b))` the error propagates and the FILTER drops the row,
//     so an IRI value was never counted as matching no alternative.
//   * `"none"^^xsd:boolean` answers `DATATYPE() = xsd:boolean` though its
//     lexical form is outside xsd:boolean's lexical space. SHACL 4.1.2 makes
//     an ill-typed literal a non-match of `sh:datatype`.

const OR_DATATYPES_SHAPE: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix : <http://ex.org/> .
:S a sh:NodeShape ; sh:targetClass :T ;
  sh:property [ sh:path :comment ;
    sh:or ( [ sh:datatype xsd:string ] [ sh:datatype rdf:HTML ]
            [ sh:datatype rdf:langString ] [ sh:datatype xsd:boolean ] ) ] .
"#;

fn values_of(r: &serde_json::Value) -> Vec<String> {
    let mut v: Vec<String> = r["violations"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|x| x["value"].as_str().unwrap_or("(none)").to_string())
        .collect();
    v.sort();
    v
}

/// The W3C case in miniature: an IRI, an integer and an ill-typed boolean
/// each match no alternative, and each is its own violation.
#[test]
fn or_over_datatypes_reports_an_iri_and_an_ill_typed_literal() {
    let data = r#"
@prefix : <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:bad a :T ; :comment :SomeIri , 42 , "A string" , "none"^^xsd:boolean , "true"^^xsd:boolean .
"#;
    let r = validate(data, OR_DATATYPES_SHAPE);
    assert_eq!(r["conforms"], false, "{r}");
    assert_eq!(
        values_of(&r),
        vec![
            "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_string(),
            "\"none\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_string(),
            "http://ex.org/SomeIri".to_string(),
        ],
        "{r}"
    );
}

/// The other direction: every alternative's well-typed values still conform.
#[test]
fn or_over_datatypes_still_admits_every_alternative() {
    let data = r#"
@prefix : <http://ex.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:good a :T ; :comment "<div>HTML</div>"^^rdf:HTML , "a language string"@en , "A string" ,
  "false"^^xsd:boolean , "1"^^xsd:boolean .
"#;
    let r = validate(data, OR_DATATYPES_SHAPE);
    assert_eq!(r["violation_count"], 0, "{r}");
    assert_eq!(r["conforms"], true, "{r}");
}

/// Same cause on a plain `sh:datatype` property constraint: the violation
/// filter was `DATATYPE(?val) != <D>`, which is a type error for an IRI, so an
/// IRI value passed `sh:datatype xsd:string`.
#[test]
fn an_iri_value_violates_sh_datatype() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix : <http://ex.org/> .
:S a sh:NodeShape ; sh:targetClass :T ;
  sh:property [ sh:path :label ; sh:datatype xsd:string ] .
"#;
    let data = r#"
@prefix : <http://ex.org/> .
:a a :T ; :label :NotALiteral , "fine" .
"#;
    let r = validate(data, shapes);
    assert_eq!(values_of(&r), vec!["http://ex.org/NotALiteral".to_string()], "{r}");
}

/// Node-level `sh:or` over member shapes: a member whose property carries
/// `sh:datatype` must not be satisfied by an IRI or an ill-typed value.
#[test]
fn node_level_or_member_datatype_is_not_satisfied_by_an_iri_or_ill_typed_value() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix : <http://ex.org/> .
:S a sh:NodeShape ; sh:targetClass :T ;
  sh:or ( [ sh:property [ sh:path :flag ; sh:datatype xsd:boolean ] ]
          [ sh:property [ sh:path :flag ; sh:datatype xsd:integer ] ] ) .
"#;
    let data = r#"
@prefix : <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:iri a :T ; :flag :NotALiteral .
:ill a :T ; :flag "none"^^xsd:boolean .
:ok  a :T ; :flag true .
"#;
    let r = validate(data, shapes);
    let mut foci: Vec<String> = r["violations"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|x| x["focus_node"].as_str().unwrap_or_default().to_string())
        .collect();
    foci.sort();
    assert_eq!(foci, vec!["http://ex.org/ill".to_string(), "http://ex.org/iri".to_string()], "{r}");
}

/// `sh:not [ sh:datatype xsd:boolean ]`: an ill-typed boolean does not
/// conform to the inner shape, so the negation is satisfied.
#[test]
fn an_ill_typed_literal_satisfies_sh_not_datatype() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix : <http://ex.org/> .
:S a sh:NodeShape ; sh:targetClass :T ;
  sh:property [ sh:path :flag ; sh:not [ sh:datatype xsd:boolean ] ] .
"#;
    let data = r#"
@prefix : <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:a a :T ; :flag "none"^^xsd:boolean , true .
"#;
    let r = validate(data, shapes);
    assert_eq!(
        values_of(&r),
        vec!["\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_string()],
        "{r}"
    );
}

/// `sh:qualifiedValueShape [ sh:datatype xsd:boolean ]`: an ill-typed boolean
/// is not a value that conforms, so it does not count towards the minimum.
#[test]
fn an_ill_typed_literal_does_not_count_towards_a_qualified_datatype_shape() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix : <http://ex.org/> .
:S a sh:NodeShape ; sh:targetClass :T ;
  sh:property [ sh:path :flag ;
    sh:qualifiedValueShape [ sh:datatype xsd:boolean ] ; sh:qualifiedMinCount 1 ] .
"#;
    let data = r#"
@prefix : <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:a a :T ; :flag "none"^^xsd:boolean .
"#;
    let r = validate(data, shapes);
    assert_eq!(r["violation_count"], 1, "{r}");
    assert_eq!(r["violations"][0]["constraint"], "qualifiedMinCount", "{r}");
}

/// `sh:node` compiles the referenced shape into one boolean expression and
/// reports `FILTER(!(expr))`. A datatype test in that expression erroring on
/// an IRI dropped the row, so an IRI passed a nested `sh:datatype`, both on
/// the referenced shape itself and on a property shape inside it; an
/// ill-typed literal passed both too.
#[test]
fn sh_node_datatype_rejects_an_iri_and_an_ill_typed_value() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix : <http://ex.org/> .
:S a sh:NodeShape ; sh:targetClass :T ;
  sh:property [ sh:path :flag ; sh:node :Boolean ] ;
  sh:property [ sh:path :child ; sh:node :HasBooleanFlag ] .
:Boolean a sh:NodeShape ; sh:datatype xsd:boolean .
:HasBooleanFlag a sh:NodeShape ; sh:property [ sh:path :flag ; sh:datatype xsd:boolean ] .
"#;
    let data = r#"
@prefix : <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
:iri a :T ; :flag :NotALiteral .
:ill a :T ; :flag "none"^^xsd:boolean .
:ok  a :T ; :flag true ; :child :c1 .
:c1 :flag true .
:childIri a :T ; :child :c2 .
:c2 :flag :NotALiteral .
:childIll a :T ; :child :c3 .
:c3 :flag "none"^^xsd:boolean .
"#;
    let r = validate(data, shapes);
    let mut foci: Vec<String> = r["violations"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|x| x["focus_node"].as_str().unwrap_or_default().to_string())
        .collect();
    foci.sort();
    assert_eq!(
        foci,
        ["childIll", "childIri", "ill", "iri"].map(|f| format!("http://ex.org/{f}")).to_vec(),
        "{r}"
    );
}
