//! OWL 2 EL classification by completion (#30, #324).
//!
//! The axioms are read by the tableau's own OWL parser
//! ([`crate::tableaux::parse_owl`]), normalised into the four EL normal forms
//! (`A ⊑ B`, `A₁ ⊓ A₂ ⊑ B`, `A ⊑ ∃r.B`, `∃r.A ⊑ B`, with `B` possibly `⊥`),
//! and saturated with the completion rules of Baader, Brandt and Lutz,
//! *Pushing the EL Envelope* (IJCAI 2005): CR1-CR4 for told subsumption,
//! conjunction and existentials, ⊥-propagation, the role hierarchy, and
//! transitive roles as the chain `r ∘ r ⊑ r`. Ranges are compiled into the
//! existential's filler (`A ⊑ ∃r.B` becomes `A ⊑ ∃r.(B ⊓ ran(r))`), which is the
//! treatment in *Pushing the EL Envelope Further* (OWLED 2008).
//!
//! ## What it guarantees
//!
//! Every subsumption it reports is entailed: each normalised axiom is entailed
//! by the original or is a definition of a fresh name, and an axiom it cannot
//! normalise is dropped, which only loses entailments. The table between named
//! classes is COMPLETE, and the report says `complete: true`, only when nothing
//! was dropped: no axiom outside EL, no inverse or functional role, and none of
//! the constructs the parser does not read at all (`owl:propertyChainAxiom`,
//! nominals, datatype restrictions, and the rest of
//! [`DlReasoner::unmodelled_constructs`]). Otherwise `complete` is false and
//! `incompleteness` names what was set aside.
//!
//! ## Why it is not OWL-RL any more
//!
//! Until #324 this module ran the `owl-rl-ext` rules and read back
//! `rdfs:subClassOf`, under a header saying that was "sufficient for the EL
//! fragment". It is not: RL derives through a left-hand conjunction or
//! existential only for an individual (`cls-int1`, `cls-svf1`), never at TBox
//! level, so `A ⊑ B, A ⊑ C, B ⊓ C ⊑ D` came back without `A ⊑ D`, and so did
//! `A ⊑ ∃r.B, B ⊑ C, ∃r.C ⊑ D`. The read-back also stopped at 100,000 rows
//! without saying so. The RL rules still run, but only on input outside EL,
//! where they can add sound subsumptions the EL fragment does not see (through
//! `owl:allValuesFrom`, for instance); their answer is unioned in, read from the
//! store directly with no row cap.
//!
//! For a complete classification of input outside EL, use `onto_reason` with
//! `owl-dl`, which reports its own `complete` flag.

use crate::graph::GraphStore;
use crate::reason::Reasoner;
use crate::tableaux::{Concept, DlReasoner, ParseResult, parse_owl};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

const RDFS_SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const OWL_THING: &str = "<http://www.w3.org/2002/07/owl#Thing>";

#[derive(Clone, Debug, Serialize)]
pub struct ClassificationReport {
    /// `el-completion` when the EL saturation alone produced the table,
    /// `el-completion+owl-rl-ext` when the RL rules were unioned in because the
    /// input is outside EL.
    pub profile: String,
    /// True only when the table is every subsumption between named classes
    /// that the ontology entails (see the module docs for the conditions).
    pub complete: bool,
    /// Why `complete` is false, one entry per cause. Empty when complete.
    pub incompleteness: Vec<String>,
    /// False when the saturation derived `⊤ ⊑ ⊥` or an individual in `⊥`. An
    /// inconsistent ontology entails every subsumption, and the table lists
    /// only those derived, so `complete` is then false.
    pub consistent: bool,
    /// Always false: there is no row cap. Kept so a consumer can check for a
    /// cut-off table instead of assuming there is none; the old read-back
    /// stopped at 100,000 rows and had no way to say so.
    pub truncated: bool,
    /// Named classes classified, `owl:Thing` excluded.
    pub class_count: usize,
    pub subsumption_count: usize,
    /// `[sub, super]` pairs between named classes, `owl:Thing` and `X ⊑ X`
    /// left out. Sorted by sub then super for stable output. An unsatisfiable
    /// class is under every class and its pairs are not listed; it is in
    /// `unsatisfiable` instead, as HermiT reports it.
    pub subsumptions: Vec<(String, String)>,
    pub unsatisfiable: Vec<String>,
}

/// Node 0 is `⊤` and node 1 is `⊥` in every normalised TBox.
const TOP: usize = 0;
const BOTTOM: usize = 1;

/// The construct that keeps a concept out of EL on the side it occurs.
fn construct(c: &Concept) -> &'static str {
    match c {
        Concept::ForAll(..) => "owl:allValuesFrom",
        Concept::Or(..) => "owl:unionOf",
        Concept::NegAtom(..) => "owl:complementOf",
        Concept::MaxCard(..) => "max cardinality",
        Concept::MinCard(..) => "min cardinality above 1",
        _ => "a class expression",
    }
}

/// The first sub-expression that cannot stand on the LEFT of an EL axiom.
fn lhs_offender(c: &Concept) -> Option<&'static str> {
    match c {
        Concept::Top | Concept::Bottom | Concept::Atom(_) | Concept::MinCard(_, 0, _) => None,
        Concept::And(cs) | Concept::Or(cs) => cs.iter().find_map(lhs_offender),
        Concept::Exists(_, f) | Concept::MinCard(_, 1, f) => lhs_offender(f),
        other => Some(construct(other)),
    }
}

/// Whether a concept can stand on the RIGHT, possibly after the Horn rewrite
/// `X ⊑ ¬N ⊔ D` into `X ⊓ N ⊑ D` (`add_horn`).
fn rhs_el(c: &Concept) -> bool {
    match c {
        Concept::Top | Concept::Bottom | Concept::Atom(_) | Concept::MinCard(_, 0, _) => true,
        Concept::And(cs) => cs.iter().all(rhs_el),
        Concept::Exists(_, f) | Concept::MinCard(_, _, f) => rhs_el(f),
        other => horn_split(other).is_some(),
    }
}

/// `D₁ ⊔ … ⊔ Dₙ` as (the negated disjuncts that are EL left-hand sides, the
/// one remaining positive disjunct), or `None` when more than one disjunct is
/// positive or the positive one is not EL. NNF writes `A ⊑ ¬B` and `A ⊑ ¬∃r.B`
/// as `¬B` and `∀r.¬B`, so disjointness reaches this module in that form and
/// has to be read back out of it.
fn horn_split(c: &Concept) -> Option<(Vec<Concept>, Option<&Concept>)> {
    let disjuncts: Vec<&Concept> = match c {
        Concept::Or(cs) => cs.iter().collect(),
        other => vec![other],
    };
    let mut negs = Vec::new();
    let mut pos = None;
    for d in disjuncts {
        // `x ⊑ D ⊔ R` is `x ⊓ ¬D ⊑ R` exactly, so any disjunct whose
        // negation is an EL left-hand side can move across.
        let n = d.negate();
        if lhs_offender(&n).is_none() {
            negs.push(n);
        // The positive disjunct must be one `add_rhs` takes directly: the four
        // forms excluded here are the ones it sends back to this function.
        } else if pos.is_none()
            && !matches!(d, Concept::Or(_) | Concept::ForAll(..) | Concept::NegAtom(_) | Concept::MaxCard(..))
            && rhs_el(d)
        {
            pos = Some(d);
        } else {
            return None;
        }
    }
    Some((negs, pos))
}

/// A TBox in EL normal form over dense node ids.
struct Normalised {
    told: Vec<Vec<usize>>,
    /// `conj[a]` holds `(b, c)` for every `a ⊓ b ⊑ c`, stored under both conjuncts.
    conj: Vec<Vec<(usize, usize)>>,
    /// `ex_rhs[a]` holds `(r, b)` for every `a ⊑ ∃r.b`.
    ex_rhs: Vec<Vec<(u32, usize)>>,
    /// `∃r.a ⊑ b`, keyed by `(r, a)` and by `r`.
    ex_lhs: HashMap<(u32, usize), Vec<usize>>,
    ex_lhs_by_role: HashMap<u32, Vec<(usize, usize)>>,
    ranges: HashMap<u32, Vec<usize>>,
    atom_node: HashMap<u32, usize>,
    lhs_cache: HashMap<Concept, usize>,
    rhs_cache: HashMap<Concept, usize>,
    thing: u32,
    nothing: u32,
    /// Constructs set aside, with how many axioms (or parts of one) each cost.
    dropped: BTreeMap<String, usize>,
}

impl Normalised {
    fn new(thing: u32, nothing: u32) -> Self {
        let mut n = Self {
            told: Vec::new(),
            conj: Vec::new(),
            ex_rhs: Vec::new(),
            ex_lhs: HashMap::new(),
            ex_lhs_by_role: HashMap::new(),
            ranges: HashMap::new(),
            atom_node: HashMap::new(),
            lhs_cache: HashMap::new(),
            rhs_cache: HashMap::new(),
            thing,
            nothing,
            dropped: BTreeMap::new(),
        };
        n.fresh();
        n.fresh();
        n
    }

    fn len(&self) -> usize {
        self.told.len()
    }

    fn fresh(&mut self) -> usize {
        self.told.push(Vec::new());
        self.conj.push(Vec::new());
        self.ex_rhs.push(Vec::new());
        self.told.len() - 1
    }

    fn atom(&mut self, a: u32) -> usize {
        if a == self.thing {
            return TOP;
        }
        if a == self.nothing {
            return BOTTOM;
        }
        if let Some(&n) = self.atom_node.get(&a) {
            return n;
        }
        let n = self.fresh();
        self.atom_node.insert(a, n);
        n
    }

    fn drop_part(&mut self, why: String) {
        *self.dropped.entry(why).or_default() += 1;
    }

    fn add_conj(&mut self, a: usize, b: usize, c: usize) {
        self.conj[a].push((b, c));
        if a != b {
            self.conj[b].push((a, c));
        }
    }

    fn add_ex_lhs(&mut self, r: u32, a: usize, b: usize) {
        self.ex_lhs.entry((r, a)).or_default().push(b);
        self.ex_lhs_by_role.entry(r).or_default().push((a, b));
    }

    /// A node `n` with `c ⊑ n`. The caller has checked `lhs_offender(c)`.
    fn name_lhs(&mut self, c: &Concept) -> usize {
        match c {
            Concept::Top | Concept::MinCard(_, 0, _) => return TOP,
            Concept::Bottom => return BOTTOM,
            Concept::Atom(a) => return self.atom(*a),
            _ => {}
        }
        if let Some(&n) = self.lhs_cache.get(c) {
            return n;
        }
        let n = match c {
            Concept::And(cs) => {
                let parts: Vec<usize> = cs.iter().map(|p| self.name_lhs(p)).collect();
                let mut acc = parts[0];
                for &p in &parts[1..] {
                    let f = self.fresh();
                    self.add_conj(acc, p, f);
                    acc = f;
                }
                acc
            }
            // `C₁ ⊔ C₂ ⊑ n` is `C₁ ⊑ n` and `C₂ ⊑ n`: two EL axioms, no
            // disjunction left to reason with.
            Concept::Or(cs) => {
                let f = self.fresh();
                for p in cs {
                    let pn = self.name_lhs(p);
                    self.told[pn].push(f);
                }
                f
            }
            Concept::Exists(r, filler) | Concept::MinCard(r, _, filler) => {
                let fn_ = self.name_lhs(filler);
                let f = self.fresh();
                self.add_ex_lhs(*r, fn_, f);
                f
            }
            other => unreachable!("lhs_offender admitted {other:?}"),
        };
        self.lhs_cache.insert(c.clone(), n);
        n
    }

    /// A node `n` with `n ⊑ c`, as far as `c` is EL.
    fn name_rhs(&mut self, c: &Concept) -> usize {
        match c {
            Concept::Top => return TOP,
            Concept::Bottom => return BOTTOM,
            Concept::Atom(a) => return self.atom(*a),
            _ => {}
        }
        if let Some(&n) = self.rhs_cache.get(c) {
            return n;
        }
        let n = self.fresh();
        self.rhs_cache.insert(c.clone(), n);
        self.add_rhs(n, c);
        n
    }

    /// `x ⊑ c`, keeping every EL part of `c` and dropping the rest. Dropping a
    /// conjunct on the right weakens the axiom, which is sound.
    fn add_rhs(&mut self, x: usize, c: &Concept) {
        match c {
            Concept::Top | Concept::MinCard(_, 0, _) => {}
            Concept::Bottom => self.told[x].push(BOTTOM),
            Concept::Atom(a) => {
                let a = self.atom(*a);
                self.told[x].push(a);
            }
            Concept::And(cs) => {
                for p in cs {
                    self.add_rhs(x, p);
                }
            }
            Concept::Exists(r, f) | Concept::MinCard(r, 1, f) => {
                let fnode = self.name_rhs(f);
                self.ex_rhs[x].push((*r, fnode));
            }
            // `≥n r.F` entails `∃r.F`, so reading it as that is sound; what is
            // lost is the count.
            Concept::MinCard(r, _, f) => {
                self.drop_part("min cardinality above 1 on the right-hand side, read as owl:someValuesFrom".into());
                let fnode = self.name_rhs(f);
                self.ex_rhs[x].push((*r, fnode));
            }
            other => self.add_horn(x, other),
        }
    }

    /// `x ⊑ ¬N₁ ⊔ … ⊔ ¬Nₖ ⊔ D` is `x ⊓ N₁ ⊓ … ⊓ Nₖ ⊑ D` (D may be absent,
    /// then `⊥`). Anything with two positive disjuncts is outside EL.
    fn add_horn(&mut self, x: usize, c: &Concept) {
        let Some((negs, pos)) = horn_split(c) else {
            self.drop_part(format!("{} on the right-hand side", construct(c)));
            return;
        };
        let mut acc = x;
        for n in &negs {
            let nn = self.name_lhs(n);
            let f = self.fresh();
            self.add_conj(acc, nn, f);
            acc = f;
        }
        match pos {
            Some(d) => self.add_rhs(acc, d),
            None => self.told[acc].push(BOTTOM),
        }
    }

    fn add_axiom(&mut self, sub: &Concept, sup: &Concept) {
        if let Some(why) = lhs_offender(sub) {
            self.drop_part(format!("{why} on the left-hand side"));
            return;
        }
        let x = self.name_lhs(sub);
        self.add_rhs(x, sup);
    }
}

/// The reflexive-transitive closure of `rdfs:subPropertyOf` from `r`.
fn role_sups(sub_to_super: &HashMap<u32, HashSet<u32>>, r: u32) -> Vec<u32> {
    let mut seen = BTreeSet::from([r]);
    let mut stack = vec![r];
    while let Some(x) = stack.pop() {
        for &y in sub_to_super.get(&x).into_iter().flatten() {
            if seen.insert(y) {
                stack.push(y);
            }
        }
    }
    seen.into_iter().collect()
}

enum Step {
    Add(usize, usize),
    Link(usize, u32, usize),
}

/// The completion sets: `s[x]` is every node `x` is subsumed by, and the
/// `(x, r, y)` edges are the derived `x ⊑ ∃r.y`.
struct Saturation<'a> {
    t: &'a Normalised,
    sups: HashMap<u32, Vec<u32>>,
    sub_to_super: &'a HashMap<u32, HashSet<u32>>,
    transitive: &'a HashSet<u32>,
    s: Vec<HashSet<usize>>,
    context: Vec<bool>,
    edges: HashSet<(usize, u32, usize)>,
    pred: Vec<Vec<(usize, u32)>>,
    succ: Vec<Vec<(u32, usize)>>,
    queue: Vec<Step>,
}

impl<'a> Saturation<'a> {
    fn new(t: &'a Normalised, p: &'a ParseResult) -> Self {
        let n = t.len();
        Self {
            t,
            sups: HashMap::new(),
            sub_to_super: &p.sub_to_super,
            transitive: &p.transitive_roles,
            s: vec![HashSet::new(); n],
            context: vec![false; n],
            edges: HashSet::new(),
            pred: vec![Vec::new(); n],
            succ: vec![Vec::new(); n],
            queue: Vec::new(),
        }
    }

    fn init(&mut self, x: usize) {
        if !self.context[x] {
            self.context[x] = true;
            self.queue.push(Step::Add(x, x));
            self.queue.push(Step::Add(x, TOP));
        }
    }

    fn sups_of(&mut self, r: u32) -> Vec<u32> {
        let sts = self.sub_to_super;
        self.sups.entry(r).or_insert_with(|| role_sups(sts, r)).clone()
    }

    fn run(&mut self) {
        while let Some(step) = self.queue.pop() {
            match step {
                Step::Add(x, a) => self.add(x, a),
                Step::Link(x, r, y) => self.link(x, r, y),
            }
        }
    }

    fn add(&mut self, x: usize, a: usize) {
        if !self.s[x].insert(a) {
            return;
        }
        let t = self.t;
        // CR1: a ⊑ b.
        for &b in &t.told[a] {
            self.queue.push(Step::Add(x, b));
        }
        // CR2: a ⊓ o ⊑ c.
        for &(o, c) in &t.conj[a] {
            if self.s[x].contains(&o) {
                self.queue.push(Step::Add(x, c));
            }
        }
        // CR3: a ⊑ ∃r.b.
        for &(r, b) in &t.ex_rhs[a] {
            self.init(b);
            self.queue.push(Step::Link(x, r, b));
        }
        // CR4 and ⊥: every predecessor of x learns what x's new subsumer gives it.
        for i in 0..self.pred[x].len() {
            let (p, r) = self.pred[x][i];
            if a == BOTTOM {
                self.queue.push(Step::Add(p, BOTTOM));
            }
            for &b in t.ex_lhs.get(&(r, a)).into_iter().flatten() {
                self.queue.push(Step::Add(p, b));
            }
        }
    }

    fn link(&mut self, x: usize, r: u32, y: usize) {
        let t = self.t;
        for s in self.sups_of(r) {
            if !self.edges.insert((x, s, y)) {
                continue;
            }
            self.pred[y].push((x, s));
            self.succ[x].push((s, y));
            if self.s[y].contains(&BOTTOM) {
                self.queue.push(Step::Add(x, BOTTOM));
            }
            // CR4: ∃s.a ⊑ b with a ⊑ y's subsumers; walk whichever side is shorter.
            if let Some(rules) = t.ex_lhs_by_role.get(&s) {
                if rules.len() <= self.s[y].len() {
                    for &(a, b) in rules {
                        if self.s[y].contains(&a) {
                            self.queue.push(Step::Add(x, b));
                        }
                    }
                } else {
                    for &a in &self.s[y] {
                        for &b in t.ex_lhs.get(&(s, a)).into_iter().flatten() {
                            self.queue.push(Step::Add(x, b));
                        }
                    }
                }
            }
            // A transitive role is the chain s ∘ s ⊑ s.
            if self.transitive.contains(&s) {
                for &(s2, z) in &self.succ[y] {
                    if s2 == s {
                        self.queue.push(Step::Link(x, s, z));
                    }
                }
                for &(w, s2) in &self.pred[x] {
                    if s2 == s {
                        self.queue.push(Step::Link(w, s, y));
                    }
                }
            }
        }
    }
}

/// Classify the loaded ontology. Works on a copy of the triples, so the
/// graph is not mutated.
pub fn classify(graph: &Arc<GraphStore>) -> anyhow::Result<ClassificationReport> {
    let triples = graph.all_triples()?;
    let p = parse_owl(triples.clone());
    let mut t = Normalised::new(p.thing_id, p.nothing_id);

    for (sub, sup) in &p.axioms {
        t.add_axiom(sub, sup);
    }
    for (a, b) in &p.disjoint_pairs {
        t.add_axiom(&Concept::And(vec![a.clone(), b.clone()]), &Concept::Bottom);
    }
    for (r, c) in &p.role_domains {
        t.add_axiom(&Concept::Exists(*r, Box::new(Concept::Top)), c);
    }
    for (r, c) in &p.role_ranges {
        let n = t.name_rhs(c);
        t.ranges.entry(*r).or_default().push(n);
    }

    // Ranges into fillers: `a ⊑ ∃r.b` becomes `a ⊑ ∃r.(b ⊓ ran(s))` for every
    // `s ⊒ r`. Done once over the TBox's existentials, which is all of them:
    // the saturation derives edges, never new existential axioms.
    let mut range_of: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut ranged: HashMap<(usize, u32), usize> = HashMap::new();
    for a in 0..t.len() {
        for i in 0..t.ex_rhs[a].len() {
            let (r, b) = t.ex_rhs[a][i];
            let rans = range_of
                .entry(r)
                .or_insert_with(|| {
                    role_sups(&p.sub_to_super, r)
                        .iter()
                        .flat_map(|s| t.ranges.get(s).into_iter().flatten().copied())
                        .collect()
                })
                .clone();
            if rans.is_empty() {
                continue;
            }
            let node = match ranged.get(&(b, r)) {
                Some(&n) => n,
                None => {
                    let n = t.fresh();
                    t.told[n].push(b);
                    t.told[n].extend(rans);
                    ranged.insert((b, r), n);
                    n
                }
            };
            t.ex_rhs[a][i] = (r, node);
        }
    }

    // The ABox, each individual as its own node. An assertion `a r b` is
    // `a ⊑ ∃r.b` with the ranges put on `b` itself, not on a filler copy: the
    // r-successor IS `b`, and a range that only reached a copy would miss
    // `∃s.(C ⊓ E) ⊑ ⊥` when `C` comes from the range and `E` from `b`'s type.
    // It matters only for consistency: no ABox fact changes a TBox subsumption
    // of a consistent ontology.
    let mut individuals: BTreeSet<usize> = BTreeSet::new();
    for (&ind, types) in &p.individual_types {
        let i = t.atom(ind);
        individuals.insert(i);
        for &c in types {
            let c = t.atom(c);
            t.told[i].push(c);
        }
    }
    for (&ind, cs) in &p.individual_anon_types {
        let i = t.atom(ind);
        individuals.insert(i);
        for c in cs {
            t.add_rhs(i, c);
        }
    }
    for &(a, r, b) in &p.role_assertions {
        let (ia, ib) = (t.atom(a), t.atom(b));
        individuals.insert(ia);
        individuals.insert(ib);
        t.ex_rhs[ia].push((r, ib));
        for s in role_sups(&p.sub_to_super, r) {
            if let Some(rans) = t.ranges.get(&s).cloned() {
                t.told[ib].extend(rans);
            }
        }
    }

    let classes: Vec<(u32, usize)> = {
        let mut v: Vec<(u32, usize)> = p
            .named_classes
            .iter()
            .filter(|&&c| c != p.thing_id && c != p.nothing_id)
            .map(|&c| (c, t.atom(c)))
            .collect();
        v.sort_unstable();
        v
    };

    let mut sat = Saturation::new(&t, &p);
    sat.init(TOP);
    for &(_, n) in &classes {
        sat.init(n);
    }
    for &i in &individuals {
        sat.init(i);
    }
    sat.run();

    let consistent = !sat.s[TOP].contains(&BOTTOM)
        && individuals.iter().all(|&i| !sat.s[i].contains(&BOTTOM));
    let node_class: HashMap<usize, u32> = classes.iter().map(|&(c, n)| (n, c)).collect();
    let name = |c: u32| p.interner.resolve(c).trim_matches(|ch| ch == '<' || ch == '>').to_string();

    let mut unsat: BTreeSet<String> = BTreeSet::new();
    let mut subs: BTreeSet<(String, String)> = BTreeSet::new();
    for &(c, n) in &classes {
        if sat.s[n].contains(&BOTTOM) {
            unsat.insert(name(c));
            continue;
        }
        for d in &sat.s[n] {
            if let Some(&dc) = node_class.get(d)
                && dc != c
            {
                subs.insert((name(c), name(dc)));
            }
        }
    }

    let mut incompleteness: Vec<String> = t
        .dropped
        .iter()
        .map(|(why, n)| format!("{why}: {n} axiom part(s) outside OWL 2 EL not used"))
        .collect();
    if !p.inverse_roles.is_empty() {
        incompleteness.push(format!(
            "inverse or symmetric roles ({} role(s)): outside OWL 2 EL, each read as an unrelated role",
            p.inverse_roles.len()
        ));
    }
    let functional = p.functional_roles.len() + p.inv_functional_roles.len();
    if functional > 0 {
        incompleteness.push(format!(
            "functional or inverse-functional roles ({functional}): outside OWL 2 EL, not used"
        ));
    }
    for (construct, n) in DlReasoner::unmodelled_constructs(graph) {
        incompleteness.push(format!("{construct} ({n} occurrence(s)): not read by the parser"));
    }
    if !consistent {
        incompleteness.push(
            "the ontology is inconsistent: it entails every subsumption, and only the derived ones are listed"
                .into(),
        );
    }
    let complete = incompleteness.is_empty();

    // Outside EL the RL rules can still find sound subsumptions the EL part
    // does not reach, so they are unioned in. On EL input they could add
    // nothing: the table is already every entailed subsumption.
    let profile = if complete {
        "el-completion"
    } else {
        let sandbox = Arc::new(GraphStore::new());
        if !triples.is_empty() {
            let mut nt = String::with_capacity(triples.len() * 64);
            for (s, p, o) in &triples {
                nt.push_str(s);
                nt.push(' ');
                nt.push_str(p);
                nt.push(' ');
                nt.push_str(o);
                nt.push_str(" .\n");
            }
            sandbox.load_ntriples(&nt)?;
        }
        Reasoner::run(&sandbox, "owl-rl-ext", true)?;
        let named: HashSet<String> = classes.iter().map(|&(c, _)| name(c)).collect();
        for (s, pr, o) in sandbox.all_triples()? {
            if pr != RDFS_SUBCLASS || o == OWL_THING || s == o {
                continue;
            }
            let (s, o) = (s.trim_matches(|c| c == '<' || c == '>'), o.trim_matches(|c| c == '<' || c == '>'));
            if named.contains(s) && named.contains(o) && !unsat.contains(s) {
                subs.insert((s.to_string(), o.to_string()));
            }
        }
        "el-completion+owl-rl-ext"
    };

    let subsumptions: Vec<(String, String)> = subs.into_iter().collect();
    Ok(ClassificationReport {
        profile: profile.to_string(),
        complete,
        incompleteness,
        consistent,
        truncated: false,
        class_count: classes.len(),
        subsumption_count: subsumptions.len(),
        subsumptions,
        unsatisfiable: unsat.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_emits_transitive_closure_of_subclassof() {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex: <http://ex.org/> .
            ex:A a owl:Class .
            ex:B a owl:Class ; rdfs:subClassOf ex:A .
            ex:C a owl:Class ; rdfs:subClassOf ex:B .
        "#,
            None,
        )
        .unwrap();
        let r = classify(&g).unwrap();
        // Transitive: C ⊑ A must be derived.
        let has_c_a = r.subsumptions.iter().any(|(s, o)| {
            s == "http://ex.org/C" && o == "http://ex.org/A"
        });
        assert!(has_c_a, "transitive subsumption not derived; got: {:?}", r.subsumptions);
        assert!(r.subsumption_count >= 3);
    }

    #[test]
    fn classify_excludes_trivial_self_subsumptions() {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix ex: <http://ex.org/> . ex:A a owl:Class .",
            None,
        )
        .unwrap();
        let r = classify(&g).unwrap();
        for (s, o) in &r.subsumptions {
            assert_ne!(s, o, "self-subsumption emitted");
        }
    }

    #[test]
    fn classify_does_not_mutate_original() {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . @prefix ex: <http://ex.org/> . ex:A a owl:Class . ex:B a owl:Class ; rdfs:subClassOf ex:A .",
            None,
        )
        .unwrap();
        let pre = g.triple_count();
        let _ = classify(&g).unwrap();
        assert_eq!(pre, g.triple_count());
    }

    const PREFIXES: &str = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
        @prefix ex: <http://ex.org/> .\n";

    fn classify_ttl(body: &str) -> ClassificationReport {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(&format!("{PREFIXES}{body}"), None).unwrap();
        classify(&g).unwrap()
    }

    fn has(r: &ClassificationReport, sub: &str, sup: &str) -> bool {
        r.subsumptions
            .iter()
            .any(|(s, o)| s == &format!("http://ex.org/{sub}") && o == &format!("http://ex.org/{sup}"))
    }

    /// #324, first example: CR2 (a conjunction on the left-hand side).
    #[test]
    fn conjunction_on_the_left_is_derived() {
        let r = classify_ttl(
            "ex:A rdfs:subClassOf ex:B , ex:C .
             [ owl:intersectionOf ( ex:B ex:C ) ] rdfs:subClassOf ex:D .",
        );
        assert!(has(&r, "A", "D"), "A ⊑ D missing; got {:?}", r.subsumptions);
    }

    /// #324, second example: CR3 + CR4 (an existential on the left-hand side).
    #[test]
    fn existential_on_the_left_is_derived() {
        let r = classify_ttl(
            "ex:r a owl:ObjectProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:B ] .
             ex:B rdfs:subClassOf ex:C .
             [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:C ] rdfs:subClassOf ex:D .",
        );
        assert!(has(&r, "A", "D"), "A ⊑ D missing; got {:?}", r.subsumptions);
    }

    /// The issue's second example does come back from OWL-RL — `scm-svf1`
    /// turns `B ⊑ C` into `∃r.B ⊑ ∃r.C` because both restrictions are in the
    /// graph — so it does not separate the two engines. These do: a filler
    /// reached through a conjunction, a domain, and a transitive role. Measured
    /// on the RL read-back before #324, none of the three `A ⊑ D` came back.
    #[test]
    fn existential_through_a_conjunctive_filler_is_derived() {
        let r = classify_ttl(
            "ex:r a owl:ObjectProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:B ] .
             ex:B rdfs:subClassOf ex:C , ex:E .
             [ a owl:Restriction ; owl:onProperty ex:r ;
               owl:someValuesFrom [ owl:intersectionOf ( ex:C ex:E ) ] ] rdfs:subClassOf ex:D .",
        );
        assert!(has(&r, "A", "D"), "A ⊑ D missing; got {:?}", r.subsumptions);
        assert!(r.complete, "{:?}", r.incompleteness);
    }

    #[test]
    fn the_domain_of_an_existential_is_derived() {
        let r = classify_ttl(
            "ex:r a owl:ObjectProperty ; rdfs:domain ex:D .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:B ] .",
        );
        assert!(has(&r, "A", "D"), "A ⊑ D missing; got {:?}", r.subsumptions);
        assert!(r.complete, "{:?}", r.incompleteness);
    }

    #[test]
    fn an_existential_through_a_transitive_role_is_derived() {
        let r = classify_ttl(
            "ex:r a owl:ObjectProperty , owl:TransitiveProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:B ] .
             ex:B rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:C ] .
             [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:C ] rdfs:subClassOf ex:D .",
        );
        assert!(has(&r, "A", "D"), "A ⊑ D missing; got {:?}", r.subsumptions);
        assert!(r.complete, "{:?}", r.incompleteness);
    }

    /// A range reaches the filler: `A ⊑ ∃r.B`, `range(r) = C`, `∃r.(B ⊓ C) ⊑ D`.
    #[test]
    fn a_range_reaches_the_existential_filler() {
        let r = classify_ttl(
            "ex:r a owl:ObjectProperty ; rdfs:range ex:C .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:B ] .
             [ a owl:Restriction ; owl:onProperty ex:r ;
               owl:someValuesFrom [ owl:intersectionOf ( ex:B ex:C ) ] ] rdfs:subClassOf ex:D .",
        );
        assert!(has(&r, "A", "D"), "A ⊑ D missing; got {:?}", r.subsumptions);
    }

    /// Disjointness reaches an existential's filler and makes `A` unsatisfiable.
    #[test]
    fn bottom_propagates_back_through_an_existential() {
        let r = classify_ttl(
            "ex:r a owl:ObjectProperty .
             ex:B owl:disjointWith ex:C .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:r ;
               owl:someValuesFrom [ owl:intersectionOf ( ex:B ex:C ) ] ] .",
        );
        assert_eq!(r.unsatisfiable, vec!["http://ex.org/A".to_string()]);
        assert!(r.complete && r.consistent, "{:?}", r.incompleteness);
    }

    /// Input outside EL says so and names why, and still gets the sound part.
    #[test]
    fn input_outside_el_is_marked_incomplete() {
        let r = classify_ttl(
            "ex:r a owl:ObjectProperty .
             ex:A rdfs:subClassOf ex:B , [ owl:unionOf ( ex:C ex:E ) ] .
             ex:F rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:r ; owl:allValuesFrom ex:B ] .",
        );
        assert!(!r.complete);
        assert_eq!(r.profile, "el-completion+owl-rl-ext");
        assert!(r.incompleteness.iter().any(|w| w.contains("owl:unionOf")), "{:?}", r.incompleteness);
        assert!(r.incompleteness.iter().any(|w| w.contains("owl:allValuesFrom")), "{:?}", r.incompleteness);
        assert!(has(&r, "A", "B"));
    }

    /// A property chain is EL but the parser does not read it, so the table
    /// cannot be called complete.
    #[test]
    fn an_unread_construct_is_marked_incomplete() {
        let r = classify_ttl(
            "ex:r a owl:ObjectProperty . ex:s a owl:ObjectProperty . ex:t a owl:ObjectProperty .
             ex:t owl:propertyChainAxiom ( ex:r ex:s ) .
             ex:A rdfs:subClassOf ex:B .",
        );
        assert!(!r.complete);
        assert!(r.incompleteness.iter().any(|w| w.contains("owl:propertyChainAxiom")), "{:?}", r.incompleteness);
    }

    /// An individual in two disjoint classes makes the ontology inconsistent,
    /// and an inconsistent ontology entails every subsumption.
    #[test]
    fn an_inconsistent_abox_is_reported() {
        let r = classify_ttl(
            "ex:B owl:disjointWith ex:C .
             ex:a a ex:B , ex:C .",
        );
        assert!(!r.consistent);
        assert!(!r.complete);
    }

    /// #324: the read-back used to stop at 100,000 rows and say nothing. A
    /// chain of 460 classes has 460·459/2 = 105,570 strict subsumptions.
    #[test]
    fn a_table_over_one_hundred_thousand_pairs_is_not_cut() {
        let n = 460;
        let mut body = String::new();
        for i in 1..n {
            body.push_str(&format!("ex:C{i} rdfs:subClassOf ex:C{} .\n", i - 1));
        }
        let r = classify_ttl(&body);
        assert_eq!(r.subsumption_count, n * (n - 1) / 2);
        assert_eq!(r.subsumptions.len(), r.subsumption_count);
        assert!(r.complete && !r.truncated);
    }
}
