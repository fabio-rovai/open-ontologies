//! Incremental reasoning: derive the consequences of an addition without
//! recomputing the closure.
//!
//! Full materialisation recomputes the fixpoint over every triple in the
//! store. Measured on LUBM, that is 0.3 s at 100k triples and 95 s at 13.4M:
//! superlinear, while loading stays linear. Adding a hundred facts to a
//! materialised graph and paying ninety seconds to learn what they imply is
//! what stops a graph being kept live, and it is the reason systems fall back
//! to nightly rebuilds.
//!
//! The fix is the standard one, semi-naive evaluation. A rule can only produce
//! something new if at least one of its premises is new, so each round joins
//! the DELTA against the closure rather than the closure against itself. The
//! closure is read once, the delta is small, and the work is proportional to
//! what changed instead of to what exists.
//!
//! Supported here are the rules where an addition actually propagates:
//! subclass and subproperty chains, domain and range, transitivity, symmetry,
//! inverses, sameAs, and equivalence. Schema-level additions (a new
//! subClassOf axiom, a new restriction) change what the whole store entails
//! and are not incremental in this sense: `applies_to` reports that case
//! rather than pretending, and the caller runs a full pass.

use crate::graph::GraphStore;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const RDF_TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const RDFS_SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const RDFS_SUBPROP: &str = "<http://www.w3.org/2000/01/rdf-schema#subPropertyOf>";
const RDFS_DOMAIN: &str = "<http://www.w3.org/2000/01/rdf-schema#domain>";
const RDFS_RANGE: &str = "<http://www.w3.org/2000/01/rdf-schema#range>";
const OWL_TRANSITIVE: &str = "<http://www.w3.org/2002/07/owl#TransitiveProperty>";
const OWL_SYMMETRIC: &str = "<http://www.w3.org/2002/07/owl#SymmetricProperty>";
const OWL_INVERSE: &str = "<http://www.w3.org/2002/07/owl#inverseOf>";
const OWL_SAMEAS: &str = "<http://www.w3.org/2002/07/owl#sameAs>";
const OWL_EQUIV_CLASS: &str = "<http://www.w3.org/2002/07/owl#equivalentClass>";
const OWL_EQUIV_PROP: &str = "<http://www.w3.org/2002/07/owl#equivalentProperty>";

/// Predicates whose addition changes what the existing store entails, so the
/// delta cannot be reasoned over in isolation.
const SCHEMA_PREDICATES: [&str; 7] = [
    RDFS_SUBCLASS,
    RDFS_SUBPROP,
    RDFS_DOMAIN,
    RDFS_RANGE,
    OWL_INVERSE,
    OWL_EQUIV_CLASS,
    OWL_EQUIV_PROP,
];

type Triple = (String, String, String);

pub struct IncrementalReasoner;

/// Schema read once from the closure: the rules an addition is evaluated
/// against. Transitive closures are precomputed, so propagating a new type
/// through a hierarchy is a lookup rather than a search.
struct Schema {
    superclasses: HashMap<String, HashSet<String>>,
    superproperties: HashMap<String, HashSet<String>>,
    domains: HashMap<String, Vec<String>>,
    ranges: HashMap<String, Vec<String>>,
    transitive: HashSet<String>,
    symmetric: HashSet<String>,
    inverses: HashMap<String, Vec<String>>,
}

fn close(direct: &HashMap<String, HashSet<String>>) -> HashMap<String, HashSet<String>> {
    let mut out: HashMap<String, HashSet<String>> = HashMap::new();
    for key in direct.keys() {
        let mut seen: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = direct.get(key).into_iter().flatten().cloned().collect();
        while let Some(next) = stack.pop() {
            if seen.insert(next.clone())
                && let Some(parents) = direct.get(&next)
            {
                stack.extend(parents.iter().cloned());
            }
        }
        out.insert(key.clone(), seen);
    }
    out
}

impl Schema {
    /// Read the schema with targeted queries rather than a full scan.
    ///
    /// The first version of this module called `all_triples()` and built its
    /// indexes over the entire store, which made "incremental" reasoning
    /// SLOWER than full materialisation on a 1.3M-triple graph: 3.6 s against
    /// 2.7 s, because reading 1.9M triples into memory dwarfs the work the
    /// delta actually implies. Schema axioms are a few thousand triples at
    /// most, so they are fetched directly and everything else is joined on
    /// demand.
    fn read(graph: &Arc<GraphStore>) -> anyhow::Result<Self> {
        let pairs = |pred: &str| -> anyhow::Result<Vec<(String, String)>> {
            let q = format!("SELECT ?s ?o WHERE {{ ?s {pred} ?o }} LIMIT 100000");
            let raw = graph.sparql_select(&q)?;
            let parsed: serde_json::Value = serde_json::from_str(&raw)?;
            Ok(parsed
                .get("results")
                .and_then(|r| r.as_array())
                .map(|rows| {
                    rows.iter()
                        .filter_map(|r| {
                            Some((
                                r.get("s")?.as_str()?.to_string(),
                                r.get("o")?.as_str()?.to_string(),
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default())
        };
        let typed = |cls: &str| -> anyhow::Result<HashSet<String>> {
            let q = format!("SELECT ?s WHERE {{ ?s {RDF_TYPE} {cls} }} LIMIT 100000");
            let raw = graph.sparql_select(&q)?;
            let parsed: serde_json::Value = serde_json::from_str(&raw)?;
            Ok(parsed
                .get("results")
                .and_then(|r| r.as_array())
                .map(|rows| {
                    rows.iter()
                        .filter_map(|r| Some(r.get("s")?.as_str()?.to_string()))
                        .collect()
                })
                .unwrap_or_default())
        };

        let mut sub_class: HashMap<String, HashSet<String>> = HashMap::new();
        for (s, o) in pairs(RDFS_SUBCLASS)? {
            sub_class.entry(s).or_default().insert(o);
        }
        for (s, o) in pairs(OWL_EQUIV_CLASS)? {
            sub_class.entry(s.clone()).or_default().insert(o.clone());
            sub_class.entry(o).or_default().insert(s);
        }

        let mut sub_prop: HashMap<String, HashSet<String>> = HashMap::new();
        for (s, o) in pairs(RDFS_SUBPROP)? {
            sub_prop.entry(s).or_default().insert(o);
        }
        for (s, o) in pairs(OWL_EQUIV_PROP)? {
            sub_prop.entry(s.clone()).or_default().insert(o.clone());
            sub_prop.entry(o).or_default().insert(s);
        }

        let mut domains: HashMap<String, Vec<String>> = HashMap::new();
        for (s, o) in pairs(RDFS_DOMAIN)? {
            domains.entry(s).or_default().push(o);
        }
        let mut ranges: HashMap<String, Vec<String>> = HashMap::new();
        for (s, o) in pairs(RDFS_RANGE)? {
            ranges.entry(s).or_default().push(o);
        }
        let mut inverses: HashMap<String, Vec<String>> = HashMap::new();
        for (s, o) in pairs(OWL_INVERSE)? {
            inverses.entry(s.clone()).or_default().push(o.clone());
            inverses.entry(o).or_default().push(s);
        }

        Ok(Schema {
            superclasses: close(&sub_class),
            superproperties: close(&sub_prop),
            domains,
            ranges,
            transitive: typed(OWL_TRANSITIVE)?,
            symmetric: typed(OWL_SYMMETRIC)?,
            inverses,
        })
    }
}

impl IncrementalReasoner {
    /// Whether an addition can be reasoned over incrementally, and why not.
    pub fn applies_to(delta: &[Triple]) -> Result<(), String> {
        for (_, p, _) in delta {
            if SCHEMA_PREDICATES.contains(&p.as_str()) {
                return Err(format!(
                    "{p} is a schema axiom: it changes what the existing store entails, \
                     so the closure must be recomputed with onto_reason"
                ));
            }
        }
        Ok(())
    }

    /// Derive and materialise the consequences of `delta` against the closure
    /// already in the store, without reading the store into memory.
    pub fn run(graph: &Arc<GraphStore>, delta: &[Triple], materialize: bool) -> anyhow::Result<String> {
        if let Err(reason) = Self::applies_to(delta) {
            return Ok(serde_json::json!({
                "ok": false,
                "incremental": false,
                "reason": reason,
            })
            .to_string());
        }

        let schema = Schema::read(graph)?;

        // Targeted join: the objects an edge leads on to, and the subjects
        // that lead into it. Only asked for transitive properties, and only
        // about terms the delta actually mentions.
        let neighbours = |term: &str, pred: &str, forward: bool| -> Vec<String> {
            let q = if forward {
                format!("SELECT ?x WHERE {{ {term} {pred} ?x }} LIMIT 10000")
            } else {
                format!("SELECT ?x WHERE {{ ?x {pred} {term} }} LIMIT 10000")
            };
            graph
                .sparql_select(&q)
                .ok()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                .and_then(|v| v.get("results").and_then(|r| r.as_array()).cloned())
                .map(|rows| {
                    rows.iter()
                        .filter_map(|r| Some(r.get("x")?.as_str()?.to_string()))
                        .collect()
                })
                .unwrap_or_default()
        };

        let mut derived: HashSet<Triple> = HashSet::new();
        let mut frontier: Vec<Triple> = delta.to_vec();
        let mut rounds = 0usize;

        // Semi-naive: each round derives only from what the previous round
        // produced. Re-deriving something the store already holds is harmless
        // because the store is a set, so no membership index is needed.
        while !frontier.is_empty() && rounds < 20 {
            rounds += 1;
            let mut next: Vec<Triple> = Vec::new();
            {
                let mut emit = |t: Triple, next: &mut Vec<Triple>| {
                    if derived.insert(t.clone()) {
                        next.push(t);
                    }
                };

                for (s, p, o) in &frontier {
                    if p == RDF_TYPE {
                        if let Some(supers) = schema.superclasses.get(o) {
                            for sup in supers {
                                emit((s.clone(), p.clone(), sup.clone()), &mut next);
                            }
                        }
                        continue;
                    }

                    if let Some(supers) = schema.superproperties.get(p) {
                        for sup in supers {
                            emit((s.clone(), sup.clone(), o.clone()), &mut next);
                        }
                    }
                    for d in schema.domains.get(p).into_iter().flatten() {
                        emit((s.clone(), RDF_TYPE.to_string(), d.clone()), &mut next);
                    }
                    if o.starts_with('<') {
                        for r in schema.ranges.get(p).into_iter().flatten() {
                            emit((o.clone(), RDF_TYPE.to_string(), r.clone()), &mut next);
                        }
                        if schema.symmetric.contains(p) {
                            emit((o.clone(), p.clone(), s.clone()), &mut next);
                        }
                        for inv in schema.inverses.get(p).into_iter().flatten() {
                            emit((o.clone(), inv.clone(), s.clone()), &mut next);
                        }
                    }
                    if schema.transitive.contains(p) {
                        for far in neighbours(o, p, true) {
                            emit((s.clone(), p.clone(), far), &mut next);
                        }
                        for near in neighbours(s, p, false) {
                            emit((near, p.clone(), o.clone()), &mut next);
                        }
                    }
                    if p == OWL_SAMEAS && o.starts_with('<') {
                        let q = format!("SELECT ?p ?v WHERE {{ {s} ?p ?v }} LIMIT 10000");
                        if let Ok(raw) = graph.sparql_select(&q)
                            && let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw)
                        {
                            {
                                for row in v.get("results").and_then(|r| r.as_array()).into_iter().flatten() {
                                    if let (Some(pp), Some(vv)) = (
                                        row.get("p").and_then(|x| x.as_str()),
                                        row.get("v").and_then(|x| x.as_str()),
                                    ) {
                                        emit((o.clone(), pp.to_string(), vv.to_string()), &mut next);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            frontier = next;
        }

        if materialize && !derived.is_empty() {
            let mut ntriples = String::new();
            for (s, p, o) in &derived {
                ntriples.push_str(s);
                ntriples.push(' ');
                ntriples.push_str(p);
                ntriples.push(' ');
                ntriples.push_str(o);
                ntriples.push_str(" .\n");
            }
            graph.load_ntriples(&ntriples)?;
        }

        let sample: Vec<String> = derived
            .iter()
            .take(10)
            .map(|(s, p, o)| format!("{s} {p} {o}"))
            .collect();

        Ok(serde_json::json!({
            "ok": true,
            "incremental": true,
            "delta_triples": delta.len(),
            "inferred_count": derived.len(),
            "rounds": rounds,
            "materialized": materialize && !derived.is_empty(),
            "sample_inferences": sample,
        })
        .to_string())
    }
}

/// Parse N-Triples into the (subject, predicate, object) shape used above,
/// keeping the angle brackets and quotes so terms round-trip unchanged.
pub fn parse_ntriples(text: &str) -> Vec<Triple> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_suffix('.').unwrap_or(line).trim();
        // Object may contain spaces inside a literal, so split only twice.
        let mut parts = line.splitn(3, char::is_whitespace);
        if let (Some(s), Some(p), Some(o)) = (parts.next(), parts.next(), parts.next()) {
            out.push((s.trim().to_string(), p.trim().to_string(), o.trim().to_string()));
        }
    }
    out
}
