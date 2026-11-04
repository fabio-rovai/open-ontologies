// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Model certificates for the SHIQ tableaux reasoner, end to end.
//!
//! `src/tableaux.rs` answers satisfiability, classification and ABox
//! consistency, and until now every answer was a bare verdict. The positive
//! answers now carry a finite model, and `lean/Dl/` holds a checker for it whose
//! soundness is a machine-checked theorem: `Dl.satisfiable_of_checkModel` says
//! that an interpretation the checker accepts is a model, so the axiom set it
//! was checked against really is satisfiable. These tests close that loop.
//!
//!   1. The reasoner emits and the checker accepts. A class the reasoner says is
//!      satisfiable comes with a model of the TBox in which that class is not
//!      empty, and an ABox the reasoner says is consistent comes with a model of
//!      the TBox and the ABox together.
//!   2. The checker can say no, and each way it can say no is exercised on its
//!      own. A model with a required successor removed, a model that violates a
//!      disjointness, a model whose domain omits a named individual, and a model
//!      that satisfies every TBox axiom while leaving empty the very class the
//!      reasoner called satisfiable. Four separate forgeries, four separate
//!      runs, four separate rejections. A gate that cannot fail is decoration.
//!   3. The negative answers carry NOTHING, and the emitter says so rather than
//!      inventing a certificate. An unsatisfiable class produces `Refuted` and
//!      writes no file.
//!   4. The emitter writes nothing instead of emitting something that will not
//!      check. The case pinned here was found by this layer as a defect in the
//!      reasoner rather than in the certificate layer: an asserted edge skipped
//!      its `rdfs:range`, so the reasoner called an inconsistent ABox consistent
//!      and the emitter caught the discrepancy on the way out. The reasoner has
//!      since been fixed and the test now pins the repair.
//!
//! The checker needs a Lean toolchain (`lake`) AND the `oo-dlmodel` target in
//! `lean/lakefile.toml`. Without either these tests skip loudly through
//! `common::skip_unless`; the CI job that installs Lean runs them with
//! `OO_REQUIRE_FIXTURES=1`, which turns that skip into a failure.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::tableaux::{DlReasoner, ModelOutcome};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The Lean package to build the checker from. `OO_LEAN_DIR` exists so the
/// suite can be pointed at a copy of `lean/` while the `oo-dlmodel` target is
/// being landed in the real `lakefile.toml`; unset, which is the normal case,
/// it is the repository's own `lean/`.
fn lean_dir() -> PathBuf {
    std::env::var("OO_LEAN_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo().join("lean"))
}

fn lake_available() -> bool {
    // `.current_dir(lean_dir())` is not cosmetic: elan resolves the toolchain
    // from the working directory's `lean-toolchain`, and the crate root has none
    // in its ancestry. See the same probe in `tests/lean_certificate_test.rs`.
    Command::new("lake")
        .arg("--version")
        .current_dir(lean_dir())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// The build outcome, computed once per test binary. `Err` carries the reason,
/// so a missing toolchain and a missing lakefile target are distinguishable.
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
            .arg("oo-dlmodel")
            .current_dir(lean_dir())
            .output()
            .expect("run lake build oo-dlmodel");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if !out.status.success() {
            // A missing target is a lakefile that has not been updated yet, not
            // a broken proof. The two must not be reported as the same thing.
            if text.contains("unknown target") || text.contains("no such target") {
                return Err(format!(
                    "the `oo-dlmodel` target in lean/lakefile.toml. Add:\n\
                     \n\
                     [[lean_lib]]\nname = \"Dl\"\nroots = [\"Dl.All\"]\nglobs = [\"Dl.+\"]\n\
                     \n\
                     [[lean_exe]]\nname = \"oo-dlmodel\"\nroot = \"DlMain\"\n\
                     \n\
                     lake said: {text}"
                ));
            }
            panic!("lake build oo-dlmodel failed:\n{text}");
        }
        let exe = lean_dir()
            .join(".lake")
            .join("build")
            .join("bin")
            .join("oo-dlmodel");
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
            "the model-certificate tests need both, and check nothing without them",
        ),
    }
}

fn checker() -> &'static Path {
    build().as_ref().expect("checked by skip()").as_path()
}

/// Run `oo-dlmodel` over a directory. Returns (exit code, stdout + stderr).
fn check_dir(dir: &Path) -> (i32, String) {
    check_files(&dir.join("axioms.tsv"), &dir.join("model.tsv"))
}

fn check_files(axioms: &Path, model: &Path) -> (i32, String) {
    let out = Command::new(checker())
        .arg(axioms)
        .arg(model)
        .output()
        .expect("run oo-dlmodel");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), text)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oo-dlmodel-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn reasoner_for(ttl: &str) -> DlReasoner {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    DlReasoner::from_graph(&store).unwrap()
}

const PREFIXES: &str = r#"
    @prefix : <http://ex.org/> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
"#;

/// A TBox with an existential, a disjointness and a range. Small enough that the
/// expected model can be written out by hand, which is the point: a test whose
/// expected output nobody can predict is not a test.
const EMPLOYMENT_TBOX: &str = r#"
    :Person a owl:Class .
    :Company a owl:Class .
    :Person owl:disjointWith :Company .
    :worksFor a owl:ObjectProperty ; rdfs:range :Company .
    :Employed a owl:Class ;
        rdfs:subClassOf :Person ,
            [ a owl:Restriction ; owl:onProperty :worksFor ; owl:someValuesFrom :Company ] .
"#;

fn employment_certificate(name: &str) -> PathBuf {
    let dir = scratch(name);
    let reasoner = reasoner_for(&format!("{PREFIXES}{EMPLOYMENT_TBOX}"));
    let outcome = reasoner
        .certify_class_satisfiable("<http://ex.org/Employed>", &dir)
        .unwrap();
    assert!(
        outcome.is_certified(),
        "the emitter refused a class it reports satisfiable: {}",
        outcome.describe()
    );
    dir
}

/// Read `model.tsv`, apply an edit, write it back under a new name, and return
/// the path to the forged file. The axiom file is untouched, so the only thing
/// that can change the verdict is the interpretation.
fn forge(dir: &Path, name: &str, edit: impl Fn(Vec<String>) -> Vec<String>) -> PathBuf {
    let text = std::fs::read_to_string(dir.join("model.tsv")).unwrap();
    let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    let out = edit(lines);
    let path = dir.join(format!("model.{name}.tsv"));
    std::fs::write(&path, out.join("\n") + "\n").unwrap();
    path
}

// ── 1. The reasoner emits and the checker accepts ───────────────────────

#[test]
fn a_satisfiable_class_carries_a_model_the_checker_accepts() {
    if skip() {
        return;
    }
    let dir = employment_certificate("sat-class");

    let axioms = std::fs::read_to_string(dir.join("axioms.tsv")).unwrap();
    let model = std::fs::read_to_string(dir.join("model.tsv")).unwrap();

    // The claim the reasoner actually made has to be IN the axiom set, or the
    // certificate would only say "this ontology has some model", which is not
    // what was asked.
    assert!(
        axioms.contains("nonempty\tatom <http://ex.org/Employed>"),
        "the satisfiability claim is missing from the axiom file:\n{axioms}"
    );
    assert!(
        model.lines().any(|l| l.starts_with("domain\t")),
        "the model has no domain:\n{model}"
    );

    let (code, out) = check_dir(&dir);
    assert_eq!(code, 0, "oo-dlmodel rejected a genuine certificate: {out}");
    assert!(
        out.contains("\"theorem\":\"Dl.satisfiable_of_checkModel\""),
        "the accepted verdict does not name the theorem it stands for: {out}"
    );
}

#[test]
fn a_consistent_abox_carries_a_model_the_checker_accepts() {
    if skip() {
        return;
    }
    let dir = scratch("abox");
    let reasoner = reasoner_for(&format!(
        r#"{PREFIXES}{EMPLOYMENT_TBOX}
        :alice a owl:NamedIndividual, :Person, :Employed .
        :acme a owl:NamedIndividual, :Company .
        :alice :worksFor :acme .
        "#
    ));
    let outcome = reasoner.certify_abox_consistent(&dir).unwrap();
    assert!(
        outcome.is_certified(),
        "the emitter refused an ABox it reports consistent: {}",
        outcome.describe()
    );

    let axioms = std::fs::read_to_string(dir.join("axioms.tsv")).unwrap();
    assert!(
        axioms.contains("indiv\t<http://ex.org/alice>"),
        "a named individual is missing from the axiom file:\n{axioms}"
    );
    assert!(
        axioms.contains("rel\t<http://ex.org/alice>\t<http://ex.org/worksFor>\t<http://ex.org/acme>"),
        "the role assertion is missing from the axiom file:\n{axioms}"
    );

    let (code, out) = check_dir(&dir);
    assert_eq!(code, 0, "oo-dlmodel rejected a genuine ABox certificate: {out}");
}

// ── 2. Four forgeries, each rejected on its own ─────────────────────────

#[test]
fn forged_missing_successor_is_rejected() {
    if skip() {
        return;
    }
    let dir = employment_certificate("forge-successor");
    let forged = forge(&dir, "no-successor", |lines| {
        lines
            .into_iter()
            .filter(|l| !l.starts_with("edge\t"))
            .collect()
    });
    let (code, out) = check_files(&dir.join("axioms.tsv"), &forged);
    assert_eq!(code, 1, "a model with no successor at all was accepted: {out}");
    assert!(
        out.contains("\"kind\":\"sub\""),
        "the rejection does not blame the subclass axiom that needs the successor: {out}"
    );
}

#[test]
fn forged_disjointness_violation_is_rejected() {
    if skip() {
        return;
    }
    let dir = employment_certificate("forge-disjoint");
    // Put the person element into Company as well. Person and Company are
    // declared disjoint, so exactly one axiom can now fail.
    let forged = forge(&dir, "disjoint", |mut lines| {
        let person_elem = lines
            .iter()
            .find(|l| l.starts_with("class\t<http://ex.org/Person>\t"))
            .and_then(|l| l.split('\t').nth(2).map(|s| s.to_string()))
            .expect("the model puts something in Person");
        lines.push(format!("class\t<http://ex.org/Company>\t{person_elem}"));
        lines
    });
    let (code, out) = check_files(&dir.join("axioms.tsv"), &forged);
    assert_eq!(code, 1, "a model violating a disjointness was accepted: {out}");
    assert!(
        out.contains("\"kind\":\"disjoint\""),
        "the rejection does not blame the disjointness: {out}"
    );
}

#[test]
fn forged_domain_omitting_a_named_individual_is_rejected() {
    if skip() {
        return;
    }
    let dir = scratch("forge-individual");
    let reasoner = reasoner_for(&format!(
        r#"{PREFIXES}{EMPLOYMENT_TBOX}
        :alice a owl:NamedIndividual, :Person, :Employed .
        :acme a owl:NamedIndividual, :Company .
        :alice :worksFor :acme .
        "#
    ));
    assert!(reasoner.certify_abox_consistent(&dir).unwrap().is_certified());

    // Drop alice's element from the domain and leave everything else. The
    // individual now denotes something that is not in the interpretation at all.
    let forged = forge(&dir, "no-alice", |lines| {
        let alice_elem = lines
            .iter()
            .find(|l| l.starts_with("ind\t<http://ex.org/alice>\t"))
            .and_then(|l| l.split('\t').nth(2).map(|s| s.to_string()))
            .expect("alice has a denotation");
        lines
            .into_iter()
            .filter(|l| l.as_str() != format!("domain\t{alice_elem}"))
            .collect()
    });
    let (code, out) = check_files(&dir.join("axioms.tsv"), &forged);
    assert_eq!(code, 1, "a model whose domain omits a named individual was accepted: {out}");
    assert!(
        out.contains("\"wellformed\":false"),
        "the rejection does not blame well-formedness: {out}"
    );
}

#[test]
fn a_model_of_the_axioms_that_drops_the_claim_is_rejected() {
    if skip() {
        return;
    }
    let dir = employment_certificate("forge-claim");
    // This is the sharp one. Take the class the reasoner called satisfiable and
    // empty its extension. Every TBox axiom about it becomes vacuously true, so
    // the result IS a model of the ontology. It is not a model of the ANSWER,
    // and the `nonempty` line is what makes the difference visible.
    let forged = forge(&dir, "empty-claim", |lines| {
        lines
            .into_iter()
            .filter(|l| !l.starts_with("class\t<http://ex.org/Employed>\t"))
            .collect()
    });
    let (code, out) = check_files(&dir.join("axioms.tsv"), &forged);
    assert_eq!(
        code, 1,
        "a model of the ontology in which the class is empty was accepted as a certificate \
         that the class is satisfiable: {out}"
    );
    assert!(
        out.contains("\"kind\":\"nonempty\""),
        "the rejection does not blame the satisfiability claim, so the certificate is not \
         certifying the answer: {out}"
    );
}

// ── 3. The negative answers carry nothing ───────────────────────────────

/// No `skip()`: this checks only that the emitter writes nothing, which needs no
/// Lean toolchain. A suite that checks nothing at all without `lake` would let
/// the "we emit nothing for a negative answer" promise go untested everywhere
/// the toolchain is absent, and that is the promise most worth keeping.
#[test]
fn an_unsatisfiable_class_gets_no_certificate() {
    let dir = scratch("unsat");
    let reasoner = reasoner_for(&format!(
        r#"{PREFIXES}
        :A a owl:Class . :B a owl:Class .
        :A owl:disjointWith :B .
        :Impossible a owl:Class ; rdfs:subClassOf :A, :B .
        "#
    ));
    let outcome = reasoner
        .certify_class_satisfiable("<http://ex.org/Impossible>", &dir)
        .unwrap();
    assert_eq!(
        outcome,
        ModelOutcome::Refuted,
        "an unsatisfiable class should get no certificate, not a bad one"
    );
    assert!(
        !dir.join("axioms.tsv").exists() && !dir.join("model.tsv").exists(),
        "the emitter wrote files for an answer it cannot certify"
    );
}

// ── 4. The emitter writes nothing rather than something that will not check ──

/// The first thing this layer caught, and now the regression for its repair.
///
/// THIS TEST USED TO ENCODE THE BUG. `build_abox_tableau` wrote an asserted role
/// assertion straight into the edge map, and `create_successor` was the ONLY
/// place `rdfs:domain` and `rdfs:range` were consulted, so on an asserted edge
/// neither constraint ever reached the individuals. The assertion below read
/// `abox.consistent && !abox.undecided` and said in its own message that the
/// answer was wrong and that the test should be rewritten once the reasoner was
/// fixed. It has been: both edge-creating paths now go through
/// `Tableau::add_role_edge`, which applies domain and range wherever the edge
/// came from. The assertion is now on the CORRECT behaviour.
///
/// The ontology below is inconsistent and it takes three axioms to see why:
/// `worksFor` has range `Company`, `alice worksFor carol` forces carol into
/// `Company`, carol is asserted a `Person`, and `Person` and `Company` are
/// disjoint. `check_abox` must report it INCONSISTENT, and must report
/// `undecided: false`, because it really did find the clash rather than run out
/// of budget. A false `consistent` here is the failure that matters: an ontology
/// with a contradiction in it passing.
///
/// The certificate layer's half of this test is unchanged in meaning and only in
/// verdict. It never certifies something that will not check, and an ABox with no
/// model has nothing to certify, so it writes no file. What changed is which
/// negative it returns: it used to hand back `Refused`, because the reasoner
/// claimed a completion graph that was not in fact a model of the range axiom and
/// the emitter caught the discrepancy on the way out. Now the reasoner finds the
/// clash itself, so the emitter gets no completion graph at all and returns
/// `Refuted`, the same answer it gives for any unsatisfiable input. Either way
/// the gate holds: nothing is written.
#[test]
fn an_asserted_edge_applies_its_range_and_the_emitter_certifies_nothing() {
    let dir = scratch("range-gap");
    let reasoner = reasoner_for(&format!(
        r#"{PREFIXES}
        :Person a owl:Class .
        :Company a owl:Class .
        :Person owl:disjointWith :Company .
        :worksFor a owl:ObjectProperty ; rdfs:range :Company .
        :alice a owl:NamedIndividual, :Person .
        :carol a owl:NamedIndividual, :Person .
        :alice :worksFor :carol .
        "#
    ));

    // The reasoner's own answer, and it is now right.
    let abox = reasoner.check_abox();
    assert!(
        !abox.consistent,
        "the asserted edge forces carol into Company, which is disjoint from the \
         Person she is asserted to be; reporting this ABox consistent is a false clean"
    );
    assert!(
        !abox.undecided,
        "the clash is reachable well inside the budget, so this must be a proof \
         and not a hedge"
    );

    let outcome = reasoner.certify_abox_consistent(&dir).unwrap();
    match &outcome {
        ModelOutcome::Refuted => {}
        other => panic!(
            "an ABox with no model must be refuted and carry no certificate, got {}",
            other.describe()
        ),
    }
    assert!(
        !dir.join("axioms.tsv").exists() && !dir.join("model.tsv").exists(),
        "a negative answer must write nothing"
    );
}

// ── 5. Qualified number restrictions count DISTINCT successors ──────────

fn fixture(name: &str) -> PathBuf {
    repo().join("tests").join("fixtures").join("dlmodel").join(name)
}

/// `≥2 R.C` needs two different successors, and a hand-written certificate that
/// lists one successor twice must not pass. `lean/Dl/Count.lean` is where that
/// is proved, through a pigeonhole lemma; this runs it through the executable.
///
/// The fixtures are written by hand rather than emitted, because the reasoner's
/// own completion graphs never produce a duplicated edge and so could not
/// exercise the case at all.
#[test]
fn qualified_number_restrictions_count_distinct_successors() {
    if skip() {
        return;
    }
    let axioms = fixture("qnr_axioms.tsv");

    let (code, out) = check_files(&axioms, &fixture("qnr_model.tsv"));
    assert_eq!(code, 0, "a genuine two-member committee was rejected: {out}");

    let (code, out) = check_files(&axioms, &fixture("qnr_model_duplicate.tsv"));
    assert_eq!(
        code, 1,
        "one member listed twice was accepted as two members: {out}"
    );
    assert!(
        out.contains("min 2"),
        "the rejection does not blame the minimum cardinality: {out}"
    );
}

/// A file the parser cannot read is exit 2, which is neither "this is a model"
/// nor "this is not a model". A harness that collapsed the three would report an
/// unreadable certificate as a refuted one.
#[test]
fn an_unreadable_file_is_neither_verdict() {
    if skip() {
        return;
    }
    let (code, _) = check_files(&fixture("malformed_axioms.tsv"), &fixture("qnr_model.tsv"));
    assert_eq!(code, 2, "a malformed axiom file should be exit 2");

    let (code, _) = check_files(&fixture("qnr_axioms.tsv"), Path::new("/nonexistent/model.tsv"));
    assert_eq!(code, 2, "a missing model file should be exit 2");
}

// ── 6. A real ontology, class by class ──────────────────────────────────

/// The hand-written fixtures above prove the machinery works. This proves it
/// works on something nobody wrote for it.
///
/// Every named class of a shipped ontology is put through the reasoner, and
/// every class it reports satisfiable has its model checked by `oo-dlmodel`. The
/// interesting number is the disagreements, which must be zero: the emitter's
/// own gate and the verified checker have to agree on every single one, or one
/// of the two is wrong.
#[test]
fn a_shipped_ontology_certifies_class_by_class() {
    if skip() {
        return;
    }
    let onto = repo().join("benchmark/generated/boro-building-ai.ttl");
    if common::skip_unless(
        onto.exists(),
        "benchmark/generated/boro-building-ai.ttl",
        "it ships with the repository; a checkout without benchmark/ cannot run this",
    ) {
        return;
    }

    let store = Arc::new(GraphStore::new());
    store.load_file(onto.to_str().unwrap()).unwrap();
    let reasoner = DlReasoner::from_graph(&store).unwrap();

    let mut classes: Vec<String> = store
        .all_triples()
        .unwrap()
        .into_iter()
        .filter(|(_, p, o)| {
            (p == "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
                && (o == "<http://www.w3.org/2002/07/owl#Class>"
                    || o == "<http://www.w3.org/2000/01/rdf-schema#Class>"))
                || p == "<http://www.w3.org/2000/01/rdf-schema#subClassOf>"
        })
        .map(|(s, _, _)| s)
        .filter(|s| s.starts_with('<'))
        .collect();
    classes.sort();
    classes.dedup();
    assert!(
        !classes.is_empty(),
        "no named classes found, so this test would pass without checking anything"
    );

    let dir = scratch("shipped");
    let mut certified = 0usize;
    let mut accepted = 0usize;
    let mut disagreements: Vec<String> = Vec::new();
    for (i, class) in classes.iter().enumerate() {
        let d = dir.join(i.to_string());
        if let ModelOutcome::Certified { .. } = reasoner
            .certify_class_satisfiable(class, &d)
            .unwrap_or_else(|e| panic!("certifying {class}: {e}"))
        {
            certified += 1;
            let (code, out) = check_dir(&d);
            if code == 0 {
                accepted += 1;
            } else {
                disagreements.push(format!("{class}: {out}"));
            }
        }
    }

    assert!(
        disagreements.is_empty(),
        "the emitter and the verified checker disagreed on {} of {certified} certificates:\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
    assert!(
        certified > 0,
        "no class of a consistent ontology was certified, so nothing was checked"
    );
    assert_eq!(certified, accepted);
}

/// An ontology using constructs the certificate layer does not model gets a
/// perfectly valid certificate about a WEAKER axiom set than it states, and the
/// report has to say so next to the verdict rather than in a source comment.
/// A reader who is not told will draw a stronger conclusion than the proof
/// supports, which is the failure this whole layer exists to avoid.
#[test]
fn a_certificate_over_a_weaker_axiom_set_says_so() {
    let dir = std::env::temp_dir().join(format!("oo-dl-weaker-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let ttl = r#"
        @prefix : <http://ex.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        :Person a owl:Class .
        :alice a :Person .
        :alice owl:sameAs :alicia .
        :grandparentOf owl:propertyChainAxiom ( :parentOf :parentOf ) .
    "#;
    let store = std::sync::Arc::new(open_ontologies::graph::GraphStore::new());
    store.load_turtle(ttl, None).unwrap();

    let dropped = open_ontologies::tableaux::DlReasoner::unmodelled_constructs(&store);
    let names: Vec<&str> = dropped.iter().map(|(k, _)| k.as_str()).collect();
    assert!(
        names.contains(&"owl:sameAs") && names.contains(&"owl:propertyChainAxiom"),
        "both unmodelled constructs must be named, got {names:?}"
    );

    // And the other direction, so the disclosure cannot be a constant.
    let plain = std::sync::Arc::new(open_ontologies::graph::GraphStore::new());
    plain
        .load_turtle("@prefix : <http://ex.org/> . :Person a <http://www.w3.org/2002/07/owl#Class> .", None)
        .unwrap();
    assert!(
        open_ontologies::tableaux::DlReasoner::unmodelled_constructs(&plain).is_empty(),
        "an ontology using nothing unmodelled must report nothing dropped"
    );
}
