//! TurboQuant-backed cosine index over (IRI -> L2-normalised text embedding).
//!
//! An alternative backend to [`crate::hnsw_index::CosineIndex`], built on
//! Google Research's TurboQuant quantizer (arXiv:2504.19874) via the
//! `turbovec` crate. It exists because the instant-distance HNSW graph is
//! immutable: every `VecStore::upsert` drops the index and the next search
//! pays a full rebuild. TurboQuant has no training phase and no graph, so an
//! add is an append and a remove is O(1).
//!
//! Two things this index is *not*:
//!
//! - It is not exact. Scores are inner products over 2-4 bit codes. Callers
//!   that surface a similarity number re-score the returned candidates
//!   against the float32 vectors they already hold; this type is a candidate
//!   generator.
//! - It is not a replacement for [`crate::hnsw_index::PoincareIndex`].
//!   TurboQuant quantises for inner product, and hyperbolic distance is not
//!   an inner product on the ambient coordinates.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use turbovec::IdMapIndex;

/// Bit width of the quantised codes. 4 is the highest TurboQuant offers and
/// the best-recall setting; 2 and 3 trade recall for memory.
pub const DEFAULT_BIT_WIDTH: usize = 4;

/// Round a dimensionality up to the next multiple of 8.
///
/// `turbovec` requires `dim % 8 == 0`. Embedding models in practice emit
/// multiples of 8 already (384, 768, 1024, 1536), but the vector store accepts
/// any width, so vectors are zero-padded to the next multiple. Padding with
/// zeros leaves every inner product unchanged.
fn padded_dim(raw_dim: usize) -> usize {
    raw_dim.div_ceil(8) * 8
}

/// TurboQuant-backed cosine index keyed by IRI.
pub struct TurboCosineIndex {
    inner: IdMapIndex,
    iri_to_id: HashMap<String, u64>,
    id_to_iri: HashMap<u64, String>,
    next_id: u64,
    /// Padded width the index was constructed with.
    dim: usize,
    /// Unpadded width callers supply.
    raw_dim: usize,
}

impl TurboCosineIndex {
    /// Build an index from an iterable of `(iri, vector)` pairs. Vectors must
    /// already be L2-normalised, as `VecStore::upsert` guarantees, so that the
    /// inner products TurboQuant scores are cosine similarities.
    pub fn build<I, S>(entries: I) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = (S, Vec<f32>)>,
        S: Into<String>,
    {
        let entries: Vec<(String, Vec<f32>)> =
            entries.into_iter().map(|(i, v)| (i.into(), v)).collect();
        let raw_dim = entries
            .first()
            .map(|(_, v)| v.len())
            .ok_or_else(|| anyhow::anyhow!("cannot build a TurboCosineIndex from zero entries"))?;
        let dim = padded_dim(raw_dim);

        let mut flat = Vec::with_capacity(entries.len() * dim);
        let mut ids = Vec::with_capacity(entries.len());
        let mut iri_to_id = HashMap::with_capacity(entries.len());
        let mut id_to_iri = HashMap::with_capacity(entries.len());
        for (next_id, (iri, vec)) in entries.into_iter().enumerate() {
            if vec.len() != raw_dim {
                anyhow::bail!(
                    "vector for {iri} has dim {}, expected {raw_dim}",
                    vec.len()
                );
            }
            let id = next_id as u64;
            flat.extend_from_slice(&vec);
            flat.resize(flat.len() + (dim - raw_dim), 0.0);
            ids.push(id);
            iri_to_id.insert(iri.clone(), id);
            id_to_iri.insert(id, iri);
        }

        let mut inner = IdMapIndex::new(dim, DEFAULT_BIT_WIDTH)
            .map_err(|e| anyhow::anyhow!("turbovec index construction failed: {e}"))?;
        inner
            .add_with_ids(&flat, &ids)
            .map_err(|e| anyhow::anyhow!("turbovec add failed: {e}"))?;

        let next_id = ids.len() as u64;
        Ok(Self {
            inner,
            iri_to_id,
            id_to_iri,
            next_id,
            dim,
            raw_dim,
        })
    }

    /// Insert or replace one `(iri, vector)` pair.
    ///
    /// This is the reason the type exists. `instant-distance` graphs are
    /// immutable, so `VecStore::upsert` has to drop the whole cosine index and
    /// pay a full rebuild on the next search. Here an insert is an append and
    /// a replace is a remove followed by an append, both against a live index.
    ///
    /// A replaced IRI gets a fresh id rather than reusing its old one: ids are
    /// never recycled, so a stale allowlist entry naming a removed id fails
    /// loudly (`SearchError::UnknownId`) instead of quietly resolving to
    /// whatever vector took its place.
    pub fn upsert(&mut self, iri: &str, vector: &[f32]) -> anyhow::Result<()> {
        if vector.len() != self.raw_dim {
            anyhow::bail!(
                "vector for {iri} has dim {}, index dim is {}",
                vector.len(),
                self.raw_dim
            );
        }
        if let Some(old_id) = self.iri_to_id.remove(iri) {
            self.inner.remove(old_id);
            self.id_to_iri.remove(&old_id);
        }

        let id = self.next_id;
        self.next_id += 1;
        let mut padded = Vec::with_capacity(self.dim);
        padded.extend_from_slice(vector);
        padded.resize(self.dim, 0.0);
        self.inner
            .add_with_ids(&padded, &[id])
            .map_err(|e| anyhow::anyhow!("turbovec add failed for {iri}: {e}"))?;
        self.iri_to_id.insert(iri.to_string(), id);
        self.id_to_iri.insert(id, iri.to_string());
        Ok(())
    }

    /// Remove one IRI. Returns whether it was present.
    pub fn remove(&mut self, iri: &str) -> bool {
        match self.iri_to_id.remove(iri) {
            Some(id) => {
                self.id_to_iri.remove(&id);
                self.inner.remove(id)
            }
            None => false,
        }
    }

    /// Number of indexed vectors.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the index holds no vectors.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Serialise the index for persistence in the `hnsw_index_cache` SQLite
    /// blob column, alongside its IRI table.
    ///
    /// `turbovec`'s own `to_bytes` carries the codes and the `u64` ids but
    /// knows nothing about IRIs, so the id table and the id counter travel
    /// with it. The counter matters: reloading without it would restart id
    /// allocation at 0 and hand a live id to a different IRI.
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let payload = PersistedIndex {
            format_version: PERSIST_FORMAT_VERSION,
            raw_dim: self.raw_dim,
            next_id: self.next_id,
            id_to_iri: self
                .id_to_iri
                .iter()
                .map(|(id, iri)| (*id, iri.clone()))
                .collect(),
            index_bytes: self.inner.to_bytes(),
        };
        Ok(bincode::serialize(&payload)?)
    }

    /// Reconstruct an index from [`Self::to_bytes`] output.
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let payload: PersistedIndex = bincode::deserialize(bytes)?;
        if payload.format_version != PERSIST_FORMAT_VERSION {
            anyhow::bail!(
                "turbovec index blob is format version {}, this build reads {PERSIST_FORMAT_VERSION}",
                payload.format_version
            );
        }
        let inner = IdMapIndex::from_bytes(&payload.index_bytes)
            .map_err(|e| anyhow::anyhow!("turbovec index blob is corrupt: {e}"))?;
        let id_to_iri: HashMap<u64, String> = payload.id_to_iri.into_iter().collect();
        let iri_to_id = id_to_iri
            .iter()
            .map(|(id, iri)| (iri.clone(), *id))
            .collect();
        Ok(Self {
            inner,
            iri_to_id,
            id_to_iri,
            next_id: payload.next_id,
            dim: padded_dim(payload.raw_dim),
            raw_dim: payload.raw_dim,
        })
    }

    /// Top-k search restricted to a set of IRIs, for `onto_align`'s
    /// candidate-shortlist pass.
    ///
    /// The restriction is honoured inside the SIMD kernel rather than by
    /// over-fetching and discarding, so a selective allowlist costs less than
    /// an unrestricted search rather than the same. IRIs the index does not
    /// hold are dropped: a caller shortlisting against a graph that has moved
    /// on should get the intersection, not an error.
    pub fn search_within(
        &self,
        query: &[f32],
        top_k: usize,
        allowed: &[String],
    ) -> anyhow::Result<Vec<(String, f32)>> {
        if self.inner.is_empty() || top_k == 0 || !self.query_is_scoreable(query) {
            return Ok(Vec::new());
        }
        let ids: Vec<u64> = allowed
            .iter()
            .filter_map(|iri| self.iri_to_id.get(iri).copied())
            .collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let q = self.pad_query(query);
        let (scores, hit_ids) = self
            .inner
            .search_with_allowlist(&q, top_k, Some(&ids))
            .map_err(|e| anyhow::anyhow!("turbovec allowlist search failed: {e}"))?;
        Ok(hit_ids
            .into_iter()
            .zip(scores)
            .filter_map(|(id, score)| self.id_to_iri.get(&id).map(|iri| (iri.clone(), score)))
            .collect())
    }

    /// Whether a query vector can be scored at all.
    ///
    /// `turbovec`'s allowlist-free `search` is the panicking form and panics
    /// on a non-finite or overflow-range coordinate. An embedding provider
    /// that returns a NaN must not be able to take the server down through
    /// this path, and a query with no representable direction has no answer
    /// worth returning, so it is rejected here and reported as no results.
    fn query_is_scoreable(&self, query: &[f32]) -> bool {
        if query.len() != self.raw_dim {
            tracing::warn!(
                "turbovec query rejected: dim {} does not match the index dim {}. \
                 Returning no results rather than scoring a truncated or padded query.",
                query.len(),
                self.raw_dim
            );
            return false;
        }
        let ok = query.iter().all(|v| v.is_finite() && v.abs() < 1e16);
        if !ok {
            tracing::warn!(
                "turbovec query rejected: a coordinate is non-finite or out of \
                 scoring range. Returning no results rather than scoring it."
            );
        }
        ok
    }

    /// Zero-pad a query to the index's padded width.
    fn pad_query(&self, query: &[f32]) -> Vec<f32> {
        let mut q = Vec::with_capacity(self.dim);
        q.extend_from_slice(query);
        q.resize(self.dim, 0.0);
        q
    }

    /// Approximate top-k cosine search. Returns `(iri, similarity)` pairs
    /// sorted by similarity descending, on the same scale as the brute-force
    /// `VecStore::search_cosine` but quantised.
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        if self.inner.is_empty() || top_k == 0 || !self.query_is_scoreable(query) {
            return Vec::new();
        }
        let q = self.pad_query(query);
        let (scores, ids) = self.inner.search(&q, top_k);
        ids.into_iter()
            .zip(scores)
            .filter_map(|(id, score)| self.id_to_iri.get(&id).map(|iri| (iri.clone(), score)))
            .collect()
    }
}

/// Bump when the on-disk layout changes so a stale blob is rejected rather
/// than misread. `turbovec` versions its own payload independently.
const PERSIST_FORMAT_VERSION: u32 = 1;

/// What [`TurboCosineIndex::to_bytes`] writes and [`TurboCosineIndex::from_bytes`] reads.
#[derive(Serialize, Deserialize)]
struct PersistedIndex {
    format_version: u32,
    raw_dim: usize,
    next_id: u64,
    id_to_iri: Vec<(u64, String)>,
    index_bytes: Vec<u8>,
}
