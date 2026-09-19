//! Certifying UNSATISFIABILITY, the half `src/tableaux.rs` could not certify.
//!
//! `ModelOutcome` hands over a finite interpretation when the answer is yes.
//! That works because a model is a finite object and checking it is evaluation.
//! The negative answer has no such object. "This ontology has no model" is a
//! claim about every interpretation, including the infinite ones SHIQ forces,
//! so nothing finite can be exhibited and nothing can be evaluated. What can be
//! exhibited is a DERIVATION: a closed tableau, whose every leaf is a branch no
//! interpretation satisfies and whose every step preserves satisfiability
//! downward. `lean/Dl/Tableau.lean` proves that; `lean/Dl/Refute.lean` checks
//! one; this writes one.
//!
//! # Why this is a second prover rather than instrumentation
//!
//! The obvious move is to record what `Tableau::expand` did and print it. That
//! would be wrong, and not only awkward. The fast reasoner uses blocking,
//! merging, inverse propagation and absorbed GCIs. Blocking and merging are
//! COMPLETENESS devices: they exist so that a search for a model terminates.
//! A refutation needs none of them, and the certified calculus deliberately has
//! neither, so most of what the fast engine records could not be written down
//! here anyway.
//!
//! So this is a small independent search over the certified rules only. It is
//! slower and weaker than `Tableau`, and that is the correct trade: if it finds
//! a refutation, the refutation is checkable; if it does not, `Tableau`'s answer
//! stands exactly as it did before, as testimony. Nothing about this module can
//! make a wrong answer look right, because it does not decide anything. The
//! Lean checker does, from the bytes.
//!
//! # What it cannot do
//!
//! Transitive, symmetric, inverse and inverse-functional roles have no rules in
//! the certified calculus, so an ontology inconsistent only through one of
//! those gets no certificate. The emitter says so rather than guessing.

use crate::tableaux::{axiom_line, name_is_safe, Concept, DlAxiom, Interner};
use std::collections::HashSet;
use std::path::Path;

// ── The Lean-side concept shape ─────────────────────────────────────────

/// A concept as `lean/Dl/Syntax.lean` spells it.
///
/// Not the same type as [`Concept`]. The reasoner's concepts are in negation
/// normal form with n-ary conjunction and disjunction; Lean's have an arbitrary
/// negation and binary connectives. The difference matters because the rules
/// fire on the Lean shape: applying the ∧-rule to `And([c, d, e])` gives two
/// conclusions in Lean, `c` and `and d e`, not three.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Lc {
    Top,
    Bot,
    Atom(u32),
    Neg(Box<Lc>),
    And(Box<Lc>, Box<Lc>),
    Or(Box<Lc>, Box<Lc>),
    Ex(u32, Box<Lc>),
    All(u32, Box<Lc>),
    /// `≥n R.C`, written `(n, role, filler)`.
    Min(u32, u32, Box<Lc>),
    /// `≤n R.C`.
    Max(u32, u32, Box<Lc>),
}

/// Fold an n-ary list the way `write_nary` in `src/tableaux.rs` does, which is
/// right-associatively with the connective's unit for the empty list.
///
/// This MUST agree with that function, because the certificate cites concepts
/// that have to be equal, as terms, to the ones in `axioms.tsv`. The agreement
/// is not asserted: `lowering_agrees_with_the_axiom_writer` below renders both
/// ways and compares the strings.
fn lower_nary(cs: &[Concept], and: bool) -> Lc {
    match cs.split_first() {
        None => {
            if and {
                Lc::Top
            } else {
                Lc::Bot
            }
        }
        Some((head, [])) => lower(head),
        Some((head, rest)) => {
            let (h, t) = (Box::new(lower(head)), Box::new(lower_nary(rest, and)));
            if and {
                Lc::And(h, t)
            } else {
                Lc::Or(h, t)
            }
        }
    }
}

pub(crate) fn lower(c: &Concept) -> Lc {
    match c {
        Concept::Top => Lc::Top,
        Concept::Bottom => Lc::Bot,
        Concept::Atom(a) => Lc::Atom(*a),
        Concept::NegAtom(a) => Lc::Neg(Box::new(Lc::Atom(*a))),
        Concept::And(cs) => lower_nary(cs, true),
        Concept::Or(cs) => lower_nary(cs, false),
        Concept::Exists(r, f) => Lc::Ex(*r, Box::new(lower(f))),
        Concept::ForAll(r, f) => Lc::All(*r, Box::new(lower(f))),
        Concept::MinCard(r, n, f) => Lc::Min(*n, *r, Box::new(lower(f))),
        Concept::MaxCard(r, n, f) => Lc::Max(*n, *r, Box::new(lower(f))),
    }
}

/// The one place a name reaches the certificate, and therefore the one place the
/// round-trip guard has to be. Same discipline as the model emitter's
/// `push_name`, and for the same reason: the grammar is space-separated, so a
/// name carrying a space would be read back as two tokens.
fn push_name(out: &mut String, it: &Interner, id: u32, bad: &mut Option<String>) {
    let s = it.resolve(id);
    if !name_is_safe(s) && bad.is_none() {
        *bad = Some(s.to_string());
    }
    out.push_str(s);
}

fn write_lc(out: &mut String, it: &Interner, c: &Lc, bad: &mut Option<String>) {
    match c {
        Lc::Top => out.push_str("top"),
        Lc::Bot => out.push_str("bot"),
        Lc::Atom(a) => {
            out.push_str("atom ");
            push_name(out, it, *a, bad);
        }
        Lc::Neg(f) => {
            out.push_str("not ");
            write_lc(out, it, f, bad);
        }
        Lc::And(f, g) => {
            out.push_str("and ");
            write_lc(out, it, f, bad);
            out.push(' ');
            write_lc(out, it, g, bad);
        }
        Lc::Or(f, g) => {
            out.push_str("or ");
            write_lc(out, it, f, bad);
            out.push(' ');
            write_lc(out, it, g, bad);
        }
        Lc::Ex(r, f) => {
            out.push_str("some ");
            push_name(out, it, *r, bad);
            out.push(' ');
            write_lc(out, it, f, bad);
        }
        Lc::All(r, f) => {
            out.push_str("all ");
            push_name(out, it, *r, bad);
            out.push(' ');
            write_lc(out, it, f, bad);
        }
        Lc::Min(n, r, f) => {
            out.push_str("min ");
            out.push_str(&n.to_string());
            out.push(' ');
            push_name(out, it, *r, bad);
            out.push(' ');
            write_lc(out, it, f, bad);
        }
        Lc::Max(n, r, f) => {
            out.push_str("max ");
            out.push_str(&n.to_string());
            out.push(' ');
            push_name(out, it, *r, bad);
            out.push(' ');
            write_lc(out, it, f, bad);
        }
    }
}

// ── Branches ────────────────────────────────────────────────────────────

/// One entry on a tableau branch, mirroring `Dl.Constraint`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum C {
    Conc(u32, Lc),
    Role(u32, u32, u32),
    Diff(u32, u32),
}

/// A branch, kept as a set for membership and as a vector for order.
///
/// `fired` records which generating rule applications have already happened, so
/// the ∃ and ≥ rules cannot invent witnesses for the same constraint forever.
/// That is the only thing standing between this search and an infinite one, and
/// it is why this prover is incomplete rather than non-terminating: a genuine
/// SHIQ model can need an infinite chain, and refusing to build one costs
/// nothing here because a refutation never needs it.
#[derive(Clone, Default)]
struct Branch {
    set: HashSet<C>,
    conc_of: HashSet<u32>,
    fired: HashSet<(u32, Lc)>,
    fired_ne: HashSet<usize>,
}

impl Branch {
    fn has(&self, c: &C) -> bool {
        self.set.contains(c)
    }

    fn add(&mut self, c: C) -> bool {
        if let C::Conc(x, _) = &c {
            self.conc_of.insert(*x);
        }
        self.set.insert(c)
    }

    fn has_conc(&self, x: u32) -> bool {
        self.conc_of.contains(&x)
    }

    fn concepts(&self) -> Vec<(u32, Lc)> {
        let mut v: Vec<(u32, Lc)> = self
            .set
            .iter()
            .filter_map(|c| match c {
                C::Conc(x, f) => Some((*x, f.clone())),
                _ => None,
            })
            .collect();
        v.sort_by_key(|(x, f)| (*x, format!("{f:?}")));
        v
    }

    fn edges(&self) -> Vec<(u32, u32, u32)> {
        let mut v: Vec<(u32, u32, u32)> = self
            .set
            .iter()
            .filter_map(|c| match c {
                C::Role(r, x, y) => Some((*r, *x, *y)),
                _ => None,
            })
            .collect();
        v.sort_unstable();
        v
    }
}

// ── The certificate tree ────────────────────────────────────────────────

/// A step of a closed tableau, mirroring `Dl.Cert` constructor for constructor.
enum Step {
    Bot(u32),
    NegC(u32, Lc),
    DisjC(u32, Lc, Lc),
    DiffC(u32),
    MinMax(u32, u32, Lc, u32, u32),
    MaxC(u32, u32, Lc, u32, Vec<u32>),
    Inst(u32, Lc, Box<Step>),
    Rel(u32, u32, u32, Box<Step>),
    Sub(u32, Lc, Lc, Box<Step>),
    Dom(u32, u32, u32, Lc, Box<Step>),
    Rng(u32, u32, u32, Lc, Box<Step>),
    SubRole(u32, u32, u32, u32, Box<Step>),
    NonEmpty(u32, Lc, Box<Step>),
    AndS(u32, Lc, Lc, Box<Step>),
    AllS(u32, u32, u32, Lc, Box<Step>),
    ExS(u32, u32, u32, Lc, Box<Step>),
    MinS(u32, u32, Lc, u32, Vec<u32>, Box<Step>),
    OrS(u32, Lc, Lc, Box<Step>, Box<Step>),
}

fn count(s: &Step) -> usize {
    match s {
        Step::Bot(..)
        | Step::NegC(..)
        | Step::DisjC(..)
        | Step::DiffC(..)
        | Step::MinMax(..)
        | Step::MaxC(..) => 1,
        Step::Inst(_, _, k)
        | Step::Rel(_, _, _, k)
        | Step::Sub(_, _, _, k)
        | Step::Dom(_, _, _, _, k)
        | Step::Rng(_, _, _, _, k)
        | Step::SubRole(_, _, _, _, k)
        | Step::NonEmpty(_, _, k)
        | Step::AndS(_, _, _, k)
        | Step::AllS(_, _, _, _, k)
        | Step::ExS(_, _, _, _, k)
        | Step::MinS(_, _, _, _, _, k) => 1 + count(k),
        Step::OrS(_, _, _, l, r) => 1 + count(l) + count(r),
    }
}

/// Render the tree in the prefix order `Dl.Parse.cert?` reads.
///
/// Indentation is for a human only. Newlines are not significant in the format,
/// because the arities carry the tree shape, so nothing here can become
/// load-bearing by accident.
fn write_step(out: &mut String, it: &Interner, s: &Step, ind: usize, bad: &mut Option<String>) {
    let pad = "  ".repeat(ind);
    let kid = |out: &mut String, k: &Step, bad: &mut Option<String>| {
        out.push('\n');
        write_step(out, it, k, ind + 1, bad);
    };
    let n = |out: &mut String, id: u32, bad: &mut Option<String>| {
        push_name(out, it, id, bad);
    };
    out.push_str(&pad);
    match s {
        Step::Bot(x) => {
            out.push_str("bot ");
            n(out, *x, bad);
        }
        Step::DiffC(x) => {
            out.push_str("diff ");
            n(out, *x, bad);
        }
        Step::NegC(x, c) => {
            out.push_str("neg ");
            n(out, *x, bad);
            out.push(' ');
            write_lc(out, it, c, bad);
        }
        Step::DisjC(x, c, d) => {
            out.push_str("disjoint ");
            n(out, *x, bad);
            out.push(' ');
            write_lc(out, it, c, bad);
            out.push(' ');
            write_lc(out, it, d, bad);
        }
        Step::MinMax(x, r, c, m, k) => {
            out.push_str("minmax ");
            n(out, *x, bad);
            out.push(' ');
            n(out, *r, bad);
            out.push_str(&format!(" {m} {k} "));
            write_lc(out, it, c, bad);
        }
        Step::MaxC(x, r, c, k, ys) => {
            out.push_str("maxclash ");
            n(out, *x, bad);
            out.push(' ');
            n(out, *r, bad);
            out.push_str(&format!(" {k} {} ", ys.len()));
            for y in ys {
                n(out, *y, bad);
                out.push(' ');
            }
            write_lc(out, it, c, bad);
        }
        Step::Inst(a, c, k) => {
            out.push_str("inst ");
            n(out, *a, bad);
            out.push(' ');
            write_lc(out, it, c, bad);
            kid(out, k, bad);
        }
        Step::Rel(a, r, b, k) => {
            out.push_str("rel ");
            n(out, *a, bad);
            out.push(' ');
            n(out, *r, bad);
            out.push(' ');
            n(out, *b, bad);
            kid(out, k, bad);
        }
        Step::Sub(x, c, d, k) => {
            out.push_str("sub ");
            n(out, *x, bad);
            out.push(' ');
            write_lc(out, it, c, bad);
            out.push(' ');
            write_lc(out, it, d, bad);
            kid(out, k, bad);
        }
        Step::Dom(x, y, r, c, k) => {
            out.push_str("domain ");
            n(out, *x, bad);
            out.push(' ');
            n(out, *y, bad);
            out.push(' ');
            n(out, *r, bad);
            out.push(' ');
            write_lc(out, it, c, bad);
            kid(out, k, bad);
        }
        Step::Rng(x, y, r, c, k) => {
            out.push_str("range ");
            n(out, *x, bad);
            out.push(' ');
            n(out, *y, bad);
            out.push(' ');
            n(out, *r, bad);
            out.push(' ');
            write_lc(out, it, c, bad);
            kid(out, k, bad);
        }
        Step::SubRole(x, y, r, t, k) => {
            out.push_str("subrole ");
            n(out, *x, bad);
            out.push(' ');
            n(out, *y, bad);
            out.push(' ');
            n(out, *r, bad);
            out.push(' ');
            n(out, *t, bad);
            kid(out, k, bad);
        }
        Step::NonEmpty(y, c, k) => {
            out.push_str("nonempty ");
            n(out, *y, bad);
            out.push(' ');
            write_lc(out, it, c, bad);
            kid(out, k, bad);
        }
        Step::AndS(x, c, d, k) => {
            out.push_str("and ");
            n(out, *x, bad);
            out.push(' ');
            write_lc(out, it, c, bad);
            out.push(' ');
            write_lc(out, it, d, bad);
            kid(out, k, bad);
        }
        Step::AllS(x, y, r, c, k) => {
            out.push_str("all ");
            n(out, *x, bad);
            out.push(' ');
            n(out, *y, bad);
            out.push(' ');
            n(out, *r, bad);
            out.push(' ');
            write_lc(out, it, c, bad);
            kid(out, k, bad);
        }
        Step::ExS(x, y, r, c, k) => {
            out.push_str("some ");
            n(out, *x, bad);
            out.push(' ');
            n(out, *y, bad);
            out.push(' ');
            n(out, *r, bad);
            out.push(' ');
            write_lc(out, it, c, bad);
            kid(out, k, bad);
        }
        Step::MinS(x, r, c, k, ys, next) => {
            out.push_str("min ");
            n(out, *x, bad);
            out.push(' ');
            n(out, *r, bad);
            out.push_str(&format!(" {k} {} ", ys.len()));
            for y in ys {
                n(out, *y, bad);
                out.push(' ');
            }
            write_lc(out, it, c, bad);
            kid(out, next, bad);
        }
        Step::OrS(x, c, d, l, r) => {
            out.push_str("or ");
            n(out, *x, bad);
            out.push(' ');
            write_lc(out, it, c, bad);
            out.push(' ');
            write_lc(out, it, d, bad);
            kid(out, l, bad);
            kid(out, r, bad);
        }
    }
}

// ── The axioms, in the shape the rules read them ────────────────────────

/// The axioms the certified calculus has rules for. Everything else is dropped
/// here and named in [`RefuteOutcome::NotFound`] if the search fails, so a
/// reader can tell "no refutation exists" from "no refutation this calculus can
/// write".
struct Rules {
    subs: Vec<(Lc, Lc)>,
    disjoints: Vec<(Lc, Lc)>,
    domains: Vec<(u32, Lc)>,
    ranges: Vec<(u32, Lc)>,
    subroles: Vec<(u32, u32)>,
    insts: Vec<(u32, Lc)>,
    rels: Vec<(u32, u32, u32)>,
    indivs: HashSet<u32>,
    nonempties: Vec<Lc>,
    /// Role names the axiom set mentions, which is what `roleNames A` is in
    /// Lean and what several rules need to put a target in the carrier.
    roles: HashSet<u32>,
    /// Individual names the axiom set mentions. Invented witnesses must avoid
    /// these, or the assignment the soundness proof builds would disagree with
    /// `I.ind` and the rule would be unsound.
    inds: HashSet<u32>,
    /// Axiom kinds present in the ontology that this calculus cannot use.
    unsupported: Vec<&'static str>,
}

fn concept_roles(c: &Concept, out: &mut HashSet<u32>) {
    match c {
        Concept::Top | Concept::Bottom | Concept::Atom(_) | Concept::NegAtom(_) => {}
        Concept::And(cs) | Concept::Or(cs) => cs.iter().for_each(|d| concept_roles(d, out)),
        Concept::Exists(r, f) | Concept::ForAll(r, f) => {
            out.insert(*r);
            concept_roles(f, out);
        }
        Concept::MinCard(r, _, f) | Concept::MaxCard(r, _, f) => {
            out.insert(*r);
            concept_roles(f, out);
        }
    }
}

impl Rules {
    fn new(axioms: &[DlAxiom]) -> Rules {
        let mut r = Rules {
            subs: vec![],
            disjoints: vec![],
            domains: vec![],
            ranges: vec![],
            subroles: vec![],
            insts: vec![],
            rels: vec![],
            indivs: HashSet::new(),
            nonempties: vec![],
            roles: HashSet::new(),
            inds: HashSet::new(),
            unsupported: vec![],
        };
        let note = |r: &mut Rules, k: &'static str| {
            if !r.unsupported.contains(&k) {
                r.unsupported.push(k);
            }
        };
        for a in axioms {
            // `roleNames` and `indNames` in `lean/Dl/Syntax.lean`, clause for
            // clause. They are computed from EVERY axiom, including the ones
            // this calculus has no rule for, because that is what the Lean side
            // computes and the two have to agree.
            match a {
                DlAxiom::Sub(c, d) | DlAxiom::Disjoint(c, d) => {
                    concept_roles(c, &mut r.roles);
                    concept_roles(d, &mut r.roles);
                }
                DlAxiom::Domain(role, c) | DlAxiom::Range(role, c) => {
                    r.roles.insert(*role);
                    concept_roles(c, &mut r.roles);
                }
                DlAxiom::Inst(i, c) => {
                    r.inds.insert(*i);
                    concept_roles(c, &mut r.roles);
                }
                DlAxiom::NonEmpty(c) => concept_roles(c, &mut r.roles),
                DlAxiom::SubRole(a1, b1) | DlAxiom::Inv(a1, b1) => {
                    r.roles.insert(*a1);
                    r.roles.insert(*b1);
                }
                DlAxiom::Trans(role) | DlAxiom::Sym(role) | DlAxiom::InvFunc(role) => {
                    r.roles.insert(*role);
                }
                DlAxiom::Rel(a1, role, b1) => {
                    r.roles.insert(*role);
                    r.inds.insert(*a1);
                    r.inds.insert(*b1);
                }
                DlAxiom::Indiv(i) => {
                    r.inds.insert(*i);
                }
            }
            match a {
                DlAxiom::Sub(c, d) => r.subs.push((lower(c), lower(d))),
                DlAxiom::Disjoint(c, d) => r.disjoints.push((lower(c), lower(d))),
                DlAxiom::Domain(role, c) => r.domains.push((*role, lower(c))),
                DlAxiom::Range(role, c) => r.ranges.push((*role, lower(c))),
                DlAxiom::SubRole(a1, b1) => r.subroles.push((*a1, *b1)),
                DlAxiom::Inst(i, c) => r.insts.push((*i, lower(c))),
                DlAxiom::Rel(a1, role, b1) => r.rels.push((*a1, *role, *b1)),
                DlAxiom::Indiv(i) => {
                    r.indivs.insert(*i);
                }
                DlAxiom::NonEmpty(c) => r.nonempties.push(lower(c)),
                DlAxiom::Trans(_) => note(&mut r, "trans"),
                DlAxiom::Sym(_) => note(&mut r, "sym"),
                DlAxiom::Inv(..) => note(&mut r, "inv"),
                DlAxiom::InvFunc(_) => note(&mut r, "invfunc"),
            }
        }
        r
    }
}

// ── The search ──────────────────────────────────────────────────────────

struct Prover<'a> {
    rules: &'a Rules,
    /// Fresh names, interned as they are invented. Checked against `inds`
    /// rather than assumed distinct from it.
    fresh: Vec<u32>,
    next_fresh: usize,
    budget: usize,
}

impl<'a> Prover<'a> {
    /// A name no axiom mentions. The Lean rules require exactly that, on top of
    /// the name being new to the branch.
    fn fresh_name(&mut self, it: &mut Interner) -> Option<u32> {
        for k in 0..64 {
            let id = it.intern(&format!("_oo_w{}_{}", self.next_fresh, k));
            if !self.rules.inds.contains(&id) && !self.fresh.contains(&id) {
                self.next_fresh += 1;
                self.fresh.push(id);
                return Some(id);
            }
        }
        None
    }

    fn spend(&mut self) -> bool {
        if self.budget == 0 {
            return false;
        }
        self.budget -= 1;
        true
    }

    /// A clash on this branch, if there is one.
    fn clash(&self, br: &Branch) -> Option<Step> {
        for (x, c) in br.concepts() {
            if c == Lc::Bot {
                return Some(Step::Bot(x));
            }
            if br.has(&C::Conc(x, Lc::Neg(Box::new(c.clone())))) {
                return Some(Step::NegC(x, c));
            }
            if br.has(&C::Diff(x, x)) {
                return Some(Step::DiffC(x));
            }
            if let Lc::Min(m, r, f) = &c {
                for (y, d) in br.concepts() {
                    if y != x {
                        continue;
                    }
                    if let Lc::Max(n, r2, g) = &d
                        && r == r2
                        && f == g
                        && n < m
                    {
                        return Some(Step::MinMax(x, *r, (**f).clone(), *m, *n));
                    }
                }
            }
        }
        for (c, d) in &self.rules.disjoints {
            for (x, f) in br.concepts() {
                if &f == c && br.has(&C::Conc(x, d.clone())) {
                    return Some(Step::DisjC(x, c.clone(), d.clone()));
                }
            }
        }
        // The cardinality clash: more pairwise-distinct successors in the filler
        // than the bound allows. The witnesses have to be distinct ON THE
        // BRANCH, which is what the ≥ rule's inequalities are for.
        for (x, c) in br.concepts() {
            let Lc::Max(n, r, f) = &c else { continue };
            let mut ys: Vec<u32> = br
                .edges()
                .into_iter()
                .filter(|(rr, xx, _)| rr == r && *xx == x)
                .map(|(_, _, y)| y)
                .filter(|y| br.has(&C::Conc(*y, (**f).clone())))
                .collect();
            ys.sort_unstable();
            ys.dedup();
            let mut picked: Vec<u32> = vec![];
            for y in ys {
                if picked
                    .iter()
                    .all(|&p| br.has(&C::Diff(p, y)) && br.has(&C::Diff(y, p)))
                {
                    picked.push(y);
                }
                if picked.len() == *n as usize + 1 {
                    return Some(Step::MaxC(x, *r, (**f).clone(), *n, picked));
                }
            }
        }
        None
    }

    /// The first rule that adds something the branch does not already carry.
    /// Every one of these strictly grows the branch, which is what keeps the
    /// deterministic phase finite.
    #[allow(clippy::type_complexity)]
    fn det(&self, br: &Branch) -> Option<(Branch, Box<dyn Fn(Box<Step>) -> Step>)> {
        // Conjunction first: it is the cheapest and it feeds everything else.
        for (x, c) in br.concepts() {
            if let Lc::And(f, g) = &c {
                let (a, b) = (C::Conc(x, (**f).clone()), C::Conc(x, (**g).clone()));
                if !br.has(&a) || !br.has(&b) {
                    let mut nb = br.clone();
                    nb.add(a);
                    nb.add(b);
                    let (fc, gc) = ((**f).clone(), (**g).clone());
                    return Some((nb, Box::new(move |k| Step::AndS(x, fc.clone(), gc.clone(), k))));
                }
            }
        }
        for (a, c) in &self.rules.insts {
            if !self.rules.indivs.contains(a) {
                continue;
            }
            let goal = C::Conc(*a, c.clone());
            if !br.has(&goal) {
                let mut nb = br.clone();
                nb.add(goal);
                let (aa, cc) = (*a, c.clone());
                return Some((nb, Box::new(move |k| Step::Inst(aa, cc.clone(), k))));
            }
        }
        for (a, r, b) in &self.rules.rels {
            let goal = C::Role(*r, *a, *b);
            if !br.has(&goal) {
                let mut nb = br.clone();
                nb.add(goal);
                let (aa, rr, bb) = (*a, *r, *b);
                return Some((nb, Box::new(move |k| Step::Rel(aa, rr, bb, k))));
            }
        }
        for (c, d) in &self.rules.subs {
            for (x, f) in br.concepts() {
                if &f != c {
                    continue;
                }
                let goal = C::Conc(x, d.clone());
                if !br.has(&goal) {
                    let mut nb = br.clone();
                    nb.add(goal);
                    let (cc, dd) = (c.clone(), d.clone());
                    return Some((nb, Box::new(move |k| Step::Sub(x, cc.clone(), dd.clone(), k))));
                }
            }
        }
        for (r, x, y) in br.edges() {
            for (role, c) in &self.rules.domains {
                if *role != r || !br.has_conc(x) {
                    continue;
                }
                let goal = C::Conc(x, c.clone());
                if !br.has(&goal) {
                    let mut nb = br.clone();
                    nb.add(goal);
                    let cc = c.clone();
                    return Some((nb, Box::new(move |k| Step::Dom(x, y, r, cc.clone(), k))));
                }
            }
            for (role, c) in &self.rules.ranges {
                if *role != r || !br.has_conc(x) || !self.rules.roles.contains(&r) {
                    continue;
                }
                let goal = C::Conc(y, c.clone());
                if !br.has(&goal) {
                    let mut nb = br.clone();
                    nb.add(goal);
                    let cc = c.clone();
                    return Some((nb, Box::new(move |k| Step::Rng(x, y, r, cc.clone(), k))));
                }
            }
            for (role, t) in &self.rules.subroles {
                if *role != r || !br.has_conc(x) {
                    continue;
                }
                let goal = C::Role(*t, x, y);
                if !br.has(&goal) {
                    let mut nb = br.clone();
                    nb.add(goal);
                    let tt = *t;
                    return Some((nb, Box::new(move |k| Step::SubRole(x, y, r, tt, k))));
                }
            }
        }
        for (x, c) in br.concepts() {
            let Lc::All(r, f) = &c else { continue };
            if !self.rules.roles.contains(r) {
                continue;
            }
            for (rr, xx, y) in br.edges() {
                if rr != *r || xx != x {
                    continue;
                }
                let goal = C::Conc(y, (**f).clone());
                if !br.has(&goal) {
                    let mut nb = br.clone();
                    nb.add(goal);
                    let (rrr, ff) = (*r, (**f).clone());
                    return Some((nb, Box::new(move |k| Step::AllS(x, y, rrr, ff.clone(), k))));
                }
            }
        }
        None
    }

    fn close(&mut self, br: &Branch, it: &mut Interner) -> Option<Step> {
        if !self.spend() {
            return None;
        }
        if let Some(leaf) = self.clash(br) {
            return Some(leaf);
        }
        if let Some((nb, wrap)) = self.det(br) {
            return self.close(&nb, it).map(|k| wrap(Box::new(k)));
        }
        // Non-emptiness: the only rule that fires on an empty branch, so without
        // it a pure TBox question never starts. Once per axiom.
        for (i, c) in self.rules.nonempties.iter().enumerate() {
            if br.fired_ne.contains(&i) {
                continue;
            }
            let y = self.fresh_name(it)?;
            let mut nb = br.clone();
            nb.fired_ne.insert(i);
            nb.add(C::Conc(y, c.clone()));
            let cc = c.clone();
            return self
                .close(&nb, it)
                .map(|k| Step::NonEmpty(y, cc, Box::new(k)));
        }
        // Disjunction. Both children must close, which is the only reason a
        // tableau is a tree rather than a list.
        for (x, c) in br.concepts() {
            let Lc::Or(f, g) = &c else { continue };
            let mut lb = br.clone();
            lb.add(C::Conc(x, (**f).clone()));
            let mut rb = br.clone();
            rb.add(C::Conc(x, (**g).clone()));
            let l = self.close(&lb, it)?;
            let r = self.close(&rb, it)?;
            return Some(Step::OrS(
                x,
                (**f).clone(),
                (**g).clone(),
                Box::new(l),
                Box::new(r),
            ));
        }
        // Generating rules last, and once per constraint.
        for (x, c) in br.concepts() {
            if br.fired.contains(&(x, c.clone())) {
                continue;
            }
            match &c {
                Lc::Ex(r, f) => {
                    if !self.rules.roles.contains(r) {
                        continue;
                    }
                    let y = self.fresh_name(it)?;
                    let mut nb = br.clone();
                    nb.fired.insert((x, c.clone()));
                    nb.add(C::Role(*r, x, y));
                    nb.add(C::Conc(y, (**f).clone()));
                    let (rr, ff) = (*r, (**f).clone());
                    return self
                        .close(&nb, it)
                        .map(|k| Step::ExS(x, y, rr, ff, Box::new(k)));
                }
                Lc::Min(n, r, f) => {
                    if !self.rules.roles.contains(r) {
                        continue;
                    }
                    let mut ys = vec![];
                    for _ in 0..*n {
                        let y = self.fresh_name(it)?;
                        ys.push(y);
                    }
                    let mut nb = br.clone();
                    nb.fired.insert((x, c.clone()));
                    for &y in &ys {
                        nb.add(C::Role(*r, x, y));
                        nb.add(C::Conc(y, (**f).clone()));
                    }
                    for &a in &ys {
                        for &b in &ys {
                            if a != b {
                                nb.add(C::Diff(a, b));
                            }
                        }
                    }
                    let (rr, ff, nn, yy) = (*r, (**f).clone(), *n, ys.clone());
                    return self
                        .close(&nb, it)
                        .map(|k| Step::MinS(x, rr, ff, nn, yy, Box::new(k)));
                }
                _ => {}
            }
        }
        None
    }
}

// ── The public face ─────────────────────────────────────────────────────

/// What [`certify_unsatisfiable`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuteOutcome {
    /// `axioms.tsv` and `refutation.cert` were written. `oo-dlrefute` is what
    /// turns that into a verified statement; this layer only claims to have
    /// written a tree it believes closes.
    Certified { axioms: usize, steps: usize },
    /// The search found no closed tableau within the budget. This is NOT a
    /// claim that the ontology is consistent, and the string says what the
    /// calculus could not use.
    NotFound(String),
    /// A certificate could not be written even though a tree was found, because
    /// a name would not survive the round trip.
    Refused(String),
}

impl RefuteOutcome {
    pub fn is_certified(&self) -> bool {
        matches!(self, RefuteOutcome::Certified { .. })
    }

    pub fn describe(&self) -> String {
        match self {
            RefuteOutcome::Certified { axioms, steps } => {
                format!("certified: {axioms} axioms refuted in {steps} steps")
            }
            RefuteOutcome::NotFound(why) => format!("no certificate: {why}"),
            RefuteOutcome::Refused(why) => format!("no certificate: {why}"),
        }
    }
}

/// The default search budget, in rule applications.
pub const DEFAULT_BUDGET: usize = 20_000;

/// Look for a closed tableau over `axioms`, and write it if one is found.
///
/// Writes two files into `dir`: `axioms.tsv`, which is the same transcription
/// the model certificate uses, and `refutation.cert`. Check them with
/// `oo-dlrefute AXIOMS.tsv REFUTATION.cert`.
pub(crate) fn certify_unsatisfiable(
    axioms: &[DlAxiom],
    interner: &mut Interner,
    dir: &Path,
    budget: usize,
) -> anyhow::Result<RefuteOutcome> {
    let rules = Rules::new(axioms);
    let mut prover = Prover {
        rules: &rules,
        fresh: vec![],
        next_fresh: 0,
        budget,
    };
    let start = Branch::default();
    let Some(tree) = prover.close(&start, interner) else {
        let mut why = format!("no closed tableau within {budget} steps");
        if !rules.unsupported.is_empty() {
            why.push_str(&format!(
                "; the calculus has no rules for {} and dropped {} axiom kind(s), \
                 so an inconsistency that needs one of those cannot be written here",
                rules.unsupported.join(", "),
                rules.unsupported.len()
            ));
        }
        return Ok(RefuteOutcome::NotFound(why));
    };

    // Same discipline as the model emitter: build the text first and let
    // `push_name` record the first refusal, so the coverage question cannot
    // arise from two loops disagreeing about which names were checked.
    let mut bad: Option<String> = None;
    let mut axiom_text = String::new();
    for a in axioms {
        axiom_text.push_str(&axiom_line(interner, a, &mut bad));
        axiom_text.push('\n');
    }
    let mut cert_text = String::new();
    write_step(&mut cert_text, interner, &tree, 0, &mut bad);
    cert_text.push('\n');

    if let Some(name) = bad {
        return Ok(RefuteOutcome::Refused(format!(
            "the name {name:?} carries a space, a tab, a newline or nothing at all, \
             so it would not survive the round trip through the certificate"
        )));
    }

    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("axioms.tsv"), axiom_text)?;
    std::fs::write(dir.join("refutation.cert"), cert_text)?;
    Ok(RefuteOutcome::Certified {
        axioms: axioms.len(),
        steps: count(&tree),
    })
}
