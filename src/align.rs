use std::sync::Arc;
use oxigraph::io::RdfFormat;
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
    /// Detect RDF format from content (not filename).
    fn detect_content_format(content: &str) -> RdfFormat {
        let trimmed = content.trim_start();
        if trimmed.starts_with("<?xml") || trimmed.starts_with("<rdf:RDF") || trimmed.starts_with("<owl:") {
            RdfFormat::RdfXml
        } else if trimmed.starts_with("{") {
            // Could be JSON-LD but we don't support that; fall back to Turtle
            RdfFormat::Turtle
        } else {
            RdfFormat::Turtle
        }
    }

    /// Extract class IRIs and their labels, parsing the BCP-47 language tag off
    /// each literal and retaining only labels permitted by `preferred` (empty =
    /// keep all languages — multilingual mode). Untagged literals are always
    /// kept.
    fn extract_classes(store: &GraphStore, preferred: &[String]) -> Vec<ClassInfo> {
        let query = r#"
            SELECT ?class ?label ?altLabel WHERE {
                ?class a <http://www.w3.org/2002/07/owl#Class> .
                OPTIONAL { ?class <http://www.w3.org/2000/01/rdf-schema#label> ?label }
                OPTIONAL { ?class <http://www.w3.org/2004/02/skos/core#prefLabel> ?label }
                OPTIONAL { ?class <http://www.w3.org/2004/02/skos/core#altLabel> ?altLabel }
                OPTIONAL {
                    ?class <http://www.geneontology.org/formats/oboInOwl#hasRelatedSynonym> ?synNode .
                    ?synNode <http://www.w3.org/2000/01/rdf-schema#label> ?altLabel .
                }
                OPTIONAL {
                    ?class <http://www.geneontology.org/formats/oboInOwl#hasExactSynonym> ?synNode2 .
                    ?synNode2 <http://www.w3.org/2000/01/rdf-schema#label> ?altLabel .
                }
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

                for key in ["label", "altLabel"] {
                    if let Some(raw) = row[key].as_str() {
                        let label = crate::language::parse_literal(raw);
                        if !label.text.is_empty()
                            && crate::language::label_matches_policy(&label, preferred)
                            && !entry.labels.contains(&label)
                        {
                            entry.labels.push(label);
                        }
                    }
                }
            }
        }

        // If no policy-matching label was found, fall back to the IRI local
        // name as a language-neutral label.
        for info in class_map.values_mut() {
            if info.labels.is_empty() {
                info.labels
                    .push(crate::language::Label::new(local_name(&info.iri), None));
            }
        }

        // Sorted by IRI. `into_values()` yields HashMap iteration order, which Rust
        // seeds per process. That order propagates into the candidate-generation
        // loop, and because the confidence sort below is stable, equal-confidence
        // candidates keep it; the greedy 1-to-1 retain then picks a different
        // winner among ties on every run. Measured before this fix: five runs of
        // the same binary on OAEI Anatomy produced five different alignments
        // (1151/1153/1151/1152/1154 candidates, all five sets distinct).
        let mut out: Vec<ClassInfo> = class_map.into_values().collect();
        out.sort_by(|a, b| a.iri.cmp(&b.iri));
        out
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
                let na = normalize_label(&la.text);
                let nb = normalize_label(&lb.text);

                // Jaro-Winkler on full normalized strings
                let jw = jaro_winkler(&na, &nb);

                // Token Jaccard: catches partial overlaps
                // e.g. "spinal cord grey matter" vs "spinal cord" = 2/4 = 0.5
                let tokens_a: std::collections::HashSet<&str> = na.split_whitespace().collect();
                let tokens_b: std::collections::HashSet<&str> = nb.split_whitespace().collect();
                let intersection = tokens_a.intersection(&tokens_b).count() as f64;
                let union = tokens_a.union(&tokens_b).count() as f64;
                let jaccard = if union > 0.0 { intersection / union } else { 0.0 };

                // Take the best of Jaro-Winkler and token Jaccard
                let sim = jw.max(jaccard);
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

    /// Minimum embedding cosine for a pair to bypass the label-similarity
    /// pre-filter. This is the gate that admits cross-lingual matches
    /// (e.g. `Dog`↔`Chien`), which share no surface tokens and so score ~0 on
    /// label similarity but high on a multilingual embedding model.
    #[cfg(feature = "embeddings")]
    const CROSS_LINGUAL_EMBED_MIN: f64 = 0.80;

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
        // Back-compat wrapper: degenerate borderline range (low == high) reproduces old behaviour.
        self.align_with_thresholds(source, target, min_confidence, min_confidence, dry_run)
    }

    /// Two-threshold alignment with the default weighted-sum fusion (self-calibrating
    /// weights). For RRF fusion or any future strategy, call `align_with_fusion` directly.
    pub fn align_with_thresholds(
        &self,
        source: &str,
        target: Option<&str>,
        high_threshold: f64,
        low_threshold: f64,
        dry_run: bool,
    ) -> anyhow::Result<String> {
        self.align_with_fusion(source, target, high_threshold, low_threshold, dry_run, "weighted_sum")
    }

    /// Two-threshold alignment with borderline-candidate surfacing for LLM-orchestrated review,
    /// parameterised by fusion strategy.
    ///
    /// `fusion` is one of:
    /// - `"weighted_sum"` — combine per-signal scores with the self-calibrating weights
    ///   learned from `onto_align_feedback` (default; equal weights at cold start)
    /// - `"rrf"` — Reciprocal Rank Fusion at k=60 (Cormack et al., SIGIR 2009; validated for
    ///   ontology alignment by Agent-OM at VLDB 2025). No learned weights required; useful
    ///   as a cold-start baseline or alternative to the weighted sum on graphs where
    ///   feedback hasn't been collected yet.
    ///
    /// Bucket semantics are identical to `align_with_thresholds`: candidates with
    /// confidence ≥ `high_threshold` land in `auto_applied` (and are persisted unless
    /// `dry_run`); those in `[low_threshold, high_threshold)` land in `borderline` with
    /// enriched context; those below `low_threshold` are dropped.
    pub fn align_with_fusion(
        &self,
        source: &str,
        target: Option<&str>,
        high_threshold: f64,
        low_threshold: f64,
        dry_run: bool,
        fusion: &str,
    ) -> anyhow::Result<String> {
        // Clamp: degenerate range means "everything above high_threshold; nothing borderline".
        let low_threshold = low_threshold.min(high_threshold);
        // Active language policy (empty = keep all languages / multilingual).
        let preferred = crate::runtime::preferred_languages();
        // For RRF the inline weighted-sum confidence is throwaway (recomputed after
        // collection). We collect all label-prefiltered candidates and only apply
        // `low_threshold` to the FINAL fused score post-rerank.
        let inline_low_threshold = if fusion == "rrf" { f64::NEG_INFINITY } else { low_threshold };
        // Load source into a temporary graph (detect format from content)
        let source_store = GraphStore::new();
        if std::path::Path::new(source).exists() {
            source_store.load_file(source)?;
        } else {
            let format = Self::detect_content_format(source);
            source_store.load_content(source, format)?;
        }
        let source_classes = Self::extract_classes(&source_store, &preferred);

        // Load target into a temporary graph (or use the main store)
        let target_store_owned;
        let target_ref: &GraphStore;
        if let Some(target_content) = target {
            target_store_owned = GraphStore::new();
            if std::path::Path::new(target_content).exists() {
                target_store_owned.load_file(target_content)?;
            } else {
                let format = Self::detect_content_format(target_content);
                target_store_owned.load_content(target_content, format)?;
            }
            target_ref = &target_store_owned;
        } else {
            target_ref = &*self.graph;
        }
        let target_classes = Self::extract_classes(target_ref, &preferred);

        // Get learned weights (or defaults)
        let weights = self.get_learned_weights();

        // HNSW prefilter (optional, transparent): when both source and target class
        // IRIs have embeddings in the vecstore, use HNSW to compute a per-source
        // shortlist of candidate target IRIs and skip pairs not in the shortlist.
        // The default top-k is 50, which is well above the typical # of high-label-
        // similarity matches per source class but tight enough to cut the Cartesian
        // product cost on large ontologies. Gracefully degrades to full Cartesian
        // when embeddings are missing.
        #[cfg(feature = "embeddings")]
        let hnsw_prefilter: Option<std::collections::HashMap<String, std::collections::HashSet<String>>> = {
            const HNSW_TOP_K: usize = 50;
            if let Some(ref vs_arc) = self.vecstore {
                let target_iri_set: std::collections::HashSet<&str> =
                    target_classes.iter().map(|c| c.iri.as_str()).collect();
                let mut vs = vs_arc.lock().unwrap();
                let mut by_source: std::collections::HashMap<String, std::collections::HashSet<String>> =
                    std::collections::HashMap::new();
                let mut any_used = false;
                for sc in &source_classes {
                    if let Some(src_vec) = vs.load_text_vec(&sc.iri) {
                        let shortlist = vs.search_cosine_hnsw(&src_vec, HNSW_TOP_K);
                        if !shortlist.is_empty() {
                            let filtered: std::collections::HashSet<String> = shortlist
                                .into_iter()
                                .filter(|(iri, _)| target_iri_set.contains(iri.as_str()))
                                .map(|(iri, _)| iri)
                                .collect();
                            if !filtered.is_empty() {
                                by_source.insert(sc.iri.clone(), filtered);
                                any_used = true;
                            }
                        }
                    }
                }
                if any_used { Some(by_source) } else { None }
            } else {
                None
            }
        };
        #[cfg(not(feature = "embeddings"))]
        let hnsw_prefilter: Option<std::collections::HashMap<String, std::collections::HashSet<String>>> = None;

        // Compute candidates: cartesian product of source × target classes,
        // optionally pruned to the HNSW shortlist when available.
        let mut candidates: Vec<serde_json::Value> = Vec::new();

        for sc in &source_classes {
            // HNSW prefilter: if we computed a shortlist for this source, skip
            // targets outside the shortlist. If the prefilter map exists but has
            // no entry for THIS source (e.g. source had no embedding), fall back
            // to scanning all targets — preserves correctness on partially-
            // embedded inputs.
            let shortlist_for_source: Option<&std::collections::HashSet<String>> =
                hnsw_prefilter.as_ref().and_then(|m| m.get(&sc.iri));

            for tc in &target_classes {
                if let Some(shortlist) = shortlist_for_source
                    && !shortlist.contains(&tc.iri)
                {
                    continue;
                }
                // Skip self-matches (same IRI)
                if sc.iri == tc.iri {
                    continue;
                }

                let label_sim = Self::label_similarity(sc, tc);

                // Embedding similarity is computed BEFORE the label pre-filter so
                // it can vouch for cross-lingual pairs that share no surface
                // tokens (label_sim ~ 0) but embed close in a multilingual model.
                #[cfg(feature = "embeddings")]
                let embedding_sim = {
                    if let Some(ref vs) = self.vecstore {
                        let vs = vs.lock().unwrap();
                        // `load_text_vec`, not `get_text_vec`: the latter can
                        // only answer for a resident store, and would silently
                        // score every pair at 0.0 under text-vector eviction.
                        match (vs.load_text_vec(&sc.iri), vs.load_text_vec(&tc.iri)) {
                            (Some(a), Some(b)) => Self::embedding_similarity_score(&a, &b),
                            _ => 0.0,
                        }
                    } else {
                        0.0
                    }
                };

                // Pre-filter: skip pairs where label similarity is too low.
                // Raised from 0.7 to 0.75 to reduce false positives on anatomy-style
                // ontologies where many terms share token overlap (e.g., "bone" variants).
                // With embeddings, a strong embedding match bypasses the gate so
                // cross-lingual equivalences are not discarded before scoring.
                #[cfg(feature = "embeddings")]
                if label_sim < 0.75 && embedding_sim < Self::CROSS_LINGUAL_EMBED_MIN {
                    continue;
                }
                #[cfg(not(feature = "embeddings"))]
                if label_sim < 0.75 {
                    continue;
                }

                let prop_overlap = Self::property_overlap(&source_store, &sc.iri, target_ref, &tc.iri);
                let parent_ovlp = Self::parent_overlap(&source_store, &sc.iri, target_ref, &tc.iri);
                let inst_overlap = Self::instance_overlap(&source_store, &sc.iri, target_ref, &tc.iri);
                let restr_sim = Self::restriction_similarity(&source_store, &sc.iri, target_ref, &tc.iri);
                let neigh_sim = Self::neighborhood_similarity(&source_store, &sc.iri, target_ref, &tc.iri);

                #[cfg(feature = "embeddings")]
                let signals = [label_sim, prop_overlap, parent_ovlp, inst_overlap, restr_sim, neigh_sim, embedding_sim];

                #[cfg(not(feature = "embeddings"))]
                let signals = [label_sim, prop_overlap, parent_ovlp, inst_overlap, restr_sim, neigh_sim];

                // Compute confidence. When structural signals are all zero
                // (common in lightweight OWL files), fall back to the strongest
                // lexical/semantic signal with a penalty rather than the weighted
                // sum (which would be ~0.25 * label_sim and too low to pass any
                // threshold).
                let structural_sum: f64 = signals[1..6].iter().sum();
                let confidence: f64 = if structural_sum == 0.0 {
                    // No structural evidence: apply a 15% penalty to the best
                    // available signal. With embeddings this lets a strong
                    // cross-lingual embedding match still surface (typically as a
                    // borderline candidate for review).
                    #[cfg(feature = "embeddings")]
                    let best_signal = label_sim.max(embedding_sim);
                    #[cfg(not(feature = "embeddings"))]
                    let best_signal = label_sim;
                    best_signal * 0.85
                } else {
                    signals.iter().zip(weights.iter()).map(|(s, w)| s * w).sum()
                };

                // Drop pairs below the low threshold entirely (for RRF, this is
                // f64::NEG_INFINITY — we keep all label-prefiltered candidates and apply
                // the real threshold after rerank).
                if confidence < inline_low_threshold {
                    continue;
                }

                let relation = Self::classify_relation(label_sim, prop_overlap, parent_ovlp);
                let requires_review = confidence < high_threshold;

                #[allow(unused_mut)]
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
                    "requires_review": requires_review,
                }));
            }
        }

        // RRF rerank: recompute confidence from per-signal rankings, then re-derive
        // requires_review from the new confidence and apply the real low_threshold.
        if fusion == "rrf" {
            Self::rrf_rerank(&mut candidates);
            candidates.retain(|c| c["confidence"].as_f64().unwrap_or(0.0) >= low_threshold);
            for c in &mut candidates {
                let conf = c["confidence"].as_f64().unwrap_or(0.0);
                c["requires_review"] = serde_json::json!(conf < high_threshold);
            }
        }

        // Sort by confidence descending, ties broken by (source IRI, target IRI).
        //
        // Confidence alone is not a total order: on Anatomy many pairs tie exactly,
        // because the zero-structural-signal branch gives them all label_sim * 0.85.
        // `sort_by` is stable, so a confidence-only comparator leaves tied pairs in
        // input order, which makes the 1-to-1 selection below depend on how the
        // candidate list happened to be built. Breaking ties on the IRIs makes the
        // comparator total, so the result no longer depends on upstream ordering
        // even if that ordering changes again.
        candidates.sort_by(|a, b| {
            b["confidence"].as_f64().unwrap_or(0.0)
                .partial_cmp(&a["confidence"].as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a["source_iri"].as_str().unwrap_or("").cmp(b["source_iri"].as_str().unwrap_or("")))
                .then_with(|| a["target_iri"].as_str().unwrap_or("").cmp(b["target_iri"].as_str().unwrap_or("")))
        });

        // Stable matching: for each source class, keep only the top-scoring target.
        // For each target class, keep only the top-scoring source.
        // This enforces a 1-to-1 matching constraint that dramatically reduces
        // false positives on benchmarks like OAEI Anatomy.
        {
            let mut used_sources: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut used_targets: std::collections::HashSet<String> = std::collections::HashSet::new();
            candidates.retain(|c| {
                let src = c["source_iri"].as_str().unwrap_or("").to_string();
                let tgt = c["target_iri"].as_str().unwrap_or("").to_string();
                if used_sources.contains(&src) || used_targets.contains(&tgt) {
                    false
                } else {
                    used_sources.insert(src);
                    used_targets.insert(tgt);
                    true
                }
            });
        }

        // Auto-apply above high_threshold.
        let mut applied_count = 0;
        if !dry_run {
            for candidate in &mut candidates {
                let conf = candidate["confidence"].as_f64().unwrap_or(0.0);
                if conf >= high_threshold {
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

        // Enrich borderline candidates with context for LLM-orchestrated review.
        // Cheap: at most one extract_parents call per borderline candidate per side.
        let source_class_map: std::collections::HashMap<&str, &ClassInfo> =
            source_classes.iter().map(|c| (c.iri.as_str(), c)).collect();
        let target_class_map: std::collections::HashMap<&str, &ClassInfo> =
            target_classes.iter().map(|c| (c.iri.as_str(), c)).collect();
        for candidate in &mut candidates {
            if !candidate["requires_review"].as_bool().unwrap_or(false) {
                continue;
            }
            let s_iri = candidate["source_iri"].as_str().unwrap_or("").to_string();
            let t_iri = candidate["target_iri"].as_str().unwrap_or("").to_string();
            let s_labels: Vec<String> = source_class_map
                .get(s_iri.as_str())
                .map(|c| c.labels.iter().map(|l| l.tagged()).collect())
                .unwrap_or_default();
            let t_labels: Vec<String> = target_class_map
                .get(t_iri.as_str())
                .map(|c| c.labels.iter().map(|l| l.tagged()).collect())
                .unwrap_or_default();
            let s_parents = Self::extract_parents(&source_store, &s_iri);
            let t_parents = Self::extract_parents(target_ref, &t_iri);
            candidate["context"] = serde_json::json!({
                "source_labels":  s_labels,
                "target_labels":  t_labels,
                "source_parents": s_parents,
                "target_parents": t_parents,
            });
        }

        // Partition into auto-applied vs borderline buckets (back-compat: `candidates`
        // keeps the unified sorted list; new `auto_applied` / `borderline` are views).
        let auto_applied: Vec<serde_json::Value> = candidates
            .iter()
            .filter(|c| !c["requires_review"].as_bool().unwrap_or(false))
            .cloned()
            .collect();
        let borderline: Vec<serde_json::Value> = candidates
            .iter()
            .filter(|c| c["requires_review"].as_bool().unwrap_or(false))
            .cloned()
            .collect();

        let total = candidates.len();
        let borderline_count = borderline.len();

        let summary_for_review = if borderline_count > 0 {
            format!(
                "Found {} auto-applied matches and {} borderline pair(s) needing review. \
                 For each borderline pair, inspect the `context` field (labels, parents) \
                 and call `onto_align_feedback` with accepted=true|false to record your \
                 verdict — the self-calibrating weight model will learn from it.",
                auto_applied.len(),
                borderline_count
            )
        } else {
            String::new()
        };

        Ok(serde_json::json!({
            "candidates": candidates,
            "auto_applied": auto_applied,
            "borderline": borderline,
            "applied_count": applied_count,
            "total_candidates": total,
            "borderline_count": borderline_count,
            "high_threshold": high_threshold,
            "low_threshold": low_threshold,
            // Back-compat alias — older callers read "threshold".
            "threshold": high_threshold,
            "summary_for_review": summary_for_review,
        }).to_string())
    }

    /// Reciprocal Rank Fusion (Cormack, Clarke, Buettcher SIGIR 2009) at k=60.
    /// For each per-signal ranking, accumulate `1 / (k + rank)` into each candidate's
    /// RRF score. Normalise to `[0, 1]` against the max possible (a candidate that
    /// is rank-1 in every signal), then overwrite each candidate's `confidence` field.
    ///
    /// `signals` is the list of signal field-names in the candidate's `signals` object
    /// to fuse over. By design RRF is robust to signals on different scales — it only
    /// uses the per-signal ordering, not the raw scores. This is the property that
    /// makes it a sensible cold-start alternative to the learned weighted-sum.
    fn rrf_rerank(candidates: &mut [serde_json::Value]) {
        const K: f64 = 60.0;
        if candidates.is_empty() {
            return;
        }

        #[cfg(feature = "embeddings")]
        const SIGNALS: &[&str] = &[
            "label_similarity",
            "property_overlap",
            "parent_overlap",
            "instance_overlap",
            "restriction_similarity",
            "neighborhood_similarity",
            "embedding_similarity",
        ];
        #[cfg(not(feature = "embeddings"))]
        const SIGNALS: &[&str] = &[
            "label_similarity",
            "property_overlap",
            "parent_overlap",
            "instance_overlap",
            "restriction_similarity",
            "neighborhood_similarity",
        ];

        let n = candidates.len();
        let mut rrf = vec![0.0_f64; n];

        for sig in SIGNALS {
            let mut idx: Vec<usize> = (0..n).collect();
            idx.sort_by(|&a, &b| {
                let sa = candidates[a]["signals"][sig].as_f64().unwrap_or(0.0);
                let sb = candidates[b]["signals"][sig].as_f64().unwrap_or(0.0);
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            });
            for (rank_zero, &i) in idx.iter().enumerate() {
                let rank = (rank_zero + 1) as f64;
                rrf[i] += 1.0 / (K + rank);
            }
        }

        // Max possible: rank 1 in every signal -> SIGNALS.len() * 1/(K + 1)
        let max_possible = SIGNALS.len() as f64 * (1.0 / (K + 1.0));
        for (i, c) in candidates.iter_mut().enumerate() {
            let normalised = (rrf[i] / max_possible).clamp(0.0, 1.0);
            c["confidence"] = serde_json::json!((normalised * 1000.0).round() / 1000.0);
        }
    }

    /// Classify the relation type based on signal strengths.
    fn classify_relation(label_sim: f64, prop_overlap: f64, parent_overlap: f64) -> &'static str {
        // The strength of the claim must track the strength of the evidence.
        // Handing exactMatch to every mid-similarity pair produces a wall of
        // identical proposals and overstates what a bare name resemblance
        // justifies: names alone at mid similarity warrant closeMatch, and
        // only structural agreement upgrades the claim.
        if label_sim > 0.8 && prop_overlap > 0.5 {
            "owl:equivalentClass"
        } else if label_sim > 0.8 {
            "skos:exactMatch"
        } else if parent_overlap > 0.5 {
            "rdfs:subClassOf"
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

    /// Signal names in weight-vector order.
    #[cfg(not(feature = "embeddings"))]
    const SIGNAL_NAMES: [&'static str; 6] = [
        "label_similarity", "property_overlap", "parent_overlap",
        "instance_overlap", "restriction_similarity", "neighborhood_similarity",
    ];

    #[cfg(feature = "embeddings")]
    const SIGNAL_NAMES: [&'static str; 7] = [
        "label_similarity", "property_overlap", "parent_overlap",
        "instance_overlap", "restriction_similarity", "neighborhood_similarity",
        "embedding_similarity",
    ];

    /// Get learned weights from align_feedback, or defaults if not enough data.
    fn get_learned_weights(&self) -> Vec<f64> {
        let conn = self.db.conn();

        // Count only feedback rows that have signal values stored
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM align_feedback WHERE signals_json IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if count < 10 {
            return Self::DEFAULT_WEIGHTS.to_vec();
        }

        // Compute per-signal acceptance rates via likelihood ratio.
        // For each signal, compute mean value in accepted vs rejected sets.
        // Signals where accepted-mean >> rejected-mean are more discriminative.
        let n_signals = Self::DEFAULT_WEIGHTS.len();
        let mut acc_sum = vec![0.0_f64; n_signals];
        let mut acc_count = 0_u32;
        let mut rej_sum = vec![0.0_f64; n_signals];
        let mut rej_count = 0_u32;

        let mut stmt = match conn.prepare(
            "SELECT accepted, signals_json FROM align_feedback WHERE signals_json IS NOT NULL"
        ) {
            Ok(s) => s,
            Err(_) => return Self::DEFAULT_WEIGHTS.to_vec(),
        };

        let rows = match stmt.query_map([], |row| {
            let accepted: i32 = row.get(0)?;
            let json_str: String = row.get(1)?;
            Ok((accepted, json_str))
        }) {
            Ok(r) => r,
            Err(_) => return Self::DEFAULT_WEIGHTS.to_vec(),
        };

        for row in rows.flatten() {
            let (accepted, json_str) = row;
            let signals: std::collections::HashMap<String, f64> = match serde_json::from_str(&json_str) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let vals: Vec<f64> = Self::SIGNAL_NAMES
                .iter()
                .map(|name| signals.get(*name).copied().unwrap_or(0.0))
                .collect();

            if accepted != 0 {
                for (i, v) in vals.iter().enumerate() {
                    acc_sum[i] += v;
                }
                acc_count += 1;
            } else {
                for (i, v) in vals.iter().enumerate() {
                    rej_sum[i] += v;
                }
                rej_count += 1;
            }
        }

        // Need both accepted and rejected samples to learn
        if acc_count < 3 || rej_count < 3 {
            return Self::DEFAULT_WEIGHTS.to_vec();
        }

        // Compute discriminative power: ratio of accepted mean to rejected mean.
        // Clamp to avoid division by zero and extreme outliers.
        let mut raw_weights = vec![0.0_f64; n_signals];
        for i in 0..n_signals {
            let acc_mean = acc_sum[i] / acc_count as f64;
            let rej_mean = rej_sum[i] / rej_count as f64;
            let ratio = (acc_mean + 0.01) / (rej_mean + 0.01);
            raw_weights[i] = ratio.clamp(0.1, 10.0);
        }

        // Blend with defaults (70% learned, 30% prior) for stability
        for (w, default) in raw_weights.iter_mut().zip(Self::DEFAULT_WEIGHTS.iter()) {
            *w = 0.7 * *w + 0.3 * default;
        }

        // Normalise to sum to 1.0
        let total: f64 = raw_weights.iter().sum();
        if total > 0.0 {
            for w in &mut raw_weights {
                *w /= total;
            }
        } else {
            return Self::DEFAULT_WEIGHTS.to_vec();
        }

        raw_weights
    }

    /// Record user feedback on an alignment candidate.
    pub fn record_feedback(
        &self,
        source_iri: &str,
        target_iri: &str,
        predicted_relation: &str,
        accepted: bool,
        signals: Option<&std::collections::HashMap<String, f64>>,
    ) -> anyhow::Result<String> {
        let conn = self.db.conn();
        let signals_json = signals.map(|s| serde_json::to_string(s).unwrap_or_default());
        conn.execute(
            "INSERT INTO align_feedback (source_iri, target_iri, predicted_relation, accepted, signals_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![source_iri, target_iri, predicted_relation, accepted as i32, signals_json],
        )?;

        let feedback_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM align_feedback WHERE signals_json IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        Ok(serde_json::json!({
            "ok": true,
            "source_iri": source_iri,
            "target_iri": target_iri,
            "predicted_relation": predicted_relation,
            "accepted": accepted,
            "feedback_count": feedback_count,
            "weights_learning": if feedback_count >= 10 { "active" } else { "collecting" },
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
    /// Labels with their parsed language tags. Populated from rdfs:label /
    /// skos:prefLabel / skos:altLabel (and OBO synonyms), filtered by the
    /// active language policy. Falls back to a single language-neutral label
    /// derived from the IRI local name when no policy-matching label exists.
    pub labels: Vec<crate::language::Label>,
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
    // Replace underscores and hyphens with spaces, then camelCase split
    let cleaned = label.replace(['_', '-'], " ");
    let mut result = String::with_capacity(cleaned.len() + 8);
    for (i, ch) in cleaned.chars().enumerate() {
        if i > 0 && ch.is_uppercase() && !cleaned.as_bytes().get(i.wrapping_sub(1)).is_some_and(|c| c.is_ascii_uppercase()) {
            result.push(' ');
        }
        result.push(ch);
    }
    // Collapse multiple spaces and lowercase
    result.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_label() {
        assert_eq!(normalize_label("DomesticCat"), "domestic cat");
        assert_eq!(normalize_label("dog"), "dog");
        assert_eq!(normalize_label("MyFavoritePizza"), "my favorite pizza");
        assert_eq!(normalize_label("Auricularis_Superior"), "auricularis superior");
        assert_eq!(normalize_label("Spinal_Cord"), "spinal cord");
        assert_eq!(normalize_label("head-and-neck"), "head and neck");
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
            labels: vec![crate::language::Label::new("Dog", None)],
        };
        let b = ClassInfo {
            iri: "http://other.org/Canine".into(),
            labels: vec![
                crate::language::Label::new("Dog", None),
                crate::language::Label::new("Canine", None),
            ],
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
            labels: vec![crate::language::Label::new("DomesticCat", None)],
        };
        let b = ClassInfo {
            iri: "http://other.org/HouseCat".into(),
            labels: vec![crate::language::Label::new("Domestic Cat", None)],
        };
        let sim = AlignmentEngine::label_similarity(&a, &b);
        assert!(sim > 0.95, "CamelCase split should match: {}", sim);
    }

    #[test]
    fn extract_classes_parses_language_tags_and_policy() {
        let gs = GraphStore::new();
        gs.load_content(
            r#"
                @prefix owl: <http://www.w3.org/2002/07/owl#> .
                @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
                @prefix ex: <http://example.org/> .
                ex:Dog a owl:Class ; rdfs:label "Dog"@en, "Chien"@fr, "Perro"@es .
            "#,
            RdfFormat::Turtle,
        )
        .unwrap();

        // Multilingual policy (empty) keeps all three, each with its tag parsed
        // off — no more `Dog"@en` mangling.
        let all = AlignmentEngine::extract_classes(&gs, &[]);
        let dog = all.iter().find(|c| c.iri.ends_with("Dog")).unwrap();
        let texts: std::collections::HashSet<&str> =
            dog.labels.iter().map(|l| l.text.as_str()).collect();
        assert!(texts.contains("Dog"));
        assert!(texts.contains("Chien"));
        assert!(texts.contains("Perro"));
        let chien = dog.labels.iter().find(|l| l.text == "Chien").unwrap();
        assert_eq!(chien.lang.as_deref(), Some("fr"));

        // English-only policy drops the fr/es labels.
        let en = AlignmentEngine::extract_classes(&gs, &["en".to_string()]);
        let dog_en = en.iter().find(|c| c.iri.ends_with("Dog")).unwrap();
        let texts_en: Vec<&str> = dog_en.labels.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts_en, vec!["Dog"]);
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
            None,
        ).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["ok"].as_bool().unwrap());

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

    #[test]
    fn claim_strength_tracks_evidence_strength() {
        // Strong name plus structural agreement is the only equivalence.
        assert_eq!(
            AlignmentEngine::classify_relation(0.9, 0.6, 0.0),
            "owl:equivalentClass"
        );
        // A strong name alone is an exact match.
        assert_eq!(AlignmentEngine::classify_relation(0.9, 0.1, 0.0), "skos:exactMatch");
        // Shared parents without a strong name is a subclass claim.
        assert_eq!(AlignmentEngine::classify_relation(0.3, 0.1, 0.7), "rdfs:subClassOf");
        // A middling name with no structural support must not claim exactness.
        assert_eq!(AlignmentEngine::classify_relation(0.7, 0.1, 0.1), "skos:closeMatch");
        assert_eq!(AlignmentEngine::classify_relation(0.61, 0.0, 0.0), "skos:closeMatch");
    }
}
