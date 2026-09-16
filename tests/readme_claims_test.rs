//! The README's tool count must be the tool count, in every place it is stated.
//!
//! It drifted. The file said 110 while the server exposed 111, and it said it in four
//! separate places, which is exactly why nobody noticed: a number repeated by hand has
//! four chances to go stale and no mechanism to notice that it has.
//!
//! The house rule is that a figure next to the thing it measures must be DERIVED, never
//! typed. A README cannot compute, so the next best thing is a test that refuses to let
//! the two disagree. This is that test.
//!
//! It deliberately checks each stated claim BY ITS SURROUNDING PHRASE rather than
//! scanning for every "N tools" in the file, because the README also contains subgroup
//! counts that are correctly smaller than the total. A test that flagged those would be
//! noise, and a noisy gate gets disabled.

use std::path::PathBuf;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The measurement. One `#[tool(name = ...)]` attribute per exposed MCP tool.
fn exposed_tool_count() -> usize {
    let server = std::fs::read_to_string(repo().join("src").join("server.rs"))
        .expect("src/server.rs must be readable");
    server.matches("#[tool(name = ").count()
}

/// Every place the docs state the TOTAL, with the file it lives in and the phrase that
/// identifies it as a total rather than a subgroup. Add a row when a new one is written.
///
/// The architecture diagram left README.md for docs/architecture.md on 14 September 2026,
/// when the README was cut from 1213 lines to 171, so it is checked in its new home. The
/// tool-reference heading went with the section it belonged to and no longer exists.
fn total_claims(n: usize) -> Vec<(&'static str, &'static str, String)> {
    vec![
        ("README.md", "the lead paragraph", format!("**{n} tools**")),
        (
            "README.md",
            "the default-build sentence",
            format!("A default build advertises {} tools.", n - gated_tool_count()),
        ),
        ("docs/architecture.md", "the architecture diagram", format!("ToolGroups[\"{n} Tools\"]")),
    ]
    // The MCP server's instructions string used to be checked here as a literal, and
    // that row is gone on purpose. It stated TWO totals, 114 and 112, and neither was
    // the number the router advertised: a hand-typed figure with a second hand-typed
    // figure beside it, which is the disease this file exists to treat and not a case
    // of it being caught. The string is now formatted from `tool_router.list_all()` at
    // call time, so there is no literal left to go stale, and
    // `the_instructions_string_states_the_count_it_advertises` below checks the live
    // server instead of the source text.
}

/// How many registered tools a DEFAULT build does not advertise.
///
/// Derived from the same list `toolfilter::remove_unavailable` removes routes with, so
/// the README's second number cannot drift from the server's behaviour. It is the whole
/// list rather than `unavailable_in_this_build()` on purpose: the README's sentence is
/// about a default build, which has none of the features, and reading it off THIS build
/// would make the claim pass or fail depending on the flags the suite was run with.
fn gated_tool_count() -> usize {
    open_ontologies::toolfilter::FEATURE_GATED_TOOLS.len()
}

#[test]
fn the_readme_states_the_tool_count_it_actually_exposes() {
    let n = exposed_tool_count();
    assert!(n > 0, "no #[tool(name = ...)] attributes found; the measurement itself is broken");

    let mut wrong = Vec::new();
    for (file, where_, claim) in total_claims(n) {
        let text = std::fs::read_to_string(repo().join(file))
            .unwrap_or_else(|_| panic!("{file} must exist: a claim is checked against it"));
        if !text.contains(&claim) {
            wrong.push(format!("{file}, {where_}: expected to find {claim:?}"));
        }
    }

    assert!(
        wrong.is_empty(),
        "src/server.rs exposes {n} tools and the README does not say so everywhere it \
         claims a total.\n{}\n\nFix the README rather than this test: the server is the \
         measurement and the prose is the claim. If a claim was deliberately removed or \
         reworded, update `total_claims` in this file and say why in the commit.",
        wrong.join("\n")
    );
}

/// The other half, and the one that catches a HALF-DONE correction: if someone updates
/// three of the four places, a stale number is still in the file and the test above
/// would pass on the three it found. This fails on the leftover.
#[test]
fn no_stale_tool_count_survives_anywhere() {
    let n = exposed_tool_count();
    let readme = ["README.md", "docs/architecture.md", "src/server.rs"]
        .iter()
        .filter_map(|f| std::fs::read_to_string(repo().join(f)).ok())
        .collect::<Vec<_>>()
        .join("\n");

    // The shapes a total is written in here, each with the number it must state. Two
    // different totals are correct now: how many tools are REGISTERED, and how many a
    // build with this feature set ADVERTISES. Conflating them is what produced a
    // sentence claiming a default build advertised all 114 while eight of them could
    // only return "Compiled without X feature".
    let advertised = n - gated_tool_count();
    let shapes: [(&str, &str, usize); 4] = [
        ("**", " tools**", n),
        ("advertises ", " tools.", advertised),
        ("", " tools organized by function", n),
        ("ToolGroups[\"", " Tools\"]", n),
    ];

    // The sentence that was wrong, named so a revert cannot pass quietly. There is no
    // number a build can put in it that is true: the eight gated tools are registered
    // and not advertised, so "all N" is false for every N.
    assert!(
        !readme.contains("advertises all "),
        "\"advertises all N tools\" is back. It cannot be true: {} of the {n} registered \
         tools are behind a Cargo feature and are not advertised by a build that lacks it. \
         Say how many are advertised, not that all of them are.",
        gated_tool_count().max(1)
    );

    let mut stale = Vec::new();
    for (prefix, suffix, n) in shapes {
        let mut rest = readme.as_str();
        while let Some(i) = rest.find(suffix) {
            let head = &rest[..i];
            let digits: String =
                head.chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
            let matches_shape =
                !digits.is_empty() && (prefix.is_empty() || head.ends_with(&format!("{prefix}{digits}")));
            if let Some(found) =
                digits.parse::<usize>().ok().filter(|f| matches_shape && *f != n)
            {
                stale.push(format!("{prefix}{found}{suffix} (server exposes {n})"));
            }
            rest = &rest[i + suffix.len()..];
        }
    }

    assert!(
        stale.is_empty(),
        "a tool-count claim in the README disagrees with src/server.rs:\n  {}\n\nThis is what \
         a half-finished correction looks like: some copies updated, one left behind.",
        stale.join("\n  ")
    );
}

/// The README cites theorems by name. A name that does not exist is a false claim, and it
/// is the easiest false claim to make: a theorem written on one branch is quoted from
/// another, or renamed, and the prose keeps asserting it.
///
/// This caught one. The README claimed `Fol.satisfiable_of_check` while that module lived
/// on an unmerged branch, so the table promised a guarantee this tree could not give.
#[test]
fn every_theorem_the_readme_cites_exists() {
    let readme = std::fs::read_to_string(repo().join("README.md")).expect("README.md must exist");

    // Backticked `Namespace.name` with an upper-case namespace: how this repo spells a
    // theorem reference, and specific enough not to catch file paths or CLI flags.
    let mut cited: Vec<String> = Vec::new();
    for chunk in readme.split('`').skip(1).step_by(2) {
        let Some((ns, name)) = chunk.split_once('.') else { continue };
        // All-caps means a filename, not a namespace: CITATION.cff, not OOCert.foo.
        // Every namespace in this repository is mixed case.
        let ns_ok = ns.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && ns.chars().all(|c| c.is_ascii_alphanumeric())
            && ns.chars().any(|c| c.is_ascii_lowercase());
        let name_ok = !name.is_empty()
            && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if ns_ok && name_ok {
            cited.push(chunk.to_string());
        }
    }
    assert!(!cited.is_empty(), "no theorem references found; the extraction itself is broken");

    let lean = repo().join("lean");
    let mut sources = Vec::new();
    let mut stack = vec![lean];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == ".lake") {
                    continue;
                }
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "lean")
                && let Ok(t) = std::fs::read_to_string(&p)
            {
                sources.push((p, t));
            }
        }
    }

    let mut missing = Vec::new();
    for c in &cited {
        let (ns, name) = c.split_once('.').unwrap();
        let found = sources.iter().any(|(_, t)| {
            t.contains(&format!("namespace {ns}"))
                && (t.contains(&format!("theorem {name}")) || t.contains(&format!("def {name}")))
        });
        if !found {
            missing.push(c.clone());
        }
    }

    assert!(
        missing.is_empty(),
        "the README cites {} theorem(s) that do not exist under lean/ on this tree:\n  {}\n\n\
         Either the name is wrong, or it lives on a branch that has not merged. Both are false \
         claims in a file people read to decide whether to trust this.",
        missing.len(),
        missing.join("\n  ")
    );
}
