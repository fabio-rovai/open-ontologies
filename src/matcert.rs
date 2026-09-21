//! A numeric claim, and the two different things this repository can say
//! about one.
//!
//! An ontology pipeline computes numbers: a similarity score, an alignment
//! metric, a risk figure. Until now every one of those was handed on with the
//! standing of an assertion, while the triples beside them carried
//! certificates. This closes that, and it closes it in two pieces that are
//! deliberately NOT merged, because they support different sentences.
//!
//! # 1. Certified, and it means what it says
//!
//! [`check`] writes a `matcert/1` file and runs `oo-matcert`, which recomputes
//! every entry from the definition and, on acceptance, prints
//! `MatCert.mul_of_check`. The token comes back through
//! [`crate::verdict::CheckerRun::accepted_naming`] like every other certified
//! word here, so it cannot be spelled without a run that earned it.
//!
//! Integers only. That is not a limitation to be lifted later, it is the
//! condition under which the sentence is true.
//!
//! # 2. An opinion, with its bound printed
//!
//! [`freivalds`] multiplies both sides by random vectors and compares, which
//! costs O(n²) per trial against O(n³) for the recomputation. Its soundness is
//! probabilistic: a wrong product survives k trials with probability at most
//! 2⁻ᵏ. Stating THAT needs probability theory, and `lean/` has no Mathlib by
//! deliberate choice, so no theorem here covers it. It returns this engine's
//! own word and prints the bound, and the word is not `certified`.
//!
//! # Floating point is refused, and this is the important paragraph
//!
//! [`freivalds_f64`] exists and reports a TOLERANCE rather than an equality.
//! A proof over the reals says nothing about IEEE-754: rounding makes a
//! correct product compare unequal and an incorrect one compare equal, and a
//! certificate proved over ℝ and run on hardware is unsound silently. So the
//! float path never reaches Lean, never names a theorem, and carries the
//! tolerance it used in its own report. A caller who needs a guarantee
//! converts to integers, pays O(n³), and gets one.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::verdict::{CheckerBinary, CheckerRun, Certified};

/// Row-major, rectangular. Ragged input is an error rather than padded.
pub type IntMatrix = Vec<Vec<i64>>;

fn rectangular(m: &IntMatrix, w: usize) -> bool {
    m.iter().all(|r| r.len() == w)
}

/// The `matcert/1` file the Lean checker reads.
pub fn write_cert(a: &IntMatrix, b: &IntMatrix, c: &IntMatrix, dir: &Path) -> anyhow::Result<PathBuf> {
    let (m, p, n) = (a.len(), b.len(), c.first().map(|r| r.len()).unwrap_or(0));
    if !rectangular(a, p) || !rectangular(b, n) || !rectangular(c, n) {
        anyhow::bail!(
            "ragged input: A must be {m}x{p}, B {p}x{n}, C {m}x{n}. Refused rather than padded, \
             because a padded row is a different matrix and would be checked as one."
        );
    }
    std::fs::create_dir_all(dir)?;
    let path = dir.join("product.matcert");
    let mut s = format!("matcert/1\ndims\t{m}\t{p}\t{n}\n");
    for (tag, mat) in [("A", a), ("B", b), ("C", c)] {
        for row in mat {
            s.push_str(tag);
            s.push('\t');
            s.push_str(&row.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "));
            s.push('\n');
        }
    }
    std::fs::write(&path, s)?;
    Ok(path)
}

/// Where `oo-matcert` is, by the rule every other checker here follows.
pub fn find_checker() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("OO_MATCERT") {
        let p = PathBuf::from(p);
        return p.exists().then_some(p);
    }
    let built = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lean/.lake/build/bin/oo-matcert");
    built.exists().then_some(built)
}

/// What a run of the checker produced.
pub enum Checked {
    /// Accepted, with the token that proves a checker said so.
    Accepted(Certified, String),
    /// Ran and refused. Exit 1 is a rejection; 2 is a file it could not read.
    Refused { exit: i32, output: String },
    /// No checker on this machine. Never silently a pass.
    Absent(String),
}

/// Write the certificate and run the checker over it.
pub fn check(a: &IntMatrix, b: &IntMatrix, c: &IntMatrix, dir: &Path) -> anyhow::Result<Checked> {
    let path = write_cert(a, b, c, dir)?;
    let Some(bin) = find_checker() else {
        return Ok(Checked::Absent(
            "oo-matcert was not found. Build it with `lake build oo-matcert` in lean/, or set \
             $OO_MATCERT. A missing checker is not an acceptance."
                .to_string(),
        ));
    };
    let mut cmd = Command::new(&bin);
    cmd.arg(&path);
    // The run names the file it is about (#164). It is the certificate, which
    // is also the only thing on the command line, so the digest the token
    // carries is a digest of exactly the claim that was checked.
    let run = CheckerRun::spawn(&CheckerBinary::found_at(bin), cmd, &[&path])?;
    match run.accepted_naming(&["MatCert.mul_of_check"]) {
        Some(token) => Ok(Checked::Accepted(token, run.output())),
        None => Ok(Checked::Refused { exit: run.exit(), output: run.output() }),
    }
}

/// Freivalds over the integers: this engine's own opinion, with its bound.
///
/// `trials` random 0/1 vectors. A wrong product survives all of them with
/// probability at most 2⁻ᵗʳⁱᵃˡˢ. Forty is the default elsewhere in the
/// literature because 2⁻⁴⁰ is below the rate at which hardware flips a bit,
/// which is the honest comparison: past that point the arithmetic is not the
/// least reliable thing in the room.
///
/// Deterministic for a given seed, so a report can be reproduced.
pub fn freivalds(a: &IntMatrix, b: &IntMatrix, c: &IntMatrix, trials: u32, seed: u64) -> serde_json::Value {
    let m = a.len();
    let p = b.len();
    let n = c.first().map(|r| r.len()).unwrap_or(0);
    if !rectangular(a, p) || !rectangular(b, n) || !rectangular(c, n) || c.len() != m {
        return serde_json::json!({
            "verdict": "not_checked_shapes_disagree",
            "means": "the three matrices do not have compatible shapes, so there is no claim \
                      to check.",
        });
    }
    let mut state = seed | 1;
    let mut next = || {
        // xorshift64*, so the report is reproducible from the seed.
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    for _ in 0..trials {
        let r: Vec<i64> = (0..n).map(|_| (next() & 1) as i64).collect();
        // B·r, then A·(B·r)
        let br: Vec<i64> = (0..p)
            .map(|i| (0..n).map(|j| b[i][j].wrapping_mul(r[j])).fold(0i64, |s, v| s.wrapping_add(v)))
            .collect();
        let abr: Vec<i64> = (0..m)
            .map(|i| (0..p).map(|k| a[i][k].wrapping_mul(br[k])).fold(0i64, |s, v| s.wrapping_add(v)))
            .collect();
        let cr: Vec<i64> = (0..m)
            .map(|i| (0..n).map(|j| c[i][j].wrapping_mul(r[j])).fold(0i64, |s, v| s.wrapping_add(v)))
            .collect();
        if abr != cr {
            return serde_json::json!({
                "verdict": "refuted_by_this_engine",
                "trials_run": trials,
                "means": "a random vector separated A(Br) from Cr, so the claimed product is \
                          WRONG. A refutation from Freivalds is certain, unlike its \
                          acceptance: one counterexample settles it.",
            });
        }
    }
    serde_json::json!({
        "verdict": "survived_freivalds",
        "trials": trials,
        "error_bound": format!("2^-{trials}"),
        "seed": seed,
        "means": "the claimed product survived every trial. This is an OPINION of this engine \
                  and not a certificate: the bound is probabilistic and no theorem in lean/ \
                  covers it, because stating it needs probability theory this development \
                  deliberately has no dependency for. For a certified answer run `check`, \
                  which recomputes every entry in O(n^3) and comes back with a token from a \
                  checker that named the statement it discharged. The name of that statement \
                  is deliberately NOT repeated here: it should appear only where it was \
                  earned, so that grepping for it finds acceptances and nothing else.",
    })
}

/// The same idea over doubles, and the reason it can never be more than a
/// smoke test.
///
/// Reports agreement within a TOLERANCE, never equality. Two implementations
/// of the same correct product disagree in the last bits, and a wrong one can
/// agree inside any tolerance you pick. The tolerance travels in the report so
/// a reader can judge it rather than inherit it.
pub fn freivalds_f64(
    a: &[Vec<f64>],
    b: &[Vec<f64>],
    c: &[Vec<f64>],
    trials: u32,
    tolerance: f64,
) -> serde_json::Value {
    let m = a.len();
    let p = b.len();
    let n = c.first().map(|r| r.len()).unwrap_or(0);
    let mut worst = 0.0f64;
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    for _ in 0..trials {
        let r: Vec<f64> = (0..n)
            .map(|_| {
                state ^= state >> 12;
                state ^= state << 25;
                state ^= state >> 27;
                (state & 1) as f64
            })
            .collect();
        for i in 0..m {
            let br: Vec<f64> =
                (0..p).map(|k| (0..n).map(|j| b[k][j] * r[j]).sum::<f64>()).collect();
            let abr: f64 = (0..p).map(|k| a[i][k] * br[k]).sum();
            let cr: f64 = (0..n).map(|j| c[i][j] * r[j]).sum();
            worst = worst.max((abr - cr).abs());
        }
    }
    serde_json::json!({
        "verdict": if worst <= tolerance { "within_tolerance" } else { "outside_tolerance" },
        "worst_absolute_difference": worst,
        "tolerance": tolerance,
        "trials": trials,
        "means": "AGREEMENT WITHIN A TOLERANCE, never equality, and never a certificate. A \
                  proof over the real numbers says nothing about IEEE-754: rounding makes a \
                  correct product compare unequal and an incorrect one compare equal. This \
                  path never reaches Lean and names no theorem. If you need a guarantee, \
                  convert to integers and use `check`.",
    })
}
