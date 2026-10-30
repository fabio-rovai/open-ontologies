//! The OWL 2 functional-syntax WRITER (issue #322) against its reader.
//!
//! The writer claims to be the inverse of `src/ofn.rs`, so the test of it is
//! the round trip: for an OFN document `D`, `G = ofn::parse(D)`, and
//! `ofn::parse(write_ofn(G))` must be RDFC-1.0-isomorphic to `G`. Two further
//! properties are checked on every case: a second parse of `D`, which hands
//! out different blank-node labels, writes byte-identical text, and writing
//! the re-read graph gives the same text again (the output is a fixpoint).
//!
//! `ore_pool_round_trip` runs the same three checks over the ORE 2015 corpus
//! when `OO_ORE_POOL` points at its `files` directory. The corpus is not in
//! the repository (it is git-ignored under benchmark/reasoner/corpora), so
//! that test is `#[ignore]`d and its measured result is recorded in the
//! CHANGELOG entry for #322.

mod common;

use std::collections::BTreeSet;

use open_ontologies::canon::{canonical_nquads, HashAlgorithm, DEFAULT_CALL_BUDGET};
use open_ontologies::graph::GraphStore;
use open_ontologies::ofn;
use open_ontologies::ofn_writer::{write_ofn, OfnOutput};
use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::model::{GraphName, Quad, Triple};

fn quads(triples: &[Triple]) -> Vec<Quad> {
    triples.iter().map(|t| t.clone().in_graph(GraphName::DefaultGraph)).collect()
}

/// The graph in canonical N-Quads. The in-house RDFC-1.0 is tried first; a
/// graph whose blank-node symmetry exhausts its budget is canonicalised by
/// Oxigraph's RDFC-1.0, which has no budget, and that is said in the result.
fn canonical(triples: &[Triple]) -> (String, &'static str) {
    if let Ok(c) = canonical_nquads(&quads(triples), DEFAULT_CALL_BUDGET, HashAlgorithm::Sha256) {
        return (c, "canon.rs");
    }
    let store = GraphStore::new();
    let nt: String = triples.iter().map(|t| format!("{t} .\n")).collect();
    store.load_ntriples(&nt).unwrap();
    let mut lines: Vec<String> = store
        .canonicalize_blank_nodes()
        .unwrap()
        .all_triples()
        .unwrap()
        .into_iter()
        .map(|(s, p, o)| format!("{s} {p} {o} .\n"))
        .collect();
    lines.sort();
    lines.dedup();
    (lines.concat(), "oxigraph")
}

fn diff(a: &str, b: &str) -> String {
    let a: BTreeSet<&str> = a.lines().collect();
    let b: BTreeSet<&str> = b.lines().collect();
    format!(
        "only in the original:\n  {}\nonly after the round trip:\n  {}",
        a.difference(&b).copied().collect::<Vec<_>>().join("\n  "),
        b.difference(&a).copied().collect::<Vec<_>>().join("\n  ")
    )
}

/// Why `doc` does not round-trip, or `None` when it does. Every failure mode
/// is a message rather than a panic so the corpus run can count them.
fn round_trip_failure(doc: &str) -> Option<String> {
    let prefixes = ofn::declared_prefixes(doc);
    let g = ofn::parse(doc).ok()?;
    let out = match write_ofn(&g, &prefixes) {
        Ok(o) => o,
        Err(e) => return Some(format!("writer refused: {e}")),
    };
    if !out.not_exported.is_empty() || !out.not_round_trippable.is_empty() {
        return Some(format!(
            "not exported: {:?}; not round-trippable: {:?}",
            out.not_exported, out.not_round_trippable
        ));
    }
    let back = match ofn::parse(&out.text) {
        Ok(b) => b,
        Err(e) => return Some(format!("the reader rejects the writer's output: {e}")),
    };
    let (want, _) = canonical(&g);
    let (got, _) = canonical(&back);
    if want != got {
        return Some(format!("not isomorphic\n{}", diff(&want, &got)));
    }
    let again = write_ofn(&ofn::parse(doc).unwrap(), &prefixes).unwrap();
    if again.text != out.text {
        return Some("two runs gave different bytes".into());
    }
    let fixpoint = write_ofn(&back, &ofn::declared_prefixes(&out.text)).unwrap();
    if fixpoint.text != out.text {
        return Some("writing the re-read graph gave different bytes".into());
    }
    None
}

fn assert_round_trips(doc: &str) {
    if let Some(why) = round_trip_failure(doc) {
        let out = write_ofn(&ofn::parse(doc).unwrap(), &ofn::declared_prefixes(doc)).unwrap();
        panic!("{why}\n--- written ---\n{}", out.text);
    }
}

fn document(axioms: &str) -> String {
    format!(
        "Prefix(:=<http://e.org/#>)\nPrefix(swrlb:=<http://www.w3.org/2003/11/swrlb#>)\nOntology(<http://e.org/o>\n{axioms}\n)\n"
    )
}

fn write_turtle(ttl: &str) -> OfnOutput {
    let triples: Vec<Triple> = RdfParser::from_format(RdfFormat::Turtle)
        .for_slice(ttl.as_bytes())
        .map(|q| Triple::from(q.unwrap()))
        .collect();
    write_ofn(&triples, &[("".into(), "http://e.org/#".into())]).unwrap()
}

// --------------------------------------------------------------------------- //
// Round trips
// --------------------------------------------------------------------------- //

#[test]
fn the_all_constructs_fixture_round_trips_exactly_and_deterministically() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ofn/all_constructs.ofn");
    assert_round_trips(&std::fs::read_to_string(path).unwrap());
}

/// The structures the fixture does not reach and the writer has a rule for:
/// an anonymous operand in the middle of an annotated n-ary chain (written
/// pairwise it would be two nodes), nested annotation reifications, two axioms
/// that differ only in their annotations, annotated axioms rooted at a blank
/// node, an anonymous ontology, and every SWRL atom.
#[test]
fn the_structures_the_fixture_does_not_reach_round_trip() {
    for axioms in [
        "EquivalentClasses(Annotation(:n \"x\") :A ObjectSomeValuesFrom(:r :B) ObjectIntersectionOf(:C :D) :E)",
        "EquivalentObjectProperties(:p ObjectInverseOf(:q) :s)",
        "SubClassOf(Annotation(Annotation(Annotation(:n \"3\") :n \"2\") :n \"1\") :A ObjectUnionOf(:B :C))",
        "SubClassOf(Annotation(:n \"one\") :A ObjectSomeValuesFrom(:r :B))\nSubClassOf(Annotation(:n \"two\") :A ObjectSomeValuesFrom(:r :B))",
        "DifferentIndividuals(Annotation(:n \"d\") :a _:b :c)\nNegativeDataPropertyAssertion(Annotation(:n \"x\") :age _:b \"3\"^^xsd:integer)",
        "SameIndividual(:a _:x :b)\nClassAssertion(ObjectOneOf(_:x :c) _:y)\nAnnotationAssertion(:n _:y _:x)",
        "Declaration(DataProperty(:dp))\nSubClassOf(:A DataSomeValuesFrom(:dp :dp2 xsd:integer))\nSubClassOf(:A DataMinCardinality(0 :dp))",
        "Declaration(DataProperty(:age))\nHasKey(Annotation(:n \"k\") ObjectSomeValuesFrom(:r :B) (:r ObjectInverseOf(:s)) (:age))",
        "DLSafeRule(Annotation(:n \"rule\") Body(ClassAtom(:A Variable(:x)) ObjectPropertyAtom(:r Variable(:x) Variable(:y)) DataPropertyAtom(:age Variable(:x) Variable(:v)) BuiltInAtom(swrlb:greaterThan Variable(:v) \"18\"^^xsd:integer) DataRangeAtom(xsd:integer Variable(:v))) Head(SameIndividualAtom(Variable(:x) :i) DifferentIndividualsAtom(Variable(:y) _:z) ClassAtom(ObjectComplementOf(:B) Variable(:y))))",
        "DLSafeRule(Body() Head(BuiltInAtom(swrlb:now)))",
        "Declaration(AnnotationProperty(:n))\nAnnotationAssertion(Annotation(:n \"on an annotation assertion\") rdfs:comment :A \"text\"@en)",
        "Declaration(Class(owl:Thing))\nSubClassOf(:A owl:Nothing)\nClassAssertion(owl:Thing :i)",
        "DisjointUnion(Annotation(:n \"u\") :U ObjectSomeValuesFrom(:r :A) :B)",
        "SubObjectPropertyOf(ObjectPropertyChain(ObjectInverseOf(:r) :s) ObjectInverseOf(:t))",
        "DataPropertyAssertion(:age :i \"multi\nline\")\nAnnotationAssertion(rdfs:label <http://e.org/weird%20iri> \"x\")",
    ] {
        assert_round_trips(&document(axioms));
    }
    // Six anonymous individuals that look alike unless the class each hangs
    // under is part of what labels them (ORE file ore_ont_15184 has this
    // shape): with the wrong labelling the 720 permutations make two runs
    // agree by luck once in 720.
    let alike: String = (0..6)
        .map(|i| format!("SubClassOf(:C{i} ObjectHasValue(:d _:a{i}))\nAnnotationAssertion(:n _:a{i} \"v\")\n"))
        .collect();
    assert_round_trips(&document(&alike));
    assert_round_trips("Ontology(\nSubClassOf(<http://e.org/A> <http://e.org/B>)\n)");
}

#[test]
fn output_is_one_axiom_per_line_with_declared_prefixes_and_declarations_first() {
    let doc = document("SubClassOf(:B :A)\nDeclaration(Class(:A))\nAnnotationAssertion(rdfs:label :A \"a\")");
    let out = write_ofn(&ofn::parse(&doc).unwrap(), &ofn::declared_prefixes(&doc)).unwrap();
    let body: Vec<&str> = out.text.lines().filter(|l| !l.is_empty()).collect();
    assert!(body.contains(&"Prefix(:=<http://e.org/#>)"), "{}", out.text);
    assert!(body.contains(&"Prefix(owl:=<http://www.w3.org/2002/07/owl#>)"), "{}", out.text);
    let start = body.iter().position(|l| *l == "Ontology(:o").or(body.iter().position(|l| *l == "Ontology(<http://e.org/o>"));
    let start = start.unwrap_or_else(|| panic!("no header in\n{}", out.text));
    assert_eq!(
        &body[start + 1..],
        &[
            "Declaration(Class(:A))",
            "AnnotationAssertion(rdfs:label :A \"a\")",
            "SubClassOf(:B :A)",
            ")"
        ],
        "{}",
        out.text
    );
    assert_eq!(out.axioms_written, 3);
    assert_eq!(out.triples_exported, out.triples_read);
}

// --------------------------------------------------------------------------- //
// What does not map back
// --------------------------------------------------------------------------- //

const PLANTED: &str = "@prefix : <http://e.org/#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
<http://e.org/o> a owl:Ontology .
:Good a owl:Class .
:Good rdfs:subClassOf :Better .
# A restriction with no filler: no row of Table 1.
:Planted rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ] .
# A union whose list cell has no rdf:rest.
:Broken owl:equivalentClass [ a owl:Class ; owl:unionOf _:cell ] .
_:cell rdf:first :X .
# RDFS vocabulary with no OWL 2 axiom behind it.
:p a rdf:Property .
";

fn report_of(out: &OfnOutput) -> Vec<(String, usize)> {
    out.not_exported.iter().map(|n| (n.construct.clone(), n.occurrences)).collect()
}

#[test]
fn a_planted_unmappable_structure_is_reported_by_name_and_count_and_not_written() {
    let out = write_turtle(PLANTED);
    let report = report_of(&out);
    for want in [
        ("rdfs:subClassOf", 1),
        ("rdf:type owl:Restriction", 1),
        ("owl:onProperty", 1),
        ("owl:equivalentClass", 1),
        ("rdf:type owl:Class", 1),
        ("owl:unionOf", 1),
        ("rdf:first", 1),
        ("rdf:type rdf:Property", 1),
    ] {
        assert!(
            report.contains(&(want.0.to_string(), want.1)),
            "{want:?} missing from {:#?}",
            out.not_exported
        );
    }
    assert_eq!(report.iter().map(|(_, n)| n).sum::<usize>(), 8, "{:#?}", out.not_exported);
    // Every triple is accounted for: written or reported.
    assert_eq!(out.triples_exported + 8, out.triples_read);
    let why = |c: &str| out.not_exported.iter().find(|n| n.construct == c).unwrap().why.clone();
    assert!(why("owl:onProperty").contains("Table 1"), "{}", why("owl:onProperty"));
    assert!(why("rdf:first").contains("rdf:List"), "{}", why("rdf:first"));

    // And none of it is in the document.
    for absent in [":Planted", ":Broken", ":X", "ObjectUnionOf", "rdf:Property", ":p"] {
        assert!(!out.text.contains(absent), "`{absent}` was written:\n{}", out.text);
    }
    assert!(out.text.contains("SubClassOf(:Good :Better)"), "{}", out.text);
    assert!(out.text.contains("Declaration(Class(:Good))"), "{}", out.text);
    // What was written reads back as exactly the triples counted as exported.
    assert_eq!(ofn::parse(&out.text).unwrap().len(), out.triples_exported);
    let json = out.report_json();
    assert_eq!(json["exports_fewer_triples_than_read"], true);
    assert_eq!(json["constructs_not_exported"].as_array().unwrap().len(), out.not_exported.len());
}

#[test]
fn spellings_other_writers_use_are_reported_because_the_reader_would_change_them() {
    let out = write_turtle(
        "@prefix : <http://e.org/#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
<http://e.org/o> a owl:Ontology .
:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :r ; owl:cardinality 1 ] .
[] a owl:AllDifferent ; owl:distinctMembers ( :a :b :c ) .
[] a owl:AllDisjointClasses ; owl:members ( :C :D ) .
:B owl:equivalentClass [ a owl:Class ; owl:oneOf [ a rdf:List ; rdf:first :a ; rdf:rest rdf:nil ] ] .
",
    );
    let why: BTreeSet<String> = out.not_exported.iter().map(|n| n.why.clone()).collect();
    assert!(why.iter().any(|w| w.contains("canonical xsd:nonNegativeInteger")), "{why:#?}");
    assert!(why.iter().any(|w| w.contains("fewer than 3")), "{why:#?}");
    assert!(why.iter().any(|w| w.contains("malformed rdf:List cell")), "{why:#?}");
    assert!(report_of(&out).iter().any(|(c, _)| c == "owl:distinctMembers"), "{:#?}", out.not_exported);
    assert_eq!(out.axioms_written, 0, "{}", out.text);
}

#[test]
fn a_shared_anonymous_expression_is_written_but_flagged_as_not_round_trippable() {
    let out = write_turtle(
        "@prefix : <http://e.org/#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
<http://e.org/o> a owl:Ontology .
:A rdfs:subClassOf _:r . :B rdfs:subClassOf _:r .
_:r a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :C .
",
    );
    assert!(out.not_exported.is_empty(), "{:#?}", out.not_exported);
    assert_eq!(out.not_round_trippable.len(), 1, "{:#?}", out.not_round_trippable);
    assert_eq!(out.not_round_trippable[0].occurrences, 1);
}

#[test]
fn undeclared_properties_are_written_as_annotations_and_counted() {
    let out = write_turtle(
        "@prefix : <http://e.org/#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
<http://e.org/o> a owl:Ontology .
:p a owl:ObjectProperty .
:a :p :b . :a :q :c . :a :q \"lit\" .
",
    );
    assert!(out.text.contains("ObjectPropertyAssertion(:p :a :b)"), "{}", out.text);
    assert!(out.text.contains("AnnotationAssertion(:q :a :c)"), "{}", out.text);
    assert_eq!(out.undeclared_property_assertions, 2);
}

#[test]
fn a_graph_without_an_ontology_header_says_the_header_was_added() {
    let out = write_turtle("@prefix : <http://e.org/#> .\n:A <http://www.w3.org/2000/01/rdf-schema#subClassOf> :B .\n");
    assert!(out.header_synthesised);
    assert!(out.text.contains("SubClassOf(:A :B)"), "{}", out.text);
}

/// Anonymous individuals are the only blank nodes whose labels are printed,
/// and they are RDFC-1.0 labels, so two Turtle parses (different labels from
/// the parser) give the same bytes.
#[test]
fn anonymous_individual_labels_are_canonical_across_loads() {
    let ttl = "@prefix : <http://e.org/#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
<http://e.org/o> a owl:Ontology .
:n a owl:AnnotationProperty .
_:x a :A ; :n _:y . _:y a :B ; :n \"y\" . _:z :n _:x .
";
    let a = write_turtle(ttl);
    let b = write_turtle(ttl);
    assert_eq!(a.text, b.text);
    assert!(a.text.contains("_:c14n"), "{}", a.text);
    assert!(a.not_exported.is_empty(), "{:#?}", a.not_exported);
}

// --------------------------------------------------------------------------- //
// The CLI and its report
// --------------------------------------------------------------------------- //

#[test]
fn the_cli_convert_command_writes_ofn_and_reports_what_it_left_out() {
    let _gate = common::exec_gate();
    let tmp = tempfile::tempdir().unwrap();
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ofn/all_constructs.ofn");
    let run = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .args(["--no-connect", "--data-dir"])
            .arg(tmp.path())
            .args(args)
            .output()
            .unwrap()
    };
    let stdout = run(&["convert", fixture, "--to", "ofn"]);
    assert!(stdout.status.success(), "{}", String::from_utf8_lossy(&stdout.stderr));
    let text = String::from_utf8(stdout.stdout).unwrap();
    assert!(stdout.stderr.is_empty(), "a complete export printed a report: {}", String::from_utf8_lossy(&stdout.stderr));
    let original = ofn::parse(&std::fs::read_to_string(fixture).unwrap()).unwrap();
    assert_eq!(canonical(&ofn::parse(&text).unwrap()).0, canonical(&original).0);

    let planted = tmp.path().join("planted.ttl");
    std::fs::write(&planted, PLANTED).unwrap();
    let out_path = tmp.path().join("planted.ofn");
    let result = run(&["convert", planted.to_str().unwrap(), "--to", "ofn", "--output", out_path.to_str().unwrap(), "--pretty"]);
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stdout));
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let names: Vec<&str> = report["constructs_not_exported"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["construct"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"rdf:type rdf:Property"), "{report:#}");
    assert!(std::fs::read_to_string(&out_path).unwrap().contains("SubClassOf(:Good :Better)"));

    // Without --output the document is stdout and the report goes to stderr.
    let piped = run(&["convert", planted.to_str().unwrap(), "--to", "ofn"]);
    assert!(ofn::parse(&String::from_utf8(piped.stdout).unwrap()).is_ok());
    let err: serde_json::Value = serde_json::from_slice(&piped.stderr).unwrap();
    assert_eq!(err["exports_fewer_triples_than_read"], true);
}

// --------------------------------------------------------------------------- //
// The ORE 2015 corpus
// --------------------------------------------------------------------------- //

/// `OO_ORE_POOL=<.../pool_sample/files> cargo test --test ofn_writer_test --
/// --ignored ore_pool_round_trip --nocapture`. `OO_ORE_LIMIT` caps the number
/// of files (smallest first) and `OO_ORE_MAX_BYTES` their size.
#[test]
#[ignore = "needs the ORE 2015 corpus, which is not in the repository"]
fn ore_pool_round_trip() {
    let Ok(dir) = std::env::var("OO_ORE_POOL") else {
        panic!("set OO_ORE_POOL to the ORE 2015 pool_sample/files directory");
    };
    let limit: usize = std::env::var("OO_ORE_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    let max_bytes: u64 = std::env::var("OO_ORE_MAX_BYTES").ok().and_then(|v| v.parse().ok()).unwrap_or(u64::MAX);
    let mut files: Vec<(u64, std::path::PathBuf)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .map(|p| (std::fs::metadata(&p).unwrap().len(), p))
        .filter(|(n, _)| *n <= max_bytes)
        .collect();
    files.sort();
    files.truncate(limit);
    let (mut handled, mut exact, mut refused_by_reader) = (0, 0, 0);
    let mut failures: Vec<(String, String)> = Vec::new();
    for (_, path) in &files {
        let doc = std::fs::read_to_string(path).unwrap();
        if ofn::parse(&doc).is_err() {
            refused_by_reader += 1;
            continue;
        }
        handled += 1;
        match round_trip_failure(&doc) {
            None => exact += 1,
            Some(why) => failures.push((path.file_name().unwrap().to_string_lossy().into(), why)),
        }
    }
    println!(
        "ORE round trip: {exact}/{handled} exact; {refused_by_reader} of {} files refused by the reader",
        files.len()
    );
    for (f, why) in &failures {
        let short: String = why.chars().take(600).collect();
        println!("  {f}: {short}");
    }
    assert!(failures.is_empty(), "{} file(s) did not round-trip", failures.len());
}
