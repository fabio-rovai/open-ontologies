//! Does a tool answer the same question the same way when the identical
//! triples arrive in a different serialisation?
//!
//! A tool that asks a question about the loaded store, rather than running a
//! query someone wrote, has no business caring which file format the triples
//! came from. Turtle and N-Triples put everything in the default graph; TriG
//! and N-Quads put it wherever the document says. `GraphStore::sparql_select`
//! leaves the evaluator on its default dataset specification, which is the
//! default graph alone, so any tool built on it silently answers about a
//! fraction of the store the moment a dataset serialisation is loaded.
//!
//! That failure mode does not look like a failure. It looks like a clean
//! report over a store that holds nothing, which is why #108 sat open on two
//! separate tools and neither arrived as a bug report. Each test here loads
//! the same content twice, once as Turtle and once as TriG with every triple
//! inside a named graph, and asserts the two answers agree.
//!
//! **A passing test here is worth only as much as its fixture.** Two tools
//! answering "nothing" agree perfectly, so every comparison needs the Turtle
//! side to have found something first. The first cut of the community test
//! passed while proving nothing, because its fixture was schema triples and
//! `edges()` deliberately excludes those. Where a positive assertion is
//! cheap, it is written before the comparison.
//!
//! **What this file does not cover.** Roughly thirty modules run internally
//! authored SELECTs through `sparql_select`. The four covered here were
//! measured; the rest are unmeasured rather than known-good, and `enforce`,
//! `drift`, `align`, `plan`, `support` and `structembed` are the ones most
//! likely to be wrong in the same way. Add a case here before fixing one, so
//! the fix is measured rather than assumed. The rule for which callers want
//! which dataset is on `GraphStore::sparql_select`.

use open_ontologies::graph::GraphStore;
use oxigraph::io::RdfFormat;
use std::sync::Arc;

/// The same ontology, in the two shapes. `TRIG` wraps `TURTLE`'s statements in
/// a single named graph and adds nothing else, so any difference in a tool's
/// answer is the tool reading the default graph alone.
const TURTLE: &str = r#"
@prefix ex: <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
ex:Building a owl:Class ; rdfs:label "Building" .
ex:Bridge a owl:Class ; rdfs:subClassOf ex:Building .
ex:height a owl:DatatypeProperty ; rdfs:domain ex:Building .
ex:spans a owl:ObjectProperty ; rdfs:domain ex:Bridge ; rdfs:range ex:Building .
"#;

const TRIG: &str = r#"
@prefix ex: <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
ex:schema {
    ex:Building a owl:Class ; rdfs:label "Building" .
    ex:Bridge a owl:Class ; rdfs:subClassOf ex:Building .
    ex:height a owl:DatatypeProperty ; rdfs:domain ex:Building .
    ex:spans a owl:ObjectProperty ; rdfs:domain ex:Bridge ; rdfs:range ex:Building .
}
"#;

fn turtle_store() -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(TURTLE, None).expect("load Turtle");
    g
}

fn trig_store() -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_content(TRIG, RdfFormat::TriG).expect("load TriG");
    g
}

/// Both stores hold the same triples. If this ever fails, every other test in
/// the file is comparing two different things and its verdict means nothing.
#[test]
fn the_two_stores_hold_the_same_triples() {
    assert_eq!(
        turtle_store().triple_count(),
        trig_store().triple_count(),
        "the fixtures must differ only in which graph the triples sit in"
    );
}

#[test]
fn vocab_check_reads_declarations_from_every_graph() {
    // The closed-world check that catches hallucinated terms in generated
    // data. It builds the ontology's declared vocabulary with a bare pattern,
    // so an ontology loaded from TriG declared nothing as far as the check was
    // concerned.
    //
    // Measured rather than assumed, because the failure is not the one it
    // looks like it should be: the check does not flag every term as
    // hallucinated. It notices it has zero declared terms and bails with
    // `conforms: false`, an empty `hallucinated_terms`, and a warning telling
    // the caller to load an ontology first. So it is the loud member of this
    // family, and still wrong in a way that costs more than it looks: the
    // advice in the warning is to do the thing the caller has already done,
    // which sends them looking at their input rather than at the tool.
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:b1 a ex:Building ; ex:height "65" .
    "#;
    let from_turtle =
        open_ontologies::vocab_check::check_data_vocab(&turtle_store(), data, &[]).expect("turtle");
    let from_trig =
        open_ontologies::vocab_check::check_data_vocab(&trig_store(), data, &[]).expect("trig");
    let t: serde_json::Value = serde_json::from_str(&from_turtle).unwrap();
    let q: serde_json::Value = serde_json::from_str(&from_trig).unwrap();
    assert_eq!(
        t, q,
        "the same data against the same ontology must give the same report\nturtle: {t}\ntrig:   {q}"
    );
}

#[test]
fn shacl_validate_reads_instances_from_every_graph() {
    // Already fixed, and pinned here so the fix is measured by the same rule
    // as the rest rather than only by its own test.
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Building ;
            sh:property [ sh:path ex:height ; sh:minCount 1 ] .
    "#;
    let instances = "@prefix ex: <http://example.org/> . ex:b1 a ex:Building .";
    let turtle = turtle_store();
    turtle.load_turtle(instances, None).unwrap();
    let trig = trig_store();
    trig.load_content(
        "@prefix ex: <http://example.org/> . ex:data { ex:b1 a ex:Building . }",
        RdfFormat::TriG,
    )
    .expect("load instances into a named graph");

    let t: serde_json::Value = serde_json::from_str(
        &open_ontologies::shacl::ShaclValidator::validate(&turtle, shapes).unwrap(),
    )
    .unwrap();
    let q: serde_json::Value = serde_json::from_str(
        &open_ontologies::shacl::ShaclValidator::validate(&trig, shapes).unwrap(),
    )
    .unwrap();
    assert_eq!(t["focus_nodes"], q["focus_nodes"], "turtle {t}\ntrig {q}");
    assert_eq!(t["conforms"], q["conforms"], "turtle {t}\ntrig {q}");
    assert_eq!(
        t["violation_count"], q["violation_count"],
        "turtle {t}\ntrig {q}"
    );
}

#[test]
fn community_detection_reads_every_graph() {
    // Clusters the entity graph and reports a skeleton per community. Reading
    // the default graph alone over a TriG store finds no entities and reports
    // no communities, which reads as "this corpus has no structure" rather
    // than as "nothing was looked at".
    //
    // The fixture is instance relations rather than the shared schema one,
    // because `edges()` deliberately excludes rdf:type, rdfs:subClassOf,
    // domain, range and the owl class axioms: it clusters the entity graph,
    // not the schema. A schema-only fixture makes both sides empty and the
    // comparison vacuous, which is how the first cut of this test passed
    // while proving nothing.
    let entities = "ex:b1 ex:spans ex:b2 . ex:b2 ex:spans ex:b3 . ex:b3 ex:spans ex:b1 .";
    let turtle = Arc::new(GraphStore::new());
    turtle
        .load_turtle(
            &format!("@prefix ex: <http://example.org/> . {entities}"),
            None,
        )
        .unwrap();
    let trig = Arc::new(GraphStore::new());
    trig.load_content(
        &format!("@prefix ex: <http://example.org/> . ex:data {{ {entities} }}"),
        RdfFormat::TriG,
    )
    .unwrap();

    let t: serde_json::Value = serde_json::from_str(
        &open_ontologies::communities::Communities::new(turtle)
            .detect(1, 5)
            .expect("turtle"),
    )
    .unwrap();
    let q: serde_json::Value = serde_json::from_str(
        &open_ontologies::communities::Communities::new(trig)
            .detect(1, 5)
            .expect("trig"),
    )
    .unwrap();
    assert!(
        t["communities"].as_array().is_some_and(|a| !a.is_empty()),
        "the Turtle side must find something, or this test proves nothing: {t}"
    );
    assert_eq!(
        t["communities"], q["communities"],
        "turtle: {t}\ntrig:   {q}"
    );
}

#[test]
fn shape_induction_reads_every_graph() {
    // Induces shapes from the instances of a class. Over a TriG store it
    // enumerated an empty lattice, which is indistinguishable from a class
    // that genuinely has no instances.
    let instances = "ex:b1 a ex:Building ; ex:height \"65\" .";
    let turtle = turtle_store();
    turtle
        .load_turtle(
            &format!("@prefix ex: <http://example.org/> . {instances}"),
            None,
        )
        .unwrap();
    let trig = trig_store();
    trig.load_content(
        &format!("@prefix ex: <http://example.org/> . ex:data {{ {instances} }}"),
        RdfFormat::TriG,
    )
    .unwrap();

    let t =
        open_ontologies::shape_combinatorics::enumerate(&turtle, "http://example.org/Building", 3)
            .expect("turtle");
    let q =
        open_ontologies::shape_combinatorics::enumerate(&trig, "http://example.org/Building", 3)
            .expect("trig");
    assert_eq!(
        serde_json::to_value(&t).unwrap(),
        serde_json::to_value(&q).unwrap(),
        "the same instances must induce the same lattice"
    );
}
