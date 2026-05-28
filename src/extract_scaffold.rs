//! Schema-guided structured-extraction scaffolding (#28, OntoGPT SPIRES,
//! MCP-native).
//!
//! Per the MCP-native convention: the server does NOT run an LLM. It provides
//! the *scaffolding* — schema → prompt template, prompt → schema-conformance
//! check — and the orchestrator (Claude) supplies the LLM output that the
//! server validates.
//!
//! ## What this primitive does
//!
//! Given an ontology class IRI and the loaded ontology, emit a prompt
//! template that asks an LLM to extract instances of that class as JSON,
//! constrained by the class's `rdfs:label`, `rdfs:comment`, and the property
//! shapes derived from `rdfs:domain` + `rdfs:range` triples that target it.
//! The companion validator function (`validate_extraction`) then checks an
//! LLM-supplied JSON against the schema.

use crate::graph::GraphStore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractionScaffold {
    pub class_iri: String,
    pub class_label: Option<String>,
    pub class_comment: Option<String>,
    /// `{property_iri: range_iri or "literal"}`.
    pub property_schema: Vec<PropertySpec>,
    /// Prompt template ready to send to the orchestrator's LLM.
    pub prompt_template: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertySpec {
    pub property_iri: String,
    pub property_label: Option<String>,
    pub range: String,
    pub required: bool,
}

/// Build an extraction scaffold for `class_iri` from the loaded ontology.
pub fn build_scaffold(graph: &Arc<GraphStore>, class_iri: &str) -> anyhow::Result<ExtractionScaffold> {
    let label = single_str(graph, class_iri, "http://www.w3.org/2000/01/rdf-schema#label");
    let comment = single_str(graph, class_iri, "http://www.w3.org/2000/01/rdf-schema#comment");

    // Properties whose rdfs:domain is class_iri.
    let q = format!(
        r#"SELECT ?p ?range ?lbl WHERE {{
            ?p <http://www.w3.org/2000/01/rdf-schema#domain> <{}> .
            OPTIONAL {{ ?p <http://www.w3.org/2000/01/rdf-schema#range> ?range }}
            OPTIONAL {{ ?p <http://www.w3.org/2000/01/rdf-schema#label> ?lbl }}
        }}"#,
        class_iri
    );
    let mut props: Vec<PropertySpec> = Vec::new();
    if let Ok(js) = graph.sparql_select(&q)
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&js)
        && let Some(rows) = v["results"].as_array()
    {
        for row in rows {
            if let Some(p) = row["p"].as_str() {
                let p = p.trim_matches(|c| c == '<' || c == '>').to_string();
                let range = row["range"]
                    .as_str()
                    .map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string())
                    .unwrap_or_else(|| "literal".to_string());
                let lbl = row["lbl"]
                    .as_str()
                    .map(|s| s.trim_matches('"').to_string());
                props.push(PropertySpec {
                    property_iri: p,
                    property_label: lbl,
                    range,
                    required: false,
                });
            }
        }
    }

    let mut prompt = format!(
        "Extract every instance of `{}` from the supplied text and emit as JSON.\n\
         The class represents: {}\n\
         Description: {}\n\n\
         Each instance must be an object with these fields:\n",
        class_iri,
        label.as_deref().unwrap_or("(no label)"),
        comment.as_deref().unwrap_or("(no description)")
    );
    for p in &props {
        prompt.push_str(&format!(
            "  - `{}` (range: `{}`{})\n",
            p.property_label.as_deref().unwrap_or(&p.property_iri),
            p.range,
            if p.required { ", required" } else { "" }
        ));
    }
    prompt.push_str("\nReturn a JSON array `[ {...}, {...} ]`. Use `null` for unknown fields.");

    Ok(ExtractionScaffold {
        class_iri: class_iri.to_string(),
        class_label: label,
        class_comment: comment,
        property_schema: props,
        prompt_template: prompt,
    })
}

/// Validate an LLM-supplied JSON extraction against the scaffold. Returns
/// per-instance OK/error reports.
#[derive(Clone, Debug, Serialize)]
pub struct ValidationReport {
    pub total: usize,
    pub valid: usize,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ValidationIssue {
    pub instance_index: usize,
    pub field: Option<String>,
    pub message: String,
}

pub fn validate_extraction(
    scaffold: &ExtractionScaffold,
    extraction_json: &str,
) -> anyhow::Result<ValidationReport> {
    let v: serde_json::Value = serde_json::from_str(extraction_json)?;
    let arr = v.as_array().ok_or_else(|| anyhow::anyhow!("extraction must be a JSON array"))?;
    let mut issues: Vec<ValidationIssue> = Vec::new();
    let mut valid = 0usize;
    for (i, inst) in arr.iter().enumerate() {
        let Some(obj) = inst.as_object() else {
            issues.push(ValidationIssue {
                instance_index: i,
                field: None,
                message: "instance is not a JSON object".to_string(),
            });
            continue;
        };
        let mut instance_ok = true;
        for spec in &scaffold.property_schema {
            let key = spec.property_label.as_deref().unwrap_or(&spec.property_iri);
            if spec.required && !obj.contains_key(key) {
                issues.push(ValidationIssue {
                    instance_index: i,
                    field: Some(key.to_string()),
                    message: format!("required field `{}` missing", key),
                });
                instance_ok = false;
            }
        }
        if instance_ok {
            valid += 1;
        }
    }
    Ok(ValidationReport {
        total: arr.len(),
        valid,
        issues,
    })
}

fn single_str(graph: &Arc<GraphStore>, iri: &str, pred: &str) -> Option<String> {
    let q = format!("SELECT ?v WHERE {{ <{}> <{}> ?v }} LIMIT 1", iri, pred);
    let js = graph.sparql_select(&q).ok()?;
    let v: serde_json::Value = serde_json::from_str(&js).ok()?;
    v["results"][0]["v"]
        .as_str()
        .map(|s| s.trim_matches(|c| c == '"' || c == '<' || c == '>').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_with_class() -> Arc<GraphStore> {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex: <http://ex.org/> .
            ex:Person a owl:Class ;
                rdfs:label "Person" ;
                rdfs:comment "A human being." .
            ex:name a owl:DatatypeProperty ;
                rdfs:domain ex:Person ;
                rdfs:range <http://www.w3.org/2001/XMLSchema#string> ;
                rdfs:label "name" .
            ex:age a owl:DatatypeProperty ;
                rdfs:domain ex:Person ;
                rdfs:range <http://www.w3.org/2001/XMLSchema#integer> ;
                rdfs:label "age" .
        "#,
            None,
        )
        .unwrap();
        g
    }

    #[test]
    fn build_scaffold_pulls_class_metadata_and_properties() {
        let g = graph_with_class();
        let s = build_scaffold(&g, "http://ex.org/Person").unwrap();
        assert_eq!(s.class_label.as_deref(), Some("Person"));
        assert!(s.class_comment.as_deref().unwrap_or("").contains("human"));
        assert_eq!(s.property_schema.len(), 2);
        assert!(s.prompt_template.contains("Person"));
        assert!(s.prompt_template.contains("name"));
        assert!(s.prompt_template.contains("age"));
    }

    #[test]
    fn validate_extraction_accepts_well_formed_array() {
        let g = graph_with_class();
        let s = build_scaffold(&g, "http://ex.org/Person").unwrap();
        let extraction = r#"[{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]"#;
        let r = validate_extraction(&s, extraction).unwrap();
        assert_eq!(r.total, 2);
        assert_eq!(r.valid, 2);
        assert!(r.issues.is_empty());
    }

    #[test]
    fn validate_extraction_flags_non_object_instances() {
        let g = graph_with_class();
        let s = build_scaffold(&g, "http://ex.org/Person").unwrap();
        let extraction = r#"["not an object", {"name": "Bob"}]"#;
        let r = validate_extraction(&s, extraction).unwrap();
        assert_eq!(r.total, 2);
        assert_eq!(r.issues.len(), 1);
        assert_eq!(r.issues[0].instance_index, 0);
    }

    #[test]
    fn validate_extraction_rejects_non_array_root() {
        let g = graph_with_class();
        let s = build_scaffold(&g, "http://ex.org/Person").unwrap();
        let err = validate_extraction(&s, r#"{"not": "array"}"#).expect_err("should error");
        assert!(format!("{}", err).contains("must be a JSON array"));
    }
}
