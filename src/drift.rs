use crate::graph::GraphStore;
use crate::state::StateDb;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Drift detection between two ontology versions with self-calibrating confidence.
pub struct DriftDetector {
    db: StateDb,
}

impl DriftDetector {
    pub fn new(db: StateDb) -> Self {
        Self { db }
    }

    /// Detect drift between two Turtle strings.
    pub fn detect(&self, v1_turtle: &str, v2_turtle: &str) -> anyhow::Result<String> {
        let store1 = Arc::new(GraphStore::new());
        let store2 = Arc::new(GraphStore::new());
        store1.load_turtle(v1_turtle, None)?;
        store2.load_turtle(v2_turtle, None)?;

        let v1_vocab = self.extract_vocabulary(&store1);
        let v2_vocab = self.extract_vocabulary(&store2);

        let v1_iris: HashSet<&str> = v1_vocab.keys().map(|s| s.as_str()).collect();
        let v2_iris: HashSet<&str> = v2_vocab.keys().map(|s| s.as_str()).collect();

        let added: Vec<String> = v2_iris.difference(&v1_iris).map(|s| s.to_string()).collect();
        let removed: Vec<String> = v1_iris.difference(&v2_iris).map(|s| s.to_string()).collect();

        // Find likely renames
        let weights = self.get_learned_weights();
        let mut likely_renames = Vec::new();

        for r in &removed {
            for a in &added {
                let signals = self.compute_signals(r, a, &v1_vocab, &v2_vocab, &store1, &store2);
                let confidence = self.score_confidence(&signals, &weights);
                if confidence > 0.3 {
                    likely_renames.push(serde_json::json!({
                        "from": r,
                        "to": a,
                        "confidence": confidence,
                        "predicted": "rename",
                        "signals": signals,
                    }));
                }
            }
        }

        // Sort by confidence descending
        likely_renames.sort_by(|a, b| {
            let ca = a["confidence"].as_f64().unwrap_or(0.0);
            let cb = b["confidence"].as_f64().unwrap_or(0.0);
            cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Drift velocity: (added + removed) / (total_v1 + total_v2)
        let total = v1_iris.len() + v2_iris.len();
        let drift_velocity = if total > 0 {
            (added.len() + removed.len()) as f64 / total as f64
        } else {
            0.0
        };

        let result = serde_json::json!({
            "added": added,
            "removed": removed,
            "likely_renames": likely_renames,
            "drift_velocity": drift_velocity,
            "v1_count": v1_iris.len(),
            "v2_count": v2_iris.len(),
        });

        Ok(result.to_string())
    }

    /// Record feedback for a rename prediction.
    #[allow(clippy::too_many_arguments)]
    pub fn record_feedback(
        &self,
        from_iri: &str,
        to_iri: &str,
        predicted: &str,
        confidence: f64,
        actual: &str,
        signal_domain_range: bool,
        signal_label_sim: f64,
        signal_hierarchy: bool,
        signal_individuals: bool,
    ) {
        let conn = self.db.conn();
        let id = format!("{}_{}", from_iri, to_iri);
        let _ = conn.execute(
            "INSERT OR REPLACE INTO drift_feedback \
             (id, from_iri, to_iri, predicted, confidence, actual, \
              signal_domain_range, signal_label_sim, signal_hierarchy, signal_individuals) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                id, from_iri, to_iri, predicted, confidence, actual,
                signal_domain_range as i32, signal_label_sim,
                signal_hierarchy as i32, signal_individuals as i32,
            ],
        );
    }

    /// Get learned weights from feedback. Returns 4 weights for: domain_range, label_sim, hierarchy, individuals.
    pub fn get_learned_weights(&self) -> Vec<f64> {
        let conn = self.db.conn();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM drift_feedback", [], |r| r.get(0))
            .unwrap_or(0);

        if count < 10 {
            // Not enough data — use equal weights
            return vec![0.25, 0.25, 0.25, 0.25];
        }

        // Simple weight learning: for each signal, compute correlation with correct predictions
        let mut stmt = conn
            .prepare(
                "SELECT signal_domain_range, signal_label_sim, signal_hierarchy, signal_individuals, \
                 CASE WHEN predicted = actual THEN 1.0 ELSE 0.0 END as correct \
                 FROM drift_feedback",
            )
            .unwrap();

        let rows: Vec<(f64, f64, f64, f64, f64)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i32>(0)? as f64,
                    row.get::<_, f64>(1)?,
                    row.get::<_, i32>(2)? as f64,
                    row.get::<_, i32>(3)? as f64,
                    row.get::<_, f64>(4)?,
                ))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        if rows.is_empty() {
            return vec![0.25, 0.25, 0.25, 0.25];
        }

        // Compute correlation of each signal with correctness
        let _n = rows.len() as f64;
        let mut weights = vec![0.0f64; 4];
        for row in &rows {
            weights[0] += row.0 * row.4;
            weights[1] += row.1 * row.4;
            weights[2] += row.2 * row.4;
            weights[3] += row.3 * row.4;
        }

        // Normalize
        let total: f64 = weights.iter().sum();
        if total > 0.0 {
            for w in &mut weights {
                *w /= total;
            }
        } else {
            weights = vec![0.25, 0.25, 0.25, 0.25];
        }

        weights
    }

    fn extract_vocabulary(&self, store: &GraphStore) -> HashMap<String, VocabEntry> {
        let mut vocab = HashMap::new();

        // Classes
        let class_query = "SELECT DISTINCT ?c WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> }";
        if let Ok(json) = store.sparql_select(class_query) {
            for iri in parse_iris(&json, "c") {
                vocab.entry(iri.clone()).or_insert_with(|| VocabEntry {
                    iri,
                    kind: "class".to_string(),
                    label: None,
                    domain: None,
                    range: None,
                });
            }
        }

        // Properties
        let prop_query = "SELECT DISTINCT ?p WHERE { \
            { ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> } UNION \
            { ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> } \
        }";
        if let Ok(json) = store.sparql_select(prop_query) {
            for iri in parse_iris(&json, "p") {
                vocab.entry(iri.clone()).or_insert_with(|| VocabEntry {
                    iri,
                    kind: "property".to_string(),
                    label: None,
                    domain: None,
                    range: None,
                });
            }
        }

        // Labels
        let label_query = "SELECT ?s ?l WHERE { ?s <http://www.w3.org/2000/01/rdf-schema#label> ?l }";
        if let Ok(json) = store.sparql_select(label_query)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json)
                && let Some(results) = parsed["results"].as_array() {
                    for row in results {
                        if let (Some(s), Some(l)) = (row["s"].as_str(), row["l"].as_str()) {
                            let s = s.trim_matches(|c| c == '<' || c == '>');
                            let l = l.trim_matches('"').split("^^").next().unwrap_or("").trim_matches('"');
                            if let Some(entry) = vocab.get_mut(s) {
                                entry.label = Some(l.to_string());
                            }
                        }
                    }
                }

        // Domain/Range
        let dr_query = "SELECT ?p ?d ?r WHERE { \
            OPTIONAL { ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?d } \
            OPTIONAL { ?p <http://www.w3.org/2000/01/rdf-schema#range> ?r } \
        }";
        if let Ok(json) = store.sparql_select(dr_query)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json)
                && let Some(results) = parsed["results"].as_array() {
                    for row in results {
                        if let Some(p) = row["p"].as_str() {
                            let p = p.trim_matches(|c| c == '<' || c == '>');
                            if let Some(entry) = vocab.get_mut(p) {
                                if let Some(d) = row["d"].as_str() {
                                    entry.domain = Some(d.trim_matches(|c| c == '<' || c == '>').to_string());
                                }
                                if let Some(r) = row["r"].as_str() {
                                    entry.range = Some(r.trim_matches(|c| c == '<' || c == '>').to_string());
                                }
                            }
                        }
                    }
                }

        vocab
    }

    fn compute_signals(
        &self,
        removed: &str,
        added: &str,
        v1_vocab: &HashMap<String, VocabEntry>,
        v2_vocab: &HashMap<String, VocabEntry>,
        _store1: &GraphStore,
        _store2: &GraphStore,
    ) -> serde_json::Value {
        let v1_entry = v1_vocab.get(removed);
        let v2_entry = v2_vocab.get(added);

        // Signal 1: domain/range match
        let domain_range_match = match (v1_entry, v2_entry) {
            (Some(e1), Some(e2)) => e1.domain == e2.domain && e1.range == e2.range
                && (e1.domain.is_some() || e1.range.is_some()),
            _ => false,
        };

        // Signal 2: label similarity
        let label_sim = match (
            v1_entry.and_then(|e| e.label.as_ref()),
            v2_entry.and_then(|e| e.label.as_ref()),
        ) {
            (Some(l1), Some(l2)) => jaro_winkler(l1, l2),
            _ => {
                // Fall back to IRI local name similarity
                let name1 = local_name(removed);
                let name2 = local_name(added);
                jaro_winkler(name1, name2)
            }
        };

        // Signal 3: same kind (class<->class or property<->property)
        let same_kind = match (v1_entry, v2_entry) {
            (Some(e1), Some(e2)) => e1.kind == e2.kind,
            _ => false,
        };

        serde_json::json!({
            "domain_range_match": domain_range_match,
            "label_similarity": label_sim,
            "same_kind": same_kind,
            "hierarchy_match": false,
        })
    }

    fn score_confidence(&self, signals: &serde_json::Value, weights: &[f64]) -> f64 {
        let dr = if signals["domain_range_match"].as_bool().unwrap_or(false) { 1.0 } else { 0.0 };
        let ls = signals["label_similarity"].as_f64().unwrap_or(0.0);
        let sk = if signals["same_kind"].as_bool().unwrap_or(false) { 1.0 } else { 0.0 };
        let hm = if signals["hierarchy_match"].as_bool().unwrap_or(false) { 1.0 } else { 0.0 };

        let w = if weights.len() >= 4 {
            weights
        } else {
            &[0.25, 0.25, 0.25, 0.25]
        };

        dr * w[0] + ls * w[1] + sk * w[2] + hm * w[3]
    }
}

#[allow(dead_code)]
struct VocabEntry {
    iri: String,
    kind: String,
    label: Option<String>,
    domain: Option<String>,
    range: Option<String>,
}

fn parse_iris(json: &str, var: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v["results"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|r| {
            r[var].as_str().map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string())
        })
        .collect()
}

fn local_name(iri: &str) -> &str {
    iri.rsplit_once('#')
        .or_else(|| iri.rsplit_once('/'))
        .map(|(_, name)| name)
        .unwrap_or(iri)
}

/// Jaro-Winkler string similarity (public for testing).
pub fn jaro_winkler(s1: &str, s2: &str) -> f64 {
    if s1 == s2 {
        return 1.0;
    }
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }

    let jaro = jaro_similarity(s1, s2);

    // Winkler prefix bonus
    let prefix_len = s1
        .chars()
        .zip(s2.chars())
        .take(4)
        .take_while(|(a, b)| a == b)
        .count() as f64;

    jaro + prefix_len * 0.1 * (1.0 - jaro)
}

fn jaro_similarity(s1: &str, s2: &str) -> f64 {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let s1_len = s1_chars.len();
    let s2_len = s2_chars.len();

    if s1_len == 0 && s2_len == 0 {
        return 1.0;
    }

    let match_distance = (s1_len.max(s2_len) / 2).saturating_sub(1);

    let mut s1_matched = vec![false; s1_len];
    let mut s2_matched = vec![false; s2_len];

    let mut matches = 0.0;
    let mut transpositions = 0.0;

    for i in 0..s1_len {
        let start = i.saturating_sub(match_distance);
        let end = (i + match_distance + 1).min(s2_len);

        for j in start..end {
            if s2_matched[j] || s1_chars[i] != s2_chars[j] {
                continue;
            }
            s1_matched[i] = true;
            s2_matched[j] = true;
            matches += 1.0;
            break;
        }
    }

    if matches == 0.0 {
        return 0.0;
    }

    let mut k = 0;
    for i in 0..s1_len {
        if !s1_matched[i] {
            continue;
        }
        while !s2_matched[k] {
            k += 1;
        }
        if s1_chars[i] != s2_chars[k] {
            transpositions += 1.0;
        }
        k += 1;
    }

    (matches / s1_len as f64
        + matches / s2_len as f64
        + (matches - transpositions / 2.0) / matches)
        / 3.0
}
