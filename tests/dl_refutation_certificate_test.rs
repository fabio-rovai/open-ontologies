//! Refutation certificates for the SHIQ tableaux reasoner, end to end.
//!
//! `tests/dl_model_certificate_test.rs` closes the loop for the POSITIVE
//! answers: a class the reasoner calls satisfiable comes with a model, and
//! `Dl.satisfiable_of_checkModel` says an accepted model really is one. The
//! negative answers had nothing. `oo-dlmodel`'s own header used to say so, and
//! said that certifying them "needs a closed tableau with its blocking
//! argument". Half of that was right. A closed tableau is what it needs;
//! blocking is a COMPLETENESS device, for making a search for a model
//! terminate, and a refutation never needs one.
//!
//! So the negative answers now carry a derivation instead of a structure, and
//! `Dl.unsatisfiable_of_check` says an accepted derivation means the axiom set
//! has no model of ANY size. These tests close that loop.
//!
//!   1. The reasoner emits and the checker accepts, once for each shape of
//!      derivation the calculus can build: a flat clash, a bound clash that
//!      needs no witnesses, the ≥ rule inventing witnesses that a later bound
//!      then refutes, and a disjunction, which is the only rule that makes the
//!      certificate a tree rather than a list.
//!   2. The checker can say no, and each way is exercised on its own. A step
//!      citing an axiom nobody asserted, a witness reusing a name the branch
//!      already carries, and a file whose tree does not consume it. A gate that
//!      cannot fail is decoration.
//!   3. The emitter declines rather than inventing. A satisfiable class gets
//!      no certificate, and an inconsistency that needs a rule the calculus
//!      lacks gets no certificate AND is told which rule.
//!
//! The checker needs a Lean toolchain (`lake`) AND the `oo-dlrefute` target in
//! `lean/lakefile.toml`. Without either these tests skip loudly through
//! `common::skip_unless`; the CI job that installs Lean runs them with
//! `OO_REQUIRE_FIXTURES=1`, which turns that skip into a failure.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::tableaux::DlReasoner;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn lean_dir() -> PathBuf {
    std::env::var("OO_LEAN_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo().join("lean"))
}

fn lake_available() -> bool {
    // `.current_dir(lean_dir())` is not cosmetic: elan resolves the toolchain
    // from the working directory's `lean-toolchain`, and the crate root has none
    // in its ancestry.
    Command::new("lake")
        .arg("--version")
        .current_dir(lean_dir())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn build() -> &'static Result<PathBuf, String> {
    static BUILT: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    BUILT.get_or_init(|| {
        if !lake_available() {
            return Err(
                "lake (the Lean 4 build tool); install elan from \
                 https://github.com/leanprover/elan, and lean/lean-toolchain pins the version"
                    .to_string(),
            );
        }
        let out = Command::new("lake")
            .arg("build")
            .arg("oo-dlrefute")
            .current_dir(lean_dir())
            .output()
            .expect("run lake build oo-dlrefute");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if !out.status.success() {
            if text.contains("unknown target") || text.contains("no such target") {
                return Err(format!(
                    "the `oo-dlrefute` target in lean/lakefile.toml. Add:\n\
                     \n\
                     [[lean_exe]]\nname = \"oo-dlrefute\"\nroot = \"DlRefuteMain\"\n\
                     \n\
                     lake said: {text}"
                ));
            }
            panic!("lake build oo-dlrefute failed:\n{text}");
        }
        let exe = lean_dir()
            .join(".lake")
            .join("build")
            .join("bin")
            .join("oo-dlrefute");
        if !exe.exists() {
            panic!("checker binary missing at {}", exe.display());
        }
        Ok(exe)
    })
}

fn skip() -> bool {
    match build() {
        Ok(_) => false,
        Err(why) => common::skip_unless(
            false,
            why,
            "the refutation-certificate tests need both, and check nothing without them",
        ),
    }
}

fn checker() -> &'static Path {
    build().as_ref().expect("checked by skip()").as_path()
}

/// Run `oo-dlrefute` over a directory. Returns (exit code, stdout + stderr).
fn check_dir(dir: &Path) -> (i32, String) {
    let out = Command::new(checker())
        .arg(dir.join("axioms.tsv"))
        .arg(dir.join("refutation.cert"))
        .output()
        .expect("run oo-dlrefute");
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oo-dlrefute-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const PREFIXES: &str = r#"
    @prefix : <http://ex.org/> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
"#;

/// Emit a refutation for `class` and return the directory, failing loudly if the
/// emitter declined. Every caller here uses an ontology whose inconsistency is
/// inside the certified fragment, so a decline is a defect rather than the
/// documented refusal.
fn refutation(name: &str, tbox: &str, class: &str) -> PathBuf {
    let dir = scratch(name);
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{tbox}"), None).unwrap_or_else(|e| panic!("{e}"));
    let reasoner = DlReasoner::from_graph(&store).unwrap();
    let outcome = reasoner.certify_class_unsatisfiable(class, &dir).unwrap();
    assert!(
        outcome.is_certified(),
        "the emitter declined a class whose inconsistency is inside the certified \
         fragment: {}",
        outcome.describe()
    );
    dir
}

fn cert_text(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("refutation.cert")).unwrap()
}

// ── 1. The reasoner emits and the checker accepts ───────────────────────

/// Two disjoint superclasses. The smallest thing a TBox can get wrong, and the
/// derivation is a straight line: name a member, lift it to each superclass,
/// clash on the disjointness.
const DISJOINT_TBOX: &str = r#"
    :A a owl:Class . :B a owl:Class .
    :A owl:disjointWith :B .
    :Impossible a owl:Class ; rdfs:subClassOf :A , :B .
"#;

#[test]
fn a_disjointness_contradiction_is_certified_and_the_checker_accepts_it() {
    if skip() {
        return;
    }
    let dir = refutation("disjoint", DISJOINT_TBOX, "<http://ex.org/Impossible>");
    let (code, out) = check_dir(&dir);
    assert_eq!(code, 0, "the checker rejected a certificate the emitter wrote: {out}");
    assert!(out.contains("\"verdict\":\"unsatisfiable\""), "{out}");
    assert!(
        out.contains("Dl.unsatisfiable_of_check"),
        "the verdict must name the theorem it rests on: {out}"
    );
}

/// `≥2 R.⊤` against `≤1 R.⊤`. The bound clash needs NO witnesses on the branch,
/// so this closes without the ≥ rule ever running, and the certificate is four
/// steps rather than the seven the witness route costs.
const BOUND_TBOX: &str = r#"
    :sib a owl:ObjectProperty .
    :Twins a owl:Class ; rdfs:subClassOf
      [ a owl:Restriction ; owl:onProperty :sib ; owl:minCardinality 2 ] ,
      [ a owl:Restriction ; owl:onProperty :sib ; owl:maxCardinality 1 ] .
"#;

#[test]
fn a_bound_clash_closes_without_inventing_any_witnesses() {
    if skip() {
        return;
    }
    let dir = refutation("bound", BOUND_TBOX, "<http://ex.org/Twins>");
    let text = cert_text(&dir);
    assert!(
        text.contains("minmax "),
        "the two bounds contradict each other directly and the certificate should say so: {text}"
    );
    assert!(
        !text.contains("\nmin ") && !text.contains("  min "),
        "no witness should have been invented for a clash that needs none: {text}"
    );
    let (code, out) = check_dir(&dir);
    assert_eq!(code, 0, "{out}");
}

/// `≥2 R.C` against `≤1 R.D` with `C ⊑ D`. The bounds do NOT contradict each
/// other directly, because they are about different fillers. The refutation has
/// to invent two witnesses, lift each through the subsumption, and only then
/// find them violating the bound. This is the derivation the ≥ rule exists for,
/// and it is the one usually said to need merging. It does not: the bound enters
/// as a clash rather than as a rule.
const WITNESS_TBOX: &str = r#"
    :r a owl:ObjectProperty .
    :C a owl:Class . :D a owl:Class .
    :C rdfs:subClassOf :D .
    :Bad a owl:Class ; rdfs:subClassOf
      [ a owl:Restriction ; owl:onProperty :r ; owl:minQualifiedCardinality 2 ;
        owl:onClass :C ] ,
      [ a owl:Restriction ; owl:onProperty :r ; owl:maxQualifiedCardinality 1 ;
        owl:onClass :D ] .
"#;

#[test]
fn the_at_least_rule_invents_witnesses_that_a_later_bound_refutes() {
    if skip() {
        return;
    }
    let dir = refutation("witness", WITNESS_TBOX, "<http://ex.org/Bad>");
    let text = cert_text(&dir);
    assert!(
        text.contains("min ") && text.contains("maxclash "),
        "this one cannot be closed by the bound clash and must run the ≥ rule: {text}"
    );
    // Two distinct witnesses, or the cardinality clash would be counting one
    // individual twice. The `Nodup` side condition is what the checker enforces;
    // this pins that the emitter actually supplies two.
    let min_line = text
        .lines()
        .find(|l| l.trim_start().starts_with("min "))
        .expect("a ≥ step");
    let fields: Vec<&str> = min_line.split_whitespace().collect();
    assert_eq!(fields[3], "2", "the witness count is written before the names: {min_line}");
    assert_ne!(fields[4], fields[5], "the two witnesses must be different names: {min_line}");
    let (code, out) = check_dir(&dir);
    assert_eq!(code, 0, "{out}");
}

/// A disjunction under two disjointness axioms. Both branches have to close, and
/// this is the only rule that makes the certificate a tree.
const TREE_TBOX: &str = r#"
    :A a owl:Class . :B a owl:Class . :N a owl:Class .
    :A owl:disjointWith :N . :B owl:disjointWith :N .
    :Bad a owl:Class ; rdfs:subClassOf [ a owl:Class ; owl:unionOf ( :A :B ) ] , :N .
"#;

#[test]
fn a_disjunction_makes_a_tree_and_both_leaves_must_close() {
    if skip() {
        return;
    }
    let dir = refutation("tree", TREE_TBOX, "<http://ex.org/Bad>");
    let text = cert_text(&dir);
    assert!(text.contains("or "), "a disjunction should appear: {text}");
    assert_eq!(
        text.matches("disjoint ").count(),
        2,
        "one leaf per branch, and a tableau with one closed branch is not closed: {text}"
    );
    let (code, out) = check_dir(&dir);
    assert_eq!(code, 0, "{out}");
}

// ── 2. The checker can say no ───────────────────────────────────────────

/// A subsumption step citing an axiom nobody asserted. This is the laundering
/// move: a derivation that reads correctly, resting on a premise off the books.
#[test]
fn a_step_citing_an_axiom_the_ontology_lacks_is_rejected() {
    if skip() {
        return;
    }
    let dir = refutation("forge-axiom", DISJOINT_TBOX, "<http://ex.org/Impossible>");
    let forged = cert_text(&dir).replace("<http://ex.org/B>", "<http://ex.org/Invented>");
    std::fs::write(dir.join("refutation.cert"), forged).unwrap();
    let (code, out) = check_dir(&dir);
    assert_eq!(code, 1, "a forged premise must be rejected: {out}");
    assert!(
        out.contains("\"ok\":false"),
        "and the rejection must be legible: {out}"
    );
}

/// A witness reusing a name the branch already constrains. Freshness is the
/// whole of the ∃ and ≥ rules' soundness.
#[test]
fn a_witness_that_reuses_a_branch_name_is_rejected() {
    if skip() {
        return;
    }
    let dir = refutation("forge-fresh", WITNESS_TBOX, "<http://ex.org/Bad>");
    let text = cert_text(&dir);
    let min_line = text
        .lines()
        .find(|l| l.trim_start().starts_with("min "))
        .expect("a ≥ step");
    let fields: Vec<&str> = min_line.split_whitespace().collect();
    // Replace the second witness with the individual the ≥ step is about, which
    // is on the branch already.
    let (subject, second) = (fields[1], fields[5]);
    let forged = text.replace(second, subject);
    std::fs::write(dir.join("refutation.cert"), forged).unwrap();
    let (code, out) = check_dir(&dir);
    assert_eq!(code, 1, "a stale witness must be rejected: {out}");
}

/// A file the tree does not consume. A producer that truncated its derivation,
/// or wrote a second one, must not read as having finished the first.
#[test]
fn a_file_with_tokens_left_over_is_a_parse_error_not_a_verdict() {
    if skip() {
        return;
    }
    let dir = refutation("forge-trailing", DISJOINT_TBOX, "<http://ex.org/Impossible>");
    let mut text = cert_text(&dir);
    text.push_str("\nbot leftover\n");
    std::fs::write(dir.join("refutation.cert"), text).unwrap();
    let (code, out) = check_dir(&dir);
    assert_eq!(
        code, 2,
        "\"I could not read the file\" and \"this is not a refutation\" are different \
         answers and the exit codes must keep them apart: {out}"
    );
}

// ── 3. The emitter declines rather than inventing ───────────────────────

#[test]
fn a_satisfiable_class_gets_no_certificate_and_no_files() {
    if skip() {
        return;
    }
    let dir = scratch("satisfiable");
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(&format!(
            "{PREFIXES}:A a owl:Class . :Fine a owl:Class ; rdfs:subClassOf :A ."
        ), None)
        .unwrap();
    let reasoner = DlReasoner::from_graph(&store).unwrap();
    let outcome = reasoner
        .certify_class_unsatisfiable("<http://ex.org/Fine>", &dir)
        .unwrap();
    assert!(!outcome.is_certified(), "{}", outcome.describe());
    assert!(
        !dir.join("refutation.cert").exists(),
        "nothing should be written for an answer that was never negative"
    );
}

/// An inconsistency that lives entirely in transitivity, which the certified
/// calculus has no rule for. The emitter must decline AND say which rule it
/// lacked, because "no refutation exists" and "no refutation this calculus can
/// write" are different sentences.
#[test]
fn an_inconsistency_outside_the_calculus_is_declined_by_name() {
    if skip() {
        return;
    }
    let dir = scratch("trans");
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(&format!(
            "{PREFIXES}
            :r a owl:ObjectProperty , owl:TransitiveProperty .
            :C a owl:Class .
            :Bad a owl:Class ; rdfs:subClassOf
              [ a owl:Restriction ; owl:onProperty :r ; owl:someValuesFrom
                [ a owl:Restriction ; owl:onProperty :r ; owl:someValuesFrom :C ] ] ,
              [ a owl:Restriction ; owl:onProperty :r ; owl:allValuesFrom
                [ a owl:Class ; owl:complementOf :C ] ] .
            "
        ), None)
        .unwrap();
    let reasoner = DlReasoner::from_graph(&store).unwrap();
    let outcome = reasoner
        .certify_class_unsatisfiable("<http://ex.org/Bad>", &dir)
        .unwrap();
    assert!(!outcome.is_certified(), "{}", outcome.describe());
    assert!(
        outcome.describe().contains("trans"),
        "the decline has to name the rule it lacked: {}",
        outcome.describe()
    );
    assert!(
        !dir.join("refutation.cert").exists(),
        "and write nothing rather than something that will not check"
    );
}
