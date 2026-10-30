//! `sh:datatype` naming one of the twelve XSD integer-derived datatypes or
//! `xsd:dateTimeStamp` (#343).
//!
//! Oxigraph 0.5.9's store dropped these IRIs, and after #335 fixed the store
//! the SPARQL evaluator still read each as the type it derives from, so
//! `DATATYPE("0"^^xsd:nonNegativeInteger)` was `xsd:integer`. A conforming
//! literal and one of the base type answered alike, so the engine refused
//! every constraint naming one of the thirteen: it recorded the constraint in
//! `skipped_constraints` and gave no verdict. Answering anyway had reported a
//! violation against every value that satisfied the shape, nine of them in
//! `jsonld-escaping-conformance`, which is how the defect was found.
//!
//! The oxigraph pin now carries the STR and DATATYPE ports, so `DATATYPE()`
//! reads the literal as written and the refusal is gone. Each test below is one
//! of the refusals this file used to pin, turned round: the same shape over the
//! same data now gets a verdict, and a value of the base type, which the old
//! evaluator could not tell from a conforming one, is now a violation.
//!
//! `DATATYPE()` settles the datatype IRI and not the lexical form, and the
//! evaluator parses every one of the twelve as an unbounded `xsd:integer`, so
//! `"300"^^xsd:byte` is a number to it. The lexical spaces are therefore tested
//! as regexes, and the last tests here walk every bound.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

const PREFIXES: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    @prefix ex: <http://example.org/> .
"#;

fn report(store: &Arc<GraphStore>, shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(store, shapes).unwrap()).unwrap()
}

fn loaded(data: &str) -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{data}"), None).unwrap();
    store
}

/// Two profiles whose `ex:passes` is a conforming `xsd:nonNegativeInteger`.
fn conforming() -> Arc<GraphStore> {
    loaded(
        r#"ex:a a ex:Profile ; ex:passes "0"^^xsd:nonNegativeInteger ; ex:name "extruct" .
           ex:b a ex:Profile ; ex:passes "1"^^xsd:nonNegativeInteger ; ex:name "nodejs" ."#,
    )
}

/// The same two values written as `xsd:integer`: equal by value, a different
/// datatype, and exactly what the 0.5.9 evaluator turned the first store into.
fn base_typed() -> Arc<GraphStore> {
    loaded(
        r#"ex:a a ex:Profile ; ex:passes "0"^^xsd:integer ; ex:name "extruct" .
           ex:b a ex:Profile ; ex:passes "1"^^xsd:integer ; ex:name "nodejs" ."#,
    )
}

/// A verdict was given, nothing was skipped, and it is `conforms` with this
/// many violations.
fn decided(r: &serde_json::Value, conforms: bool, violations: usize, what: &str) {
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(skipped.is_empty(), "{what}: nothing may be skipped any more: {r}");
    assert_eq!(r["conforms"], serde_json::json!(conforms), "{what}: {r}");
    assert_eq!(r["violation_count"], serde_json::json!(violations), "{what}: {r}");
}

#[test]
fn a_derived_datatype_is_decided_on_the_direct_constraint() {
    let shapes = format!(
        "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
            sh:property [ sh:path ex:passes ; sh:datatype xsd:nonNegativeInteger ] ."
    );
    decided(&report(&conforming(), &shapes), true, 0, "conforming values");
    decided(&report(&base_typed(), &shapes), false, 2, "xsd:integer values");
}

/// `sh:datatype` on a node shape, reached through `sh:targetObjectsOf`, which
/// binds the literal itself as the focus node.
#[test]
fn a_derived_datatype_is_decided_on_a_node_shape() {
    let shapes = format!(
        "{PREFIXES} ex:S a sh:NodeShape ; sh:targetObjectsOf ex:passes ;
            sh:datatype xsd:nonNegativeInteger ."
    );
    decided(&report(&conforming(), &shapes), true, 0, "conforming values");
    decided(&report(&base_typed(), &shapes), false, 2, "xsd:integer values");
}

// ── The same question asked by every path that tests a datatype ──────────────
//
// The direct constraint refused, and so did each path that tests a datatype as
// a MEMBER of something else: `sh:or`, `sh:not`, `sh:qualifiedValueShape` and
// a nested `sh:node`. Each now answers, and each answer turns round when the
// values are written as `xsd:integer`.

fn members(body: &str, over_conforming: (bool, usize), over_base: (bool, usize)) {
    let shapes = format!("{PREFIXES}{body}");
    let (c, n) = over_conforming;
    decided(&report(&conforming(), &shapes), c, n, &format!("{body}\nconforming values"));
    let (c, n) = over_base;
    decided(&report(&base_typed(), &shapes), c, n, &format!("{body}\nxsd:integer values"));
}

#[test]
fn an_or_member_naming_a_derived_datatype_is_decided() {
    members(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:passes ;
             sh:or ( [ sh:datatype xsd:nonNegativeInteger ] [ sh:datatype xsd:string ] ) ] .",
        (true, 0),
        (false, 2),
    );
}

/// On the old pin, answering this one was a FALSE CLEAN: every value is an
/// `xsd:nonNegativeInteger` and so violates the negation, but `DATATYPE()` said
/// `xsd:integer`, which the negation allows.
#[test]
fn a_not_member_naming_a_derived_datatype_is_decided() {
    members(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:passes ; sh:not [ sh:datatype xsd:nonNegativeInteger ] ] .",
        (false, 2),
        (true, 0),
    );
}

#[test]
fn a_qualified_shape_naming_a_derived_datatype_is_decided() {
    members(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:passes ;
             sh:qualifiedValueShape [ sh:datatype xsd:nonNegativeInteger ] ; sh:qualifiedMinCount 1 ] .",
        (true, 0),
        (false, 2),
    );
}

/// `sh:node` to a shape whose OWN constraint is the datatype.
#[test]
fn a_node_shape_reference_naming_a_derived_datatype_is_decided() {
    members(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:passes ; sh:node ex:Count ] .
         ex:Count a sh:NodeShape ; sh:datatype xsd:nonNegativeInteger .",
        (true, 0),
        (false, 2),
    );
}

/// The primitive datatypes were never refused, and the four member paths over
/// `xsd:integer` and `xsd:string` still catch a value that matches none of
/// them.
#[test]
fn members_naming_a_primitive_datatype_are_still_evaluated() {
    let cases = [
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:name ; sh:or ( [ sh:datatype xsd:integer ] [ sh:datatype xsd:date ] ) ] .",
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:name ; sh:not [ sh:datatype xsd:string ] ] .",
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:name ;
             sh:qualifiedValueShape [ sh:datatype xsd:integer ] ; sh:qualifiedMinCount 1 ] .",
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:name ; sh:node ex:Int ] .
         ex:Int a sh:NodeShape ; sh:datatype xsd:integer .",
    ];
    for body in cases {
        let r = report(&conforming(), &format!("{PREFIXES}{body}"));
        decided(&r, false, 2, body);
    }
}

// ── The lexical spaces ───────────────────────────────────────────────────────

/// The lexical forms among `values` that `sh:datatype <local>` reports, each
/// value on its own focus node.
fn reported(local: &str, values: &[String]) -> Vec<String> {
    let mut data = String::new();
    for (i, v) in values.iter().enumerate() {
        data.push_str(&format!("ex:x{i} a ex:T ; ex:v \"{v}\"^^xsd:{local} .\n"));
    }
    let shapes = format!(
        "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:T ;
            sh:property [ sh:path ex:v ; sh:datatype xsd:{local} ] ."
    );
    let r = report(&loaded(&data), &shapes);
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(skipped.is_empty(), "xsd:{local}: {r}");
    let suffix = format!("\"^^<{XSD}{local}>");
    let mut out: Vec<String> = r["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let term = v["value"].as_str().unwrap_or_else(|| panic!("no value: {v}"));
            term.strip_prefix('"')
                .and_then(|t| t.strip_suffix(&suffix))
                .unwrap_or_else(|| panic!("{term} is not an xsd:{local} literal"))
                .to_string()
        })
        .collect();
    out.sort();
    out
}

fn sorted(v: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = v.iter().map(|s| s.to_string()).collect();
    v.sort();
    v
}

/// Every bounded type, at and just past each bound, with the forms XSD allows
/// around them: a `+` sign, leading zeros, and `-0`, which is zero and so lies
/// in the unsigned types. The bounds come from Rust's own integer types, so
/// the test does not share a single digit with the regex it checks.
#[test]
fn a_bounded_type_reports_exactly_the_values_outside_its_range() {
    let types: [(&str, i128, i128); 8] = [
        ("byte", i8::MIN.into(), i8::MAX.into()),
        ("short", i16::MIN.into(), i16::MAX.into()),
        ("int", i32::MIN.into(), i32::MAX.into()),
        ("long", i64::MIN.into(), i64::MAX.into()),
        ("unsignedByte", 0, u8::MAX.into()),
        ("unsignedShort", 0, u16::MAX.into()),
        ("unsignedInt", 0, u32::MAX.into()),
        ("unsignedLong", 0, u64::MAX.into()),
    ];
    for (local, min, max) in types {
        let inside = [
            max.to_string(),
            min.to_string(),
            format!("+0{max}"),
            (max / 2).to_string(),
            (max - 1).to_string(),
            "0".to_string(),
            "-0".to_string(),
            "007".to_string(),
        ];
        let outside = [(max + 1).to_string(), (min - 1).to_string(), format!("{max}0"), "c".to_string(), "1.0".to_string()];
        let values: Vec<String> = inside.iter().chain(outside.iter()).cloned().collect();
        let mut want: Vec<String> = outside.to_vec();
        want.sort();
        assert_eq!(reported(local, &values), want, "xsd:{local} over {values:?}");
    }
}

#[test]
fn an_unbounded_type_reports_exactly_the_values_of_the_wrong_sign() {
    let huge = "123456789012345678901234567890";
    let cases: [(&str, Vec<String>, Vec<&str>); 4] = [
        (
            "nonNegativeInteger",
            ["0", "-0", "+5", "007", huge, "-1", "-007", "c"].map(String::from).to_vec(),
            vec!["-1", "-007", "c"],
        ),
        (
            "positiveInteger",
            ["1", "+1", "007", huge, "0", "+000", "-0", "-1"].map(String::from).to_vec(),
            vec!["0", "+000", "-0", "-1"],
        ),
        (
            "nonPositiveInteger",
            ["0", "-0", "+0", "-5", "-007", "1", "+1"].map(String::from).to_vec(),
            vec!["1", "+1"],
        ),
        (
            "negativeInteger",
            ["-1", "-007", "-123456789012345678901234567890", "0", "-0", "1"].map(String::from).to_vec(),
            vec!["0", "-0", "1"],
        ),
    ];
    for (local, values, want) in cases {
        assert_eq!(reported(local, &values), sorted(&want), "xsd:{local} over {values:?}");
    }
}

/// An `xsd:dateTimeStamp` is an `xsd:dateTime` whose timezone is required.
#[test]
fn a_date_time_stamp_needs_its_timezone() {
    let values = [
        "2026-01-01T00:00:00Z",
        "2026-01-01T00:00:00.5+14:00",
        "2026-01-01T00:00:00-05:30",
        "2026-01-01T00:00:00",
        "2026-01-01",
        "c",
    ]
    .map(String::from);
    assert_eq!(
        reported("dateTimeStamp", &values),
        sorted(&["2026-01-01T00:00:00", "2026-01-01", "c"]),
    );
}
