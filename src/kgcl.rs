//! KGCL (Knowledge Graph Change Language) serialization for drift reports.
//!
//! Spec: <https://github.com/INCATools/kgcl>
//! Paper: Mungall et al., Database (Oxford) 2025, doi:10.1093/database/baae133
//!
//! Maps `DriftDetector::detect` output to KGCL change records. Two surface forms:
//! - **CNL** (Controlled Natural Language) — line-oriented, consumed by ROBOT and BioPortal
//! - **JSON-LD-style structured** — for machine consumers / replay tooling

use serde::{Deserialize, Serialize};

/// A single KGCL change. We model only the subset the drift detector can produce.
/// Other KGCL types (NodeMove, NewSynonym, EdgeChange, etc.) are out of scope here —
/// the drift detector compares class/property vocabularies, not edges or annotations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum KgclChange {
    /// Creation of a new node (class or property). KGCL: `NodeCreation`.
    NodeCreation {
        id: String,
        about_node: String,
        /// "class" or "property" — informational, not part of core KGCL.
        node_kind: String,
    },
    /// Deprecation of a node, optionally pointing to a replacement.
    /// KGCL: `NodeObsoletion` (with optional `has_direct_replacement`).
    NodeObsoletion {
        id: String,
        about_node: String,
        node_kind: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        has_direct_replacement: Option<String>,
        /// Confidence carried from drift detector when this change came from a likely-rename.
        /// Stored in the KGCL `change_description` slot when serialised; surfaced as a field for tooling.
        #[serde(skip_serializing_if = "Option::is_none")]
        confidence: Option<f64>,
    },
}

impl KgclChange {
    /// Render this change as KGCL CNL (Controlled Natural Language).
    ///
    /// Grammar (from KGCL paper §4):
    /// - `create <kind> <iri>`
    /// - `obsolete <kind> <iri>`
    /// - `obsolete <kind> <iri> with replacement <iri>`
    pub fn to_cnl(&self) -> String {
        match self {
            KgclChange::NodeCreation { about_node, node_kind, .. } => {
                format!("create {} <{}>", node_kind, about_node)
            }
            KgclChange::NodeObsoletion {
                about_node,
                node_kind,
                has_direct_replacement: Some(repl),
                ..
            } => {
                format!(
                    "obsolete {} <{}> with replacement <{}>",
                    node_kind, about_node, repl
                )
            }
            KgclChange::NodeObsoletion {
                about_node,
                node_kind,
                has_direct_replacement: None,
                ..
            } => {
                format!("obsolete {} <{}>", node_kind, about_node)
            }
        }
    }
}

/// A bundle of KGCL changes representing a drift between two ontology versions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KgclReport {
    pub changes: Vec<KgclChange>,
}

impl KgclReport {
    /// CNL representation, one change per line.
    pub fn to_cnl(&self) -> String {
        self.changes
            .iter()
            .map(|c| c.to_cnl())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Structured JSON representation (for machine consumers).
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "@context": "http://w3id.org/kgcl/",
            "changes": self.changes,
        })
    }
}

/// Convert a drift report (the JSON `DriftDetector::detect` returns) into KGCL changes.
///
/// `rename_confidence_threshold`: likely_renames with confidence ≥ threshold are emitted as
/// `NodeObsoletion(old) with replacement(new)` + `NodeCreation(new)`, and the involved IRIs are
/// removed from the plain added/removed lists to avoid double-counting. Renames below threshold
/// fall through to plain Creation/Obsoletion pairs.
pub fn drift_to_kgcl(
    drift_json: &serde_json::Value,
    rename_confidence_threshold: f64,
) -> KgclReport {
    let mut changes = Vec::new();
    let mut next_id = 1u32;
    let mut next_id_fn = move || {
        let id = format!("kgcl_change_{:04}", next_id);
        next_id += 1;
        id
    };

    let added: Vec<String> = drift_json["added"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let removed: Vec<String> = drift_json["removed"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Pull confident renames first, deduplicating against added/removed.
    let mut consumed_added = std::collections::HashSet::new();
    let mut consumed_removed = std::collections::HashSet::new();

    if let Some(renames) = drift_json["likely_renames"].as_array() {
        for r in renames {
            let confidence = r["confidence"].as_f64().unwrap_or(0.0);
            if confidence < rename_confidence_threshold {
                continue;
            }
            let from = match r["from"].as_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            let to = match r["to"].as_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            // First confident rename for this (from, to) wins — sorted by confidence desc upstream.
            if consumed_removed.contains(&from) || consumed_added.contains(&to) {
                continue;
            }
            consumed_removed.insert(from.clone());
            consumed_added.insert(to.clone());

            changes.push(KgclChange::NodeObsoletion {
                id: next_id_fn(),
                about_node: from,
                node_kind: "node".to_string(),
                has_direct_replacement: Some(to.clone()),
                confidence: Some(confidence),
            });
            changes.push(KgclChange::NodeCreation {
                id: next_id_fn(),
                about_node: to,
                node_kind: "node".to_string(),
            });
        }
    }

    for iri in &removed {
        if consumed_removed.contains(iri) {
            continue;
        }
        changes.push(KgclChange::NodeObsoletion {
            id: next_id_fn(),
            about_node: iri.clone(),
            node_kind: "node".to_string(),
            has_direct_replacement: None,
            confidence: None,
        });
    }

    for iri in &added {
        if consumed_added.contains(iri) {
            continue;
        }
        changes.push(KgclChange::NodeCreation {
            id: next_id_fn(),
            about_node: iri.clone(),
            node_kind: "node".to_string(),
        });
    }

    KgclReport { changes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cnl_creation() {
        let c = KgclChange::NodeCreation {
            id: "kgcl_change_0001".to_string(),
            about_node: "http://example.org/Cat".to_string(),
            node_kind: "class".to_string(),
        };
        assert_eq!(c.to_cnl(), "create class <http://example.org/Cat>");
    }

    #[test]
    fn cnl_obsoletion_no_replacement() {
        let c = KgclChange::NodeObsoletion {
            id: "kgcl_change_0001".to_string(),
            about_node: "http://example.org/Dog".to_string(),
            node_kind: "class".to_string(),
            has_direct_replacement: None,
            confidence: None,
        };
        assert_eq!(c.to_cnl(), "obsolete class <http://example.org/Dog>");
    }

    #[test]
    fn cnl_obsoletion_with_replacement() {
        let c = KgclChange::NodeObsoletion {
            id: "kgcl_change_0001".to_string(),
            about_node: "http://example.org/authoredBy".to_string(),
            node_kind: "property".to_string(),
            has_direct_replacement: Some("http://example.org/writtenBy".to_string()),
            confidence: Some(0.91),
        };
        assert_eq!(
            c.to_cnl(),
            "obsolete property <http://example.org/authoredBy> with replacement <http://example.org/writtenBy>"
        );
    }

    #[test]
    fn drift_to_kgcl_plain_add_remove() {
        let drift = json!({
            "added": ["http://example.org/Bird"],
            "removed": ["http://example.org/Cat"],
            "likely_renames": [],
        });
        let report = drift_to_kgcl(&drift, 0.7);
        assert_eq!(report.changes.len(), 2);
        // Order: obsoletions first, then creations.
        assert!(matches!(report.changes[0], KgclChange::NodeObsoletion { .. }));
        assert!(matches!(report.changes[1], KgclChange::NodeCreation { .. }));

        let cnl = report.to_cnl();
        assert!(cnl.contains("obsolete node <http://example.org/Cat>"));
        assert!(cnl.contains("create node <http://example.org/Bird>"));
    }

    #[test]
    fn drift_to_kgcl_high_confidence_rename_collapses() {
        let drift = json!({
            "added": ["http://example.org/writtenBy"],
            "removed": ["http://example.org/authoredBy"],
            "likely_renames": [{
                "from": "http://example.org/authoredBy",
                "to": "http://example.org/writtenBy",
                "confidence": 0.91,
                "predicted": "rename",
                "signals": {}
            }],
        });
        let report = drift_to_kgcl(&drift, 0.7);
        // Exactly one obsoletion-with-replacement + one creation; no duplicate plain entries.
        assert_eq!(report.changes.len(), 2);
        match &report.changes[0] {
            KgclChange::NodeObsoletion {
                about_node,
                has_direct_replacement: Some(repl),
                confidence: Some(c),
                ..
            } => {
                assert_eq!(about_node, "http://example.org/authoredBy");
                assert_eq!(repl, "http://example.org/writtenBy");
                assert!((c - 0.91).abs() < 1e-9);
            }
            other => panic!("expected NodeObsoletion with replacement, got {:?}", other),
        }
        match &report.changes[1] {
            KgclChange::NodeCreation { about_node, .. } => {
                assert_eq!(about_node, "http://example.org/writtenBy");
            }
            other => panic!("expected NodeCreation, got {:?}", other),
        }

        let cnl = report.to_cnl();
        assert!(cnl.contains(
            "obsolete node <http://example.org/authoredBy> with replacement <http://example.org/writtenBy>"
        ));
    }

    #[test]
    fn drift_to_kgcl_low_confidence_rename_does_not_collapse() {
        let drift = json!({
            "added": ["http://example.org/writtenBy"],
            "removed": ["http://example.org/authoredBy"],
            "likely_renames": [{
                "from": "http://example.org/authoredBy",
                "to": "http://example.org/writtenBy",
                "confidence": 0.35,
                "predicted": "rename",
                "signals": {}
            }],
        });
        let report = drift_to_kgcl(&drift, 0.7);
        // Below threshold — plain obsoletion + plain creation, no replacement link.
        assert_eq!(report.changes.len(), 2);
        match &report.changes[0] {
            KgclChange::NodeObsoletion {
                has_direct_replacement: None,
                confidence: None,
                ..
            } => {}
            other => panic!("expected plain NodeObsoletion, got {:?}", other),
        }
    }

    #[test]
    fn drift_to_kgcl_id_uniqueness() {
        let drift = json!({
            "added": ["http://example.org/A", "http://example.org/B"],
            "removed": ["http://example.org/X", "http://example.org/Y"],
            "likely_renames": [],
        });
        let report = drift_to_kgcl(&drift, 0.7);
        let mut ids: Vec<&str> = report
            .changes
            .iter()
            .map(|c| match c {
                KgclChange::NodeCreation { id, .. } => id.as_str(),
                KgclChange::NodeObsoletion { id, .. } => id.as_str(),
            })
            .collect();
        ids.sort();
        let count_before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count_before, "IDs must be unique within a report");
    }

    #[test]
    fn report_to_json_has_context() {
        let report = KgclReport {
            changes: vec![KgclChange::NodeCreation {
                id: "kgcl_change_0001".to_string(),
                about_node: "http://example.org/Cat".to_string(),
                node_kind: "class".to_string(),
            }],
        };
        let j = report.to_json();
        assert_eq!(j["@context"].as_str().unwrap(), "http://w3id.org/kgcl/");
        assert!(j["changes"].is_array());
    }
}
