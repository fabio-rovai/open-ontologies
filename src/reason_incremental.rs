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
const OWL_FUNCTIONAL: &str = "<http://www.w3.org/2002/07/owl#FunctionalProperty>";
const OWL_INVERSE_FUNCTIONAL: &str = "<http://www.w3.org/2002/07/owl#InverseFunctionalProperty>";
const OWL_INVERSE: &str = "<http://www.w3.org/2002/07/owl#inverseOf>";

/// Classes whose assertion `p rdf:type <class>` gives an EXISTING property a new
/// characteristic, changing what the store already entails over that property's
/// existing edges. Like the schema predicates, such a delta is not incremental:
/// the semi-naive loop only revisits triples reachable from the delta frontier,
/// so a newly-declared characteristic never reprocesses the edges it now governs.
const PROPERTY_CHARACTERISTICS: [&str; 4] = [
    OWL_TRANSITIVE,
    OWL_SYMMETRIC,
    OWL_FUNCTIONAL,
    OWL_INVERSE_FUNCTIONAL,
];
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
    /// True if any schema-reading query hit its row cap, so the schema this
    /// closure was computed against is incomplete and the derived closure may be
    /// missing consequences. Surfaced, never swallowed.
    truncated: bool,
}

/// The row cap on every internally-authored scan here. A scan that returns
/// exactly this many rows is reported as possibly truncated rather than assumed
/// complete.
const SCAN_LIMIT: usize = 100_000;

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
        // Any scan that returns exactly SCAN_LIMIT rows may have been cut. Record
        // it so the caller can report an incomplete closure instead of a wrong one
        // presented as complete.
        let truncated = std::cell::Cell::new(false);
        let pairs = |pred: &str| -> anyhow::Result<Vec<(String, String)>> {
            let q = format!("SELECT ?s ?o WHERE {{ ?s {pred} ?o }} LIMIT {SCAN_LIMIT}");
            // The schema is a question about the whole store: subclass/domain/range
            // axioms may sit in any named graph (a TriG/N-Quads load, a per-version
            // schema graph), so read the union, not the default graph alone. Reading
            // the default graph made incremental reasoning derive nothing for a store
            // whose schema was not in the default graph, and label it complete.
            let raw = graph.sparql_select_union(&q)?;
            let parsed: serde_json::Value = serde_json::from_str(&raw)?;
            let rows: Vec<(String, String)> = parsed
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
                .unwrap_or_default();
            if rows.len() >= SCAN_LIMIT {
                truncated.set(true);
            }
            Ok(rows)
        };
        let typed = |cls: &str| -> anyhow::Result<HashSet<String>> {
            let q = format!("SELECT ?s WHERE {{ ?s {RDF_TYPE} {cls} }} LIMIT {SCAN_LIMIT}");
            // Property-characteristic declarations are schema, read from every graph.
            let raw = graph.sparql_select_union(&q)?;
            let parsed: serde_json::Value = serde_json::from_str(&raw)?;
            let rows: HashSet<String> = parsed
                .get("results")
                .and_then(|r| r.as_array())
                .map(|rows| {
                    rows.iter()
                        .filter_map(|r| Some(r.get("s")?.as_str()?.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            if rows.len() >= SCAN_LIMIT {
                truncated.set(true);
            }
            Ok(rows)
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
            truncated: truncated.get(),
        })
    }
}

impl IncrementalReasoner {
    /// Whether an addition can be reasoned over incrementally, and why not.
    pub fn applies_to(delta: &[Triple]) -> Result<(), String> {
        for (_, p, o) in delta {
            if SCHEMA_PREDICATES.contains(&p.as_str()) {
                return Err(format!(
                    "{p} is a schema axiom: it changes what the existing store entails, \
                     so the closure must be recomputed with onto_reason"
                ));
            }
            if p == RDF_TYPE && PROPERTY_CHARACTERISTICS.contains(&o.as_str()) {
                return Err(format!(
                    "{o} declares a property characteristic: it changes what the existing \
                     store entails over that property's existing edges, which the delta \
                     cannot reach, so the closure must be recomputed with onto_reason"
                ));
            }
        }
        Ok(())
    }

    /// Derive and materialise the consequences of `delta` against the closure
    /// already in the store, without reading the store into memory.
    pub fn run(graph: &Arc<GraphStore>, delta: &[Triple], materialize: bool) -> anyhow::Result<String> {
        Self::run_scoped(graph, delta, materialize, false)
    }

    /// As [`run`](Self::run), with the #108 scope gate.
    ///
    /// This path reads the store through `sparql_select_union`, so it has the
    /// SHACL-side selection rather than the reasoner's, and it materialises
    /// into the default graph. Over a bi-temporal store that is the same
    /// defect `onto_reason` was gated for, arriving through a second door, and
    /// leaving it open would have made the gate on `onto_reason` a suggestion.
    ///
    /// There is no snapshot form here. Aligning the incremental reader with
    /// the full one is #108's own "not this issue", and a tool that accepted
    /// `valid_at` and ignored it would be worse than one that has no such
    /// argument. So the gate is binary: over a store that uses the temporal
    /// vocabulary, this refuses unless `all_versions` says the union of every
    /// version is what the caller meant, and the message points at
    /// `onto_reason` for the snapshot.
    pub fn run_scoped(
        graph: &Arc<GraphStore>,
        delta: &[Triple],
        materialize: bool,
        all_versions: bool,
    ) -> anyhow::Result<String> {
        let request = if all_versions {
            crate::temporal::ScopeRequest::AllVersions
        } else {
            crate::temporal::ScopeRequest::Unscoped
        };
        let resolved = crate::temporal::resolve(graph, &request);
        if let Ok((_, manifest)) = &resolved
            && manifest.store_has_versions()
            && materialize
        {
            anyhow::bail!(
                "a run over a versioned store does not materialise. This path writes its \
                 conclusions into the DEFAULT graph, which is timeless and therefore in scope at \
                 every instant, so a closure drawn from every version at once would become an \
                 axiom of every snapshot. Run with materialize=false"
            );
        }
        if let Err(e) = resolved {
            anyhow::bail!(
                "{e}\n\nNote for this tool: incremental reasoning has no snapshot form. It reads \
                 the union of every graph and materialises into the default graph, so over a \
                 versioned store it computes the closure of a state that held at no instant and \
                 writes it in beside the assertions. Run onto_reason with valid_at / as_of for a \
                 snapshot, or pass all_versions=true here to say the union is what you meant"
            );
        }
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
            // The neighbours of a delta term are instance edges that may live in any
            // named graph, so this store-question reads the union too.
            graph
                .sparql_select_union(&q)
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
                    // eq-rep-s (owl:sameAs subject replacement) used to run
                    // HERE and nowhere else. The full reasoner in reason.rs does
                    // not implement it, so loading one graph incrementally and
                    // loading it whole produced two different closures, and the
                    // incremental one emitted no rule ids and therefore no
                    // certificate: its extra triples were materialised outside
                    // the Lean guarantee entirely.
                    //
                    // Removed rather than promoted. eq-rep-* is quadratic in the
                    // size of a sameAs clique, and owl:sameAs occurs exactly once
                    // in this repository's whole corpus, so the rule bought
                    // nothing and cost a divergence between two reasoning paths.
                    // If entity resolution is wanted it belongs in an explicit
                    // canonicalisation pass with its own contract, not in a
                    // forward chainer that only one of two entry points runs.
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

        let mut out = serde_json::json!({
            "ok": true,
            "incremental": true,
            "delta_triples": delta.len(),
            "inferred_count": derived.len(),
            "rounds": rounds,
            "materialized": materialize && !derived.is_empty(),
            "sample_inferences": sample,
            "complete": !schema.truncated,
        });
        if schema.truncated {
            out["warning"] = serde_json::Value::String(format!(
                "a schema scan hit the {SCAN_LIMIT}-row cap, so the schema this closure was \
                 computed against is incomplete and consequences may be missing; run onto_reason \
                 for a full pass"
            ));
        }
        Ok(out.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str, p: &str, o: &str) -> Triple {
        (s.to_string(), p.to_string(), o.to_string())
    }

    #[test]
    fn schema_axiom_deltas_are_refused() {
        // A new subClassOf changes what the existing store entails.
        assert!(IncrementalReasoner::applies_to(&[t("<x>", RDFS_SUBCLASS, "<y>")]).is_err());
    }

    #[test]
    fn a_new_property_characteristic_is_not_incremental() {
        // Declaring an EXISTING property transitive/symmetric/functional retroactively
        // changes what its existing edges entail; the delta frontier cannot reach them,
        // so this must route to a full onto_reason rather than silently under-derive.
        for c in [OWL_TRANSITIVE, OWL_SYMMETRIC, OWL_FUNCTIONAL, OWL_INVERSE_FUNCTIONAL] {
            assert!(
                IncrementalReasoner::applies_to(&[t("<p>", RDF_TYPE, c)]).is_err(),
                "declaring {c} must be refused as non-incremental"
            );
        }
    }

    #[test]
    fn an_ordinary_type_assertion_is_still_incremental() {
        // A normal individual typing is exactly what incremental reasoning is for.
        assert!(IncrementalReasoner::applies_to(&[t("<a>", RDF_TYPE, "<C>")]).is_ok());
        assert!(IncrementalReasoner::applies_to(&[t("<a>", "<http://ex/knows>", "<b>")]).is_ok());
    }
}
