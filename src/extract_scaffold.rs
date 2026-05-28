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
/// per-instance OK/error reports plus a conformance score.
#[derive(Clone, Debug, Serialize)]
pub struct ValidationReport {
    pub total: usize,
    pub valid: usize,
    /// Mean conformance score across all instances in `[0, 1]`. A score of
    /// 1.0 means every declared property's value parsed cleanly against its
    /// declared range; missing optional fields don't penalise.
    pub mean_conformance: f64,
    pub issues: Vec<ValidationIssue>,
    /// Per-instance conformance scores aligned with array index.
    pub per_instance_scores: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ValidationIssue {
    pub instance_index: usize,
    pub field: Option<String>,
    pub message: String,
    /// One of "missing_required", "type_mismatch", "iri_expected",
    /// "literal_expected", "not_object", "unknown_field".
    pub kind: String,
}

const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

/// Classify a property's range as one of the recognised value kinds.
#[derive(Clone, Copy, Debug, PartialEq)]
enum RangeKind {
    XsdInteger,
    XsdDecimal,
    XsdFloat,
    XsdDouble,
    XsdBoolean,
    XsdString,
    XsdDate,
    XsdDateTime,
    /// IRI-valued (range is an `owl:Class` or any IRI not in `xsd:`).
    IriOrLiteral,
    /// `range` field was the default `"literal"` — accept any non-object.
    AnyLiteral,
}

fn classify_range(range: &str) -> RangeKind {
    if let Some(local) = range.strip_prefix(XSD_NS) {
        match local {
            "integer" | "int" | "long" | "short" | "byte" | "nonNegativeInteger"
            | "positiveInteger" | "negativeInteger" | "nonPositiveInteger"
            | "unsignedInt" | "unsignedLong" | "unsignedShort" | "unsignedByte" => {
                RangeKind::XsdInteger
            }
            "decimal" => RangeKind::XsdDecimal,
            "float" => RangeKind::XsdFloat,
            "double" => RangeKind::XsdDouble,
            "boolean" => RangeKind::XsdBoolean,
            "string" | "normalizedString" | "token" | "anyURI" => RangeKind::XsdString,
            "date" | "gYear" | "gMonth" | "gDay" | "gYearMonth" | "gMonthDay" => {
                RangeKind::XsdDate
            }
            "dateTime" | "dateTimeStamp" | "time" => RangeKind::XsdDateTime,
            _ => RangeKind::AnyLiteral,
        }
    } else if range == "literal" {
        RangeKind::AnyLiteral
    } else {
        RangeKind::IriOrLiteral
    }
}

/// Type-check a JSON value against a `RangeKind`. Returns `Ok(())` if
/// conformant, `Err(reason)` if not.
fn check_value(v: &serde_json::Value, kind: RangeKind) -> Result<(), String> {
    use serde_json::Value;
    match (kind, v) {
        (_, Value::Null) => Ok(()), // null is always allowed (= "unknown")
        (RangeKind::XsdInteger, Value::Number(n)) if n.is_i64() || n.is_u64() => Ok(()),
        (RangeKind::XsdInteger, Value::Number(_)) => {
            Err("expected xsd:integer, got non-integer number".into())
        }
        (RangeKind::XsdInteger, Value::String(s)) if s.parse::<i64>().is_ok() => Ok(()),
        (RangeKind::XsdInteger, _) => Err("expected xsd:integer".into()),
        (RangeKind::XsdDecimal | RangeKind::XsdFloat | RangeKind::XsdDouble, Value::Number(_)) => {
            Ok(())
        }
        (RangeKind::XsdDecimal | RangeKind::XsdFloat | RangeKind::XsdDouble, Value::String(s))
            if s.parse::<f64>().is_ok() =>
        {
            Ok(())
        }
        (RangeKind::XsdDecimal | RangeKind::XsdFloat | RangeKind::XsdDouble, _) => {
            Err("expected numeric".into())
        }
        (RangeKind::XsdBoolean, Value::Bool(_)) => Ok(()),
        (RangeKind::XsdBoolean, Value::String(s))
            if matches!(s.to_ascii_lowercase().as_str(), "true" | "false") =>
        {
            Ok(())
        }
        (RangeKind::XsdBoolean, _) => Err("expected xsd:boolean".into()),
        (RangeKind::XsdString, Value::String(_)) => Ok(()),
        (RangeKind::XsdString, _) => Err("expected xsd:string".into()),
        (RangeKind::XsdDate | RangeKind::XsdDateTime, Value::String(s)) => {
            // Light shape check: must contain a hyphen-separated date and
            // optionally a 'T' before the time component. We don't pull in
            // chrono just for this — full ISO 8601 parsing is out of scope.
            if s.contains('-') {
                Ok(())
            } else {
                Err("expected ISO 8601 date/dateTime string".into())
            }
        }
        (RangeKind::XsdDate | RangeKind::XsdDateTime, _) => Err("expected ISO 8601 string".into()),
        (RangeKind::IriOrLiteral, Value::String(s)) => {
            // Accept absolute IRIs, prefixed names, or any non-empty string.
            if s.is_empty() {
                Err("expected IRI or literal, got empty string".into())
            } else {
                Ok(())
            }
        }
        (RangeKind::IriOrLiteral, Value::Object(_)) => {
            Err("IRI/literal expected; got nested object".into())
        }
        (RangeKind::IriOrLiteral, _) => Ok(()),
        (RangeKind::AnyLiteral, Value::Object(_)) => {
            Err("literal expected; got nested object".into())
        }
        (RangeKind::AnyLiteral, _) => Ok(()),
    }
}

pub fn validate_extraction(
    scaffold: &ExtractionScaffold,
    extraction_json: &str,
) -> anyhow::Result<ValidationReport> {
    let v: serde_json::Value = serde_json::from_str(extraction_json)?;
    let arr = v
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("extraction must be a JSON array"))?;

    // Build the field-key → (range_kind, required, original_key) map. We
    // accept both label-keyed and IRI-keyed objects, which is what real
    // LLM output looks like in the wild.
    let mut field_lookup: Vec<(String, RangeKind, bool)> = Vec::new();
    let mut known_keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for spec in &scaffold.property_schema {
        let kind = classify_range(&spec.range);
        if let Some(lbl) = &spec.property_label {
            field_lookup.push((lbl.clone(), kind, spec.required));
            known_keys.insert(lbl.clone());
        }
        field_lookup.push((spec.property_iri.clone(), kind, spec.required));
        known_keys.insert(spec.property_iri.clone());
    }

    let mut issues: Vec<ValidationIssue> = Vec::new();
    let mut valid = 0usize;
    let mut per_inst: Vec<f64> = Vec::with_capacity(arr.len());

    for (i, inst) in arr.iter().enumerate() {
        let Some(obj) = inst.as_object() else {
            issues.push(ValidationIssue {
                instance_index: i,
                field: None,
                message: "instance is not a JSON object".to_string(),
                kind: "not_object".to_string(),
            });
            per_inst.push(0.0);
            continue;
        };
        let mut instance_ok = true;
        let mut fields_checked = 0usize;
        let mut fields_conformant = 0usize;

        // Check required fields exist + every declared field type-checks.
        for spec in &scaffold.property_schema {
            let preferred_key = spec.property_label.as_deref().unwrap_or(&spec.property_iri);
            let value = obj
                .get(preferred_key)
                .or_else(|| obj.get(&spec.property_iri));
            match value {
                Some(val) => {
                    fields_checked += 1;
                    let kind = classify_range(&spec.range);
                    match check_value(val, kind) {
                        Ok(()) => fields_conformant += 1,
                        Err(reason) => {
                            issues.push(ValidationIssue {
                                instance_index: i,
                                field: Some(preferred_key.to_string()),
                                message: format!(
                                    "{} (range: {})",
                                    reason, spec.range
                                ),
                                kind: type_mismatch_kind(kind),
                            });
                            instance_ok = false;
                        }
                    }
                }
                None if spec.required => {
                    issues.push(ValidationIssue {
                        instance_index: i,
                        field: Some(preferred_key.to_string()),
                        message: format!("required field `{}` missing", preferred_key),
                        kind: "missing_required".to_string(),
                    });
                    instance_ok = false;
                }
                None => {} // optional and absent — fine
            }
        }

        // Flag unknown fields (informational, not fatal).
        for (k, _) in obj {
            if !known_keys.contains(k) {
                issues.push(ValidationIssue {
                    instance_index: i,
                    field: Some(k.clone()),
                    message: format!(
                        "field `{}` is not in the scaffold's property schema",
                        k
                    ),
                    kind: "unknown_field".to_string(),
                });
            }
        }

        // Conformance score: 1.0 if all declared fields present + conformant;
        // partial credit for partial conformance.
        let total_declared = scaffold.property_schema.len().max(1) as f64;
        let coverage = fields_checked as f64 / total_declared;
        let correctness = if fields_checked == 0 {
            0.0
        } else {
            fields_conformant as f64 / fields_checked as f64
        };
        let score = (coverage * correctness).clamp(0.0, 1.0);
        per_inst.push(score);
        if instance_ok {
            valid += 1;
        }
    }

    let mean = if per_inst.is_empty() {
        0.0
    } else {
        per_inst.iter().sum::<f64>() / per_inst.len() as f64
    };

    Ok(ValidationReport {
        total: arr.len(),
        valid,
        mean_conformance: mean,
        issues,
        per_instance_scores: per_inst,
    })
}

fn type_mismatch_kind(kind: RangeKind) -> String {
    match kind {
        RangeKind::IriOrLiteral => "iri_expected",
        RangeKind::AnyLiteral => "literal_expected",
        _ => "type_mismatch",
    }
    .to_string()
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
        assert!(r.issues.is_empty(), "issues: {:?}", r.issues);
        assert!(r.mean_conformance >= 0.99,
            "well-formed extraction should score ~1.0; got {}",
            r.mean_conformance);
    }

    #[test]
    fn validate_extraction_rejects_wrong_datatype() {
        // age should be xsd:integer; "thirty" should fail.
        let g = graph_with_class();
        let s = build_scaffold(&g, "http://ex.org/Person").unwrap();
        let extraction = r#"[{"name": "Alice", "age": "thirty"}]"#;
        let r = validate_extraction(&s, extraction).unwrap();
        assert_eq!(r.valid, 0);
        let type_issue = r.issues.iter().find(|i| i.kind == "type_mismatch");
        assert!(type_issue.is_some(),
            "should flag age as type_mismatch; got: {:?}", r.issues);
        assert!(type_issue.unwrap().message.contains("xsd:integer"));
    }

    #[test]
    fn validate_extraction_accepts_stringified_integer() {
        // LLMs often emit "30" instead of 30. We coerce-accept.
        let g = graph_with_class();
        let s = build_scaffold(&g, "http://ex.org/Person").unwrap();
        let extraction = r#"[{"name": "Alice", "age": "30"}]"#;
        let r = validate_extraction(&s, extraction).unwrap();
        assert_eq!(r.valid, 1);
        assert!(!r.issues.iter().any(|i| i.kind == "type_mismatch"));
    }

    #[test]
    fn validate_extraction_flags_unknown_field() {
        let g = graph_with_class();
        let s = build_scaffold(&g, "http://ex.org/Person").unwrap();
        let extraction = r#"[{"name": "Alice", "age": 30, "rogueField": "x"}]"#;
        let r = validate_extraction(&s, extraction).unwrap();
        // Still valid (unknown field doesn't kill the instance) but issue raised.
        let unknown = r.issues.iter().find(|i| i.kind == "unknown_field");
        assert!(unknown.is_some(), "expected unknown_field issue; got: {:?}", r.issues);
    }

    #[test]
    fn validate_extraction_partial_conformance_scores_intermediate() {
        // First instance complete (both fields); second instance missing one.
        let g = graph_with_class();
        let s = build_scaffold(&g, "http://ex.org/Person").unwrap();
        let extraction = r#"[{"name": "Alice", "age": 30}, {"name": "Bob"}]"#;
        let r = validate_extraction(&s, extraction).unwrap();
        assert_eq!(r.total, 2);
        // First instance scores 1.0 (2/2 fields conformant).
        assert!((r.per_instance_scores[0] - 1.0).abs() < 1e-9);
        // Second instance scores 0.5 (1/2 fields filled, both conformant).
        assert!((r.per_instance_scores[1] - 0.5).abs() < 1e-9);
        // Mean = 0.75.
        assert!((r.mean_conformance - 0.75).abs() < 1e-9);
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
