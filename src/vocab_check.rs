//! Closed-world vocabulary check for generated DATA graphs (#vocab).
//!
//! The gap this fills: SHACL is **open-world**. It silently ignores any predicate
//! or class it has no shape for, so a data graph full of *invented* terms
//! (`ies:hasDeparturePort` where the ontology only defines
//! `ies:scheduledDeparturePort`) still reports `conforms = true`. For checking
//! LLM-generated RDF that is nearly useless — the model can hallucinate a
//! plausible-but-undeclared term and every open-world validator waves it through.
//!
//! `check_data_vocab` runs the complementary CLOSED-WORLD gate: every predicate
//! and every `rdf:type` object in the data whose namespace belongs to the ontology
//! must be *declared* in the ontology. Standard `rdf`/`rdfs`/`owl`/`xsd`/`sh`
//! vocabulary and the caller's own instance-data IRIs are never policed.
//!
//! This mirrors `shacl::check_shapes` (which checks that proposed SHACL references
//! real terms) but points the same idea at generated data instead of shapes.

use crate::graph::GraphStore;
use serde::Serialize;
use std::collections::BTreeSet;
use std::sync::Arc;

/// Namespaces that are always allowed and never flagged.
const STD_NS: &[&str] = &[
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "http://www.w3.org/2000/01/rdf-schema#",
    "http://www.w3.org/2002/07/owl#",
    "http://www.w3.org/2001/XMLSchema#",
    "http://www.w3.org/ns/shacl#",
    "http://www.w3.org/2004/02/skos/core#",
];

#[derive(Serialize)]
pub struct VocabReport {
    /// True iff an ontology was present AND no hallucinated terms were found.
    pub conforms: bool,
    /// Terms used in the data, in a policed namespace, but not declared in the ontology.
    pub hallucinated_terms: Vec<String>,
    /// The namespaces that were policed (ontology's own namespaces + any caller-supplied).
    pub checked_namespaces: Vec<String>,
    pub predicates_checked: usize,
    pub types_checked: usize,
    pub ontology_terms: usize,
    /// Set when the check could not be performed meaningfully (e.g. no ontology loaded).
    /// A closed-world check with no ontology must NEVER silently pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// Namespace of an IRI: everything up to and including the last `#` or `/`.
fn ns_of(iri: &str) -> String {
    match iri.rfind(['#', '/']) {
        Some(i) => iri[..=i].to_string(),
        None => iri.to_string(),
    }
}

/// Strip surrounding `<>` (oxigraph Display wraps NamedNodes in angle brackets).
fn strip_brackets(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(s)
}

/// Run a SELECT and collect the IRI values of one variable (literals/blanks skipped).
fn select_iris(graph: &Arc<GraphStore>, query: &str, var: &str) -> anyhow::Result<Vec<String>> {
    let json_str = graph.sparql_select(query)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
    let mut out = Vec::new();
    if let Some(rows) = parsed["results"].as_array() {
        for row in rows {
            if let Some(v) = row.get(var).and_then(|v| v.as_str()) {
                let iri = strip_brackets(v);
                if iri.starts_with("http://") || iri.starts_with("https://") {
                    out.push(iri.to_string());
                }
            }
        }
    }
    Ok(out)
}

const CLASS_OR_PROP_FILTER: &str = r#"FILTER(
    ?k = <http://www.w3.org/2002/07/owl#Class>
 || ?k = <http://www.w3.org/2000/01/rdf-schema#Class>
 || ?k = <http://www.w3.org/2002/07/owl#ObjectProperty>
 || ?k = <http://www.w3.org/2002/07/owl#DatatypeProperty>
 || ?k = <http://www.w3.org/2002/07/owl#AnnotationProperty>
 || ?k = <http://www.w3.org/2002/07/owl#FunctionalProperty>
 || ?k = <http://www.w3.org/2002/07/owl#InverseFunctionalProperty>
 || ?k = <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property>)"#;

const DEFINED_BY_AXIOM_FILTER: &str = r#"FILTER(
    ?p = <http://www.w3.org/2000/01/rdf-schema#domain>
 || ?p = <http://www.w3.org/2000/01/rdf-schema#range>
 || ?p = <http://www.w3.org/2000/01/rdf-schema#subClassOf>
 || ?p = <http://www.w3.org/2000/01/rdf-schema#subPropertyOf>)"#;

/// Closed-world vocabulary check of `data_ttl` against the loaded ontology `onto`.
/// `extra_ns` adds namespaces to police beyond the ontology's own.
/// Returns a JSON `VocabReport`.
pub fn check_data_vocab(
    onto: &Arc<GraphStore>,
    data_ttl: &str,
    extra_ns: &[String],
) -> anyhow::Result<String> {
    // 1. Parse the data into an isolated store (parse errors surface here).
    let data = Arc::new(GraphStore::new());
    data.load_turtle(data_ttl, None)
        .map_err(|e| anyhow::anyhow!("data failed to parse as Turtle: {e}"))?;

    // 2. The ontology's declared vocabulary (full IRIs): typed classes/properties
    //    plus anything given a domain/range/subClassOf/subPropertyOf axiom.
    let mut onto_terms: BTreeSet<String> = BTreeSet::new();
    for t in select_iris(
        onto,
        &format!("SELECT DISTINCT ?t WHERE {{ ?t a ?k . {CLASS_OR_PROP_FILTER} }}"),
        "t",
    )? {
        onto_terms.insert(t);
    }
    for t in select_iris(
        onto,
        &format!("SELECT DISTINCT ?t WHERE {{ ?t ?p ?o . {DEFINED_BY_AXIOM_FILTER} }}"),
        "t",
    )? {
        onto_terms.insert(t);
    }

    // Guard: a closed-world check with no ontology vocabulary must never silently
    // pass — that is exactly the vacuous-conformance footgun this tool exists to kill.
    if onto_terms.is_empty() && extra_ns.is_empty() {
        let report = VocabReport {
            conforms: false,
            hallucinated_terms: Vec::new(),
            checked_namespaces: Vec::new(),
            predicates_checked: 0,
            types_checked: 0,
            ontology_terms: 0,
            warning: Some(
                "no ontology vocabulary found (0 declared terms) — load an ontology \
                 first, or pass `namespaces` to police; nothing was checked"
                    .to_string(),
            ),
        };
        return Ok(serde_json::to_string(&report)?);
    }

    // 3. Namespaces to police: the ontology's own, plus caller-supplied, minus standard.
    let mut checked: BTreeSet<String> = onto_terms.iter().map(|t| ns_of(t)).collect();
    checked.extend(extra_ns.iter().cloned());
    for std in STD_NS {
        checked.remove(*std);
    }

    // 4. Terms actually used in the data: every predicate + every asserted class.
    let preds = select_iris(&data, "SELECT DISTINCT ?p WHERE { ?s ?p ?o }", "p")?;
    let types = select_iris(&data, "SELECT DISTINCT ?c WHERE { ?s a ?c }", "c")?;

    // 5. Flag any policed-namespace term not declared in the ontology.
    let mut bad: BTreeSet<String> = BTreeSet::new();
    for iri in preds.iter().chain(types.iter()) {
        if checked.contains(&ns_of(iri)) && !onto_terms.contains(iri) {
            bad.insert(iri.clone());
        }
    }

    let report = VocabReport {
        conforms: bad.is_empty(),
        hallucinated_terms: bad.into_iter().collect(),
        checked_namespaces: checked.into_iter().collect(),
        predicates_checked: preds.len(),
        types_checked: types.len(),
        ontology_terms: onto_terms.len(),
        warning: None,
    };
    Ok(serde_json::to_string(&report)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONTO: &str = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/onto#> .
        ex:Person a owl:Class .
        ex:Airport a owl:Class .
        ex:worksFor a owl:ObjectProperty ; rdfs:domain ex:Person .
        ex:scheduledDeparturePort a owl:ObjectProperty .
    "#;

    fn onto() -> Arc<GraphStore> {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(ONTO, None).unwrap();
        g
    }

    #[test]
    fn clean_data_conforms() {
        let data = r#"
            @prefix ex: <http://example.org/onto#> .
            @prefix d: <http://data.example/> .
            d:Alice a ex:Person ; ex:worksFor d:Acme .
        "#;
        let out = check_data_vocab(&onto(), data, &[]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["conforms"], true, "clean data should conform: {out}");
    }

    #[test]
    fn hallucinated_predicate_is_caught() {
        // ex:hasDeparturePort is NOT declared; open-world SHACL would miss it.
        let data = r#"
            @prefix ex: <http://example.org/onto#> .
            @prefix d: <http://data.example/> .
            d:Flight a ex:Airport ; ex:hasDeparturePort d:LHR .
        "#;
        let out = check_data_vocab(&onto(), data, &[]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["conforms"], false, "should flag undeclared term: {out}");
        let bad = v["hallucinated_terms"].as_array().unwrap();
        assert!(
            bad.iter().any(|t| t.as_str().unwrap().ends_with("hasDeparturePort")),
            "hasDeparturePort should be flagged: {out}"
        );
    }

    #[test]
    fn instance_and_standard_ns_not_flagged() {
        // d: instances and rdf:type must never be flagged.
        let data = r#"
            @prefix ex: <http://example.org/onto#> .
            @prefix d: <http://data.example/> .
            d:Bob a ex:Person .
        "#;
        let out = check_data_vocab(&onto(), data, &[]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["conforms"], true, "instance IRIs must not be policed: {out}");
    }
}
