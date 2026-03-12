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

    /// Extract property IRIs whose domain is the given class.
    fn extract_properties(store: &GraphStore, class_iri: &str) -> Vec<String> {
        let query = format!(
            r#"SELECT DISTINCT ?prop WHERE {{
                ?prop <http://www.w3.org/2000/01/rdf-schema#domain> <{class_iri}> .
            }}"#
        );
        Self::extract_iris(store, &query, "prop")
    }

    /// Extract rdfs:subClassOf parents for a class.
    fn extract_parents(store: &GraphStore, class_iri: &str) -> Vec<String> {
        let query = format!(
            r#"SELECT DISTINCT ?parent WHERE {{
                <{class_iri}> <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?parent .
                FILTER(isIRI(?parent))
            }}"#
        );
        Self::extract_iris(store, &query, "parent")
    }

    /// Extract property ranges for a class's properties.
    fn extract_ranges(store: &GraphStore, class_iri: &str) -> Vec<String> {
        let query = format!(
            r#"SELECT DISTINCT ?range WHERE {{
                ?prop <http://www.w3.org/2000/01/rdf-schema#domain> <{class_iri}> .
                ?prop <http://www.w3.org/2000/01/rdf-schema#range> ?range .
            }}"#
        );
        Self::extract_iris(store, &query, "range")
    }

    /// Helper: run a SPARQL SELECT and extract a single variable's values.
    fn extract_iris(store: &GraphStore, query: &str, var: &str) -> Vec<String> {
        let result = match store.sparql_select(query) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let parsed: serde_json::Value = match serde_json::from_str(&result) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        parsed["results"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|row| {
                row[var]
                    .as_str()
                    .map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string())
            })
            .collect()
    }

    /// Compute property signature overlap (Jaccard on domain properties + ranges).
    fn property_overlap(store_a: &GraphStore, class_a: &str, store_b: &GraphStore, class_b: &str) -> f64 {
        let props_a = Self::extract_properties(store_a, class_a);
        let props_b = Self::extract_properties(store_b, class_b);
        let ranges_a = Self::extract_ranges(store_a, class_a);
        let ranges_b = Self::extract_ranges(store_b, class_b);

        // Combine property local names + range local names for comparison
        let sig_a: Vec<String> = props_a.iter().chain(ranges_a.iter()).map(|s| local_name(s)).collect();
        let sig_b: Vec<String> = props_b.iter().chain(ranges_b.iter()).map(|s| local_name(s)).collect();

        jaccard_similarity(&sig_a, &sig_b)
    }

    /// Compute parent overlap (Jaccard on rdfs:subClassOf parents by local name).
    fn parent_overlap(store_a: &GraphStore, class_a: &str, store_b: &GraphStore, class_b: &str) -> f64 {
        let parents_a: Vec<String> = Self::extract_parents(store_a, class_a)
            .iter().map(|s| local_name(s)).collect();
        let parents_b: Vec<String> = Self::extract_parents(store_b, class_b)
            .iter().map(|s| local_name(s)).collect();
        jaccard_similarity(&parents_a, &parents_b)
    }

    /// Compute instance overlap — shared individuals typed under both classes (by local name).
    fn instance_overlap(store_a: &GraphStore, class_a: &str, store_b: &GraphStore, class_b: &str) -> f64 {
        let query_a = format!(
            r#"SELECT DISTINCT ?ind WHERE {{ ?ind a <{class_a}> . FILTER(isIRI(?ind)) }}"#
        );
        let query_b = format!(
            r#"SELECT DISTINCT ?ind WHERE {{ ?ind a <{class_b}> . FILTER(isIRI(?ind)) }}"#
        );
        let inds_a: Vec<String> = Self::extract_iris(store_a, &query_a, "ind")
            .iter().map(|s| local_name(s)).collect();
        let inds_b: Vec<String> = Self::extract_iris(store_b, &query_b, "ind")
            .iter().map(|s| local_name(s)).collect();
        jaccard_similarity(&inds_a, &inds_b)
    }

    /// Compute restriction similarity — compare owl:someValuesFrom / owl:allValuesFrom restrictions.
    fn restriction_similarity(store_a: &GraphStore, class_a: &str, store_b: &GraphStore, class_b: &str) -> f64 {
        let restriction_query = |class: &str| format!(
            r#"SELECT DISTINCT ?prop ?filler WHERE {{
                <{class}> <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?r .
                ?r a <http://www.w3.org/2002/07/owl#Restriction> .
                ?r <http://www.w3.org/2002/07/owl#onProperty> ?prop .
                {{
                    ?r <http://www.w3.org/2002/07/owl#someValuesFrom> ?filler .
                }} UNION {{
                    ?r <http://www.w3.org/2002/07/owl#allValuesFrom> ?filler .
                }}
            }}"#
        );

        let extract_restriction_sigs = |store: &GraphStore, class: &str| -> Vec<String> {
            let query = restriction_query(class);
            let result = match store.sparql_select(&query) {
                Ok(r) => r,
                Err(_) => return Vec::new(),
            };
            let parsed: serde_json::Value = match serde_json::from_str(&result) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            parsed["results"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|row| {
                    let prop = row["prop"].as_str()?;
                    let filler = row["filler"].as_str()?;
                    Some(format!("{}→{}", local_name(prop), local_name(filler)))
                })
                .collect()
        };

        let sigs_a = extract_restriction_sigs(store_a, class_a);
        let sigs_b = extract_restriction_sigs(store_b, class_b);
        jaccard_similarity(&sigs_a, &sigs_b)
    }

    /// Compute graph neighborhood similarity — 2-hop property comparison.
    fn neighborhood_similarity(store_a: &GraphStore, class_a: &str, store_b: &GraphStore, class_b: &str) -> f64 {
        let neighborhood_query = |class: &str| format!(
            r#"SELECT DISTINCT ?prop WHERE {{
                {{
                    ?prop <http://www.w3.org/2000/01/rdf-schema#domain> <{class}> .
                }} UNION {{
                    <{class}> <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?parent .
                    ?prop <http://www.w3.org/2000/01/rdf-schema#domain> ?parent .
                }} UNION {{
                    ?prop <http://www.w3.org/2000/01/rdf-schema#range> <{class}> .
                }}
            }}"#
        );

        let neigh_a: Vec<String> = Self::extract_iris(store_a, &neighborhood_query(class_a), "prop")
            .iter().map(|s| local_name(s)).collect();
        let neigh_b: Vec<String> = Self::extract_iris(store_b, &neighborhood_query(class_b), "prop")
            .iter().map(|s| local_name(s)).collect();
        jaccard_similarity(&neigh_a, &neigh_b)
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

/// Jaccard similarity between two sets of strings.
fn jaccard_similarity(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let set_a: std::collections::HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let set_b: std::collections::HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;
    if union == 0.0 { 0.0 } else { intersection / union }
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
    fn test_property_overlap_identical() {
        let a = vec!["http://ex.org/hasName".into(), "http://ex.org/hasAge".into()];
        let b = vec!["http://ex.org/hasName".into(), "http://ex.org/hasAge".into()];
        let sim = jaccard_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_property_overlap_partial() {
        let a = vec!["http://ex.org/hasName".into(), "http://ex.org/hasAge".into()];
        let b = vec!["http://ex.org/hasName".into(), "http://ex.org/hasColor".into()];
        let sim = jaccard_similarity(&a, &b);
        assert!((sim - 1.0 / 3.0).abs() < 0.001); // intersection=1, union=3
    }

    #[test]
    fn test_property_overlap_empty() {
        let a: Vec<String> = vec![];
        let b: Vec<String> = vec![];
        let sim = jaccard_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 0.001);
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
