//! Named graphs survive a dataset round trip (issue #95).
//!
//! Temporal assertions live in named graphs whose validity is described in the
//! default graph. `GraphStore::serialize` used to render every format with
//! `serialize_triple`, which drops the graph name, so a TriG save/reload
//! flattened all assertions into the default graph and lost the
//! `validFrom`/`validTo`/`recordedAt` bindings on export. These tests pin that
//! TriG and N-Quads preserve the graph name, that the triple formats still
//! flatten (the only thing they can do), and that a full round trip keeps a
//! temporal snapshot answerable.

use open_ontologies::graph::GraphStore;
use open_ontologies::temporal::Temporal;
use oxigraph::io::RdfFormat;
use std::sync::Arc;

const DATASET: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1 { ex:HEK293 a ex:AdherentCellLine . }
ex:g2 { ex:HEK293 a ex:SuspensionCellLine . }

{
  ex:g1 t:validFrom "2024-01-01" ; t:validTo "2026-05-01" .
  ex:g2 t:validFrom "2026-05-01" .
}
"#;

fn named_graphs(store: &GraphStore) -> String {
    store
        .sparql_select("SELECT ?g WHERE { GRAPH ?g { ?s ?p ?o } }")
        .unwrap()
}

#[test]
fn trig_round_trip_preserves_named_graphs() {
    let first = GraphStore::new();
    first.load_content(DATASET, RdfFormat::TriG).unwrap();

    // The named graphs are present before export.
    let before = named_graphs(&first);
    assert!(before.contains("g1") && before.contains("g2"), "{before}");

    // Export to TriG and reload into a fresh store.
    let trig = first.serialize("trig").unwrap();
    assert!(
        trig.contains("g1") && trig.contains("g2"),
        "serialized TriG still names its graphs:\n{trig}"
    );
    let second = GraphStore::new();
    second.load_content(&trig, RdfFormat::TriG).unwrap();

    // The graph names survived the round trip — this is what fails when
    // serialize flattens to triples.
    let after = named_graphs(&second);
    assert!(
        after.contains("g1") && after.contains("g2"),
        "named graphs lost on TriG round trip:\n{after}"
    );

    // And the default-graph validity metadata survived with them.
    let validity = second
        .sparql_select(
            "SELECT ?from WHERE { <http://example.org/g1> \
             <https://open-ontologies.org/temporal#validFrom> ?from }",
        )
        .unwrap();
    assert!(validity.contains("2024-01-01"), "{validity}");
}

#[test]
fn nquads_round_trip_preserves_named_graphs() {
    let first = GraphStore::new();
    first.load_content(DATASET, RdfFormat::TriG).unwrap();

    let nq = first.serialize("nquads").unwrap();
    // Every quad in a named graph carries its graph term at the end of the line.
    assert!(
        nq.contains("<http://example.org/g1>") && nq.contains("<http://example.org/g2>"),
        "serialized N-Quads name their graphs:\n{nq}"
    );

    let second = GraphStore::new();
    second.load_content(&nq, RdfFormat::NQuads).unwrap();
    let after = named_graphs(&second);
    assert!(after.contains("g1") && after.contains("g2"), "{after}");
}

#[test]
fn triple_formats_still_flatten_named_graphs() {
    // A triple format cannot carry graph names; it flattens into one graph.
    // The triples are all still there, but no named graph survives — this is
    // the correct, lossy behaviour for N-Triples, and pinning it guards the
    // dataset-vs-triple branch in `serialize`.
    let store = GraphStore::new();
    store.load_content(DATASET, RdfFormat::TriG).unwrap();

    let nt = store.serialize("ntriples").unwrap();
    assert!(nt.contains("AdherentCellLine"), "the triples survive:\n{nt}");

    let reloaded = GraphStore::new();
    reloaded.load_content(&nt, RdfFormat::NTriples).unwrap();
    let after = named_graphs(&reloaded);
    assert!(
        !after.contains("g1") && !after.contains("g2"),
        "N-Triples has no named graphs to recover:\n{after}"
    );
}

#[test]
fn temporal_snapshot_answerable_after_trig_round_trip() {
    let first = GraphStore::new();
    first.load_content(DATASET, RdfFormat::TriG).unwrap();
    let trig = first.serialize("trig").unwrap();

    let reloaded = Arc::new(GraphStore::new());
    reloaded.load_content(&trig, RdfFormat::TriG).unwrap();

    // Ask what held in mid-2024: only the adherent period, and it must come
    // back by name — impossible if the round trip had flattened the graphs.
    let snap: serde_json::Value =
        serde_json::from_str(&Temporal::new(reloaded).snapshot(Some("2024-06-01"), None).unwrap())
            .unwrap();
    let in_scope: Vec<String> = snap["in_scope"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["graph"].as_str().unwrap().to_string())
        .collect();
    assert!(
        in_scope.iter().any(|g| g.ends_with("g1")),
        "g1 in scope at 2024-06-01: {in_scope:?}"
    );
    assert!(
        !in_scope.iter().any(|g| g.ends_with("g2")),
        "g2 not yet valid at 2024-06-01: {in_scope:?}"
    );
}
