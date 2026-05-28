//! ARGOS-style policy governance for state-changing actions (#40, ISWC 2025
//! WOP).
//!
//! Lightweight policy gate that sits between an `onto_certify_action` /
//! `onto_action_apply` call and execution. Policies are SPARQL ASK queries
//! parameterised by the proposed action's target IRIs; each policy returns
//! "allow" or "deny" with a human-readable rationale.
//!
//! ## Why this isn't `onto_certify_action`
//!
//! CIVeX gates on *causal* properties (utility, blast radius, identifiability).
//! ARGOS gates on *policy* properties (who is allowed to touch what, under
//! which clearance, during which window). The two layers compose: CIVeX
//! handles statistical risk, ARGOS handles authorisation.

use crate::graph::GraphStore;
use crate::state::StateDb;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PolicyRule {
    pub name: String,
    /// One of `"allow"` or `"deny"`. The rule's verdict when the SPARQL
    /// ASK returns `true`.
    pub effect: String,
    /// SPARQL ASK. Can use the placeholder `{target}` which is substituted
    /// by each target IRI when the rule is checked.
    pub condition: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyReport {
    /// Overall verdict: `"allow"` iff at least one `allow` rule fires for
    /// every target AND no `deny` rule fires for any target.
    pub verdict: String,
    pub rule_results: Vec<RuleResult>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuleResult {
    pub rule_name: String,
    pub target: String,
    pub effect: String,
    pub fired: bool,
}

const ENSURE: &str = "
CREATE TABLE IF NOT EXISTS policy_rules (
    name TEXT PRIMARY KEY,
    effect TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
    condition TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
)";

fn ensure(db: &StateDb) -> anyhow::Result<()> {
    db.conn().execute(ENSURE, [])?;
    Ok(())
}

pub fn register_rule(db: &StateDb, rule: &PolicyRule) -> anyhow::Result<()> {
    if !matches!(rule.effect.as_str(), "allow" | "deny") {
        anyhow::bail!("invalid effect `{}`: must be allow or deny", rule.effect);
    }
    ensure(db)?;
    db.conn().execute(
        "INSERT OR REPLACE INTO policy_rules (name, effect, condition, description) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![rule.name, rule.effect, rule.condition, rule.description],
    )?;
    Ok(())
}

pub fn list_rules(db: &StateDb) -> anyhow::Result<Vec<PolicyRule>> {
    ensure(db)?;
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT name, effect, condition, description FROM policy_rules ORDER BY name",
    )?;
    let rows: Vec<PolicyRule> = stmt
        .query_map([], |r| {
            Ok(PolicyRule {
                name: r.get(0)?,
                effect: r.get(1)?,
                condition: r.get(2)?,
                description: r.get(3)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// Check a proposed action against every registered policy rule.
///
/// Semantics:
///   - For each (rule, target) pair, substitute `{target}` into the rule's
///     SPARQL ASK and evaluate.
///   - The rule "fires" iff the ASK returns `true`.
///   - Verdict: `"deny"` if any `deny` rule fires for any target;
///     `"allow"` otherwise.
pub fn check_action(
    db: &StateDb,
    graph: &Arc<GraphStore>,
    target_iris: &[String],
) -> anyhow::Result<PolicyReport> {
    let rules = list_rules(db)?;
    let mut results: Vec<RuleResult> = Vec::new();
    let mut any_deny = false;
    for rule in &rules {
        for t in target_iris {
            let q = rule.condition.replace("{target}", t);
            let q = if q.trim().to_uppercase().starts_with("ASK") {
                q
            } else {
                format!("ASK {{ {} }}", q)
            };
            let fired = match graph.sparql_select(&q) {
                Ok(s) => s.contains("\"result\":true"),
                Err(_) => false,
            };
            if fired && rule.effect == "deny" {
                any_deny = true;
            }
            results.push(RuleResult {
                rule_name: rule.name.clone(),
                target: t.clone(),
                effect: rule.effect.clone(),
                fired,
            });
        }
    }
    let verdict = if any_deny { "deny" } else { "allow" }.to_string();
    Ok(PolicyReport { verdict, rule_results: results })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fresh_db() -> StateDb {
        StateDb::open(Path::new(":memory:")).unwrap()
    }

    fn graph_with_lock() -> Arc<GraphStore> {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix ex: <http://ex.org/> .
            @prefix audit: <http://example.org/audit#> .
            ex:Critical a owl:Class ; audit:locked "yes" .
            ex:Free a owl:Class .
        "#,
            None,
        )
        .unwrap();
        g
    }

    #[test]
    fn check_action_allows_when_no_deny_fires() {
        let db = fresh_db();
        let g = graph_with_lock();
        register_rule(
            &db,
            &PolicyRule {
                name: "deny_locked".to_string(),
                effect: "deny".to_string(),
                condition:
                    "ASK { <{target}> <http://example.org/audit#locked> \"yes\" }".to_string(),
                description: None,
            },
        )
        .unwrap();
        let report =
            check_action(&db, &g, &["http://ex.org/Free".to_string()]).unwrap();
        assert_eq!(report.verdict, "allow");
        // The deny rule did NOT fire for Free.
        assert!(report.rule_results.iter().any(|r| r.rule_name == "deny_locked" && !r.fired));
    }

    #[test]
    fn check_action_denies_when_any_deny_fires() {
        let db = fresh_db();
        let g = graph_with_lock();
        register_rule(
            &db,
            &PolicyRule {
                name: "deny_locked".to_string(),
                effect: "deny".to_string(),
                condition:
                    "ASK { <{target}> <http://example.org/audit#locked> \"yes\" }".to_string(),
                description: None,
            },
        )
        .unwrap();
        let report =
            check_action(&db, &g, &["http://ex.org/Critical".to_string()]).unwrap();
        assert_eq!(report.verdict, "deny");
        assert!(report.rule_results.iter().any(|r| r.fired && r.effect == "deny"));
    }

    #[test]
    fn register_rule_rejects_invalid_effect() {
        let db = fresh_db();
        let err = register_rule(
            &db,
            &PolicyRule {
                name: "x".to_string(),
                effect: "maybe".to_string(),
                condition: "ASK { ?s ?p ?o }".to_string(),
                description: None,
            },
        )
        .expect_err("should reject");
        assert!(format!("{}", err).contains("invalid effect"));
    }

    #[test]
    fn check_action_with_no_rules_returns_allow() {
        let db = fresh_db();
        let g = graph_with_lock();
        let report =
            check_action(&db, &g, &["http://ex.org/Free".to_string()]).unwrap();
        assert_eq!(report.verdict, "allow");
        assert!(report.rule_results.is_empty());
    }

    #[test]
    fn target_substitution_replaces_braces() {
        // A rule with multiple target substitutions still works.
        let db = fresh_db();
        let g = graph_with_lock();
        register_rule(
            &db,
            &PolicyRule {
                name: "self_loop_deny".to_string(),
                effect: "deny".to_string(),
                condition:
                    "ASK { <{target}> ?p <{target}> }".to_string(),
                description: None,
            },
        )
        .unwrap();
        // The rule doesn't fire because Critical doesn't have a self-loop.
        let report = check_action(&db, &g, &["http://ex.org/Critical".to_string()]).unwrap();
        assert_eq!(report.verdict, "allow");
    }
}
