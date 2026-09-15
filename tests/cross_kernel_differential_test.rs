//! Two proof assistants, one certificate format, and the question of whether the two
//! formalisations actually say the same thing.
//!
//! `lean/OOCert/Horn.lean` and `isabelle/OO_Check.thy` are independent formalisations of
//! the same Horn-certificate checker. The Isabelle one was written from the W3C primary
//! sources and the fixture DATA, with nothing under `lean/` read; a transliteration would
//! have been worthless, because a shared definitional bug survives translation untouched.
//! This file runs both over the same bytes and requires them to agree.
//!
//! # What agreement here is, and is not
//!
//! Checking a Horn certificate is PURELY SYNTACTIC. Neither checker consults its own
//! semantics: no interpretation, no domain, no truth. Two checkers built on contradictory
//! model theories agree on every certificate and on every forgery. So agreement in this
//! file is evidence about the FILE FORMAT and the CHECKING DISCIPLINE — how a binding is
//! read, whether premise order is part of the contract, what an out-of-range index is —
//! and it is not a proof of anything. It does not show the two semantics agree, it does
//! not add a theorem to either side, and it must never be reported as if it did.
//!
//! The evidence that the DEFINITIONS agree lives elsewhere: in which rule arms each side
//! can discharge and from which conditions (`isabelle/OO_Builtin_Sound.thy`,
//! `lean/OOCert/HornBuiltin.lean`), and in whether the conditions describe anything at
//! all (`isabelle/OO_NonVacuity.thy`, `lean/OOCert/HornWitness.lean`).
//!
//! What agreement here DOES buy is real and worth having: wherever the two checkers
//! accept and reject the same files, the certificate format has one meaning rather than
//! two. Where they do not, it has two, and that is the interesting part.
//!
//! # The disagreements, and what closed them
//!
//! Four were found, in two classes, and none was papered over. They are pinned by
//! `resolved_d1_*`, `resolved_d2_*`, `resolved_d2b_*` and `resolved_d2c_*` below, their
//! certificates are committed under `isabelle/fixtures-differential/` and
//! `isabelle/fixtures-added/`, and the corpus test now requires ZERO divergence and fails
//! on any row at all. Read those tests: the analysis is there, not here.
//!
//! Three were designed after reading both checkers. The fourth was found by the fuzzer,
//! in a rule file one character away from a probe written for something else, which is
//! why `binding_defect` works from the cause rather than from which edit produced the row:
//! a quarantine keyed on the edit would have hidden it.
//!
//! All four had ONE root cause. **Isabelle validated the binding list as a data structure
//! and Lean did not.** `check_step` requires `distinct (map fst b)` and
//! `binding_covers b r` before it instantiates anything; Lean's `substOf` is
//! `fun v => (List.lookup v l).getD v`, which turns any binding list into a total
//! function with a silent default and used to be applied without the list being inspected
//! at all. D1 is the duplicate key, D2 the missing one.
//!
//! Neither checker was unsound. In all four Lean accepted and Isabelle rejected, and
//! Lean's acceptance held in Lean's own theorem, because `EntailsR` quantifies over every
//! total substitution and the one Lean used is a real one; Isabelle's rejections were
//! false alarms, which is the harmless direction. What was defective is the FORMAT: it did
//! not say whether a binding must have distinct keys or must cover the cited rule's
//! variables, so Lean's answer to the first was whatever `List.lookup` happens to do, and
//! its answer to the second fabricated a term out of a variable's name and put it in the
//! conclusion. A certificate's validity depended on which verified checker read it.
//!
//! **`docs/decisions/0008-a-binding-is-data-and-evidence-admits-one-reading.md` settles
//! it: both shapes are refused.** The LEAN moved, and `bindingWellFormed` is now a conjunct
//! of `checkHornStep`, because the Isabelle was written from the W3C sources without reading
//! the Lean and is worth nothing once it is edited to agree. 47 rows of the corpus diverged
//! before it and none does after, and the 47 moved into rejected-by-both and nowhere else.
//! `the_two_kernels_agree_on_the_whole_corpus` now requires BOTH kernels to refuse every row
//! carrying a malformed binding, which is 286 of them and not only the 47, so the analysis
//! that used to excuse a disagreement is a gate that can fail.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

// ── where things live ───────────────────────────────────────────────────────

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn lean_dir() -> PathBuf {
    repo().join("lean")
}

fn isabelle_dir() -> PathBuf {
    repo().join("isabelle")
}

fn horn_fixture(name: &str) -> PathBuf {
    repo().join("tests").join("fixtures").join("horn").join(name)
}

fn differential_fixture(name: &str) -> PathBuf {
    isabelle_dir().join("fixtures-differential").join(name)
}

// ── availability, and a loud skip ───────────────────────────────────────────

fn lake_available() -> bool {
    // `.current_dir(lean_dir())`: elan resolves the toolchain from the working
    // directory, and the crate root has no `lean-toolchain` in its ancestry.
    Command::new("lake")
        .arg("--version")
        .current_dir(lean_dir())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A Poly/ML library directory is one this test can actually LINK against, which is
/// three files and not one. Ubuntu's `polyml` package is the reason the distinction is
/// checked rather than assumed: it installs `/usr/bin/poly` and `libpolymain.a` and no
/// `libpolyml.a` at all, so a search that stopped at `poly` would report the Isabelle
/// half available and then fail in the linker with nothing saying what was missing.
fn poly_usable(dir: &Path) -> bool {
    dir.join("poly").exists()
        && dir.join("libpolymain.a").exists()
        && dir.join("libpolyml.a").exists()
}

/// The Isabelle side needs Poly/ML to link the exported checker, and the exported
/// checker itself, which `isabelle/export.sh` writes out of the session and which is
/// committed so this test does not need a full Isabelle build to run.
///
/// The same search `isabelle/build_native.sh` does, in the same order and for the same
/// reasons. `POLYDIR` wins, because it is the answer that is right on a machine nobody
/// anticipated and it is how CI points at the Poly/ML component it unpacks. This test
/// was macOS-only until 15 September 2026, and an `/Applications` scan is not a search
/// on a Linux runner. Then `isabelle getenv ISABELLE_HOME`, then the macOS app bundle,
/// then a `poly` on PATH with its libraries beside it.
fn poly_dir() -> Option<PathBuf> {
    // Whatever build_native.sh would link against, this test must agree with, or the
    // skip decision and the build decision are made on different evidence.
    if let Ok(dir) = std::env::var("POLYDIR")
        && !dir.is_empty()
    {
        let dir = PathBuf::from(dir);
        return poly_usable(&dir).then_some(dir);
    }

    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "x86_64" };
    let os = if cfg!(target_os = "macos") { "darwin" } else { "linux" };
    let platform = format!("{arch}-{os}");

    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(out) = Command::new("isabelle").args(["getenv", "-b", "ISABELLE_HOME"]).output()
        && out.status.success()
    {
        let home = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !home.is_empty() {
            roots.push(PathBuf::from(home));
        }
    }
    if let Ok(entries) = std::fs::read_dir("/Applications") {
        roots.extend(entries.flatten().map(|e| e.path()));
    }

    let mut found: Vec<PathBuf> = Vec::new();
    for root in roots {
        let contrib = root.join("contrib");
        let Ok(cs) = std::fs::read_dir(&contrib) else { continue };
        for c in cs.flatten() {
            if c.file_name().to_string_lossy().starts_with("polyml-") {
                let d = c.path().join(&platform);
                if poly_usable(&d) {
                    found.push(d);
                }
            }
        }
    }
    found.sort();
    if let Some(d) = found.pop() {
        return Some(d);
    }

    // A from-source Poly/ML keeps its archives next to its binary.
    let out = Command::new("sh").args(["-c", "command -v poly"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let exe = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    let dir = exe.parent()?.to_path_buf();
    poly_usable(&dir).then_some(dir)
}

fn isabelle_available() -> bool {
    poly_dir().is_some() && isabelle_dir().join("driver").join("oo_horn_generated.ML").exists()
}

/// Both halves are needed, and a skip has to say WHICH half is missing. A test that
/// returns early reports `ok`, so `common::skip_unless` prints a marker and, under
/// `OO_REQUIRE_FIXTURES=1`, turns the skip into a failure.
fn skip() -> bool {
    if common::skip_unless(
        lake_available(),
        "lake (the Lean 4 build tool), for the Lean half of the differential",
        "install elan from https://github.com/leanprover/elan; lean/lean-toolchain pins the version",
    ) {
        return true;
    }
    common::skip_unless(
        isabelle_available(),
        "Poly/ML and isabelle/driver/oo_horn_generated.ML, for the Isabelle half of the \
         differential",
        "the generated ML is committed, so the full Isabelle distribution is NOT needed: \
         unpack https://isabelle.in.tum.de/components/polyml-5.9.2-2.tar.gz and set \
         POLYDIR to its platform directory, which is what CI does. Installing \
         Isabelle2025-2 also works and is what isabelle/export.sh needs to regenerate \
         the ML from the session",
    )
}

// ── building the two checkers, once per test binary ─────────────────────────

/// The Lean proofs are part of `lake build`, so a failure here is a failure and never a
/// skip: the binary that comes out of it is the one the theorems are about.
fn lean_checker() -> &'static Path {
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
        let exe = lean_dir().join(".lake").join("build").join("bin").join("oo-horn");
        assert!(exe.exists(), "checker binary missing at {}", exe.display());
        exe
    })
}

/// The Isabelle side links the code `export_code` wrote into a native executable. The
/// LINK is what happens here; nothing is re-derived from the theories, and the theories
/// are checked by `isabelle build -d isabelle -c OOHorn`, which this test does not run.
fn isabelle_checker() -> &'static Path {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        let script = isabelle_dir().join("build_native.sh");
        let out = Command::new("sh")
            .arg(&script)
            .current_dir(repo())
            .output()
            .expect("run isabelle/build_native.sh");
        assert!(
            out.status.success(),
            "isabelle/build_native.sh failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let exe = isabelle_dir().join("build").join("oo-horn-isabelle");
        assert!(exe.exists(), "checker binary missing at {}", exe.display());
        exe
    })
}

// ── one run of one checker ──────────────────────────────────────────────────

/// What a run of either checker says, reduced to the three things that are comparable.
/// `exit` is the discipline both sides implement: 0 accepted, 1 rejected, 2 unreadable or
/// unparseable. `verdict` is present only when the run accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Outcome {
    exit: i32,
    verdict: Option<String>,
}

fn json_field(json: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let start = json.find(&pat)? + pat.len();
    let rest = &json[start..];
    Some(rest[..rest.find('"')?].to_string())
}

fn run_lean(rules: &Path, asserted: &Path, cert: &Path) -> Outcome {
    let out = Command::new(lean_checker())
        .arg("check")
        .arg(rules)
        .arg(asserted)
        .arg(cert)
        .output()
        .expect("run oo-horn");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let exit = out.status.code().unwrap_or(-1);
    Outcome { exit, verdict: if exit == 0 { json_field(&stdout, "verdict") } else { None } }
}

fn run_isabelle(rules: &Path, asserted: &Path, cert: &Path) -> Outcome {
    let out = Command::new(isabelle_checker())
        .arg("check")
        .arg(rules)
        .arg(asserted)
        .arg(cert)
        .output()
        .expect("run oo-horn-isabelle");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let exit = out.status.code().unwrap_or(-1);
    Outcome { exit, verdict: if exit == 0 { json_field(&stdout, "verdict") } else { None } }
}

// ── the certificate format, as data, so it can be mutated ───────────────────

/// One line of `horn.tsv`:
/// `idx TAB k TAB (var TAB term)*k TAB cs TAB cp TAB co TAB (ps TAB pp TAB po)*m`.
///
/// Note what this parser does NOT need: the rule table. The field count is
/// `2 + 2k + 3 + 3m` and `k` is field 1, so `m` is recoverable from the line alone. Both
/// checkers parse this way; it is why an out-of-range rule index is a rejection (exit 1)
/// on both sides rather than a parse error (exit 2) on either.
#[derive(Debug, Clone)]
struct Step {
    idx: String,
    binds: Vec<(String, String)>,
    concl: [String; 3],
    prems: Vec<[String; 3]>,
}

fn parse_step(line: &str) -> Option<Step> {
    let f: Vec<&str> = line.split('\t').collect();
    if f.len() < 5 {
        return None;
    }
    let k: usize = f[1].parse().ok()?;
    if f.len() < 5 + 2 * k || !(f.len() - 5 - 2 * k).is_multiple_of(3) {
        return None;
    }
    let binds =
        (0..k).map(|i| (f[2 + 2 * i].to_string(), f[3 + 2 * i].to_string())).collect::<Vec<_>>();
    let c = 2 + 2 * k;
    let concl = [f[c].to_string(), f[c + 1].to_string(), f[c + 2].to_string()];
    let mut prems = Vec::new();
    let mut i = c + 3;
    while i + 2 < f.len() {
        prems.push([f[i].to_string(), f[i + 1].to_string(), f[i + 2].to_string()]);
        i += 3;
    }
    Some(Step { idx: f[0].to_string(), binds, concl, prems })
}

fn render_step(s: &Step) -> String {
    let mut parts = vec![s.idx.clone(), s.binds.len().to_string()];
    for (v, t) in &s.binds {
        parts.push(v.clone());
        parts.push(t.clone());
    }
    for t in std::iter::once(&s.concl).chain(s.prems.iter()) {
        parts.extend(t.iter().cloned());
    }
    parts.join("\t")
}

fn parse_cert(text: &str) -> Option<Vec<Step>> {
    text.lines().filter(|l| !l.is_empty()).map(parse_step).collect()
}

fn render_cert(steps: &[Step]) -> String {
    let mut s = String::new();
    for st in steps {
        s.push_str(&render_step(st));
        s.push('\n');
    }
    s
}

// ── how deep a certificate is, and how much rests on prefix visibility ──────

/// The asserted graph as a set of triples, so a premise can be told apart from a
/// derivation. A premise that is asserted is available to any step in any order; a
/// premise that is NOT asserted is available only because some other step concluded it,
/// and THAT is the only place the strict-prefix-visibility discipline does any work.
fn asserted_triples(path: &Path) -> std::collections::BTreeSet<[String; 3]> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return std::collections::BTreeSet::new();
    };
    text.lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() == 3).then(|| [f[0].to_string(), f[1].to_string(), f[2].to_string()])
        })
        .collect()
}

/// What a certificate's SHAPE is, as opposed to whether it checks.
///
/// The property the whole induction rests on is that a step may cite only what came
/// strictly before it and never itself. A one-step certificate cannot express a
/// violation of it and cannot exercise it either: every premise is asserted, and the
/// checker's ordering logic is never consulted. Counting rows is therefore not a measure
/// of this corpus at all, and these four numbers are.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Shape {
    /// The longest derivation chain. A step all of whose premises are asserted has depth
    /// 0; a step citing a conclusion has one more than the deepest step it cites.
    depth: usize,
    /// Steps with a premise that is not asserted and IS the conclusion of a strictly
    /// earlier step. The discipline is what lets these be accepted.
    citing_earlier: usize,
    /// Steps with a premise that is not asserted and is their OWN conclusion or the
    /// conclusion of a LATER step. The discipline is what makes these rejections, and
    /// without it each one is a conclusion fabricated out of nothing.
    citing_self_or_later: usize,
    /// The most steps that cite one single earlier conclusion. Width rather than depth:
    /// a shared premise is where an off-by-one in a visibility check would show up on
    /// one consumer and not the others.
    fan_out: usize,
}

fn shape_of(steps: &[Step], asserted: &std::collections::BTreeSet<[String; 3]>) -> Shape {
    let mut depth = vec![0usize; steps.len()];
    let mut out = Shape::default();
    let mut fan: BTreeMap<usize, usize> = BTreeMap::new();
    for i in 0..steps.len() {
        let (mut d, mut earlier, mut wrong) = (0usize, false, false);
        for p in &steps[i].prems {
            if asserted.contains(p) {
                continue;
            }
            // The deepest strictly-earlier step concluding this premise. Deepest and not
            // first, because the number being reported is the longest chain the
            // certificate contains and a shallower producer would understate it.
            let best = (0..i).filter(|j| &steps[*j].concl == p).max_by_key(|j| depth[*j]);
            match best {
                Some(j) => {
                    earlier = true;
                    d = d.max(depth[j] + 1);
                    *fan.entry(j).or_default() += 1;
                }
                None => {
                    if &steps[i].concl == p || steps[i + 1..].iter().any(|s| &s.concl == p) {
                        wrong = true;
                    }
                }
            }
        }
        depth[i] = d;
        out.depth = out.depth.max(d);
        out.citing_earlier += usize::from(earlier);
        out.citing_self_or_later += usize::from(wrong);
    }
    out.fan_out = fan.values().copied().max().unwrap_or(0);
    out
}

fn shape_of_case(case: &Case) -> Shape {
    let Ok(text) = std::fs::read_to_string(&case.cert) else { return Shape::default() };
    let Some(steps) = parse_cert(&text) else { return Shape::default() };
    shape_of(&steps, &asserted_triples(&case.asserted))
}

/// The (producer, consumer) pair a mutation needs in order to say anything: the earliest
/// step whose premise is the conclusion of a step before it. `None` means the
/// certificate is flat and the deep mutations do not apply to it.
///
/// This works from the certificate alone and does not consult the asserted graph, so a
/// premise that happens to be both asserted and concluded is still a pair. That is the
/// safe direction: it generates a row rather than skipping one, and the row is only ever
/// required to produce the SAME answer from both checkers.
fn first_internal_citation(steps: &[Step]) -> Option<(usize, usize)> {
    (0..steps.len()).find_map(|i| {
        steps[i]
            .prems
            .iter()
            .find_map(|p| (0..i).find(|j| &steps[*j].concl == p))
            .map(|j| (j, i))
    })
}

// ── the mutations ───────────────────────────────────────────────────────────

/// A fresh IRI no fixture mentions, so substituting it is always a lie.
const ALIEN: &str = "<http://ex.invalid/never-derived>";

/// Each mutation is a named, deterministic edit of a certificate's TEXT. Returning
/// `None` means the mutation does not apply to this certificate (too few steps,
/// bindings or premises) and the row is simply not generated.
type Mutation = (&'static str, fn(&str) -> Option<String>);

fn mutations() -> Vec<Mutation> {
    vec![
        ("rule_index_plus_one", |t| {
            let mut c = parse_cert(t)?;
            let n: usize = c.first()?.idx.parse().ok()?;
            c[0].idx = (n + 1).to_string();
            Some(render_cert(&c))
        }),
        ("rule_index_out_of_range", |t| {
            let mut c = parse_cert(t)?;
            c.first()?;
            c[0].idx = "9999".to_string();
            Some(render_cert(&c))
        }),
        ("rule_index_not_a_number", |t| {
            let mut c = parse_cert(t)?;
            c.first()?;
            c[0].idx = "four".to_string();
            Some(render_cert(&c))
        }),
        ("permute_binding_values", |t| {
            let mut c = parse_cert(t)?;
            if c.first()?.binds.len() < 2 {
                return None;
            }
            let v0 = c[0].binds[0].1.clone();
            c[0].binds[0].1 = c[0].binds[1].1.clone();
            c[0].binds[1].1 = v0;
            Some(render_cert(&c))
        }),
        ("binding_value_replaced", |t| {
            let mut c = parse_cert(t)?;
            if c.first()?.binds.is_empty() {
                return None;
            }
            c[0].binds[0].1 = ALIEN.to_string();
            Some(render_cert(&c))
        }),
        // D1. The key is duplicated with a WRONG second value. Whether this is a
        // rejection depends entirely on how a duplicate key is resolved, which is the
        // thing the format did not say until decision 0008. See
        // `resolved_d1_a_duplicate_binding_key_is_refused_by_both`.
        ("duplicate_binding_key", |t| {
            let mut c = parse_cert(t)?;
            if c.first()?.binds.is_empty() {
                return None;
            }
            let key = c[0].binds[0].0.clone();
            c[0].binds.push((key, ALIEN.to_string()));
            Some(render_cert(&c))
        }),
        // D2's shape, though over ordinary IRI terms it does NOT diverge: Lean
        // substitutes the variable's own name, which is not an IRI, so both reject. The
        // case where it does diverge is hand-built in `fixtures-differential/`.
        ("drop_binding_pair", |t| {
            let mut c = parse_cert(t)?;
            if c.first()?.binds.is_empty() {
                return None;
            }
            c[0].binds.remove(0);
            Some(render_cert(&c))
        }),
        ("drop_last_premise", |t| {
            let mut c = parse_cert(t)?;
            if c.first()?.prems.is_empty() {
                return None;
            }
            c[0].prems.pop();
            Some(render_cert(&c))
        }),
        ("permute_premises", |t| {
            let mut c = parse_cert(t)?;
            if c.first()?.prems.len() < 2 {
                return None;
            }
            c[0].prems.swap(0, 1);
            Some(render_cert(&c))
        }),
        ("premise_object_replaced", |t| {
            let mut c = parse_cert(t)?;
            if c.first()?.prems.is_empty() {
                return None;
            }
            c[0].prems[0][2] = ALIEN.to_string();
            Some(render_cert(&c))
        }),
        ("conclusion_object_replaced", |t| {
            let mut c = parse_cert(t)?;
            c.first()?;
            c[0].concl[2] = ALIEN.to_string();
            Some(render_cert(&c))
        }),
        // The step now cites its own conclusion as its first premise. On a rule whose
        // body and head differ this also breaks instantiation, which is why no shipped
        // fixture reaches the no-self-support property on its own.
        ("step_cites_own_conclusion", |t| {
            let mut c = parse_cert(t)?;
            if c.first()?.prems.is_empty() {
                return None;
            }
            let concl = c[0].concl.clone();
            c[0].prems[0] = concl;
            Some(render_cert(&c))
        }),
        ("reverse_steps", |t| {
            let mut c = parse_cert(t)?;
            if c.len() < 2 {
                return None;
            }
            c.reverse();
            Some(render_cert(&c))
        }),
        // ── the edits only a DEEP certificate can express ────────────────────
        //
        // Each of these returns `None` on a flat certificate, so before the deep cases
        // existed they produced nothing at all. That is the measure of the hole they
        // fill: the discipline says a step may cite only what came strictly before it
        // and never itself, and on a one-step certificate there is no "before" to get
        // wrong. `the_corpus_exercises_prefix_visibility_at_depth` counts the rows.
        //
        // The step is moved to sit BEFORE the step whose conclusion it cites. Nothing
        // else changes: same rule, same bindings, same premises, same conclusion. The
        // certificate is now invalid for one reason only.
        ("move_step_before_its_premise", |t| {
            let mut c = parse_cert(t)?;
            let (j, i) = first_internal_citation(&c)?;
            let step = c.remove(i);
            c.insert(j, step);
            Some(render_cert(&c))
        }),
        // The same two steps exchanged rather than one moved, which also drags the
        // producer past its own inputs.
        ("swap_a_step_with_its_producer", |t| {
            let mut c = parse_cert(t)?;
            let (j, i) = first_internal_citation(&c)?;
            c.swap(i, j);
            Some(render_cert(&c))
        }),
        // A chain cut in the middle. The steps that remain are each individually sound
        // and correctly ordered; what is gone is the one that produced a premise the
        // rest stand on. Truncating at the END is an accepting mutation (a prefix of a
        // valid certificate is valid) and `drop_last_step` covers it; truncating in the
        // MIDDLE must be a rejection, and the two together are what say the checker is
        // tracking what was derived rather than counting lines.
        ("truncate_chain_in_the_middle", |t| {
            let mut c = parse_cert(t)?;
            let (j, _) = first_internal_citation(&c)?;
            c.remove(j);
            Some(render_cert(&c))
        }),
        // Two steps that support each other. The consumer already cites the producer;
        // this points the producer's first premise at the consumer's conclusion, closing
        // the loop. A checker that asked only "is this premise concluded SOMEWHERE" —
        // the obvious and wrong reading of the format — accepts it, and with it accepts
        // any conclusion whatsoever, because a cycle needs no input.
        ("two_steps_cite_each_other", |t| {
            let mut c = parse_cert(t)?;
            let (j, i) = first_internal_citation(&c)?;
            if c[j].prems.is_empty() {
                return None;
            }
            c[j].prems[0] = c[i].concl.clone();
            Some(render_cert(&c))
        }),
        // Self-support at a step that is NOT the first, which `step_cites_own_conclusion`
        // cannot reach: it edits step 0, where a checker that only compared against the
        // asserted set would reject for the ordinary reason and never consult the
        // visibility rule at all.
        ("a_deep_step_cites_itself", |t| {
            let mut c = parse_cert(t)?;
            let (_, i) = first_internal_citation(&c)?;
            let earlier: Vec<[String; 3]> = c[..i].iter().map(|s| s.concl.clone()).collect();
            let at = c[i].prems.iter().position(|p| earlier.contains(p))?;
            c[i].prems[at] = c[i].concl.clone();
            Some(render_cert(&c))
        }),
        // The first step reaches FORWARD to the last step's conclusion. Applies to any
        // certificate with two steps, deep or flat, and is the plainest statement of the
        // rule: what is cited must already exist.
        ("a_step_cites_a_later_conclusion", |t| {
            let mut c = parse_cert(t)?;
            if c.len() < 2 || c[0].prems.is_empty() {
                return None;
            }
            c[0].prems[0] = c.last()?.concl.clone();
            Some(render_cert(&c))
        }),
        ("swap_first_two_steps", |t| {
            let mut c = parse_cert(t)?;
            if c.len() < 2 {
                return None;
            }
            c.swap(0, 1);
            Some(render_cert(&c))
        }),
        // Accepting mutations, so the corpus is not all rejections. A duplicated step is
        // redundant, and a prefix of a valid certificate is a valid certificate.
        ("duplicate_first_step", |t| {
            let mut c = parse_cert(t)?;
            let first = c.first()?.clone();
            c.push(first);
            Some(render_cert(&c))
        }),
        ("drop_last_step", |t| {
            let mut c = parse_cert(t)?;
            if c.len() < 2 {
                return None;
            }
            c.pop();
            Some(render_cert(&c))
        }),
        // The field separator, injected into a term. One tab shifts the field count by
        // one, so `(NF - 5 - 2k)` stops being a multiple of three and the line is
        // unparseable on both sides: exit 2, not a rejection.
        ("tab_inside_a_term", |t| {
            let first = t.lines().find(|l| !l.is_empty())?;
            let s = parse_step(first)?;
            let broken = first.replacen(&s.concl[0], &format!("{}\t{}", s.concl[0], "x"), 1);
            Some(t.replacen(first, &broken, 1))
        }),
        // Three tabs keep the count a multiple of three, so the line PARSES — into a
        // different step, with a spurious premise. It must be rejected, not accepted.
        ("three_tabs_inside_terms", |t| {
            let first = t.lines().find(|l| !l.is_empty())?;
            let s = parse_step(first)?;
            let broken = first.replacen(&s.concl[0], &format!("{}\tx\ty\tz", s.concl[0]), 1);
            Some(t.replacen(first, &broken, 1))
        }),
        ("truncate_last_field", |t| {
            let first = t.lines().find(|l| !l.is_empty())?;
            let cut = first.rfind('\t')?;
            Some(t.replacen(first, &first[..cut], 1))
        }),
        ("empty_certificate", |_| Some(String::new())),
    ]
}

/// Edits of the RULE TABLE rather than the certificate. These are where the VERDICT
/// WORD is at stake: a table that is not the built-in one must earn the relativised
/// verdict on both sides, and a certificate over it may still check.
type RuleMutation = (&'static str, fn(&str) -> Option<String>);

fn rule_mutations() -> Vec<RuleMutation> {
    vec![
        // Names are labels, not keys: the certificate cites an INDEX. Both sides must
        // still accept, and both must downgrade the verdict, because both decide the
        // absolute verdict by comparing the whole table.
        ("rule_renamed", |t| {
            let first = t.lines().find(|l| !l.is_empty())?;
            let name = first.split('\t').next()?;
            Some(t.replacen(&format!("{name}\t"), "renamed-rule\t", 1))
        }),
        ("rules_reversed", |t| {
            let mut ls: Vec<&str> = t.lines().filter(|l| !l.is_empty()).collect();
            if ls.len() < 2 {
                return None;
            }
            ls.reverse();
            Some(ls.join("\n") + "\n")
        }),
        ("rule_appended", |t| {
            Some(
                t.to_string()
                    + "invented\t1\t?s\t<http://ex.invalid/p>\t?o\t?s\t<http://ex.invalid/q>\t?o\n",
            )
        }),
        ("rules_truncated_to_two", |t| {
            let ls: Vec<&str> = t.lines().filter(|l| !l.is_empty()).collect();
            if ls.len() < 3 {
                return None;
            }
            Some(ls[..2].join("\n") + "\n")
        }),
        ("rule_variable_renamed", |t| {
            let first = t.lines().find(|l| !l.is_empty())?;
            if !first.contains("?x") {
                return None;
            }
            Some(t.replacen(first, &first.replace("?x", "?renamedvar"), 1))
        }),
        ("rule_body_length_wrong", |t| {
            let first = t.lines().find(|l| !l.is_empty())?;
            let f: Vec<&str> = first.split('\t').collect();
            let n: usize = f[1].parse().ok()?;
            let mut g = f.clone();
            let bumped = (n + 1).to_string();
            g[1] = &bumped;
            Some(t.replacen(first, &g.join("\t"), 1))
        }),
    ]
}

// ── the corpus ──────────────────────────────────────────────────────────────

struct Case {
    name: String,
    /// The mutation that produced it, or `"base"`. Disagreements are classified by this.
    kind: String,
    rules: PathBuf,
    asserted: PathBuf,
    cert: PathBuf,
}

/// The repository's own RDF, as source graphs. Directories rather than a hand-typed file
/// list, so a fixture added to the repository is picked up rather than silently missed.
/// Sorted, so the corpus is the same on every run.
fn source_graphs() -> Vec<PathBuf> {
    let dirs = [
        "benchmark/reference",
        "benchmark/generated",
        "benchmark/data",
        "benchmark/epc",
        "benchmark/gvr",
        "benchmark/mushroom",
        "demo/derived",
        "demo/corpus/dcat-us",
        "tests/w3c-shacl/core/complex",
        "tests/data",
    ];
    let mut out: Vec<PathBuf> = Vec::new();
    for d in dirs {
        let dir = repo().join(d);
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("ttl") {
                out.push(p);
            }
        }
    }
    out.push(repo().join("tests").join("test_ontology.ttl"));
    out.sort();
    out
}

/// Generate one certificate per (graph, rule table) pair, using the engine as the
/// producer. Steps are capped: a prefix of a valid certificate is a valid certificate,
/// because a step may only cite what came strictly before it, so truncation cannot turn
/// a bad certificate into a good one.
const MAX_STEPS: usize = 30;

fn generate_base_certificates(scratch: &Path) -> Vec<Case> {
    use open_ontologies::graph::GraphStore;
    use open_ontologies::reason::Reasoner;

    let mut cases = Vec::new();
    for (i, graph) in source_graphs().iter().enumerate() {
        let Ok(ttl) = std::fs::read_to_string(graph) else { continue };
        let store = Arc::new(GraphStore::new());
        if store.load_turtle(&ttl, None).is_err() {
            continue;
        }
        for table in ["builtin_rules.tsv", "user_rules.tsv"] {
            let stem = graph.file_stem().and_then(|s| s.to_str()).unwrap_or("graph").to_string();
            let tag = format!("{i:02}-{stem}-{}", table.trim_end_matches(".tsv"));
            let dir = scratch.join(&tag);
            if std::fs::create_dir_all(&dir).is_err() {
                continue;
            }
            if Reasoner::run_horn(&store, &horn_fixture(table), &dir).is_err() {
                continue;
            }
            let Ok(horn) = std::fs::read_to_string(dir.join("horn.tsv")) else { continue };
            let steps: Vec<&str> = horn.lines().filter(|l| !l.is_empty()).collect();
            if steps.is_empty() {
                // A graph the table derives nothing from certifies nothing. Keeping it
                // would pad the count with rows on which both checkers accept the empty
                // certificate, which is agreement about nothing.
                continue;
            }
            let capped: String =
                steps.iter().take(MAX_STEPS).map(|l| format!("{l}\n")).collect::<String>();
            let cert = dir.join("cert.tsv");
            std::fs::write(&cert, &capped).unwrap();
            cases.push(Case {
                name: tag,
                kind: "base".to_string(),
                rules: dir.join("rules.tsv"),
                asserted: dir.join("asserted.tsv"),
                cert,
            });
        }
    }
    cases
}

/// Every committed fixture, including the ones `isabelle/fixtures-added/` carries for
/// properties the shipped set cannot reach.
fn fixture_cases() -> Vec<Case> {
    let added = |n: &str| isabelle_dir().join("fixtures-added").join(n);
    let rows: Vec<(&str, PathBuf, PathBuf, PathBuf)> = vec![
        ("fx-good", horn_fixture("builtin_rules.tsv"), horn_fixture("asserted.tsv"), horn_fixture("good.tsv")),
        ("fx-good-user-table", horn_fixture("user_rules.tsv"), horn_fixture("asserted.tsv"), horn_fixture("good.tsv")),
        ("fx-good-short-table", horn_fixture("short_rules.tsv"), horn_fixture("asserted.tsv"), horn_fixture("good.tsv")),
        ("fx-bad-index", horn_fixture("builtin_rules.tsv"), horn_fixture("asserted.tsv"), horn_fixture("bad_index.tsv")),
        ("fx-bad-binding", horn_fixture("builtin_rules.tsv"), horn_fixture("asserted.tsv"), horn_fixture("bad_binding.tsv")),
        ("fx-bad-conclusion", horn_fixture("builtin_rules.tsv"), horn_fixture("asserted.tsv"), horn_fixture("bad_conclusion.tsv")),
        ("fx-bad-premise", horn_fixture("builtin_rules.tsv"), horn_fixture("asserted.tsv"), horn_fixture("bad_premise.tsv")),
        ("fx-bad-self", horn_fixture("builtin_rules.tsv"), horn_fixture("asserted.tsv"), horn_fixture("bad_self.tsv")),
        ("fx-good-chain", horn_fixture("builtin_rules.tsv"), added("asserted_chain.tsv"), added("good_chain.tsv")),
        ("fx-bad-self-chain", horn_fixture("builtin_rules.tsv"), added("asserted_chain.tsv"), added("bad_self_chain.tsv")),
        ("fx-bad-order", horn_fixture("builtin_rules.tsv"), horn_fixture("asserted.tsv"), added("bad_order.tsv")),
        // The D1 certificate is in the corpus like every other committed fixture, and
        // not held back because it is the one that disagrees. `binding_defect` classifies
        // it, `resolved_d1_a_duplicate_binding_key_is_refused_by_both` pins it, and its mutants are checked
        // like anything else.
        ("fx-bad-dup-key", horn_fixture("builtin_rules.tsv"), horn_fixture("asserted.tsv"), added("bad_dup_key.tsv")),
        ("fx-generalized", horn_fixture("builtin_rules.tsv"), added("asserted_gen.tsv"), added("generalized.tsv")),
    ];
    rows.into_iter()
        .filter(|(_, r, a, c)| r.exists() && a.exists() && c.exists())
        .map(|(n, r, a, c)| Case {
            name: n.to_string(),
            kind: "base".to_string(),
            rules: r,
            asserted: a,
            cert: c,
        })
        .collect()
}

/// The corners of the term format that NO generated certificate reaches, because the
/// engine writes IRIs and the repository's fixtures are ASCII. Each is a committed
/// `probe_<name>_{rules,asserted,cert}.tsv` triple under `isabelle/fixtures-differential/`.
/// They join the corpus as base cases, so they are mutated and fuzzed like everything
/// else; `the_untested_corners_of_the_term_format_agree` pins what each one should do.
fn probe_cases() -> Vec<Case> {
    let dir = isabelle_dir().join("fixtures-differential");
    let Ok(entries) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let f = e.file_name().to_string_lossy().to_string();
            f.strip_prefix("probe_")
                .and_then(|r| r.strip_suffix("_rules.tsv"))
                .map(|s| s.to_string())
        })
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|n| Case {
            name: format!("probe-{n}"),
            kind: "base".to_string(),
            rules: dir.join(format!("probe_{n}_rules.tsv")),
            asserted: dir.join(format!("probe_{n}_asserted.tsv")),
            cert: dir.join(format!("probe_{n}_cert.tsv")),
        })
        .filter(|c| c.rules.exists() && c.asserted.exists() && c.cert.exists())
        .collect()
}

// ── deep certificates, because the shipped ones are one step tall ───────────

const RDF_TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
/// `rdfs9` in `tests/fixtures/horn/builtin_rules.tsv`, zero-based. Every certificate
/// below cites this one rule, so `deep_cases` checks the row still is what it thinks
/// before writing anything: a table edit that shifted the indices would otherwise turn
/// this whole family into certificates about a different rule, which both checkers would
/// reject in agreement and which would look exactly like a passing test.
const RDFS9: usize = 4;

fn class(i: usize) -> String {
    format!("<http://ex.org/deep/C{i}>")
}

/// One `rdfs9` step: `?x type ?a`, `?a subClassOf ?b` ⊢ `?x type ?b`. The premise ORDER
/// is the rule's body order and is part of the contract on both sides.
fn rdfs9_step(x: &str, a: &str, b: &str) -> Step {
    Step {
        idx: RDFS9.to_string(),
        binds: vec![
            ("x".to_string(), x.to_string()),
            ("a".to_string(), a.to_string()),
            ("b".to_string(), b.to_string()),
        ],
        concl: [x.to_string(), RDF_TYPE.to_string(), b.to_string()],
        prems: vec![
            [x.to_string(), RDF_TYPE.to_string(), a.to_string()],
            [a.to_string(), SUBCLASS.to_string(), b.to_string()],
        ],
    }
}

/// A chain `d` steps tall over a subclass ladder, then `w` steps fanning out from its
/// tip. Step 0's premises are all asserted; step k cites step k-1's conclusion and
/// nothing else that is not asserted; each of the final `w` steps cites the SAME
/// conclusion, which is where a visibility check that is right for one consumer and
/// wrong for the rest would show.
///
/// Every one of these is a genuine `rdfs9` derivation and both checkers must accept it.
/// The value is not the acceptance, which is easy; it is that the mutations below then
/// have something to break.
fn ladder(d: usize, w: usize) -> (String, String) {
    let ind = "<http://ex.org/deep/a>";
    let mut asserted = format!("{ind}\t{RDF_TYPE}\t{}\n", class(0));
    for k in 0..d {
        asserted.push_str(&format!("{}\t{SUBCLASS}\t{}\n", class(k), class(k + 1)));
    }
    for j in 0..w {
        asserted.push_str(&format!("{}\t{SUBCLASS}\t<http://ex.org/deep/B{j}>\n", class(d)));
    }
    let mut steps: Vec<Step> = (0..d).map(|k| rdfs9_step(ind, &class(k), &class(k + 1))).collect();
    steps.extend(
        (0..w).map(|j| rdfs9_step(ind, &class(d), &format!("<http://ex.org/deep/B{j}>"))),
    );
    (asserted, render_cert(&steps))
}

/// The deep half of the corpus: chains, a wide fan, and two of each combined. Written to
/// scratch rather than committed because they are PARAMETERISED — the point is the
/// distribution of depths, and a fixture per depth would be twelve files saying one
/// thing. The hand-built adversarial pair that cannot be generated is committed instead,
/// under `tests/fixtures/horn/deep/`; see `deep_fixture_cases`.
fn deep_cases(scratch: &Path) -> Vec<Case> {
    let rules = horn_fixture("builtin_rules.tsv");
    let Ok(table) = std::fs::read_to_string(&rules) else { return Vec::new() };
    let row = table.lines().nth(RDFS9).unwrap_or_default();
    assert!(
        row.starts_with("rdfs9\t2\t"),
        "row {RDFS9} of builtin_rules.tsv is {row:?}, not rdfs9 with a two-atom body. The \
         deep certificates cite that index by number and would now be about a different \
         rule, which both kernels would reject together and which would read as a pass"
    );

    // (name, chain height, fan-out width). Heights chosen so the distribution is not one
    // number: a two-step cert, a handful, and one long enough that an off-by-one in a
    // visibility bound has somewhere to hide.
    let shapes = [
        ("chain-02", 2, 0),
        ("chain-04", 4, 0),
        ("chain-08", 8, 0),
        ("chain-20", 20, 0),
        ("wide-12", 1, 12),
        ("ladder-06x08", 6, 8),
    ];
    let mut cases = Vec::new();
    for (name, d, w) in shapes {
        let dir = scratch.join("deep").join(name);
        if std::fs::create_dir_all(&dir).is_err() {
            continue;
        }
        let (asserted, cert) = ladder(d, w);
        let ap = dir.join("asserted.tsv");
        let cp = dir.join("cert.tsv");
        if std::fs::write(&ap, asserted).is_err() || std::fs::write(&cp, cert).is_err() {
            continue;
        }
        cases.push(Case {
            name: format!("deep-{name}"),
            kind: "base".to_string(),
            rules: rules.clone(),
            asserted: ap,
            cert: cp,
        });
    }
    cases
}

/// The two adversarial certificates that no generator produces, because a generator
/// emits derivations and these are not derivations. Both instantiate their rule
/// PERFECTLY: every binding covers the rule, every premise matches the body, the
/// conclusion matches the head. The only thing wrong with either is the order, which is
/// exactly the property under test, and both are pinned by their own tests below.
fn deep_fixture_cases() -> Vec<Case> {
    let d = |n: &str| repo().join("tests").join("fixtures").join("horn").join("deep").join(n);
    let rules = horn_fixture("builtin_rules.tsv");
    let rows: Vec<(&str, PathBuf, PathBuf)> = vec![
        ("deep-self-support", d("self_support_asserted.tsv"), d("self_support_cert.tsv")),
        ("deep-mutual-support", d("mutual_asserted.tsv"), d("mutual_cert.tsv")),
        ("deep-mutual-seeded", d("mutual_seeded_asserted.tsv"), d("mutual_cert.tsv")),
    ];
    rows.into_iter()
        .filter(|(_, a, c)| a.exists() && c.exists())
        .map(|(n, a, c)| Case {
            name: n.to_string(),
            kind: "base".to_string(),
            rules: rules.clone(),
            asserted: a,
            cert: c,
        })
        .collect()
}

/// Expand each base case into its mutants.
fn mutate(bases: &[Case], scratch: &Path) -> Vec<Case> {
    let mut out = Vec::new();
    for base in bases {
        let Ok(cert_text) = std::fs::read_to_string(&base.cert) else { continue };
        let Ok(rules_text) = std::fs::read_to_string(&base.rules) else { continue };
        let dir = scratch.join("mut").join(&base.name);
        if std::fs::create_dir_all(&dir).is_err() {
            continue;
        }
        for (name, f) in mutations() {
            let Some(text) = f(&cert_text) else { continue };
            if text == cert_text {
                continue;
            }
            let p = dir.join(format!("{name}.tsv"));
            if std::fs::write(&p, &text).is_err() {
                continue;
            }
            out.push(Case {
                name: format!("{}/{name}", base.name),
                kind: name.to_string(),
                rules: base.rules.clone(),
                asserted: base.asserted.clone(),
                cert: p,
            });
        }
        for (name, f) in rule_mutations() {
            let Some(text) = f(&rules_text) else { continue };
            if text == rules_text {
                continue;
            }
            let p = dir.join(format!("rules-{name}.tsv"));
            if std::fs::write(&p, &text).is_err() {
                continue;
            }
            out.push(Case {
                name: format!("{}/{name}", base.name),
                kind: name.to_string(),
                rules: p,
                asserted: base.asserted.clone(),
                cert: base.cert.clone(),
            });
        }
    }
    out
}

/// A directory of this run's own, and a DIFFERENT one on every call.
///
/// The process id alone was enough while one test generated a corpus. Three do now, and
/// `cargo test` runs them in parallel by default, so a shared path would have one test
/// deleting another's certificates mid-run — a flake that looks like a disagreement,
/// which is the one failure this file must never report falsely.
fn scratch_dir() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("oo-cross-kernel-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("create scratch dir");
    d
}

// ── classifying a divergence by its cause, not by the edit that produced it ──

/// A rule, read far enough to know which variables it has. Not a third checker: this is
/// used only to EXPLAIN a divergence the two checkers have already produced, never to
/// decide whether they agree. If it is wrong, a row goes unexplained and the test fails,
/// which is the safe direction.
struct Rule {
    vars: Vec<String>,
}

fn parse_rule_line(line: &str) -> Option<Rule> {
    let f: Vec<&str> = line.split('\t').collect();
    if f.len() < 5 {
        return None;
    }
    let n: usize = f[1].parse().ok()?;
    if f.len() != 5 + 3 * n {
        return None;
    }
    let mut vars = Vec::new();
    for field in &f[2..] {
        if let Some(v) = field.strip_prefix('?')
            && !vars.iter().any(|x| x == v)
        {
            vars.push(v.to_string());
        }
    }
    Some(Rule { vars })
}

/// Whether the binding list is malformed in one of the two ways decision 0008 refuses:
/// a repeated key (D1), or a variable of the cited rule with no binding (D2).
///
/// It used to EXCUSE a divergence. It no longer can, because there is nothing left to
/// excuse: `bindingWellFormed` is a conjunct of Lean's `checkHornStep` and the two
/// kernels now agree on every one of these rows. It survives, pointed the other way, as
/// the POSITIVE gate inside `the_two_kernels_agree_on_the_whole_corpus`: a file this says is
/// malformed must be accepted by NEITHER checker. That is a stronger use of the same
/// analysis: as an excuse it could only ever be satisfied by a disagreement, and as a
/// gate it is checked against all 1,718 rows whether they diverge or not.
///
/// It is not a third checker. If it is wrong in the permissive direction a row simply
/// goes ungated; if it is wrong in the strict direction the gate fails, which is the
/// safe way round.
fn binding_defect(rules_path: &Path, cert_path: &Path) -> Option<&'static str> {
    let rules_text = std::fs::read_to_string(rules_path).ok()?;
    let rules: Vec<Rule> =
        rules_text.lines().filter(|l| !l.is_empty()).map(parse_rule_line).collect::<Option<_>>()?;
    let cert_text = std::fs::read_to_string(cert_path).ok()?;
    let steps = parse_cert(&cert_text)?;

    for step in &steps {
        let Ok(idx) = step.idx.parse::<usize>() else { return None };
        let Some(rule) = rules.get(idx) else { continue };
        let mut seen: Vec<&str> = Vec::new();
        for (k, _) in &step.binds {
            if seen.contains(&k.as_str()) {
                return Some(
                    "D1: the binding list repeats a key. Decision 0008 refuses it. Isabelle \
                     has always said R_binding_dup_key; Lean used to resolve it with \
                     List.lookup, which is first-wins, and accept.",
                );
            }
            seen.push(k);
        }
        if rule.vars.iter().any(|v| !seen.contains(&v.as_str())) {
            return Some(
                "D2: the binding does not cover every variable of the cited rule. Decision \
                 0008 refuses it. Isabelle has always said R_binding_incomplete; Lean's \
                 substOf used to send the unbound variable to its own NAME and carry on, so \
                 the step checked whenever that name was the term the certificate used.",
            );
        }
    }
    None
}

// ── a seeded fuzzer over the file format ────────────────────────────────────

/// The hand-written mutations above test the failures someone thought of. This tests the
/// ones nobody did, and it is aimed squarely at the PARSERS, which is where two
/// independent readers of one format are most likely to part company: field counts,
/// numbers that are not numbers, empty fields, and the field separator appearing inside
/// a term.
///
/// Seeded, so a run is reproducible. It is NOT stable across changes to the corpus: one
/// stream feeds every base in order, so adding a base shifts every draw after it and a
/// given row stops being generated. That is why a row the fuzzer finds interesting gets
/// COMMITTED as a fixture with its own test. `resolved_d2c_*` is one it found and
/// this configuration no longer reaches. The fuzzer is a search, and the fixtures are
/// what the search found.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        // xorshift64*. Written out rather than pulled in: a dev-dependency for four
        // lines of arithmetic is a dependency in the trust surface of a test whose whole
        // subject is trust surfaces.
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
}

/// Tokens a corrupted field may be replaced with, chosen to hit the dispatch points both
/// parsers have: the `?` that makes a rule field a variable, the `<`/`>` that make a term
/// an IRI, the `_:` that make it a blank node, a quote, an empty field, numbers in and
/// out of range, and a raw TAB.
const FUZZ_TOKENS: &[&str] = &[
    "",
    "?",
    "?x",
    "?renamed",
    "<http://ex.org/a>",
    "<",
    ">",
    "<>",
    "_:b",
    "_:",
    "\"lit\"",
    "\"lit\"@en",
    "0",
    "1",
    "4",
    "27",
    "9999",
    "-1",
    "007",
    "not-a-number",
    "x",
    "a\tb",
];

fn fuzz_tsv(text: &str, rng: &mut Rng, edits: usize) -> String {
    let mut lines: Vec<Vec<String>> = text
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.split('\t').map(|s| s.to_string()).collect())
        .collect();
    if lines.is_empty() {
        return text.to_string();
    }
    for _ in 0..edits {
        let li = rng.below(lines.len());
        if lines[li].is_empty() {
            continue;
        }
        let fi = rng.below(lines[li].len());
        match rng.below(6) {
            0 => {
                lines[li].remove(fi);
            }
            1 => {
                let v = lines[li][fi].clone();
                lines[li].insert(fi, v);
            }
            2 => {
                let fj = rng.below(lines[li].len());
                lines[li].swap(fi, fj);
            }
            3 => {
                lines[li][fi] = FUZZ_TOKENS[rng.below(FUZZ_TOKENS.len())].to_string();
            }
            4 => {
                let n: u64 = lines[li][fi].parse().unwrap_or(0);
                lines[li][fi] = n.wrapping_add(1 + rng.below(3) as u64).to_string();
            }
            _ => {
                let t = FUZZ_TOKENS[rng.below(FUZZ_TOKENS.len())];
                lines[li][fi] = format!("{}{t}", lines[li][fi]);
            }
        }
    }
    lines.iter().map(|f| f.join("\t") + "\n").collect()
}

const FUZZ_SEED: u64 = 0x0000_00D1_FFED_0001;

/// Six fuzzed variants of each base, three corrupting the certificate and three the rule
/// table, with a growing number of edits.
fn fuzz(bases: &[Case], scratch: &Path) -> Vec<Case> {
    let mut rng = Rng(FUZZ_SEED);
    let mut out = Vec::new();
    for base in bases {
        let Ok(cert_text) = std::fs::read_to_string(&base.cert) else { continue };
        let Ok(rules_text) = std::fs::read_to_string(&base.rules) else { continue };
        let dir = scratch.join("fuzz").join(&base.name);
        if std::fs::create_dir_all(&dir).is_err() {
            continue;
        }
        for edits in 1..=3 {
            let text = fuzz_tsv(&cert_text, &mut rng, edits);
            let p = dir.join(format!("cert-{edits}.tsv"));
            if std::fs::write(&p, &text).is_ok() {
                out.push(Case {
                    name: format!("{}/fuzz-cert-{edits}", base.name),
                    kind: format!("fuzz_cert_{edits}"),
                    rules: base.rules.clone(),
                    asserted: base.asserted.clone(),
                    cert: p,
                });
            }
            let text = fuzz_tsv(&rules_text, &mut rng, edits);
            let p = dir.join(format!("rules-{edits}.tsv"));
            if std::fs::write(&p, &text).is_ok() {
                out.push(Case {
                    name: format!("{}/fuzz-rules-{edits}", base.name),
                    kind: format!("fuzz_rules_{edits}"),
                    rules: p,
                    asserted: base.asserted.clone(),
                    cert: base.cert.clone(),
                });
            }
        }
    }
    out
}

// ── the tests ───────────────────────────────────────────────────────────────

#[test]
fn the_skip_says_which_half_is_missing() {
    // The skip path is the one nobody looks at until CI is silently green over nothing,
    // so it is exercised here rather than assumed. `skip_unless` returns false when the
    // thing IS available, and prints a SKIPPED_FIXTURE marker when it is not.
    let present = common::skip_unless(true, "a thing that is here", "nothing to do");
    assert!(!present, "skip_unless must not skip when the tool is available");
    if lake_available() && isabelle_available() {
        assert!(!skip(), "both toolchains are present, so this must not skip");
    } else {
        assert!(skip(), "a missing toolchain must skip loudly, not run half a differential");
    }
}

/// The two embedded rule tables are shown equal THROUGH the committed fixture, which is
/// what makes comparing the verdict word mean anything.
///
/// `lean_horn_certificate_test.rs` already pins `oo-horn rules` against
/// `tests/fixtures/horn/builtin_rules.tsv` byte for byte. This adds the other half:
/// `verdict_of` assigns the ABSOLUTE verdict to a table only when that table IS
/// `builtin_table`, so `table_verdict: entailed` over the same bytes says Isabelle's
/// embedded table is the fixture too. Lean's `Builtin.asHorn` and Isabelle's
/// `builtin_table` are therefore the same 27 rules, by two independent comparisons
/// against one file, rather than by anybody's say-so.
#[test]
fn both_embedded_rule_tables_are_the_committed_fixture() {
    if skip() {
        return;
    }
    let lean = Command::new(lean_checker()).arg("rules").output().expect("oo-horn rules");
    assert!(lean.status.success());
    let printed = String::from_utf8_lossy(&lean.stdout).to_string();
    let committed = std::fs::read_to_string(horn_fixture("builtin_rules.tsv")).unwrap();
    assert_eq!(
        printed.trim_end(),
        committed.trim_end(),
        "Lean's built-in table has drifted from the committed fixture"
    );

    let isa = Command::new(isabelle_checker())
        .arg("table")
        .arg(horn_fixture("builtin_rules.tsv"))
        .output()
        .expect("oo-horn-isabelle table");
    let out = String::from_utf8_lossy(&isa.stdout).to_string();
    assert_eq!(
        json_field(&out, "table_verdict").as_deref(),
        Some("entailed"),
        "Isabelle's verdict_of must call the committed fixture the built-in table; if it \
         does not, the two embedded tables differ and every verdict comparison below is \
         comparing two different things: {out}"
    );

    // And the negative, or the assertion above is unfalsifiable: a table that is NOT the
    // built-in one must not earn the absolute verdict on either side.
    let isa = Command::new(isabelle_checker())
        .arg("table")
        .arg(horn_fixture("user_rules.tsv"))
        .output()
        .expect("oo-horn-isabelle table");
    let out = String::from_utf8_lossy(&isa.stdout).to_string();
    assert_eq!(
        json_field(&out, "table_verdict").as_deref(),
        Some("entailed_under_supplied_rules"),
        "a user table must not be mistaken for the built-in one: {out}"
    );
}

/// **The differential.** Both checkers, every certificate, the accept/reject bit, the
/// exit code and the verdict word.
#[test]
fn the_two_kernels_agree_on_the_whole_corpus() {
    if skip() {
        return;
    }
    let scratch = scratch_dir();
    let mut bases = fixture_cases();
    bases.extend(probe_cases());
    bases.extend(deep_fixture_cases());
    bases.extend(deep_cases(&scratch));
    bases.extend(generate_base_certificates(&scratch));
    let mutants = mutate(&bases, &scratch);
    let fuzzed = fuzz(&bases, &scratch);
    let total = bases.len() + mutants.len() + fuzzed.len();

    assert!(
        total >= 200,
        "a differential over {total} certificates is not evidence of anything. Agreement \
         on a handful of hand-written files is what a shared bug looks like. Something \
         has gone wrong in corpus generation"
    );

    let mut agree_accept = 0usize;
    let mut agree_reject = 0usize;
    let mut agree_unreadable = 0usize;
    // Rows whose binding is malformed under decision 0008, by which shape and by the edit
    // that produced them. Counted rather than excused. The 47 rows that used to DIVERGE are
    // among these; most of the rest were already rejected by both kernels for some other
    // reason, which is why the gate below runs on all of them and not only on divergences.
    let mut refused: BTreeMap<String, usize> = BTreeMap::new();
    let mut accepted_malformed: Vec<String> = Vec::new();
    let mut divergent: Vec<String> = Vec::new();

    for case in bases.iter().chain(mutants.iter()).chain(fuzzed.iter()) {
        let l = run_lean(&case.rules, &case.asserted, &case.cert);
        let i = run_isabelle(&case.rules, &case.asserted, &case.cert);

        // The positive gate. A binding decision 0008 refuses must be accepted by NEITHER
        // kernel, whatever the two of them say about each other. Checked on every row, not
        // only on the ones that disagree, so it cannot be satisfied by silence.
        if let Some(why) = binding_defect(&case.rules, &case.cert) {
            let class = if why.starts_with("D1") { "D1 duplicate key" } else { "D2 uncovered" };
            *refused.entry(format!("{class} [{}]", case.kind)).or_default() += 1;
            if l.exit == 0 || i.exit == 0 {
                accepted_malformed.push(format!(
                    "{}\n    {why}\n    lean:     exit {} verdict {:?}\n    isabelle: exit {} \
                     verdict {:?}\n    rules={} asserted={} cert={}",
                    case.name,
                    l.exit,
                    l.verdict,
                    i.exit,
                    i.verdict,
                    case.rules.display(),
                    case.asserted.display(),
                    case.cert.display()
                ));
            }
        }

        if l == i {
            match l.exit {
                0 => agree_accept += 1,
                1 => agree_reject += 1,
                _ => agree_unreadable += 1,
            }
            continue;
        }
        divergent.push(format!(
            "{}\n    lean:     exit {} verdict {:?}\n    isabelle: exit {} verdict {:?}\n    \
             rules={} asserted={} cert={}",
            case.name,
            l.exit,
            l.verdict,
            i.exit,
            i.verdict,
            case.rules.display(),
            case.asserted.display(),
            case.cert.display()
        ));
    }

    let refused_total: usize = refused.values().sum();
    // The corpus measured by SHAPE and not only by row count, printed next to the
    // agreement number so the two are read together. A row count says how many files
    // were compared; these say how many of them could express the ordering property the
    // checker's induction rests on. See
    // `the_corpus_exercises_prefix_visibility_at_depth`, which holds the floor.
    let shapes: Vec<Shape> =
        bases.iter().chain(mutants.iter()).chain(fuzzed.iter()).map(shape_of_case).collect();
    let deepest = shapes.iter().map(|s| s.depth).max().unwrap_or(0);
    let widest = shapes.iter().map(|s| s.fan_out).max().unwrap_or(0);
    let uses = shapes.iter().filter(|s| s.citing_earlier > 0).count();
    let violates = shapes.iter().filter(|s| s.citing_self_or_later > 0).count();

    println!(
        "cross-kernel differential\n  \
         certificates:     {total} ({} base, {} mutated, {} fuzzed, seed {FUZZ_SEED:#x})\n  \
         prefix visibility: {} rows exercise it ({uses} rest on it to be accepted, \
         {violates} must be rejected by it); deepest chain {deepest}, widest fan-out {widest}\n  \
         both accept:      {agree_accept}\n  \
         both reject:      {agree_reject}\n  \
         both exit 2:      {agree_unreadable}\n  \
         divergent:        {}\n  \
         malformed binding, refused by both (decision 0008): {refused_total}",
        bases.len(),
        mutants.len(),
        fuzzed.len(),
        uses + violates,
        divergent.len()
    );
    for (kind, n) in &refused {
        println!("  {kind}: {n} rows");
    }

    assert!(
        accepted_malformed.is_empty(),
        "A CERTIFICATE WITH A MALFORMED BINDING WAS ACCEPTED. Decision 0008 says a repeated \
         key and a binding that does not cover the cited rule's variables are both refused, \
         and `bindingWellFormed` is a conjunct of `checkHornStep` for that reason. A row \
         here means the gate is not doing what the decision record says it does.\n\n{}",
        accepted_malformed.join("\n\n")
    );

    assert!(
        divergent.is_empty(),
        "THE TWO KERNELS DISAGREE. This was 47 rows before decision 0008 and it is zero \
         after; a row reappearing here is a NEW finding and the headline result of this \
         test, not a nuisance. Do not adjust either checker to make it go away, and in \
         particular do not adjust the Isabelle, which was written from the specifications \
         without reading the Lean and is worth nothing once it is edited to agree. Decide \
         from the specification which side is wrong, write the decision down, and move the \
         side the decision says moves.\n\n{}",
        divergent.join("\n\n")
    );

    // The gate above is worthless if the corpus stopped containing the shapes it refuses.
    // The floor is 47 because that is how many rows DIVERGED on this corpus and seed before
    // decision 0008, so every one of them must still be generated and must still be caught.
    // The actual count is higher and is printed rather than asserted, because pinning it
    // would make an unrelated fixture addition look like a regression.
    assert!(
        refused_total >= 47,
        "only {refused_total} rows carry a binding decision 0008 refuses, and there were 47 \
         before it. The corpus has stopped generating the shape the gate is about, so the \
         gate is passing over nothing"
    );

    // A corpus that is all rejections would prove only that both sides can say no.
    assert!(agree_accept >= 20, "only {agree_accept} certificates were accepted by both");
    assert!(agree_reject >= 100, "only {agree_reject} certificates were rejected by both");
    assert!(agree_unreadable >= 10, "only {agree_unreadable} inputs were unparseable on both");

    let _ = std::fs::remove_dir_all(&scratch);
}

/// **D1, resolved by decision 0008.** A binding list with a repeated key.
///
/// `isabelle/fixtures-added/bad_dup_key.tsv` binds `x` twice: first to `<http://ex.org/a>`,
/// which is the value that makes the step check, then to `<http://ex.org/zzz>`, which
/// does not.
///
/// It used to diverge. Isabelle's `check_step` tested `distinct (map fst b)` first and
/// returned `R_binding_dup_key`; Lean's `substOf` was `fun v => (List.lookup v l).getD v`,
/// `List.lookup` returns the FIRST match, so `x` resolved to `<http://ex.org/a>`, the step
/// checked, and the run earned the ABSOLUTE verdict `entailed`. Neither was unsound, because
/// `checkHornStep_sound` quantifies over `substOf st.binds`, a perfectly good total
/// substitution whichever value it picks, and that is exactly why the divergence was a
/// FORMAT defect rather than a bug in either checker. Whether this certificate was valid
/// turned on a tie-break inside a standard-library function.
///
/// **How it was resolved.** Decision 0008 refuses a repeated key, and the LEAN moved:
/// `bindingWellFormed` is now a conjunct of `checkHornStep`. The Isabelle is untouched,
/// because it was written from the W3C sources without reading the Lean and is worth
/// nothing once it is edited to agree.
///
/// Refusing rather than making first-wins normative is argued in the decision record. The
/// short form: `List.lookup` and `map_of` are first-wins while `dict()` and
/// `HashMap::from_iter` are last-wins, so a normative first-wins rule is one that half of
/// all unthinking implementations would break silently, and a duplicate key carries no
/// information any producer needs.
#[test]
fn resolved_d1_a_duplicate_binding_key_is_refused_by_both() {
    if skip() {
        return;
    }
    let cert = isabelle_dir().join("fixtures-added").join("bad_dup_key.tsv");
    if common::skip_unless(cert.exists(), "isabelle/fixtures-added/bad_dup_key.tsv", "it is committed") {
        return;
    }
    let rules = horn_fixture("builtin_rules.tsv");
    let asserted = horn_fixture("asserted.tsv");

    // The fixture still has the shape the test is about. Without this, a fixture edited
    // into a well-formed certificate would make the agreement below mean nothing.
    let text = std::fs::read_to_string(&cert).unwrap();
    let step = parse_step(text.lines().next().unwrap()).expect("the fixture must parse");
    let keys: Vec<&str> = step.binds.iter().map(|(k, _)| k.as_str()).collect();
    assert!(
        keys.iter().enumerate().any(|(n, k)| keys[..n].contains(k)),
        "bad_dup_key.tsv must still repeat a binding key: {keys:?}"
    );

    let lean = run_lean(&rules, &asserted, &cert);
    let isa = run_isabelle(&rules, &asserted, &cert);
    assert_eq!(lean.exit, 1, "Lean must now refuse a repeated key rather than resolve it");
    assert_eq!(lean.verdict, None, "and it must award no verdict at all, absolute or relativised");
    assert_eq!(isa.exit, 1, "Isabelle rejects it as malformed, as it always did");
    assert_eq!(lean, isa, "D1 is resolved: the two kernels now read these bytes the same way");
}

/// The control for D1, and the reason the test above is not just a checker that says no.
/// Drop the repeated pair and the certificate is the ordinary one both kernels accept
/// with the absolute verdict, over the same rule table and the same graph.
#[test]
fn resolved_d1_the_same_certificate_without_the_repeat_is_accepted() {
    if skip() {
        return;
    }
    let rules = horn_fixture("builtin_rules.tsv");
    let asserted = horn_fixture("asserted.tsv");
    let good = horn_fixture("good.tsv");
    let dup = isabelle_dir().join("fixtures-added").join("bad_dup_key.tsv");
    if common::skip_unless(dup.exists() && good.exists(), "the D1 pair", "they are committed") {
        return;
    }

    // `bad_dup_key.tsv` IS `good.tsv` with one binding pair inserted: same rule index, same
    // conclusion, same premises. So the only difference the kernels can be reacting to is
    // the repeated key.
    let a = parse_step(std::fs::read_to_string(&good).unwrap().lines().next().unwrap()).unwrap();
    let b = parse_step(std::fs::read_to_string(&dup).unwrap().lines().next().unwrap()).unwrap();
    assert_eq!(a.idx, b.idx);
    assert_eq!(a.concl, b.concl);
    assert_eq!(a.prems, b.prems);
    assert_eq!(b.binds.len(), a.binds.len() + 1, "one pair apart, and that pair is the repeat");

    let lean = run_lean(&rules, &asserted, &good);
    let isa = run_isabelle(&rules, &asserted, &good);
    assert_eq!(lean.exit, 0, "the well-formed certificate must still be accepted by Lean");
    assert_eq!(lean.verdict.as_deref(), Some("entailed"), "with the absolute verdict");
    assert_eq!(isa.exit, 0, "and by Isabelle");
    assert_eq!(lean, isa);
}

/// **D2, resolved by decision 0008.** A binding that does not cover every variable of the
/// rule, where the missing variable's NAME is itself a term in the graph.
///
/// The certificate in `isabelle/fixtures-differential/` cites a one-atom rule
/// `?s <p> ?o -> ?s <q> ?o` over a graph whose only triple is `s <p> <b>` — note the
/// subject is the bare term `s`, spelled exactly like the rule's variable. The binding
/// supplies `o` and omits `s`.
///
/// * Isabelle: `binding_covers` requires every variable of the rule to have a binding,
///   and returns `R_binding_incomplete`. Its instantiation `iptriple` is PARTIAL, so an
///   unbound variable has no value at all and there is nothing to fall through to.
/// * Lean, before decision 0008: `substOf` was total by construction,
///   `(List.lookup v l).getD v`, so an unbound variable took its own NAME as its value.
///   `s` became the term `s`, the body instantiated to the asserted triple, the head
///   instantiated to the claimed conclusion, and the step checked.
///
/// Which side was wrong depended on which question was being asked, and the two questions
/// have opposite answers. That is worth saying plainly rather than picking a winner.
///
/// On "is the conclusion entailed under this rule table?", LEAN IS RIGHT. `EntailsR`
/// quantifies over every total substitution, so a step whose body instantiates into known
/// triples under SOME total substitution really does entail its conclusion, and the
/// substitution Lean used is a real one. Isabelle's rejection is incompleteness, and a
/// false alarm is the harmless direction.
///
/// On "is this a well-formed certificate?", ISABELLE IS RIGHT, and
/// `resolved_d2b_an_unsafe_rule_head_is_refused_by_both` is why: Lean's default did not
/// merely admit more certificates, it FABRICATED a term out of a variable's name and put
/// it in the conclusion. A certificate could therefore be accepted whose conclusion is not
/// a writable RDF triple at all.
///
/// This also contradicts, ACROSS the two checkers, a claim proved inside one of them.
/// Isabelle's `coverage_implied` says removing the coverage check cannot change its
/// accept/reject bit, and that is true — of Isabelle, where instantiation is partial, so
/// a missing binding kills the step anyway. It does not transfer: Lean's instantiation is
/// total, so the same check is load-bearing there and in the opposite direction. A
/// property proved of one formalisation's checker is not a property of the format, and
/// this is what that looks like when it bites.
///
/// **How it was resolved.** Decision 0008 refuses an incomplete binding, and the LEAN
/// moved: `bindsCover` requires every variable of the cited rule, body AND head, to have
/// a binding before anything is instantiated.
///
/// That is a deliberate choice of well-formedness over completeness, and the decision
/// record says so rather than pretending Lean was wrong on entailment. It is free because
/// it loses no certificate anybody meant: the control below, and
/// `the_refusal_costs_no_certificate_anybody_meant`, show the same claim written with the
/// variable bound is accepted by both.
#[test]
fn resolved_d2_a_binding_that_omits_a_rule_variable_is_refused_by_both() {
    if skip() {
        return;
    }
    let rules = differential_fixture("varname_rules.tsv");
    let asserted = differential_fixture("varname_asserted.tsv");
    let cert = differential_fixture("varname_unbound_cert.tsv");
    if common::skip_unless(
        rules.exists() && asserted.exists() && cert.exists(),
        "isabelle/fixtures-differential/varname_*.tsv",
        "they are committed",
    ) {
        return;
    }

    let lean = run_lean(&rules, &asserted, &cert);
    let isa = run_isabelle(&rules, &asserted, &cert);
    assert_eq!(
        lean.exit, 1,
        "Lean must now refuse the incomplete binding rather than send the omitted variable \
         to its own name"
    );
    assert_eq!(lean.verdict, None);
    assert_eq!(isa.exit, 1, "Isabelle rejects: binding_incomplete, as it always did");
    assert_eq!(lean, isa, "D2 is resolved: the two kernels now read these bytes the same way");

    // The control, and it is load-bearing: bind `s` as well and BOTH accept. Without it
    // the assertions above would be satisfied by a checker that had simply stopped
    // accepting anything over this rule table.
    let full = differential_fixture("varname_bound_cert.tsv");
    assert!(full.exists(), "the control certificate must be committed");
    let lean = run_lean(&rules, &asserted, &full);
    let isa = run_isabelle(&rules, &asserted, &full);
    assert_eq!(lean.exit, 0, "the fully bound certificate must still be accepted by Lean");
    assert_eq!(lean.verdict.as_deref(), Some("entailed_under_supplied_rules"));
    assert_eq!(isa.exit, 0, "and by Isabelle");
    assert_eq!(lean, isa, "with the same verdict");
}

/// **What decision 0008 costs, measured rather than asserted.**
///
/// The claim in the decision record is that refusing an incomplete binding loses no
/// certificate anybody meant, because the omitted variable can always be bound to the
/// term the conclusion already shows. This runs that repair on the sharp case, D2b, whose
/// rule head carries a variable the body never binds.
///
/// It also fixes the BOUNDARY of the decision in a test, which matters more than the
/// reassurance. The repaired certificate still concludes `<a> <q> z`, whose object is the
/// bare term `z`, which is not an IRI, not a blank node, not a literal and not writable RDF, and
/// BOTH kernels accept it. Decision 0008 requires the term to be WRITTEN by the
/// certificate's author; it does not require it to be writable RDF, and nothing here
/// should be read as saying it does.
#[test]
fn the_refusal_costs_no_certificate_anybody_meant() {
    if skip() {
        return;
    }
    let rules = differential_fixture("unsafehead_rules.tsv");
    let asserted = differential_fixture("unsafehead_asserted.tsv");
    let cert = differential_fixture("unsafehead_cert.tsv");
    if common::skip_unless(
        rules.exists() && asserted.exists() && cert.exists(),
        "isabelle/fixtures-differential/unsafehead_*.tsv",
        "they are committed",
    ) {
        return;
    }

    let text = std::fs::read_to_string(&cert).unwrap();
    let mut step = parse_step(text.lines().next().unwrap()).expect("the probe must parse");
    assert_eq!(step.concl[2], "z", "the claimed conclusion must carry the bare term `z`");
    assert!(!step.binds.iter().any(|(k, _)| k == "z"), "and `z` must start out unbound");

    // The repair, and it is the only one available: bind `z` to what the conclusion says.
    step.binds.push(("z".to_string(), "z".to_string()));
    let repaired = std::env::temp_dir().join(format!("oo-d2b-repaired-{}.tsv", std::process::id()));
    std::fs::write(&repaired, render_step(&step) + "\n").unwrap();

    let lean = run_lean(&rules, &asserted, &repaired);
    let isa = run_isabelle(&rules, &asserted, &repaired);
    assert_eq!(
        lean.exit, 0,
        "the repaired certificate must be accepted, or decision 0008 costs more than it \
         claims: it would be refusing the inference rather than the omission"
    );
    assert_eq!(lean.verdict.as_deref(), Some("entailed_under_supplied_rules"));
    assert_eq!(isa.exit, 0, "and both kernels must accept it, or the format still has two readings");
    assert_eq!(lean, isa);
    let _ = std::fs::remove_file(&repaired);
}

/// **Divergence D2c: the same hole, found by the fuzzer rather than designed.**
///
/// The three files here are not hand-built. `d2c_fuzzfound_rules.tsv` is
/// `probe_degen2_rules.tsv` with ONE field replaced by the seeded fuzzer: the body's
/// object pattern `?o` became a bare `?`, which both parsers read as a variable whose
/// NAME IS THE EMPTY STRING. The graph's object is an empty field. So Lean's default
/// sends the unbound variable `""` to the term `""`, the body instantiates to the
/// asserted triple, and the step checks. Isabelle wants a binding for `""` and does not
/// find one.
///
/// It is worth its own test because of where it came from. D2a needed a graph whose
/// subject was spelled like a variable, and the obvious objection is that nobody writes
/// such a graph. This one needed a single character to change in a rule file. The hole was
/// reachable by a typo, and `substOf`'s silent default was what turned a typo into an
/// accepted certificate rather than an error. That is the case that decided decision 0008
/// against making the permissive reading normative: a default nobody would write down,
/// reached by a keystroke.
#[test]
fn resolved_d2c_the_hole_the_fuzzer_found_through_a_typo_is_closed() {
    if skip() {
        return;
    }
    let rules = differential_fixture("d2c_fuzzfound_rules.tsv");
    let asserted = differential_fixture("d2c_fuzzfound_asserted.tsv");
    let cert = differential_fixture("d2c_fuzzfound_cert.tsv");
    if common::skip_unless(
        rules.exists() && asserted.exists() && cert.exists(),
        "isabelle/fixtures-differential/d2c_fuzzfound_*.tsv",
        "they are committed",
    ) {
        return;
    }

    // One field apart from the probe it was fuzzed from, and that field is the bare `?`.
    let original = std::fs::read_to_string(differential_fixture("probe_degen2_rules.tsv")).unwrap();
    let fuzzed = std::fs::read_to_string(&rules).unwrap();
    let a: Vec<&str> = original.trim_end().split('\t').collect();
    let b: Vec<&str> = fuzzed.trim_end().split('\t').collect();
    assert_eq!(a.len(), b.len(), "the fuzzer changed a field, not the field count");
    let differing: Vec<usize> = (0..a.len()).filter(|i| a[*i] != b[*i]).collect();
    assert_eq!(differing.len(), 1, "exactly one field differs: {a:?} vs {b:?}");
    assert_eq!(b[differing[0]], "?", "and the one that differs is a bare question mark");

    assert!(
        binding_defect(&rules, &cert).is_some_and(|w| w.starts_with("D2")),
        "this must still be classified as D2 and not as something new"
    );

    let lean = run_lean(&rules, &asserted, &cert);
    let isa = run_isabelle(&rules, &asserted, &cert);
    assert_eq!(
        lean.exit, 1,
        "Lean must now refuse the empty-named variable rather than send it to the empty term"
    );
    assert_eq!(isa.exit, 1, "Isabelle rejects: binding_incomplete, as it always did");
    assert_eq!(lean, isa, "D2c is resolved: the two kernels now read these bytes the same way");

    // The control, over the probe the fuzzer started from. Its `?o` is intact and its
    // binding covers the rule, and both kernels accept it, so the refusal above is about
    // the typo and not about anything else in this family of files.
    let ok_rules = differential_fixture("probe_degen2_rules.tsv");
    let ok_asserted = differential_fixture("probe_degen2_asserted.tsv");
    let ok_cert = differential_fixture("probe_degen2_cert.tsv");
    let lean = run_lean(&ok_rules, &ok_asserted, &ok_cert);
    let isa = run_isabelle(&ok_rules, &ok_asserted, &ok_cert);
    assert_eq!(lean.exit, 0, "the un-fuzzed probe must still be accepted by Lean");
    assert_eq!(isa.exit, 0, "and by Isabelle");
    assert_eq!(lean, isa);
}

/// **D2b, the sharp form of D2, resolved by decision 0008.** A rule whose HEAD carries a
/// variable its body never binds, and a certificate that does not bind it either.
///
/// The rule is `?s <p> ?o -> ?s <q> ?z`. The graph is one ordinary triple of IRIs,
/// `<a> <p> <b>`; nothing about it is contrived. The certificate binds `s` and `o`, omits
/// `z`, and claims the conclusion `<a> <q> z`.
///
/// Lean used to accept it. `substOf` sent the unbound `z` to the string `"z"`, the head
/// instantiated to exactly the claimed conclusion, and the run reported
/// `entailed_under_supplied_rules`. Isabelle rejected it, `R_binding_incomplete`.
///
/// This was the same hole as D2 and a worse consequence. The accepted conclusion contained
/// the term `z`, which is not an IRI, not a blank node and not a literal: it is not RDF,
/// and no serialiser can write it. Lean's term type is an opaque string, so nothing in
/// the checker noticed. The term was never written by the certificate's author; it was
/// minted out of a VARIABLE NAME in the rule file.
///
/// It was still not unsoundness. `EntailsR` is a statement about `Interp`, whose `ι` is
/// total over terms, so `<a> <q> z` genuinely holds in every interpretation that models
/// the graph and satisfies the rule. The theorem was true. What was false is the thing a
/// reader takes the theorem to be about.
///
/// The engine already refused to EVALUATE such a rule: `parse_rules` in `src/reason.rs`
/// rejects "a head variable the body never binds", and `reason_horn_emit_test.rs` gates
/// it. That guard was in the PRODUCER only. `oo-horn` is a checker anyone may point at any
/// rule file, and it had no such guard, so the guarantee evaporated exactly when the
/// certificate did not come from this engine — which is the only case in which an
/// independent checker is worth having. Decision 0008 puts the guard in the CHECKER, which
/// is where a guard about what a certificate means has to live.
///
/// What it does NOT do is make the conclusion writable RDF. Bind `z` explicitly and both
/// kernels accept `<a> <q> z`. See `the_refusal_costs_no_certificate_anybody_meant`, which
/// is where that boundary is pinned so nobody reads this test as closing it.
#[test]
fn resolved_d2b_an_unsafe_rule_head_is_refused_by_both() {
    if skip() {
        return;
    }
    let rules = differential_fixture("unsafehead_rules.tsv");
    let asserted = differential_fixture("unsafehead_asserted.tsv");
    let cert = differential_fixture("unsafehead_cert.tsv");
    if common::skip_unless(
        rules.exists() && asserted.exists() && cert.exists(),
        "isabelle/fixtures-differential/unsafehead_*.tsv",
        "they are committed",
    ) {
        return;
    }

    // The conclusion the certificate claims really does carry a bare `z`. If this ever
    // stops being true the test below is checking something else.
    let text = std::fs::read_to_string(&cert).unwrap();
    let step = parse_step(text.lines().next().unwrap()).expect("the probe must parse");
    assert_eq!(step.concl[2], "z", "the claimed conclusion must carry the bare term `z`: {text}");
    assert!(
        !step.binds.iter().any(|(k, _)| k == "z"),
        "and the binding must NOT mention z, or there is nothing to fabricate: {text}"
    );

    let lean = run_lean(&rules, &asserted, &cert);
    let isa = run_isabelle(&rules, &asserted, &cert);
    assert_eq!(
        lean.exit, 1,
        "Lean must no longer accept a conclusion containing a term minted from a variable's \
         name"
    );
    assert_eq!(lean.verdict, None);
    assert_eq!(isa.exit, 1, "Isabelle rejects: binding_incomplete, as it always did");
    assert_eq!(lean, isa, "D2b is resolved: the two kernels now read these bytes the same way");
}

/// The corners of the term format that no generated certificate reaches. Both
/// formalisations flag them as untested and each names a different reason to worry:
/// Isabelle's DECISION M38 says the fixtures are pure ASCII, "which is exactly the
/// circumstance in which a String.literal implementation would stay silent", and its M1
/// says terms are opaque byte strings with no value-space normalisation.
///
/// Both concerns are settled here, empirically, in the agreeing direction:
///
/// * A non-ASCII IRI round-trips through both checkers, and altering one byte of it is
///   rejected by both. Isabelle's verified core is over `string = char list` and its
///   driver uses SML's own `String.explode` on a byte string, so the bytes reach the
///   theorem unchanged; the ASCII trap M38 warns about is a trap that was avoided, not
///   one that was never there.
/// * `"01"^^xsd:integer` and `"1"^^xsd:integer` are ONE value and TWO terms, and both
///   checkers reject a certificate that swaps one for the other. M1's scope note holds
///   on both sides: no datatype-aware rule is in the table, and neither checker
///   normalises.
/// * `http://ex.org/a` without angle brackets does not collide with `<http://ex.org/a>`
///   on either side, which is the injectivity that Isabelle's DECISION D-PARSE-2 needs
///   for bracket-stripping to be invisible to the accept/reject bit. Isabelle strips the
///   brackets and Lean does not, so if the renaming were not injective this is where the
///   two would part company.
#[test]
fn the_untested_corners_of_the_term_format_agree() {
    if skip() {
        return;
    }
    // (probe, expected exit, what it is for). The exit code rather than a bare bit,
    // because 2 is a THIRD answer and a checker that turned an unparseable file into a
    // rejection would still look like agreement on the accept/reject bit alone.
    let expected: &[(&str, i32, &str)] = &[
        ("utf8", 0, "non-ASCII UTF-8 IRIs in the rule, the graph and the certificate"),
        ("utf8bad", 1, "one byte of a non-ASCII IRI altered in the certificate"),
        ("literals", 0, "a language-tagged literal and a datatyped literal"),
        ("litdt", 1, "\"01\"^^xsd:integer standing in for \"1\"^^xsd:integer"),
        ("bnode", 0, "blank node labels in every position"),
        ("degen", 0, "the degenerate spellings <> and _:"),
        ("degen2", 0, "a bare < and an empty field as terms"),
        ("inject", 1, "an IRI without its angle brackets, against one with them"),
        ("emptybody", 0, "a rule with an empty body, cited with no premises"),
        ("repeated", 0, "a rule whose body repeats one atom, cited twice"),
        ("extrabind", 0, "a binding for a variable the rule never mentions"),
        ("qmarkkey", 1, "binding keys written WITH the question mark"),
        // Degenerate files, where an off-by-one in either parser would show up.
        ("emptyall", 0, "an empty rule table and an empty certificate"),
        ("emptyrules", 1, "an empty rule table and a step that cites rule 0"),
        ("emptygraph", 0, "an empty graph and a rule with an empty body"),
        ("allempty", 0, "all three files empty"),
        ("duprow", 0, "the built-in table with one row duplicated after the cited index"),
        ("trailtab", 2, "a trailing tab on every certificate line"),
        ("crlf", 1, "CRLF line endings, so every last field carries a stray carriage return"),
        ("blank", 0, "blank lines at the start and end of all three files"),
        ("nonl", 0, "no trailing newline anywhere"),
        ("hugeidx", 1, "a rule index of 200 digits"),
    ];
    let dir = isabelle_dir().join("fixtures-differential");

    // Two probes are about BYTES, and `.gitattributes` says `*.tsv text eol=lf`, which
    // would rewrite one of them into a file that tests nothing while still reporting
    // green. A `-text` exception keeps the carriage returns; this is the guard that
    // notices if the exception is ever dropped, because a probe that has been normalised
    // away passes silently and that is the failure this repository exists to catch.
    for f in ["probe_crlf_rules.tsv", "probe_crlf_asserted.tsv", "probe_crlf_cert.tsv"] {
        let p = dir.join(f);
        if p.exists() {
            let bytes = std::fs::read(&p).unwrap();
            assert!(
                bytes.contains(&b'\r'),
                "{f} has lost its carriage returns. `.gitattributes` normalises *.tsv to \
                 LF and this file needs the `-text` exception; without the CR it is a \
                 probe that passes without probing anything"
            );
        }
    }
    let p = dir.join("probe_nonl_cert.tsv");
    if p.exists() {
        let bytes = std::fs::read(&p).unwrap();
        assert!(
            !bytes.ends_with(b"\n"),
            "probe_nonl_cert.tsv is supposed to end WITHOUT a newline; something has \
             added one and the probe now tests the ordinary case"
        );
    }

    let mut checked = 0;
    for (name, want_exit, what) in expected {
        let r = dir.join(format!("probe_{name}_rules.tsv"));
        let a = dir.join(format!("probe_{name}_asserted.tsv"));
        let c = dir.join(format!("probe_{name}_cert.tsv"));
        if common::skip_unless(
            r.exists() && a.exists() && c.exists(),
            &format!("isabelle/fixtures-differential/probe_{name}_*.tsv"),
            "they are committed",
        ) {
            continue;
        }
        let lean = run_lean(&r, &a, &c);
        let isa = run_isabelle(&r, &a, &c);
        assert_eq!(lean, isa, "the two kernels disagree on {name} ({what})");
        assert_eq!(
            lean.exit, *want_exit,
            "{name} ({what}) was expected to exit {want_exit}, and both kernels say \
             {}. Agreement on the WRONG answer is the failure mode a differential cannot \
             see on its own, which is why the expected answer is written down here",
            lean.exit
        );
        checked += 1;
    }
    assert_eq!(checked, expected.len(), "every probe must have run");

    // Every committed probe must appear in the table above. Without this, adding a
    // fixture and forgetting to say what it should do would leave it checked for
    // agreement only, which is the weaker half.
    let listed: std::collections::BTreeSet<&str> = expected.iter().map(|(n, _, _)| *n).collect();
    let on_disk: Vec<String> = probe_cases()
        .iter()
        .map(|c| c.name.trim_start_matches("probe-").to_string())
        .collect();
    for name in &on_disk {
        assert!(
            listed.contains(name.as_str()),
            "isabelle/fixtures-differential/probe_{name}_*.tsv is committed but this test \
             does not say what it should do"
        );
    }
}

/// Agreement is about the checking discipline. It is not a proof, and nothing in this
/// file's output may be read as one.
///
/// This test exists because the claim is easy to overstate and the overstatement is the
/// exact failure the project is built to catch. It pins the two facts that bound what
/// the differential can mean: a certificate that is accepted over a USER table earns the
/// relativised verdict on BOTH sides, and the two sides name DIFFERENT theorems for it,
/// because each side's warrant is its own.
#[test]
fn agreement_is_about_the_definitions_and_is_not_itself_a_proof() {
    if skip() {
        return;
    }
    let rules = horn_fixture("user_rules.tsv");
    let asserted = horn_fixture("asserted.tsv");
    let cert = horn_fixture("good.tsv");

    let lean_out = Command::new(lean_checker())
        .arg("check")
        .arg(&rules)
        .arg(&asserted)
        .arg(&cert)
        .output()
        .expect("run oo-horn");
    let lean_json = String::from_utf8_lossy(&lean_out.stdout).to_string();
    let isa_out = Command::new(isabelle_checker())
        .arg("check")
        .arg(&rules)
        .arg(&asserted)
        .arg(&cert)
        .output()
        .expect("run oo-horn-isabelle");
    let isa_json = String::from_utf8_lossy(&isa_out.stdout).to_string();

    assert_eq!(json_field(&lean_json, "verdict").as_deref(), Some("entailed_under_supplied_rules"));
    assert_eq!(json_field(&isa_json, "verdict").as_deref(), Some("entailed_under_supplied_rules"));

    let lean_thm = json_field(&lean_json, "theorem").unwrap();
    let isa_thm = json_field(&isa_json, "theorem").unwrap();
    assert_ne!(
        lean_thm, isa_thm,
        "each side must name ITS OWN theorem. If they ever printed the same string, a \
         reader could take one side's agreement as the other side's warrant, which is \
         precisely the laundering this layer exists to prevent"
    );
    assert!(lean_thm.starts_with("OOCert."), "{lean_thm}");
    assert!(isa_thm.starts_with("OOHorn."), "{isa_thm}");

    for json in [&lean_json, &isa_json] {
        assert!(
            json.contains("assumed, not checked"),
            "a run over a user table must say the rules are assumed: {json}"
        );
    }
}

/// **The measure, not the claim.** How deep the corpus actually is.
///
/// A differential over 1,718 certificates sounds like a lot until you ask how many of
/// them can express the property the checker's induction rests on. A step may cite only
/// what came strictly before it and never itself; a ONE-STEP certificate cannot violate
/// that and cannot exercise it either, because every premise it has is asserted and the
/// ordering logic is never reached. Before the deep cases were added, exactly one base
/// certificate in the corpus contained a step citing an earlier step's conclusion, so
/// the discipline was differentially tested by a single two-step fixture.
///
/// This test prints the distribution and holds a floor under it. It deliberately does
/// NOT require either proof assistant: the shape of the corpus is a fact about the
/// files, and it should still be measurable on a machine that cannot run the kernels.
///
/// It also CHECKS THE DOCUMENTS that quote these figures against the figures, at the
/// bottom, rather than in a test of its own. Generating this corpus means running the
/// engine over every ontology the repository ships, three tests in this file already pay
/// for that, and a fourth copy of the cost to compare two strings is not worth it. The
/// reason the check exists at all is in the comment where it is made.
#[test]
fn the_corpus_exercises_prefix_visibility_at_depth() {
    let scratch = scratch_dir();
    let mut bases = fixture_cases();
    bases.extend(probe_cases());
    bases.extend(deep_fixture_cases());
    bases.extend(deep_cases(&scratch));
    bases.extend(generate_base_certificates(&scratch));
    let mutants = mutate(&bases, &scratch);
    let fuzzed = fuzz(&bases, &scratch);

    let report = |label: &str, cases: &[Case]| -> (usize, usize) {
        let shapes: Vec<Shape> = cases.iter().map(shape_of_case).collect();
        let mut hist: BTreeMap<usize, usize> = BTreeMap::new();
        for s in &shapes {
            *hist.entry(s.depth).or_default() += 1;
        }
        let uses = shapes.iter().filter(|s| s.citing_earlier > 0).count();
        let violates = shapes.iter().filter(|s| s.citing_self_or_later > 0).count();
        println!(
            "  {label:<10} {:>5} certificates   depth {}   max fan-out {}\n    \
             {uses:>5} cite an earlier conclusion, {violates:>5} cite themselves or a later step",
            cases.len(),
            hist.iter().map(|(d, n)| format!("{d}:{n}")).collect::<Vec<_>>().join(" "),
            shapes.iter().map(|s| s.fan_out).max().unwrap_or(0),
        );
        (uses, violates)
    };

    println!("prefix-visibility coverage (depth d:count, d = longest derivation chain)");
    let (base_uses, base_violates) = report("base", &bases);
    let (mut_uses, mut_violates) = report("mutated", &mutants);
    let (fuzz_uses, fuzz_violates) = report("fuzzed", &fuzzed);
    let uses = base_uses + mut_uses + fuzz_uses;
    let violates = base_violates + mut_violates + fuzz_violates;
    println!(
        "  TOTAL      {} rows exercise the discipline: {uses} rest on it to be accepted, \
         {violates} must be rejected by it",
        uses + violates
    );

    let deepest = bases.iter().map(|c| shape_of_case(c).depth).max().unwrap_or(0);
    let widest = bases.iter().map(|c| shape_of_case(c).fan_out).max().unwrap_or(0);
    assert!(
        deepest >= 15,
        "the deepest base certificate is {deepest} steps of chaining. A shallow corpus \
         tests the checker's ordering logic on one or two links, which is where an \
         off-by-one hides"
    );
    assert!(widest >= 10, "no base certificate has {widest} or more steps citing one conclusion");
    assert!(
        base_uses >= 8,
        "only {base_uses} base certificates cite an earlier conclusion at all; the corpus \
         has gone flat again and the discipline is back to being tested by one fixture"
    );
    assert!(
        violates >= 40,
        "only {violates} rows in the whole corpus cite themselves or a later step, so the \
         REJECTING half of the discipline is barely exercised"
    );

    // ── the documents, against the corpus that was just counted ──────────────
    //
    // The house rule is that a figure next to the thing it describes must be DERIVED and
    // never typed, and this corpus is the case that made the rule. Three documents quoted
    // it, two different versions of it were measured on two branches eleven minutes apart
    // and merged separately, and what landed on main was a README sentence reporting zero
    // divergent rows next to an inventory reporting fifty-four, neither of them measured
    // against the tree they were committed to. `isabelle/README.md` said so about itself
    // and was left stale on purpose, because the work that found it was not allowed to
    // touch that directory.
    //
    // The corpus TOTAL is the wrong thing to pin on its own: it moves whenever a Turtle
    // file lands in one of the ten directories `source_graphs` reads, and a growing
    // corpus is not a regression. What is checked instead is that every document quoting
    // it quotes THIS measurement, so a corpus that grows fails until the prose is
    // corrected, and a correction that reaches one document out of two fails as well.
    // That is the shape `tests/readme_claims_test.rs` uses for the tool count.
    //
    // The divergence count is deliberately NOT here. It cannot be measured without both
    // kernels, and `the_two_kernels_agree_on_the_whole_corpus` asserts it directly rather
    // than in prose: the documents say the two agree, and that sentence is true exactly
    // when that test passes.
    let total = bases.len() + mutants.len() + fuzzed.len();
    let exercising = uses + violates;
    let claims: [(&str, &str, String); 2] = [
        (
            "README.md",
            "the paragraph on what the discipline has caught",
            format!(
                "a corpus of {} certificates, {exercising} of which exercise the ordering \
                 property",
                commas(total)
            ),
        ),
        (
            "docs/reasoning-systems-inventory.md",
            "the paragraph on corpus depth",
            format!(
                "{} rows reaching depth {deepest} and fan-out {widest}, of which \
                 {exercising} exercise the ordering discipline",
                commas(total)
            ),
        ),
    ];

    // Whitespace is collapsed on both sides before comparing. These documents are hard
    // wrapped at about a hundred columns, so any phrase long enough to be worth checking
    // straddles a line break, and a literal `contains` would fail on prose that is
    // correct and pass on nothing. It is the claim being checked, not its typesetting.
    let flatten = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");

    let mut wrong = Vec::new();
    for (file, where_, claim) in &claims {
        let text = std::fs::read_to_string(repo().join(file))
            .unwrap_or_else(|_| panic!("{file} must exist: a claim is checked against it"));
        if !flatten(&text).contains(&flatten(claim)) {
            wrong.push(format!("{file}, {where_}:\n    expected to find {claim:?}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "The corpus this file generates is {} certificates, {exercising} of them \
         exercising prefix visibility, deepest base chain {deepest} and widest fan-out \
         {widest}, and a document says otherwise.\n\n{}\n\nCorrect the document rather \
         than this test: the corpus is the measurement and the prose is the claim. If a \
         sentence was deliberately reworded, change the expected phrase here in the same \
         commit and say why.",
        commas(total),
        wrong.join("\n\n")
    );

    let _ = std::fs::remove_dir_all(&scratch);
}

/// Thousands separators, because the documents are written for people and a claim checked
/// against `2075` would pass over prose that says `2,075`.
fn commas(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.char_indices() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// **Self-support, in the one form that isolates it.** A step whose only non-asserted
/// premise is its OWN conclusion, and which is otherwise perfect.
///
/// `tests/fixtures/horn/deep/self_support_cert.tsv` cites `rdfs9` with `a` and `b` bound
/// to the SAME class, so the rule's body is `<a> type <A>`, `<A> subClassOf <A>` and its
/// head is `<a> type <A>`. The head and the first premise are then the same triple. The
/// binding covers every variable of the rule, has no repeated key, and instantiates the
/// body into exactly the premises the step lists: nothing a duplicate-key or
/// coverage check would catch is wrong with it.
///
/// `self_support_asserted.tsv` supplies `<A> subClassOf <A>` and NOTHING else, so the
/// remaining premise is unavailable unless a step may cite itself. Both checkers reject,
/// and Isabelle names the reason: `premise_unknown:0`.
///
/// Why it is worth committing rather than mutating into existence: this is the case the
/// induction in both formalisations is FOR. A checker that resolved premises against
/// "the asserted graph plus every conclusion in the file" rather than "plus every
/// conclusion strictly before this one" accepts it, and having accepted it accepts the
/// derivation of anything at all, since a self-supporting step needs no input. The
/// shipped fixture set could not express it: `bad_self.tsv` swaps a premise for the
/// conclusion under a binding that no longer instantiates the body, so it is rejected
/// for two reasons at once and says nothing about which.
#[test]
fn a_step_may_not_cite_its_own_conclusion() {
    if skip() {
        return;
    }
    let dir = repo().join("tests").join("fixtures").join("horn").join("deep");
    let rules = horn_fixture("builtin_rules.tsv");
    let asserted = dir.join("self_support_asserted.tsv");
    let cert = dir.join("self_support_cert.tsv");
    if common::skip_unless(
        asserted.exists() && cert.exists(),
        "tests/fixtures/horn/deep/self_support_*.tsv",
        "they are committed",
    ) {
        return;
    }

    // The shape is asserted from the file, not assumed, or a later edit could turn this
    // into a test of an ordinary rejection without anyone noticing.
    let steps = parse_cert(&std::fs::read_to_string(&cert).unwrap()).expect("it must parse");
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].prems[0], steps[0].concl, "premise 0 must BE the conclusion");
    let known = asserted_triples(&asserted);
    assert!(!known.contains(&steps[0].concl), "and it must not be asserted, or nothing is proved");
    assert!(known.contains(&steps[0].prems[1]), "while the other premise must be asserted");

    let lean = run_lean(&rules, &asserted, &cert);
    let isa = run_isabelle(&rules, &asserted, &cert);
    assert_eq!(lean, isa, "the two kernels disagree about self-support: {lean:?} vs {isa:?}");
    assert_eq!(lean.exit, 1, "a self-supporting step must be REJECTED, not accepted and not a parse error");
}

/// **Mutual support.** Two steps, each citing the other's conclusion, and a control that
/// shows the rejection is about the ordering and about nothing else in the three files.
///
/// `mutual_cert.tsv` is two `rdfs9` steps over `<A> subClassOf <B>` and
/// `<B> subClassOf <A>`: step 0 concludes `<a> type <B>` from `<a> type <A>`, and step 1
/// concludes `<a> type <A>` from `<a> type <B>`. Each premise the other step supplies.
/// `mutual_asserted.tsv` asserts only the two subclass axioms, so `<a>` is never said to
/// be of any type at all, and the pair manufactures both conclusions out of each other.
///
/// The control is the SAME certificate against `mutual_seeded_asserted.tsv`, which adds
/// the single triple `<a> type <A>`. Now step 0's first premise is asserted, step 1's is
/// step 0's conclusion, the order is legal, and both checkers accept with the ABSOLUTE
/// verdict. One triple is the whole difference between a cycle and a derivation, and the
/// discipline is the only thing that tells them apart.
#[test]
fn two_steps_may_not_support_each_other() {
    if skip() {
        return;
    }
    let dir = repo().join("tests").join("fixtures").join("horn").join("deep");
    let rules = horn_fixture("builtin_rules.tsv");
    let cert = dir.join("mutual_cert.tsv");
    let cycle = dir.join("mutual_asserted.tsv");
    let seeded = dir.join("mutual_seeded_asserted.tsv");
    if common::skip_unless(
        cert.exists() && cycle.exists() && seeded.exists(),
        "tests/fixtures/horn/deep/mutual_*.tsv",
        "they are committed",
    ) {
        return;
    }

    let steps = parse_cert(&std::fs::read_to_string(&cert).unwrap()).expect("it must parse");
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].prems[0], steps[1].concl, "step 0 must cite step 1's conclusion");
    assert_eq!(steps[1].prems[0], steps[0].concl, "and step 1 must cite step 0's");
    let unseeded = asserted_triples(&cycle);
    assert!(
        !unseeded.contains(&steps[0].concl) && !unseeded.contains(&steps[1].concl),
        "neither conclusion may be asserted, or the cycle is not load-bearing"
    );
    assert_eq!(
        asserted_triples(&seeded).difference(&unseeded).count(),
        1,
        "the control must differ by exactly ONE triple"
    );

    let lean = run_lean(&rules, &cycle, &cert);
    let isa = run_isabelle(&rules, &cycle, &cert);
    assert_eq!(lean, isa, "the two kernels disagree about mutual support: {lean:?} vs {isa:?}");
    assert_eq!(lean.exit, 1, "a pair of steps supporting each other must be REJECTED");

    let lean = run_lean(&rules, &seeded, &cert);
    let isa = run_isabelle(&rules, &seeded, &cert);
    assert_eq!(lean, isa, "the two kernels disagree on the control: {lean:?} vs {isa:?}");
    assert_eq!(
        lean.exit, 0,
        "the SAME certificate, with one triple asserted, must be accepted; if it is not, \
         the rejection above was about something other than the ordering and this pair \
         proves nothing"
    );
    assert_eq!(lean.verdict.as_deref(), Some("entailed"));
}

/// The deep certificates are accepted, and are deep. Two separate claims: a generator
/// that quietly emitted flat certificates would still pass the differential, because
/// both kernels agree on flat certificates too.
#[test]
fn the_generated_deep_certificates_check_and_are_deep() {
    if skip() {
        return;
    }
    let scratch = scratch_dir();
    let cases = deep_cases(&scratch);
    assert!(!cases.is_empty(), "the deep generator produced nothing");
    for case in &cases {
        let shape = shape_of_case(case);
        assert!(
            shape.depth >= 1 && shape.citing_earlier >= 1,
            "{} is flat: {shape:?}. A base case that cites nothing exercises no ordering",
            case.name
        );
        let lean = run_lean(&case.rules, &case.asserted, &case.cert);
        let isa = run_isabelle(&case.rules, &case.asserted, &case.cert);
        assert_eq!(lean, isa, "the two kernels disagree on {}: {lean:?} vs {isa:?}", case.name);
        assert_eq!(
            lean.exit, 0,
            "{} is a real rdfs9 derivation in a legal order and must be ACCEPTED (depth \
             {}, fan-out {}); if it is rejected the generator is wrong and every deep \
             mutation below is mutating rubbish",
            case.name, shape.depth, shape.fan_out
        );
        assert_eq!(lean.verdict.as_deref(), Some("entailed"), "{} over the built-in table", case.name);
    }
    let _ = std::fs::remove_dir_all(&scratch);
}
