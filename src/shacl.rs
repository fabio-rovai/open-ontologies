use crate::graph::GraphStore;
use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::sparql::QueryResults;
use oxigraph::store::Store;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

/// SHACL validator that checks data in a `GraphStore` against SHACL shapes.
///
/// Shapes are parsed from inline Turtle into a temporary Oxigraph store.
/// Constraints are translated into SPARQL queries run against the main graph.
/// Supports `sh:minCount`, `sh:maxCount`, and `sh:datatype` constraints.
pub struct ShaclValidator;

impl ShaclValidator {
    /// Validate the data in `graph` against SHACL shapes (inline Turtle).
    /// Returns a JSON report: `{conforms, violation_count, violations[]}`.
    pub fn validate(graph: &Arc<GraphStore>, shapes_ttl: &str) -> anyhow::Result<String> {
        // 1. Parse shapes Turtle into a temporary store
        let shapes_store = Store::new()?;
        let reader = Cursor::new(shapes_ttl.as_bytes());
        let parser = RdfParser::from_format(RdfFormat::Turtle).for_reader(reader);
        for quad in parser {
            shapes_store.insert(&quad?)?;
        }

        // 2. Find all sh:NodeShape with sh:targetClass
        let shapes = query_solutions(
            &shapes_store,
            r#"
            PREFIX sh: <http://www.w3.org/ns/shacl#>
            SELECT ?shape ?targetClass WHERE {
                ?shape a sh:NodeShape ;
                       sh:targetClass ?targetClass .
            }
            "#,
        )?;

        let mut violations: Vec<serde_json::Value> = Vec::new();

        for shape in &shapes {
            let target_class = match shape.get("targetClass") {
                Some(tc) => strip_angle_brackets(tc),
                None => continue,
            };

            // 3. Find property constraints for this shape
            let shape_iri = match shape.get("shape") {
                Some(s) => s.clone(),
                None => continue,
            };

            let props = query_solutions(
                &shapes_store,
                &format!(
                    r#"
                    PREFIX sh: <http://www.w3.org/ns/shacl#>
                    SELECT ?prop ?path ?minCount ?maxCount ?datatype ?message WHERE {{
                        {} sh:property ?prop .
                        ?prop sh:path ?path .
                        OPTIONAL {{ ?prop sh:minCount ?minCount }}
                        OPTIONAL {{ ?prop sh:maxCount ?maxCount }}
                        OPTIONAL {{ ?prop sh:datatype ?datatype }}
                        OPTIONAL {{ ?prop sh:message ?message }}
                    }}
                    "#,
                    shape_iri
                ),
            )?;

            // 4. For each constraint, run SPARQL queries against the main graph
            for prop in &props {
                let path = match prop.get("path") {
                    Some(p) => strip_angle_brackets(p),
                    None => continue,
                };

                let message = prop
                    .get("message")
                    .map(|m| strip_quotes(m))
                    .unwrap_or_default();

                // sh:minCount
                if let Some(min_count_str) = prop.get("minCount") {
                    let min_count = strip_quotes(min_count_str)
                        .parse::<u64>()
                        .unwrap_or(0);
                    if min_count > 0 {
                        let query = format!(
                            r#"SELECT ?focus (COUNT(?val) AS ?cnt) WHERE {{
                                ?focus a <{target_class}> .
                                OPTIONAL {{ ?focus <{path}> ?val }}
                            }} GROUP BY ?focus HAVING (COUNT(?val) < {min_count})"#
                        );
                        let results = graph_sparql_select(graph, &query)?;
                        for row in &results {
                            if let Some(focus) = row.get("focus") {
                                let msg = if message.is_empty() {
                                    format!(
                                        "Property <{}> has fewer than {} values",
                                        path, min_count
                                    )
                                } else {
                                    message.clone()
                                };
                                violations.push(serde_json::json!({
                                    "severity": "Violation",
                                    "focus_node": strip_angle_brackets(focus),
                                    "path": path,
                                    "constraint": "minCount",
                                    "message": msg,
                                }));
                            }
                        }
                    }
                }

                // sh:maxCount
                if let Some(max_count_str) = prop.get("maxCount") {
                    let max_count = strip_quotes(max_count_str)
                        .parse::<u64>()
                        .unwrap_or(u64::MAX);
                    let query = format!(
                        r#"SELECT ?focus (COUNT(?val) AS ?cnt) WHERE {{
                            ?focus a <{target_class}> .
                            ?focus <{path}> ?val .
                        }} GROUP BY ?focus HAVING (COUNT(?val) > {max_count})"#
                    );
                    let results = graph_sparql_select(graph, &query)?;
                    for row in &results {
                        if let Some(focus) = row.get("focus") {
                            let msg = if message.is_empty() {
                                format!(
                                    "Property <{}> has more than {} values",
                                    path, max_count
                                )
                            } else {
                                message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": "Violation",
                                "focus_node": strip_angle_brackets(focus),
                                "path": path,
                                "constraint": "maxCount",
                                "message": msg,
                            }));
                        }
                    }
                }

                // sh:datatype
                if let Some(dt_str) = prop.get("datatype") {
                    let dt = strip_angle_brackets(dt_str);
                    let query = format!(
                        r#"SELECT ?focus ?val WHERE {{
                            ?focus a <{target_class}> .
                            ?focus <{path}> ?val .
                            FILTER(DATATYPE(?val) != <{dt}>)
                        }}"#
                    );
                    let results = graph_sparql_select(graph, &query)?;
                    for row in &results {
                        if let Some(focus) = row.get("focus") {
                            let msg = if message.is_empty() {
                                format!(
                                    "Value does not have datatype <{}>",
                                    dt
                                )
                            } else {
                                message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": "Violation",
                                "focus_node": strip_angle_brackets(focus),
                                "path": path,
                                "constraint": "datatype",
                                "message": msg,
                            }));
                        }
                    }
                }
            }
        }

        let conforms = violations.is_empty();
        let report = serde_json::json!({
            "conforms": conforms,
            "violation_count": violations.len(),
            "violations": violations,
        });

        Ok(report.to_string())
    }

    /// Structural dry-run check on proposed SHACL shapes.
    ///
    /// Verifies that the shapes parse as Turtle and that every IRI they reference
    /// (`sh:targetClass`, `sh:path`, `sh:class`) actually exists in the loaded
    /// ontology, plus a lightweight XSD-prefix check on `sh:datatype`. Does NOT
    /// validate data against the shapes — that's `validate`. This is the primitive
    /// the orchestrating LLM needs to iterate on proposed SHACL before applying.
    ///
    /// Output is a JSON report with `ok` (true if no structural issues), `parses`,
    /// `shape_count`, and an `issues` array describing each missing reference.
    pub fn check_shapes(graph: &Arc<GraphStore>, shapes_ttl: &str) -> anyhow::Result<String> {
        // 1. Parse the proposed shapes into a temporary Oxigraph store.
        let shapes_store = Store::new()?;
        let reader = Cursor::new(shapes_ttl.as_bytes());
        let parser = RdfParser::from_format(RdfFormat::Turtle).for_reader(reader);
        for quad in parser {
            match quad {
                Ok(q) => shapes_store.insert(&q)?,
                Err(e) => {
                    return Ok(serde_json::json!({
                        "ok": false,
                        "parses": false,
                        "parse_error": format!("{}", e),
                        "issues": [],
                        "issue_count": 0,
                        "shape_count": 0,
                    })
                    .to_string());
                }
            };
        }

        // 2. Walk every NodeShape and collect its referenced IRIs (target_class +
        //    per-property path + optional class constraint + datatype).
        let shapes = query_solutions(
            &shapes_store,
            r#"
            PREFIX sh: <http://www.w3.org/ns/shacl#>
            SELECT ?shape ?targetClass WHERE {
                ?shape a sh:NodeShape ;
                       sh:targetClass ?targetClass .
            }
            "#,
        )?;

        let mut issues: Vec<serde_json::Value> = Vec::new();
        let mut shape_reports: Vec<serde_json::Value> = Vec::new();

        for shape in &shapes {
            let shape_iri = match shape.get("shape") {
                Some(s) => s.clone(),
                None => continue,
            };
            let target_class = match shape.get("targetClass") {
                Some(tc) => strip_angle_brackets(tc),
                None => continue,
            };

            let target_class_exists = class_exists(graph, &target_class)?;
            if !target_class_exists {
                issues.push(serde_json::json!({
                    "shape": strip_angle_brackets(&shape_iri),
                    "kind": "missing_target_class",
                    "value": target_class,
                    "message": format!(
                        "sh:targetClass <{}> is not declared as owl:Class or rdfs:Class in the loaded ontology",
                        target_class
                    ),
                }));
            }

            let props = query_solutions(
                &shapes_store,
                &format!(
                    r#"
                    PREFIX sh: <http://www.w3.org/ns/shacl#>
                    SELECT ?prop ?path ?class ?datatype WHERE {{
                        {} sh:property ?prop .
                        ?prop sh:path ?path .
                        OPTIONAL {{ ?prop sh:class ?class }}
                        OPTIONAL {{ ?prop sh:datatype ?datatype }}
                    }}
                    "#,
                    shape_iri
                ),
            )?;

            let mut prop_reports: Vec<serde_json::Value> = Vec::new();
            for prop in &props {
                let path = match prop.get("path") {
                    Some(p) => strip_angle_brackets(p),
                    None => continue,
                };
                let path_exists = property_exists(graph, &path)?;
                if !path_exists {
                    issues.push(serde_json::json!({
                        "shape": strip_angle_brackets(&shape_iri),
                        "kind": "missing_path",
                        "value": path.clone(),
                        "message": format!(
                            "sh:path <{}> is not declared as a property (owl:ObjectProperty, owl:DatatypeProperty, or rdf:Property) in the loaded ontology",
                            path
                        ),
                    }));
                }

                let class_constraint = prop.get("class").map(|c| strip_angle_brackets(c));
                let class_exists_value = match &class_constraint {
                    Some(iri) => {
                        let exists = class_exists(graph, iri)?;
                        if !exists {
                            issues.push(serde_json::json!({
                                "shape": strip_angle_brackets(&shape_iri),
                                "kind": "missing_class_constraint",
                                "value": iri.clone(),
                                "message": format!(
                                    "sh:class <{}> is not declared as owl:Class or rdfs:Class in the loaded ontology",
                                    iri
                                ),
                            }));
                        }
                        Some(exists)
                    }
                    None => None,
                };

                let datatype = prop.get("datatype").map(|d| strip_angle_brackets(d));
                let datatype_ok = datatype.as_deref().map(is_recognised_xsd_datatype);
                if let (Some(dt), Some(false)) = (datatype.as_deref(), datatype_ok) {
                    let dt_owned: String = dt.to_owned();
                    issues.push(serde_json::json!({
                        "shape": strip_angle_brackets(&shape_iri),
                        "kind": "unrecognised_datatype",
                        "value": dt_owned,
                        "message": format!(
                            "sh:datatype <{}> does not look like an XSD datatype IRI (expected something starting with http://www.w3.org/2001/XMLSchema#)",
                            dt
                        ),
                    }));
                }

                prop_reports.push(serde_json::json!({
                    "path": path,
                    "path_exists": path_exists,
                    "class_constraint": class_constraint,
                    "class_constraint_exists": class_exists_value,
                    "datatype": datatype,
                    "datatype_recognised": datatype_ok,
                }));
            }

            shape_reports.push(serde_json::json!({
                "shape_iri": strip_angle_brackets(&shape_iri),
                "target_class": target_class,
                "target_class_exists": target_class_exists,
                "property_constraints": prop_reports,
            }));
        }

        let ok = issues.is_empty();
        Ok(serde_json::json!({
            "ok": ok,
            "parses": true,
            "shape_count": shape_reports.len(),
            "issue_count": issues.len(),
            "issues": issues,
            "shapes": shape_reports,
        })
        .to_string())
    }
}

/// Run a SPARQL SELECT against a temporary shapes `Store` and return results
/// as a vec of maps (variable name -> string value).
fn query_solutions(
    store: &Store,
    query: &str,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    match store.query(query)? {
        QueryResults::Solutions(solutions) => {
            let vars: Vec<String> = solutions
                .variables()
                .iter()
                .map(|v| v.as_str().to_string())
                .collect();
            let mut rows = Vec::new();
            for solution in solutions {
                let solution = solution?;
                let mut row = HashMap::new();
                for var in &vars {
                    if let Some(term) = solution.get(var.as_str()) {
                        row.insert(var.clone(), term.to_string());
                    }
                }
                rows.push(row);
            }
            Ok(rows)
        }
        _ => Ok(Vec::new()),
    }
}

/// Run a SPARQL SELECT against the main `GraphStore` and return results
/// as a vec of maps, using the existing `sparql_select` JSON output.
fn graph_sparql_select(
    graph: &Arc<GraphStore>,
    query: &str,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    let json_str = graph.sparql_select(query)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
    let mut rows = Vec::new();
    if let Some(results) = parsed["results"].as_array() {
        for result in results {
            if let Some(obj) = result.as_object() {
                let mut row = HashMap::new();
                for (key, val) in obj {
                    if let Some(s) = val.as_str() {
                        row.insert(key.clone(), s.to_string());
                    }
                }
                rows.push(row);
            }
        }
    }
    Ok(rows)
}

/// Check whether `iri` is declared as `owl:Class` or `rdfs:Class` in the graph.
fn class_exists(graph: &Arc<GraphStore>, iri: &str) -> anyhow::Result<bool> {
    let query = format!(
        r#"SELECT ?x WHERE {{
            <{iri}> a ?type .
            FILTER(?type = <http://www.w3.org/2002/07/owl#Class>
                || ?type = <http://www.w3.org/2000/01/rdf-schema#Class>)
        }} LIMIT 1"#
    );
    let results = graph_sparql_select(graph, &query)?;
    Ok(!results.is_empty())
}

/// Check whether `iri` is declared as an `owl:ObjectProperty`,
/// `owl:DatatypeProperty`, or `rdf:Property` in the graph.
fn property_exists(graph: &Arc<GraphStore>, iri: &str) -> anyhow::Result<bool> {
    let query = format!(
        r#"SELECT ?x WHERE {{
            <{iri}> a ?type .
            FILTER(?type = <http://www.w3.org/2002/07/owl#ObjectProperty>
                || ?type = <http://www.w3.org/2002/07/owl#DatatypeProperty>
                || ?type = <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property>)
        }} LIMIT 1"#
    );
    let results = graph_sparql_select(graph, &query)?;
    Ok(!results.is_empty())
}

/// Quick prefix check for XSD datatypes (the SHACL spec allows others,
/// but the overwhelming majority of real-world `sh:datatype` constraints are XSD).
fn is_recognised_xsd_datatype(iri: &str) -> bool {
    iri.starts_with("http://www.w3.org/2001/XMLSchema#")
}

/// Trim angle brackets from IRI strings like `<http://example.org/foo>`.
fn strip_angle_brackets(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('<') && s.ends_with('>') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Trim quotes and handle typed literals like `"1"^^<http://...>`.
fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    // Handle typed literals: "value"^^<datatype>
    let s = if let Some(idx) = s.find("^^") {
        &s[..idx]
    } else {
        s
    };
    // Handle language-tagged literals: "value"@en
    let s = if let Some(idx) = s.find("\"@") {
        &s[..idx + 1]
    } else {
        s
    };
    // Strip surrounding quotes
    let s = s.trim_matches('"');
    s.to_string()
}
