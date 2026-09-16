//! Entailment preservation under graph projection, end to end.
//!
//! `src/projection_check.rs` reports how much of a seed's neighbourhood
//! survived a retrieval. That number is neither necessary nor sufficient for
//! the property that matters, and it moves the WRONG WAY: it rises as the
//! projection grows, so a system tuned on it learns to retrieve more rather
//! than to retrieve the right thing, and a retrieval-augmented answer grounded
//! in such a slice can be false while every displayed metric is green.
//!
//! `src/projection_entailment.rs` asks the property instead: for each claim the
//! answer rests on, does the projection entail it exactly when the source does,
//! with a machine-checked certificate for each one it preserves. These tests
//! close the loop, and every gate is shown FAILING on deliberately broken
//! input, because a gate that cannot fail is decoration.
//!
//! # The headline
//!
//! `a_green_coverage_ratio_does_not_mean_preserved` is built from
//! `benchmark/reference/pizza-reference.owl`, a real ontology this repository
//! ships, and its converse from the same file. Together they are the whole
//! argument for the change, executable rather than argued:
//!
//! * the ontology minus ONE `rdfs:subClassOf` triple scores
//!   `aggregate_coverage_ratio: 1.0` with `ok: true`, and the conclusion the
//!   answer rests on is GONE;
//! * a three-triple slice scores about 0.001, and every claim survives with a
//!   Lean-checked certificate.
//!
//! # The verdict discipline
//!
//! Three kinds of word and they are never collapsed. A preserved entailment
//! whose certificate the Lean checker ACCEPTED is one thing; the engine's
//! unchecked opinion is another; a coverage ratio is a third and is not a
//! warrant at all. `an_unchecked_result_never_prints_the_checked_word` is
//! modelled line for line on
//! `tests/lean_horn_certificate_test.rs::a_user_rule_never_earns_the_absolute_verdict`
//! and carries no skip guard, because it needs the checker to be ABSENT and is
//! therefore the one gate a machine without Lean still enforces.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::projection_entailment as pe;
use open_ontologies::projection_entailment::{GoalRefusal, GoalVerdict, Membership, Projection};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// Every test in this binary takes this lock.
///
/// `a_truncated_run_disarms_the_differential` has to move
/// `runtime::reasoner_max_iterations`, which is a PROCESS-WIDE atomic, and the
/// default harness runs the tests in one binary on several threads. Without the
/// lock that test truncated everyone else's reasoner: four unrelated gates
/// failed with `fixpoint_not_reached` and a goal the source plainly derives came
/// back `ungrounded_in_source`. A shared global is not a thing to be polite
/// about, so the serialisation is explicit and every test pays it. The whole
/// file runs in about a second, so it costs nothing worth having.
fn serial() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ── Fixtures and the loud skip ─────────────────────────────────────────────

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn lean_dir() -> PathBuf {
    repo().join("lean")
}

fn lake_available() -> bool {
    // `.current_dir(lean_dir())` is not cosmetic. elan resolves the toolchain
    // from the working directory's `lean-toolchain`, and the crate root is the
    // one directory in this repository with none in its ancestry. Run from
    // there against an elan with no default toolchain, which is what
    // `leanprover/lean-action` installs, `lake --version` exits non-zero and
    // the probe reports lake as missing while every developer machine stays
    // green. `tests/lean_certificate_test.rs` records the CI failure that cost.
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

/// Build the checkers once per test binary. The proofs are part of the build,
/// so a failure here is a failure and never a skip.
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

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oo-preserve-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn loaded(ttl: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(ttl, None).expect("load");
    g
}

// ── The real ontology the headline pair is built from ──────────────────────

const PIZZA_FILE: &str = "benchmark/reference/pizza-reference.owl";
const PZ: &str = "https://raw.githubusercontent.com/owlcs/pizza-ontology/refs/heads/master/pizza.owl#";
const SUBCLASS: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

/// The shipped pizza ontology, SKOLEMISED.
///
/// Skolemising is not a convenience. `owl:Restriction` superclasses are blank
/// nodes, a Turtle round trip mints fresh labels for them, and every
/// blank-node-bearing triple of the slice then differs textually from the
/// corresponding triple of the source: the subset check reports spurious
/// extras and the monotonicity gate fires against a correct engine. Replacing
/// each blank node with a Skolem IRI BEFORE the retriever sees the graph is the
/// only thing that makes a re-parsed slice comparable at all.
fn pizza() -> (Arc<GraphStore>, Vec<String>) {
    let raw = Arc::new(GraphStore::new());
    raw.load_file(&repo().join(PIZZA_FILE).display().to_string()).expect("load pizza");
    let (g, map) = pe::skolemise(&raw).expect("skolemise");
    assert!(!map.is_empty(), "the pizza ontology has blank nodes to skolemise");
    let q = format!("SELECT ?s WHERE {{ ?s <{SUBCLASS}> <{PZ}NamedPizza> }} ORDER BY ?s");
    let js = g.sparql_select(&q).unwrap();
    let v: serde_json::Value = serde_json::from_str(&js).unwrap();
    let seeds: Vec<String> = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["s"].as_str().unwrap().trim_matches(|c| c == '<' || c == '>').to_string())
        .collect();
    assert!(seeds.len() >= 20, "expected the named pizzas as seeds, found {}", seeds.len());
    (g, seeds)
}

fn opts(dir: &str, seeds: &[String]) -> pe::Opts {
    pe::Opts {
        profile: "owl-rl-ext".into(),
        work_dir: scratch(dir),
        seed_iris: seeds.to_vec(),
        ..Default::default()
    }
}

fn run(
    g: &Arc<GraphStore>,
    p: Projection<'_>,
    goals_ttl: &str,
    o: &pe::Opts,
) -> pe::PreservationReport {
    let (goals, refused) = pe::parse_goals_turtle(goals_ttl).unwrap();
    pe::check_entailment_preservation(g, p, &goals, &refused, o).expect("a report")
}

fn coverage(r: &pe::PreservationReport) -> f64 {
    r.coverage_proxy
        .report
        .as_ref()
        .map(|x| x.aggregate_coverage_ratio)
        .expect("the proxy was computed")
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. The headline, and its converse, on a real shipped ontology
// ═══════════════════════════════════════════════════════════════════════════

/// A projection at coverage 1.00 with `ok: true` that has dropped the one
/// triple the answer depends on.
///
/// Built from the real file, not a fixture invented to flatter the design. The
/// dropped triple is `NamedPizza rdfs:subClassOf Pizza`; the goal is
/// `Veneziana rdfs:subClassOf Food`, which the source derives by `rdfs11`
/// through that link. `NamedPizza` is not one of the seeds, so the proxy's
/// per-seed neighbourhoods are untouched and the number CANNOT SEE the damage.
///
/// The two assertions are in one block on purpose: the inverted-metric shape is
/// pinned in the suite rather than argued about in a doc.
#[test]
fn a_green_coverage_ratio_does_not_mean_preserved() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let (g, seeds) = pizza();
    let load_bearing = (
        format!("<{PZ}NamedPizza>"),
        format!("<{SUBCLASS}>"),
        format!("<{PZ}Pizza>"),
    );
    let ttl: String = g
        .all_triples()
        .unwrap()
        .into_iter()
        .filter(|t| *t != load_bearing)
        .map(|t| format!("{} {} {} .\n", t.0, t.1, t.2))
        .collect();
    let goals = format!("<{PZ}Veneziana> <{SUBCLASS}> <{PZ}Food> .\n");
    let r = run(&g, Projection::Turtle(&ttl), &goals, &opts("headline", &seeds));

    let cov = coverage(&r);
    let proxy_ok = r.coverage_proxy.report.as_ref().unwrap().ok;
    assert!(
        cov >= 0.99 && proxy_ok && r.per_goal[0].verdict == GoalVerdict::LostUnderProfileUnchecked,
        "THE WHOLE ARGUMENT. coverage_ratio={cov} (ok={proxy_ok}) over a slice that dropped the \
         one triple the answer rests on, and the verdict is {:?}. If this assertion ever passes \
         with a preserved verdict, or fails because the ratio moved, say which and why.\n{}",
        r.per_goal[0].verdict,
        r.headline
    );
    assert_eq!(r.exit_code, 1, "a lost goal must fail the process: {}", r.headline);
    assert!(!r.ok);
    assert!(
        !r.coverage_proxy.is_a_warrant,
        "the proxy must never claim to be a warrant"
    );
}

/// The converse, from the same file: coverage near zero, every claim preserved
/// with a machine-checked certificate.
///
/// Both directions, or the metric is untested.
#[test]
fn a_low_coverage_slice_can_preserve_every_claim() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let (g, seeds) = pizza();
    let ttl = format!(
        "<{PZ}Veneziana> <{SUBCLASS}> <{PZ}NamedPizza> .\n\
         <{PZ}NamedPizza> <{SUBCLASS}> <{PZ}Pizza> .\n\
         <{PZ}Pizza> <{SUBCLASS}> <{PZ}Food> .\n"
    );
    let goals = format!(
        "<{PZ}Veneziana> <{SUBCLASS}> <{PZ}Food> .\n<{PZ}Veneziana> <{SUBCLASS}> <{PZ}Pizza> .\n"
    );
    let r = run(&g, Projection::Turtle(&ttl), &goals, &opts("converse", &seeds));

    let cov = coverage(&r);
    assert!(
        cov < 0.10 && r.preserved_checked == 2 && r.lost == 0,
        "a slice at coverage {cov} preserved {}/{} goals with a checked certificate, {} lost.\n{}",
        r.preserved_checked,
        r.goals_total,
        r.lost,
        r.headline
    );
    assert_eq!(r.exit_code, 0, "{}", r.headline);
    for gr in &r.per_goal {
        assert_eq!(gr.verdict, "preserved_checked");
        assert_eq!(gr.warrant, "OOCert.certificate_sound");
        let c = gr.certificate.as_ref().expect("a checked goal carries its certificate");
        assert!(c.check_with.contains("oo-cert"), "{c:?}");
        assert!(c.steps > 0, "a derived goal's slice has at least one step: {c:?}");
    }
}

/// A real retrieval, not a hand-built subset: `onto_segment_retrieve` over the
/// skolemised ontology, audited by the tool it is advertised to pair with.
#[test]
fn a_real_retrieval_of_a_real_ontology_is_audited() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let (g, seeds) = pizza();
    let seg = open_ontologies::segment_retrieve::retrieve_segment(&g, &seeds, 2, false).unwrap();
    assert!(
        !seg.turtle.contains("<_:"),
        "a skolemised source must produce a slice with no angle-bracketed blank node"
    );
    let goals = format!(
        "<{PZ}Veneziana> <{SUBCLASS}> <{PZ}Pizza> .\n<{PZ}Veneziana> <{SUBCLASS}> <{PZ}Food> .\n"
    );
    let r = run(&g, Projection::Turtle(&seg.turtle), &goals, &opts("retrieval", &seeds));
    assert!(r.subset.verified, "a slice of the skolemised source IS a subset: {:?}", r.subset);
    assert_eq!(r.monotonicity.status, "armed", "{:?}", r.monotonicity);
    assert_eq!(r.monotonicity.violation_count, 0, "{:?}", r.monotonicity);
    // The 2-hop budget reaches NamedPizza and Pizza but not Food, so the
    // retriever really does lose the deeper conclusion. Reported, not asserted
    // as a fixed number, because the retriever is free to change.
    eprintln!(
        "REAL RETRIEVAL: {} triples, coverage {:.5}, checked {}, lost {}",
        seg.triple_count,
        coverage(&r),
        r.preserved_checked,
        r.lost
    );
    assert_eq!(r.preserved_checked + r.lost, 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. The four cells
// ═══════════════════════════════════════════════════════════════════════════

const PREFIXES: &str = "@prefix : <http://ex.org/> .\n\
                        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n";
const CHAIN: &str = ":A rdfs:subClassOf :B . :B rdfs:subClassOf :C . :a a :A .\n";

#[test]
fn a_preserved_claim_is_checked() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    let r = run(
        &g,
        Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
        &format!("{PREFIXES}:a a :C .\n"),
        &opts("cell-preserved", &["http://ex.org/a".into()]),
    );
    assert_eq!(r.per_goal[0].verdict, "preserved_checked", "{}", r.headline);
    assert_eq!(r.per_goal[0].warrant, "OOCert.certificate_sound");
    assert_eq!(r.exit_code, 0);
}

/// The fourth cell, and the one the brief is really about: a claim NEITHER
/// graph derives is not a lossy slice, it is a generator that invented
/// something, and the two have opposite fixes.
#[test]
fn a_claim_entailed_by_neither_is_not_lost() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    let r = run(
        &g,
        Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
        &format!("{PREFIXES}:a a :Ghost .\n"),
        &opts("cell-ungrounded", &[]),
    );
    let v = r.per_goal[0].verdict;
    assert_eq!(v, GoalVerdict::UngroundedInSource, "{}", r.headline);
    assert!(!v.is_preserved(), "an ungrounded claim is not preserved");
    assert_ne!(v, GoalVerdict::LostUnderProfileUnchecked, "and it is not lost either");
    assert!(
        r.per_goal[0].means.contains("generator"),
        "the report must send this to the right team: {}",
        r.per_goal[0].means
    );
    assert_eq!(r.ungrounded_in_source, 1);
    assert_eq!(r.exit_code, 1, "an ungrounded claim is a finding, not a pass");
}

/// The likeliest bug to actually ship. The certificate holds only INFERRED
/// triples, so a goal literally in the slice has NO derivation line, and the
/// naive implementation searches `derivations.tsv`, finds nothing, and reports
/// it LOST. That is inverted, and it would pass review because a happy-path
/// test naturally uses a derived goal.
#[test]
fn an_asserted_goal_is_preserved_asserted_not_lost() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    let r = run(
        &g,
        Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
        &format!("{PREFIXES}:A rdfs:subClassOf :B .\n"),
        &opts("asserted", &[]),
    );
    let gr = &r.per_goal[0];
    assert_eq!(gr.projection, Membership::Asserted, "{gr:?}");
    assert_eq!(gr.verdict, GoalVerdict::PreservedAsserted);
    assert_ne!(gr.verdict, "preserved_checked", "a lookup is not a theorem");
    assert_eq!(gr.warrant, "none", "and it names no theorem");
    assert!(gr.certificate.is_none());
    assert_eq!(r.exit_code, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. The verdict discipline
// ═══════════════════════════════════════════════════════════════════════════

/// The laundering guard, modelled on
/// `lean_horn_certificate_test.rs::a_user_rule_never_earns_the_absolute_verdict`.
///
/// NO SKIP GUARD, deliberately: this test needs the checker to be ABSENT, so it
/// runs everywhere and is the one gate a machine without Lean still enforces.
/// The assertions are over the SERIALISED report rather than over the enum,
/// because the enum is where the discipline is easy and the serialisation is
/// where it leaks.
#[test]
fn an_unchecked_result_never_prints_the_checked_word() {
    let _serial = serial();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    let mut o = opts("unchecked", &[]);
    o.checker = Some(PathBuf::from("/nonexistent/oo-cert"));
    let r = run(
        &g,
        Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
        &format!("{PREFIXES}:a a :C .\n"),
        &o,
    );
    assert_eq!(r.per_goal[0].verdict, GoalVerdict::PreservedUnchecked);
    assert_eq!(r.preserved_checked, 0);

    let json = serde_json::to_string(&r).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    for gr in v["per_goal"].as_array().unwrap() {
        let verdict = gr["verdict"].as_str().unwrap();
        assert!(
            !verdict.contains("checked") || verdict == "preserved_unchecked",
            "an unchecked result printed the checked word: {verdict}"
        );
        assert_eq!(gr["warrant"].as_str().unwrap(), "none");
    }
    assert!(
        !json.contains("OOCert.certificate_sound"),
        "a run that never ran the checker must not name its theorem anywhere: {json}"
    );
    assert_eq!(v["checker"]["status"].as_str().unwrap(), "absent", "{json}");
    assert!(
        v["checker"]["install"].as_str().unwrap().contains("elan"),
        "the skip must carry the install line: {json}"
    );
}

/// `require_checker` turns an absent checker into an ERROR rather than an
/// honest unchecked verdict. The CI leg sets it, because a job that is supposed
/// to provide Lean and silently did not would otherwise go green over a run
/// where nothing was checked.
#[test]
fn require_checker_turns_an_absent_checker_into_a_failure() {
    let _serial = serial();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    let mut o = opts("require", &[]);
    o.checker = Some(PathBuf::from("/nonexistent/oo-cert"));
    o.require_checker = true;
    let (goals, refused) = pe::parse_goals_turtle(&format!("{PREFIXES}:a a :C .\n")).unwrap();
    let err = pe::check_entailment_preservation(
        &g,
        Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
        &goals,
        &refused,
        &o,
    )
    .expect_err("an absent checker under require_checker must be an error");
    assert!(err.to_string().contains("elan"), "the error must carry the install line: {err}");
}

/// A user-supplied Horn table makes every preserved verdict conditional on
/// assumptions the user wrote. A rule reading "every supplier is compliant"
/// produces certificates that check green for ever, so the word must say what
/// it is relative to and must never be shortened.
#[test]
fn a_user_rule_run_never_earns_the_plain_preserved_word() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    let dir = scratch("user-rules");
    let rules = dir.join("rules.tsv");
    // The rdfs9 shape, written as data: a user table that happens to be sound.
    // Soundness is not the point; being SUPPLIED is.
    // `name TAB body_length TAB (3 fields per body atom) TAB (3 fields head)`,
    // the format `oo-horn rules` prints and `tests/fixtures/horn/builtin_rules.tsv`
    // carries.
    std::fs::write(
        &rules,
        "my-rdfs9\t2\t\
         ?x\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t?sub\t\
         ?sub\t<http://www.w3.org/2000/01/rdf-schema#subClassOf>\t?sup\t\
         ?x\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t?sup\n",
    )
    .unwrap();
    let mut o = opts("user-rules-run", &[]);
    o.rules = Some(rules);
    let r = run(
        &g,
        Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
        &format!("{PREFIXES}:a a :C .\n"),
        &o,
    );
    let json = serde_json::to_string(&r).unwrap();
    assert!(
        !json.contains("\"preserved_checked\":") || r.preserved_checked > 0,
        "sanity"
    );
    for gr in &r.per_goal {
        assert_ne!(
            gr.verdict,
            "preserved_checked",
            "a run over a SUPPLIED rule table must not report the plain preserved word: {json}"
        );
        if gr.verdict.is_checked() {
            assert_eq!(gr.verdict, "preserved_under_supplied_rules_checked");
            assert_eq!(gr.warrant, "OOCert.horn_certificate_sound");
        }
    }
    assert_eq!(
        r.preserved_checked, 1,
        "the guard must not be vacuous: the goal has to be PRESERVED under the supplied table, \
         or this test would pass on a run that proved nothing.\n{json}"
    );
    assert_eq!(r.per_goal[0].verdict, "preserved_under_supplied_rules_checked");
    assert!(
        r.rules_sha256.is_some(),
        "the table that was in force must be identified: {json}"
    );
    assert!(
        !json.contains("\"verdict\":\"preserved_checked\""),
        "the plain word must not appear anywhere in the serialisation: {json}"
    );
}

/// A rejection is a defect in THIS module or in the emitter, and it downgrades
/// to nothing. The tempting code is `.unwrap_or(PreservedUnchecked)`, which
/// converts a defect in the slice extractor into a marginally weaker verdict
/// nobody will ever investigate.
#[test]
fn a_rejected_sub_certificate_stops_the_line() {
    let _serial = serial();
    if skip() {
        return;
    }
    let bin = checker();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    let o = opts("forged", &[]);

    // First the honest run, so the forgery is the only difference.
    let honest = run(
        &g,
        Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
        &format!("{PREFIXES}:a a :C .\n"),
        &o,
    );
    assert_eq!(honest.per_goal[0].verdict, "preserved_checked");

    // Now forge the projection's derivation file: a well-formed rdfs9 step
    // whose subClassOf premise is neither asserted nor derived earlier, with
    // the goal as its conclusion. `slice_for` will pull exactly this line.
    let d = o.work_dir.join("projection").join("derivations.tsv");
    let forged = "rdfs9\t<http://ex.org/a>\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t<http://ex.org/Z>\t\
                  <http://ex.org/a>\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t<http://ex.org/A>\t\
                  <http://ex.org/A>\t<http://www.w3.org/2000/01/rdf-schema#subClassOf>\t<http://ex.org/Ghost>\n";
    let mut content = std::fs::read_to_string(&d).unwrap();
    content.push_str(forged);
    std::fs::write(&d, &content).unwrap();

    // The checker must reject the whole file, which is what makes the slice
    // for :a a :Z a rejected sub-certificate.
    let out = Command::new(bin)
        .arg(o.work_dir.join("projection").join("asserted.tsv"))
        .arg(&d)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "the forgery must be rejected by oo-cert");

    // Re-read the doctored certificate through the module and ask for the
    // forged conclusion. The extractor builds a one-line slice whose premise is
    // in neither asserted.tsv nor an earlier line.
    let cert = pe::CertificateIndex::read(&o.work_dir.join("projection")).unwrap();
    let goal = (
        "<http://ex.org/a>".to_string(),
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>".to_string(),
        "<http://ex.org/Z>".to_string(),
    );
    let lines = cert.slice_for(&goal).expect("the forged line concludes the goal");
    let slice = o.work_dir.join("forged-slice.tsv");
    cert.write_slice(&lines, &slice).unwrap();
    let status = pe::run_oo_cert(None, &cert.asserted_path(), &slice);
    assert!(
        matches!(status, pe::CheckerStatus::Rejected { .. }),
        "a premise in neither asserted.tsv nor an earlier line must be rejected: {status:?}"
    );
    assert!(
        !matches!(status, pe::CheckerStatus::Accepted(..)),
        "and it must NOT be downgraded to an accepted or unchecked result"
    );

}

/// A checker that says NO must stop the line, not downgrade.
///
/// The tempting code is `.unwrap_or(GoalVerdict::PreservedUnchecked)`, which
/// converts a defect in the slice extractor, or in the emitter's premise
/// ordering, into a marginally weaker verdict nobody will ever investigate.
/// `tools/shacl_differential.py` refuses to give a FALSE_CLEAN that treatment
/// and neither does this.
///
/// The engine and extractor are correct, so a rejection cannot be produced from
/// honest input: the checker is replaced by a stub that exits 1, which is
/// exactly the observable this arm reads. What is under test is the MAPPING
/// from a rejection to a verdict, and the real checker's ability to reject is
/// pinned separately in `a_rejected_sub_certificate_stops_the_line`.
#[test]
#[cfg(unix)]
fn a_rejected_slice_never_downgrades_to_preserved_unchecked() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let dir = scratch("always-no");
    let stub = dir.join("always-no");
    std::fs::write(&stub, "#!/bin/sh\necho '{\"ok\":false,\"first_rejected\":0}'\nexit 1\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    let mut o = opts("always-no-run", &[]);
    o.checker = Some(stub);
    let r = run(
        &g,
        Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
        &format!("{PREFIXES}:a a :C .\n"),
        &o,
    );
    let v = r.per_goal[0].verdict;
    assert_ne!(v, GoalVerdict::PreservedUnchecked, "a rejection must NOT downgrade: {v:?}");
    assert_ne!(v, "preserved_checked");
    assert!(!v.is_preserved(), "a rejected sub-certificate is not a preserved goal: {v:?}");
    assert_eq!(
        v,
        GoalVerdict::CertificateRejected,
        "and it gets its own word rather than borrowing projection_only's, whose `means` would \
         say the source does not derive the goal, which is false here"
    );
    assert_eq!(r.certificate_rejected, 1);
    assert_eq!(r.per_goal[0].warrant, "none");
    assert!(
        r.monotonicity.disagreement.is_none(),
        "a certificate rejection is not a statement about monotonicity and must not be filed in \
         its block: {:?}",
        r.monotonicity
    );
    let d = r
        .disagreement
        .as_ref()
        .expect("a rejection must produce a top-level disagreement block");
    assert_eq!(d.severity, "STOP_THE_LINE");
    assert_eq!(d.what, "certificate_rejected");
    assert!(d.means.contains("defect in the slice extractor"), "{}", d.means);
    assert_eq!(r.exit_code, 2, "a stop-the-line exits 2, never 1: {}", r.headline);
    assert!(!r.ok);
}

/// The rejection-to-stop-the-line mapping, on a certificate directory doctored
/// before the report is assembled.
#[test]
fn a_rejection_is_reported_at_stop_the_line_and_exits_two() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    let o = opts("stop-the-line", &[]);
    // Run once to create the work dir, then make the PROJECTION's certificate
    // a file the checker rejects, and keep the reasoner from overwriting it by
    // pointing the second run at a copy.
    let _ = run(
        &g,
        Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
        &format!("{PREFIXES}:a a :C .\n"),
        &o,
    );
    let d = o.work_dir.join("projection").join("derivations.tsv");
    let mut content = std::fs::read_to_string(&d).unwrap();
    // A forged conclusion on the goal's own line: rdfs9 with premises that do
    // not support it. `slice_for` prefers the FIRST line concluding the goal,
    // so prepend it.
    let forged = "rdfs9\t<http://ex.org/a>\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t<http://ex.org/C>\t\
                  <http://ex.org/a>\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t<http://ex.org/A>\t\
                  <http://ex.org/A>\t<http://www.w3.org/2000/01/rdf-schema#subClassOf>\t<http://ex.org/B>\n";
    content.insert_str(0, forged);
    std::fs::write(&d, &content).unwrap();

    let cert = pe::CertificateIndex::read(&o.work_dir.join("projection")).unwrap();
    let goal = (
        "<http://ex.org/a>".to_string(),
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>".to_string(),
        "<http://ex.org/C>".to_string(),
    );
    let lines = cert.slice_for(&goal).unwrap();
    let slice = o.work_dir.join("rejected-slice.tsv");
    cert.write_slice(&lines, &slice).unwrap();
    let status = pe::run_oo_cert(None, &cert.asserted_path(), &slice);
    assert!(
        matches!(status, pe::CheckerStatus::Rejected { .. }),
        "the conclusion is not what rdfs9 yields from those premises: {status:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. The free differential and its three disarming conditions
// ═══════════════════════════════════════════════════════════════════════════

/// A retriever that normalises an IRI, re-prefixes, or adds a tidy declaration
/// produces a projection that is not a subset. Every extra conclusion of it is
/// then ordinary, and announcing a soundness bug in the engine would be a
/// spectacular false alarm.
#[test]
fn a_non_subset_projection_disarms_the_differential() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    let extra = format!("{PREFIXES}{CHAIN}:Z rdfs:subClassOf :Y .\n");
    let r = run(
        &g,
        Projection::Turtle(&extra),
        &format!("{PREFIXES}:a a :C .\n"),
        &opts("non-subset", &[]),
    );
    assert!(!r.subset.verified, "{:?}", r.subset);
    assert_eq!(r.subset.extra_count, 1);
    assert!(
        r.subset.extra_triples.iter().any(|t| t.contains("ex.org/Z")),
        "the extra triple must be NAMED: {:?}",
        r.subset
    );
    assert_eq!(r.monotonicity.status, "disarmed");
    assert_eq!(r.monotonicity.reason, "projection_is_not_a_subset");
    assert!(r.monotonicity.disagreement.is_none(), "no stop-the-line may fire from a disarmed gate");
    assert_ne!(r.exit_code, 2);
}

/// A truncated closure is a LOWER BOUND and cannot refute anything. The source
/// is the larger graph and therefore the likelier to truncate, and a truncated
/// `closure(G)` missing a conclusion `closure(P)` reached manufactures a false
/// accusation of unsoundness against a correct engine.
#[test]
fn a_truncated_run_disarms_the_differential() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    // A chain long enough that one round cannot close it.
    let mut ttl = String::from(PREFIXES);
    for i in 0..8 {
        ttl.push_str(&format!(":C{i} rdfs:subClassOf :C{}.\n", i + 1));
    }
    ttl.push_str(":a a :C0 .\n");
    let g = loaded(&ttl);
    let previous = open_ontologies::runtime::reasoner_max_iterations();
    open_ontologies::runtime::set_reasoner_max_iterations(1);
    let r = run(
        &g,
        Projection::Turtle(&ttl),
        &format!("{PREFIXES}:a a :C8 .\n"),
        &opts("truncated", &[]),
    );
    open_ontologies::runtime::set_reasoner_max_iterations(previous);

    assert!(!r.source_fixpoint_reached, "one iteration cannot close an eight-link chain");
    assert_eq!(r.monotonicity.status, "disarmed", "{:?}", r.monotonicity);
    assert_eq!(r.monotonicity.reason, "fixpoint_not_reached");
    assert!(
        r.per_goal.iter().all(|gr| gr.verdict.is_preserved() || gr.bounded_by_iteration_cap),
        "every NEGATIVE verdict from a truncated run must be marked a lower bound: {:?}",
        r.per_goal
    );
}

/// The gate is shown able to FIRE. The engine cannot be made unsound on demand,
/// so the pure function is handed a pair where the projection's closure exceeds
/// the source's with every guard true. Anything less and a gate whose real
/// trigger is a defect nobody has could never be demonstrated at all.
#[test]
fn the_differential_can_fire() {
    let _serial = serial();
    let t = |s: &str| (s.to_string(), "<p>".to_string(), "<o>".to_string());
    let src: std::collections::HashSet<pe::Spelled> = [t("<a>")].into_iter().collect();
    let prj: std::collections::HashSet<pe::Spelled> =
        [t("<a>"), t("<ghost>")].into_iter().collect();
    let r = pe::differential(
        &src,
        &prj,
        pe::Guards { subset_verified: true, blank_nodes_unmatched: false, both_reached_fixpoint: true },
    );
    let d = r.disagreement.expect("a P-only conclusion under armed guards is a disagreement");
    assert_eq!(d.severity, "STOP_THE_LINE");
    assert_eq!(d.what, "monotonicity_violated");
    assert!(d.means.contains("SOUNDNESS BUG IN THE ENGINE"), "{}", d.means);
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. Goals: what is asked, and what is refused
// ═══════════════════════════════════════════════════════════════════════════

/// The store rewrites `"01"^^xsd:integer` to `"1"^^xsd:integer` on the way in.
/// A goal typed in the original spelling and NOT routed through the same parser
/// matches nothing and is reported lost, which is a false loss with a
/// green-looking pipeline around it.
///
/// Measured here rather than taken from the docs: the assertion on the
/// canonicaliser is inline, so if oxigraph ever stops canonicalising, this
/// fails loudly instead of quietly protecting nothing.
#[test]
fn a_literal_spelling_is_canonicalised_before_asking() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let ttl = "@prefix : <http://ex.org/> .\n\
               @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
               :a :n \"1\"^^xsd:integer .\n";
    let g = loaded(ttl);

    // The canonicaliser, asserted inline.
    let raw = (
        "<http://ex.org/a>".to_string(),
        "<http://ex.org/n>".to_string(),
        "\"01\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_string(),
    );
    let (goals, _) = pe::parse_goals_turtle(
        "@prefix : <http://ex.org/> .\n\
         @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
         :a :n \"01\"^^xsd:integer .\n",
    )
    .unwrap();
    assert_ne!(
        goals[0].triple, raw,
        "the parser must canonicalise the literal; if it does not, this whole guard is dead"
    );
    assert_eq!(goals[0].triple.2, "\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>");

    // Routed through the parser: preserved.
    let o = opts("literal", &[]);
    let r = pe::check_entailment_preservation(
        &g,
        Projection::Turtle(ttl),
        &goals,
        &[],
        &o,
    )
    .unwrap();
    assert_eq!(r.per_goal[0].verdict, GoalVerdict::PreservedAsserted, "{}", r.headline);

    // The broken variant: the caller's raw spelling asked directly of the
    // certificate is NOT derivable, which is what a tool that skipped the
    // parser would have reported as a loss.
    let cert = pe::CertificateIndex::read(&o.work_dir.join("projection")).unwrap();
    assert_eq!(
        cert.membership(&raw),
        Membership::NotDerivable,
        "the raw spelling matches nothing, which is exactly the false loss"
    );
    assert_eq!(cert.membership(&goals[0].triple), Membership::Asserted);
}

#[test]
fn a_blank_node_goal_and_a_negative_claim_are_refused_and_counted() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    let (goals, refused) = pe::parse_goals_turtle(
        "@prefix : <http://ex.org/> .\n\
         @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         :a rdfs:subClassOf :A .\n\
         :b :p [ :q :r ] .\n\
         :npa a owl:NegativePropertyAssertion .\n",
    )
    .unwrap();
    // `[ :q :r ]` is TWO triples, both blank-node-bearing: `:b :p _:x` and
    // `_:x :q :r`. Three refusals, not two, and the count is what the report
    // carries so a refusal can never shrink the denominator unnoticed.
    assert_eq!(goals.len(), 1, "one askable goal");
    let reasons: Vec<GoalRefusal> = refused.iter().map(|r| r.reason).collect();
    assert!(reasons.contains(&GoalRefusal::BlankNode), "{refused:?}");
    assert!(reasons.contains(&GoalRefusal::NotAPositiveGroundTriple), "{refused:?}");
    assert!(
        refused
            .iter()
            .any(|r| r.reason == GoalRefusal::NotAPositiveGroundTriple
                && r.means.contains("onto_shacl")),
        "a closed-world question must be sent to SHACL by name: {refused:?}"
    );

    let r = pe::check_entailment_preservation(
        &g,
        Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
        &goals,
        &refused,
        &opts("refusals", &[]),
    )
    .unwrap();
    assert_eq!(r.refused, 3, "a refusal must be COUNTED, never shrink the denominator");
    assert_eq!(r.goals_total, 1);
    assert_eq!(r.exit_code, 1, "a refused goal is a finding");
    assert!(!r.ok);
}

#[test]
fn an_empty_goal_set_is_an_error() {
    let _serial = serial();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    let (goals, refused) =
        pe::parse_goals_turtle("@prefix : <http://ex.org/> .\n:b :p [ :q :r ] .\n").unwrap();
    assert!(goals.is_empty());
    let err = pe::check_entailment_preservation(
        &g,
        Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
        &goals,
        &refused,
        &opts("empty", &[]),
    )
    .expect_err("zero goals must not produce a green report");
    assert!(err.to_string().contains("2 goal(s) were refused"), "{err}");
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. The demoted proxy, and the emitter contract the slices depend on
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn the_coverage_label_is_in_every_report() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    for seeds in [vec![], vec!["http://ex.org/a".to_string()]] {
        let r = run(
            &g,
            Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
            &format!("{PREFIXES}:a a :C .\n"),
            &opts("label", &seeds),
        );
        assert!(!r.coverage_proxy.is_a_warrant);
        let json = serde_json::to_string(&r).unwrap();
        assert!(
            json.contains("neither necessary nor sufficient"),
            "the label must travel with the number, seeds={seeds:?}: {json}"
        );
        assert!(json.contains("\"is_a_warrant\":false"), "{json}");
    }
}

/// The ordering claim the whole slice extractor rests on, pinned rather than
/// left as a comment.
///
/// `run_full` computes a round's conclusions from `triple_set` as it stood at
/// the START of that round and inserts them only afterwards, so every premise
/// of a line was asserted or concluded strictly earlier in the file. `oo-cert`
/// rejects a premise that is "neither asserted nor derived earlier", so if this
/// ever stopped holding every extracted sub-certificate would be rejected and
/// the tool would start reporting stop-the-line on correct runs.
#[test]
fn file_order_is_a_topological_order_of_the_derivations() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let (g, _) = pizza();
    let dir = scratch("topo");
    open_ontologies::reason::Reasoner::run_full(
        &g,
        "owl-rl-ext",
        false,
        open_ontologies::reason::InferenceTarget::DefaultGraph,
        Some(&dir),
    )
    .unwrap();
    let cert = pe::CertificateIndex::read(&dir).unwrap();
    assert!(cert.derivation_count() > 100, "the pizza ontology derives plenty");
    let mut checked = 0usize;
    for i in 0..cert.derivation_count() {
        for p in cert.premises_of(i).unwrap() {
            if cert.asserted_set().contains(p) {
                continue;
            }
            let j = cert
                .conclusion_index(p)
                .unwrap_or_else(|| panic!("premise {p:?} of line {i} is neither asserted nor derived"));
            assert!(
                j < i,
                "premise {p:?} of line {i} is concluded LATER, at line {j}; the file order is not \
                 a topological order and every extracted sub-certificate would be rejected"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "nothing was actually checked");
    eprintln!("topological order verified over {} premise references", checked);
}

/// A named graph of the same store keeps blank node identity, so subsethood is
/// decided over EVERY triple rather than over the ground ones only.
#[test]
fn a_named_graph_projection_is_compared_over_every_triple() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let g = Arc::new(GraphStore::new());
    g.load_turtle(&format!("{PREFIXES}{CHAIN}"), None).unwrap();
    // Copy the slice into a named graph by MODEL TERM, which is what
    // `graph_store` reverses.
    g.load_nquads(
        "<http://ex.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://ex.org/B> <http://ex.org/slice> .\n\
         <http://ex.org/B> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://ex.org/C> <http://ex.org/slice> .\n\
         <http://ex.org/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://ex.org/A> <http://ex.org/slice> .\n",
    )
    .unwrap();
    let r = run(
        &g,
        Projection::NamedGraph("http://ex.org/slice"),
        &format!("{PREFIXES}:a a :C .\n"),
        &opts("named-graph", &[]),
    );
    assert_eq!(r.subset.decided_over, "all triples", "{:?}", r.subset);
    assert!(r.subset.verified);
    assert_eq!(r.monotonicity.status, "armed");
    assert_eq!(r.per_goal[0].verdict, "preserved_checked", "{}", r.headline);
}

/// An earlier `reason` run materialising into the store makes `asserted.tsv` a
/// lie: the engine's own output appears as an axiom, `ungrounded_in_source`
/// becomes unreachable, and every goal looks supported. Decision 0005 item 7
/// records this exact defect, found inside this repository, in a different
/// tool.
#[test]
fn a_store_that_was_reasoned_into_is_reported_unreliable() {
    let _serial = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{PREFIXES}{CHAIN}"));
    open_ontologies::reason::Reasoner::run_full(
        &g,
        "owl-rl-ext",
        true,
        open_ontologies::reason::InferenceTarget::Inferred,
        None,
    )
    .unwrap();
    let r = run(
        &g,
        Projection::Turtle(&format!("{PREFIXES}{CHAIN}")),
        &format!("{PREFIXES}:a a :C .\n"),
        &opts("hygiene", &[]),
    );
    assert!(r.source_hygiene.materialised_inferences_in_source > 0, "{:?}", r.source_hygiene);
    assert!(r.source_hygiene.source_side_unreliable);
    assert!(
        r.source_hygiene.means.contains("asserted.tsv"),
        "{}",
        r.source_hygiene.means
    );
}
