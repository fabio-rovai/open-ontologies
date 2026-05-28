//! Dynamics layer — action schemas + applicability + apply (#43).
//!
//! Foundation of the three-layer architecture (Dynamics → Causal → Planner).
//! Provides a structured representation of *what changes* in the ontology:
//!
//! - [`ActionSchema`] — the action's name, typed parameters, SPARQL ASK
//!   preconditions, and KGCL-shaped effect templates with `{param}` substitution
//! - [`register`] / [`lookup`] — persistence in SQLite, keyed by name
//! - [`ActionSchema::applicable`] — evaluate preconditions against the loaded graph
//! - [`ActionSchema::apply`] — execute effects, produce a KGCL patch + IES4 event
//!
//! Designed as the action-representation surface that the Causal engine
//! (`src/civex.rs`) certifies and the Planner (forthcoming) composes.
//!
//! ## Bounded scope (v0.4.0 ship)
//!
//! - Deterministic single-effect actions (BC+ subset)
//! - Preconditions as SPARQL ASK queries; effects as `AddTriple`, `RemoveTriple`,
//!   or the shortcut `AddClass`
//! - `{param}` string substitution into IRIs and literals
//! - IES4 event logging via existing `lineage::LineageLog`
//!
//! Deferred (v0.4.x / v0.5.x):
//!
//! - Non-deterministic dynamics (multiple outcomes per action)
//! - Ramification rules (cascading effects via DL closure)
//! - Concurrent action semantics
//! - ASP solver backend for full BC+

use crate::graph::GraphStore;
use crate::state::StateDb;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A typed parameter binding slot for an [`ActionSchema`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    /// Optional OWL class IRI; if set, the bound IRI must be an instance of
    /// this class for the action to be applicable. (The check is currently
    /// caller-supplied via a precondition SPARQL ASK; future work: enforce
    /// type-checking automatically.)
    #[serde(default)]
    pub type_iri: Option<String>,
}

/// An effect on the ontology. Strings can carry `{param_name}` placeholders
/// that are substituted at apply time.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EffectSpec {
    /// Add a triple `(subject, predicate, object)`. All three positions can
    /// reference parameters.
    AddTriple {
        subject: String,
        predicate: String,
        object: String,
    },
    /// Remove an existing triple.
    RemoveTriple {
        subject: String,
        predicate: String,
        object: String,
    },
    /// Shortcut: declare `iri` as `owl:Class`. Equivalent to
    /// `AddTriple { subject: iri, predicate: rdf:type, object: owl:Class }`.
    AddClass { iri: String },
}

/// One non-deterministic outcome of an action (#49). Each outcome carries
/// a categorical probability and its own effect list. When an
/// [`ActionSchema`] has a non-empty [`ActionSchema::outcomes`], `apply`
/// samples one outcome and executes its effects (the deterministic
/// `effects` field is ignored). When `outcomes` is empty, the schema
/// behaves as before — pure deterministic single-effect BC+ subset.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outcome {
    /// Categorical probability in `[0, 1]`. The sum across all outcomes
    /// must equal 1.0 ± 1e-6 or `apply` returns an error.
    pub probability: f64,
    /// Effects to execute when this outcome is sampled.
    pub effects: Vec<EffectSpec>,
    /// Optional human-readable label (e.g. `"success"`, `"degraded"`,
    /// `"failure"`). Echoed into the `ApplyResult.kgcl_patch_cnl`.
    #[serde(default)]
    pub label: Option<String>,
}

/// A reusable action schema. Reified via [`register`]; instantiated with
/// bindings via [`ActionSchema::apply`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionSchema {
    pub name: String,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    /// SPARQL queries that must succeed for the action to be applicable.
    /// Both `ASK` and `SELECT` are accepted: an `ASK` is satisfied iff its
    /// boolean result is true; a `SELECT` is satisfied iff it returns at
    /// least one row. Strings can contain `{param}` placeholders.
    #[serde(default)]
    pub preconditions: Vec<String>,
    pub effects: Vec<EffectSpec>,
    /// Whether the action has an inverse. Consumed by the Causal engine
    /// (`civex::ActionFrame.reversible`).
    pub reversible: bool,
    /// Optional human-readable description.
    #[serde(default)]
    pub description: Option<String>,
    /// Non-deterministic outcomes (#49). When non-empty, `apply` samples one
    /// outcome and executes its effects; the deterministic `effects` field
    /// is ignored. When empty (default), the schema is deterministic and
    /// behaves as in v0.4 base.
    #[serde(default)]
    pub outcomes: Vec<Outcome>,
}

/// The outcome of applying an action.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApplyResult {
    pub action_name: String,
    pub bindings: Vec<(String, String)>,
    /// KGCL Controlled-Natural-Language description of the patch applied.
    pub kgcl_patch_cnl: Vec<String>,
    /// IES4-style event IRI for the audit trail.
    pub event_iri: String,
    pub triples_added: usize,
    pub triples_removed: usize,
    /// Triples materialised by a follow-up reasoner pass (ramification, #47).
    /// `0` when ramification is disabled or the reasoner produced no new
    /// entailments. The reasoner profile actually used is in
    /// `ramification_profile`.
    #[serde(default)]
    pub derived_triples_added: usize,
    /// Reasoner profile run for ramification, or `None` if disabled.
    #[serde(default)]
    pub ramification_profile: Option<String>,
    /// Index of the sampled outcome when the schema is non-deterministic
    /// (#49). `None` for deterministic schemas (the default case).
    #[serde(default)]
    pub sampled_outcome: Option<usize>,
    /// Label of the sampled outcome, if any (e.g. `"success"`).
    #[serde(default)]
    pub sampled_outcome_label: Option<String>,
}

impl ActionSchema {
    /// Substitute `{param_name}` placeholders in `template` using `bindings`.
    pub fn substitute(&self, template: &str, bindings: &[(String, String)]) -> String {
        let mut s = template.to_string();
        for (k, v) in bindings {
            s = s.replace(&format!("{{{}}}", k), v);
        }
        s
    }

    /// Evaluate all preconditions against the loaded graph with the given
    /// bindings. Returns `true` iff every precondition is satisfied.
    pub fn applicable(&self, graph: &Arc<GraphStore>, bindings: &[(String, String)]) -> bool {
        for precond in &self.preconditions {
            let q = self.substitute(precond, bindings);
            let result_str = match graph.sparql_select(&q) {
                Ok(r) => r,
                Err(_) => return false,
            };
            let parsed: serde_json::Value = match serde_json::from_str(&result_str) {
                Ok(v) => v,
                Err(_) => return false,
            };
            // ASK queries: server returns {"result": bool}.
            if let Some(b) = parsed["result"].as_bool() {
                if !b {
                    return false;
                }
                continue;
            }
            // SELECT queries: satisfied iff at least one binding row.
            let any_row = parsed["results"]
                .as_array()
                .map(|a| !a.is_empty())
                .unwrap_or(false);
            if !any_row {
                return false;
            }
        }
        true
    }

    /// Apply the action's effects. Adds + removes triples, records an IES4-
    /// style event into the lineage log, and returns the KGCL patch CNL
    /// description.
    ///
    /// **Non-deterministic actions (#49):** when [`Self::outcomes`] is
    /// non-empty, an outcome is sampled and its effects are applied. The
    /// sampling seed is derived from `SystemTime::now()`. For reproducible
    /// sampling, use [`Self::apply_with_seed`] instead.
    pub fn apply(
        &self,
        graph: &Arc<GraphStore>,
        db: &StateDb,
        bindings: &[(String, String)],
    ) -> anyhow::Result<ApplyResult> {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        self.apply_inner(graph, db, bindings, seed)
    }

    /// Reproducible variant of [`Self::apply`]: callers supply the sampling
    /// `seed` so non-deterministic outcomes are deterministic given the same
    /// schema + bindings + seed.
    pub fn apply_with_seed(
        &self,
        graph: &Arc<GraphStore>,
        db: &StateDb,
        bindings: &[(String, String)],
        seed: u64,
    ) -> anyhow::Result<ApplyResult> {
        self.apply_inner(graph, db, bindings, seed)
    }

    fn apply_inner(
        &self,
        graph: &Arc<GraphStore>,
        db: &StateDb,
        bindings: &[(String, String)],
        seed: u64,
    ) -> anyhow::Result<ApplyResult> {
        // Decide which effect list to use.
        let (effects_to_apply, sampled_outcome, sampled_label): (&[EffectSpec], Option<usize>, Option<String>) =
            if self.outcomes.is_empty() {
                (self.effects.as_slice(), None, None)
            } else {
                // Validate probabilities sum to ~1.0.
                let total: f64 = self.outcomes.iter().map(|o| o.probability).sum();
                if (total - 1.0).abs() > 1e-6 {
                    anyhow::bail!(
                        "non-deterministic schema `{}` has outcome probabilities summing to {} (must equal 1.0 \u{00b1} 1e-6)",
                        self.name,
                        total
                    );
                }
                if self.outcomes.iter().any(|o| o.probability < 0.0) {
                    anyhow::bail!(
                        "non-deterministic schema `{}` has a negative outcome probability",
                        self.name
                    );
                }
                let idx = sample_categorical(seed, &self.outcomes);
                let outcome = &self.outcomes[idx];
                (outcome.effects.as_slice(), Some(idx), outcome.label.clone())
            };

        let mut to_add: Vec<(String, String, String)> = Vec::new();
        let mut to_remove: Vec<(String, String, String)> = Vec::new();
        let mut kgcl_cnl: Vec<String> = Vec::new();
        if let Some(ref label) = sampled_label {
            kgcl_cnl.push(format!("sampled outcome <{}>", label));
        }

        for effect in effects_to_apply {
            match effect {
                EffectSpec::AddTriple {
                    subject,
                    predicate,
                    object,
                } => {
                    let s = self.substitute(subject, bindings);
                    let p = self.substitute(predicate, bindings);
                    let o = self.substitute(object, bindings);
                    kgcl_cnl.push(format!("create edge <{}> <{}> <{}>", s, p, o));
                    to_add.push((s, p, o));
                }
                EffectSpec::RemoveTriple {
                    subject,
                    predicate,
                    object,
                } => {
                    let s = self.substitute(subject, bindings);
                    let p = self.substitute(predicate, bindings);
                    let o = self.substitute(object, bindings);
                    kgcl_cnl.push(format!("delete edge <{}> <{}> <{}>", s, p, o));
                    to_remove.push((s, p, o));
                }
                EffectSpec::AddClass { iri } => {
                    let s = self.substitute(iri, bindings);
                    let p = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string();
                    let o = "http://www.w3.org/2002/07/owl#Class".to_string();
                    kgcl_cnl.push(format!("create class <{}>", s));
                    to_add.push((s, p, o));
                }
            }
        }

        // Apply additions via Turtle insertion.
        if !to_add.is_empty() {
            let mut turtle = String::new();
            for (s, p, o) in &to_add {
                turtle.push_str(&format!("<{}> <{}> <{}> .\n", s, p, o));
            }
            graph.load_turtle(&turtle, None)?;
        }

        // Apply removals via SPARQL UPDATE.
        for (s, p, o) in &to_remove {
            let update = format!("DELETE DATA {{ <{}> <{}> <{}> }}", s, p, o);
            let _ = graph.sparql_update(&update);
        }

        // IES4-style event IRI for the audit trail.
        let timestamp_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        let event_iri = format!(
            "http://open-ontologies.local/dynamics/event/{}_{}",
            self.name, timestamp_micros
        );

        // Log to lineage as a Dynamics-layer event.
        let lineage = crate::lineage::LineageLog::new(db.clone());
        lineage.record(
            "dynamics",
            "DY",
            &format!("apply:{}", self.name),
            &format!("added={}, removed={}", to_add.len(), to_remove.len()),
        );

        Ok(ApplyResult {
            action_name: self.name.clone(),
            bindings: bindings.to_vec(),
            kgcl_patch_cnl: kgcl_cnl,
            event_iri,
            triples_added: to_add.len(),
            triples_removed: to_remove.len(),
            derived_triples_added: 0,
            ramification_profile: None,
            sampled_outcome,
            sampled_outcome_label: sampled_label,
        })
    }

    /// Apply the action's effects, then run the reasoner to materialise any
    /// downstream entailments (ramification, #47). The resulting `ApplyResult`
    /// carries `derived_triples_added` as the count of new triples the
    /// reasoner produced beyond the literal effects.
    ///
    /// `profile` is forwarded to [`crate::reason::Reasoner::run`]; passing
    /// `"rdfs"` is the cheapest meaningful choice, `"owl-rl"` is the standard,
    /// `"owl-rl-ext"` adds restriction-pattern entailment, and `"owl-dl"`
    /// delegates to the tableaux reasoner.
    pub fn apply_with_ramification(
        &self,
        graph: &Arc<GraphStore>,
        db: &StateDb,
        bindings: &[(String, String)],
        profile: &str,
    ) -> anyhow::Result<ApplyResult> {
        let mut result = self.apply(graph, db, bindings)?;
        let pre_count = graph.triple_count();
        // Materialise entailments in-place.
        let _ = crate::reason::Reasoner::run(graph, profile, true)?;
        let post_count = graph.triple_count();
        result.derived_triples_added = post_count.saturating_sub(pre_count);
        result.ramification_profile = Some(profile.to_string());
        Ok(result)
    }
}

// ─── Outcome sampling helpers (#49) ─────────────────────────────────────────

/// Tiny inline xorshift64 PRNG. Deterministic given the seed; sufficient for
/// categorical sampling over a handful of outcomes. Avoids adding `rand` /
/// `fastrand` as deps just for this.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    if x == 0 {
        x = 0x9E3779B97F4A7C15; // arbitrary non-zero
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Sample one outcome index from a categorical distribution given the
/// `outcomes` (each carries `probability`) and a `seed`. Probabilities are
/// assumed to be validated (sum == 1.0, non-negative) by the caller.
fn sample_categorical(seed: u64, outcomes: &[Outcome]) -> usize {
    let mut state = seed;
    let r_u64 = xorshift64(&mut state);
    // Map to a uniform [0, 1) by taking the high 53 bits (mantissa width of f64).
    let r = (r_u64 >> 11) as f64 / (1u64 << 53) as f64;
    let mut cumulative = 0.0;
    for (i, o) in outcomes.iter().enumerate() {
        cumulative += o.probability;
        if r < cumulative {
            return i;
        }
    }
    // Floating-point slop on the boundary; default to the last outcome.
    outcomes.len() - 1
}

// ─── SQLite-backed schema persistence ───────────────────────────────────────

const ENSURE_TABLE: &str = "
CREATE TABLE IF NOT EXISTS dynamics_action_schemas (
    name TEXT PRIMARY KEY,
    schema_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
)";

fn ensure_table(db: &StateDb) -> anyhow::Result<()> {
    let conn = db.conn();
    conn.execute(ENSURE_TABLE, [])?;
    Ok(())
}

/// Persist an action schema under its name. Replaces any existing schema with
/// the same name.
pub fn register(db: &StateDb, schema: &ActionSchema) -> anyhow::Result<()> {
    ensure_table(db)?;
    let json = serde_json::to_string(schema)?;
    let conn = db.conn();
    conn.execute(
        "INSERT OR REPLACE INTO dynamics_action_schemas (name, schema_json) VALUES (?1, ?2)",
        rusqlite::params![schema.name, json],
    )?;
    Ok(())
}

/// Look up a registered schema by name. Returns `Ok(None)` if not found.
pub fn lookup(db: &StateDb, name: &str) -> anyhow::Result<Option<ActionSchema>> {
    ensure_table(db)?;
    let conn = db.conn();
    let row: Option<String> = conn
        .query_row(
            "SELECT schema_json FROM dynamics_action_schemas WHERE name = ?1",
            rusqlite::params![name],
            |r| r.get(0),
        )
        .ok();
    match row {
        Some(s) => Ok(Some(serde_json::from_str(&s)?)),
        None => Ok(None),
    }
}

/// List all registered schema names.
pub fn list_names(db: &StateDb) -> anyhow::Result<Vec<String>> {
    ensure_table(db)?;
    let conn = db.conn();
    let mut stmt = conn.prepare("SELECT name FROM dynamics_action_schemas ORDER BY name")?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> StateDb {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        StateDb::open(&path).unwrap()
    }

    fn sample_schema() -> ActionSchema {
        ActionSchema {
            name: "rename_class".to_string(),
            parameters: vec![
                Parameter {
                    name: "old".to_string(),
                    type_iri: Some("http://www.w3.org/2002/07/owl#Class".to_string()),
                },
                Parameter {
                    name: "new".to_string(),
                    type_iri: None,
                },
            ],
            preconditions: vec![
                "SELECT ?x WHERE { <{old}> ?p ?o }".to_string(),
            ],
            effects: vec![
                EffectSpec::AddClass { iri: "{new}".to_string() },
                EffectSpec::RemoveTriple {
                    subject: "{old}".to_string(),
                    predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                    object: "http://www.w3.org/2002/07/owl#Class".to_string(),
                },
            ],
            reversible: false,
            description: Some("Replace class {old} with {new}".to_string()),
            outcomes: vec![],
        }
    }

    #[test]
    fn register_lookup_round_trip() {
        let db = test_db();
        let schema = sample_schema();
        register(&db, &schema).expect("register");
        let recovered = lookup(&db, "rename_class").expect("lookup").expect("found");
        assert_eq!(recovered.name, schema.name);
        assert_eq!(recovered.parameters.len(), 2);
        assert_eq!(recovered.effects.len(), 2);
        assert!(!recovered.reversible);
        assert!(recovered.description.as_deref().unwrap().contains("Replace"));
    }

    #[test]
    fn lookup_missing_returns_none() {
        let db = test_db();
        assert!(lookup(&db, "does_not_exist").unwrap().is_none());
    }

    #[test]
    fn list_names_returns_sorted() {
        let db = test_db();
        let mut a = sample_schema();
        a.name = "zebra".into();
        register(&db, &a).unwrap();
        let mut b = sample_schema();
        b.name = "alpha".into();
        register(&db, &b).unwrap();
        let names = list_names(&db).unwrap();
        assert_eq!(names, vec!["alpha", "zebra"]);
    }

    #[test]
    fn substitute_replaces_braced_placeholders() {
        let s = sample_schema();
        let out = s.substitute(
            "<{old}> a <{new}>",
            &[
                ("old".to_string(), "http://ex.org/A".to_string()),
                ("new".to_string(), "http://ex.org/B".to_string()),
            ],
        );
        assert_eq!(out, "<http://ex.org/A> a <http://ex.org/B>");
    }

    #[test]
    fn applicable_returns_true_when_select_has_rows() {
        let db = test_db();
        let graph = Arc::new(GraphStore::new());
        graph
            .load_turtle(
                r#"
                @prefix owl: <http://www.w3.org/2002/07/owl#> .
                @prefix ex:  <http://ex.org/> .
                ex:Cat a owl:Class .
            "#,
                None,
            )
            .unwrap();

        let schema = sample_schema();
        let _ = db; // unused but kept for parity with apply tests
        let applicable = schema.applicable(
            &graph,
            &[("old".to_string(), "http://ex.org/Cat".to_string())],
        );
        assert!(applicable, "precondition should match an existing class");
    }

    #[test]
    fn applicable_returns_false_when_select_empty() {
        let graph = Arc::new(GraphStore::new());
        graph
            .load_turtle("@prefix ex: <http://ex.org/> . ex:Dog a ex:Animal .", None)
            .unwrap();
        let schema = sample_schema();
        let applicable = schema.applicable(
            &graph,
            &[("old".to_string(), "http://ex.org/Missing".to_string())],
        );
        assert!(!applicable);
    }

    fn nondet_two_outcome_schema() -> ActionSchema {
        // 70/30 split between two distinct AddClass outcomes; reproducible
        // under any deterministic seed.
        ActionSchema {
            name: "noisy_add".to_string(),
            parameters: vec![],
            preconditions: vec![],
            effects: vec![], // ignored when outcomes is non-empty
            reversible: false,
            description: None,
            outcomes: vec![
                Outcome {
                    probability: 0.7,
                    effects: vec![EffectSpec::AddClass {
                        iri: "http://ex.org/Success".to_string(),
                    }],
                    label: Some("success".to_string()),
                },
                Outcome {
                    probability: 0.3,
                    effects: vec![EffectSpec::AddClass {
                        iri: "http://ex.org/Failure".to_string(),
                    }],
                    label: Some("failure".to_string()),
                },
            ],
        }
    }

    #[test]
    fn nondeterministic_apply_with_seed_is_reproducible() {
        // Same schema + same seed must produce the same sampled outcome and
        // the same triple-pattern.
        let db = test_db();
        let g1 = Arc::new(GraphStore::new());
        let g2 = Arc::new(GraphStore::new());
        let schema = nondet_two_outcome_schema();

        let r1 = schema.apply_with_seed(&g1, &db, &[], 42).expect("apply 1");
        let r2 = schema.apply_with_seed(&g2, &db, &[], 42).expect("apply 2");

        assert_eq!(r1.sampled_outcome, r2.sampled_outcome,
            "same seed must sample the same outcome");
        assert_eq!(r1.sampled_outcome_label, r2.sampled_outcome_label);
    }

    #[test]
    fn nondeterministic_apply_distribution_matches_probabilities() {
        // Over 1000 seeded calls the 70/30 split should be within reasonable
        // bounds. This is a smoke test of the categorical sampler, not a
        // rigorous statistical test.
        let db = test_db();
        let schema = nondet_two_outcome_schema();
        let mut counts = [0usize; 2];
        for seed in 0u64..1000 {
            let g = Arc::new(GraphStore::new());
            let r = schema
                .apply_with_seed(&g, &db, &[], seed.wrapping_mul(0x9E37_79B9_7F4A_7C15))
                .expect("apply");
            let idx = r.sampled_outcome.expect("nondet schema must record outcome");
            counts[idx] += 1;
        }
        let p0 = counts[0] as f64 / 1000.0;
        // 70% outcome should land between 60% and 80% with very high probability.
        assert!(
            (0.60..=0.80).contains(&p0),
            "expected ~70% outcome 0, got {:.3} (counts: {:?})",
            p0,
            counts
        );
    }

    #[test]
    fn nondeterministic_apply_rejects_invalid_probability_sum() {
        let db = test_db();
        let g = Arc::new(GraphStore::new());
        let mut schema = nondet_two_outcome_schema();
        // Break the invariant: sum to 0.9, not 1.0.
        schema.outcomes[0].probability = 0.6;
        schema.outcomes[1].probability = 0.3;
        let err = schema.apply_with_seed(&g, &db, &[], 0).expect_err("should reject");
        let s = format!("{}", err);
        assert!(s.contains("summing to"), "expected probability sum error, got: {}", s);
    }

    #[test]
    fn deterministic_schema_still_works_when_outcomes_is_empty() {
        // Back-compat: a schema with empty outcomes uses the existing
        // effects field and reports sampled_outcome=None.
        let db = test_db();
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            r#"@prefix owl: <http://www.w3.org/2002/07/owl#> .
               @prefix ex: <http://ex.org/> .
               ex:Cat a owl:Class ."#,
            None,
        ).unwrap();

        let schema = sample_schema();
        let r = schema
            .apply(
                &g,
                &db,
                &[
                    ("old".to_string(), "http://ex.org/Cat".to_string()),
                    ("new".to_string(), "http://ex.org/Feline".to_string()),
                ],
            )
            .expect("apply");
        assert!(r.sampled_outcome.is_none());
        assert!(r.sampled_outcome_label.is_none());
        assert_eq!(r.triples_added, 1);
        assert_eq!(r.triples_removed, 1);
    }

    #[test]
    fn apply_with_ramification_materialises_subclass_entailment() {
        // Acceptance criterion from #47: adding a subClassOf edge between
        // two classes where the child already has an instance must, under
        // OWL-RL ramification, also assert the instance as a member of the
        // parent class.
        let db = test_db();
        let graph = Arc::new(GraphStore::new());
        graph
            .load_turtle(
                r#"
                @prefix owl: <http://www.w3.org/2002/07/owl#> .
                @prefix ex:  <http://ex.org/> .
                ex:Animal a owl:Class .
                ex:Cat a owl:Class .
                ex:tigger a ex:Cat .
            "#,
                None,
            )
            .unwrap();

        let schema = ActionSchema {
            name: "add_subclass_edge".to_string(),
            parameters: vec![
                Parameter { name: "child".to_string(), type_iri: None },
                Parameter { name: "parent".to_string(), type_iri: None },
            ],
            preconditions: vec![],
            effects: vec![EffectSpec::AddTriple {
                subject: "{child}".to_string(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
                object: "{parent}".to_string(),
            }],
            reversible: true,
            description: None,
            outcomes: vec![],
        };
        let bindings = vec![
            ("child".to_string(), "http://ex.org/Cat".to_string()),
            ("parent".to_string(), "http://ex.org/Animal".to_string()),
        ];

        let result = schema
            .apply_with_ramification(&graph, &db, &bindings, "owl-rl")
            .expect("apply+ramify");

        assert_eq!(result.triples_added, 1, "literal effect: one subClassOf edge");
        assert_eq!(result.ramification_profile.as_deref(), Some("owl-rl"));
        // The reasoner must have derived at least one new triple (tigger a Animal).
        assert!(
            result.derived_triples_added >= 1,
            "OWL-RL should derive at least `tigger a Animal`; derived={}",
            result.derived_triples_added
        );

        // Verify the entailment landed in the graph.
        let q = "ASK { <http://ex.org/tigger> a <http://ex.org/Animal> }";
        let r = graph.sparql_select(q).unwrap();
        assert!(
            r.contains("\"result\":true"),
            "expected tigger a Animal to be entailed under OWL-RL; got: {}",
            r
        );
    }

    #[test]
    fn apply_without_ramification_leaves_derived_count_zero() {
        let db = test_db();
        let graph = Arc::new(GraphStore::new());
        graph
            .load_turtle(
                r#"
                @prefix owl: <http://www.w3.org/2002/07/owl#> .
                @prefix ex:  <http://ex.org/> .
                ex:Cat a owl:Class .
            "#,
                None,
            )
            .unwrap();

        let schema = sample_schema();
        let result = schema
            .apply(
                &graph,
                &db,
                &[
                    ("old".to_string(), "http://ex.org/Cat".to_string()),
                    ("new".to_string(), "http://ex.org/Feline".to_string()),
                ],
            )
            .expect("apply");

        assert_eq!(result.derived_triples_added, 0);
        assert!(result.ramification_profile.is_none());
    }

    #[test]
    fn apply_adds_and_removes_triples_returns_kgcl_patch() {
        let db = test_db();
        let graph = Arc::new(GraphStore::new());
        graph
            .load_turtle(
                r#"
                @prefix owl: <http://www.w3.org/2002/07/owl#> .
                @prefix ex:  <http://ex.org/> .
                ex:Cat a owl:Class .
            "#,
                None,
            )
            .unwrap();

        let schema = sample_schema();
        let result = schema
            .apply(
                &graph,
                &db,
                &[
                    ("old".to_string(), "http://ex.org/Cat".to_string()),
                    ("new".to_string(), "http://ex.org/Feline".to_string()),
                ],
            )
            .expect("apply");

        assert_eq!(result.action_name, "rename_class");
        assert_eq!(result.triples_added, 1, "AddClass effect");
        assert_eq!(result.triples_removed, 1, "RemoveTriple effect");
        assert!(result.kgcl_patch_cnl.iter().any(|s| s.contains("Feline")));
        assert!(result.kgcl_patch_cnl.iter().any(|s| s.contains("delete edge")));
        assert!(result.event_iri.contains("rename_class"));

        // Verify the graph actually reflects the change.
        let q = "ASK WHERE { <http://ex.org/Feline> a <http://www.w3.org/2002/07/owl#Class> }";
        let result_json = graph.sparql_select(q).unwrap();
        assert!(result_json.contains("\"result\":true"));
    }
}
