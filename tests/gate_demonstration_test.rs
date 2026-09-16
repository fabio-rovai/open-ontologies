//! Every gate, shown firing on deliberately broken input, with what it said.
//!
//! A gate that cannot fail is decoration, and a gate whose failure nobody has
//! read is close to it. The other test files assert that each gate fires; this
//! one feeds each of them its broken input, PRINTS what the tool actually said,
//! and asserts the same thing. Run it with `--nocapture` and the output is the
//! evidence:
//!
//! ```text
//! cargo test --test gate_demonstration_test -- --nocapture
//! ```
//!
//! Nothing here is a mock. Every line printed comes out of
//! `check_entailment_preservation` or `closure_diff` over input constructed a
//! few lines above it.

mod common;

use open_ontologies::closure_diff as cd;
use open_ontologies::graph::GraphStore;
use open_ontologies::projection_entailment as pe;
use open_ontologies::projection_entailment::{GoalVerdict, Projection};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, MutexGuard};

/// One test in this binary moves the process-wide iteration cap.
fn serial() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn lean_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lean")
}

fn lake_available() -> bool {
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

fn scratch(n: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-gate-{n}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn loaded(ttl: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(ttl, None).unwrap();
    g
}

const P: &str = "@prefix : <http://ex.org/> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n";
const CHAIN: &str = ":A rdfs:subClassOf :B . :B rdfs:subClassOf :C . :a a :A .\n";

fn opts(n: &str, seeds: &[&str]) -> pe::Opts {
    pe::Opts {
        profile: "owl-rl-ext".into(),
        work_dir: scratch(n),
        seed_iris: seeds.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

fn run(g: &Arc<GraphStore>, ttl: &str, goals: &str, o: &pe::Opts) -> pe::PreservationReport {
    let (goals, refused) = pe::parse_goals_turtle(goals).unwrap();
    pe::check_entailment_preservation(g, Projection::Turtle(ttl), &goals, &refused, o).unwrap()
}

fn banner(n: usize, name: &str, broken: &str) {
    eprintln!("\n─── GATE {n}: {name}");
    eprintln!("    BROKEN INPUT: {broken}");
}

/// Prints every gate's reaction to its own broken input, in order.
///
/// One test rather than a dozen, so the output reads top to bottom and the
/// reader is not reassembling it from an interleaved parallel run.
#[test]
fn every_gate_fires_on_its_own_broken_input() {
    let _s = serial();
    if skip() {
        return;
    }

    // ── 1 ────────────────────────────────────────────────────────────────
    banner(
        1,
        "a green coverage ratio is not preservation",
        "a slice holding EVERY triple of the source except `:B rdfs:subClassOf :C`",
    );
    {
        let g = loaded(&format!("{P}{CHAIN}"));
        let slice: String = g
            .all_triples()
            .unwrap()
            .into_iter()
            .filter(|t| t.0 != "<http://ex.org/B>")
            .map(|t| format!("{} {} {} .\n", t.0, t.1, t.2))
            .collect();
        let r = run(&g, &slice, &format!("{P}:a a :C .\n"), &opts("g1", &["http://ex.org/a"]));
        let c = r.coverage_proxy.report.as_ref().unwrap();
        eprintln!(
            "    SAID: aggregate_coverage_ratio={} ok={} | verdict={:?} exit_code={}",
            c.aggregate_coverage_ratio, c.ok, r.per_goal[0].verdict, r.exit_code
        );
        assert_eq!(c.aggregate_coverage_ratio, 1.0);
        assert!(c.ok);
        assert_eq!(r.per_goal[0].verdict, GoalVerdict::LostUnderProfileUnchecked);
        assert_eq!(r.exit_code, 1);
    }

    // ── 2 ────────────────────────────────────────────────────────────────
    banner(2, "a claim neither graph derives is not a loss", "the goal `:a a :Ghost`");
    {
        let g = loaded(&format!("{P}{CHAIN}"));
        let r = run(&g, &format!("{P}{CHAIN}"), &format!("{P}:a a :Ghost .\n"), &opts("g2", &[]));
        eprintln!("    SAID: verdict={:?} exit_code={}", r.per_goal[0].verdict, r.exit_code);
        eprintln!("    MEANS: {}", r.per_goal[0].means);
        assert_eq!(r.per_goal[0].verdict, GoalVerdict::UngroundedInSource);
    }

    // ── 3 ────────────────────────────────────────────────────────────────
    banner(3, "an unchecked result never prints the checked word", "the checker path /nonexistent/oo-cert");
    {
        let g = loaded(&format!("{P}{CHAIN}"));
        let mut o = opts("g3", &[]);
        o.checker = Some(PathBuf::from("/nonexistent/oo-cert"));
        let r = run(&g, &format!("{P}{CHAIN}"), &format!("{P}:a a :C .\n"), &o);
        let j = serde_json::to_string(&r).unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        eprintln!(
            "    SAID: verdict={} warrant={} checker.status={}",
            v["per_goal"][0]["verdict"], v["per_goal"][0]["warrant"], v["checker"]["status"]
        );
        eprintln!("    INSTALL LINE: {}", v["checker"]["install"].as_str().unwrap_or(""));
        assert!(!j.contains("OOCert.certificate_sound"));
        assert_eq!(v["checker"]["status"], "absent");
    }

    // ── 4 ────────────────────────────────────────────────────────────────
    banner(4, "a rejected sub-certificate stops the line", "a checker stub that always exits 1");
    #[cfg(unix)]
    {
        let d = scratch("g4-stub");
        let stub = d.join("no");
        std::fs::write(&stub, "#!/bin/sh\necho '{\"ok\":false,\"first_rejected\":0}'\nexit 1\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        let g = loaded(&format!("{P}{CHAIN}"));
        let mut o = opts("g4", &[]);
        o.checker = Some(stub);
        let r = run(&g, &format!("{P}{CHAIN}"), &format!("{P}:a a :C .\n"), &o);
        let dis = r.disagreement.as_ref().unwrap();
        eprintln!(
            "    SAID: verdict={:?} severity={} what={} exit_code={}",
            r.per_goal[0].verdict, dis.severity, dis.what, r.exit_code
        );
        eprintln!("    DETAIL: {}", dis.detail);
        assert_eq!(r.per_goal[0].verdict, GoalVerdict::CertificateRejected);
        assert_eq!(dis.severity, "STOP_THE_LINE");
        assert_eq!(r.exit_code, 2);
    }

    // ── 5 ────────────────────────────────────────────────────────────────
    banner(5, "a non-subset projection disarms the differential", "a slice carrying `:Z rdfs:subClassOf :Y`, which the source lacks");
    {
        let g = loaded(&format!("{P}{CHAIN}"));
        let r = run(
            &g,
            &format!("{P}{CHAIN}:Z rdfs:subClassOf :Y .\n"),
            &format!("{P}:a a :C .\n"),
            &opts("g5", &[]),
        );
        eprintln!(
            "    SAID: subset.verified={} extra_count={} extras={:?} monotonicity={} reason={}",
            r.subset.verified,
            r.subset.extra_count,
            r.subset.extra_triples,
            r.monotonicity.status,
            r.monotonicity.reason
        );
        assert!(!r.subset.verified);
        assert_eq!(r.monotonicity.reason, "projection_is_not_a_subset");
        assert!(r.monotonicity.disagreement.is_none());
    }

    // ── 6 ────────────────────────────────────────────────────────────────
    banner(6, "a truncated run disarms the differential", "the iteration cap forced to 1 over an eight-link chain");
    {
        let mut ttl = String::from(P);
        for i in 0..8 {
            ttl.push_str(&format!(":C{i} rdfs:subClassOf :C{}.\n", i + 1));
        }
        ttl.push_str(":a a :C0 .\n");
        let g = loaded(&ttl);
        let prev = open_ontologies::runtime::reasoner_max_iterations();
        open_ontologies::runtime::set_reasoner_max_iterations(1);
        let r = run(&g, &ttl, &format!("{P}:a a :C8 .\n"), &opts("g6", &[]));
        open_ontologies::runtime::set_reasoner_max_iterations(prev);
        eprintln!(
            "    SAID: source_fixpoint_reached={} monotonicity={} reason={} bounded_by_iteration_cap={}",
            r.source_fixpoint_reached,
            r.monotonicity.status,
            r.monotonicity.reason,
            r.per_goal[0].bounded_by_iteration_cap
        );
        assert!(!r.source_fixpoint_reached);
        assert_eq!(r.monotonicity.reason, "fixpoint_not_reached");
    }

    // ── 7 ────────────────────────────────────────────────────────────────
    banner(7, "the differential can FIRE", "a hand-built pair where closure(P) exceeds closure(G), every guard true");
    {
        let t = |s: &str| (s.to_string(), "<p>".to_string(), "<o>".to_string());
        let src: std::collections::HashSet<pe::Spelled> = [t("<a>")].into_iter().collect();
        let prj: std::collections::HashSet<pe::Spelled> =
            [t("<a>"), t("<ghost>")].into_iter().collect();
        let r = pe::differential(
            &src,
            &prj,
            pe::Guards { subset_verified: true, blank_nodes_unmatched: false, both_reached_fixpoint: true },
        );
        let d = r.disagreement.as_ref().unwrap();
        eprintln!(
            "    SAID: status={} violations={:?} severity={} what={}",
            r.status, r.violations, d.severity, d.what
        );
        eprintln!("    MEANS: {}", d.means);
        assert_eq!(d.severity, "STOP_THE_LINE");
    }

    // ── 8 ────────────────────────────────────────────────────────────────
    banner(8, "a blank node and a closed-world claim are refused by name", "goals `:b :p [ :q :r ]` and `:npa a owl:NegativePropertyAssertion`");
    {
        let (goals, refused) = pe::parse_goals_turtle(
            "@prefix : <http://ex.org/> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             :a rdfs:subClassOf :A .\n\
             :b :p [ :q :r ] .\n\
             :npa a owl:NegativePropertyAssertion .\n",
        )
        .unwrap();
        for r in &refused {
            eprintln!("    SAID: refused {:?} <- {}", r.reason, r.as_written);
        }
        eprintln!("    MEANS: {}", refused.last().unwrap().means);
        let g = loaded(&format!("{P}{CHAIN}"));
        let r = pe::check_entailment_preservation(
            &g,
            Projection::Turtle(&format!("{P}{CHAIN}")),
            &goals,
            &refused,
            &opts("g8", &[]),
        )
        .unwrap();
        eprintln!("    COUNTED: refused={} goals_total={} exit_code={}", r.refused, r.goals_total, r.exit_code);
        assert_eq!(r.refused, 3);
        assert_eq!(r.exit_code, 1);
    }

    // ── 9 ────────────────────────────────────────────────────────────────
    banner(9, "an empty goal set is an error", "a goal document whose only triple carries a blank node");
    {
        let g = loaded(&format!("{P}{CHAIN}"));
        let (goals, refused) =
            pe::parse_goals_turtle("@prefix : <http://ex.org/> .\n:b :p [ :q :r ] .\n").unwrap();
        let err = pe::check_entailment_preservation(
            &g,
            Projection::Turtle(&format!("{P}{CHAIN}")),
            &goals,
            &refused,
            &opts("g9", &[]),
        )
        .unwrap_err();
        eprintln!("    SAID: {err}");
        assert!(err.to_string().contains("no goals to ask"));
    }

    // ── 10 ───────────────────────────────────────────────────────────────
    banner(10, "the raw literal spelling matches nothing", "the goal `\"01\"^^xsd:integer` asked WITHOUT the store's canonicalisation");
    {
        let ttl = "@prefix : <http://ex.org/> .\n\
                   @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
                   :a :n \"1\"^^xsd:integer .\n";
        let g = loaded(ttl);
        let o = opts("g10", &[]);
        let (goals, _) = pe::parse_goals_turtle(
            "@prefix : <http://ex.org/> .\n@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n:a :n \"01\"^^xsd:integer .\n",
        )
        .unwrap();
        let r =
            pe::check_entailment_preservation(&g, Projection::Turtle(ttl), &goals, &[], &o).unwrap();
        let raw = (
            "<http://ex.org/a>".to_string(),
            "<http://ex.org/n>".to_string(),
            "\"01\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_string(),
        );
        let cert = pe::CertificateIndex::read(&o.work_dir.join("projection")).unwrap();
        eprintln!(
            "    SAID: as_written={} -> goal={} verdict={:?}",
            r.per_goal[0].as_written, r.per_goal[0].goal, r.per_goal[0].verdict
        );
        eprintln!("    AND the raw spelling asked directly: {:?}", cert.membership(&raw));
        assert_eq!(r.per_goal[0].verdict, GoalVerdict::PreservedAsserted);
        assert_eq!(cert.membership(&raw), pe::Membership::NotDerivable);
    }

    // ── 11 ───────────────────────────────────────────────────────────────
    banner(11, "owl-dl is refused rather than silently downgraded", "profile owl-dl");
    {
        let g = loaded(&format!("{P}{CHAIN}"));
        let mut o = cd::DiffOptions { out: scratch("g11"), ..Default::default() };
        o.profile = "owl-dl".into();
        let err = cd::closure_diff(&g, &format!("{P}{CHAIN}"), &o).unwrap_err();
        eprintln!("    SAID: {err}");
        assert!(err.to_string().contains("no rule trace"));
    }

    // ── 12 ───────────────────────────────────────────────────────────────
    banner(12, "a rejected SOURCE certificate demotes every row", "a checker stub that always exits 1, over the closure diff");
    #[cfg(unix)]
    {
        let d = scratch("g12-stub");
        let stub = d.join("no");
        std::fs::write(&stub, "#!/bin/sh\necho '{\"ok\":false}'\nexit 1\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        let g = loaded(&format!("{P}{CHAIN}"));
        let o = cd::DiffOptions { out: scratch("g12"), checker: Some(stub), ..Default::default() };
        let r = cd::closure_diff(&g, &format!("{P}:a a :A .\n"), &o).unwrap();
        eprintln!(
            "    SAID: source_certificate.verdict={} checker_exit={:?} theorem={:?}",
            r.source_certificate.verdict, r.source_certificate.checker_exit, r.source_certificate.theorem
        );
        eprintln!("    SKIPPED: {}", r.source_certificate.skipped.clone().unwrap_or_default());
        assert_eq!(r.source_certificate.verdict, "rejected");
        assert!(r.entailments_lost.iter().all(|e| !e.warrant.is_checked()));
    }

    // ── 13 ───────────────────────────────────────────────────────────────
    banner(13, "a re-parsed blank node does not accuse the engine", "a slice that is the source re-serialised, with skolemising OFF");
    {
        let ttl = "@prefix : <http://ex.org/> .\n\
                   @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                   @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
                   :C rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :D ] .\n";
        let g = loaded(ttl);
        let o = cd::DiffOptions {
            out: scratch("g13"),
            skolemise_source: false,
            ..Default::default()
        };
        let r = cd::closure_diff(&g, &g.serialize("turtle").unwrap(), &o).unwrap();
        eprintln!(
            "    SAID: violations={} not_compared(source={}, projection={})",
            r.monotonicity_violations.len(),
            r.not_compared.source_triples,
            r.not_compared.projection_triples
        );
        eprintln!("    GATE SKIPPED BECAUSE: {}", r.monotonicity_gate_skipped.clone().unwrap_or_default());
        assert!(r.monotonicity_violations.is_empty());
        assert!(r.monotonicity_gate_skipped.is_some());
    }

    eprintln!("\n─── every gate above fired on input built to trip it.");
}
