use std::sync::Arc;
use crate::drift::jaro_winkler;
use crate::graph::GraphStore;
use crate::state::StateDb;

/// Schema alignment engine — detects equivalentClass/exactMatch/subClassOf
/// candidates between two ontologies using weighted signals.
pub struct AlignmentEngine {
    db: StateDb,
    graph: Arc<GraphStore>,
    #[cfg(feature = "embeddings")]
    vecstore: Option<Arc<std::sync::Mutex<crate::vecstore::VecStore>>>,
}

impl AlignmentEngine {
    pub fn new(db: StateDb, graph: Arc<GraphStore>) -> Self {
        Self {
            db,
            graph,
            #[cfg(feature = "embeddings")]
            vecstore: None,
        }
    }

    #[cfg(feature = "embeddings")]
    pub fn new_with_vecstore(db: StateDb, graph: Arc<GraphStore>, vecstore: Arc<std::sync::Mutex<crate::vecstore::VecStore>>) -> Self {
        Self { db, graph, vecstore: Some(vecstore) }
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

    /// Default signal weights: label, property, parent, instance, restriction, neighborhood.
    #[cfg(not(feature = "embeddings"))]
    const DEFAULT_WEIGHTS: [f64; 6] = [0.25, 0.20, 0.15, 0.15, 0.15, 0.10];

    /// Default signal weights with embedding signal: label, property, parent, instance, restriction, neighborhood, embedding.
    #[cfg(feature = "embeddings")]
    const DEFAULT_WEIGHTS: [f64; 7] = [0.20, 0.15, 0.12, 0.12, 0.12, 0.09, 0.20];

    #[cfg(feature = "embeddings")]
    /// Compute embedding similarity score using cosine similarity on text vectors.
    pub fn embedding_similarity_score(vec_a: &[f32], vec_b: &[f32]) -> f64 {
        crate::poincare::cosine_similarity(vec_a, vec_b) as f64
    }

    /// Run alignment between source and target ontologies.
    /// If `target` is None, aligns source against the loaded store (`self.graph`).
    /// If `dry_run` is true, returns candidates without inserting triples.
    pub fn align(
        &self,
        source: &str,
        target: Option<&str>,
        min_confidence: f64,
        dry_run: bool,
    ) -> anyhow::Result<String> {
        // Load source into a temporary graph
        let source_store = GraphStore::new();
        source_store.load_turtle(source, None)?;
        let source_classes = Self::extract_classes(&source_store);

        // Load target into a temporary graph (or use the main store)
        let target_store_owned;
        let target_ref: &GraphStore;
        if let Some(target_ttl) = target {
            target_store_owned = GraphStore::new();
            target_store_owned.load_turtle(target_ttl, None)?;
            target_ref = &target_store_owned;
        } else {
            target_ref = &*self.graph;
        }
        let target_classes = Self::extract_classes(target_ref);

        // Get learned weights (or defaults)
        let weights = self.get_learned_weights();

        // Compute candidates: cartesian product of source × target classes
        let mut candidates: Vec<serde_json::Value> = Vec::new();

        for sc in &source_classes {
            for tc in &target_classes {
                // Skip self-matches (same IRI)
                if sc.iri == tc.iri {
                    continue;
                }

                let label_sim = Self::label_similarity(sc, tc);
                let prop_overlap = Self::property_overlap(&source_store, &sc.iri, target_ref, &tc.iri);
                let parent_ovlp = Self::parent_overlap(&source_store, &sc.iri, target_ref, &tc.iri);
                let inst_overlap = Self::instance_overlap(&source_store, &sc.iri, target_ref, &tc.iri);
                let restr_sim = Self::restriction_similarity(&source_store, &sc.iri, target_ref, &tc.iri);
                let neigh_sim = Self::neighborhood_similarity(&source_store, &sc.iri, target_ref, &tc.iri);

                #[cfg(feature = "embeddings")]
                let embedding_sim = {
                    if let Some(ref vs) = self.vecstore {
                        let vs = vs.lock().unwrap();
                        match (vs.get_text_vec(&sc.iri), vs.get_text_vec(&tc.iri)) {
                            (Some(a), Some(b)) => Self::embedding_similarity_score(a, b),
                            _ => 0.0,
                        }
                    } else {
                        0.0
                    }
                };

                #[cfg(feature = "embeddings")]
                let signals = [label_sim, prop_overlap, parent_ovlp, inst_overlap, restr_sim, neigh_sim, embedding_sim];

                #[cfg(not(feature = "embeddings"))]
                let signals = [label_sim, prop_overlap, parent_ovlp, inst_overlap, restr_sim, neigh_sim];

                // When structural signals are all zero (no structural data to compare),
                // fall back to label similarity as the sole indicator.
                // Structural signals are indices 1..6 (excluding label and embedding).
                let structural_sum: f64 = signals[1..6].iter().sum();
                let confidence: f64 = if structural_sum == 0.0 {
                    label_sim
                } else {
                    signals.iter().zip(weights.iter()).map(|(s, w)| s * w).sum()
                };

                // Skip low-confidence pairs (below half of threshold to reduce noise)
                if confidence < min_confidence * 0.5 {
                    continue;
                }

                let relation = Self::classify_relation(label_sim, prop_overlap, parent_ovlp);

                let mut signals_json = serde_json::json!({
                    "label_similarity": (label_sim * 1000.0).round() / 1000.0,
                    "property_overlap": (prop_overlap * 1000.0).round() / 1000.0,
                    "parent_overlap": (parent_ovlp * 1000.0).round() / 1000.0,
                    "instance_overlap": (inst_overlap * 1000.0).round() / 1000.0,
                    "restriction_similarity": (restr_sim * 1000.0).round() / 1000.0,
                    "neighborhood_similarity": (neigh_sim * 1000.0).round() / 1000.0,
                });
                #[cfg(feature = "embeddings")]
                {
                    signals_json["embedding_similarity"] = serde_json::json!((embedding_sim * 1000.0).round() / 1000.0);
                }

                candidates.push(serde_json::json!({
                    "source_iri": sc.iri,
                    "target_iri": tc.iri,
                    "relation": relation,
                    "confidence": (confidence * 1000.0).round() / 1000.0,
                    "signals": signals_json,
                    "applied": false,
                }));
            }
        }

        // Sort by confidence descending
        candidates.sort_by(|a, b| {
            b["confidence"].as_f64().unwrap_or(0.0)
                .partial_cmp(&a["confidence"].as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Auto-apply above threshold
        let mut applied_count = 0;
        if !dry_run {
            for candidate in &mut candidates {
                let conf = candidate["confidence"].as_f64().unwrap_or(0.0);
                if conf >= min_confidence {
                    let source_iri = candidate["source_iri"].as_str().unwrap();
                    let target_iri = candidate["target_iri"].as_str().unwrap();
                    let relation = candidate["relation"].as_str().unwrap();

                    let triple = Self::relation_to_triple(source_iri, target_iri, relation);
                    if self.graph.load_turtle(&triple, None).is_ok() {
                        candidate["applied"] = serde_json::Value::Bool(true);
                        applied_count += 1;
                    }
                }
            }
        }

        let total = candidates.len();

        Ok(serde_json::json!({
            "candidates": candidates,
            "applied_count": applied_count,
            "total_candidates": total,
            "threshold": min_confidence,
        }).to_string())
    }

    /// Classify the relation type based on signal strengths.
    fn classify_relation(label_sim: f64, prop_overlap: f64, parent_overlap: f64) -> &'static str {
        if label_sim > 0.8 && prop_overlap > 0.5 {
            "owl:equivalentClass"
        } else if label_sim > 0.8 {
            "skos:exactMatch"
        } else if parent_overlap > 0.5 {
            "rdfs:subClassOf"
        } else if label_sim > 0.6 {
            "skos:exactMatch"
        } else {
            "skos:closeMatch"
        }
    }

    /// Generate a Turtle triple for the given relation.
    fn relation_to_triple(source: &str, target: &str, relation: &str) -> String {
        let predicate = match relation {
            "owl:equivalentClass" => "http://www.w3.org/2002/07/owl#equivalentClass",
            "skos:exactMatch" => "http://www.w3.org/2004/02/skos/core#exactMatch",
            "skos:closeMatch" => "http://www.w3.org/2004/02/skos/core#closeMatch",
            "rdfs:subClassOf" => "http://www.w3.org/2000/01/rdf-schema#subClassOf",
            _ => "http://www.w3.org/2004/02/skos/core#relatedMatch",
        };
        format!("<{}> <{}> <{}> .\n", source, predicate, target)
    }

    /// Get learned weights from align_feedback, or defaults if not enough data.
    fn get_learned_weights(&self) -> Vec<f64> {
        let conn = self.db.conn();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM align_feedback", [], |r| r.get(0))
            .unwrap_or(0);

        if count < 10 {
            return Self::DEFAULT_WEIGHTS.to_vec();
        }

        // For now, return defaults. Full learning requires storing per-signal values
        // in align_feedback, which we add in the feedback task.
        Self::DEFAULT_WEIGHTS.to_vec()
    }

    /// Record user feedback on an alignment candidate.
    pub fn record_feedback(
        &self,
        source_iri: &str,
        target_iri: &str,
        predicted_relation: &str,
        accepted: bool,
    ) -> anyhow::Result<String> {
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO align_feedback (source_iri, target_iri, predicted_relation, accepted)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![source_iri, target_iri, predicted_relation, accepted as i32],
        )?;

        Ok(serde_json::json!({
            "ok": true,
            "source_iri": source_iri,
            "target_iri": target_iri,
            "predicted_relation": predicted_relation,
            "accepted": accepted,
        }).to_string())
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
    fn test_align_identical_classes() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        let db = StateDb::open(&path).unwrap();
        let graph = Arc::new(GraphStore::new());

        let source = r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex: <http://example.org/> .
            ex:Dog a owl:Class ; rdfs:label "Dog" .
            ex:Cat a owl:Class ; rdfs:label "Cat" .
        "#;

        let target = r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix other: <http://other.org/> .
            other:Dog a owl:Class ; rdfs:label "Dog" .
            other:Feline a owl:Class ; rdfs:label "Cat" .
        "#;

        let engine = AlignmentEngine::new(db, graph);
        let result = engine.align(source, Some(target), 0.5, false).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        let candidates = parsed["candidates"].as_array().unwrap();
        assert!(candidates.len() >= 2, "Should find at least 2 candidates: {:?}", candidates);

        // Dog<->Dog should have very high confidence
        let dog_match = candidates.iter().find(|c| {
            c["source_iri"].as_str().unwrap().contains("Dog")
                && c["target_iri"].as_str().unwrap().contains("Dog")
        });
        assert!(dog_match.is_some(), "Should match Dog<->Dog");
        assert!(dog_match.unwrap()["confidence"].as_f64().unwrap() > 0.8);
    }

    #[test]
    fn test_align_auto_apply() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        let db = StateDb::open(&path).unwrap();
        let graph = Arc::new(GraphStore::new());

        let source = r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex: <http://example.org/> .
            ex:Dog a owl:Class ; rdfs:label "Dog" .
        "#;

        let target = r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix other: <http://other.org/> .
            other:Dog a owl:Class ; rdfs:label "Dog" .
        "#;

        let engine = AlignmentEngine::new(db, graph.clone());
        let result = engine.align(source, Some(target), 0.5, false).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert!(parsed["applied_count"].as_u64().unwrap() > 0);

        // Verify triples were inserted into the main graph
        let count = graph.triple_count();
        assert!(count > 0, "Auto-apply should insert triples into main graph");
    }

    #[test]
    fn test_align_dry_run() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        let db = StateDb::open(&path).unwrap();
        let graph = Arc::new(GraphStore::new());

        let source = r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex: <http://example.org/> .
            ex:Dog a owl:Class ; rdfs:label "Dog" .
        "#;

        let target = r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix other: <http://other.org/> .
            other:Dog a owl:Class ; rdfs:label "Dog" .
        "#;

        let engine = AlignmentEngine::new(db, graph.clone());
        let result = engine.align(source, Some(target), 0.5, true).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["applied_count"].as_u64().unwrap(), 0);
        assert_eq!(graph.triple_count(), 0, "Dry run should not insert triples");
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

    #[test]
    fn test_align_feedback() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        let db = StateDb::open(&path).unwrap();
        let graph = Arc::new(GraphStore::new());

        let engine = AlignmentEngine::new(db.clone(), graph);
        let result = engine.record_feedback(
            "http://ex.org/Dog",
            "http://other.org/Canine",
            "owl:equivalentClass",
            true,
        ).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["ok"].as_bool().unwrap(), true);

        // Verify it was stored
        let conn = db.conn();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM align_feedback", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[cfg(feature = "embeddings")]
    #[test]
    fn test_embedding_similarity_signal() {
        let sim = AlignmentEngine::embedding_similarity_score(
            &[0.9, 0.1, 0.0],
            &[0.85, 0.15, 0.0],
        );
        assert!(sim > 0.95, "Similar vectors should give high score: {sim}");

        let sim2 = AlignmentEngine::embedding_similarity_score(
            &[1.0, 0.0, 0.0],
            &[0.0, 0.0, 1.0],
        );
        assert!(sim2 < 0.1, "Orthogonal vectors should give low score: {sim2}");
    }
}
