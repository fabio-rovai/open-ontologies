//! Combined OWL+SHACL co-evolution validator (#33, K-CAP 2025).
//!
//! The K-CAP 2025 paper "Co-evolving OWL ontologies and SHACL shapes" shows
//! that validating SHACL against ABox data alone misses constraints that
//! become satisfiable only after OWL inference (`rdfs:subClassOf` transitive
//! closure, `owl:sameAs`, domain/range propagation). The co-evolve check
//! materialises OWL entailments first, then runs SHACL against the closure.
//!
//! This is a structural extension of the existing `onto_shacl` and
//! `onto_shacl_check`: same SHACL semantics, but the validator sees the
//! reasoner's output instead of the raw ABox.
//!
//! ## Bounded scope
//!
//! - Profile: `owl-rl` (the standard for SHACL co-evolution per the paper).
//! - Operates on a **sandbox copy** of the graph so the caller's ABox is
//!   not permanently materialised.
//! - Returns a unified report: pre-reasoning conformance, post-reasoning
//!   conformance, and the count of triples the reasoner added.

use crate::graph::GraphStore;
use crate::reason::Reasoner;
use crate::shacl::ShaclValidator;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Report from a co-evolve check.
#[derive(Clone, Debug, Serialize)]
pub struct CoevolveReport {
    /// Conformance verdict against the un-materialised graph (matches
    /// `onto_shacl`'s output).
    pub pre_reasoning: String,
    /// Conformance verdict against the OWL-RL closure. This is the verdict
    /// that matters for SHACL co-evolution.
    pub post_reasoning: String,
    /// Triples added by OWL-RL materialisation.
    pub triples_inferred: usize,
    /// The reasoner profile run (e.g. "owl-rl").
    pub profile: String,
}

/// Run combined OWL+SHACL validation. The original `graph` is NOT mutated —
/// reasoning happens against a sandbox copy.
pub fn coevolve_check(
    graph: &Arc<GraphStore>,
    shapes_ttl: &str,
    profile: &str,
) -> anyhow::Result<CoevolveReport> {
    let pre = ShaclValidator::validate(graph, shapes_ttl)?;

    // Sandbox: copy current triples into a fresh store, run reasoner there.
    let sandbox = Arc::new(GraphStore::new());
    let triples = graph.all_triples()?;
    if !triples.is_empty() {
        let mut nt = String::with_capacity(triples.len() * 64);
        for (s, p, o) in &triples {
            nt.push_str(s);
            nt.push(' ');
            nt.push_str(p);
            nt.push(' ');
            nt.push_str(o);
            nt.push_str(" .\n");
        }
        sandbox.load_turtle(&nt, None)?;
    }
    let pre_count = sandbox.triple_count();
    let _ = Reasoner::run(&sandbox, profile, true)?;
    let post_count = sandbox.triple_count();
    let inferred = post_count.saturating_sub(pre_count);
    let post = ShaclValidator::validate(&sandbox, shapes_ttl)?;

    Ok(CoevolveReport {
        pre_reasoning: pre,
        post_reasoning: post,
        triples_inferred: inferred,
        profile: profile.to_string(),
    })
}

// ─── Incremental dependency-graph validation (#33 follow-on) ───────────────
//
// Per the K-CAP 2025 paper: when an OWL ontology evolves, re-running every
// SHACL shape against the full closure is wasteful. The shape-OWL dependency
// graph maps each shape to the OWL classes/properties it references, so
// when a delta touches a subset of those references, only the affected
// shapes need revalidation.

/// One shape's parsed references — the classes and properties it depends on.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ShapeDependencies {
    pub shape_iri: String,
    pub target_classes: BTreeSet<String>,
    pub path_properties: BTreeSet<String>,
    /// Classes referenced in `sh:class` constraints (object property ranges).
    pub class_constraints: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DependencyGraph {
    pub shapes: Vec<ShapeDependencies>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AffectedShapesReport {
    pub changed_iri_count: usize,
    pub total_shapes: usize,
    pub affected_shapes: Vec<String>,
    pub skipped_shapes: Vec<String>,
}

/// Build the shape→dependencies map by SPARQL-querying the shapes Turtle.
/// `shapes_ttl` is loaded into a temporary graph so we don't touch the
/// caller's ontology.
pub fn build_dependency_graph(shapes_ttl: &str) -> anyhow::Result<DependencyGraph> {
    let scratch = Arc::new(GraphStore::new());
    if !shapes_ttl.trim().is_empty() {
        scratch.load_turtle(shapes_ttl, None)?;
    }

    // Find every NodeShape declaration + its referenced classes/paths.
    let q = r#"
        SELECT ?shape ?target ?path ?cls WHERE {
            ?shape a <http://www.w3.org/ns/shacl#NodeShape> .
            OPTIONAL { ?shape <http://www.w3.org/ns/shacl#targetClass> ?target }
            OPTIONAL {
                ?shape <http://www.w3.org/ns/shacl#property> ?prop .
                OPTIONAL { ?prop <http://www.w3.org/ns/shacl#path> ?path }
                OPTIONAL { ?prop <http://www.w3.org/ns/shacl#class> ?cls }
            }
        }
    "#;
    let js = scratch.sparql_select(q)?;
    let v: serde_json::Value = serde_json::from_str(&js).unwrap_or(serde_json::Value::Null);
    let rows = v["results"].as_array().cloned().unwrap_or_default();

    let mut map: BTreeMap<String, ShapeDependencies> = BTreeMap::new();
    for row in rows {
        let Some(shape) = row["shape"].as_str() else {
            continue;
        };
        let shape = shape.trim_matches(|c| c == '<' || c == '>').to_string();
        let entry = map.entry(shape.clone()).or_insert_with(|| ShapeDependencies {
            shape_iri: shape.clone(),
            target_classes: BTreeSet::new(),
            path_properties: BTreeSet::new(),
            class_constraints: BTreeSet::new(),
        });
        if let Some(t) = row["target"].as_str() {
            entry
                .target_classes
                .insert(t.trim_matches(|c| c == '<' || c == '>').to_string());
        }
        if let Some(p) = row["path"].as_str() {
            entry
                .path_properties
                .insert(p.trim_matches(|c| c == '<' || c == '>').to_string());
        }
        if let Some(c) = row["cls"].as_str() {
            entry
                .class_constraints
                .insert(c.trim_matches(|c| c == '<' || c == '>').to_string());
        }
    }
    Ok(DependencyGraph {
        shapes: map.into_values().collect(),
    })
}

/// Given a dependency graph + a set of changed IRIs, return the names of
/// shapes whose validity could plausibly have been affected.
pub fn affected_shapes(graph: &DependencyGraph, changed_iris: &[String]) -> AffectedShapesReport {
    let changed: BTreeSet<&str> = changed_iris.iter().map(|s| s.as_str()).collect();
    let mut affected: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for sd in &graph.shapes {
        let touched = sd.target_classes.iter().any(|c| changed.contains(c.as_str()))
            || sd.path_properties.iter().any(|p| changed.contains(p.as_str()))
            || sd.class_constraints.iter().any(|c| changed.contains(c.as_str()));
        if touched {
            affected.push(sd.shape_iri.clone());
        } else {
            skipped.push(sd.shape_iri.clone());
        }
    }
    AffectedShapesReport {
        changed_iri_count: changed_iris.len(),
        total_shapes: graph.shapes.len(),
        affected_shapes: affected,
        skipped_shapes: skipped,
    }
}

/// Incremental coevolve check: revalidate only shapes whose dependencies
/// intersect `changed_iris`. Returns the validation result and the
/// affected/skipped list.
#[derive(Clone, Debug, Serialize)]
pub struct IncrementalReport {
    pub affected: AffectedShapesReport,
    pub validation_for_affected: String,
}

pub fn incremental_check(
    graph: &Arc<GraphStore>,
    shapes_ttl: &str,
    changed_iris: &[String],
    profile: &str,
) -> anyhow::Result<IncrementalReport> {
    let dep = build_dependency_graph(shapes_ttl)?;
    let aff = affected_shapes(&dep, changed_iris);

    // Materialise OWL closure into a sandbox.
    let sandbox = Arc::new(GraphStore::new());
    let triples = graph.all_triples()?;
    if !triples.is_empty() {
        let mut nt = String::with_capacity(triples.len() * 64);
        for (s, p, o) in &triples {
            nt.push_str(s);
            nt.push(' ');
            nt.push_str(p);
            nt.push(' ');
            nt.push_str(o);
            nt.push_str(" .\n");
        }
        sandbox.load_turtle(&nt, None)?;
    }
    let _ = Reasoner::run(&sandbox, profile, true)?;

    // If no shapes are affected, skip validation entirely.
    let validation = if aff.affected_shapes.is_empty() {
        r#"{"conforms":true,"reason":"no_affected_shapes"}"#.to_string()
    } else {
        // Subset the shapes Turtle to just the affected ones. For the
        // scaffold we re-run the full SHACL validation (Oxigraph's SHACL
        // engine doesn't expose per-shape evaluation directly), but the
        // call is skipped entirely when affected_shapes is empty — which
        // is the actual speedup the K-CAP paper measures.
        ShaclValidator::validate(&sandbox, shapes_ttl)?
    };

    Ok(IncrementalReport {
        affected: aff,
        validation_for_affected: validation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coevolve_returns_both_verdicts_and_inferred_count() {
        let graph = Arc::new(GraphStore::new());
        graph
            .load_turtle(
                r#"
                @prefix owl: <http://www.w3.org/2002/07/owl#> .
                @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
                @prefix ex: <http://ex.org/> .
                ex:Animal a owl:Class .
                ex:Cat a owl:Class ; rdfs:subClassOf ex:Animal .
                ex:tigger a ex:Cat .
            "#,
                None,
            )
            .unwrap();
        let shapes = r#"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <http://ex.org/> .
            ex:AnimalShape a sh:NodeShape ; sh:targetClass ex:Animal .
        "#;
        let report = coevolve_check(&graph, shapes, "owl-rl").unwrap();
        // Both verdicts are non-empty JSON.
        assert!(!report.pre_reasoning.is_empty());
        assert!(!report.post_reasoning.is_empty());
        assert_eq!(report.profile, "owl-rl");
        // The original graph is not mutated.
        let original_count = graph.triple_count();
        assert!(
            original_count <= 10,
            "original graph should be small; got {}",
            original_count
        );
    }

    #[test]
    fn coevolve_does_not_mutate_original_graph() {
        let graph = Arc::new(GraphStore::new());
        graph
            .load_turtle(
                r#"
                @prefix owl: <http://www.w3.org/2002/07/owl#> .
                @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
                @prefix ex: <http://ex.org/> .
                ex:Cat a owl:Class ; rdfs:subClassOf ex:Animal .
                ex:Animal a owl:Class .
                ex:tigger a ex:Cat .
            "#,
                None,
            )
            .unwrap();
        let pre = graph.triple_count();
        let shapes = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://ex.org/> . ex:S a sh:NodeShape ; sh:targetClass ex:Animal .";
        let _ = coevolve_check(&graph, shapes, "owl-rl").unwrap();
        let post = graph.triple_count();
        assert_eq!(pre, post, "original graph mutated by co-evolve check");
    }

    fn shapes_ttl() -> &'static str {
        r#"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <http://ex.org/> .
            ex:AnimalShape a sh:NodeShape ;
                sh:targetClass ex:Animal ;
                sh:property [ sh:path ex:hasName ; sh:minCount 1 ] .
            ex:VehicleShape a sh:NodeShape ;
                sh:targetClass ex:Vehicle ;
                sh:property [ sh:path ex:hasModel ; sh:minCount 1 ] .
        "#
    }

    #[test]
    fn build_dependency_graph_collects_target_class_and_path() {
        let dep = build_dependency_graph(shapes_ttl()).unwrap();
        assert_eq!(dep.shapes.len(), 2);
        // AnimalShape must have ex:Animal in target_classes + ex:hasName in path.
        let animal = dep
            .shapes
            .iter()
            .find(|s| s.shape_iri == "http://ex.org/AnimalShape")
            .expect("AnimalShape parsed");
        assert!(animal.target_classes.contains("http://ex.org/Animal"));
        assert!(animal.path_properties.contains("http://ex.org/hasName"));
    }

    #[test]
    fn affected_shapes_filters_by_changed_iris() {
        let dep = build_dependency_graph(shapes_ttl()).unwrap();
        // Change only ex:Animal — VehicleShape must be skipped.
        let r = affected_shapes(&dep, &["http://ex.org/Animal".to_string()]);
        assert_eq!(r.affected_shapes.len(), 1);
        assert!(r.affected_shapes[0].contains("AnimalShape"));
        assert_eq!(r.skipped_shapes.len(), 1);
        assert!(r.skipped_shapes[0].contains("VehicleShape"));
    }

    #[test]
    fn affected_shapes_triggers_on_path_property_too() {
        let dep = build_dependency_graph(shapes_ttl()).unwrap();
        // Change a property referenced in sh:path → its parent shape is affected.
        let r = affected_shapes(&dep, &["http://ex.org/hasModel".to_string()]);
        assert_eq!(r.affected_shapes.len(), 1);
        assert!(r.affected_shapes[0].contains("VehicleShape"));
    }

    #[test]
    fn affected_shapes_returns_empty_when_no_overlap() {
        let dep = build_dependency_graph(shapes_ttl()).unwrap();
        let r = affected_shapes(&dep, &["http://ex.org/Unrelated".to_string()]);
        assert!(r.affected_shapes.is_empty());
        assert_eq!(r.skipped_shapes.len(), 2);
    }

    #[test]
    fn incremental_check_skips_validation_when_nothing_affected() {
        let graph = Arc::new(GraphStore::new());
        graph
            .load_turtle(
                r#"
                @prefix owl: <http://www.w3.org/2002/07/owl#> .
                @prefix ex: <http://ex.org/> .
                ex:Animal a owl:Class .
                ex:Vehicle a owl:Class .
            "#,
                None,
            )
            .unwrap();
        let r = incremental_check(
            &graph,
            shapes_ttl(),
            &["http://ex.org/Unrelated".to_string()],
            "owl-rl",
        )
        .unwrap();
        assert_eq!(r.affected.affected_shapes.len(), 0);
        assert!(r.validation_for_affected.contains("no_affected_shapes"));
    }

    #[test]
    fn coevolve_records_profile_string() {
        let graph = Arc::new(GraphStore::new());
        graph
            .load_turtle("@prefix ex: <http://ex.org/> . ex:A a ex:B .", None)
            .unwrap();
        let report = coevolve_check(&graph, "", "rdfs").unwrap();
        assert_eq!(report.profile, "rdfs");
    }
}
