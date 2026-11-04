// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! The three new printers: SMT-LIB 2, LADR, and the checker's own format.
//!
//! One representation, five printers. `src/tptp.rs` computes `Form` once and
//! `fof`, `clif`, `smtlib`, `ladr` and `checkfmt` are folds over it. These
//! tests pin what each of the three new ones writes.
//!
//! **Every expected string here was computed by hand.** A golden file captured
//! from the emitter agrees with the emitter by construction and proves
//! nothing, which is the same argument `fol_translation_correspondence_test.rs`
//! makes at the top of itself and the reason that file exists at all. The
//! SMT-LIB cases are unrolled from SMT-LIB 2.6's own grammar; the LADR cases
//! from the Prover9/Mace4 manual's connective table; the checker-format cases
//! from `Fol.Parse.showForm` in `lean/Fol/Parse.lean`, constructor by
//! constructor.
//!
//! The digest is the sharpest of them. `4403d8aaa0c422f7` is pinned THREE
//! times: by a `#guard` in `lean/Fol/Parse.lean`, by the fixture
//! `tests/fixtures/folmodel/problem.tsv`, and here. Two independent
//! implementations of FNV-1a 64 and of one canonical printer have to agree on
//! it, and a change to either fails on its own side.
//!
//! The groups:
//!
//!   1. The checker format: `showForm`, the digest, the whitespace refusal.
//!   2. The goal is negated ONCE, on the way into the model-finding half.
//!   3. SMT-LIB 2, hand-computed, both encodings, and the symbols it refuses.
//!   4. LADR, hand-computed, the mangling, and the variable-letter gate.
//!   5. One problem, three files: the printers cannot be asked different
//!      questions.

use open_ontologies::tptp::{
    Concept, FolProblem, Form, OwlAxiom, P1, P2, Syntax, Term, Translation, checkfmt, ladr,
    smtlib, smtlib::SmtEncoding, sym,
};

fn v(n: u32) -> Term {
    Term::Var(n)
}
fn k(a: &str) -> Term {
    Term::Const(a.to_string())
}
fn cls(a: &str, t: Term) -> Form {
    Form::App1(P1::Cls(a.to_string()), t)
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
fn neg(f: Form) -> Form {
    Form::Neg(Box::new(f))
}
fn and(f: Form, g: Form) -> Form {
    Form::And(Box::new(f), Box::new(g))
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

/// The five formulas of the worked example in `lean/Fol/Syntax.lean`, built
/// here from `Form` rather than read off the fixture, so that this side and
/// the Lean side are two independent constructions of the same object.
fn worked_example() -> Vec<(&'static str, Form)> {
    vec![
        ("axiom", all(0, neg(and(thing(v(0)), lit(v(0)))))),
        ("axiom", thing(k("a"))),
        (
            "axiom",
            all(
                0,
                imp(
                    cls("Person", v(0)),
                    ex(1, and(op("worksFor", v(0), v(1)), cls("Company", v(1)))),
                ),
            ),
        ),
        ("axiom", cls("Person", k("a"))),
        ("goal_negated", neg(cls("Company", k("a")))),
    ]
}

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/folmodel")
        .join(name)
}

// ── 1. The checker format ───────────────────────────────────────────────────

/// `Fol.Parse.showForm`, constructor by constructor.
///
/// Prefix, no parentheses, single spaces. The grammar is unambiguous because
/// every constructor has fixed arity, which is why the file can afford to have
/// no brackets at all and why a symbol containing a space would re-parse as a
/// DIFFERENT formula rather than as a syntax error.
#[test]
fn show_form_is_the_leans_printer() {
    let cases: Vec<(Form, &str)> = vec![
        (Form::Tru, "tru"),
        (Form::Fls, "fls"),
        (thing(v(3)), "app1 thing var 3"),
        (lit(k("a")), "app1 lit const i:a"),
        (cls("Person", v(0)), "app1 c:Person var 0"),
        (Form::App1(P1::Dt("int".into()), v(0)), "app1 d:int var 0"),
        (op("worksFor", v(0), v(1)), "app2 op:worksFor var 0 var 1"),
        (
            Form::App2(P2::Dp("age".into()), k("a"), v(1)),
            "app2 dp:age const i:a var 1",
        ),
        (Form::Eq(k("a"), k("b")), "eq const i:a const i:b"),
        (neg(Form::Tru), "neg tru"),
        (and(Form::Tru, Form::Fls), "and tru fls"),
        (Form::Or(Box::new(Form::Tru), Box::new(Form::Fls)), "or tru fls"),
        (imp(Form::Tru, Form::Fls), "imp tru fls"),
        (all(7, Form::Tru), "all 7 tru"),
        (ex(2, Form::Fls), "ex 2 fls"),
    ];
    for (f, want) in cases {
        assert_eq!(checkfmt::show_form(&f).expect("writable"), want, "for {f:?}");
    }
}

/// The formulas this side builds print exactly the fixture the Lean side
/// reads. Byte for byte, in file order.
#[test]
fn the_worked_example_round_trips_against_the_lean_fixture() {
    let text = std::fs::read_to_string(fixture("problem.tsv")).expect("fixture");
    let want: Vec<(&str, &str)> = text
        .lines()
        .map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            (c[0], c[2])
        })
        .collect();
    let got = worked_example();
    assert_eq!(got.len(), want.len(), "the fixture has {} lines", want.len());
    for ((role, f), (wrole, wform)) in got.iter().zip(&want) {
        assert_eq!(role, wrole);
        assert_eq!(&checkfmt::show_form(f).expect("writable"), wform);
    }
}

/// FNV-1a 64 and the canonical string, against the value pinned in the Lean.
///
/// `lean/Fol/Parse.lean` carries `#guard problemDigest … = "4403d8aaa0c422f7"`,
/// so a change to either printer or to either hash fails a build on one side
/// and this test on the other. The constant is spelled out here rather than
/// read from the fixture on purpose: reading it would make this test agree
/// with whatever the fixture said.
#[test]
fn the_digest_agrees_with_the_lean() {
    let canonical: Vec<String> = worked_example()
        .iter()
        .map(|(role, f)| format!("{role}\t{}", checkfmt::show_form(f).expect("writable")))
        .collect();
    let digest = checkfmt::hex16(checkfmt::fnv1a64(&canonical.join("\n")));
    assert_eq!(digest, "4403d8aaa0c422f7");
    // And the model fixture the Lean checker accepts carries that same digest,
    // which is what binds the two files to each other.
    let model = std::fs::read_to_string(fixture("model.tsv")).expect("fixture");
    assert!(
        model.contains("problem\t4403d8aaa0c422f7"),
        "the shipped model.tsv must carry the digest this side computes"
    );
}

/// The digest is over the parsed formulas and the ROLES, not over the bytes.
///
/// A file that relabelled `goal_negated` as `axiom` would otherwise digest the
/// same, and a report reads the role to decide whether a non-entailment was
/// even asked about.
#[test]
fn relabelling_the_goal_changes_the_digest() {
    let canon = |entries: &[(&str, Form)]| {
        let v: Vec<String> = entries
            .iter()
            .map(|(r, f)| format!("{r}\t{}", checkfmt::show_form(f).expect("writable")))
            .collect();
        checkfmt::hex16(checkfmt::fnv1a64(&v.join("\n")))
    };
    let real = worked_example();
    let mut forged: Vec<(&str, Form)> =
        real.iter().map(|(r, f)| (*r, f.clone())).collect();
    forged.last_mut().expect("five entries").0 = "axiom";
    assert_ne!(canon(&real), canon(&forged));
}

/// A symbol containing a space, a tab, a CR or a newline is REFUSED.
///
/// `bare()` strips the angle brackets before an IRI reaches `Term::Const`, so
/// the symbols are raw IRIs out of the graph and this is live rather than
/// defensive. Shown failing on each of the four characters: a gate that cannot
/// fail is decoration.
#[test]
fn a_symbol_the_token_stream_cannot_survive_is_refused() {
    for bad in [" ", "\t", "\r", "\n"] {
        let f = cls(&format!("http://e/A{bad}B"), v(0));
        let e = checkfmt::show_form(&f).expect_err("must refuse");
        assert_eq!(e.character, bad.chars().next().expect("one char"));
        assert_eq!(e.syntax, "the oo-folmodel checker format");
        assert!(e.to_string().contains("DIFFERENT formula"), "{e}");
    }
    // And the same symbol in a constant position, which is the one `bare()`
    // actually feeds raw IRIs into.
    let f = thing(k("http://e/a b"));
    assert!(checkfmt::show_form(&f).is_err());
    // A symbol WITHOUT one of those is written, so the gate is not a constant.
    assert_eq!(
        checkfmt::show_form(&thing(k("http://e/a"))).expect("writable"),
        "app1 thing const i:http://e/a"
    );
}

// ── 2. The goal is negated once ─────────────────────────────────────────────

/// `formulas()` carries the conjecture; `checker_entries()` carries its
/// NEGATION under the role `goal_negated`, and nothing downstream negates
/// again.
///
/// This is the one difference between the refutation half of the export and
/// the model-finding half, and getting it wrong in either direction is silent:
/// a file asserting the unnegated conjecture is satisfiable exactly when the
/// ontology already says so, and a checker that negated a second time would
/// certify a countermodel to the wrong sentence.
#[test]
fn the_goal_is_negated_exactly_once() {
    let axioms = vec![OwlAxiom::ClassAssert(
        Concept::Atom("http://e/Person".into()),
        "http://e/a".into(),
    )];
    let goal = OwlAxiom::ClassAssert(Concept::Atom("http://e/Company".into()), "http://e/a".into());
    let p = FolProblem::build(&axioms, Some(&goal)).expect("freshness holds");

    let exported = p.formulas();
    let (_, role, conj) = exported.last().expect("a conjecture row");
    assert_eq!(*role, "conjecture");

    let entries = p.checker_entries();
    let last = entries.last().expect("a goal row");
    assert_eq!(last.role, "goal_negated");
    assert_eq!(last.form, Form::Neg(Box::new((*conj).clone())));
    // Every other row keeps its formula and is an axiom.
    assert!(entries[..entries.len() - 1].iter().all(|e| e.role == "axiom"));
    assert_eq!(entries.len(), exported.len());
    // A problem with no goal has no goal_negated row at all.
    let bare = FolProblem::build(&axioms, None).expect("freshness holds");
    assert!(bare.checker_entries().iter().all(|e| e.role == "axiom"));
}

// ── 3. SMT-LIB 2 ────────────────────────────────────────────────────────────

/// Hand-computed from SMT-LIB 2.6. Fully parenthesised, `=>` for implication,
/// `(forall ((x S)) …)` for the binder, and `true`/`false` for the two truth
/// constants, which SMT-LIB has and TPTP spells `$true`/`$false`.
#[test]
fn smtlib_renders_every_constructor() {
    let cases: Vec<(Form, &str)> = vec![
        (Form::Tru, "true"),
        (Form::Fls, "false"),
        (thing(v(0)), "(thing X0)"),
        (lit(v(1)), "(lit X1)"),
        (cls("http://e/A", v(0)), "(|c:http://e/A| X0)"),
        (
            op("http://e/r", v(0), k("http://e/a")),
            "(|op:http://e/r| X0 |i:http://e/a|)",
        ),
        (
            Form::Eq(k("http://e/a"), k("http://e/b")),
            "(= |i:http://e/a| |i:http://e/b|)",
        ),
        (neg(thing(v(0))), "(not (thing X0))"),
        (and(Form::Tru, Form::Fls), "(and true false)"),
        (Form::Or(Box::new(Form::Tru), Box::new(Form::Fls)), "(or true false)"),
        (imp(Form::Tru, Form::Fls), "(=> true false)"),
        (all(0, thing(v(0))), "(forall ((X0 U)) (thing X0))"),
        (ex(2, lit(v(2))), "(exists ((X2 U)) (lit X2))"),
    ];
    for (f, want) in cases {
        assert_eq!(smtlib::form(&f).expect("writable"), want, "for {f:?}");
    }
}

/// One real translated axiom, unrolled by hand through `OwlLean.trAx` and then
/// through the SMT-LIB grammar, so that the two unrollings meet.
///
/// `trAx (subClass (atom A) (some_ r (atom B)))` is
/// `allObj 0 (imp (cls A (v 0)) (exObj 2 (and (op r (v 0) (v 2)) (cls B (v 2)))))`
/// with `allObj n f = all n (imp (thing (v n)) f)` and
/// `exObj n f = ex n (and (thing (v n)) f)`. The bound variable is X2 because
/// `trAx` starts the counter at 2, which is the same fact
/// `some_values_from_allocates_x2` pins on the TPTP side.
#[test]
fn smtlib_of_a_translated_axiom() {
    let f = Translation::axiom(&OwlAxiom::SubClass(
        Concept::Atom("http://e/A".into()),
        Concept::Some_("http://e/r".into(), Box::new(Concept::Atom("http://e/B".into()))),
    ))
    .expect("freshness holds at trAx's own call sites");
    assert_eq!(
        smtlib::form(&f).expect("writable"),
        "(forall ((X0 U)) (=> (thing X0) (=> (|c:http://e/A| X0) \
         (exists ((X2 U)) (and (thing X2) (and (|op:http://e/r| X0 X2) (|c:http://e/B| X2)))))))"
    );
}

/// The two sort declarations, and the difference between them, which is what
/// decides whether an `unsat` means anything.
#[test]
fn the_two_encodings_declare_the_carrier_differently() {
    assert_eq!(smtlib::sort_decl(SmtEncoding::Unbounded), "(declare-sort U 0)");
    assert_eq!(smtlib::logic(SmtEncoding::Unbounded), "UF");
    assert_eq!(SmtEncoding::Unbounded.name(), "unbounded");
    assert_eq!(SmtEncoding::Unbounded.bound(), None);

    assert_eq!(
        smtlib::sort_decl(SmtEncoding::Finite(3)),
        "(declare-datatypes ((U 0)) (((e0) (e1) (e2))))"
    );
    assert_eq!(smtlib::logic(SmtEncoding::Finite(3)), "UFDT");
    assert_eq!(SmtEncoding::Finite(3).name(), "finite(3)");
    assert_eq!(SmtEncoding::Finite(3).bound(), Some(3));
}

/// `thing` and `lit` are legal simple symbols and stay bare; everything else
/// carries a colon, which SMT-LIB's simple-symbol set excludes, and is quoted.
#[test]
fn smtlib_quotes_exactly_what_it_must() {
    assert_eq!(smtlib::symbol("thing").expect("writable"), "thing");
    assert_eq!(smtlib::symbol("lit").expect("writable"), "lit");
    assert_eq!(smtlib::symbol("c:X").expect("writable"), "|c:X|");
    assert_eq!(
        smtlib::symbol("i:http://e/a#b").expect("writable"),
        "|i:http://e/a#b|"
    );
}

/// A symbol containing `|` or `\` is REFUSED, because a quoted SMT-LIB symbol
/// admits every character except those two and defines no escape for either.
/// Shown failing on both.
#[test]
fn smtlib_refuses_the_two_characters_it_cannot_escape() {
    for bad in ['|', '\\'] {
        let e = smtlib::symbol(&format!("c:http://e/A{bad}B")).expect_err("must refuse");
        assert_eq!(e.character, bad);
        assert_eq!(e.syntax, "SMT-LIB 2");
        // And through the whole formula printer, not only the symbol function.
        let f = cls(&format!("http://e/A{bad}B"), v(0));
        assert!(smtlib::form(&f).is_err(), "form() must propagate the refusal");
    }
}

/// `--smt-domain 0` asks for an empty carrier, which no first-order structure
/// has and which `Fol.FinModel`, defined only at `n+1`, cannot represent.
#[test]
fn an_empty_carrier_is_refused_at_the_flag() {
    let e = Syntax::parse("smtlib", None, None, Some(0)).expect_err("must refuse");
    assert!(e.to_string().contains("empty carrier"), "{e}");
    assert!(Syntax::parse("smtlib", None, None, Some(1)).is_ok());
    assert!(Syntax::parse("smtlib", None, None, None).is_ok());
}

// ── 4. LADR, for Mace4 ──────────────────────────────────────────────────────

/// Hand-computed from the Prover9/Mace4 manual's connective table: `-` for
/// negation, `&`, `|`, `->`, `all x`, `exists x`, and `$T`/`$F` for the truth
/// constants. Fully parenthesised, because LADR's precedence table is not
/// worth relying on and a misparse there is silent.
#[test]
fn ladr_renders_every_constructor() {
    let problem = FolProblem {
        background: vec![],
        ind_axioms: vec![],
        axioms: vec![
            ("x".into(), cls("http://e/A", v(0))),
            ("x".into(), lit(v(0))),
            ("x".into(), thing(v(0))),
            ("x".into(), op("http://e/r", v(0), k("http://e/a"))),
        ],
        conjecture: None,
        individuals: Default::default(),
    };
    let t = ladr::SymbolTable::build(&problem).expect("no variable letters");
    // Sorted order: c:http://e/A, lit, thing → p0, p1, p2.
    assert_eq!(t.unary.get("c:http://e/A").map(String::as_str), Some("p0"));
    assert_eq!(t.unary.get("lit").map(String::as_str), Some("p1"));
    assert_eq!(t.unary.get("thing").map(String::as_str), Some("p2"));
    assert_eq!(t.binary.get("op:http://e/r").map(String::as_str), Some("r0"));
    assert_eq!(t.consts.get("i:http://e/a").map(String::as_str), Some("c0"));

    let cases: Vec<(Form, &str)> = vec![
        (Form::Tru, "$T"),
        (Form::Fls, "$F"),
        (thing(v(0)), "p2(x0)"),
        (cls("http://e/A", k("http://e/a")), "p0(c0)"),
        (op("http://e/r", v(0), v(1)), "r0(x0,x1)"),
        (
            Form::Eq(k("http://e/a"), k("http://e/a")),
            "(c0 = c0)",
        ),
        (neg(thing(v(0))), "-(p2(x0))"),
        (and(Form::Tru, Form::Fls), "($T & $F)"),
        (Form::Or(Box::new(Form::Tru), Box::new(Form::Fls)), "($T | $F)"),
        (imp(Form::Tru, Form::Fls), "($T -> $F)"),
        (all(0, thing(v(0))), "(all x0 (p2(x0)))"),
        (ex(1, thing(v(1))), "(exists x1 (p2(x1)))"),
    ];
    for (f, want) in cases {
        assert_eq!(ladr::form(&f, &t).expect("in the table"), want, "for {f:?}");
    }
}

/// THE LADR TRAP, as a gate that fires.
///
/// LADR reads a name whose first letter is in {u,v,w,x,y,z} as a VARIABLE.
/// Measured on LADR 2009-11A: `p0(w0). -p0(k0).` is echoed in Mace4's own
/// CLAUSES FOR SEARCH block as `p0(x).` and `-p0(k0).`, the search is
/// exhausted, and the run reports no model with no error at all. A mangler
/// that emitted such a name would make every Mace4 run silently wrong, so the
/// check is a function and this is it failing.
#[test]
fn the_ladr_variable_letters_are_refused() {
    for bad in ["u0", "v1", "w0", "x3", "y", "z_name", "X0", "Z9"] {
        assert!(ladr::is_variable_name(bad), "{bad} must read as a variable");
        let e = ladr::check_not_variable(bad).expect_err("must refuse");
        assert_eq!(e.name, bad);
        assert!(e.to_string().contains("VARIABLE"), "{e}");
    }
    // The prefixes the mangler actually uses are not variables, so the gate is
    // not a constant. `c…` is included deliberately: LADR names its own Skolem
    // constants `c1, c2, …` and picks one not already in the problem, measured.
    for good in ["p0", "r0", "c0", "p17", "thing"] {
        assert!(!ladr::is_variable_name(good), "{good} must read as a symbol");
        assert!(ladr::check_not_variable(good).is_ok());
    }
    // Bound variables ARE variables under the same convention, which is the
    // point: `x0` has to read as one.
    assert!(ladr::is_variable_name("x0"));
}

/// A symbol the table never issued cannot be rendered, and cannot be
/// demangled back into one either.
#[test]
fn the_mangling_is_a_closed_table() {
    let problem = FolProblem {
        background: vec![],
        ind_axioms: vec![],
        axioms: vec![("x".into(), thing(v(0)))],
        conjecture: None,
        individuals: Default::default(),
    };
    let t = ladr::SymbolTable::build(&problem).expect("no variable letters");
    assert_eq!(t.len(), 1);
    assert_eq!(t.demangle("p0"), Some("thing"));
    // A Skolem name Mace4 invented is not in the table, which is how the
    // ingestion knows to drop it rather than claim it interpreted something.
    assert_eq!(t.demangle("f1"), None);
    assert_eq!(t.demangle("c7"), None);
    // And a formula over a symbol outside the table is an error, not a guess.
    assert!(ladr::form(&cls("http://e/A", v(0)), &t).is_err());
}

// ── 5. One problem, three files ─────────────────────────────────────────────

/// The SMT-LIB file, the LADR file and `problem.tsv` are folds over the same
/// `checker_entries()`, so they carry the same formulas in the same order.
///
/// This is the mechanical reason the solver cannot be asked a different
/// question from the one the checker then checks. It is the same argument
/// `FolProblem::formulas` makes for TPTP and CLIF, one level down.
#[test]
fn the_three_printers_carry_the_same_problem() {
    let axioms = vec![
        OwlAxiom::SubClass(
            Concept::Atom("http://e/A".into()),
            Concept::Some_("http://e/r".into(), Box::new(Concept::Atom("http://e/B".into()))),
        ),
        OwlAxiom::ClassAssert(Concept::Atom("http://e/A".into()), "http://e/a".into()),
    ];
    let goal = OwlAxiom::ClassAssert(Concept::Atom("http://e/B".into()), "http://e/a".into());
    let p = FolProblem::build(&axioms, Some(&goal)).expect("freshness holds");
    let entries = p.checker_entries();

    let (tsv, digest) = p.to_problem_tsv().expect("writable");
    assert_eq!(tsv.lines().count(), entries.len());
    assert_eq!(digest.len(), 16);
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));

    let smt = p.to_smtlib(SmtEncoding::Finite(2)).expect("writable");
    assert_eq!(smt.matches("\n(assert ").count(), entries.len());

    let t = ladr::SymbolTable::build(&p).expect("no variable letters");
    let la = ladr::problem(&p, &t, 5).expect("writable");
    // One clause line per entry, each terminated by LADR's `.`
    let clauses = la
        .lines()
        .skip_while(|l| !l.starts_with("formulas(assumptions)"))
        .filter(|l| l.starts_with("  ") && !l.trim_start().starts_with('%'))
        .count();
    assert_eq!(clauses, entries.len());

    // And the LAST formula of each is the negated goal, not the conjecture.
    let goal_form = &entries.last().expect("a goal").form;
    assert!(tsv.lines().last().expect("a line").starts_with("goal_negated\t"));
    assert!(smt.contains(&format!("(assert {})", smtlib::form(goal_form).expect("writable"))));
    assert!(la.contains(&format!("  {}.", ladr::form(goal_form, &t).expect("in table"))));
}

/// The vocabulary is collected from the NEGATED goal too, so a symbol that
/// occurs only in the conjecture is still declared. Without that, the model
/// file would omit it and `Fol.covers` would reject the run on an attribution
/// failure that is really an exporter bug.
#[test]
fn the_vocabulary_includes_the_goals_symbols() {
    let axioms = vec![OwlAxiom::ClassAssert(
        Concept::Atom("http://e/A".into()),
        "http://e/a".into(),
    )];
    let goal = OwlAxiom::ClassAssert(Concept::Atom("http://e/OnlyInTheGoal".into()), "http://e/a".into());
    let p = FolProblem::build(&axioms, Some(&goal)).expect("freshness holds");
    let v = p.vocabulary();
    assert!(v.unary.contains("c:http://e/OnlyInTheGoal"), "{:?}", v.unary);
    assert!(v.unary.contains("thing"));
    assert!(v.consts.contains("i:http://e/a"));
    assert!(!v.is_empty());
}

/// One namer, and the seven images are pairwise disjoint whatever the IRIs
/// are. `thing` and `lit` carry no colon; every other arm carries its own.
///
/// This is what makes the monomorphisation at `Sym := String` in
/// `lean/Fol/Syntax.lean` sound: without the prefixes a class IRI and a
/// datatype IRI with the same spelling would become ONE predicate, and a model
/// could satisfy a problem the ontology does not.
#[test]
fn the_seven_symbol_images_are_disjoint() {
    let same = "http://e/X";
    let images = [
        sym::p1(&P1::Thing),
        sym::p1(&P1::Lit),
        sym::p1(&P1::Cls(same.into())),
        sym::p1(&P1::Dt(same.into())),
        sym::p2(&P2::Op(same.into())),
        sym::p2(&P2::Dp(same.into())),
        sym::constant(same),
    ];
    let unique: std::collections::BTreeSet<&String> = images.iter().collect();
    assert_eq!(unique.len(), 7, "{images:?}");
    assert_eq!(images[2], "c:http://e/X");
    assert_eq!(images[3], "d:http://e/X");
    assert_eq!(images[4], "op:http://e/X");
    assert_eq!(images[5], "dp:http://e/X");
    assert_eq!(images[6], "i:http://e/X");
}

/// The namer is the one the OTHER two printers already used, so a drift in it
/// shows up in all five. Checked by stripping the quoting each adds.
#[test]
fn the_older_printers_read_the_same_namer() {
    let f = cls("http://e/A", v(0));
    let tptp = open_ontologies::tptp::fof::form(&f);
    let clif = open_ontologies::tptp::clif::form(&f);
    let smt = smtlib::form(&f).expect("writable");
    let image = sym::p1(&P1::Cls("http://e/A".into()));
    assert!(tptp.contains(&format!("'{image}'")), "{tptp}");
    assert!(clif.contains(&format!("\"{image}\"")), "{clif}");
    assert!(smt.contains(&format!("|{image}|")), "{smt}");
}
