//! **The animated asset in the README states the checker's own output.**
//!
//! `docs/assets/demo-certify.svg` shows a terminal printing `"asserted":3`,
//! `"derivations":3`, a theorem name, and then `exit 1` on a forged run. Those
//! are load-bearing claims about this repository, sitting in the first screen
//! a reader sees, and nothing about an SVG stops them drifting away from the
//! code once a fixture or a verdict string changes.
//!
//! So the figures are not trusted here, they are re-derived: the committed
//! fixtures are fed to the same checker the picture depicts, and the numbers
//! it prints must be the numbers drawn. The forged run must fail, because an
//! asset that advertises a refusal while the checker quietly accepts would be
//! worse than no asset at all.
//!
//! The premise test needs no Lean and always runs. The checker test needs a
//! Lean toolchain and skips loudly without one, which the `lean` CI job turns
//! into a failure with `OO_REQUIRE_FIXTURES=1`.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn lean_dir() -> PathBuf {
    repo().join("lean")
}

fn fixtures() -> PathBuf {
    repo().join("tests").join("fixtures").join("horn").join("supplier")
}

fn svg() -> String {
    let p = repo().join("docs").join("assets").join("demo-certify.svg");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// Every integer the text writes for `"<key>":`. The asset prints the same
/// count more than once, so this returns all of them and the caller insists
/// they agree.
fn stated(hay: &str, key: &str) -> Vec<u64> {
    let pat = format!("\"{key}\":");
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(p) = hay[i..].find(&pat) {
        let s = i + p + pat.len();
        let digits: String = hay[s..].chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            out.push(digits.parse().expect("digits parse"));
        }
        i = s;
    }
    out
}

fn rows(path: &Path) -> Vec<Vec<String>> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect()
}

/// The asset says, in words, "The premises were untouched. The conclusion did
/// not follow." That is the whole argument for why the refusal is interesting:
/// a checker that rejected a certificate whose premises had also been edited
/// would be proving nothing about forged conclusions. So the fixtures have to
/// actually have that shape.
#[test]
fn the_forged_certificate_differs_only_in_a_conclusion() {
    let honest = rows(&fixtures().join("derivations.tsv"));
    let forged = rows(&fixtures().join("forged.tsv"));
    assert_eq!(honest.len(), forged.len(), "the forgery added or dropped a derivation");

    // Layout: 0 rule, 1 binding count, 2..8 bindings, 8..11 conclusion,
    // 11.. premises.
    const CONCLUSION: std::ops::Range<usize> = 8..11;
    const PREMISES_FROM: usize = 11;

    let differing: Vec<usize> = (0..honest.len()).filter(|&i| honest[i] != forged[i]).collect();
    assert_eq!(
        differing.len(),
        1,
        "exactly one derivation should be forged, {} differ",
        differing.len()
    );

    let i = differing[0];
    assert_ne!(
        honest[i][CONCLUSION], forged[i][CONCLUSION],
        "the forged row must change its conclusion"
    );
    assert_eq!(
        honest[i][PREMISES_FROM..],
        forged[i][PREMISES_FROM..],
        "the forged row must leave its premises exactly as they were, or the \
         refusal shown in the asset would not be about the conclusion"
    );
}

/// The counts drawn in the terminal pane are the counts the checker prints.
#[test]
fn the_asset_states_the_numbers_the_checker_prints() {
    if skip() {
        return;
    }
    let f = fixtures();
    let (ok, accepted) = run(&f.join("derivations.tsv"));
    assert!(ok, "the honest certificate should be accepted:\n{accepted}");

    let art = svg();
    for key in ["asserted", "derivations"] {
        let drawn = stated(&art, key);
        let printed = stated(&accepted, key);
        assert!(!drawn.is_empty(), "the asset draws no \"{key}\" figure any more");
        assert_eq!(printed.len(), 1, "checker printed \"{key}\" {} times", printed.len());
        for d in &drawn {
            assert_eq!(
                *d, printed[0],
                "the asset draws \"{key}\":{d} but the checker printed {}",
                printed[0]
            );
        }
    }

    for fragment in ["\"verdict\":\"entailed\"", "OOCert.entails_of_builtin_horn"] {
        assert!(
            accepted.contains(fragment),
            "the asset shows {fragment}, absent from the checker output:\n{accepted}"
        );
        assert!(art.contains(fragment), "the asset stopped showing {fragment}");
    }
}

/// And the refusal is real. A picture of a rejection is only worth drawing if
/// the checker actually rejects.
#[test]
fn the_asset_shows_a_refusal_the_checker_really_makes() {
    if skip() {
        return;
    }
    let (ok, out) = run(&fixtures().join("forged.tsv"));
    assert!(!ok, "the forged certificate was accepted:\n{out}");
    assert!(out.contains("\"ok\":false"), "expected a refusal, got:\n{out}");

    let art = svg();
    assert!(art.contains("{\"ok\":false"), "the asset stopped showing the refusal");
    assert!(art.contains("exit 1"), "the asset stopped showing the non-zero exit");
}

fn lake_available() -> bool {
    // Run from `lean/`: elan resolves the toolchain from the working
    // directory, and the crate root has no `lean-toolchain` in its ancestry.
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

/// Build once, then check the supplier fixtures against the built-in rules.
fn run(derivations: &Path) -> (bool, String) {
    static BUILT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    let exe = BUILT.get_or_init(|| {
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
    });
    let out = Command::new(exe)
        .arg("check")
        .arg(repo().join("tests").join("fixtures").join("horn").join("builtin_rules.tsv"))
        .arg(fixtures().join("asserted.tsv"))
        .arg(derivations)
        .output()
        .expect("run oo-horn");
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}
