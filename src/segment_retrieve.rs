//! TBox-slice retrieval for ontology-grounded RAG (#34, SEMANTiCS 2025
//! GrOWL-RAG).
//!
//! Given a set of seed IRIs, returns a Turtle-serialised k-hop neighbourhood
//! restricted to the TBox (terminological structure): subclass / equivalent
//! class / property-domain / property-range / property-superproperty.
//! Intentionally omits ABox triples (instance data) unless explicitly asked
//! for, so the slice is suitable for grounding an LLM's reasoning over
//! structure rather than data.
//!
//! Pairs with `graph_projection_lossy_check` (#35): this primitive produces
//! the slice; that one audits what was dropped.

use crate::graph::GraphStore;
use serde::Serialize;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize)]
pub struct SegmentResult {
    /// Slice as Turtle (Oxigraph-canonical).
    pub turtle: String,
    /// Number of distinct IRIs reached.
    pub iri_count: usize,
    /// Number of triples emitted.
    pub triple_count: usize,
    /// IRIs visited but not expanded (hit the hop budget).
    pub frontier_iris: Vec<String>,
}

/// TBox-relevant predicates that get traversed when retrieving a slice.
const TBOX_PREDICATES: &[&str] = &[
    "http://www.w3.org/2000/01/rdf-schema#subClassOf",
    "http://www.w3.org/2000/01/rdf-schema#subPropertyOf",
    "http://www.w3.org/2000/01/rdf-schema#domain",
    "http://www.w3.org/2000/01/rdf-schema#range",
    "http://www.w3.org/2002/07/owl#equivalentClass",
    "http://www.w3.org/2002/07/owl#equivalentProperty",
    "http://www.w3.org/2002/07/owl#disjointWith",
    "http://www.w3.org/2002/07/owl#inverseOf",
];

/// Retrieve a TBox slice. `seed_iris` are the starting points; `hops`
/// bounds the BFS depth (default 2 if 0). `include_abox = true` also pulls
/// instance triples (`?inst a <seed>`).
pub fn retrieve_segment(
    graph: &Arc<GraphStore>,
    seed_iris: &[String],
    hops: u32,
    include_abox: bool,
) -> anyhow::Result<SegmentResult> {
    let max_hops = if hops == 0 { 2 } else { hops };
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut frontier: Vec<String> = seed_iris.to_vec();
    let mut triples: BTreeSet<(String, String, String)> = BTreeSet::new();

    for _ in 0..max_hops {
        let mut next_frontier: Vec<String> = Vec::new();
        for iri in &frontier {
            if !visited.insert(iri.clone()) {
                continue;
            }
            // Outgoing TBox edges.
            for pred in TBOX_PREDICATES {
                let q = format!(
                    r#"SELECT ?o WHERE {{ <{iri}> <{pred}> ?o }} LIMIT 200"#,
                    iri = iri,
                    pred = pred
                );
                if let Ok(js) = graph.sparql_select(&q)
                    && let Ok(v) = serde_json::from_str::<serde_json::Value>(&js)
                    && let Some(rows) = v["results"].as_array()
                {
                    for row in rows {
                        if let Some(o) = row["o"].as_str() {
                            let o = o.trim_matches(|c| c == '<' || c == '>').to_string();
                            triples.insert((iri.clone(), pred.to_string(), o.clone()));
                            if !visited.contains(&o) {
                                next_frontier.push(o);
                            }
                        }
                    }
                }
            }
            // Also pull rdf:type for each seed so the slice declares classhood.
            let q = format!(
                "SELECT ?o WHERE {{ <{}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?o }}",
                iri
            );
            if let Ok(js) = graph.sparql_select(&q)
                && let Ok(v) = serde_json::from_str::<serde_json::Value>(&js)
                && let Some(rows) = v["results"].as_array()
            {
                for row in rows {
                    if let Some(o) = row["o"].as_str() {
                        let o = o.trim_matches(|c| c == '<' || c == '>').to_string();
                        triples.insert((
                            iri.clone(),
                            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                            o,
                        ));
                    }
                }
            }
            if include_abox {
                // Instance triples whose type is the seed.
                let q = format!(
                    "SELECT ?s WHERE {{ ?s <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{}> }} LIMIT 100",
                    iri
                );
                if let Ok(js) = graph.sparql_select(&q)
                    && let Ok(v) = serde_json::from_str::<serde_json::Value>(&js)
                    && let Some(rows) = v["results"].as_array()
                {
                    for row in rows {
                        if let Some(s) = row["s"].as_str() {
                            let s = s.trim_matches(|c| c == '<' || c == '>').to_string();
                            triples.insert((
                                s.clone(),
                                "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                                iri.clone(),
                            ));
                        }
                    }
                }
            }
        }
        frontier = next_frontier;
        if frontier.is_empty() {
            break;
        }
    }

    let mut turtle = String::new();
    for (s, p, o) in &triples {
        turtle.push_str(&format!("<{}> <{}> <{}> .\n", s, p, o));
    }

    Ok(SegmentResult {
        turtle,
        iri_count: visited.len(),
        triple_count: triples.len(),
        frontier_iris: frontier,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_with_hierarchy() -> Arc<GraphStore> {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex: <http://ex.org/> .
            ex:LivingThing a owl:Class .
            ex:Animal a owl:Class ; rdfs:subClassOf ex:LivingThing .
            ex:Cat a owl:Class ; rdfs:subClassOf ex:Animal .
            ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
            ex:tigger a ex:Cat .
        "#,
            None,
        )
        .unwrap();
        g
    }

    #[test]
    fn retrieve_one_hop_pulls_immediate_parent() {
        let g = graph_with_hierarchy();
        let r = retrieve_segment(&g, &["http://ex.org/Cat".to_string()], 1, false).unwrap();
        assert!(r.turtle.contains("ex.org/Cat"));
        assert!(r.turtle.contains("ex.org/Animal"));
        // Should NOT have walked to LivingThing yet (1 hop only).
        assert!(!r.turtle.contains("LivingThing"));
        assert!(r.iri_count >= 1);
        assert!(r.triple_count >= 1);
    }

    #[test]
    fn retrieve_two_hops_walks_transitive_chain() {
        let g = graph_with_hierarchy();
        let r = retrieve_segment(&g, &["http://ex.org/Cat".to_string()], 2, false).unwrap();
        assert!(r.turtle.contains("ex.org/Animal"));
        assert!(r.turtle.contains("ex.org/LivingThing"),
            "two-hop walk should reach LivingThing; got:\n{}", r.turtle);
    }

    #[test]
    fn retrieve_excludes_abox_by_default() {
        let g = graph_with_hierarchy();
        let r = retrieve_segment(&g, &["http://ex.org/Cat".to_string()], 1, false).unwrap();
        assert!(!r.turtle.contains("ex.org/tigger"),
            "instance triples must NOT appear when include_abox=false");
    }

    #[test]
    fn retrieve_includes_abox_when_requested() {
        let g = graph_with_hierarchy();
        let r = retrieve_segment(&g, &["http://ex.org/Cat".to_string()], 1, true).unwrap();
        assert!(r.turtle.contains("ex.org/tigger"),
            "instance triples must appear when include_abox=true; got:\n{}", r.turtle);
    }

    #[test]
    fn retrieve_handles_seeds_with_no_outgoing_edges() {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            "@prefix ex: <http://ex.org/> . @prefix owl: <http://www.w3.org/2002/07/owl#> . ex:Lonely a owl:Class .",
            None,
        )
        .unwrap();
        let r = retrieve_segment(&g, &["http://ex.org/Lonely".to_string()], 2, false).unwrap();
        // Should still emit the rdf:type triple for the seed.
        assert!(r.turtle.contains("ex.org/Lonely"));
    }
}
