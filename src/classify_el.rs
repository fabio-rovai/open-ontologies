//! OWL-EL classification primitive (#30, ELK-style).
//!
//! OWL 2 EL is the polynomial-time profile suitable for ontologies with
//! large class hierarchies but limited expressivity. ELK (Kazakov et al.,
//! 2011) is the canonical EL reasoner.
//!
//! ## Scope
//!
//! This primitive runs the existing OWL-RL reasoner on the loaded graph
//! (which is sufficient for the EL fragment: subClassOf transitivity,
//! property hierarchies, existential restrictions via owl-rl-ext) and
//! emits the **full classification table** — every materialised
//! `?sub rdfs:subClassOf ?super` pair, sorted, deduplicated.
//!
//! For deep OWL-DL (SHOIQ), use `onto_dl_check` / `onto_dl_explain` which
//! delegate to the tableaux reasoner.

use crate::graph::GraphStore;
use crate::reason::Reasoner;
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize)]
pub struct ClassificationReport {
    pub profile: String,
    pub class_count: usize,
    pub subsumption_count: usize,
    /// `[sub, super]` pairs. Sorted by sub then super for stable output.
    pub subsumptions: Vec<(String, String)>,
}

/// Classify the loaded ontology in the OWL-EL fragment. Runs OWL-RL
/// materialisation in a sandbox so the original graph is not mutated.
pub fn classify(graph: &Arc<GraphStore>) -> anyhow::Result<ClassificationReport> {
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
    let _ = Reasoner::run(&sandbox, "owl-rl-ext", true)?;

    let q = "SELECT ?sub ?sup WHERE { ?sub <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?sup } LIMIT 100000";
    let js = sandbox.sparql_select(q)?;
    let v: serde_json::Value = serde_json::from_str(&js)?;
    let rows = v["results"].as_array().cloned().unwrap_or_default();

    let mut subs: Vec<(String, String)> = rows
        .iter()
        .filter_map(|r| {
            let s = r["sub"].as_str()?.trim_matches(|c| c == '<' || c == '>').to_string();
            let o = r["sup"].as_str()?.trim_matches(|c| c == '<' || c == '>').to_string();
            // Skip trivial X ⊑ X and X ⊑ owl:Thing.
            if s == o || o == "http://www.w3.org/2002/07/owl#Thing" {
                return None;
            }
            Some((s, o))
        })
        .collect();
    subs.sort();
    subs.dedup();

    // Count distinct class IRIs participating in any subsumption.
    let mut classes: std::collections::BTreeSet<&String> = std::collections::BTreeSet::new();
    for (s, o) in &subs {
        classes.insert(s);
        classes.insert(o);
    }

    Ok(ClassificationReport {
        profile: "owl-rl-ext".to_string(),
        class_count: classes.len(),
        subsumption_count: subs.len(),
        subsumptions: subs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_emits_transitive_closure_of_subclassof() {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex: <http://ex.org/> .
            ex:A a owl:Class .
            ex:B a owl:Class ; rdfs:subClassOf ex:A .
            ex:C a owl:Class ; rdfs:subClassOf ex:B .
        "#,
            None,
        )
        .unwrap();
        let r = classify(&g).unwrap();
        // Transitive: C ⊑ A must be derived.
        let has_c_a = r.subsumptions.iter().any(|(s, o)| {
            s == "http://ex.org/C" && o == "http://ex.org/A"
        });
        assert!(has_c_a, "transitive subsumption not derived; got: {:?}", r.subsumptions);
        assert!(r.subsumption_count >= 3);
    }

    #[test]
    fn classify_excludes_trivial_self_subsumptions() {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix ex: <http://ex.org/> . ex:A a owl:Class .",
            None,
        )
        .unwrap();
        let r = classify(&g).unwrap();
        for (s, o) in &r.subsumptions {
            assert_ne!(s, o, "self-subsumption emitted");
        }
    }

    #[test]
    fn classify_does_not_mutate_original() {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . @prefix ex: <http://ex.org/> . ex:A a owl:Class . ex:B a owl:Class ; rdfs:subClassOf ex:A .",
            None,
        )
        .unwrap();
        let pre = g.triple_count();
        let _ = classify(&g).unwrap();
        assert_eq!(pre, g.triple_count());
    }
}
