use std::sync::Arc;

use crate::graph::GraphStore;

/// RDFS inference rules as (name, SPARQL UPDATE) tuples.
const RDFS_RULES: &[(&str, &str)] = &[
    (
        "rdfs9-subclass",
        "INSERT { ?x a ?super } WHERE { \
         ?x a ?sub . \
         ?sub <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?super . \
         FILTER(?sub != ?super) \
         FILTER NOT EXISTS { ?x a ?super } }",
    ),
    (
        "rdfs11-subclass-trans",
        "INSERT { ?a <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c } WHERE { \
         ?a <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?b . \
         ?b <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c . \
         FILTER(?a != ?b && ?b != ?c && ?a != ?c) \
         FILTER NOT EXISTS { ?a <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c } }",
    ),
    (
        "rdfs2-domain",
        "INSERT { ?s a ?class } WHERE { \
         ?s ?p ?o . \
         ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?class . \
         FILTER NOT EXISTS { ?s a ?class } }",
    ),
    (
        "rdfs3-range",
        "INSERT { ?o a ?class } WHERE { \
         ?s ?p ?o . \
         ?p <http://www.w3.org/2000/01/rdf-schema#range> ?class . \
         FILTER(isIRI(?o)) \
         FILTER NOT EXISTS { ?o a ?class } }",
    ),
    (
        "rdfs5-subprop-trans",
        "INSERT { ?a <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?c } WHERE { \
         ?a <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?b . \
         ?b <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?c . \
         FILTER(?a != ?b && ?b != ?c && ?a != ?c) \
         FILTER NOT EXISTS { ?a <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?c } }",
    ),
    (
        "rdfs7-subprop",
        "INSERT { ?s ?super ?o } WHERE { \
         ?s ?sub ?o . \
         ?sub <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?super . \
         FILTER(?sub != ?super) \
         FILTER NOT EXISTS { ?s ?super ?o } }",
    ),
];

/// OWL RL inference rules (additional, on top of RDFS).
const OWL_RL_RULES: &[(&str, &str)] = &[
    (
        "owl-transitive",
        "INSERT { ?x ?p ?z } WHERE { \
         ?p a <http://www.w3.org/2002/07/owl#TransitiveProperty> . \
         ?x ?p ?y . \
         ?y ?p ?z . \
         FILTER(?x != ?z) \
         FILTER NOT EXISTS { ?x ?p ?z } }",
    ),
    (
        "owl-symmetric",
        "INSERT { ?o ?p ?s } WHERE { \
         ?p a <http://www.w3.org/2002/07/owl#SymmetricProperty> . \
         ?s ?p ?o . \
         FILTER NOT EXISTS { ?o ?p ?s } }",
    ),
    (
        "owl-inverse",
        "INSERT { ?o ?q ?s } WHERE { \
         ?p <http://www.w3.org/2002/07/owl#inverseOf> ?q . \
         ?s ?p ?o . \
         FILTER NOT EXISTS { ?o ?q ?s } }",
    ),
    (
        "owl-sameas-sym",
        "INSERT { ?b <http://www.w3.org/2002/07/owl#sameAs> ?a } WHERE { \
         ?a <http://www.w3.org/2002/07/owl#sameAs> ?b . \
         FILTER NOT EXISTS { ?b <http://www.w3.org/2002/07/owl#sameAs> ?a } }",
    ),
];

/// Reasoner that runs RDFS and OWL-RL inference rules via iterative
/// SPARQL INSERT WHERE queries against a `GraphStore`.
pub struct Reasoner;

impl Reasoner {
    /// Run inference rules against the graph store.
    ///
    /// `profile`: "rdfs" or "owl-rl" (owl-rl includes rdfs rules).
    /// `materialize`: if true, insert inferred triples into the store.
    ///                if false, count what would be inferred without modifying the store.
    pub fn run(
        graph: &Arc<GraphStore>,
        profile: &str,
        materialize: bool,
    ) -> anyhow::Result<String> {
        let rules: Vec<(&str, &str)> = match profile {
            "owl-rl" => RDFS_RULES
                .iter()
                .chain(OWL_RL_RULES.iter())
                .copied()
                .collect(),
            _ => RDFS_RULES.to_vec(),
        };

        let profile_used = match profile {
            "owl-rl" => "owl-rl",
            _ => "rdfs",
        };

        if !materialize {
            return Self::dry_run(graph, &rules, profile_used);
        }

        let mut total_inferred: usize = 0;
        let mut iterations: usize = 0;

        loop {
            iterations += 1;
            let before = graph.triple_count();

            for (_name, rule) in &rules {
                // Ignore errors on individual rules — some may not match anything
                let _ = graph.sparql_update(rule);
            }

            let after = graph.triple_count();
            let delta = after.saturating_sub(before);
            total_inferred += delta;

            if delta == 0 || iterations >= 20 {
                break;
            }
        }

        // Gather a sample of inferred triples
        let sample = Self::sample_inferences(graph);

        Ok(serde_json::json!({
            "profile_used": profile_used,
            "inferred_count": total_inferred,
            "iterations": iterations,
            "sample_inferences": sample
        })
        .to_string())
    }

    /// Dry run: snapshot the store, run rules on a temporary copy,
    /// count inferred triples, discard the copy. Original store is unchanged.
    fn dry_run(
        graph: &Arc<GraphStore>,
        rules: &[(&str, &str)],
        profile_used: &str,
    ) -> anyhow::Result<String> {
        let snapshot = graph.serialize("ntriples")?;
        let temp_store = Arc::new(GraphStore::new());
        temp_store.load_ntriples(&snapshot)?;

        let mut total_inferred: usize = 0;
        let mut iterations: usize = 0;

        loop {
            iterations += 1;
            let before = temp_store.triple_count();

            for (_name, rule) in rules {
                let _ = temp_store.sparql_update(rule);
            }

            let after = temp_store.triple_count();
            let delta = after.saturating_sub(before);
            total_inferred += delta;

            if delta == 0 || iterations >= 20 {
                break;
            }
        }

        let sample = Self::sample_inferences(&temp_store);

        Ok(serde_json::json!({
            "profile_used": profile_used,
            "inferred_count": total_inferred,
            "iterations": iterations,
            "dry_run": true,
            "sample_inferences": sample
        })
        .to_string())
    }

    /// Retrieve a small sample of rdf:type triples for reporting.
    fn sample_inferences(store: &Arc<GraphStore>) -> Vec<String> {
        let query = "SELECT ?s ?type WHERE { ?s a ?type } LIMIT 10";
        match store.sparql_select(query) {
            Ok(result) => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result) {
                    if let Some(results) = parsed["results"].as_array() {
                        return results
                            .iter()
                            .filter_map(|row| {
                                let s = row["s"].as_str()?;
                                let t = row["type"].as_str()?;
                                Some(format!("{} a {}", s, t))
                            })
                            .collect();
                    }
                }
                Vec::new()
            }
            Err(_) => Vec::new(),
        }
    }
}
