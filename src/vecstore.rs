//! In-memory vector store with dual-space search (cosine + Poincaré)
//! and SQLite persistence.

use crate::hnsw_index::{CosineIndex, PoincareIndex};
use crate::poincare::{cosine_similarity, l2_normalize, poincare_distance};
use crate::state::StateDb;
use std::collections::HashMap;

#[derive(Clone)]
struct VecEntry {
    /// `None` when the store is evicting text vectors: the float32 lives only
    /// in the `embeddings` table and is loaded on demand.
    text_vec: Option<Vec<f32>>,
    /// Retained in both modes. The dimensionality is needed by callers that
    /// only want to know the shape, and an evicted entry has no vector to ask.
    text_dim: usize,
    struct_vec: Vec<f32>,
}

/// Brute-force dual-space vector store with an opt-in HNSW cosine index.
pub struct VecStore {
    db: StateDb,
    entries: HashMap<String, VecEntry>,
    /// Lazily-built HNSW index over `text_vec`s for accelerated cosine
    /// search. Invalidated on every mutation; rebuilt on first
    /// `search_cosine_hnsw` after a mutation. The existing
    /// `search_cosine` linear scan is unchanged and continues to work
    /// without HNSW.
    cosine_index: Option<CosineIndex>,
    /// Lazily-built HNSW index over `struct_vec`s for accelerated Poincaré
    /// search. Same invalidation semantics as `cosine_index`. The existing
    /// brute-force `search_poincare` is unchanged.
    poincare_index: Option<PoincareIndex>,
    /// Fingerprint of the embedding configuration producing the vectors in this
    /// store — see `crate::embed_fingerprint`. Written alongside every vector
    /// and every cached index, and compared on load.
    ///
    /// `None` means "this store was built without knowing the configuration".
    /// That is the case for unit tests, which construct a `VecStore` with no
    /// `EmbeddingsConfig` in reach; the server always sets it. When it is
    /// `None` the fingerprint checks are skipped entirely, so an unconfigured
    /// store behaves exactly as it did before this column existed.
    embeddings_fp: Option<String>,
    /// Lazily-built TurboQuant index over `text_vec`s. Unlike the two HNSW
    /// indices above it is *not* invalidated by a mutation: `upsert` and
    /// `remove` maintain it in place, which is the entire reason it exists.
    #[cfg(feature = "turbovec")]
    turbo_index: Option<crate::turbo_index::TurboCosineIndex>,
    /// When true, float32 text vectors are not retained in memory. They are
    /// written through to the `embeddings` table on upsert and loaded back on
    /// demand: a shortlist per query for the exact re-score, the whole set for
    /// paths that genuinely need every vector.
    ///
    /// Only worth turning on with the TurboQuant backend. The HNSW graph holds
    /// its own float32 copies of every point, so evicting the store's copy
    /// while querying through HNSW moves the memory rather than saving it.
    evict_text: bool,
}

impl VecStore {
    pub fn new(db: StateDb) -> Self {
        Self {
            db,
            entries: HashMap::new(),
            cosine_index: None,
            poincare_index: None,
            embeddings_fp: None,
            #[cfg(feature = "turbovec")]
            turbo_index: None,
            evict_text: false,
        }
    }

    /// Stop retaining float32 text vectors in memory.
    ///
    /// Set it before loading or upserting anything. See the `evict_text` field
    /// for what it costs and when it pays.
    pub fn with_text_vectors_evicted(mut self) -> Self {
        self.evict_text = true;
        self
    }

    /// Bytes of float32 text vector currently held in memory. Zero in eviction
    /// mode, which is the whole point of it.
    pub fn resident_text_vector_bytes(&self) -> usize {
        self.entries
            .values()
            .map(|e| e.text_vec.as_ref().map_or(0, |v| v.len() * 4))
            .sum()
    }

    /// Record which embedding configuration produced (and will produce) the
    /// vectors in this store.
    ///
    /// Set it before `load_from_db`: the load path is where a mismatch is
    /// detected, and detecting it afterwards would mean the stale vectors are
    /// already in memory and about to be searched.
    pub fn with_embeddings_fingerprint(mut self, fp: String) -> Self {
        self.embeddings_fp = Some(fp);
        self
    }

    /// The configuration fingerprint, if this store knows it.
    pub fn embeddings_fingerprint(&self) -> Option<&str> {
        self.embeddings_fp.as_deref()
    }

    /// Whether a fingerprint read back from the database describes the same
    /// configuration this store is running.
    ///
    /// Three-way on purpose, and the stored-`None` case is the interesting one:
    /// a row written before this column existed carries no answer, and treating
    /// unknown as matching would let exactly the corruption this guards against
    /// survive an upgrade. So unknown counts as a mismatch — one loud rebuild
    /// the first time a pre-upgrade database is opened.
    fn fingerprint_matches(&self, stored: Option<&str>) -> bool {
        match (self.embeddings_fp.as_deref(), stored) {
            // Store has no configuration to compare against: behave as before.
            (None, _) => true,
            (Some(current), Some(found)) => current == found,
            (Some(_), None) => false,
        }
    }

    pub fn upsert(&mut self, iri: &str, text_vec: &[f32], struct_vec: &[f32]) {
        let normalised = l2_normalize(text_vec);
        // In eviction mode the row IS the storage, so it is written before the
        // in-memory bookkeeping: a later read must never find an entry whose
        // vector was never persisted.
        if self.evict_text
            && let Err(e) = self.write_through(iri, &normalised, struct_vec)
        {
            tracing::error!("failed to persist embedding for {iri}: {e}");
        }
        self.entries.insert(iri.to_string(), VecEntry {
            text_vec: if self.evict_text { None } else { Some(normalised.clone()) },
            text_dim: normalised.len(),
            struct_vec: struct_vec.to_vec(),
        });
        // Invalidate BOTH HNSW indices — instant-distance is immutable.
        self.cosine_index = None;
        self.poincare_index = None;
        // The TurboQuant index is not immutable, so it is updated rather than
        // dropped. A failure here (a dimensionality that disagrees with the
        // index the store was built with) drops the index instead of being
        // swallowed, so the next search rebuilds from scratch and stays
        // correct rather than silently missing the new entry.
        #[cfg(feature = "turbovec")]
        if self
            .turbo_index
            .as_mut()
            .is_some_and(|idx| idx.upsert(iri, &normalised).is_err())
        {
            self.turbo_index = None;
        }
    }

    pub fn remove(&mut self, iri: &str) {
        if self.evict_text && self.entries.contains_key(iri) {
            let conn = self.db.conn();
            if let Err(e) = conn.execute("DELETE FROM embeddings WHERE iri = ?1", [iri]) {
                tracing::error!("failed to delete embedding row for {iri}: {e}");
            }
        }
        self.entries.remove(iri);
        self.cosine_index = None;
        self.poincare_index = None;
        #[cfg(feature = "turbovec")]
        if let Some(idx) = self.turbo_index.as_mut() {
            idx.remove(iri);
        }
    }

    /// Write one entry straight to the `embeddings` table. Eviction mode only:
    /// with vectors resident, `persist` does this in bulk instead.
    fn write_through(&self, iri: &str, text_vec: &[f32], struct_vec: &[f32]) -> anyhow::Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "INSERT OR REPLACE INTO embeddings (iri, text_vec, struct_vec, text_dim, struct_dim, model_fp) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                iri,
                f32_slice_to_bytes(text_vec),
                f32_slice_to_bytes(struct_vec),
                text_vec.len() as i64,
                struct_vec.len() as i64,
                self.embeddings_fp,
            ],
        )?;
        Ok(())
    }

    /// One text vector, from memory or from the row, whichever holds it.
    ///
    /// Prefer this over [`Self::get_text_vec`], which can only answer for a
    /// resident store and returns `None` under eviction.
    pub fn load_text_vec(&self, iri: &str) -> Option<Vec<f32>> {
        match self.entries.get(iri) {
            None => None,
            Some(e) => match &e.text_vec {
                Some(v) => Some(v.clone()),
                None => {
                    let conn = self.db.conn();
                    conn.query_row(
                        "SELECT text_vec FROM embeddings WHERE iri = ?1",
                        [iri],
                        |row| row.get::<_, Vec<u8>>(0),
                    )
                    .ok()
                    .map(|b| bytes_to_f32_vec(&b))
                }
            },
        }
    }

    /// A named subset of text vectors in one query. This is the hot path under
    /// eviction: the re-score needs the shortlist and nothing else, so it must
    /// not degenerate into one round trip per candidate.
    fn fetch_text_vecs(&self, iris: &[String]) -> HashMap<String, Vec<f32>> {
        if !self.evict_text {
            return iris
                .iter()
                .filter_map(|iri| {
                    self.entries
                        .get(iri)
                        .and_then(|e| e.text_vec.as_ref())
                        .map(|v| (iri.clone(), v.clone()))
                })
                .collect();
        }
        if iris.is_empty() {
            return HashMap::new();
        }
        let placeholders = std::iter::repeat_n("?", iris.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!("SELECT iri, text_vec FROM embeddings WHERE iri IN ({placeholders})");
        let conn = self.db.conn();
        let mut out = HashMap::with_capacity(iris.len());
        if let Ok(mut stmt) = conn.prepare(&sql) {
            let params = rusqlite::params_from_iter(iris.iter());
            if let Ok(rows) = stmt.query_map(params, |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            }) {
                for row in rows.flatten() {
                    out.insert(row.0, bytes_to_f32_vec(&row.1));
                }
            }
        }
        out
    }

    /// Every text vector, sorted by IRI so the order is deterministic in both
    /// modes. Used by the paths that genuinely need the whole set: the exact
    /// scan, the product search, index builds, and the fingerprint.
    ///
    /// Borrowed when the vectors are resident. A scan that cloned the entire
    /// corpus per query would cost more than the arithmetic it exists to do,
    /// and would make a resident store slower than an evicted one.
    fn all_text_vecs_cow(&self) -> Vec<(&str, std::borrow::Cow<'_, [f32]>)> {
        if !self.evict_text {
            let mut out: Vec<(&str, std::borrow::Cow<'_, [f32]>)> = self
                .entries
                .iter()
                .filter_map(|(iri, e)| {
                    e.text_vec
                        .as_ref()
                        .map(|v| (iri.as_str(), std::borrow::Cow::Borrowed(v.as_slice())))
                })
                .collect();
            out.sort_by(|a, b| a.0.cmp(b.0));
            return out;
        }
        self.stream_text_vecs()
            .into_iter()
            .map(|(iri, v)| {
                let key = self
                    .entries
                    .get_key_value(&iri)
                    .map(|(k, _)| k.as_str())
                    .unwrap_or("");
                (key, std::borrow::Cow::Owned(v))
            })
            .collect()
    }

    /// Owned form, for the index builders that need to hand vectors on.
    fn all_text_vecs(&self) -> Vec<(String, Vec<f32>)> {
        self.all_text_vecs_cow()
            .into_iter()
            .map(|(iri, v)| (iri.to_string(), v.into_owned()))
            .collect()
    }

    /// The evicted arm of [`Self::all_text_vecs_cow`]: read every row back.
    fn stream_text_vecs(&self) -> Vec<(String, Vec<f32>)> {
        let conn = self.db.conn();
        let mut out = Vec::with_capacity(self.entries.len());
        if let Ok(mut stmt) =
            conn.prepare("SELECT iri, text_vec FROM embeddings ORDER BY iri ASC")
            && let Ok(rows) = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
        {
            {
                for (iri, bytes) in rows.flatten() {
                    // The table can outlive an entry that was removed from the
                    // set in a mode that did not delete rows; trust `entries`.
                    if self.entries.contains_key(&iri) {
                        out.push((iri, bytes_to_f32_vec(&bytes)));
                    }
                }
            }
        }
        out
    }

    pub fn search_cosine(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let query_norm = l2_normalize(query);
        let mut scores: Vec<(String, f32)> = self
            .all_text_vecs_cow()
            .into_iter()
            .map(|(iri, v)| {
                let sim = cosine_similarity(&query_norm, &v);
                (iri.to_string(), sim)
            })
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    /// HNSW-accelerated cosine search. Approximate top-k via the HNSW index;
    /// builds the index lazily on first call (and after any mutation).
    ///
    /// Same query/output semantics as [`Self::search_cosine`] (results sorted
    /// by cosine similarity descending, top_k truncation, same scale), but
    /// sub-linear query time once the index is warm. The trade-off vs the
    /// exact brute-force scan: approximate top-k under default HNSW params,
    /// rebuild cost on every mutation.
    ///
    /// Use this when:
    /// - The store has more than a few hundred entries
    /// - You expect many queries between mutations (`embed-once,
    ///   search-many-times`)
    /// - Approximate top-k is acceptable
    ///
    /// Otherwise stick with [`Self::search_cosine`].
    pub fn search_cosine_hnsw(&mut self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        if self.entries.is_empty() {
            return Vec::new();
        }
        if self.cosine_index.is_none() {
            // Lazy build from current entries. Vectors are already L2-normalised
            // (the upsert path guarantees that), so the HNSW index sees unit
            // vectors and the cosine distance == 1 - dot product.
            let points = self.all_text_vecs();
            self.cosine_index = Some(CosineIndex::build(points));
        }
        let query_norm = l2_normalize(query);
        match self.cosine_index.as_mut() {
            Some(idx) => idx.search(&query_norm, top_k),
            None => Vec::new(),
        }
    }

    pub fn search_poincare(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scores: Vec<(String, f32)> = self.entries.iter()
            .map(|(iri, e)| (iri.clone(), poincare_distance(query, &e.struct_vec)))
            .collect();
        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    /// HNSW-accelerated Poincaré search. Mirrors [`Self::search_cosine_hnsw`]
    /// but over the structural-embedding space (`struct_vec`) with hyperbolic
    /// distance. Builds the Poincaré index lazily on first call; rebuilds on
    /// any mutation.
    pub fn search_poincare_hnsw(&mut self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        if self.entries.is_empty() {
            return Vec::new();
        }
        if self.poincare_index.is_none() {
            let points: Vec<(String, Vec<f32>)> = self
                .entries
                .iter()
                .map(|(iri, e)| (iri.clone(), e.struct_vec.clone()))
                .collect();
            self.poincare_index = Some(PoincareIndex::build(points));
        }
        match self.poincare_index.as_mut() {
            Some(idx) => idx.search(query, top_k),
            None => Vec::new(),
        }
    }

    pub fn search_product(
        &self,
        text_query: &[f32],
        struct_query: &[f32],
        top_k: usize,
        alpha: f32,
    ) -> Vec<(String, f32)> {
        let text_norm = l2_normalize(text_query);
        let mut scores: Vec<(String, f32)> = self
            .all_text_vecs_cow()
            .into_iter()
            .filter_map(|(iri, text)| {
                let struct_vec = &self.entries.get(iri)?.struct_vec;
                let cos = cosine_similarity(&text_norm, &text);
                let poinc = poincare_distance(struct_query, struct_vec);
                let poinc_sim = 1.0 / (1.0 + poinc);
                Some((iri.to_string(), alpha * cos + (1.0 - alpha) * poinc_sim))
            })
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    /// Deterministic FNV-1a 64-bit fingerprint of the entry set. Stable across
    /// processes; used to detect when a cached HNSW index is stale because the
    /// underlying vectors have changed. Includes both keys and text-vec bytes
    /// in the hash so re-embedding the same IRI with a new vector triggers a
    /// rebuild.
    pub fn entries_fingerprint(&self) -> Vec<u8> {
        // `all_text_vecs` returns IRI-sorted pairs in both modes, and SQLite's
        // default BINARY collation orders the same way Rust's `String` does,
        // so an evicted store and a resident one hash the same stream and can
        // share a database and its index caches.
        let mut hash: u64 = 0xcbf29ce484222325;
        for (iri, vec) in self.all_text_vecs_cow() {
            for byte in iri.as_bytes() {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
            for f in vec.iter() {
                for byte in f.to_le_bytes() {
                    hash ^= byte as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                }
            }
        }
        hash.to_le_bytes().to_vec()
    }

    /// Force-rebuild the HNSW cosine index using explicit HNSW parameters.
    /// Drops any previously-built index. The new index is held in memory; call
    /// [`Self::persist_cosine_index`] to save it.
    pub fn rebuild_cosine_index(&mut self, params: crate::hnsw_index::BuildParams) {
        if self.entries.is_empty() {
            self.cosine_index = None;
            return;
        }
        let points = self.all_text_vecs();
        self.cosine_index = Some(crate::hnsw_index::CosineIndex::build_with_params(
            points, params,
        ));
    }

    /// Force-rebuild the HNSW Poincaré index using explicit HNSW parameters.
    /// Same semantics as [`Self::rebuild_cosine_index`] but for the
    /// structural-embedding space.
    pub fn rebuild_poincare_index(&mut self, params: crate::hnsw_index::BuildParams) {
        if self.entries.is_empty() {
            self.poincare_index = None;
            return;
        }
        let points: Vec<(String, Vec<f32>)> = self
            .entries
            .iter()
            .map(|(iri, e)| (iri.clone(), e.struct_vec.clone()))
            .collect();
        self.poincare_index = Some(crate::hnsw_index::PoincareIndex::build_with_params(
            points, params,
        ));
    }

    /// Persist the current HNSW cosine index to SQLite (table `hnsw_index_cache`).
    /// Builds the index first if it isn't built. Subsequent `load_cosine_index()`
    /// calls (e.g. at process startup via `load_from_db`) read it back and skip
    /// the rebuild as long as the entry fingerprint matches.
    pub fn persist_cosine_index(&mut self) -> anyhow::Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }
        if self.cosine_index.is_none() {
            let points = self.all_text_vecs();
            self.cosine_index = Some(CosineIndex::build(points));
        }
        let bytes = match self.cosine_index.as_ref() {
            Some(idx) => idx.to_bytes()?,
            None => return Ok(()),
        };
        let fp = self.entries_fingerprint();
        let count = self.entries.len() as i64;
        let conn = self.db.conn();
        conn.execute(
            "INSERT OR REPLACE INTO hnsw_index_cache (kind, entries_hash, entry_count, serialised, model_fp) \
             VALUES ('cosine', ?1, ?2, ?3, ?4)",
            rusqlite::params![fp, count, bytes, self.embeddings_fp],
        )?;
        Ok(())
    }

    /// Try to load a previously-persisted HNSW cosine index. If the stored
    /// fingerprint matches the current entries' fingerprint, the index is
    /// deserialised in-place and subsequent `search_cosine_hnsw` calls skip
    /// the rebuild. If the fingerprint mismatches (or no cache exists), this
    /// is a no-op and the next `search_cosine_hnsw` rebuilds normally.
    pub fn load_cosine_index(&mut self) -> anyhow::Result<bool> {
        let conn = self.db.conn();
        let row: Option<(Vec<u8>, Vec<u8>, Option<String>)> = conn
            .query_row(
                "SELECT entries_hash, serialised, model_fp FROM hnsw_index_cache WHERE kind = 'cosine'",
                [],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .ok();
        let (stored_hash, bytes, stored_fp) = match row {
            Some(x) => x,
            None => return Ok(false),
        };
        // Two independent checks, neither sufficient alone. `entries_hash`
        // catches a changed entry set; `model_fp` catches an unchanged entry
        // set that a different model now queries against — which
        // `entries_hash` structurally cannot see, the stored bytes being
        // identical.
        if !self.fingerprint_matches(stored_fp.as_deref()) {
            tracing::warn!(
                "cached cosine HNSW index was built under a different embedding configuration \
                 — discarding it and rebuilding. One-off cost per configuration change."
            );
            return Ok(false);
        }
        let current_hash = self.entries_fingerprint();
        if stored_hash != current_hash {
            // Stale — let the rebuild path handle it next time.
            return Ok(false);
        }
        self.cosine_index = Some(CosineIndex::from_bytes(&bytes)?);
        Ok(true)
    }

    /// Async background flush of the cosine index. Serialises the index
    /// synchronously (in-memory bincode work, typically < 100ms for ontologies
    /// under ~10k classes), then dispatches the SQLite write to a tokio
    /// `spawn_blocking` task. Returns a JoinHandle so the caller can await
    /// completion if they care; otherwise fire-and-forget is fine.
    ///
    /// Use when persisting from inside an async MCP tool handler over a
    /// large index, where the SQLite write latency would otherwise hold up
    /// the handler thread. For small indices the sync `persist_cosine_index`
    /// is just as fast.
    pub fn persist_cosine_index_async(
        &mut self,
    ) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<()>>> {
        if self.entries.is_empty() {
            return Ok(tokio::task::spawn(async { Ok::<(), anyhow::Error>(()) }));
        }
        if self.cosine_index.is_none() {
            let points = self.all_text_vecs();
            self.cosine_index = Some(CosineIndex::build(points));
        }
        let bytes = self
            .cosine_index
            .as_ref()
            .expect("cosine index just built or pre-existing")
            .to_bytes()?;
        let fp = self.entries_fingerprint();
        let count = self.entries.len() as i64;
        let db = self.db.clone();
        let model_fp = self.embeddings_fp.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let conn = db.conn();
            conn.execute(
                "INSERT OR REPLACE INTO hnsw_index_cache (kind, entries_hash, entry_count, serialised, model_fp) \
                 VALUES ('cosine', ?1, ?2, ?3, ?4)",
                rusqlite::params![fp, count, bytes, model_fp],
            )?;
            Ok::<(), anyhow::Error>(())
        });
        Ok(handle)
    }

    /// Async background flush of the Poincaré index. See
    /// [`Self::persist_cosine_index_async`] for semantics.
    pub fn persist_poincare_index_async(
        &mut self,
    ) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<()>>> {
        if self.entries.is_empty() {
            return Ok(tokio::task::spawn(async { Ok::<(), anyhow::Error>(()) }));
        }
        if self.poincare_index.is_none() {
            let points: Vec<(String, Vec<f32>)> = self
                .entries
                .iter()
                .map(|(iri, e)| (iri.clone(), e.struct_vec.clone()))
                .collect();
            self.poincare_index = Some(PoincareIndex::build(points));
        }
        let bytes = self
            .poincare_index
            .as_ref()
            .expect("poincare index just built or pre-existing")
            .to_bytes()?;
        let fp = self.entries_fingerprint();
        let count = self.entries.len() as i64;
        let db = self.db.clone();
        let model_fp = self.embeddings_fp.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let conn = db.conn();
            conn.execute(
                "INSERT OR REPLACE INTO hnsw_index_cache (kind, entries_hash, entry_count, serialised, model_fp) \
                 VALUES ('poincare', ?1, ?2, ?3, ?4)",
                rusqlite::params![fp, count, bytes, model_fp],
            )?;
            Ok::<(), anyhow::Error>(())
        });
        Ok(handle)
    }

    /// Persist the Poincaré index. Mirrors [`Self::persist_cosine_index`] but
    /// uses `kind = 'poincare'` in the cache row. Both indices use the SAME
    /// entries fingerprint (the entry set is identical; only the index over
    /// it differs) so a single fingerprint mismatch invalidates both kinds.
    pub fn persist_poincare_index(&mut self) -> anyhow::Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }
        if self.poincare_index.is_none() {
            let points: Vec<(String, Vec<f32>)> = self
                .entries
                .iter()
                .map(|(iri, e)| (iri.clone(), e.struct_vec.clone()))
                .collect();
            self.poincare_index = Some(PoincareIndex::build(points));
        }
        let bytes = match self.poincare_index.as_ref() {
            Some(idx) => idx.to_bytes()?,
            None => return Ok(()),
        };
        let fp = self.entries_fingerprint();
        let count = self.entries.len() as i64;
        let conn = self.db.conn();
        conn.execute(
            "INSERT OR REPLACE INTO hnsw_index_cache (kind, entries_hash, entry_count, serialised, model_fp) \
             VALUES ('poincare', ?1, ?2, ?3, ?4)",
            rusqlite::params![fp, count, bytes, self.embeddings_fp],
        )?;
        Ok(())
    }

    /// Try to load a persisted Poincaré index. Same fingerprint-validation as
    /// [`Self::load_cosine_index`].
    pub fn load_poincare_index(&mut self) -> anyhow::Result<bool> {
        let conn = self.db.conn();
        let row: Option<(Vec<u8>, Vec<u8>, Option<String>)> = conn
            .query_row(
                "SELECT entries_hash, serialised, model_fp FROM hnsw_index_cache WHERE kind = 'poincare'",
                [],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .ok();
        let (stored_hash, bytes, stored_fp) = match row {
            Some(x) => x,
            None => return Ok(false),
        };
        // Two independent checks, neither sufficient alone. `entries_hash`
        // catches a changed entry set; `model_fp` catches an unchanged entry
        // set that a different model now queries against — which
        // `entries_hash` structurally cannot see, the stored bytes being
        // identical.
        if !self.fingerprint_matches(stored_fp.as_deref()) {
            tracing::warn!(
                "cached poincare HNSW index was built under a different embedding configuration \
                 — discarding it and rebuilding. One-off cost per configuration change."
            );
            return Ok(false);
        }
        let current_hash = self.entries_fingerprint();
        if stored_hash != current_hash {
            return Ok(false);
        }
        self.poincare_index = Some(PoincareIndex::from_bytes(&bytes)?);
        Ok(true)
    }

    /// Exact top-k cosine search, accelerated by the TurboQuant index.
    ///
    /// The index is a candidate generator, not the answer: it returns a
    /// shortlist several times wider than `top_k` scored over 2-4 bit codes,
    /// and every candidate is then re-scored against the float32 vector the
    /// store already holds. So the result is identical to
    /// [`Self::search_cosine`] whenever the shortlist covers the true top-k,
    /// and no approximate similarity number ever reaches a caller.
    ///
    /// Builds the index on first call. Unlike [`Self::search_cosine_hnsw`],
    /// subsequent mutations do not force a rebuild.
    #[cfg(feature = "turbovec")]
    pub fn search_cosine_turbo(&mut self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        if self.entries.is_empty() || top_k == 0 {
            return Vec::new();
        }
        if self.turbo_index.is_none() {
            match self.build_turbo_index() {
                Ok(idx) => self.turbo_index = Some(idx),
                Err(e) => {
                    tracing::warn!("turbovec index build failed ({e}); falling back to the exact scan");
                    return self.search_cosine(query, top_k);
                }
            }
        }
        let query_norm = l2_normalize(query);
        let shortlist = match self.turbo_index.as_ref() {
            Some(idx) => idx.search(&query_norm, Self::shortlist_width(top_k)),
            None => return self.search_cosine(query, top_k),
        };
        // Re-score the shortlist exactly. The quantised score is discarded.
        let candidates: Vec<String> = shortlist.into_iter().map(|(iri, _quantised)| iri).collect();
        let vecs = self.fetch_text_vecs(&candidates);
        let mut rescored: Vec<(String, f32)> = candidates
            .into_iter()
            .filter_map(|iri| {
                let sim = cosine_similarity(&query_norm, vecs.get(&iri)?);
                Some((iri, sim))
            })
            .collect();
        rescored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        rescored.truncate(top_k);
        rescored
    }

    /// How many quantised candidates to pull for a requested `top_k`.
    ///
    /// Four times the request plus a floor of 32: the multiplier covers the
    /// quantisation error on large requests, the floor covers small ones,
    /// where 4 x 3 candidates would leave no margin at all.
    #[cfg(feature = "turbovec")]
    fn shortlist_width(top_k: usize) -> usize {
        (top_k.saturating_mul(4)).max(top_k.saturating_add(32))
    }

    #[cfg(feature = "turbovec")]
    fn build_turbo_index(&self) -> anyhow::Result<crate::turbo_index::TurboCosineIndex> {
        crate::turbo_index::TurboCosineIndex::build(self.all_text_vecs())
    }

    /// Number of vectors in the live TurboQuant index, or `None` when it has
    /// not been built yet.
    #[cfg(feature = "turbovec")]
    pub fn turbo_index_len(&self) -> Option<usize> {
        self.turbo_index.as_ref().map(|idx| idx.len())
    }

    /// Persist the TurboQuant index into the shared index-cache table under
    /// its own `kind`, so it coexists with the two HNSW caches rather than
    /// competing with them for the row.
    #[cfg(feature = "turbovec")]
    pub fn persist_turbo_index(&mut self) -> anyhow::Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }
        if self.turbo_index.is_none() {
            self.turbo_index = Some(self.build_turbo_index()?);
        }
        let bytes = match self.turbo_index.as_ref() {
            Some(idx) => idx.to_bytes()?,
            None => return Ok(()),
        };
        let fp = self.entries_fingerprint();
        let count = self.entries.len() as i64;
        let conn = self.db.conn();
        conn.execute(
            "INSERT OR REPLACE INTO hnsw_index_cache (kind, entries_hash, entry_count, serialised, model_fp) \
             VALUES ('turbo_cosine', ?1, ?2, ?3, ?4)",
            rusqlite::params![fp, count, bytes, self.embeddings_fp],
        )?;
        Ok(())
    }

    /// Load a persisted TurboQuant index. Returns whether one was adopted.
    ///
    /// Both guards from the HNSW load path apply unchanged: `model_fp` catches
    /// an index built under a different embedding configuration, and
    /// `entries_hash` catches an entry set that has moved on since the cache
    /// was written.
    #[cfg(feature = "turbovec")]
    pub fn load_turbo_index(&mut self) -> anyhow::Result<bool> {
        let conn = self.db.conn();
        let row: Option<(Vec<u8>, Vec<u8>, Option<String>)> = conn
            .query_row(
                "SELECT entries_hash, serialised, model_fp FROM hnsw_index_cache WHERE kind = 'turbo_cosine'",
                [],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .ok();
        let (stored_hash, bytes, stored_fp) = match row {
            Some(x) => x,
            None => return Ok(false),
        };
        if !self.fingerprint_matches(stored_fp.as_deref()) {
            tracing::warn!(
                "cached TurboQuant index was built under a different embedding configuration \
                 — discarding it and rebuilding. One-off cost per configuration change."
            );
            return Ok(false);
        }
        if stored_hash != self.entries_fingerprint() {
            return Ok(false);
        }
        self.turbo_index = Some(crate::turbo_index::TurboCosineIndex::from_bytes(&bytes)?);
        Ok(true)
    }

    pub fn persist(&self) -> anyhow::Result<()> {
        if self.evict_text {
            // Every upsert already wrote its row. Re-running the bulk path here
            // would DELETE the table and re-insert from memory, which under
            // eviction holds no text vectors at all.
            return Ok(());
        }
        let conn = self.db.conn();
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM embeddings", [])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO embeddings (iri, text_vec, struct_vec, text_dim, struct_dim, model_fp) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
            )?;
            for (iri, entry) in &self.entries {
                let text_bytes = f32_slice_to_bytes(entry.text_vec.as_deref().unwrap_or(&[]));
                let struct_bytes = f32_slice_to_bytes(&entry.struct_vec);
                stmt.execute(rusqlite::params![
                    iri,
                    text_bytes,
                    struct_bytes,
                    entry.text_dim as i64,
                    entry.struct_vec.len() as i64,
                    self.embeddings_fp,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_from_db(&mut self) -> anyhow::Result<()> {
        // Scope the connection + statement so the conn MutexGuard is dropped
        // before we call `load_cosine_index` (which re-acquires it).
        let mut rejected_known = 0usize;
        let mut rejected_unknown = 0usize;
        {
            let conn = self.db.conn();
            let mut stmt =
                conn.prepare("SELECT iri, text_vec, struct_vec, model_fp FROM embeddings")?;
            let rows = stmt.query_map([], |row| {
                let iri: String = row.get(0)?;
                let text_bytes: Vec<u8> = row.get(1)?;
                let struct_bytes: Vec<u8> = row.get(2)?;
                let model_fp: Option<String> = row.get(3)?;
                Ok((iri, text_bytes, struct_bytes, model_fp))
            })?;

            for row in rows {
                let (iri, text_bytes, struct_bytes, model_fp) = row?;
                // Drop rather than load. A vector produced by a different model
                // is not merely stale, it is meaningless against the current
                // query space — keeping it would preserve exactly the silent
                // relevance regression this column exists to end.
                if !self.fingerprint_matches(model_fp.as_deref()) {
                    if model_fp.is_none() {
                        rejected_unknown += 1;
                    } else {
                        rejected_known += 1;
                    }
                    continue;
                }
                let text_vec = bytes_to_f32_vec(&text_bytes);
                self.entries.insert(iri, VecEntry {
                    text_dim: text_vec.len(),
                    text_vec: if self.evict_text { None } else { Some(text_vec) },
                    struct_vec: bytes_to_f32_vec(&struct_bytes),
                });
            }
        }

        // Loud on purpose. A silent rebuild fixes the correctness problem and
        // leaves the operator wondering why the first query after a restart
        // took 40 seconds.
        if rejected_known > 0 {
            tracing::warn!(
                "discarded {rejected_known} embedding(s) produced by a different embedding \
                 configuration — they cannot be compared against queries from the current one. \
                 Re-embed to restore them. Current configuration: {}",
                self.embeddings_fp.as_deref().unwrap_or("<unset>")
            );
        }
        if rejected_unknown > 0 {
            tracing::warn!(
                "discarded {rejected_unknown} embedding(s) with no recorded model fingerprint \
                 (written before the fingerprint column existed). They may well have come from \
                 the current model, but nothing recorded which, and assuming they did is the \
                 failure this check exists to prevent. Re-embed once and this will not recur."
            );
        }
        // Invalidate any previously-built HNSW indices; try to load persisted
        // ones. If the persisted fingerprint matches the entries we just loaded,
        // the next `search_cosine_hnsw` / `search_poincare_hnsw` skips rebuild.
        self.cosine_index = None;
        self.poincare_index = None;
        let _ = self.load_cosine_index()?;
        let _ = self.load_poincare_index()?;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The resident text vector, if there is one.
    ///
    /// Returns `None` under eviction even for an IRI the store holds, because
    /// there is no in-memory vector to borrow. Use [`Self::load_text_vec`] on
    /// any path that must work in both modes.
    pub fn get_text_vec(&self, iri: &str) -> Option<&[f32]> {
        self.entries.get(iri).and_then(|e| e.text_vec.as_deref())
    }

    pub fn get_struct_vec(&self, iri: &str) -> Option<&[f32]> {
        self.entries.get(iri).map(|e| e.struct_vec.as_slice())
    }
}

fn f32_slice_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn bytes_to_f32_vec(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}
