// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Reading a solver's model back, and every way of failing to.
//!
//! Two printed formats, one structure. Z3 prints `define-fun` bodies that have
//! to be EVALUATED; Mace4 prints dense integer tables that have to be INDEXED.
//! The point of the shape of `src/fol_model.rs` is that a third finder costs a
//! third `parse_model` and nothing else, so these tests exercise the two front
//! ends separately and the shared half once.
//!
//! # Every solver string below was MEASURED
//!
//! Z3 4.16.0 and LADR 2009-11A on this machine, copied out of a real run and
//! then edited only where the test says it is a forgery. A hand-invented
//! format would test the parser against a format nobody emits.
//!
//! # G3: every way of lying, each shown failing on its own
//!
//! A gate that cannot fail is decoration, so each of these has a positive case
//! beside it that the same code path accepts:
//!
//!   * a model omitting a symbol the problem uses (`Uninterpreted`);
//!   * a model whose carrier is empty (`EmptyDomain`);
//!   * a model interpreting a predicate at an arity the problem does not use
//!     (`Unsupported`, through the `define-fun` sort check on the Z3 side and
//!     through the `relation(p(_,_))` arity on the Mace4 side);
//!   * a table whose width is not the declared carrier (`BadWidth`);
//!   * a constant outside the carrier (`OutOfRange`);
//!   * a body using a construct the evaluator does not know (`Unsupported`,
//!     naming the construct rather than approximating it);
//!   * no model at all where one was expected (`NoModel`).
//!
//! The fifth forgery of G3, a model built for a DIFFERENT problem, is not an
//! ingestion failure at all: it is caught by the digest inside the verified
//! checker, and `lean_fol_model_test.rs` shows that one failing.
//!
//! # Why a bug here cannot produce a false certificate
//!
//! The checker re-evaluates the ORIGINAL formulas against whatever structure
//! comes out of this module, so a misread table yields a structure that fails
//! `Fol.check` and the pipeline reports a stop-the-line disagreement. The
//! failure mode of a parser bug here is a false alarm, never a false clean.

use open_ontologies::fol_model::{FiniteModel, IngestError, mace4, z3};
use open_ontologies::tptp::{FolProblem, Form, P1, P2, Term, Vocabulary, ladr};

fn vocab(unary: &[&str], binary: &[&str], consts: &[&str]) -> Vocabulary {
    Vocabulary {
        unary: unary.iter().map(|s| s.to_string()).collect(),
        binary: binary.iter().map(|s| s.to_string()).collect(),
        consts: consts.iter().map(|s| s.to_string()).collect(),
    }
}

/// Z3 4.16.0, `(get-model)` over an enumeration carrier of 2, verbatim from a
/// run of the worked example in `lean/Fol/Syntax.lean`. Four body shapes
/// appear in it and all four are ones Z3 really emits: a bare `true`, a bare
/// `false`, an `ite` over an `=`, and a nullary `define-fun` returning an
/// element.
const Z3_MODEL: &str = r#"sat
(
  (define-fun |i:a| () U
    e0)
  (define-fun |c:Person| ((x!0 U)) Bool
    true)
  (define-fun |c:Company| ((x!0 U)) Bool
    (ite (= x!0 e1) true
      false))
  (define-fun lit ((x!0 U)) Bool
    false)
  (define-fun |op:worksFor| ((x!0 U) (x!1 U)) Bool
    true)
  (define-fun thing ((x!0 U)) Bool
    true)
)"#;

fn full_vocab() -> Vocabulary {
    vocab(
        &["thing", "lit", "c:Person", "c:Company"],
        &["op:worksFor"],
        &["i:a"],
    )
}

// ── Z3: the positive case ───────────────────────────────────────────────────

#[test]
fn a_z3_model_is_read_as_the_structure_it_describes() {
    let m = z3::parse_model(Z3_MODEL, &full_vocab(), 2).expect("a readable model");
    assert_eq!(m.domain, 2);
    assert_eq!(m.source, "z3");
    // `true` at every element.
    assert_eq!(m.unary["thing"], vec![true, true]);
    assert_eq!(m.unary["lit"], vec![false, false]);
    // `(ite (= x!0 e1) true false)` is the singleton {1}, EVALUATED and not
    // pattern-matched: this is the one body shape a naive reader gets wrong.
    assert_eq!(m.unary["c:Company"], vec![false, true]);
    assert_eq!(m.binary["op:worksFor"], vec![vec![true, true], vec![true, true]]);
    assert_eq!(m.consts["i:a"], 0);
    assert!(m.dropped.is_empty());
    m.self_check("z3").expect("well shaped");
    m.covers(&full_vocab(), "z3").expect("complete");
}

/// The bodies Z3 actually emits on larger models: `and`, `or`, `not`, `=>`,
/// `distinct`, `xor` and `let`. Measured shapes, one per case, evaluated
/// pointwise over a carrier of 4.
#[test]
fn the_evaluator_handles_the_connectives_z3_emits() {
    let model = r#"(
  (define-fun p ((x!0 U)) Bool
    (and (not (= x!0 e1)) (not (= x!0 e3))))
  (define-fun q ((x!0 U)) Bool
    (or (= x!0 e0) (= x!0 e2)))
  (define-fun s ((x!0 U)) Bool
    (=> (= x!0 e0) false))
  (define-fun t ((x!0 U)) Bool
    (distinct x!0 e0 ))
  (define-fun u ((x!0 U)) Bool
    (xor (= x!0 e0) (= x!0 e1)))
  (define-fun w ((x!0 U)) Bool
    (let ((a!1 (= x!0 e2))) (not a!1)))
  (define-fun r ((x!0 U) (x!1 U)) Bool
    (or (and (= x!0 e1) (= x!1 e0)) (and (not (= x!0 e1)) (= x!1 e1))))
)"#;
    let v = vocab(&["p", "q", "s", "t", "u", "w"], &["r"], &[]);
    let m = z3::parse_model(model, &v, 4).expect("a readable model");
    assert_eq!(m.unary["p"], vec![true, false, true, false]);
    assert_eq!(m.unary["q"], vec![true, false, true, false]);
    assert_eq!(m.unary["s"], vec![false, true, true, true]);
    assert_eq!(m.unary["t"], vec![false, true, true, true]);
    assert_eq!(m.unary["u"], vec![true, true, false, false]);
    assert_eq!(m.unary["w"], vec![true, true, false, true]);
    // r(1,0) and r(i,1) for i != 1.
    assert_eq!(
        m.binary["r"],
        vec![
            vec![false, true, false, false],
            vec![true, false, false, false],
            vec![false, true, false, false],
            vec![false, true, false, false],
        ]
    );
}

/// Symbols the translation never emitted are the REDUCT and are reported
/// rather than hidden. Taking one needs no lemma, because the checker
/// re-evaluates formulas that cannot mention them.
#[test]
fn symbols_outside_the_problem_are_dropped_and_named() {
    let model = r#"(
  (define-fun thing ((x!0 U)) Bool true)
  (define-fun |k!17| ((x!0 U)) Bool false)
  (define-fun |f!aux| () U e0)
)"#;
    let m = z3::parse_model(model, &vocab(&["thing"], &[], &[]), 1).expect("readable");
    assert_eq!(m.dropped, vec!["k!17".to_string(), "f!aux".to_string()]);
    assert_eq!(m.unary.len(), 1);
}

// ── Z3: the forgeries ───────────────────────────────────────────────────────

/// G3.4, at the ingestion boundary. A symbol the problem uses that the solver
/// left uninterpreted is an ERROR here and not a default.
///
/// The checker would catch it one step later as an attribution failure, and
/// `FinModel`'s fields are total so soundness would survive it either way.
/// What would not survive is the sentence "the solver's model was checked",
/// because the thing checked would be the solver's model completed with
/// defaults. It is caught here so the message can name the solver.
#[test]
fn a_z3_model_missing_a_symbol_is_rejected() {
    let e = z3::parse_model(Z3_MODEL, &vocab(&["thing", "c:Missing"], &[], &[]), 2)
        .expect_err("must reject");
    match &e {
        IngestError::Uninterpreted { solver, symbol, arity } => {
            assert_eq!(*solver, "z3");
            assert_eq!(symbol, "c:Missing");
            assert_eq!(*arity, "unary");
        }
        other => panic!("wrong arm: {other:?}"),
    }
    assert!(e.to_string().contains("would DEFAULT"), "{e}");
    // The same model with the symbol removed from the problem is accepted, so
    // the gate is not a constant.
    assert!(z3::parse_model(Z3_MODEL, &vocab(&["thing"], &[], &[]), 2).is_ok());
}

/// G3.2 at the ingestion boundary: an empty carrier, refused before anything
/// is read. `Fol.FinModel` is defined only at `n+1`, so the empty carrier is
/// unrepresentable downstream and this is the file-level half of that.
#[test]
fn an_empty_carrier_is_rejected() {
    let e = z3::parse_model(Z3_MODEL, &full_vocab(), 0).expect_err("must reject");
    assert!(matches!(e, IngestError::EmptyDomain { solver: "z3" }), "{e:?}");
    // A carrier of one is fine, so the gate is about the EMPTY carrier and not
    // about small ones. A model for a carrier of one, measured from a run of
    // the shipped ontology fixture at `--max-domain 1`.
    let one = r#"( (define-fun thing ((x!0 U)) Bool true) (define-fun |i:a| () U e0) )"#;
    let m = z3::parse_model(one, &vocab(&["thing"], &[], &["i:a"]), 1).expect("readable");
    assert_eq!(m.domain, 1);
    assert_eq!(m.unary["thing"], vec![true]);
}

/// Reading a carrier-2 model at a carrier of 1 FAILS rather than truncating.
///
/// `Z3_MODEL`'s body for `c:Company` names the element `e1`, which does not
/// exist at domain 1. Silently treating an unknown `eN` as false would build a
/// structure the solver never described, so the evaluator refuses by name.
#[test]
fn a_model_read_at_the_wrong_carrier_is_rejected() {
    let e = z3::parse_model(Z3_MODEL, &full_vocab(), 1).expect_err("must reject");
    match &e {
        IngestError::Unsupported { construct, .. } => assert_eq!(construct, "e1"),
        other => panic!("wrong arm: {other:?}"),
    }
}

/// G3.3 at the ingestion boundary: a predicate interpreted at an arity the
/// problem does not use.
///
/// The first version of this parser took the `define-fun`'s RETURN SORT for
/// its body, and every ingestion failed with `z3's model uses "Bool"`. The
/// stop-the-line block caught it on the first real run; the sort check is what
/// turns that into a message naming the symbol.
#[test]
fn a_z3_symbol_at_the_wrong_arity_is_rejected() {
    // `thing` supplied as a BINARY predicate while the problem uses it unary.
    let model = r#"(
  (define-fun thing ((x!0 U) (x!1 U)) Bool true)
)"#;
    let e = z3::parse_model(model, &vocab(&["thing"], &[], &[]), 2).expect_err("must reject");
    match &e {
        IngestError::Unsupported { solver, construct, context } => {
            assert_eq!(*solver, "z3");
            assert!(construct.contains("2 argument(s)"), "{construct}");
            assert!(context.contains("thing"), "{context}");
        }
        other => panic!("wrong arm: {other:?}"),
    }
    // A constant whose define-fun returns Bool rather than U.
    let model2 = r#"( (define-fun |i:a| () Bool true) )"#;
    assert!(z3::parse_model(model2, &vocab(&[], &[], &["i:a"]), 2).is_err());
    // Correct arities are accepted.
    let good = r#"( (define-fun thing ((x!0 U)) Bool true) (define-fun |i:a| () U e1) )"#;
    let m = z3::parse_model(good, &vocab(&["thing"], &[], &["i:a"]), 2).expect("readable");
    assert_eq!(m.consts["i:a"], 1);
}

/// A construct the evaluator does not know is NAMED, never approximated. An
/// approximated structure is a different structure, and certifying it would
/// say something true about the wrong object.
#[test]
fn an_unknown_construct_is_named_rather_than_guessed() {
    let model = r#"( (define-fun p ((x!0 U)) Bool (bvult x!0 e1)) )"#;
    let e = z3::parse_model(model, &vocab(&["p"], &[], &[]), 2).expect_err("must reject");
    match &e {
        IngestError::Unsupported { construct, .. } => assert_eq!(construct, "bvult"),
        other => panic!("wrong arm: {other:?}"),
    }
    assert!(e.to_string().contains("DIFFERENT structure"), "{e}");
}

/// A constant naming an element outside the declared carrier.
#[test]
fn a_z3_constant_outside_the_carrier_is_rejected() {
    let model = r#"( (define-fun |i:a| () U e5) )"#;
    // `e5` is not a carrier element at domain 2, so it is not even an element
    // name: the evaluator has nothing to map it to and says so.
    let e = z3::parse_model(model, &vocab(&[], &[], &["i:a"]), 2).expect_err("must reject");
    assert!(matches!(e, IngestError::Unsupported { .. }), "{e:?}");
    // At a carrier that contains it, the same file reads.
    let m = z3::parse_model(model, &vocab(&[], &[], &["i:a"]), 6).expect("readable");
    assert_eq!(m.consts["i:a"], 5);
}

/// No model where one was expected.
#[test]
fn a_z3_run_with_no_model_is_rejected() {
    let e = z3::parse_model("unsat\n", &full_vocab(), 2).expect_err("must reject");
    assert!(matches!(e, IngestError::NoModel { solver: "z3", .. }), "{e:?}");
    let e2 = z3::parse_model("(error \"line 3: unknown sort\")", &full_vocab(), 2)
        .expect_err("must reject");
    assert!(matches!(e2, IngestError::NoModel { .. }), "{e2:?}");
}

/// The S-expression reader itself, on the three things Z3's output contains
/// that a naive splitter gets wrong: `|quoted symbols|` holding slashes and
/// colons, `;;` comment lines, and nesting.
#[test]
fn the_sexp_reader_survives_z3s_own_punctuation() {
    let text = ";; universe for U:\n;;   U!val!0\n( (define-fun |c:http://e/A#x| ((x!0 U)) Bool true) )";
    let m = z3::parse_model(text, &vocab(&["c:http://e/A#x"], &[], &[]), 1).expect("readable");
    assert_eq!(m.unary["c:http://e/A#x"], vec![true]);
    // An unclosed bracket is a syntax error and not a partial read.
    assert!(matches!(
        z3::parse_sexps("(define-fun p ("),
        Err(IngestError::Syntax { .. })
    ));
    assert!(matches!(z3::parse_sexps(")"), Err(IngestError::Syntax { .. })));
}

// ── Mace4: the positive case ────────────────────────────────────────────────

/// LADR 2009-11A, verbatim. `f1` is a Skolem function and `c2` a Skolem
/// constant Mace4 chose itself; both are outside the mangling table and are
/// the reduct.
const MACE4_MODEL: &str = r#"============================== MODEL =================================

interpretation( 2, [number=1, seconds=0], [

        function(c0, [ 0 ]),

        function(c2, [ 1 ]),

        function(f1(_), [ 1, 0 ]),

        relation(p0(_), [ 1, 0 ]),

        relation(p1(_), [ 0, 0 ]),

        relation(r0(_,_), [
			   0, 1,
			   0, 0 ])
]).

============================== end of model ==========================
"#;

/// A problem whose mangling is p0 = `c:A`, p1 = `lit`, r0 = `op:r`,
/// c0 = `i:a`, built through the real `SymbolTable::build` so the test uses
/// the mangling the writer would have used.
fn mace4_table() -> ladr::SymbolTable {
    let p = FolProblem {
        background: vec![],
        ind_axioms: vec![],
        axioms: vec![
            ("x".into(), Form::App1(P1::Cls("A".into()), Term::Var(0))),
            ("x".into(), Form::App1(P1::Lit, Term::Var(0))),
            (
                "x".into(),
                Form::App2(P2::Op("r".into()), Term::Var(0), Term::Const("a".into())),
            ),
        ],
        conjecture: None,
        individuals: Default::default(),
    };
    ladr::SymbolTable::build(&p).expect("no variable letters")
}

#[test]
fn a_mace4_model_is_read_as_the_structure_it_describes() {
    let t = mace4_table();
    assert_eq!(t.unary.get("c:A").map(String::as_str), Some("p0"));
    assert_eq!(t.unary.get("lit").map(String::as_str), Some("p1"));
    assert_eq!(t.binary.get("op:r").map(String::as_str), Some("r0"));
    assert_eq!(t.consts.get("i:a").map(String::as_str), Some("c0"));

    let m = mace4::parse_model(MACE4_MODEL, &t).expect("a readable model");
    assert_eq!(m.domain, 2);
    assert_eq!(m.source, "mace4");
    assert_eq!(m.unary["c:A"], vec![true, false]);
    assert_eq!(m.unary["lit"], vec![false, false]);
    // ROW MAJOR: the table [0,1,0,0] is r(0,1) alone.
    assert_eq!(m.binary["op:r"], vec![vec![false, true], vec![false, false]]);
    assert_eq!(m.consts["i:a"], 0);
    // The Skolem names are the reduct and are reported.
    assert_eq!(m.dropped, vec!["c2".to_string(), "f1".to_string()]);
    m.self_check("mace4").expect("well shaped");
}

// ── Mace4: the forgeries ────────────────────────────────────────────────────

/// THE LADR VARIABLE-LETTER TRAP, caught mechanically.
///
/// A mangled constant that LADR read as a variable simply DISAPPEARS from the
/// model: there is no `function(w0, …)` block for it, because there is no such
/// symbol in the problem Mace4 solved. Measured on LADR 2009-11A, input
/// `p0(w0). -p0(k0).` is echoed as `p0(x).` and the search reports
/// `exit (exhausted)` with no error at all. The guard is that every symbol the
/// TABLE issued must come back interpreted, so the disappearance is an error
/// naming the symbol rather than a silent no-model.
#[test]
fn a_constant_missing_from_a_mace4_model_is_rejected() {
    let t = mace4_table();
    let without_c0 = MACE4_MODEL.replace("function(c0, [ 0 ]),", "");
    let m = mace4::parse_model(&without_c0, &t).expect("the block still parses");
    let v = vocab(&["c:A", "lit"], &["op:r"], &["i:a"]);
    let e = m.covers(&v, "mace4").expect_err("must reject");
    match &e {
        IngestError::Uninterpreted { solver, symbol, arity } => {
            assert_eq!(*solver, "mace4");
            assert_eq!(symbol, "i:a");
            assert_eq!(*arity, "constant");
        }
        other => panic!("wrong arm: {other:?}"),
    }
    // With the block present the same check passes, so the gate is not a
    // constant.
    let whole = mace4::parse_model(MACE4_MODEL, &t).expect("readable");
    assert!(whole.covers(&v, "mace4").is_ok());
}

/// A relation table whose length is not the declared carrier.
#[test]
fn a_mace4_table_of_the_wrong_width_is_rejected() {
    let t = mace4_table();
    let forged = MACE4_MODEL.replace("relation(p0(_), [ 1, 0 ])", "relation(p0(_), [ 1, 0, 1 ])");
    let e = mace4::parse_model(&forged, &t).expect_err("must reject");
    match &e {
        IngestError::BadWidth { solver, symbol, want, got } => {
            assert_eq!(*solver, "mace4");
            assert_eq!(symbol, "c:A");
            assert_eq!((*want, *got), (2, 3));
        }
        other => panic!("wrong arm: {other:?}"),
    }
    // A binary table of the wrong area, separately.
    let forged2 = MACE4_MODEL.replace("0, 1,\n\t\t\t   0, 0 ]", "0, 1, 0 ]");
    assert!(mace4::parse_model(&forged2, &t).is_err());
}

/// A constant pointing outside the carrier.
#[test]
fn a_mace4_constant_outside_the_carrier_is_rejected() {
    let t = mace4_table();
    let forged = MACE4_MODEL.replace("function(c0, [ 0 ])", "function(c0, [ 5 ])");
    let e = mace4::parse_model(&forged, &t).expect_err("must reject");
    match &e {
        IngestError::OutOfRange { solver, symbol, index, domain } => {
            assert_eq!(*solver, "mace4");
            assert_eq!(symbol, "i:a");
            assert_eq!((*index, *domain), (5, 2));
        }
        other => panic!("wrong arm: {other:?}"),
    }
}

/// G3.3, Mace4 half: a symbol the TABLE issued, coming back at an arity the
/// fragment has no slot for. That is a defect in the mangler rather than a
/// reduct, so it is named rather than dropped.
#[test]
fn a_mace4_symbol_at_the_wrong_arity_is_rejected() {
    let t = mace4_table();
    let forged = MACE4_MODEL.replace(
        "relation(p0(_), [ 1, 0 ])",
        "relation(p0(_,_), [ 1, 0, 0, 1 ])",
    );
    let e = mace4::parse_model(&forged, &t).expect_err("must reject");
    match &e {
        IngestError::Unsupported { solver, construct, context } => {
            assert_eq!(*solver, "mace4");
            assert!(construct.contains("relation of arity 2"), "{construct}");
            assert!(context.contains("c:A"), "{context}");
            assert!(context.contains("unary symbol"), "{context}");
        }
        other => panic!("wrong arm: {other:?}"),
    }
    // A constant coming back as a unary FUNCTION is the same failure, and it
    // is the shape a Skolem function would take if the mangler had issued its
    // name. Mace4's own `f1(_)` is dropped instead, because the table never
    // issued `f1`.
    let forged2 = MACE4_MODEL.replace("function(c0, [ 0 ])", "function(c0(_), [ 0, 1 ])");
    assert!(mace4::parse_model(&forged2, &t).is_err());
    // And the table is what decides that, so the unforged file still reads.
    assert!(mace4::parse_model(MACE4_MODEL, &t).is_ok());
}

/// An exhausted or failed Mace4 run prints no interpretation block at all.
#[test]
fn a_mace4_run_with_no_model_is_rejected() {
    let text = "------ process 123 exit (exhausted) ------\n\nExiting with failure.\n";
    let e = mace4::parse_model(text, &mace4_table()).expect_err("must reject");
    match &e {
        IngestError::NoModel { solver, saw } => {
            assert_eq!(*solver, "mace4");
            assert!(saw.contains("exhausted"), "{saw}");
        }
        other => panic!("wrong arm: {other:?}"),
    }
}

// ── The shared half ─────────────────────────────────────────────────────────

/// `model.tsv` is written in the shape `lean/Fol/Syntax.lean` specifies:
/// declarations before rows, dense bit strings, one `p2` row per source
/// element, and the digest on its own line.
#[test]
fn the_model_file_is_written_in_the_checkers_format() {
    let m = z3::parse_model(Z3_MODEL, &full_vocab(), 2).expect("readable");
    let tsv = m.to_model_tsv("4403d8aaa0c422f7", &[1, 2]);
    let lines: Vec<&str> = tsv.lines().collect();
    assert_eq!(lines[0], "domain\t2");
    assert_eq!(lines[1], "problem\t4403d8aaa0c422f7");
    assert_eq!(lines[2], "source\tz3");
    assert!(tsv.contains("decl1\tthing\n"));
    assert!(tsv.contains("decl2\top:worksFor\n"));
    assert!(tsv.contains("declc\ti:a\n"));
    assert!(tsv.contains("p1\tc:Company\t01\n"), "{tsv}");
    assert!(tsv.contains("p1\tlit\t00\n"), "{tsv}");
    // One p2 row per source element, in order, each of width 2.
    assert!(tsv.contains("p2\top:worksFor\t0\t11\n"), "{tsv}");
    assert!(tsv.contains("p2\top:worksFor\t1\t11\n"), "{tsv}");
    assert!(tsv.contains("const\ti:a\t0\n"));
    assert!(tsv.contains("cardinality_search\t1,2\n"));
    // A carrier with no search recorded omits the line rather than writing an
    // empty one, which the Lean parser would read as a field of its own.
    assert!(!m.to_model_tsv("x", &[]).contains("cardinality_search"));
}

/// `self_check` is the writer's own gate, and it fires. The Lean parser checks
/// all of this again at exit 2, but exit 2 is "unreadable", which is not a
/// verdict in either direction: a writer bug that produced one would look like
/// a mysterious file rather than a named defect here.
#[test]
fn the_writers_own_shape_gate_fires() {
    let mut m = z3::parse_model(Z3_MODEL, &full_vocab(), 2).expect("readable");
    m.self_check("z3").expect("starts well shaped");

    let mut short = m.clone();
    short.unary.insert("thing".into(), vec![true]);
    assert!(matches!(
        short.self_check("z3"),
        Err(IngestError::BadWidth { want: 2, got: 1, .. })
    ));

    let mut wide = m.clone();
    wide.consts.insert("i:a".into(), 9);
    assert!(matches!(
        wide.self_check("z3"),
        Err(IngestError::OutOfRange { index: 9, domain: 2, .. })
    ));

    m.domain = 0;
    assert!(matches!(m.self_check("z3"), Err(IngestError::EmptyDomain { .. })));
}

/// A structure with no symbols at all is still a structure, and the writer
/// says so rather than refusing. The EMPTY PROBLEM is what is refused, and it
/// is refused by the checker at exit 2, because every structure satisfies an
/// empty formula list and a certificate over one would say nothing.
#[test]
fn an_empty_signature_is_not_an_error() {
    let m = FiniteModel {
        domain: 1,
        unary: Default::default(),
        binary: Default::default(),
        consts: Default::default(),
        source: "hand",
        dropped: vec![],
    };
    m.self_check("hand").expect("well shaped");
    assert!(m.covers(&Vocabulary::default(), "hand").is_ok());
    assert_eq!(m.to_model_tsv("x", &[]).lines().count(), 3);
}
