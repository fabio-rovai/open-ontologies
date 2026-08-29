//! Claim support: is a fact in the graph actually backed by the source it
//! cites, and does it cite one at all.
//!
//! Conformance and support are independent failures. `onto_vocab_check`
//! answers "is this expressible in the declared vocabulary", which a
//! perfectly-sourced fact can fail and a completely fabricated one can pass.
//! Support answers the other half: the graph says X and points at document D,
//! but does D say X? A graph can be flawless on both counts only if both are
//! checked, and most pipelines check neither.
//!
//! Two things happen here, split along the line the project always draws:
//!
//!   - what is computable is computed. Which claims cite no source at all is
//!     a query, not a judgement, and an unsourced-claim rate is a number you
//!     can put in a report today;
//!   - what needs reading is not guessed. For claims that do cite a source,
//!     the engine emits a verification TASK (the claim in words, the source,
//!     what to decide) and the connected model returns a verdict through
//!     `record_verdict`. No model client lives in here.
//!
//! Inspired by the ProVe line of work on reference verification for
//! knowledge graphs (Amaral et al., KCL), generalised: any provenance
//! predicate, any source, verdicts supplied by whoever is connected.

use crate::graph::GraphStore;
use crate::state::StateDb;
use std::sync::Arc;

pub struct SupportChecker {
    graph: Arc<GraphStore>,
    db: StateDb,
}

#[derive(serde::Serialize)]
pub struct SupportTask {
    /// Stable id for the claim, so a verdict can be recorded against it.
    pub claim_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    /// The claim as a sentence, so the judgement does not require reading RDF.
    pub claim: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_label: Option<String>,
}

const INSTRUCTION: &str = concat!(
    "For each task, decide whether the cited source supports the claim: ",
    "'supported' if the source states it, 'refuted' if the source states ",
    "something incompatible with it, 'unrelated' if the source is silent on ",
    "it. Read the source; do not decide from the claim's plausibility, which ",
    "is exactly the failure this check exists to catch. Record each decision ",
    "with onto_support_verdict. Claims listed under unsourced cite nothing at ",
    "all and need a source before they can be judged."
);

const PROV_DERIVED: &str = "http://www.w3.org/ns/prov#wasDerivedFrom";

fn strip(value: &str) -> String {
    let v = value.trim();
    if v.starts_with('<') && v.ends_with('>') {
        return v[1..v.len() - 1].to_string();
    }
    if let Some(body) = v.strip_prefix('"') {
        for cut in ["\"^^", "\"@", "\""] {
            if let Some(i) = body.find(cut) {
                return body[..i].to_string();
            }
        }
    }
    v.to_string()
}

fn local(iri: &str) -> &str {
    iri.rsplit(['#', '/']).next().unwrap_or(iri)
}

/// `hasGrowthCondition` reads as "has growth condition" in a sentence a
/// person or a model can judge without knowing the vocabulary.
fn humanise(predicate: &str) -> String {
    let name = local(predicate);
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push(' ');
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out.replace('_', " ").to_lowercase()
}

impl SupportChecker {
    pub fn new(graph: Arc<GraphStore>, db: StateDb) -> Self {
        Self { graph, db }
    }

    fn rows(&self, query: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        // Every query here is an internally-authored question about the whole
        // store (which claims cite no source, the source-of map, labels), so it
        // must read the union of all graphs, not the default graph alone. See
        // GraphStore::sparql_select.
        let raw = self.graph.sparql_select_union(query)?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)?;
        Ok(parsed
            .get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default())
    }

    /// Build verification tasks, and report what cites nothing.
    pub fn check(&self, prov_predicate: Option<&str>, limit: usize) -> anyhow::Result<String> {
        let prov = prov_predicate.unwrap_or(PROV_DERIVED);

        // Subjects that cite a source, and what they cite.
        let sourced = self.rows(&format!(
            "SELECT ?s ?src ?srcLabel WHERE {{ ?s <{prov}> ?src . \
             OPTIONAL {{ ?src <http://www.w3.org/2000/01/rdf-schema#label> ?srcLabel }} }} LIMIT 5000"
        ))?;
        let mut source_of: std::collections::BTreeMap<String, (String, Option<String>)> =
            std::collections::BTreeMap::new();
        for row in &sourced {
            if let (Some(s), Some(src)) = (
                row.get("s").and_then(|v| v.as_str()),
                row.get("src").and_then(|v| v.as_str()),
            ) {
                source_of.entry(strip(s)).or_insert((
                    strip(src),
                    row.get("srcLabel").and_then(|v| v.as_str()).map(strip),
                ));
            }
        }

        // The claims themselves: assertions about named things, excluding
        // schema triples and the provenance links.
        let claims = self.rows(&format!(
            "SELECT ?s ?p ?o WHERE {{ ?s ?p ?o . \
             FILTER(isIRI(?s)) \
             FILTER(?p != <{prov}>) \
             FILTER(?p != <http://www.w3.org/2000/01/rdf-schema#label>) \
             FILTER(?p != <http://www.w3.org/2000/01/rdf-schema#subClassOf>) \
             FILTER(?p != <http://www.w3.org/2000/01/rdf-schema#domain>) \
             FILTER(?p != <http://www.w3.org/2000/01/rdf-schema#range>) \
             FILTER(?p != <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                    || (isIRI(?o) && ?o != <http://www.w3.org/2002/07/owl#Class> \
                        && ?o != <http://www.w3.org/2002/07/owl#ObjectProperty> \
                        && ?o != <http://www.w3.org/2002/07/owl#DatatypeProperty>)) \
             }} LIMIT 5000"
        ))?;

        let labels = self.labels()?;
        let show = |iri: &str| -> String {
            labels.get(iri).cloned().unwrap_or_else(|| local(iri).to_string())
        };

        let judged = self.judged_ids()?;
        let mut tasks = Vec::new();
        let mut unsourced = Vec::new();
        let mut total = 0usize;

        for row in &claims {
            let (s, p, o) = match (
                row.get("s").and_then(|v| v.as_str()),
                row.get("p").and_then(|v| v.as_str()),
                row.get("o").and_then(|v| v.as_str()),
            ) {
                (Some(s), Some(p), Some(o)) => (strip(s), strip(p), strip(o)),
                _ => continue,
            };
            total += 1;

            let claim_id = claim_id(&s, &p, &o);
            let object_text = if o.starts_with("http") { show(&o) } else { o.clone() };
            let sentence = format!("{} {} {}.", show(&s), humanise(&p), object_text);

            match source_of.get(&s) {
                None => {
                    if unsourced.len() < limit {
                        unsourced.push(serde_json::json!({
                            "claim_id": claim_id,
                            "claim": sentence,
                        }));
                    }
                }
                Some((src, src_label)) => {
                    if judged.contains(&claim_id) || tasks.len() >= limit {
                        continue;
                    }
                    tasks.push(SupportTask {
                        claim_id,
                        subject: s,
                        predicate: p,
                        object: o,
                        claim: sentence,
                        source: src.clone(),
                        source_label: src_label.clone(),
                    });
                }
            }
        }

        let unsourced_total = total.saturating_sub(
            claims
                .iter()
                .filter(|row| {
                    row.get("s")
                        .and_then(|v| v.as_str())
                        .map(|s| source_of.contains_key(&strip(s)))
                        .unwrap_or(false)
                })
                .count(),
        );

        Ok(serde_json::json!({
            "ok": true,
            "claims_total": total,
            "unsourced_total": unsourced_total,
            "unsourced_rate": if total > 0 {
                (unsourced_total as f64 / total as f64 * 1000.0).round() / 1000.0
            } else { 0.0 },
            "already_judged": judged.len(),
            "tasks": tasks,
            "unsourced": unsourced,
            "instruction": INSTRUCTION,
        })
        .to_string())
    }

    /// Record a verdict for one claim.
    pub fn record_verdict(
        &self,
        claim_id: &str,
        verdict: &str,
        note: Option<&str>,
    ) -> anyhow::Result<String> {
        let verdict = verdict.trim().to_lowercase();
        if !["supported", "refuted", "unrelated"].contains(&verdict.as_str()) {
            return Ok(serde_json::json!({
                "error": "verdict must be one of: supported, refuted, unrelated"
            })
            .to_string());
        }
        let conn = self.db.conn();
        conn.execute(
            "INSERT OR REPLACE INTO support_verdicts (claim_id, verdict, note, timestamp) \
             VALUES (?1, ?2, ?3, datetime('now'))",
            rusqlite::params![claim_id, verdict, note],
        )?;
        Ok(serde_json::json!({"ok": true, "claim_id": claim_id, "verdict": verdict}).to_string())
    }

    /// The two-axis picture: how much of the graph is sourced, and of the
    /// claims judged so far, how many the sources actually bear out.
    pub fn report(&self, prov_predicate: Option<&str>) -> anyhow::Result<String> {
        let check: serde_json::Value = serde_json::from_str(&self.check(prov_predicate, 0)?)?;
        let conn = self.db.conn();
        let mut stmt =
            conn.prepare("SELECT verdict, COUNT(*) FROM support_verdicts GROUP BY verdict")?;
        let mut counts = std::collections::BTreeMap::new();
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (verdict, n) = row?;
            counts.insert(verdict, n);
        }
        let judged: i64 = counts.values().sum();
        let supported = *counts.get("supported").unwrap_or(&0);

        Ok(serde_json::json!({
            "ok": true,
            "claims_total": check.get("claims_total"),
            "unsourced_total": check.get("unsourced_total"),
            "unsourced_rate": check.get("unsourced_rate"),
            "verdicts": counts,
            "judged": judged,
            "support_rate": if judged > 0 {
                (supported as f64 / judged as f64 * 1000.0).round() / 1000.0
            } else { 0.0 },
            "note": "unsourced_rate is computed from the graph. support_rate covers only \
                     the claims judged so far, so read it alongside judged, not alone.",
        })
        .to_string())
    }

    fn judged_ids(&self) -> anyhow::Result<std::collections::BTreeSet<String>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT claim_id FROM support_verdicts")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn labels(&self) -> anyhow::Result<std::collections::HashMap<String, String>> {
        let rows = self.rows(
            "SELECT ?s ?l WHERE { ?s <http://www.w3.org/2000/01/rdf-schema#label> ?l } LIMIT 20000",
        )?;
        let mut out = std::collections::HashMap::new();
        for row in rows {
            if let (Some(s), Some(l)) = (
                row.get("s").and_then(|v| v.as_str()),
                row.get("l").and_then(|v| v.as_str()),
            ) {
                out.entry(strip(s)).or_insert_with(|| strip(l));
            }
        }
        Ok(out)
    }
}

/// A short, stable id for a triple, so verdicts survive reloads and reorders.
fn claim_id(s: &str, p: &str, o: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.update(b"\x1f");
    h.update(p.as_bytes());
    h.update(b"\x1f");
    h.update(o.as_bytes());
    format!("{:x}", h.finalize())[..16].to_string()
}
