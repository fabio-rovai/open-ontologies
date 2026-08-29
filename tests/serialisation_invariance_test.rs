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

#[test]
fn support_check_reads_claims_from_every_graph() {
    // onto_support_check audits provenance: which claims cite no source, plus a
    // verification task per sourced claim. Built on the default-graph select, it
    // reported an empty spotless corpus for any TriG/N-Quads store.
    use open_ontologies::state::StateDb;
    use open_ontologies::support::SupportChecker;
    let claim = "@prefix ex: <http://example.org/> . @prefix prov: <http://www.w3.org/ns/prov#> . \
                 ex:claim1 ex:about ex:Bridge ; prov:wasDerivedFrom ex:src1 .";
    let turtle = Arc::new(GraphStore::new());
    turtle.load_turtle(claim, None).unwrap();
    let trig = Arc::new(GraphStore::new());
    trig.load_content(
        "@prefix ex: <http://example.org/> . @prefix prov: <http://www.w3.org/ns/prov#> . \
         ex:g { ex:claim1 ex:about ex:Bridge ; prov:wasDerivedFrom ex:src1 . }",
        RdfFormat::TriG,
    )
    .unwrap();

    let dt = tempfile::TempDir::new().unwrap();
    let t: serde_json::Value = serde_json::from_str(
        &SupportChecker::new(turtle, StateDb::open(&dt.path().join("s.db")).unwrap())
            .check(None, 100)
            .unwrap(),
    )
    .unwrap();
    let dq = tempfile::TempDir::new().unwrap();
    let q: serde_json::Value = serde_json::from_str(
        &SupportChecker::new(trig, StateDb::open(&dq.path().join("s.db")).unwrap())
            .check(None, 100)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(t["claims_total"].as_u64(), Some(1), "turtle side must see the claim: {t}");
    assert_eq!(t["claims_total"], q["claims_total"], "turtle {t}\ntrig {q}");
    assert_eq!(t["tasks"], q["tasks"], "turtle {t}\ntrig {q}");
}

#[cfg(feature = "embeddings")]
#[test]
fn structural_embeddings_train_on_every_graph() {
    // Poincare structural training reads the class hierarchy; on the default
    // graph alone it trained on nothing for a TriG store, degrading
    // onto_embed(structure)/onto_search/onto_similarity to noise with no error.
    use open_ontologies::structembed::StructuralTrainer;
    let onto = "ex:Animal a owl:Class . ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal . \
                ex:Cat a owl:Class ; rdfs:subClassOf ex:Animal .";
    let prefixes = "@prefix ex: <http://example.org/> . @prefix owl: <http://www.w3.org/2002/07/owl#> . \
                    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .";
    let turtle = Arc::new(GraphStore::new());
    turtle.load_turtle(&format!("{prefixes} {onto}"), None).unwrap();
    let trig = Arc::new(GraphStore::new());
    trig.load_content(&format!("{prefixes} ex:g {{ {onto} }}"), RdfFormat::TriG).unwrap();

    let t = StructuralTrainer::new(8, 5, 0.1).train(&turtle).unwrap();
    let q = StructuralTrainer::new(8, 5, 0.1).train(&trig).unwrap();
    assert_eq!(t.len(), 3, "turtle side must embed all three classes: {}", t.len());
    assert_eq!(t.len(), q.len(), "turtle {} vs trig {}", t.len(), q.len());
}

#[test]
fn align_extracts_target_classes_from_every_graph() {
    // onto_align with target=None aligns a source ontology against the loaded
    // store. Class extraction on the default graph alone saw zero target classes
    // for a TriG store and reported nothing to align.
    use open_ontologies::align::AlignmentEngine;
    use open_ontologies::state::StateDb;
    let target_onto = "ex:Dog a owl:Class ; rdfs:label \"Dog\" .";
    let prefixes = "@prefix ex: <http://example.org/> . @prefix owl: <http://www.w3.org/2002/07/owl#> . \
                    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .";
    let source = format!("{prefixes} ex:Dog2 a owl:Class ; rdfs:label \"Dog\" .");

    let turtle = Arc::new(GraphStore::new());
    turtle.load_turtle(&format!("{prefixes} {target_onto}"), None).unwrap();
    let trig = Arc::new(GraphStore::new());
    trig.load_content(&format!("{prefixes} ex:g {{ {target_onto} }}"), RdfFormat::TriG).unwrap();

    let dt = tempfile::TempDir::new().unwrap();
    let t: serde_json::Value = serde_json::from_str(
        &AlignmentEngine::new(StateDb::open(&dt.path().join("s.db")).unwrap(), turtle)
            .align(&source, None, 0.3, true)
            .unwrap(),
    )
    .unwrap();
    let dq = tempfile::TempDir::new().unwrap();
    let q: serde_json::Value = serde_json::from_str(
        &AlignmentEngine::new(StateDb::open(&dq.path().join("s.db")).unwrap(), trig)
            .align(&source, None, 0.3, true)
            .unwrap(),
    )
    .unwrap();
    assert!(
        t["total_candidates"].as_u64().unwrap_or(0) >= 1,
        "turtle side must find the Dog/Dog2 candidate, or this proves nothing: {t}"
    );
    assert_eq!(t["total_candidates"], q["total_candidates"], "turtle {t}\ntrig {q}");
}

#[test]
fn enforce_reads_design_pattern_data_from_every_graph() {
    // onto_enforce checks design-pattern compliance with internally-authored
    // SELECTs. On the default graph alone it saw no classes in a TriG store and
    // reported a false clean (compliance 1.0, zero violations). The value_partition
    // pack flags a parent whose >=2 children are not pairwise disjoint.
    use open_ontologies::enforce::Enforcer;
    use open_ontologies::state::StateDb;
    let onto = "ex:Animal a owl:Class . \
                ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal . \
                ex:Cat a owl:Class ; rdfs:subClassOf ex:Animal .";
    let prefixes = "@prefix ex: <http://example.org/> . @prefix owl: <http://www.w3.org/2002/07/owl#> . \
                    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .";
    let turtle = Arc::new(GraphStore::new());
    turtle.load_turtle(&format!("{prefixes} {onto}"), None).unwrap();
    let trig = Arc::new(GraphStore::new());
    trig.load_content(&format!("{prefixes} ex:g {{ {onto} }}"), RdfFormat::TriG).unwrap();

    let dt = tempfile::TempDir::new().unwrap();
    let t: serde_json::Value = serde_json::from_str(
        &Enforcer::new(StateDb::open(&dt.path().join("s.db")).unwrap(), turtle)
            .enforce("value_partition").unwrap(),
    ).unwrap();
    let dq = tempfile::TempDir::new().unwrap();
    let q: serde_json::Value = serde_json::from_str(
        &Enforcer::new(StateDb::open(&dq.path().join("s.db")).unwrap(), trig)
            .enforce("value_partition").unwrap(),
    ).unwrap();
    let tv = t["violations"].as_array().unwrap().len();
    assert!(tv >= 1, "turtle side must find the non-disjoint partition, or this proves nothing: {t}");
    assert_eq!(tv, q["violations"].as_array().unwrap().len(), "turtle {t}\ntrig {q}");
    assert_eq!(t["compliance"], q["compliance"], "turtle {t}\ntrig {q}");
}

#[test]
fn plan_blast_radius_reads_dependents_from_every_graph() {
    // onto_plan's blast radius counts how many triples reference an IRI being
    // removed. Computed over the default graph alone, it read zero dependents for
    // a TriG store, so a removal that breaks references looked safe.
    use open_ontologies::plan::Planner;
    use open_ontologies::state::StateDb;
    let onto = "ex:Building a owl:Class . \
                ex:Bridge a owl:Class ; rdfs:subClassOf ex:Building . \
                ex:height a owl:DatatypeProperty ; rdfs:domain ex:Building .";
    let prefixes = "@prefix ex: <http://example.org/> . @prefix owl: <http://www.w3.org/2002/07/owl#> . \
                    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .";
    // Proposed change removes ex:Building (still referenced by Bridge and height).
    let new_turtle = format!("{prefixes} ex:Bridge a owl:Class .");

    let turtle = Arc::new(GraphStore::new());
    turtle.load_turtle(&format!("{prefixes} {onto}"), None).unwrap();
    let trig = Arc::new(GraphStore::new());
    trig.load_content(&format!("{prefixes} ex:g {{ {onto} }}"), RdfFormat::TriG).unwrap();

    let dt = tempfile::TempDir::new().unwrap();
    let t: serde_json::Value = serde_json::from_str(
        &Planner::new(StateDb::open(&dt.path().join("s.db")).unwrap(), turtle)
            .plan(&new_turtle).unwrap(),
    ).unwrap();
    let dq = tempfile::TempDir::new().unwrap();
    let q: serde_json::Value = serde_json::from_str(
        &Planner::new(StateDb::open(&dq.path().join("s.db")).unwrap(), trig)
            .plan(&new_turtle).unwrap(),
    ).unwrap();
    let ta = t["blast_radius"]["triples_affected"].as_u64().unwrap();
    assert!(ta >= 1, "turtle side must count Building's dependents, or this proves nothing: {t}");
    assert_eq!(ta, q["blast_radius"]["triples_affected"].as_u64().unwrap(), "turtle {t}\ntrig {q}");
}
