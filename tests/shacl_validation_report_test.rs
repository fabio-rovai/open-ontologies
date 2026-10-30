//! `onto_shacl` and `shacl` write a W3C `sh:ValidationReport` graph (#318).
//!
//! Three things were missing. The default path's results had no value node:
//! the `sh:class`, `sh:datatype`, `sh:nodeKind` and `sh:pattern` queries
//! selected the offending value and dropped it, so someone repairing the data
//! had to query the store again for what the validator already knew. There was
//! no RDF form of the report at all, so it could not be loaded into a store or
//! compared with pyshacl's. And there was no decision about what an
//! undetermined run (`conforms: null`) becomes in a vocabulary whose
//! `sh:conforms` is a boolean: the one wrong answer is `true`.
//!
//! The W3C half of the acceptance is `tests/shacl_validation_report_w3c_test.rs`.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use open_ontologies::shacl_report::ReportFormat;
use open_ontologies::temporal::ScopeRequest;
use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Cursor, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

const PREFIXES: &str = "PREFIX sh: <http://www.w3.org/ns/shacl#> \
                        PREFIX oo: <https://open-ontologies.org/shacl#> \
                        PREFIX ex: <http://ex.org/> \
                        PREFIX xsd: <http://www.w3.org/2001/XMLSchema#> ";

const DATA: &str = r#"
@prefix ex: <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:alice a ex:Person ;
    ex:age "forty" ;
    ex:knows ex:rock ;
    ex:home "Leeds" ;
    ex:code "ab-1" .
ex:rock a ex:Stone .
"#;

/// One property shape per component, so each result is attributable.
const FOUR_COMPONENTS: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] ;
    sh:property [ sh:path ex:knows ; sh:class ex:Person ] ;
    sh:property [ sh:path ex:home ; sh:nodeKind sh:IRI ] ;
    sh:property [ sh:path ex:code ; sh:pattern "^[0-9]+$" ] .
"#;

fn store(data: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(data, None).unwrap();
    g
}

fn json_report(data: &str, shapes: &str) -> Value {
    serde_json::from_str(&ShaclValidator::validate(&store(data), shapes).unwrap()).unwrap()
}

fn rdf_report(data: &str, shapes: &str, format: ReportFormat) -> String {
    ShaclValidator::report_scoped(&store(data), shapes, &ScopeRequest::Unscoped)
        .unwrap()
        .render(format)
        .unwrap()
}

fn parsed(text: &str, format: RdfFormat) -> Store {
    let s = Store::new().unwrap();
    for q in RdfParser::from_format(format).for_reader(Cursor::new(text.as_bytes())) {
        s.insert(&q.unwrap()).unwrap();
    }
    s
}

fn ask(s: &Store, pattern: &str) -> bool {
    let q = format!("{PREFIXES} ASK {{ {pattern} }}");
    match SparqlEvaluator::new().parse_query(&q).unwrap().on_store(s).execute().unwrap() {
        QueryResults::Boolean(b) => b,
        _ => unreachable!(),
    }
}

fn value_for(report: &Value, constraint: &str) -> Option<String> {
    report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["constraint"] == constraint)
        .and_then(|v| v["value"].as_str().map(str::to_string))
}

#[test]
fn value_is_reported_for_datatype_class_node_kind_and_pattern() {
    let r = json_report(DATA, FOUR_COMPONENTS);
    assert_eq!(r["violation_count"], 4, "{r}");
    assert_eq!(value_for(&r, "datatype").as_deref(), Some("\"forty\""), "{r}");
    assert_eq!(value_for(&r, "class").as_deref(), Some("http://ex.org/rock"), "{r}");
    assert_eq!(value_for(&r, "nodeKind").as_deref(), Some("\"Leeds\""), "{r}");
    assert_eq!(value_for(&r, "pattern").as_deref(), Some("\"ab-1\""), "{r}");
}

#[test]
fn a_node_shape_result_carries_the_focus_node_as_its_value() {
    // SHACL 2.1.2: the value node of a node shape is the focus node itself.
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
ex:S a sh:NodeShape ; sh:targetNode ex:alice ; sh:nodeKind sh:Literal .
"#;
    let r = json_report(DATA, shapes);
    assert_eq!(value_for(&r, "nodeKind").as_deref(), Some("http://ex.org/alice"), "{r}");
}

#[test]
fn the_json_report_changes_only_by_the_value_key() {
    let r = json_report(DATA, FOUR_COMPONENTS);
    let mut keys: Vec<String> = r["violations"][0].as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "constraint",
            "focus_node",
            "message",
            "node_shape",
            "path",
            "result_path",
            "severity",
            "source_constraint_component",
            "source_shape",
            "value",
        ],
        "the RDF writer's terms must not leak into the JSON: {r}"
    );
}

#[test]
fn the_turtle_report_carries_every_result_predicate() {
    let s = parsed(&rdf_report(DATA, FOUR_COMPONENTS, ReportFormat::Turtle), RdfFormat::Turtle);
    assert!(ask(&s, "?r a sh:ValidationReport ; sh:conforms false"));
    assert!(!ask(&s, "?r oo:undetermined ?u"), "a decided run carries no marker");
    assert!(ask(
        &s,
        r#"?r sh:result ?x . ?x a sh:ValidationResult ;
             sh:focusNode ex:alice ; sh:resultPath ex:age ; sh:value "forty" ;
             sh:sourceShape ?shape ;
             sh:sourceConstraintComponent sh:DatatypeConstraintComponent ;
             sh:resultSeverity sh:Violation ; sh:resultMessage ?m"#
    ));
    let n = parsed(&rdf_report(DATA, FOUR_COMPONENTS, ReportFormat::NTriples), RdfFormat::NTriples);
    assert!(ask(&n, "?x sh:value ex:rock ; sh:sourceConstraintComponent sh:ClassConstraintComponent"));
}

#[test]
fn a_conforming_run_is_sh_conforms_true() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
    sh:property [ sh:path ex:knows ; sh:minCount 1 ] .
"#;
    let s = parsed(&rdf_report(DATA, shapes, ReportFormat::Turtle), RdfFormat::Turtle);
    assert!(ask(&s, "?r a sh:ValidationReport ; sh:conforms true"));
    assert!(!ask(&s, "?r sh:result ?x"));
}

/// A skipped constraint suppresses the verdict while another shape's
/// violation stands. The graph must not say `true`, must say why, and must
/// keep the violation that was found.
#[test]
fn an_undetermined_run_is_written_false_with_the_marker_and_keeps_its_results() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] ;
    sh:property [ sh:path ex:home ; sh:minLength "three" ] .
"#;
    let j = json_report(DATA, shapes);
    assert!(j["conforms"].is_null(), "precondition: {j}");
    let s = parsed(&rdf_report(DATA, shapes, ReportFormat::Turtle), RdfFormat::Turtle);
    assert!(!ask(&s, "?r sh:conforms true"), "undetermined must never read as a pass");
    assert!(ask(&s, "?r sh:conforms false ; oo:undetermined true ; oo:undeterminedReason ?why"));
    assert!(ask(&s, r#"?r sh:result ?x . ?x sh:value "forty""#));
}

#[test]
fn an_undetermined_run_with_no_results_is_still_not_a_pass() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
ex:S a sh:NodeShape ; sh:targetClass ex:Nobody ;
    sh:property [ sh:path ex:age ; sh:minCount 1 ] .
"#;
    assert!(json_report(DATA, shapes)["conforms"].is_null());
    let s = parsed(&rdf_report(DATA, shapes, ReportFormat::NTriples), RdfFormat::NTriples);
    assert!(ask(&s, "?r sh:conforms false ; oo:undetermined true"));
    assert!(!ask(&s, "?r sh:conforms true"));
}

#[test]
fn a_custom_severity_iri_and_a_language_tag_survive_into_the_graph() {
    // The JSON keeps `"Too old"` without its tag; the graph needs the terms.
    // The severity is outside the SHACL namespace, so the JSON carries its
    // full IRI (#334) rather than a local name read back as `sh:MySeverity`.
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix sev: <http://ex.org/sev#> .
ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ;
                  sh:severity sev:MySeverity ; sh:message "Too old"@en ] .
"#;
    let j = json_report(DATA, shapes);
    assert_eq!(j["violations"][0]["severity"], "http://ex.org/sev#MySeverity", "{j}");
    assert_eq!(j["violations"][0]["message"], "Too old", "{j}");
    let s = parsed(&rdf_report(DATA, shapes, ReportFormat::Turtle), RdfFormat::Turtle);
    assert!(ask(
        &s,
        r#"?x sh:resultSeverity <http://ex.org/sev#MySeverity> ; sh:resultMessage "Too old"@en"#
    ));
}

/// #334, W3C `core/misc/severity-002`. The JSON `severity` was `sh:severity`
/// cut after its last `#` on every path, so a custom severity in a hash
/// namespace came out as `"MySeverity"` and every reader, the W3C full-tuple
/// comparison included, had to guess its namespace and guessed `sh:`. (One in
/// a slash namespace survived whole, by accident of where the cut fell.) The
/// three SHACL severities keep their local name, which is what existing
/// readers match on; any other IRI is carried whole. Checked on each path that
/// reads `sh:severity`: a node shape's own constraint, a property shape, a
/// qualified shape and `sh:sparql`.
#[test]
fn a_custom_severity_keeps_its_namespace_in_the_json_on_every_path() {
    let shapes = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix sev: <http://ex.org/sev#> .
ex:NodeLevel a sh:NodeShape ; sh:targetNode ex:alice ;
    sh:nodeKind sh:BlankNode ; sh:severity sev:MySeverity .
ex:PropLevel a sh:NodeShape ; sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ; sh:severity sev:MySeverity ] ;
    sh:property [ sh:path ex:home ; sh:nodeKind sh:IRI ; sh:severity sh:Warning ] ;
    sh:property [ sh:path ex:code ; sh:pattern "^[0-9]+$" ] ;
    sh:property [ sh:path ex:knows ; sh:qualifiedValueShape [ sh:class ex:Person ] ;
                  sh:qualifiedMinCount 1 ; sh:severity sev:MySeverity ] .
ex:SparqlLevel a sh:NodeShape ; sh:targetClass ex:Person ;
    sh:sparql [ sh:severity sev:MySeverity ;
        sh:select "SELECT $this WHERE { $this <http://ex.org/home> ?h }" ] .
"#;
    let j = json_report(DATA, shapes);
    let mut got: Vec<(String, String)> = j["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| (v["constraint"].as_str().unwrap().to_string(), v["severity"].as_str().unwrap().to_string()))
        .collect();
    got.sort();
    let custom = "http://ex.org/sev#MySeverity";
    let mut want: Vec<(String, String)> = [
        ("datatype", custom),
        ("nodeKind", custom),
        ("nodeKind", "Warning"),
        ("pattern", "Violation"),
        ("qualifiedMinCount", custom),
        ("sparql", custom),
    ]
    .iter()
    .map(|(c, s)| (c.to_string(), s.to_string()))
    .collect();
    want.sort();
    assert_eq!(got, want, "{j}");
}

/// `sh:or` asserted on a node shape over member shapes wrote every result as
/// `"Violation"` whatever the shape declared, and the RDF report followed it
/// with `sh:resultSeverity sh:Violation`. The node shape's `sh:severity` is
/// the result's severity (SHACL 3.6.2) as on every other path.
#[test]
fn a_node_level_or_result_carries_the_shapes_declared_severity() {
    for (declared, json, rdf) in [
        ("sh:Warning", "Warning", "sh:Warning"),
        ("sev:MySeverity", "http://ex.org/sev#MySeverity", "<http://ex.org/sev#MySeverity>"),
        ("", "Violation", "sh:Violation"),
    ] {
        let severity = if declared.is_empty() { String::new() } else { format!("sh:severity {declared} ;") };
        let shapes = format!(
            r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
@prefix sev: <http://ex.org/sev#> .
ex:S a sh:NodeShape ; sh:targetClass ex:Person ; {severity}
    sh:or ( [ sh:property [ sh:path ex:email ; sh:minCount 1 ] ]
            [ sh:property [ sh:path ex:phone ; sh:minCount 1 ] ] ) .
"#
        );
        let j = json_report(DATA, &shapes);
        assert_eq!(j["violation_count"], 1, "{declared}: {j}");
        assert_eq!(j["violations"][0]["constraint"], "or", "{declared}: {j}");
        assert_eq!(j["violations"][0]["severity"], json, "{declared}: {j}");
        let s = parsed(&rdf_report(DATA, &shapes, ReportFormat::Turtle), RdfFormat::Turtle);
        assert!(
            ask(&s, &format!("?x sh:sourceConstraintComponent sh:OrConstraintComponent ; sh:resultSeverity {rdf}")),
            "{declared}: the RDF report must carry {rdf}"
        );
    }
}

/// #335, fixed in the store. W3C `core/node/maxExclusive-001` and
/// `minExclusive-001` expect `sh:focusNode 4.0 ; sh:value 4.0`, i.e.
/// `"4.0"^^xsd:decimal`, and the report said `"4"^^xsd:decimal`, a different
/// RDF term. Oxigraph 0.5.9 encoded a literal of a datatype it knew by value
/// at insert and decoded it canonically, in the data store and in the shapes
/// store alike, so "4.0" and "4" loaded as ONE triple and no code here could
/// say which one a result was about. The pinned fork (oxigraph/oxigraph#2010)
/// value-encodes a literal only when decoding gives it back as written, and
/// stores any other as a generic typed literal.
///
/// This test was the pin of the loss (it asserted one triple, `"4"` in
/// `sh:value` and in the focus node); it now asserts the terms as written.
#[test]
fn a_non_canonical_literal_is_kept_as_written_and_reported_as_written() {
    let decimal = |lex: &str| format!("\"{lex}\"^^<http://www.w3.org/2001/XMLSchema#decimal>");

    // The store: two distinct terms are two triples, each written as given.
    let g = Arc::new(GraphStore::new());
    let added = g
        .load_turtle(
            r#"@prefix ex: <http://ex.org/> .
               @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
               ex:a ex:p "4.0"^^xsd:decimal , "4"^^xsd:decimal .
               ex:b ex:p "04"^^xsd:integer , "1"^^xsd:boolean , "1.0E0"^^xsd:double ."#,
            None,
        )
        .unwrap();
    assert_eq!(added, 5, "\"4.0\" and \"4\" are two RDF terms and must be two triples");
    let nt = g.serialize("ntriples").unwrap();
    for written in [
        decimal("4.0"),
        decimal("4"),
        "\"04\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_string(),
        "\"1\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_string(),
        "\"1.0E0\"^^<http://www.w3.org/2001/XMLSchema#double>".to_string(),
    ] {
        assert!(nt.contains(&written), "{written} must come back as written:\n{nt}");
    }

    // sh:value of a data-graph value written "4.0".
    let data = r#"@prefix ex: <http://ex.org/> .
                  @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
                  ex:a a ex:Person ; ex:age "4.0"^^xsd:decimal ."#;
    let datatype = r#"@prefix sh: <http://www.w3.org/ns/shacl#> .
                      @prefix ex: <http://ex.org/> .
                      @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
                      ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
                        sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] ."#;
    let j = json_report(data, datatype);
    assert_eq!(value_for(&j, "datatype"), Some(decimal("4.0")), "{j}");
    let s = parsed(&rdf_report(data, datatype, ReportFormat::Turtle), RdfFormat::Turtle);
    assert!(ask(&s, r#"?x sh:value ?v . FILTER(sameTerm(?v, "4.0"^^xsd:decimal))"#), "the RDF report too");

    // A focus node written "4.0" in the shapes graph, handed in as text.
    let target = r#"@prefix sh: <http://www.w3.org/ns/shacl#> .
                    @prefix ex: <http://ex.org/> .
                    ex:S a sh:NodeShape ; sh:targetNode 4.0 ; sh:maxExclusive 4 ."#;
    let j = json_report("@prefix ex: <http://ex.org/> . ex:x ex:y ex:z .", target);
    assert_eq!(j["violations"][0]["focus_node"], decimal("4.0"), "{j}");
    assert_eq!(j["violations"][0]["value"], decimal("4.0"), "{j}");
}

/// What #335's store fix did NOT reach, and #343 did. On the 0.5.9 pin the
/// SPARQL evaluator turned a literal of a datatype it knew into its value
/// before any function saw it, so inside a query `STR("4.0"^^xsd:decimal)`
/// was `"4"` and `DATATYPE("5"^^xsd:int)` was `xsd:integer`, whether the
/// literal came from the store or the query text. Two consequences followed:
///
/// * a lexical constraint judged the canonical string, so `sh:pattern "\\.0$"`
///   reported a lone `"4.0"^^xsd:decimal`, whose lexical form matches;
/// * `DATATYPE()` could not tell the twelve integer-derived types or
///   `xsd:dateTimeStamp` from their base type, so the SHACL engine refused
///   every constraint naming one.
///
/// The pin now carries oxigraph 1392252f (STR) and the DATATYPE half of
/// oxigraph/oxigraph#2010 on top of 0.5.13, so both functions read the term as
/// written. This test pinned the old behaviour and now pins the fixed one;
/// `tests/datatype_preservation_test.rs` checks the full datatype set.
#[test]
fn inside_a_query_a_literal_is_read_as_written() {
    let data = r#"@prefix ex: <http://ex.org/> .
                  @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
                  ex:a a ex:Person ; ex:age "4.0"^^xsd:decimal ; ex:n "5"^^xsd:int ."#;
    let g = store(data);
    let json = g
        .sparql_select("SELECT (STR(?a) AS ?s) (DATATYPE(?n) AS ?d) WHERE { ?x <http://ex.org/age> ?a ; <http://ex.org/n> ?n }")
        .unwrap();
    let row = &serde_json::from_str::<Value>(&json).unwrap()["results"][0];
    assert_eq!(row["s"], "\"4.0\"", "STR() must give the lexical form as written: {json}");
    assert_eq!(row["d"], "<http://www.w3.org/2001/XMLSchema#int>", "DATATYPE() must give the datatype as written: {json}");

    // The lone "4.0" matches "\.0$", so the pattern constraint holds. On the
    // old pin STR() read "4" and this reported one violation.
    let pattern = r#"@prefix sh: <http://www.w3.org/ns/shacl#> .
                     @prefix ex: <http://ex.org/> .
                     ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
                       sh:property [ sh:path ex:age ; sh:pattern "\\.0$" ] ."#;
    let j = json_report(data, pattern);
    assert_eq!(j["violation_count"], 0, "{j}");
    assert_eq!(j["conforms"], true, "{j}");

    // And the constraint still bites: "4" does not end in ".0".
    let j = json_report(
        r#"@prefix ex: <http://ex.org/> .
           @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
           ex:a a ex:Person ; ex:age "4"^^xsd:decimal ."#,
        pattern,
    );
    assert_eq!(j["violation_count"], 1, "{j}");
    assert_eq!(value_for(&j, "pattern"), Some("\"4\"^^<http://www.w3.org/2001/XMLSchema#decimal>".into()), "{j}");
}

// ─────────────────────────────────────────────────────────────────────────────
// The command line
// ─────────────────────────────────────────────────────────────────────────────

fn run(dir: &Path, args: &[&str]) -> (i32, String) {
    let out = {
        let _gate = common::exec_gate();
        Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .env("OPEN_ONTOLOGIES_STORAGE_MODE", "persistent")
            .arg("--data-dir")
            .arg(dir.join("data"))
            .arg("--no-connect")
            .args(args)
            .output()
            .expect("binary should run")
    };
    (
        out.status.code().unwrap_or(-1),
        format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)),
    )
}

fn write(dir: &Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p.to_str().unwrap().to_string()
}

fn loaded_store() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("data")).unwrap();
    std::fs::write(d.path().join("data").join("config.toml"), "[storage]\nmode = 'persistent'\n")
        .unwrap();
    let data = write(d.path(), "data.ttl", DATA);
    let (code, out) = run(d.path(), &["load", &data]);
    assert_eq!(code, 0, "load failed: {out}");
    d
}

/// The exit code is the verdict's (#329) in every format. `sh:conforms false`
/// is written for both a failing and an undetermined run, so a code read off
/// the graph could not tell 1 from 2.
#[test]
fn cli_turtle_report_keeps_the_verdict_exit_codes() {
    let d = loaded_store();
    let failing = write(d.path(), "f.ttl", FOUR_COMPONENTS);
    let passing = write(
        d.path(),
        "p.ttl",
        "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://ex.org/> .
         ex:S a sh:NodeShape ; sh:targetNode ex:alice ; sh:property [ sh:path ex:knows ; sh:minCount 1 ] .",
    );
    let undetermined = write(
        d.path(),
        "u.ttl",
        "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://ex.org/> .
         ex:S a sh:NodeShape ; sh:targetClass ex:Person ; sh:property [ sh:path ex:home ; sh:minLength \"three\" ] .",
    );
    for (shapes, format, rdf, code, conforms) in [
        (&failing, "turtle", RdfFormat::Turtle, 1, "false"),
        (&passing, "turtle", RdfFormat::Turtle, 0, "true"),
        (&undetermined, "turtle", RdfFormat::Turtle, 2, "false"),
        (&failing, "ntriples", RdfFormat::NTriples, 1, "false"),
        (&undetermined, "ntriples", RdfFormat::NTriples, 2, "false"),
    ] {
        let (got, out) = run(d.path(), &["shacl", shapes, "--report-format", format]);
        assert_eq!(got, code, "{format} over {shapes}: {out}");
        let s = parsed(&out, rdf);
        assert!(ask(&s, &format!("?r a sh:ValidationReport ; sh:conforms {conforms}")), "{out}");
        assert_eq!(ask(&s, "?r oo:undetermined true"), code == 2, "{out}");
    }
}

#[test]
fn cli_refuses_an_unknown_format_and_the_verified_combination() {
    let d = loaded_store();
    let shapes = write(d.path(), "f.ttl", FOUR_COMPONENTS);
    let (code, out) = run(d.path(), &["shacl", &shapes, "--report-format", "tutle"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("report_format must be"), "{out}");
    let (code, out) = run(d.path(), &["shacl", &shapes, "--verified", "--report-format", "turtle"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("cannot be combined"), "{out}");
}

#[tokio::test]
async fn a_batch_refuses_an_rdf_format_rather_than_answering_in_json() {
    let dir = tempfile::tempdir().unwrap();
    let shapes = write(dir.path(), "s.ttl", FOUR_COMPONENTS);
    let db = open_ontologies::state::StateDb::open(&dir.path().join("state.db")).unwrap();
    let runner = open_ontologies::batch::BatchRunner::new(db, store(DATA), false);
    let payload =
        json!([{"command": "shacl", "args": [shapes, "--report-format", "turtle"]}]).to_string();
    let (results, _) = runner.run_collect(&payload, false).await;
    let err = results[0]["result"]["error"].as_str().unwrap_or_default();
    assert!(err.contains("not available in a batch"), "{}", results[0]);
}

// ─────────────────────────────────────────────────────────────────────────────
// MCP
// ─────────────────────────────────────────────────────────────────────────────

struct McpChild {
    child: Child,
    input: ChildStdin,
    lines: Receiver<String>,
}

impl McpChild {
    fn start(state: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .args(["--no-connect", "--data-dir"])
            .arg(state)
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        Self { child, input, lines }
    }

    fn send(&mut self, message: Value) {
        writeln!(self.input, "{message}").unwrap();
        self.input.flush().unwrap();
    }

    fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let line = self.lines.recv_timeout(Duration::from_secs(20)).expect("MCP response");
            let response: Value = serde_json::from_str(&line).unwrap();
            if response["id"] == id {
                assert!(response.get("error").is_none(), "{response}");
                return response["result"].clone();
            }
        }
    }

    fn tool_text(&mut self, id: u64, name: &str, arguments: Value) -> String {
        let r = self.request(id, "tools/call", json!({"name": name, "arguments": arguments}));
        r["content"][0]["text"].as_str().unwrap().to_string()
    }
}

impl Drop for McpChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn mcp_onto_shacl_exposes_report_format() {
    let dir = tempfile::tempdir().unwrap();
    let mut mcp = McpChild::start(&dir.path().join("state"));
    mcp.request(
        1,
        "initialize",
        json!({"protocolVersion": "2025-03-26", "capabilities": {},
               "clientInfo": {"name": "shacl-report-test", "version": "1"}}),
    );
    mcp.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    let tools = mcp.request(2, "tools/list", json!({}));
    let shacl = tools["tools"].as_array().unwrap().iter().find(|t| t["name"] == "onto_shacl").unwrap();
    assert!(shacl["inputSchema"]["properties"]["report_format"].is_object(), "{shacl}");

    mcp.tool_text(3, "onto_load", json!({"turtle": DATA}));
    let ttl = mcp.tool_text(
        4,
        "onto_shacl",
        json!({"shapes": FOUR_COMPONENTS, "inline": true, "report_format": "turtle"}),
    );
    let s = parsed(&ttl, RdfFormat::Turtle);
    assert!(ask(&s, r#"?r sh:conforms false ; sh:result ?x . ?x sh:value "forty""#), "{ttl}");

    let json_text = mcp.tool_text(5, "onto_shacl", json!({"shapes": FOUR_COMPONENTS, "inline": true}));
    let j: Value = serde_json::from_str(&json_text).unwrap();
    assert_eq!(j["conforms"], false, "the default is still JSON: {j}");

    let bad = mcp.tool_text(
        6,
        "onto_shacl",
        json!({"shapes": FOUR_COMPONENTS, "inline": true, "report_format": "rdfxml"}),
    );
    assert!(bad.contains("report_format must be"), "{bad}");
}
