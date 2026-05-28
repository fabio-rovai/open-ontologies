//! Property-combination lattice + data-driven SHACL shape induction (#36,
//! K-CAP 2025 Kastor).
//!
//! Two layers:
//!
//!   1. **`enumerate`** — pure combinatorial enumeration of property-subset
//!      candidates up to `max_size`. Bounded at `2^max_size` to prevent
//!      pathological blowup.
//!
//!   2. **`induce_shapes`** — Kastor-style data-driven induction. For each
//!      candidate subset, evaluates against the loaded ontology's instance
//!      data and computes `support` (fraction of class instances that have
//!      ALL properties in the subset) and `confidence` (fraction of
//!      instances-with-this-subset that are also instances of the class).
//!      Ranks candidates by `support × confidence` and emits the top-k as
//!      proposed SHACL `NodeShape`s with `sh:property` blocks.

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

/// One induced candidate shape with its quality scores.
#[derive(Clone, Debug, Serialize)]
pub struct CandidateShape {
    /// Property IRIs the shape requires.
    pub properties: Vec<String>,
    /// Fraction of class instances that have every property in this subset
    /// (in `[0, 1]`).
    pub support: f64,
    /// Fraction of instances-with-this-subset that are members of the class
    /// (in `[0, 1]`). Higher confidence = more class-discriminative subset.
    pub confidence: f64,
    /// `support × confidence`, used to rank candidates.
    pub score: f64,
    /// Number of class instances covered by this shape.
    pub instances_covered: u64,
    /// SHACL NodeShape Turtle text proposing this constraint.
    pub shape_ttl: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct InductionReport {
    pub class_iri: String,
    pub class_instance_count: u64,
    pub candidates_evaluated: usize,
    pub shapes: Vec<CandidateShape>,
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

/// Induce candidate SHACL shapes from data (#36 Kastor proper). For each
/// non-empty property subset enumerated up to `max_size`, evaluate support +
/// confidence against the loaded instance data, then return the top
/// `top_k` by `support × confidence` (default 10).
///
/// `min_support` and `min_confidence` filter weak candidates (default 0.1
/// and 0.5 respectively).
pub fn induce_shapes(
    graph: &Arc<GraphStore>,
    class_iri: &str,
    max_size: usize,
    top_k: usize,
    min_support: f64,
    min_confidence: f64,
) -> anyhow::Result<InductionReport> {
    let lattice = enumerate(graph, class_iri, max_size)?;

    // Count class instances.
    let class_count = count_query(
        graph,
        &format!(
            "SELECT (COUNT(?x) AS ?n) WHERE {{ ?x a <{}> }}",
            class_iri
        ),
    )?;

    let mut candidates: Vec<CandidateShape> = Vec::new();
    for subset in &lattice.subsets {
        if subset.is_empty() {
            continue;
        }
        // count_class_with_subset: instances of class that have all
        // properties in subset.
        let class_with = count_class_with_subset(graph, class_iri, subset)?;
        // count_with_subset: instances ANYWHERE that have all properties.
        let any_with = count_any_with_subset(graph, subset)?;

        let support = if class_count == 0 {
            0.0
        } else {
            class_with as f64 / class_count as f64
        };
        let confidence = if any_with == 0 {
            0.0
        } else {
            class_with as f64 / any_with as f64
        };
        if support < min_support || confidence < min_confidence {
            continue;
        }
        let score = support * confidence;
        let shape_ttl = render_shape_ttl(class_iri, subset);
        candidates.push(CandidateShape {
            properties: subset.clone(),
            support,
            confidence,
            score,
            instances_covered: class_with,
            shape_ttl,
        });
    }
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(top_k);

    let evaluated = lattice.subsets.iter().filter(|s| !s.is_empty()).count();
    Ok(InductionReport {
        class_iri: class_iri.to_string(),
        class_instance_count: class_count,
        candidates_evaluated: evaluated,
        shapes: candidates,
    })
}

fn count_query(graph: &Arc<GraphStore>, q: &str) -> anyhow::Result<u64> {
    let js = graph.sparql_select(q)?;
    let v: serde_json::Value = serde_json::from_str(&js).unwrap_or(serde_json::Value::Null);
    let n_str = v["results"][0]["n"].as_str().unwrap_or("0");
    // SPARQL aggregate values come as literal strings like
    // `"5"^^<http://www.w3.org/2001/XMLSchema#integer>`. Strip the typed
    // wrapper, keep just the digits.
    let digits: String = n_str.chars().take_while(|c| c.is_ascii_digit()).collect();
    // If the leading char was a quote (Oxigraph string-encodes literals)
    // skip it and read until the next quote.
    let cleaned: String = if n_str.starts_with('"') {
        n_str
            .chars()
            .skip(1)
            .take_while(|c| c.is_ascii_digit())
            .collect()
    } else if !digits.is_empty() {
        digits
    } else {
        n_str.chars().filter(|c| c.is_ascii_digit()).collect()
    };
    Ok(cleaned.parse::<u64>().unwrap_or(0))
}

fn count_class_with_subset(
    graph: &Arc<GraphStore>,
    class_iri: &str,
    subset: &[String],
) -> anyhow::Result<u64> {
    if subset.is_empty() {
        return Ok(0);
    }
    let mut patterns: Vec<String> = vec![format!("?x a <{}>", class_iri)];
    for (i, p) in subset.iter().enumerate() {
        patterns.push(format!("?x <{}> ?o{}", p, i));
    }
    let q = format!(
        "SELECT (COUNT(DISTINCT ?x) AS ?n) WHERE {{ {} }}",
        patterns.join(" . ")
    );
    count_query(graph, &q)
}

fn count_any_with_subset(graph: &Arc<GraphStore>, subset: &[String]) -> anyhow::Result<u64> {
    if subset.is_empty() {
        return Ok(0);
    }
    let mut patterns: Vec<String> = Vec::new();
    for (i, p) in subset.iter().enumerate() {
        patterns.push(format!("?x <{}> ?o{}", p, i));
    }
    let q = format!(
        "SELECT (COUNT(DISTINCT ?x) AS ?n) WHERE {{ {} }}",
        patterns.join(" . ")
    );
    count_query(graph, &q)
}

/// Render a SHACL NodeShape proposing the subset's properties as
/// `sh:property` blocks with `sh:minCount 1`.
fn render_shape_ttl(class_iri: &str, subset: &[String]) -> String {
    let mut out = String::new();
    out.push_str("@prefix sh: <http://www.w3.org/ns/shacl#> .\n");
    out.push_str(&format!(
        "<{}-shape> a sh:NodeShape ;\n",
        class_iri
    ));
    out.push_str(&format!("  sh:targetClass <{}> ;\n", class_iri));
    for (i, p) in subset.iter().enumerate() {
        let prefix = if i + 1 == subset.len() { "" } else { " ;" };
        out.push_str(&format!(
            "  sh:property [ sh:path <{}> ; sh:minCount 1 ]{}\n",
            p, prefix
        ));
    }
    out.push_str(".\n");
    out
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

    fn graph_with_class_and_instances() -> Arc<GraphStore> {
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

            # 3 instances of Person, 2 with name+age, 1 with only name.
            ex:alice a ex:Person ; ex:name "Alice" ; ex:age 30 .
            ex:bob a ex:Person ; ex:name "Bob" ; ex:age 25 .
            ex:charlie a ex:Person ; ex:name "Charlie" .

            # 1 NON-Person instance with the same property — should lower
            # confidence on {name}.
            ex:nonperson_dog ex:name "Rex" .
        "#,
            None,
        )
        .unwrap();
        g
    }

    #[test]
    fn induce_shapes_returns_high_support_subsets() {
        let g = graph_with_class_and_instances();
        let r = induce_shapes(
            &g,
            "http://ex.org/Person",
            3,
            10,
            0.1,
            0.5,
        )
        .unwrap();
        assert_eq!(r.class_instance_count, 3);
        // {name} has support 3/3 = 1.0; confidence 3/4 = 0.75 (Rex is not
        // a Person). Should be the top candidate.
        let top = r
            .shapes
            .iter()
            .find(|c| c.properties == vec!["http://ex.org/name".to_string()])
            .expect("name subset should be in induced shapes");
        assert!((top.support - 1.0).abs() < 1e-9);
        assert!((top.confidence - 0.75).abs() < 1e-9);
        assert_eq!(top.instances_covered, 3);
    }

    #[test]
    fn induce_shapes_filters_by_min_support() {
        let g = graph_with_class_and_instances();
        // {email} has support 0 (no Person instance has it).
        let r = induce_shapes(
            &g,
            "http://ex.org/Person",
            3,
            10,
            0.5,
            0.0,
        )
        .unwrap();
        let email_in_shapes = r
            .shapes
            .iter()
            .any(|c| c.properties == vec!["http://ex.org/email".to_string()]);
        assert!(!email_in_shapes,
            "email has zero support, should be filtered by min_support=0.5");
    }

    #[test]
    fn induce_shapes_renders_valid_shacl_turtle() {
        let g = graph_with_class_and_instances();
        let r = induce_shapes(
            &g,
            "http://ex.org/Person",
            2,
            5,
            0.1,
            0.5,
        )
        .unwrap();
        let top = r.shapes.first().expect("at least one shape");
        assert!(top.shape_ttl.contains("sh:NodeShape"));
        assert!(top.shape_ttl.contains("sh:targetClass"));
        assert!(top.shape_ttl.contains("sh:property"));
        assert!(top.shape_ttl.contains("sh:minCount 1"));
        // Should parse as valid Turtle.
        let test_graph = Arc::new(GraphStore::new());
        let load_result = test_graph.load_turtle(&top.shape_ttl, None);
        assert!(load_result.is_ok(),
            "induced shape Turtle should parse: error: {:?}\nturtle:\n{}",
            load_result.err(), top.shape_ttl);
    }

    #[test]
    fn induce_shapes_ranks_by_score_descending() {
        let g = graph_with_class_and_instances();
        let r = induce_shapes(
            &g,
            "http://ex.org/Person",
            3,
            10,
            0.1,
            0.5,
        )
        .unwrap();
        // Score must be monotonically non-increasing.
        for w in r.shapes.windows(2) {
            assert!(
                w[0].score >= w[1].score,
                "shapes not ranked: {} < {} (props {:?} vs {:?})",
                w[0].score, w[1].score, w[0].properties, w[1].properties
            );
        }
    }

    #[test]
    fn induce_shapes_handles_class_with_no_instances() {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . @prefix ex: <http://ex.org/> . ex:Empty a owl:Class . ex:p a owl:DatatypeProperty ; rdfs:domain ex:Empty .",
            None,
        )
        .unwrap();
        let r = induce_shapes(
            &g,
            "http://ex.org/Empty",
            3,
            10,
            0.1,
            0.5,
        )
        .unwrap();
        assert_eq!(r.class_instance_count, 0);
        // No support for any subset → empty result.
        assert!(r.shapes.is_empty());
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
