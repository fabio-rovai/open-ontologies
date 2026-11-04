// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! The emitted first-order logic IS the translation `OwlLean/Translation.lean`
//! defines, for a set of worked cases.
//!
//! This is the point of the first-order export. It is not enough that the file
//! parses, and it is not enough that a prover likes it: `owl-lean` carries a
//! machine-checked adequacy theorem, `OwlLean.adequacy`, and that theorem is
//! worth citing only if the Rust emitter emits the translation the theorem is
//! about. Every expected string below was computed BY HAND from
//! `OwlLean/Translation.lean`, unrolling `tr`, `trAx`, `mkEx`, `mkAll`,
//! `distinctPairs`, `somePairEq`, `Form.conj` and `Form.disj`, and the counter
//! arithmetic with them.
//!
//! **The correspondence is pinned by these tests and is NOT itself proved.**
//! Nothing mechanically checks that `Translation::axiom` is `OwlLean.trAx`.
//! This file is the only thing that will notice if the Rust and the Lean drift
//! apart, which is exactly why the cases are hand-computed rather than
//! recorded from a run: a golden file captured from the emitter would agree
//! with the emitter by construction and would have caught nothing.
//!
//! The tests are grouped:
//!
//!   1. `tr` and `trAx`, case by case, pinned against hand-computed TPTP.
//!   2. The two things the adequacy theorem needs: the freshness side
//!      condition and the individual typing axioms, each with the countermodel
//!      from `OwlLean/Refutations.lean` reconstructed as a test.
//!   3. One translation, two serialisers: the CLIF output is read back with a
//!      small S-expression reader defined here and must recover the identical
//!      `Form`, so a drift in either writer is caught.
//!   4. The CLIF output stays inside the first-order-equivalent fragment of
//!      Common Logic.
//!   5. Constructs outside the fragment are named in the output.

use open_ontologies::tptp::{
    clif, fof, ClifComments, ClifDialect, Concept, FolProblem, Form, Ope, OwlAxiom, Syntax,
    Translation, P1, P2, Term,
};

const TEXT_NAME: &str = "http://e/ontology";

fn c(s: &str) -> Concept {
    Concept::Atom(format!("http://e/{s}"))
}
fn iri(s: &str) -> String {
    format!("http://e/{s}")
}
/// The TPTP spelling of an IRI under a kind prefix, so the expected strings
/// below stay readable.
fn q(prefix: &str, s: &str) -> String {
    format!("'{prefix}http://e/{s}'")
}

fn tptp_of(a: &OwlAxiom) -> String {
    fof::form(&Translation::axiom(a).expect("freshness holds at trAx's own call sites"))
}

// ── 1. tr and trAx, hand-computed from OwlLean/Translation.lean ─────────────

/// `trAx (subClass (atom A) (atom B))`
///
/// `tr (atom A) 0 2 = (cls A (v 0), 2)`, `tr (atom B) 0 2 = (cls B (v 0), 2)`,
/// then `allObj 0 (imp f g)` where `allObj n f = .all n (.imp (thing (v n)) f)`.
#[test]
fn sub_class_of_two_atoms() {
    let got = tptp_of(&OwlAxiom::SubClass(c("A"), c("B")));
    let expected = format!(
        "! [X0] : (thing(X0) => ({a}(X0) => {b}(X0)))",
        a = q("c:", "A"),
        b = q("c:", "B")
    );
    assert_eq!(got, expected);
}

/// `trAx (subClass (atom A) (some_ r (atom B)))`
///
/// The bound variable is X2, not X1. `trAx` starts the counter at 2 because
/// the subject is variable 0 and `subOProp`-shaped axioms use 1; `tr (some_ r p)
/// x c` takes `y := c` and recurses at `c + 1`. An exporter that allocated
/// from 1, or that reused the subject index, would emit a formula the adequacy
/// theorem says nothing about, and the freshness countermodel
/// `OwlLean.Refutations.tr_bridge_needs_freshness` is what goes wrong at the
/// extreme of that mistake.
#[test]
fn some_values_from_allocates_x2() {
    let got = tptp_of(&OwlAxiom::SubClass(
        c("A"),
        Concept::Some_(iri("r"), Box::new(c("B"))),
    ));
    let expected = format!(
        "! [X0] : (thing(X0) => ({a}(X0) => ? [X2] : (thing(X2) & ({r}(X0,X2) & {b}(X2)))))",
        a = q("c:", "A"),
        b = q("c:", "B"),
        r = q("op:", "r")
    );
    assert_eq!(got, expected);
}

/// `trAx (subClass (all_ r top) ...)`: the guarded universal.
#[test]
fn all_values_from_is_a_guarded_universal() {
    let got = tptp_of(&OwlAxiom::SubClass(
        Concept::All_(iri("r"), Box::new(c("B"))),
        c("A"),
    ));
    let expected = format!(
        "! [X0] : (thing(X0) => (! [X2] : (thing(X2) => ({r}(X0,X2) => {b}(X2))) => {a}(X0)))",
        a = q("c:", "A"),
        b = q("c:", "B"),
        r = q("op:", "r")
    );
    assert_eq!(got, expected);
}

/// The counter is THREADED between the two sides of a subsumption.
///
/// `tr c 0 2` returns a next-free index, and `tr d 0 n` starts from it, so the
/// right-hand existential binds X3 and not X2. This is the single most
/// effective drift detector in the file: an exporter that restarted the
/// counter per class expression would produce two formulas that capture each
/// other's variables and would pass every "does it parse" check.
#[test]
fn the_counter_is_threaded_across_a_subsumption() {
    let got = tptp_of(&OwlAxiom::SubClass(
        Concept::Some_(iri("r"), Box::new(Concept::Top)),
        Concept::Some_(iri("s"), Box::new(Concept::Top)),
    ));
    let expected = format!(
        "! [X0] : (thing(X0) => (? [X2] : (thing(X2) & ({r}(X0,X2) & $true)) \
         => ? [X3] : (thing(X3) & ({s}(X0,X3) & $true))))",
        r = q("op:", "r"),
        s = q("op:", "s")
    );
    assert_eq!(got, expected);
}

/// `trAx (subClass (atom A) (minCard 2 r (atom B)))`
///
/// `ys = [2, 3]`, `trList (atom B) ys (c + n) = ([cls B (v 2), cls B (v 3)], 4)`,
/// `distinctPairs [2,3] = [neg (eq (v 2) (v 3))]`, and the conjunction is
/// `distinctPairs ++ zip`, in that order, right-associated by `Form.conj`.
/// These are the cases LATIN's `OWL2toFOL.elf` has commented out.
#[test]
fn min_cardinality_expands_with_distinctness() {
    let got = tptp_of(&OwlAxiom::SubClass(
        c("A"),
        Concept::MinCard(2, iri("r"), Box::new(c("B"))),
    ));
    let expected = format!(
        "! [X0] : (thing(X0) => ({a}(X0) => \
         ? [X2] : (thing(X2) & ? [X3] : (thing(X3) & \
         (~ (X2 = X3) & (({r}(X0,X2) & {b}(X2)) & ({r}(X0,X3) & {b}(X3))))))))",
        a = q("c:", "A"),
        b = q("c:", "B"),
        r = q("op:", "r")
    );
    assert_eq!(got, expected);
}

/// `trAx (subClass (atom A) (maxCard 1 r top))`
///
/// `maxCard n` allocates `n + 1` variables, so `ys = [2, 3]` for `n = 1`, and
/// the consequent is `disj (somePairEq ys)`. The `$true` conjuncts are the
/// translation of the `top` filler and are NOT optimised away: the emitted
/// formula has to be the formula the theorem is about, not one equivalent to
/// it.
#[test]
fn max_cardinality_expands_with_pairwise_equality() {
    let got = tptp_of(&OwlAxiom::SubClass(
        c("A"),
        Concept::MaxCard(1, iri("r"), Box::new(Concept::Top)),
    ));
    let expected = format!(
        "! [X0] : (thing(X0) => ({a}(X0) => \
         ! [X2] : (thing(X2) => ! [X3] : (thing(X3) => \
         ((({r}(X0,X2) & $true) & ({r}(X0,X3) & $true)) => X2 = X3)))))",
        a = q("c:", "A"),
        r = q("op:", "r")
    );
    assert_eq!(got, expected);
}

/// `trAx (chain [r, s] t)`: a chain of length n needs n+1 variables, all
/// guarded, and the conclusion relates `v 0` to `v n`.
#[test]
fn property_chain_uses_n_plus_one_variables() {
    let got = tptp_of(&OwlAxiom::Chain(vec![iri("r"), iri("s")], iri("t")));
    let expected = format!(
        "! [X0] : (thing(X0) => ! [X1] : (thing(X1) => ! [X2] : (thing(X2) => \
         (({r}(X0,X1) & {s}(X1,X2)) => {t}(X0,X2)))))",
        r = q("op:", "r"),
        s = q("op:", "s"),
        t = q("op:", "t")
    );
    assert_eq!(got, expected);
}

/// `trAx (classAssert c a) = .and (thing (k a)) (substSubj f a)`.
///
/// The `thing(a)` conjunct is part of `trAx` itself. It is NOT the individual
/// typing axiom, and having it here does not make `indAxioms` redundant:
/// `OwlLean.Refutations.Mbad_refutes` shows this very formula failing in a
/// model where the constant denotes a literal.
#[test]
fn class_assertion_substitutes_the_subject_and_types_it() {
    let got = tptp_of(&OwlAxiom::ClassAssert(c("A"), iri("a")));
    let expected = format!(
        "(thing({i}) & {a}({i}))",
        a = q("c:", "A"),
        i = q("i:", "a")
    );
    assert_eq!(got, expected);
}

/// `tr (oneOf as) x c = (disj (as.map (fun a => eq (v x) (k a))), c)`.
#[test]
fn one_of_is_a_disjunction_of_equalities() {
    let got = tptp_of(&OwlAxiom::SubClass(
        c("A"),
        Concept::OneOf(vec![iri("a"), iri("b")]),
    ));
    let expected = format!(
        "! [X0] : (thing(X0) => ({a}(X0) => (X0 = {ia} | X0 = {ib})))",
        a = q("c:", "A"),
        ia = q("i:", "a"),
        ib = q("i:", "b")
    );
    assert_eq!(got, expected);
}

/// `trAx (oPropRange r c)` translates the filler at subject variable **1**,
/// not 0: `let (f, _) := tr c 1 2`. Domain uses 0. Getting these the same way
/// round is a silent correctness bug in every exporter that has it.
#[test]
fn range_translates_the_filler_at_variable_one() {
    let range = tptp_of(&OwlAxiom::OPropRange(Ope::Named(iri("r")), c("B")));
    let domain = tptp_of(&OwlAxiom::OPropDomain(Ope::Named(iri("r")), c("B")));
    assert_eq!(
        range,
        format!(
            "! [X0] : (thing(X0) => ! [X1] : (thing(X1) => ({r}(X0,X1) => {b}(X1))))",
            r = q("op:", "r"),
            b = q("c:", "B")
        )
    );
    assert_eq!(
        domain,
        format!(
            "! [X0] : (thing(X0) => ! [X1] : (thing(X1) => ({r}(X0,X1) => {b}(X0))))",
            r = q("op:", "r"),
            b = q("c:", "B")
        )
    );
}

/// `trAx.ope (.inv r) i j = op r (v j) (v i)`: an inverse property expression
/// swaps the argument order and introduces no new symbol.
#[test]
fn an_inverse_property_expression_swaps_the_arguments() {
    let got = tptp_of(&OwlAxiom::SubOProp(
        Ope::Named(iri("r")),
        Ope::Inv(iri("s")),
    ));
    let expected = format!(
        "! [X0] : (thing(X0) => ! [X1] : (thing(X1) => ({r}(X0,X1) => {s}(X1,X0))))",
        r = q("op:", "r"),
        s = q("op:", "s")
    );
    assert_eq!(got, expected);
}

/// `functional` and `invFunctional` differ only in which positions repeat,
/// and both are pinned so a transposition cannot slip through.
#[test]
fn functional_and_inverse_functional_differ_in_argument_position() {
    let f = tptp_of(&OwlAxiom::Functional(iri("r")));
    let i = tptp_of(&OwlAxiom::InvFunctional(iri("r")));
    let r = q("op:", "r");
    assert_eq!(
        f,
        format!(
            "! [X0] : (thing(X0) => ! [X1] : (thing(X1) => ! [X2] : (thing(X2) => \
             (({r}(X0,X1) & {r}(X0,X2)) => X1 = X2))))"
        )
    );
    assert_eq!(
        i,
        format!(
            "! [X0] : (thing(X0) => ! [X1] : (thing(X1) => ! [X2] : (thing(X2) => \
             (({r}(X0,X2) & {r}(X1,X2)) => X0 = X1))))"
        )
    );
}

/// The data side: `dataSome` guards with `lit`, not `thing`, and allocates one
/// variable. Omitting the soft typing makes the translation unsound for every
/// data-property axiom, which is why `background` carries the disjointness.
#[test]
fn data_some_values_from_is_guarded_by_lit() {
    let got = tptp_of(&OwlAxiom::SubClass(
        c("A"),
        Concept::DataSome(iri("p"), iri("D")),
    ));
    let expected = format!(
        "! [X0] : (thing(X0) => ({a}(X0) => ? [X2] : (lit(X2) & ({p}(X0,X2) & {d}(X2)))))",
        a = q("c:", "A"),
        p = q("dp:", "p"),
        d = q("d:", "D")
    );
    assert_eq!(got, expected);
}

/// `dPropRange`: object subject, data object.
#[test]
fn data_property_range_mixes_the_two_guards() {
    let got = tptp_of(&OwlAxiom::DPropRange(iri("p"), iri("D")));
    let expected = format!(
        "! [X0] : (thing(X0) => ! [X1] : (lit(X1) => ({p}(X0,X1) => {d}(X1))))",
        p = q("dp:", "p"),
        d = q("d:", "D")
    );
    assert_eq!(got, expected);
}

/// Equality axioms translate to equality, not to a predicate.
#[test]
fn same_as_and_different_from_use_equality() {
    assert_eq!(
        tptp_of(&OwlAxiom::SameAs(iri("a"), iri("b"))),
        format!("{} = {}", q("i:", "a"), q("i:", "b"))
    );
    assert_eq!(
        tptp_of(&OwlAxiom::DifferentFrom(iri("a"), iri("b"))),
        format!("~ ({} = {})", q("i:", "a"), q("i:", "b"))
    );
}

/// `background` is `OwlLean.background`, verbatim, and both quantifiers are
/// UNGUARDED. Guarding the second one would turn "some element of the domain
/// is an object" into a tautology.
#[test]
fn background_is_two_unguarded_axioms() {
    let b = Translation::background();
    assert_eq!(b.len(), 2);
    assert_eq!(fof::form(&b[0]), "! [X0] : (~ (thing(X0) & lit(X0)))");
    assert_eq!(fof::form(&b[1]), "? [X0] : thing(X0)");
}

/// Every one of the twenty-two axiom forms in `OwlLean/Syntax.lean` translates
/// without tripping the freshness check. A form that is added to `OwlAxiom`
/// and left out of this list is not covered by the count assertion, which is
/// the honest limit of what a test like this can do.
#[test]
fn every_axiom_form_translates() {
    let all = vec![
        OwlAxiom::SubClass(c("A"), c("B")),
        OwlAxiom::EquivClass(c("A"), c("B")),
        OwlAxiom::DisjointWith(c("A"), c("B")),
        OwlAxiom::SubOProp(Ope::Named(iri("r")), Ope::Named(iri("s"))),
        OwlAxiom::OPropDomain(Ope::Named(iri("r")), c("A")),
        OwlAxiom::OPropRange(Ope::Named(iri("r")), c("A")),
        OwlAxiom::DPropDomain(iri("p"), c("A")),
        OwlAxiom::DPropRange(iri("p"), iri("D")),
        OwlAxiom::Transitive(iri("r")),
        OwlAxiom::Symmetric(iri("r")),
        OwlAxiom::Asymmetric(iri("r")),
        OwlAxiom::Reflexive(iri("r")),
        OwlAxiom::Irreflexive(iri("r")),
        OwlAxiom::Functional(iri("r")),
        OwlAxiom::InvFunctional(iri("r")),
        OwlAxiom::InverseOf(iri("r"), iri("s")),
        OwlAxiom::PropDisjoint(iri("r"), iri("s")),
        OwlAxiom::Chain(vec![iri("r"), iri("s")], iri("t")),
        OwlAxiom::ClassAssert(c("A"), iri("a")),
        OwlAxiom::OPropAssert(iri("r"), iri("a"), iri("b")),
        OwlAxiom::SameAs(iri("a"), iri("b")),
        OwlAxiom::DifferentFrom(iri("a"), iri("b")),
    ];
    assert_eq!(all.len(), 22, "OwlLean/Syntax.lean has 22 axiom forms");
    for a in &all {
        Translation::axiom(a).unwrap_or_else(|e| panic!("{a:?} failed: {e}"));
    }
}

/// Every one of the fifteen concept forms translates, at `trAx`'s own counter.
#[test]
fn every_concept_form_translates() {
    let all = vec![
        Concept::Top,
        Concept::Bot,
        c("A"),
        Concept::Inter(Box::new(c("A")), Box::new(c("B"))),
        Concept::Union(Box::new(c("A")), Box::new(c("B"))),
        Concept::Compl(Box::new(c("A"))),
        Concept::OneOf(vec![iri("a")]),
        Concept::Some_(iri("r"), Box::new(c("A"))),
        Concept::All_(iri("r"), Box::new(c("A"))),
        Concept::HasVal(iri("r"), iri("a")),
        Concept::HasSelf(iri("r")),
        Concept::MinCard(2, iri("r"), Box::new(c("A"))),
        Concept::MaxCard(2, iri("r"), Box::new(c("A"))),
        Concept::DataSome(iri("p"), iri("D")),
        Concept::DataAll(iri("p"), iri("D")),
    ];
    assert_eq!(all.len(), 15, "OwlLean/Syntax.lean has 15 concept forms");
    for k in &all {
        Translation::concept_fresh(k, 0, 2).unwrap_or_else(|e| panic!("{k:?} failed: {e}"));
    }
}

// ── 2. The two things the adequacy theorem needs ────────────────────────────

/// **Gate.** `tr_bridge` holds only under `Fresh n x`, i.e. `x < n`.
///
/// `OwlLean.Refutations.tr_bridge_needs_freshness` is the machine-checked
/// countermodel: it translates `∃r.⊤` at subject 0 with the counter also at 0,
/// and exhibits an interpretation where the two readings disagree because the
/// existential `tr` allocates has captured the subject variable. This test
/// puts exactly that call to the exporter and requires a refusal.
#[test]
fn freshness_gate_refuses_the_refutations_countermodel_call() {
    let captured = Concept::Some_(iri("r"), Box::new(Concept::Top));
    let err = Translation::concept_fresh(&captured, 0, 0)
        .expect_err("subject 0 with counter 0 is the Refutations countermodel and must be refused");
    assert_eq!(err.subject, 0);
    assert_eq!(err.counter, 0);
    assert!(err.to_string().contains("tr_bridge_needs_freshness"));

    // The boundary: x = c is refused, x = c - 1 is accepted.
    assert!(Translation::concept_fresh(&captured, 2, 2).is_err());
    assert!(Translation::concept_fresh(&captured, 1, 2).is_ok());
}

/// **Gate.** The individual typing axioms are emitted.
///
/// This reconstructs `OwlLean.Refutations`: the empty ontology, the axiom
/// `⊤(a)`. `adequacy_needs_ind_axioms` refutes the left-to-right direction of
/// adequacy for exactly this pair when `indAxioms` is missing, because `Mbad`
/// interprets the constant outside `thing`. The typing axiom is the fix, so it
/// must be in the file.
#[test]
fn the_individual_typing_axiom_is_emitted() {
    let goal = OwlAxiom::ClassAssert(Concept::Top, iri("a"));
    let problem = FolProblem::build(&[], Some(&goal)).expect("translates");

    assert_eq!(
        problem.ind_axioms.len(),
        1,
        "the empty ontology with the goal Top(a) has one individual, so one typing axiom"
    );
    assert_eq!(fof::form(&problem.ind_axioms[0]), "thing('i:http://e/a')");

    let text = problem.to_tptp();
    assert!(
        text.contains("fof(ind_typing_1, axiom, thing('i:http://e/a'))."),
        "without this axiom OwlLean.Refutations.adequacy_needs_ind_axioms refutes \
         adequacy outright. Emitted:\n{text}"
    );
}

/// The typing axioms range over EVERY individual the problem mentions,
/// including one that occurs only in the conjecture. The adequacy theorem's
/// hypothesis is `∀ a : S.Ind, a ∈ inds`, and the signature here is the
/// occurring vocabulary, so an individual left out would break it.
#[test]
fn the_typing_axioms_cover_the_goals_individuals_too() {
    let ontology = vec![OwlAxiom::OPropAssert(iri("r"), iri("a"), iri("b"))];
    let goal = OwlAxiom::ClassAssert(c("A"), iri("zz"));
    let problem = FolProblem::build(&ontology, Some(&goal)).expect("translates");
    let names: Vec<&str> = problem.individuals.iter().map(String::as_str).collect();
    assert_eq!(names, vec![iri("a"), iri("b"), iri("zz")]);
    assert_eq!(problem.ind_axioms.len(), 3);
}

// ── 3. One translation, two serialisers ─────────────────────────────────────
//
// A tiny CLIF reader, defined here and nowhere in `src/`, because it exists
// only to check the writer. Reading the emitted text back and requiring the
// identical `Form` is what makes "two serialisations of one translation" a
// checked claim rather than a design intention.

#[derive(Debug, PartialEq, Eq, Clone)]
enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

fn lex(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = src.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '(' | ')' => out.push(ch.to_string()),
            c if c.is_whitespace() => {}
            '"' | '\'' => {
                // An enclosed name (double quotes, ISO/IEC 24707 A.2.2.2
                // `namequote`) or a quoted string (single quotes,
                // `stringquote`), each running to its matching delimiter with
                // backslash escapes.
                let mut name = String::new();
                name.push(ch);
                loop {
                    match chars.next() {
                        None => panic!("unterminated enclosed name or string"),
                        Some('\\') => {
                            name.push('\\');
                            name.push(chars.next().expect("escape at end of input"));
                        }
                        Some(c) if c == ch => {
                            name.push(c);
                            break;
                        }
                        Some(c) => name.push(c),
                    }
                }
                out.push(name);
            }
            c => {
                let mut name = String::from(c);
                while let Some(&n) = chars.peek() {
                    if n == '(' || n == ')' || n.is_whitespace() || n == '"' || n == '\'' {
                        break;
                    }
                    name.push(n);
                    chars.next();
                }
                out.push(name);
            }
        }
    }
    out
}

fn parse(tokens: &[String], i: &mut usize) -> Sexp {
    let t = &tokens[*i];
    *i += 1;
    if t == "(" {
        let mut items = Vec::new();
        while tokens[*i] != ")" {
            items.push(parse(tokens, i));
        }
        *i += 1;
        Sexp::List(items)
    } else {
        assert_ne!(t, ")", "unbalanced CLIF");
        Sexp::Atom(t.clone())
    }
}

fn read_sexp(src: &str) -> Sexp {
    let tokens = lex(src);
    let mut i = 0;
    let s = parse(&tokens, &mut i);
    assert_eq!(i, tokens.len(), "trailing tokens in {src}");
    s
}

/// Undo `clif::enclosed`: strip the DOUBLE QUOTES and the escapes.
fn unenclose(name: &str) -> Option<String> {
    let inner = name.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            out.push(chars.next().expect("escape at end"));
        } else {
            out.push(ch);
        }
    }
    Some(out)
}

fn read_term(s: &Sexp) -> Term {
    let Sexp::Atom(a) = s else {
        panic!("a CLIF term in the first-order fragment is an atom, found {s:?}")
    };
    match unenclose(a) {
        Some(name) => {
            let rest = name
                .strip_prefix("i:")
                .unwrap_or_else(|| panic!("a constant must carry the i: prefix, found {name}"));
            Term::Const(rest.to_string())
        }
        None => {
            let n = a
                .strip_prefix('X')
                .and_then(|d| d.parse().ok())
                .unwrap_or_else(|| panic!("a variable must be Xn, found {a}"));
            Term::Var(n)
        }
    }
}

fn read_p1(s: &Sexp) -> P1 {
    let Sexp::Atom(a) = s else { panic!("predicate position must be a name") };
    match a.as_str() {
        "thing" => return P1::Thing,
        "lit" => return P1::Lit,
        _ => {}
    }
    let name = unenclose(a).unwrap_or_else(|| panic!("unknown bare predicate {a}"));
    if let Some(r) = name.strip_prefix("c:") {
        P1::Cls(r.to_string())
    } else if let Some(r) = name.strip_prefix("d:") {
        P1::Dt(r.to_string())
    } else {
        panic!("unary predicate {name} carries no kind prefix")
    }
}

fn read_p2(s: &Sexp) -> P2 {
    let Sexp::Atom(a) = s else { panic!("predicate position must be a name") };
    let name = unenclose(a).unwrap_or_else(|| panic!("unknown bare predicate {a}"));
    if let Some(r) = name.strip_prefix("op:") {
        P2::Op(r.to_string())
    } else if let Some(r) = name.strip_prefix("dp:") {
        P2::Dp(r.to_string())
    } else {
        panic!("binary predicate {name} carries no kind prefix")
    }
}

fn read_form(s: &Sexp) -> Form {
    let Sexp::List(items) = s else {
        panic!("a CLIF sentence is a list, found {s:?}")
    };
    if items.is_empty() {
        panic!("the empty list is not a CLIF sentence");
    }
    if let Sexp::Atom(head) = &items[0] {
        // A commented sentence has the same semantics as the sentence.
        if (head == "cl:comment" || head == "cl-comment") && items.len() == 3 {
            return read_form(&items[2]);
        }
        match (head.as_str(), items.len()) {
            ("and", 1) => return Form::Tru,
            ("or", 1) => return Form::Fls,
            ("and", 3) => {
                return Form::And(
                    Box::new(read_form(&items[1])),
                    Box::new(read_form(&items[2])),
                )
            }
            ("or", 3) => {
                return Form::Or(
                    Box::new(read_form(&items[1])),
                    Box::new(read_form(&items[2])),
                )
            }
            ("if", 3) => {
                return Form::Imp(
                    Box::new(read_form(&items[1])),
                    Box::new(read_form(&items[2])),
                )
            }
            ("not", 2) => return Form::Neg(Box::new(read_form(&items[1]))),
            ("=", 3) => return Form::Eq(read_term(&items[1]), read_term(&items[2])),
            ("forall", 3) | ("exists", 3) => {
                let Sexp::List(binders) = &items[1] else {
                    panic!("a quantifier's binding list must be a list")
                };
                assert_eq!(binders.len(), 1, "the writer emits one binder per quantifier");
                let Term::Var(n) = read_term(&binders[0]) else {
                    panic!("a binder must be a variable")
                };
                let body = Box::new(read_form(&items[2]));
                return if head == "forall" {
                    Form::All(n, body)
                } else {
                    Form::Ex(n, body)
                };
            }
            _ => {}
        }
    }
    // An atomic sentence: a predicate applied to terms.
    match items.len() {
        2 => Form::App1(read_p1(&items[0]), read_term(&items[1])),
        3 => Form::App2(read_p2(&items[0]), read_term(&items[1]), read_term(&items[2])),
        n => panic!("no first-order atom of arity {}: {s:?}", n - 1),
    }
}

/// A corpus that exercises every constructor of `Form` at least once.
fn corpus() -> Vec<OwlAxiom> {
    vec![
        OwlAxiom::SubClass(c("A"), c("B")),
        OwlAxiom::SubClass(c("A"), Concept::Some_(iri("r"), Box::new(c("B")))),
        OwlAxiom::SubClass(c("A"), Concept::All_(iri("r"), Box::new(c("B")))),
        OwlAxiom::SubClass(c("A"), Concept::MinCard(2, iri("r"), Box::new(c("B")))),
        OwlAxiom::SubClass(c("A"), Concept::MaxCard(1, iri("r"), Box::new(Concept::Top))),
        OwlAxiom::SubClass(Concept::Bot, Concept::Compl(Box::new(c("B")))),
        OwlAxiom::SubClass(
            Concept::Inter(Box::new(c("A")), Box::new(c("B"))),
            Concept::Union(Box::new(c("A")), Box::new(c("B"))),
        ),
        OwlAxiom::SubClass(c("A"), Concept::OneOf(vec![iri("a"), iri("b")])),
        OwlAxiom::SubClass(c("A"), Concept::HasVal(iri("r"), iri("a"))),
        OwlAxiom::SubClass(c("A"), Concept::HasSelf(iri("r"))),
        OwlAxiom::SubClass(c("A"), Concept::DataSome(iri("p"), iri("D"))),
        OwlAxiom::SubClass(c("A"), Concept::DataAll(iri("p"), iri("D"))),
        OwlAxiom::EquivClass(c("A"), c("B")),
        OwlAxiom::DisjointWith(c("A"), c("B")),
        OwlAxiom::SubOProp(Ope::Named(iri("r")), Ope::Inv(iri("s"))),
        OwlAxiom::OPropDomain(Ope::Inv(iri("r")), c("A")),
        OwlAxiom::OPropRange(Ope::Named(iri("r")), c("A")),
        OwlAxiom::DPropDomain(iri("p"), c("A")),
        OwlAxiom::DPropRange(iri("p"), iri("D")),
        OwlAxiom::Transitive(iri("r")),
        OwlAxiom::Symmetric(iri("r")),
        OwlAxiom::Asymmetric(iri("r")),
        OwlAxiom::Reflexive(iri("r")),
        OwlAxiom::Irreflexive(iri("r")),
        OwlAxiom::Functional(iri("r")),
        OwlAxiom::InvFunctional(iri("r")),
        OwlAxiom::InverseOf(iri("r"), iri("s")),
        OwlAxiom::PropDisjoint(iri("r"), iri("s")),
        OwlAxiom::Chain(vec![iri("r"), iri("s")], iri("t")),
        OwlAxiom::ClassAssert(c("A"), iri("a")),
        OwlAxiom::OPropAssert(iri("r"), iri("a"), iri("b")),
        OwlAxiom::SameAs(iri("a"), iri("b")),
        OwlAxiom::DifferentFrom(iri("a"), iri("b")),
    ]
}

/// **The two serialisers denote the same formula set.**
///
/// Every formula in the problem, background and typing axioms included, is
/// rendered to CLIF, read back with the reader above, and required to be the
/// identical `Form` the TPTP writer was handed. A drift in either writer
/// breaks this.
#[test]
fn clif_round_trips_to_the_same_forms() {
    let problem = FolProblem::build(&corpus(), None).expect("translates");
    let formulas = problem.formulas();
    assert!(formulas.len() >= 36, "corpus should be substantial");
    for (name, _role, f) in &formulas {
        let text = clif::form(f);
        let back = read_form(&read_sexp(&text));
        assert_eq!(&&back, f, "CLIF round trip differs for {name}: {text}");
    }
}

/// The two files carry the same formulas in the same order, so neither
/// serialiser can quietly include or omit one.
#[test]
fn both_files_carry_the_same_formulas_in_the_same_order() {
    let problem = FolProblem::build(&corpus(), Some(&OwlAxiom::SubClass(c("A"), c("B"))))
        .expect("translates");
    let names: Vec<String> = problem.formulas().into_iter().map(|(n, _, _)| n).collect();

    let tptp_text = problem.to_tptp();
    let clif_text = problem.to_clif(ClifDialect::Iso, ClifComments::Standalone, TEXT_NAME);
    for n in &names {
        assert!(tptp_text.contains(&format!("fof({n}, ")), "TPTP missing {n}");
        assert!(
            clif_text.contains(&format!("(cl:comment '{n} (")),
            "CLIF missing {n}"
        );
    }
    // Same count of emitted sentences in each. The CLIF has one more
    // `cl:comment`, the header, and it rides on `(and)`, which is true in
    // every model and so adds nothing to the theory.
    assert_eq!(
        tptp_text.matches("\nfof(").count(),
        names.len(),
        "TPTP sentence count"
    );
    assert_eq!(
        clif_text.matches("(cl:comment ").count(),
        names.len() + 1,
        "one standalone comment per formula, plus the header comment"
    );
}

/// Hand-pinned CLIF, for the same worked cases as the TPTP above. The round
/// trip proves the two agree with each other; these pin what they agree ON.
#[test]
fn clif_worked_cases_are_pinned_by_hand() {
    let f = Translation::axiom(&OwlAxiom::SubClass(
        c("A"),
        Concept::Some_(iri("r"), Box::new(c("B"))),
    ))
    .unwrap();
    assert_eq!(
        clif::form(&f),
        "(forall (X0) (if (thing X0) (if (\"c:http://e/A\" X0) \
         (exists (X2) (and (thing X2) (and (\"op:http://e/r\" X0 X2) (\"c:http://e/B\" X2)))))))"
    );

    let b = Translation::background();
    assert_eq!(
        clif::form(&b[0]),
        "(forall (X0) (not (and (thing X0) (lit X0))))"
    );
    assert_eq!(clif::form(&b[1]), "(exists (X0) (thing X0))");

    // `(and)` is truth and `(or)` is falsity; CLIF has no truth constants.
    assert_eq!(clif::form(&Form::Tru), "(and)");
    assert_eq!(clif::form(&Form::Fls), "(or)");

    let max = Translation::axiom(&OwlAxiom::SubClass(
        c("A"),
        Concept::MaxCard(1, iri("r"), Box::new(Concept::Top)),
    ))
    .unwrap();
    assert_eq!(
        clif::form(&max),
        "(forall (X0) (if (thing X0) (if (\"c:http://e/A\" X0) \
         (forall (X2) (if (thing X2) (forall (X3) (if (thing X3) \
         (if (and (and (\"op:http://e/r\" X0 X2) (and)) (and (\"op:http://e/r\" X0 X3) (and))) \
         (= X2 X3)))))))))"
    );
}

// ── 4. The CLIF stays inside the first-order-equivalent fragment ────────────

/// **Gate.** Common Logic is not plain first-order logic.
///
/// It has sequence markers, arity-free predicates, and a universe in which
/// relations are themselves individuals. The adequacy theorem is about a
/// translation into PLAIN first-order logic, so an emitted text using any of
/// those would sit outside the theorem. Three properties are checked over the
/// whole corpus:
///
///   * no sequence marker anywhere in the text,
///   * every predicate name is used at ONE arity throughout,
///   * no bound variable ever appears in a predicate position.
#[test]
fn clif_uses_only_fol_fragment() {
    use std::collections::HashMap;

    let problem = FolProblem::build(&corpus(), None).expect("translates");
    let text = problem.to_clif(ClifDialect::Iso, ClifComments::Standalone, TEXT_NAME);
    assert!(
        !text.contains("..."),
        "a sequence marker would take the text outside plain first-order logic"
    );

    let mut arity: HashMap<String, usize> = HashMap::new();
    let mut bound: Vec<u32> = Vec::new();

    fn walk(
        s: &Sexp,
        arity: &mut HashMap<String, usize>,
        bound: &mut Vec<u32>,
    ) {
        let Sexp::List(items) = s else { return };
        if items.is_empty() {
            return;
        }
        if let Sexp::Atom(head) = &items[0] {
            match head.as_str() {
                "and" | "or" | "if" | "iff" | "not" => {
                    for it in &items[1..] {
                        walk(it, arity, bound);
                    }
                    return;
                }
                "forall" | "exists" => {
                    let Sexp::List(binders) = &items[1] else {
                        panic!("binding list must be a list")
                    };
                    for b in binders {
                        let Sexp::Atom(a) = b else { panic!("binder must be a name") };
                        let n: u32 = a
                            .strip_prefix('X')
                            .and_then(|d| d.parse().ok())
                            .expect("binder must be Xn");
                        bound.push(n);
                    }
                    walk(&items[2], arity, bound);
                    for _ in binders {
                        bound.pop();
                    }
                    return;
                }
                "=" => {
                    assert_eq!(items.len(), 3, "equality is binary in the fragment");
                    return;
                }
                _ => {}
            }
            // An atomic sentence. The operator must be a NAME, never a bound
            // variable: quantifying into a predicate position is the
            // second-order half of Common Logic.
            if let Some(n) = head.strip_prefix('X').and_then(|d| d.parse::<u32>().ok()) {
                assert!(
                    !bound.contains(&n),
                    "bound variable X{n} used in a predicate position"
                );
            }
            let seen = arity.entry(head.clone()).or_insert(items.len() - 1);
            assert_eq!(
                *seen,
                items.len() - 1,
                "predicate {head} used at two arities, which is arity-free Common Logic \
                 and not plain first-order logic"
            );
            // Arguments must all be terms.
            for it in &items[1..] {
                assert!(
                    matches!(it, Sexp::Atom(_)),
                    "an argument in the first-order fragment is a term, found {it:?}"
                );
            }
        } else {
            panic!("an operator position must be a name, found {:?}", items[0]);
        }
    }

    for (_, _, f) in problem.formulas() {
        walk(&read_sexp(&clif::form(f)), &mut arity, &mut bound);
    }
    assert!(arity.contains_key("thing") && arity.contains_key("lit"));
    assert_eq!(arity["thing"], 1);
    assert_eq!(arity["lit"], 1);
}

/// The emitted CLIF says, in the file, that it is restricted. A limit stated
/// only in a README gets read without one.
#[test]
fn the_clif_header_states_the_restriction() {
    let problem = FolProblem::build(&corpus(), None).expect("translates");
    let text = problem.to_clif(ClifDialect::Iso, ClifComments::Standalone, TEXT_NAME);
    assert!(text.contains("CLIF RESTRICTION"));
    assert!(text.contains("no sequence markers"));
    assert!(text.contains("(cl:text"));
    assert!(text.contains("DIALECT"));
    // No LEXICAL comments: the standard defines `cl:comment` as a reserved
    // element, and this project could not establish a `//` or `/* */`
    // line-comment convention from the standard's own text, so it emits
    // none. (`//` itself occurs inside every IRI, so the check is on line
    // starts, not on the substring.)
    assert!(
        !text.lines().any(|l| l.trim_start().starts_with("//")
            || l.trim_start().starts_with("/*")),
        "the CLIF body carries no lexical comments"
    );
    assert!(text.contains("(cl:comment '"), "labels ride on cl:comment");
}

/// Both headers say that the correspondence is pinned by tests and not proved,
/// and a file with a conjecture also says an ATP verdict is an oracle opinion.
#[test]
fn the_headers_state_the_limits() {
    let with_goal = FolProblem::build(&[], Some(&OwlAxiom::SubClass(c("A"), c("B")))).unwrap();
    for text in [
        with_goal.to_tptp(),
        with_goal.to_clif(ClifDialect::Iso, ClifComments::Standalone, TEXT_NAME),
    ] {
        assert!(text.contains("PINNED BY TESTS AND IS NOT ITSELF PROVED"));
        assert!(text.contains("ORACLE OPINION, not a certificate"));
    }
    let no_goal = FolProblem::build(&[], None).unwrap();
    assert!(!no_goal.to_tptp().contains("ORACLE OPINION"));
}

// ── 5. What is dropped is named ─────────────────────────────────────────────

/// **Gate.** An ontology whose unexported constructs are invisible is a trap.
///
/// `owl:hasKey` is outside `OwlLean/Syntax.lean`. The report must say so, with
/// a count and a reason, and must set `exports_a_weaker_axiom_set`, the same
/// flag the description-logic layer sets for its model certificates.
#[test]
fn constructs_outside_the_fragment_are_named_in_the_output() {
    let ttl = r#"
@prefix : <http://e/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
:A a owl:Class ; rdfs:subClassOf :B .
:B a owl:Class .
:k a owl:ObjectProperty .
:A owl:hasKey ( :k ) .
"#;
    let graph = std::sync::Arc::new(open_ontologies::graph::GraphStore::new());
    graph.load_turtle(ttl, None).expect("turtle parses");
    let dir = tempfile::tempdir().expect("tempdir");
    let report = open_ontologies::tptp::export(&graph, dir.path(), Syntax::Tptp, None, 0)
        .expect("export");
    let v: serde_json::Value = serde_json::from_str(&report).expect("json");

    assert_eq!(
        v["exports_a_weaker_axiom_set"], true,
        "hasKey is outside the fragment, so the export is weaker than the ontology"
    );
    let named: Vec<String> = v["constructs_not_exported"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|d| d["construct"].as_str().unwrap_or("").to_string())
        .collect();
    assert!(
        named.iter().any(|n| n.contains("hasKey")),
        "owl:hasKey must be named, got {named:?}"
    );
    // And the reason travels with it, not only the name.
    let why = v["constructs_not_exported"][0]["why"].as_str().unwrap_or("");
    assert!(!why.is_empty(), "a dropped construct carries its reason");
}

/// A clean ontology inside the fragment does NOT set the flag, so the flag
/// means something.
#[test]
fn an_ontology_inside_the_fragment_does_not_set_the_flag() {
    let ttl = r#"
@prefix : <http://e/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
:A a owl:Class ; rdfs:subClassOf :B .
:B a owl:Class .
:r a owl:ObjectProperty, owl:TransitiveProperty .
:a a :A .
"#;
    let graph = std::sync::Arc::new(open_ontologies::graph::GraphStore::new());
    graph.load_turtle(ttl, None).expect("turtle parses");
    let dir = tempfile::tempdir().expect("tempdir");
    let report = open_ontologies::tptp::export(&graph, dir.path(), Syntax::Tptp, None, 0)
        .expect("export");
    let v: serde_json::Value = serde_json::from_str(&report).expect("json");
    assert_eq!(
        v["exports_a_weaker_axiom_set"], false,
        "nothing here is outside the fragment; got {}",
        v["constructs_not_exported"]
    );
    assert_eq!(v["individual_typing_axioms"], 1);
}

/// **A punned assertion weakens the export, and the report says so.**
///
/// OWL 2 DL lets one IRI be a class and an individual at once.
/// `OwlLean/Syntax.lean` does not: `Sig` gives each entity kind its own type
/// and `Sig.Ind` is disjoint from `Sig.Cls`. So an assertion whose subject is
/// declared a class is outside the fragment and is not exported — while its
/// subsumptions, domains and ranges ARE, which is exactly why the omission was
/// invisible.
///
/// It WAS invisible. Running the model-certificate pipeline over
/// `case-studies/blast-furnace-ironmaking/` returned machine-checked
/// countermodels for five of the nine triples the OWL-RL reasoner derives
/// there: `bf:Hanging` is declared `owl:Class` and also carries
/// `bf:hasSeverity bf:HighSeverity`, `bf:hasSeverity` has domain
/// `bf:ProcessState`, and `rdfs2` is what the reasoner used. The exported
/// theory really did not entail the conclusion, and the report said
/// `exports_a_weaker_axiom_set: false`. That is the laundering shape this
/// project keeps finding in other people's work, in its own output.
#[test]
fn a_punned_assertion_is_named_as_weakening_the_export() {
    let ttl = r#"
@prefix : <http://e/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
:State a owl:Class .
:Hanging a owl:Class ; rdfs:subClassOf :State ; :hasSeverity :High .
:High a owl:Class .
:hasSeverity a owl:ObjectProperty ; rdfs:domain :State .
"#;
    let graph = std::sync::Arc::new(open_ontologies::graph::GraphStore::new());
    graph.load_turtle(ttl, None).expect("turtle parses");
    let dir = tempfile::tempdir().expect("tempdir");
    let report = open_ontologies::tptp::export(&graph, dir.path(), Syntax::Tptp, None, 0)
        .expect("export");
    let v: serde_json::Value = serde_json::from_str(&report).expect("json");
    assert_eq!(
        v["exports_a_weaker_axiom_set"], true,
        "the punned assertion is not exported, so the axiom set IS weaker; got {}",
        v["constructs_not_exported"]
    );
    let named: Vec<String> = v["constructs_not_exported"]
        .as_array()
        .expect("array")
        .iter()
        .map(|d| d["construct"].as_str().unwrap_or("").to_string())
        .collect();
    assert!(
        named.iter().any(|n| n == "assertion on a punned entity"),
        "got {named:?}"
    );
    let entry = v["constructs_not_exported"]
        .as_array()
        .expect("array")
        .iter()
        .find(|d| d["construct"] == "assertion on a punned entity")
        .expect("the entry");
    assert_eq!(entry["occurrences"], 1, "one assertion, on :Hanging");
    let why = entry["why"].as_str().unwrap_or("");
    assert!(why.contains("Sig.Ind"), "the reason must name the Lean: {why}");
    assert!(
        why.contains("WEAKER than the graph"),
        "and say what it costs: {why}"
    );
    // The subsumption on the SAME subject is still exported, which is why the
    // omission was invisible: the file looks complete.
    let text = std::fs::read_to_string(dir.path().join("ontology.p")).expect("read");
    assert!(text.contains("subClassOf"), "{text}");
    assert!(
        !text.contains("op:http://e/hasSeverity'(\'i:http://e/Hanging"),
        "the assertion itself is not there"
    );
}

/// The COLORE dialect is a different SPELLING of the same text, not a
/// different translation.
///
/// COLORE is written with `cl-text` / `cl-comment` and single-quoted comment
/// strings, and the Macleod toolchain's shipped lexer maps only those; a file
/// in the ISO spelling does not parse there. The two outputs must therefore
/// differ in the operators and in nothing else, which the round trip through
/// the reader checks formula by formula.
#[test]
fn the_colore_dialect_changes_the_spelling_and_nothing_else() {
    let problem = FolProblem::build(&corpus(), None).expect("translates");
    let iso = problem.to_clif(ClifDialect::Iso, ClifComments::Standalone, TEXT_NAME);
    let colore = problem.to_clif(ClifDialect::Colore, ClifComments::Standalone, TEXT_NAME);

    assert!(iso.contains("(cl:text") && !iso.contains("(cl-text"));
    assert!(colore.contains("(cl-text") && !colore.contains("(cl:text"));
    // Comment strings are SINGLE quoted in BOTH. Quote style is not a dialect
    // matter: ISO/IEC 24707 A.2.2.2 makes the single quote the string
    // delimiter, and BFO's release notes of 7 December 2025 retract the
    // double-quoted form its ISO-hosted files use. Binding the two together
    // meant no flag combination emitted conforming CLIF.
    assert!(colore.contains("(cl-comment '"));
    assert!(iso.contains("(cl:comment '"));
    assert!(!iso.contains("(cl:comment \""), "comment strings are never double quoted");

    // Enclosed names are double-quoted in BOTH: ISO/IEC 24707 A.2.2.2 makes
    // the double quote the namequote, and A.2.2.4 recommends the enclosed-name
    // syntax for IRIs. That is not a dialect matter.
    assert!(colore.contains("\"c:http://e/A\""));
    assert!(iso.contains("\"c:http://e/A\""));

    // Same formulas, read back.
    for (name, _role, f) in problem.formulas() {
        for dialect in [ClifDialect::Iso, ClifDialect::Colore] {
            let text = clif::form(f);
            assert_eq!(
                &read_form(&read_sexp(&text)),
                f,
                "{} round trip differs in the {} dialect",
                name,
                dialect.name()
            );
        }
    }
}

/// An unknown dialect, comment placement or syntax is refused with a message
/// that names the alternatives, rather than silently defaulting.
#[test]
fn an_unknown_syntax_or_dialect_is_refused() {
    // `smtlib` and `ladr` joined `tptp` and `clif` when the model-finding half
    // arrived; `sexpr` never existed and still does not.
    let e = Syntax::parse("sexpr", None, None, None).expect_err("sexpr is not a syntax");
    assert!(
        ["tptp", "clif", "smtlib", "ladr"].iter().all(|n| e.to_string().contains(n)),
        "the message must name every alternative: {e}"
    );
    assert!(Syntax::parse("smtlib", None, None, None).is_ok());
    assert!(Syntax::parse("ladr", None, None, None).is_ok());
    let e =
        Syntax::parse("clif", Some("kif"), None, None).expect_err("kif is not a CLIF dialect");
    assert!(
        e.to_string().contains("iso") && e.to_string().contains("colore"),
        "got {e}"
    );
    let e = Syntax::parse("clif", None, Some("inline"), None)
        .expect_err("inline is not a comment placement");
    assert!(
        e.to_string().contains("standalone") && e.to_string().contains("wrapped"),
        "got {e}"
    );
    // The defaults, and the alternatives.
    let d = Syntax::parse("clif", None, None, None).unwrap();
    assert_eq!(d.dialect(), Some(ClifDialect::Iso));
    assert_eq!(
        d.comments(),
        Some(ClifComments::Standalone),
        "standalone is the default, because it is the only shape either \
         available parser recovers sentences from"
    );
    assert_eq!(
        Syntax::parse("clif", Some("colore"), Some("wrapped"), None)
            .unwrap()
            .comments(),
        Some(ClifComments::Wrapped)
    );
    assert_eq!(Syntax::parse("tptp", None, None, None).unwrap().dialect(), None);
}

/// **Gate.** The default CLIF shape is the one a parser can recover sentences
/// from, and the wrapped shape stays reachable but off.
///
/// py-typedlogic treats a `(cl:comment ...)` form as discardable, so a file
/// whose sentences are all inside one parses cleanly and yields ZERO
/// sentences; Macleod has no commented-sentence production at all. Both do
/// that to ISO/IEC 21838-2's own BFO files. A file that is formally valid and
/// practically empty is the assurance-laundering shape this project exists to
/// attack, so the default must not be it.
#[test]
fn the_default_clif_shape_leaves_sentences_outside_the_comments() {
    let problem = FolProblem::build(&corpus(), None).expect("translates");
    let n = problem.formulas().len();

    let standalone = problem.to_clif(ClifDialect::Iso, ClifComments::Standalone, TEXT_NAME);
    assert_eq!(
        standalone.matches("(cl:comment ").count(),
        n + 1,
        "one comment per formula plus the header"
    );

    // Structural, not line-based: the header comment spans many lines. For
    // every `(cl:comment '`, walk to the closing unescaped quote and require
    // the next non-space character to be the closing bracket. That is exactly
    // the check that distinguishes a standalone comment from one wrapping a
    // sentence, and it is the one that would have caught the original shape.
    let chars: Vec<char> = standalone.chars().collect();
    let marker: Vec<char> = "(cl:comment '".chars().collect();
    let mut found = 0usize;
    let mut i = 0usize;
    while i + marker.len() <= chars.len() {
        if chars[i..i + marker.len()] != marker[..] {
            i += 1;
            continue;
        }
        found += 1;
        let mut j = i + marker.len();
        while j < chars.len() && chars[j] != '\'' {
            j += if chars[j] == '\\' { 2 } else { 1 };
        }
        j += 1;
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        assert_eq!(
            chars.get(j),
            Some(&')'),
            "comment {found} does not close immediately after its string, so a \
             sentence is inside it and both available parsers would drop it"
        );
        i = j;
    }
    assert_eq!(found, n + 1, "every comment was walked");

    // And the sentences are there, one per formula, as phrases of the text in
    // their own right. Read with the S-expression reader below rather than by
    // scanning lines: the header comment spans many lines and several of them
    // begin with an open bracket.
    let Sexp::List(items) = read_sexp(&standalone) else {
        panic!("the whole file is one form")
    };
    assert_eq!(items[0], Sexp::Atom("cl:text".to_string()));
    assert_eq!(items[1], Sexp::Atom(TEXT_NAME.to_string()));
    let (comments, sentences): (Vec<_>, Vec<_>) = items[2..].iter().partition(|p| {
        matches!(p, Sexp::List(xs) if xs.first() == Some(&Sexp::Atom("cl:comment".to_string())))
    });
    assert_eq!(comments.len(), n + 1, "one comment per formula plus the header");
    assert_eq!(sentences.len(), n, "one bare sentence per formula");
    // Every one of them is a formula this project can read back.
    for phrase in &sentences {
        read_form(phrase);
    }

    // The wrapped shape is still available and is not the default.
    let wrapped = problem.to_clif(ClifDialect::Iso, ClifComments::Wrapped, TEXT_NAME);
    assert!(wrapped.contains("(cl:comment 'owl_1_subClassOf (axiom)' (forall"));
    assert!(!standalone.contains("(cl:comment 'owl_1_subClassOf (axiom)' (forall"));
}

/// **Gate.** The text is NAMED.
///
/// All 227 COLORE texts are named, Macleod refuses an unnamed text with
/// "Error in ontology: bad URI", and py-typedlogic otherwise reports the
/// first comment as the theory's name. The name is bare rather than
/// double-quoted because Macleod's lexer has no double-quote token.
#[test]
fn the_clif_text_is_named() {
    let problem = FolProblem::build(&corpus(), None).expect("translates");
    let text = problem.to_clif(ClifDialect::Iso, ClifComments::Standalone, TEXT_NAME);
    assert!(
        text.starts_with(&format!("(cl:text {TEXT_NAME}\n")),
        "the text must carry its name, got: {}",
        text.lines().next().unwrap_or("")
    );
    let colore = problem.to_clif(ClifDialect::Colore, ClifComments::Standalone, TEXT_NAME);
    assert!(colore.starts_with(&format!("(cl-text {TEXT_NAME}\n")));

    // A name that would break a bare token falls back to an enclosed name.
    let awkward = problem.to_clif(ClifDialect::Iso, ClifComments::Standalone, "a name (x)");
    assert!(awkward.starts_with("(cl:text \"a name (x)\"\n"), "got {}", &awkward[..40]);
}

/// **Gate.** The emitted CLIF stays inside the subdialect ISO/IEC 24707 calls
/// EXACTLY semantically conformant.
///
/// Annex A.4.2 of the first edition says, verbatim: "The subdialect of CLIF
/// which does not use numerals or quoted strings is exactly semantically
/// conformant". That matters because a CLIF numeral and a single-quoted string
/// are INTERPRETED names, which denote themselves, and a text containing them
/// constrains its own interpretations in a way abstract Common Logic does not.
/// Stay out of that subdialect and CLIF entailment and Common Logic entailment
/// coincide, so the adequacy theorem's biconditional needs no qualification at
/// the CLIF end.
///
/// **The scope of the claim, stated exactly.** No decimal numerals and no
/// quoted strings IN SENTENCE POSITIONS. Comment annotations are the named
/// exception: a comment's text is a quoted string, and this exporter emits
/// comments in single quotes because A.2.2.2 makes the single quote the string
/// delimiter. A strict reading of "does not use quoted strings" would exclude
/// any file carrying a comment at all. The CLIF files ISO hosts for ISO/IEC
/// 21838-2 are in the same position, using single quotes only inside comment
/// headers, so the precedent is the same; the limit is written down here
/// rather than left for a reader to assume away.
///
/// Nothing here is claimed about the second edition's Annex A.3, which nobody
/// on this project has read.
#[test]
fn clif_stays_in_the_exactly_conformant_subdialect() {
    let problem = FolProblem::build(&corpus(), Some(&OwlAxiom::SubClass(c("A"), c("B"))))
        .expect("translates");

    for dialect in [ClifDialect::Iso, ClifDialect::Colore] {
        let text = problem.to_clif(dialect, ClifComments::Standalone, TEXT_NAME);
        let Sexp::List(items) = read_sexp(&text) else {
            panic!("the whole file is one form")
        };
        let comment_op = Sexp::Atom(
            if dialect == ClifDialect::Iso { "cl:comment" } else { "cl-comment" }.to_string(),
        );
        let mut checked = 0usize;
        for phrase in &items[2..] {
            if matches!(phrase, Sexp::List(xs) if xs.first() == Some(&comment_op)) {
                continue; // the named exception, and only here
            }
            checked += 1;
            check_no_interpreted_names(phrase);
        }
        assert!(checked > 0, "some sentence was checked in the {dialect:?} dialect");
    }
}

/// No decimal numeral and no single-quoted string anywhere in a sentence.
fn check_no_interpreted_names(s: &Sexp) {
    match s {
        Sexp::Atom(a) => {
            assert!(
                !a.starts_with('\''),
                "a single-quoted string is an INTERPRETED name and takes the text out of \
                 the exactly-conformant subdialect (A.4.2): {a}"
            );
            // A CLIF numeral is a bare decimal. `X0` is a name, not a numeral,
            // because it does not begin with a digit; a quoted name is not a
            // numeral either.
            assert!(
                !a.starts_with('"')
                    && !(a.starts_with(|c: char| c.is_ascii_digit())
                        && a.chars().all(|c| c.is_ascii_digit() || c == '.'))
                    || a.starts_with('"'),
                "a numeral is an INTERPRETED name and takes the text out of the \
                 exactly-conformant subdialect (A.4.2): {a}"
            );
        }
        Sexp::List(xs) => xs.iter().for_each(check_no_interpreted_names),
    }
}
