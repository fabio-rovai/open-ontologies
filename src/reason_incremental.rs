// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

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
use oxigraph::io::{RdfFormat, RdfParser};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
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
    derivations: Option<crate::reason::DerivationGraph>,
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
    fn read(graph: &Arc<GraphStore>, track_derivations: bool) -> anyhow::Result<Self> {
        // Any scan that returns exactly SCAN_LIMIT rows may have been cut. Record
        // it so the caller can report an incomplete closure instead of a wrong one
        // presented as complete.
        let truncated = std::cell::Cell::new(false);
        let schema_triples = std::cell::RefCell::new(Vec::new());
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
            if track_derivations {
                schema_triples.borrow_mut().extend(
                    rows.iter()
                        .map(|(s, o)| (s.clone(), pred.to_string(), o.clone())),
                );
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
            if track_derivations {
                schema_triples.borrow_mut().extend(
                    rows.iter()
                        .map(|s| (s.clone(), RDF_TYPE.to_string(), cls.to_string())),
                );
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

        let transitive = typed(OWL_TRANSITIVE)?;
        let symmetric = typed(OWL_SYMMETRIC)?;
        // Schema closure is small and records the actual chains behind a
        // precomputed superclass lookup. A synthetic A subClassOf C premise
        // with no recorded A -> B -> C derivation would defeat retraction.
        let derivations = if track_derivations {
            let schema_store = Arc::new(GraphStore::new());
            let text = schema_triples
                .borrow()
                .iter()
                .map(|(s, p, o)| format!("{s} {p} {o} .\n"))
                .collect::<String>();
            schema_store.load_ntriples(&text)?;
            Some(crate::reason::Reasoner::derivation_graph(
                &schema_store,
                "owl-rl",
            )?)
        } else {
            None
        };
        Ok(Schema {
            derivations,
            superclasses: close(&sub_class),
            superproperties: close(&sub_prop),
            domains,
            ranges,
            transitive,
            symmetric,
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
    /// already in the store. The inference loop uses targeted queries;
    /// durable materialisation additionally reconciles the assertion snapshot.
    pub fn run(
        graph: &Arc<GraphStore>,
        delta: &[Triple],
        materialize: bool,
    ) -> anyhow::Result<String> {
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
        Self::run_scoped_with_tracking(graph, delta, materialize, all_versions, false)
    }

    /// Opt in to recording the same semi-naive rule instances durably.
    pub fn run_scoped_with_tracking(
        graph: &Arc<GraphStore>,
        delta: &[Triple],
        materialize: bool,
        all_versions: bool,
        track_derivations: bool,
    ) -> anyhow::Result<String> {
        anyhow::ensure!(
            !track_derivations || materialize,
            "track_derivations requires materialize=true"
        );

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

        let source_token = if track_derivations {
            Some(graph.reasoning_token()?)
        } else {
            None
        };
        let sources = if track_derivations {
            graph.reasoning_sources()?
        } else {
            Vec::new()
        };
        let asserted = if track_derivations {
            Some(
                graph
                    .triples_in_scope(&crate::graph::ReadScope::AllGraphs)?
                    .0,
            )
        } else {
            None
        };
        let schema = Schema::read(graph, track_derivations)?;

        // Targeted join: the objects an edge leads on to, and the subjects
        // that lead into it. Only asked for transitive properties, and only
        // about terms the delta actually mentions.
        let neighbours_truncated = std::cell::Cell::new(false);
        let neighbours = |term: &str, pred: &str, forward: bool| -> anyhow::Result<Vec<String>> {
            let q = if forward {
                format!("SELECT ?x WHERE {{ {term} {pred} ?x }} LIMIT 10000")
            } else {
                format!("SELECT ?x WHERE {{ ?x {pred} {term} }} LIMIT 10000")
            };
            // The neighbours of a delta term are instance edges that may live in any
            // named graph, so this store-question reads the union too.
            let raw = graph.sparql_select_union(&q)?;
            let value: serde_json::Value = serde_json::from_str(&raw)?;
            let rows = value["results"].as_array().ok_or_else(|| {
                anyhow::anyhow!("incremental neighbour query returned no solution rows")
            })?;
            if rows.len() >= 10000 {
                neighbours_truncated.set(true);
            }
            rows.iter()
                .map(|row| {
                    row["x"].as_str().map(str::to_owned).ok_or_else(|| {
                        anyhow::anyhow!("incremental neighbour query returned an unbound term")
                    })
                })
                .collect()
        };

        let mut derived: HashSet<Triple> = HashSet::new();
        let mut frontier: Vec<Triple> = delta.to_vec();
        let mut rounds = 0usize;
        let mut instances = schema
            .derivations
            .as_ref()
            .map(|d| d.instances.clone())
            .unwrap_or_default();

        // Semi-naive: each round derives only from what the previous round
        // produced. Re-deriving something the store already holds is harmless
        // because the store is a set, so no membership index is needed.
        while !frontier.is_empty() && rounds < 20 {
            rounds += 1;
            let mut next: Vec<Triple> = Vec::new();
            {
                let mut emit = |t: Triple,
                                rule: &'static str,
                                premises: Vec<Triple>,
                                next: &mut Vec<Triple>| {
                    if track_derivations {
                        instances.push(crate::reason::RuleInstance {
                            rule,
                            conclusion: t.clone(),
                            premises,
                        });
                    }
                    if derived.insert(t.clone()) {
                        next.push(t);
                    }
                };

                for (s, p, o) in &frontier {
                    if p == RDF_TYPE {
                        if let Some(supers) = schema.superclasses.get(o) {
                            for sup in supers {
                                emit(
                                    (s.clone(), p.clone(), sup.clone()),
                                    "rdfs9",
                                    if track_derivations {
                                        vec![
                                            (s.clone(), p.clone(), o.clone()),
                                            (o.clone(), RDFS_SUBCLASS.into(), sup.clone()),
                                        ]
                                    } else {
                                        Vec::new()
                                    },
                                    &mut next,
                                );
                            }
                        }
                        continue;
                    }

                    if let Some(supers) = schema.superproperties.get(p) {
                        for sup in supers {
                            emit(
                                (s.clone(), sup.clone(), o.clone()),
                                "rdfs7",
                                if track_derivations {
                                    vec![
                                        (s.clone(), p.clone(), o.clone()),
                                        (p.clone(), RDFS_SUBPROP.into(), sup.clone()),
                                    ]
                                } else {
                                    Vec::new()
                                },
                                &mut next,
                            );
                        }
                    }
                    for d in schema.domains.get(p).into_iter().flatten() {
                        emit(
                            (s.clone(), RDF_TYPE.to_string(), d.clone()),
                            "rdfs2",
                            if track_derivations {
                                vec![
                                    (s.clone(), p.clone(), o.clone()),
                                    (p.clone(), RDFS_DOMAIN.into(), d.clone()),
                                ]
                            } else {
                                Vec::new()
                            },
                            &mut next,
                        );
                    }
                    if o.starts_with('<') {
                        for r in schema.ranges.get(p).into_iter().flatten() {
                            emit(
                                (o.clone(), RDF_TYPE.to_string(), r.clone()),
                                "rdfs3",
                                if track_derivations {
                                    vec![
                                        (s.clone(), p.clone(), o.clone()),
                                        (p.clone(), RDFS_RANGE.into(), r.clone()),
                                    ]
                                } else {
                                    Vec::new()
                                },
                                &mut next,
                            );
                        }
                        if schema.symmetric.contains(p) {
                            emit(
                                (o.clone(), p.clone(), s.clone()),
                                "prp-symp",
                                if track_derivations {
                                    vec![
                                        (p.clone(), RDF_TYPE.into(), OWL_SYMMETRIC.into()),
                                        (s.clone(), p.clone(), o.clone()),
                                    ]
                                } else {
                                    Vec::new()
                                },
                                &mut next,
                            );
                        }
                        for inv in schema.inverses.get(p).into_iter().flatten() {
                            let forward = schema.derivations.as_ref().is_some_and(|d| {
                                d.closure
                                    .contains(&(p.clone(), OWL_INVERSE.into(), inv.clone()))
                            });
                            let (rule, axiom) = if forward {
                                ("prp-inv1", (p.clone(), OWL_INVERSE.into(), inv.clone()))
                            } else {
                                ("prp-inv2", (inv.clone(), OWL_INVERSE.into(), p.clone()))
                            };
                            emit(
                                (o.clone(), inv.clone(), s.clone()),
                                rule,
                                if track_derivations {
                                    vec![axiom, (s.clone(), p.clone(), o.clone())]
                                } else {
                                    Vec::new()
                                },
                                &mut next,
                            );
                        }
                    }
                    if schema.transitive.contains(p) {
                        for far in neighbours(o, p, true)? {
                            emit(
                                (s.clone(), p.clone(), far.clone()),
                                "prp-trp",
                                if track_derivations {
                                    vec![
                                        (p.clone(), RDF_TYPE.into(), OWL_TRANSITIVE.into()),
                                        (s.clone(), p.clone(), o.clone()),
                                        (o.clone(), p.clone(), far),
                                    ]
                                } else {
                                    Vec::new()
                                },
                                &mut next,
                            );
                        }
                        for near in neighbours(s, p, false)? {
                            emit(
                                (near.clone(), p.clone(), o.clone()),
                                "prp-trp",
                                if track_derivations {
                                    vec![
                                        (p.clone(), RDF_TYPE.into(), OWL_TRANSITIVE.into()),
                                        (near, p.clone(), s.clone()),
                                        (s.clone(), p.clone(), o.clone()),
                                    ]
                                } else {
                                    Vec::new()
                                },
                                &mut next,
                            );
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

        let durable = if let Some(asserted) = asserted {
            let mut closure: HashSet<_> = asserted.iter().cloned().collect();
            anyhow::ensure!(
                delta.iter().all(|t| closure.contains(t)),
                "incremental materialisation requires delta facts to be loaded as assertions first"
            );
            closure.extend(derived.iter().cloned());
            closure.extend(
                schema
                    .derivations
                    .as_ref()
                    .expect("tracked schema")
                    .closure
                    .iter()
                    .cloned(),
            );
            let dag = crate::reason::DerivationGraph {
                profile_used: "owl-rl-incremental".into(),
                asserted,
                closure,
                instances,
                // This DAG is the addition's support, not a replacement for a
                // complete profile run. It must not retire other derivations.
                fixpoint_reached: false,
                iterations: rounds,
                clashes: Vec::new(),
                refused_unserialisable: 0,
            };
            Some(graph.commit_reasoning(
                &dag,
                crate::reason::InferenceTarget::DefaultGraph,
                source_token.expect("tracked reasoning captured a source token"),
                &sources,
            )?)
        } else {
            if materialize {
                graph.commit_materialization(
                    &derived.iter().cloned().collect::<Vec<_>>(),
                    crate::reason::InferenceTarget::DefaultGraph,
                )?;
            }
            None
        };

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
            "complete": !schema.truncated && !neighbours_truncated.get() && frontier.is_empty(),
        });
        if let Some(durable) = durable {
            out["truth_maintenance"] = durable;
        }
        if neighbours_truncated.get() || !frontier.is_empty() {
            out["incomplete_reason"] = serde_json::json!(
                "a neighbour scan or iteration limit was reached; run full reasoning for a complete closure"
            );
        }
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
pub fn parse_ntriples(text: &str) -> anyhow::Result<Vec<Triple>> {
    RdfParser::from_format(RdfFormat::NTriples)
        .for_reader(Cursor::new(text.as_bytes()))
        .map(|quad| {
            let quad = quad?;
            Ok((
                quad.subject.to_string(),
                quad.predicate.to_string(),
                quad.object.to_string(),
            ))
        })
        .collect()
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
        for c in [
            OWL_TRANSITIVE,
            OWL_SYMMETRIC,
            OWL_FUNCTIONAL,
            OWL_INVERSE_FUNCTIONAL,
        ] {
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

    #[test]
    fn parse_ntriples_accepts_inline_comments() {
        let parsed = parse_ntriples(
            "<urn:alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:Person> . # valid N-Triples comment\n",
        )
        .unwrap();

        assert_eq!(
            parsed,
            vec![t(
                "<urn:alice>",
                RDF_TYPE,
                "<urn:Person>"
            )]
        );
    }

    #[test]
    fn parse_ntriples_rejects_malformed_lines_instead_of_silently_dropping_them() {
        let err = parse_ntriples("<urn:alice> <urn:knows> <urn:bob> .\nnot a triple").unwrap_err();
        assert!(!err.to_string().is_empty());
    }
}
