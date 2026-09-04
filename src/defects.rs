//! Defects in the ontology itself, found before any data is judged against it.
//!
//! A self-contradicting ontology makes every fact-level conclusion suspect, so
//! the declarations are checked on their own first. This asks a different
//! question from `onto_dl_check`. Satisfiability asks whether a model exists;
//! these checks ask whether a pair of declarations will manufacture
//! contradictions once instances arrive. A property declared both transitive
//! and functional is satisfiable and is still a trap, so the tableaux reasoner
//! is right to pass it and it is still worth reporting.
//!
//! Every kind here is decided from the declarations alone. Nothing is inferred,
//! and no data is required.

use std::sync::Arc;

use crate::graph::GraphStore;

const OWL: &str = "http://www.w3.org/2002/07/owl#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";

/// One check: the name it is reported under, the query that finds it, and the
/// sentence that says why the reader should care.
struct Check {
    kind: &'static str,
    /// How much the finding should weigh on the reader.
    ///
    /// `error` means something can have no instances at all. `warning` means
    /// the declarations will manufacture contradictions once data arrives, or
    /// that a hierarchy says nothing. `info` means the entailment is unaffected
    /// and only a reader is left uninformed. Without the rank the useful
    /// finding is buried: a sweep over the 153 readable ontologies in this repo
    /// returned 27 findings, 25 of them the mildest kind.
    severity: &'static str,
    query: String,
    explanation: &'static str,
}

pub struct Defects;

impl Defects {
    /// The most rows of any one kind that are listed individually.
    ///
    /// Anything reported item by item needs an upper bound. One bad
    /// declaration in a shared vocabulary produces thousands of identical
    /// rows, and a reader facing a thousand identical rows decides nothing
    /// from them. The total is still reported under `truncated`, so the cap
    /// hides the listing and never the scale.
    pub const MAX_PER_KIND: usize = 50;

    pub fn check(graph: &Arc<GraphStore>) -> anyhow::Result<String> {
        let checks = Self::checks();
        let kinds_checked: Vec<&str> = checks.iter().map(|c| c.kind).collect();

        let mut defects: Vec<serde_json::Value> = Vec::new();
        let mut truncated = serde_json::Map::new();
        let mut total = 0usize;
        let (mut errors, mut warnings, mut infos) = (0usize, 0usize, 0usize);

        for check in &checks {
            // The union dataset, so the answer does not depend on whether the
            // ontology arrived as Turtle or inside a named graph.
            let raw = graph.sparql_select_union(&check.query)?;
            let parsed: serde_json::Value = serde_json::from_str(&raw)?;
            let rows = parsed["results"].as_array().cloned().unwrap_or_default();

            total += rows.len();
            match check.severity {
                "error" => errors += rows.len(),
                "warning" => warnings += rows.len(),
                _ => infos += rows.len(),
            }
            if rows.len() > Self::MAX_PER_KIND {
                truncated.insert(check.kind.to_string(), serde_json::json!(rows.len()));
            }

            for row in rows.iter().take(Self::MAX_PER_KIND) {
                let mut entry = serde_json::Map::new();
                entry.insert("kind".into(), serde_json::json!(check.kind));
                entry.insert("severity".into(), serde_json::json!(check.severity));
                entry.insert("explanation".into(), serde_json::json!(check.explanation));
                if let Some(bindings) = row.as_object() {
                    for (var, value) in bindings {
                        entry.insert(var.clone(), value.clone());
                    }
                }
                defects.push(serde_json::Value::Object(entry));
            }
        }

        Ok(serde_json::json!({
            "defect_count": total,
            // Always all three keys, so a reader never has to tell "none of
            // this kind" from "this key was not written".
            "severity_counts": {"error": errors, "warning": warnings, "info": infos},
            "reported_count": defects.len(),
            "defects": defects,
            "kinds_checked": kinds_checked,
            "truncated": truncated,
        })
        .to_string())
    }

    fn checks() -> Vec<Check> {
        vec![
            Check {
                kind: "transitive_and_functional",
                severity: "warning",
                query: format!(
                    "SELECT DISTINCT ?property WHERE {{
                       ?property a <{OWL}TransitiveProperty> .
                       ?property a <{OWL}FunctionalProperty> .
                     }}"
                ),
                explanation: "A transitive property chains and a functional property admits one \
                              object, so any chain of length two forces two objects to be the \
                              same. The pair is satisfiable and will produce contradictions as \
                              soon as instances arrive.",
            },
            Check {
                kind: "symmetric_and_asymmetric",
                severity: "error",
                query: format!(
                    "SELECT DISTINCT ?property WHERE {{
                       ?property a <{OWL}SymmetricProperty> .
                       ?property a <{OWL}AsymmetricProperty> .
                     }}"
                ),
                explanation: "Symmetric requires the reverse edge and asymmetric forbids it, so \
                              the property can never hold between two distinct individuals.",
            },
            Check {
                kind: "subclass_cycle",
                severity: "warning",
                query: format!(
                    "SELECT DISTINCT ?class WHERE {{
                       ?class <{RDFS}subClassOf>+ ?other .
                       ?other <{RDFS}subClassOf>+ ?class .
                       FILTER(?class != ?other)
                     }}"
                ),
                explanation: "Classes on a subclass cycle are all equivalent, which is almost \
                              never what a hierarchy was meant to say.",
            },
            Check {
                kind: "sub_property_cycle",
                severity: "warning",
                query: format!(
                    "SELECT DISTINCT ?property WHERE {{
                       ?property <{RDFS}subPropertyOf>+ ?other .
                       ?other <{RDFS}subPropertyOf>+ ?property .
                       FILTER(?property != ?other)
                     }}"
                ),
                explanation: "Properties on a sub-property cycle are all equivalent, so the \
                              hierarchy asserts nothing.",
            },
            Check {
                kind: "disjoint_with_ancestor",
                severity: "error",
                query: format!(
                    "SELECT DISTINCT ?class ?ancestor WHERE {{
                       ?class <{RDFS}subClassOf>+ ?ancestor .
                       {{ ?class <{OWL}disjointWith> ?ancestor }}
                       UNION
                       {{ ?ancestor <{OWL}disjointWith> ?class }}
                     }}"
                ),
                explanation: "A class that is a kind of something it is also disjoint from can \
                              have no instances at all.",
            },
            Check {
                kind: "inherited_disjoint",
                severity: "error",
                query: format!(
                    "SELECT DISTINCT ?class ?first ?second WHERE {{
                       ?class <{RDFS}subClassOf>+ ?first .
                       ?class <{RDFS}subClassOf>+ ?second .
                       {{ ?first <{OWL}disjointWith> ?second }}
                       UNION
                       {{ ?second <{OWL}disjointWith> ?first }}
                       FILTER(STR(?first) < STR(?second))
                     }}"
                ),
                explanation: "The class sits under two ancestors declared disjoint, so it can \
                              have no instances.",
            },
            Check {
                kind: "self_inverse",
                severity: "info",
                query: format!(
                    "SELECT DISTINCT ?property WHERE {{
                       ?property <{OWL}inverseOf> ?property .
                       FILTER NOT EXISTS {{ ?property a <{OWL}SymmetricProperty> }}
                     }}"
                ),
                explanation: "A property declared its own inverse is symmetric by consequence. \
                              Declare it symmetric and say so, or the inverse was a slip.",
            },
            Check {
                kind: "inverse_not_mutual",
                severity: "info",
                query: format!(
                    "SELECT DISTINCT ?property ?inverse WHERE {{
                       ?property <{OWL}inverseOf> ?inverse .
                       FILTER NOT EXISTS {{ ?inverse <{OWL}inverseOf> ?property }}
                     }}"
                ),
                explanation: "One direction of the pair was declared and the other was not. The \
                              entailment holds either way, but a reader consulting the second \
                              property alone learns nothing about the first.",
            },
        ]
    }
}
