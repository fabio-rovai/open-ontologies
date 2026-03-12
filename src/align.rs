use std::sync::Arc;
use crate::drift::jaro_winkler;
use crate::graph::GraphStore;
use crate::state::StateDb;

/// Schema alignment engine — detects equivalentClass/exactMatch/subClassOf
/// candidates between two ontologies using weighted signals.
pub struct AlignmentEngine {
    db: StateDb,
    graph: Arc<GraphStore>,
}

impl AlignmentEngine {
    pub fn new(db: StateDb, graph: Arc<GraphStore>) -> Self {
        Self { db, graph }
    }

    /// Extract class IRIs and their labels from a temporary graph via SPARQL.
    fn extract_classes(store: &GraphStore) -> Vec<ClassInfo> {
        let query = r#"
            SELECT ?class ?label ?altLabel WHERE {
                ?class a <http://www.w3.org/2002/07/owl#Class> .
                OPTIONAL { ?class <http://www.w3.org/2000/01/rdf-schema#label> ?label }
                OPTIONAL { ?class <http://www.w3.org/2004/02/skos/core#prefLabel> ?label }
                OPTIONAL { ?class <http://www.w3.org/2004/02/skos/core#altLabel> ?altLabel }
            }
        "#;

        let result = match store.sparql_select(query) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        let parsed: serde_json::Value = match serde_json::from_str(&result) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        let mut class_map: std::collections::HashMap<String, ClassInfo> =
            std::collections::HashMap::new();

        if let Some(rows) = parsed["results"].as_array() {
            for row in rows {
                let iri = match row["class"].as_str() {
                    Some(s) => s.trim_matches(|c| c == '<' || c == '>').to_string(),
                    None => continue,
                };

                let entry = class_map.entry(iri.clone()).or_insert_with(|| ClassInfo {
                    iri: iri.clone(),
                    labels: Vec::new(),
                });

                if let Some(label) = row["label"].as_str() {
                    let l = label.trim_matches('"').to_string();
                    if !entry.labels.contains(&l) {
                        entry.labels.push(l);
                    }
                }
                if let Some(alt) = row["altLabel"].as_str() {
                    let a = alt.trim_matches('"').to_string();
                    if !entry.labels.contains(&a) {
                        entry.labels.push(a);
                    }
                }
            }
        }

        // If no label found, use IRI local name
        for info in class_map.values_mut() {
            if info.labels.is_empty() {
                info.labels.push(local_name(&info.iri));
            }
        }

        class_map.into_values().collect()
    }

    /// Compute label similarity between two classes (best match across all label variants).
    fn label_similarity(a: &ClassInfo, b: &ClassInfo) -> f64 {
        let mut best = 0.0f64;
        for la in &a.labels {
            for lb in &b.labels {
                let sim = jaro_winkler(
                    &normalize_label(la),
                    &normalize_label(lb),
                );
                best = best.max(sim);
            }
        }
        best
    }
}

/// Metadata about a class extracted from an ontology.
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub iri: String,
    pub labels: Vec<String>,
}

/// Extract local name from an IRI (after last # or /).
fn local_name(iri: &str) -> String {
    iri.rsplit_once('#')
        .or_else(|| iri.rsplit_once('/'))
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| iri.to_string())
}

/// Normalize a label for comparison: lowercase, split camelCase, trim.
fn normalize_label(label: &str) -> String {
    // Insert space before uppercase letters (camelCase splitting)
    let mut result = String::with_capacity(label.len() + 8);
    for (i, ch) in label.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            result.push(' ');
        }
        result.push(ch);
    }
    result.to_lowercase().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_label() {
        assert_eq!(normalize_label("DomesticCat"), "domestic cat");
        assert_eq!(normalize_label("dog"), "dog");
        assert_eq!(normalize_label("MyFavoritePizza"), "my favorite pizza");
    }

    #[test]
    fn test_local_name() {
        assert_eq!(local_name("http://example.org/Dog"), "Dog");
        assert_eq!(local_name("http://example.org#Cat"), "Cat");
    }

    #[test]
    fn test_label_similarity() {
        let a = ClassInfo {
            iri: "http://ex.org/Dog".into(),
            labels: vec!["Dog".into()],
        };
        let b = ClassInfo {
            iri: "http://other.org/Canine".into(),
            labels: vec!["Dog".into(), "Canine".into()],
        };
        // Exact label match should give 1.0
        let sim = AlignmentEngine::label_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_label_similarity_camelcase() {
        let a = ClassInfo {
            iri: "http://ex.org/DomesticCat".into(),
            labels: vec!["DomesticCat".into()],
        };
        let b = ClassInfo {
            iri: "http://other.org/HouseCat".into(),
            labels: vec!["Domestic Cat".into()],
        };
        let sim = AlignmentEngine::label_similarity(&a, &b);
        assert!(sim > 0.95, "CamelCase split should match: {}", sim);
    }
}
