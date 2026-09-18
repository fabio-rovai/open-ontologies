//! What the TSTP checker verifies, and every way of making it say no.
//!
//! A checker that has never rejected anything is not a checker. Half of this
//! file is therefore negative: a genuine Vampire refutation is taken apart one
//! mutation at a time, and each mutation must be caught by the field that is
//! supposed to catch it. The mutations are the ones a real defect would
//! produce — a stale problem file, a conjecture presented as an axiom, a
//! parent that does not exist, a cycle, a resolvent that is not the resolvent
//! — rather than syntactic damage that any parser would notice.
//!
//! The file is grouped:
//!
//!   1. The reader is the emitter's inverse, over hand-computed cases.
//!   2. A REAL Vampire 5.1.0 and a REAL E 3.2.5 refutation of a problem THIS
//!      repository's exporter produced, recorded under `tests/fixtures/tstp/`,
//!      with the problem regenerated from the exporter so the fixture cannot
//!      drift away from it, and a live run when a prover is on `PATH`.
//!   3. The negative tests.
//!   4. The verdict ladder, including the rule the whole design turns on: an
//!      UNCHECKED STEP MUST PREVENT THE STRONGEST WORD.
//!   5. The replayed rules, one worked case each, with a wrong version of each
//!      that must not check out.
//!
//! Nothing here claims the derivations are proofs. The strongest word in the
//! vocabulary is `refutation_fully_replayed` and it means the steps were
//! recomputed in unverified Rust, in a calculus no theorem in `lean/` is about.

mod common;

use std::path::PathBuf;

use open_ontologies::tptp::{Concept, FolProblem, OwlAxiom, fof};
use open_ontologies::tstp::{self, Formula, Report};

fn fixture(name: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tstp").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn iri(s: &str) -> String {
    format!("http://example.org/{s}")
}

/// The ontology the recorded fixtures are about: Greek ⊑ Person, Person ⊑
/// Mortal, socrates : Greek, with socrates : Mortal as the conjecture.
fn socrates_problem() -> FolProblem {
    let axioms = vec![
        OwlAxiom::SubClass(Concept::Atom(iri("Greek")), Concept::Atom(iri("Person"))),
        OwlAxiom::SubClass(Concept::Atom(iri("Person")), Concept::Atom(iri("Mortal"))),
        OwlAxiom::ClassAssert(Concept::Atom(iri("Greek")), iri("socrates")),
    ];
    let goal = OwlAxiom::ClassAssert(Concept::Atom(iri("Mortal")), iri("socrates"));
    FolProblem::build(&axioms, Some(&goal)).expect("freshness holds at trAx's own call sites")
}

fn parse1(text: &str) -> Formula {
    let mut v = tstp::parse_problem(text).expect("the reader must accept its own test input");
    assert_eq!(v.len(), 1, "expected exactly one annotated formula in {text:?}");
    v.remove(0).formula
}

// ── 1. The reader is the emitter's inverse ─────────────────────────────────

/// The whole leaf check rests on this. If the reader normalised anything on
/// the way in, a leaf that differs from the problem could come back matching
/// it, and the one gate that catches a prover answering about the wrong file
/// would be the gate that cannot fail.
///
/// The cases are the emitter's own output over worked OWL axioms, not strings
/// typed here, because what has to hold is that THIS reader inverts THAT
/// writer. What is typed here is the reader's expected AST, which is computed
/// from the `Form` by `from_tptp_form` and is a total embedding with no
/// choices in it.
#[test]
fn the_reader_recovers_exactly_the_formula_the_emitter_wrote() {
    use open_ontologies::tptp::{Ope, Translation};
    let cases: Vec<OwlAxiom> = vec![
        OwlAxiom::SubClass(Concept::Atom(iri("A")), Concept::Atom(iri("B"))),
        OwlAxiom::SubClass(
            Concept::Atom(iri("A")),
            Concept::Some_(iri("r"), Box::new(Concept::Atom(iri("B")))),
        ),
        OwlAxiom::SubClass(
            Concept::Atom(iri("A")),
            Concept::All_(iri("r"), Box::new(Concept::Atom(iri("B")))),
        ),
        OwlAxiom::SubClass(
            Concept::Inter(
                Box::new(Concept::Atom(iri("A"))),
                Box::new(Concept::Compl(Box::new(Concept::Atom(iri("B"))))),
            ),
            Concept::Bot,
        ),
        OwlAxiom::SubClass(Concept::Top, Concept::Union(
            Box::new(Concept::Atom(iri("A"))),
            Box::new(Concept::Atom(iri("B"))),
        )),
        OwlAxiom::SubClass(
            Concept::Atom(iri("A")),
            Concept::MinCard(2, iri("r"), Box::new(Concept::Atom(iri("B")))),
        ),
        OwlAxiom::SubClass(
            Concept::Atom(iri("A")),
            Concept::MaxCard(1, iri("r"), Box::new(Concept::Atom(iri("B")))),
        ),
        OwlAxiom::SubClass(Concept::Atom(iri("A")), Concept::OneOf(vec![iri("a"), iri("b")])),
        OwlAxiom::SubClass(Concept::Atom(iri("A")), Concept::HasVal(iri("r"), iri("a"))),
        OwlAxiom::SubClass(Concept::Atom(iri("A")), Concept::HasSelf(iri("r"))),
        OwlAxiom::SubClass(Concept::Atom(iri("A")), Concept::DataSome(iri("d"), iri("D"))),
        OwlAxiom::SubOProp(Ope::Named(iri("r")), Ope::Inv(iri("s"))),
        OwlAxiom::OPropDomain(Ope::Named(iri("r")), Concept::Atom(iri("A"))),
        OwlAxiom::OPropRange(Ope::Named(iri("r")), Concept::Atom(iri("A"))),
        OwlAxiom::Transitive(iri("r")),
        OwlAxiom::Symmetric(iri("r")),
        OwlAxiom::Irreflexive(iri("r")),
        OwlAxiom::Functional(iri("r")),
        OwlAxiom::InverseOf(iri("r"), iri("s")),
        OwlAxiom::Chain(vec![iri("r"), iri("s")], iri("t")),
        OwlAxiom::ClassAssert(Concept::Atom(iri("A")), iri("a")),
        OwlAxiom::OPropAssert(iri("r"), iri("a"), iri("b")),
        OwlAxiom::SameAs(iri("a"), iri("b")),
        OwlAxiom::DifferentFrom(iri("a"), iri("b")),
        OwlAxiom::DPropRange(iri("d"), iri("D")),
    ];
    for ax in &cases {
        let form = open_ontologies::tptp::Translation::axiom(ax).expect("in-fragment");
        let text = format!("fof(t, axiom, {}).", fof::form(&form));
        let back = parse1(&text);
        assert_eq!(
            back,
            tstp::from_tptp_form(&form),
            "the reader did not invert the emitter for {ax:?}\nemitted: {text}"
        );
    }
    // The two background axioms go through the same door.
    for form in Translation::background() {
        let text = format!("fof(t, axiom, {}).", fof::form(&form));
        assert_eq!(parse1(&text), tstp::from_tptp_form(&form));
    }
}

/// Decision 0005's quoting trap, on the reading side. In TPTP a double-quoted
/// string is a DISTINCT OBJECT, pairwise unequal to every other by fiat, and a
/// single-quoted atom is an ordinary constant. A reader that unquoted both to
/// the same name would silently identify them, which is the same unsoundness
/// the writer avoids by never emitting the double-quoted form for an IRI.
#[test]
fn a_distinct_object_is_not_the_quoted_atom_of_the_same_text() {
    let single = parse1("fof(t, axiom, p('i:a')).");
    let double = parse1("fof(t, axiom, p(\"i:a\")).");
    assert_ne!(single, double);
}

/// E prints an input status line before its result. Taking the first would
/// report the wrong verdict for the whole run.
#[test]
fn the_last_szs_status_line_is_the_one_reported() {
    let text = "# SZS status Started\nnoise\n# SZS status Theorem for x\n";
    assert_eq!(tstp::szs_status(text).as_deref(), Some("Theorem"));
    assert_eq!(tstp::szs_status("nothing here"), None);
}

/// Only what lies between the SZS markers is read. E prints its saturation
/// state before the proof in the same syntax; a reader that swallowed it would
/// build a derivation out of clauses the prover had abandoned.
#[test]
fn only_the_szs_block_is_read() {
    let text = "fof(decoy, axiom, p(a)).\n\
                % SZS output start Proof\n\
                fof(real, axiom, q(a), file('p', a1)).\n\
                % SZS output end Proof\n\
                fof(trailer, axiom, r(a)).\n";
    let d = tstp::parse_derivation(text).expect("the block parses");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].name, "real");
}

// ── 2. Real refutations of a problem this exporter produced ────────────────

/// The fixture is the exporter's output and nothing else. Regenerated here
/// from the same `FolProblem` the library builds, so a change to the
/// translation breaks this test rather than leaving a stale file behind that
/// the prover fixtures were recorded against.
#[test]
fn the_fixture_problem_is_what_this_exporter_emits() {
    assert_eq!(
        socrates_problem().to_tptp(),
        fixture("problem.p"),
        "tests/fixtures/tstp/problem.p is no longer what `fol --format tptp` emits. Re-record \
         it AND re-record the two prover outputs beside it; a proof of the old problem says \
         nothing about the new one"
    );
}

fn assert_real_refutation(r: &Report, who: &str) {
    assert_eq!(r.szs_status.as_deref(), Some("Theorem"), "{who}");
    assert!(r.problem_error.is_none() && r.derivation_error.is_none(), "{who}: {r:?}");
    assert!(r.derivation_wellformed, "{who}: {:?}", r.wellformedness_failures);
    assert!(r.root_is_false, "{who}");
    assert!(r.leaves_match_problem, "{who}: {:?}", r.leaf_failures);
    assert!(r.leaf_failures.is_empty(), "{who}");
    assert!(r.steps_not_reconstructed.is_empty(), "{who}: {:?}", r.steps_not_reconstructed);
    assert_eq!(r.conjecture_in_problem.as_deref(), Some("goal_classAssertion"), "{who}");
    assert!(r.conjecture_used, "{who}");
    assert!(r.conjecture_negation_checked, "{who}");
    assert_eq!(r.what_was_refuted, Some("axioms_and_the_negated_conjecture"), "{who}");
    // The prover's own clausification is never claimed to be checked, so the
    // strongest word is out of reach and must be.
    assert_eq!(r.verdict, "refutation_partially_replayed", "{who}");
    assert!(r.steps_checked > 0 && r.steps_checked < r.steps_total, "{who}");
}

/// A REAL Vampire 5.1.0 refutation, recorded from `vampire --proof tptp` over
/// `tests/fixtures/tstp/problem.p`.
///
/// This proof uses AVATAR, so it carries two `introduced(definition, …)`
/// leaves and a SAT-solver tail. Neither is checked and both are named, which
/// is the point: the report shows exactly how much of a real proof this module
/// can and cannot replay.
#[test]
fn a_real_vampire_refutation_checks_out() {
    let r = tstp::check(&fixture("problem.p"), &fixture("vampire-5.1.0.tstp"));
    assert_real_refutation(&r, "vampire");
    // Vampire echoes the axioms unchanged, so every leaf matches at the
    // strictest level.
    assert_eq!(r.leaf_match_levels.get("identical").copied(), Some(4));
    assert!(!r.leaf_match_levels.contains_key("alpha_equivalent"));
    assert!(
        r.steps_checked_by_rule.get("binary_resolvent").copied().unwrap_or(0) >= 5,
        "{:?}",
        r.steps_checked_by_rule
    );
    assert_eq!(r.steps_checked_by_rule.get("negation_of_the_conjecture").copied(), Some(1));
    assert_eq!(r.leaves_introduced.len(), 2, "AVATAR invents two definitions here");
    let unchecked: Vec<&str> = r.steps_unchecked.iter().map(|u| u.rule.as_str()).collect();
    for expected in ["cnf_transformation", "avatar_split_clause", "sat_conversion"] {
        assert!(unchecked.contains(&expected), "{expected} must be named: {unchecked:?}");
    }
}

/// A REAL E 3.2.5 refutation of the same file. E renames bound variables on
/// every axiom it reads, which is why the leaf check has an
/// `alpha_equivalent` level at all, and it nests its inference records, which
/// is why almost nothing in an E proof can be replayed. Both facts are
/// asserted rather than left to be discovered.
#[test]
fn a_real_eprover_refutation_checks_out() {
    let r = tstp::check(&fixture("problem.p"), &fixture("eprover-3.2.5.tstp"));
    assert_real_refutation(&r, "eprover");
    assert!(
        r.leaf_match_levels.get("alpha_equivalent").copied().unwrap_or(0) >= 2,
        "E renames variables, so some leaves must match only up to alpha: {:?}",
        r.leaf_match_levels
    );
    let inline = r
        .steps_unchecked
        .iter()
        .filter(|u| u.why.contains("inline inference record"))
        .map(|u| u.count)
        .sum::<usize>();
    assert!(inline > 0, "E nests inference records and the report must say so: {:?}", r.steps_unchecked);
}

/// The recorded fixtures are only worth having if the live prover still says
/// the same thing. Skips loudly without Vampire, and fails under
/// `OO_REQUIRE_FIXTURES=1`.
#[test]
fn a_live_vampire_run_agrees_with_the_recorded_one() {
    if common::skip_unless(
        tstp::Prover::Vampire.available(),
        "vampire on PATH",
        tstp::Prover::Vampire.install_line(),
    ) {
        return;
    }
    let dir = std::env::temp_dir().join(format!("oo-tstp-live-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let problem = socrates_problem().to_tptp();
    let pfile = dir.join("problem.p");
    std::fs::write(&pfile, &problem).unwrap();
    let out = std::process::Command::new("vampire")
        .args(tstp::Prover::Vampire.argv(&pfile, 10))
        .output()
        .expect("vampire is on PATH");
    let text = String::from_utf8_lossy(&out.stdout).to_string()
        + &String::from_utf8_lossy(&out.stderr);
    let r = tstp::check(&problem, &text);
    let _ = std::fs::remove_dir_all(&dir);
    // Vampire's strategy schedule is not pinned, so the exact step counts move
    // between builds. What must not move is the verdict and every field the
    // verdict is computed from.
    assert_real_refutation(&r, "live vampire");
}

// ── 3. The negative tests ──────────────────────────────────────────────────

/// Mutate a leaf so it is no longer the formula the problem carries under that
/// name. This is the check that catches a prover run against a stale file, and
/// it is the reason the leaf comparison is over parsed ASTs rather than over
/// the name alone.
#[test]
fn a_mutated_leaf_is_rejected() {
    let proof = fixture("vampire-5.1.0.tstp").replace(
        "'c:http://example.org/Greek'(X0) => 'c:http://example.org/Person'(X0)",
        "'c:http://example.org/Greek'(X0) => 'c:http://example.org/Mortal'(X0)",
    );
    assert_ne!(proof, fixture("vampire-5.1.0.tstp"), "the mutation must change the text");
    let r = tstp::check(&fixture("problem.p"), &proof);
    assert!(!r.leaves_match_problem);
    assert_eq!(r.verdict, "derivation_rejected");
    assert!(
        r.leaf_failures.iter().any(|f| f.why.contains("a different formula")),
        "{:?}",
        r.leaf_failures
    );
    // The structure is untouched, so the structural fields must still be true.
    // Collapsing them into one boolean would hide which gate actually bit.
    assert!(r.derivation_wellformed);
    assert!(r.root_is_false);
}

/// A leaf that names a formula the problem does not have. The prover was given
/// a different file, and the name is the only evidence of it.
#[test]
fn a_leaf_naming_a_formula_the_problem_lacks_is_rejected() {
    let proof = fixture("vampire-5.1.0.tstp")
        .replace("'owl_1_subClassOf'", "'owl_1_subClassOf_from_some_other_file'");
    let r = tstp::check(&fixture("problem.p"), &proof);
    assert!(!r.leaves_match_problem);
    assert_eq!(r.verdict, "derivation_rejected");
    assert!(
        r.leaf_failures.iter().any(|f| f.why.contains("no formula of that name")),
        "{:?}",
        r.leaf_failures
    );
}

/// The conjecture presented as an axiom. The formula is ours, the name is
/// ours, and the refutation would prove nothing about entailment: asserting
/// the goal and refuting the axioms together is not the same argument. Caught
/// by the role comparison, which is why the role is compared at all.
#[test]
fn a_conjecture_smuggled_in_as_an_axiom_is_rejected() {
    let proof = fixture("vampire-5.1.0.tstp").replace("fof(f7,conjecture,", "fof(f7,axiom,");
    let r = tstp::check(&fixture("problem.p"), &proof);
    assert!(!r.leaves_match_problem);
    assert_eq!(r.verdict, "derivation_rejected");
    assert!(
        r.leaf_failures.iter().any(|f| f.why.contains("refutes nothing")),
        "{:?}",
        r.leaf_failures
    );
}

/// A parent reference that resolves to nothing. A derivation with a hole in it
/// is not a derivation, however good the rest of it looks.
#[test]
fn a_dangling_parent_is_rejected() {
    let proof = fixture("vampire-5.1.0.tstp")
        .replace("inference(resolution,[],[f18,f20])", "inference(resolution,[],[f18,f999])");
    let r = tstp::check(&fixture("problem.p"), &proof);
    assert!(!r.derivation_wellformed);
    assert_eq!(r.verdict, "derivation_rejected");
    assert!(
        r.wellformedness_failures.iter().any(|w| w.contains("f999")),
        "{:?}",
        r.wellformedness_failures
    );
}

/// A cycle. Well-foundedness is the whole content of "the conclusion follows
/// from the premises"; without it a derivation can conclude anything from
/// itself.
#[test]
fn a_cycle_is_rejected() {
    let problem = "fof(a1, axiom, p(a)).\n";
    let proof = "% SZS output start Proof\n\
        fof(a1, axiom, p(a), file('p', a1)).\n\
        fof(c1, plain, q(a), inference(resolution,[],[a1,c2])).\n\
        fof(c2, plain, r(a), inference(resolution,[],[c1])).\n\
        fof(c3, plain, $false, inference(resolution,[],[c1,c2])).\n\
        % SZS output end Proof\n";
    let r = tstp::check(problem, proof);
    assert!(!r.derivation_wellformed);
    assert_eq!(r.verdict, "derivation_rejected");
    assert!(
        r.wellformedness_failures.iter().any(|w| w.contains("cycle")),
        "{:?}",
        r.wellformedness_failures
    );
}

/// Two annotated formulas with the same name make every reference to that name
/// ambiguous, so the DAG is not determined by the text.
#[test]
fn a_duplicated_name_is_rejected() {
    let problem = "fof(a1, axiom, p(a)).\n";
    let proof = "% SZS output start Proof\n\
        fof(a1, axiom, p(a), file('p', a1)).\n\
        fof(c1, plain, q(a), inference(resolution,[],[a1])).\n\
        fof(c1, plain, r(a), inference(resolution,[],[a1])).\n\
        fof(c2, plain, $false, inference(resolution,[],[c1])).\n\
        % SZS output end Proof\n";
    let r = tstp::check(problem, proof);
    assert_eq!(r.verdict, "derivation_rejected");
    assert!(
        r.wellformedness_failures.iter().any(|w| w.contains("more than one")),
        "{:?}",
        r.wellformedness_failures
    );
}

/// A genuine derivation checked against a DIFFERENT problem. Every name
/// resolves and every structural gate holds; only the formulas disagree. This
/// is what a stale export or a mismatched pair of files actually looks like.
#[test]
fn a_genuine_derivation_of_another_problem_is_rejected() {
    let other = fixture("problem.p").replace("http://example.org/Greek", "http://example.org/Hindu");
    let r = tstp::check(&other, &fixture("vampire-5.1.0.tstp"));
    assert!(!r.leaves_match_problem);
    assert_eq!(r.verdict, "derivation_rejected");
}

/// A resolution step whose conclusion is not the resolvent. The rule is one
/// this module implements and the shapes apply, so it gets its own word and is
/// NOT filed under "unchecked" where a forged step could hide behind a rule
/// name.
#[test]
fn a_resolution_conclusion_that_is_not_the_resolvent_is_not_reconstructed() {
    let problem = "fof(a1, axiom, p(a)).\nfof(a2, axiom, ( ~ p(a) | q(a) )).\n";
    let proof = "% SZS output start Proof\n\
        fof(a1, axiom, p(a), file('p', a1)).\n\
        fof(a2, axiom, ( ~p(a) | q(a) ), file('p', a2)).\n\
        fof(c1, plain, r(a), inference(resolution,[],[a2,a1])).\n\
        fof(c2, plain, $false, inference(resolution,[],[c1])).\n\
        % SZS output end Proof\n";
    let r = tstp::check(problem, proof);
    assert!(r.derivation_wellformed && r.leaves_match_problem && r.root_is_false);
    assert_eq!(r.verdict, "refutation_step_not_reconstructed");
    assert!(
        r.steps_not_reconstructed.iter().any(|s| s.node == "c1" && s.rule == "resolution"),
        "{:?}",
        r.steps_not_reconstructed
    );
}

/// A negated conjecture that is not the negation of the conjecture. Vampire's
/// own step here is exact, so a forged one has nowhere to hide.
#[test]
fn a_forged_negated_conjecture_is_not_reconstructed() {
    let proof = fixture("vampire-5.1.0.tstp").replace(
        "fof(f8,negated_conjecture,(\n  ~(thing('i:http://example.org/socrates') & \
         'c:http://example.org/Mortal'('i:http://example.org/socrates'))),",
        "fof(f8,negated_conjecture,(\n  ~thing('i:http://example.org/socrates')),",
    );
    assert_ne!(proof, fixture("vampire-5.1.0.tstp"), "the mutation must change the text");
    let r = tstp::check(&fixture("problem.p"), &proof);
    assert_eq!(r.verdict, "refutation_step_not_reconstructed");
    assert!(
        r.steps_not_reconstructed.iter().any(|s| s.rule == "negated_conjecture"),
        "{:?}",
        r.steps_not_reconstructed
    );
    assert!(!r.conjecture_negation_checked);
}

/// The occurs check is load-bearing. Without it `p(X, f(X))` and `p(Y, Y)`
/// unify, and a step that cannot be taken would be reported as replayed.
#[test]
fn the_occurs_check_refuses_a_resolution_that_needs_a_cyclic_binding() {
    let problem = "cnf(a1, axiom, p(X0, f(X0))).\ncnf(a2, axiom, ~p(Y0, Y0)).\n";
    let proof = "% SZS output start Proof\n\
        cnf(a1, axiom, p(X0, f(X0)), file('p', a1)).\n\
        cnf(a2, axiom, ~p(Y0, Y0), file('p', a2)).\n\
        cnf(c1, plain, $false, inference(resolution,[],[a1,a2])).\n\
        % SZS output end Proof\n";
    let r = tstp::check(problem, proof);
    assert_eq!(r.verdict, "refutation_step_not_reconstructed");
}

// ── 4. The verdict ladder ──────────────────────────────────────────────────

const ALL_CHECKED_PROBLEM: &str =
    "fof(a1, axiom, p(a)).\nfof(a2, axiom, ( ~ p(a) | q(a) )).\nfof(a3, axiom, ~ q(a)).\n";

fn all_checked_proof(rule_for_c1: &str) -> String {
    format!(
        "% SZS status Unsatisfiable\n\
         % SZS output start Proof\n\
         fof(a1, axiom, p(a), file('p', a1)).\n\
         fof(a2, axiom, ( ~p(a) | q(a) ), file('p', a2)).\n\
         fof(a3, axiom, ~q(a), file('p', a3)).\n\
         fof(c1, plain, q(a), inference({rule_for_c1},[],[a2,a1])).\n\
         fof(c2, plain, $false, inference(resolution,[],[c1,a3])).\n\
         % SZS output end Proof\n"
    )
}

/// The strongest word exists, is reachable, and says what it says. Every
/// reachable step here is recomputed from its premises.
#[test]
fn a_derivation_whose_every_step_is_replayed_reaches_the_strongest_word() {
    let r = tstp::check(ALL_CHECKED_PROBLEM, &all_checked_proof("resolution"));
    assert_eq!(r.verdict, "refutation_fully_replayed");
    assert_eq!(r.steps_checked, r.steps_total);
    assert!(r.steps_unchecked.is_empty());
    assert_eq!(r.what_was_refuted, Some("axioms_alone"), "this problem carries no conjecture");
    // And the word still is not a claim of unsatisfiability.
    let means = tstp::verdict_means("refutation_fully_replayed");
    assert!(means.contains("NOT a proof of unsatisfiability"), "{means}");
    assert!(means.contains("unverified Rust"), "{means}");
}

/// THE RULE THE DESIGN TURNS ON. One step renamed to a rule this module does
/// not replay, everything else identical, and the strongest word must be out
/// of reach. If this ever passes, the vocabulary has stopped meaning anything.
#[test]
fn one_unchecked_step_prevents_the_strongest_word() {
    let strong = tstp::check(ALL_CHECKED_PROBLEM, &all_checked_proof("resolution"));
    assert_eq!(strong.verdict, "refutation_fully_replayed");

    let weakened = tstp::check(ALL_CHECKED_PROBLEM, &all_checked_proof("cnf_transformation"));
    assert_ne!(weakened.verdict, "refutation_fully_replayed");
    assert_eq!(weakened.verdict, "refutation_partially_replayed");
    assert_eq!(weakened.steps_total, strong.steps_total);
    assert_eq!(weakened.steps_checked, strong.steps_checked - 1);
    assert_eq!(
        weakened.steps_unchecked.iter().map(|u| u.count).sum::<usize>(),
        1,
        "the unchecked step must be counted, not absorbed"
    );
    assert_eq!(weakened.steps_unchecked[0].rule, "cnf_transformation");
    assert!(!weakened.steps_unchecked[0].why.is_empty(), "an unchecked rule carries a reason");
    // Everything else is untouched, which is what makes this a test of the
    // ladder rather than of the structural gates.
    assert!(weakened.derivation_wellformed && weakened.leaves_match_problem);
}

/// An INTRODUCED leaf is a formula the prover invented. Nothing here checks
/// that the extension is conservative, so it counts as unchecked and keeps the
/// strongest word out of reach even when every inference step is replayed.
#[test]
fn an_introduced_definition_prevents_the_strongest_word() {
    let problem = "fof(a1, axiom, p(a)).\nfof(a2, axiom, ( ~ p(a) | q(a) )).\n";
    let proof = "% SZS output start Proof\n\
        fof(a1, axiom, p(a), file('p', a1)).\n\
        fof(a2, axiom, ( ~p(a) | q(a) ), file('p', a2)).\n\
        fof(d1, definition, ( ~q(a) ), introduced(definition,[new_symbols(definition,[sP0])])).\n\
        fof(c1, plain, q(a), inference(resolution,[],[a2,a1])).\n\
        fof(c2, plain, $false, inference(resolution,[],[c1,d1])).\n\
        % SZS output end Proof\n";
    let r = tstp::check(problem, proof);
    assert!(r.leaves_match_problem, "an introduced leaf is not a FAILED leaf: {:?}", r.leaf_failures);
    assert_eq!(r.leaves_introduced.len(), 1);
    assert_eq!(r.steps_checked, r.steps_total, "every inference step here is replayed");
    assert_eq!(r.verdict, "refutation_partially_replayed");
}

/// No empty clause means no refutation to check, and that is a separate word
/// from "the checker said no". A satisfiable problem reaching
/// `derivation_rejected` would be a false alarm on every consistent ontology.
#[test]
fn a_run_with_no_empty_clause_offers_no_refutation() {
    let problem = "fof(a1, axiom, p(a)).\n";
    let proof = "% SZS status Satisfiable\n\
        % SZS output start Saturation\n\
        fof(a1, axiom, p(a), file('p', a1)).\n\
        % SZS output end Saturation\n";
    let r = tstp::check(problem, proof);
    assert_eq!(r.verdict, "no_refutation_offered");
    assert!(!r.root_is_false);
    assert_eq!(r.szs_status.as_deref(), Some("Satisfiable"));
    assert_eq!(r.what_was_refuted, None, "nothing was refuted, so the field stays null");
}

/// A prover asked for a verdict and not for a proof prints one line. That is
/// not a refutation and it is not a rejection either.
#[test]
fn an_output_with_no_derivation_is_unparsed_and_says_how_to_fix_it() {
    let r = tstp::check("fof(a1, axiom, p(a)).\n", "% SZS status Theorem for x\n");
    assert_eq!(r.verdict, "derivation_unparsed");
    let why = r.derivation_error.expect("the reason is reported");
    assert!(why.contains("--proof-object") && why.contains("--proof"), "{why}");
}

/// An unreadable PROBLEM is not a statement about the prover, and the verdict
/// says so rather than blaming the derivation.
#[test]
fn an_unreadable_problem_is_its_own_verdict() {
    let r = tstp::check("this is not a TPTP file", &fixture("vampire-5.1.0.tstp"));
    assert_eq!(r.verdict, "problem_unparsed");
    assert!(r.problem_error.is_some());
    assert!(r.derivation_error.is_none(), "the derivation was never blamed");
}

/// Every verdict word has a sentence, and no sentence says the word means the
/// problem is unsatisfiable. The vocabulary is fixed here so that adding a
/// word without a meaning fails a test rather than printing
/// `unrecognised verdict` at a reader.
#[test]
fn every_verdict_word_has_a_meaning_and_none_of_them_claims_a_proof() {
    let words = [
        "problem_unparsed",
        "derivation_unparsed",
        "no_refutation_offered",
        "derivation_rejected",
        "refutation_step_not_reconstructed",
        "refutation_structure_checked",
        "refutation_partially_replayed",
        "refutation_fully_replayed",
    ];
    for w in words {
        let m = tstp::verdict_means(w);
        assert_ne!(m, "unrecognised verdict", "{w} has no sentence");
        assert!(
            !m.contains("certified") && !m.contains("proves that"),
            "{w} claims too much: {m}"
        );
    }
    assert_eq!(tstp::verdict_means("model_checked"), "unrecognised verdict");
}

/// The report says, in the output itself, that nothing here is certified and
/// that `lean/` is not involved. A limit stated only in a decision record gets
/// read without one.
#[test]
fn the_report_says_in_its_own_json_that_nothing_is_certified() {
    let r = tstp::check(&fixture("problem.p"), &fixture("vampire-5.1.0.tstp"));
    let j = tstp::report_json(&r);
    let checked_by = j["checked_by"].as_str().expect("checked_by is present");
    assert!(checked_by.contains("UNVERIFIED"), "{checked_by}");
    assert!(checked_by.contains("lean/ is not involved"), "{checked_by}");
    assert!(checked_by.contains("ORACLE OPINION"), "{checked_by}");
    let szs = j["szs_status_means"].as_str().expect("the echoed status is labelled");
    assert!(szs.contains("untrusted"), "{szs}");
}

// ── 5. The replayed rules, one worked case each ────────────────────────────

/// Run a hand-built derivation and return the report.
fn hand(problem: &str, body: &str) -> Report {
    tstp::check(problem, &format!("% SZS output start Proof\n{body}% SZS output end Proof\n"))
}

/// Subsumption resolution's conclusion IS the binary resolvent of its
/// premises, because the side clause's remainder is contained in the main
/// clause's. One check covers both rules and neither is weakened by it.
#[test]
fn subsumption_resolution_is_checked_as_the_binary_resolvent_it_is() {
    let problem = "cnf(a1, axiom, ( p(X0) | q(X0) )).\ncnf(a2, axiom, ( ~p(bb) | q(bb) )).\n";
    let r = hand(
        problem,
        "cnf(a1, axiom, ( p(X0) | q(X0) ), file('p', a1)).\n\
         cnf(a2, axiom, ( ~p(bb) | q(bb) ), file('p', a2)).\n\
         cnf(c1, plain, q(bb), inference(subsumption_resolution,[],[a2,a1])).\n\
         cnf(c2, plain, $false, inference(resolution,[],[c1])).\n",
    );
    assert_eq!(r.steps_checked_by_rule.get("binary_resolvent").copied(), Some(1));
    assert!(
        r.steps_not_reconstructed.iter().all(|s| s.node != "c1"),
        "{:?}",
        r.steps_not_reconstructed
    );
}

/// Two premises that happen to use the same variable name must be renamed
/// apart before unification. Without the renaming this resolvent does not
/// exist, because `X0` would have to be both `dd` and `cc`.
#[test]
fn resolution_standardises_the_premises_apart() {
    let problem = "cnf(a1, axiom, p(X0, cc)).\ncnf(a2, axiom, ( ~p(dd, X0) | r(X0) )).\n";
    let r = hand(
        problem,
        "cnf(a1, axiom, p(X0, cc), file('p', a1)).\n\
         cnf(a2, axiom, ( ~p(dd, X0) | r(X0) ), file('p', a2)).\n\
         cnf(c1, plain, r(cc), inference(resolution,[],[a1,a2])).\n\
         cnf(c2, plain, $false, inference(resolution,[],[c1])).\n",
    );
    assert_eq!(
        r.steps_checked_by_rule.get("binary_resolvent").copied(),
        Some(1),
        "{:?} {:?}",
        r.steps_checked_by_rule,
        r.steps_not_reconstructed
    );
}

/// `C ∨ s ≠ t` with `σ = mgu(s, t)` gives `Cσ`, and a conclusion that is not
/// `Cσ` does not check out.
#[test]
fn equality_resolution_is_replayed_and_a_wrong_one_is_not() {
    let problem = "cnf(a1, axiom, ( X0 != aa | q(X0) )).\ncnf(a2, axiom, ~q(aa)).\n";
    let good = hand(
        problem,
        "cnf(a1, axiom, ( X0 != aa | q(X0) ), file('p', a1)).\n\
         cnf(a2, axiom, ~q(aa), file('p', a2)).\n\
         cnf(c1, plain, q(aa), inference(equality_resolution,[],[a1])).\n\
         cnf(c2, plain, $false, inference(resolution,[],[c1,a2])).\n",
    );
    assert_eq!(good.verdict, "refutation_fully_replayed", "{good:?}");
    assert_eq!(good.steps_checked_by_rule.get("equality_resolved").copied(), Some(1));

    let bad = hand(
        problem,
        "cnf(a1, axiom, ( X0 != aa | q(X0) ), file('p', a1)).\n\
         cnf(a2, axiom, ~q(aa), file('p', a2)).\n\
         cnf(c1, plain, q(bb), inference(equality_resolution,[],[a1])).\n\
         cnf(c2, plain, $false, inference(resolution,[],[c1])).\n",
    );
    assert_eq!(bad.verdict, "refutation_step_not_reconstructed");
}

/// Two literals of one clause unified and merged.
#[test]
fn factoring_is_replayed_and_a_wrong_one_is_not() {
    let problem = "cnf(a1, axiom, ( q(X0) | q(aa) )).\ncnf(a2, axiom, ~q(aa)).\n";
    let good = hand(
        problem,
        "cnf(a1, axiom, ( q(X0) | q(aa) ), file('p', a1)).\n\
         cnf(a2, axiom, ~q(aa), file('p', a2)).\n\
         cnf(c1, plain, q(aa), inference(factoring,[],[a1])).\n\
         cnf(c2, plain, $false, inference(resolution,[],[c1,a2])).\n",
    );
    assert_eq!(good.verdict, "refutation_fully_replayed", "{good:?}");
    assert_eq!(good.steps_checked_by_rule.get("factor").copied(), Some(1));

    let bad = hand(
        problem,
        "cnf(a1, axiom, ( q(X0) | q(aa) ), file('p', a1)).\n\
         cnf(a2, axiom, ~q(aa), file('p', a2)).\n\
         cnf(c1, plain, q(bb), inference(factoring,[],[a1])).\n\
         cnf(c2, plain, $false, inference(resolution,[],[c1])).\n",
    );
    assert_eq!(bad.verdict, "refutation_step_not_reconstructed");
}

/// Literals `s != s` with syntactically identical sides, deleted. A conclusion
/// that deletes something else does not check out.
#[test]
fn trivial_inequality_removal_is_replayed_and_a_wrong_one_is_not() {
    let problem = "cnf(a1, axiom, ( aa != aa | q(aa) )).\ncnf(a2, axiom, ~q(aa)).\n";
    let good = hand(
        problem,
        "cnf(a1, axiom, ( aa != aa | q(aa) ), file('p', a1)).\n\
         cnf(a2, axiom, ~q(aa), file('p', a2)).\n\
         cnf(c1, plain, q(aa), inference(trivial_inequality_removal,[],[a1])).\n\
         cnf(c2, plain, $false, inference(resolution,[],[c1,a2])).\n",
    );
    assert_eq!(good.verdict, "refutation_fully_replayed", "{good:?}");
    assert_eq!(good.steps_checked_by_rule.get("trivial_inequality_removed").copied(), Some(1));

    let bad = hand(
        problem,
        "cnf(a1, axiom, ( aa != aa | q(aa) ), file('p', a1)).\n\
         cnf(a2, axiom, ~q(aa), file('p', a2)).\n\
         cnf(c1, plain, q(bb), inference(trivial_inequality_removal,[],[a1])).\n\
         cnf(c2, plain, $false, inference(resolution,[],[c1])).\n",
    );
    assert_eq!(bad.verdict, "refutation_step_not_reconstructed");
}

/// Vampire's `flattening` reassociates a disjunction and changes nothing else,
/// which is exactly "the same clause". A conclusion that drops a literal is
/// not the same clause and is caught.
#[test]
fn flattening_is_checked_as_the_same_clause() {
    let problem = "cnf(a1, axiom, ( ( p(aa) | q(aa) ) | r(aa) )).\n";
    let good = hand(
        problem,
        "cnf(a1, axiom, ( ( p(aa) | q(aa) ) | r(aa) ), file('p', a1)).\n\
         cnf(c1, plain, ( p(aa) | q(aa) | r(aa) ), inference(flattening,[],[a1])).\n\
         cnf(c2, plain, $false, inference(resolution,[],[c1])).\n",
    );
    assert_eq!(good.steps_checked_by_rule.get("same_clause").copied(), Some(1));

    let bad = hand(
        problem,
        "cnf(a1, axiom, ( ( p(aa) | q(aa) ) | r(aa) ), file('p', a1)).\n\
         cnf(c1, plain, ( p(aa) | q(aa) ), inference(flattening,[],[a1])).\n\
         cnf(c2, plain, $false, inference(resolution,[],[c1])).\n",
    );
    assert!(
        bad.steps_not_reconstructed.iter().any(|s| s.rule == "flattening"),
        "{:?}",
        bad.steps_not_reconstructed
    );
}

/// A step this module would check, over a premise that is not a clause, is
/// reported as UNCHECKED with the reason rather than as a failure. A prover
/// applying a clause rule to a formula is not evidence of anything and must
/// not be reported as if it were.
#[test]
fn a_clause_rule_over_a_non_clause_premise_is_unchecked_with_the_reason() {
    let problem = "fof(a1, axiom, ( p(aa) & q(aa) )).\n";
    let r = hand(
        problem,
        "fof(a1, axiom, ( p(aa) & q(aa) ), file('p', a1)).\n\
         fof(c1, plain, p(aa), inference(flattening,[],[a1])).\n\
         fof(c2, plain, $false, inference(resolution,[],[c1])).\n",
    );
    assert!(r.steps_not_reconstructed.iter().all(|s| s.rule != "flattening"));
    let u = r
        .steps_unchecked
        .iter()
        .find(|u| u.rule == "flattening")
        .expect("the rule is named as unchecked");
    assert!(u.why.contains("clause-shaped"), "{}", u.why);
}

/// A leaf whose only difference from the problem is bracketing of an n-ary
/// connective is matched, at its own level, and the level is reported.
/// Associativity of `&` and `|` is not in question; how much normalisation a
/// leaf needed is worth knowing.
#[test]
fn a_rebracketed_leaf_matches_at_its_own_level() {
    let problem = "fof(a1, axiom, ( p(aa) & ( q(aa) & r(aa) ) )).\n";
    let r = hand(
        problem,
        "fof(a1, axiom, ( ( p(aa) & q(aa) ) & r(aa) ), file('p', a1)).\n\
         fof(c1, plain, $false, inference(cnf_transformation,[],[a1])).\n",
    );
    assert!(r.leaves_match_problem, "{:?}", r.leaf_failures);
    assert_eq!(r.leaf_match_levels.get("associativity_normalised").copied(), Some(1));
    assert!(!r.leaf_match_levels.contains_key("identical"));
}

/// Nodes the empty clause does not depend on are counted and left out of the
/// step tallies. A prover's output often carries them, and counting them would
/// report work that the refutation does not rest on.
#[test]
fn nodes_the_refutation_does_not_rest_on_are_counted_separately() {
    let problem = "fof(a1, axiom, p(a)).\nfof(a2, axiom, ( ~ p(a) | q(a) )).\n";
    let r = hand(
        problem,
        "fof(a1, axiom, p(a), file('p', a1)).\n\
         fof(a2, axiom, ( ~p(a) | q(a) ), file('p', a2)).\n\
         fof(c1, plain, q(a), inference(resolution,[],[a2,a1])).\n\
         fof(c2, plain, $false, inference(resolution,[],[a2,a1])).\n\
         fof(spare, plain, r(a), inference(cnf_transformation,[],[a1])).\n",
    );
    assert!(r.nodes_outside_the_refutation >= 2, "c1 and spare are not cited: {r:?}");
    assert!(
        r.steps_unchecked.iter().all(|u| u.rule != "cnf_transformation"),
        "a node outside the refutation must not be counted: {:?}",
        r.steps_unchecked
    );
}
