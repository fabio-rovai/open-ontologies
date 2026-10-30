//! **The XSD calendar lexical spaces, on both SHACL evaluation paths.**
//!
//! Engine defect E1 in `docs/decisions/0018-conformance-is-not-truth.md`:
//! `"2026-02-31"^^xsd:date` satisfied `sh:datatype xsd:date`. SHACL 4.1.2 makes
//! a literal whose lexical form is outside its datatype's lexical space a
//! violation, and XSD 1.1 section 3.3.9 puts the day-of-month constraint IN the
//! lexical space of `xsd:date`, so the day that does not exist had to be
//! reported and was not. The lexical test was a regex that counted digits.
//!
//! # Why the calendar is in the regex
//!
//! The month lengths are regex work, and so, it turns out, is the leap year.
//! Divisibility by 4, by 100 and by 400 depends only on a year's last four
//! digits, so 29 February of a leap year is a finite pattern over those digits
//! however long the year is. `CALENDAR_DATE` in `src/shacl.rs` spells it out
//! and says why it is exact.
//!
//! An earlier version of this fix kept the regex as an under-approximating
//! pre-filter and judged the day in Rust after the query. That works on the two
//! direct `sh:datatype` paths and nowhere else. `datatype_conforms` is spliced
//! at eight sites, six of them positive (`sh:or` members, `sh:not`,
//! `sh:qualifiedValueShape` and nested `sh:node`), and no Rust pass follows
//! those, so a valid 29 February would have failed every one of them. The
//! calendar therefore lives in the one expression every site splices, and the
//! Rust pass is gone. `every_site_that_asks_the_datatype_question_gets_the_calendar_answer`
//! below is the gate on that.
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
    // The form has to reach the expression as written, or the verdict below
    // is a measurement of Oxigraph and not of this validator. The expression
    // judges `STR()` of the value INSIDE the query, which is not always the
    // term the store hands back: see
    // `the_store_keeps_the_twenty_four_hour_forms_and_the_query_engine_rolls_them_over`.
    assert_eq!(
        as_queried(&store),
        std::collections::BTreeSet::from([lex.to_string()]),
        "the query engine does not see {lex} as written, so nothing below is about the validator"
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

/// The datatypes that are NOT calendar datatypes still answer from their own
/// regex, and the calendar did not disturb them.
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

/// The lexical forms the SPARQL engine presents for `ex:v`, which is what the
/// calendar expression judges: `STR()` of the value inside the query.
fn as_queried(store: &Arc<GraphStore>) -> std::collections::BTreeSet<String> {
    let json: serde_json::Value = serde_json::from_str(
        &store
            .sparql_select_union("SELECT DISTINCT (STR(?o) AS ?lex) WHERE { ?s <http://ex.org/v> ?o }")
            .unwrap(),
    )
    .unwrap();
    let mut out = std::collections::BTreeSet::new();
    collect_strings(&json, &mut out);
    out.remove("lex");
    out
}

/// The 24-hour forms, which the store keeps and the query engine does not.
///
/// XSD allows `24:00:00` and it denotes midnight starting the next day, so the
/// 24 hour admits no other non-zero field. Before the oxigraph pin moved
/// (#335) the store canonicalised these forms on the way in, so no verdict was
/// about the written form and they were left out of the tables above. The
/// store now keeps them as written. The SPARQL evaluator does not: `spareval`
/// reads an `xsd:dateTime` it can parse as a VALUE before any function sees
/// it, and its parser accepts a 24 hour with non-zero minutes, seconds or
/// fraction and rolls it over, so `STR()` of `2026-01-01T24:00:01` is
/// `2026-01-02T00:00:01`. Every site judges what `STR()` gives it, so
/// `24:00:00` conforms, which is right, and `24:00:01` and `24:30:00` conform
/// too, which is wrong: written that way they are outside the lexical space. A
/// 24 hour the parser refuses, `24:59:59`, stays as written and is reported.
///
/// `oo-shacl` reads the written form off the store and reports `24:00:01`, so
/// this is the calendar form on which the two evaluators still differ;
/// `tests/shacl_verified_tool_test.rs` measures that side and decision 0018
/// records it. It is pinned as an expectation, so that a query engine that
/// stops rolling the form over fails here and the record is amended, which is
/// what happened to E1's own test.
#[test]
fn the_store_keeps_the_twenty_four_hour_forms_and_the_query_engine_rolls_them_over() {
    for (written, queried, conforms_here) in [
        ("2026-01-01T24:00:00", "2026-01-02T00:00:00", true),
        ("2026-01-01T24:00:00.5", "2026-01-02T00:00:00.5", true),
        ("2026-01-01T24:00:01", "2026-01-02T00:00:01", true),
        ("2026-01-01T24:30:00", "2026-01-02T00:30:00", true),
        ("2026-01-01T24:59:59", "2026-01-01T24:59:59", false),
    ] {
        let store = Arc::new(GraphStore::new());
        store
            .load_turtle(
                &format!("{PREFIXES}ex:subject a ex:C ; ex:v \"{written}\"^^xsd:dateTime ."),
                None,
            )
            .unwrap();
        let mut stored = std::collections::BTreeSet::new();
        collect_strings(
            &serde_json::from_str(
                &store
                    .sparql_select_union("SELECT ?o WHERE { ?s <http://ex.org/v> ?o }")
                    .unwrap(),
            )
            .unwrap(),
            &mut stored,
        );
        assert!(stored.contains(written), "the store no longer keeps {written} as written: {stored:?}");
        assert_eq!(
            as_queried(&store),
            std::collections::BTreeSet::from([queried.to_string()]),
            "{written} was expected to reach the expression as {queried}"
        );
        let json = ShaclValidator::validate(
            &store,
            &format!("{PREFIXES}{}", property_shape("xsd:dateTime")),
        )
        .unwrap();
        let report: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            report["conforms"],
            serde_json::json!(conforms_here),
            "{written}, queried as {queried}: {report}"
        );
    }
}

// ---------------------------------------------------------------------------
// Every site that splices `datatype_conforms`, not only the two direct paths.
// ---------------------------------------------------------------------------

/// Whether a site reports the values that are OUTSIDE the lexical space, which
/// is every site but one, or the values INSIDE it, which is `sh:not`.
#[derive(Clone, Copy, PartialEq)]
enum Reports {
    IllFormed,
    WellFormed,
}

/// The eight places the engine asks "does this value have datatype D", each
/// written as the smallest shapes graph that reaches it. The two direct
/// `sh:datatype` paths negate the expression; the other six splice it
/// positively, and those six are the ones a Rust pass after the query never
/// reached. Every shape here selects `ex:C`, whose instances carry the value on
/// `ex:v` and point at an inner node carrying the same value, except the node
/// shape, which targets the values themselves.
fn sites() -> Vec<(&'static str, String, Reports)> {
    vec![
        (
            "sh:property sh:datatype",
            "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
                 sh:property [ sh:path ex:v ; sh:datatype xsd:date ] ."
                .to_string(),
            Reports::IllFormed,
        ),
        (
            "node shape sh:datatype",
            "ex:S a sh:NodeShape ; sh:targetObjectsOf ex:v ; sh:datatype xsd:date .".to_string(),
            Reports::IllFormed,
        ),
        (
            "sh:or member",
            "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
                 sh:property [ sh:path ex:v ;
                     sh:or ( [ sh:datatype xsd:date ] [ sh:datatype xsd:dateTime ] ) ] ."
                .to_string(),
            Reports::IllFormed,
        ),
        (
            "sh:not",
            "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
                 sh:property [ sh:path ex:v ; sh:not [ sh:datatype xsd:date ] ] ."
                .to_string(),
            Reports::WellFormed,
        ),
        (
            "sh:qualifiedValueShape",
            "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
                 sh:property [ sh:path ex:v ;
                     sh:qualifiedValueShape [ sh:datatype xsd:date ] ; sh:qualifiedMinCount 1 ] ."
                .to_string(),
            Reports::IllFormed,
        ),
        (
            "node-level sh:or over member shapes",
            "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
                 sh:or (
                     [ sh:property [ sh:path ex:v ; sh:datatype xsd:date ] ]
                     [ sh:property [ sh:path ex:absent ; sh:minCount 1 ] ]
                 ) ."
                .to_string(),
            Reports::IllFormed,
        ),
        (
            "nested sh:node, datatype on the shape",
            "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
                 sh:property [ sh:path ex:v ; sh:node ex:DateShape ] .
             ex:DateShape a sh:NodeShape ; sh:datatype xsd:date ."
                .to_string(),
            Reports::IllFormed,
        ),
        (
            "nested sh:node, datatype on its property",
            "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
                 sh:property [ sh:path ex:inner ; sh:node ex:InnerShape ] .
             ex:InnerShape a sh:NodeShape ;
                 sh:property [ sh:path ex:v ; sh:datatype xsd:date ] ."
                .to_string(),
            Reports::IllFormed,
        ),
    ]
}

/// One store over `forms`: subject `ex:f{i}` carries form `i` on `ex:v` and
/// points at `ex:g{i}`, which carries it too.
fn site_store(forms: &[String]) -> Arc<GraphStore> {
    let mut data = String::from(PREFIXES);
    for (i, lex) in forms.iter().enumerate() {
        data.push_str(&format!(
            "ex:f{i} a ex:C ; ex:v \"{lex}\"^^xsd:date ; ex:inner ex:g{i} .\n\
             ex:g{i} ex:v \"{lex}\"^^xsd:date .\n"
        ));
    }
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&data, None).expect("the site fixtures load");
    // Every form has to survive the store, or a verdict below is about
    // Oxigraph's rewrite and not about the expression.
    let stored: serde_json::Value = serde_json::from_str(
        &store
            .sparql_select_union("SELECT DISTINCT (STR(?o) AS ?lex) WHERE { ?s <http://ex.org/v> ?o }")
            .unwrap(),
    )
    .unwrap();
    let mut lexical = std::collections::BTreeSet::new();
    collect_strings(&stored, &mut lexical);
    for lex in forms {
        assert!(
            lexical.contains(lex.as_str()),
            "the store rewrote {lex}, so a verdict over it is not about the validator"
        );
    }
    store
}

/// Every string in a SPARQL JSON result, with a literal's quotes taken off.
fn collect_strings(v: &serde_json::Value, out: &mut std::collections::BTreeSet<String>) {
    match v {
        serde_json::Value::String(s) => {
            out.insert(match s.strip_prefix('"') {
                Some(rest) => rest.split('"').next().unwrap_or_default().to_string(),
                None => s.clone(),
            });
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| collect_strings(x, out)),
        serde_json::Value::Object(o) => o.values().for_each(|x| collect_strings(x, out)),
        _ => {}
    }
}

/// The forms one site reported, read back from the focus node: `ex:f{i}` for
/// the shapes over `ex:C`, and the literal itself for the node shape.
fn reported_forms(
    site: &str,
    shapes: &str,
    store: &Arc<GraphStore>,
    forms: &[String],
) -> std::collections::BTreeSet<String> {
    let json = ShaclValidator::validate(store, &format!("{PREFIXES}{shapes}"))
        .unwrap_or_else(|e| panic!("{site}: the validator returned an error: {e}"));
    let report: serde_json::Value = serde_json::from_str(&json).unwrap();
    // A site that skipped its constraint would come back `conforms: null` and
    // report nothing, which would read here as "every value accepted".
    assert!(
        report["conforms"].is_boolean(),
        "{site}: no verdict, so the site did not evaluate the datatype: {report}"
    );
    report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let focus = v["focus_node"].as_str().unwrap_or_default();
            match focus.strip_prefix("http://ex.org/f") {
                Some(i) => forms[i.parse::<usize>().unwrap()].clone(),
                None => focus.split('"').nth(1).unwrap_or(focus).to_string(),
            }
        })
        .collect()
}

fn check_sites(table: &[(String, bool)]) {
    let forms: Vec<String> = table.iter().map(|(f, _)| f.clone()).collect();
    let store = site_store(&forms);
    for (site, shapes, reports) in sites() {
        let got = reported_forms(site, &shapes, &store, &forms);
        let want: std::collections::BTreeSet<String> = table
            .iter()
            .filter(|(_, ill)| *ill == (reports == Reports::IllFormed))
            .map(|(f, _)| f.clone())
            .collect();
        assert_eq!(
            got,
            want,
            "{site} disagreed with the calendar.\nreported, should not be: {:?}\nshould be reported, was not: {:?}",
            got.difference(&want).collect::<Vec<_>>(),
            want.difference(&got).collect::<Vec<_>>()
        );
    }
}

/// **Option 2 of PR #348, as a gate.** The same four dates at all eight sites:
/// a valid 29 February in a leap year and in a leap century is accepted
/// wherever the datatype is asked about, and a day that does not exist, or a
/// 29 February in a century that is not a leap year, is not a date anywhere.
/// Under `sh:not` that means the impossible date CONFORMS, because it is not a
/// date, and the real one is reported.
///
/// With the calendar judged in Rust after the query, which is what this branch
/// did before it was merged with #333, the six positive sites rejected
/// `2024-02-29` and `2000-02-29`: the regex they spliced could not vouch for
/// any 29 February and nothing followed it.
#[test]
fn every_site_that_asks_the_datatype_question_gets_the_calendar_answer() {
    check_sites(&[
        ("2024-02-29".to_string(), false),
        ("2000-02-29".to_string(), false),
        ("2026-02-31".to_string(), true),
        ("1900-02-29".to_string(), true),
    ]);
}

/// The same eight sites over every day of every month, 00 to 13 and 00 to 32,
/// in six years chosen for the leap rule: a common year, a leap year, a leap
/// century, two centuries that are not, and a five-digit leap year. The answer
/// is `chrono`'s, not this engine's: a form is in the lexical space exactly
/// when `chrono` can build that date. 2,772 forms, and 22,176 site verdicts.
#[test]
fn every_site_agrees_with_chrono_on_every_day_of_six_boundary_years() {
    let mut table = Vec::new();
    for year in [1900, 2000, 2023, 2024, 2100, 12024] {
        for month in 0u32..=13 {
            for day in 0u32..=32 {
                let exists = chrono::NaiveDate::from_ymd_opt(year, month, day).is_some();
                table.push((format!("{year:04}-{month:02}-{day:02}"), !exists));
            }
        }
    }
    assert_eq!(table.len(), 2_772);
    check_sites(&table);
}
