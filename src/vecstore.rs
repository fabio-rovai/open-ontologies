//! In-memory vector store with dual-space search (cosine + Poincaré)
//! and SQLite persistence.

use crate::poincare::{cosine_similarity, l2_normalize, poincare_distance};
use crate::state::StateDb;
use std::collections::HashMap;

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

    pub fn upsert(&mut self, iri: &str, text_vec: &[f32], struct_vec: &[f32]) {
        self.entries.insert(iri.to_string(), VecEntry {
            text_vec: l2_normalize(text_vec),
            struct_vec: struct_vec.to_vec(),
        });
    }

    pub fn remove(&mut self, iri: &str) {
        self.entries.remove(iri);
    }

    pub fn search_cosine(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let query_norm = l2_normalize(query);
        let mut scores: Vec<(String, f32)> = self.entries.iter()
            .map(|(iri, e)| (iri.clone(), cosine_similarity(&query_norm, &e.text_vec)))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    pub fn search_poincare(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scores: Vec<(String, f32)> = self.entries.iter()
            .map(|(iri, e)| (iri.clone(), poincare_distance(query, &e.struct_vec)))
            .collect();
        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

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
                let poinc_sim = 1.0 / (1.0 + poinc);
                let combined = alpha * cos + (1.0 - alpha) * poinc_sim;
                (iri.clone(), combined)
            })
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    pub fn persist(&self) -> anyhow::Result<()> {
        let conn = self.db.conn();
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM embeddings", [])?;
        {
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
        }
        tx.commit()?;
        Ok(())
    }

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

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get_text_vec(&self, iri: &str) -> Option<&[f32]> {
        self.entries.get(iri).map(|e| e.text_vec.as_slice())
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
