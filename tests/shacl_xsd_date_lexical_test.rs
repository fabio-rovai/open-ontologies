//! **The XSD calendar lexical spaces, on both SHACL evaluation paths.**
//!
//! Engine defect E1 in `docs/decisions/0018-conformance-is-not-truth.md`:
//! `"2026-02-31"^^xsd:date` satisfied `sh:datatype xsd:date`. SHACL 4.1.2 makes
//! a literal whose lexical form is outside its datatype's lexical space a
//! violation, and XSD 1.1 section 3.3.9 puts the day-of-month constraint IN the
//! lexical space of `xsd:date`, so the day that does not exist had to be
//! reported and was not. The lexical test was a regex that counted digits.
//!
//! # Why the fix is not a cleverer regex
//!
//! The month lengths ARE regex work and are in the regex. The rest is not.
//!
//! 29 February needs the proleptic Gregorian leap rule, which is a divisibility
//! test over a year of unbounded length; a regex can encode it, as an automaton
//! over digit suffixes that nobody can read and nobody can review, and the
//! failure mode of getting it wrong is reporting a violation on a VALID date,
//! which is the direction decision 0018 named as the reason to measure the W3C
//! suite before landing anything here. `xsd:dateTime` settles it: the old regex
//! accepted `99:99:99` and a `+99:99` timezone, so closing the day alone would
//! have left two more fields open in the same helper.
//!
//! So the regex became an UNDER-approximation that vouches only for the forms
//! it is certain of, and `xsd_temporal_lex` parses the rest. The regex is still
//! doing work: it keeps the rows a large store hands to the parser down to the
//! ill-formed ones plus 29 February.
//!
//! # What this file measures
//!
//! One fixture table, run through BOTH evaluation paths. E2 in the same
//! decision record is what happens when the node-shape path and the
//! `sh:property` path are given one constraint and produce two answers, so
//! every fixture here is asserted twice and a fix that reached one path would
//! fail this file.
//!
//! The table carries both directions on purpose. A test file holding only
//! rejections would pass with a lexical test that rejects everything, and that
//! is the failure this fix was most at risk of.
//!
//! The same fixtures are run against the verified Lean evaluator in
//! `tests/shacl_verified_tool_test.rs`, which is where the agreement between
//! the two evaluators is measured rather than asserted. This file needs no
//! external binary and runs everywhere.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

const PREFIXES: &str = r#"
@prefix ex: <http://ex.org/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
"#;

/// `(lexical form, is it OUTSIDE the lexical space of xsd:date)`.
///
/// Shared with `tests/shacl_verified_tool_test.rs`, which runs the same list
/// through the Lean evaluator. Keep the two in step: the agreement measurement
/// is only worth taking over the same fixtures.
pub const DATE_FIXTURES: &[(&str, bool)] = &[
    // Outside it, by calendar. This is E1.
    ("2026-02-31", true),
    ("2026-13-01", true),
    ("2026-00-01", true),
    ("2026-01-00", true),
    ("2026-04-31", true),
    ("2026-06-31", true),
    ("2026-09-31", true),
    ("2026-11-31", true),
    ("2023-02-29", true),
    // The century rule, both ways. 1900 is divisible by 100 and not by 400.
    ("1900-02-29", true),
    ("2100-02-29", true),
    // Outside it, by spelling. These were already reported before the fix.
    ("31/02/2026", true),
    ("2026-1-01", true),
    ("026-01-01", true),
    // A year longer than four digits may not carry a leading zero.
    ("02026-01-01", true),
    // The timezone, which the old regex did not range-check at all.
    ("2026-01-01+15:00", true),
    ("2026-01-01+14:30", true),
    ("2026-01-01Q", true),
    // INSIDE it. Every one of these must conform, and this half of the table is
    // the one that catches a lexical test tightened too far.
    ("2026-02-28", false),
    ("2024-02-29", false),
    ("2000-02-29", false),
    ("1600-02-29", false),
    ("2026-01-31", false),
    ("2026-04-30", false),
    ("2026-12-31", false),
    // Five digits, which XSD allows and the OLD regex reported a violation on.
    ("12026-01-01", false),
    ("2026-01-01Z", false),
    ("2026-01-01+14:00", false),
    ("2026-01-01-05:00", false),
];

/// The same, for `xsd:dateTime`, which carries the whole date portion and three
/// more fields the old regex never ranged.
pub const DATE_TIME_FIXTURES: &[(&str, bool)] = &[
    ("2026-02-31T00:00:00", true),
    ("1900-02-29T00:00:00", true),
    ("2026-01-01T25:00:00", true),
    ("2026-01-01T00:60:00", true),
    ("2026-01-01T00:00:60", true),
    // A fraction separator with no fraction after it.
    ("2026-01-01T00:00:00.", true),
    // A date is not a dateTime.
    ("2026-01-01", true),
    ("2026-01-01T00:00:00", false),
    ("2026-01-01T23:59:59", false),
    ("2024-02-29T12:00:00.5Z", false),
    ("2026-01-01T00:00:00+14:00", false),
];

/// Lexical forms `lean/Shacl/Term.lean` will not read, and this engine will not
/// read either.
///
/// A negative year, and the year `0000`, are well formed in XSD 1.1. The Lean
/// declines them because every day-number convention it relies on agrees only
/// on non-negative years, and it says `notJudged` rather than guessing. This
/// engine mirrors that, which means it reports no violation over them, which is
/// exactly what the old regex did, so nothing about these forms changed. They
/// are listed because a silent hole and a declared one are different things.
pub const NOT_JUDGED: &[&str] = &["-0044-03-15", "-0044-02-30", "0000-01-01", "0000-02-30"];

fn verdict(lex: &str, datatype: &str, shapes: &str) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            &format!("{PREFIXES}ex:subject a ex:C ; ex:v \"{lex}\"^^{datatype} ."),
            None,
        )
        .unwrap_or_else(|e| panic!("{lex} has to reach the store before any verdict means anything: {e}"));
    // The literal has to survive the store, or the verdict below is a
    // measurement of Oxigraph and not of this validator. Oxigraph DOES rewrite
    // some of these: see `the_store_rewrites_the_twenty_four_hour_forms`.
    let stored = store
        .sparql_select_union("SELECT ?o WHERE { ?s <http://ex.org/v> ?o }")
        .unwrap();
    assert!(
        stored.contains(lex),
        "the store rewrote {lex}, so nothing below is about the validator: {stored}"
    );
    let json = ShaclValidator::validate(&store, &format!("{PREFIXES}{shapes}")).unwrap();
    serde_json::from_str(&json).unwrap()
}

fn property_shape(datatype: &str) -> String {
    format!(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
             sh:property [ sh:path ex:v ; sh:datatype {datatype} ] ."
    )
}

/// `sh:targetObjectsOf` binds the LITERAL as the focus node, so this is the
/// node-shape path over the same value.
fn node_shape(datatype: &str) -> String {
    format!("ex:S a sh:NodeShape ; sh:targetObjectsOf ex:v ; sh:datatype {datatype} .")
}

fn check_table(table: &[(&str, bool)], datatype: &str) {
    for (lex, ill_formed) in table {
        for (path, shapes) in [
            ("sh:property", property_shape(datatype)),
            ("node shape", node_shape(datatype)),
        ] {
            let report = verdict(lex, datatype, &shapes);
            assert_eq!(
                report["conforms"],
                serde_json::json!(!ill_formed),
                "{lex} is {} the lexical space of {datatype} and the {path} path said otherwise: {report}",
                if *ill_formed { "outside" } else { "inside" }
            );
            assert_eq!(
                report["violation_count"],
                serde_json::json!(u64::from(*ill_formed)),
                "{lex} on the {path} path: {report}"
            );
            if *ill_formed {
                let v = &report["violations"][0];
                assert_eq!(v["constraint"], "datatype", "{report}");
                assert_eq!(
                    v["source_constraint_component"],
                    "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                    "a lexical-space violation is a sh:datatype violation and not something else: {report}"
                );
            }
        }
    }
}

#[test]
fn every_xsd_date_fixture_gets_one_answer_on_both_evaluation_paths() {
    check_table(DATE_FIXTURES, "xsd:date");
}

/// `xsd:dateTime` was fixed at the same time and not left behind. It carries the
/// same date portion, so leaving it would have meant `"2026-02-31"^^xsd:date`
/// reported and `"2026-02-31T00:00:00"^^xsd:dateTime` not, which is E2's shape
/// of defect wearing a different datatype.
#[test]
fn every_xsd_date_time_fixture_gets_one_answer_on_both_evaluation_paths() {
    check_table(DATE_TIME_FIXTURES, "xsd:dateTime");
}

/// The leap rule is the Gregorian one, century exception and all.
///
/// Called out of the table because it is the single fact a hand-written
/// approximation gets wrong. A rule of "divisible by four" would pass every
/// other fixture in this file and would report `1900-02-29` as valid and
/// nothing as invalid in 2100.
#[test]
fn the_leap_rule_is_gregorian_so_1900_is_not_a_leap_year_and_2000_is() {
    let shapes = property_shape("xsd:date");
    for (year, leap) in [(1600, true), (1900, false), (2000, true), (2024, true), (2100, false)] {
        let report = verdict(&format!("{year}-02-29"), "xsd:date", &shapes);
        assert_eq!(
            report["conforms"],
            serde_json::json!(leap),
            "{year} {} a leap year, so {year}-02-29 {} the lexical space: {report}",
            if leap { "is" } else { "is not" },
            if leap { "is in" } else { "is outside" }
        );
    }
}

/// A year this engine declines to read is reported as nothing, not as a
/// violation.
///
/// This is the direction that would make the fix worse than the defect. The
/// Lean evaluator declines the whole validation run over these forms rather
/// than answering; this engine reports no violation, which is what it did
/// before the fix as well. The difference is recorded in decision 0018 as a
/// residual and narrower gap rather than dressed up as agreement.
#[test]
fn a_year_this_engine_declines_to_read_is_not_reported_as_a_violation() {
    for lex in NOT_JUDGED {
        for shapes in [property_shape("xsd:date"), node_shape("xsd:date")] {
            let report = verdict(lex, "xsd:date", &shapes);
            assert_eq!(
                report["conforms"],
                serde_json::json!(true),
                "{lex} is well formed XSD 1.1 and must not be reported: {report}"
            );
        }
    }
}

/// The datatypes that are NOT calendar datatypes still answer from the regex
/// alone, and the pre-filter did not disturb them.
#[test]
fn the_non_calendar_lexical_tests_are_unchanged() {
    for (lex, datatype, ill) in [
        ("aldi", "xsd:integer", true),
        ("5", "xsd:integer", false),
        ("1.5", "xsd:decimal", false),
        ("1.2.3", "xsd:decimal", true),
        ("NaN", "xsd:double", false),
        ("maybe", "xsd:boolean", true),
        ("true", "xsd:boolean", false),
    ] {
        let report = verdict(lex, datatype, &property_shape(datatype));
        assert_eq!(
            report["conforms"],
            serde_json::json!(!ill),
            "{lex}^^{datatype}: {report}"
        );
    }
}

/// The two forms this file cannot measure, and the reason, recorded rather than
/// left as a hole in the table.
///
/// XSD allows `24:00:00` and it denotes midnight starting the next day.
/// Oxigraph canonicalises it on the way in, so `2026-01-01T24:00:00` is stored
/// as `2026-01-02T00:00:00` and `2026-01-01T24:00:01`, which is ill formed
/// because the 24 hour admits no other field, is stored as the well-formed
/// `2026-01-02T00:00:01`. No verdict over this store is a verdict about the
/// form that was written, in either direction, so both are out of the table
/// above. `xsd_temporal_lex` implements the rule anyway, because the store is
/// not the only caller it could ever have, and `oo-shacl` reads N-Triples off
/// the same store and sees the same rewrite.
#[test]
fn the_store_rewrites_the_twenty_four_hour_forms() {
    for (written, stored_as) in [
        ("2026-01-01T24:00:00", "2026-01-02T00:00:00"),
        ("2026-01-01T24:00:01", "2026-01-02T00:00:01"),
    ] {
        let store = Arc::new(GraphStore::new());
        store
            .load_turtle(
                &format!("{PREFIXES}ex:subject a ex:C ; ex:v \"{written}\"^^xsd:dateTime ."),
                None,
            )
            .unwrap();
        let json = store
            .sparql_select_union("SELECT ?o WHERE { ?s <http://ex.org/v> ?o }")
            .unwrap();
        assert!(
            json.contains(stored_as) && !json.contains(written),
            "{written} was expected to arrive as {stored_as}: {json}"
        );
    }
}
