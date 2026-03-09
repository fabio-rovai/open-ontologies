use std::collections::HashSet;
use std::io::Cursor;

use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::sparql::QueryResults;
use oxigraph::store::Store;

use crate::graph::GraphStore;
use crate::state::StateDb;
use std::sync::Arc;

pub struct OntologyService;

impl OntologyService {
    /// Validate RDF syntax. Returns a JSON report (never errors on bad input).
    pub fn validate_string(content: &str) -> anyhow::Result<String> {
        match GraphStore::validate_turtle(content) {
            Ok(count) => Ok(serde_json::json!({
                "valid": true,
                "triple_count": count,
                "errors": []
            })
            .to_string()),
            Err(e) => Ok(serde_json::json!({
                "valid": false,
                "triple_count": 0,
                "errors": [e.to_string()]
            })
            .to_string()),
        }
    }

    /// Validate an RDF file.
    pub fn validate_file(path: &str) -> anyhow::Result<String> {
        match GraphStore::validate_file(path) {
            Ok(count) => Ok(serde_json::json!({
                "valid": true,
                "path": path,
                "triple_count": count,
                "errors": []
            })
            .to_string()),
            Err(e) => Ok(serde_json::json!({
                "valid": false,
                "path": path,
                "triple_count": 0,
                "errors": [e.to_string()]
            })
            .to_string()),
        }
    }

    /// Convert between RDF formats.
    pub fn convert(content: &str, _from: &str, to: &str) -> anyhow::Result<String> {
        let store = GraphStore::new();
        store.load_turtle(content, None)?;
        store.serialize(to)
    }

    /// Diff two ontologies. Returns added/removed triples.
    pub fn diff(old_content: &str, new_content: &str) -> anyhow::Result<String> {
        let old_store = Store::new()?;
        let new_store = Store::new()?;

        let old_reader = Cursor::new(old_content.as_bytes());
        for quad in RdfParser::from_format(RdfFormat::Turtle).for_reader(old_reader) {
            old_store.insert(&quad?)?;
        }

        let new_reader = Cursor::new(new_content.as_bytes());
        for quad in RdfParser::from_format(RdfFormat::Turtle).for_reader(new_reader) {
            new_store.insert(&quad?)?;
        }

        let old_triples: HashSet<String> = old_store
            .iter()
            .filter_map(|q| q.ok())
            .map(|q| format!("{} {} {}", q.subject, q.predicate, q.object))
            .collect();

        let new_triples: HashSet<String> = new_store
            .iter()
            .filter_map(|q| q.ok())
            .map(|q| format!("{} {} {}", q.subject, q.predicate, q.object))
            .collect();

        let added: Vec<&String> = new_triples.difference(&old_triples).collect();
        let removed: Vec<&String> = old_triples.difference(&new_triples).collect();

        Ok(serde_json::json!({
            "added": added.len(),
            "removed": removed.len(),
            "added_triples": added,
            "removed_triples": removed,
        })
        .to_string())
    }

    /// Lint an ontology -- check for missing labels, comments, domains.
    pub fn lint(content: &str) -> anyhow::Result<String> {
        let store = Store::new()?;
        let reader = Cursor::new(content.as_bytes());
        for quad in RdfParser::from_format(RdfFormat::Turtle).for_reader(reader) {
            store.insert(&quad?)?;
        }

        let mut issues: Vec<serde_json::Value> = Vec::new();

        // Find classes without rdfs:label
        let query = r#"
            SELECT ?class WHERE {
                { ?class a <http://www.w3.org/2002/07/owl#Class> }
                UNION
                { ?class a <http://www.w3.org/2000/01/rdf-schema#Class> }
                FILTER NOT EXISTS { ?class <http://www.w3.org/2000/01/rdf-schema#label> ?label }
            }
        "#;
        if let Ok(QueryResults::Solutions(solutions)) = store.query(query) {
            for row in solutions.flatten() {
                if let Some(term) = row.get("class") {
                    issues.push(serde_json::json!({
                        "severity": "warning",
                        "type": "missing_label",
                        "entity": term.to_string(),
                        "message": format!("{} has no rdfs:label", term),
                    }));
                }
            }
        }

        // Find classes without rdfs:comment
        let query = r#"
            SELECT ?class WHERE {
                { ?class a <http://www.w3.org/2002/07/owl#Class> }
                UNION
                { ?class a <http://www.w3.org/2000/01/rdf-schema#Class> }
                FILTER NOT EXISTS { ?class <http://www.w3.org/2000/01/rdf-schema#comment> ?comment }
            }
        "#;
        if let Ok(QueryResults::Solutions(solutions)) = store.query(query) {
            for row in solutions.flatten() {
                if let Some(term) = row.get("class") {
                    issues.push(serde_json::json!({
                        "severity": "warning",
                        "type": "missing_comment",
                        "entity": term.to_string(),
                        "message": format!("{} has no rdfs:comment", term),
                    }));
                }
            }
        }

        // Find properties without domain
        let query = r#"
            SELECT ?prop WHERE {
                { ?prop a <http://www.w3.org/2002/07/owl#ObjectProperty> }
                UNION
                { ?prop a <http://www.w3.org/2002/07/owl#DatatypeProperty> }
                FILTER NOT EXISTS { ?prop <http://www.w3.org/2000/01/rdf-schema#domain> ?d }
            }
        "#;
        if let Ok(QueryResults::Solutions(solutions)) = store.query(query) {
            for row in solutions.flatten() {
                if let Some(term) = row.get("prop") {
                    issues.push(serde_json::json!({
                        "severity": "info",
                        "type": "missing_domain",
                        "entity": term.to_string(),
                        "message": format!("{} has no rdfs:domain", term),
                    }));
                }
            }
        }

        Ok(serde_json::json!({
            "issues": issues,
            "issue_count": issues.len(),
        })
        .to_string())
    }

    /// Save a named version (snapshot) of the current graph store.
    pub fn save_version(db: &StateDb, store: &Arc<GraphStore>, label: &str) -> anyhow::Result<String> {
        let content = store.snapshot("ntriples")?;
        let count = store.triple_count();
        let conn = db.conn();
        conn.execute(
            "INSERT INTO ontology_versions (label, triple_count, content, format) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![label, count as i64, content, "ntriples"],
        )?;
        Ok(serde_json::json!({
            "ok": true,
            "label": label,
            "triple_count": count,
        }).to_string())
    }

    /// List all saved ontology versions.
    pub fn list_versions(db: &StateDb) -> anyhow::Result<String> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, label, triple_count, format, created_at FROM ontology_versions ORDER BY id DESC"
        )?;
        let versions: Vec<serde_json::Value> = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "label": row.get::<_, String>(1)?,
                "triple_count": row.get::<_, i64>(2)?,
                "format": row.get::<_, String>(3)?,
                "created_at": row.get::<_, String>(4)?,
            }))
        })?.filter_map(|r| r.ok()).collect();
        Ok(serde_json::json!({"versions": versions}).to_string())
    }

    /// Rollback the graph store to a previously saved version.
    pub fn rollback_version(db: &StateDb, store: &Arc<GraphStore>, label: &str) -> anyhow::Result<String> {
        let conn = db.conn();
        let content: String = conn.query_row(
            "SELECT content FROM ontology_versions WHERE label = ?1 ORDER BY id DESC LIMIT 1",
            rusqlite::params![label],
            |row| row.get(0),
        )?;
        store.clear()?;
        let count = store.load_ntriples(&content)?;
        Ok(serde_json::json!({
            "ok": true,
            "label": label,
            "triples_restored": count,
        }).to_string())
    }
}
