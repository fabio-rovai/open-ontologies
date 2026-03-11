use crate::graph::GraphStore;
use crate::monitor::Monitor;
use crate::state::StateDb;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

/// Terraform-style plan/apply/migrate for ontology changes.
pub struct Planner {
    db: StateDb,
    graph: Arc<GraphStore>,
    last_plan: RefCell<Option<PlanState>>,
}

struct PlanState {
    new_turtle: String,
    added_classes: Vec<String>,
    removed_classes: Vec<String>,
    added_properties: Vec<String>,
    removed_properties: Vec<String>,
}

impl Planner {
    pub fn new(db: StateDb, graph: Arc<GraphStore>) -> Self {
        Self {
            db,
            graph,
            last_plan: RefCell::new(None),
        }
    }

    /// Compute a diff plan between current store and proposed new Turtle.
    pub fn plan(&self, new_turtle: &str) -> anyhow::Result<String> {
        let current_classes = self.extract_classes_from_store(&self.graph);
        let current_properties = self.extract_properties_from_store(&self.graph);

        // Load new Turtle into a temp store
        let temp_store = Arc::new(GraphStore::new());
        temp_store.load_turtle(new_turtle, None)?;

        let new_classes = self.extract_classes_from_store(&temp_store);
        let new_properties = self.extract_properties_from_store(&temp_store);

        let added_classes: Vec<String> = new_classes.difference(&current_classes).cloned().collect();
        let removed_classes: Vec<String> = current_classes.difference(&new_classes).cloned().collect();
        let added_properties: Vec<String> = new_properties.difference(&current_properties).cloned().collect();
        let removed_properties: Vec<String> = current_properties.difference(&new_properties).cloned().collect();

        // Blast radius: count triples referencing removed IRIs
        let mut triples_affected: u64 = 0;
        for iri in removed_classes.iter().chain(removed_properties.iter()) {
            triples_affected += self.count_references(iri);
        }

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

        // Risk scoring
        let risk_score = if !removed_classes.is_empty() && triples_affected > 0 {
            "high"
        } else if !removed_classes.is_empty() || !removed_properties.is_empty() {
            "medium"
        } else {
            "low"
        };

        // Cache the plan state
        *self.last_plan.borrow_mut() = Some(PlanState {
            new_turtle: new_turtle.to_string(),
            added_classes: added_classes.clone(),
            removed_classes: removed_classes.clone(),
            added_properties: added_properties.clone(),
            removed_properties: removed_properties.clone(),
        });

        let result = serde_json::json!({
            "added_classes": added_classes,
            "removed_classes": removed_classes,
            "added_properties": added_properties,
            "removed_properties": removed_properties,
            "blast_radius": {
                "triples_affected": triples_affected,
            },
            "locked_violations": locked_violations,
            "risk_score": risk_score,
        });

        Ok(result.to_string())
    }

    /// Apply the last planned changes.
    /// Modes: "safe" (clear + reload), "force" (same but ignores monitor), "migrate" (adds bridges)
    pub fn apply(&self, mode: &str) -> anyhow::Result<String> {
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

        let plan = self.last_plan.borrow();
        let plan = match plan.as_ref() {
            Some(p) => p,
            None => anyhow::bail!("No plan found. Run plan() first."),
        };

        if mode == "migrate" {
            return self.apply_migrate(plan);
        }

        // Safe/force mode: clear store, load new turtle
        self.graph.clear()?;
        let count = self.graph.load_turtle(&plan.new_turtle, None)?;

        Ok(serde_json::json!({
            "ok": true,
            "mode": mode,
            "triples_loaded": count,
            "added_classes": plan.added_classes.len(),
            "removed_classes": plan.removed_classes.len(),
        }).to_string())
    }

    fn apply_migrate(&self, plan: &PlanState) -> anyhow::Result<String> {
        let mut migration_triples = 0u64;

        // Generate equivalentClass/equivalentProperty bridges for renames
        // Heuristic: if a class/property was removed and one was added, they might be renames
        for removed in &plan.removed_classes {
            if let Some(added) = plan.added_classes.first() {
                // Generate equivalentClass bridge
                let update = format!(
                    "INSERT DATA {{ <{}> <http://www.w3.org/2002/07/owl#equivalentClass> <{}> . \
                     <{}> <http://www.w3.org/2002/07/owl#deprecated> \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> . \
                     <{}> <http://www.w3.org/2000/01/rdf-schema#comment> \"Deprecated: migrated to {}\" . }}",
                    removed, added, removed, removed, added
                );
                if let Ok(n) = self.graph.sparql_update(&update) {
                    migration_triples += n as u64;
                }
            }
        }

        for removed in &plan.removed_properties {
            if let Some(added) = plan.added_properties.first() {
                let update = format!(
                    "INSERT DATA {{ <{}> <http://www.w3.org/2002/07/owl#equivalentProperty> <{}> . \
                     <{}> <http://www.w3.org/2002/07/owl#deprecated> \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> . \
                     <{}> <http://www.w3.org/2000/01/rdf-schema#comment> \"Deprecated: migrated to {}\" . }}",
                    removed, added, removed, removed, added
                );
                if let Ok(n) = self.graph.sparql_update(&update) {
                    migration_triples += n as u64;
                }
            }
        }

        // Also load the new turtle
        let count = self.graph.load_turtle(&plan.new_turtle, None)?;

        Ok(serde_json::json!({
            "ok": true,
            "mode": "migrate",
            "triples_loaded": count,
            "migration_triples": migration_triples,
            "bridges_created": plan.removed_classes.len() + plan.removed_properties.len(),
        }).to_string())
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
        if let Ok(json) = store.sparql_select(query) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                if let Some(results) = parsed["results"].as_array() {
                    for row in results {
                        if let Some(iri) = row[var].as_str() {
                            let iri = iri.trim_matches(|c| c == '<' || c == '>');
                            set.insert(iri.to_string());
                        }
                    }
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
        if let Ok(json) = self.graph.sparql_select(&query) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                if let Some(results) = parsed["results"].as_array() {
                    if let Some(first) = results.first() {
                        if let Some(count_str) = first["count"].as_str() {
                            let cleaned = count_str
                                .trim_matches('"')
                                .split("^^")
                                .next()
                                .unwrap_or("0")
                                .trim_matches('"');
                            return cleaned.parse().unwrap_or(0);
                        }
                    }
                }
            }
        }
        0
    }
}
