//! Explicit exceptions to a disjointness, compiled away before any tool sees
//! them (#347).
//!
//! A machine-extracted or merged taxonomy often states `Vehicle owl:disjointWith
//! Building` as a default and then files `Houseboat` under both. Classically
//! `Houseboat` is unsatisfiable, and every tool here says so. Issue #347 asked
//! for the disjointness to become defeasible. That is not built, and
//! `docs/decisions/0019-an-exception-is-compiled-and-not-inferred.md` gives the
//! four reasons. What is built is an EXPLICIT exception, written as an
//! annotation on the disjointness axiom:
//!
//! ```turtle
//! @prefix exc: <https://open-ontologies.org/exceptions#> .
//! ex:Vehicle owl:disjointWith ex:Building .
//! [] a owl:Axiom ;
//!    owl:annotatedSource ex:Vehicle ;
//!    owl:annotatedProperty owl:disjointWith ;
//!    owl:annotatedTarget ex:Building ;
//!    exc:exceptFor ex:Houseboat .
//! ```
//!
//! and a COMPILE that rewrites the axiom into classical axioms that no longer
//! contradict the exception. Every other tool (the tableau, the rule engine,
//! SHACL, `defects`, the Lean certificates) then reads an ordinary OWL
//! ontology, and they cannot disagree with each other, because none of them
//! needs to know the annotation exists.
//!
//! The reading compiled is this one, over the ASSERTED hierarchy of named
//! classes: for an annotated `X ⊥ Y` with honoured exceptions E, let C be the
//! exceptions and every class below one. A class `a` under X and a class `b`
//! under Y stay compatible exactly when some member of C is told-below both,
//! and are disjoint otherwise. It is written out as:
//!
//! - Z, the exempt set, is C and every ancestor of a member of C. X and Y are
//!   always in Z, because every honoured e is under both.
//! - `X owl:disjointWith Y` is removed.
//! - `S owl:disjointWith Y` is emitted for every named S under X that is not in
//!   Z, and `T owl:disjointWith X` for every named T under Y that is not in Z:
//!   nothing licensed is below them, so they meet nothing on the other side.
//! - For `a` in Z under X and `b` in Z under Y, `a owl:disjointWith b` is
//!   emitted when no member of C is below both, keeping only the most general
//!   such pairs. With one exception this adds nothing; with two exceptions
//!   under different intermediate classes it is what keeps those classes
//!   apart.
//!
//! An exception that is not under both X and Y is rejected by name and not
//! used. The cost is reported rather than hidden: two classes that stay
//! compatible do so for every individual, so one typed only as `Vehicle` and
//! `Building` is no longer a contradiction, and a class added under X or Y
//! later gets no pair until the compile runs again.
//!
//! The hierarchy is the TOLD one: `rdfs:subClassOf` between named classes,
//! `owl:equivalentClass` between named classes in both directions, and the
//! named conjuncts of an `owl:intersectionOf` a class is declared under or
//! equivalent to. Nothing is inferred, so the compile is a function of the
//! asserted triples and needs no reasoner.
//!
//! THE AXIOM IS NOT LEFT DANGLING. After the compile, `X owl:disjointWith Y` is
//! gone, and an `owl:Axiom` reification still naming `owl:disjointWith` would
//! describe an axiom the document no longer holds. An RDF consumer that
//! rebuilds an axiom from its reification would then put the disjointness back
//! and contradict every tool here. So the compile rewrites the pair into a
//! RECORD in this namespace: the main triple becomes `X exc:compiledDisjointWith
//! Y`, an annotation property, and the reification's `owl:annotatedProperty`
//! names that property. The record is inert to every reasoner, it is still a
//! well-formed annotated OWL 2 annotation assertion, and it is what a second
//! run reads to add the pairs for classes filed under X or Y since the first.
//! A second run over an unchanged graph changes nothing.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use serde_json::{Value, json};

use crate::graph::GraphStore;

/// The namespace of the two terms this module reads and writes. It follows the
/// house pattern of one namespace per feature under `open-ontologies.org`
/// (`shacl#` for the report terms, `temporal#` for the versioning vocabulary).
pub const NS: &str = "https://open-ontologies.org/exceptions#";

/// On a reified `owl:disjointWith` axiom: a class that is an exception to it.
pub const EXCEPT_FOR: &str = "https://open-ontologies.org/exceptions#exceptFor";

/// The annotation property a compiled disjointness is recorded under, so the
/// record survives the axiom and a second run can find it.
pub const COMPILED_DISJOINT_WITH: &str =
    "https://open-ontologies.org/exceptions#compiledDisjointWith";

const OWL: &str = "http://www.w3.org/2002/07/owl#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/// What the reader is told every time, because it is the price of the compile
/// and the one thing a caller cannot see in the emitted triples.
const COST: &str = "Two classes on opposite sides stay compatible exactly when an exception, or \
                    a class below one, is told-below both, and that holds for every individual: \
                    one typed only as two compatible classes (for instance as both sides of the \
                    axiom and nothing more specific) is no longer a contradiction. \
                    exempted_classes lists the classes that keep some compatibility. A class \
                    filed under either side after this run gets no pair until \
                    compile-exceptions runs again, and a re-run adds pairs and never retracts \
                    one.";

type Row = HashMap<String, String>;

fn select(graph: &GraphStore, query: &str, union: bool) -> anyhow::Result<Vec<Row>> {
    let raw = if union {
        graph.sparql_select_union(query)?
    } else {
        graph.sparql_select(query)?
    };
    let parsed: Value = serde_json::from_str(&raw)?;
    Ok(parsed["results"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_object)
                .map(|o| {
                    o.iter()
                        .map(|(k, v)| {
                            let v = v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string());
                            (k.clone(), v)
                        })
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Whether `pattern` matches: in the default graph alone, or in the union of
/// every graph.
fn ask(graph: &GraphStore, pattern: &str, union: bool) -> anyhow::Result<bool> {
    let query = format!("ASK {{ {pattern} }}");
    let raw = if union { graph.sparql_select_union(&query)? } else { graph.sparql_select(&query)? };
    let parsed: Value = serde_json::from_str(&raw)?;
    Ok(parsed["result"].as_bool().unwrap_or(false))
}

fn is_named(term: &str) -> bool {
    term.starts_with('<')
}

fn disjoint_with() -> String {
    format!("<{OWL}disjointWith>")
}

fn compiled_with() -> String {
    format!("<{COMPILED_DISJOINT_WITH}>")
}

/// The told hierarchy of named classes, both ways round.
struct Hierarchy {
    up: HashMap<String, BTreeSet<String>>,
    down: HashMap<String, BTreeSet<String>>,
}

impl Hierarchy {
    fn read(graph: &GraphStore) -> anyhow::Result<Self> {
        let query = format!(
            "SELECT DISTINCT ?sub ?sup WHERE {{
               {{ ?sub <{RDFS}subClassOf> ?sup }}
               UNION {{ ?sub <{OWL}equivalentClass> ?sup }}
               UNION {{ ?sup <{OWL}equivalentClass> ?sub }}
               UNION {{ ?sub <{RDFS}subClassOf>|<{OWL}equivalentClass> ?x .
                        ?x <{OWL}intersectionOf>/<{RDF}rest>*/<{RDF}first> ?sup }}
               FILTER(isIRI(?sub) && isIRI(?sup) && ?sub != ?sup)
             }}"
        );
        let mut up: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut down: HashMap<String, BTreeSet<String>> = HashMap::new();
        for row in select(graph, &query, true)? {
            let (Some(sub), Some(sup)) = (row.get("sub"), row.get("sup")) else { continue };
            up.entry(sub.clone()).or_default().insert(sup.clone());
            down.entry(sup.clone()).or_default().insert(sub.clone());
        }
        Ok(Self { up, down })
    }

    /// Everything reachable from `from` along `edges`, `from` included.
    fn reach(edges: &HashMap<String, BTreeSet<String>>, from: &str) -> BTreeSet<String> {
        let mut seen = BTreeSet::from([from.to_string()]);
        let mut stack = vec![from.to_string()];
        while let Some(c) = stack.pop() {
            for next in edges.get(&c).into_iter().flatten() {
                if seen.insert(next.clone()) {
                    stack.push(next.clone());
                }
            }
        }
        seen
    }

    fn descendants(&self, class: &str) -> BTreeSet<String> {
        Self::reach(&self.down, class)
    }

    fn ancestors(&self, class: &str) -> BTreeSet<String> {
        Self::reach(&self.up, class)
    }
}

/// One reified axiom carrying `exceptFor`, in the direction it was written.
#[derive(Clone, Debug)]
struct Reification {
    node: String,
    source: String,
    target: String,
}

/// Every annotation on one unordered pair of classes.
#[derive(Debug)]
struct Pair {
    source: String,
    target: String,
    /// Reifications naming `owl:disjointWith`: not compiled yet.
    fresh: Vec<Reification>,
    /// Reifications naming `exc:compiledDisjointWith`: a record of a compile.
    records: Vec<Reification>,
    /// Every `exceptFor` value on any of them. One axiom annotated twice has
    /// the union of the two annotations, as OWL 2 reads it.
    exceptions: BTreeSet<String>,
}

struct Annotations {
    pairs: BTreeMap<(String, String), Pair>,
    /// `exceptFor` where this module cannot act on it, each with the reason.
    unsupported: Vec<Value>,
}

impl Annotations {
    fn read(graph: &GraphStore) -> anyhow::Result<Self> {
        let query = format!(
            "SELECT DISTINCT ?ax ?source ?property ?target ?exception WHERE {{
               ?ax <{EXCEPT_FOR}> ?exception .
               OPTIONAL {{ ?ax <{OWL}annotatedSource> ?source }}
               OPTIONAL {{ ?ax <{OWL}annotatedProperty> ?property }}
               OPTIONAL {{ ?ax <{OWL}annotatedTarget> ?target }}
             }}"
        );
        let mut rows = select(graph, &query, true)?;
        // Deterministic whatever order the store hands rows back in.
        rows.sort_by_key(|r| {
            ["ax", "source", "property", "target", "exception"]
                .map(|k| r.get(k).cloned().unwrap_or_default())
        });
        let (dw, cw) = (disjoint_with(), compiled_with());
        let mut pairs: BTreeMap<(String, String), Pair> = BTreeMap::new();
        let mut unsupported = Vec::new();
        for row in rows {
            let ax = row.get("ax").cloned().unwrap_or_default();
            let exception = row.get("exception").cloned().unwrap_or_default();
            let (source, property, target) = (row.get("source"), row.get("property"), row.get("target"));
            let (Some(s), Some(p), Some(t)) = (source, property, target) else {
                unsupported.push(json!({
                    "annotation": ax,
                    "exception": exception,
                    "reason": "exceptFor_outside_an_axiom_annotation",
                }));
                continue;
            };
            if *p != dw && *p != cw {
                unsupported.push(json!({
                    "annotation": ax,
                    "exception": exception,
                    "annotated_property": p,
                    "reason": "exceptFor_on_an_axiom_other_than_disjointWith",
                }));
                continue;
            }
            if !is_named(s) || !is_named(t) {
                unsupported.push(json!({
                    "annotation": ax,
                    "exception": exception,
                    "reason": "disjointness_not_between_named_classes",
                }));
                continue;
            }
            let key = if s <= t { (s.clone(), t.clone()) } else { (t.clone(), s.clone()) };
            let pair = pairs.entry(key).or_insert_with(|| Pair {
                source: s.clone(),
                target: t.clone(),
                fresh: Vec::new(),
                records: Vec::new(),
                exceptions: BTreeSet::new(),
            });
            let list = if *p == cw { &mut pair.records } else { &mut pair.fresh };
            if !list.iter().any(|r| r.node == ax) {
                list.push(Reification { node: ax.clone(), source: s.clone(), target: t.clone() });
            }
            pair.exceptions.insert(exception);
        }
        Ok(Self { pairs, unsupported })
    }
}

/// Every asserted `owl:disjointWith` between named classes, in any graph.
fn disjoint_triples(graph: &GraphStore) -> anyhow::Result<HashSet<(String, String)>> {
    let query = format!(
        "SELECT DISTINCT ?a ?b WHERE {{ ?a <{OWL}disjointWith> ?b . FILTER(isIRI(?a) && isIRI(?b)) }}"
    );
    Ok(select(graph, &query, true)?
        .into_iter()
        .filter_map(|r| Some((r.get("a")?.clone(), r.get("b")?.clone())))
        .collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// The disjointness is asserted and annotated: compile it.
    Compile,
    /// Only a record of an earlier compile: emit the pairs again.
    Recompile,
    /// A record whose every exception is now rejected: the disjointness it
    /// was compiled from holds in full again, so it is asserted again.
    Restore,
    /// Nothing to do, for the reason given.
    Skip(&'static str),
}

impl Action {
    fn status(self) -> &'static str {
        match self {
            Action::Compile => "compiled",
            Action::Recompile => "recompiled",
            Action::Restore => "restored",
            Action::Skip(_) => "not_compiled",
        }
    }
}

/// What the compile does to one annotated pair, decided before anything is
/// written, so `defects` can ask the same question the compile answers.
struct Analysis {
    source: String,
    target: String,
    action: Action,
    /// The `owl:disjointWith` triples asserted for the pair, either direction.
    present: Vec<(String, String)>,
    honoured: Vec<String>,
    rejected: Vec<Value>,
    /// Each class under an honoured exception, with that exception.
    covered: BTreeMap<String, String>,
    emitted: Vec<(String, String)>,
    exempted_classes: Vec<String>,
    exempted_beyond_the_exceptions: Vec<String>,
    still_under_both: Vec<String>,
}

fn analyse(pair: &Pair, h: &Hierarchy, disjoint: &HashSet<(String, String)>) -> Analysis {
    let mut present: Vec<(String, String)> = Vec::new();
    for (a, b) in [(&pair.source, &pair.target), (&pair.target, &pair.source)] {
        let t = (a.clone(), b.clone());
        if disjoint.contains(&t) && !present.contains(&t) {
            present.push(t);
        }
    }
    // Reported in the direction the axiom is written, when it is.
    let (source, target) =
        present.first().cloned().unwrap_or_else(|| (pair.source.clone(), pair.target.clone()));
    let under_source = h.descendants(&source);
    let under_target = h.descendants(&target);

    let mut honoured = Vec::new();
    let mut rejected = Vec::new();
    for e in &pair.exceptions {
        if !is_named(e) {
            rejected.push(json!({"exception": e, "reason": "exception_not_a_named_class"}));
            continue;
        }
        let (us, ut) = (under_source.contains(e), under_target.contains(e));
        if us && ut {
            honoured.push(e.clone());
        } else {
            rejected.push(json!({
                "exception": e,
                "reason": "exception_not_under_both",
                "under_source": us,
                "under_target": ut,
            }));
        }
    }

    let fresh = !pair.fresh.is_empty() && !present.is_empty();
    let action = match (honoured.is_empty(), fresh, pair.records.is_empty()) {
        (false, true, _) => Action::Compile,
        (false, false, false) => Action::Recompile,
        (true, true, _) => Action::Skip("no_exception_is_under_both"),
        (true, false, false) => Action::Restore,
        (_, false, true) => Action::Skip("annotation_without_axiom"),
    };

    // C, the classes the exceptions license: each honoured exception and
    // everything below it, mapped to the exception that covers it.
    let mut covered = BTreeMap::new();
    for e in &honoured {
        for d in h.descendants(e) {
            covered.entry(d).or_insert_with(|| e.clone());
        }
    }
    // Z, the exempt set: C and every ancestor of a member of C. A class
    // outside Z has no licensed class below it, so it is disjoint from the
    // whole other side, and one pair against that side's root says so.
    let mut exempt: BTreeSet<String> = covered.keys().cloned().collect();
    for c in covered.keys() {
        exempt.extend(h.ancestors(c));
    }

    let (mut emitted, mut exempted_classes, mut beyond, mut still) = (vec![], vec![], vec![], vec![]);
    if matches!(action, Action::Compile | Action::Recompile) {
        let mut pairs: BTreeSet<(String, String)> = under_source
            .iter()
            .filter(|s| !exempt.contains(*s))
            .map(|s| (s.clone(), target.clone()))
            .chain(
                under_target
                    .iter()
                    .filter(|t| !exempt.contains(*t))
                    .map(|t| (t.clone(), source.clone())),
            )
            .collect();

        // Inside Z the rule is per pair. Two classes on opposite sides stay
        // compatible exactly when some member of C is told-below both, and
        // are made disjoint otherwise. Without this, two exceptions under
        // different intermediate classes exempted every one of those classes
        // from every other, and the axiom could vanish entirely.
        let licensed_below = |class: &str| -> BTreeSet<String> {
            h.descendants(class).into_iter().filter(|d| covered.contains_key(d)).collect()
        };
        let side = |under: &BTreeSet<String>| -> BTreeMap<String, BTreeSet<String>> {
            exempt.iter().filter(|c| under.contains(*c)).map(|c| (c.clone(), licensed_below(c))).collect()
        };
        let (side_s, side_t) = (side(&under_source), side(&under_target));
        let incompatible: BTreeSet<(String, String)> = side_s
            .iter()
            .flat_map(|(a, below_a)| {
                side_t
                    .iter()
                    .filter(move |(b, below_b)| a != *b && below_a.is_disjoint(below_b))
                    .map(move |(b, _)| (a.clone(), b.clone()))
            })
            .collect();
        for (a, b) in &incompatible {
            // A pair whose classes sit below an incompatible pair is entailed
            // by it, so only the most general pairs are written.
            let (up_a, up_b) = (h.ancestors(a), h.ancestors(b));
            let entailed = up_a.iter().filter(|x| side_s.contains_key(*x)).any(|x| {
                up_b.iter().any(|y| (x != a || y != b) && incompatible.contains(&(x.clone(), y.clone())))
            });
            // owl:disjointWith is symmetric, so a class on both sides is
            // never paired twice with the same partner.
            if !entailed && !pairs.contains(&(b.clone(), a.clone())) {
                pairs.insert((a.clone(), b.clone()));
            }
        }
        emitted = pairs.into_iter().collect();
        exempted_classes = exempt
            .iter()
            .filter(|c| under_source.contains(*c) || under_target.contains(*c))
            .cloned()
            .collect();
        beyond = exempted_classes.iter().filter(|c| !covered.contains_key(*c)).cloned().collect();
        still = under_source
            .intersection(&under_target)
            .filter(|c| !exempt.contains(*c))
            .cloned()
            .collect();
    }

    Analysis {
        source,
        target,
        action,
        present,
        honoured,
        rejected,
        covered,
        emitted,
        exempted_classes,
        exempted_beyond_the_exceptions: beyond,
        still_under_both: still,
    }
}

/// `None` is the default graph.
type Location = Option<String>;

/// Where a triple pattern holds: the default graph and every named graph.
fn locations(graph: &GraphStore, pattern: &str) -> anyhow::Result<Vec<Location>> {
    let mut out = Vec::new();
    if ask(graph, pattern, false)? {
        out.push(None);
    }
    for row in select(graph, &format!("SELECT DISTINCT ?g WHERE {{ GRAPH ?g {{ {pattern} }} }}"), false)? {
        if let Some(g) = row.get("g") {
            out.push(Some(g.clone()));
        }
    }
    Ok(out)
}

fn in_location(loc: &Location, body: &str) -> String {
    match loc {
        None => body.to_string(),
        Some(g) => format!("GRAPH {g} {{ {body} }}"),
    }
}

/// How many triples go into one `INSERT DATA`.
const INSERT_BATCH: usize = 5_000;

/// Writes, counting what actually changed.
struct Writer<'a> {
    graph: &'a GraphStore,
    /// Every `owl:disjointWith` in the store, kept current as pairs are queued,
    /// so a pair already asserted anywhere is never written a second time.
    disjoint: HashSet<(String, String)>,
    /// Triples to add, per graph, written in batches by `flush`. One update
    /// per triple was measured at minutes for 20,000 pairs, because every
    /// update also counts the store before and after.
    pending: BTreeMap<Location, BTreeSet<String>>,
    inserted: usize,
    removed: usize,
    rewritten: usize,
}

impl Writer<'_> {
    fn insert(&mut self, loc: &Location, triple: String) {
        self.pending.entry(loc.clone()).or_default().insert(triple);
    }

    fn insert_pair(&mut self, loc: &Location, a: &str, b: &str) {
        if self.disjoint.insert((a.to_string(), b.to_string())) {
            self.insert(loc, format!("{a} {} {b} .", disjoint_with()));
        }
    }

    fn declare(&mut self, loc: &Location) -> anyhow::Result<()> {
        for p in [EXCEPT_FOR, COMPILED_DISJOINT_WITH] {
            let triple = format!("<{p}> a <{OWL}AnnotationProperty> .");
            let queued = self.pending.values().any(|t| t.contains(&triple));
            if !queued && !ask(self.graph, &triple, true)? {
                self.insert(loc, triple);
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        for (loc, triples) in std::mem::take(&mut self.pending) {
            let triples: Vec<String> = triples.into_iter().collect();
            for chunk in triples.chunks(INSERT_BATCH) {
                let body = in_location(&loc, &chunk.join("\n"));
                self.inserted += self.graph.sparql_update(&format!("INSERT DATA {{ {body} }}"))?;
            }
        }
        Ok(())
    }
}

/// The compile, and what `defects` needs to know about declared exceptions.
pub struct Exceptions;

impl Exceptions {
    /// Compile every annotated disjointness in `graph`, in place, and report
    /// what was done. A graph with no `exceptFor` is left exactly as it was.
    pub fn compile(graph: &Arc<GraphStore>) -> anyhow::Result<Value> {
        let annotations = Annotations::read(graph)?;
        // Neither the hierarchy nor the disjointness index is read when there
        // is nothing to compile, so a graph without `exceptFor` costs one query.
        let (hierarchy, disjoint) = if annotations.pairs.is_empty() {
            (Hierarchy { up: HashMap::new(), down: HashMap::new() }, HashSet::new())
        } else {
            (Hierarchy::read(graph)?, disjoint_triples(graph)?)
        };
        let analyses: Vec<Analysis> = annotations
            .pairs
            .values()
            .map(|p| analyse(p, &hierarchy, &disjoint))
            .collect();

        let mut w = Writer {
            graph: graph.as_ref(),
            disjoint,
            pending: BTreeMap::new(),
            inserted: 0,
            removed: 0,
            rewritten: 0,
        };
        let (dw, cw) = (disjoint_with(), compiled_with());
        let mut axioms = Vec::new();
        for (pair, a) in annotations.pairs.values().zip(&analyses) {
            let mut removed: Vec<Value> = Vec::new();
            match a.action {
                Action::Compile => {
                    // The pairs and the record go where the axiom was, so a
                    // disjointness kept in a named graph stays in it.
                    let mut written_to: Vec<Location> = Vec::new();
                    for (s, t) in &a.present {
                        for loc in locations(graph, &format!("{s} {dw} {t} ."))? {
                            if !written_to.contains(&loc) {
                                written_to.push(loc);
                            }
                        }
                    }
                    for loc in &written_to {
                        w.declare(loc)?;
                        for r in &pair.fresh {
                            w.insert(loc, format!("{} {cw} {} .", r.source, r.target));
                        }
                        for (s, t) in &a.emitted {
                            w.insert_pair(loc, s, t);
                        }
                    }
                    for r in &pair.fresh {
                        let pattern = format!(
                            "?ax <{OWL}annotatedProperty> {dw} ; <{OWL}annotatedSource> {} ; \
                             <{OWL}annotatedTarget> {} ; <{EXCEPT_FOR}> ?e .",
                            r.source, r.target
                        );
                        graph.sparql_update(&format!(
                            "DELETE {{ GRAPH ?g {{ ?ax <{OWL}annotatedProperty> {dw} }} }} \
                             INSERT {{ GRAPH ?g {{ ?ax <{OWL}annotatedProperty> {cw} }} }} \
                             WHERE {{ GRAPH ?g {{ {pattern} }} }} ; \
                             DELETE {{ ?ax <{OWL}annotatedProperty> {dw} }} \
                             INSERT {{ ?ax <{OWL}annotatedProperty> {cw} }} \
                             WHERE {{ {pattern} }}"
                        ))?;
                        w.rewritten += 1;
                    }
                    for (s, t) in &a.present {
                        let triple = format!("{s} {dw} {t} .");
                        for loc in locations(graph, &triple)? {
                            graph.sparql_update(&format!("DELETE DATA {{ {} }}", in_location(&loc, &triple)))?;
                            w.removed += 1;
                        }
                        w.disjoint.remove(&(s.clone(), t.clone()));
                        removed.push(json!([s, t]));
                    }
                }
                Action::Recompile | Action::Restore => {
                    let mut written_to: Vec<Location> = Vec::new();
                    for r in &pair.records {
                        for loc in locations(graph, &format!("{} {cw} {} .", r.source, r.target))? {
                            if !written_to.contains(&loc) {
                                written_to.push(loc);
                            }
                        }
                    }
                    if written_to.is_empty() {
                        written_to.push(None);
                    }
                    for loc in &written_to {
                        if a.action == Action::Restore {
                            if a.present.is_empty() {
                                w.insert_pair(loc, &a.source, &a.target);
                            }
                        } else {
                            for (s, t) in &a.emitted {
                                w.insert_pair(loc, s, t);
                            }
                        }
                    }
                }
                Action::Skip(_) => {}
            }
            let mut entry = json!({
                "source": a.source,
                "target": a.target,
                "status": a.action.status(),
                "removed": removed,
                "emitted": a.emitted.iter().map(|(s, t)| json!([s, t])).collect::<Vec<_>>(),
                "exceptions_honoured": a.honoured,
                "exceptions_rejected": a.rejected,
                "exempted_classes": a.exempted_classes,
                "exempted_beyond_the_exceptions": a.exempted_beyond_the_exceptions,
                "still_under_both": a.still_under_both,
            });
            if let Action::Skip(reason) = a.action {
                entry["reason"] = json!(reason);
            }
            axioms.push(entry);
        }
        w.flush()?;

        let count = |status: &str| axioms.iter().filter(|a| a["status"] == status).count();
        let summary = json!({
            "annotated_axioms": axioms.len(),
            "compiled": count("compiled"),
            "recompiled": count("recompiled"),
            "restored": count("restored"),
            "not_compiled": count("not_compiled"),
            "removed": w.removed,
            "emitted": analyses.iter().map(|a| a.emitted.len()).sum::<usize>(),
            "inserted_triples": w.inserted,
            "exceptions_honoured": analyses.iter().map(|a| a.honoured.len()).sum::<usize>(),
            "exceptions_rejected": analyses.iter().map(|a| a.rejected.len()).sum::<usize>(),
            "exempted_classes": analyses.iter().map(|a| a.exempted_classes.len()).sum::<usize>(),
            "unsupported_annotations": annotations.unsupported.len(),
        });
        Ok(json!({
            "exceptions_namespace": NS,
            "changed": w.inserted + w.removed + w.rewritten > 0,
            "summary": summary,
            "axioms": axioms,
            "unsupported_annotations": annotations.unsupported,
            "cost": COST,
        }))
    }

    /// The exceptions a compile WOULD honour on a disjointness that is still
    /// asserted, for `defects` to point at. Reads only; writes nothing.
    pub fn declared(graph: &Arc<GraphStore>) -> anyhow::Result<Declared> {
        let annotations = Annotations::read(graph)?;
        let mut by_pair = HashMap::new();
        if annotations.pairs.is_empty() {
            return Ok(Declared { by_pair });
        }
        let hierarchy = Hierarchy::read(graph)?;
        let disjoint = disjoint_triples(graph)?;
        for (key, pair) in &annotations.pairs {
            let a = analyse(pair, &hierarchy, &disjoint);
            if a.action == Action::Compile {
                by_pair.insert(key.clone(), a.covered);
            }
        }
        Ok(Declared { by_pair })
    }
}

/// For each annotated disjointness still asserted, the classes a compile would
/// make satisfiable, each with the exception that covers it.
pub struct Declared {
    by_pair: HashMap<(String, String), BTreeMap<String, String>>,
}

impl Declared {
    pub fn is_empty(&self) -> bool {
        self.by_pair.is_empty()
    }

    /// The declared exception covering `class` under the disjointness of `a`
    /// and `b` (either order): `class` itself or one of its ancestors.
    pub fn covering(&self, a: &str, b: &str, class: &str) -> Option<&str> {
        let key = if a <= b { (a.to_string(), b.to_string()) } else { (b.to_string(), a.to_string()) };
        self.by_pair.get(&key)?.get(class).map(String::as_str)
    }
}

/// The hint a `defects` row carries when the class it is about is a declared
/// exception that has not been compiled yet.
pub fn defects_hint(exception: &str) -> String {
    format!(
        "{exception} is declared an exception to this disjointness with exceptFor ({NS}), and \
         the disjointness is still asserted, so every reasoner here finds this class \
         unsatisfiable. Run `open-ontologies compile-exceptions FILE --out COMPILED` (or the MCP \
         tool onto_compile_exceptions) to compile the exception into ordinary disjointness \
         axioms."
    )
}

fn bare(term: &str) -> &str {
    term.trim_start_matches('<').trim_end_matches('>')
}

fn list(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_str).map(|s| bare(s).to_string()).collect())
        .unwrap_or_default()
}

/// A compile report as text, for `--human`.
pub fn render_human(value: &Value) -> String {
    if let Some(msg) = value.get("error").and_then(Value::as_str) {
        return format!("Error: {msg}");
    }
    let s = &value["summary"];
    let n = |k: &str| s[k].as_u64().unwrap_or(0);
    let mut out = format!(
        "compile-exceptions: {} annotated disjointness axiom(s): {} compiled, {} recompiled, {} \
         restored, {} not compiled\n",
        n("annotated_axioms"),
        n("compiled"),
        n("recompiled"),
        n("restored"),
        n("not_compiled")
    );
    for a in value["axioms"].as_array().into_iter().flatten() {
        let source = bare(a["source"].as_str().unwrap_or("?"));
        let target = bare(a["target"].as_str().unwrap_or("?"));
        let status = a["status"].as_str().unwrap_or("?");
        out.push_str(&format!("\n{source} disjointWith {target}: {status}"));
        if let Some(reason) = a["reason"].as_str() {
            out.push_str(&format!(" ({reason})"));
        }
        out.push('\n');
        let honoured = list(&a["exceptions_honoured"]);
        if !honoured.is_empty() {
            out.push_str(&format!("  exceptions honoured: {}\n", honoured.join(", ")));
        }
        for r in a["exceptions_rejected"].as_array().into_iter().flatten() {
            let e = bare(r["exception"].as_str().unwrap_or("?"));
            let reason = r["reason"].as_str().unwrap_or("?");
            out.push_str(&format!("  exception rejected: {e} ({reason})\n"));
        }
        for r in a["removed"].as_array().into_iter().flatten() {
            let t = list(r);
            if t.len() == 2 {
                out.push_str(&format!("  removed: {} disjointWith {}\n", t[0], t[1]));
            }
        }
        let emitted = a["emitted"].as_array().map(Vec::len).unwrap_or(0);
        if emitted > 0 {
            out.push_str(&format!("  disjointness pairs emitted: {emitted}\n"));
        }
        let exempted = list(&a["exempted_classes"]);
        if !exempted.is_empty() {
            out.push_str(&format!("  exempted: {}\n", exempted.join(", ")));
        }
        let beyond = list(&a["exempted_beyond_the_exceptions"]);
        if !beyond.is_empty() {
            out.push_str(&format!(
                "  exempted without being an exception (the cost): {}\n",
                beyond.join(", ")
            ));
        }
        let still = list(&a["still_under_both"]);
        if !still.is_empty() {
            out.push_str(&format!(
                "  still unsatisfiable, under both and no exception: {}\n",
                still.join(", ")
            ));
        }
    }
    for u in value["unsupported_annotations"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "\nexceptFor not acted on: {} ({})\n",
            bare(u["annotation"].as_str().unwrap_or("?")),
            u["reason"].as_str().unwrap_or("?")
        ));
    }
    out.push_str(&format!(
        "\n{} triple(s) inserted, {} removed",
        n("inserted_triples"),
        n("removed")
    ));
    if let Some(path) = value["written"].as_str() {
        out.push_str(&format!("; written to {path}"));
    }
    out.push('\n');
    if n("compiled") + n("recompiled") > 0 {
        out.push_str(&format!("{COST}\n"));
    }
    out
}
