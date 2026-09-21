//! The monotonicity differential, run for real over every ontology this
//! repository ships.
//!
//! OWL RL is monotone and a projection is a subset of its source, so every
//! entailment of the projection must be an entailment of the source. If the
//! engine ever reports something entailed by the projection and not by the
//! source, that is not a retrieval finding: it is a SOUNDNESS BUG IN THE
//! ENGINE, and the tool treats it as stop-the-line rather than a curiosity.
//! That gives a defect detector for free, on every real run.
//!
//! It is worth nothing unless it actually runs on real input, so this test
//! walks the corpus `git ls-files` reports, skolemises each ontology, retrieves
//! a real slice with `onto_segment_retrieve`, and diffs the two closures with
//! the gate ARMED. What it found is printed, including if it found nothing:
//! "we looked at 240 ontologies and the engine was sound on all of them" is a
//! result, and "the gate was disarmed on 240 ontologies" would be a different
//! and much weaker one, so the two are counted separately and neither is
//! allowed to pass for the other.
//!
//! The corpus is enumerated, not hand-picked, and anything excluded is named
//! with the reason, the same discipline `tests/lean_certificate_test.rs` keeps.

mod common;

use open_ontologies::closure_diff as cd;
use open_ontologies::graph::GraphStore;
use open_ontologies::projection_entailment as pe;
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
        lean_dir().join(".lake").join("build").join("bin").join("oo-cert")
    })
}

/// Files over this are excluded and NAMED. The certificate test uses 4 MB for
/// one reasoner run per file; this does two plus a retrieval per file, so the
/// cap is lower and the difference is stated rather than silently inherited.
const SIZE_CAP: u64 = 1024 * 1024;

const SKIP_DIRS: [&str; 8] = [
    "node_modules", "target", ".lake", ".git", ".venv", "venv", "site-packages", "__pycache__",
];

/// The RDF files this repository actually contains, from `git ls-files`.
///
/// Enumerating from git rather than from the filesystem is what makes "every
/// RDF file in this repository" a statement with one meaning: a filesystem walk
/// also sees whatever a developer has generated locally, and the run stops
/// being comparable between machines.
fn corpus() -> Vec<PathBuf> {
    let out = Command::new("git")
        .args(["ls-files", "-z", "*.ttl", "*.owl", "*.rdf", "*.nt"])
        .current_dir(repo())
        .output()
        .expect("run git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    let mut files: Vec<PathBuf> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        // `tests/fixtures/clash-coverage/` holds one contradictory graph per clash
        // detector, each written to make exactly that detector fire (#160), plus a
        // near-miss twin. They are test data and not ontologies: swept as corpus they
        // would turn a measurement of the shipped corpus into a statement about this
        // repository's own fixtures, which is measuring the ruler. The same reasoning
        // and the same treatment as `tests/fixtures/horn-coverage/`.
        // `tests/clash_detector_coverage_test.rs` asserts this exclusion is still here.
        .filter(|p| !p.starts_with("tests/fixtures/clash-coverage/"))
        // `docs/assets/claims/` holds the tiny ontologies the certified-claims
        // figure is drawn from. They exist to demonstrate a downgrade and a
        // disagreement, not to be ontologies, and counted as corpus they would
        // dilute a measurement of what this repository actually ships. Same
        // reasoning and same treatment as `tests/fixtures/horn-coverage/`.
        .filter(|p| !p.starts_with("docs/assets/claims/"))
        .filter(|p| !SKIP_DIRS.iter().any(|d| p.split('/').any(|seg| seg == *d)))
        .map(|p| repo().join(p))
        .collect();
    files.sort();
    files
}

/// Up to `n` seeds: the non-blank subjects with the most outbound triples, so
/// the retrieved slice is about the busiest part of the ontology rather than
/// an arbitrary corner.
fn seeds(g: &Arc<GraphStore>, n: usize) -> Vec<String> {
    let q = format!(
        "SELECT ?s (COUNT(*) AS ?c) WHERE {{ ?s ?p ?o . FILTER(!isBlank(?s)) }} \
         GROUP BY ?s ORDER BY DESC(?c) ?s LIMIT {n}"
    );
    let Ok(js) = g.sparql_select(&q) else { return Vec::new() };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&js) else { return Vec::new() };
    v["results"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|r| r["s"].as_str())
                .filter(|s| s.starts_with('<'))
                .map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Default)]
struct Tally {
    swept: usize,
    armed: usize,
    disarmed: Vec<(String, String)>,
    violations: Vec<(String, String)>,
    incompleteness: Vec<(String, String)>,
    excluded: Vec<(String, String)>,
    source_checked: usize,
    source_unchecked: Vec<(String, String)>,
    total_lost: usize,
    total_lost_in_vocab: usize,
    empty_slices: usize,
}

/// Every shipped ontology, skolemised, sliced by the real retriever, and
/// diffed with the monotonicity gate live.
#[test]
fn the_monotonicity_differential_over_every_shipped_ontology() {
    if skip() {
        return;
    }
    checker();
    let started = std::time::Instant::now();
    let mut t = Tally::default();

    for path in corpus() {
        let rel = path.strip_prefix(repo()).unwrap().display().to_string();
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size > SIZE_CAP {
            t.excluded.push((rel, format!("{size} bytes, over the {SIZE_CAP} byte cap")));
            continue;
        }
        let raw = Arc::new(GraphStore::new());
        if let Err(e) = raw.load_file(&path.display().to_string()) {
            t.excluded
                .push((rel, format!("does not parse: {}", e.to_string().lines().next().unwrap_or(""))));
            continue;
        }
        if raw.triple_count() == 0 {
            t.excluded.push((rel, "holds no triples".into()));
            continue;
        }

        let out = std::env::temp_dir()
            .join(format!("oo-mono-{}-{}", std::process::id(), t.swept));
        let _ = std::fs::remove_dir_all(&out);
        let opts = cd::DiffOptions {
            profile: "owl-rl-ext".into(),
            out: out.clone(),
            checker: None,
            skolemise_source: true,
            max_rows: 5,
            seed_iris: Vec::new(),
        };
        // Skolemise first, then retrieve FROM the skolemised store. Without
        // that the slice carries blank nodes the source no longer has, every
        // blank-node-bearing triple looks like an addition, and the gate is
        // suppressed for exactly the ontologies that need it most.
        let src = match cd::SourceClosure::build(&raw, &opts) {
            Ok(s) => s,
            Err(e) => {
                t.excluded.push((rel, format!("closure: {e}")));
                continue;
            }
        };
        let sk = src.store().clone();
        let s = seeds(&sk, 12);
        if s.is_empty() {
            t.excluded.push((rel, "no non-blank subject to seed a retrieval from".into()));
            let _ = std::fs::remove_dir_all(&out);
            continue;
        }
        let seg = match open_ontologies::segment_retrieve::retrieve_segment(&sk, &s, 2, false) {
            Ok(x) => x,
            Err(e) => {
                t.excluded.push((rel, format!("retrieval: {e}")));
                let _ = std::fs::remove_dir_all(&out);
                continue;
            }
        };
        if seg.turtle.trim().is_empty() {
            t.empty_slices += 1;
            let _ = std::fs::remove_dir_all(&out);
            continue;
        }
        let r = match src.diff(&seg.turtle, &opts) {
            Ok(r) => r,
            Err(e) => {
                t.excluded.push((rel, format!("diff: {e}")));
                let _ = std::fs::remove_dir_all(&out);
                continue;
            }
        };
        t.swept += 1;
        t.total_lost += r.lost_total;
        t.total_lost_in_vocab += r.lost_in_projection_vocabulary;
        match &r.monotonicity_gate_skipped {
            None => t.armed += 1,
            Some(why) => t.disarmed.push((rel.clone(), why.chars().take(120).collect())),
        }
        for v in &r.monotonicity_violations {
            t.violations.push((rel.clone(), format!("{} {} {}", v.triple.0, v.triple.1, v.triple.2)));
        }
        for w in r.incompleteness_warnings.iter().take(2) {
            t.incompleteness
                .push((rel.clone(), format!("{:?} {} {} {}", w.rule, w.triple.0, w.triple.1, w.triple.2)));
        }
        if r.source_certificate.verdict == "checked" {
            t.source_checked += 1;
        } else {
            t.source_unchecked.push((
                rel.clone(),
                r.source_certificate.skipped.clone().unwrap_or_default().chars().take(120).collect(),
            ));
        }
        let _ = std::fs::remove_dir_all(&out);
    }

    // ── Report what it found, including if it found nothing ──────────────
    eprintln!("\n══ MONOTONICITY SWEEP over the shipped corpus ══");
    eprintln!("swept                     {}", t.swept);
    eprintln!("gate ARMED                {}", t.armed);
    eprintln!("gate DISARMED             {}", t.disarmed.len());
    eprintln!("source certificate CHECKED {}", t.source_checked);
    eprintln!("VIOLATIONS (stop-the-line) {}", t.violations.len());
    eprintln!("incompleteness warnings   {}", t.incompleteness.len());
    eprintln!("conclusions lost, total   {}", t.total_lost);
    eprintln!("  of them in the slice's own vocabulary {}", t.total_lost_in_vocab);
    eprintln!("empty slices (no retrieval) {}", t.empty_slices);
    eprintln!("excluded                  {}", t.excluded.len());
    eprintln!("seconds                   {:.1}", started.elapsed().as_secs_f64());
    for (f, why) in t.disarmed.iter().take(10) {
        eprintln!("DISARMED {f}: {why}");
    }
    for (f, why) in t.source_unchecked.iter().take(10) {
        eprintln!("UNCHECKED {f}: {why}");
    }
    for (f, w) in t.incompleteness.iter().take(10) {
        eprintln!("WARN {f}: {w}");
    }
    for (f, v) in &t.violations {
        eprintln!("STOP_THE_LINE {f}: {v}");
    }
    for (f, why) in t.excluded.iter().take(20) {
        eprintln!("EXCLUDED {f}: {why}");
    }

    // ── What this run is allowed to conclude ─────────────────────────────
    assert!(
        t.swept >= 100,
        "too few ontologies actually swept ({}); excluded {:?}",
        t.swept,
        t.excluded.iter().take(10).collect::<Vec<_>>()
    );
    assert!(
        t.armed * 2 >= t.swept,
        "the gate was disarmed on more than half the corpus ({} armed of {} swept), so this run \
         establishes very little about the engine. A disarmed gate is not a pass: {:?}",
        t.armed,
        t.swept,
        t.disarmed.iter().take(5).collect::<Vec<_>>()
    );
    assert!(
        t.source_checked * 2 >= t.swept,
        "fewer than half the source certificates were CHECKED ({} of {}), so most rows carry the \
         engine's opinion rather than a theorem: {:?}",
        t.source_checked,
        t.swept,
        t.source_unchecked.iter().take(5).collect::<Vec<_>>()
    );
    assert!(
        t.violations.is_empty(),
        "OWL RL is monotone and every projection here IS a subset, so a conclusion of a \
         projection that is not a conclusion of its source is a SOUNDNESS BUG IN THE ENGINE, not \
         a retrieval finding. {} found:\n{}",
        t.violations.len(),
        t.violations
            .iter()
            .map(|(f, v)| format!("  {f}: {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        t.total_lost > 0,
        "every slice preserved every conclusion of its source, which means the retrieval was not \
         actually lossy and this sweep tested nothing"
    );
}

/// The same sweep's precondition, stated as its own gate: a skolemised source
/// makes the retriever's slice a verified subset, which is what arms the
/// differential at all.
#[test]
fn skolemising_the_source_makes_a_retrieved_slice_a_verified_subset() {
    if skip() {
        return;
    }
    checker();
    let mut checked = 0usize;
    let mut not_subset: Vec<String> = Vec::new();
    for rel in [
        "benchmark/reference/pizza-reference.owl",
        "benchmark/reference/ies4.ttl",
        "case-studies/foundry-owl-crosswalk/vendor/ies-common.ttl",
    ] {
        let path = repo().join(rel);
        if !path.exists() {
            continue;
        }
        let raw = Arc::new(GraphStore::new());
        if raw.load_file(&path.display().to_string()).is_err() {
            continue;
        }
        let (sk, map) = pe::skolemise(&raw).unwrap();
        let s = seeds(&sk, 12);
        let seg = open_ontologies::segment_retrieve::retrieve_segment(&sk, &s, 2, false).unwrap();
        assert!(
            !seg.turtle.contains("<_:"),
            "{rel}: a skolemised source must not produce an angle-bracketed blank node"
        );
        let p = GraphStore::new();
        p.load_turtle(&seg.turtle, None).unwrap_or_else(|e| {
            panic!("{rel}: the slice must parse, or the auditor sees 0% coverage: {e}")
        });
        let src: std::collections::HashSet<_> = sk.all_triples().unwrap().into_iter().collect();
        let extra = p.all_triples().unwrap().into_iter().filter(|t| !src.contains(t)).count();
        eprintln!(
            "{rel}: {} skolem constants, {} triples retrieved, {extra} not in the source",
            map.len(),
            seg.triple_count
        );
        if extra > 0 {
            not_subset.push(format!("{rel}: {extra} extra"));
        }
        checked += 1;
    }
    assert!(checked > 0, "no fixture was available to check");
    assert!(
        not_subset.is_empty(),
        "a slice retrieved from the skolemised source must be a subset of it: {not_subset:?}"
    );
}
