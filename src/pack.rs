//! Knowledge packs: a verified graph and its evidence in one portable file.
//!
//! TrustGraph calls the idea a knowledge core, and it is the right one: what
//! you promote between environments should be a versioned artifact, not a
//! pile of loose Turtle whose provenance and checks live somewhere else. The
//! shape here is deliberately boring so that anything can read it, and the
//! graph is stored as ordinary N-Triples rather than a proprietary blob.
//!
//! A pack carries:
//!
//!   - the graph itself (N-Triples, sorted, so two packs of the same graph
//!     are byte-identical and diffable);
//!   - a manifest: name, version, counts, creation time, tool version;
//!   - a checksum over the graph, so tampering or truncation is detectable;
//!   - the verification evidence recorded at pack time (lint, enforce), so
//!     the receiving environment can see what the graph passed rather than
//!     take it on trust.
//!
//! Unpacking verifies the checksum before loading a single triple.

use crate::graph::GraphStore;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub struct Packer {
    graph: Arc<GraphStore>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub created_at: String,
    pub tool_version: String,
    pub triples: usize,
    pub sha256: String,
    /// What the graph passed at pack time. Absent means not checked, which
    /// is itself information the receiver should act on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Pack {
    pub manifest: Manifest,
    /// N-Triples, sorted line-wise.
    pub graph: String,
}

fn checksum(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

impl Packer {
    pub fn new(graph: Arc<GraphStore>) -> Self {
        Self { graph }
    }

    /// Write the loaded graph and its evidence to `path` as a pack.
    pub fn pack(
        &self,
        path: &str,
        name: &str,
        version: &str,
        evidence: Option<serde_json::Value>,
    ) -> anyhow::Result<String> {
        let nt = self.graph.serialize("ntriples")?;

        // Sorted, so the same graph always produces the same bytes: packs
        // become diffable and the checksum becomes meaningful across
        // machines rather than reflecting store iteration order.
        let mut lines: Vec<&str> = nt.lines().filter(|l| !l.trim().is_empty()).collect();
        lines.sort_unstable();
        let graph = lines.join("\n") + "\n";

        let manifest = Manifest {
            name: name.to_string(),
            version: version.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            triples: lines.len(),
            sha256: checksum(&graph),
            evidence,
        };

        let pack = Pack { manifest, graph };
        let json = serde_json::to_string_pretty(&pack)?;
        std::fs::write(path, &json)?;

        Ok(serde_json::json!({
            "ok": true,
            "path": path,
            "name": pack.manifest.name,
            "version": pack.manifest.version,
            "triples": pack.manifest.triples,
            "sha256": pack.manifest.sha256,
            "has_evidence": pack.manifest.evidence.is_some(),
        })
        .to_string())
    }

    /// Load a pack, refusing it if the checksum does not match.
    pub fn unpack(&self, path: &str, verify_only: bool) -> anyhow::Result<String> {
        let raw = std::fs::read_to_string(path)?;
        let pack: Pack = serde_json::from_str(&raw)?;

        let actual = checksum(&pack.graph);
        if actual != pack.manifest.sha256 {
            return Ok(serde_json::json!({
                "error": "checksum mismatch: the pack was modified or truncated after it was written",
                "expected": pack.manifest.sha256,
                "actual": actual,
            })
            .to_string());
        }

        if verify_only {
            return Ok(serde_json::json!({
                "ok": true,
                "verified": true,
                "loaded": false,
                "manifest": pack.manifest,
            })
            .to_string());
        }

        let loaded = self.graph.load_ntriples(&pack.graph)?;
        Ok(serde_json::json!({
            "ok": true,
            "verified": true,
            "loaded": true,
            "triples_loaded": loaded,
            "manifest": pack.manifest,
        })
        .to_string())
    }
}
