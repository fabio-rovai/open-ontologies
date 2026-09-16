//! First-order export: hand an ontology to the automated-theorem-proving
//! ecosystem, in TPTP FOF and in ISO/IEC 24707 CLIF.
//!
//! # What makes this different from every other OWL-to-FOL exporter
//!
//! A sibling project, `owl-lean`, carries a machine-checked adequacy theorem
//! for an OWL-to-first-order translation: `OwlLean.adequacy`, with axiom
//! footprint `[propext, Classical.choice, Quot.sound]`, no `sorry` and no
//! Mathlib. Everyone else's exporter is validated empirically (FOWL, over 168
//! ChEBI modules) or proved on paper (Hets, via the institution satisfaction
//! condition). The translation in this file is written to be the SAME
//! translation that theorem is about, function for function, so a reader can
//! cite a kernel-checked result about what the emitted file means.
//!
//! `adequacy` says, for an ontology `O` and an axiom `a`:
//!
//! ```text
//! Entails O a  ↔  FOL.Entails (background ++ indAxioms inds ++ O.map trAx) (trAx a)
//! ```
//!
//! given `hinds : ∀ x : S.Ind, x ∈ inds`, i.e. `inds` enumerates the whole
//! individual vocabulary of the signature.
//!
//! # The correspondence is PINNED BY TESTS AND IS NOT ITSELF PROVED
//!
//! This is Rust and that is Lean. Nothing mechanically checks that
//! `Translation::concept` below is `OwlLean.tr`, or that `Translation::axiom`
//! is `OwlLean.trAx`. What exists is
//! `tests/fol_translation_correspondence_test.rs`, which takes worked cases,
//! computes the expected formula by hand from `OwlLean/Translation.lean`, and
//! pins the emitted string against it. If the Rust and the Lean drift apart,
//! that test is the only thing that will notice. Read the claim as: the
//! translation is the one the theorem is about, up to a test suite, not up to
//! a proof.
//!
//! # The two things the theorem needs, and how they are handled here
//!
//! ## Freshness
//!
//! `tr` allocates bound variables from a counter, and `tr_bridge` needs
//! `Fresh n x`, that is `x < n`: the subject variable must lie strictly below
//! the counter. It is not bookkeeping.
//! `OwlLean.Refutations.tr_bridge_needs_freshness` is a machine-checked
//! countermodel in which the translation of `∃r.⊤` at subject `0` with counter
//! `0` captures its own subject, so the formula stops saying "`a` has an
//! `r`-successor" and starts saying "something is `r`-related to itself".
//!
//! How this file handles it: the exporter never picks a counter. Every entry
//! into the concept translation goes through [`Translation::concept_fresh`],
//! which returns `Err` unless `x < counter`, and the four call sites reproduce
//! `trAx`'s own choices (`tr c 0 2`, `tr c 1 2`, `tr c 0 1`) rather than
//! inventing them. An edit that changes a counter fails a test instead of
//! silently emitting a captured formula.
//!
//! ## Individual typing axioms
//!
//! Nothing in `background` forces a constant to denote an OBJECT, so an
//! arbitrary first-order model may interpret an individual name outside
//! `thing`. `OwlLean.Refutations.adequacy_needs_ind_axioms` refutes the
//! left-to-right direction of adequacy outright with the empty ontology and
//! the axiom `⊤(a)`: the countermodel `Mbad` sends the name to a literal. The
//! fix is `indAxioms`, i.e. `Thing(a)` for every individual name. This was a
//! real defect found while proving the theorem, not a hypothetical.
//!
//! How this file handles it: [`FolProblem::ind_axioms`] emits `thing(i)` for
//! every individual in the signature, and the signature is built from exactly
//! the individual names occurring in the exported axioms and in the goal. The
//! theorem's hypothesis is `∀ x : S.Ind, x ∈ inds`, which holds here because
//! `S.Ind` IS the occurring set by construction. (`owl-lean`'s README lists
//! weakening that hypothesis to "the occurring vocabulary" as open work; the
//! exporter sits on the side of the gap where the hypothesis is satisfied.)
//! `tests/fol_translation_correspondence_test.rs` reconstructs the `Mbad`
//! scenario and fails if the typing axiom is ever absent.
//!
//! # What an ATP's answer is worth
//!
//! Nothing here certifies anything. A derivation certificate from the
//! forward-chaining reasoner can be checked because `lean/` holds a checker
//! whose soundness is a theorem (decision 0002). A superposition refutation
//! cannot: checking one needs a verified first-order calculus with
//! unification, which does not exist in core Lean. So when E or Vampire says
//! "Theorem", that is an ORACLE OPINION, exactly like pyshacl's verdict in
//! `tools/shacl_differential.py`, and `tools/fol_differential.py` reports
//! disagreement rather than adjudicating it. The word "proved" does not appear
//! next to an ATP verdict anywhere in this codebase.
//!
//! # The quoting trap, which points opposite ways in the two syntaxes
//!
//! [`fof::quoted`] uses SINGLE-quoted TPTP atoms and [`clif::enclosed`] uses
//! DOUBLE-quoted CLIF enclosed names. That is not an inconsistency and it is
//! the reverse of what most people assume, so both halves are written down.
//!
//! In TPTP a single-quoted atom is an ordinary constant, and a DOUBLE-quoted
//! string is a "distinct object", which is pairwise unequal to every other
//! distinct object by fiat. Had the TPTP printer used double quotes for IRIs it
//! would have silently asserted that all individuals are pairwise distinct.
//! OWL has no unique name assumption, so that would make every `owl:sameAs`
//! export unsound, and the file would still parse and still prove things.
//!
//! In CLIF the polarity flips. A single-quoted string is an INTERPRETED name
//! that denotes itself, and a double-quoted enclosed name is an ordinary
//! interpretable name, which is why A.2.2.4 recommends the enclosed-name
//! syntax for writing IRIs. Using single quotes for IRIs there would take the
//! text out of the subdialect A.4.2 calls exactly semantically conformant.
//!
//! So: single quotes in TPTP, double quotes in CLIF, for the same IRI, for
//! opposite reasons.
//!
//! # Several serialisers, one translation
//!
//! [`Form`] is computed once. [`fof`], [`clif`], [`cgif`], [`smtlib`] and
//! [`ladr`] are renderings of it and contain no OWL-specific logic at all;
//! each is a fold over `Form`. Two of them are Common Logic dialects: ISO/IEC
//! 24707 defines CLIF, CGIF and XCL, and emitting one and calling that Common
//! Logic support is a partial claim. CGIF is emitted in its CORE dialect and
//! in the compact sub-dialect clause 7.1.1 names; see [`cgif`]. CLIF is
//! restricted to the first-order-equivalent fragment of Common Logic on
//! purpose: no sequence markers, fixed arity at every position, no
//! quantification into a predicate position. Common Logic is NOT plain
//! first-order logic, and the adequacy theorem is about a translation into
//! plain first-order logic, so anything outside that fragment would sit
//! outside the theorem. [`Form`] cannot express the excluded constructs, which
//! is the structural reason the restriction holds; `clif_uses_only_fol_fragment`
//! in the correspondence test is the check that it still holds.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

// ── The target logic: OwlLean/FOL/Basic.lean ────────────────────────────────

/// A first-order term. Mirrors `OwlLean.FOL.Term`: a `Nat`-indexed variable or
/// a constant, and constants are individual names (`FSig.Const := S.Ind`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Term {
    Var(u32),
    Const(String),
}

/// Unary predicate symbols. Mirrors `OwlLean.OwlP1`.
///
/// The four arms are a disjoint sum in Lean and are disjoint symbols here, so
/// an IRI used as both a class and a datatype yields two unrelated predicates.
/// That is what the Structural Specification's separation of entity kinds
/// means, and it is what `Sig` encodes by giving each kind its own type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum P1 {
    Cls(String),
    Dt(String),
    Thing,
    Lit,
}

/// Binary predicate symbols. Mirrors `OwlLean.OwlP2`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum P2 {
    Op(String),
    Dp(String),
}

/// A first-order formula. Mirrors `OwlLean.FOL.Form` constructor for
/// constructor, including the explicit `Nat` on each binder: `owl-lean` uses
/// named variables rather than de Bruijn indices precisely so that freshness
/// is a proof obligation and not a silent convention, and an exporter that
/// renumbered would be emitting a different formula.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Form {
    App1(P1, Term),
    App2(P2, Term, Term),
    Eq(Term, Term),
    Tru,
    Fls,
    Neg(Box<Form>),
    And(Box<Form>, Box<Form>),
    Or(Box<Form>, Box<Form>),
    Imp(Box<Form>, Box<Form>),
    All(u32, Box<Form>),
    Ex(u32, Box<Form>),
}

impl Form {
    fn neg(f: Form) -> Form {
        Form::Neg(Box::new(f))
    }
    fn and(f: Form, g: Form) -> Form {
        Form::And(Box::new(f), Box::new(g))
    }
    fn or(f: Form, g: Form) -> Form {
        Form::Or(Box::new(f), Box::new(g))
    }
    fn imp(f: Form, g: Form) -> Form {
        Form::Imp(Box::new(f), Box::new(g))
    }
    fn all(n: u32, f: Form) -> Form {
        Form::All(n, Box::new(f))
    }
    fn ex(n: u32, f: Form) -> Form {
        Form::Ex(n, Box::new(f))
    }

    /// `OwlLean.FOL.Form.conj`. Right-associated, `[]` is `tru`, `[f]` is `f`.
    /// The shape matters: `conj [a, b, c]` is `and a (and b c)` and not
    /// `and (and a b) c`, and the correspondence test pins the difference.
    fn conj(mut fs: Vec<Form>) -> Form {
        match fs.len() {
            0 => Form::Tru,
            1 => fs.pop().expect("length checked"),
            _ => {
                let head = fs.remove(0);
                Form::and(head, Form::conj(fs))
            }
        }
    }

    /// `OwlLean.FOL.Form.disj`.
    fn disj(mut fs: Vec<Form>) -> Form {
        match fs.len() {
            0 => Form::Fls,
            1 => fs.pop().expect("length checked"),
            _ => {
                let head = fs.remove(0);
                Form::or(head, Form::disj(fs))
            }
        }
    }

    /// Every individual name occurring in the formula. Used to build the
    /// signature's individual vocabulary, which is what `indAxioms` ranges
    /// over and what the adequacy theorem's `hinds` hypothesis is about.
    fn individuals(&self, out: &mut BTreeSet<String>) {
        fn t(term: &Term, out: &mut BTreeSet<String>) {
            if let Term::Const(a) = term {
                out.insert(a.clone());
            }
        }
        match self {
            Form::App1(_, x) => t(x, out),
            Form::App2(_, x, y) | Form::Eq(x, y) => {
                t(x, out);
                t(y, out);
            }
            Form::Tru | Form::Fls => {}
            Form::Neg(f) | Form::All(_, f) | Form::Ex(_, f) => f.individuals(out),
            Form::And(f, g) | Form::Or(f, g) | Form::Imp(f, g) => {
                f.individuals(out);
                g.individuals(out);
            }
        }
    }
}

// Smart constructors for the atoms `Translation.lean` defines by name, kept
// with the same names so the two files read alike.
fn v(n: u32) -> Term {
    Term::Var(n)
}
fn k(a: &str) -> Term {
    Term::Const(a.to_string())
}
fn cls(a: &str, t: Term) -> Form {
    Form::App1(P1::Cls(a.to_string()), t)
}
fn dt(d: &str, t: Term) -> Form {
    Form::App1(P1::Dt(d.to_string()), t)
}
fn thing(t: Term) -> Form {
    Form::App1(P1::Thing, t)
}
fn lit(t: Term) -> Form {
    Form::App1(P1::Lit, t)
}
fn op(r: &str, t: Term, u: Term) -> Form {
    Form::App2(P2::Op(r.to_string()), t, u)
}
fn dp(p: &str, t: Term, u: Term) -> Form {
    Form::App2(P2::Dp(p.to_string()), t, u)
}

/// `OwlLean.exObj`: guarded existential over the object domain.
fn ex_obj(n: u32, f: Form) -> Form {
    Form::ex(n, Form::and(thing(v(n)), f))
}
/// `OwlLean.allObj`: guarded universal over the object domain.
fn all_obj(n: u32, f: Form) -> Form {
    Form::all(n, Form::imp(thing(v(n)), f))
}
/// `OwlLean.exDat`.
fn ex_dat(n: u32, f: Form) -> Form {
    Form::ex(n, Form::and(lit(v(n)), f))
}
/// `OwlLean.allDat`.
fn all_dat(n: u32, f: Form) -> Form {
    Form::all(n, Form::imp(lit(v(n)), f))
}

/// `OwlLean.substT`.
fn subst_t(x: u32, u: &Term, t: &Term) -> Term {
    match t {
        Term::Var(m) if *m == x => u.clone(),
        other => other.clone(),
    }
}

/// `OwlLean.substVar`. Capture-avoiding for the constant terms it is used
/// with: a binder that rebinds the index stops the descent.
fn subst_var(x: u32, u: &Term, f: &Form) -> Form {
    match f {
        Form::App1(p, t) => Form::App1(p.clone(), subst_t(x, u, t)),
        Form::App2(p, t, w) => Form::App2(p.clone(), subst_t(x, u, t), subst_t(x, u, w)),
        Form::Eq(t, w) => Form::Eq(subst_t(x, u, t), subst_t(x, u, w)),
        Form::Tru => Form::Tru,
        Form::Fls => Form::Fls,
        Form::Neg(g) => Form::neg(subst_var(x, u, g)),
        Form::And(g, h) => Form::and(subst_var(x, u, g), subst_var(x, u, h)),
        Form::Or(g, h) => Form::or(subst_var(x, u, g), subst_var(x, u, h)),
        Form::Imp(g, h) => Form::imp(subst_var(x, u, g), subst_var(x, u, h)),
        Form::All(n, g) => {
            if *n == x {
                Form::All(*n, g.clone())
            } else {
                Form::all(*n, subst_var(x, u, g))
            }
        }
        Form::Ex(n, g) => {
            if *n == x {
                Form::Ex(*n, g.clone())
            } else {
                Form::ex(*n, subst_var(x, u, g))
            }
        }
    }
}

/// `OwlLean.mkEx`.
fn mk_ex(ns: &[u32], f: Form) -> Form {
    match ns.split_first() {
        None => f,
        Some((n, rest)) => ex_obj(*n, mk_ex(rest, f)),
    }
}

/// `OwlLean.mkAll`.
fn mk_all(ns: &[u32], f: Form) -> Form {
    match ns.split_first() {
        None => f,
        Some((n, rest)) => all_obj(*n, mk_all(rest, f)),
    }
}

/// `OwlLean.distinctPairs`. The order is the Lean order: for `n :: ns`, every
/// pair `(n, m)` with `m` in `ns` first, then recursively over `ns`.
fn distinct_pairs(ys: &[u32]) -> Vec<Form> {
    match ys.split_first() {
        None => Vec::new(),
        Some((n, rest)) => {
            let mut out: Vec<Form> = rest
                .iter()
                .map(|m| Form::neg(Form::Eq(v(*n), v(*m))))
                .collect();
            out.extend(distinct_pairs(rest));
            out
        }
    }
}

/// `OwlLean.somePairEq`.
fn some_pair_eq(ys: &[u32]) -> Vec<Form> {
    match ys.split_first() {
        None => Vec::new(),
        Some((n, rest)) => {
            let mut out: Vec<Form> = rest.iter().map(|m| Form::Eq(v(*n), v(*m))).collect();
            out.extend(some_pair_eq(rest));
            out
        }
    }
}

// ── The source logic: OwlLean/Syntax.lean ───────────────────────────────────

/// Class expressions. Mirrors `OwlLean.Concept`. `MinCard`/`MaxCard` are the
/// QUALIFIED forms; the unqualified ones are recovered with `Top` as filler,
/// exactly as in the Lean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Concept {
    Top,
    Bot,
    Atom(String),
    Inter(Box<Concept>, Box<Concept>),
    Union(Box<Concept>, Box<Concept>),
    Compl(Box<Concept>),
    OneOf(Vec<String>),
    Some_(String, Box<Concept>),
    All_(String, Box<Concept>),
    HasVal(String, String),
    HasSelf(String),
    MinCard(u32, String, Box<Concept>),
    MaxCard(u32, String, Box<Concept>),
    DataSome(String, String),
    DataAll(String, String),
}

/// Object property expressions. Mirrors `OwlLean.OPE`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ope {
    Named(String),
    Inv(String),
}

/// Axioms. Mirrors `OwlLean.Axiom`, all twenty-two forms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwlAxiom {
    SubClass(Concept, Concept),
    EquivClass(Concept, Concept),
    DisjointWith(Concept, Concept),
    SubOProp(Ope, Ope),
    OPropDomain(Ope, Concept),
    OPropRange(Ope, Concept),
    DPropDomain(String, Concept),
    DPropRange(String, String),
    Transitive(String),
    Symmetric(String),
    Asymmetric(String),
    Reflexive(String),
    Irreflexive(String),
    Functional(String),
    InvFunctional(String),
    InverseOf(String, String),
    PropDisjoint(String, String),
    Chain(Vec<String>, String),
    ClassAssert(Concept, String),
    OPropAssert(String, String, String),
    SameAs(String, String),
    DifferentFrom(String, String),
}

// ── The translation: OwlLean/Translation.lean ───────────────────────────────

/// A violation of the freshness side condition the adequacy theorem carries.
///
/// `tr_bridge` holds only under `Fresh n x`, i.e. `x < n`. This error exists
/// so that a future edit to a counter fails loudly rather than emitting a
/// formula in which the subject variable has been captured. See
/// `OwlLean.Refutations.tr_bridge_needs_freshness`, the machine-checked
/// countermodel for exactly this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotFresh {
    pub subject: u32,
    pub counter: u32,
}

impl std::fmt::Display for NotFresh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "freshness violated: subject variable X{} is not below the counter {}. \
             OwlLean.tr_bridge holds only under `Fresh n x` (x < n); without it the \
             existential `tr` allocates CAPTURES the subject variable, and \
             OwlLean.Refutations.tr_bridge_needs_freshness is the countermodel",
            self.subject, self.counter
        )
    }
}

impl std::error::Error for NotFresh {}

/// The translation. Every method is a transcription of the Lean function named
/// in its doc comment; nothing here is an optimisation or a normalisation of
/// it, because the emitted formula has to be the formula the theorem is about
/// and not one equivalent to it.
pub struct Translation;

impl Translation {
    /// `OwlLean.tr` under the theorem's freshness side condition.
    ///
    /// Callers must satisfy `subject < counter`. The unchecked body is
    /// [`Translation::concept`], which exists only because the recursive calls
    /// inside it maintain the invariant themselves (`tr_mono`: the counter
    /// never decreases) and re-checking on every node would be noise.
    pub fn concept_fresh(c: &Concept, subject: u32, counter: u32) -> Result<(Form, u32), NotFresh> {
        if subject >= counter {
            return Err(NotFresh { subject, counter });
        }
        Ok(Self::concept(c, subject, counter))
    }

    /// `OwlLean.tr`: translate a class expression at subject variable `x` with
    /// next free index `c`, returning the formula and the next free index.
    fn concept(c: &Concept, x: u32, counter: u32) -> (Form, u32) {
        match c {
            Concept::Top => (Form::Tru, counter),
            Concept::Bot => (Form::Fls, counter),
            Concept::Atom(a) => (cls(a, v(x)), counter),
            Concept::Inter(p, q) => {
                let (f, c1) = Self::concept(p, x, counter);
                let (g, c2) = Self::concept(q, x, c1);
                (Form::and(f, g), c2)
            }
            Concept::Union(p, q) => {
                let (f, c1) = Self::concept(p, x, counter);
                let (g, c2) = Self::concept(q, x, c1);
                (Form::or(f, g), c2)
            }
            Concept::Compl(p) => {
                let (f, c1) = Self::concept(p, x, counter);
                (Form::neg(f), c1)
            }
            Concept::OneOf(as_) => (
                Form::disj(as_.iter().map(|a| Form::Eq(v(x), k(a))).collect()),
                counter,
            ),
            Concept::Some_(r, p) => {
                let y = counter;
                let (f, c1) = Self::concept(p, y, counter + 1);
                (ex_obj(y, Form::and(op(r, v(x), v(y)), f)), c1)
            }
            Concept::All_(r, p) => {
                let y = counter;
                let (f, c1) = Self::concept(p, y, counter + 1);
                (all_obj(y, Form::imp(op(r, v(x), v(y)), f)), c1)
            }
            Concept::HasVal(r, a) => (op(r, v(x), k(a)), counter),
            Concept::HasSelf(r) => (op(r, v(x), v(x)), counter),
            Concept::DataSome(p, t) => (
                ex_dat(
                    counter,
                    Form::and(dp(p, v(x), v(counter)), dt(t, v(counter))),
                ),
                counter + 1,
            ),
            Concept::DataAll(p, t) => (
                all_dat(
                    counter,
                    Form::imp(dp(p, v(x), v(counter)), dt(t, v(counter))),
                ),
                counter + 1,
            ),
            // The two cases LATIN's OWL2toFOL.elf has commented out, and the
            // ones 37.2% of constrained real ontologies and gchq/HQDM's 357
            // cardinality-bearing restrictions depend on.
            Concept::MinCard(n, r, p) => {
                let ys: Vec<u32> = (0..*n).map(|i| counter + i).collect();
                let (bodies, c1) = Self::concept_list(p, &ys, counter + n);
                let mut parts = distinct_pairs(&ys);
                parts.extend(
                    ys.iter()
                        .zip(bodies)
                        .map(|(y, f)| Form::and(op(r, v(x), v(*y)), f)),
                );
                (mk_ex(&ys, Form::conj(parts)), c1)
            }
            Concept::MaxCard(n, r, p) => {
                let ys: Vec<u32> = (0..=*n).map(|i| counter + i).collect();
                let (bodies, c1) = Self::concept_list(p, &ys, counter + n + 1);
                let body: Vec<Form> = ys
                    .iter()
                    .zip(bodies)
                    .map(|(y, f)| Form::and(op(r, v(x), v(*y)), f))
                    .collect();
                (
                    mk_all(
                        &ys,
                        Form::imp(Form::conj(body), Form::disj(some_pair_eq(&ys))),
                    ),
                    c1,
                )
            }
        }
    }

    /// `OwlLean.tr.trList`: translate one concept once per subject variable,
    /// threading the freshness counter left to right.
    fn concept_list(p: &Concept, ys: &[u32], counter: u32) -> (Vec<Form>, u32) {
        match ys.split_first() {
            None => (Vec::new(), counter),
            Some((y, rest)) => {
                let (f, c1) = Self::concept(p, *y, counter);
                let (mut fs, c2) = Self::concept_list(p, rest, c1);
                fs.insert(0, f);
                (fs, c2)
            }
        }
    }

    /// `OwlLean.trAx.ope`.
    fn ope(r: &Ope, i: u32, j: u32) -> Form {
        match r {
            Ope::Named(r) => op(r, v(i), v(j)),
            Ope::Inv(r) => op(r, v(j), v(i)),
        }
    }

    /// `OwlLean.trAx`: translate an axiom to a closed formula.
    ///
    /// The counters are `trAx`'s own and are not a choice this file makes.
    /// Each is routed through [`Translation::concept_fresh`], so `0 < 2`,
    /// `1 < 2` and `0 < 1` are checked at run time rather than asserted in a
    /// comment.
    pub fn axiom(a: &OwlAxiom) -> Result<Form, NotFresh> {
        Ok(match a {
            OwlAxiom::SubClass(c, d) => {
                let (f, n) = Self::concept_fresh(c, 0, 2)?;
                let (g, _) = Self::concept_fresh(d, 0, n)?;
                all_obj(0, Form::imp(f, g))
            }
            OwlAxiom::EquivClass(c, d) => {
                let (f, n) = Self::concept_fresh(c, 0, 2)?;
                let (g, _) = Self::concept_fresh(d, 0, n)?;
                all_obj(
                    0,
                    Form::and(Form::imp(f.clone(), g.clone()), Form::imp(g, f)),
                )
            }
            OwlAxiom::DisjointWith(c, d) => {
                let (f, n) = Self::concept_fresh(c, 0, 2)?;
                let (g, _) = Self::concept_fresh(d, 0, n)?;
                all_obj(0, Form::neg(Form::and(f, g)))
            }
            OwlAxiom::SubOProp(r, s) => all_obj(
                0,
                all_obj(1, Form::imp(Self::ope(r, 0, 1), Self::ope(s, 0, 1))),
            ),
            OwlAxiom::OPropDomain(r, c) => {
                let (f, _) = Self::concept_fresh(c, 0, 2)?;
                all_obj(0, all_obj(1, Form::imp(Self::ope(r, 0, 1), f)))
            }
            OwlAxiom::OPropRange(r, c) => {
                let (f, _) = Self::concept_fresh(c, 1, 2)?;
                all_obj(0, all_obj(1, Form::imp(Self::ope(r, 0, 1), f)))
            }
            OwlAxiom::DPropDomain(p, c) => {
                let (f, _) = Self::concept_fresh(c, 0, 2)?;
                all_obj(0, all_dat(1, Form::imp(dp(p, v(0), v(1)), f)))
            }
            OwlAxiom::DPropRange(p, t) => all_obj(
                0,
                all_dat(1, Form::imp(dp(p, v(0), v(1)), dt(t, v(1)))),
            ),
            OwlAxiom::Transitive(r) => all_obj(
                0,
                all_obj(
                    1,
                    all_obj(
                        2,
                        Form::imp(
                            Form::and(op(r, v(0), v(1)), op(r, v(1), v(2))),
                            op(r, v(0), v(2)),
                        ),
                    ),
                ),
            ),
            OwlAxiom::Symmetric(r) => all_obj(
                0,
                all_obj(1, Form::imp(op(r, v(0), v(1)), op(r, v(1), v(0)))),
            ),
            OwlAxiom::Asymmetric(r) => all_obj(
                0,
                all_obj(
                    1,
                    Form::imp(op(r, v(0), v(1)), Form::neg(op(r, v(1), v(0)))),
                ),
            ),
            OwlAxiom::Reflexive(r) => all_obj(0, op(r, v(0), v(0))),
            OwlAxiom::Irreflexive(r) => all_obj(0, Form::neg(op(r, v(0), v(0)))),
            OwlAxiom::Functional(r) => all_obj(
                0,
                all_obj(
                    1,
                    all_obj(
                        2,
                        Form::imp(
                            Form::and(op(r, v(0), v(1)), op(r, v(0), v(2))),
                            Form::Eq(v(1), v(2)),
                        ),
                    ),
                ),
            ),
            OwlAxiom::InvFunctional(r) => all_obj(
                0,
                all_obj(
                    1,
                    all_obj(
                        2,
                        Form::imp(
                            Form::and(op(r, v(0), v(2)), op(r, v(1), v(2))),
                            Form::Eq(v(0), v(1)),
                        ),
                    ),
                ),
            ),
            OwlAxiom::InverseOf(r, s) => all_obj(
                0,
                all_obj(
                    1,
                    Form::and(
                        Form::imp(op(r, v(0), v(1)), op(s, v(1), v(0))),
                        Form::imp(op(s, v(1), v(0)), op(r, v(0), v(1))),
                    ),
                ),
            ),
            OwlAxiom::PropDisjoint(r, s) => all_obj(
                0,
                all_obj(
                    1,
                    Form::neg(Form::and(op(r, v(0), v(1)), op(s, v(0), v(1)))),
                ),
            ),
            // `OwlLean.trAx.chainForm`: a chain of length n needs n+1 variables.
            OwlAxiom::Chain(rs, s) => {
                let n = rs.len() as u32;
                let vars: Vec<u32> = (0..=n).collect();
                let body: Vec<Form> = rs
                    .iter()
                    .enumerate()
                    .map(|(i, r)| op(r, v(i as u32), v(i as u32 + 1)))
                    .collect();
                mk_all(
                    &vars,
                    Form::imp(Form::conj(body), op(s, v(0), v(n))),
                )
            }
            // The `thing(a)` conjunct here is NOT the individual typing axiom.
            // It is part of `trAx` itself, and it is what `Mbad` refutes when
            // the typing axioms are absent from the background theory.
            OwlAxiom::ClassAssert(c, a) => {
                let (f, _) = Self::concept_fresh(c, 0, 1)?;
                Form::and(thing(k(a)), subst_var(0, &k(a), &f))
            }
            OwlAxiom::OPropAssert(r, a, b) => op(r, k(a), k(b)),
            OwlAxiom::SameAs(a, b) => Form::Eq(k(a), k(b)),
            OwlAxiom::DifferentFrom(a, b) => Form::neg(Form::Eq(k(a), k(b))),
        })
    }

    /// `OwlLean.background`: the two domains are disjoint, and the object
    /// domain is non-empty.
    ///
    /// These are the Direct Semantics' side conditions on an interpretation
    /// and MUST accompany any translated ontology. Note that both quantifiers
    /// are UNGUARDED in the Lean (`.all 0`, `.ex 0`, not `allObj`/`exObj`),
    /// which is exactly what makes the second one say "some element of the
    /// domain is an object" rather than a tautology.
    ///
    /// `owl-lean` also records that the disjointness axiom is never USED by
    /// the proof: `ofStruc` cuts the single first-order domain into two Lean
    /// types whether or not they overlap as sets, and every quantifier `tr`
    /// emits is guarded. It is discharged, not consumed. It is emitted anyway,
    /// because it is part of the theory the theorem is stated about and
    /// because a consumer reading the file has no other way to learn that the
    /// two sorts are meant to be disjoint.
    pub fn background() -> Vec<Form> {
        vec![
            Form::all(0, Form::neg(Form::and(thing(v(0)), lit(v(0))))),
            Form::ex(0, thing(v(0))),
        ]
    }

    /// `OwlLean.indAxioms`: `Thing(a)` for each individual name.
    ///
    /// The absence of these was a real defect found while proving adequacy,
    /// refuted by `OwlLean.Refutations.adequacy_needs_ind_axioms`. Real
    /// OWL-to-FOL tools emit them; the theorem says what it costs to leave
    /// them out.
    pub fn ind_axioms(inds: &BTreeSet<String>) -> Vec<Form> {
        inds.iter().map(|a| thing(k(a))).collect()
    }
}

// ── One problem, two serialisations ─────────────────────────────────────────

/// The first-order content of an export: a background theory, the individual
/// typing axioms, the translated ontology, and optionally one conjecture.
///
/// Computed once. [`FolProblem::to_tptp`] and [`FolProblem::to_clif`] are
/// renderings of it and share every formula; neither contains OWL-specific
/// logic.
#[derive(Clone, Debug)]
pub struct FolProblem {
    pub background: Vec<Form>,
    pub ind_axioms: Vec<Form>,
    /// Translated ontology axioms, each paired with a short label naming the
    /// OWL axiom form it came from, so a reader of the file can find the
    /// source construct without re-deriving it.
    pub axioms: Vec<(String, Form)>,
    pub conjecture: Option<(String, Form)>,
    /// Individual names the typing axioms range over, in a fixed order.
    pub individuals: BTreeSet<String>,
}

/// One line of the model-finding problem: a role, a diagnostic label, and the
/// formula the solver is asked about and the checker then checks.
#[derive(Clone, Debug)]
pub struct CheckerEntry {
    /// `axiom` or `goal_negated`, and nothing else.
    pub role: &'static str,
    /// The emitter's own label. DIAGNOSTIC ONLY: not digested, and no theorem
    /// looks at it.
    pub label: String,
    pub form: Form,
}

/// The symbols a problem uses, by arity, in a fixed order.
///
/// Three separate sets rather than one, because `Fol.FinModel` has three
/// separate fields: `c:X` as a unary predicate and `op:X` as a binary one
/// could not collide even without the [`sym`] prefixes.
#[derive(Clone, Debug, Default)]
pub struct Vocabulary {
    pub unary: BTreeSet<String>,
    pub binary: BTreeSet<String>,
    pub consts: BTreeSet<String>,
}

impl Vocabulary {
    pub fn len(&self) -> usize {
        self.unary.len() + self.binary.len() + self.consts.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn collect(f: &Form, v: &mut Vocabulary) {
    fn t(term: &Term, v: &mut Vocabulary) {
        if let Term::Const(a) = term {
            v.consts.insert(sym::constant(a));
        }
    }
    match f {
        Form::App1(p, x) => {
            v.unary.insert(sym::p1(p));
            t(x, v);
        }
        Form::App2(p, x, y) => {
            v.binary.insert(sym::p2(p));
            t(x, v);
            t(y, v);
        }
        Form::Eq(x, y) => {
            t(x, v);
            t(y, v);
        }
        Form::Tru | Form::Fls => {}
        Form::Neg(g) | Form::All(_, g) | Form::Ex(_, g) => collect(g, v),
        Form::And(g, h) | Form::Or(g, h) | Form::Imp(g, h) => {
            collect(g, v);
            collect(h, v);
        }
    }
}

impl FolProblem {
    /// Build the problem for an ontology and an optional goal.
    ///
    /// The individual vocabulary is collected from the translated formulas
    /// themselves, including the goal's, so `indAxioms` covers exactly the
    /// constants that can appear. That is what makes the adequacy theorem's
    /// `hinds : ∀ x : S.Ind, x ∈ inds` hold for the signature this export
    /// defines.
    pub fn build(
        axioms: &[OwlAxiom],
        goal: Option<&OwlAxiom>,
    ) -> Result<FolProblem, NotFresh> {
        let mut translated = Vec::with_capacity(axioms.len());
        let mut individuals = BTreeSet::new();
        for a in axioms {
            let f = Translation::axiom(a)?;
            f.individuals(&mut individuals);
            translated.push((axiom_label(a).to_string(), f));
        }
        let conjecture = match goal {
            None => None,
            Some(g) => {
                let f = Translation::axiom(g)?;
                f.individuals(&mut individuals);
                Some((axiom_label(g).to_string(), f))
            }
        };
        Ok(FolProblem {
            background: Translation::background(),
            ind_axioms: Translation::ind_axioms(&individuals),
            axioms: translated,
            conjecture,
            individuals,
        })
    }

    /// Every formula in the problem, in emission order, paired with the role
    /// it plays. Both serialisers walk exactly this, which is the mechanical
    /// reason they cannot disagree about which formulas are in the file.
    pub fn formulas(&self) -> Vec<(String, &'static str, &Form)> {
        let mut out: Vec<(String, &'static str, &Form)> = Vec::new();
        for (i, f) in self.background.iter().enumerate() {
            out.push((format!("background_{}", i + 1), "axiom", f));
        }
        for (i, f) in self.ind_axioms.iter().enumerate() {
            out.push((format!("ind_typing_{}", i + 1), "axiom", f));
        }
        for (i, (label, f)) in self.axioms.iter().enumerate() {
            out.push((format!("owl_{}_{}", i + 1, label), "axiom", f));
        }
        if let Some((label, f)) = &self.conjecture {
            out.push((format!("goal_{label}"), "conjecture", f));
        }
        out
    }

    /// Every formula as the MODEL-FINDING half of the pipeline sees it.
    ///
    /// This is [`FolProblem::formulas`] with one difference, and the
    /// difference is the whole point of the model direction: a conjecture
    /// becomes a `goal_negated` line carrying `¬φ`. A countermodel to
    /// `Γ ⊨ φ` is a model of `Γ ∪ {¬φ}`, so the negation happens ONCE, here,
    /// and neither the solver nor `oo-folmodel` negates anything again.
    ///
    /// The SMT-LIB file, the LADR file and `problem.tsv` are all folds over
    /// this one list. That is the mechanical reason the solver cannot be asked
    /// a different question from the one the checker then checks, and it is
    /// the same argument [`FolProblem::formulas`] makes for TPTP and CLIF.
    pub fn checker_entries(&self) -> Vec<CheckerEntry> {
        let mut out = Vec::new();
        for (label, role, f) in self.formulas() {
            match role {
                "conjecture" => out.push(CheckerEntry {
                    role: "goal_negated",
                    label,
                    form: Form::neg(f.clone()),
                }),
                _ => out.push(CheckerEntry { role: "axiom", label, form: f.clone() }),
            }
        }
        out
    }

    /// The symbols the problem actually uses, by arity.
    ///
    /// Collected from [`FolProblem::checker_entries`] and therefore including
    /// the negated goal's, because a model file that did not interpret a
    /// symbol occurring only in the goal would fail `Fol.covers`.
    pub fn vocabulary(&self) -> Vocabulary {
        let mut v = Vocabulary::default();
        for e in self.checker_entries() {
            collect(&e.form, &mut v);
        }
        v
    }

    /// `problem.tsv` for `oo-folmodel`, and its digest.
    pub fn to_problem_tsv(&self) -> Result<(String, String), UnwritableSymbol> {
        let entries = self.checker_entries();
        let mut file = String::new();
        let mut canonical: Vec<String> = Vec::with_capacity(entries.len());
        for e in &entries {
            let shown = checkfmt::show_form(&e.form)?;
            let _ = writeln!(file, "{}\t{}\t{}", e.role, e.label, shown);
            canonical.push(format!("{}\t{}", e.role, shown));
        }
        let digest = checkfmt::hex16(checkfmt::fnv1a64(&canonical.join("\n")));
        Ok((file, digest))
    }

    /// SMT-LIB 2, with the goal already negated.
    pub fn to_smtlib(&self, enc: smtlib::SmtEncoding) -> Result<String, UnwritableSymbol> {
        let v = self.vocabulary();
        let mut s = header(";", self.conjecture.is_some(), SMTLIB_STYLE);
        let _ = writeln!(s, "\n(set-logic {})", smtlib::logic(enc));
        let _ = writeln!(s, "(set-option :produce-models true)");
        let _ = writeln!(s, "{}", smtlib::sort_decl(enc));
        for p in &v.unary {
            let _ = writeln!(s, "(declare-fun {} (U) Bool)", smtlib::symbol(p)?);
        }
        for p in &v.binary {
            let _ = writeln!(s, "(declare-fun {} (U U) Bool)", smtlib::symbol(p)?);
        }
        for c in &v.consts {
            let _ = writeln!(s, "(declare-fun {} () U)", smtlib::symbol(c)?);
        }
        for e in self.checker_entries() {
            let _ = writeln!(s, "; {} ({})", e.label, e.role);
            let _ = writeln!(s, "(assert {})", smtlib::form(&e.form)?);
        }
        let _ = writeln!(s, "(check-sat)");
        let _ = writeln!(s, "(get-model)");
        Ok(s)
    }

    /// TPTP FOF.
    pub fn to_tptp(&self) -> String {
        let mut s = String::new();
        s.push_str(&header("%", self.conjecture.is_some(), TPTP_STYLE));
        for (name, role, f) in self.formulas() {
            let _ = writeln!(s, "fof({name}, {role}, {}).", fof::form(f));
        }
        s
    }

    /// ISO/IEC 24707 CLIF, restricted to the first-order-equivalent fragment.
    ///
    /// Everything in the file is CLIF: there are no lexical comments, because
    /// the standard defines `cl:comment` as a reserved element and this
    /// project could not establish a `//` or `/* */` line-comment convention
    /// from the standard's own text. The header therefore rides on
    /// `cl:comment` attached to `(and)`, the empty conjunction, which ISO/IEC
    /// 24707 A.2.3.7 makes the truth value TRUE. It is true in every model, so
    /// it adds nothing to the theory and the file still denotes exactly the
    /// formula set [`FolProblem::formulas`] lists.
    pub fn to_clif(
        &self,
        dialect: ClifDialect,
        comments: ClifComments,
        name: &str,
    ) -> String {
        let mut s = String::new();
        // A NAMED text. All 227 COLORE texts are named, Macleod rejects an
        // unnamed one with "Error in ontology: bad URI", and py-typedlogic
        // otherwise reports the first comment as the theory's name.
        let _ = writeln!(s, "({} {}", dialect.text_op(), clif::text_name(name));
        let _ = writeln!(
            s,
            "  {}",
            clif::standalone_comment(dialect, &header_text(self.conjecture.is_some(), CLIF_STYLE))
        );
        for (label, role, f) in self.formulas() {
            match comments {
                ClifComments::Standalone => {
                    let _ = writeln!(
                        s,
                        "  {}",
                        clif::standalone_comment(dialect, &format!("{label} ({role})"))
                    );
                    let _ = writeln!(s, "  {}", clif::form(f));
                }
                ClifComments::Wrapped => {
                    let _ = writeln!(
                        s,
                        "  {}",
                        clif::commented(dialect, &format!("{label} ({role})"), f)
                    );
                }
            }
        }
        s.push_str(")\n");
        s
    }

    /// ISO/IEC 24707 CGIF, core dialect, in a compact sub-dialect.
    ///
    /// The third of Common Logic's three dialects to be emitted here, over the
    /// same [`FolProblem::formulas`] the TPTP and CLIF writers walk. The text
    /// is a B.2.11 `text`: `"[", [comment], "Proposition", ":", [CGname], CG,
    /// [endComment], "]"`, which is what Table B.1's row E20 maps a named
    /// `(cl:text N …)` to, so the CGIF file is the standard's own image of the
    /// CLIF file rather than a second convention.
    ///
    /// Unlike the CLIF writer this has no dialect flag and no comment-placement
    /// flag. There is one spelling of the CGIF operators, and B.2.4 gives
    /// comments a lexical syntax of their own, so the `cl:comment` trap that
    /// made the CLIF writer's default a measured decision does not arise:
    /// nothing has to be put INSIDE a comment for a label to attach to it.
    pub fn to_cgif(&self, name: &str) -> Result<String, cgif::UnwritableComment> {
        let mut s = String::new();
        let _ = writeln!(s, "[Proposition: {}", cgif::text_name(name));
        let header = cgif::comment(&header_text(self.conjecture.is_some(), CGIF_STYLE))?;
        for line in header.lines() {
            let _ = writeln!(s, "  {line}");
        }
        for (label, role, f) in self.formulas() {
            let _ = writeln!(s, "  {}", cgif::comment(&format!("{label} ({role})"))?);
            let _ = writeln!(s, "  {}", cgif::sentence(f));
        }
        s.push_str("]\n");
        Ok(s)
    }
}

/// The header every serialiser carries, with each line prefixed by the
/// syntax's comment marker. It is not decoration: a file that leaves its own
/// limits to a README gets read without one.
fn header(comment: &str, has_goal: bool, style: HeaderStyle) -> String {
    header_text(has_goal, style)
        .lines()
        .map(|l| {
            if l.is_empty() {
                format!("{comment}\n")
            } else {
                format!("{comment} {l}\n")
            }
        })
        .collect()
}

/// The syntax-specific paragraphs of the header.
///
/// `extra` is what the syntax itself needs a reader to know. `goal` replaces
/// the paragraph about the conjecture, and it has to be swappable rather than
/// shared: TPTP and CLIF carry the goal as a CONJECTURE and a prover negates
/// it internally, while SMT-LIB and LADR carry the NEGATION as an assertion
/// and a `sat` answer is the non-entailment. A reader told the wrong one of
/// those two reads the file backwards.
#[derive(Clone, Copy)]
struct HeaderStyle {
    extra: &'static [&'static str],
    goal: &'static [&'static str],
}

/// What a refutation-style consumer is told: its verdict is an oracle opinion.
const ORACLE_GOAL: &[&str] = &[
    "",
    "A prover's verdict on this file is an ORACLE OPINION, not a certificate.",
    "Checking a superposition refutation needs a verified first-order calculus",
    "with unification, which does not exist in core Lean. Disagreement between",
    "a prover and this engine is a bug in one of the two and nothing here says",
    "which.",
];

/// What a model-finding consumer is told: the goal is already negated, and its
/// SATISFIABILITY answer is the one this repository can certify.
const MODEL_GOAL: &[&str] = &[
    "",
    "THE GOAL IS ALREADY NEGATED IN THIS FILE. A countermodel to `G |= phi` is a",
    "model of `G + {not phi}`, so the negation is asserted here and nothing",
    "downstream negates anything again. A `sat` answer therefore says the",
    "conjecture is NOT entailed, and an `unsat` answer says nothing this",
    "repository will print the word `unsatisfiable` for unless the encoding was",
    "unbounded.",
    "",
    "A MODEL THIS FILE YIELDS CAN BE CERTIFIED; A REFUTATION CANNOT. Hand the",
    "solver's structure to `oo-folmodel` (lean/Fol/) and a machine-checked",
    "theorem, Fol.satisfiable_of_check, turns an accepted structure into",
    "satisfiability of exactly these formulas. Nothing in this repository",
    "certifies the other direction. See docs/decisions/0006.",
];

const TPTP_STYLE: HeaderStyle = HeaderStyle { extra: &[], goal: ORACLE_GOAL };

/// The header, unprefixed, so CLIF can carry it inside a `cl:comment` string.
fn header_text(has_goal: bool, style: HeaderStyle) -> String {
    let mut lines: Vec<&str> = vec![
        "Generated by open-ontologies `fol` export.",
        "",
        "The translation is the one owl-lean's machine-checked adequacy theorem",
        "`OwlLean.adequacy` is about (axioms: propext, Classical.choice, Quot.sound;",
        "no sorry, no Mathlib). The CORRESPONDENCE between this emitter and that Lean",
        "translation is PINNED BY TESTS AND IS NOT ITSELF PROVED.",
        "",
        "`thing` and `lit` are the soft-typing predicates: object positions are",
        "relativised by `thing`, data positions by `lit`. The first two axioms are",
        "owl-lean's `background`; the `ind_typing_*` axioms are its `indAxioms`, and",
        "omitting them refutes adequacy outright (OwlLean.Refutations).",
    ];
    lines.extend(style.extra.iter().copied());
    if has_goal {
        lines.extend(style.goal.iter().copied());
    }
    lines.join("\n")
}

/// The CLIF paragraphs of the header.
const CLIF_NOTES: &[&str] = &{
    [
            "",
            "CLIF RESTRICTION. Common Logic is not plain first-order logic. This text",
            "uses only the first-order-equivalent fragment: no sequence markers, fixed",
            "arity at every position, and no quantification into a predicate position.",
            "Sequence markers are what actually take Common Logic past first order",
            "(ISO/IEC 24707 clause 6.5: the logic with them is not compact), and the",
            "adequacy theorem is about plain first-order logic, so anything outside this",
            "fragment would sit outside the theorem.",
            "",
            "`(and)` with no arguments is truth and `(or)` with no arguments is falsity",
            "(A.2.3.7). CLIF defines no truth constants, and these are its readings.",
            "Names are enclosed names in DOUBLE QUOTES (A.2.2.2 namequote), which A.2.2.4",
            "recommends for writing IRIs as names. The vertical bar is an ordinary name",
            "character in CLIF and is not a quoting construct.",
            "",
            "EXACTLY CONFORMANT SUBDIALECT. ISO/IEC 24707 first edition Annex A.4.2: \"The",
            "subdialect of CLIF which does not use numerals or quoted strings is exactly",
            "semantically conformant\". This text stays in it, so CLIF entailment and",
            "Common Logic entailment coincide here and the adequacy theorem needs no",
            "qualification at the CLIF end. The scope of that claim, exactly: no decimal",
            "numerals and no quoted strings IN SENTENCE POSITIONS. Comment annotations are",
            "the named exception, since a comment text is a quoted string by definition;",
            "the CLIF files ISO hosts for ISO/IEC 21838-2 are in the same position. The",
            "property is checked, not asserted: clif_stays_in_the_exactly_conformant_subdialect",
            "in tests/fol_translation_correspondence_test.rs. Nothing here is claimed about",
            "the second edition's Annex A.3, which this project has not read.",
            "",
            "DIALECT. This file uses one of the two spellings in circulation. `cl:text`",
            "and `cl:comment` are ISO/IEC 24707's reserved tokens and are what ISO",
            "publishes with ISO/IEC 21838-2 (BFO-2020 CLIF). The COLORE repository and",
            "the Macleod toolchain's shipped lexer use `cl-text` and `cl-comment`",
            "instead, and will not read the colon spelling. Re-export with the other",
            "dialect rather than hand-editing.",
    ]
};

const CLIF_STYLE: HeaderStyle = HeaderStyle { extra: CLIF_NOTES, goal: ORACLE_GOAL };

/// The CGIF paragraphs of the header.
const CGIF_NOTES: &[&str] = &[
    "",
    "CGIF, ISO/IEC 24707 Annex B. This is CORE CGIF, not extended CGIF. A core",
    "concept has no type field, so a class membership is an ordinary relation",
    "`(\"c:IRI\" ?X0)` and not `[C: *x]`; B.1.2 gives that reduction itself.",
    "Nothing here uses a type label, a type expression, `@every`, `[If: ...",
    "[Then: ...]]`, `[Either: [Or: ...]]`, `[Equiv: [Iff: ...]]`, a concept in an",
    "arc sequence, or an import. B.4 states that core CGIF is a fully conformant",
    "Common Logic dialect on its own.",
    "",
    "THE COMPACT SUB-DIALECT. Clause 7.1.1: \"A compact sub-dialect is a dialect",
    "that does not recognize sequence markers.\" This text is one, and that is the",
    "restriction that matters: clause 6.5 says Common Logic with sequence markers",
    "\"is not compact, and therefore not first-order\", and the adequacy theorem is",
    "about plain first-order logic. No `[*...x]` and no `?...x` appears, in either",
    "position B.2.5 and B.2.3 allow one. By the same clause this text is also an",
    "unstructured sub-dialect (no titlings, no importation) and a single domain",
    "sub-dialect (no domain restrictions). Two further exclusions have no clause-7",
    "name: no `#?` type label, which B.2.7 says is how CGIF quantifies over",
    "relations, and no actor, which is how B.2.1 writes a function.",
    "",
    "THE ENCODING. `[]` is truth and `~[]` is falsity (B.2.5, B.2.8). Conjunction",
    "is juxtaposition and has no operator (B.2.6). An equation is a coreference",
    "concept `[: ?X0 ?X1]` (B.2.5); CGIF has no `=`. Implication is `~[A ~[B]]`",
    "and disjunction is `~[~[A] ~[B]]`, which are B.3.5's own `ifThen` and",
    "`eitherOr` rewrites into core. A universal is `~[[*X0] ~[...]]`, which is",
    "B.3.7's \"nest of two negations\".",
    "",
    "EVERY BINDER HAS A CONTEXT OF ITS OWN, and every sentence is bracketed.",
    "B.2.10 forbids two defining labels with one name in one context, and the",
    "translation reuses variable indices between an axiom's antecedent and its",
    "consequent. Renumbering them would emit a different formula, so each binder",
    "is given a context instead. No defining label in this text stands in the",
    "scope of another with the same name, so nothing here depends on B.2.10's",
    "shadowing sentence, which contradicts the sentence before it.",
    "",
    "NAMES. B.1.1's `identifier` is letters, digits and underscore, so every IRI",
    "is a DOUBLE-QUOTED enclosed name, the text's own name included. `X0`, `thing`",
    "and `lit` are identifiers. A defining label is `Xn` and a constant begins",
    "`i:`, so the two can never collide, which B.2.10 forbids outright.",
    "",
    "INTERPRETED NAMES. No numeral and no single-quoted string stands anywhere in",
    "this text. A.4.2 states the `exactly semantically conformant` result for",
    "CLIF and Annex B states no analogue for CGIF, so the property is held here",
    "and the label is not borrowed. CGIF needs no exception for its comments: B.2.4",
    "makes a comment a LEXICAL construct, delimited rather than quoted, which is",
    "not what a CLIF `cl:comment` can be. Its own delimiters are the one string a",
    "comment may not contain, and B.2.4 gives no escape for them, so a comment",
    "carrying one is REFUSED by this exporter rather than rewritten.",
];

const CGIF_STYLE: HeaderStyle = HeaderStyle { extra: CGIF_NOTES, goal: ORACLE_GOAL };

/// The SMT-LIB paragraphs.
const SMTLIB_NOTES: &[&str] = &[
    "",
    "SMT-LIB 2. One sort `U`, unary and binary predicates over it, and nullary",
    "constants. There are no function symbols of positive arity, no theories and",
    "no sorts beyond `U`, because `OwlLean.FOL` has none: `FSig` is exactly P1,",
    "P2 and Const.",
    "",
    "`U` is declared one of two ways and the difference decides what an answer",
    "means. `declare-sort` leaves the cardinality open, so `unsat` is real",
    "unsatisfiability. An enumeration datatype `(e0 … e(k-1))` fixes it at k, so",
    "`unsat` establishes only that NO MODEL OF SIZE k EXISTS, which is not",
    "unsatisfiability: `(forall ((x U)) (exists ((y U)) (and (r x y) (not (= x",
    "y))))) ` is unsat at k=1 and sat at k=2.",
    "",
    "`e0 …`, `U` and `X0 …` cannot collide with a translated symbol: every one",
    "of those is `thing`, `lit`, or carries a colon prefix.",
];

const SMTLIB_STYLE: HeaderStyle = HeaderStyle { extra: SMTLIB_NOTES, goal: MODEL_GOAL };

/// The LADR paragraphs.
const LADR_NOTES: &[&str] = &[
    "",
    "LADR, for Mace4. EVERY SYMBOL IN THIS FILE IS MANGLED and the table is",
    "written beside it as `symbols.tsv`: p0… are the unary predicates, r0… the",
    "binary ones, c0… the constants, and x0… are bound variables. Do not read an",
    "IRI back out of this file by hand.",
    "",
    "The mangling is not cosmetic. LADR reads a name whose first letter is in",
    "{u,v,w,x,y,z} as a VARIABLE, so an IRI beginning with one of those would",
    "silently become universally quantified and Mace4 would search a different",
    "problem and report `exhausted` with no error. Measured on LADR 2009-11A:",
    "the input `p0(w0). -p0(k0).` is echoed in Mace4's own CLAUSES FOR SEARCH",
    "block as `p0(x).` and `-p0(k0).`. LADR also has no quoting construct that",
    "survives the colon and the slash an IRI contains.",
    "",
    "Mace4 CLAUSIFIES AND SKOLEMISES, so the model it prints interprets names",
    "this file never declared (f1, and fresh c… it chose itself). Those are",
    "dropped on the way back in, which is taking the reduct of its structure,",
    "and the driver reports the dropped names rather than passing over them.",
    "",
    "Mace4's minimum domain size is 2: `-n 1` is a fatal error, measured. A",
    "one-element model can be found by the SMT-LIB route and not by this one.",
];

const LADR_STYLE: HeaderStyle = HeaderStyle { extra: LADR_NOTES, goal: MODEL_GOAL };

fn axiom_label(a: &OwlAxiom) -> &'static str {
    match a {
        OwlAxiom::SubClass(..) => "subClassOf",
        OwlAxiom::EquivClass(..) => "equivalentClass",
        OwlAxiom::DisjointWith(..) => "disjointWith",
        OwlAxiom::SubOProp(..) => "subObjectPropertyOf",
        OwlAxiom::OPropDomain(..) => "objectPropertyDomain",
        OwlAxiom::OPropRange(..) => "objectPropertyRange",
        OwlAxiom::DPropDomain(..) => "dataPropertyDomain",
        OwlAxiom::DPropRange(..) => "dataPropertyRange",
        OwlAxiom::Transitive(..) => "transitiveProperty",
        OwlAxiom::Symmetric(..) => "symmetricProperty",
        OwlAxiom::Asymmetric(..) => "asymmetricProperty",
        OwlAxiom::Reflexive(..) => "reflexiveProperty",
        OwlAxiom::Irreflexive(..) => "irreflexiveProperty",
        OwlAxiom::Functional(..) => "functionalProperty",
        OwlAxiom::InvFunctional(..) => "inverseFunctionalProperty",
        OwlAxiom::InverseOf(..) => "inverseOf",
        OwlAxiom::PropDisjoint(..) => "propertyDisjointWith",
        OwlAxiom::Chain(..) => "propertyChainAxiom",
        OwlAxiom::ClassAssert(..) => "classAssertion",
        OwlAxiom::OPropAssert(..) => "objectPropertyAssertion",
        OwlAxiom::SameAs(..) => "sameAs",
        OwlAxiom::DifferentFrom(..) => "differentFrom",
    }
}

// ── The namer: one module, four consumers ───────────────────────────────────

/// The seven prefixed images of the Lean's symbol arms. ONE namer.
///
/// `lean/Fol/Syntax.lean` monomorphises `OwlLean.FOL.Form` at `Sym := String`,
/// which flattens `OwlP1`'s four arms, `OwlP2`'s two and `FSig.Const` into one
/// string space. That flattening is sound only if the map is INJECTIVE, and
/// the injectivity is supplied by these prefixes: `thing` and `lit` carry no
/// colon and every other arm carries its own, so the seven images are pairwise
/// disjoint whatever the IRIs are.
///
/// ```text
/// OwlP1.Thing  ->  thing        OwlP2.Op r   ->  op:r
/// OwlP1.Lit    ->  lit          OwlP2.Dp d   ->  dp:d
/// OwlP1.Cls a  ->  c:a          FSig.Const a ->  i:a
/// OwlP1.Dt d   ->  d:d
/// ```
///
/// Four consumers: [`fof`] wraps these in single-quoted TPTP atoms, [`clif`]
/// in double-quoted CLIF enclosed names, [`smtlib`] in `|…|` quoted symbols,
/// and [`checkfmt`] writes them bare into the file the Lean checker reads. The
/// fifth, [`ladr`], does not use them at all: LADR has no quoting construct
/// that survives a colon or a slash, so it mangles instead and keeps a table.
///
/// Writing a second namer by stripping the quotes off the TPTP output is the
/// obvious shortcut and is exactly what this module exists to prevent. If the
/// prefixes ever drift apart between two syntaxes, a class IRI and a datatype
/// IRI with the same spelling become one predicate in one of them.
pub mod sym {
    use super::{P1, P2};

    /// The image of a unary predicate symbol.
    pub fn p1(p: &P1) -> String {
        match p {
            P1::Cls(a) => format!("c:{a}"),
            P1::Dt(d) => format!("d:{d}"),
            P1::Thing => "thing".to_string(),
            P1::Lit => "lit".to_string(),
        }
    }

    /// The image of a binary predicate symbol.
    pub fn p2(p: &P2) -> String {
        match p {
            P2::Op(r) => format!("op:{r}"),
            P2::Dp(d) => format!("dp:{d}"),
        }
    }

    /// The image of an individual name.
    pub fn constant(a: &str) -> String {
        format!("i:{a}")
    }

    /// The characters a symbol may not contain, and why.
    ///
    /// `bare()` strips the angle brackets before an IRI reaches `Term::Const`
    /// or `P1::Cls`, so the symbols here are RAW IRIs out of the graph and
    /// this refusal is live rather than belt and braces. In the checker format
    /// a symbol containing a space re-parses as a DIFFERENT formula (the token
    /// stream splits on spaces) and one containing a tab re-parses as a
    /// different set of fields. The 13 September 2026 audit of this repository
    /// found exactly that truncation class silently disabling SHACL
    /// constraints, so the writer returns an error rather than emitting one.
    pub fn unwritable_char(s: &str) -> Option<char> {
        s.chars().find(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
    }
}

/// A symbol that cannot be written into a target syntax, named with the reason.
///
/// Returned rather than escaped or dropped: a dropped symbol is a different
/// theory and an escaped one is a different symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnwritableSymbol {
    pub symbol: String,
    pub character: char,
    pub syntax: &'static str,
    pub why: &'static str,
}

impl std::fmt::Display for UnwritableSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the symbol {:?} contains {:?}, which cannot be written in {}: {}",
            self.symbol, self.character, self.syntax, self.why
        )
    }
}

impl std::error::Error for UnwritableSymbol {}

// ── Serialiser 1: TPTP FOF ──────────────────────────────────────────────────

/// TPTP FOF rendering of [`Form`]. A fold over the formula and nothing else.
///
/// Named `fof` rather than `tptp` so it sits beside [`clif`] as one of two
/// syntaxes, which is what it is.
pub mod fof {
    use super::{Form, P1, P2, Term, sym};

    /// Symbols are single-quoted TPTP atoms carrying a kind prefix.
    ///
    /// The prefix comes from [`super::sym`] and is not cosmetic: `OwlP1` and
    /// `OwlP2` are disjoint sums in the Lean, so a class and an object
    /// property that happen to share an IRI are two unrelated symbols, and a
    /// rendering that collapsed them would emit a different theory. Every
    /// prefix contains a colon, which is not legal in an unquoted TPTP
    /// lower-word, so no prefixed symbol can ever collide with the bare atoms
    /// `thing` and `lit`.
    ///
    /// Only `\` and `'` are escaped, and the prefixes contain neither, so
    /// quoting the prefixed image is byte for byte what quoting the prefix and
    /// the escaped IRI separately used to produce.
    fn quoted(image: &str) -> String {
        if image == "thing" || image == "lit" {
            return image.to_string();
        }
        let mut s = String::with_capacity(image.len() + 4);
        s.push('\'');
        for ch in image.chars() {
            if ch == '\\' || ch == '\'' {
                s.push('\\');
            }
            s.push(ch);
        }
        s.push('\'');
        s
    }

    fn p1(p: &P1) -> String {
        quoted(&sym::p1(p))
    }

    fn p2(p: &P2) -> String {
        quoted(&sym::p2(p))
    }

    fn term(t: &Term) -> String {
        match t {
            Term::Var(n) => format!("X{n}"),
            Term::Const(a) => quoted(&sym::constant(a)),
        }
    }

    /// True when the rendering is already a TPTP unitary formula and so needs
    /// no extra bracket under `~` or after a quantifier's colon.
    ///
    /// The binary connectives qualify because [`form`] brackets them itself.
    /// `Eq`, `Neg` and the quantifiers do not: `~ ~ p` is not legal TPTP,
    /// unary connectives being non-associative, and an unbracketed `X0 = X1`
    /// under a `~` is likewise not a unit formula.
    fn already_unit(f: &Form) -> bool {
        matches!(
            f,
            Form::App1(..)
                | Form::App2(..)
                | Form::Tru
                | Form::Fls
                | Form::And(..)
                | Form::Or(..)
                | Form::Imp(..)
        )
    }

    fn unit(f: &Form) -> String {
        if already_unit(f) {
            form(f)
        } else {
            format!("({})", form(f))
        }
    }

    /// Render a formula. Compound connectives are fully bracketed, which keeps
    /// the output independent of any reader's precedence table.
    pub fn form(f: &Form) -> String {
        match f {
            Form::App1(p, t) => format!("{}({})", p1(p), term(t)),
            Form::App2(p, t, u) => format!("{}({},{})", p2(p), term(t), term(u)),
            Form::Eq(t, u) => format!("{} = {}", term(t), term(u)),
            Form::Tru => "$true".to_string(),
            Form::Fls => "$false".to_string(),
            Form::Neg(g) => format!("~ {}", unit(g)),
            Form::And(g, h) => format!("({} & {})", form(g), form(h)),
            Form::Or(g, h) => format!("({} | {})", form(g), form(h)),
            Form::Imp(g, h) => format!("({} => {})", form(g), form(h)),
            Form::All(n, g) => format!("! [X{n}] : {}", unit(g)),
            Form::Ex(n, g) => format!("? [X{n}] : {}", unit(g)),
        }
    }
}

// ── Serialiser 2: ISO/IEC 24707 CLIF ────────────────────────────────────────

/// Which spelling of the Common Logic operators to emit.
///
/// This is not a style choice. Two spellings are in circulation and the two
/// main consumers disagree, so a file in one dialect is unreadable to half the
/// ecosystem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClifDialect {
    /// `cl:text`, `cl:comment`.
    ///
    /// The spelling in ISO/IEC 24707's own reserved-token list, and what ISO
    /// publishes the BFO axiomatisation in.
    Iso,
    /// `cl-text`, `cl-comment`.
    ///
    /// What the COLORE repository is written in and what the Macleod
    /// toolchain's shipped lexer maps; `src/macleod/parsing/parser.py` has the
    /// colon spellings present and commented out.
    Colore,
}

/// Where a formula's label goes.
///
/// This is not cosmetic. It decides whether the file has any content a reader
/// can recover, and the measurement is in `docs/first-order-export.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClifComments {
    /// A standalone `(cl:comment '...')` phrase, then the sentence on its own.
    ///
    /// The default, because it is the only shape either available CLIF parser
    /// gives back any content from. py-typedlogic treats a `(cl:comment ...)`
    /// form as discardable, so a file whose sentences are all inside one
    /// parses cleanly and yields ZERO sentences; Macleod has no
    /// commented-sentence production at all. A file that is formally valid and
    /// practically empty is the assurance-laundering shape this project exists
    /// to attack, so it is not the default here.
    Standalone,
    /// `(cl:comment '...' SENTENCE)`, the shape ISO/IEC 21838-2's BFO files
    /// use.
    ///
    /// Off by default. Correct CLIF, and unreadable by both parsers that exist
    /// today, including on BFO's own files.
    Wrapped,
}

impl ClifComments {
    pub fn parse(s: &str) -> anyhow::Result<ClifComments> {
        match s.to_ascii_lowercase().as_str() {
            "standalone" | "separate" => Ok(ClifComments::Standalone),
            "wrapped" | "bfo" => Ok(ClifComments::Wrapped),
            other => anyhow::bail!(
                "unknown CLIF comment placement {other:?}; expected `standalone` (the \
                 default, the only shape either available parser recovers sentences from) \
                 or `wrapped` (what BFO uses, and what both parsers read as empty)"
            ),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            ClifComments::Standalone => "standalone",
            ClifComments::Wrapped => "wrapped",
        }
    }
}

impl ClifDialect {
    pub fn parse(s: &str) -> anyhow::Result<ClifDialect> {
        match s.to_ascii_lowercase().as_str() {
            "iso" | "iso24707" | "colon" => Ok(ClifDialect::Iso),
            "colore" | "macleod" | "hyphen" => Ok(ClifDialect::Colore),
            other => anyhow::bail!(
                "unknown CLIF dialect {other:?}; expected `iso` (cl:text, what ISO/IEC \
                 21838-2 publishes) or `colore` (cl-text, what COLORE and Macleod read)"
            ),
        }
    }
    fn text_op(self) -> &'static str {
        match self {
            ClifDialect::Iso => "cl:text",
            ClifDialect::Colore => "cl-text",
        }
    }
    fn comment_op(self) -> &'static str {
        match self {
            ClifDialect::Iso => "cl:comment",
            ClifDialect::Colore => "cl-comment",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            ClifDialect::Iso => "iso",
            ClifDialect::Colore => "colore",
        }
    }
}

/// CLIF rendering of [`Form`], restricted to the first-order-equivalent
/// fragment of Common Logic.
///
/// Common Logic is not first-order logic: it has sequence markers, arity-free
/// predicates, and a single universe in which relations are themselves
/// individuals. The adequacy theorem is about a translation into PLAIN
/// first-order logic, so the emitted text stays inside the fragment where the
/// two coincide:
///
/// * no sequence markers (`...x`), which are what actually take Common Logic
///   past first order: clause 6.5 of ISO/IEC 24707 states that a logic with
///   them is not compact and therefore not first-order,
/// * fixed arity everywhere: `thing` and `lit` and every class and datatype at
///   arity one, every property at arity two, always. Arity-freedom alone does
///   not escape first-order logic (clause 6.6.1 gives the de-punning
///   reduction), but fixing arity is what makes the text expressible in TPTP
///   FOF at all,
/// * no quantification into a predicate position: every bound variable occurs
///   only as an argument.
///
/// [`Form`] cannot express any of the three, which is the structural reason
/// the restriction holds. `clif_uses_only_fol_fragment` in
/// `tests/fol_translation_correspondence_test.rs` is the check that it still
/// does.
pub mod clif {
    use super::{ClifDialect, Form, P1, P2, Term, sym};

    /// Names are written as CLIF **enclosed names**, delimited by DOUBLE
    /// QUOTES, with `"` and `\` escaped.
    ///
    /// Not vertical bars. ISO/IEC 24707 A.2.2.2 sets `namequote = '"'`, and
    /// A.2.2.4 recommends the enclosed-name syntax specifically for writing
    /// IRIs as names. The vertical bar is an ordinary name character in CLIF
    /// (it is in the `char` production), so `|x|` would lex as a bare name
    /// containing two pipes and would not protect a slash or a colon. The
    /// bar convention belongs to Common Lisp and to KIF, not to CLIF.
    ///
    /// The kind prefixes come from [`super::sym`], the same module the TPTP
    /// and SMT-LIB serialisers read, for the same reason: `OwlP1` and `OwlP2`
    /// are disjoint sums in the Lean.
    ///
    /// `pub` because [`super::cgif`] writes the SAME construct and must not
    /// write its own. That is not convenience: ISO/IEC 24707:2018 B.1.1 defines
    /// `CGname` as `identifier | '"', (namesequence - identifier), '"' |
    /// numeral | enclosedname | quotedstring`, where `enclosedname` is A.2.2's
    /// category, CLIF's. The two syntaxes quote a name the same way because the
    /// standard says they do, and a second copy here could only ever drift.
    pub fn enclosed(image: &str) -> String {
        let mut s = String::with_capacity(image.len() + 4);
        s.push('"');
        for ch in image.chars() {
            if ch == '\\' || ch == '"' {
                s.push('\\');
            }
            s.push(ch);
        }
        s.push('"');
        s
    }

    /// A comment string, in SINGLE quotes, with `\\` and `'` escaped.
    ///
    /// ISO/IEC 24707 A.2.2.2 makes the single quote the string delimiter
    /// (`stringquote`) and the double quote the enclosed-name delimiter
    /// (`namequote`), so a comment's text is a quoted string and takes single
    /// quotes. This exporter emitted double quotes until it was measured: the
    /// CLIF files ISO hosts for ISO/IEC 21838-2 carry 369 double-quoted
    /// `cl:comment` forms, and BFO's own release notes of 7 December 2025
    /// retract exactly that, saying "Comment texts are surrounded by single,
    /// not double quotes". BFO master has 356 single-quoted and none
    /// double-quoted. Matching the corpus was the wrong test; the corpus had
    /// been withdrawn by its maintainers.
    ///
    /// The quote style does NOT vary with the dialect. Binding the two
    /// together meant no combination of flags could emit conforming CLIF.
    pub fn comment_string(text: &str) -> String {
        let mut s = String::with_capacity(text.len() + 2);
        s.push('\'');
        for ch in text.chars() {
            if ch == '\\' || ch == '\'' {
                s.push('\\');
            }
            s.push(ch);
        }
        s.push('\'');
        s
    }

    /// A text name. Bare when the IRI has no character that would break a
    /// name token, and a double-quoted enclosed name otherwise.
    ///
    /// Bare is preferred because both available parsers accept it and name the
    /// theory correctly, while Macleod's lexer has no double-quote token at
    /// all. CLIF's `char` production admits `:` and `/`, and COLORE names all
    /// 227 of its texts with a bare IRI.
    pub fn text_name(iri: &str) -> String {
        let safe = !iri.is_empty()
            && !iri
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '(' | ')' | '\'' | '"' | '|'));
        if safe {
            iri.to_string()
        } else {
            let mut s = String::from("\"");
            for ch in iri.chars() {
                if ch == '\\' || ch == '"' {
                    s.push('\\');
                }
                s.push(ch);
            }
            s.push('"');
            s
        }
    }

    /// A standalone `(cl:comment '...')` phrase.
    ///
    /// Lexical comments are not used anywhere in the emitted text: ISO/IEC
    /// 24707 defines `cl:comment` as a reserved element, whereas a `//` or
    /// `/* */` line comment is a convention this project could not establish
    /// from the standard's own text.
    pub fn standalone_comment(dialect: ClifDialect, label: &str) -> String {
        format!("({} {})", dialect.comment_op(), comment_string(label))
    }

    /// `(cl:comment '...' SENTENCE)`, the shape BFO uses.
    ///
    /// Correct CLIF and, measured, unreadable: py-typedlogic discards the form
    /// and returns an empty theory, and Macleod has no production for it. It
    /// does the same to BFO's own files. Reachable only through
    /// `ClifComments::Wrapped`, which is not the default.
    pub fn commented(dialect: ClifDialect, label: &str, f: &Form) -> String {
        format!(
            "({} {} {})",
            dialect.comment_op(),
            comment_string(label),
            form(f)
        )
    }

    fn p1(p: &P1) -> String {
        match p {
            P1::Thing => "thing".to_string(),
            P1::Lit => "lit".to_string(),
            other => enclosed(&sym::p1(other)),
        }
    }

    fn p2(p: &P2) -> String {
        enclosed(&sym::p2(p))
    }

    fn term(t: &Term) -> String {
        match t {
            Term::Var(n) => format!("X{n}"),
            Term::Const(a) => enclosed(&sym::constant(a)),
        }
    }

    /// Render a formula.
    ///
    /// `(and)` and `(or)` with no arguments are truth and falsity: CLIF's
    /// boolean sentence takes zero or more arguments and defines no truth
    /// constants of its own.
    pub fn form(f: &Form) -> String {
        match f {
            Form::App1(p, t) => format!("({} {})", p1(p), term(t)),
            Form::App2(p, t, u) => format!("({} {} {})", p2(p), term(t), term(u)),
            Form::Eq(t, u) => format!("(= {} {})", term(t), term(u)),
            Form::Tru => "(and)".to_string(),
            Form::Fls => "(or)".to_string(),
            Form::Neg(g) => format!("(not {})", form(g)),
            Form::And(g, h) => format!("(and {} {})", form(g), form(h)),
            Form::Or(g, h) => format!("(or {} {})", form(g), form(h)),
            Form::Imp(g, h) => format!("(if {} {})", form(g), form(h)),
            Form::All(n, g) => format!("(forall (X{n}) {})", form(g)),
            Form::Ex(n, g) => format!("(exists (X{n}) {})", form(g)),
        }
    }
}

// ── Serialiser 3: ISO/IEC 24707 CGIF ────────────────────────────────────────

/// CGIF rendering of [`Form`]: **core CGIF**, in a compact sub-dialect.
///
/// ISO/IEC 24707 Common Logic has three dialects, and Common Logic conformance
/// is a claim about the language and not about one serialisation of it. This
/// engine emitted CLIF only, which made its Common Logic support partial in a
/// way nothing said out loud. This module is the second of the three. XCL,
/// Annex C, is still not emitted and is named in the docs as absent.
///
/// # Core, not extended, and what that costs
///
/// B.3.1 lists what extended CGIF adds to core: type labels and type
/// expressions on concepts, `@every` for universal quantification, the Boolean
/// contexts `[If: … [Then: …]]`, `[Either: [Or: …]]` and
/// `[Equiv: [Iff: …]]`, concepts in an arc sequence, actors with zero or
/// several output arcs, and importing a text into a text. **None of that is
/// emitted here.** Everything below is core CGIF (B.2), which B.4 states is a
/// fully conformant CL dialect in its own right: "every CL sentence can be
/// translated to a semantically equivalent sentence in each of them".
///
/// Core costs exactly one thing in readability. A core concept has no type
/// field, so `[Cat: *x]` is not available and a class membership is written as
/// an ordinary relation. B.1.2 gives that reduction itself: "The concept
/// `[Go:*x]`, for example, becomes an untyped concept `[*x]` and a conceptual
/// relation `(Go ?x)`." That reduction is the whole difference, and taking it
/// means the file needs no Annex B.3 rewrite pass to be read.
///
/// # The restriction, in the standard's own vocabulary
///
/// Clause 7.1.1 names three sub-dialects, and this emitter is all three:
///
/// * a **compact sub-dialect**, "a dialect that does not recognize sequence
///   markers". That is the restriction that matters: clause 6.5 says Common
///   Logic with sequence markers "is not compact, and therefore not
///   first-order", and the adequacy theorem is about plain first-order logic.
///   No `[*...x]` and no `?...x` is emitted, in either position the grammar
///   allows one (B.2.5's `existentialConcept`, B.2.3's `arcSequence`).
/// * an **unstructured sub-dialect**, "a dialect that does not recognize
///   titlings and importation statements".
/// * a **single domain sub-dialect**, "a dialect that does not recognize
///   domain restrictions".
///
/// Two further exclusions have no clause-7 name and are listed because they
/// would leave first-order logic just as surely:
///
/// * **no `#?` type label.** B.2.7's `ordinaryRelation` admits `["#", "?"],
///   CGname`, and its own comment says why: "By allowing the type label of a
///   conceptual relation to be a bound label, CGIF supports the CL ability to
///   quantify over relations and functions." That is quantifying into a
///   predicate position, and it is the CGIF spelling of the thing the CLIF
///   restriction already excludes.
/// * **no actors.** B.2.1's `actor` is how a CL FUNCTION is written. `OwlLean.
///   FOL`'s `FSig` has P1, P2 and Const and no function symbol of positive
///   arity, so [`Form`] cannot express one and none is emitted.
///
/// [`Form`] cannot express any of the five, which is the structural reason the
/// restriction holds; `cgif_stays_in_the_compact_first_order_sub_dialect` in
/// `tests/fol_cgif_export_test.rs` is the check that it still does.
///
/// # The core encoding, production by production
///
/// | `Form` | CGIF | why |
/// |---|---|---|
/// | `App1(p,t)` | `(p t)` | B.2.7 `ordinaryRelation` |
/// | `App2(p,t,u)` | `(p t u)` | the same, at arity two |
/// | `Eq(t,u)` | `[: t u]` | B.2.5 `coreferenceConcept`. CGIF has NO equality relation |
/// | `Tru` | `[]` | B.2.5: "an empty context `[ ]` is translated to CLIF as `(and)`, which is true by definition" |
/// | `Fls` | `~[]` | B.2.8: "The negation of the blank CG, written `~[ ]`, is always false" |
/// | `Neg(g)` | `~[ g ]` | B.2.8 `negation = "~", context` |
/// | `And(g,h)` | `g h` | juxtaposition. B.2.6 makes a CG's nodes a conjunction |
/// | `Or(g,h)` | `~[ ~[g] ~[h] ]` | B.3.5's own `eitherOr` rewrite into core |
/// | `Imp(g,h)` | `~[ g ~[h] ]` | B.3.5's own `ifThen` rewrite into core |
/// | `All(n,g)` | `~[ [*Xn] ~[g] ]` | B.3.7's `@every` rewrite: a nest of two negations |
/// | `Ex(n,g)` | `[ [*Xn] g ]` | B.2.6: a CG with existential concepts is `∃names. rest` |
///
/// The three derived forms are not this project's inventions. B.3.5 gives
/// `ifThen` as `"~[", CG(ante), "~[", CG(conse), "]", "]"` and `eitherOr` as a
/// negation containing one `~[…]` per disjunct, and B.3.7 says of a CG
/// containing universal concepts that "the output string shall be a nest of two
/// negations. The outer context shall contain the translations of all the
/// universal concepts, and the inner context shall contain the translations of
/// all other nodes". This module emits what the standard's own rewrite rules
/// produce.
///
/// # Every binder gets its own context, and why that is not decoration
///
/// B.2.10: "If a concept x with a defining label with name n is directly
/// contained in some context c, then c shall not contain any concept other than
/// x with a defining label with the same CG name n." Two things follow, and
/// both bite.
///
/// **An existential is emitted as `[ [*Xn] … ]` and never as a bare `[*Xn]`
/// juxtaposed with its body.** `OwlLean.trAx` restarts its variable counter for
/// each axiom and reuses indices between an axiom's antecedent and its
/// consequent (`tr c 0 2` and `tr d 0 2` are both called at 2), so two `[*X2]`
/// really can arise in one sentence. Giving each binder a context of its own
/// means no context ever directly contains two defining labels at all, whatever
/// the counter did.
///
/// Renumbering would also have fixed it and is forbidden: `owl-lean` uses named
/// variables rather than de Bruijn indices precisely so that freshness is a
/// proof obligation, and an exporter that renumbered would be emitting a
/// different formula.
///
/// **Each sentence is wrapped in a context of its own** for the same reason at
/// the file level, so nothing a sentence binds can reach the next sentence.
/// `[ s ]` is `(and s)`, which is `s`.
///
/// **Shadowing is avoided rather than relied on.** B.2.10 says both that a
/// context "shall not contain any concept other than x with a defining label
/// with the same CG name n" — where *contains* is transitive — and, in the very
/// next sentence, that a nested context may redeclare one. The two cannot both
/// hold. This emitter never produces a defining label inside the scope of
/// another with the same name, so it does not depend on which reading is right,
/// and `no_defining_label_is_ever_shadowed` is the check.
///
/// # Names
///
/// B.1.1's `identifier = letter, {letter | digit | "_"}` is far narrower than
/// CLIF's `namecharsequence`: no colon, no slash, no dot, no hyphen. An IRI is
/// therefore ALWAYS written as a double-quoted enclosed name, including in the
/// text's name slot, where the CLIF writer emits a bare IRI for Macleod's sake.
/// B.1.1 states the rule: "the category CGname requires that all CLIF name
/// sequences except those in the CGIF category identifier shall be enclosed in
/// quotes".
///
/// `X0`, `X1`, … and the bare `thing` and `lit` are legal identifiers. Every
/// other image out of [`sym`] carries a colon and is enclosed. Those two facts
/// together are why a defining label can never collide with a constant, which
/// B.2.10's last clause forbids outright: "No constant with CG name n shall be
/// in the scope associated with some concept with a defining label with CG name
/// n." A defining label here is `Xn`; a constant here begins `i:`.
///
/// # Interpreted names
///
/// A.4.2 says of CLIF, in the 2018 edition as in the first, that "The
/// subdialect of CLIF which does not use numerals or quoted strings is exactly
/// semantically conformant". Annex B states **no** analogue for CGIF, so this
/// module does not borrow the label. What it does is hold the property: no
/// numeral and no single-quoted string stands anywhere in this output.
///
/// CGIF gets that for free where CLIF could not. A CLIF label has to ride on
/// `cl:comment`, whose argument is a quoted string, so the CLIF writer needs a
/// named exception for its comments. B.2.4 makes a CGIF comment a LEXICAL
/// construct, `/* … */`, which is not a name at all, so the CGIF output holds
/// the property with no exception to declare.
pub mod cgif {
    use super::{Form, P1, P2, Term, clif, sym};

    /// The delimiter that may not appear inside a comment.
    ///
    /// B.2.4: "The string enclosed by the delimiters `/*` and `*/` shall not
    /// contain a substring `*/`." There is no escape for it, so a text that
    /// contained one could not be commented at all. Nothing this emitter puts
    /// in a comment can contain it — the header is a constant and a label is
    /// `background_1 (axiom)` — and `no_emitted_comment_can_close_itself_early`
    /// is the check rather than the assumption.
    pub const COMMENT_CLOSE: &str = "*/";

    /// A comment text this exporter cannot write, with the reason.
    ///
    /// Returned rather than escaped or dropped, which is the rule
    /// [`super::UnwritableSymbol`] already follows for SMT-LIB and LADR: a
    /// dropped comment loses the label that says which axiom a sentence is,
    /// and a rewritten one is a different comment. B.2.4 gives no escape for
    /// the closing delimiter, so there is no third option.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UnwritableComment {
        pub text: String,
        pub why: &'static str,
    }

    impl std::fmt::Display for UnwritableComment {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "the comment {:?} cannot be written in CGIF: {}", self.text, self.why)
        }
    }

    impl std::error::Error for UnwritableComment {}

    /// Whether a string may be written inside a CGIF comment.
    pub fn comment_is_writable(text: &str) -> bool {
        !text.contains(COMMENT_CLOSE)
    }

    /// A `/* … */` comment. B.2.4 permits one "intermixed with the concepts and
    /// conceptual relations of any conceptual graph", which is where every
    /// comment in the emitted file stands.
    ///
    /// This is also exactly what ISO/IEC 24707 B.4's Table B.1 maps a CLIF
    /// `(cl:comment 'string' P)` to: "A comment and a CG: `"/*"`, 'string',
    /// `"*/"`, cl2cg(P)". So the CGIF file's label shape is the standard's own
    /// image of the CLIF file's label shape, rather than a second convention.
    pub fn comment(text: &str) -> Result<String, UnwritableComment> {
        if !comment_is_writable(text) {
            return Err(UnwritableComment {
                text: text.to_string(),
                why: "ISO/IEC 24707 B.2.4: \"The string enclosed by the delimiters `/*` and \
                      `*/` shall not contain a substring `*/`\", and the standard defines no \
                      escape for it. Rewriting the text would emit a different comment and \
                      dropping it would lose the label that says which axiom the sentence is",
            });
        }
        Ok(format!("/*{text}*/"))
    }

    /// A text name. Always a double-quoted enclosed name.
    ///
    /// Unlike the CLIF writer, which emits a bare IRI because Macleod's lexer
    /// has no double-quote token, this has no choice: B.1.1's `identifier`
    /// admits letters, digits and underscore only, so no IRI is one.
    pub fn text_name(iri: &str) -> String {
        clif::enclosed(iri)
    }

    fn p1(p: &P1) -> String {
        match p {
            P1::Thing => "thing".to_string(),
            P1::Lit => "lit".to_string(),
            other => clif::enclosed(&sym::p1(other)),
        }
    }

    fn p2(p: &P2) -> String {
        clif::enclosed(&sym::p2(p))
    }

    /// An arc: B.2.9's `reference = ["?"], CGname`. A bound coreference label
    /// carries the `?`; a constant carries nothing.
    fn arc(t: &Term) -> String {
        match t {
            Term::Var(n) => format!("?X{n}"),
            Term::Const(a) => clif::enclosed(&sym::constant(a)),
        }
    }

    /// Render a formula as a CG: a sequence of one or more nodes, whose
    /// conjunction B.2.6 makes the meaning of the graph.
    ///
    /// `And` is the only arm that returns more than one node, which is not an
    /// implementation detail: conjunction has no operator in CGIF. Sowa puts it
    /// plainly and B.2.6 says it formally — "A conceptual graph consists of an
    /// unordered set of concepts, conceptual relations, negations, and
    /// comments", and a CG's meaning is the conjunction of its nodes.
    pub fn graph(f: &Form) -> String {
        match f {
            Form::App1(p, t) => format!("({} {})", p1(p), arc(t)),
            Form::App2(p, t, u) => format!("({} {} {})", p2(p), arc(t), arc(u)),
            // B.2.5: a coreference concept is "translated to a conjunction of
            // equations". With two references that conjunction is one equation.
            // CGIF has no `=` and could not have one: `=` is a CLIF reserved
            // token and a CGIF identifier must begin with a letter.
            Form::Eq(t, u) => format!("[: {} {}]", arc(t), arc(u)),
            Form::Tru => "[]".to_string(),
            Form::Fls => "~[]".to_string(),
            Form::Neg(g) => format!("~[{}]", graph(g)),
            Form::And(g, h) => format!("{} {}", graph(g), graph(h)),
            Form::Or(g, h) => format!("~[~[{}] ~[{}]]", graph(g), graph(h)),
            Form::Imp(g, h) => format!("~[{} ~[{}]]", graph(g), graph(h)),
            Form::All(n, g) => format!("~[[*X{n}] ~[{}]]", graph(g)),
            Form::Ex(n, g) => format!("[[*X{n}] {}]", graph(g)),
        }
    }

    /// One formula as a single node: `[ CG ]`.
    ///
    /// The bracket is a B.2.5 `context`, whose meaning is the conjunction of
    /// what it holds, so it changes nothing about the theory. It buys two
    /// things. A formula whose root is `And` is several nodes and would
    /// otherwise spread across the text's own graph, so the label comment in
    /// front of it would attach to the first node and not to the formula. And
    /// the context bounds every quantifier the formula opens, so no sentence
    /// can reach into the next one.
    pub fn sentence(f: &Form) -> String {
        format!("[{}]", graph(f))
    }
}

// ── Serialiser 4: SMT-LIB 2 ─────────────────────────────────────────────────

/// SMT-LIB 2 rendering of [`Form`]. The third printer over the same [`Form`],
/// and no more OWL-specific than the other two.
///
/// # Why this one exists and the other two were not enough
///
/// TPTP and CLIF are refutation formats: the consumer takes a conjecture,
/// negates it and searches for a contradiction, and by decision 0005 what it
/// hands back is an oracle opinion. SMT-LIB is read by solvers that also build
/// MODELS, and a model is a finite object this repository can check. The file
/// this module writes is therefore asked a different question, and it asserts
/// the NEGATED goal rather than declaring a conjecture, because a countermodel
/// to `Γ ⊨ φ` is a model of `Γ ∪ {¬φ}`.
///
/// # The two encodings, and why the difference is load-bearing
///
/// [`SmtEncoding::Unbounded`] declares `U` with `declare-sort`, so a model may
/// be of any cardinality and `unsat` really means unsatisfiable. Nothing
/// constrains the solver to return a finite structure, so a `sat` here is an
/// oracle opinion.
///
/// [`SmtEncoding::Finite`] declares `U` as an enumeration datatype with
/// exactly `k` nullary constructors `e0 … e(k-1)`. Now `sat` comes with a
/// structure over a known finite carrier, which is exactly what `oo-folmodel`
/// can check, and `unsat` establishes only that no model of size `k` exists.
/// Reporting the second as unsatisfiability is the ten-minute mistake
/// decision 0006 item 4 exists to stop: `∀x∃y (r(x,y) ∧ x≠y)` is `unsat` at
/// carrier 1 and `sat` at carrier 2.
///
/// `e0 … e(k-1)`, `U` and `X0 …` cannot collide with a problem symbol, because
/// every problem symbol is `thing`, `lit`, or carries one of the five colon
/// prefixes from [`sym`], and none of those spellings contains a colon.
pub mod smtlib {
    use super::{Form, P1, P2, Term, UnwritableSymbol, sym};

    /// Which sort declaration the file carries.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum SmtEncoding {
        /// `(declare-sort U 0)`. Any cardinality, so `unsat` is real
        /// unsatisfiability and `sat` carries no size bound.
        Unbounded,
        /// `(declare-datatypes ((U 0)) (((e0) … )))`, exactly `k` elements.
        /// `unsat` establishes only `no_model_up_to_size_k`.
        Finite(u32),
    }

    impl SmtEncoding {
        /// The word a report prints for this encoding. Decision 0006 makes the
        /// verdict depend on it mechanically, so it is a value and not prose.
        pub fn name(self) -> String {
            match self {
                SmtEncoding::Unbounded => "unbounded".to_string(),
                SmtEncoding::Finite(k) => format!("finite({k})"),
            }
        }
        pub fn bound(self) -> Option<u32> {
            match self {
                SmtEncoding::Unbounded => None,
                SmtEncoding::Finite(k) => Some(k),
            }
        }
    }

    /// An SMT-LIB 2 symbol.
    ///
    /// `thing` and `lit` are legal simple symbols and are written bare.
    /// Everything else carries a colon, which SMT-LIB's simple-symbol
    /// character set excludes (a leading colon is a keyword), so it goes in
    /// `|…|`. A quoted symbol may contain anything except `|` and `\`
    /// (SMT-LIB 2.6 §3.1), and there is no escape for either, so a symbol
    /// containing one is REFUSED rather than mangled into a different symbol.
    pub fn symbol(image: &str) -> Result<String, UnwritableSymbol> {
        if image == "thing" || image == "lit" {
            return Ok(image.to_string());
        }
        if let Some(c) = image.chars().find(|c| matches!(c, '|' | '\\')) {
            return Err(UnwritableSymbol {
                symbol: image.to_string(),
                character: c,
                syntax: "SMT-LIB 2",
                why: "a quoted symbol admits every character except | and \\, and SMT-LIB \
                      defines no escape for either (2.6 section 3.1)",
            });
        }
        Ok(format!("|{image}|"))
    }

    fn p1(p: &P1) -> Result<String, UnwritableSymbol> {
        symbol(&sym::p1(p))
    }
    fn p2(p: &P2) -> Result<String, UnwritableSymbol> {
        symbol(&sym::p2(p))
    }
    fn term(t: &Term) -> Result<String, UnwritableSymbol> {
        match t {
            Term::Var(n) => Ok(format!("X{n}")),
            Term::Const(a) => symbol(&sym::constant(a)),
        }
    }

    /// Render a formula. Fully parenthesised, because S-expressions are.
    pub fn form(f: &Form) -> Result<String, UnwritableSymbol> {
        Ok(match f {
            Form::App1(p, t) => format!("({} {})", p1(p)?, term(t)?),
            Form::App2(p, t, u) => format!("({} {} {})", p2(p)?, term(t)?, term(u)?),
            Form::Eq(t, u) => format!("(= {} {})", term(t)?, term(u)?),
            Form::Tru => "true".to_string(),
            Form::Fls => "false".to_string(),
            Form::Neg(g) => format!("(not {})", form(g)?),
            Form::And(g, h) => format!("(and {} {})", form(g)?, form(h)?),
            Form::Or(g, h) => format!("(or {} {})", form(g)?, form(h)?),
            Form::Imp(g, h) => format!("(=> {} {})", form(g)?, form(h)?),
            Form::All(n, g) => format!("(forall ((X{n} U)) {})", form(g)?),
            Form::Ex(n, g) => format!("(exists ((X{n} U)) {})", form(g)?),
        })
    }

    /// The sort declaration for an encoding.
    pub fn sort_decl(enc: SmtEncoding) -> String {
        match enc {
            SmtEncoding::Unbounded => "(declare-sort U 0)".to_string(),
            SmtEncoding::Finite(k) => {
                let ctors: Vec<String> = (0..k).map(|i| format!("(e{i})")).collect();
                format!("(declare-datatypes ((U 0)) (({})))", ctors.join(" "))
            }
        }
    }

    /// The logic name. `UF` is quantified uninterpreted functions; `UFDT` adds
    /// the datatypes the finite encoding declares the carrier with.
    pub fn logic(enc: SmtEncoding) -> &'static str {
        match enc {
            SmtEncoding::Unbounded => "UF",
            SmtEncoding::Finite(_) => "UFDT",
        }
    }
}

// ── Serialiser 5: LADR, for Mace4 ───────────────────────────────────────────

/// LADR rendering of [`Form`], for Mace4, the finite model finder that ships
/// with Prover9.
///
/// # Why a dead toolchain is here at all
///
/// Prover9 is unmaintained and its refutations are uncheckable, so by
/// decision 0005 it is an oracle like any other prover. Mace4 ships in the
/// same distribution, is a FINITE MODEL FINDER, and prints exactly the kind of
/// object `lean/Fol/` can certify. The dead toolchain has a live half.
///
/// # The trap this module is built around
///
/// LADR's default convention is that a name whose first letter is in
/// `{u,v,w,x,y,z}` is a VARIABLE. A mangler that turned an IRI into `w0` would
/// turn a constant into a universally quantified variable, and Mace4 would
/// silently search a different and usually unsatisfiable problem. Measured on
/// this machine with LADR 2009-11A: the input
///
/// ```text
/// p0(w0).
/// -p0(k0).
/// ```
///
/// is echoed by Mace4 in its own `CLAUSES FOR SEARCH` block as `p0(x).` and
/// `-p0(k0).`, the search is then exhausted, and the run reports no model at
/// all. Nothing errors. So this module never writes an IRI: it MANGLES every
/// symbol into `p0…` (unary), `r0…` (binary) and `c0…` (constants), renders
/// bound variables as `x0, x1, …` so that they still ARE variables under the
/// same convention, and keeps a [`SymbolTable`] to read the answer back.
/// [`check_not_variable`] is run over every emitted name, so the gate exists
/// as a function that can be made to fire rather than as a comment.
///
/// Mangling also solves the quoting problem, which LADR has no answer to: an
/// IRI contains `:` and `/` and LADR has no quoting construct that survives
/// either.
pub mod ladr {
    use super::{FolProblem, Form, Term, sym};
    use std::collections::BTreeMap;
    use std::fmt::Write as _;

    /// A name LADR would read as a variable, so the writer refused it.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct LadrVariableName {
        pub name: String,
        pub why: &'static str,
    }

    impl std::fmt::Display for LadrVariableName {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "LADR would read {:?} as a VARIABLE, not a symbol: {}", self.name, self.why)
        }
    }
    impl std::error::Error for LadrVariableName {}

    /// True when LADR's default convention reads this name as a variable.
    pub fn is_variable_name(name: &str) -> bool {
        matches!(
            name.chars().next(),
            Some('u' | 'v' | 'w' | 'x' | 'y' | 'z' | 'U' | 'V' | 'W' | 'X' | 'Y' | 'Z')
        )
    }

    /// The gate. Every name this module emits in a symbol position goes
    /// through it, so that the trap above cannot be reintroduced by an edit to
    /// the mangler.
    pub fn check_not_variable(name: &str) -> Result<(), LadrVariableName> {
        if is_variable_name(name) {
            return Err(LadrVariableName {
                name: name.to_string(),
                why: "LADR treats a name beginning with u, v, w, x, y or z as universally \
                      quantified. A constant mangled to such a name makes Mace4 search a \
                      different problem and report `exhausted` with no error",
            });
        }
        Ok(())
    }

    /// Which slot of `Fol.FinModel` a symbol belongs in. Three separate
    /// fields there, so this is a kind and not a number.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum SymKind {
        Unary,
        Binary,
        Constant,
    }

    impl SymKind {
        pub fn name(self) -> &'static str {
            match self {
                SymKind::Unary => "unary",
                SymKind::Binary => "binary",
                SymKind::Constant => "constant",
            }
        }
    }

    /// The mangling, and its inverse.
    ///
    /// Built in sorted order so that a given problem always produces the same
    /// file, which is what makes a Mace4 run reproducible and a diff readable.
    ///
    /// The inverse carries the KIND as well as the image, and that is not
    /// bookkeeping. A model finder that returned `relation(p0(_,_))` for a
    /// symbol the problem uses as a unary predicate would otherwise be filed
    /// as a perfectly good binary interpretation, and the unary slot would
    /// then be reported as uninterpreted: a confusing message for a defect
    /// that is really a disagreement about the signature. The table is the
    /// authority on the arity, because the table is what wrote the file.
    #[derive(Debug, Clone, Default)]
    pub struct SymbolTable {
        /// `c:Person` → `p3`
        pub unary: BTreeMap<String, String>,
        /// `op:worksFor` → `r0`
        pub binary: BTreeMap<String, String>,
        /// `i:a` → `c0`
        pub consts: BTreeMap<String, String>,
        inverse: BTreeMap<String, (String, SymKind)>,
    }

    impl SymbolTable {
        /// Mangle every symbol the problem uses.
        pub fn build(problem: &FolProblem) -> Result<SymbolTable, LadrVariableName> {
            let v = problem.vocabulary();
            let mut t = SymbolTable::default();
            for (i, s) in v.unary.iter().enumerate() {
                let m = format!("p{i}");
                check_not_variable(&m)?;
                t.unary.insert(s.clone(), m.clone());
                t.inverse.insert(m, (s.clone(), SymKind::Unary));
            }
            for (i, s) in v.binary.iter().enumerate() {
                let m = format!("r{i}");
                check_not_variable(&m)?;
                t.binary.insert(s.clone(), m.clone());
                t.inverse.insert(m, (s.clone(), SymKind::Binary));
            }
            for (i, s) in v.consts.iter().enumerate() {
                let m = format!("c{i}");
                check_not_variable(&m)?;
                t.consts.insert(s.clone(), m.clone());
                t.inverse.insert(m, (s.clone(), SymKind::Constant));
            }
            Ok(t)
        }

        /// The symbol a mangled name stands for, or `None` when LADR invented
        /// it. Mace4 clausifies and Skolemises, so its models interpret names
        /// this table never issued; those are the reduct and are dropped.
        pub fn demangle(&self, mangled: &str) -> Option<&str> {
            self.inverse.get(mangled).map(|(s, _)| s.as_str())
        }

        /// The image AND the slot it belongs in. The arity a model file claims
        /// is checked against this rather than believed.
        pub fn demangle_kind(&self, mangled: &str) -> Option<(&str, SymKind)> {
            self.inverse.get(mangled).map(|(s, k)| (s.as_str(), *k))
        }

        pub fn len(&self) -> usize {
            self.unary.len() + self.binary.len() + self.consts.len()
        }
        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }
    }

    fn term(t: &Term, tab: &SymbolTable) -> Result<String, LadrVariableName> {
        Ok(match t {
            // A bound variable is deliberately `x…`, because under the same
            // convention that makes `c0` a symbol, `x0` is a variable.
            Term::Var(n) => format!("x{n}"),
            Term::Const(a) => lookup(&tab.consts, &sym::constant(a))?,
        })
    }

    fn lookup(m: &BTreeMap<String, String>, image: &str) -> Result<String, LadrVariableName> {
        match m.get(image) {
            Some(s) => Ok(s.clone()),
            // Unreachable through `problem`, which builds the table from the
            // same vocabulary it then renders. Reported rather than panicking
            // so that a future caller that builds the two separately gets a
            // sentence instead of a crash.
            None => Err(LadrVariableName {
                name: image.to_string(),
                why: "the symbol is not in the table this file was mangled with, so the \
                      LADR name for it does not exist",
            }),
        }
    }

    /// Render a formula. Fully parenthesised: LADR's precedence table is not
    /// worth relying on and a misparse here is silent.
    pub fn form(f: &Form, tab: &SymbolTable) -> Result<String, LadrVariableName> {
        Ok(match f {
            Form::App1(p, t) => format!("{}({})", lookup(&tab.unary, &sym::p1(p))?, term(t, tab)?),
            Form::App2(p, t, u) => format!(
                "{}({},{})",
                lookup(&tab.binary, &sym::p2(p))?,
                term(t, tab)?,
                term(u, tab)?
            ),
            Form::Eq(t, u) => format!("({} = {})", term(t, tab)?, term(u, tab)?),
            // LADR's own truth constants. Measured: Mace4 2009-11A accepts a
            // bare `$T.` as an assumption.
            Form::Tru => "$T".to_string(),
            Form::Fls => "$F".to_string(),
            Form::Neg(g) => format!("-({})", form(g, tab)?),
            Form::And(g, h) => format!("({} & {})", form(g, tab)?, form(h, tab)?),
            Form::Or(g, h) => format!("({} | {})", form(g, tab)?, form(h, tab)?),
            Form::Imp(g, h) => format!("({} -> {})", form(g, tab)?, form(h, tab)?),
            Form::All(n, g) => format!("(all x{n} ({}))", form(g, tab)?),
            Form::Ex(n, g) => format!("(exists x{n} ({}))", form(g, tab)?),
        })
    }

    /// The whole input file, goal already negated.
    ///
    /// `max_seconds` is written into the file rather than passed on the
    /// command line so that a saved file reproduces the run it came from.
    pub fn problem(
        p: &FolProblem,
        tab: &SymbolTable,
        max_seconds: u32,
    ) -> Result<String, LadrVariableName> {
        let mut s = super::header("%", p.conjecture.is_some(), super::LADR_STYLE);
        let _ = writeln!(s, "\nassign(max_seconds, {max_seconds}).\n");
        let _ = writeln!(s, "formulas(assumptions).");
        for e in p.checker_entries() {
            let _ = writeln!(s, "  % {} ({})", e.label, e.role);
            let _ = writeln!(s, "  {}.", form(&e.form, tab)?);
        }
        let _ = writeln!(s, "end_of_list.");
        Ok(s)
    }

    /// The mangling, as data, so a report can say what a Mace4 name meant.
    pub fn table_tsv(tab: &SymbolTable) -> String {
        let mut s = String::new();
        for (image, m) in &tab.unary {
            let _ = writeln!(s, "unary\t{m}\t{image}");
        }
        for (image, m) in &tab.binary {
            let _ = writeln!(s, "binary\t{m}\t{image}");
        }
        for (image, m) in &tab.consts {
            let _ = writeln!(s, "constant\t{m}\t{image}");
        }
        s
    }
}

// ── Serialiser 6: the checker format ────────────────────────────────────────

/// `problem.tsv`, the file `oo-folmodel` reads, and the digest that binds a
/// model file to it.
///
/// The grammar and the hash are specified in `lean/Fol/Syntax.lean` and
/// `lean/Fol/Parse.lean`. This module is the second implementation of both,
/// and the pinned digest `4403d8aaa0c422f7` for the worked example is what
/// holds the two together: `lean/Fol/Parse.lean` has a `#guard` on it and
/// `tests/fol_checker_format_test.rs` has an assertion on it, so a change to
/// either printer or to the hash fails a build on one side and a test on the
/// other.
///
/// FNV-1a IDENTIFIES, IT DOES NOT COMMIT. It is not a cryptographic hash. It
/// exists so that two implementations can be compared, not so that one can be
/// defended against someone who controls the file. Decision 0003 item 5, in
/// the same words, for the same reason.
pub mod checkfmt {
    use super::{Form, P1, P2, Term, UnwritableSymbol, sym};

    /// Refuse a symbol the token stream cannot survive. See
    /// [`sym::unwritable_char`] for why this is live rather than defensive.
    fn clean(image: String) -> Result<String, UnwritableSymbol> {
        match sym::unwritable_char(&image) {
            None => Ok(image),
            Some(c) => Err(UnwritableSymbol {
                symbol: image,
                character: c,
                syntax: "the oo-folmodel checker format",
                why: "the formula is a space-separated token stream and the file is \
                      tab-separated, so a symbol containing either re-parses as a DIFFERENT \
                      formula or as a different set of fields",
            }),
        }
    }

    fn p1(p: &P1) -> Result<String, UnwritableSymbol> {
        clean(sym::p1(p))
    }
    fn p2(p: &P2) -> Result<String, UnwritableSymbol> {
        clean(sym::p2(p))
    }

    /// `Fol.Parse.showTerm`.
    pub fn show_term(t: &Term) -> Result<String, UnwritableSymbol> {
        Ok(match t {
            Term::Var(n) => format!("var {n}"),
            Term::Const(a) => format!("const {}", clean(sym::constant(a))?),
        })
    }

    /// `Fol.Parse.showForm`. Prefix, no parentheses, single spaces.
    pub fn show_form(f: &Form) -> Result<String, UnwritableSymbol> {
        Ok(match f {
            Form::App1(p, t) => format!("app1 {} {}", p1(p)?, show_term(t)?),
            Form::App2(p, t, u) => {
                format!("app2 {} {} {}", p2(p)?, show_term(t)?, show_term(u)?)
            }
            Form::Eq(t, u) => format!("eq {} {}", show_term(t)?, show_term(u)?),
            Form::Tru => "tru".to_string(),
            Form::Fls => "fls".to_string(),
            Form::Neg(g) => format!("neg {}", show_form(g)?),
            Form::And(g, h) => format!("and {} {}", show_form(g)?, show_form(h)?),
            Form::Or(g, h) => format!("or {} {}", show_form(g)?, show_form(h)?),
            Form::Imp(g, h) => format!("imp {} {}", show_form(g)?, show_form(h)?),
            Form::All(n, g) => format!("all {n} {}", show_form(g)?),
            Form::Ex(n, g) => format!("ex {n} {}", show_form(g)?),
        })
    }

    /// FNV-1a, 64 bit, over the UTF-8 bytes. Offset basis
    /// 14695981039346656037, prime 1099511628211, wrapping at 2^64.
    ///
    /// Written out rather than taken from a library because the Lean side
    /// writes it out too: `String.hash` there is an opaque extern with no
    /// specification a second implementation could target.
    pub fn fnv1a64(s: &str) -> u64 {
        s.as_bytes().iter().fold(14695981039346656037u64, |h, b| {
            (h ^ (*b as u64)).wrapping_mul(1099511628211)
        })
    }

    /// Sixteen lowercase hex digits, most significant first, zero padded.
    pub fn hex16(x: u64) -> String {
        format!("{x:016x}")
    }
}

// ── Reading an ontology out of the graph ────────────────────────────────────

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const RDFS_SUBCLASS: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const RDFS_SUBPROP: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
const RDFS_DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";
const OWL: &str = "http://www.w3.org/2002/07/owl#";

fn owl(local: &str) -> String {
    format!("{OWL}{local}")
}

/// A construct present in the graph and absent from the export, with the count
/// and the reason.
///
/// The description says why, not merely that. An ontology whose unexported
/// constructs are invisible is a trap; the description-logic layer already
/// carries `certifies_a_weaker_axiom_set` for the same reason.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Dropped {
    pub construct: String,
    pub occurrences: u64,
    pub why: String,
}

/// A construct rewritten into the fragment before translation, exactly and at
/// the OWL level.
///
/// These are not translation choices. `EquivalentObjectProperties(p q)` is
/// `SubObjectPropertyOf(p q)` and `SubObjectPropertyOf(q p)` under the Direct
/// Semantics, and `AllDisjointClasses` is its pairwise expansion. Listing them
/// keeps the difference between "the fragment covers this" and "the fragment
/// covers a rewriting of this" visible to a reader of the report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Reduced {
    pub construct: String,
    pub occurrences: u64,
    pub how: String,
}

/// What the reader made of the graph.
pub struct ReadOntology {
    pub axioms: Vec<OwlAxiom>,
    pub dropped: Vec<Dropped>,
    pub reduced: Vec<Reduced>,
    /// Properties whose entity kind was not declared and had to be inferred.
    pub inferred_object_properties: Vec<String>,
    pub inferred_data_properties: Vec<String>,
    /// Terms the reader treated as datatypes rather than as classes.
    ///
    /// `OwlP1` separates `cls` from `dt`, so `c:xsd:anyURI` and `d:xsd:anyURI`
    /// are unrelated predicates. More to the point, `OwlLean/Syntax.lean` has
    /// no axiom form asserting that an individual belongs to a DATATYPE:
    /// `classAssert` takes a `Concept`, and no `Concept` constructor denotes
    /// datatype membership. A goal of that shape has to be refused rather than
    /// translated as a class assertion under a symbol occurring in no axiom.
    pub datatypes: BTreeSet<String>,
    /// Properties the reader treated as object properties, and as data
    /// properties, declarations and inferences together.
    ///
    /// A goal must be built with the SAME symbols the axioms use. `OwlP2` is a
    /// disjoint sum, so `op:p` and `dp:p` are unrelated predicates, and a
    /// conjecture that picked the wrong arm would ask about a symbol occurring
    /// in no axiom. The prover would answer CounterSatisfiable, correctly, and
    /// the report would read as a disagreement with the engine when it was a
    /// defect in the question.
    pub object_properties: BTreeSet<String>,
    pub data_properties: BTreeSet<String>,
    /// Literal-valued triples whose predicate is an annotation property.
    /// Ignoring these does not weaken the axiom set, because an annotation has
    /// no Direct Semantics content. Reported so that is a stated fact rather
    /// than a silent omission.
    pub annotations_ignored: u64,
    /// Class expressions that are not named classes, keyed by the node that
    /// carries them.
    ///
    /// A derived triple can name one: `rdfs9` concludes `x rdf:type _:b` where
    /// `_:b` is a restriction. Without this map the conjecture would be built
    /// over an atomic class symbol that occurs nowhere in the axioms, and the
    /// prover would report no verdict on a question nobody meant to ask. That
    /// is worse than refusing the goal, because it reads as a limitation of
    /// the prover rather than as a defect here.
    pub anonymous_classes: BTreeMap<String, Concept>,
}

/// Maximum cardinality the export will expand.
///
/// `minCard n` emits `n(n-1)/2` distinctness literals and `n` guarded
/// existentials, so a four-figure cardinality is a file no prover will read.
/// Anything above the cap is DROPPED AND NAMED rather than truncated.
const MAX_CARDINALITY: u32 = 25;

/// The CLIF text name used when the source declares no `owl:Ontology` IRI.
///
/// Named rather than left anonymous: all 227 COLORE texts carry a name,
/// Macleod refuses an unnamed text with "Error in ontology: bad URI", and
/// py-typedlogic otherwise takes the first comment for the theory's name.
const UNNAMED_TEXT: &str = "http://open-ontologies.org/fol/unnamed-export";

struct Reader {
    /// subject -> [(predicate, object)]
    by_subject: BTreeMap<String, Vec<(String, String)>>,
    object_properties: BTreeSet<String>,
    data_properties: BTreeSet<String>,
    annotation_properties: BTreeSet<String>,
    classes: BTreeSet<String>,
    annotations_ignored: u64,
    dropped: BTreeMap<String, (u64, String)>,
    reduced: BTreeMap<String, (u64, String)>,
    inferred_object: BTreeSet<String>,
    inferred_data: BTreeSet<String>,
}

/// Strip the N-Triples spelling oxigraph hands back: `<iri>` becomes `iri`,
/// `_:b0` and literals are left as they are.
fn bare(term: &str) -> &str {
    if term.len() >= 2 && term.starts_with('<') && term.ends_with('>') {
        &term[1..term.len() - 1]
    } else {
        term
    }
}

fn is_iri(term: &str) -> bool {
    term.starts_with('<') && term.ends_with('>')
}

fn is_literal(term: &str) -> bool {
    term.starts_with('"')
}

impl Reader {
    fn new(triples: Vec<(String, String, String)>) -> Reader {
        let mut by_subject: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for (s, p, o) in triples {
            by_subject.entry(s).or_default().push((p, o));
        }
        let mut r = Reader {
            by_subject,
            object_properties: BTreeSet::new(),
            data_properties: BTreeSet::new(),
            annotation_properties: BTreeSet::new(),
            classes: BTreeSet::new(),
            annotations_ignored: 0,
            dropped: BTreeMap::new(),
            reduced: BTreeMap::new(),
            inferred_object: BTreeSet::new(),
            inferred_data: BTreeSet::new(),
        };
        r.collect_declarations();
        r
    }

    fn objects(&self, s: &str, p: &str) -> Vec<&str> {
        self.by_subject
            .get(s)
            .map(|ps| {
                ps.iter()
                    .filter(|(q, _)| bare(q) == p)
                    .map(|(_, o)| o.as_str())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn object(&self, s: &str, p: &str) -> Option<&str> {
        self.objects(s, p).into_iter().next()
    }

    fn has_type(&self, s: &str, ty: &str) -> bool {
        self.objects(s, RDF_TYPE).iter().any(|o| bare(o) == ty)
    }

    fn drop(&mut self, construct: &str, why: &str) {
        let e = self
            .dropped
            .entry(construct.to_string())
            .or_insert((0, why.to_string()));
        e.0 += 1;
    }

    fn reduce(&mut self, construct: &str, how: &str) {
        let e = self
            .reduced
            .entry(construct.to_string())
            .or_insert((0, how.to_string()));
        e.0 += 1;
    }

    fn collect_declarations(&mut self) {
        let subjects: Vec<String> = self.by_subject.keys().cloned().collect();
        for s in subjects {
            let bare_s = bare(&s).to_string();
            if self.has_type(&s, &owl("ObjectProperty")) {
                self.object_properties.insert(bare_s.clone());
            }
            if self.has_type(&s, &owl("DatatypeProperty")) {
                self.data_properties.insert(bare_s.clone());
            }
            if self.has_type(&s, &owl("AnnotationProperty")) {
                self.annotation_properties.insert(bare_s.clone());
            }
            // A property characteristic implies an OBJECT property only when
            // nothing has declared the property a data property. FOAF declares
            // `foaf:msnChatID` as `owl:DatatypeProperty` AND
            // `owl:InverseFunctionalProperty`, which OWL 2 DL forbids and
            // RDF-serialised vocabularies do anyway. Letting the characteristic
            // win made the axioms speak about `op:msnChatID` while every goal
            // about it spoke about `dp:msnChatID`, and the differential
            // reported six disagreements that were entirely this.
            if !self.data_properties.contains(&bare_s) {
                for extra in [
                    "TransitiveProperty",
                    "SymmetricProperty",
                    "AsymmetricProperty",
                    "ReflexiveProperty",
                    "IrreflexiveProperty",
                    "InverseFunctionalProperty",
                ] {
                    if self.has_type(&s, &owl(extra)) {
                        self.object_properties.insert(bare_s.clone());
                    }
                }
            }
            if self.has_type(&s, &owl("Class")) || self.has_type(&s, &owl("Restriction")) {
                self.classes.insert(bare_s.clone());
            }
        }
        // An IRI declared BOTH `owl:ObjectProperty` and `owl:DatatypeProperty`
        // is outside OWL 2 DL, which forbids that overlap. `Sig` keeps the
        // kinds apart as distinct types, so there is no reading of such a
        // property that the fragment supports: it would have to be `op:p` and
        // `dp:p` at once, and those are unrelated predicates. The data
        // declaration is taken and the conflict is recorded, because a silent
        // choice between two declarations is the kind of thing that later
        // looks like a reasoning bug.
        let punned = self
            .object_properties
            .intersection(&self.data_properties)
            .count() as u64;
        if punned > 0 {
            let e = self
                .dropped
                .entry("property declared both object and data".to_string())
                .or_insert((
                    0,
                    "OWL 2 DL forbids the overlap and OwlLean.Sig keeps OProp and DProp \
                     apart as distinct types, so no single symbol can carry both readings. \
                     The data declaration is used and the object reading is not exported"
                        .to_string(),
                ));
            e.0 += punned;
            let both: Vec<String> = self
                .object_properties
                .intersection(&self.data_properties)
                .cloned()
                .collect();
            for p in both {
                self.object_properties.remove(&p);
            }
        }
    }

    /// Is this node a datatype? Declared `rdfs:Datatype`, or a member of the
    /// two families every ontology uses without declaring.
    ///
    /// The goal builder in `triple_as_axiom` asks the same question of the
    /// `datatypes` set this reader publishes, so the axioms and the conjectures
    /// cannot disagree about which symbol a term gets.
    fn is_datatype_node(&self, node: &str) -> bool {
        let b = bare(node);
        b.starts_with("http://www.w3.org/2001/XMLSchema#")
            || b == "http://www.w3.org/2000/01/rdf-schema#Literal"
            || b == "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString"
            || self.has_type(node, RDFS_DATATYPE)
    }

    /// Is this property an object property? Declarations win; an undeclared
    /// property is classified by whether it ever takes a literal object, and
    /// every such inference is reported.
    fn is_object_property(&mut self, p: &str) -> bool {
        if self.object_properties.contains(p) {
            return true;
        }
        if self.data_properties.contains(p) {
            return false;
        }
        let has_literal_object = self
            .by_subject
            .values()
            .flatten()
            .any(|(q, o)| bare(q) == p && is_literal(o));
        if has_literal_object {
            self.inferred_data.insert(p.to_string());
            false
        } else {
            self.inferred_object.insert(p.to_string());
            true
        }
    }

    /// Walk an `rdf:first`/`rdf:rest` chain.
    ///
    /// Strict, as decision 0002 requires of the reasoner's list walk: every
    /// node must carry both, and the chain must reach `rdf:nil`. A list that
    /// does not is `None`, and the caller drops the axiom and names it.
    /// Deriving less from malformed input is the sound direction.
    fn list(&self, head: &str) -> Option<Vec<String>> {
        let mut out = Vec::new();
        let mut node = head.to_string();
        let mut seen = BTreeSet::new();
        loop {
            if bare(&node) == RDF_NIL {
                return Some(out);
            }
            if !seen.insert(node.clone()) {
                return None; // cyclic rest chain
            }
            let first = self.object(&node, RDF_FIRST)?.to_string();
            let rest = self.object(&node, RDF_REST)?.to_string();
            out.push(first);
            node = rest;
        }
    }

    /// Read a class expression. `None` means outside the fragment; the reason
    /// has already been recorded.
    fn concept(&mut self, node: &str) -> Option<Concept> {
        if is_literal(node) {
            self.drop(
                "literal in class position",
                "a literal is not a class expression",
            );
            return None;
        }
        let b = bare(node).to_string();
        if b == owl("Thing") {
            return Some(Concept::Top);
        }
        if b == owl("Nothing") {
            return Some(Concept::Bot);
        }
        if self.is_datatype_node(node) {
            // `OwlP1` separates `cls` from `dt`, and no `Concept` constructor
            // denotes membership of a datatype. Reading `X rdf:type xsd:anyURI`
            // as a CLASS assertion would put an axiom in the file that the
            // ontology does not state, which is the unsound direction: a
            // weakening is safe and an addition is not.
            self.drop(
                "datatype in a class position",
                "OwlLean/Syntax.lean has no Concept constructor for membership of a \
                 datatype, and OwlP1 keeps cls and dt apart, so this cannot be read as \
                 a class without inventing an axiom",
            );
            return None;
        }
        if b == owl("topObjectProperty") || b == owl("bottomObjectProperty") {
            self.drop(
                "owl:topObjectProperty / owl:bottomObjectProperty",
                "fixed-interpretation properties; OwlLean/Syntax.lean has no constructor \
                 for them and treating them as ordinary names would lose their semantics",
            );
            return None;
        }

        // Restriction
        if self.has_type(node, &owl("Restriction")) {
            return self.restriction(node);
        }

        // Boolean and enumeration constructors
        if let Some(l) = self.object(node, &owl("intersectionOf")).map(str::to_string) {
            if self.has_type(node, RDFS_DATATYPE) {
                self.drop(
                    "owl:intersectionOf on a datatype",
                    "OwlLean/Syntax.lean has no datatype-expression layer; only named \
                     datatypes appear, via dataSome / dataAll / dPropRange",
                );
                return None;
            }
            let items = match self.list(&l) {
                Some(i) => i,
                None => {
                    self.drop(
                        "owl:intersectionOf with a malformed list",
                        "a node missing rdf:first or rdf:rest, or a chain not reaching \
                         rdf:nil; deriving less from malformed input is the sound direction",
                    );
                    return None;
                }
            };
            return self.fold_boolean(&items, true);
        }
        if let Some(l) = self.object(node, &owl("unionOf")).map(str::to_string) {
            if self.has_type(node, RDFS_DATATYPE) {
                self.drop(
                    "owl:unionOf on a datatype",
                    "OwlLean/Syntax.lean has no datatype-expression layer",
                );
                return None;
            }
            let items = match self.list(&l) {
                Some(i) => i,
                None => {
                    self.drop("owl:unionOf with a malformed list", "see above");
                    return None;
                }
            };
            return self.fold_boolean(&items, false);
        }
        if let Some(c) = self.object(node, &owl("complementOf")).map(str::to_string) {
            if self.has_type(node, RDFS_DATATYPE) {
                self.drop(
                    "owl:datatypeComplementOf / complementOf on a datatype",
                    "no datatype-expression layer in the fragment",
                );
                return None;
            }
            return Some(Concept::Compl(Box::new(self.concept(&c)?)));
        }
        if let Some(l) = self.object(node, &owl("oneOf")).map(str::to_string) {
            if self.has_type(node, RDFS_DATATYPE) {
                self.drop(
                    "owl:oneOf on a datatype (a data enumeration)",
                    "Concept.oneOf ranges over individuals, not literals",
                );
                return None;
            }
            let items = match self.list(&l) {
                Some(i) => i,
                None => {
                    self.drop("owl:oneOf with a malformed list", "see above");
                    return None;
                }
            };
            if items.iter().any(|i| is_literal(i)) {
                self.drop(
                    "owl:oneOf containing a literal",
                    "Concept.oneOf ranges over individuals, not literals",
                );
                return None;
            }
            return Some(Concept::OneOf(
                items.iter().map(|i| bare(i).to_string()).collect(),
            ));
        }

        if is_iri(node) || node.starts_with("_:") {
            return Some(Concept::Atom(b));
        }
        self.drop("unreadable class position", "neither an IRI nor a blank node");
        None
    }

    fn fold_boolean(&mut self, items: &[String], intersection: bool) -> Option<Concept> {
        let mut parts = Vec::with_capacity(items.len());
        for i in items {
            parts.push(self.concept(i)?);
        }
        // An n-ary constructor folds RIGHT, so `and [a,b,c]` is
        // `Inter a (Inter b c)`, matching the shape of `Form::conj`.
        let mut it = parts.into_iter().rev();
        let mut acc = it.next()?;
        for p in it {
            acc = if intersection {
                Concept::Inter(Box::new(p), Box::new(acc))
            } else {
                Concept::Union(Box::new(p), Box::new(acc))
            };
        }
        Some(acc)
    }

    fn cardinality(&mut self, node: &str, pred: &str) -> Option<u32> {
        let raw = self.object(node, &owl(pred))?.to_string();
        // A typed literal arrives as `"2"^^<...integer>`.
        let digits: String = raw
            .trim_start_matches('"')
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let n: u32 = match digits.parse() {
            Ok(n) => n,
            Err(_) => {
                self.drop(
                    "cardinality that is not a non-negative integer",
                    "the restriction is unreadable, so it is not exported",
                );
                return None;
            }
        };
        if n > MAX_CARDINALITY {
            self.drop(
                "cardinality above the expansion cap",
                &format!(
                    "minCard n emits n(n-1)/2 distinctness literals; the cap is {MAX_CARDINALITY} \
                     and this restriction is dropped rather than truncated"
                ),
            );
            return None;
        }
        Some(n)
    }

    fn restriction(&mut self, node: &str) -> Option<Concept> {
        let prop = match self.object(node, &owl("onProperty")).map(str::to_string) {
            Some(p) => p,
            None => {
                self.drop(
                    "owl:Restriction with no owl:onProperty",
                    "not a well-formed restriction",
                );
                return None;
            }
        };
        if !is_iri(&prop) {
            // `owl:onProperty [ owl:inverseOf p ]`: Concept.some_ takes an
            // S.OProp and not an OPE, so an inverse property expression inside
            // a restriction is outside the fragment.
            self.drop(
                "restriction on an inverse property expression",
                "Concept.some_ / all_ / minCard / maxCard take a named property \
                 (S.OProp), not an OPE, in OwlLean/Syntax.lean",
            );
            return None;
        }
        let p = bare(&prop).to_string();
        let is_obj = self.is_object_property(&p);

        if let Some(f) = self.object(node, &owl("someValuesFrom")).map(str::to_string) {
            return if is_obj {
                Some(Concept::Some_(p, Box::new(self.concept(&f)?)))
            } else {
                Some(Concept::DataSome(p, bare(&f).to_string()))
            };
        }
        if let Some(f) = self.object(node, &owl("allValuesFrom")).map(str::to_string) {
            return if is_obj {
                Some(Concept::All_(p, Box::new(self.concept(&f)?)))
            } else {
                Some(Concept::DataAll(p, bare(&f).to_string()))
            };
        }
        if let Some(val) = self.object(node, &owl("hasValue")).map(str::to_string) {
            if !is_obj || is_literal(&val) {
                self.drop(
                    "owl:hasValue on a data property",
                    "Concept.hasVal takes an S.OProp and an S.Ind; a literal value is \
                     outside the fragment",
                );
                return None;
            }
            return Some(Concept::HasVal(p, bare(&val).to_string()));
        }
        if let Some(val) = self.object(node, &owl("hasSelf")).map(str::to_string) {
            if val.starts_with("\"true\"") {
                return Some(Concept::HasSelf(p));
            }
            self.drop(
                "owl:hasSelf with a value other than true",
                "only the self-restriction is in the fragment",
            );
            return None;
        }
        if self.object(node, &owl("onDataRange")).is_some() {
            self.drop(
                "data cardinality restriction (owl:onDataRange)",
                "OwlLean/Syntax.lean has minCard / maxCard on object properties only",
            );
            return None;
        }
        if !is_obj
            && ["minCardinality", "maxCardinality", "cardinality"]
                .iter()
                .any(|c| self.object(node, &owl(c)).is_some())
        {
            self.drop(
                "cardinality restriction on a data property",
                "OwlLean/Syntax.lean has minCard / maxCard on object properties only",
            );
            return None;
        }

        // Qualified forms first: `owl:onClass` is what distinguishes them.
        let on_class = self.object(node, &owl("onClass")).map(str::to_string);
        let filler = match &on_class {
            Some(c) => self.concept(&c.clone())?,
            None => Concept::Top,
        };

        for (pred, qualified) in [
            ("minQualifiedCardinality", true),
            ("minCardinality", false),
        ] {
            if self.object(node, &owl(pred)).is_some() {
                if qualified && on_class.is_none() {
                    self.drop(
                        "owl:minQualifiedCardinality with no owl:onClass",
                        "not a well-formed qualified restriction",
                    );
                    return None;
                }
                let n = self.cardinality(node, pred)?;
                return Some(Concept::MinCard(n, p, Box::new(filler)));
            }
        }
        for (pred, qualified) in [
            ("maxQualifiedCardinality", true),
            ("maxCardinality", false),
        ] {
            if self.object(node, &owl(pred)).is_some() {
                if qualified && on_class.is_none() {
                    self.drop(
                        "owl:maxQualifiedCardinality with no owl:onClass",
                        "not a well-formed qualified restriction",
                    );
                    return None;
                }
                let n = self.cardinality(node, pred)?;
                return Some(Concept::MaxCard(n, p, Box::new(filler)));
            }
        }
        for pred in ["qualifiedCardinality", "cardinality"] {
            if self.object(node, &owl(pred)).is_some() {
                let n = self.cardinality(node, pred)?;
                self.reduce(
                    "exact cardinality",
                    "expanded as Inter (minCard n) (maxCard n), which is what the \
                     owl-lean fragment note means by \"yes, as min+max\"",
                );
                return Some(Concept::Inter(
                    Box::new(Concept::MinCard(n, p.clone(), Box::new(filler.clone()))),
                    Box::new(Concept::MaxCard(n, p, Box::new(filler))),
                ));
            }
        }

        self.drop(
            "owl:Restriction with no recognised constraint",
            "carries owl:onProperty but none of someValuesFrom, allValuesFrom, \
             hasValue, hasSelf or a cardinality",
        );
        None
    }

    /// Split the literal-valued triples into genuine data property assertions
    /// and annotations.
    ///
    /// An annotation property is one declared `owl:AnnotationProperty`, or one
    /// of the vocabulary terms the OWL 2 Structural Specification fixes as
    /// built-in annotation properties, or a term from the usual annotation
    /// vocabularies. The distinction matters because only the first kind is a
    /// weakening of the axiom set.
    fn count_literal_triples(&self) -> (u64, u64) {
        const BUILT_IN: [&str; 9] = [
            "http://www.w3.org/2000/01/rdf-schema#label",
            "http://www.w3.org/2000/01/rdf-schema#comment",
            "http://www.w3.org/2000/01/rdf-schema#seeAlso",
            "http://www.w3.org/2000/01/rdf-schema#isDefinedBy",
            "http://www.w3.org/2002/07/owl#versionInfo",
            "http://www.w3.org/2002/07/owl#deprecated",
            "http://www.w3.org/2002/07/owl#backwardCompatibleWith",
            "http://www.w3.org/2002/07/owl#incompatibleWith",
            "http://www.w3.org/2002/07/owl#priorVersion",
        ];
        const PREFIXES: [&str; 4] = [
            "http://www.w3.org/2004/02/skos/core#",
            "http://purl.org/dc/elements/1.1/",
            "http://purl.org/dc/terms/",
            "http://www.w3.org/ns/prov#",
        ];
        let (mut data, mut anno) = (0u64, 0u64);
        for (p, o) in self.by_subject.values().flatten() {
            if !is_literal(o) {
                continue;
            }
            let bp = bare(p);
            let is_anno = self.annotation_properties.contains(bp)
                || BUILT_IN.contains(&bp)
                || PREFIXES.iter().any(|pre| bp.starts_with(pre));
            if is_anno {
                anno += 1;
            } else {
                data += 1;
            }
        }
        (data, anno)
    }

    /// Constructs that have no place in the fragment at all. Counted by
    /// scanning for the predicate or the class, the same shape as
    /// `DlReasoner::unmodelled_constructs`.
    /// How many assertions a punned subject carries.
    ///
    /// Mirrors the assertion loop's own conditions exactly, so the count is
    /// the number of axioms that WOULD have been exported had the subject not
    /// been declared an entity of another kind. Declarations, subsumptions,
    /// domains, ranges and annotations are not assertions and are not counted:
    /// those ARE exported for a punned subject, which is why the omission was
    /// invisible.
    fn count_punned_assertions(&mut self, s: &str) -> u64 {
        let mut n = 0u64;
        let pairs = self.by_subject.get(s).cloned().unwrap_or_default();
        for (p, o) in &pairs {
            let bp = bare(p).to_string();
            if bp == RDF_TYPE {
                let bo = bare(o).to_string();
                if !bo.starts_with(OWL)
                    && !bo.starts_with("http://www.w3.org/2000/01/rdf-schema#")
                    && self.concept(o).is_some()
                {
                    n += 1;
                }
                continue;
            }
            if bp.starts_with(OWL)
                || bp.starts_with("http://www.w3.org/2000/01/rdf-schema#")
                || bp.starts_with("http://www.w3.org/1999/02/22-rdf-syntax-ns#")
                || is_literal(o)
            {
                continue;
            }
            if self.is_object_property(&bp) {
                n += 1;
            }
        }
        n
    }

    fn count_out_of_fragment(&mut self) {
        const OUT: [(&str, &str, &str); 6] = [
            (
                "owl:hasKey",
                "hasKey",
                "no constructor in OwlLean/Syntax.lean; measured at 4.3% of constrained \
                 ontologies and excluded from the fragment on that measurement",
            ),
            (
                "owl:withRestrictions",
                "withRestrictions",
                "datatype facets need the OWL 2 datatype map, which is a front end concern \
                 owl-lean has not started",
            ),
            (
                "owl:onDatatype",
                "onDatatype",
                "datatype facets; see owl:withRestrictions",
            ),
            (
                "owl:datatypeComplementOf",
                "datatypeComplementOf",
                "no datatype-expression layer in the fragment",
            ),
            (
                "owl:NegativePropertyAssertion",
                "NegativePropertyAssertion",
                "no constructor in OwlLean/Syntax.lean",
            ),
            (
                "owl:AllDisjointProperties",
                "AllDisjointProperties",
                "reduced to pairwise owl:propertyDisjointWith where every member is a \
                 declared object property, dropped otherwise",
            ),
        ];
        let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
        for (s, ps) in &self.by_subject {
            for (p, o) in ps {
                for (label, local, _) in OUT {
                    let iri = owl(local);
                    if bare(p) == iri || bare(o) == iri || bare(s) == iri {
                        *counts.entry(label).or_default() += 1;
                    }
                }
            }
        }
        for (label, _, why) in OUT {
            if let Some(n) = counts.get(label) {
                let e = self
                    .dropped
                    .entry(label.to_string())
                    .or_insert((0, why.to_string()));
                e.0 += n;
            }
        }
    }

    fn read(mut self) -> ReadOntology {
        let mut axioms = Vec::new();
        let subjects: Vec<String> = self.by_subject.keys().cloned().collect();

        for s in &subjects {
            let pairs = self.by_subject.get(s).cloned().unwrap_or_default();
            let bare_s = bare(s).to_string();

            for (p, o) in &pairs {
                let bp = bare(p).to_string();
                let bo = bare(o).to_string();

                match bp.as_str() {
                    RDFS_SUBCLASS => {
                        if let (Some(c), Some(d)) =
                            (self.concept(s), self.concept(o))
                        {
                            axioms.push(OwlAxiom::SubClass(c, d));
                        }
                    }
                    RDFS_SUBPROP => {
                        let so = self.is_object_property(&bare_s);
                        let oo = is_iri(o) && self.is_object_property(&bo);
                        if so && oo {
                            axioms.push(OwlAxiom::SubOProp(
                                Ope::Named(bare_s.clone()),
                                Ope::Named(bo.clone()),
                            ));
                        } else {
                            self.drop(
                                "rdfs:subPropertyOf outside the object-property case",
                                "OwlLean/Syntax.lean's subOProp relates two OPEs; there is \
                                 no data-property hierarchy constructor",
                            );
                        }
                    }
                    RDFS_DOMAIN => {
                        if self.is_object_property(&bare_s) {
                            if let Some(c) = self.concept(o) {
                                axioms.push(OwlAxiom::OPropDomain(
                                    Ope::Named(bare_s.clone()),
                                    c,
                                ));
                            }
                        } else if let Some(c) = self.concept(o) {
                            axioms.push(OwlAxiom::DPropDomain(bare_s.clone(), c));
                        }
                    }
                    RDFS_RANGE => {
                        if self.is_object_property(&bare_s) {
                            if let Some(c) = self.concept(o) {
                                axioms.push(OwlAxiom::OPropRange(
                                    Ope::Named(bare_s.clone()),
                                    c,
                                ));
                            }
                        } else {
                            axioms.push(OwlAxiom::DPropRange(bare_s.clone(), bo.clone()));
                        }
                    }
                    _ => {}
                }

                if bp == owl("equivalentClass") {
                    if self.has_type(s, RDFS_DATATYPE) || self.has_type(o, RDFS_DATATYPE) {
                        self.drop(
                            "owl:equivalentClass between datatypes",
                            "a datatype definition; no datatype-expression layer in the fragment",
                        );
                    } else if let (Some(c), Some(d)) = (self.concept(s), self.concept(o)) {
                        axioms.push(OwlAxiom::EquivClass(c, d));
                    }
                } else if bp == owl("disjointWith") {
                    if let (Some(c), Some(d)) = (self.concept(s), self.concept(o)) {
                        axioms.push(OwlAxiom::DisjointWith(c, d));
                    }
                } else if bp == owl("inverseOf") {
                    if is_iri(s) && is_iri(o) {
                        axioms.push(OwlAxiom::InverseOf(bare_s.clone(), bo.clone()));
                    } else {
                        self.drop(
                            "owl:inverseOf on an anonymous property expression",
                            "Axiom.inverseOf relates two named object properties",
                        );
                    }
                } else if bp == owl("propertyDisjointWith") {
                    axioms.push(OwlAxiom::PropDisjoint(bare_s.clone(), bo.clone()));
                } else if bp == owl("equivalentProperty") {
                    let so = self.is_object_property(&bare_s);
                    let oo = is_iri(o) && self.is_object_property(&bo);
                    if so && oo {
                        self.reduce(
                            "owl:equivalentProperty",
                            "expanded as two subOProp axioms, which is what \
                             EquivalentObjectProperties means under the Direct Semantics",
                        );
                        axioms.push(OwlAxiom::SubOProp(
                            Ope::Named(bare_s.clone()),
                            Ope::Named(bo.clone()),
                        ));
                        axioms.push(OwlAxiom::SubOProp(
                            Ope::Named(bo.clone()),
                            Ope::Named(bare_s.clone()),
                        ));
                    } else {
                        self.drop(
                            "owl:equivalentProperty outside the object-property case",
                            "there is no data-property hierarchy constructor in the fragment",
                        );
                    }
                } else if bp == owl("propertyChainAxiom") {
                    match self.list(o) {
                        Some(items) if items.iter().all(|i| is_iri(i)) => {
                            axioms.push(OwlAxiom::Chain(
                                items.iter().map(|i| bare(i).to_string()).collect(),
                                bare_s.clone(),
                            ));
                        }
                        _ => {
                            self.drop(
                                "owl:propertyChainAxiom with a malformed or anonymous chain",
                                "Axiom.chain takes a list of named object properties",
                            );
                        }
                    }
                } else if bp == owl("sameAs") {
                    axioms.push(OwlAxiom::SameAs(bare_s.clone(), bo.clone()));
                } else if bp == owl("differentFrom") {
                    axioms.push(OwlAxiom::DifferentFrom(bare_s.clone(), bo.clone()));
                } else if bp == owl("disjointUnionOf") {
                    match self.list(o) {
                        Some(items) => {
                            self.reduce(
                                "owl:disjointUnionOf",
                                "expanded as one equivalentClass against the union plus the \
                                 pairwise disjointness axioms, which is its definition",
                            );
                            let parts: Vec<Concept> = items
                                .iter()
                                .filter_map(|i| self.concept(i))
                                .collect();
                            if parts.len() == items.len()
                                && let Some(whole) = self.concept(s)
                                    && let Some(u) = fold_union(&parts) {
                                        axioms.push(OwlAxiom::EquivClass(whole, u));
                                        for (i, a) in parts.iter().enumerate() {
                                            for b in &parts[i + 1..] {
                                                axioms.push(OwlAxiom::DisjointWith(
                                                    a.clone(),
                                                    b.clone(),
                                                ));
                                            }
                                        }
                                    }
                        }
                        None => {
                            self.drop("owl:disjointUnionOf with a malformed list", "see above");
                        }
                    }
                }
            }

            // Property characteristics, from rdf:type. Every one of them takes
            // an S.OProp in OwlLean/Syntax.lean, so a characteristic on a data
            // property is outside the fragment and is named rather than
            // exported under the wrong symbol.
            for (ty, mk) in [
                ("TransitiveProperty", 0u8),
                ("SymmetricProperty", 1),
                ("AsymmetricProperty", 2),
                ("ReflexiveProperty", 3),
                ("IrreflexiveProperty", 4),
                ("InverseFunctionalProperty", 5),
                ("FunctionalProperty", 6),
            ] {
                if !self.has_type(s, &owl(ty)) {
                    continue;
                }
                if !self.is_object_property(&bare_s) {
                    self.drop(
                        &format!("owl:{ty} on a data property"),
                        "every property characteristic in OwlLean/Syntax.lean takes an \
                         S.OProp; OWL 2 DL forbids these on a data property and RDF \
                         vocabularies assert them anyway",
                    );
                    continue;
                }
                axioms.push(match mk {
                    0 => OwlAxiom::Transitive(bare_s.clone()),
                    1 => OwlAxiom::Symmetric(bare_s.clone()),
                    2 => OwlAxiom::Asymmetric(bare_s.clone()),
                    3 => OwlAxiom::Reflexive(bare_s.clone()),
                    4 => OwlAxiom::Irreflexive(bare_s.clone()),
                    5 => OwlAxiom::InvFunctional(bare_s.clone()),
                    _ => OwlAxiom::Functional(bare_s.clone()),
                });
            }

            // n-ary disjointness and difference, via their pairwise readings.
            if self.has_type(s, &owl("AllDisjointClasses"))
                && let Some(l) = self.object(s, &owl("members")).map(str::to_string) {
                    match self.list(&l) {
                        Some(items) => {
                            self.reduce(
                                "owl:AllDisjointClasses",
                                "expanded pairwise into Axiom.disjointWith",
                            );
                            let parts: Vec<Concept> =
                                items.iter().filter_map(|i| self.concept(i)).collect();
                            for (i, a) in parts.iter().enumerate() {
                                for b in &parts[i + 1..] {
                                    axioms.push(OwlAxiom::DisjointWith(a.clone(), b.clone()));
                                }
                            }
                        }
                        None => {
                            self.drop("owl:AllDisjointClasses with a malformed list", "see above");
                        }
                    }
                }
            if self.has_type(s, &owl("AllDifferent")) {
                let head = self
                    .object(s, &owl("members"))
                    .or_else(|| self.object(s, &owl("distinctMembers")))
                    .map(str::to_string);
                if let Some(l) = head {
                    match self.list(&l) {
                        Some(items) => {
                            self.reduce(
                                "owl:AllDifferent",
                                "expanded pairwise into Axiom.differentFrom",
                            );
                            for (i, a) in items.iter().enumerate() {
                                for b in &items[i + 1..] {
                                    axioms.push(OwlAxiom::DifferentFrom(
                                        bare(a).to_string(),
                                        bare(b).to_string(),
                                    ));
                                }
                            }
                        }
                        None => {
                            self.drop("owl:AllDifferent with a malformed list", "see above");
                        }
                    }
                }
            }
        }

        // Assertions. An individual is a subject that is neither a class, a
        // property, nor a piece of OWL syntax.
        //
        // A subject that IS one of those and still carries an assertion is
        // PUNNED: the same IRI used as a class in one triple and as an
        // individual in another. OWL 2 DL allows that, `OwlLean/Syntax.lean`
        // does not — `Sig` gives each entity kind its own type — so the
        // assertion is outside the fragment and is not exported.
        //
        // It used to be dropped SILENTLY, and that was found by running the
        // model-certificate pipeline over the shipped case studies: five of
        // the nine derivations the OWL-RL reasoner claims on
        // `case-studies/blast-furnace-ironmaking/` came back with a
        // machine-checked countermodel, because `bf:Hanging`, a class, also
        // carries `bf:hasSeverity bf:HighSeverity` and the domain axiom on
        // that property is what the reasoner used. The export was genuinely
        // weaker than the graph and the report said `exports_a_weaker_axiom_set:
        // false`, which is the laundering shape this project exists to catch,
        // in its own output. Counted here so the report is true; the reading
        // itself is unchanged.
        let mut punned = 0u64;
        for s in &subjects {
            let bare_s = bare(s).to_string();
            let is_entity = self.classes.contains(&bare_s)
                || self.object_properties.contains(&bare_s)
                || self.data_properties.contains(&bare_s);
            if is_entity || bare_s.starts_with(OWL) {
                if is_entity {
                    punned += self.count_punned_assertions(s);
                }
                continue;
            }
            let pairs = self.by_subject.get(s).cloned().unwrap_or_default();
            for (p, o) in &pairs {
                let bp = bare(p).to_string();
                if bp == RDF_TYPE {
                    let bo = bare(o).to_string();
                    if bo.starts_with(OWL) || bo.starts_with("http://www.w3.org/2000/01/rdf-schema#")
                    {
                        continue; // a declaration, not a class assertion
                    }
                    if let Some(c) = self.concept(o) {
                        axioms.push(OwlAxiom::ClassAssert(c, bare_s.clone()));
                    }
                    continue;
                }
                if bp.starts_with(OWL)
                    || bp.starts_with("http://www.w3.org/2000/01/rdf-schema#")
                    || bp.starts_with("http://www.w3.org/1999/02/22-rdf-syntax-ns#")
                {
                    continue;
                }
                if is_literal(o) {
                    continue; // a data property assertion; see below
                }
                if self.is_object_property(&bp) {
                    axioms.push(OwlAxiom::OPropAssert(
                        bp.clone(),
                        bare_s.clone(),
                        bare(o).to_string(),
                    ));
                }
            }
        }

        if punned > 0 {
            self.dropped.insert(
                "assertion on a punned entity".to_string(),
                (
                    punned,
                    "the subject is declared a class or a property AND carries an assertion, so                      the same IRI is used as two entity kinds. OWL 2 DL permits the punning;                      OwlLean/Syntax.lean does not, because `Sig` gives each entity kind its own                      type and `Sig.Ind` is disjoint from `Sig.Cls`. The assertion is therefore                      outside the fragment and the exported theory is WEAKER than the graph. An                      RDFS or OWL-RL reasoner will still derive consequences from it, so a                      conjecture that rests on one is genuinely not entailed by this export and                      is not evidence of a defect in either side"
                        .to_string(),
                ),
            );
        }
        self.count_out_of_fragment();
        // Data property assertions have no constructor in the fragment: the
        // Axiom type has oPropAssert and no dPropAssert.
        //
        // ANNOTATIONS ARE NOT COUNTED HERE. `rdfs:label` and its kin carry no
        // Direct Semantics content, so ignoring them does not weaken the axiom
        // set, and counting them would bury the constructs that DO weaken it
        // under hundreds of labels. gUFO alone has 222 literal-valued triples
        // of which almost all are annotations. They are reported separately,
        // under their own heading, so nothing is silent either way.
        let (dpa, annotations) = self.count_literal_triples();
        self.annotations_ignored = annotations;
        if dpa > 0 {
            self.dropped.insert(
                "data property assertion".to_string(),
                (
                    dpa,
                    "OwlLean/Syntax.lean's Axiom type has oPropAssert and no dPropAssert; \
                     a triple with a literal object is not exported"
                        .to_string(),
                ),
            );
        }

        // Resolve every anonymous class expression once more, so a conjecture
        // naming one can be built. The drop and reduce ledgers are snapshotted
        // across this pass: it re-reads nodes the axiom walk already read, and
        // counting them twice would misreport how much the export omits.
        let dropped_before = self.dropped.clone();
        let reduced_before = self.reduced.clone();
        let mut anonymous_classes = BTreeMap::new();
        for s in &subjects {
            let anon = s.starts_with("_:")
                || self.has_type(s, &owl("Restriction"))
                || self.object(s, &owl("intersectionOf")).is_some()
                || self.object(s, &owl("unionOf")).is_some()
                || self.object(s, &owl("complementOf")).is_some()
                || self.object(s, &owl("oneOf")).is_some();
            if !anon {
                continue;
            }
            if let Some(k) = self.concept(s) {
                anonymous_classes.insert(bare(s).to_string(), k);
            }
        }
        self.dropped = dropped_before;
        self.reduced = reduced_before;

        // The datatype vocabulary: anything declared rdfs:Datatype, anything
        // standing in a datatype position in an exported axiom, plus the two
        // families every ontology uses without declaring.
        let mut datatypes: BTreeSet<String> = self
            .by_subject
            .keys()
            .filter(|s| self.is_datatype_node(s))
            .map(|s| bare(s).to_string())
            .collect();
        fn concept_datatypes(c: &Concept, out: &mut BTreeSet<String>) {
            match c {
                Concept::DataSome(_, t) | Concept::DataAll(_, t) => {
                    out.insert(t.clone());
                }
                Concept::Inter(a, b) | Concept::Union(a, b) => {
                    concept_datatypes(a, out);
                    concept_datatypes(b, out);
                }
                Concept::Compl(a) => concept_datatypes(a, out),
                Concept::Some_(_, a) | Concept::All_(_, a) => concept_datatypes(a, out),
                Concept::MinCard(_, _, a) | Concept::MaxCard(_, _, a) => {
                    concept_datatypes(a, out)
                }
                _ => {}
            }
        }
        for a in &axioms {
            match a {
                OwlAxiom::DPropRange(_, t) => {
                    datatypes.insert(t.clone());
                }
                OwlAxiom::SubClass(c, d)
                | OwlAxiom::EquivClass(c, d)
                | OwlAxiom::DisjointWith(c, d) => {
                    concept_datatypes(c, &mut datatypes);
                    concept_datatypes(d, &mut datatypes);
                }
                OwlAxiom::OPropDomain(_, c)
                | OwlAxiom::OPropRange(_, c)
                | OwlAxiom::DPropDomain(_, c)
                | OwlAxiom::ClassAssert(c, _) => concept_datatypes(c, &mut datatypes),
                _ => {}
            }
        }

        let mut object_properties = self.object_properties.clone();
        object_properties.extend(self.inferred_object.iter().cloned());
        let mut data_properties = self.data_properties.clone();
        data_properties.extend(self.inferred_data.iter().cloned());

        ReadOntology {
            axioms,
            datatypes,
            object_properties,
            data_properties,
            annotations_ignored: self.annotations_ignored,
            anonymous_classes,
            dropped: self
                .dropped
                .into_iter()
                .map(|(construct, (occurrences, why))| Dropped {
                    construct,
                    occurrences,
                    why,
                })
                .collect(),
            reduced: self
                .reduced
                .into_iter()
                .map(|(construct, (occurrences, how))| Reduced {
                    construct,
                    occurrences,
                    how,
                })
                .collect(),
            inferred_object_properties: self.inferred_object.into_iter().collect(),
            inferred_data_properties: self.inferred_data.into_iter().collect(),
        }
    }
}

fn fold_union(parts: &[Concept]) -> Option<Concept> {
    let mut it = parts.iter().rev();
    let mut acc = it.next()?.clone();
    for p in it {
        acc = Concept::Union(Box::new(p.clone()), Box::new(acc));
    }
    Some(acc)
}

/// Read the loaded graph into the `OwlLean/Syntax.lean` fragment.
pub fn read_graph(triples: Vec<(String, String, String)>) -> ReadOntology {
    Reader::new(triples).read()
}

/// Translate one RDF triple into a fragment axiom, for use as a conjecture.
///
/// A derived triple from `reason --certificate` is one of a handful of shapes.
/// Anything else returns `None` and the caller must report it rather than
/// quietly not asking the question.
pub fn triple_as_axiom(
    read: &ReadOntology,
    s: &str,
    p: &str,
    o: &str,
) -> Result<OwlAxiom, String> {
    let (bs, bp, bo) = (bare(s), bare(p), bare(o));
    // A class position that is not a named class must resolve to the class
    // EXPRESSION the graph gives it, or the goal is not asked at all. Building
    // an atom out of a blank node would ask about a symbol that occurs in no
    // axiom, and the prover's inevitable non-answer would look like a limit of
    // the prover instead of a defect here.
    // Declarations win; an undeclared property that was never seen with a
    // literal object is an object property, which is what the reader assumed
    // when it built the axioms.
    let is_object = |prop: &str| -> bool { !read.data_properties.contains(prop) };
    // A datatype is one the reader saw in a datatype position, one declared
    // rdfs:Datatype, or a member of the two families every ontology uses
    // without declaring.
    let is_datatype = |node: &str| -> bool {
        read.datatypes.contains(node)
            || node.starts_with("http://www.w3.org/2001/XMLSchema#")
            || node == "http://www.w3.org/2000/01/rdf-schema#Literal"
            || node == "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString"
    };
    let class_at = |node: &str| -> Result<Concept, String> {
        // The same readings `Reader::concept` gives these two. Without them a
        // goal `X rdfs:subClassOf owl:Thing` becomes a subsumption under an
        // ATOM that occurs in no axiom, rather than under the translation's
        // `top`, and the prover reports a countermodel to a question nobody
        // asked. Fifty-two of FOAF's inferences came back that way, this cause
        // and the entity-kind one below between them.
        if node == owl("Thing") {
            return Ok(Concept::Top);
        }
        if node == owl("Nothing") {
            return Ok(Concept::Bot);
        }
        if is_datatype(node) {
            return Err(format!(
                "{node} is a DATATYPE, and OwlLean/Syntax.lean has no axiom form asserting \
                 that something belongs to one: Axiom.classAssert takes a Concept and no \
                 Concept constructor denotes datatype membership. Translating it as a class \
                 assertion would name `c:{node}`, a symbol the axioms never mention, and the \
                 prover would report a countermodel to a question nobody asked"
            ));
        }
        if node.starts_with("_:") {
            return read.anonymous_classes.get(node).cloned().ok_or_else(|| {
                format!(
                    "the class position is the blank node {node}, which carries no class \
                     expression in THIS graph. Blank node labels are not stable across \
                     loads, so a goal naming one must come from the same graph the \
                     export was built from"
                )
            });
        }
        Ok(read
            .anonymous_classes
            .get(node)
            .cloned()
            .unwrap_or_else(|| Concept::Atom(node.to_string())))
    };
    let no_form = || {
        Err(format!(
            "no axiom form in OwlLean/Syntax.lean corresponds to the triple \
             ({bs} {bp} {bo}), so the question cannot be put to a prover in this \
             translation"
        ))
    };
    if is_literal(s) || is_literal(o) {
        return Err(format!(
            "a literal stands in a position OwlLean/Syntax.lean has no constructor for \
             ({bs} {bp} {bo}); the Axiom type has oPropAssert and no dPropAssert"
        ));
    }
    match bp {
        RDF_TYPE => {
            if bo.starts_with(OWL) || bo.starts_with("http://www.w3.org/2000/01/rdf-schema#") {
                return Err(format!(
                    "{bo} is a vocabulary declaration, not a class assertion, so there is \
                     nothing to ask"
                ));
            }
            Ok(OwlAxiom::ClassAssert(class_at(bo)?, bs.to_string()))
        }
        RDFS_SUBCLASS => Ok(OwlAxiom::SubClass(class_at(bs)?, class_at(bo)?)),
        RDFS_SUBPROP => {
            if is_object(bs) && is_object(bo) {
                Ok(OwlAxiom::SubOProp(
                    Ope::Named(bs.to_string()),
                    Ope::Named(bo.to_string()),
                ))
            } else {
                Err(format!(
                    "{bs} or {bo} is a data property, and OwlLean/Syntax.lean's subOProp \
                     relates two OPEs; there is no data-property hierarchy constructor"
                ))
            }
        }
        RDFS_DOMAIN => {
            if is_object(bs) {
                Ok(OwlAxiom::OPropDomain(Ope::Named(bs.to_string()), class_at(bo)?))
            } else {
                Ok(OwlAxiom::DPropDomain(bs.to_string(), class_at(bo)?))
            }
        }
        RDFS_RANGE => {
            if is_object(bs) {
                Ok(OwlAxiom::OPropRange(Ope::Named(bs.to_string()), class_at(bo)?))
            } else {
                Ok(OwlAxiom::DPropRange(bs.to_string(), bo.to_string()))
            }
        }
        _ if bp == owl("sameAs") => Ok(OwlAxiom::SameAs(bs.to_string(), bo.to_string())),
        _ if bp == owl("differentFrom") => {
            Ok(OwlAxiom::DifferentFrom(bs.to_string(), bo.to_string()))
        }
        _ if bp == owl("inverseOf") => Ok(OwlAxiom::InverseOf(bs.to_string(), bo.to_string())),
        _ if bp == owl("equivalentClass") => {
            Ok(OwlAxiom::EquivClass(class_at(bs)?, class_at(bo)?))
        }
        _ if bp == owl("disjointWith") => {
            Ok(OwlAxiom::DisjointWith(class_at(bs)?, class_at(bo)?))
        }
        _ if bp.starts_with(OWL)
            || bp.starts_with("http://www.w3.org/2000/01/rdf-schema#")
            || bp.starts_with("http://www.w3.org/1999/02/22-rdf-syntax-ns#") =>
        {
            no_form()
        }
        _ if is_object(bp) => Ok(OwlAxiom::OPropAssert(
            bp.to_string(),
            bs.to_string(),
            bo.to_string(),
        )),
        _ => Err(format!(
            "{bp} is a data property, and OwlLean/Syntax.lean's Axiom type has \
             oPropAssert and no dPropAssert"
        )),
    }
}

// ── The export itself ───────────────────────────────────────────────────────

/// Output syntax. FIVE serialisers over ONE translation, never five
/// translations.
///
/// Two of the five are Common Logic dialects. ISO/IEC 24707 defines three —
/// CLIF, CGIF and XCL — and emitting one of them and calling the support
/// "Common Logic" is the kind of partial claim this repository exists to stop
/// making. XCL is still absent and is named as absent in the report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Syntax {
    /// TPTP FOF, the format every first-order prover reads and the one the
    /// differential oracle runs.
    Tptp,
    /// ISO/IEC 24707 Common Logic Interchange Format, restricted to the
    /// first-order-equivalent fragment. The standards-track interchange
    /// syntax for the same first-order content, and the one ISO/IEC 21838-2
    /// publishes BFO in.
    Clif(ClifDialect, ClifComments),
    /// ISO/IEC 24707 Conceptual Graph Interchange Format, CORE dialect, in
    /// the compact sub-dialect clause 7.1.1 names. The second of Common
    /// Logic's three dialects this engine emits; XCL, Annex C, is still not
    /// emitted. There is no dialect flag: unlike CLIF, CGIF has one spelling
    /// of its operators and a lexical comment syntax of its own.
    Cgif,
    /// SMT-LIB 2, the format the SAT/SMT family reads. The goal is asserted
    /// NEGATED, because this is the syntax a MODEL comes back out of and a
    /// model is the thing this repository can certify.
    Smtlib(smtlib::SmtEncoding),
    /// LADR, for Mace4. Mangled symbols, the table beside the file.
    Ladr,
}

impl Syntax {
    /// `dialect` and `comments` are consulted only for CLIF and default to
    /// `iso` and `standalone`; `domain` only for SMT-LIB, where `None` is the
    /// unbounded encoding.
    pub fn parse(
        s: &str,
        dialect: Option<&str>,
        comments: Option<&str>,
        domain: Option<u32>,
    ) -> anyhow::Result<Syntax> {
        match s.to_ascii_lowercase().as_str() {
            "tptp" | "fof" | "tptp-fof" => Ok(Syntax::Tptp),
            "clif" | "cl" | "common-logic" => Ok(Syntax::Clif(
                ClifDialect::parse(dialect.unwrap_or("iso"))?,
                ClifComments::parse(comments.unwrap_or("standalone"))?,
            )),
            "cgif" | "cg" | "conceptual-graph" => Ok(Syntax::Cgif),
            "smtlib" | "smt" | "smt2" | "smt-lib" => Ok(Syntax::Smtlib(match domain {
                None => smtlib::SmtEncoding::Unbounded,
                Some(0) => anyhow::bail!(
                    "--smt-domain 0 asks for a model with an empty carrier; a first-order \
                     structure cannot have one, and `Fol.FinModel` is defined only at n+1"
                ),
                Some(k) => smtlib::SmtEncoding::Finite(k),
            })),
            "ladr" | "mace4" | "prover9" => Ok(Syntax::Ladr),
            other => anyhow::bail!(
                "unknown first-order syntax {other:?}; expected `tptp`, `clif`, `cgif`, \
                 `smtlib` or `ladr`"
            ),
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            Syntax::Tptp => "p",
            Syntax::Clif(..) => "clif",
            Syntax::Cgif => "cgif",
            Syntax::Smtlib(_) => "smt2",
            Syntax::Ladr => "in",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Syntax::Tptp => "tptp",
            Syntax::Clif(..) => "clif",
            Syntax::Cgif => "cgif",
            Syntax::Smtlib(_) => "smtlib",
            Syntax::Ladr => "ladr",
        }
    }
    pub fn dialect(self) -> Option<ClifDialect> {
        match self {
            Syntax::Clif(d, _) => Some(d),
            _ => None,
        }
    }
    pub fn comments(self) -> Option<ClifComments> {
        match self {
            Syntax::Clif(_, c) => Some(c),
            _ => None,
        }
    }
    pub fn encoding(self) -> Option<smtlib::SmtEncoding> {
        match self {
            Syntax::Smtlib(e) => Some(e),
            _ => None,
        }
    }
    /// Render. Fallible from the SMT-LIB and LADR arms on: both refuse a
    /// symbol they cannot write rather than mangling it into a different one.
    fn render(self, problem: &FolProblem, name: &str) -> anyhow::Result<String> {
        Ok(match self {
            Syntax::Tptp => problem.to_tptp(),
            Syntax::Clif(d, c) => problem.to_clif(d, c, name),
            Syntax::Cgif => problem.to_cgif(name)?,
            Syntax::Smtlib(e) => problem.to_smtlib(e)?,
            Syntax::Ladr => {
                let tab = ladr::SymbolTable::build(problem)?;
                ladr::problem(problem, &tab, 30)?
            }
        })
    }
}

/// One goal that could not be asked.
#[derive(Debug, serde::Serialize)]
pub struct GoalNotAsked {
    pub triple: [String; 3],
    pub why: String,
}

/// Export the loaded ontology to `dir`, and optionally one problem per goal.
///
/// `goals` is a TSV whose first three tab-separated columns are a triple in
/// N-Triples spelling. `derivations.tsv` from `reason --certificate` has the
/// rule in column one, so `goals_skip_columns` is 1 for that file and 0 for a
/// plain triple list.
pub fn export(
    graph: &std::sync::Arc<crate::graph::GraphStore>,
    dir: &std::path::Path,
    syntax: Syntax,
    goals: Option<&std::path::Path>,
    goals_skip_columns: usize,
) -> anyhow::Result<String> {
    let triples = graph.all_triples()?;
    let initial_triples = triples.len();
    // The text name, for CLIF. The ontology's own IRI when it declares one,
    // because a name that identifies the source is worth more than a unique
    // one, and a stated fallback otherwise.
    let ontology_iri = triples
        .iter()
        .find(|(_, p, o)| bare(p) == RDF_TYPE && bare(o) == owl("Ontology"))
        .map(|(s, _, _)| bare(s).to_string())
        .filter(|s| !s.starts_with("_:"))
        .unwrap_or_else(|| UNNAMED_TEXT.to_string());
    let read = read_graph(triples);

    let problem = FolProblem::build(&read.axioms, None)?;
    std::fs::create_dir_all(dir)?;
    let main = dir.join(format!("ontology.{}", syntax.extension()));
    std::fs::write(&main, syntax.render(&problem, &ontology_iri)?)?;
    // LADR mangles every symbol, so the file is unreadable without the table.
    // Written beside it rather than into it, because Mace4 echoes its input
    // and a comment per symbol would bury the clause block a reader needs.
    if syntax == Syntax::Ladr {
        std::fs::write(
            dir.join("symbols.tsv"),
            ladr::table_tsv(&ladr::SymbolTable::build(&problem)?),
        )?;
    }
    // The checker's own format, for every syntax, so that a run of any solver
    // over any of these files can be handed to `oo-folmodel` without going
    // back through the engine. The digest is what binds the two.
    let (problem_tsv, problem_digest) = problem.to_problem_tsv()?;
    std::fs::write(dir.join("problem.tsv"), &problem_tsv)?;

    let mut goal_files = Vec::new();
    let mut not_asked: Vec<GoalNotAsked> = Vec::new();
    if let Some(path) = goals {
        let text = std::fs::read_to_string(path)?;
        let goals_dir = dir.join("goals");
        std::fs::create_dir_all(&goals_dir)?;
        let mut manifest = Vec::new();
        for (i, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < goals_skip_columns + 3 {
                not_asked.push(GoalNotAsked {
                    triple: [line.to_string(), String::new(), String::new()],
                    why: format!(
                        "fewer than {} tab-separated columns",
                        goals_skip_columns + 3
                    ),
                });
                continue;
            }
            let (s, p, o) = (
                cols[goals_skip_columns],
                cols[goals_skip_columns + 1],
                cols[goals_skip_columns + 2],
            );
            let ax = match triple_as_axiom(&read, s, p, o) {
                Ok(ax) => ax,
                Err(why) => {
                    not_asked.push(GoalNotAsked {
                        triple: [s.to_string(), p.to_string(), o.to_string()],
                        why,
                    });
                    continue;
                }
            };
            let gp = FolProblem::build(&read.axioms, Some(&ax))?;
            let name = format!("goal_{i:05}.{}", syntax.extension());
            std::fs::write(
                goals_dir.join(&name),
                syntax.render(&gp, &format!("{ontology_iri}#goal-{i:05}"))?,
            )?;
            let (gtsv, gdigest) = gp.to_problem_tsv()?;
            let gtsv_name = format!("goal_{i:05}.problem.tsv");
            std::fs::write(goals_dir.join(&gtsv_name), gtsv)?;
            if syntax == Syntax::Ladr {
                std::fs::write(
                    goals_dir.join(format!("goal_{i:05}.symbols.tsv")),
                    ladr::table_tsv(&ladr::SymbolTable::build(&gp)?),
                )?;
            }
            manifest.push(serde_json::json!({
                "file": name,
                "checker_problem_file": gtsv_name,
                "checker_problem_digest": gdigest,
                "triple": [s, p, o],
                "axiom_form": axiom_label(&ax),
            }));
            goal_files.push(1);
        }
        std::fs::write(
            dir.join("goals.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "goals": manifest,
                "not_asked": not_asked,
            }))?,
        )?;
    }

    let report = serde_json::json!({
        "syntax": syntax.name(),
        "clif_dialect": syntax.dialect().map(|d| d.name()),
        "clif_comments": syntax.comments().map(|c| c.name()),
        "clif_text_name": syntax.dialect().map(|_| ontology_iri.clone()),
        // CGIF has no dialect FLAG, and it does have a dialect: core rather
        // than extended, and a compact sub-dialect of that. Reported as a value
        // so a consumer never has to read the header to learn which.
        "cgif_dialect": (syntax == Syntax::Cgif).then_some("core"),
        "cgif_sub_dialect": (syntax == Syntax::Cgif).then_some(
            "compact (no sequence markers, ISO/IEC 24707 clause 7.1.1), unstructured (no \
             titlings or importation) and single domain (no domain restrictions). Also no `#?` \
             type label, which is how B.2.7 quantifies over relations, and no actor, which is \
             how B.2.1 writes a function"),
        "cgif_text_name": (syntax == Syntax::Cgif).then(|| ontology_iri.clone()),
        "common_logic_dialects_emitted": ["clif", "cgif"],
        "common_logic_dialects_not_emitted": ["xcl (ISO/IEC 24707 Annex C, the XML dialect)"],
        "smt_encoding": syntax.encoding().map(|e| e.name()),
        "dir": dir.display().to_string(),
        "ontology_file": main.display().to_string(),
        "checker_problem_file": dir.join("problem.tsv").display().to_string(),
        "checker_problem_digest": problem_digest,
        "ladr_symbol_table": (syntax == Syntax::Ladr)
            .then(|| dir.join("symbols.tsv").display().to_string()),
        "initial_triples": initial_triples,
        "axioms_exported": problem.axioms.len(),
        "background_axioms": problem.background.len(),
        "individual_typing_axioms": problem.ind_axioms.len(),
        "individuals": problem.individuals.len(),
        "goal_problems": goal_files.len(),
        "goals_not_asked": not_asked.len(),
        // The description-logic layer already carries this flag for its model
        // certificates. An ontology whose unexported constructs are invisible
        // is a trap, so the flag and the list are in the OUTPUT and not only
        // in a comment.
        "exports_a_weaker_axiom_set": !read.dropped.is_empty(),
        "constructs_not_exported": read.dropped,
        "reduced_to_fragment": read.reduced,
        "annotations_ignored": {
            "count": read.annotations_ignored,
            "why": "an annotation carries no OWL 2 Direct Semantics content, so ignoring \
                    one does not weaken the axiom set. Counted separately from \
                    constructs_not_exported for that reason, and counted rather than \
                    passed over in silence",
        },
        "entity_kinds_inferred": {
            "why": "a property with no owl:ObjectProperty or owl:DatatypeProperty \
                    declaration is classified by whether it ever takes a literal object. \
                    That is an inference, not a reading, so it is listed",
            "as_object_property": read.inferred_object_properties,
            "as_data_property": read.inferred_data_properties,
        },
        "translation": {
            "mirrors": "owl-lean OwlLean/Translation.lean (tr, trAx, background, indAxioms)",
            "theorem": "OwlLean.adequacy",
            "theorem_axioms": ["propext", "Classical.choice", "Quot.sound"],
            "correspondence": "PINNED BY tests/fol_translation_correspondence_test.rs AND \
                               NOT ITSELF PROVED. Nothing mechanically checks that this Rust \
                               is that Lean",
            "freshness": "every entry into the concept translation goes through \
                          Translation::concept_fresh, which refuses unless the subject \
                          variable is strictly below the counter. OwlLean.tr_bridge holds \
                          only under that condition and \
                          OwlLean.Refutations.tr_bridge_needs_freshness is the countermodel",
            "individual_typing": "thing(a) is emitted for every individual name in the \
                                  signature. Without these axioms adequacy is FALSE: \
                                  OwlLean.Refutations.adequacy_needs_ind_axioms refutes the \
                                  left-to-right direction with the empty ontology and the \
                                  axiom Top(a)",
        },
        "not_certified": "a prover's verdict on these files is an ORACLE OPINION and never a \
                          certificate. Checking a superposition refutation needs a verified \
                          first-order calculus with unification, which does not exist in core \
                          Lean. Use tools/fol_differential.py, which reports disagreement and \
                          does not adjudicate it",
    });
    Ok(report.to_string())
}
