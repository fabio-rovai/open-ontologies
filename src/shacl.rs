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
