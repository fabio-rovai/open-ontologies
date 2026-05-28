//! Full BC+ semantics extensions for the Dynamics layer (#43 follow-up).
//!
//! v0.4 base shipped the deterministic-single-effect subset of BC+
//! (Babb & Lee, arXiv 2506.18044) in `src/dynamics.rs`. This module adds the
//! three semantic pieces previously deferred:
//!
//!   1. **Concurrent actions** — multiple action instances fire in one tick,
//!      with effect-conflict detection.
//!   2. **Static causal laws** — invariants the post-state must always
//!      satisfy (a SPARQL ASK that must hold). Encoded as a separate
//!      registration surface so they survive across actions.
//!   3. **Default values** — caused-by-default triples added to the
//!      pre-state when their condition fires and they aren't already
//!      asserted.
//!
//! ## What's intentionally NOT shipped
//!
//! - **ASP solver backend.** Full BC+ programs compile to Answer Set
//!   Programming for closed-world enumeration of stable models. Wrapping
//!   `clingo` as a subprocess is feasible (same pattern as
//!   `src/plan_classical.rs` / `src/civex_pywhy.rs`) but adds another
//!   external runtime; we ship the in-process semantics first and gate ASP
//!   behind a separate optional feature when a use case lands.
//!
//! - **Non-deterministic outcomes in concurrent actions.** A concurrent
//!   tick currently requires each action to be deterministic OR each
//!   action's sampled outcome to be supplied explicitly via the bindings.
//!   Mixing nondeterminism + concurrency cleanly requires sample-coupled
//!   stable-model enumeration; deferred until #48 PyWhy ships the causal
//!   estimation that would consume those distributions.

use crate::dynamics::{lookup, ActionSchema, EffectSpec};
use crate::graph::GraphStore;
use crate::state::StateDb;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

// ─── Concurrent actions ─────────────────────────────────────────────────────

/// One operator instance in a concurrent tick.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConcurrentStep {
    pub action_name: String,
    #[serde(default)]
    pub bindings: BTreeMap<String, String>,
}

/// Result of applying a concurrent tick.
#[derive(Clone, Debug, Serialize)]
pub struct ConcurrentApplyResult {
    pub steps_total: usize,
    pub steps_applied: usize,
    /// Conflict descriptions when concurrent steps disagree on a triple.
    /// Empty when the tick was conflict-free.
    pub conflicts: Vec<ConflictReport>,
    pub triples_added: usize,
    pub triples_removed: usize,
    /// Invariants (static causal laws) that failed AFTER the tick. Empty
    /// when every registered invariant still holds.
    pub invariant_violations: Vec<String>,
}

/// One conflict between two concurrent steps.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ConflictReport {
    pub triple: (String, String, String),
    pub adding_step: usize,
    pub removing_step: usize,
    pub reason: String,
}

/// Apply a tick of `steps` concurrently. Effects are pre-computed for every
/// step against the **pre-tick** state, then conflict-checked, then committed
/// atomically. If conflicts exist, NO step is applied (atomic-fail).
/// Invariants (registered via [`register_invariant`]) are re-checked after
/// commit; if any fails the tick is rolled back.
pub fn apply_concurrent(
    db: &StateDb,
    graph: &Arc<GraphStore>,
    steps: &[ConcurrentStep],
) -> anyhow::Result<ConcurrentApplyResult> {
    let mut planned_adds: Vec<BTreeSet<(String, String, String)>> = Vec::with_capacity(steps.len());
    let mut planned_removes: Vec<BTreeSet<(String, String, String)>> = Vec::with_capacity(steps.len());

    // ── Step 1: compute each action's effects against the pre-tick state.
    for step in steps {
        let schema: ActionSchema = lookup(db, &step.action_name)?
            .ok_or_else(|| anyhow::anyhow!("unknown action: {}", step.action_name))?;
        let bindings: Vec<(String, String)> = step
            .bindings
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Effects must come from `effects` directly (non-deterministic
        // sampling in concurrent context is deferred — see module docstring).
        if !schema.outcomes.is_empty() {
            anyhow::bail!(
                "non_deterministic_in_concurrent_tick: action `{}` has outcomes; not supported in concurrent ticks. \
                 Pre-sample with apply_with_seed and pass an `effects`-shaped schema instead.",
                step.action_name
            );
        }
        let mut adds = BTreeSet::new();
        let mut removes = BTreeSet::new();
        for effect in &schema.effects {
            let triple = match effect {
                EffectSpec::AddTriple { subject, predicate, object } => {
                    let s = schema.substitute(subject, &bindings);
                    let p = schema.substitute(predicate, &bindings);
                    let o = schema.substitute(object, &bindings);
                    adds.insert((s, p, o));
                    continue;
                }
                EffectSpec::RemoveTriple { subject, predicate, object } => {
                    let s = schema.substitute(subject, &bindings);
                    let p = schema.substitute(predicate, &bindings);
                    let o = schema.substitute(object, &bindings);
                    removes.insert((s, p, o));
                    continue;
                }
                EffectSpec::AddClass { iri } => {
                    let s = schema.substitute(iri, &bindings);
                    (
                        s,
                        "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                        "http://www.w3.org/2002/07/owl#Class".to_string(),
                    )
                }
            };
            adds.insert(triple);
        }
        planned_adds.push(adds);
        planned_removes.push(removes);
    }

    // ── Step 2: detect conflicts (add ∩ remove across distinct steps).
    let mut conflicts: Vec<ConflictReport> = Vec::new();
    for (i, adds) in planned_adds.iter().enumerate() {
        for (j, removes) in planned_removes.iter().enumerate() {
            if i == j {
                continue;
            }
            for t in adds.intersection(removes) {
                conflicts.push(ConflictReport {
                    triple: t.clone(),
                    adding_step: i,
                    removing_step: j,
                    reason: "concurrent add/remove of the same triple".to_string(),
                });
            }
        }
    }
    if !conflicts.is_empty() {
        return Ok(ConcurrentApplyResult {
            steps_total: steps.len(),
            steps_applied: 0,
            conflicts,
            triples_added: 0,
            triples_removed: 0,
            invariant_violations: Vec::new(),
        });
    }

    // ── Step 3: commit. Union of all adds, union of all removes.
    let all_adds: BTreeSet<(String, String, String)> = planned_adds.iter().flatten().cloned().collect();
    let all_removes: BTreeSet<(String, String, String)> = planned_removes.iter().flatten().cloned().collect();

    if !all_adds.is_empty() {
        let mut ttl = String::new();
        for (s, p, o) in &all_adds {
            ttl.push_str(&format!("<{}> <{}> <{}> .\n", s, p, o));
        }
        graph.load_turtle(&ttl, None)?;
    }
    for (s, p, o) in &all_removes {
        let update = format!("DELETE DATA {{ <{}> <{}> <{}> }}", s, p, o);
        let _ = graph.sparql_update(&update);
    }

    // ── Step 4: invariants.
    let violations = check_invariants(db, graph)?;
    if !violations.is_empty() {
        // Rollback: re-add removes, remove adds.
        if !all_removes.is_empty() {
            let mut ttl = String::new();
            for (s, p, o) in &all_removes {
                ttl.push_str(&format!("<{}> <{}> <{}> .\n", s, p, o));
            }
            graph.load_turtle(&ttl, None)?;
        }
        for (s, p, o) in &all_adds {
            let update = format!("DELETE DATA {{ <{}> <{}> <{}> }}", s, p, o);
            let _ = graph.sparql_update(&update);
        }
        return Ok(ConcurrentApplyResult {
            steps_total: steps.len(),
            steps_applied: 0,
            conflicts: Vec::new(),
            triples_added: 0,
            triples_removed: 0,
            invariant_violations: violations,
        });
    }

    Ok(ConcurrentApplyResult {
        steps_total: steps.len(),
        steps_applied: steps.len(),
        conflicts: Vec::new(),
        triples_added: all_adds.len(),
        triples_removed: all_removes.len(),
        invariant_violations: Vec::new(),
    })
}

// ─── Static causal laws (invariants) ────────────────────────────────────────

/// A static causal law: a named invariant the post-state must satisfy. The
/// invariant is encoded as a SPARQL ASK that MUST return true.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StaticCausalLaw {
    pub name: String,
    /// SPARQL ASK query. Must return `true` for the law to hold.
    pub ask_query: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
}

const ENSURE_LAWS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS bcplus_static_laws (
    name TEXT PRIMARY KEY,
    ask_query TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
)";

fn ensure_laws_table(db: &StateDb) -> anyhow::Result<()> {
    db.conn().execute(ENSURE_LAWS_TABLE, [])?;
    Ok(())
}

/// Persist a static causal law. Overwrites any law with the same name.
pub fn register_invariant(db: &StateDb, law: &StaticCausalLaw) -> anyhow::Result<()> {
    ensure_laws_table(db)?;
    db.conn().execute(
        "INSERT OR REPLACE INTO bcplus_static_laws (name, ask_query, description) VALUES (?1, ?2, ?3)",
        rusqlite::params![law.name, law.ask_query, law.description],
    )?;
    Ok(())
}

/// List all registered invariants.
pub fn list_invariants(db: &StateDb) -> anyhow::Result<Vec<StaticCausalLaw>> {
    ensure_laws_table(db)?;
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT name, ask_query, description FROM bcplus_static_laws ORDER BY name",
    )?;
    let rows: Vec<StaticCausalLaw> = stmt
        .query_map([], |r| {
            Ok(StaticCausalLaw {
                name: r.get(0)?,
                ask_query: r.get(1)?,
                description: r.get(2)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// Remove a registered invariant by name.
pub fn remove_invariant(db: &StateDb, name: &str) -> anyhow::Result<bool> {
    ensure_laws_table(db)?;
    let n = db.conn().execute(
        "DELETE FROM bcplus_static_laws WHERE name = ?1",
        rusqlite::params![name],
    )?;
    Ok(n > 0)
}

/// Evaluate every registered invariant against the current graph. Returns
/// the names + descriptions of any that fail.
pub fn check_invariants(db: &StateDb, graph: &Arc<GraphStore>) -> anyhow::Result<Vec<String>> {
    let laws = list_invariants(db)?;
    let mut violations: Vec<String> = Vec::new();
    for law in laws {
        let q = law.ask_query.trim();
        let q = if q.to_uppercase().starts_with("ASK") {
            q.to_string()
        } else {
            format!("ASK {{ {} }}", q)
        };
        let holds = match graph.sparql_select(&q) {
            Ok(s) => s.contains("\"result\":true"),
            Err(_) => false,
        };
        if !holds {
            violations.push(format!(
                "{}: {}",
                law.name,
                law.description
                    .unwrap_or_else(|| "(no description)".to_string())
            ));
        }
    }
    Ok(violations)
}

// ─── Default values ─────────────────────────────────────────────────────────

/// A caused-by-default fact: when `condition_ask` holds, ensure each triple
/// in `defaults` is asserted (added if not already present).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefaultLaw {
    pub name: String,
    /// SPARQL ASK that activates the default.
    pub condition_ask: String,
    /// Triples to assert when the condition fires.
    pub defaults: Vec<(String, String, String)>,
    #[serde(default)]
    pub description: Option<String>,
}

const ENSURE_DEFAULTS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS bcplus_default_laws (
    name TEXT PRIMARY KEY,
    law_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
)";

fn ensure_defaults_table(db: &StateDb) -> anyhow::Result<()> {
    db.conn().execute(ENSURE_DEFAULTS_TABLE, [])?;
    Ok(())
}

/// Register a default-value law.
pub fn register_default(db: &StateDb, law: &DefaultLaw) -> anyhow::Result<()> {
    ensure_defaults_table(db)?;
    let json = serde_json::to_string(law)?;
    db.conn().execute(
        "INSERT OR REPLACE INTO bcplus_default_laws (name, law_json) VALUES (?1, ?2)",
        rusqlite::params![law.name, json],
    )?;
    Ok(())
}

/// List registered default laws.
pub fn list_defaults(db: &StateDb) -> anyhow::Result<Vec<DefaultLaw>> {
    ensure_defaults_table(db)?;
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT law_json FROM bcplus_default_laws ORDER BY name",
    )?;
    let rows: Vec<DefaultLaw> = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok().and_then(|s| serde_json::from_str(&s).ok()))
        .collect();
    Ok(rows)
}

/// Apply every registered default-value law whose condition holds. Adds only
/// triples that don't already exist; idempotent.
pub fn apply_defaults(
    db: &StateDb,
    graph: &Arc<GraphStore>,
) -> anyhow::Result<DefaultsApplyResult> {
    let laws = list_defaults(db)?;
    let mut added: Vec<(String, String, String)> = Vec::new();
    let mut fired: Vec<String> = Vec::new();
    for law in laws {
        let q = law.condition_ask.trim();
        let q = if q.to_uppercase().starts_with("ASK") {
            q.to_string()
        } else {
            format!("ASK {{ {} }}", q)
        };
        let condition_holds = match graph.sparql_select(&q) {
            Ok(s) => s.contains("\"result\":true"),
            Err(_) => false,
        };
        if !condition_holds {
            continue;
        }
        fired.push(law.name.clone());
        for (s, p, o) in &law.defaults {
            // Only add if not already present.
            let exists_q = format!("ASK {{ <{}> <{}> <{}> }}", s, p, o);
            let exists = match graph.sparql_select(&exists_q) {
                Ok(r) => r.contains("\"result\":true"),
                Err(_) => false,
            };
            if !exists {
                let ttl = format!("<{}> <{}> <{}> .\n", s, p, o);
                if graph.load_turtle(&ttl, None).is_ok() {
                    added.push((s.clone(), p.clone(), o.clone()));
                }
            }
        }
    }
    Ok(DefaultsApplyResult { laws_fired: fired, triples_added: added })
}

/// Outcome of [`apply_defaults`].
#[derive(Clone, Debug, Serialize)]
pub struct DefaultsApplyResult {
    pub laws_fired: Vec<String>,
    pub triples_added: Vec<(String, String, String)>,
}

#[allow(dead_code)]
fn in_memory_db() -> anyhow::Result<StateDb> {
    StateDb::open(Path::new(":memory:"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamics::{register, ActionSchema, EffectSpec, Parameter};

    fn fresh_db() -> StateDb {
        StateDb::open(Path::new(":memory:")).unwrap()
    }

    fn fresh_graph() -> Arc<GraphStore> {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix ex: <http://ex.org/> .
            ex:Cat a owl:Class .
            ex:Dog a owl:Class .
            ex:Bird a owl:Class .
        "#,
            None,
        )
        .unwrap();
        g
    }

    fn add_class_schema() -> ActionSchema {
        ActionSchema {
            name: "add_class".to_string(),
            parameters: vec![Parameter { name: "iri".to_string(), type_iri: None }],
            preconditions: vec![],
            effects: vec![EffectSpec::AddClass { iri: "{iri}".to_string() }],
            reversible: true,
            description: None,
            outcomes: vec![],
        }
    }

    #[test]
    fn concurrent_tick_applies_independent_steps_atomically() {
        let db = fresh_db();
        let graph = fresh_graph();
        register(&db, &add_class_schema()).unwrap();
        let mut b1 = BTreeMap::new();
        b1.insert("iri".to_string(), "http://ex.org/Fish".to_string());
        let mut b2 = BTreeMap::new();
        b2.insert("iri".to_string(), "http://ex.org/Reptile".to_string());

        let result = apply_concurrent(
            &db,
            &graph,
            &[
                ConcurrentStep { action_name: "add_class".to_string(), bindings: b1 },
                ConcurrentStep { action_name: "add_class".to_string(), bindings: b2 },
            ],
        )
        .unwrap();
        assert_eq!(result.steps_applied, 2);
        assert_eq!(result.conflicts.len(), 0);
        assert!(result.triples_added >= 2);
        // Both classes must now be declared.
        for iri in ["http://ex.org/Fish", "http://ex.org/Reptile"] {
            let q = format!("ASK {{ <{}> a <http://www.w3.org/2002/07/owl#Class> }}", iri);
            let r = graph.sparql_select(&q).unwrap();
            assert!(r.contains("\"result\":true"), "missing: {}", iri);
        }
    }

    #[test]
    fn concurrent_tick_detects_add_remove_conflict_and_atomically_fails() {
        let db = fresh_db();
        let graph = fresh_graph();
        // Schema A adds triple T; Schema B removes T. Concurrent tick must
        // detect the conflict and apply nothing.
        let add_t = ActionSchema {
            name: "add_t".to_string(),
            parameters: vec![],
            preconditions: vec![],
            effects: vec![EffectSpec::AddTriple {
                subject: "http://ex.org/X".to_string(),
                predicate: "http://ex.org/p".to_string(),
                object: "http://ex.org/Y".to_string(),
            }],
            reversible: true,
            description: None,
            outcomes: vec![],
        };
        let remove_t = ActionSchema {
            name: "remove_t".to_string(),
            parameters: vec![],
            preconditions: vec![],
            effects: vec![EffectSpec::RemoveTriple {
                subject: "http://ex.org/X".to_string(),
                predicate: "http://ex.org/p".to_string(),
                object: "http://ex.org/Y".to_string(),
            }],
            reversible: true,
            description: None,
            outcomes: vec![],
        };
        register(&db, &add_t).unwrap();
        register(&db, &remove_t).unwrap();

        let result = apply_concurrent(
            &db,
            &graph,
            &[
                ConcurrentStep { action_name: "add_t".to_string(), bindings: BTreeMap::new() },
                ConcurrentStep { action_name: "remove_t".to_string(), bindings: BTreeMap::new() },
            ],
        )
        .unwrap();
        assert_eq!(result.steps_applied, 0);
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.triples_added, 0);
        assert_eq!(result.triples_removed, 0);
    }

    #[test]
    fn nondeterministic_action_in_concurrent_tick_is_rejected() {
        // Concurrent ticks require pre-sampled deterministic effects.
        let db = fresh_db();
        let graph = fresh_graph();
        let nondet = ActionSchema {
            name: "noisy".to_string(),
            parameters: vec![],
            preconditions: vec![],
            effects: vec![],
            reversible: true,
            description: None,
            outcomes: vec![
                crate::dynamics::Outcome {
                    probability: 0.5,
                    effects: vec![EffectSpec::AddClass { iri: "http://ex.org/A".to_string() }],
                    label: Some("a".to_string()),
                },
                crate::dynamics::Outcome {
                    probability: 0.5,
                    effects: vec![EffectSpec::AddClass { iri: "http://ex.org/B".to_string() }],
                    label: Some("b".to_string()),
                },
            ],
        };
        register(&db, &nondet).unwrap();
        let err = apply_concurrent(
            &db,
            &graph,
            &[ConcurrentStep { action_name: "noisy".to_string(), bindings: BTreeMap::new() }],
        )
        .expect_err("should reject");
        assert!(format!("{}", err).contains("non_deterministic_in_concurrent_tick"));
    }

    #[test]
    fn static_causal_law_register_and_check_round_trip() {
        let db = fresh_db();
        let graph = fresh_graph();
        let law = StaticCausalLaw {
            name: "cat_must_exist".to_string(),
            ask_query: "ASK { <http://ex.org/Cat> a <http://www.w3.org/2002/07/owl#Class> }".into(),
            description: Some("Cat must always be a class".into()),
        };
        register_invariant(&db, &law).unwrap();
        let violations = check_invariants(&db, &graph).unwrap();
        assert!(violations.is_empty(), "cat exists, no violation expected");

        // Remove Cat → invariant should fire.
        let _ = graph.sparql_update(
            "DELETE DATA { <http://ex.org/Cat> a <http://www.w3.org/2002/07/owl#Class> }",
        );
        let violations = check_invariants(&db, &graph).unwrap();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("cat_must_exist"));
    }

    #[test]
    fn concurrent_tick_rolls_back_when_invariant_violated() {
        let db = fresh_db();
        let graph = fresh_graph();
        // Invariant: Cat must remain a class.
        register_invariant(
            &db,
            &StaticCausalLaw {
                name: "cat_must_exist".to_string(),
                ask_query: "ASK { <http://ex.org/Cat> a <http://www.w3.org/2002/07/owl#Class> }".into(),
                description: None,
            },
        )
        .unwrap();
        // Schema that removes Cat.
        let remove_cat = ActionSchema {
            name: "remove_cat".to_string(),
            parameters: vec![],
            preconditions: vec![],
            effects: vec![EffectSpec::RemoveTriple {
                subject: "http://ex.org/Cat".to_string(),
                predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
                object: "http://www.w3.org/2002/07/owl#Class".to_string(),
            }],
            reversible: true,
            description: None,
            outcomes: vec![],
        };
        register(&db, &remove_cat).unwrap();

        let initial_count = graph.triple_count();
        let result = apply_concurrent(
            &db,
            &graph,
            &[ConcurrentStep { action_name: "remove_cat".to_string(), bindings: BTreeMap::new() }],
        )
        .unwrap();
        assert_eq!(result.steps_applied, 0);
        assert_eq!(result.invariant_violations.len(), 1);
        // Rollback worked: the graph still has the original count.
        assert_eq!(graph.triple_count(), initial_count);
    }

    #[test]
    fn default_law_fires_only_when_condition_holds() {
        let db = fresh_db();
        let graph = fresh_graph();
        // Default: when ex:Cat is a class, assert that Cat has a default label.
        let law = DefaultLaw {
            name: "cat_default_label".to_string(),
            condition_ask:
                "ASK { <http://ex.org/Cat> a <http://www.w3.org/2002/07/owl#Class> }".into(),
            defaults: vec![(
                "http://ex.org/Cat".into(),
                "http://www.w3.org/2000/01/rdf-schema#label".into(),
                "http://ex.org/cat_default_iri".into(),
            )],
            description: None,
        };
        register_default(&db, &law).unwrap();
        let result = apply_defaults(&db, &graph).unwrap();
        assert_eq!(result.laws_fired, vec!["cat_default_label"]);
        assert_eq!(result.triples_added.len(), 1);

        // Idempotent: re-apply, nothing added.
        let result2 = apply_defaults(&db, &graph).unwrap();
        assert_eq!(result2.triples_added.len(), 0);
    }

    #[test]
    fn default_law_does_not_fire_when_condition_is_false() {
        let db = fresh_db();
        let graph = fresh_graph();
        let law = DefaultLaw {
            name: "fish_default_label".to_string(),
            // ex:Fish is NOT in the seed graph.
            condition_ask:
                "ASK { <http://ex.org/Fish> a <http://www.w3.org/2002/07/owl#Class> }".into(),
            defaults: vec![(
                "http://ex.org/Fish".into(),
                "http://www.w3.org/2000/01/rdf-schema#label".into(),
                "http://ex.org/fish_iri".into(),
            )],
            description: None,
        };
        register_default(&db, &law).unwrap();
        let result = apply_defaults(&db, &graph).unwrap();
        assert!(result.laws_fired.is_empty());
        assert!(result.triples_added.is_empty());
    }
}
