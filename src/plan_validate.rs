//! Plan validator (#45, LLM-Modulo `Validator` role).
//!
//! Companion to [`crate::plan_pddl::compile_domain`]. The compile primitive
//! emits PDDL for an external solver (the orchestrator's responsibility per
//! the LLM-Modulo convention); this validator consumes the solver's output
//! and checks step-by-step that each operator is applicable to the cumulative
//! state and that its effects compose without contradiction.
//!
//! ## What validation means here
//!
//! Given a sequence of `(action_name, bindings)` operator instances, the
//! validator:
//!
//! 1. Forks the loaded graph into an isolated sandbox so the real store is
//!    not mutated.
//! 2. For each step in order:
//!    a. Looks up the registered [`crate::dynamics::ActionSchema`].
//!    b. Re-evaluates the schema's preconditions against the cumulative
//!    sandbox state under the step's bindings.
//!    c. If applicable, executes the schema's effects against the sandbox.
//!    d. If not applicable, returns immediately with the failing step index
//!    and a diagnostic, leaving subsequent steps unevaluated.
//! 3. Returns the validation report — valid + final triple count, or invalid
//!    + first failing step.
//!
//! Goal triples (optional) are checked after the final step: every goal
//! `(s, p, o)` must be derivable as a SPARQL ASK over the sandbox. Missing
//! goals do NOT invalidate the plan; they are reported in `unsatisfied_goals`
//! so the orchestrator can iterate.
//!
//! ## Why this matters
//!
//! Per LLM-Modulo (Kambhampati arXiv 2402.01817), the LLM/orchestrator can
//! produce candidate plans (via Fast Downward, prompting, or other means);
//! the server stays the *validator*. This module is exactly that primitive —
//! it does not solve, it judges.

use crate::dynamics::{lookup, ActionSchema};
use crate::graph::GraphStore;
use crate::state::StateDb;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// One operator instance in a candidate plan.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlanStep {
    pub action_name: String,
    #[serde(default)]
    pub bindings: BTreeMap<String, String>,
}

/// Outcome of validating a candidate plan.
#[derive(Clone, Debug, Serialize)]
pub struct PlanValidationResult {
    pub valid: bool,
    pub steps_total: usize,
    pub steps_validated: usize,
    /// Set only when `valid == false`: zero-based index of the first step that
    /// failed.
    pub failed_at_step: Option<usize>,
    /// Set only when `valid == false`: human-readable diagnostic.
    pub failure_reason: Option<String>,
    pub initial_triple_count: usize,
    pub final_triple_count: usize,
    /// Goal facts that did NOT hold after running all steps; empty when no
    /// goals were supplied, or when every goal was satisfied. Even an invalid
    /// plan can satisfy SOME goals, so this is independent of `valid`.
    pub unsatisfied_goals: Vec<(String, String, String)>,
    /// Per-step effect tallies (added + removed triples), in order.
    pub per_step_added: Vec<usize>,
    pub per_step_removed: Vec<usize>,
}

/// Clone the loaded graph into a fresh sandbox `GraphStore`. The original
/// `graph` is not mutated.
fn snapshot_into_sandbox(graph: &Arc<GraphStore>) -> anyhow::Result<Arc<GraphStore>> {
    let triples = graph.all_triples()?;
    let sandbox = Arc::new(GraphStore::new());
    if triples.is_empty() {
        return Ok(sandbox);
    }
    // `all_triples` returns N-Triple-formatted positions (e.g. `<iri>`,
    // `"literal"`, `_:bn`), which are accepted by `load_turtle` as a subset
    // of Turtle. Concatenate into one document.
    let mut nt = String::with_capacity(triples.len() * 64);
    for (s, p, o) in &triples {
        nt.push_str(s);
        nt.push(' ');
        nt.push_str(p);
        nt.push(' ');
        nt.push_str(o);
        nt.push_str(" .\n");
    }
    sandbox.load_turtle(&nt, None)?;
    Ok(sandbox)
}

/// Validate a candidate plan against the loaded graph. Does NOT mutate the
/// original graph — all simulation happens in a forked sandbox.
///
/// `goal_facts` is an optional list of `(s, p, o)` IRIs/literals (already in
/// N-Triple-position form, e.g. `<http://ex.org/Cat>`) that must hold in the
/// post-state for the plan to fully achieve its goal.
pub fn validate_plan(
    db: &StateDb,
    graph: &Arc<GraphStore>,
    steps: &[PlanStep],
    goal_facts: &[(String, String, String)],
) -> anyhow::Result<PlanValidationResult> {
    let sandbox = snapshot_into_sandbox(graph)?;
    let initial_triple_count = sandbox.triple_count();

    let mut per_step_added: Vec<usize> = Vec::with_capacity(steps.len());
    let mut per_step_removed: Vec<usize> = Vec::with_capacity(steps.len());

    // Scratch DB so per-step lineage entries don't pollute the production
    // StateDb. Uses SQLite's in-memory mode — no file is created, the DB is
    // discarded when the function returns. The lookup() call still uses the
    // real db — schemas live in production state.
    let scratch_db = StateDb::open(std::path::Path::new(":memory:"))?;

    for (i, step) in steps.iter().enumerate() {
        let schema: ActionSchema = match lookup(db, &step.action_name)? {
            Some(s) => s,
            None => {
                return Ok(PlanValidationResult {
                    valid: false,
                    steps_total: steps.len(),
                    steps_validated: i,
                    failed_at_step: Some(i),
                    failure_reason: Some(format!(
                        "unknown action `{}` at step {}",
                        step.action_name, i
                    )),
                    initial_triple_count,
                    final_triple_count: sandbox.triple_count(),
                    unsatisfied_goals: goal_facts.to_vec(),
                    per_step_added,
                    per_step_removed,
                });
            }
        };
        let bindings: Vec<(String, String)> = step
            .bindings
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        if !schema.applicable(&sandbox, &bindings) {
            return Ok(PlanValidationResult {
                valid: false,
                steps_total: steps.len(),
                steps_validated: i,
                failed_at_step: Some(i),
                failure_reason: Some(format!(
                    "preconditions not satisfied at step {} (`{}`)",
                    i, step.action_name
                )),
                initial_triple_count,
                final_triple_count: sandbox.triple_count(),
                unsatisfied_goals: goal_facts.to_vec(),
                per_step_added,
                per_step_removed,
            });
        }
        let outcome = schema.apply(&sandbox, &scratch_db, &bindings)?;
        per_step_added.push(outcome.triples_added);
        per_step_removed.push(outcome.triples_removed);
    }

    // All steps applied. Now check goals.
    let mut unsatisfied: Vec<(String, String, String)> = Vec::new();
    for (s, p, o) in goal_facts {
        let q = format!("ASK {{ {} {} {} }}", s, p, o);
        let satisfied = match sandbox.sparql_select(&q) {
            Ok(r) => r.contains("\"result\":true"),
            Err(_) => false,
        };
        if !satisfied {
            unsatisfied.push((s.clone(), p.clone(), o.clone()));
        }
    }

    Ok(PlanValidationResult {
        valid: true,
        steps_total: steps.len(),
        steps_validated: steps.len(),
        failed_at_step: None,
        failure_reason: None,
        initial_triple_count,
        final_triple_count: sandbox.triple_count(),
        unsatisfied_goals: unsatisfied,
        per_step_added,
        per_step_removed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamics::{register, ActionSchema, EffectSpec, Parameter};

    fn test_db() -> StateDb {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        StateDb::open(&path).unwrap()
    }

    fn add_subclass_schema() -> ActionSchema {
        ActionSchema {
            name: "add_subclass_edge".to_string(),
            parameters: vec![
                Parameter { name: "child".to_string(), type_iri: None },
                Parameter { name: "parent".to_string(), type_iri: None },
            ],
            preconditions: vec![
                "ASK { <{child}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> }".to_string(),
                "ASK { <{parent}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> }".to_string(),
            ],
            effects: vec![EffectSpec::AddTriple {
                subject: "{child}".to_string(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
                object: "{parent}".to_string(),
            }],
            reversible: true,
            description: None,
            outcomes: vec![],
        }
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

    fn graph_with_two_classes() -> Arc<GraphStore> {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix ex: <http://ex.org/> .
            ex:Animal a owl:Class .
            ex:Cat a owl:Class .
        "#,
            None,
        )
        .unwrap();
        g
    }

    #[test]
    fn validate_empty_plan_is_valid_and_satisfies_no_goals() {
        let db = test_db();
        let graph = graph_with_two_classes();
        let result = validate_plan(&db, &graph, &[], &[]).unwrap();
        assert!(result.valid);
        assert_eq!(result.steps_validated, 0);
        assert_eq!(result.initial_triple_count, result.final_triple_count);
        assert!(result.unsatisfied_goals.is_empty());
    }

    #[test]
    fn validate_single_step_with_satisfied_preconditions_succeeds() {
        let db = test_db();
        let graph = graph_with_two_classes();
        register(&db, &add_subclass_schema()).unwrap();

        let mut bindings = BTreeMap::new();
        bindings.insert("child".to_string(), "http://ex.org/Cat".to_string());
        bindings.insert("parent".to_string(), "http://ex.org/Animal".to_string());

        let steps = vec![PlanStep {
            action_name: "add_subclass_edge".to_string(),
            bindings,
        }];
        let result = validate_plan(&db, &graph, &steps, &[]).unwrap();

        assert!(result.valid);
        assert_eq!(result.steps_validated, 1);
        assert_eq!(result.per_step_added, vec![1]);
        assert_eq!(result.per_step_removed, vec![0]);
        // Original graph must be untouched.
        let post_q = graph
            .sparql_select("ASK { <http://ex.org/Cat> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://ex.org/Animal> }")
            .unwrap();
        assert!(
            !post_q.contains("\"result\":true"),
            "original graph should not have been mutated by validation"
        );
    }

    #[test]
    fn validate_step_with_missing_action_reports_failure() {
        let db = test_db();
        let graph = graph_with_two_classes();
        let steps = vec![PlanStep {
            action_name: "no_such_action".to_string(),
            bindings: BTreeMap::new(),
        }];
        let result = validate_plan(&db, &graph, &steps, &[]).unwrap();
        assert!(!result.valid);
        assert_eq!(result.failed_at_step, Some(0));
        assert!(result.failure_reason.unwrap().contains("unknown action"));
        assert_eq!(result.steps_validated, 0);
    }

    #[test]
    fn validate_step_with_unsatisfied_precondition_reports_failure() {
        let db = test_db();
        let graph = graph_with_two_classes();
        register(&db, &add_subclass_schema()).unwrap();

        let mut bindings = BTreeMap::new();
        bindings.insert("child".to_string(), "http://ex.org/Cat".to_string());
        // parent is NOT declared as owl:Class in the seed graph.
        bindings.insert("parent".to_string(), "http://ex.org/UnknownThing".to_string());

        let steps = vec![PlanStep {
            action_name: "add_subclass_edge".to_string(),
            bindings,
        }];
        let result = validate_plan(&db, &graph, &steps, &[]).unwrap();
        assert!(!result.valid);
        assert_eq!(result.failed_at_step, Some(0));
        assert!(result
            .failure_reason
            .as_ref()
            .unwrap()
            .contains("preconditions not satisfied"));
    }

    #[test]
    fn validate_multi_step_plan_chains_state_through() {
        // Step 1 establishes the precondition that Step 2 needs.
        let db = test_db();
        let graph = graph_with_two_classes();
        register(&db, &add_class_schema()).unwrap();
        register(&db, &add_subclass_schema()).unwrap();

        // Step 1: declare ex:Feline as a class.
        let mut s1 = BTreeMap::new();
        s1.insert("iri".to_string(), "http://ex.org/Feline".to_string());
        // Step 2: ex:Cat rdfs:subClassOf ex:Feline (which only exists after step 1).
        let mut s2 = BTreeMap::new();
        s2.insert("child".to_string(), "http://ex.org/Cat".to_string());
        s2.insert("parent".to_string(), "http://ex.org/Feline".to_string());

        let steps = vec![
            PlanStep { action_name: "add_class".to_string(), bindings: s1 },
            PlanStep { action_name: "add_subclass_edge".to_string(), bindings: s2 },
        ];
        let result = validate_plan(&db, &graph, &steps, &[]).unwrap();
        assert!(
            result.valid,
            "step 2 should pass because step 1 establishes its precondition; reason: {:?}",
            result.failure_reason
        );
        assert_eq!(result.steps_validated, 2);
    }

    #[test]
    fn validate_reports_unsatisfied_goals_without_invalidating_plan() {
        let db = test_db();
        let graph = graph_with_two_classes();
        register(&db, &add_subclass_schema()).unwrap();

        let mut bindings = BTreeMap::new();
        bindings.insert("child".to_string(), "http://ex.org/Cat".to_string());
        bindings.insert("parent".to_string(), "http://ex.org/Animal".to_string());

        let steps = vec![PlanStep {
            action_name: "add_subclass_edge".to_string(),
            bindings,
        }];
        // Goal that the plan DOES achieve.
        let achieved = (
            "<http://ex.org/Cat>".to_string(),
            "<http://www.w3.org/2000/01/rdf-schema#subClassOf>".to_string(),
            "<http://ex.org/Animal>".to_string(),
        );
        // Goal that the plan does NOT achieve (no such edge produced).
        let missing = (
            "<http://ex.org/Cat>".to_string(),
            "<http://www.w3.org/2000/01/rdf-schema#subClassOf>".to_string(),
            "<http://ex.org/Mammal>".to_string(),
        );
        let result = validate_plan(&db, &graph, &steps, &[achieved, missing.clone()]).unwrap();
        assert!(result.valid, "plan itself is well-formed");
        assert_eq!(result.unsatisfied_goals.len(), 1);
        assert_eq!(result.unsatisfied_goals[0], missing);
    }
}
