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
use serde::Serialize;
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
