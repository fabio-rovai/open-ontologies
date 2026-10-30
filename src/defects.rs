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
    /// The query variable naming the thing the finding is ABOUT, and whether
    /// that thing is a class or a property. One subject can appear in several
    /// rows: `inherited_disjoint` returns one row per (class, disjoint pair),
    /// so a class under two disjoint pairs is two findings and one affected
    /// class (#341).
    subject: &'static str,
    subject_is: SubjectIs,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubjectIs {
    Class,
    Property,
}

/// One row of one check, kept whole so two graphs' findings can be compared.
///
/// `plan` uses this to report only the defects a change INTRODUCES (#336): a
/// store that already had a cycle should not make every later plan look as
/// though it caused one.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Finding {
    pub kind: &'static str,
    pub severity: &'static str,
    /// The class or property the finding is about.
    pub subject: String,
    /// Every binding of the row, sorted by variable name, subject included.
    pub bindings: Vec<(String, String)>,
}

impl Finding {
    /// The finding as it is listed in a report: kind, severity, the reason to
    /// care, and the row's bindings.
    pub fn to_json(&self) -> serde_json::Value {
        let mut entry = serde_json::Map::new();
        entry.insert("kind".into(), serde_json::json!(self.kind));
        entry.insert("severity".into(), serde_json::json!(self.severity));
        let explanation = Defects::checks()
            .into_iter()
            .find(|c| c.kind == self.kind)
            .map(|c| c.explanation)
            .unwrap_or_default();
        entry.insert("explanation".into(), serde_json::json!(explanation));
        for (var, value) in &self.bindings {
            entry.insert(var.clone(), serde_json::json!(value));
        }
        serde_json::Value::Object(entry)
    }
}

pub struct Defects;

impl Defects {
    /// The most rows of any one kind that are listed individually, unless the
    /// caller asks for another bound.
    ///
    /// Anything reported item by item needs an upper bound. One bad
    /// declaration in a shared vocabulary produces thousands of identical
    /// rows, and a reader facing a thousand identical rows decides nothing
    /// from them. The total is still reported under `truncated`, so the cap
    /// hides the listing and never the scale. It is a DEFAULT and not a wall:
    /// `--limit N` on the CLI and `limit` on `onto_defects` move it, and `0`
    /// lists every row, because a total the reader can see and cannot reach is
    /// a report that stops one step short (#341).
    pub const MAX_PER_KIND: usize = 50;

    /// The report, listing at most [`Defects::MAX_PER_KIND`] rows of each kind.
    pub fn check(graph: &Arc<GraphStore>) -> anyhow::Result<String> {
        Self::check_with_limit(graph, Self::MAX_PER_KIND)
    }

    /// The report, listing at most `limit` rows of each kind. `0` lists all.
    ///
    /// Two counts, because they answer two questions. `defect_count` counts
    /// FINDINGS, one per query row: for `inherited_disjoint` a row is a
    /// (class, disjoint pair), so a class under two disjoint pairs is two
    /// findings. `affected_classes` and `affected_properties` count the
    /// DISTINCT subjects those findings are about, which is the number to
    /// hold against a reasoner's list of unsatisfiable classes. `kind_counts`
    /// gives both per kind. Before #341 only the first existed, and on a large
    /// ontology it read as a count of broken classes that was larger than the
    /// reasoner's, with nothing in the output to say why.
    pub fn check_with_limit(graph: &Arc<GraphStore>, limit: usize) -> anyhow::Result<String> {
        let checks = Self::checks();
        let kinds_checked: Vec<&str> = checks.iter().map(|c| c.kind).collect();
        let findings = Self::findings(graph)?;
        let cap = if limit == 0 { usize::MAX } else { limit };
        // A class declared an exception with `exceptFor` is still an error
        // here, because the disjointness is still asserted and every reasoner
        // still finds the class unsatisfiable; the tools must not disagree.
        // The row says so and names the compile that resolves it (#347). It
        // is decoration on the listing and not part of `Finding`, so `plan`,
        // which compares findings, does not see an annotation as a change.
        let declared = crate::exceptions::Exceptions::declared(graph)?;
        let declared_exception = |f: &Finding| -> Option<String> {
            if f.kind != "inherited_disjoint" || declared.is_empty() {
                return None;
            }
            let get = |var: &str| f.bindings.iter().find(|(k, _)| k == var).map(|(_, v)| v.as_str());
            let (first, second) = (get("first")?, get("second")?);
            declared.covering(first, second, &f.subject).map(str::to_string)
        };
        let declared_count = findings.iter().filter(|f| declared_exception(f).is_some()).count();

        let mut defects: Vec<serde_json::Value> = Vec::new();
        let mut truncated = serde_json::Map::new();
        let mut kind_counts = serde_json::Map::new();
        let (mut errors, mut warnings, mut infos) = (0usize, 0usize, 0usize);
        let mut classes: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        let mut properties: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();

        for check in &checks {
            let rows: Vec<&Finding> = findings.iter().filter(|f| f.kind == check.kind).collect();
            match check.severity {
                "error" => errors += rows.len(),
                "warning" => warnings += rows.len(),
                _ => infos += rows.len(),
            }
            let subjects: std::collections::BTreeSet<&str> =
                rows.iter().map(|f| f.subject.as_str()).collect();
            let affected_key = match check.subject_is {
                SubjectIs::Class => {
                    classes.extend(subjects.iter().copied());
                    "affected_classes"
                }
                SubjectIs::Property => {
                    properties.extend(subjects.iter().copied());
                    "affected_properties"
                }
            };
            kind_counts.insert(
                check.kind.to_string(),
                serde_json::json!({"findings": rows.len(), affected_key: subjects.len()}),
            );
            // `truncated` names a kind exactly when some of its rows are not
            // listed, so it is keyed on the bound in force rather than on the
            // default one.
            if rows.len() > cap {
                truncated.insert(check.kind.to_string(), serde_json::json!(rows.len()));
            }
            defects.extend(rows.iter().take(cap).map(|f| {
                let mut row = f.to_json();
                if let Some(exception) = declared_exception(f) {
                    row["declared_exception"] = serde_json::json!(true);
                    row["hint"] = serde_json::json!(crate::exceptions::defects_hint(&exception));
                    row["exception"] = serde_json::json!(exception);
                }
                row
            }));
        }

        Ok(serde_json::json!({
            // FINDINGS, one per query row. See `affected_classes` for the
            // number of distinct classes they are about.
            "defect_count": findings.len(),
            "affected_classes": classes.len(),
            "affected_properties": properties.len(),
            "kind_counts": kind_counts,
            // Always all three keys, so a reader never has to tell "none of
            // this kind" from "this key was not written".
            "severity_counts": {"error": errors, "warning": warnings, "info": infos},
            // `inherited_disjoint` findings whose class is a declared
            // `exceptFor` exception not yet compiled, listed or not. Still
            // counted as errors above.
            "declared_exceptions": declared_count,
            "reported_count": defects.len(),
            "limit_per_kind": limit,
            "defects": defects,
            "kinds_checked": kinds_checked,
            "truncated": truncated,
        })
        .to_string())
    }

    /// Every finding of every kind, uncapped, in check order and then in the
    /// order of the subject and bindings.
    pub fn findings(graph: &Arc<GraphStore>) -> anyhow::Result<Vec<Finding>> {
        let mut out = Vec::new();
        for check in Self::checks() {
            // The union dataset, so the answer does not depend on whether the
            // ontology arrived as Turtle or inside a named graph.
            let raw = graph.sparql_select_union(&check.query)?;
            let parsed: serde_json::Value = serde_json::from_str(&raw)?;
            let rows = parsed["results"].as_array().cloned().unwrap_or_default();
            let mut these: Vec<Finding> = rows
                .iter()
                .filter_map(|row| row.as_object())
                .map(|bindings| {
                    let mut b: Vec<(String, String)> = bindings
                        .iter()
                        .map(|(k, v)| {
                            let v = v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string());
                            (k.clone(), v)
                        })
                        .collect();
                    b.sort();
                    let subject = b
                        .iter()
                        .find(|(k, _)| k == check.subject)
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default();
                    Finding {
                        kind: check.kind,
                        severity: check.severity,
                        subject,
                        bindings: b,
                    }
                })
                .collect();
            these.sort();
            out.extend(these);
        }
        Ok(out)
    }

    fn checks() -> Vec<Check> {
        vec![
            Check {
                kind: "transitive_and_functional",
                subject: "property",
                subject_is: SubjectIs::Property,
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
                subject: "property",
                subject_is: SubjectIs::Property,
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
                subject: "class",
                subject_is: SubjectIs::Class,
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
                subject: "property",
                subject_is: SubjectIs::Property,
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
                subject: "class",
                subject_is: SubjectIs::Class,
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
                subject: "class",
                subject_is: SubjectIs::Class,
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
                subject: "property",
                subject_is: SubjectIs::Property,
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
                subject: "property",
                subject_is: SubjectIs::Property,
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
