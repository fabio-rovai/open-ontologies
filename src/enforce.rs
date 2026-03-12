use crate::graph::GraphStore;
use crate::state::StateDb;
use std::sync::Arc;

/// Design pattern enforcement with built-in and custom rule packs.
pub struct Enforcer {
    db: StateDb,
    graph: Arc<GraphStore>,
}

impl Enforcer {
    pub fn new(db: StateDb, graph: Arc<GraphStore>) -> Self {
        Self { db, graph }
    }

    /// Run enforcement for a rule pack. Built-in packs: "generic", "boro", "value_partition".
    /// Custom rules stored in SQLite are also checked for the given pack name.
    pub fn enforce(&self, rule_pack: &str) -> anyhow::Result<String> {
        self.enforce_with_feedback(rule_pack, None)
    }

    /// Run enforcement with optional feedback-based suppression.
    pub fn enforce_with_feedback(&self, rule_pack: &str, feedback_db: Option<&StateDb>) -> anyhow::Result<String> {
        let mut violations = Vec::new();
        let mut total_rules = 0u32;
        let mut passed_rules = 0u32;

        match rule_pack {
            "generic" => self.run_generic_rules(&mut violations, &mut total_rules, &mut passed_rules),
            "boro" => self.run_boro_rules(&mut violations, &mut total_rules, &mut passed_rules),
            "value_partition" => self.run_value_partition_rules(&mut violations, &mut total_rules, &mut passed_rules),
            _ => {}
        }

        // Also run custom rules for this pack
        self.run_custom_rules(rule_pack, &mut violations, &mut total_rules, &mut passed_rules);

        // Apply feedback adjustments
        let mut filtered_violations: Vec<serde_json::Value> = Vec::new();
        let mut suppressed_count: u64 = 0;

        for violation in violations {
            if let Some(db) = feedback_db {
                let rule = violation["rule"].as_str().unwrap_or("");
                let entity = violation["entity"].as_str().unwrap_or("");
                match crate::feedback::get_feedback_adjustment(db, "enforce", rule, entity) {
                    crate::feedback::FeedbackAction::Suppress => {
                        suppressed_count += 1;
                        continue;
                    }
                    crate::feedback::FeedbackAction::Downgrade => {
                        let original = violation["severity"].as_str().unwrap_or("info");
                        let downgraded = crate::feedback::downgrade_severity(original);
                        let mut adjusted = violation.clone();
                        adjusted["original_severity"] = serde_json::json!(original);
                        adjusted["adjusted_severity"] = serde_json::json!(downgraded);
                        adjusted["severity"] = serde_json::json!(downgraded);
                        filtered_violations.push(adjusted);
                    }
                    crate::feedback::FeedbackAction::Keep => {
                        filtered_violations.push(violation);
                    }
                }
            } else {
                filtered_violations.push(violation);
            }
        }

        let compliance = if total_rules > 0 {
            passed_rules as f64 / total_rules as f64
        } else {
            1.0
        };

        let result = serde_json::json!({
            "rule_pack": rule_pack,
            "violations": filtered_violations,
            "total_rules": total_rules,
            "passed_rules": passed_rules,
            "compliance": compliance,
            "suppressed_count": suppressed_count,
        });

        Ok(result.to_string())
    }

    /// Add a custom SPARQL rule to the database.
    pub fn add_custom_rule(&self, id: &str, rule_pack: &str, query: &str, severity: &str, message: &str) {
        let conn = self.db.conn();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO enforce_rules (id, rule_pack, query, severity, message, enabled) \
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            rusqlite::params![id, rule_pack, query, severity, message],
        );
    }

    fn run_generic_rules(&self, violations: &mut Vec<serde_json::Value>, total: &mut u32, passed: &mut u32) {
        // Rule: orphan classes (no subclass parent, not used as domain/range)
        self.check_orphan_classes(violations, total, passed);
        // Rule: missing domain
        self.check_missing_domain(violations, total, passed);
        // Rule: missing range
        self.check_missing_range(violations, total, passed);
        // Rule: missing label
        self.check_missing_label(violations, total, passed);
    }

    fn check_orphan_classes(&self, violations: &mut Vec<serde_json::Value>, total: &mut u32, passed: &mut u32) {
        *total += 1;
        let query = "SELECT DISTINCT ?c WHERE { \
            ?c a <http://www.w3.org/2002/07/owl#Class> . \
            FILTER NOT EXISTS { ?c <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?parent } \
            FILTER NOT EXISTS { ?child <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c } \
            FILTER NOT EXISTS { ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?c } \
            FILTER NOT EXISTS { ?p <http://www.w3.org/2000/01/rdf-schema#range> ?c } \
        }";

        let orphans = self.query_iris(query, "c");
        if orphans.is_empty() {
            *passed += 1;
        } else {
            for orphan in orphans {
                violations.push(serde_json::json!({
                    "rule": "orphan_class",
                    "severity": "warning",
                    "entity": orphan,
                    "message": "Class has no parent, children, or property references",
                }));
            }
        }
    }

    fn check_missing_domain(&self, violations: &mut Vec<serde_json::Value>, total: &mut u32, passed: &mut u32) {
        *total += 1;
        let query = "SELECT DISTINCT ?p WHERE { \
            { ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> } UNION \
            { ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> } \
            FILTER NOT EXISTS { ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?d } \
        }";

        let props = self.query_iris(query, "p");
        if props.is_empty() {
            *passed += 1;
        } else {
            for prop in props {
                violations.push(serde_json::json!({
                    "rule": "missing_domain",
                    "severity": "warning",
                    "entity": prop,
                    "message": "Property has no rdfs:domain",
                }));
            }
        }
    }

    fn check_missing_range(&self, violations: &mut Vec<serde_json::Value>, total: &mut u32, passed: &mut u32) {
        *total += 1;
        let query = "SELECT DISTINCT ?p WHERE { \
            { ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> } UNION \
            { ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> } \
            FILTER NOT EXISTS { ?p <http://www.w3.org/2000/01/rdf-schema#range> ?r } \
        }";

        let props = self.query_iris(query, "p");
        if props.is_empty() {
            *passed += 1;
        } else {
            for prop in props {
                violations.push(serde_json::json!({
                    "rule": "missing_range",
                    "severity": "warning",
                    "entity": prop,
                    "message": "Property has no rdfs:range",
                }));
            }
        }
    }

    fn check_missing_label(&self, violations: &mut Vec<serde_json::Value>, total: &mut u32, passed: &mut u32) {
        *total += 1;
        let query = "SELECT DISTINCT ?c WHERE { \
            ?c a <http://www.w3.org/2002/07/owl#Class> . \
            FILTER NOT EXISTS { ?c <http://www.w3.org/2000/01/rdf-schema#label> ?l } \
        }";

        let classes = self.query_iris(query, "c");
        if classes.is_empty() {
            *passed += 1;
        } else {
            for cls in classes {
                violations.push(serde_json::json!({
                    "rule": "missing_label",
                    "severity": "info",
                    "entity": cls,
                    "message": "Class has no rdfs:label",
                }));
            }
        }
    }

    fn run_boro_rules(&self, violations: &mut Vec<serde_json::Value>, total: &mut u32, passed: &mut u32) {
        // BORO rule: entities that subclass ies:Entity must have a corresponding State class
        *total += 1;
        let query = "SELECT DISTINCT ?entity WHERE { \
            ?entity <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://ies.data.gov.uk/ontology/ies4#Entity> . \
            FILTER NOT EXISTS { \
                ?state <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://ies.data.gov.uk/ontology/ies4#State> . \
                ?state <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?entity . \
            } \
        }";

        let entities = self.query_iris(query, "entity");
        if entities.is_empty() {
            *passed += 1;
        } else {
            for entity in entities {
                violations.push(serde_json::json!({
                    "rule": "missing_state_class",
                    "severity": "error",
                    "entity": entity,
                    "message": "BORO: Entity subclass has no corresponding State class",
                }));
            }
        }
    }

    fn run_value_partition_rules(&self, violations: &mut Vec<serde_json::Value>, total: &mut u32, passed: &mut u32) {
        // Find classes that have multiple subclasses (partition candidates)
        let parent_query = "SELECT DISTINCT ?parent WHERE { \
            ?child1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?parent . \
            ?child2 <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?parent . \
            FILTER(?child1 != ?child2) \
            FILTER NOT EXISTS { ?parent <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?grandparent } \
        }";

        let parents = self.query_iris(parent_query, "parent");

        for parent in &parents {
            // Get children of this parent
            let children_query = format!(
                "SELECT DISTINCT ?child WHERE {{ ?child <http://www.w3.org/2000/01/rdf-schema#subClassOf> <{}> }}",
                parent
            );
            let children = self.query_iris(&children_query, "child");

            if children.len() < 2 {
                continue;
            }

            // Check if children are pairwise disjoint
            *total += 1;
            let mut all_disjoint = true;
            for i in 0..children.len() {
                for j in (i + 1)..children.len() {
                    let disjoint_query = format!(
                        "ASK {{ <{}> <http://www.w3.org/2002/07/owl#disjointWith> <{}> }}",
                        children[i], children[j]
                    );
                    if let Ok(result) = self.graph.sparql_select(&disjoint_query)
                        && !result.contains("true") {
                            all_disjoint = false;
                            break;
                        }
                }
                if !all_disjoint {
                    break;
                }
            }

            if all_disjoint {
                *passed += 1;
            } else {
                violations.push(serde_json::json!({
                    "rule": "partition_not_disjoint",
                    "severity": "warning",
                    "entity": parent,
                    "message": format!("Value partition: children of {} are not pairwise disjoint", parent),
                }));
            }
        }
    }

    fn run_custom_rules(&self, rule_pack: &str, violations: &mut Vec<serde_json::Value>, total: &mut u32, passed: &mut u32) {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare("SELECT id, query, severity, message FROM enforce_rules WHERE rule_pack = ?1 AND enabled = 1")
            .unwrap();

        let rules: Vec<(String, String, String, String)> = stmt
            .query_map(rusqlite::params![rule_pack], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                ))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        for (id, query, severity, message) in rules {
            *total += 1;
            if let Ok(result) = self.graph.sparql_select(&query) {
                // ASK query: true = violation
                if result.contains("\"result\":true") || result.contains("\"result\": true") {
                    violations.push(serde_json::json!({
                        "rule": id,
                        "severity": severity,
                        "entity": "graph",
                        "message": message,
                    }));
                } else {
                    *passed += 1;
                }
            }
        }
    }

    fn query_iris(&self, query: &str, var: &str) -> Vec<String> {
        if let Ok(json) = self.graph.sparql_select(query)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json)
                && let Some(results) = parsed["results"].as_array() {
                    return results
                        .iter()
                        .filter_map(|r| {
                            r[var].as_str().map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string())
                        })
                        .collect();
                }
        Vec::new()
    }
}
