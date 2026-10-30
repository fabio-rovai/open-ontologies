//! Weighted repair: the cheapest set of asserted axioms whose removal makes
//! every class coherent, with the evidence for each half of that sentence.
//!
//! `defects` says THAT a class can have no instances, and `onto_justify` says
//! WHICH minimal axiom sets are responsible. Neither says which axioms to drop.
//! When axioms carry confidences (machine-extracted ontologies, merged sources,
//! model-elicited axioms), the natural repair is a minimum-weight hitting set of
//! the conflicts: drop the cheapest set of axioms that breaks every one of them.
//! Kalyanpur, Parsia, Sirin and Cuenca Grau, "Repairing unsatisfiable concepts
//! in OWL ontologies" (ESWC 2006), rank axioms for removal and build repair
//! plans as hitting sets over the justifications of the broken classes; this is
//! that idea with the ranking replaced by an explicit objective and the search
//! replaced by one that can say when it is optimal.
//!
//! # The fragment, and nothing wider
//!
//! Coherence is judged in the ATOMIC NAMED-CLASS FRAGMENT, exactly the one
//! `defects` judges with `disjoint_with_ancestor` and `inherited_disjoint`:
//! asserted `rdfs:subClassOf` and `owl:disjointWith` between two IRIs, plus
//! `owl:equivalentClass` between two IRIs read as ONE axiom contributing two
//! subclass edges. Each asserted triple is one axiom. A class `C` is incoherent
//! here when it reaches `X` and `Y` through `rdfs:subClassOf*` and `X
//! owl:disjointWith Y` (or the reverse) is asserted, `X = Y` and `C = X`
//! included. The one case that goes beyond `defects` is a class asserted
//! disjoint with itself, which `defects` does not look for.
//!
//! Everything else is COUNTED and never read: class expressions on either side
//! of those three predicates, `owl:AllDisjointClasses`, `owl:disjointUnionOf`,
//! `owl:complementOf` on a named class, property axioms, and the rest of the
//! graph by bucket, so that the total of the buckets is the number of triples
//! and nothing is passed over in silence. A class coherent here can still have
//! no instances under the whole ontology, and the `scope` block says so.
//!
//! # Weights, and why summing confidences is wrong
//!
//! With `cost` weights (the default) a weight is what dropping the axiom costs,
//! and the repair minimises the sum over the dropped axioms. With
//! `probability` weights a weight is the probability that the axiom is right,
//! and the tempting reading, "minimise the confidence thrown away", is not the
//! most probable repair. Under independence the probability of keeping the set
//! `K` is `prod_{i in K} p_i * prod_{i not in K} (1 - p_i)`, whose logarithm is
//! a constant plus `sum_{i in K} ln(p_i / (1 - p_i))`. Maximising it is
//! minimising `sum_{i dropped} ln(p_i / (1 - p_i))`: the LOG-ODDS of what is
//! dropped, not its probability. Log-linear description logics (Niepert,
//! Noessner and Stuckenschmidt, IJCAI 2011) put an axiom's weight in exactly
//! that exponent, and their most probable coherent ontology is this repair.
//! The two objectives disagree on ordinary inputs: two conflicts `{a=0.6,
//! c=0.9}` and `{b=0.6, c=0.9}` are repaired under `cost` by dropping `c`
//! (0.9 < 1.2) and under `probability` by dropping `a` and `b` (2 x 0.405 =
//! 0.81 < 2.197). `tests/repair_test.rs` pins both answers.
//!
//! Two edges of the probability scale are handled by name. `p = 1` is a HARD
//! axiom and is never dropped; a conflict made only of hard axioms cannot be
//! repaired within the soft ones, and is reported as `unrepairable` with its
//! class rather than looped on. `p <= 0.5` has log-odds `<= 0`, so dropping it
//! cannot lower the probability of the repaired ontology: every such axiom in a
//! collected conflict is dropped AT NO COST and listed with that reason. The
//! objective is the clamped one, `max(0, ln(p / (1 - p)))`. The unclamped most
//! probable world would also drop every `p < 0.5` axiom that is in no conflict;
//! a repair removes only what coherence needs, so those are kept and counted.
//!
//! # The search: implicit hitting sets, not every MinA first
//!
//! Enumerating every conflict before choosing is exponential for no benefit.
//! This is the implicit hitting set loop of Davies and Bacchus, "Solving MAXSAT
//! by solving a sequence of simpler SAT instances" (CP 2011):
//!
//! 1. Find the incoherent classes. For each ROOT among them (an incoherent
//!    class none of whose direct superclasses is incoherent, the distinction
//!    Kalyanpur, Parsia, Sirin and Hendler draw in "Debugging unsatisfiable
//!    classes in OWL ontologies", J. Web Semantics 2005), extract a conflict:
//!    the asserted edges of a shortest subclass path from the class to `X`,
//!    those of a shortest path to `Y`, and the `X`/`Y` disjointness axiom.
//!    Shrink it one axiom at a time while the class stays incoherent, so every
//!    conflict is minimal for its class.
//! 2. Compute a hitting set of every conflict collected so far. Rounds use a
//!    greedy one, which is all a round needs to expose a conflict the removal
//!    missed. Once a greedy repair comes out coherent, the EXACT minimum-cost
//!    hitting set is computed by branch and bound under a node budget. Its
//!    bound is the larger of two: a packing of pairwise disjoint conflicts, and
//!    an apportioning bound that charges each axiom `c / f` to each of the `f`
//!    open conflicts it is in, which is the one that bites when conflicts share
//!    a disjointness axiom.
//! 3. Remove it and check coherence again. If the fragment is coherent and the
//!    hitting set was the exact one, the repair is optimal: every repair has to
//!    hit every collected conflict, so the minimum over them is a lower bound on
//!    every repair, and this repair meets it. If not, a conflict disjoint from
//!    the removed set exists; collect it and repeat.
//!
//! Step 3's argument relies on MONOTONICITY, and in this fragment it holds:
//! removing an axiom can only remove subclass paths and disjointness pairs, so a
//! subset in which a class is incoherent leaves it incoherent in every superset.
//! A bound that fires (`max_nodes` inside the hitting-set search, `max_rounds`
//! on the loop) makes the answer `optimal: false` with the best repair found and
//! the best lower bound proved, and names the bound.
//!
//! # Where the answer comes from, and what checked it
//!
//! Three different things are going on and they carry three different words,
//! as in [`crate::justify`].
//!
//! * **Conflicts are re-run.** Every reported conflict is re-checked over
//!   exactly its own axioms (the class is incoherent there) and over each
//!   subset missing one axiom (the class is coherent there). That is a property
//!   of this module's coherence check, verified by execution, and no theorem.
//! * **Coherence is re-run.** The repaired fragment is checked again from
//!   scratch, and with `out` the written file is read back and checked again,
//!   by this module and by `defects`.
//! * **Optimality is refuted by oo-lrat, when asked.** With `certificate_dir`,
//!   the claim "no set of soft axioms with scaled cost at most `w* - 1` hits
//!   every collected conflict" is written as CNF: one variable per soft
//!   conflict axiom (true means dropped), one clause per conflict, and the
//!   pseudo-Boolean bound `sum c_i x_i <= w* - 1` as a reduced BDD (Abio,
//!   Nieuwenhuis, Oliveras, Rodriguez-Carbonell and Mayer-Eichberger, "A new
//!   look at BDDs for pseudo-Boolean constraints", JAIR 2012). [`crate::sat::refute`]
//!   runs picosat and hands the LRAT proof to `oo-lrat`; when it accepts, the
//!   word is `refuted` and the theorem is `Lrat.unsat_of_check`. What that
//!   theorem covers is that the CNF has no model. That the CNF says what the
//!   sentence above says is the encoder's claim, tested by brute force in this
//!   module's unit tests and NOT machine-checked. Without picosat or `oo-lrat`
//!   the status is `undetermined` with the reason, never `refuted`.
//!
//! The certificate is about INTEGER costs. Decimal `cost` weights are scaled by
//! the smallest power of ten that makes them integers (exact when every weight
//! has at most six decimals), and log-odds are scaled by 10^6 and rounded; the
//! output names the scale, whether the scaling was exact, and for rounded
//! log-odds a bound on how far the real-valued optimum can be from this one.

use crate::graph::GraphStore;
use crate::sat::{Clause, SatOutcome};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const DISJOINT: &str = "<http://www.w3.org/2002/07/owl#disjointWith>";
const EQUIVALENT: &str = "<http://www.w3.org/2002/07/owl#equivalentClass>";
const ANNOTATED_SOURCE: &str = "<http://www.w3.org/2002/07/owl#annotatedSource>";
const ANNOTATED_PROPERTY: &str = "<http://www.w3.org/2002/07/owl#annotatedProperty>";
const ANNOTATED_TARGET: &str = "<http://www.w3.org/2002/07/owl#annotatedTarget>";
const RDF_TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const OWL_NOTHING: &str = "<http://www.w3.org/2002/07/owl#Nothing>";
const OWL_NS: &str = "http://www.w3.org/2002/07/owl#";
const RDFS_NS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/// Branch-and-bound nodes ONE hitting-set computation may expand. A run that
/// hits it keeps the best hitting set found and stops claiming optimality for
/// that round; if it is the last round, the answer says `optimal: false` and
/// names this bound.
pub const DEFAULT_MAX_NODES: usize = 100_000;

/// Rounds of the implicit hitting set loop before it stops searching and
/// completes the repair greedily. Every round adds at least one new conflict.
pub const DEFAULT_MAX_ROUNDS: usize = 500;

/// New conflicts collected per round at most. Collecting several per round
/// costs nothing in correctness and saves rounds when the roots are many.
const MAX_NEW_CONFLICTS_PER_ROUND: usize = 16;

/// Nodes of the BDD for the certificate's cost bound before the encoder gives
/// up and reports `undetermined`.
const MAX_ENCODING_NODES: usize = 200_000;

/// Log-odds are scaled by this and rounded to integers.
const PROBABILITY_SCALE: i64 = 1_000_000;

/// Decimals a `cost` weight may carry and still be scaled exactly.
const MAX_DECIMALS: u32 = 6;

/// Listings longer than this are cut, with the full count beside them.
const MAX_LISTED: usize = 200;

// ───────────────────────────────────────────────────────────────────────────
// Options
// ───────────────────────────────────────────────────────────────────────────

/// How a weight is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeightKind {
    /// The weight is what dropping the axiom costs.
    Cost,
    /// The weight is the probability that the axiom is right.
    Probability,
}

impl WeightKind {
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "cost" => Ok(Self::Cost),
            "probability" => Ok(Self::Probability),
            other => anyhow::bail!(
                "weights must be `cost` or `probability`, got `{other}`. They are different \
                 objectives: under `probability` the repair minimises the LOG-ODDS of what is \
                 dropped, which is the most probable coherent subset under independence, and \
                 summing raw probabilities is not"
            ),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Cost => "cost",
            Self::Probability => "probability",
        }
    }
}

pub struct RepairOptions {
    /// The annotation property carrying an axiom's weight on its `owl:Axiom`
    /// reification. `None` gives every axiom the default weight.
    pub weight_property: Option<String>,
    /// The weight of an axiom with no weight annotation.
    pub default_weight: f64,
    pub weights: WeightKind,
    /// Where to write the optimality certificate. `None` writes none.
    pub certificate_dir: Option<PathBuf>,
    pub max_nodes: usize,
    pub max_rounds: usize,
    /// Where to write the repaired graph as Turtle. `None` writes nothing.
    pub out: Option<PathBuf>,
}

impl Default for RepairOptions {
    fn default() -> Self {
        Self {
            weight_property: None,
            default_weight: 1.0,
            weights: WeightKind::Cost,
            certificate_dir: None,
            max_nodes: DEFAULT_MAX_NODES,
            max_rounds: DEFAULT_MAX_ROUNDS,
            out: None,
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Reading the fragment
// ───────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Kind {
    SubClassOf,
    DisjointWith,
    EquivalentClass,
}

impl Kind {
    fn of(p: &str) -> Option<Self> {
        match p {
            SUBCLASS => Some(Self::SubClassOf),
            DISJOINT => Some(Self::DisjointWith),
            EQUIVALENT => Some(Self::EquivalentClass),
            _ => None,
        }
    }

    fn spelled(self) -> &'static str {
        match self {
            Self::SubClassOf => SUBCLASS,
            Self::DisjointWith => DISJOINT,
            Self::EquivalentClass => EQUIVALENT,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::SubClassOf => "subClassOf",
            Self::DisjointWith => "disjointWith",
            Self::EquivalentClass => "equivalentClass",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cost {
    /// `p = 1` under `probability`. Never dropped.
    Hard,
    /// `p <= 0.5` under `probability`. Dropped at no cost when in a conflict.
    Free,
    /// The scaled integer cost of dropping it.
    Soft(i64),
}

#[derive(Clone, Debug)]
struct Axiom {
    kind: Kind,
    /// N-Triples spellings, `<iri>`.
    s: String,
    o: String,
    weight: f64,
    written: String,
    annotated: bool,
    cost: Cost,
    /// The objective contribution in real numbers: the weight under `cost`,
    /// the log-odds under `probability` (0 for a free axiom).
    real_cost: f64,
}

impl Axiom {
    fn json(&self) -> Value {
        json!({"s": bare(&self.s), "p": bare(self.kind.spelled()), "o": bare(&self.o)})
    }
}

fn bare(term: &str) -> &str {
    term.strip_prefix('<').and_then(|t| t.strip_suffix('>')).unwrap_or(term)
}

fn is_iri(term: &str) -> bool {
    term.starts_with('<')
}

/// The lexical form of an N-Triples literal, or `None` for an IRI or a blank
/// node.
fn lexical(term: &str) -> Option<&str> {
    let rest = term.strip_prefix('"')?;
    let mut escaped = false;
    for (i, c) in rest.char_indices() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(&rest[..i]);
        }
    }
    None
}

/// Digits after the decimal point, or `None` for an exponent form.
fn decimals(lex: &str) -> Option<u32> {
    if lex.contains(['e', 'E']) {
        return None;
    }
    Some(lex.split_once('.').map(|(_, f)| f.trim().len() as u32).unwrap_or(0))
}

struct Reification {
    source: Option<String>,
    property: Option<String>,
    target: Option<String>,
    weights: Vec<String>,
}

/// Everything read from the graph: the axioms of the fragment with their
/// costs, and the counts of everything that was not read.
struct Read {
    axioms: Vec<Axiom>,
    /// Every distinct triple lands in exactly one bucket.
    buckets: BTreeMap<&'static str, usize>,
    /// The class-level and property-level axiom kinds outside the fragment.
    not_considered: BTreeMap<&'static str, usize>,
    /// `C rdfs:subClassOf owl:Nothing` and the like, read as an ordinary edge.
    into_nothing: usize,
    weight_annotations_used: usize,
    weight_annotations_unused: BTreeMap<&'static str, usize>,
    cost_scale: i64,
    scaling_exact: bool,
    /// `probability` only: axioms with `p < 0.5` read in the fragment. Those in
    /// no conflict are kept, and the count says how many.
    below_half: BTreeSet<usize>,
}

fn read(graph: &GraphStore, opts: &RepairOptions) -> anyhow::Result<Read> {
    let inferred = format!("<{}>", crate::reason::INFERRED_GRAPH);
    let mut triples: BTreeSet<(String, String, String)> = BTreeSet::new();
    for (s, p, o, g) in graph.all_quads()? {
        if g == inferred {
            continue;
        }
        triples.insert((s, p, o));
    }

    // Reification nodes first, so their triples are bucketed as annotations
    // and never as class-expression structure.
    let weight_property = opts.weight_property.as_deref().map(|w| {
        let w = w.trim();
        if w.starts_with('<') { w.to_string() } else { format!("<{w}>") }
    });
    let mut reif: BTreeMap<String, Reification> = BTreeMap::new();
    for (s, p, _) in &triples {
        if p == ANNOTATED_SOURCE {
            reif.entry(s.clone()).or_insert(Reification {
                source: None,
                property: None,
                target: None,
                weights: Vec::new(),
            });
        }
    }
    for (s, p, o) in &triples {
        let Some(r) = reif.get_mut(s) else { continue };
        match p.as_str() {
            ANNOTATED_SOURCE => r.source = Some(o.clone()),
            ANNOTATED_PROPERTY => r.property = Some(o.clone()),
            ANNOTATED_TARGET => r.target = Some(o.clone()),
            _ => {}
        }
        if weight_property.as_deref() == Some(p.as_str()) {
            r.weights.push(o.clone());
        }
    }

    let mut buckets: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut not_considered: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut fragment: Vec<(Kind, String, String)> = Vec::new();
    let mut into_nothing = 0usize;
    let declaration_types: HashSet<String> = [
        "Class",
        "ObjectProperty",
        "DatatypeProperty",
        "AnnotationProperty",
        "Ontology",
        "NamedIndividual",
    ]
    .iter()
    .map(|l| format!("<{OWL_NS}{l}>"))
    .chain([format!("<{RDFS_NS}Class>"), format!("<{RDF_NS}Property>"), format!("<{RDFS_NS}Datatype>")])
    .collect();
    let characteristic_types: HashSet<String> = [
        "FunctionalProperty",
        "InverseFunctionalProperty",
        "TransitiveProperty",
        "SymmetricProperty",
        "AsymmetricProperty",
        "ReflexiveProperty",
        "IrreflexiveProperty",
    ]
    .iter()
    .map(|l| format!("<{OWL_NS}{l}>"))
    .collect();
    let property_predicates: HashSet<String> = [
        format!("<{RDFS_NS}domain>"),
        format!("<{RDFS_NS}range>"),
        format!("<{RDFS_NS}subPropertyOf>"),
        format!("<{OWL_NS}inverseOf>"),
        format!("<{OWL_NS}equivalentProperty>"),
        format!("<{OWL_NS}propertyDisjointWith>"),
        format!("<{OWL_NS}propertyChainAxiom>"),
    ]
    .into_iter()
    .collect();
    let all_disjoint = format!("<{OWL_NS}AllDisjointClasses>");
    let disjoint_union = format!("<{OWL_NS}disjointUnionOf>");
    let complement = format!("<{OWL_NS}complementOf>");

    for (s, p, o) in &triples {
        if reif.contains_key(s) {
            *buckets.entry("axiom_annotations").or_default() += 1;
            continue;
        }
        if let Some(kind) = Kind::of(p) {
            if is_iri(s) && is_iri(o) {
                if o == OWL_NOTHING || (kind == Kind::EquivalentClass && s == OWL_NOTHING) {
                    into_nothing += 1;
                }
                fragment.push((kind, s.clone(), o.clone()));
                *buckets.entry("axioms_considered").or_default() += 1;
            } else {
                let k = match kind {
                    Kind::SubClassOf => "subClassOf_not_between_two_named_classes",
                    Kind::DisjointWith => "disjointWith_not_between_two_named_classes",
                    Kind::EquivalentClass => "equivalentClass_not_between_two_named_classes",
                };
                *not_considered.entry(k).or_default() += 1;
                *buckets.entry("axioms_not_considered").or_default() += 1;
            }
            continue;
        }
        let outside: Option<&'static str> = if p == RDF_TYPE && *o == all_disjoint {
            Some("AllDisjointClasses")
        } else if *p == disjoint_union {
            Some("disjointUnionOf")
        } else if *p == complement && is_iri(s) {
            Some("complementOf_on_a_named_class")
        } else if property_predicates.contains(p)
            || (p == RDF_TYPE && characteristic_types.contains(o))
        {
            Some("property_axioms")
        } else {
            None
        };
        if let Some(k) = outside {
            *not_considered.entry(k).or_default() += 1;
            *buckets.entry("axioms_not_considered").or_default() += 1;
            continue;
        }
        let bucket = if p == RDF_TYPE && declaration_types.contains(o) {
            "declarations"
        } else if s.starts_with("_:") {
            "blank_node_structure"
        } else {
            "other"
        };
        *buckets.entry(bucket).or_default() += 1;
    }

    // Weights, keyed by the axiom they annotate.
    let asserted: HashSet<(&str, &str, &str)> =
        triples.iter().map(|(s, p, o)| (s.as_str(), p.as_str(), o.as_str())).collect();
    let in_fragment: HashSet<(Kind, &str, &str)> =
        fragment.iter().map(|(k, s, o)| (*k, s.as_str(), o.as_str())).collect();
    let mut weights: HashMap<(Kind, String, String), (f64, String)> = HashMap::new();
    let mut used = 0usize;
    let mut unused: BTreeMap<&'static str, usize> = BTreeMap::new();
    if weight_property.is_some() {
        for (node, r) in &reif {
            if r.weights.is_empty() {
                continue;
            }
            let (Some(s), Some(p), Some(o)) = (&r.source, &r.property, &r.target) else {
                *unused.entry("reification_incomplete").or_default() += 1;
                continue;
            };
            let key = Kind::of(p).filter(|k| in_fragment.contains(&(*k, s.as_str(), o.as_str())));
            let Some(kind) = key else {
                let why = if asserted.contains(&(s.as_str(), p.as_str(), o.as_str())) {
                    "axiom_outside_the_fragment"
                } else {
                    "axiom_not_asserted"
                };
                *unused.entry(why).or_default() += 1;
                continue;
            };
            for w in &r.weights {
                let Some(lex) = lexical(w) else {
                    anyhow::bail!(
                        "the weight of {s} {p} {o} on {node} is {w}, which is not a literal. A \
                         weight that cannot be read is not a default weight, so nothing was repaired"
                    );
                };
                let value: f64 = lex.trim().parse().map_err(|_| {
                    anyhow::anyhow!(
                        "the weight of {s} {p} {o} is \"{lex}\", which is not a number. A weight \
                         that cannot be read is not a default weight, so nothing was repaired"
                    )
                })?;
                let k = (kind, s.clone(), o.clone());
                if let Some((prev, prev_lex)) = weights.get(&k)
                    && *prev != value
                {
                    anyhow::bail!(
                        "{s} {p} {o} carries two different weights, \"{prev_lex}\" and \"{lex}\". \
                         Choosing one would be a guess, so nothing was repaired"
                    );
                }
                weights.insert(k, (value, lex.trim().to_string()));
            }
            used += 1;
        }
    }

    // Costs.
    let default_written = format!("{}", opts.default_weight);
    check_weight(opts.weights, opts.default_weight, "the default weight")?;
    let mut raw: Vec<(Kind, String, String, f64, String, bool)> = Vec::with_capacity(fragment.len());
    for (kind, s, o) in fragment {
        let (w, written, annotated) = match weights.get(&(kind, s.clone(), o.clone())) {
            Some((w, lex)) => (*w, lex.clone(), true),
            None => (opts.default_weight, default_written.clone(), false),
        };
        check_weight(opts.weights, w, &format!("the weight of {s} {} {o}", kind.spelled()))?;
        raw.push((kind, s, o, w, written, annotated));
    }

    let (cost_scale, scaling_exact) = match opts.weights {
        WeightKind::Probability => (PROBABILITY_SCALE, false),
        WeightKind::Cost => {
            let mut d = 0u32;
            let mut exact = true;
            for (_, _, _, _, written, _) in &raw {
                match decimals(written) {
                    Some(n) if n <= MAX_DECIMALS => d = d.max(n),
                    _ => {
                        d = MAX_DECIMALS;
                        exact = false;
                    }
                }
            }
            (10i64.pow(d), exact)
        }
    };

    let mut axioms = Vec::with_capacity(raw.len());
    let mut below_half = BTreeSet::new();
    for (kind, s, o, w, written, annotated) in raw {
        let (cost, real_cost) = match opts.weights {
            WeightKind::Cost => {
                let scaled = w * cost_scale as f64;
                if scaled > 1e15 {
                    anyhow::bail!(
                        "the weight of {s} {} {o} is {w}, which is too large to scale to an integer \
                         cost exactly",
                        kind.spelled()
                    );
                }
                (Cost::Soft(scaled.round() as i64), w)
            }
            WeightKind::Probability => {
                if w >= 1.0 {
                    (Cost::Hard, f64::INFINITY)
                } else if w <= 0.5 {
                    (Cost::Free, 0.0)
                } else {
                    let logit = (w / (1.0 - w)).ln();
                    (Cost::Soft((logit * PROBABILITY_SCALE as f64).round() as i64), logit)
                }
            }
        };
        if opts.weights == WeightKind::Probability && w < 0.5 {
            below_half.insert(axioms.len());
        }
        axioms.push(Axiom { kind, s, o, weight: w, written, annotated, cost, real_cost });
    }

    Ok(Read {
        axioms,
        buckets,
        not_considered,
        into_nothing,
        weight_annotations_used: used,
        weight_annotations_unused: unused,
        cost_scale,
        scaling_exact,
        below_half,
    })
}

fn check_weight(kind: WeightKind, w: f64, what: &str) -> anyhow::Result<()> {
    if !w.is_finite() {
        anyhow::bail!("{what} is {w}, which is not a finite number");
    }
    match kind {
        WeightKind::Cost if w < 0.0 => anyhow::bail!(
            "{what} is {w}. A cost is never negative: dropping an axiom cannot pay the repair"
        ),
        WeightKind::Probability if !(0.0..=1.0).contains(&w) => {
            anyhow::bail!("{what} is {w}, which is not a probability")
        }
        _ => Ok(()),
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The fragment as a graph, and the coherence check
// ───────────────────────────────────────────────────────────────────────────

struct Fragment {
    classes: Vec<String>,
    axioms: Vec<Axiom>,
    /// class -> (superclass, axiom), sorted.
    up: Vec<Vec<(u32, u32)>>,
    /// class -> (subclass, axiom), sorted.
    down: Vec<Vec<(u32, u32)>>,
    /// class -> (partner, axiom), both directions, sorted.
    disjoint_at: Vec<Vec<(u32, u32)>>,
    /// (x, y, axiom).
    disjoint: Vec<(u32, u32, u32)>,
    /// axiom -> (subject class, object class).
    ends: Vec<(u32, u32)>,
}

impl Fragment {
    fn new(axioms: Vec<Axiom>) -> Self {
        let mut names: BTreeSet<&str> = BTreeSet::new();
        for a in &axioms {
            names.insert(&a.s);
            names.insert(&a.o);
        }
        let classes: Vec<String> = names.into_iter().map(str::to_string).collect();
        let index: HashMap<&str, u32> =
            classes.iter().enumerate().map(|(i, c)| (c.as_str(), i as u32)).collect();
        let n = classes.len();
        let mut up = vec![Vec::new(); n];
        let mut down = vec![Vec::new(); n];
        let mut disjoint_at = vec![Vec::new(); n];
        let mut disjoint = Vec::new();
        let mut ends = Vec::with_capacity(axioms.len());
        for (i, a) in axioms.iter().enumerate() {
            let (s, o, i) = (index[a.s.as_str()], index[a.o.as_str()], i as u32);
            ends.push((s, o));
            match a.kind {
                Kind::SubClassOf => {
                    up[s as usize].push((o, i));
                    down[o as usize].push((s, i));
                }
                Kind::EquivalentClass => {
                    up[s as usize].push((o, i));
                    down[o as usize].push((s, i));
                    up[o as usize].push((s, i));
                    down[s as usize].push((o, i));
                }
                Kind::DisjointWith => {
                    disjoint.push((s, o, i));
                    disjoint_at[s as usize].push((o, i));
                    if s != o {
                        disjoint_at[o as usize].push((s, i));
                    }
                }
            }
        }
        for v in up.iter_mut().chain(down.iter_mut()).chain(disjoint_at.iter_mut()) {
            v.sort_unstable();
        }
        Self { classes, axioms, up, down, disjoint_at, disjoint, ends }
    }

    /// Every class that reaches `z` through kept subclass edges, `z` included,
    /// as a bitset.
    fn below(&self, z: u32, keep: &[bool]) -> Vec<u64> {
        let mut bits = vec![0u64; self.classes.len().div_ceil(64)];
        let mut queue = VecDeque::from([z]);
        bits[(z / 64) as usize] |= 1 << (z % 64);
        while let Some(c) = queue.pop_front() {
            for &(sub, a) in &self.down[c as usize] {
                if keep[a as usize] && bits[(sub / 64) as usize] & (1 << (sub % 64)) == 0 {
                    bits[(sub / 64) as usize] |= 1 << (sub % 64);
                    queue.push_back(sub);
                }
            }
        }
        bits
    }

    /// The incoherent classes over the kept axioms: for every kept
    /// disjointness `X / Y`, the classes below both.
    fn incoherent(&self, keep: &[bool]) -> BTreeSet<u32> {
        let words = self.classes.len().div_ceil(64);
        let mut out = vec![0u64; words];
        let mut cache: HashMap<u32, Vec<u64>> = HashMap::new();
        for &(x, y, a) in &self.disjoint {
            if !keep[a as usize] {
                continue;
            }
            for z in [x, y] {
                cache.entry(z).or_insert_with(|| self.below(z, keep));
            }
            let (bx, by) = (&cache[&x], &cache[&y]);
            for i in 0..words {
                out[i] |= bx[i] & by[i];
            }
        }
        let mut set = BTreeSet::new();
        for (i, w) in out.iter().enumerate() {
            let mut w = *w;
            while w != 0 {
                let b = w.trailing_zeros();
                set.insert(i as u32 * 64 + b);
                w &= w - 1;
            }
        }
        set
    }

    /// The incoherent classes none of whose direct kept superclasses is in
    /// `inc`. When a cycle leaves none, the first class of `inc`.
    fn roots(&self, inc: &BTreeSet<u32>, keep: &[bool]) -> Vec<u32> {
        let roots: Vec<u32> = inc
            .iter()
            .copied()
            .filter(|&c| {
                !self.up[c as usize]
                    .iter()
                    .any(|&(sup, a)| keep[a as usize] && sup != c && inc.contains(&sup))
            })
            .collect();
        if roots.is_empty() {
            inc.iter().take(1).copied().collect()
        } else {
            roots
        }
    }

    /// A conflict for `c` over the kept axioms: the edges of a shortest path
    /// to `X`, those of a shortest path to `Y`, and the disjointness. `None`
    /// when `c` is coherent.
    fn extract(&self, c: u32, keep: &[bool]) -> Option<BTreeSet<u32>> {
        let mut prev: HashMap<u32, (u32, Option<(u32, u32)>)> = HashMap::new();
        prev.insert(c, (0, None));
        let mut order = vec![c];
        let mut queue = VecDeque::from([c]);
        while let Some(z) = queue.pop_front() {
            let d = prev[&z].0;
            for &(sup, a) in &self.up[z as usize] {
                if keep[a as usize] && !prev.contains_key(&sup) {
                    prev.insert(sup, (d + 1, Some((z, a))));
                    order.push(sup);
                    queue.push_back(sup);
                }
            }
        }
        let mut best: Option<(u32, u32, u32, u32)> = None;
        for &z in &order {
            for &(w, a) in &self.disjoint_at[z as usize] {
                if !keep[a as usize] {
                    continue;
                }
                let Some(&(dw, _)) = prev.get(&w) else { continue };
                let cand = (prev[&z].0 + dw, a, z, w);
                if best.is_none_or(|b| (cand.0, cand.1) < (b.0, b.1)) {
                    best = Some(cand);
                }
            }
        }
        let (_, a, x, y) = best?;
        let mut out = BTreeSet::from([a]);
        for mut z in [x, y] {
            while let Some((p, edge)) = prev[&z].1 {
                out.insert(edge);
                z = p;
            }
        }
        Some(out)
    }

    /// Is `c` incoherent over exactly `subset`? The oracle every conflict is
    /// shrunk and re-run against.
    fn incoherent_in(&self, c: u32, subset: &BTreeSet<u32>) -> bool {
        let mut up: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut disjoint: Vec<(u32, u32)> = Vec::new();
        for &a in subset {
            let (s, o) = self.ends[a as usize];
            match self.axioms[a as usize].kind {
                Kind::SubClassOf => up.entry(s).or_default().push(o),
                Kind::EquivalentClass => {
                    up.entry(s).or_default().push(o);
                    up.entry(o).or_default().push(s);
                }
                Kind::DisjointWith => disjoint.push((s, o)),
            }
        }
        let mut reached = HashSet::from([c]);
        let mut queue = VecDeque::from([c]);
        while let Some(z) = queue.pop_front() {
            for &sup in up.get(&z).map(Vec::as_slice).unwrap_or(&[]) {
                if reached.insert(sup) {
                    queue.push_back(sup);
                }
            }
        }
        disjoint.iter().any(|(x, y)| reached.contains(x) && reached.contains(y))
    }

    /// Shrink a conflict one axiom at a time while `c` stays incoherent.
    /// Linear deletion is exact for a monotone oracle.
    fn minimise(&self, c: u32, conflict: &BTreeSet<u32>) -> BTreeSet<u32> {
        let mut cur = conflict.clone();
        for &a in conflict {
            let mut trial = cur.clone();
            trial.remove(&a);
            if self.incoherent_in(c, &trial) {
                cur = trial;
            }
        }
        cur
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The exact minimum-cost hitting set
// ───────────────────────────────────────────────────────────────────────────

/// One hitting-set instance: the soft axioms that occur in some conflict not
/// already hit by a free axiom, and those conflicts over them.
struct HsProblem {
    /// Local index -> axiom id.
    items: Vec<u32>,
    costs: Vec<i64>,
    /// Each set sorted by (cost, index); no set is a superset of another.
    sets: Vec<Vec<usize>>,
    /// Free axioms that occur in a collected conflict: dropped at no cost.
    forced: BTreeSet<u32>,
}

fn hs_problem(frag: &Fragment, conflicts: &[Conflict]) -> anyhow::Result<HsProblem> {
    let mut forced = BTreeSet::new();
    for c in conflicts {
        for &a in &c.axioms {
            if frag.axioms[a as usize].cost == Cost::Free {
                forced.insert(a);
            }
        }
    }
    let mut local: BTreeMap<u32, usize> = BTreeMap::new();
    let mut sets: Vec<BTreeSet<u32>> = Vec::new();
    for c in conflicts {
        if c.axioms.iter().any(|a| forced.contains(a)) {
            continue;
        }
        let soft: BTreeSet<u32> = c
            .axioms
            .iter()
            .copied()
            .filter(|&a| matches!(frag.axioms[a as usize].cost, Cost::Soft(_)))
            .collect();
        if soft.is_empty() {
            anyhow::bail!(
                "a conflict for {} holds only hard axioms although the class was not found \
                 incoherent under the hard axioms alone. This is a defect in this module; please \
                 report it",
                bare(&frag.classes[c.class as usize])
            );
        }
        sets.push(soft);
    }
    // Drop supersets: hitting the smaller set hits them.
    sets.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    sets.dedup();
    let mut kept: Vec<BTreeSet<u32>> = Vec::new();
    for s in sets {
        if !kept.iter().any(|k| k.is_subset(&s)) {
            kept.push(s);
        }
    }
    for s in &kept {
        for &a in s {
            let next = local.len();
            local.entry(a).or_insert(next);
        }
    }
    let mut items = vec![0u32; local.len()];
    let mut costs = vec![0i64; local.len()];
    for (&a, &i) in &local {
        items[i] = a;
        costs[i] = match frag.axioms[a as usize].cost {
            Cost::Soft(c) => c,
            _ => unreachable!("only soft axioms are items"),
        };
    }
    let sets = kept
        .into_iter()
        .map(|s| {
            let mut v: Vec<usize> = s.iter().map(|a| local[a]).collect();
            v.sort_by_key(|&i| (costs[i], i));
            v
        })
        .collect();
    Ok(HsProblem { items, costs, sets, forced })
}

struct HsSolution {
    /// Local indices.
    chosen: Vec<usize>,
    cost: i64,
    /// The search ran to completion, so `chosen` is a minimum.
    closed: bool,
    /// The optimum when closed, the root bound when not.
    lower_bound: i64,
    nodes: usize,
}

enum Exam {
    Infeasible,
    Covered,
    Open { lb: i64, lb_count: usize, branch: usize },
}

struct Bnb<'a> {
    p: &'a HsProblem,
    occ: Vec<Vec<usize>>,
    hit: Vec<u32>,
    excluded: Vec<bool>,
    cur: Vec<usize>,
    cur_cost: i64,
    best: Vec<usize>,
    best_cost: i64,
    nodes: usize,
    max_nodes: usize,
    aborted: bool,
    stamp: Vec<u64>,
    epoch: u64,
    degree: Vec<u32>,
    degree_epoch: Vec<u64>,
}

impl<'a> Bnb<'a> {
    fn new(p: &'a HsProblem, occ: Vec<Vec<usize>>, incumbent: Vec<usize>, max_nodes: usize) -> Self {
        let best_cost = incumbent.iter().map(|&e| p.costs[e]).sum();
        Bnb {
            p,
            occ,
            hit: vec![0; p.sets.len()],
            excluded: vec![false; p.items.len()],
            cur: Vec::new(),
            cur_cost: 0,
            best: incumbent,
            best_cost,
            nodes: 0,
            max_nodes,
            aborted: false,
            stamp: vec![0; p.items.len()],
            epoch: 0,
            degree: vec![0; p.items.len()],
            degree_epoch: vec![0; p.items.len()],
        }
    }

    fn choose(&mut self, e: usize) {
        self.cur.push(e);
        self.cur_cost += self.p.costs[e];
        for &s in &self.occ[e] {
            self.hit[s] += 1;
        }
    }

    fn unchoose(&mut self, e: usize) {
        self.cur.pop();
        self.cur_cost -= self.p.costs[e];
        for &s in &self.occ[e] {
            self.hit[s] -= 1;
        }
    }

    /// Infeasible, covered, or a lower bound and the set to branch on.
    ///
    /// Two bounds, and the larger is used. PACKING: pairwise disjoint unhit
    /// sets, most expensive first, each contributing its cheapest available
    /// item, since every completion has to choose at least one item per packed
    /// set. APPORTIONING: an item in `f` unhit sets pays `c / f` towards each,
    /// so every completion pays at least the sum over unhit sets of the
    /// cheapest such share. The second is far stronger when conflicts overlap,
    /// which is the usual case: conflicts that share a disjointness axiom. It
    /// is computed in integers rounded DOWN, so rounding can only weaken it.
    fn examine(&mut self) -> Exam {
        self.epoch += 1;
        let mut unhit: Vec<(i64, usize, usize)> = Vec::new();
        for (si, s) in self.p.sets.iter().enumerate() {
            if self.hit[si] > 0 {
                continue;
            }
            let mut min = None;
            let mut count = 0usize;
            for &e in s {
                if !self.excluded[e] {
                    count += 1;
                    if min.is_none() {
                        min = Some(self.p.costs[e]);
                    }
                }
            }
            let Some(min) = min else { return Exam::Infeasible };
            unhit.push((min, count, si));
        }
        if unhit.is_empty() {
            return Exam::Covered;
        }
        for &(_, _, si) in &unhit {
            for &e in &self.p.sets[si] {
                if !self.excluded[e] {
                    if self.degree_epoch[e] != self.epoch {
                        self.degree_epoch[e] = self.epoch;
                        self.degree[e] = 0;
                    }
                    self.degree[e] += 1;
                }
            }
        }
        // 720720 is lcm(1..=16), so a share is exact for any item in at most
        // sixteen unhit sets and rounded down beyond that.
        const D: i128 = 720_720;
        let mut shares: i128 = 0;
        for &(_, _, si) in &unhit {
            let cheapest = self.p.sets[si]
                .iter()
                .filter(|&&e| !self.excluded[e])
                .map(|&e| (self.p.costs[e] as i128 * D) / self.degree[e] as i128)
                .min()
                .unwrap_or(0);
            shares += cheapest;
        }
        let apportioned = ((shares + D - 1) / D) as i64;
        let branch = unhit
            .iter()
            .min_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)).then(a.2.cmp(&b.2)))
            .map(|u| u.2)
            .expect("non-empty");
        unhit.sort_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(&b.2)));
        let (mut lb, mut lb_count) = (0i64, 0usize);
        for &(min, _, si) in &unhit {
            let s = &self.p.sets[si];
            if s.iter().any(|&e| !self.excluded[e] && self.stamp[e] == self.epoch) {
                continue;
            }
            for &e in s {
                if !self.excluded[e] {
                    self.stamp[e] = self.epoch;
                }
            }
            lb += min;
            lb_count += 1;
        }
        Exam::Open { lb: lb.max(apportioned), lb_count, branch }
    }

    fn dfs(&mut self) {
        if self.aborted {
            return;
        }
        self.nodes += 1;
        if self.nodes > self.max_nodes {
            self.aborted = true;
            return;
        }
        match self.examine() {
            Exam::Infeasible => {}
            Exam::Covered => {
                if (self.cur_cost, self.cur.len()) < (self.best_cost, self.best.len()) {
                    self.best = self.cur.clone();
                    self.best_cost = self.cur_cost;
                }
            }
            Exam::Open { lb, lb_count, branch } => {
                if (self.cur_cost + lb, self.cur.len() + lb_count) >= (self.best_cost, self.best.len())
                {
                    return;
                }
                // Branch i takes the i-th item and excludes the ones before
                // it, so no hitting set is reached twice and none is missed.
                let avail: Vec<usize> =
                    self.p.sets[branch].iter().copied().filter(|&e| !self.excluded[e]).collect();
                let mut excluded_here = Vec::new();
                for e in avail {
                    self.choose(e);
                    self.dfs();
                    self.unchoose(e);
                    if self.aborted {
                        break;
                    }
                    self.excluded[e] = true;
                    excluded_here.push(e);
                }
                for e in excluded_here {
                    self.excluded[e] = false;
                }
            }
        }
    }
}

/// A hitting set by ratio-greedy, made irredundant. The first incumbent.
fn greedy(p: &HsProblem, occ: &[Vec<usize>]) -> Vec<usize> {
    let mut hit = vec![false; p.sets.len()];
    let mut chosen: Vec<usize> = Vec::new();
    let mut taken = vec![false; p.items.len()];
    while hit.iter().any(|h| !h) {
        let mut best: Option<(usize, usize)> = None; // (item, newly hit)
        for e in 0..p.items.len() {
            if taken[e] {
                continue;
            }
            let newly = occ[e].iter().filter(|&&s| !hit[s]).count();
            if newly == 0 {
                continue;
            }
            let better = match best {
                None => true,
                // newly / cost larger, compared without division.
                Some((b, bn)) => {
                    (newly as i128) * (p.costs[b] as i128) > (bn as i128) * (p.costs[e] as i128)
                }
            };
            if better {
                best = Some((e, newly));
            }
        }
        let Some((e, _)) = best else { break };
        taken[e] = true;
        chosen.push(e);
        for &s in &occ[e] {
            hit[s] = true;
        }
    }
    chosen.sort_by_key(|&e| std::cmp::Reverse((p.costs[e], e)));
    let mut keep: Vec<usize> = chosen.clone();
    for e in chosen {
        let others: Vec<usize> = keep.iter().copied().filter(|&x| x != e).collect();
        let still = occ[e].iter().all(|&s| p.sets[s].iter().any(|i| others.contains(i)));
        if still {
            keep = others;
        }
    }
    keep
}

fn occurrences(p: &HsProblem) -> Vec<Vec<usize>> {
    let mut occ = vec![Vec::new(); p.items.len()];
    for (si, s) in p.sets.iter().enumerate() {
        for &e in s {
            occ[e].push(si);
        }
    }
    occ
}

/// The root bound alone: what every hitting set of `p` costs at least. Valid
/// for every repair, because every repair hits every collected conflict.
fn root_bound(p: &HsProblem) -> i64 {
    if p.sets.is_empty() {
        return 0;
    }
    let occ = occurrences(p);
    match Bnb::new(p, occ, Vec::new(), 0).examine() {
        Exam::Open { lb, .. } => lb,
        _ => 0,
    }
}

fn solve(p: &HsProblem, max_nodes: usize) -> HsSolution {
    if p.sets.is_empty() {
        return HsSolution { chosen: Vec::new(), cost: 0, closed: true, lower_bound: 0, nodes: 0 };
    }
    let occ = occurrences(p);
    let incumbent = greedy(p, &occ);
    let mut b = Bnb::new(p, occ, incumbent, max_nodes);
    let root_lb = match b.examine() {
        Exam::Open { lb, .. } => lb,
        _ => 0,
    };
    b.dfs();
    let closed = !b.aborted;
    let mut chosen = b.best.clone();
    chosen.sort_unstable();
    HsSolution {
        chosen,
        cost: b.best_cost,
        closed,
        lower_bound: if closed { b.best_cost } else { root_lb },
        nodes: b.nodes,
    }
}

/// Recursion in the search and in the encoder is as deep as the instance, so
/// both run on a thread with a stack sized for that rather than on whatever
/// thread called this module.
fn on_big_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn_scoped(scope, f)
            .expect("a thread can be spawned")
            .join()
            .expect("the search does not panic")
    })
}

// ───────────────────────────────────────────────────────────────────────────
// The certificate: sum c_i x_i <= K as a reduced BDD, then CNF
// ───────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Node {
    False,
    True,
    At(usize),
}

/// A reduced ordered BDD for `sum_i a_i x_i <= K` with `a_i > 0`, built by the
/// interval algorithm of Abio et al. (JAIR 2012): every node carries the
/// interval of right-hand sides it stands for, so a right-hand side inside a
/// known interval reuses the node instead of building another.
struct Bdd {
    coeffs: Vec<i64>,
    suffix: Vec<i64>,
    levels: Vec<BTreeMap<i64, (i64, Node)>>,
    /// (level, child when the level's variable is true, child when false).
    nodes: Vec<(usize, Node, Node)>,
    too_big: bool,
}

fn shift(v: i64, a: i64) -> i64 {
    if v == i64::MIN || v == i64::MAX { v } else { v.saturating_add(a) }
}

impl Bdd {
    fn new(coeffs: Vec<i64>) -> Self {
        let n = coeffs.len();
        let mut suffix = vec![0i64; n + 1];
        for i in (0..n).rev() {
            suffix[i] = suffix[i + 1] + coeffs[i];
        }
        Self { coeffs, suffix, levels: vec![BTreeMap::new(); n + 1], nodes: Vec::new(), too_big: false }
    }

    fn build(&mut self, i: usize, k: i64) -> ((i64, i64), Node) {
        if k < 0 {
            return ((i64::MIN, -1), Node::False);
        }
        if k >= self.suffix[i] {
            return ((self.suffix[i], i64::MAX), Node::True);
        }
        if let Some((&beta, &(gamma, node))) = self.levels[i].range(..=k).next_back()
            && gamma >= k
        {
            return ((beta, gamma), node);
        }
        if self.too_big {
            return ((k, k), Node::False);
        }
        let a = self.coeffs[i];
        let (lo_iv, lo) = self.build(i + 1, k);
        let (hi_iv, hi) = self.build(i + 1, k - a);
        let beta = lo_iv.0.max(shift(hi_iv.0, a));
        let gamma = lo_iv.1.min(shift(hi_iv.1, a));
        let node = if lo == hi {
            lo
        } else {
            self.nodes.push((i, hi, lo));
            if self.nodes.len() > MAX_ENCODING_NODES {
                self.too_big = true;
            }
            Node::At(self.nodes.len() - 1)
        };
        self.levels[i].insert(beta, (gamma, node));
        ((beta, gamma), node)
    }
}

/// The CNF whose refutation says no hitting set of the problem's sets costs at
/// most `bound`. Variables `1..=m` are the items (true = dropped); the BDD's
/// nodes follow. `Err` names why no CNF was written.
fn encode(p: &HsProblem, bound: i64) -> Result<(Vec<Clause>, usize, usize), String> {
    let m = p.items.len();
    let mut clauses: Vec<Clause> = p
        .sets
        .iter()
        .map(|s| s.iter().map(|&e| e as i32 + 1).collect())
        .collect();
    // Items of cost zero cannot move the sum, so they are left out of the
    // bound and stay in the conflict clauses.
    let mut order: Vec<usize> = (0..m).filter(|&e| p.costs[e] > 0).collect();
    order.sort_by_key(|&e| (std::cmp::Reverse(p.costs[e]), e));
    let mut bdd = Bdd::new(order.iter().map(|&e| p.costs[e]).collect());
    let (_, root) = bdd.build(0, bound);
    if bdd.too_big {
        return Err(format!(
            "the cost bound needs more than {MAX_ENCODING_NODES} BDD nodes over {} weighted \
             axioms, so no CNF was written",
            order.len()
        ));
    }
    let var_of = |n: usize| (m + 1 + n) as i32;
    let lit = |node: Node| match node {
        Node::At(n) => Some(var_of(n)),
        _ => None,
    };
    for (n, &(level, hi, lo)) in bdd.nodes.iter().enumerate() {
        let v = var_of(n);
        let x = order[level] as i32 + 1;
        // x false: the low child must hold.
        match lo {
            Node::True => {}
            Node::False => clauses.push(vec![-v, x]),
            Node::At(_) => clauses.push(vec![-v, x, lit(lo).expect("a node")]),
        }
        // x true: the high child must hold.
        match hi {
            Node::True => {}
            Node::False => clauses.push(vec![-v, -x]),
            Node::At(_) => clauses.push(vec![-v, -x, lit(hi).expect("a node")]),
        }
    }
    match root {
        Node::At(n) => clauses.push(vec![var_of(n)]),
        Node::True => {
            return Err("the bound is at least the cost of dropping everything, so there is \
                        nothing to refute"
                .into());
        }
        Node::False => {
            return Err("the bound is negative, so it is false by arithmetic and needs no proof".into());
        }
    }
    Ok((clauses, m + bdd.nodes.len(), bdd.nodes.len()))
}

// ───────────────────────────────────────────────────────────────────────────
// The entry point
// ───────────────────────────────────────────────────────────────────────────

struct Conflict {
    class: u32,
    axioms: BTreeSet<u32>,
    round: usize,
}

/// The cheapest set of asserted axioms whose removal makes every class
/// coherent in the atomic named-class fragment, with the conflicts it hits,
/// the evidence for its optimality, and the scope it was judged in.
pub fn repair(graph: &Arc<GraphStore>, opts: &RepairOptions) -> anyhow::Result<Value> {
    let r = read(graph, opts)?;
    let n_axioms = r.axioms.len();
    let frag = Fragment::new(r.axioms);
    let scale = r.cost_scale;

    let all = vec![true; n_axioms];
    let before = frag.incoherent(&all);
    let hard_keep: Vec<bool> = frag.axioms.iter().map(|a| a.cost == Cost::Hard).collect();
    let hard_inc = if hard_keep.iter().any(|&h| h) {
        frag.incoherent(&hard_keep)
    } else {
        BTreeSet::new()
    };

    // Classes no soft axiom can rescue, each with one hard-only conflict.
    let mut unrepairable: Vec<Value> = Vec::new();
    for c in frag.roots(&hard_inc, &hard_keep).into_iter().take(MAX_LISTED) {
        let Some(raw) = frag.extract(c, &hard_keep) else { continue };
        let m = frag.minimise(c, &raw);
        unrepairable.push(json!({
            "class": bare(&frag.classes[c as usize]),
            "axioms": m.iter().map(|&a| frag.axioms[a as usize].json()).collect::<Vec<_>>(),
        }));
    }

    // ── The implicit hitting set loop ────────────────────────────────────
    let mut conflicts: Vec<Conflict> = Vec::new();
    let mut seen: HashSet<BTreeSet<u32>> = HashSet::new();
    let mut keep = all.clone();
    let mut removed: BTreeSet<u32> = BTreeSet::new();
    let mut rounds = 0usize;
    let mut nodes = 0usize;
    let mut lb_scaled = 0i64;
    let mut bound_fired: Option<&'static str> = None;
    let mut last: Option<(HsProblem, HsSolution)> = None;
    let mut greedy_completion = 0usize;
    // Whether the hitting set now removed is the EXACT one over every
    // collected conflict. Intermediate rounds use a greedy hitting set, which
    // is all a round needs to expose a conflict the removal missed; the exact
    // search runs only once a greedy repair comes out coherent, and the
    // optimality argument needs nothing more than that the LAST one is exact.
    let mut removed_is_exact = false;
    let mut exact_solves = 0usize;

    loop {
        let inc: BTreeSet<u32> = frag.incoherent(&keep).difference(&hard_inc).copied().collect();
        if inc.is_empty() {
            if conflicts.is_empty() || removed_is_exact {
                break;
            }
            let problem = hs_problem(&frag, &conflicts)?;
            let max_nodes = opts.max_nodes;
            let sol = on_big_stack(|| solve(&problem, max_nodes));
            nodes += sol.nodes;
            exact_solves += 1;
            lb_scaled = lb_scaled.max(sol.lower_bound);
            removed = problem.forced.clone();
            removed.extend(sol.chosen.iter().map(|&e| problem.items[e]));
            keep = all.clone();
            for &a in &removed {
                keep[a as usize] = false;
            }
            last = Some((problem, sol));
            removed_is_exact = true;
            continue;
        }
        if rounds >= opts.max_rounds {
            bound_fired = Some("max_rounds");
            // The search stopped; finish with a valid repair, cheapest axiom
            // of each new conflict first, and claim nothing about it.
            loop {
                let inc: BTreeSet<u32> =
                    frag.incoherent(&keep).difference(&hard_inc).copied().collect();
                let Some(&t) = frag.roots(&inc, &keep).first() else { break };
                let Some(raw) = frag.extract(t, &keep) else {
                    anyhow::bail!("a class reported incoherent has no conflict; this is a defect")
                };
                let m = frag.minimise(t, &raw);
                let pick = m
                    .iter()
                    .copied()
                    .filter(|&a| frag.axioms[a as usize].cost != Cost::Hard)
                    .min_by_key(|&a| match frag.axioms[a as usize].cost {
                        Cost::Soft(c) => (c, a),
                        _ => (0, a),
                    })
                    .ok_or_else(|| anyhow::anyhow!("a fixable conflict holds only hard axioms"))?;
                if seen.insert(m.clone()) {
                    conflicts.push(Conflict { class: t, axioms: m, round: rounds + 1 });
                }
                removed.insert(pick);
                keep[pick as usize] = false;
                greedy_completion += 1;
            }
            break;
        }
        let mut added = 0usize;
        for t in frag.roots(&inc, &keep) {
            let Some(raw) = frag.extract(t, &keep) else { continue };
            let m = frag.minimise(t, &raw);
            if seen.insert(m.clone()) {
                conflicts.push(Conflict { class: t, axioms: m, round: rounds + 1 });
                added += 1;
                if added >= MAX_NEW_CONFLICTS_PER_ROUND {
                    break;
                }
            }
        }
        if added == 0 {
            anyhow::bail!(
                "a class is still incoherent after the repair and no new conflict could be \
                 extracted for it. Under a monotone check that cannot happen, so this is a defect \
                 in this module; please report it"
            );
        }
        rounds += 1;
        let problem = hs_problem(&frag, &conflicts)?;
        let chosen = greedy(&problem, &occurrences(&problem));
        lb_scaled = lb_scaled.max(root_bound(&problem));
        removed = problem.forced.clone();
        removed.extend(chosen.iter().map(|&e| problem.items[e]));
        keep = all.clone();
        for &a in &removed {
            keep[a as usize] = false;
        }
        removed_is_exact = false;
    }

    let closed = last.as_ref().is_none_or(|(_, s)| s.closed);
    debug_assert!(bound_fired.is_some() || conflicts.is_empty() || removed_is_exact);
    if bound_fired.is_none() && !closed {
        bound_fired = Some("max_nodes");
    }
    let optimal = bound_fired.is_none();
    let total_scaled: i64 = removed
        .iter()
        .map(|&a| match frag.axioms[a as usize].cost {
            Cost::Soft(c) => c,
            _ => 0,
        })
        .sum();
    if optimal {
        lb_scaled = total_scaled;
    }
    let total_real: f64 = removed
        .iter()
        .map(|&a| {
            let ax = &frag.axioms[a as usize];
            if ax.cost == Cost::Free { 0.0 } else { ax.real_cost }
        })
        .sum();

    // ── Conflicts, re-run ───────────────────────────────────────────────
    let mut conflict_failures: Vec<Value> = Vec::new();
    let mut checks = 0usize;
    let mut conflicts_json: Vec<Value> = Vec::new();
    for (id, c) in conflicts.iter().enumerate() {
        checks += 1;
        let sufficient = frag.incoherent_in(c.class, &c.axioms);
        let mut minimal = true;
        for &a in &c.axioms {
            checks += 1;
            let mut trial = c.axioms.clone();
            trial.remove(&a);
            if frag.incoherent_in(c.class, &trial) {
                minimal = false;
                conflict_failures.push(json!({"conflict": id, "kind": "axiom_not_necessary",
                    "axiom": frag.axioms[a as usize].json()}));
            }
        }
        if !sufficient {
            conflict_failures.push(json!({"conflict": id, "kind": "class_coherent_over_the_conflict"}));
        }
        conflicts_json.push(json!({
            "id": id,
            "class": bare(&frag.classes[c.class as usize]),
            "axioms": c.axioms.iter().map(|&a| {
                let ax = &frag.axioms[a as usize];
                let mut j = ax.json();
                j["weight"] = json!(ax.weight);
                if ax.cost == Cost::Hard { j["hard"] = json!(true); }
                j
            }).collect::<Vec<_>>(),
            "size": c.axioms.len(),
            "found_in_round": c.round,
            "sufficiency": if sufficient { "re_run_and_incoherent" } else { "RE_RUN_AND_COHERENT" },
            "minimality": if minimal {
                "re_run_without_each_axiom_and_coherent"
            } else {
                "FAILED: the class stayed incoherent without an axiom"
            },
        }));
    }

    // ── Coherence, re-run ───────────────────────────────────────────────
    let after = frag.incoherent(&keep);
    let fixable_left: Vec<u32> = after.difference(&hard_inc).copied().collect();
    let made_coherent: Vec<u32> = before.difference(&after).copied().collect();

    // ── The dropped axioms ──────────────────────────────────────────────
    let mut dropped: Vec<Value> = Vec::new();
    let mut free_dropped = 0usize;
    for &a in &removed {
        let ax = &frag.axioms[a as usize];
        let hits: Vec<usize> = conflicts
            .iter()
            .enumerate()
            .filter(|(_, c)| c.axioms.contains(&a))
            .map(|(i, _)| i)
            .collect();
        let (reason, cost_scaled) = match ax.cost {
            Cost::Free => {
                free_dropped += 1;
                ("probability_at_most_one_half_dropped_at_no_cost", 0)
            }
            Cost::Soft(c) if optimal => ("minimum_cost_hitting_set", c),
            Cost::Soft(c) => ("best_repair_found_not_proved_minimum", c),
            Cost::Hard => ("HARD_AXIOM_DROPPED_THIS_IS_A_DEFECT", 0),
        };
        dropped.push(json!({
            "axiom": ax.json(),
            "weight": ax.weight,
            "weight_as_written": ax.written,
            "weight_source": if ax.annotated { "annotation" } else { "default" },
            "cost": if ax.cost == Cost::Free { 0.0 } else { ax.real_cost },
            "cost_scaled": cost_scaled,
            "reason": reason,
            "hits": hits,
        }));
    }

    // ── The certificate ─────────────────────────────────────────────────
    let mut stop_the_line = 0u64;
    let certificate = match &opts.certificate_dir {
        None => json!({
            "status": "not_requested",
            "means": "optimality rests on the branch and bound closing. Pass certificate_dir to \
                      have it refuted by oo-lrat as well",
        }),
        Some(_) if conflicts.is_empty() => json!({
            "status": "not_needed",
            "reason": "no conflict a soft axiom could break was found, so the empty repair is the \
                       only one and there is nothing to be optimal about",
        }),
        Some(_) if !optimal => json!({
            "status": "not_attempted",
            "reason": "the search did not establish optimality, so there is no claim to certify",
        }),
        Some(dir) => {
            let (problem, sol) = last.as_ref().expect("an optimal run with conflicts has a last round");
            if sol.cost == 0 {
                json!({
                    "status": "not_needed",
                    "reason": "the repair costs 0 in the scaled integer costs and no cost is \
                               negative, so no repair can cost less",
                })
            } else {
                let (v, stop) = certify(&frag, problem, sol.cost, dir, scale)?;
                stop_the_line += stop;
                v
            }
        }
    };

    // ── The repaired graph, written and read back ───────────────────────
    let out_json = match &opts.out {
        None => Value::Null,
        Some(path) => write_out(graph, &frag, &removed, path)?,
    };

    // ── Scope ───────────────────────────────────────────────────────────
    let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    for a in &frag.axioms {
        *by_kind.entry(a.kind.name()).or_default() += 1;
    }
    let not_considered_total: usize = r.not_considered.values().sum();
    let triples_total: usize = r.buckets.values().sum();
    let scope = json!({
        "fragment": "atomic named classes: rdfs:subClassOf, owl:disjointWith and \
                     owl:equivalentClass between two IRIs. Each asserted triple is one axiom, \
                     and owl:equivalentClass is one axiom read as two subclass edges",
        "incoherent_means": "the class reaches X and Y through rdfs:subClassOf* and X \
                             owl:disjointWith Y (or the reverse) is asserted, X = Y and the class \
                             itself included. The same judgement as the onto_defects kinds \
                             disjoint_with_ancestor and inherited_disjoint, plus a class asserted \
                             disjoint with itself, which onto_defects does not look for",
        "coherence_judged_in_this_fragment_only": true,
        "means": "a class coherent here can still have no instances under the whole ontology: \
                  through a class expression, owl:AllDisjointClasses, owl:disjointUnionOf, \
                  owl:complementOf or a property axiom, none of which was read. They are counted \
                  under not_considered so that their number is never hidden",
        "axioms_considered": by_kind,
        "axioms_considered_total": n_axioms,
        "not_considered": r.not_considered,
        "not_considered_total": not_considered_total,
        "owl_nothing_read_as_an_ordinary_class": r.into_nothing,
        "triples": r.buckets,
        "triples_total": triples_total,
        "inferred_graph_ignored": true,
    });

    let classes = |set: &mut dyn Iterator<Item = u32>| -> Value {
        let all: Vec<&str> = set.map(|c| bare(&frag.classes[c as usize])).collect();
        json!({
            "count": all.len(),
            "classes": all.iter().take(MAX_LISTED).collect::<Vec<_>>(),
            "truncated": all.len() > MAX_LISTED,
        })
    };

    let lower_bound_real = lb_scaled as f64 / scale as f64;
    let mut basis = if optimal && conflicts.is_empty() && hard_inc.is_empty() {
        "the fragment was already coherent, so the empty repair is optimal".to_string()
    } else if optimal && conflicts.is_empty() {
        "no class outside the unrepairable ones was incoherent, so the empty repair is optimal"
            .to_string()
    } else if optimal {
        "the branch and bound over every collected conflict closed, so no cheaper set of soft \
         axioms hits them all; every repair has to hit every collected conflict, so that minimum \
         is a lower bound on every repair, and this repair meets it and was re-run coherent"
            .to_string()
    } else if bound_fired == Some("max_nodes") {
        format!(
            "max_nodes ({}) fired inside the last hitting-set search, so the repair is the best \
             found and lower_bound is the best bound proved. Raise max_nodes to continue",
            opts.max_nodes
        )
    } else {
        format!(
            "max_rounds ({}) fired before the loop closed; the repair was completed greedily, \
             one cheapest axiom per remaining conflict, and lower_bound is the best bound proved. \
             Raise max_rounds to continue",
            opts.max_rounds
        )
    };
    if !r.scaling_exact && optimal && !conflicts.is_empty() {
        let soft_items = last.as_ref().map(|(p, _)| p.items.len()).unwrap_or(0);
        basis.push_str(&format!(
            ". Optimal for the scaled integer costs; costs were rounded to 1/{scale}, so a repair \
             better in the real-valued objective could exist only within {} of this one",
            soft_items as f64 / scale as f64
        ));
    }

    let weights_means = match opts.weights {
        WeightKind::Cost => "a weight is what dropping the axiom costs, and the repair minimises \
                             their sum over the dropped axioms",
        WeightKind::Probability => "a weight is the probability that the axiom is right. Under \
                                    independence the most probable coherent subset minimises the \
                                    sum over the DROPPED axioms of ln(p / (1 - p)), so that is the \
                                    cost. p = 1 is hard and never dropped; p <= 0.5 costs nothing \
                                    to drop and is dropped whenever it is in a collected conflict",
    };

    let mut out = json!({
        "weights": opts.weights.name(),
        "weights_means": weights_means,
        "weight_property": opts.weight_property.as_deref().map(|w| bare(w.trim()).to_string()),
        "default_weight": opts.default_weight,
        "weight_annotations_used": r.weight_annotations_used,
        "weight_annotations_not_used": r.weight_annotations_unused,
        "incoherent_before": classes(&mut before.iter().copied()),
        "dropped": dropped,
        "dropped_count": removed.len(),
        "total_weight": total_real,
        "total_cost_scaled": total_scaled,
        "cost_scale": scale,
        "cost_scaling_exact": r.scaling_exact,
        "classes_made_coherent": classes(&mut made_coherent.iter().copied()),
        "still_incoherent": classes(&mut after.iter().copied()),
        "unrepairable": {
            "count": hard_inc.len(),
            "conflicts": unrepairable,
            "means": "classes incoherent under the HARD axioms alone (p = 1), so no set of soft \
                      axioms can make them coherent. Each root is listed with one conflict made \
                      only of hard axioms; their subclasses follow from them",
        },
        "conflicts": conflicts_json,
        "conflict_count": conflicts.len(),
        "conflicts_mean": "each conflict is a set of asserted axioms over which its class is \
                           incoherent, minimal for that class. Any repair has to drop at least one \
                           of them",
        "optimal": optimal,
        "optimality": {
            "basis": basis,
            "relies_on": "monotonicity: in this fragment removing an axiom can only remove subclass \
                          paths and disjointness pairs, so a set of axioms over which a class is \
                          incoherent leaves it incoherent in every superset",
            "bound_fired": bound_fired,
            "rounds": rounds,
            "exact_hitting_set_searches": exact_solves,
            "branch_and_bound_nodes": nodes,
            "max_nodes": opts.max_nodes,
            "max_rounds": opts.max_rounds,
            "algorithm": "implicit hitting sets (Davies and Bacchus, CP 2011): a hitting set of the \
                          conflicts collected so far, remove it, re-check, collect a conflict the \
                          removal missed, repeat. Rounds use a greedy hitting set; once a greedy \
                          repair is coherent the exact minimum-cost one is computed, and only a \
                          coherent EXACT one ends the loop as optimal",
        },
        "lower_bound": lower_bound_real,
        "lower_bound_scaled": lb_scaled,
        "coherence_after": {
            "re_run": true,
            "incoherent_classes": after.len(),
            "incoherent_classes_a_soft_axiom_could_fix": fixable_left.len(),
            "verdict": if fixable_left.is_empty() && hard_inc.is_empty() {
                "coherent_in_this_fragment"
            } else if fixable_left.is_empty() {
                "coherent_in_this_fragment_except_the_unrepairable_classes"
            } else {
                "STILL_INCOHERENT_THIS_IS_A_DEFECT"
            },
        },
        "certificate": certificate,
        "claims": {
            "conflicts": "re_run: each conflict was checked over exactly its own axioms, where its \
                          class is incoherent, and without each one of them, where it is not",
            "coherence": "re_run: the repaired fragment was checked again from scratch",
            "optimality": if optimal { "searched: branch and bound closed over every collected \
                                        conflict; with certificate_dir, refuted by oo-lrat" }
                          else { "NOT established: a bound fired" },
        },
        "conflict_checks_run": checks,
        "conflicts_verified": conflict_failures.is_empty(),
        "scope": scope,
    });
    if opts.weights == WeightKind::Probability {
        let in_conflict: HashSet<u32> =
            conflicts.iter().flat_map(|c| c.axioms.iter().copied()).collect();
        let kept_low = r.below_half.iter().filter(|&&a| !in_conflict.contains(&(a as u32))).count();
        out["dropped_at_no_cost"] = json!(free_dropped);
        out["low_probability_axioms_kept"] = json!({
            "count": kept_low,
            "means": "axioms with p < 0.5 in no collected conflict. The unclamped most probable \
                      world would drop them too; a repair removes only what coherence needs, so \
                      they are kept",
        });
    }
    if !conflict_failures.is_empty() {
        out["conflict_failures"] = json!(conflict_failures);
        out["conflict_failures_mean"] = json!(
            "a conflict failed its own re-run. That is a defect in this module, it is reported \
             rather than hidden, and the repair above must not be trusted. Please report it"
        );
    }
    if greedy_completion > 0 {
        out["greedy_completion_steps"] = json!(greedy_completion);
    }
    if !out_json.is_null() {
        out["out"] = out_json;
    }
    if stop_the_line > 0 {
        out["stop_the_line"] = json!(stop_the_line);
        out["exit_code"] = json!(1);
    }
    Ok(out)
}

fn certify(
    frag: &Fragment,
    problem: &HsProblem,
    w_star: i64,
    dir: &Path,
    scale: i64,
) -> anyhow::Result<(Value, u64)> {
    let bound = w_star - 1;
    let encoded = on_big_stack(|| encode(problem, bound));
    let (clauses, variables, bdd_nodes) = match encoded {
        Ok(e) => e,
        Err(reason) => return Ok((json!({"status": "undetermined", "reason": reason}), 0)),
    };
    std::fs::create_dir_all(dir)?;
    let mut table = String::from("variable\tcost_scaled\tsubject\tpredicate\tobject\n");
    for (i, &a) in problem.items.iter().enumerate() {
        let ax = &frag.axioms[a as usize];
        table.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            i + 1,
            problem.costs[i],
            bare(&ax.s),
            bare(ax.kind.spelled()),
            bare(&ax.o)
        ));
    }
    std::fs::write(dir.join("variables.tsv"), table)?;
    let encoding = json!({
        "variables": variables,
        "axiom_variables": problem.items.len(),
        "clauses": clauses.len(),
        "conflict_clauses": problem.sets.len(),
        "cost_bound_encoding": "reduced BDD (Abio et al., JAIR 2012), two clauses per node",
        "bdd_nodes": bdd_nodes,
    });
    let statement = format!(
        "no set of soft axioms with scaled integer cost at most {bound} hits every collected \
         conflict (costs scaled by {scale}; the repair costs {w_star})"
    );
    let not_checked = "that the CNF says the statement is the encoder's claim, tested by brute \
                       force and not machine-checked. What the theorem covers is that this CNF \
                       has no model";
    let check_with = format!(
        "oo-lrat {} {}",
        dir.join("problem.cnf").display(),
        dir.join("problem.lrat").display()
    );
    let v = match crate::sat::refute(&clauses, dir)? {
        SatOutcome::Refuted(cert) => (
            json!({
                "status": "refuted",
                "theorem": cert.theorem(),
                "subject_sha256": cert.subject_sha256(),
                "statement": statement,
                "scaled_integer_costs": true,
                "cost_scale": scale,
                "bound_refuted": bound,
                "dir": dir.display().to_string(),
                "files": ["problem.cnf", "problem.lrat", "variables.tsv"],
                "check_with": check_with,
                "encoding": encoding,
                "not_machine_checked": not_checked,
            }),
            0,
        ),
        SatOutcome::Undetermined(reason) => (
            json!({"status": "undetermined", "reason": reason, "statement": statement,
                   "encoding": encoding}),
            0,
        ),
        SatOutcome::Satisfiable => (
            json!({
                "status": "solver_found_a_cheaper_hitting_set",
                "statement": statement,
                "means": "the branch and bound said no set of soft axioms costing at most the \
                          bound hits every conflict, and the SAT solver found one. One of the two \
                          is wrong, this is a defect, and the optimality claim above must not be \
                          trusted. Please report it",
                "encoding": encoding,
            }),
            1,
        ),
    };
    Ok(v)
}

/// Write the graph without the dropped axioms, and without the reification
/// nodes that annotated them, since an annotation of an axiom no longer
/// asserted is an annotation about nothing. Then read the file back and check
/// it again, with this module and with `defects`.
fn write_out(
    graph: &GraphStore,
    frag: &Fragment,
    removed: &BTreeSet<u32>,
    path: &Path,
) -> anyhow::Result<Value> {
    let inferred = format!("<{}>", crate::reason::INFERRED_GRAPH);
    let gone: HashSet<(&str, &str, &str)> = removed
        .iter()
        .map(|&a| {
            let ax = &frag.axioms[a as usize];
            (ax.s.as_str(), ax.kind.spelled(), ax.o.as_str())
        })
        .collect();
    let quads = graph.all_quads()?;
    let mut src: HashMap<&str, &str> = HashMap::new();
    let mut prop: HashMap<&str, &str> = HashMap::new();
    let mut tgt: HashMap<&str, &str> = HashMap::new();
    for (s, p, o, _) in &quads {
        match p.as_str() {
            ANNOTATED_SOURCE => {
                src.insert(s.as_str(), o.as_str());
            }
            ANNOTATED_PROPERTY => {
                prop.insert(s.as_str(), o.as_str());
            }
            ANNOTATED_TARGET => {
                tgt.insert(s.as_str(), o.as_str());
            }
            _ => {}
        }
    }
    let dead: HashSet<&str> = src
        .iter()
        .filter_map(|(n, s)| {
            let (p, o) = (prop.get(n)?, tgt.get(n)?);
            gone.contains(&(*s, *p, *o)).then_some(*n)
        })
        .collect();
    let mut nq = String::new();
    let (mut axioms_removed, mut annotations_removed, mut written) = (0usize, 0usize, 0usize);
    let mut removed_seen: HashSet<(&str, &str, &str)> = HashSet::new();
    for (s, p, o, g) in &quads {
        if *g == inferred {
            continue;
        }
        if gone.contains(&(s.as_str(), p.as_str(), o.as_str())) {
            if removed_seen.insert((s.as_str(), p.as_str(), o.as_str())) {
                axioms_removed += 1;
            }
            continue;
        }
        if dead.contains(s.as_str()) {
            annotations_removed += 1;
            continue;
        }
        nq.push_str(&format!("{s} {p} {o} {g} .\n"));
        written += 1;
    }
    let repaired = GraphStore::new();
    repaired.load_nquads(&nq)?;
    let ttl = repaired.serialize("turtle")?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, &ttl)?;

    // Read back what was written, not what was meant to be written.
    let back = Arc::new(GraphStore::new());
    back.load_turtle(&std::fs::read_to_string(path)?, None)?;
    let reread = read(&back, &RepairOptions::default())?;
    let refrag = Fragment::new(reread.axioms);
    let n = refrag.axioms.len();
    let incoherent = refrag.incoherent(&vec![true; n]);
    let defect_errors = crate::defects::Defects::findings(&back)?
        .iter()
        .filter(|f| f.kind == "disjoint_with_ancestor" || f.kind == "inherited_disjoint")
        .count();
    Ok(json!({
        "path": path.display().to_string(),
        "format": "turtle",
        "quads_written": written,
        "axioms_removed": axioms_removed,
        "annotation_triples_removed": annotations_removed,
        "named_graphs_flattened": true,
        "rechecked": {
            "re_run": true,
            "incoherent_classes_in_this_fragment": incoherent.len(),
            "defects_findings_of_the_fragment_kinds": defect_errors,
            "means": "the written file was read back and checked again, by this module and by \
                      onto_defects (disjoint_with_ancestor and inherited_disjoint). Both counts \
                      are zero unless a class is unrepairable",
        },
    }))
}

// ───────────────────────────────────────────────────────────────────────────
// Human rendering
// ───────────────────────────────────────────────────────────────────────────

fn short(iri: &str) -> String {
    for (ns, prefix) in [(OWL_NS, "owl:"), (RDFS_NS, "rdfs:"), (RDF_NS, "rdf:")] {
        if let Some(local) = iri.strip_prefix(ns) {
            return format!("{prefix}{local}");
        }
    }
    iri.to_string()
}

fn axiom_line(a: &Value) -> String {
    format!(
        "{} {} {}",
        short(a["s"].as_str().unwrap_or("?")),
        short(a["p"].as_str().unwrap_or("?")),
        short(a["o"].as_str().unwrap_or("?"))
    )
}

/// The report as text, for `--human`. Not wired into `crate::output` here:
/// register it there as the renderer for the `repair` command.
pub fn render_human(value: &Value) -> String {
    use std::fmt::Write;
    if let Some(e) = value.get("error").and_then(Value::as_str) {
        return format!("Error: {e}");
    }
    let mut s = String::new();
    let kind = value["weights"].as_str().unwrap_or("?");
    let n = value["dropped_count"].as_u64().unwrap_or(0);
    let total = value["total_weight"].as_f64().unwrap_or(0.0);
    let _ = writeln!(
        s,
        "Weighted repair ({kind} weights): drop {n} axiom{}, total weight {total}.",
        if n == 1 { "" } else { "s" }
    );
    let _ = writeln!(
        s,
        "Classes incoherent before: {}. Made coherent: {}. Unrepairable within the soft axioms: {}.",
        value["incoherent_before"]["count"].as_u64().unwrap_or(0),
        value["classes_made_coherent"]["count"].as_u64().unwrap_or(0),
        value["unrepairable"]["count"].as_u64().unwrap_or(0),
    );
    if value["optimal"].as_bool() == Some(true) {
        let _ = writeln!(
            s,
            "Optimal: yes, because {}.",
            value["optimality"]["basis"].as_str().unwrap_or("")
        );
    } else {
        let _ = writeln!(
            s,
            "Optimal: NOT established ({} fired). Lower bound {}.",
            value["optimality"]["bound_fired"].as_str().unwrap_or("a bound"),
            value["lower_bound"].as_f64().unwrap_or(0.0)
        );
    }
    let cert = &value["certificate"];
    match cert["status"].as_str() {
        Some("refuted") => {
            let _ = writeln!(
                s,
                "Certificate: refuted by oo-lrat ({}): {}.",
                cert["theorem"].as_str().unwrap_or("?"),
                cert["statement"].as_str().unwrap_or("")
            );
        }
        Some("not_requested") | None => {}
        Some(other) => {
            let why = cert["reason"].as_str().or(cert["means"].as_str()).unwrap_or("");
            let _ = writeln!(s, "Certificate: {other}. {why}");
        }
    }
    if let Some(d) = value["dropped"].as_array()
        && !d.is_empty()
    {
        let _ = writeln!(s, "Dropped:");
        for x in d {
            let hits: Vec<String> = x["hits"]
                .as_array()
                .map(|h| h.iter().map(|v| format!("#{v}")).collect())
                .unwrap_or_default();
            let _ = writeln!(
                s,
                "  {}  weight {}  hits {}{}",
                axiom_line(&x["axiom"]),
                x["weight_as_written"].as_str().unwrap_or("?"),
                if hits.is_empty() { "nothing".to_string() } else { hits.join(", ") },
                if x["reason"] == "probability_at_most_one_half_dropped_at_no_cost" {
                    "  (p <= 0.5, dropped at no cost)"
                } else {
                    ""
                }
            );
        }
    }
    if let Some(c) = value["conflicts"].as_array()
        && !c.is_empty()
    {
        let _ = writeln!(s, "Conflicts (each re-run):");
        for x in c {
            let axioms: Vec<String> = x["axioms"]
                .as_array()
                .map(|a| a.iter().map(axiom_line).collect())
                .unwrap_or_default();
            let _ = writeln!(
                s,
                "  #{} {}: {}",
                x["id"],
                short(x["class"].as_str().unwrap_or("?")),
                axioms.join("; ")
            );
        }
    }
    if let Some(u) = value["unrepairable"]["conflicts"].as_array()
        && !u.is_empty()
    {
        let _ = writeln!(s, "Unrepairable (hard axioms only):");
        for x in u {
            let axioms: Vec<String> = x["axioms"]
                .as_array()
                .map(|a| a.iter().map(axiom_line).collect())
                .unwrap_or_default();
            let _ = writeln!(s, "  {}: {}", short(x["class"].as_str().unwrap_or("?")), axioms.join("; "));
        }
    }
    if let Some(o) = value.get("out").filter(|o| !o.is_null()) {
        let _ = writeln!(
            s,
            "Wrote {} ({} axioms removed); read back, {} incoherent class(es) in this fragment.",
            o["path"].as_str().unwrap_or("?"),
            o["axioms_removed"],
            o["rechecked"]["incoherent_classes_in_this_fragment"]
        );
    }
    let scope = &value["scope"];
    let _ = write!(
        s,
        "Scope: coherence judged in the atomic named-class fragment only; {} axioms considered, \
         {} of other kinds not considered.",
        scope["axioms_considered_total"].as_u64().unwrap_or(0),
        scope["not_considered_total"].as_u64().unwrap_or(0)
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem(costs: Vec<i64>, sets: Vec<Vec<usize>>) -> HsProblem {
        let sets = sets
            .into_iter()
            .map(|mut s| {
                s.sort_by_key(|&i| (costs[i], i));
                s
            })
            .collect();
        HsProblem {
            items: (0..costs.len() as u32).collect(),
            costs,
            sets,
            forced: BTreeSet::new(),
        }
    }

    fn brute(p: &HsProblem) -> i64 {
        let m = p.items.len();
        (0u32..1 << m)
            .filter(|mask| p.sets.iter().all(|s| s.iter().any(|&e| mask & (1 << e) != 0)))
            .map(|mask| (0..m).filter(|&e| mask & (1 << e) != 0).map(|e| p.costs[e]).sum())
            .min()
            .unwrap_or(0)
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }

    #[test]
    fn the_branch_and_bound_matches_brute_force() {
        let mut rng = Rng(0x9e3779b97f4a7c15);
        for _ in 0..300 {
            let m = 1 + (rng.next() % 9) as usize;
            let costs: Vec<i64> = (0..m).map(|_| (rng.next() % 7) as i64).collect();
            let k = 1 + (rng.next() % 8) as usize;
            let sets: Vec<Vec<usize>> = (0..k)
                .map(|_| {
                    let mut s: BTreeSet<usize> = BTreeSet::new();
                    let len = 1 + (rng.next() % 4) as usize;
                    for _ in 0..len {
                        s.insert((rng.next() % m as u64) as usize);
                    }
                    s.into_iter().collect()
                })
                .collect();
            let p = problem(costs, sets);
            let sol = solve(&p, 1_000_000);
            assert!(sol.closed);
            assert_eq!(sol.cost, brute(&p), "costs {:?} sets {:?}", p.costs, p.sets);
            assert!(p.sets.iter().all(|s| s.iter().any(|e| sol.chosen.contains(e))));
        }
    }

    #[test]
    fn a_budget_that_fires_says_so_and_keeps_a_valid_hitting_set() {
        // A triangle: every pair of sets overlaps, so neither root bound
        // reaches the optimum of two items (6) and the root has to branch.
        // Disjoint sets would not do: their bound is tight and the search
        // closes at the root without spending a second node.
        let p = problem(vec![3, 3, 3], vec![vec![0, 1], vec![1, 2], vec![0, 2]]);
        assert!(solve(&p, 1_000).closed, "with room, the same instance closes");
        let sol = solve(&p, 1);
        assert!(!sol.closed);
        // Packing finds 3; apportioning gives 3 x 3/2 = 4.5, rounded up to 5.
        assert_eq!(sol.lower_bound, 5);
        assert_eq!(sol.cost, 6);
        assert!(p.sets.iter().all(|s| s.iter().any(|e| sol.chosen.contains(e))));
        assert!(sol.lower_bound <= sol.cost);
    }

    /// Follow the BDD along an assignment.
    fn eval(bdd: &Bdd, root: Node, x: &[bool]) -> bool {
        let mut n = root;
        loop {
            match n {
                Node::True => return true,
                Node::False => return false,
                Node::At(i) => {
                    let (level, hi, lo) = bdd.nodes[i];
                    n = if x[level] { hi } else { lo };
                }
            }
        }
    }

    #[test]
    fn the_bdd_is_the_cost_bound() {
        let mut rng = Rng(42);
        for _ in 0..300 {
            let n = 1 + (rng.next() % 8) as usize;
            let mut coeffs: Vec<i64> = (0..n).map(|_| 1 + (rng.next() % 20) as i64).collect();
            coeffs.sort_by_key(|&c| std::cmp::Reverse(c));
            let total: i64 = coeffs.iter().sum();
            let k = (rng.next() % (total as u64 + 2)) as i64 - 1;
            let mut bdd = Bdd::new(coeffs.clone());
            let (_, root) = bdd.build(0, k);
            for mask in 0u32..1 << n {
                let x: Vec<bool> = (0..n).map(|i| mask & (1 << i) != 0).collect();
                let sum: i64 = (0..n).filter(|&i| x[i]).map(|i| coeffs[i]).sum();
                assert_eq!(eval(&bdd, root, &x), sum <= k, "coeffs {coeffs:?} k {k} x {x:?}");
            }
        }
    }

    /// Every assignment to every variable, for formulas small enough.
    fn satisfiable(clauses: &[Clause], vars: usize, fixed: &[(usize, bool)]) -> bool {
        let free: Vec<usize> = (1..=vars).filter(|v| !fixed.iter().any(|f| f.0 == *v)).collect();
        (0u64..1 << free.len()).any(|mask| {
            let mut val = vec![false; vars + 1];
            for &(v, b) in fixed {
                val[v] = b;
            }
            for (i, &v) in free.iter().enumerate() {
                val[v] = mask & (1 << i) != 0;
            }
            clauses.iter().all(|c| {
                c.iter().any(|&l| if l > 0 { val[l as usize] } else { !val[(-l) as usize] })
            })
        })
    }

    #[test]
    fn the_cnf_has_a_model_exactly_when_a_cheap_enough_hitting_set_exists() {
        let mut rng = Rng(7);
        let mut checked = 0;
        for _ in 0..200 {
            let m = 1 + (rng.next() % 4) as usize;
            let costs: Vec<i64> = (0..m).map(|_| (rng.next() % 6) as i64).collect();
            let k = 1 + (rng.next() % 3) as usize;
            let sets: Vec<Vec<usize>> = (0..k)
                .map(|_| {
                    let mut s: BTreeSet<usize> = BTreeSet::new();
                    for _ in 0..1 + rng.next() % 3 {
                        s.insert((rng.next() % m as u64) as usize);
                    }
                    s.into_iter().collect()
                })
                .collect();
            let p = problem(costs, sets);
            let opt = brute(&p);
            for bound in [opt - 1, opt] {
                let Ok((clauses, vars, _)) = encode(&p, bound) else { continue };
                if vars > 10 {
                    continue;
                }
                checked += 1;
                // Projected onto the axiom variables, a model is a hitting
                // set of cost at most `bound`, and there is one exactly when
                // the optimum is at most `bound`.
                assert_eq!(
                    satisfiable(&clauses, vars, &[]),
                    opt <= bound,
                    "costs {:?} sets {:?} bound {bound}",
                    p.costs,
                    p.sets
                );
                for mask in 0u32..1 << m {
                    let fixed: Vec<(usize, bool)> =
                        (0..m).map(|e| (e + 1, mask & (1 << e) != 0)).collect();
                    let hits = p.sets.iter().all(|s| s.iter().any(|&e| mask & (1 << e) != 0));
                    let cost: i64 = (0..m).filter(|&e| mask & (1 << e) != 0).map(|e| p.costs[e]).sum();
                    assert_eq!(satisfiable(&clauses, vars, &fixed), hits && cost <= bound);
                }
            }
        }
        assert!(checked > 20, "only {checked} encodings were checked");
    }

    #[test]
    fn a_literal_is_read_by_its_lexical_form() {
        assert_eq!(lexical("\"0.62\"^^<http://www.w3.org/2001/XMLSchema#decimal>"), Some("0.62"));
        assert_eq!(lexical("\"1\""), Some("1"));
        assert_eq!(lexical("<http://example.org/x>"), None);
        assert_eq!(decimals("0.625"), Some(3));
        assert_eq!(decimals("6.2E-1"), None);
        assert_eq!(decimals("2"), Some(0));
    }
}
