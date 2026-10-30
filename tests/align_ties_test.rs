//! The 1-to-1 selection must never resolve a tie silently.
//!
//! Stable matching keeps the best target for each source and the best source
//! for each target. When two targets score the same for one source, the
//! winner is whichever IRI sorts first, which is a fact about spelling and not
//! about the vocabularies, and the loser used to vanish from the report. A
//! reviewer then saw one confident row and no hint that a second, equally good
//! reading existed. Ties (confidences within one unit of the printed
//! precision) are now named on the surviving row under `tied_with`, the row is
//! flagged `ambiguous`, and an ambiguous row goes to review rather than being
//! written into the graph, because the engine did not choose it.

use open_ontologies::align::AlignmentEngine;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use serde_json::Value;
use std::sync::Arc;

fn engine(graph: Arc<GraphStore>) -> AlignmentEngine {
    let dir = tempfile::tempdir().unwrap();
    let db = StateDb::open(&dir.path().join("state.db")).unwrap();
    std::mem::forget(dir);
    AlignmentEngine::new(db, graph)
}

fn parse(s: String) -> Value {
    serde_json::from_str(&s).unwrap()
}

const ONE_SOURCE: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix s: <http://ties-src.example/> .
    s:redistribution a skos:Concept ; skos:prefLabel "Redistribution"@en .
    s:landfill a skos:Concept ; skos:prefLabel "Landfill"@en .
"#;

/// Two target concepts carry the same name: one under destinations, one under
/// the material hierarchy. Both are perfect name matches for the source.
const TWO_TARGETS: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix t: <http://ties-tgt.example/> .
    t:WD-RED a skos:Concept ; skos:prefLabel "Redistribution"@en .
    t:FH-RED a skos:Concept ; skos:prefLabel "Redistribution"@en .
    t:WD-LF  a skos:Concept ; skos:prefLabel "Landfill"@en .
"#;

#[test]
fn a_tied_target_is_named_and_not_dropped() {
    let r = parse(
        engine(Arc::new(GraphStore::new()))
            .align_with_thresholds(ONE_SOURCE, Some(TWO_TARGETS), 0.85, 0.4, true)
            .unwrap(),
    );
    let red = r["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| {
            c["source_iri"]
                .as_str()
                .unwrap()
                .ends_with("redistribution")
        })
        .unwrap_or_else(|| panic!("no row for the tied source: {r}"))
        .clone();
    let kept = red["target_iri"].as_str().unwrap();
    let tied: Vec<String> = red["tied_with"]
        .as_array()
        .unwrap_or_else(|| panic!("the tie must be reported on the surviving row: {red}"))
        .iter()
        .map(|t| t["target_iri"].as_str().unwrap().to_string())
        .collect();
    let other = if kept.ends_with("WD-RED") {
        "FH-RED"
    } else {
        "WD-RED"
    };
    assert!(
        tied.iter().any(|t| t.ends_with(other)),
        "the losing target must be named: {red}"
    );
    assert_eq!(red["ambiguous"], true, "{red}");
    assert_eq!(
        red["requires_review"], true,
        "the engine did not choose this row, so a person must: {red}"
    );
    assert!(r["ambiguous_count"].as_u64().unwrap_or(0) >= 1, "{r}");

    let lf = r["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["source_iri"].as_str().unwrap().ends_with("landfill"))
        .unwrap()
        .clone();
    assert_eq!(
        lf["ambiguous"], false,
        "a clear winner is not ambiguous: {lf}"
    );
    assert!(
        lf["tied_with"].as_array().is_none_or(|a| a.is_empty()),
        "{lf}"
    );
}

#[test]
fn an_ambiguous_row_is_not_written_into_the_graph() {
    let graph = Arc::new(GraphStore::new());
    let r = parse(
        engine(graph.clone())
            .align_with_thresholds(ONE_SOURCE, Some(TWO_TARGETS), 0.85, 0.4, false)
            .unwrap(),
    );
    let red = r["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| {
            c["source_iri"]
                .as_str()
                .unwrap()
                .ends_with("redistribution")
        })
        .unwrap()
        .clone();
    assert_eq!(red["applied"], false, "{red}");
    // The unambiguous Landfill row is the only triple written.
    assert_eq!(r["applied_count"], 1, "{r}");
    assert_eq!(graph.triple_count(), 1);
}

#[test]
fn a_tie_on_the_target_side_is_named_too() {
    // Two sources with the same name compete for one target.
    let two_sources = r#"
        @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
        @prefix s: <http://ties-src2.example/> .
        s:a a skos:Concept ; skos:prefLabel "Landfill"@en .
        s:b a skos:Concept ; skos:prefLabel "Landfill"@en .
    "#;
    let one_target = r#"
        @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
        @prefix t: <http://ties-tgt2.example/> .
        t:LF a skos:Concept ; skos:prefLabel "Landfill"@en .
    "#;
    let r = parse(
        engine(Arc::new(GraphStore::new()))
            .align_with_thresholds(two_sources, Some(one_target), 0.85, 0.4, true)
            .unwrap(),
    );
    let rows = r["candidates"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "one-to-one keeps one row: {r}");
    let tied = rows[0]["tied_with"].to_string();
    assert!(
        tied.contains("ties-src2.example/"),
        "the losing source must be named: {r}"
    );
    assert_eq!(rows[0]["ambiguous"], true);
}

/// A source FINER than its target: three source channels that each, correctly,
/// map to one coarser target code.
const FINE_SOURCE: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix s: <http://fine.example/> .
    s:a a skos:Concept ; skos:prefLabel "Redistribution"@en .
    s:b a skos:Concept ; skos:prefLabel "Redistribution (charity)"@en ; skos:hiddenLabel "Redistribution"@en .
    s:c a skos:Concept ; skos:prefLabel "Landfill"@en .
"#;

const COARSE_TARGET: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix t: <http://coarse.example/> .
    t:RED a skos:Concept ; skos:prefLabel "Redistribution"@en .
    t:LF  a skos:Concept ; skos:prefLabel "Landfill"@en .
"#;

#[test]
fn many_to_one_keeps_every_source_that_maps_to_one_target() {
    use open_ontologies::align::{AlignOptions, Cardinality};
    let eng = engine(Arc::new(GraphStore::new()));
    let opts = |cardinality| AlignOptions {
        cardinality,
        dry_run: true,
        ..AlignOptions::default()
    };
    let rows = |v: &Value| -> Vec<(String, String)> {
        v["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c["source_iri"].as_str().unwrap().to_string(),
                    c["target_iri"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    };

    let one = parse(
        eng.align_with_options(
            FINE_SOURCE,
            Some(COARSE_TARGET),
            &opts(Cardinality::OneToOne),
        )
        .unwrap(),
    );
    assert_eq!(
        rows(&one)
            .iter()
            .filter(|(_, t)| t.ends_with("/RED"))
            .count(),
        1,
        "{one}"
    );
    // Under one-to-one the second source is not silently gone either: it tied.
    assert_eq!(one["ambiguous_count"], 1, "{one}");

    let many = parse(
        eng.align_with_options(
            FINE_SOURCE,
            Some(COARSE_TARGET),
            &opts(Cardinality::ManyToOne),
        )
        .unwrap(),
    );
    assert_eq!(many["cardinality"], "many_to_one");
    let r = rows(&many);
    assert!(
        r.contains(&(
            "http://fine.example/a".into(),
            "http://coarse.example/RED".into()
        )),
        "{many}"
    );
    assert!(
        r.contains(&(
            "http://fine.example/b".into(),
            "http://coarse.example/RED".into()
        )),
        "{many}"
    );
    assert!(
        r.contains(&(
            "http://fine.example/c".into(),
            "http://coarse.example/LF".into()
        )),
        "{many}"
    );
    assert_eq!(
        many["ambiguous_count"], 0,
        "sharing a target is not a tie under many-to-one: {many}"
    );
}
