//! The ten OWL 2 RL rules added on the `owl-rl-completeness` branch.
//!
//! `tests/lean_certificate_test.rs` already proves that everything the engine
//! emits is accepted by the checker, over the whole shipped corpus. That is a
//! statement about soundness and it says nothing about whether a rule ever
//! fires, whether it fires the right way round, or whether what it adds is
//! worth having. This file covers those three.
//!
//! # What each section is for
//!
//! * **Coverage.** Each of the ten rules fires at least once on an ontology
//!   built for it, and the resulting certificate is accepted. A rule that never
//!   fires is a rule nobody has tested.
//! * **Direction.** `scm-avf2` concludes `?c2 rdfs:subClassOf ?c1` where its
//!   three siblings conclude `?c1 rdfs:subClassOf ?c2`, because a universal
//!   restriction is antitone in its property. The engine must derive the one
//!   and NOT the other, and the checker must reject a forged step that runs the
//!   other way. `OOCert.the_natural_avf2_direction_is_not_entailed` is the same
//!   fact proved against the semantics; this is the same fact observed in the
//!   engine.
//! * **Worth.** The four domain-and-range rules fire on schema alone, so they
//!   fire on every corpus. What they add is completeness of the EMITTED SCHEMA
//!   and not new answers about individuals, because rdfs2 followed by rdfs9
//!   already reached the instance-level consequence. That is a limitation of
//!   the rules rather than a feature, and
//!   `the_domain_rules_add_schema_and_not_answers` is where it is written down
//!   in a form that fails if it stops being true.
//!
//! # These gates were seen to fail
//!
//! A gate nobody has watched fail is decoration, so each of these was broken on
//! purpose and the failure recorded.
//!
//! Changing one character in `src/reason.rs`, `emit((r2, rdfs_subclass, r1),
//! "scm-avf2", ...)` to `emit((r1, rdfs_subclass, r2), ...)`, produced:
//!
//! ```text
//! test scm_avf2_concludes_the_subsumption_the_other_way_round ... FAILED
//!   scm-avf2 must derive `all hasComponent Wheel` subClassOf `all hasPart Wheel`
//! test every_new_rule_fires_and_the_certificate_is_accepted ... FAILED
//!   {"ok":false,"first_rejected":7,"rule":"scm-avf2",
//!    "conclusion":"<http://ex.org/AvfSub> <rdfs:subClassOf> <http://ex.org/AvfSup>"}
//! ```
//!
//! so the engine test and the proved checker both refused it, independently.
//! Flipping the matching Lean condition instead, `avf_sp`'s conclusion from
//! `I.sc c2 c1` to `I.sc c1 c2`, fails the Lean build in three places at once:
//! the `scmAvf2` case of `checkStep_sound`, the model in
//! `OOCert.avf_witness_is_a_model`, and the two `#print axioms` guards that
//! depend on it. The direction is written down in three places and they have to
//! agree.
//!
//! The forged lines in `the_checker_rejects_a_forged_step_for_each_new_rule`
//! are appended one at a time to a certificate that is checked and accepted
//! first, so a rejection there cannot be the checker refusing everything.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{InferenceTarget, Reasoner};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn lean_dir() -> PathBuf {
    repo().join("lean")
}

fn lake_available() -> bool {
    // `.current_dir(lean_dir())`: elan resolves the toolchain from the working
    // directory and the crate root has no `lean-toolchain` in its ancestry.
    Command::new("lake")
        .arg("--version")
        .current_dir(lean_dir())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn skip() -> bool {
    common::skip_unless(
        lake_available(),
        "lake (the Lean 4 build tool)",
        "install elan from https://github.com/leanprover/elan; lean/lean-toolchain pins the version",
    )
}

fn checker() -> &'static Path {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        let out = Command::new("lake")
            .arg("build")
            .current_dir(lean_dir())
            .output()
            .expect("run lake build");
        assert!(
            out.status.success(),
            "lake build failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let exe = lean_dir().join(".lake").join("build").join("bin").join("oo-cert");
        assert!(exe.exists(), "checker binary missing at {}", exe.display());
        exe
    })
}

fn check(dir: &Path) -> (bool, String) {
    let out = Command::new(checker())
        .arg(dir.join("asserted.tsv"))
        .arg(dir.join("derivations.tsv"))
        .output()
        .expect("run oo-cert");
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oo-rl-cov-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(ttl: &str, profile: &str, dir: Option<&Path>) -> (Arc<GraphStore>, serde_json::Value) {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    let out =
        Reasoner::run_full(&store, profile, true, InferenceTarget::DefaultGraph, dir).unwrap();
    (store, serde_json::from_str(&out).unwrap())
}

/// `a` is SPARQL's own keyword for `rdf:type`, but `rdfs:subClassOf` and the
/// rest are not built in, so the prefix has to travel with the query. Without
/// it every call here is a parse error rather than a false answer, which is the
/// safe failure but still a failure.
fn holds(store: &Arc<GraphStore>, s: &str, p: &str, o: &str) -> bool {
    store
        .sparql_select(&format!(
            "PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> \
             PREFIX owl: <http://www.w3.org/2002/07/owl#> ASK {{ {s} {p} {o} }}"
        ))
        .unwrap()
        .contains("true")
}

const PREFIXES: &str = r#"
    @prefix : <http://ex.org/> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
"#;

/// One ontology that fires all ten of the new rules and nothing has to be
/// contrived: two restriction pairs ordered by filler and by property, a
/// property hierarchy carrying a domain and a range, an intersection with an
/// instance, and an enumeration.
const NEW_RULES: &str = r#"
    # scm-svf1 / scm-avf1: same property, fillers ordered
    :SvfNarrow a owl:Restriction ; owl:onProperty :eats ; owl:someValuesFrom :Apple .
    :SvfWide   a owl:Restriction ; owl:onProperty :eats ; owl:someValuesFrom :Fruit .
    :AvfNarrow a owl:Restriction ; owl:onProperty :eats ; owl:allValuesFrom :Apple .
    :AvfWide   a owl:Restriction ; owl:onProperty :eats ; owl:allValuesFrom :Fruit .
    :Apple rdfs:subClassOf :Fruit .

    # scm-svf2 / scm-avf2: same filler, properties ordered
    :SvfSub a owl:Restriction ; owl:onProperty :hasPart ; owl:someValuesFrom :Wheel .
    :SvfSup a owl:Restriction ; owl:onProperty :hasComponent ; owl:someValuesFrom :Wheel .
    :AvfSub a owl:Restriction ; owl:onProperty :hasPart ; owl:allValuesFrom :Wheel .
    :AvfSup a owl:Restriction ; owl:onProperty :hasComponent ; owl:allValuesFrom :Wheel .
    :hasPart rdfs:subPropertyOf :hasComponent .

    # scm-dom1 / scm-dom2 / scm-rng1 / scm-rng2
    :hasComponent rdfs:domain :Assembly ; rdfs:range :Component .
    :Assembly rdfs:subClassOf :Artifact .
    :Component rdfs:subClassOf :Thing .

    # cls-int1 and cls-int2
    :Supplier owl:intersectionOf ( :Vendor :Approved ) .
    :acme a :Supplier .
    :both a :Vendor , :Approved .

    # cls-uni and cls-oo
    :Either owl:unionOf ( :Vendor :Approved ) .
    :Weekend owl:oneOf ( :saturday :sunday ) .
"#;

const NEW_RULE_IDS: [&str; 10] = [
    "scm-svf1", "scm-svf2", "scm-avf1", "scm-avf2", "scm-dom1", "scm-dom2", "scm-rng1",
    "scm-rng2", "cls-int2", "cls-oo",
];

#[test]
fn every_new_rule_fires_and_the_certificate_is_accepted() {
    if skip() {
        return;
    }
    let dir = scratch("new-rules");
    let (_, r) = run(&format!("{PREFIXES}{NEW_RULES}"), "owl-rl-ext", Some(&dir));
    let by_rule = r["certificate"]["by_rule"]
        .as_object()
        .expect("by_rule in the response");
    let missing: Vec<&str> = NEW_RULE_IDS
        .iter()
        .copied()
        .filter(|k| !by_rule.contains_key(*k))
        .collect();
    assert!(
        missing.is_empty(),
        "rules that never fired on the ontology built to fire them: {missing:?}\n{r}"
    );
    assert_eq!(
        r["certificate"]["derivations"], r["inferred_count"],
        "one certificate line per inferred triple: {r}"
    );
    let (ok, out) = check(&dir);
    assert!(ok, "the checker rejected the new-rules certificate:\n{out}");
}

/// The trap. Three restriction-ordering rules conclude `c1 subClassOf c2` and
/// this one concludes `c2 subClassOf c1`, so the engine must derive the
/// subsumption in the direction the W3C table gives and must NOT derive the
/// other. Flipping the conclusion in `src/reason.rs` fails the second
/// assertion here, and it fails the Lean build as well.
#[test]
fn scm_avf2_concludes_the_subsumption_the_other_way_round() {
    let ttl = format!(
        "{PREFIXES}
        :AvfSub a owl:Restriction ; owl:onProperty :hasPart ; owl:allValuesFrom :Wheel .
        :AvfSup a owl:Restriction ; owl:onProperty :hasComponent ; owl:allValuesFrom :Wheel .
        :hasPart rdfs:subPropertyOf :hasComponent ."
    );
    let (store, _) = run(&ttl, "owl-rl-ext", None);
    assert!(
        holds(&store, "<http://ex.org/AvfSup>", "rdfs:subClassOf", "<http://ex.org/AvfSub>"),
        "scm-avf2 must derive `all hasComponent Wheel` subClassOf `all hasPart Wheel`: a \
         universal restriction is antitone in its property"
    );
    assert!(
        !holds(&store, "<http://ex.org/AvfSub>", "rdfs:subClassOf", "<http://ex.org/AvfSup>"),
        "the reverse subsumption is NOT entailed and must not be derived; it is the direction \
         scm-svf2, scm-svf1 and scm-avf1 run in, and copying them here is unsound. See \
         OOCert.the_natural_avf2_direction_is_not_entailed"
    );
}

/// And the existential pair runs the way its siblings do, so the test above is
/// pinning a real asymmetry rather than a typo in one direction.
#[test]
fn scm_svf2_concludes_the_subsumption_the_usual_way_round() {
    let ttl = format!(
        "{PREFIXES}
        :SvfSub a owl:Restriction ; owl:onProperty :hasPart ; owl:someValuesFrom :Wheel .
        :SvfSup a owl:Restriction ; owl:onProperty :hasComponent ; owl:someValuesFrom :Wheel .
        :hasPart rdfs:subPropertyOf :hasComponent ."
    );
    let (store, _) = run(&ttl, "owl-rl-ext", None);
    assert!(
        holds(&store, "<http://ex.org/SvfSub>", "rdfs:subClassOf", "<http://ex.org/SvfSup>"),
        "scm-svf2 derives `some hasPart Wheel` subClassOf `some hasComponent Wheel`"
    );
    assert!(
        !holds(&store, "<http://ex.org/SvfSup>", "rdfs:subClassOf", "<http://ex.org/SvfSub>"),
        "and not the reverse"
    );
}

/// `cls-int2` takes an intersection apart, and `cls-int1` still needs every
/// member. The second half is what stops the new rule from being read as
/// "membership in one member is enough".
#[test]
fn cls_int2_takes_the_intersection_apart_without_weakening_cls_int1() {
    let ttl = format!(
        "{PREFIXES}
        :Supplier owl:intersectionOf ( :Vendor :Approved ) .
        :acme a :Supplier .
        :vee a :Vendor ."
    );
    let (store, _) = run(&ttl, "owl-rl-ext", None);
    assert!(
        holds(&store, "<http://ex.org/acme>", "a", "<http://ex.org/Vendor>")
            && holds(&store, "<http://ex.org/acme>", "a", "<http://ex.org/Approved>"),
        "cls-int2: a Supplier is a Vendor and an Approved"
    );
    assert!(
        !holds(&store, "<http://ex.org/vee>", "a", "<http://ex.org/Supplier>"),
        "cls-int1 still needs EVERY member; vee is only a Vendor. See \
         OOCert.membership_in_one_member_does_not_give_the_intersection"
    );
}

/// `cls-oo` types the members the list names and stops there. The condition it
/// rests on is the "at least" half of the enumeration's meaning only.
#[test]
fn cls_oo_types_the_listed_members_and_no_one_else() {
    let ttl = format!(
        "{PREFIXES}
        :Weekend owl:oneOf ( :saturday :sunday ) .
        :monday a :Day ."
    );
    let (store, _) = run(&ttl, "owl-rl-ext", None);
    assert!(
        holds(&store, "<http://ex.org/saturday>", "a", "<http://ex.org/Weekend>")
            && holds(&store, "<http://ex.org/sunday>", "a", "<http://ex.org/Weekend>"),
        "cls-oo types both listed members"
    );
    assert!(
        !holds(&store, "<http://ex.org/monday>", "a", "<http://ex.org/Weekend>"),
        "an individual the list never mentions is not entailed to be a member. See \
         OOCert.an_unlisted_individual_is_not_entailed"
    );
}

/// The honest limit of the four domain-and-range rules, written so that it
/// fails if it stops being true.
///
/// They fire on schema alone and they emit schema. The instance-level
/// consequence was already reachable without them: rdfs2 gives `x rdf:type c1`
/// and rdfs9 gives `x rdf:type c2` from `c1 rdfs:subClassOf c2`, which is why
/// the `rdfs` profile, which has none of these four rules, reaches the same
/// answer about `x`. What `owl-rl` adds is the axiom `p rdfs:domain c2`.
#[test]
fn the_domain_rules_add_schema_and_not_answers() {
    let ttl = format!(
        "{PREFIXES}
        :p rdfs:domain :Narrow ; rdfs:range :NarrowR .
        :Narrow rdfs:subClassOf :Wide .
        :NarrowR rdfs:subClassOf :WideR .
        :x :p :y ."
    );
    let (rdfs_store, _) = run(&ttl, "rdfs", None);
    let (owl_store, _) = run(&ttl, "owl-rl", None);

    // The answer about the individual is the same in both profiles.
    for store in [&rdfs_store, &owl_store] {
        assert!(
            holds(store, "<http://ex.org/x>", "a", "<http://ex.org/Wide>"),
            "rdfs2 then rdfs9 already reaches this, with or without scm-dom1"
        );
        assert!(
            holds(store, "<http://ex.org/y>", "a", "<http://ex.org/WideR>"),
            "and the same on the range side"
        );
    }

    // The schema axiom is what is new.
    assert!(
        !holds(&rdfs_store, "<http://ex.org/p>", "rdfs:domain", "<http://ex.org/Wide>"),
        "the rdfs profile has no scm-dom1, so it must NOT emit the weakened domain; if it does, \
         this test is measuring nothing"
    );
    assert!(
        holds(&owl_store, "<http://ex.org/p>", "rdfs:domain", "<http://ex.org/Wide>"),
        "scm-dom1 emits the weakened domain axiom"
    );
    assert!(
        holds(&owl_store, "<http://ex.org/p>", "rdfs:range", "<http://ex.org/WideR>"),
        "scm-rng1 emits the weakened range axiom"
    );
}

/// A gate that cannot fail is decoration. One forged line per new rule, each
/// appended on its own to a certificate that passes without it, and each
/// rejected.
///
/// The `scm-avf2` entry is the one worth reading: the premises are all
/// asserted, the rule id is real, and the ONLY thing wrong with it is that the
/// conclusion runs the way its three siblings run.
#[test]
fn the_checker_rejects_a_forged_step_for_each_new_rule() {
    if skip() {
        return;
    }
    const TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
    const SC: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
    const SP: &str = "<http://www.w3.org/2000/01/rdf-schema#subPropertyOf>";
    const DOM: &str = "<http://www.w3.org/2000/01/rdf-schema#domain>";
    const AVF: &str = "<http://www.w3.org/2002/07/owl#allValuesFrom>";
    const OP: &str = "<http://www.w3.org/2002/07/owl#onProperty>";
    const INT: &str = "<http://www.w3.org/2002/07/owl#intersectionOf>";
    const ONEOF: &str = "<http://www.w3.org/2002/07/owl#oneOf>";
    let e = |s: &str| format!("<http://ex.org/{s}>");

    let forgeries: Vec<(&str, String)> = vec![
        (
            "scm-avf2 with the conclusion the way its three siblings run",
            [
                "scm-avf2".to_string(),
                e("AvfSub"), SC.into(), e("AvfSup"),
                e("AvfSub"), AVF.into(), e("Wheel"),
                e("AvfSub"), OP.into(), e("hasPart"),
                e("AvfSup"), AVF.into(), e("Wheel"),
                e("AvfSup"), OP.into(), e("hasComponent"),
                e("hasPart"), SP.into(), e("hasComponent"),
            ]
            .join("\t"),
        ),
        (
            "scm-dom1 weakening a domain along a subClassOf nobody asserted",
            [
                "scm-dom1".to_string(),
                e("hasComponent"), DOM.into(), e("Ghost"),
                e("hasComponent"), DOM.into(), e("Assembly"),
                e("Assembly"), SC.into(), e("Ghost"),
            ]
            .join("\t"),
        ),
        (
            "cls-int2 concluding a class that is not a member of the list",
            [
                "cls-int2".to_string(),
                e("acme"), TYPE.into(), e("Ghost"),
                e("Supplier"), INT.into(), "_:b0".into(),
                e("acme"), TYPE.into(), e("Supplier"),
            ]
            .join("\t"),
        ),
        (
            "cls-oo typing an individual the enumeration never lists",
            [
                "cls-oo".to_string(),
                e("monday"), TYPE.into(), e("Weekend"),
                e("Weekend"), ONEOF.into(), "_:b1".into(),
            ]
            .join("\t"),
        ),
    ];

    for (why, line) in forgeries {
        let dir = scratch("forged");
        run(&format!("{PREFIXES}{NEW_RULES}"), "owl-rl-ext", Some(&dir));
        let (ok, out) = check(&dir);
        assert!(ok, "the honest certificate must pass first: {out}");

        let path = dir.join("derivations.tsv");
        let mut content = std::fs::read_to_string(&path).unwrap();
        content.push_str(&line);
        content.push('\n');
        std::fs::write(&path, content).unwrap();

        let (ok, out) = check(&dir);
        assert!(!ok, "a forged step must be rejected ({why}):\n{out}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// What the ten rules actually do to the corpus this repository ships.
///
/// The point is the RANKING and not the total. Most of the profile's missing
/// rules fire zero times here because the corpus is schema-heavy and
/// instance-light, so a rule count is a bad way to choose what to implement and
/// a firing count is a good one. This test prints the per-rule totals and
/// asserts the two claims the choice rested on: the schema rules fire, and they
/// fire on files where the instance rules find nothing.
///
/// Run with `-- --nocapture` to read the table.
#[test]
fn the_new_rules_earn_their_place_on_the_shipped_corpus() {
    if skip() {
        return;
    }
    const SIZE_CAP: u64 = 4 * 1024 * 1024;
    const SKIP_DIRS: [&str; 8] = [
        "node_modules", "target", ".lake", ".git", ".venv", "venv", "site-packages", "__pycache__",
    ];
    // `tests/fixtures/horn-coverage/` holds one graph per rule the shipped corpus
    // never fires, each written to make exactly that rule fire, so the Rust/Python
    // differential can compare the two engines on rules no real ontology reaches. They
    // are EXCLUDED here, and the exclusion is the whole point of the directory: this
    // test asks whether the ten new rules earn their place on the corpus this
    // repository SHIPS, and a rule that earns its place on a file written to make it
    // fire has earned nothing. Without this line the silent list at the bottom would
    // empty itself out of its own test data. `tools/horn_differential.py` excludes the
    // same directory for the same reason and reports it as a separate figure.
    const COVERAGE_FIXTURES: &str = "tests/fixtures/horn-coverage/";
    let coverage_dir = repo().join("tests").join("fixtures").join("horn-coverage");
    assert!(
        coverage_dir.is_dir(),
        "{} is missing. If the directory moved, the exclusion below is a no-op and this \
         test is now measuring data written to make its own assertions pass",
        coverage_dir.display()
    );

    let out = Command::new("git")
        .args(["ls-files", "-z", "*.ttl", "*.owl", "*.rdf", "*.nt"])
        .current_dir(repo())
        .output()
        .expect("run git ls-files");
    let tracked: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect();
    let excluded = tracked.iter().filter(|p| p.starts_with(COVERAGE_FIXTURES)).count();
    assert!(
        excluded > 0,
        "no tracked file under {COVERAGE_FIXTURES} was excluded. Either the fixtures are \
         untracked, in which case `git ls-files` never saw them and this guard is fine to \
         relax, or the prefix has drifted and they are being counted as corpus"
    );
    let mut files: Vec<PathBuf> = tracked
        .iter()
        .filter(|p| !p.starts_with(COVERAGE_FIXTURES))
        // `tests/fixtures/clash-coverage/` holds one contradictory graph per clash
        // detector, each written to make exactly that detector fire (#160), plus a
        // near-miss twin. They are test data and not ontologies: swept as corpus they
        // would turn a measurement of the shipped corpus into a statement about this
        // repository's own fixtures, which is measuring the ruler. The same reasoning
        // and the same treatment as `tests/fixtures/horn-coverage/`.
        // `tests/clash_detector_coverage_test.rs` asserts this exclusion is still here.
        .filter(|p| !p.starts_with("tests/fixtures/clash-coverage/"))
        .filter(|p| !SKIP_DIRS.iter().any(|d| p.split('/').any(|seg| seg == *d)))
        .map(|p| repo().join(p))
        .collect();
    files.sort();
    assert!(files.len() >= 120, "expected the whole corpus, found {}", files.len());

    let mut totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut files_with_new_rules = 0usize;
    let mut files_schema_only = 0usize;
    let mut reasoned = 0usize;

    for path in &files {
        if std::fs::metadata(path).unwrap().len() > SIZE_CAP {
            continue;
        }
        let store = Arc::new(GraphStore::new());
        if store.load_file(&path.display().to_string()).is_err() {
            continue;
        }
        reasoned += 1;
        let dir = scratch("corpus");
        let Ok(out) =
            Reasoner::run_full(&store, "owl-rl-ext", false, InferenceTarget::DefaultGraph, Some(&dir))
        else {
            let _ = std::fs::remove_dir_all(&dir);
            continue;
        };
        let r: serde_json::Value = serde_json::from_str(&out).unwrap();
        let mut new_here = 0u64;
        let mut old_here = 0u64;
        if let Some(by_rule) = r["certificate"]["by_rule"].as_object() {
            for (rule, n) in by_rule {
                let n = n.as_u64().unwrap_or(0);
                *totals.entry(rule.clone()).or_default() += n;
                if NEW_RULE_IDS.contains(&rule.as_str()) {
                    new_here += n;
                } else {
                    old_here += n;
                }
            }
        }
        if new_here > 0 {
            files_with_new_rules += 1;
            if old_here == 0 {
                files_schema_only += 1;
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    eprintln!(
        "--- firings across {reasoned} corpus files that parse and are under the size cap, \
         out of {} tracked ---",
        files.len()
    );
    for (rule, n) in &totals {
        let tag = if NEW_RULE_IDS.contains(&rule.as_str()) { "NEW" } else { "   " };
        eprintln!("{tag} {rule:>10}  {n}");
    }
    let new_total: u64 = NEW_RULE_IDS
        .iter()
        .map(|r| totals.get(*r).copied().unwrap_or(0))
        .sum();
    eprintln!(
        "new-rule firings {new_total}, on {files_with_new_rules} files, \
         of which {files_schema_only} derived NOTHING from the older rules"
    );

    assert!(
        new_total > 0,
        "the ten rules fired zero times on the whole corpus, so nothing here was measured"
    );
    assert!(
        totals.get("scm-dom1").copied().unwrap_or(0) > 0
            || totals.get("scm-rng1").copied().unwrap_or(0) > 0,
        "the domain-and-range rules fire on schema alone, so a schema-heavy corpus must trigger \
         them; if this fails the gap analysis that chose them was wrong: {totals:?}"
    );

    // The half of the measurement that is bad news, kept here rather than in a
    // note nobody reads. Three of the ten fire ZERO times on everything this
    // repository tracks. They are implemented because they are part of the
    // profile and because the fourth member of their family is the single
    // biggest contributor below, not because a file here needs them, and the
    // honest way to say so is an assertion that fails the day it stops being
    // true.
    //
    // `tests/fixtures/horn-coverage/scm-svf2.ttl` and `scm-avf2.ttl` DO fire two of
    // these three, and are excluded from the sweep above precisely so that they cannot
    // shorten this list. A graph written to make a rule fire is evidence that the rule
    // is implemented, which is what that directory is for; it is not evidence that any
    // ontology needs the rule, which is what this list is about.
    //
    // `scm-avf1` has NO fixture, and the reason is a discrepancy rather than an
    // oversight. This sweep runs the `owl-rl-ext` PROFILE and measures zero.
    // `tools/horn_differential.py` runs the 27-row SUPPLIED TABLE in
    // `tests/fixtures/horn/builtin_rules.tsv` and credits `scm-avf1` once, on
    // `benchmark/reference/pizza-reference.owl`, so it was never on that tool's list of
    // rules the corpus cannot reach and no coverage fixture was written for it. The two
    // rule sets are not the same rule set and the same file separates them elsewhere
    // too: on pizza, measured 15 September 2026, the profile derives 357 `rdfs11` and
    // 101 `scm-svf1` where the table derives 387 and 102, and the profile additionally
    // fires `cls-int2` and `cls-oo`, which the table does not contain. Neither number is
    // known to be wrong. What is wrong is quoting either as "the corpus", and this
    // comment is here so the zero below is not read as a claim about the other one.
    for silent in ["scm-svf2", "scm-avf1", "scm-avf2"] {
        assert_eq!(
            totals.get(silent).copied().unwrap_or(0),
            0,
            "{silent} was measured at zero across the whole corpus. A non-zero count is good \
             news and wants this list shortened rather than the rule removed: {totals:?}"
        );
    }
}

/// The numbers, on one file rather than on an aggregate.
///
/// A corpus total moves whenever anyone adds an RDF file, so it cannot be
/// pinned. `benchmark/reference/pizza-reference.owl` is the pizza ontology with
/// 156 `owl:someValuesFrom` and 49 `owl:allValuesFrom` axioms, it is the file
/// the restriction-ordering rules were chosen on, and it does not move.
///
/// Read the zeroes as carefully as the counts. `scm-avf1` and `scm-avf2` draw
/// nothing here despite 49 universal restrictions, because the pizza ontology
/// uses them as closure axioms whose fillers are anonymous unions, and no two
/// of those stand in a subclass or subproperty relation.
#[test]
fn the_measured_shape_of_one_committed_file() {
    let path = repo().join("benchmark/reference/pizza-reference.owl");
    assert!(path.exists(), "the measured file is missing: {}", path.display());
    let store = Arc::new(GraphStore::new());
    store.load_file(&path.display().to_string()).unwrap();
    let dir = scratch("pizza");
    let out = Reasoner::run_full(
        &store, "owl-rl-ext", false, InferenceTarget::DefaultGraph, Some(&dir),
    )
    .unwrap();
    let r: serde_json::Value = serde_json::from_str(&out).unwrap();
    let by_rule: BTreeMap<String, u64> = r["certificate"]["by_rule"]
        .as_object()
        .expect("by_rule")
        .iter()
        .map(|(k, v)| (k.clone(), v.as_u64().unwrap_or(0)))
        .collect();
    eprintln!("pizza-reference.owl inferred={} by_rule={by_rule:?}", r["inferred_count"]);

    for (rule, want) in [
        ("scm-svf1", 101u64),
        ("scm-dom1", 12),
        ("scm-rng1", 14),
        ("cls-int2", 5),
        ("cls-oo", 5),
        ("scm-svf2", 0),
        ("scm-avf1", 0),
        ("scm-avf2", 0),
        ("scm-dom2", 0),
        ("scm-rng2", 0),
    ] {
        assert_eq!(
            by_rule.get(rule).copied().unwrap_or(0),
            want,
            "{rule} on pizza-reference.owl: {by_rule:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Rules this engine deliberately does not run, kept out by a test rather than
/// by a comment. Each is a sound OWL 2 RL rule, and the reason for each is on
/// `Reasoner` in `src/reason.rs`. A certificate line citing one would mean the
/// decision was reversed without the semantic condition that has to come with
/// it, and the Lean side would refuse the certificate rather than the engine
/// refusing to emit it.
#[test]
fn the_rules_that_were_left_out_stay_out() {
    let dir = scratch("left-out");
    let (_, r) = run(&format!("{PREFIXES}{NEW_RULES}"), "owl-rl-ext", Some(&dir));
    let by_rule = r["certificate"]["by_rule"].as_object().expect("by_rule");
    for unwanted in [
        "eq-ref", "eq-rep-s", "eq-rep-p", "eq-rep-o",
        "scm-cls", "scm-op", "scm-dp",
        "dt-type1", "dt-type2", "dt-eq", "dt-diff", "dt-not-type",
        "cax-dw", "cls-com", "cls-maxc1", "prp-irp", "prp-asyp", "prp-pdw", "prp-npa1",
    ] {
        assert!(
            !by_rule.contains_key(unwanted),
            "{unwanted} is not implemented and must not appear in a certificate: {r}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);

    // `eq-ref` is the one with a visible consequence. It asserts `owl:sameAs`
    // reflexivity for every term in every position, so it would put a
    // derivation into the EMPTY graph, and `OOCert.not_everything_is_entailed`
    // is proved against a graph that has none.
    let empty = Arc::new(GraphStore::new());
    let out =
        Reasoner::run_full(&empty, "owl-rl-ext", false, InferenceTarget::DefaultGraph, None)
            .unwrap();
    let e: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        e["inferred_count"].as_u64().unwrap(),
        0,
        "the empty graph must still derive nothing: {e}"
    );
}

