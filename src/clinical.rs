use crate::drift::jaro_winkler;
use crate::graph::GraphStore;

/// A single crosswalk mapping row.
#[derive(Debug, Clone)]
pub struct CrosswalkRow {
    pub source_code: String,
    pub source_system: String,
    pub target_code: String,
    pub target_system: String,
    pub relation: String,
    pub source_label: String,
    pub target_label: String,
}

/// Clinical crosswalks backed by a Parquet file.
pub struct ClinicalCrosswalks {
    rows: Vec<CrosswalkRow>,
}

impl ClinicalCrosswalks {
    /// Load crosswalk data from a Parquet file.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        use arrow::array::StringArray;
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        use std::fs::File;

        let file = File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;

        let mut rows = Vec::new();
        for batch in reader {
            let batch = batch?;
            let source_code = batch.column_by_name("source_code")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let source_system = batch.column_by_name("source_system")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let target_code = batch.column_by_name("target_code")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let target_system = batch.column_by_name("target_system")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let relation = batch.column_by_name("relation")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let source_label = batch.column_by_name("source_label")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let target_label = batch.column_by_name("target_label")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            if let (Some(sc), Some(ss), Some(tc), Some(ts), Some(rel), Some(sl), Some(tl)) =
                (source_code, source_system, target_code, target_system, relation, source_label, target_label)
            {
                for i in 0..batch.num_rows() {
                    rows.push(CrosswalkRow {
                        source_code: sc.value(i).to_string(),
                        source_system: ss.value(i).to_string(),
                        target_code: tc.value(i).to_string(),
                        target_system: ts.value(i).to_string(),
                        relation: rel.value(i).to_string(),
                        source_label: sl.value(i).to_string(),
                        target_label: tl.value(i).to_string(),
                    });
                }
            }
        }

        Ok(Self { rows })
    }

    /// Look up all mappings for a given code and system.
    pub fn lookup(&self, code: &str, system: &str) -> Vec<&CrosswalkRow> {
        self.rows
            .iter()
            .filter(|r| r.source_code == code && r.source_system == system)
            .collect()
    }

    /// Fuzzy search across all labels using Jaro-Winkler.
    pub fn search_label(&self, text: &str) -> Vec<serde_json::Value> {
        let text_lower = text.to_lowercase();
        let mut results: Vec<(f64, &CrosswalkRow)> = self
            .rows
            .iter()
            .map(|r| {
                let sl_score = jaro_winkler(&text_lower, &r.source_label.to_lowercase());
                let tl_score = jaro_winkler(&text_lower, &r.target_label.to_lowercase());
                (sl_score.max(tl_score), r)
            })
            .filter(|(score, _)| *score > 0.6)
            .collect();

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(20);

        results
            .iter()
            .map(|(score, r)| {
                serde_json::json!({
                    "source_code": r.source_code,
                    "source_system": r.source_system,
                    "target_code": r.target_code,
                    "target_system": r.target_system,
                    "source_label": r.source_label,
                    "target_label": r.target_label,
                    "similarity": score,
                })
            })
            .collect()
    }

    /// Validate clinical terms in a loaded ontology against the crosswalk data.
    pub fn validate_clinical(&self, graph: &GraphStore) -> String {
        let label_query = "SELECT ?c ?l WHERE { \
            ?c a <http://www.w3.org/2002/07/owl#Class> . \
            ?c <http://www.w3.org/2000/01/rdf-schema#label> ?l \
        }";

        let mut validated = Vec::new();
        let mut unmatched = Vec::new();

        if let Ok(json) = graph.sparql_select(label_query)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json)
                && let Some(results) = parsed["results"].as_array() {
                    for row in results {
                        if let (Some(class_iri), Some(label)) = (row["c"].as_str(), row["l"].as_str()) {
                            let label_clean = label.trim_matches('"').split("^^").next().unwrap_or("").trim_matches('"');
                            let matches = self.search_label(label_clean);
                            if !matches.is_empty() {
                                validated.push(serde_json::json!({
                                    "class": class_iri,
                                    "label": label_clean,
                                    "matches": matches.len(),
                                    "best_match": matches[0],
                                }));
                            } else {
                                unmatched.push(serde_json::json!({
                                    "class": class_iri,
                                    "label": label_clean,
                                }));
                            }
                        }
                    }
                }

        serde_json::json!({
            "validated": validated,
            "unmatched": unmatched,
            "total_classes": validated.len() + unmatched.len(),
        })
        .to_string()
    }

    /// Enrich an ontology class with a SKOS mapping triple.
    pub fn enrich(&self, graph: &GraphStore, class_iri: &str, code: &str, system: &str) -> String {
        let mappings = self.lookup(code, system);
        if mappings.is_empty() {
            return serde_json::json!({
                "ok": false,
                "message": format!("No crosswalk found for {} in {}", code, system),
            })
            .to_string();
        }

        let target = &mappings[0];
        let update = format!(
            "INSERT DATA {{ <{}> <http://www.w3.org/2004/02/skos/core#exactMatch> <urn:{}:{}> . \
             <urn:{}:{}> <http://www.w3.org/2000/01/rdf-schema#label> \"{}\" . }}",
            class_iri,
            target.target_system, target.target_code,
            target.target_system, target.target_code,
            target.target_label,
        );

        match graph.sparql_update(&update) {
            Ok(n) => serde_json::json!({
                "ok": true,
                "enriched": class_iri,
                "mapped_to": format!("{}:{}", target.target_system, target.target_code),
                "triples_added": n,
            })
            .to_string(),
            Err(e) => serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })
            .to_string(),
        }
    }
}
