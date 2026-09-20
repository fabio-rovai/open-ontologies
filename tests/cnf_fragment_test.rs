//! The clausal fragment, and the CNF the exporter can emit for it.
//!
//! The measurement behind this is in `src/tptp.rs`: OWL 2 RL forbids an
//! existential restriction in superclass position, so for the profile this
//! engine certifies, clausification is NNF plus distribution and needs no
//! Skolem function. These tests pin that, and pin the refusal for the profile
//! where it is false.

use open_ontologies::tptp::{clauses_of, cnf_records, fragment, Form, Fragment, P1, Term};

fn c(n: &str) -> P1 {
    P1::Cls(n.to_string())
}

fn atom(n: &str, v: u32) -> Form {
    Form::App1(c(n), Term::Var(v))
}

/// `∀X. A(X) ⇒ B(X)`, the shape almost every RL axiom exports as.
fn subclass() -> Form {
    Form::All(0, Box::new(Form::Imp(Box::new(atom("A", 0)), Box::new(atom("B", 0)))))
}

#[test]
fn an_implication_becomes_one_clause() {
    let cs = clauses_of(&subclass()).expect("no skolem needed");
    assert_eq!(cs.len(), 1, "{cs:?}");
    assert_eq!(cs[0].len(), 2, "¬A ∨ B: {:?}", cs[0]);
}

#[test]
fn a_conjunction_in_the_head_becomes_two_clauses() {
    // ∀X. A(X) ⇒ (B(X) ∧ C(X))  is  (¬A ∨ B) ∧ (¬A ∨ C).
    let f = Form::All(
        0,
        Box::new(Form::Imp(
            Box::new(atom("A", 0)),
            Box::new(Form::And(Box::new(atom("B", 0)), Box::new(atom("C", 0))))),
        ),
    );
    let cs = clauses_of(&f).expect("no skolem needed");
    assert_eq!(cs.len(), 2, "{cs:?}");
}

#[test]
fn a_tautology_is_dropped_rather_than_emitted() {
    // ∀X. A(X) ⇒ A(X)  is  ¬A ∨ A, which says nothing.
    let f = Form::All(0, Box::new(Form::Imp(Box::new(atom("A", 0)), Box::new(atom("A", 0)))));
    assert_eq!(clauses_of(&f).unwrap().len(), 0);
}

#[test]
fn a_superclass_existential_is_refused_and_named() {
    // ∀X. A(X) ⇒ ∃Y. B(Y). This is `someValuesFrom` in superclass position,
    // it is outside OWL 2 RL, and it needs a Skolem FUNCTION of X.
    let f = Form::All(
        0,
        Box::new(Form::Imp(
            Box::new(atom("A", 0)),
            Box::new(Form::Ex(1, Box::new(atom("B", 1)))),
        )),
    );
    assert!(clauses_of(&f).is_none(), "an existential head needs Skolemisation");
    match fragment(&[("owl_2_subClassOf".into(), f)]) {
        Fragment::NeedsSkolem(names) => assert_eq!(names, vec!["owl_2_subClassOf".to_string()]),
        other => panic!("expected a refusal naming the axiom, got {other:?}"),
    }
}

#[test]
fn an_existential_in_the_antecedent_is_fine() {
    // ∀X. (∃Y. B(Y)) ⇒ A(X). Moving the negation inward turns that existential
    // into a UNIVERSAL, so nothing needs Skolemising. Refusing it would refuse
    // a formula that is genuinely clausal.
    let f = Form::All(
        0,
        Box::new(Form::Imp(
            Box::new(Form::Ex(1, Box::new(atom("B", 1)))),
            Box::new(atom("A", 0)),
        )),
    );
    assert!(clauses_of(&f).is_some(), "a negative existential is a universal");
}

#[test]
fn a_doubly_negated_existential_in_the_antecedent_still_needs_skolem() {
    // ∀X. (¬∃Y. B(Y)) ⇒ A(X) is (∃Y. B(Y)) ∨ A(X): the existential comes out
    // POSITIVE and does need a Skolem function. Easy to get backwards, so it
    // is pinned rather than reasoned about each time.
    let f = Form::All(
        0,
        Box::new(Form::Imp(
            Box::new(Form::Neg(Box::new(Form::Ex(1, Box::new(atom("B", 1)))))),
            Box::new(atom("A", 0)),
        )),
    );
    assert!(clauses_of(&f).is_none());
}

#[test]
fn a_clausal_problem_renders_as_cnf_records() {
    let out = cnf_records(&[("ax1".into(), "axiom".into(), subclass())]).expect("clausal");
    assert!(out.starts_with("cnf(ax1, axiom, ("), "{out}");
    assert!(out.contains(" | "), "a two-literal clause: {out}");
    assert!(!out.contains('!'), "cnf records carry no quantifier prefix: {out}");
}

#[test]
fn the_domain_axiom_is_the_one_existential_that_is_not_a_restriction() {
    // `∃X. thing(X)`, the non-empty-domain axiom, is a CLOSED existential. It
    // needs one fixed Skolem CONSTANT, not a function of anything, and it is
    // the same formula in every export. It is still refused here, because
    // this function does no Skolemisation at all: what to do about it is the
    // exporter's decision and is recorded there, not hidden in a rewrite.
    let f = Form::Ex(0, Box::new(Form::App1(P1::Thing, Term::Var(0))));
    assert!(clauses_of(&f).is_none());
}

// ── to_cnf on a whole problem ───────────────────────────────────────────────

use open_ontologies::tptp::FolProblem;

fn problem(axioms: Vec<Form>, conjecture: Option<Form>) -> FolProblem {
    FolProblem {
        background: open_ontologies::tptp::Translation::background(),
        ind_axioms: vec![],
        axioms: axioms.into_iter().enumerate().map(|(i, f)| (format!("a{i}"), f)).collect(),
        conjecture: conjecture.map(|f| ("goal".to_string(), f)),
        individuals: Default::default(),
    }
}

#[test]
fn to_cnf_witnesses_the_domain_once_and_negates_the_goal_once() {
    let p = problem(vec![subclass()], Some(atom("A", 0)));
    let out = p.to_cnf().expect("clausal");
    // background_2 was `∃X. thing(X)`; it must come out as one ground fact.
    assert!(
        out.contains(&format!("thing('i:{}')", FolProblem::DOMAIN_WITNESS)),
        "the domain axiom must become a single witness fact:\n{out}"
    );
    assert!(!out.contains('?'), "no existential may survive into CNF:\n{out}");
    assert!(!out.contains('!'), "no universal prefix in cnf records:\n{out}");
    // The conjecture is negated ONCE and its role says so.
    assert_eq!(out.matches("negated_conjecture").count(), 1, "{out}");
    assert!(out.contains("cnf(goal_goal, negated_conjecture, (~ "), "{out}");
    // The exact role token, comma-delimited: `negated_conjecture, (` contains
    // the substring `conjecture, (`, so a bare substring test would pass for
    // the wrong reason.
    assert!(!out.contains(", conjecture, ("), "a bare `conjecture` role must not appear:\n{out}");
}

#[test]
fn to_cnf_refuses_outside_the_fragment_and_names_the_axiom() {
    // A ⊑ ∃r.B in superclass position: outside OWL 2 RL, needs a Skolem function.
    let sv = Form::All(
        0,
        Box::new(Form::Imp(
            Box::new(atom("A", 0)),
            Box::new(Form::Ex(1, Box::new(atom("B", 1)))),
        )),
    );
    let p = problem(vec![subclass(), sv], None);
    let err = p.to_cnf().expect_err("a superclass existential must be refused");
    let msg = err.to_string();
    assert!(msg.contains("owl_2_a1"), "the refusal must name the axiom: {msg}");
    assert!(msg.contains("1 of "), "and count it: {msg}");
    assert!(msg.contains("tptp"), "and point at the format that still works: {msg}");
}

// ── Skolem constants, and only constants ────────────────────────────────────

#[test]
fn a_negated_universal_goal_is_a_closed_existential_and_gets_a_constant() {
    // Goal `∀X. A(X) ⇒ B(X)`. Negated it is `∃X. A(X) ∧ ¬B(X)`, which depends
    // on nothing, so ONE fresh constant witnesses it. Vampire's own FOF proof
    // of such a goal carries exactly one `skolemize` step, and this is it.
    let p = problem(vec![subclass()], Some(subclass()));
    let out = p.to_cnf().expect("a closed existential needs only a constant");
    assert!(out.contains("$sk_goal_goal_X0"), "the witness is named after its formula:\n{out}");
    assert!(!out.contains('?'), "{out}");
    // `A(c) ∧ ¬B(c)` is TWO unit clauses, both negated_conjecture.
    assert_eq!(out.matches("negated_conjecture").count(), 2, "{out}");
}

#[test]
fn an_existential_under_a_universal_still_needs_a_function_and_is_refused() {
    // Unchanged by the constants rule: `∀X. A(X) ⇒ ∃Y. r(X,Y)` needs f(X).
    let sv = Form::All(
        0,
        Box::new(Form::Imp(
            Box::new(atom("A", 0)),
            Box::new(Form::Ex(1, Box::new(atom("B", 1)))),
        )),
    );
    let err = problem(vec![sv], None).to_cnf().expect_err("needs a Skolem function");
    assert!(err.to_string().contains("Skolem FUNCTION"), "{err}");
    assert!(err.to_string().contains("under a universal"), "{err}");
}
