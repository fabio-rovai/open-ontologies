//! The goal-free form: which CONCLUSIONS a projection preserves.
//!
//! `src/projection_entailment.rs` asks the question of a supplied goal set, for
//! auditing one answer. This asks it of everything at once, for auditing a
//! retrieval STRATEGY, because a strategy is not evaluated against one question.
//!
//! Every gate here is shown failing on deliberately broken input, and the
//! verdict discipline is enforced over the SERIALISED report rather than over
//! the enum, because the enum is where the discipline is easy and the
//! serialisation is where it leaks.

mod common;

use open_ontologies::closure_diff as cd;
use open_ontologies::closure_diff::DiffOptions;
use open_ontologies::graph::GraphStore;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// See `tests/lean_projection_entailment_test.rs`: the iteration cap is a
/// process-wide atomic and one test here moves it.
fn serial() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn lean_dir() -> PathBuf {
    repo().join("lean")
}

fn lake_available() -> bool {
    // `.current_dir(lean_dir())`: elan resolves the toolchain from the working
    // directory, and the crate root is the one directory with no
    // `lean-toolchain` in its ancestry.
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
        assert!(out.status.success(), "lake build failed:\n{}", String::from_utf8_lossy(&out.stderr));
        let exe = lean_dir().join(".lake").join("build").join("bin").join("oo-cert");
        assert!(exe.exists());
        exe
    })
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-cd-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn loaded(ttl: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(ttl, None).unwrap();
    g
}

fn opts(name: &str) -> DiffOptions {
    DiffOptions { out: scratch(name), ..Default::default() }
}

const P: &str = "@prefix : <http://ex.org/> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n";

/// A hundred triples on one seed, plus the `rdfs9` chain the answer needs.
fn hundred_plus_chain() -> String {
    let mut ttl = String::from(P);
    for i in 0..100 {
        ttl.push_str(&format!(":a :p{i} :o{i} .\n"));
    }
    ttl.push_str(":a a :A .\n:A rdfs:subClassOf :B .\n:B rdfs:subClassOf :C .\n");
    ttl
}

// ═══════════════════════════════════════════════════════════════════════════

/// The brief's first sentence, as an executable claim.
#[test]
fn a_ninety_nine_percent_slice_that_drops_the_load_bearing_triple_is_not_ok() {
    let _s = serial();
    if skip() {
        return;
    }
    checker();
    let ttl = hundred_plus_chain();
    let g = loaded(&ttl);
    // Everything except the single `:A rdfs:subClassOf :B` the chain needs.
    let slice: String = g
        .all_triples()
        .unwrap()
        .into_iter()
        .filter(|t| {
            !(t.0 == "<http://ex.org/A>"
                && t.1 == "<http://www.w3.org/2000/01/rdf-schema#subClassOf>")
        })
        .map(|t| format!("{} {} {} .\n", t.0, t.1, t.2))
        .collect();
    let mut o = opts("ninety-nine");
    o.seed_iris = vec!["http://ex.org/a".into()];
    let r = cd::closure_diff(&g, &slice, &o).unwrap();
    let ratio = r.coverage_proxy.report.as_ref().unwrap().aggregate_coverage_ratio;
    assert!(
        ratio > 0.98 && r.lost_in_projection_vocabulary > 0,
        "coverage {ratio} over a slice that lost {} conclusion(s) in its own vocabulary\n{}",
        r.lost_in_projection_vocabulary,
        r.headline
    );
    // The actionable half: the retriever dropped THIS, and the conclusion went
    // with it.
    // An asserted triple that was simply not retrieved has no derivation and
    // no blocking premises; it is a lookup that is gone. The actionable row is
    // a lost INFERENCE, which names the premise the retriever dropped.
    let row = r
        .entailments_lost
        .iter()
        .find(|e| e.in_projection_vocabulary && e.rule.is_some())
        .expect("a lost INFERENCE over the projection's own terms");
    assert!(
        !row.blocking_premises.is_empty(),
        "a lost inference must name the premises the projection does not hold: {row:?}"
    );
    assert!(
        row.blocking_premises
            .iter()
            .any(|p| p.1 == "<http://www.w3.org/2000/01/rdf-schema#subClassOf>"),
        "and the named premise is the subClassOf link that was dropped: {row:?}"
    );
    assert_eq!(r.exit_code, 1);
}

/// The converse. Both directions, or the metric is untested.
#[test]
fn a_sixty_percent_slice_that_preserves_every_conclusion_is_ok() {
    let _s = serial();
    if skip() {
        return;
    }
    checker();
    let ttl = hundred_plus_chain();
    let g = loaded(&ttl);
    // Keep the chain and about half the noise.
    let mut slice = String::new();
    for (i, t) in g.all_triples().unwrap().into_iter().enumerate() {
        let is_chain = t.1 == "<http://www.w3.org/2000/01/rdf-schema#subClassOf>"
            || t.1 == "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
        if is_chain || i % 2 == 0 {
            slice.push_str(&format!("{} {} {} .\n", t.0, t.1, t.2));
        }
    }
    let mut o = opts("sixty");
    o.seed_iris = vec!["http://ex.org/a".into()];
    let r = cd::closure_diff(&g, &slice, &o).unwrap();
    let ratio = r.coverage_proxy.report.as_ref().unwrap().aggregate_coverage_ratio;
    assert!(ratio < 0.75, "the slice really is partial: {ratio}");
    assert_eq!(
        r.lost_in_projection_vocabulary, 0,
        "a slice at {ratio} kept every conclusion over its own vocabulary, and lost {} over terms \
         it does not mention: {:?}\n{}",
        r.lost_total,
        r.entailments_lost.iter().take(3).collect::<Vec<_>>(),
        r.headline
    );
    assert_eq!(r.exit_code, 0);
}

/// The laundering guard. No skip guard: this needs the checker ABSENT, so it is
/// the one gate a machine without Lean still enforces.
#[test]
fn an_unchecked_run_never_prints_the_checked_word() {
    let _s = serial();
    let g = loaded(&format!("{P}:A rdfs:subClassOf :B . :B rdfs:subClassOf :C . :a a :A .\n"));
    let mut o = opts("unchecked");
    o.checker = Some(PathBuf::from("/nonexistent/oo-cert"));
    let r = cd::closure_diff(&g, &format!("{P}:a a :A .\n"), &o).unwrap();

    assert_eq!(r.source_certificate.verdict, "engine_opinion");
    assert_eq!(
        r.source_certificate.theorem, None,
        "a theorem may not be named by a run that never ran the checker"
    );
    assert!(
        r.source_certificate.skipped.as_deref().unwrap().contains("elan"),
        "the skip must carry the install line: {:?}",
        r.source_certificate
    );
    assert!(
        r.entailments_lost.iter().all(|e| !e.warrant.is_checked()),
        "an unchecked result printed the checked word"
    );
    let json = serde_json::to_string(&r).unwrap();
    assert!(
        !json.contains("OOCert.certificate_sound"),
        "the theorem name must not appear anywhere in an unchecked report: {json}"
    );
}

/// A lookup is not a theorem, and folding assertions into the checked count
/// would let a graph of 35,000 assertions and 3 inferences report "35,003
/// checked conclusions" on a machine with no Lean.
#[test]
fn an_assertion_is_never_counted_as_checked() {
    let _s = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{P}:A rdfs:subClassOf :B . :B rdfs:subClassOf :C . :a a :A .\n"));
    let r = cd::closure_diff(&g, &format!("{P}:x a :A .\n"), &opts("warrants")).unwrap();
    assert_eq!(r.source_certificate.verdict, "checked");
    assert!(r.lost_by_warrant.contains_key("asserted_in_source"), "{:?}", r.lost_by_warrant);
    assert!(r.lost_by_warrant.contains_key("checked"), "{:?}", r.lost_by_warrant);
    assert!(
        !r.lost_by_warrant.contains_key("total"),
        "the three counts are never summed into one: {:?}",
        r.lost_by_warrant
    );
    for e in &r.entailments_lost {
        assert_eq!(e.theorem.is_some(), e.warrant.is_checked());
    }
}

/// A rejected source certificate demotes EVERY row at once. Clean rule, no
/// per-step bookkeeping.
#[test]
#[cfg(unix)]
fn a_rejected_source_certificate_demotes_every_row() {
    let _s = serial();
    if skip() {
        return;
    }
    checker();
    let dir = scratch("rejecting");
    let stub = dir.join("always-no");
    std::fs::write(&stub, "#!/bin/sh\necho '{\"ok\":false}'\nexit 1\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let g = loaded(&format!("{P}:A rdfs:subClassOf :B . :B rdfs:subClassOf :C . :a a :A .\n"));
    let mut o = opts("rejected-src");
    o.checker = Some(stub);
    let r = cd::closure_diff(&g, &format!("{P}:x a :A .\n"), &o).unwrap();
    assert_eq!(r.source_certificate.verdict, "rejected");
    assert_eq!(r.source_certificate.checker_exit, Some(1));
    assert_eq!(r.source_certificate.theorem, None);
    assert!(
        r.entailments_lost.iter().all(|e| !e.warrant.is_checked()),
        "a rejected certificate leaves nothing checked"
    );
    let json = serde_json::to_string(&r).unwrap();
    assert!(!json.contains("OOCert.certificate_sound"), "{json}");
}

/// The engine cannot be made unsound on demand, so the gate is shown able to
/// fire by building a source closure with a hole the projection's closure
/// fills.
#[test]
fn the_monotonicity_gate_fires() {
    let _s = serial();
    let t = |s: &str, o: &str| (s.to_string(), "<p>".to_string(), o.to_string());
    let src: HashSet<cd::NtTriple> = [t("<a>", "<b>")].into_iter().collect();
    let prj: HashSet<cd::NtTriple> = [t("<a>", "<b>"), t("<a>", "<ghost>")].into_iter().collect();
    let r = open_ontologies::projection_entailment::differential(
        &src,
        &prj,
        open_ontologies::projection_entailment::Guards {
            subset_verified: true,
            blank_nodes_unmatched: false,
            both_reached_fixpoint: true,
        },
    );
    assert_eq!(r.status, "armed");
    assert_eq!(r.violation_count, 1);
    let d = r.disagreement.expect("a violation is a disagreement");
    assert_eq!(d.severity, "STOP_THE_LINE");
    assert!(d.means.contains("SOUNDNESS BUG IN THE ENGINE"), "{}", d.means);

    // And `from_parts` exists so the same hole can be built at the report level.
    let verdict = cd::CertificateVerdict {
        verdict: open_ontologies::verdict::ClosureVerdict::EngineOpinion,
        theorem: None,
        asserted: 1,
        derivations: 0,
        checker_report: None,
        checker_exit: None,
        skipped: Some("hand-built".into()),
        reached_fixpoint: true,
        iterations: 1,
        iteration_cap: 64,
    };
    let sc = cd::SourceClosure::from_parts(src, HashSet::new(), HashMap::new(), verdict);
    assert_eq!(sc.closure_size(), 1);
    assert_eq!(sc.verdict().verdict, "engine_opinion");
}

/// The trap, pinned on real data. RDFC-1.0 labels are a function of the WHOLE
/// graph, so canonicalising a source and a slice SEPARATELY and then diffing
/// them is not a fix for blank nodes: it is strictly worse than doing nothing.
///
/// Measured here on `benchmark/reference/pizza-reference.owl`, taking a genuine
/// subgraph (every triple not mentioning one class): canonicalising both sides
/// and diffing reports hundreds of triples of the SUBGRAPH as absent from the
/// SOURCE, while comparing the store's own labels reports none at all.
///
/// This test exists to stop a future implementer replacing `skolemise` with
/// `GraphStore::canonicalize_blank_nodes`, which is already in the repository,
/// is correct for "are these two graphs isomorphic", and looks principled.
#[test]
fn canonicalising_both_sides_separately_is_worse_than_doing_nothing() {
    let _s = serial();
    let g = Arc::new(GraphStore::new());
    g.load_file(
        &repo()
            .join("benchmark/reference/pizza-reference.owl")
            .display()
            .to_string(),
    )
    .unwrap();
    let full = g.all_triples().unwrap();
    let sub: Vec<_> = full
        .iter()
        .filter(|t| !t.0.contains("Veneziana") && !t.2.contains("Veneziana"))
        .cloned()
        .collect();
    assert!(sub.len() < full.len(), "the subgraph must really be smaller");

    let nt = |v: &Vec<(String, String, String)>| -> String {
        v.iter().map(|t| format!("{} {} {} .\n", t.0, t.1, t.2)).collect()
    };
    let a = GraphStore::new();
    a.load_ntriples(&nt(&full)).unwrap();
    let b = GraphStore::new();
    b.load_ntriples(&nt(&sub)).unwrap();
    let ca: HashSet<_> = a.canonicalize_blank_nodes().unwrap().all_triples().unwrap().into_iter().collect();
    let cb: HashSet<_> = b.canonicalize_blank_nodes().unwrap().all_triples().unwrap().into_iter().collect();
    let canonical_extras = cb.difference(&ca).count();

    let sa: HashSet<_> = full.iter().cloned().collect();
    let sb: HashSet<_> = sub.iter().cloned().collect();
    let raw_extras = sb.difference(&sa).count();

    assert_eq!(raw_extras, 0, "the store's own labels make the subgraph a subset, as it is");
    assert!(
        canonical_extras > 100,
        "canonicalising each side separately produced {canonical_extras} spurious extras on a \
         real subgraph, against {raw_extras} for the store's own labels. If this number is now \
         small, say what changed in RDFC-1.0 or in this repository; until then, canonicalising \
         both sides and diffing is worse than not canonicalising at all, and it looks \
         principled, which is why the trap needs a test rather than a comment"
    );
    eprintln!(
        "RDFC-1.0 SEPARATELY: {canonical_extras} spurious extras out of {} subgraph triples; \
         store labels: {raw_extras}",
        cb.len()
    );
}

/// A re-parsed slice mints fresh blank node labels, so every blank-node-bearing
/// triple looks like an addition. Skolemising the source first is what makes
/// the comparison possible at all. This is the test that decides whether the
/// free differential is usable on real ontologies.
#[test]
fn a_relabelled_blank_node_is_not_a_monotonicity_violation() {
    let _s = serial();
    if skip() {
        return;
    }
    checker();
    let ttl = "@prefix : <http://ex.org/> .\n\
               @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
               @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
               :C rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :D ] .\n\
               :E rdfs:subClassOf :C .\n";
    let g = loaded(ttl);

    // Without skolemising: the slice is the same graph, re-serialised, and its
    // blank nodes are new. Nothing may be accused of anything.
    let mut o = opts("bnode-off");
    o.skolemise_source = false;
    let slice = g.serialize("turtle").unwrap();
    let r = cd::closure_diff(&g, &slice, &o).unwrap();
    assert!(r.monotonicity_violations.is_empty(), "{:?}", r.monotonicity_violations);
    assert!(
        r.monotonicity_gate_skipped.is_some(),
        "an empty violation list must never mean 'we did not look'"
    );
    assert!(r.not_compared.projection_triples > 0, "{:?}", r.not_compared);
    assert!(r.not_compared.why.contains("NP-complete"), "{}", r.not_compared.why);

    // With skolemising: nothing is uncompared and nothing is lost.
    let mut o2 = opts("bnode-on");
    o2.skolemise_source = true;
    let src = cd::SourceClosure::build(&g, &o2).unwrap();
    let slice2 = src.store().serialize("turtle").unwrap();
    let r2 = src.diff(&slice2, &o2).unwrap();
    assert_eq!(r2.not_compared.projection_triples, 0, "{:?}", r2.not_compared);
    assert_eq!(r2.not_compared.source_triples, 0, "{:?}", r2.not_compared);
    assert_eq!(r2.lost_total, 0, "{:?}", r2.entailments_lost);
    assert!(r2.monotonicity_gate_skipped.is_none(), "{:?}", r2.monotonicity_gate_skipped);
    assert!(r2.skolemised > 0);
}

/// "The retriever produced it from the source, so it is a subset" is false in
/// practice. Retrievers normalise IRIs, re-prefix, inline an imported
/// vocabulary, and add a tidy declaration nobody wrote.
#[test]
fn a_projection_that_is_not_a_subset_is_a_retrieval_finding() {
    let _s = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{P}:A rdfs:subClassOf :B .\n"));
    let r = cd::closure_diff(
        &g,
        &format!("{P}:A rdfs:subClassOf :B .\n:Invented rdfs:subClassOf :Nonsense .\n"),
        &opts("non-subset"),
    )
    .unwrap();
    assert!(!r.subset.verified);
    assert_eq!(r.subset.extra_count, 1);
    assert!(
        r.subset.extra_triples.iter().any(|t| t.contains("Invented")),
        "the extra triple must be listed: {:?}",
        r.subset
    );
    assert!(r.monotonicity_violations.is_empty(), "a non-subset accuses nobody of unsoundness");
    let why = r.monotonicity_gate_skipped.expect("the gate must say why it did not run");
    assert!(why.contains("not a subset"), "{why}");
    assert!(why.contains("RETRIEVAL finding"), "{why}");
}

/// A truncated source closure would fire a false STOP_THE_LINE on every long
/// subclass chain.
#[test]
fn the_iteration_cap_suppresses_the_soundness_alarm() {
    let _s = serial();
    if skip() {
        return;
    }
    checker();
    let mut ttl = String::from(P);
    for i in 0..8 {
        ttl.push_str(&format!(":C{i} rdfs:subClassOf :C{}.\n", i + 1));
    }
    ttl.push_str(":a a :C0 .\n");
    let g = loaded(&ttl);
    let previous = open_ontologies::runtime::reasoner_max_iterations();
    open_ontologies::runtime::set_reasoner_max_iterations(1);
    let r = cd::closure_diff(&g, &ttl, &opts("cap"));
    open_ontologies::runtime::set_reasoner_max_iterations(previous);
    let r = r.unwrap();
    assert!(!r.source_certificate.reached_fixpoint, "one round cannot close an eight-link chain");
    assert_eq!(r.source_certificate.iteration_cap, 1);
    let why = r.monotonicity_gate_skipped.expect("the gate must say why it did not run");
    assert!(why.contains("cap"), "{why}");
    assert!(r.monotonicity_violations.is_empty());
}

#[test]
fn owl_dl_is_refused() {
    let _s = serial();
    let g = loaded(&format!("{P}:A rdfs:subClassOf :B .\n"));
    let mut o = opts("owl-dl");
    o.profile = "owl-dl".into();
    let err = cd::closure_diff(&g, &format!("{P}:A rdfs:subClassOf :B .\n"), &o)
        .expect_err("owl-dl has no rule trace and must say so");
    let m = err.to_string();
    assert!(m.contains("no rule trace"), "{m}");
    assert!(m.contains("owl-rl-ext"), "the message must name the profiles that do work: {m}");
}

/// Mechanical, cheap, and the only thing that stops the warning being dropped
/// by a future renderer.
#[test]
fn the_coverage_warning_travels_with_the_number() {
    let _s = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{P}:A rdfs:subClassOf :B .\n"));
    let mut o = opts("label");
    o.seed_iris = vec!["http://ex.org/A".into()];
    let r = cd::closure_diff(&g, &format!("{P}:A rdfs:subClassOf :B .\n"), &o).unwrap();
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("coverage_ratio"), "the number is present: {json}");
    assert!(
        json.contains("neither necessary nor sufficient"),
        "and so is the label: {json}"
    );
    assert!(json.contains("\"is_a_warrant\":false"), "{json}");
    assert!(
        json.contains("gaming direction"),
        "the headline's own gaming direction must be in the PAYLOAD, not only the docs: {json}"
    );
}

/// The closure diff reads `all_triples` and the certificate TSVs, so it
/// inherits none of `neighbourhood_pairs`'s `LIMIT 1000`.
#[test]
fn a_seed_with_more_than_a_thousand_triples_is_counted_in_full() {
    let _s = serial();
    if skip() {
        return;
    }
    checker();
    let mut ttl = String::from(P);
    for i in 0..1500 {
        ttl.push_str(&format!(":a :p{i} :o{i} .\n"));
    }
    let g = loaded(&ttl);
    let mut o = opts("limit");
    o.seed_iris = vec!["http://ex.org/a".into()];
    let r = cd::closure_diff(&g, &format!("{P}:a :p0 :o0 .\n"), &o).unwrap();
    assert_eq!(
        r.lost_total, 1499,
        "the closure diff counts every triple, not the first thousand: {}",
        r.headline
    );
    // And the proxy, which does truncate, says so.
    assert_eq!(
        r.coverage_proxy.truncated_seeds.len(),
        1,
        "the seed whose ratio was computed against a truncated source must be named: {:?}",
        r.coverage_proxy
    );
}

/// A cached verdict read off disk would be an unchecked result wearing the
/// checked word one indirection away, so `load` re-runs the checker.
#[test]
fn a_reloaded_cache_re_runs_the_checker() {
    let _s = serial();
    if skip() {
        return;
    }
    checker();
    let g = loaded(&format!("{P}:A rdfs:subClassOf :B . :B rdfs:subClassOf :C . :a a :A .\n"));
    let o = opts("reload");
    let built = cd::SourceClosure::build(&g, &o).unwrap();
    assert_eq!(built.verdict().verdict, "checked");

    let mut o2 = DiffOptions { out: o.out.clone(), ..Default::default() };
    o2.checker = Some(PathBuf::from("/nonexistent/oo-cert"));
    let reloaded = cd::SourceClosure::load(&o.out.join("source"), &o2).unwrap();
    assert_eq!(
        reloaded.verdict().verdict,
        "engine_opinion",
        "a reload with no checker must NOT inherit the checked word from the directory"
    );
    assert_eq!(
        reloaded.asserted_digest(),
        built.asserted_digest(),
        "the digest identifies the graph the cache was built from"
    );
}
