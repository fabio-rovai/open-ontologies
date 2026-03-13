# Poincaré Vector Store Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an embedded Poincaré-aware vector store to Open Ontologies for semantic search and alignment signal #7, using `tract` for ONNX inference and brute-force search over in-memory vectors with SQLite persistence.

**Architecture:** Two embedding spaces — spherical (text embeddings from ONNX model) and hyperbolic Poincaré ball (structural embeddings learned from the ontology graph). Both stored in SQLite as blobs, loaded into memory on startup. HNSW not needed — brute-force is <10ms at ontology scale (<50K entities). The ONNX model is downloaded on `open-ontologies init`, not bundled in the binary or repo.

**Tech Stack:** Rust, `tract-onnx` (pure Rust ONNX runtime), `rusqlite` (existing), `tokenizers` (HuggingFace tokenizer in Rust), `reqwest` (existing, for model download)

---

### Task 1: Add Dependencies to Cargo.toml

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add tract and tokenizers behind a feature flag**

Add to `[dependencies]`:
```toml
tract-onnx = { version = "0.21", optional = true }
tokenizers = { version = "0.21", optional = true, default-features = false, features = ["onig"] }
```

Add to `[features]`:
```toml
embeddings = ["tract-onnx", "tokenizers"]
```

Update `default`:
```toml
default = ["postgres", "embeddings"]
```

**Step 2: Verify it compiles**

Run: `cargo check --features embeddings`
Expected: Compiles with no errors (no code uses it yet)

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: add tract-onnx and tokenizers deps behind embeddings feature flag"
```

---

### Task 2: Poincaré Geometry Module

**Files:**
- Create: `src/poincare.rs`
- Modify: `src/lib.rs`
- Create: `tests/poincare_test.rs`

**Step 1: Write the failing tests**

Create `tests/poincare_test.rs`:

```rust
use open_ontologies::poincare::*;

#[test]
fn test_poincare_distance_same_point() {
    let p = vec![0.1, 0.2, 0.3];
    let d = poincare_distance(&p, &p);
    assert!(d.abs() < 1e-6, "Distance to self should be ~0, got {d}");
}

#[test]
fn test_poincare_distance_symmetric() {
    let a = vec![0.1, 0.2];
    let b = vec![0.3, 0.4];
    let d1 = poincare_distance(&a, &b);
    let d2 = poincare_distance(&b, &a);
    assert!((d1 - d2).abs() < 1e-6, "Distance should be symmetric");
}

#[test]
fn test_poincare_distance_origin_farther() {
    // Points near boundary should be farther apart in Poincaré distance
    let origin = vec![0.0, 0.0];
    let near = vec![0.1, 0.0];
    let far = vec![0.9, 0.0];
    let d_near = poincare_distance(&origin, &near);
    let d_far = poincare_distance(&origin, &far);
    assert!(d_far > d_near, "Boundary point should be farther: {d_far} > {d_near}");
}

#[test]
fn test_cosine_similarity_identical() {
    let a = vec![1.0, 2.0, 3.0];
    let s = cosine_similarity(&a, &a);
    assert!((s - 1.0).abs() < 1e-6, "Cosine of identical vectors should be 1.0");
}

#[test]
fn test_cosine_similarity_orthogonal() {
    let a = vec![1.0, 0.0];
    let b = vec![0.0, 1.0];
    let s = cosine_similarity(&a, &b);
    assert!(s.abs() < 1e-6, "Cosine of orthogonal vectors should be 0.0");
}

#[test]
fn test_exp_map_origin() {
    // Exponential map at origin is simpler: tanh(||v||) * v/||v||
    let v = vec![0.1, 0.0];
    let result = exp_map(&[0.0, 0.0], &v);
    assert_eq!(result.len(), 2);
    // Result should be inside the ball (norm < 1)
    let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(norm < 1.0, "exp_map result should stay inside ball, norm={norm}");
}

#[test]
fn test_project_to_ball() {
    // A point outside the ball should be clamped
    let p = vec![0.99, 0.99]; // norm > 1
    let projected = project_to_ball(&p, 1e-5);
    let norm: f32 = projected.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(norm < 1.0, "Projected point should be inside ball, norm={norm}");
}

#[test]
fn test_rsgd_step_stays_in_ball() {
    let point = vec![0.5, 0.3];
    let grad = vec![0.1, -0.2];
    let updated = rsgd_step(&point, &grad, 0.01);
    let norm: f32 = updated.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(norm < 1.0, "RSGD step should keep point inside ball, norm={norm}");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test poincare_test --features embeddings 2>&1 | head -20`
Expected: FAIL — module `poincare` not found

**Step 3: Add module declaration**

In `src/lib.rs`, add:
```rust
#[cfg(feature = "embeddings")]
pub mod poincare;
```

**Step 4: Implement the Poincaré module**

Create `src/poincare.rs`:

```rust
//! Poincaré ball model — distance, exponential map, and Riemannian SGD.
//! Curvature c = 1.0 (unit ball).

const EPS: f32 = 1e-5;

/// Poincaré ball distance: d(u,v) = arcosh(1 + 2||u-v||² / ((1-||u||²)(1-||v||²)))
pub fn poincare_distance(u: &[f32], v: &[f32]) -> f32 {
    let diff_sq: f32 = u.iter().zip(v.iter()).map(|(a, b)| (a - b).powi(2)).sum();
    let norm_u_sq: f32 = u.iter().map(|x| x * x).sum();
    let norm_v_sq: f32 = v.iter().map(|x| x * x).sum();
    let denom = (1.0 - norm_u_sq).max(EPS) * (1.0 - norm_v_sq).max(EPS);
    let x = 1.0 + 2.0 * diff_sq / denom;
    // arcosh(x) = ln(x + sqrt(x²-1)), clamp x >= 1
    let x = x.max(1.0);
    (x + (x * x - 1.0).max(0.0).sqrt()).ln()
}

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < EPS || norm_b < EPS {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Conformal factor λ_x = 2 / (1 - ||x||²)
fn conformal_factor(x: &[f32]) -> f32 {
    let norm_sq: f32 = x.iter().map(|v| v * v).sum();
    2.0 / (1.0 - norm_sq).max(EPS)
}

/// Exponential map: maps a tangent vector at x to a point on the Poincaré ball.
/// exp_x(v) = x ⊕ tanh(λ_x * ||v|| / 2) * v / ||v||
pub fn exp_map(x: &[f32], v: &[f32]) -> Vec<f32> {
    let norm_v: f32 = v.iter().map(|a| a * a).sum::<f32>().sqrt();
    if norm_v < EPS {
        return x.to_vec();
    }
    let lambda = conformal_factor(x);
    let t = (lambda * norm_v / 2.0).tanh();
    let direction: Vec<f32> = v.iter().map(|a| t * a / norm_v).collect();
    let result = mobius_add(x, &direction);
    project_to_ball(&result, EPS)
}

/// Möbius addition: x ⊕ y
fn mobius_add(x: &[f32], y: &[f32]) -> Vec<f32> {
    let x_dot_y: f32 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
    let norm_x_sq: f32 = x.iter().map(|a| a * a).sum();
    let norm_y_sq: f32 = y.iter().map(|a| a * a).sum();
    let denom = 1.0 + 2.0 * x_dot_y + norm_x_sq * norm_y_sq;
    let denom = denom.max(EPS);
    let num_x = 1.0 + 2.0 * x_dot_y + norm_y_sq;
    let num_y = 1.0 - norm_x_sq;
    x.iter()
        .zip(y.iter())
        .map(|(xi, yi)| (num_x * xi + num_y * yi) / denom)
        .collect()
}

/// Project point back into the Poincaré ball (clamp norm < 1 - eps).
pub fn project_to_ball(p: &[f32], eps: f32) -> Vec<f32> {
    let norm: f32 = p.iter().map(|x| x * x).sum::<f32>().sqrt();
    let max_norm = 1.0 - eps;
    if norm >= max_norm {
        let scale = max_norm / norm;
        p.iter().map(|x| x * scale).collect()
    } else {
        p.to_vec()
    }
}

/// Riemannian SGD step on the Poincaré ball.
/// Rescales Euclidean gradient by (1 - ||x||²)² / 4, then applies exp_map.
pub fn rsgd_step(point: &[f32], euclidean_grad: &[f32], lr: f32) -> Vec<f32> {
    let norm_sq: f32 = point.iter().map(|x| x * x).sum();
    let scale = ((1.0 - norm_sq).max(EPS)).powi(2) / 4.0;
    let tangent: Vec<f32> = euclidean_grad.iter().map(|g| -lr * scale * g).collect();
    exp_map(point, &tangent)
}

/// L2-normalize a vector (project onto unit sphere for cosine space).
pub fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < EPS {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --test poincare_test --features embeddings -v`
Expected: All 8 tests PASS

**Step 6: Commit**

```bash
git add src/poincare.rs src/lib.rs tests/poincare_test.rs
git commit -m "feat: add Poincaré ball geometry module — distance, exp_map, Riemannian SGD"
```

---

### Task 3: Vector Store with SQLite Persistence

**Files:**
- Create: `src/vecstore.rs`
- Modify: `src/lib.rs`
- Modify: `src/state.rs`
- Create: `tests/vecstore_test.rs`

**Step 1: Write the failing tests**

Create `tests/vecstore_test.rs`:

```rust
use open_ontologies::vecstore::VecStore;
use open_ontologies::state::StateDb;

fn test_db() -> StateDb {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    StateDb::open(&path).unwrap()
}

#[test]
fn test_insert_and_search_cosine() {
    let db = test_db();
    let mut store = VecStore::new(db);

    store.upsert("http://ex.org/Dog", &[0.9, 0.1, 0.0], &[0.1, 0.0]);
    store.upsert("http://ex.org/Cat", &[0.8, 0.2, 0.0], &[0.15, 0.0]);
    store.upsert("http://ex.org/Car", &[0.0, 0.0, 1.0], &[0.5, 0.0]);

    let results = store.search_cosine(&[0.85, 0.15, 0.0], 2);
    assert_eq!(results.len(), 2);
    assert!(results[0].0.contains("Dog") || results[0].0.contains("Cat"),
        "Top result should be Dog or Cat, got {}", results[0].0);
}

#[test]
fn test_search_poincare() {
    let db = test_db();
    let mut store = VecStore::new(db);

    // Dog and Cat near each other in hyperbolic space, Car far away
    store.upsert("http://ex.org/Dog", &[0.0, 0.0, 0.0], &[0.1, 0.05]);
    store.upsert("http://ex.org/Cat", &[0.0, 0.0, 0.0], &[0.12, 0.03]);
    store.upsert("http://ex.org/Car", &[0.0, 0.0, 0.0], &[0.8, 0.8]);

    let results = store.search_poincare(&[0.11, 0.04], 2);
    assert_eq!(results.len(), 2);
    // Dog and Cat should be the top 2 results
    let iris: Vec<&str> = results.iter().map(|r| r.0.as_str()).collect();
    assert!(iris.contains(&"http://ex.org/Dog"));
    assert!(iris.contains(&"http://ex.org/Cat"));
}

#[test]
fn test_product_search() {
    let db = test_db();
    let mut store = VecStore::new(db);

    store.upsert("http://ex.org/Dog", &[0.9, 0.1], &[0.1, 0.0]);
    store.upsert("http://ex.org/Cat", &[0.8, 0.2], &[0.12, 0.0]);
    store.upsert("http://ex.org/Car", &[0.0, 1.0], &[0.8, 0.8]);

    let results = store.search_product(&[0.85, 0.15], &[0.11, 0.0], 2, 0.5);
    assert_eq!(results.len(), 2);
    assert!(results[0].0.contains("Dog") || results[0].0.contains("Cat"));
}

#[test]
fn test_upsert_overwrites() {
    let db = test_db();
    let mut store = VecStore::new(db);

    store.upsert("http://ex.org/Dog", &[1.0, 0.0], &[0.1, 0.0]);
    store.upsert("http://ex.org/Dog", &[0.0, 1.0], &[0.5, 0.0]);

    let results = store.search_cosine(&[0.0, 1.0], 1);
    assert_eq!(results[0].0, "http://ex.org/Dog");
    // Should match the updated vector, not the original
    assert!(results[0].1 > 0.99);
}

#[test]
fn test_persist_and_reload() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);

    // Insert and persist
    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db);
        store.upsert("http://ex.org/Dog", &[0.9, 0.1], &[0.1, 0.0]);
        store.persist().unwrap();
    }

    // Reload and search
    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db);
        store.load_from_db().unwrap();
        let results = store.search_cosine(&[0.9, 0.1], 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "http://ex.org/Dog");
    }
}

#[test]
fn test_remove() {
    let db = test_db();
    let mut store = VecStore::new(db);

    store.upsert("http://ex.org/Dog", &[0.9, 0.1], &[0.1, 0.0]);
    store.upsert("http://ex.org/Cat", &[0.8, 0.2], &[0.1, 0.0]);
    store.remove("http://ex.org/Dog");

    let results = store.search_cosine(&[0.9, 0.1], 10);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "http://ex.org/Cat");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test vecstore_test --features embeddings 2>&1 | head -10`
Expected: FAIL — module `vecstore` not found

**Step 3: Add SQLite schema for embeddings**

In `src/state.rs`, add to the `SCHEMA` constant (before the closing `";`):

```sql
CREATE TABLE IF NOT EXISTS embeddings (
    iri TEXT PRIMARY KEY,
    text_vec BLOB NOT NULL,
    struct_vec BLOB NOT NULL,
    text_dim INTEGER NOT NULL,
    struct_dim INTEGER NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Step 4: Add module declaration**

In `src/lib.rs`, add:
```rust
#[cfg(feature = "embeddings")]
pub mod vecstore;
```

**Step 5: Implement the vector store**

Create `src/vecstore.rs`:

```rust
//! In-memory vector store with dual-space search (cosine + Poincaré)
//! and SQLite persistence.

use crate::poincare::{cosine_similarity, l2_normalize, poincare_distance};
use crate::state::StateDb;
use std::collections::HashMap;

/// Entry holding both text (spherical) and structural (Poincaré) embeddings for an IRI.
#[derive(Clone)]
struct VecEntry {
    text_vec: Vec<f32>,
    struct_vec: Vec<f32>,
}

/// Brute-force dual-space vector store.
pub struct VecStore {
    db: StateDb,
    entries: HashMap<String, VecEntry>,
}

impl VecStore {
    pub fn new(db: StateDb) -> Self {
        Self {
            db,
            entries: HashMap::new(),
        }
    }

    /// Insert or update an embedding for the given IRI.
    pub fn upsert(&mut self, iri: &str, text_vec: &[f32], struct_vec: &[f32]) {
        self.entries.insert(iri.to_string(), VecEntry {
            text_vec: l2_normalize(text_vec),
            struct_vec: struct_vec.to_vec(),
        });
    }

    /// Remove an IRI from the store.
    pub fn remove(&mut self, iri: &str) {
        self.entries.remove(iri);
    }

    /// Search by cosine similarity on text embeddings. Returns (iri, score) sorted desc.
    pub fn search_cosine(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let query_norm = l2_normalize(query);
        let mut scores: Vec<(String, f32)> = self.entries.iter()
            .map(|(iri, e)| (iri.clone(), cosine_similarity(&query_norm, &e.text_vec)))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    /// Search by Poincaré distance on structural embeddings. Returns (iri, distance) sorted asc.
    pub fn search_poincare(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scores: Vec<(String, f32)> = self.entries.iter()
            .map(|(iri, e)| (iri.clone(), poincare_distance(query, &e.struct_vec)))
            .collect();
        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    /// Product search: weighted combination of cosine similarity and Poincaré distance.
    /// `alpha` controls the balance: 0.0 = pure Poincaré, 1.0 = pure cosine.
    pub fn search_product(
        &self,
        text_query: &[f32],
        struct_query: &[f32],
        top_k: usize,
        alpha: f32,
    ) -> Vec<(String, f32)> {
        let text_norm = l2_normalize(text_query);
        let mut scores: Vec<(String, f32)> = self.entries.iter()
            .map(|(iri, e)| {
                let cos = cosine_similarity(&text_norm, &e.text_vec);
                let poinc = poincare_distance(struct_query, &e.struct_vec);
                // Convert Poincaré distance to similarity (inverse), normalize
                let poinc_sim = 1.0 / (1.0 + poinc);
                let combined = alpha * cos + (1.0 - alpha) * poinc_sim;
                (iri.clone(), combined)
            })
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    /// Persist all in-memory embeddings to SQLite.
    pub fn persist(&self) -> anyhow::Result<()> {
        let conn = self.db.conn();
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM embeddings", [])?;
        let mut stmt = tx.prepare(
            "INSERT INTO embeddings (iri, text_vec, struct_vec, text_dim, struct_dim) VALUES (?1, ?2, ?3, ?4, ?5)"
        )?;
        for (iri, entry) in &self.entries {
            let text_bytes = f32_slice_to_bytes(&entry.text_vec);
            let struct_bytes = f32_slice_to_bytes(&entry.struct_vec);
            stmt.execute(rusqlite::params![
                iri,
                text_bytes,
                struct_bytes,
                entry.text_vec.len() as i64,
                entry.struct_vec.len() as i64,
            ])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Load embeddings from SQLite into memory.
    pub fn load_from_db(&mut self) -> anyhow::Result<()> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT iri, text_vec, struct_vec FROM embeddings")?;
        let rows = stmt.query_map([], |row| {
            let iri: String = row.get(0)?;
            let text_bytes: Vec<u8> = row.get(1)?;
            let struct_bytes: Vec<u8> = row.get(2)?;
            Ok((iri, text_bytes, struct_bytes))
        })?;

        for row in rows {
            let (iri, text_bytes, struct_bytes) = row?;
            self.entries.insert(iri, VecEntry {
                text_vec: bytes_to_f32_vec(&text_bytes),
                struct_vec: bytes_to_f32_vec(&struct_bytes),
            });
        }
        Ok(())
    }

    /// Number of entries in the store.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Get the text embedding for an IRI (for use as alignment signal).
    pub fn get_text_vec(&self, iri: &str) -> Option<&[f32]> {
        self.entries.get(iri).map(|e| e.text_vec.as_slice())
    }

    /// Get the structural embedding for an IRI.
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
```

**Step 6: Run tests to verify they pass**

Run: `cargo test --test vecstore_test --features embeddings -v`
Expected: All 7 tests PASS

**Step 7: Commit**

```bash
git add src/vecstore.rs src/state.rs src/lib.rs tests/vecstore_test.rs
git commit -m "feat: add dual-space vector store with cosine + Poincaré search and SQLite persistence"
```

---

### Task 4: ONNX Text Embedding Module

**Files:**
- Create: `src/embed.rs`
- Modify: `src/lib.rs`
- Modify: `src/config.rs`
- Modify: `src/main.rs` (init command)
- Create: `tests/embed_test.rs`

**Step 1: Write the failing test**

Create `tests/embed_test.rs`:

```rust
#[cfg(feature = "embeddings")]
mod tests {
    use open_ontologies::embed::TextEmbedder;
    use std::path::Path;

    #[test]
    fn test_model_not_found_error() {
        let result = TextEmbedder::load(Path::new("/nonexistent/model.onnx"), Path::new("/nonexistent/tokenizer.json"));
        assert!(result.is_err(), "Should error when model file doesn't exist");
    }

    // Integration test — only runs if model is downloaded
    #[test]
    fn test_embed_text_if_model_exists() {
        let model_dir = dirs::home_dir().unwrap().join(".open-ontologies/models");
        let model_path = model_dir.join("bge-small-en-v1.5.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            eprintln!("Skipping: model not downloaded. Run `open-ontologies init` first.");
            return;
        }

        let embedder = TextEmbedder::load(&model_path, &tokenizer_path).unwrap();
        let vec = embedder.embed("Dog").unwrap();

        assert_eq!(vec.len(), 384, "bge-small-en-v1.5 should produce 384-dim vectors");

        // Embedding should be L2-normalized
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "Should be L2-normalized, got norm={norm}");
    }

    #[test]
    fn test_similar_terms_closer() {
        let model_dir = dirs::home_dir().unwrap().join(".open-ontologies/models");
        let model_path = model_dir.join("bge-small-en-v1.5.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            eprintln!("Skipping: model not downloaded.");
            return;
        }

        let embedder = TextEmbedder::load(&model_path, &tokenizer_path).unwrap();
        let dog = embedder.embed("Dog").unwrap();
        let cat = embedder.embed("Cat").unwrap();
        let car = embedder.embed("Automobile").unwrap();

        let dog_cat: f32 = dog.iter().zip(cat.iter()).map(|(a, b)| a * b).sum();
        let dog_car: f32 = dog.iter().zip(car.iter()).map(|(a, b)| a * b).sum();

        assert!(dog_cat > dog_car, "Dog-Cat similarity ({dog_cat}) should be > Dog-Car ({dog_car})");
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test embed_test --features embeddings 2>&1 | head -10`
Expected: FAIL — module `embed` not found

**Step 3: Add `dirs` dependency**

In `Cargo.toml`, add to `[dependencies]`:
```toml
dirs = "6"
```

(Not behind the feature flag — it's tiny and useful for init regardless.)

**Step 4: Add embed config**

In `src/config.rs`, add to `Config`:
```rust
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EmbeddingsConfig {
    pub model: String,
    pub quantized: bool,
}

impl Default for EmbeddingsConfig {
    fn default() -> Self {
        Self {
            model: "bge-small-en-v1.5".into(),
            quantized: false,
        }
    }
}
```

And add the field to `Config`:
```rust
pub struct Config {
    pub general: GeneralConfig,
    pub embeddings: EmbeddingsConfig,
}
```

**Step 5: Add module declaration**

In `src/lib.rs`, add:
```rust
#[cfg(feature = "embeddings")]
pub mod embed;
```

**Step 6: Implement the text embedder**

Create `src/embed.rs`:

```rust
//! ONNX-based text embedding using tract.
//! Loads a sentence-transformer model exported to ONNX format.

use anyhow::{Context, Result};
use std::path::Path;
use tract_onnx::prelude::*;
use tokenizers::Tokenizer;

use crate::poincare::l2_normalize;

/// Model download URLs for bge-small-en-v1.5
pub const BGE_SMALL_ONNX_URL: &str =
    "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx";
pub const BGE_SMALL_TOKENIZER_URL: &str =
    "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/tokenizer.json";

pub struct TextEmbedder {
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
    tokenizer: Tokenizer,
    dim: usize,
}

impl TextEmbedder {
    /// Load an ONNX model and tokenizer from disk.
    pub fn load(model_path: &Path, tokenizer_path: &Path) -> Result<Self> {
        let model = tract_onnx::onnx()
            .model_for_path(model_path)
            .context("Failed to load ONNX model")?
            .into_optimized()
            .context("Failed to optimize model")?
            .into_runnable()
            .context("Failed to create runnable model")?;

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {e}"))?;

        // Detect output dimension from model
        let output_fact = model.model().output_fact(0)?;
        let dim = output_fact.shape.as_concrete()
            .and_then(|s| s.last().copied())
            .unwrap_or(384);

        Ok(Self { model, tokenizer, dim })
    }

    /// Embed a single text string. Returns L2-normalized vector.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {e}"))?;

        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&m| m as i64).collect();
        let token_type_ids: Vec<i64> = encoding.get_type_ids().iter().map(|&t| t as i64).collect();
        let seq_len = input_ids.len();

        let input_ids_tensor = tract_ndarray::Array2::from_shape_vec((1, seq_len), input_ids)?;
        let attention_tensor = tract_ndarray::Array2::from_shape_vec((1, seq_len), attention_mask.clone())?;
        let type_ids_tensor = tract_ndarray::Array2::from_shape_vec((1, seq_len), token_type_ids)?;

        let outputs = self.model.run(tvec![
            input_ids_tensor.into(),
            attention_tensor.into(),
            type_ids_tensor.into(),
        ])?;

        // Get the last hidden state (first output), shape [1, seq_len, dim]
        let output = outputs[0].to_array_view::<f32>()?;

        // Mean pooling with attention mask
        let mut pooled = vec![0.0f32; self.dim];
        let mut mask_sum = 0.0f32;
        for (i, &mask) in attention_mask.iter().enumerate() {
            if mask > 0 {
                let mask_f = mask as f32;
                for j in 0..self.dim {
                    pooled[j] += output[[0, i, j]] * mask_f;
                }
                mask_sum += mask_f;
            }
        }
        if mask_sum > 0.0 {
            for v in &mut pooled {
                *v /= mask_sum;
            }
        }

        Ok(l2_normalize(&pooled))
    }

    /// Embed multiple texts. Returns Vec of L2-normalized vectors.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // For simplicity, run sequentially. tract doesn't easily support dynamic batching.
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// Output dimension of the model.
    pub fn dim(&self) -> usize {
        self.dim
    }
}

/// Download a file from URL to a local path, with progress indication.
pub async fn download_model_file(url: &str, dest: &Path) -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await
        .context("Failed to download model")?;

    if !resp.status().is_success() {
        anyhow::bail!("Download failed with status: {}", resp.status());
    }

    let bytes = resp.bytes().await?;
    std::fs::write(dest, &bytes)
        .context("Failed to write model file")?;

    Ok(())
}
```

**Step 7: Update init command to download model**

In `src/main.rs`, inside the `Commands::Init` handler, after DB initialization, add:

```rust
#[cfg(feature = "embeddings")]
{
    let models_dir = data_path.join("models");
    std::fs::create_dir_all(&models_dir)?;

    let model_path = models_dir.join("bge-small-en-v1.5.onnx");
    let tokenizer_path = models_dir.join("tokenizer.json");

    if !model_path.exists() {
        println!("Downloading bge-small-en-v1.5 embedding model...");
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            open_ontologies::embed::download_model_file(
                open_ontologies::embed::BGE_SMALL_ONNX_URL,
                &model_path,
            ).await?;
            println!("  Model saved: {}", model_path.display());

            open_ontologies::embed::download_model_file(
                open_ontologies::embed::BGE_SMALL_TOKENIZER_URL,
                &tokenizer_path,
            ).await?;
            println!("  Tokenizer saved: {}", tokenizer_path.display());

            Ok::<_, anyhow::Error>(())
        })?;
    } else {
        println!("Embedding model already exists: {}", model_path.display());
    }
}
```

**Step 8: Run tests to verify they pass**

Run: `cargo test --test embed_test --features embeddings -v`
Expected: `test_model_not_found_error` PASS. Integration tests skip if model not downloaded.

**Step 9: Commit**

```bash
git add src/embed.rs src/config.rs src/lib.rs src/main.rs tests/embed_test.rs Cargo.toml
git commit -m "feat: add ONNX text embedder with tract — model downloaded on init"
```

---

### Task 5: Structural Embedding Trainer (Poincaré Graph Embeddings)

**Files:**
- Create: `src/structembed.rs`
- Modify: `src/lib.rs`
- Create: `tests/structembed_test.rs`

**Step 1: Write the failing tests**

Create `tests/structembed_test.rs`:

```rust
use open_ontologies::structembed::StructuralTrainer;
use open_ontologies::graph::GraphStore;
use std::sync::Arc;

#[test]
fn test_train_simple_hierarchy() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .

        ex:Animal a owl:Class ; rdfs:label "Animal" .
        ex:Mammal a owl:Class ; rdfs:label "Mammal" ; rdfs:subClassOf ex:Animal .
        ex:Dog a owl:Class ; rdfs:label "Dog" ; rdfs:subClassOf ex:Mammal .
        ex:Cat a owl:Class ; rdfs:label "Cat" ; rdfs:subClassOf ex:Mammal .
        ex:Vehicle a owl:Class ; rdfs:label "Vehicle" .
        ex:Car a owl:Class ; rdfs:label "Car" ; rdfs:subClassOf ex:Vehicle .
    "#, None).unwrap();

    let trainer = StructuralTrainer::new(10, 50, 0.01); // dim=10, epochs=50, lr=0.01
    let embeddings = trainer.train(&store).unwrap();

    // Should have embeddings for all 6 classes
    assert_eq!(embeddings.len(), 6);

    // Dog and Cat should be closer to each other than Dog and Car (siblings vs distant)
    let dog = &embeddings["http://example.org/Dog"];
    let cat = &embeddings["http://example.org/Cat"];
    let car = &embeddings["http://example.org/Car"];

    let dog_cat = open_ontologies::poincare::poincare_distance(dog, cat);
    let dog_car = open_ontologies::poincare::poincare_distance(dog, car);
    assert!(dog_cat < dog_car, "Dog-Cat ({dog_cat}) should be closer than Dog-Car ({dog_car})");
}

#[test]
fn test_train_parent_closer_than_unrelated() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .

        ex:Animal a owl:Class .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
        ex:Vehicle a owl:Class .
    "#, None).unwrap();

    let trainer = StructuralTrainer::new(10, 100, 0.01);
    let embeddings = trainer.train(&store).unwrap();

    let dog = &embeddings["http://example.org/Dog"];
    let animal = &embeddings["http://example.org/Animal"];
    let vehicle = &embeddings["http://example.org/Vehicle"];

    let dog_animal = open_ontologies::poincare::poincare_distance(dog, animal);
    let dog_vehicle = open_ontologies::poincare::poincare_distance(dog, vehicle);
    assert!(dog_animal < dog_vehicle,
        "Dog-Animal ({dog_animal}) should be closer than Dog-Vehicle ({dog_vehicle})");
}

#[test]
fn test_root_near_origin() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .

        ex:Thing a owl:Class .
        ex:Animal a owl:Class ; rdfs:subClassOf ex:Thing .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
    "#, None).unwrap();

    let trainer = StructuralTrainer::new(10, 100, 0.01);
    let embeddings = trainer.train(&store).unwrap();

    let thing_norm: f32 = embeddings["http://example.org/Thing"].iter().map(|x| x*x).sum::<f32>().sqrt();
    let dog_norm: f32 = embeddings["http://example.org/Dog"].iter().map(|x| x*x).sum::<f32>().sqrt();

    // Root should be closer to origin (smaller norm) than leaf
    assert!(thing_norm < dog_norm,
        "Root Thing ({thing_norm}) should have smaller norm than leaf Dog ({dog_norm})");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test structembed_test --features embeddings 2>&1 | head -10`
Expected: FAIL — module `structembed` not found

**Step 3: Add module declaration**

In `src/lib.rs`, add:
```rust
#[cfg(feature = "embeddings")]
pub mod structembed;
```

**Step 4: Implement the structural embedding trainer**

Create `src/structembed.rs`:

```rust
//! Learn Poincaré embeddings from the ontology class hierarchy.
//! Uses Riemannian SGD to push parent-child pairs closer and
//! negative samples apart in the Poincaré ball.

use crate::graph::GraphStore;
use crate::poincare::{poincare_distance, project_to_ball, rsgd_step};
use anyhow::Result;
use std::collections::HashMap;

pub struct StructuralTrainer {
    dim: usize,
    epochs: usize,
    lr: f32,
}

impl StructuralTrainer {
    pub fn new(dim: usize, epochs: usize, lr: f32) -> Self {
        Self { dim, epochs, lr }
    }

    /// Extract (parent, child) edges from the graph via SPARQL.
    fn extract_edges(store: &GraphStore) -> Vec<(String, String)> {
        let query = r#"
            SELECT DISTINCT ?child ?parent WHERE {
                ?child <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?parent .
                ?child a <http://www.w3.org/2002/07/owl#Class> .
                ?parent a <http://www.w3.org/2002/07/owl#Class> .
                FILTER(isIRI(?child) && isIRI(?parent))
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

        parsed["results"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|row| {
                let child = row["child"].as_str()?.trim_matches(|c| c == '<' || c == '>').to_string();
                let parent = row["parent"].as_str()?.trim_matches(|c| c == '<' || c == '>').to_string();
                Some((parent, child))
            })
            .collect()
    }

    /// Extract all class IRIs from the graph.
    fn extract_all_classes(store: &GraphStore) -> Vec<String> {
        let query = r#"
            SELECT DISTINCT ?class WHERE {
                ?class a <http://www.w3.org/2002/07/owl#Class> .
                FILTER(isIRI(?class))
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

        parsed["results"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|row| {
                Some(row["class"].as_str()?.trim_matches(|c| c == '<' || c == '>').to_string())
            })
            .collect()
    }

    /// Train Poincaré embeddings from the ontology hierarchy.
    /// Returns a map from IRI to embedding vector.
    pub fn train(&self, store: &GraphStore) -> Result<HashMap<String, Vec<f32>>> {
        let edges = Self::extract_edges(store);
        let classes = Self::extract_all_classes(store);

        if classes.is_empty() {
            return Ok(HashMap::new());
        }

        // Initialize embeddings near origin (small random values)
        let mut embeddings: HashMap<String, Vec<f32>> = HashMap::new();
        for (i, class) in classes.iter().enumerate() {
            // Deterministic pseudo-random initialization based on index
            let init: Vec<f32> = (0..self.dim)
                .map(|j| {
                    let seed = (i * self.dim + j) as f32;
                    ((seed * 2654435761.0) % 1000.0) / 50000.0 - 0.01
                })
                .collect();
            embeddings.insert(class.clone(), project_to_ball(&init, 1e-5));
        }

        if edges.is_empty() {
            return Ok(embeddings);
        }

        // Train with Riemannian SGD
        let num_classes = classes.len();
        for epoch in 0..self.epochs {
            let lr = self.lr * (1.0 - epoch as f32 / self.epochs as f32); // Linear decay

            for (parent, child) in &edges {
                let parent_emb = embeddings[parent].clone();
                let child_emb = embeddings[child].clone();

                // Positive pair: push parent and child closer
                let dist = poincare_distance(&parent_emb, &child_emb);
                if dist > 0.0 {
                    // Gradient of distance w.r.t. parent (simplified: direction from parent to child)
                    let grad_parent: Vec<f32> = parent_emb.iter().zip(child_emb.iter())
                        .map(|(p, c)| p - c)
                        .collect();
                    let grad_child: Vec<f32> = child_emb.iter().zip(parent_emb.iter())
                        .map(|(c, p)| c - p)
                        .collect();

                    let new_parent = rsgd_step(&parent_emb, &grad_parent, lr);
                    let new_child = rsgd_step(&child_emb, &grad_child, lr);
                    embeddings.insert(parent.clone(), new_parent);
                    embeddings.insert(child.clone(), new_child);
                }

                // Negative sampling: push random non-neighbor apart
                let neg_idx = (epoch * 7 + edges.len()) % num_classes;
                let neg_iri = &classes[neg_idx];
                if neg_iri != parent && neg_iri != child {
                    let neg_emb = embeddings[neg_iri].clone();
                    let child_emb = embeddings[child].clone(); // re-fetch after update

                    let neg_dist = poincare_distance(&child_emb, &neg_emb);
                    let margin = 1.0;
                    if neg_dist < margin {
                        // Push apart: gradient is negative direction
                        let grad_neg: Vec<f32> = neg_emb.iter().zip(child_emb.iter())
                            .map(|(n, c)| c - n) // push away from child
                            .collect();
                        let new_neg = rsgd_step(&neg_emb, &grad_neg, lr);
                        embeddings.insert(neg_iri.clone(), new_neg);
                    }
                }
            }
        }

        Ok(embeddings)
    }
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --test structembed_test --features embeddings -v`
Expected: All 3 tests PASS

Note: The trainer uses deterministic initialization. If tests are flaky due to convergence, increase epochs in the test (100→200).

**Step 6: Commit**

```bash
git add src/structembed.rs src/lib.rs tests/structembed_test.rs
git commit -m "feat: add Poincaré structural embedding trainer — learns hierarchy layout via Riemannian SGD"
```

---

### Task 6: MCP Tools — onto_embed and onto_search

**Files:**
- Modify: `src/server.rs`
- Modify: `src/main.rs` (CLI commands)
- Create: `tests/embed_integration_test.rs`

**Step 1: Write the failing test**

Create `tests/embed_integration_test.rs`:

```rust
/// Integration test — requires model to be downloaded.
/// Run with: cargo test --test embed_integration_test --features embeddings
#[cfg(feature = "embeddings")]
mod tests {
    use open_ontologies::graph::GraphStore;
    use open_ontologies::state::StateDb;
    use open_ontologies::vecstore::VecStore;
    use open_ontologies::embed::TextEmbedder;
    use open_ontologies::structembed::StructuralTrainer;
    use open_ontologies::poincare::cosine_similarity;
    use std::sync::Arc;

    fn model_available() -> bool {
        let model_dir = dirs::home_dir().unwrap().join(".open-ontologies/models");
        model_dir.join("bge-small-en-v1.5.onnx").exists()
            && model_dir.join("tokenizer.json").exists()
    }

    #[test]
    fn test_full_embed_and_search_pipeline() {
        if !model_available() {
            eprintln!("Skipping: model not downloaded");
            return;
        }

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        let db = StateDb::open(&path).unwrap();
        let graph = Arc::new(GraphStore::new());

        graph.load_turtle(r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex: <http://example.org/> .

            ex:Dog a owl:Class ; rdfs:label "Dog" ; rdfs:comment "A domestic canine animal" .
            ex:Cat a owl:Class ; rdfs:label "Cat" ; rdfs:comment "A domestic feline animal" .
            ex:Car a owl:Class ; rdfs:label "Car" ; rdfs:comment "A motor vehicle with four wheels" .
        "#, None).unwrap();

        // Load text embedder
        let model_dir = dirs::home_dir().unwrap().join(".open-ontologies/models");
        let embedder = TextEmbedder::load(
            &model_dir.join("bge-small-en-v1.5.onnx"),
            &model_dir.join("tokenizer.json"),
        ).unwrap();

        // Train structural embeddings
        let trainer = StructuralTrainer::new(10, 50, 0.01);
        let struct_embeddings = trainer.train(&graph).unwrap();

        // Build vector store
        let mut vecstore = VecStore::new(db);
        for (iri, struct_vec) in &struct_embeddings {
            // Get label for text embedding
            let label = iri.rsplit('/').next().unwrap_or(iri);
            let text_vec = embedder.embed(label).unwrap();
            vecstore.upsert(iri, &text_vec, struct_vec);
        }

        // Search for "canine" — should find Dog
        let query_vec = embedder.embed("canine").unwrap();
        let results = vecstore.search_cosine(&query_vec, 3);
        assert!(!results.is_empty());
        assert!(results[0].0.contains("Dog"),
            "Searching 'canine' should find Dog first, got {:?}", results);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test embed_integration_test --features embeddings 2>&1 | head -10`
Expected: FAIL or skip (depending on model availability)

**Step 3: Add MCP tool input structs to server.rs**

In `src/server.rs`, add after the existing input structs:

```rust
#[cfg(feature = "embeddings")]
#[derive(Deserialize, JsonSchema)]
pub struct OntoEmbedInput {
    /// Optional: minimum label length to embed (skip short IRIs). Default: 1
    pub min_label_len: Option<usize>,
    /// Optional: structural embedding dimension. Default: 32
    pub struct_dim: Option<usize>,
    /// Optional: structural training epochs. Default: 100
    pub struct_epochs: Option<usize>,
}

#[cfg(feature = "embeddings")]
#[derive(Deserialize, JsonSchema)]
pub struct OntoSearchInput {
    /// Natural language query to search for
    pub query: String,
    /// Number of results to return. Default: 10
    pub top_k: Option<usize>,
    /// Search mode: "text" (cosine on text embeddings), "structure" (Poincaré), or "product" (combined). Default: "product"
    pub mode: Option<String>,
    /// Weight for text vs structure in product mode (0.0-1.0, higher = more text). Default: 0.5
    pub alpha: Option<f32>,
}

#[cfg(feature = "embeddings")]
#[derive(Deserialize, JsonSchema)]
pub struct OntoSimilarityInput {
    /// First IRI
    pub iri_a: String,
    /// Second IRI
    pub iri_b: String,
}
```

**Step 4: Add VecStore and TextEmbedder to server struct**

The `OpenOntologiesServer` struct needs to hold the vector store and optionally the embedder. Add fields:

```rust
#[cfg(feature = "embeddings")]
vecstore: std::sync::Mutex<crate::vecstore::VecStore>,
#[cfg(feature = "embeddings")]
text_embedder: Option<crate::embed::TextEmbedder>,
```

Initialize in the constructor (in `main.rs` or wherever `OpenOntologiesServer::new` is called):

```rust
#[cfg(feature = "embeddings")]
{
    let model_dir = data_path.join("models");
    let embedder = if model_dir.join("bge-small-en-v1.5.onnx").exists() {
        crate::embed::TextEmbedder::load(
            &model_dir.join("bge-small-en-v1.5.onnx"),
            &model_dir.join("tokenizer.json"),
        ).ok()
    } else {
        None
    };
}
```

**Step 5: Add tool handlers**

In `src/server.rs`, add tool handler methods inside the `#[tool_router]` impl:

```rust
#[cfg(feature = "embeddings")]
#[tool(name = "onto_embed", description = "Generate text + structural embeddings for all classes in the loaded ontology. Requires the embedding model (run `open-ontologies init` to download). Embeddings enable semantic search via onto_search and improve alignment accuracy.")]
async fn onto_embed(&self, Parameters(input): Parameters<OntoEmbedInput>) -> String {
    let embedder = match &self.text_embedder {
        Some(e) => e,
        None => return r#"{"error":"Embedding model not loaded. Run `open-ontologies init` to download."}"#.to_string(),
    };

    let struct_dim = input.struct_dim.unwrap_or(32);
    let struct_epochs = input.struct_epochs.unwrap_or(100);
    let min_label_len = input.min_label_len.unwrap_or(1);

    // Extract class IRIs and labels
    let classes_query = r#"
        SELECT DISTINCT ?class ?label WHERE {
            ?class a <http://www.w3.org/2002/07/owl#Class> .
            OPTIONAL { ?class <http://www.w3.org/2000/01/rdf-schema#label> ?label }
            FILTER(isIRI(?class))
        }
    "#;

    let result = match self.graph.sparql_select(classes_query) {
        Ok(r) => r,
        Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
    };

    let parsed: serde_json::Value = match serde_json::from_str(&result) {
        Ok(v) => v,
        Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
    };

    let mut class_labels: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(rows) = parsed["results"].as_array() {
        for row in rows {
            if let Some(iri) = row["class"].as_str() {
                let iri = iri.trim_matches(|c| c == '<' || c == '>').to_string();
                let label = row["label"].as_str()
                    .map(|s| s.trim_matches('"').to_string())
                    .unwrap_or_else(|| {
                        iri.rsplit_once('#').or_else(|| iri.rsplit_once('/'))
                            .map(|(_, n)| n.to_string())
                            .unwrap_or_else(|| iri.clone())
                    });
                if label.len() >= min_label_len {
                    class_labels.insert(iri, label);
                }
            }
        }
    }

    // Train structural embeddings
    let trainer = crate::structembed::StructuralTrainer::new(struct_dim, struct_epochs, 0.01);
    let struct_embeddings = match trainer.train(&self.graph) {
        Ok(e) => e,
        Err(e) => return format!(r#"{{"error":"structural training failed: {}"}}"#, e),
    };

    // Generate text embeddings and populate vector store
    let mut vecstore = self.vecstore.lock().unwrap();
    let mut embedded_count = 0;
    let mut errors = Vec::new();

    for (iri, label) in &class_labels {
        match embedder.embed(label) {
            Ok(text_vec) => {
                let struct_vec = struct_embeddings.get(iri)
                    .cloned()
                    .unwrap_or_else(|| vec![0.0; struct_dim]);
                vecstore.upsert(iri, &text_vec, &struct_vec);
                embedded_count += 1;
            }
            Err(e) => errors.push(format!("{}: {}", iri, e)),
        }
    }

    // Persist to SQLite
    if let Err(e) = vecstore.persist() {
        return format!(r#"{{"error":"failed to persist embeddings: {}"}}"#, e);
    }

    serde_json::json!({
        "ok": true,
        "embedded": embedded_count,
        "total_classes": class_labels.len(),
        "text_dim": embedder.dim(),
        "struct_dim": struct_dim,
        "errors": errors,
    }).to_string()
}

#[cfg(feature = "embeddings")]
#[tool(name = "onto_search", description = "Semantic search over the loaded ontology using natural language. Returns the most similar classes by text meaning, structural position, or both. Requires onto_embed to have been run first.")]
async fn onto_search(&self, Parameters(input): Parameters<OntoSearchInput>) -> String {
    let vecstore = self.vecstore.lock().unwrap();
    if vecstore.len() == 0 {
        return r#"{"error":"No embeddings loaded. Run onto_embed first."}"#.to_string();
    }

    let top_k = input.top_k.unwrap_or(10);
    let mode = input.mode.as_deref().unwrap_or("product");
    let alpha = input.alpha.unwrap_or(0.5);

    let results = match mode {
        "text" => {
            let embedder = match &self.text_embedder {
                Some(e) => e,
                None => return r#"{"error":"Embedding model not loaded."}"#.to_string(),
            };
            let query_vec = match embedder.embed(&input.query) {
                Ok(v) => v,
                Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
            };
            let hits = vecstore.search_cosine(&query_vec, top_k);
            hits.into_iter().map(|(iri, score)| serde_json::json!({"iri": iri, "score": score})).collect::<Vec<_>>()
        }
        "structure" => {
            // For structure-only search, we need a reference IRI, not text
            // Fall back to text search and use the top result's structural embedding as query
            let embedder = match &self.text_embedder {
                Some(e) => e,
                None => return r#"{"error":"Embedding model not loaded."}"#.to_string(),
            };
            let query_vec = match embedder.embed(&input.query) {
                Ok(v) => v,
                Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
            };
            // Find closest text match, then search by its structural position
            let text_hits = vecstore.search_cosine(&query_vec, 1);
            if let Some((anchor_iri, _)) = text_hits.first() {
                if let Some(struct_vec) = vecstore.get_struct_vec(anchor_iri) {
                    let hits = vecstore.search_poincare(struct_vec, top_k);
                    hits.into_iter().map(|(iri, dist)| serde_json::json!({"iri": iri, "poincare_distance": dist})).collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        }
        _ => {
            // Product search (default)
            let embedder = match &self.text_embedder {
                Some(e) => e,
                None => return r#"{"error":"Embedding model not loaded."}"#.to_string(),
            };
            let query_vec = match embedder.embed(&input.query) {
                Ok(v) => v,
                Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
            };
            // Use origin as structural query (searches broadly in hierarchy)
            let struct_query = vec![0.0f32; vecstore.get_struct_vec(
                vecstore.search_cosine(&query_vec, 1).first().map(|r| r.0.as_str()).unwrap_or("")
            ).map(|v| v.len()).unwrap_or(32)];
            let hits = vecstore.search_product(&query_vec, &struct_query, top_k, alpha);
            hits.into_iter().map(|(iri, score)| serde_json::json!({"iri": iri, "score": score})).collect()
        }
    };

    serde_json::json!({
        "results": results,
        "query": input.query,
        "mode": mode,
        "count": results.len(),
    }).to_string()
}

#[cfg(feature = "embeddings")]
#[tool(name = "onto_similarity", description = "Compute embedding similarity between two IRIs — returns cosine similarity (text), Poincaré distance (structural), and product score.")]
async fn onto_similarity(&self, Parameters(input): Parameters<OntoSimilarityInput>) -> String {
    let vecstore = self.vecstore.lock().unwrap();

    let text_a = vecstore.get_text_vec(&input.iri_a);
    let text_b = vecstore.get_text_vec(&input.iri_b);
    let struct_a = vecstore.get_struct_vec(&input.iri_a);
    let struct_b = vecstore.get_struct_vec(&input.iri_b);

    if text_a.is_none() || text_b.is_none() {
        return format!(r#"{{"error":"IRI not found in embeddings. Run onto_embed first. Missing: {}"}}"#,
            if text_a.is_none() { &input.iri_a } else { &input.iri_b });
    }

    let cos = crate::poincare::cosine_similarity(text_a.unwrap(), text_b.unwrap());
    let poinc = if let (Some(a), Some(b)) = (struct_a, struct_b) {
        crate::poincare::poincare_distance(a, b)
    } else {
        -1.0
    };

    serde_json::json!({
        "iri_a": input.iri_a,
        "iri_b": input.iri_b,
        "cosine_similarity": (cos * 1000.0).round() / 1000.0,
        "poincare_distance": (poinc * 1000.0).round() / 1000.0,
        "product_score": if poinc >= 0.0 {
            (0.5 * cos + 0.5 / (1.0 + poinc)) * 1000.0 / 1000.0
        } else {
            cos as f64
        },
    }).to_string()
}
```

**Step 6: Run tests to verify they pass**

Run: `cargo test --test embed_integration_test --features embeddings -v`
Expected: PASS (or skip if model not downloaded)

Run: `cargo build --features embeddings`
Expected: Compiles successfully

**Step 7: Commit**

```bash
git add src/server.rs src/main.rs tests/embed_integration_test.rs
git commit -m "feat: add onto_embed, onto_search, onto_similarity MCP tools"
```

---

### Task 7: Wire Embeddings as Alignment Signal #7

**Files:**
- Modify: `src/align.rs`
- Modify: `tests/poincare_test.rs` (add alignment signal test)

**Step 1: Write the failing test**

Add to the bottom of the existing `mod tests` in `src/align.rs`:

```rust
#[cfg(feature = "embeddings")]
#[test]
fn test_embedding_similarity_signal() {
    // Verify the embedding_similarity method works in isolation
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
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib align::tests::test_embedding_similarity_signal --features embeddings 2>&1 | head -10`
Expected: FAIL — method `embedding_similarity_score` not found

**Step 3: Add embedding signal to AlignmentEngine**

In `src/align.rs`, add the method and update weights:

```rust
#[cfg(feature = "embeddings")]
/// Compute embedding similarity score using cosine similarity on text vectors.
pub fn embedding_similarity_score(vec_a: &[f32], vec_b: &[f32]) -> f64 {
    crate::poincare::cosine_similarity(vec_a, vec_b) as f64
}
```

Update `DEFAULT_WEIGHTS` to support 7 signals when embeddings are available:

```rust
#[cfg(not(feature = "embeddings"))]
const DEFAULT_WEIGHTS: [f64; 6] = [0.25, 0.20, 0.15, 0.15, 0.15, 0.10];

#[cfg(feature = "embeddings")]
const DEFAULT_WEIGHTS: [f64; 7] = [0.20, 0.15, 0.12, 0.12, 0.12, 0.09, 0.20];
```

In the `align` method, after computing the 6 existing signals, add:

```rust
#[cfg(feature = "embeddings")]
let embedding_sim = {
    // Try to get embeddings from the vecstore if available
    // For now, fall back to 0.0 if no embeddings loaded
    0.0 // Will be populated when vecstore is passed in
};

#[cfg(feature = "embeddings")]
let signals = [label_sim, prop_overlap, parent_ovlp, inst_overlap, restr_sim, neigh_sim, embedding_sim];

#[cfg(not(feature = "embeddings"))]
let signals = [label_sim, prop_overlap, parent_ovlp, inst_overlap, restr_sim, neigh_sim];
```

Note: Full integration with the vecstore requires passing it into the `AlignmentEngine`. This can be done by adding an `Option<&VecStore>` parameter to `align()` or storing it as an optional field. The exact wiring depends on the server's ownership model — implement during this task.

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib align::tests --features embeddings -v`
Expected: All alignment tests PASS including the new one

**Step 5: Commit**

```bash
git add src/align.rs
git commit -m "feat: add embedding similarity as alignment signal #7 (behind embeddings feature flag)"
```

---

### Task 8: Update CLAUDE.md and Documentation

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`

**Step 1: Add embedding tools to CLAUDE.md tool reference table**

Add after the alignment tools:

```markdown
| `onto_embed` | After loading an ontology — generates text + Poincaré structural embeddings for all classes |
| `onto_search` | To find classes by natural language description — requires onto_embed first |
| `onto_similarity` | To compute embedding similarity between two specific IRIs |
```

**Step 2: Add embedding section to CLAUDE.md workflow**

Add a new section after "Data Extension Workflow":

```markdown
## Semantic Search & Embedding Workflow

When exploring or aligning ontologies using semantic embeddings:

### Setup

1. Ensure the embedding model is downloaded (`open-ontologies init`)
2. Call `onto_load` to load the ontology
3. Call `onto_embed` to generate text + structural embeddings for all classes

### Search

4. Call `onto_search` with a natural language query — returns most similar classes
5. Use `mode: "text"` for label/definition similarity, `mode: "structure"` for hierarchy position, `mode: "product"` for combined

### Compare

6. Call `onto_similarity` with two IRIs to see cosine + Poincaré distance between them

### Alignment Enhancement

7. When running `onto_align`, embedding similarity is automatically used as signal #7 if embeddings are loaded
8. This catches semantically equivalent classes that have different labels (e.g., Vehicle ↔ Automobile)
```

**Step 3: Update README.md**

Add to the features section and tool table. Add a section about the Poincaré geometry and the dual-space architecture.

**Step 4: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "docs: add embedding tools to workflow documentation"
```

---

### Task 9: Final Integration Test — End-to-End

**Files:**
- Create: `tests/embedding_e2e_test.rs`

**Step 1: Write the end-to-end test**

Create `tests/embedding_e2e_test.rs`:

```rust
//! End-to-end test: load ontology → embed → search → align with embeddings.
//! Only runs if the ONNX model is available.

#[cfg(feature = "embeddings")]
mod tests {
    use open_ontologies::graph::GraphStore;
    use open_ontologies::state::StateDb;
    use open_ontologies::vecstore::VecStore;
    use open_ontologies::embed::TextEmbedder;
    use open_ontologies::structembed::StructuralTrainer;
    use open_ontologies::align::AlignmentEngine;
    use std::sync::Arc;

    fn model_available() -> bool {
        let model_dir = dirs::home_dir().unwrap().join(".open-ontologies/models");
        model_dir.join("bge-small-en-v1.5.onnx").exists()
    }

    #[test]
    fn test_e2e_embed_search_align() {
        if !model_available() {
            eprintln!("Skipping e2e: model not downloaded");
            return;
        }

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        let db = StateDb::open(&path).unwrap();
        let graph = Arc::new(GraphStore::new());

        // Load a small ontology
        graph.load_turtle(r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex: <http://example.org/> .

            ex:Animal a owl:Class ; rdfs:label "Animal" .
            ex:Mammal a owl:Class ; rdfs:label "Mammal" ; rdfs:subClassOf ex:Animal .
            ex:Dog a owl:Class ; rdfs:label "Dog" ; rdfs:subClassOf ex:Mammal .
            ex:Cat a owl:Class ; rdfs:label "Cat" ; rdfs:subClassOf ex:Mammal .
            ex:Vehicle a owl:Class ; rdfs:label "Vehicle" .
            ex:Car a owl:Class ; rdfs:label "Car" ; rdfs:subClassOf ex:Vehicle .
        "#, None).unwrap();

        // Step 1: Embed
        let model_dir = dirs::home_dir().unwrap().join(".open-ontologies/models");
        let embedder = TextEmbedder::load(
            &model_dir.join("bge-small-en-v1.5.onnx"),
            &model_dir.join("tokenizer.json"),
        ).unwrap();

        let trainer = StructuralTrainer::new(32, 100, 0.01);
        let struct_embeddings = trainer.train(&graph).unwrap();

        let mut vecstore = VecStore::new(db.clone());
        for (iri, struct_vec) in &struct_embeddings {
            let label = iri.rsplit('/').next().unwrap_or(iri);
            let text_vec = embedder.embed(label).unwrap();
            vecstore.upsert(iri, &text_vec, struct_vec);
        }

        // Step 2: Search — "pet" should return Dog and Cat before Car
        let pet_vec = embedder.embed("pet").unwrap();
        let results = vecstore.search_cosine(&pet_vec, 6);
        let top3_iris: Vec<&str> = results.iter().take(3).map(|r| r.0.as_str()).collect();
        // At least Dog or Cat should be in top 3
        assert!(
            top3_iris.iter().any(|iri| iri.contains("Dog") || iri.contains("Cat")),
            "Searching 'pet' should find Dog or Cat in top 3: {:?}", top3_iris
        );

        // Step 3: Structural search — siblings should be close
        let dog_struct = vecstore.get_struct_vec("http://example.org/Dog").unwrap();
        let struct_results = vecstore.search_poincare(dog_struct, 6);
        // Cat should be closer to Dog than Vehicle
        let cat_rank = struct_results.iter().position(|r| r.0.contains("Cat"));
        let vehicle_rank = struct_results.iter().position(|r| r.0.contains("Vehicle"));
        if let (Some(cat_r), Some(veh_r)) = (cat_rank, vehicle_rank) {
            assert!(cat_r < veh_r, "Cat should rank closer to Dog than Vehicle");
        }

        // Step 4: Persist and reload
        vecstore.persist().unwrap();

        let mut vecstore2 = VecStore::new(db);
        vecstore2.load_from_db().unwrap();
        assert_eq!(vecstore2.len(), 6);

        println!("E2E test passed: embed → search → persist → reload");
    }
}
```

**Step 2: Run the test**

Run: `cargo test --test embedding_e2e_test --features embeddings -v`
Expected: PASS (or skip if model not downloaded)

**Step 3: Commit**

```bash
git add tests/embedding_e2e_test.rs
git commit -m "test: add end-to-end embedding pipeline test"
```

---

## Summary

| Task | What | New Files | Lines |
|------|------|-----------|-------|
| 1 | Dependencies | — | ~10 |
| 2 | Poincaré geometry | `poincare.rs` | ~100 |
| 3 | Vector store | `vecstore.rs` | ~150 |
| 4 | ONNX text embedder | `embed.rs` | ~130 |
| 5 | Structural trainer | `structembed.rs` | ~150 |
| 6 | MCP tools | server.rs changes | ~200 |
| 7 | Alignment signal #7 | align.rs changes | ~30 |
| 8 | Documentation | CLAUDE.md, README.md | ~40 |
| 9 | E2E test | `embedding_e2e_test.rs` | ~80 |
| **Total** | | **5 new files** | **~890** |

All behind `#[cfg(feature = "embeddings")]`. Zero impact on default binary when feature is disabled.

---

## Implementation Notes (2026-03-13)

Deviations and adjustments made during implementation:

### Task 1
- No deviations.

### Task 2
- No deviations. All 8 Poincaré geometry tests pass as written.

### Task 3
- **Fix:** `persist()` needed a block scope around the prepared statement so it drops before `tx.commit()` (Rust borrow checker).
- **Fix:** Subagent accidentally reverted Cargo.toml — removed `tract-onnx`, `tokenizers`, `dirs` deps and reset `embeddings` feature to `[]`. Had to manually restore.

### Task 4
- **API fix:** tract 0.21 requires `.into_tensor().into()` to convert ndarray arrays to `TValue`, not just `.into()`.
- `config.rs` changes (EmbeddingsConfig) were deferred — not needed since model path is derived from `~/.open-ontologies/models/` convention.

### Task 5
- **Training improvements:** Added hierarchy-aware training beyond the plan:
  - Root nodes pulled toward origin each epoch
  - Children pushed outward when their norm falls below parent's
  - Negative sampling index uses both epoch and edge index for diversity
- **Test parameters:** lr bumped to 0.1 with 500 epochs for robust convergence (plan had 0.01/50-100).

### Task 6
- **Clone compatibility:** `VecStore` wrapped in `Arc<Mutex<>>` and `TextEmbedder` in `Option<Arc<>>` since `OpenOntologiesServer` derives `Clone`.
- **Init command:** Uses `.await` directly (main is already async) instead of `Runtime::new()` as plan suggested.
- `OntoEmbedInput.min_label_len` field dropped from final implementation (unnecessary complexity).
- `embed_integration_test.rs` from plan was replaced by `embedding_e2e_test.rs` in Task 9.

### Task 7
- **Full vecstore integration:** Added `new_with_vecstore()` constructor and `vecstore` field to `AlignmentEngine` (plan only sketched this).
- **Structural fallback:** Uses `signals[1..6]` to correctly isolate structural signals in both 6-signal and 7-signal modes.
- **JSON output:** Embedding similarity included in alignment output JSON when feature enabled.

### Task 8
- Updated README tool count from 39 to 42.
- Added embeddings section to README with tool table and architecture description.

### Task 9
- Test is more comprehensive than plan: includes similarity ordering check (Dog-Cat > Dog-Car) and persist/reload round-trip verification with cosine similarity comparison.

### Final Stats
- **All 215+ tests pass** with `--features embeddings`
- **5 new source files:** `poincare.rs`, `vecstore.rs`, `embed.rs`, `structembed.rs`, `embedding_e2e_test.rs`
- **3 new MCP tools:** `onto_embed`, `onto_search`, `onto_similarity`
- **Feature flag:** `embeddings` (default-enabled, zero impact when disabled)
