//! A test that can skip everywhere it runs, and runs nowhere it could not skip, is a
//! gate in name only. This file is the gate on that.
//!
//! `tests/common/mod.rs` already solved half the problem: `skip_unless` prints a marker
//! and, under `OO_REQUIRE_FIXTURES=1`, panics instead of skipping. Any job that installs
//! a toolchain sets the variable, and the suite cannot then quietly shrink inside it.
//!
//! That mechanism only bites on a test that RUNS. It says nothing about a test file that
//! no job invokes at all, and `tests/cross_kernel_differential_test.rs` was exactly that
//! for as long as it existed: it needs Poly/ML as well as lake, no job installed Poly/ML,
//! so it skipped in `build` (the only job that ran it) and was invoked by no leg of
//! `lean` (the only job where it could have been strict). The README states the result of
//! that comparison in its opening pages. A front-page claim was gated by whether a
//! developer happened to have Isabelle installed.
//!
//! `docs/ci-gates.md` recorded the hole in prose, in an open-questions list, with a shell
//! snippet at the bottom for re-checking the page by hand. Prose does not fail. This is
//! that snippet as a test:
//!
//! * every `tests/*.rs` that can skip is either run by some workflow under
//!   `OO_REQUIRE_FIXTURES=1`, or is on `NOT_STRICT_ON_PURPOSE` below with a reason;
//! * every entry on that list is a real file that really can skip, so the list cannot rot
//!   into a blanket exemption;
//! * `docs/ci-gates.md` has a row for each such file whose verdict matches what the
//!   workflows actually say, so the page cannot go stale the way the numbers did;
//! * and no test prints the `SKIPPED_FIXTURE:` marker by hand, which would be counted by
//!   the `build` job and would not panic under `OO_REQUIRE_FIXTURES=1`.
//!
//! It deliberately does NOT require every file to be strict. A contributor without
//! Isabelle, without a licensed crosswalk table and without a downloaded embedding model
//! must still be able to run `cargo test` and get a pass. The property being defended is
//! narrower and is the one that was violated: CI cannot skip in silence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// This file, by name, so the scans below can leave it out.
///
/// Both of them look for a fragment of source text, and this file has to CONTAIN those
/// fragments in order to look for them. Without this it matches itself: the first run in
/// CI reported that `ci_gate_coverage_test.rs` can skip and has no row in the table, on
/// the strength of the string literal `"skip_unless("` three functions down. `file!()`
/// rather than a written-out name, so a rename cannot reintroduce it.
fn this_file() -> &'static str {
    std::path::Path::new(file!())
        .file_name()
        .and_then(|n| n.to_str())
        .expect("file!() always has a file name")
}

/// Files that can skip and are not run strictly anywhere, each with the reason. A file
/// belongs here when the thing it needs is one CI deliberately does not provide; it does
/// NOT belong here because wiring it up looked like work.
///
/// Keep this list in step with the "What is still open" section of `docs/ci-gates.md`.
const NOT_STRICT_ON_PURPOSE: &[(&str, &str)] = &[
    (
        "tstp_derivation_test.rs",
        "34 of its 35 tests read derivations recorded in tests/fixtures/tstp/ and run \
         everywhere. The 35th, `a_live_vampire_run_agrees_with_the_recorded_one`, wants \
         vampire on PATH, and no workflow installs a first-order prover, so \
         OO_REQUIRE_FIXTURES=1 on this file would fail rather than gate. The recorded \
         half is not a substitute for the live half: it pins the checker against two real \
         derivations, not against whatever the installed prover does today. A \
         `brew install vampire` step, or its apt equivalent, is the smallest thing that \
         would close this and let the file go strict",
    ),
    (
        "clinical_test.rs",
        "needs data/crosswalks.parquet, a licensed table the repository deliberately does \
         not carry and `data/` is gitignored",
    ),
    (
        "embed_test.rs",
        "needs the ONNX embedding model `open-ontologies init` downloads; no job downloads it",
    ),
    (
        "embedding_e2e_test.rs",
        "needs the same ONNX model and its tokenizer",
    ),
    (
        "claimcheck_pizza_bench.rs",
        "needs /tmp/pizza_compiled.json from CompileOntology.java; only benchmark.yml has a \
         JDK, it is workflow_dispatch, and it does not produce the file",
    ),
    (
        "reasoner_budget_corpus_bench.rs",
        "its one test is #[ignore]: a wall-clock measurement run by hand, not a gate",
    ),
];

/// Every `tests/*.rs` with a skip path. `tests/common/mod.rs` defines the mechanism and
/// lives a directory down, so the glob does not reach it.
fn files_that_can_skip() -> BTreeSet<String> {
    let dir = repo().join("tests");
    let mut out = BTreeSet::new();
    for entry in std::fs::read_dir(&dir).expect("tests/ must be readable") {
        let path = entry.expect("readable dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path.file_name().expect("file name").to_string_lossy().into_owned();
        if name == this_file() {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("test file must be readable");
        // The CALL and not the identifier. This file names `skip_unless` a dozen times in
        // prose and calls it never, so a bare substring match would put the gate on the
        // gates into its own list of things to worry about.
        if text.contains("skip_unless(") {
            out.insert(name);
        }
    }
    out
}

/// Every `--test NAME` a workflow runs with `OO_REQUIRE_FIXTURES=1`, with the workflow it
/// is in.
///
/// Backslash continuations are joined FIRST. Without that the scan is wrong in the one
/// direction that matters: a leg written across two lines, with the variable on the first
/// and the `cargo test` on the second, is strict and would be read as not strict, so the
/// gate would fail on a leg that is correct. Both shapes are in the tree.
///
/// Carriage returns go before that. `.gitattributes` pins LF on the certificate formats
/// and on RDF, and says why, but not on YAML: the hosted Windows runner checks out with
/// `core.autocrlf=true`, the continuation is then `\ CR LF` rather than `\ LF`, and a join
/// that looked for the second would silently fail to join on one leg of the matrix only.
fn strict_legs() -> BTreeMap<String, String> {
    let dir = repo().join(".github").join("workflows");
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).expect(".github/workflows/ must be readable") {
        let path = entry.expect("readable dir entry").path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "yml" && ext != "yaml" {
            continue;
        }
        let workflow = path.file_name().expect("file name").to_string_lossy().into_owned();
        let text = std::fs::read_to_string(&path).expect("workflow must be readable");
        let joined = text.replace("\r\n", "\n").replace("\\\n", " ");
        for line in joined.lines() {
            if !line.contains("OO_REQUIRE_FIXTURES=1") {
                continue;
            }
            for (i, _) in line.match_indices("--test ") {
                let rest = &line[i + "--test ".len()..];
                let name: String =
                    rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                if !name.is_empty() {
                    out.insert(format!("{name}.rs"), workflow.clone());
                }
            }
        }
    }
    out
}

/// The verdict column of the Rust table in `docs/ci-gates.md`, per file.
fn ci_gates_rows() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(repo().join("docs").join("ci-gates.md"))
        .expect("docs/ci-gates.md must exist: this test checks claims against it");
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("| `") else { continue };
        let Some(end) = rest.find('`') else { continue };
        let name = &rest[..end];
        if !name.ends_with(".rs") {
            continue;
        }
        out.insert(name.to_string(), line.to_string());
    }
    out
}

#[test]
fn no_test_file_can_skip_in_ci_without_saying_so_in_advance() {
    let skippable = files_that_can_skip();
    assert!(
        skippable.len() > 10,
        "only {} test files were found to have a skip path, which means the scan is \
         broken rather than that the suite got tidy",
        skippable.len()
    );

    let strict = strict_legs();
    let excepted: BTreeMap<&str, &str> = NOT_STRICT_ON_PURPOSE.iter().copied().collect();

    let mut ungated = Vec::new();
    for file in &skippable {
        if strict.contains_key(file) || excepted.contains_key(file.as_str()) {
            continue;
        }
        ungated.push(file.clone());
    }

    assert!(
        ungated.is_empty(),
        "These test files can skip, and no workflow runs them under \
         OO_REQUIRE_FIXTURES=1:\n  {}\n\nA skipped test reports `ok`, so each of these can \
         go quiet in CI without turning a tick red. Either add a leg that runs the file \
         with OO_REQUIRE_FIXTURES=1 in a job that installs what it needs, or add it to \
         NOT_STRICT_ON_PURPOSE in tests/ci_gate_coverage_test.rs with the reason CI does \
         not provide that thing. Do not add it to the list to make this pass.",
        ungated.join("\n  ")
    );

    // The other direction. An exception for a file that no longer exists, or that no
    // longer has a skip path, is a licence for the next one.
    let mut rotten = Vec::new();
    for (file, _) in NOT_STRICT_ON_PURPOSE {
        if !skippable.contains(*file) {
            rotten.push(format!("{file}: no such file under tests/, or it can no longer skip"));
        }
        if let Some(workflow) = strict.get(*file) {
            rotten.push(format!("{file}: excepted here, but {workflow} runs it strictly"));
        }
    }
    assert!(
        rotten.is_empty(),
        "NOT_STRICT_ON_PURPOSE has gone stale:\n  {}",
        rotten.join("\n  ")
    );
}

/// The marker belongs to `tests/common/mod.rs` and to nothing else.
///
/// A test that prints `SKIPPED_FIXTURE:` by hand is counted by the `build` job's skip
/// counter and is invisible to `OO_REQUIRE_FIXTURES=1`, because the panic lives in the
/// helper and not in the string. That is not hypothetical: `claimcheck_pizza_bench.rs`
/// printed its own `SKIP:` line and was invisible to both, which is why it now goes
/// through the helper. This keeps the route closed.
#[test]
fn only_the_helper_emits_the_skip_marker() {
    let mut printed = Vec::new();
    for entry in std::fs::read_dir(repo().join("tests")).expect("tests/ must be readable") {
        let path = entry.expect("readable dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if path.file_name().map(|n| n.to_string_lossy() == this_file()) == Some(true) {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("test file must be readable");
        for line in text.lines() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue;
            }
            if (t.contains("println!") || t.contains("print!") || t.contains("write!"))
                && line.contains("SKIPPED_FIXTURE")
            {
                printed.push(format!(
                    "{}: {}",
                    path.file_name().expect("file name").to_string_lossy(),
                    line.trim()
                ));
            }
        }
    }
    assert!(
        printed.is_empty(),
        "These lines print the skip marker without going through \
         common::skip_unless:\n  {}\n\nThe marker is what the `build` job counts and the \
         panic under OO_REQUIRE_FIXTURES=1 is in the helper, so a hand-printed marker is \
         a skip that looks accounted for and is not.",
        printed.join("\n  ")
    );
}

/// `docs/ci-gates.md` is the page somebody reads to answer "does a green tick mean this
/// ran?". It had the right answer and no way to notice when it stopped having it.
#[test]
fn the_ci_gates_page_says_what_the_workflows_say() {
    let skippable = files_that_can_skip();
    let strict = strict_legs();
    let rows = ci_gates_rows();

    let mut wrong = Vec::new();
    for file in &skippable {
        let Some(row) = rows.get(file) else {
            wrong.push(format!("{file}: can skip and has no row in the Rust table"));
            continue;
        };
        let says_strict = row.contains("**strict**");
        let is_strict = strict.contains_key(file);
        if says_strict != is_strict {
            wrong.push(format!(
                "{file}: the workflows make it {}, the page says {}\n    {row}",
                if is_strict { "strict" } else { "not strict" },
                if says_strict { "strict" } else { "not strict" },
            ));
        }
    }

    // A row for a file that cannot skip is not an error, since `fol_model_ingest_test.rs`
    // is named in the prose for exactly that reason, so only rows for missing files are.
    for name in rows.keys() {
        if !repo().join("tests").join(name).exists() {
            wrong.push(format!("{name}: has a row and no file"));
        }
    }

    assert!(
        wrong.is_empty(),
        "docs/ci-gates.md and .github/workflows/ disagree:\n  {}\n\nThe workflows are the \
         measurement and the page is the claim, so correct the page, unless the \
         workflows are what is wrong, in which case correct those.",
        wrong.join("\n  ")
    );
}
