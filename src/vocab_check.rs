// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

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
    /// Declared terms used in a role their declaration does not give them: a class used
    /// as a predicate, or a property used as the object of `rdf:type` (#432).
    pub wrong_role_terms: Vec<WrongRole>,
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

/// A declared term used in a role its declaration does not give it.
#[derive(Serialize)]
pub struct WrongRole {
    pub term: String,
    /// `property` (used as a predicate) or `class` (used as the object of `rdf:type`).
    pub used_as: &'static str,
    pub declared_as: &'static str,
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
///
/// Reads every graph. The ontology's declared vocabulary is a fact about the
/// store, not about one graph in it, and reading the default graph alone made
/// an ontology loaded from TriG or N-Quads declare nothing: the check bailed
/// with `ontology_terms: 0` and told the caller to load an ontology they had
/// already loaded. See the graph-scope rule on issue #108.
fn select_iris(graph: &Arc<GraphStore>, query: &str, var: &str) -> anyhow::Result<Vec<String>> {
    let json_str = graph.sparql_select_union(query)?;
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

// The same declarations as above, split by the role they give a term. A term with
// declarations of both kinds (OWL 2 punning) fits either role.
const CLASS_FILTER: &str = r#"FILTER(
    ?k = <http://www.w3.org/2002/07/owl#Class>
 || ?k = <http://www.w3.org/2000/01/rdf-schema#Class>)"#;

const CLASS_AXIOM_FILTER: &str =
    r#"FILTER(?p = <http://www.w3.org/2000/01/rdf-schema#subClassOf>)"#;

const PROPERTY_FILTER: &str = r#"FILTER(
    ?k = <http://www.w3.org/2002/07/owl#ObjectProperty>
 || ?k = <http://www.w3.org/2002/07/owl#DatatypeProperty>
 || ?k = <http://www.w3.org/2002/07/owl#AnnotationProperty>
 || ?k = <http://www.w3.org/2002/07/owl#FunctionalProperty>
 || ?k = <http://www.w3.org/2002/07/owl#InverseFunctionalProperty>
 || ?k = <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property>)"#;

const PROPERTY_AXIOM_FILTER: &str = r#"FILTER(
    ?p = <http://www.w3.org/2000/01/rdf-schema#domain>
 || ?p = <http://www.w3.org/2000/01/rdf-schema#range>
 || ?p = <http://www.w3.org/2000/01/rdf-schema#subPropertyOf>)"#;

/// IRIs typed with a kind `kind_filter` accepts, or the subject of an axiom `axiom_filter` accepts.
fn declared_by(
    graph: &Arc<GraphStore>,
    kind_filter: &str,
    axiom_filter: &str,
) -> anyhow::Result<BTreeSet<String>> {
    let mut out: BTreeSet<String> = select_iris(
        graph,
        &format!("SELECT DISTINCT ?t WHERE {{ ?t a ?k . {kind_filter} }}"),
        "t",
    )?
    .into_iter()
    .collect();
    out.extend(select_iris(
        graph,
        &format!("SELECT DISTINCT ?t WHERE {{ ?t ?p ?o . {axiom_filter} }}"),
        "t",
    )?);
    Ok(out)
}

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
            wrong_role_terms: Vec::new(),
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

    // 6. Flag a declared term used in a role no declaration gives it.
    let classes = declared_by(onto, CLASS_FILTER, CLASS_AXIOM_FILTER)?;
    let properties = declared_by(onto, PROPERTY_FILTER, PROPERTY_AXIOM_FILTER)?;
    let mut wrong_role: Vec<WrongRole> = Vec::new();
    for (used, used_as, fits, declared_as) in [
        (&preds, "property", &properties, "class"),
        (&types, "class", &classes, "property"),
    ] {
        for iri in used {
            if onto_terms.contains(iri) && !fits.contains(iri) {
                wrong_role.push(WrongRole {
                    term: iri.clone(),
                    used_as,
                    declared_as,
                });
            }
        }
    }
    wrong_role.sort_by(|a, b| (&a.term, a.used_as).cmp(&(&b.term, b.used_as)));

    let report = VocabReport {
        conforms: bad.is_empty() && wrong_role.is_empty(),
        hallucinated_terms: bad.into_iter().collect(),
        wrong_role_terms: wrong_role,
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

    /// #432: a declared class used as a predicate, or a declared property used as an
    /// `rdf:type` object, is declared but not in that role.
    #[test]
    fn declared_term_in_the_wrong_role_is_caught() {
        let data = r#"
            @prefix ex: <http://example.org/onto#> .
            @prefix d: <http://data.example/> .
            d:s1 a ex:worksFor .
            d:s2 ex:Person d:x .
        "#;
        let out = check_data_vocab(&onto(), data, &[]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["conforms"], false,
            "wrong-role uses must not conform: {out}"
        );
        assert_eq!(
            v["wrong_role_terms"],
            serde_json::json!([
                {"term": "http://example.org/onto#Person", "used_as": "property", "declared_as": "class"},
                {"term": "http://example.org/onto#worksFor", "used_as": "class", "declared_as": "property"}
            ]),
            "{out}"
        );
        assert_eq!(v["hallucinated_terms"], serde_json::json!([]), "{out}");
    }

    /// OWL 2 punning: a term declared as both a class and a property may be used as either.
    #[test]
    fn punned_term_may_be_used_in_either_role() {
        let onto_ttl = r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix ex: <http://example.org/onto#> .
            ex:Tag a owl:Class , owl:AnnotationProperty .
        "#;
        let g = Arc::new(GraphStore::new());
        g.load_turtle(onto_ttl, None).unwrap();
        let data = r#"
            @prefix ex: <http://example.org/onto#> .
            @prefix d: <http://data.example/> .
            d:a a ex:Tag ; ex:Tag "x" .
        "#;
        let out = check_data_vocab(&g, data, &[]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["conforms"], true, "a punned term fits both roles: {out}");
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
