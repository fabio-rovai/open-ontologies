//! Property-combination lattice for shape-based extraction (#36, K-CAP 2025
//! Kastor).
//!
//! Given a class IRI, enumerate the lattice of property subsets the class's
//! instances could be characterised by. Used by shape-induction algorithms
//! that want to enumerate candidate SHACL shapes from data:
//!
//! - Level 0: `{}` (the trivial "class membership" shape).
//! - Level 1: `{p1}, {p2}, ..., {pk}` for each domain-property.
//! - Level 2: `{p1, p2}, ...` all 2-subsets.
//! - ...
//! - Level k: `{p1, p2, ..., pk}` (all properties at once).
//!
//! The lattice grows as `2^k` so the primitive bounds at `max_size`
//! (default 3) — sufficient for the K-CAP 2025 Kastor algorithm and avoids
//! pathological blowup on rich domains.

use crate::graph::GraphStore;
use serde::Serialize;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize)]
pub struct LatticeReport {
    pub class_iri: String,
    pub properties_found: usize,
    pub max_size: usize,
    pub subsets_total: usize,
    /// Each subset is a sorted list of property IRIs. Sorted globally by
    /// (size, lex) for stable output.
    pub subsets: Vec<Vec<String>>,
}

/// Enumerate property-combination subsets up to `max_size` for a class.
pub fn enumerate(graph: &Arc<GraphStore>, class_iri: &str, max_size: usize) -> anyhow::Result<LatticeReport> {
    let max_size = if max_size == 0 { 3 } else { max_size };
    let q = format!(
        "SELECT DISTINCT ?p WHERE {{ ?p <http://www.w3.org/2000/01/rdf-schema#domain> <{}> }} LIMIT 100",
        class_iri
    );
    let js = graph.sparql_select(&q)?;
    let v: serde_json::Value = serde_json::from_str(&js)?;
    let rows = v["results"].as_array().cloned().unwrap_or_default();
    let mut props: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            r["p"]
                .as_str()
                .map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string())
        })
        .collect();
    props.sort();
    props.dedup();

    let k = props.len();
    let cap = max_size.min(k);
    let mut subsets: Vec<Vec<String>> = vec![Vec::new()];

    for size in 1..=cap {
        for combo in combinations(&props, size) {
            subsets.push(combo);
        }
    }

    // Sort by (size, lex).
    subsets.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    let total = subsets.len();
    Ok(LatticeReport {
        class_iri: class_iri.to_string(),
        properties_found: k,
        max_size,
        subsets_total: total,
        subsets,
    })
}

/// Enumerate k-subsets of `items` in lexicographic order. Iterative
/// implementation; no recursion-depth risk for k up to a handful.
fn combinations(items: &[String], k: usize) -> Vec<Vec<String>> {
    let n = items.len();
    if k == 0 || k > n {
        return Vec::new();
    }
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut indices: Vec<usize> = (0..k).collect();
    loop {
        out.push(indices.iter().map(|&i| items[i].clone()).collect());
        // Find the rightmost index that can be incremented.
        let mut i = k;
        while i > 0 && indices[i - 1] == n - k + (i - 1) {
            i -= 1;
        }
        if i == 0 {
            break;
        }
        indices[i - 1] += 1;
        for j in i..k {
            indices[j] = indices[j - 1] + 1;
        }
    }
    out
}

/// Unused helper kept for completeness — exposes the underlying set type.
#[allow(dead_code)]
fn to_set(items: &[String]) -> BTreeSet<String> {
    items.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_with_properties() -> Arc<GraphStore> {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex: <http://ex.org/> .
            ex:Person a owl:Class .
            ex:name a owl:DatatypeProperty ; rdfs:domain ex:Person .
            ex:age a owl:DatatypeProperty ; rdfs:domain ex:Person .
            ex:email a owl:DatatypeProperty ; rdfs:domain ex:Person .
        "#,
            None,
        )
        .unwrap();
        g
    }

    #[test]
    fn enumerate_full_lattice_for_three_properties_max_3() {
        let g = graph_with_properties();
        let r = enumerate(&g, "http://ex.org/Person", 3).unwrap();
        assert_eq!(r.properties_found, 3);
        // Empty + 3 singletons + 3 pairs + 1 triple = 8 = 2^3.
        assert_eq!(r.subsets_total, 8);
        // First subset is the empty one.
        assert_eq!(r.subsets[0], Vec::<String>::new());
    }

    #[test]
    fn enumerate_capped_at_max_size() {
        let g = graph_with_properties();
        let r = enumerate(&g, "http://ex.org/Person", 1).unwrap();
        // Empty + 3 singletons = 4.
        assert_eq!(r.subsets_total, 4);
    }

    #[test]
    fn enumerate_returns_just_empty_subset_for_no_properties() {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix ex: <http://ex.org/> . ex:Lonely a owl:Class .",
            None,
        )
        .unwrap();
        let r = enumerate(&g, "http://ex.org/Lonely", 3).unwrap();
        assert_eq!(r.properties_found, 0);
        assert_eq!(r.subsets_total, 1);
        assert_eq!(r.subsets[0], Vec::<String>::new());
    }

    #[test]
    fn enumerate_zero_max_size_defaults_to_three() {
        let g = graph_with_properties();
        let r = enumerate(&g, "http://ex.org/Person", 0).unwrap();
        // 0 is the sentinel for "use the default of 3" — enumeration
        // produces the full 2^3 = 8 lattice.
        assert_eq!(r.subsets_total, 8);
        assert_eq!(r.max_size, 3);
    }
}
