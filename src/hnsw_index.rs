// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! HNSW-accelerated cosine search for the in-memory vector store.
//!
//! Wraps `instant-distance` to provide approximate nearest-neighbour search
//! over L2-normalised text-embedding vectors. Used as a strict alternative to
//! the existing brute-force linear scan in [`crate::vecstore::VecStore`]:
//!
//! - **Existing behaviour preserved.** `VecStore::search_cosine` continues to
//!   do exact O(n) linear scan.
//! - **New opt-in path.** `VecStore::search_cosine_hnsw` builds an HNSW index
//!   lazily on first call (and rebuilds whenever the store is mutated) and
//!   returns approximate top-k via the index.
//!
//! ## Why HNSW
//!
//! `instant-distance` implements the HNSW algorithm from Malkov &
//! Yashunin's "Efficient and robust approximate nearest neighbor search using
//! Hierarchical Navigable Small World graphs" (TPAMI 2020). It gives
//! sub-linear query time at the cost of a one-off index-build cost. For an
//! ontology with a few thousand classes, the linear scan dominates query
//! latency in `onto_search` and `onto_align`'s embedding-similarity signal;
//! HNSW closes that gap. The trade-off is: instant-distance's index is
//! immutable once built, so every mutation invalidates it and the next
//! search pays the rebuild. This works for the typical Open Ontologies
//! workflow (embed-once, search-many-times) but would need incremental
//! indexing for write-heavy workloads.
//!
//! ## Strategic context
//!
//! Per the May 2026 ecosystem research, no Rust knowledge-graph engine ships
//! native HNSW alongside its triple store — the de-facto stack is
//! `Neo4j + Qdrant + a Python adapter`. This module is the foundation for
//! Open Ontologies to fill that gap as a Rust-native MCP server with first-
//! class semantic-search inside the same process.
//!
//! ## What's NOT in this scaffold
//!
//! - Persistence of the built index (the index is rebuilt at process start;
//!   the underlying vectors are persisted via SQLite as today)
//! - MCP-tool surface for tuning HNSW parameters (`ef_search`,
//!   `ef_construction`)
//! - Wiring into `onto_align`'s embedding-similarity signal (currently uses
//!   `VecStore::get_text_vec` + direct cosine — still works, no regression)

use crate::poincare::poincare_distance;
use instant_distance::{Builder, HnswMap, Point, Search};
use serde::{Deserialize, Serialize};

/// Tunable HNSW build parameters. `None` means "use instant-distance default".
#[derive(Clone, Copy, Debug, Default)]
pub struct BuildParams {
    /// Size of the dynamic candidate list during graph construction. Higher
    /// values yield better recall at the cost of slower build time.
    pub ef_construction: Option<usize>,
    /// Size of the dynamic candidate list during query. Higher values yield
    /// better recall at the cost of slower search.
    pub ef_search: Option<usize>,
}

/// A point in the cosine HNSW index. Wraps an L2-normalised vector and
/// implements the `instant-distance::Point` trait using `1.0 - dot_product`
/// as the distance (cosine distance for L2-normalised vectors).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CosinePoint(pub Vec<f32>);

impl Point for CosinePoint {
    fn distance(&self, other: &Self) -> f32 {
        // L2-normalised vectors -> cosine similarity = dot product.
        // Distance = 1 - similarity, so lower distance = closer match.
        // (instant-distance ranks ascending by distance, which matches.)
        let dot: f32 = self
            .0
            .iter()
            .zip(other.0.iter())
            .map(|(a, b)| a * b)
            .sum();
        1.0 - dot
    }
}

/// A point in the Poincaré HNSW index. Wraps a vector in the Poincaré ball
/// (||v|| < 1) and implements the `instant-distance::Point` trait using the
/// hyperbolic Poincaré distance. The distance is symmetric and satisfies the
/// triangle inequality, so HNSW works correctly over it; the resulting index
/// accelerates queries against the structural-embedding space the same way
/// `CosinePoint` accelerates queries against the text-embedding space.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoincarePoint(pub Vec<f32>);

impl Point for PoincarePoint {
    fn distance(&self, other: &Self) -> f32 {
        poincare_distance(&self.0, &other.0)
    }
}

/// HNSW-backed cosine index over (IRI -> L2-normalised text embedding).
///
/// Build once via [`CosineIndex::build`]; query repeatedly via
/// [`CosineIndex::search`]. The index is immutable — any mutation to the
/// underlying vector set requires a fresh build.
pub struct CosineIndex {
    inner: HnswMap<CosinePoint, String>,
    search: Search,
}

impl CosineIndex {
    /// Build an HNSW index from an iterable of `(iri, vector)` pairs. The
    /// vectors must already be L2-normalised (as the VecStore guarantees on
    /// `upsert`).
    pub fn build<I, S>(entries: I) -> Self
    where
        I: IntoIterator<Item = (S, Vec<f32>)>,
        S: Into<String>,
    {
        Self::build_with_params(entries, BuildParams::default())
    }

    /// Build an HNSW index with explicit HNSW parameters.
    ///
    /// `ef_construction` controls the size of the dynamic candidate list during
    /// graph construction (higher = better recall, slower build).
    /// `ef_search` controls the candidate list during query (higher = better
    /// recall, slower search). The instant-distance defaults work well for
    /// ontologies up to ~10k classes; tune upward for higher-recall workloads.
    pub fn build_with_params<I, S>(entries: I, params: BuildParams) -> Self
    where
        I: IntoIterator<Item = (S, Vec<f32>)>,
        S: Into<String>,
    {
        let mut points = Vec::new();
        let mut iris = Vec::new();
        for (iri, vec) in entries {
            points.push(CosinePoint(vec));
            iris.push(iri.into());
        }
        let mut builder = Builder::default();
        if let Some(efc) = params.ef_construction {
            builder = builder.ef_construction(efc);
        }
        if let Some(efs) = params.ef_search {
            builder = builder.ef_search(efs);
        }
        let inner = builder.build(points, iris);
        Self {
            inner,
            search: Search::default(),
        }
    }

    /// Serialise the index to a byte buffer for persistence in SQLite.
    /// Round-trip via [`Self::from_bytes`] reconstructs an equivalent index
    /// without rebuilding from raw vectors.
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(&self.inner)?)
    }

    /// Reconstruct an index from a byte buffer produced by [`Self::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let inner: HnswMap<CosinePoint, String> = bincode::deserialize(bytes)?;
        Ok(Self {
            inner,
            search: Search::default(),
        })
    }

    /// Approximate top-k cosine search. Returns `(iri, similarity)` pairs
    /// sorted by similarity descending. Similarity is `1.0 - distance`, so
    /// the brute-force `search_cosine` and this method use the same scale.
    pub fn search(&mut self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let q = CosinePoint(query.to_vec());
        self.inner
            .search(&q, &mut self.search)
            .take(top_k)
            .map(|item| (item.value.clone(), 1.0 - item.distance))
            .collect()
    }
}

/// HNSW-backed Poincaré index over (IRI -> structural embedding in the
/// Poincaré ball). Parallel to [`CosineIndex`] but for the hyperbolic
/// structural-embedding space rather than the Euclidean text-embedding space.
/// Query results are returned as `(iri, distance)` pairs sorted by distance
/// ascending — matching the convention of the brute-force `search_poincare`.
pub struct PoincareIndex {
    inner: HnswMap<PoincarePoint, String>,
    search: Search,
}

impl PoincareIndex {
    /// Build a Poincaré HNSW index from an iterable of `(iri, vector)` pairs.
    /// The vectors must already lie inside the Poincaré ball (||v|| < 1) — the
    /// VecStore's structural-embedding trainer guarantees this.
    pub fn build<I, S>(entries: I) -> Self
    where
        I: IntoIterator<Item = (S, Vec<f32>)>,
        S: Into<String>,
    {
        Self::build_with_params(entries, BuildParams::default())
    }

    /// Build with explicit HNSW parameters. See [`CosineIndex::build_with_params`].
    pub fn build_with_params<I, S>(entries: I, params: BuildParams) -> Self
    where
        I: IntoIterator<Item = (S, Vec<f32>)>,
        S: Into<String>,
    {
        let mut points = Vec::new();
        let mut iris = Vec::new();
        for (iri, vec) in entries {
            points.push(PoincarePoint(vec));
            iris.push(iri.into());
        }
        let mut builder = Builder::default();
        if let Some(efc) = params.ef_construction {
            builder = builder.ef_construction(efc);
        }
        if let Some(efs) = params.ef_search {
            builder = builder.ef_search(efs);
        }
        let inner = builder.build(points, iris);
        Self {
            inner,
            search: Search::default(),
        }
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(&self.inner)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let inner: HnswMap<PoincarePoint, String> = bincode::deserialize(bytes)?;
        Ok(Self {
            inner,
            search: Search::default(),
        })
    }

    /// Approximate top-k Poincaré-distance search. Returns `(iri, distance)`
    /// pairs sorted by distance ascending — same convention as the brute-force
    /// `VecStore::search_poincare`.
    pub fn search(&mut self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let q = PoincarePoint(query.to_vec());
        self.inner
            .search(&q, &mut self.search)
            .take(top_k)
            .map(|item| (item.value.clone(), item.distance))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(v: Vec<f32>) -> Vec<f32> {
        let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if n > 0.0 { v.into_iter().map(|x| x / n).collect() } else { v }
    }

    #[test]
    fn cosine_distance_implementation_matches_one_minus_dot() {
        let a = CosinePoint(norm(vec![1.0, 0.0, 0.0]));
        let b = CosinePoint(norm(vec![0.0, 1.0, 0.0]));
        // Orthogonal unit vectors: similarity 0, distance 1.
        assert!((a.distance(&b) - 1.0).abs() < 1e-6);

        let c = CosinePoint(norm(vec![1.0, 0.0, 0.0]));
        // Identical vectors: similarity 1, distance 0.
        assert!(a.distance(&c).abs() < 1e-6);
    }

    #[test]
    fn build_and_search_returns_nearest_first() {
        let cat_vec    = norm(vec![1.0, 0.0, 0.0]);
        let kitten_vec = norm(vec![0.95, 0.05, 0.0]);
        let car_vec    = norm(vec![0.0, 1.0, 0.0]);

        let mut index = CosineIndex::build(vec![
            ("http://ex.org/Cat".to_string(), cat_vec),
            ("http://ex.org/Kitten".to_string(), kitten_vec),
            ("http://ex.org/Car".to_string(), car_vec),
        ]);

        let query = norm(vec![0.99, 0.01, 0.0]);
        let results = index.search(&query, 2);

        assert_eq!(results.len(), 2);
        let iris: Vec<&str> = results.iter().map(|(i, _)| i.as_str()).collect();
        assert!(iris.iter().any(|i| i.contains("Cat") || i.contains("Kitten")));
        assert!(
            !iris.iter().any(|i| i.contains("Car")),
            "Car should NOT be in top-2; got {:?}",
            iris
        );
    }

    #[test]
    fn top_k_respected_with_fewer_results_than_corpus() {
        let vecs: Vec<_> = (0..10).map(|i| {
            let v = norm(vec![1.0, i as f32 * 0.01, 0.0]);
            (format!("iri-{}", i), v)
        }).collect();
        let mut index = CosineIndex::build(vecs);
        let query = norm(vec![1.0, 0.0, 0.0]);
        let results = index.search(&query, 3);
        assert_eq!(results.len(), 3);
        for w in results.windows(2) {
            assert!(w[0].1 >= w[1].1, "results must be sorted by similarity desc; got {:?}", results);
        }
    }
}
