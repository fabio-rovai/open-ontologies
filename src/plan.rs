use crate::graph::GraphStore;
use crate::monitor::Monitor;
use crate::state::StateDb;
use rusqlite::OptionalExtension;
use std::collections::HashSet;
use std::sync::Arc;

/// Minimum name/label similarity before `migrate` will assert that one term
/// replaces another.
///
/// `owl:equivalentClass` is a hard logical assertion, so a wrong bridge is
/// worse than a missing one: downstream reasoners believe it. Measured over
/// the cases in `tests/plan_migrate_test.rs` and `tests/plan_test.rs`, true
/// renames score 0.62–0.98 and unrelated pairs 0.42–0.58, so the usable window
/// is narrow — which is the honest summary of what string similarity can do
/// here. Everything it declines is reported as `unbridged_removals` rather
/// than dropped, and every bridge it does make carries its score.
const MIGRATE_MIN_SIMILARITY: f64 = 0.6;

/// Largest number of triples put in a single DELETE DATA / INSERT DATA.
const DELTA_BATCH: usize = 500;

/// How many plans to keep. A long-lived server plans far more often than it
/// applies, and every plan stores a full copy of the proposed Turtle, so the
/// table is trimmed on write rather than left to grow without bound.
const PLAN_RETENTION: i64 = 100;

/// Owner recorded for plans computed outside an MCP session.
///
/// `plan` and `apply` are separate processes with nothing shared between them,
/// so the CLI's owner has to be a stable constant rather than anything
/// per-process.
pub const CLI_OWNER: &str = "cli";

/// Terraform-style plan/apply/migrate for ontology changes.
pub struct Planner {
    db: StateDb,
    graph: Arc<GraphStore>,
    /// Who a plan computed here belongs to, and whose plans an `apply` without
    /// an explicit id will consider. One state db is shared by every MCP
    /// session and by the CLI, so "the most recent plan" is only a safe default
    /// within one of them.
    owner: String,
}

struct PlanState {
    plan_id: String,
    new_turtle: String,
    added_classes: Vec<String>,
    removed_classes: Vec<String>,
    added_properties: Vec<String>,
    removed_properties: Vec<String>,
}

/// Which of a plan's semantic checks run.
///
/// Both are on at every entry point by default (the `plan` subcommand,
/// `batch`'s `plan`, `onto_plan`), and each has its own opt-out: conservativity
/// with `--no-conservativity` / `check_conservativity=false`, the logic check
/// with `--no-logic-check` / `check_logic=false`.
pub struct PlanChecks {
    /// Run the conservativity check with these options, or not at all.
    pub conservativity: Option<crate::conservativity::ConservativityOptions>,
    /// Run the logic check (#336): new TBox self-defects and the consistency
    /// of the proposed graph.
    pub logic: bool,
}

/// The `logic` block of a plan, and the reasons it gives for raising
/// `risk_score` to `high`, if any.
struct LogicVerdict {
    json: serde_json::Value,
    raises_risk: Vec<String>,
}

impl LogicVerdict {
    /// A block that says it did not look, with every verdict field `null`.
    fn not_checked(reason: String) -> Self {
        Self {
            json: serde_json::json!({
                "checked_by": "not_checked",
                "new_defects": null,
                "new_defects_by_severity": null,
                "new_defect_findings": null,
                "abox_consistent": null,
                "abox_consistent_before": null,
                "individuals_checked": null,
                "reason": reason,
            }),
            raises_risk: Vec::new(),
        }
    }
}

/// A three-valued consistency answer and the sentence that explains it.
struct Consistency {
    consistent: Option<bool>,
    individuals_checked: Option<usize>,
    note: String,
}

type Triple = (String, String, String);

/// The triple-level difference between the store and a proposed graph.
///
/// `plan()` used to diff only `?c a owl:Class` and `?p a owl:*Property`, so
/// instance data was invisible to it — while `apply()` cleared the whole store
/// and reloaded, deleting that same invisible data. Diffing at the triple level
/// is what lets the plan state what the apply is about to do (#91).
struct TripleDelta {
    insertions: Vec<Triple>,
    deletions: Vec<Triple>,
    /// Blank nodes are present somewhere in the two graphs, so `insertions` and
    /// `deletions` are empty and unusable: bnode labels are store-local, which
    /// makes a set difference over them meaningless, and neither `DELETE DATA`
    /// nor `INSERT DATA` may carry them at all. The caller reloads instead.
    reload: bool,
    current_total: usize,
    proposed_total: usize,
}

impl TripleDelta {
    fn counts(&self) -> (usize, usize) {
        if self.reload {
            (self.proposed_total, self.current_total)
        } else {
            (self.insertions.len(), self.deletions.len())
        }
    }

    fn strategy(&self) -> &'static str {
        if self.reload { "reload" } else { "delta" }
    }
}

impl Planner {
    pub fn new(db: StateDb, graph: Arc<GraphStore>) -> Self {
        Self::with_owner(db, graph, CLI_OWNER)
    }

    /// A planner whose plans belong to `owner` — an MCP session id, typically.
    pub fn with_owner(db: StateDb, graph: Arc<GraphStore>, owner: &str) -> Self {
        Self {
            db,
            graph,
            owner: owner.to_string(),
        }
    }

    /// Compute a diff plan between current store and proposed new Turtle.
    ///
    /// The logic check runs; conservativity does not. See [`Planner::plan_with`].
    pub fn plan(&self, new_turtle: &str) -> anyhow::Result<String> {
        self.plan_checked(new_turtle, None)
    }

    /// [`Planner::plan`], optionally answering the conservativity question as
    /// well. The logic check runs; [`Planner::plan_with`] can switch it off.
    pub fn plan_checked(
        &self,
        new_turtle: &str,
        conservativity: Option<crate::conservativity::ConservativityOptions>,
    ) -> anyhow::Result<String> {
        self.plan_with(new_turtle, PlanChecks { conservativity, logic: true })
    }

    /// The plan, with the semantic checks `checks` asks for.
    ///
    /// Everything a plan reports from the diff alone is about SHAPE: which
    /// classes appear, which disappear, how many triples move. None of it can
    /// tell you that adding one `rdfs:domain` silently reclassified nine
    /// hundred existing individuals, because that change adds no class and
    /// removes nothing. Two blocks are about MEANING, and they answer different
    /// questions:
    ///
    /// * `conservativity`: does the change alter any consequence over the
    ///   names the store already uses, under a Horn rule table? It reasons
    ///   both graphs to a fixpoint, which is seconds rather than milliseconds
    ///   on a real store.
    /// * `logic` (#336): does the change BREAK something? A named class left
    ///   with no possible instance, or an ABox that is now inconsistent. Before
    ///   it existed both scored `low`, because neither removes anything.
    ///
    /// Neither is fatal. A check that could not run says so in its block and
    /// the plan is still returned, because a diagnostic that can block a plan
    /// is a diagnostic people turn off.
    pub fn plan_with(&self, new_turtle: &str, checks: PlanChecks) -> anyhow::Result<String> {
        let mut result: serde_json::Value = serde_json::from_str(&self.plan_inner(new_turtle)?)?;
        result["conservativity"] = match checks.conservativity {
            None => serde_json::json!({
                "ran": false,
                "skipped": "not requested. Pass check_conservativity=true to ask whether this \
                            change alters any consequence over the names the store already uses, \
                            under the rule table named in profile. It reasons both graphs to a \
                            fixpoint, so it is not free.",
            }),
            Some(opts) => {
                match crate::conservativity::conservativity_check(&self.graph, new_turtle, &opts) {
                    Ok(r) => {
                        let mut v = serde_json::to_value(&r)?;
                        v["ran"] = serde_json::Value::Bool(true);
                        v
                    }
                    Err(e) => serde_json::json!({
                        "ran": false,
                        "skipped": format!(
                            "the conservativity check could not run, so this plan says nothing \
                             about what the change means: {e}"
                        ),
                    }),
                }
            }
        };

        let logic = if checks.logic {
            self.logic_verdict(new_turtle)
        } else {
            LogicVerdict::not_checked(
                "skipped on request (--no-logic-check, or check_logic=false). This plan does not \
                 say whether the change makes a class unsatisfiable or the ABox inconsistent, and \
                 risk_score is computed from the shape of the change alone."
                    .to_string(),
            )
        };
        if !logic.raises_risk.is_empty() {
            result["risk_score"] = serde_json::json!("high");
            if let Some(reasons) = result["risk_reasons"].as_array_mut() {
                reasons.extend(logic.raises_risk.iter().map(|r| serde_json::json!(r)));
            }
        }
        result["logic"] = logic.json;
        Ok(result.to_string())
    }

    /// The `logic` block of a plan: what the change breaks, judged against
    /// what the store already had.
    ///
    /// Two checks, both relative to the store, because a plan is about the
    /// change and not about the ontology: a store that already had a cycle
    /// must not make every later plan look as though it caused one.
    ///
    /// 1. The eight TBox self-defect kinds of `defects`, on the proposed graph
    ///    and on the store; only findings the store did not have are counted.
    ///    These are SPARQL queries over the declarations and need no budget.
    /// 2. Consistency of the proposed graph by the owl-dl tableaux (TBox and
    ///    ABox), under the same `[reasoner]` budget every owl-dl run uses. The
    ///    store is checked only when the proposed graph is proven
    ///    inconsistent, since that is the only case where its answer changes
    ///    the verdict, and it is the expensive half.
    ///
    /// Three values, never two. `abox_consistent` is `false` only on a clash
    /// the tableaux found, and `true` only when the search finished AND the
    /// graph uses no construct the tableaux does not model. Anything else is
    /// `null` with the reason, because `true` would say "we looked and found
    /// nothing" about a search that stopped, or about a weaker axiom set than
    /// the one stated.
    fn logic_verdict(&self, new_turtle: &str) -> LogicVerdict {
        use crate::defects::{Defects, Finding};

        let checked_by = "defects: the eight TBox self-defect kinds, proposed graph against the \
                          store; owl-dl tableaux (SHIQ): consistency of the proposed graph, and of \
                          the store when the proposed graph is inconsistent, under the [reasoner] \
                          budget";
        let proposed = Arc::new(GraphStore::new());
        if let Err(e) = proposed.load_turtle(new_turtle, None) {
            return LogicVerdict::not_checked(format!("the proposed graph could not be read: {e}"));
        }

        let mut notes: Vec<String> = Vec::new();
        let mut raises_risk: Vec<String> = Vec::new();

        // ── 1. New TBox self-defects ──────────────────────────────────────
        let new_findings: Option<Vec<Finding>> = match Defects::findings(&proposed) {
            Err(e) => {
                notes.push(format!("the defects check could not run on the proposed graph: {e}"));
                None
            }
            Ok(after) if after.is_empty() => Some(Vec::new()),
            Ok(after) => match Defects::findings(&self.graph) {
                Err(e) => {
                    notes.push(format!(
                        "the proposed graph has {} defect finding(s), and the store could not be \
                         checked to tell which are new: {e}",
                        after.len()
                    ));
                    None
                }
                Ok(before) => {
                    let before: HashSet<Finding> = before.into_iter().collect();
                    Some(after.into_iter().filter(|f| !before.contains(f)).collect())
                }
            },
        };
        let mut by_severity = serde_json::json!({"error": 0, "warning": 0, "info": 0});
        if let Some(found) = &new_findings {
            notes.push(format!(
                "defects: {} finding(s) in the proposed graph that the store did not have.",
                found.len()
            ));
            for f in found {
                if let Some(n) = by_severity[f.severity].as_u64() {
                    by_severity[f.severity] = serde_json::json!(n + 1);
                }
            }
            // An `info` finding leaves every entailment intact and only a
            // reader uninformed, so it is listed and does not move the score.
            let serious: Vec<&Finding> =
                found.iter().filter(|f| f.severity != "info").collect();
            if !serious.is_empty() {
                let first = serious[0];
                raises_risk.push(format!(
                    "the change introduces {} error- or warning-severity defect(s) the store did \
                     not have, the first {} on {}",
                    serious.len(),
                    first.kind,
                    first.subject
                ));
            }
        }

        // ── 2. Consistency ───────────────────────────────────────────────
        let after = Self::decide_consistency(&proposed);
        notes.push(format!("proposed graph: {}", after.note));
        let before = if after.consistent == Some(false) {
            let b = Self::decide_consistency(&self.graph);
            notes.push(format!("store: {}", b.note));
            if b.consistent != Some(false) {
                raises_risk.push(
                    "the proposed graph is inconsistent and the store was not proven inconsistent"
                        .to_string(),
                );
            }
            b.consistent
        } else {
            None
        };

        let json = serde_json::json!({
            "checked_by": checked_by,
            "new_defects": new_findings.as_ref().map(Vec::len),
            "new_defects_by_severity": new_findings.as_ref().map(|_| by_severity),
            "new_defect_findings": new_findings.as_ref().map(|found| {
                found
                    .iter()
                    .take(crate::defects::Defects::MAX_PER_KIND)
                    .map(Finding::to_json)
                    .collect::<Vec<_>>()
            }),
            "abox_consistent": after.consistent,
            "abox_consistent_before": before,
            "individuals_checked": after.individuals_checked,
            "reason": notes.join(" "),
        });
        LogicVerdict { json, raises_risk }
    }

    /// Whether `graph` is consistent, three-valued, with the reason in words.
    fn decide_consistency(graph: &Arc<GraphStore>) -> Consistency {
        use crate::tableaux::{Concept, DlReasoner};
        let reasoner = match DlReasoner::from_graph(graph) {
            Ok(r) => r,
            Err(e) => {
                return Consistency {
                    consistent: None,
                    individuals_checked: None,
                    note: format!("not checked: the tableaux could not read the graph: {e}"),
                };
            }
        };
        let tbox = reasoner.decide_satisfiable(&Concept::Top);
        if tbox.is_unsat() {
            return Consistency {
                consistent: Some(false),
                individuals_checked: None,
                note: "inconsistent: the TBox admits no model (owl:Thing is unsatisfiable)."
                    .to_string(),
            };
        }
        let abox = reasoner.check_abox();
        if !abox.consistent {
            return Consistency {
                consistent: Some(false),
                individuals_checked: Some(abox.individuals_checked),
                note: format!(
                    "inconsistent: the tableaux found a clash in the ABox over {} individual(s).",
                    abox.individuals_checked
                ),
            };
        }
        if tbox.is_unknown() || abox.undecided {
            let phase = if tbox.is_unknown() { "TBox consistency" } else { "ABox" };
            let by = abox.stopped_by.unwrap_or("time or steps");
            return Consistency {
                consistent: None,
                individuals_checked: Some(abox.individuals_checked),
                note: format!(
                    "not decided: the {phase} check ran out of its reasoner budget (stopped by \
                     {by}) before it finished, so no clash was found and none was ruled out. \
                     Raise [reasoner] tableaux_test_timeout_ms or classify_timeout_ms, or pass \
                     --no-logic-check to skip the check."
                ),
            };
        }
        let unmodelled = DlReasoner::unmodelled_constructs(graph);
        if !unmodelled.is_empty() {
            let names: Vec<String> =
                unmodelled.iter().map(|(k, n)| format!("{k} ({n})")).collect();
            return Consistency {
                consistent: None,
                individuals_checked: Some(abox.individuals_checked),
                note: format!(
                    "not decided: the graph uses {} which the tableaux does not model, so finding \
                     no clash over the remaining axioms is not a proof that the whole graph is \
                     consistent.",
                    names.join(", ")
                ),
            };
        }
        Consistency {
            consistent: Some(true),
            individuals_checked: Some(abox.individuals_checked),
            note: format!(
                "consistent: the search finished with no clash over {} individual(s), and the \
                 graph uses no construct the tableaux does not model.",
                abox.individuals_checked
            ),
        }
    }

    fn plan_inner(&self, new_turtle: &str) -> anyhow::Result<String> {
        let current_classes = self.extract_classes_from_store(&self.graph);
        let current_properties = self.extract_properties_from_store(&self.graph);

        // Load new Turtle into a temp store
        let temp_store = Arc::new(GraphStore::new());
        temp_store.load_turtle(new_turtle, None)?;

        let new_classes = self.extract_classes_from_store(&temp_store);
        let new_properties = self.extract_properties_from_store(&temp_store);

        let current_individuals = self.extract_individuals_from_store(&self.graph);
        let new_individuals = self.extract_individuals_from_store(&temp_store);

        let added_classes: Vec<String> = new_classes.difference(&current_classes).cloned().collect();
        let removed_classes: Vec<String> = current_classes.difference(&new_classes).cloned().collect();
        let added_properties: Vec<String> = new_properties.difference(&current_properties).cloned().collect();
        let removed_properties: Vec<String> = current_properties.difference(&new_properties).cloned().collect();
        let added_individuals: Vec<String> = new_individuals.difference(&current_individuals).cloned().collect();
        let removed_individuals: Vec<String> = current_individuals.difference(&new_individuals).cloned().collect();

        // Blast radius: count triples referencing removed IRIs. Individuals are
        // counted too — an apply deletes them just as surely as it deletes a
        // class, and the plan that stayed silent about them was the whole
        // complaint.
        let mut triples_affected: u64 = 0;
        for iri in removed_classes
            .iter()
            .chain(removed_properties.iter())
            .chain(removed_individuals.iter())
        {
            triples_affected += self.count_references(iri);
        }

        let delta = self.triple_delta(new_turtle)?;
        let (insertions, deletions) = delta.counts();

        // Check locked IRIs
        let locked_violations: Vec<serde_json::Value> = removed_classes
            .iter()
            .chain(removed_properties.iter())
            .filter_map(|iri| {
                if self.is_locked(iri) {
                    Some(serde_json::json!({
                        "iri": iri,
                        "reason": self.get_lock_reason(iri),
                    }))
                } else {
                    None
                }
            })
            .collect();

        // Risk scoring. Any removal counts, including instance data: dropping
        // 194 individuals is not a low-risk change just because the TBox is
        // untouched.
        let removes_anything = !removed_classes.is_empty()
            || !removed_properties.is_empty()
            || !removed_individuals.is_empty();
        let removed_terms =
            removed_classes.len() + removed_properties.len() + removed_individuals.len();
        let (risk_score, mut risk_reasons) = if removes_anything && triples_affected > 0 {
            (
                "high",
                vec![format!(
                    "the change removes {removed_terms} term(s) that {triples_affected} triple(s) \
                     in the store refer to"
                )],
            )
        } else if removes_anything {
            (
                "medium",
                vec![format!("the change removes {removed_terms} term(s)")],
            )
        } else {
            ("low", Vec::new())
        };
        if !locked_violations.is_empty() {
            risk_reasons.push(format!(
                "the change removes {} locked IRI(s)",
                locked_violations.len()
            ));
        }

        // Persist the plan. It has to outlive this `Planner` — see `store_plan`.
        let plan_id = self.store_plan(
            new_turtle,
            &added_classes,
            &removed_classes,
            &added_properties,
            &removed_properties,
        )?;

        let result = serde_json::json!({
            "plan_id": plan_id,
            "added_classes": added_classes,
            "removed_classes": removed_classes,
            "added_properties": added_properties,
            "removed_properties": removed_properties,
            "added_individuals": added_individuals,
            "removed_individuals": removed_individuals,
            "triple_delta": {
                "insertions": insertions,
                "deletions": deletions,
                "strategy": delta.strategy(),
            },
            "blast_radius": {
                "triples_affected": triples_affected,
            },
            "locked_violations": locked_violations,
            "risk_score": risk_score,
            // Why the score is what it is, one sentence per cause. Empty when
            // it is `low`. The logic check appends its own causes.
            "risk_reasons": risk_reasons,
        });

        Ok(result.to_string())
    }

    /// Apply the most recent plan.
    /// Modes: "safe" (clear + reload), "force" (same but ignores monitor), "migrate" (adds bridges)
    ///
    /// Apply does not re-judge the plan. It applies what was planned whatever
    /// the plan's `risk_score` or `logic` block said, exactly as it always has
    /// for a `high` score from removals; the only gate here is the monitor. A
    /// change that removes terms on purpose, or that is meant to land while a
    /// known inconsistency is fixed in the next change, has to stay applicable,
    /// and an apply that started refusing on a field it never used to read
    /// would break every pipeline built on it without a word. The verdict is in
    /// the plan's output for the pipeline to gate on: `risk_score`,
    /// `risk_reasons`, and `logic`.
    pub fn apply(&self, mode: &str) -> anyhow::Result<String> {
        self.apply_plan(None, mode)
    }

    /// Apply a specific plan by id, or the most recent one when `plan_id` is
    /// `None`.
    ///
    /// An unknown id is an error rather than a fall-back to the latest plan:
    /// quietly applying something the caller did not name would apply changes
    /// they never reviewed, which is the opposite of what plan/apply is for.
    pub fn apply_plan(&self, plan_id: Option<&str>, mode: &str) -> anyhow::Result<String> {
        // Check monitor block (unless force mode)
        if mode != "force" {
            let monitor = Monitor::new(self.db.clone(), self.graph.clone());
            if monitor.is_blocked() {
                return Ok(serde_json::json!({
                    "ok": false,
                    "blocked": true,
                    "message": "Apply blocked by monitor. Use mode='force' to override or clear the block.",
                }).to_string());
            }
        }

        let plan = self.load_plan(plan_id)?;

        if mode == "migrate" {
            return self.apply_migrate(&plan);
        }

        // Safe/force mode: bring the store to the proposed state by writing
        // only what differs. The old implementation cleared the entire store
        // and reloaded `new_turtle` wholesale, which rewrote every triple on
        // every apply and made incremental ABox changes impossible to govern.
        let delta = self.triple_delta(&plan.new_turtle)?;
        let (inserted, deleted) = delta.counts();
        if delta.reload {
            self.graph.clear()?;
            self.graph.load_turtle(&plan.new_turtle, None)?;
        } else {
            self.write_delta(&delta)?;
        }
        self.mark_applied(&plan.plan_id, mode);

        Ok(serde_json::json!({
            "ok": true,
            "plan_id": plan.plan_id,
            "mode": mode,
            "strategy": delta.strategy(),
            "triples_inserted": inserted,
            "triples_deleted": deleted,
            "triples_loaded": self.graph.triple_count(),
            "added_classes": plan.added_classes.len(),
            "removed_classes": plan.removed_classes.len(),
        }).to_string())
    }

    /// Triples that differ between the live store and `new_turtle`.
    fn triple_delta(&self, new_turtle: &str) -> anyhow::Result<TripleDelta> {
        let proposed_store = GraphStore::new();
        proposed_store.load_turtle(new_turtle, None)?;

        let current: HashSet<Triple> = self.graph.all_triples()?.into_iter().collect();
        let proposed: HashSet<Triple> = proposed_store.all_triples()?.into_iter().collect();
        let current_total = current.len();
        let proposed_total = proposed.len();

        let is_bnode = |t: &Triple| t.0.starts_with("_:") || t.2.starts_with("_:");
        if current.iter().any(is_bnode) || proposed.iter().any(is_bnode) {
            return Ok(TripleDelta {
                insertions: Vec::new(),
                deletions: Vec::new(),
                reload: true,
                current_total,
                proposed_total,
            });
        }

        Ok(TripleDelta {
            insertions: proposed.difference(&current).cloned().collect(),
            deletions: current.difference(&proposed).cloned().collect(),
            reload: false,
            current_total,
            proposed_total,
        })
    }

    fn write_delta(&self, delta: &TripleDelta) -> anyhow::Result<()> {
        // Deletions first: a triple that moves from one graph shape to another
        // must not be removed after it has been re-inserted.
        for (verb, triples) in [("DELETE", &delta.deletions), ("INSERT", &delta.insertions)] {
            for batch in triples.chunks(DELTA_BATCH) {
                let body: String = batch
                    .iter()
                    .map(|(s, p, o)| format!("{s} {p} {o} .\n"))
                    .collect();
                self.graph.sparql_update(&format!("{verb} DATA {{ {body} }}"))?;
            }
        }
        Ok(())
    }

    /// Persist a plan so a *different* `Planner` can apply it.
    ///
    /// The plan used to live in a `RefCell` on this struct, which meant it
    /// never survived the call that produced it: the `plan`/`apply` CLI
    /// subcommands, `batch`'s `exec_plan`/`exec_apply`, and the `onto_plan` /
    /// `onto_apply` MCP handlers each construct a fresh `Planner`, so `apply`
    /// always reported "No plan found" (#91). The state db is the one thing
    /// every caller shares.
    fn store_plan(
        &self,
        new_turtle: &str,
        added_classes: &[String],
        removed_classes: &[String],
        added_properties: &[String],
        removed_properties: &[String],
    ) -> anyhow::Result<String> {
        let plan_id = format!("plan-{:016x}", crate::lineage::rand_id());
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO plans \
             (plan_id, owner, new_turtle, added_classes, removed_classes, added_properties, removed_properties) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                plan_id,
                self.owner,
                new_turtle,
                serde_json::to_string(added_classes)?,
                serde_json::to_string(removed_classes)?,
                serde_json::to_string(added_properties)?,
                serde_json::to_string(removed_properties)?,
            ],
        )?;
        conn.execute(
            "DELETE FROM plans WHERE seq <= (SELECT MAX(seq) FROM plans) - ?1",
            rusqlite::params![PLAN_RETENTION],
        )?;
        Ok(plan_id)
    }

    fn load_plan(&self, plan_id: Option<&str>) -> anyhow::Result<PlanState> {
        const COLS: &str = "plan_id, new_turtle, added_classes, removed_classes, \
                            added_properties, removed_properties";
        let conn = self.db.conn();
        let found = match plan_id {
            Some(id) => conn
                .query_row(
                    &format!("SELECT {COLS} FROM plans WHERE plan_id = ?1"),
                    rusqlite::params![id],
                    Self::row_to_plan,
                )
                .optional()?,
            None => conn
                .query_row(
                    &format!("SELECT {COLS} FROM plans WHERE owner = ?1 ORDER BY seq DESC LIMIT 1"),
                    rusqlite::params![self.owner],
                    Self::row_to_plan,
                )
                .optional()?,
        };
        match (found, plan_id) {
            (Some(plan), _) => Ok(plan),
            (None, Some(id)) => anyhow::bail!(
                "No plan found with id '{id}'. Run plan() first, or omit the id to apply the most recent plan."
            ),
            (None, None) => {
                // Say when plans exist but belong to someone else. Reporting a
                // bare "no plan" would read as "nothing was ever planned",
                // which is false and sends the reader after the wrong problem.
                let others: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM plans WHERE owner <> ?1",
                        rusqlite::params![self.owner],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);
                if others > 0 {
                    anyhow::bail!(
                        "No plan found for this session. Run plan() first. \
                         ({others} plan(s) belong to another session — pass that plan's id to apply one deliberately.)"
                    )
                }
                anyhow::bail!("No plan found. Run plan() first.")
            }
        }
    }

    fn row_to_plan(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlanState> {
        fn iris(raw: String) -> Vec<String> {
            serde_json::from_str(&raw).unwrap_or_default()
        }
        Ok(PlanState {
            plan_id: row.get(0)?,
            new_turtle: row.get(1)?,
            added_classes: iris(row.get(2)?),
            removed_classes: iris(row.get(3)?),
            added_properties: iris(row.get(4)?),
            removed_properties: iris(row.get(5)?),
        })
    }

    /// Record that a plan was applied. Advisory audit trail: re-applying is
    /// still allowed, since `apply` is a full clear-and-reload and therefore
    /// idempotent against the same store.
    fn mark_applied(&self, plan_id: &str, mode: &str) {
        let conn = self.db.conn();
        let _ = conn.execute(
            "UPDATE plans SET applied_at = datetime('now'), applied_mode = ?2 WHERE plan_id = ?1",
            rusqlite::params![plan_id, mode],
        );
    }

    /// Add `owl:equivalentClass` / `owl:equivalentProperty` bridges for terms
    /// that appear to have been renamed, then bring the store to the proposed
    /// state.
    ///
    /// This used to pair *every* removed term with `added_classes.first()` /
    /// `added_properties.first()`, so a plan that added two classes and removed
    /// one asserted equivalence between whichever addition happened to sort
    /// first and the removal — a fabricated logical axiom (#91). Candidates now
    /// come from `DriftDetector`, the calibrated rename detector the rest of the
    /// crate already uses, and are assigned one-to-one so no addition can be
    /// declared the replacement for two different removals.
    fn apply_migrate(&self, plan: &PlanState) -> anyhow::Result<String> {
        // Detect renames against the store as it stands, before the delta
        // removes the very terms being matched.
        let pairs = self.rename_candidates(plan)?;

        // Then bring the store to the proposed state. This has to happen before
        // the bridges are written: they are deprecation stubs for terms the
        // proposed Turtle no longer contains, so a delta computed afterwards
        // would delete them again.
        let delta = self.triple_delta(&plan.new_turtle)?;
        let (inserted, deleted) = delta.counts();
        if delta.reload {
            self.graph.clear()?;
            self.graph.load_turtle(&plan.new_turtle, None)?;
        } else {
            self.write_delta(&delta)?;
        }

        let mut migration_triples = 0u64;
        let mut bridges = Vec::new();
        for (removed, added, similarity, predicate) in &pairs {
            let update = format!(
                "INSERT DATA {{ <{removed}> <{predicate}> <{added}> . \
                 <{removed}> <http://www.w3.org/2002/07/owl#deprecated> \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> . \
                 <{removed}> <http://www.w3.org/2000/01/rdf-schema#comment> \"Deprecated: migrated to {added}\" . }}"
            );
            if let Ok(n) = self.graph.sparql_update(&update) {
                migration_triples += n as u64;
            }
            bridges.push(serde_json::json!({
                "from": removed,
                "to": added,
                "similarity": similarity,
                "predicate": predicate,
            }));
        }

        // Every removal the matcher declined, named. A migrate that quietly
        // bridges some terms and says nothing about the rest is how the old
        // heuristic hid its mistakes.
        let bridged: HashSet<&String> = pairs.iter().map(|(r, _, _, _)| r).collect();
        let unbridged: Vec<&String> = plan
            .removed_classes
            .iter()
            .chain(plan.removed_properties.iter())
            .filter(|r| !bridged.contains(*r))
            .collect();

        self.mark_applied(&plan.plan_id, "migrate");

        Ok(serde_json::json!({
            "ok": true,
            "plan_id": plan.plan_id,
            "mode": "migrate",
            "strategy": delta.strategy(),
            "triples_inserted": inserted,
            "triples_deleted": deleted,
            "triples_loaded": self.graph.triple_count(),
            "migration_triples": migration_triples,
            "bridges_created": bridges.len(),
            "bridges": bridges,
            "unbridged_removals": unbridged,
        }).to_string())
    }

    /// Rename pairs for `migrate`, as `(removed, added, similarity, predicate)`.
    ///
    /// Candidates below [`MIGRATE_MIN_SIMILARITY`] are dropped, the rest are
    /// taken in descending order of the detector's confidence, and both sides
    /// of an accepted pair are consumed so neither can appear again. A class is
    /// only ever bridged to a class and a property to a property.
    fn rename_candidates(
        &self,
        plan: &PlanState,
    ) -> anyhow::Result<Vec<(String, String, f64, &'static str)>> {
        let current_turtle = self.graph.serialize("turtle")?;
        let detector = crate::drift::DriftDetector::new(self.db.clone());
        let drift: serde_json::Value =
            serde_json::from_str(&detector.detect(&current_turtle, &plan.new_turtle)?)?;

        let classes: HashSet<&String> = plan.removed_classes.iter().collect();
        let properties: HashSet<&String> = plan.removed_properties.iter().collect();
        let added_classes: HashSet<&String> = plan.added_classes.iter().collect();
        let added_properties: HashSet<&String> = plan.added_properties.iter().collect();

        // `likely_renames` arrives sorted by confidence, descending.
        let mut taken_from: HashSet<String> = HashSet::new();
        let mut taken_to: HashSet<String> = HashSet::new();
        let mut pairs = Vec::new();
        for candidate in drift["likely_renames"].as_array().into_iter().flatten() {
            let (Some(from), Some(to)) = (
                candidate["from"].as_str().map(str::to_string),
                candidate["to"].as_str().map(str::to_string),
            ) else {
                continue;
            };
            let similarity = candidate["signals"]["label_similarity"]
                .as_f64()
                .unwrap_or(0.0);
            if similarity < MIGRATE_MIN_SIMILARITY {
                continue;
            }
            if taken_from.contains(&from) || taken_to.contains(&to) {
                continue;
            }
            let predicate = if classes.contains(&from) && added_classes.contains(&to) {
                "http://www.w3.org/2002/07/owl#equivalentClass"
            } else if properties.contains(&from) && added_properties.contains(&to) {
                "http://www.w3.org/2002/07/owl#equivalentProperty"
            } else {
                continue;
            };
            taken_from.insert(from.clone());
            taken_to.insert(to.clone());
            pairs.push((from, to, similarity, predicate));
        }
        Ok(pairs)
    }

    /// Lock an IRI to prevent removal.
    pub fn lock_iri(&self, iri: &str, reason: &str) {
        let conn = self.db.conn();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO iri_locks (iri, reason) VALUES (?1, ?2)",
            rusqlite::params![iri, reason],
        );
    }

    /// Check if an IRI is locked.
    pub fn is_locked(&self, iri: &str) -> bool {
        let conn = self.db.conn();
        conn.query_row(
            "SELECT 1 FROM iri_locks WHERE iri = ?1",
            rusqlite::params![iri],
            |_| Ok(()),
        )
        .is_ok()
    }

    fn get_lock_reason(&self, iri: &str) -> String {
        let conn = self.db.conn();
        conn.query_row(
            "SELECT reason FROM iri_locks WHERE iri = ?1",
            rusqlite::params![iri],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .unwrap_or_default()
    }

    fn extract_classes_from_store(&self, store: &GraphStore) -> HashSet<String> {
        let query = "SELECT DISTINCT ?c WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> }";
        self.extract_iris(store, query, "c")
    }

    /// Individuals: anything typed by a declared class, plus explicit
    /// `owl:NamedIndividual`s. Blank nodes are excluded — they are structural
    /// (restrictions, lists), not instance data, and have no stable identity to
    /// diff against.
    fn extract_individuals_from_store(&self, store: &GraphStore) -> HashSet<String> {
        let query = "SELECT DISTINCT ?i WHERE { \
            { ?i a <http://www.w3.org/2002/07/owl#NamedIndividual> } \
            UNION \
            { ?i a ?c . ?c a <http://www.w3.org/2002/07/owl#Class> } \
        }";
        let mut set = self.extract_iris(store, query, "i");
        set.retain(|i| !i.starts_with("_:"));
        set
    }

    fn extract_properties_from_store(&self, store: &GraphStore) -> HashSet<String> {
        let query = "SELECT DISTINCT ?p WHERE { \
            { ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> } \
            UNION \
            { ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> } \
        }";
        self.extract_iris(store, query, "p")
    }

    fn extract_iris(&self, store: &GraphStore, query: &str, var: &str) -> HashSet<String> {
        let mut set = HashSet::new();
        if let Ok(json) = store.sparql_select_union(query)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json)
                && let Some(results) = parsed["results"].as_array() {
                    for row in results {
                        if let Some(iri) = row[var].as_str() {
                            let iri = iri.trim_matches(|c| c == '<' || c == '>');
                            set.insert(iri.to_string());
                        }
                    }
                }
        set
    }

    fn count_references(&self, iri: &str) -> u64 {
        let query = format!(
            "SELECT (COUNT(*) AS ?count) WHERE {{ \
             {{ <{iri}> ?p ?o }} UNION {{ ?s <{iri}> ?o }} UNION {{ ?s ?p <{iri}> }} \
             }}"
        );
        if let Ok(json) = self.graph.sparql_select_union(&query)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json)
                && let Some(results) = parsed["results"].as_array()
                    && let Some(first) = results.first()
                        && let Some(count_str) = first["count"].as_str() {
                            let cleaned = count_str
                                .trim_matches('"')
                                .split("^^")
                                .next()
                                .unwrap_or("0")
                                .trim_matches('"');
                            return cleaned.parse().unwrap_or(0);
                        }
        0
    }
}
