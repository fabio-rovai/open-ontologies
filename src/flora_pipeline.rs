//! End-to-end FLORA alignment pipeline (#38 follow-on).
//!
//! Wires the signal extractor to the FLORA fuzzy inference engine so the
//! caller can go from two loaded ontologies to a set of accepted
//! `(source, target)` alignment entries in a single call: load source +
//! target graphs, enumerate candidate class pairs (with a cheap shared-
//! label-token pre-filter), extract signals per candidate, run FLORA
//! adjudication, emit only the accept-verdict pairs.
//!
//! ## Signals computed
//!
//! - **label_jaccard** — token Jaccard over the local-name (the part of an
//!   IRI after `#` or last `/`). Camel-case + underscores are split into
//!   tokens. This is the textual-evidence dimension.
//!
//! - **parent_overlap** — token Jaccard over the union of tokenised parent
//!   local-names for source vs target. Structural-evidence proxy that
//!   doesn't require an alignment-to-date (works at cold start).
//!
//! - **sibling_overlap** — token Jaccard over the union of tokenised
//!   sibling local-names. Structural-evidence proxy.
//!
//! - **datatype_overlap** — token Jaccard over the union of tokenised
//!   names of datatype properties whose `rdfs:domain` is the class.
//!
//! All four signals live in `[0, 1]`. The combined FLORA inference produces
//! a verdict (`accept` / `borderline` / `reject`) which is converted to a
//! crisp `AlignmentEntry` (only accept-verdict pairs land in the result).

use crate::align_fuzzy::{adjudicate, FuzzySignals, TNorm};
use crate::eval_alignment::AlignmentEntry;
use crate::graph::GraphStore;
use serde::Serialize;
use std::collections::BTreeSet;
use std::sync::Arc;

/// Outcome of a full FLORA alignment run.
#[derive(Clone, Debug, Serialize)]
pub struct FloraAlignmentReport {
    pub source_class_count: usize,
    pub target_class_count: usize,
    pub pairs_evaluated: usize,
    pub accepts: usize,
    pub borderline: usize,
    pub rejects: usize,
    pub entries: Vec<AlignmentEntry>,
}

/// Extract the local name from an IRI: the part after the last `#` or `/`.
pub fn local_name(iri: &str) -> String {
    let bare = iri.trim_matches(|c| c == '<' || c == '>');
    if let Some(idx) = bare.rfind('#') {
        return bare[idx + 1..].to_string();
    }
    if let Some(idx) = bare.rfind('/') {
        return bare[idx + 1..].to_string();
    }
    bare.to_string()
}

/// Tokenise a local name: lowercase, split on non-alphanumeric, also split
/// camel-case boundaries (`HasAuthor` → `has`, `author`).
pub fn tokenise_name(name: &str) -> BTreeSet<String> {
    let mut tokens: BTreeSet<String> = BTreeSet::new();
    let mut current = String::new();
    let chars: Vec<char> = name.chars().collect();
    for c in chars.iter().copied() {
        if c.is_alphanumeric() {
            // Detect camel-case boundary: upper after lower OR upper before lower.
            if !current.is_empty()
                && c.is_uppercase()
                && let Some(prev) = current.chars().last()
                && prev.is_lowercase()
            {
                tokens.insert(std::mem::take(&mut current).to_ascii_lowercase());
            }
            current.push(c);
        } else if !current.is_empty() {
            tokens.insert(std::mem::take(&mut current).to_ascii_lowercase());
        }
    }
    if !current.is_empty() {
        tokens.insert(current.to_ascii_lowercase());
    }
    // Strip 1-char tokens (often punctuation residue).
    tokens.retain(|t| t.len() > 1);
    tokens
}

/// Jaccard similarity between two token sets.
fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

/// Pull parent IRIs (rdfs:subClassOf) for a class.
fn parents(graph: &Arc<GraphStore>, class_iri: &str) -> Vec<String> {
    let q = format!(
        "SELECT ?p WHERE {{ <{}> <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?p }} LIMIT 50",
        class_iri
    );
    sparql_iri_list(graph, &q, "p")
}

/// Pull sibling IRIs (classes sharing a parent with the given class).
fn siblings(graph: &Arc<GraphStore>, class_iri: &str) -> Vec<String> {
    let q = format!(
        r#"SELECT DISTINCT ?s WHERE {{
            <{}> <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?p .
            ?s <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?p .
            FILTER(?s != <{}>)
        }} LIMIT 100"#,
        class_iri, class_iri
    );
    sparql_iri_list(graph, &q, "s")
}

/// Pull datatype properties whose rdfs:domain is the class.
fn datatype_properties(graph: &Arc<GraphStore>, class_iri: &str) -> Vec<String> {
    let q = format!(
        r#"SELECT ?p WHERE {{
            ?p <http://www.w3.org/2000/01/rdf-schema#domain> <{}> .
            ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> .
        }} LIMIT 100"#,
        class_iri
    );
    let mut out = sparql_iri_list(graph, &q, "p");
    // Also include all rdfs:domain properties (some ontologies don't type as DatatypeProperty).
    let q2 = format!(
        "SELECT ?p WHERE {{ ?p <http://www.w3.org/2000/01/rdf-schema#domain> <{}> }} LIMIT 100",
        class_iri
    );
    for p in sparql_iri_list(graph, &q2, "p") {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

/// Pull every owl:Class IRI in the graph.
pub fn list_classes(graph: &Arc<GraphStore>) -> Vec<String> {
    let q = "SELECT DISTINCT ?c WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> } LIMIT 5000";
    sparql_iri_list(graph, q, "c")
}

fn sparql_iri_list(graph: &Arc<GraphStore>, q: &str, var: &str) -> Vec<String> {
    let js = match graph.sparql_select(q) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let v: serde_json::Value = serde_json::from_str(&js).unwrap_or(serde_json::Value::Null);
    let rows = v["results"].as_array().cloned().unwrap_or_default();
    rows.iter()
        .filter_map(|row| {
            row[var]
                .as_str()
                .map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string())
        })
        .filter(|s| !s.starts_with("_:")) // skip blank nodes
        .collect()
}

/// Compute the four FLORA signals for a candidate pair.
pub fn compute_signals(
    source: &Arc<GraphStore>,
    target: &Arc<GraphStore>,
    source_iri: &str,
    target_iri: &str,
) -> FuzzySignals {
    let src_label = tokenise_name(&local_name(source_iri));
    let tgt_label = tokenise_name(&local_name(target_iri));
    let label_jaccard = jaccard(&src_label, &tgt_label);

    let src_parents: BTreeSet<String> = parents(source, source_iri)
        .iter()
        .flat_map(|p| tokenise_name(&local_name(p)))
        .collect();
    let tgt_parents: BTreeSet<String> = parents(target, target_iri)
        .iter()
        .flat_map(|p| tokenise_name(&local_name(p)))
        .collect();
    let parent_overlap = jaccard(&src_parents, &tgt_parents);

    let src_siblings: BTreeSet<String> = siblings(source, source_iri)
        .iter()
        .flat_map(|s| tokenise_name(&local_name(s)))
        .collect();
    let tgt_siblings: BTreeSet<String> = siblings(target, target_iri)
        .iter()
        .flat_map(|s| tokenise_name(&local_name(s)))
        .collect();
    let sibling_overlap = jaccard(&src_siblings, &tgt_siblings);

    let src_dprops: BTreeSet<String> = datatype_properties(source, source_iri)
        .iter()
        .flat_map(|p| tokenise_name(&local_name(p)))
        .collect();
    let tgt_dprops: BTreeSet<String> = datatype_properties(target, target_iri)
        .iter()
        .flat_map(|p| tokenise_name(&local_name(p)))
        .collect();
    let datatype_overlap = jaccard(&src_dprops, &tgt_dprops);

    FuzzySignals {
        label_jaccard,
        parent_overlap,
        sibling_overlap,
        datatype_overlap,
    }
}

/// Run the full FLORA pipeline across all class-pair candidates between
/// two ontologies. Filters candidates by a cheap pre-check (at least one
/// shared label token) before invoking the expensive structural queries.
pub fn align_with_flora(
    source: &Arc<GraphStore>,
    target: &Arc<GraphStore>,
    low_threshold: f64,
    high_threshold: f64,
) -> FloraAlignmentReport {
    let src_classes = list_classes(source);
    let tgt_classes = list_classes(target);

    let mut entries: Vec<AlignmentEntry> = Vec::new();
    let mut accepts = 0usize;
    let mut borderline = 0usize;
    let mut rejects = 0usize;
    let mut pairs_evaluated = 0usize;

    // Pre-compute tokenised source labels for the cheap filter.
    let src_tokens: Vec<BTreeSet<String>> = src_classes
        .iter()
        .map(|c| tokenise_name(&local_name(c)))
        .collect();
    let tgt_tokens: Vec<BTreeSet<String>> = tgt_classes
        .iter()
        .map(|c| tokenise_name(&local_name(c)))
        .collect();

    for (i, src) in src_classes.iter().enumerate() {
        for (j, tgt) in tgt_classes.iter().enumerate() {
            // Cheap filter: skip pairs with zero shared label tokens.
            if src_tokens[i].intersection(&tgt_tokens[j]).next().is_none() {
                continue;
            }
            pairs_evaluated += 1;
            let signals = compute_signals(source, target, src, tgt);
            let decision = adjudicate(&signals, TNorm::Min, low_threshold, high_threshold);
            match decision.verdict.as_str() {
                "accept" => {
                    accepts += 1;
                    entries.push(AlignmentEntry {
                        source: src.clone(),
                        target: tgt.clone(),
                        relation: "equivalent".to_string(),
                    });
                }
                "borderline" => borderline += 1,
                _ => rejects += 1,
            }
        }
    }

    FloraAlignmentReport {
        source_class_count: src_classes.len(),
        target_class_count: tgt_classes.len(),
        pairs_evaluated,
        accepts,
        borderline,
        rejects,
        entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_name_strips_hash_prefix() {
        assert_eq!(local_name("http://cmt#Author"), "Author");
        assert_eq!(local_name("http://www.w3.org/2002/07/owl#Class"), "Class");
        assert_eq!(local_name("http://ex.org/Cat"), "Cat");
    }

    #[test]
    fn tokenise_splits_camel_case_and_underscores() {
        let t = tokenise_name("Regular_author");
        assert!(t.contains("regular") && t.contains("author"));
        let t = tokenise_name("HasReviewedPaper");
        assert!(t.contains("has") && t.contains("reviewed") && t.contains("paper"));
    }

    #[test]
    fn tokenise_drops_single_char_tokens() {
        let t = tokenise_name("a_real_name");
        assert!(!t.contains("a"));
        assert!(t.contains("real") && t.contains("name"));
    }

    #[test]
    fn compute_signals_returns_high_label_for_identical_local_names() {
        let g1 = Arc::new(GraphStore::new());
        let g2 = Arc::new(GraphStore::new());
        g1.load_turtle(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix ex: <http://a/> . ex:Author a owl:Class .",
            None,
        )
        .unwrap();
        g2.load_turtle(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix ex: <http://b/> . ex:Author a owl:Class .",
            None,
        )
        .unwrap();
        let s = compute_signals(&g1, &g2, "http://a/Author", "http://b/Author");
        assert!((s.label_jaccard - 1.0).abs() < 1e-9);
    }

    #[test]
    fn align_with_flora_matches_identical_class_names_when_structural_context_exists() {
        // FLORA correctly demands SOME structural evidence beyond label
        // equality — a bare class with no parents/siblings/datatypes lands
        // in borderline, not accept. So set up classes with shared parent
        // structure to make the test realistic.
        let g1 = Arc::new(GraphStore::new());
        let g2 = Arc::new(GraphStore::new());
        g1.load_turtle(
            r#"@prefix owl: <http://www.w3.org/2002/07/owl#> .
               @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
               @prefix ex: <http://a/> .
               ex:Agent a owl:Class .
               ex:Author a owl:Class ; rdfs:subClassOf ex:Agent .
               ex:Reviewer a owl:Class ; rdfs:subClassOf ex:Agent .
               ex:Paper a owl:Class .
            "#,
            None,
        ).unwrap();
        g2.load_turtle(
            r#"@prefix owl: <http://www.w3.org/2002/07/owl#> .
               @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
               @prefix ex: <http://b/> .
               ex:Agent a owl:Class .
               ex:Author a owl:Class ; rdfs:subClassOf ex:Agent .
               ex:Reviewer a owl:Class ; rdfs:subClassOf ex:Agent .
               ex:Paper a owl:Class .
               ex:Conference a owl:Class .
            "#,
            None,
        ).unwrap();
        let report = align_with_flora(&g1, &g2, 0.4, 0.65);
        assert_eq!(report.source_class_count, 4);
        assert_eq!(report.target_class_count, 5);
        // Author↔Author and Reviewer↔Reviewer have shared parent context.
        let has_author = report.entries.iter().any(|e|
            e.source.ends_with("Author") && e.target.ends_with("Author"));
        assert!(has_author,
            "Author↔Author should accept with parent context; got: {:?}",
            report.entries);
    }

    #[test]
    fn align_with_flora_bare_classes_yield_borderline_not_accept() {
        // The honest behaviour: identical labels but zero structural
        // evidence → borderline. FLORA refuses to accept on label alone.
        let g1 = Arc::new(GraphStore::new());
        let g2 = Arc::new(GraphStore::new());
        g1.load_turtle(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix ex: <http://a/> . ex:Author a owl:Class .",
            None,
        ).unwrap();
        g2.load_turtle(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix ex: <http://b/> . ex:Author a owl:Class .",
            None,
        ).unwrap();
        let report = align_with_flora(&g1, &g2, 0.4, 0.65);
        // Zero accepts, ≥1 borderline.
        assert_eq!(report.accepts, 0,
            "bare-class match should not accept; got entries: {:?}",
            report.entries);
        assert!(report.borderline >= 1,
            "expected borderline for identical-label bare classes; got {:?}",
            report);
    }

    #[test]
    fn list_classes_returns_owl_class_instances() {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix ex: <http://a/> . ex:A a owl:Class . ex:B a owl:Class . ex:thing a ex:A .",
            None,
        )
        .unwrap();
        let cs = list_classes(&g);
        assert_eq!(cs.len(), 2);
        assert!(cs.iter().any(|c| c == "http://a/A"));
        assert!(cs.iter().any(|c| c == "http://a/B"));
    }
}
