//! **The tool reference lists every tool, and the front page lists none of them.**
//!
//! Issue #196: the README sold surface area. A visitor met four abstract nouns
//! and a tool count before meeting a verb, and could not tell what the thing was
//! for. The breadth is real and it keeps its code; it loses its place above the
//! fold and moves to `docs/tool-reference.md`, linked once, low on the page.
//!
//! A reference page that drifts from the code is worse than no reference page,
//! because it is read as authoritative. These tests are the gate: the page is
//! generated from the `#[tool(...)]` attributes and must stay in step with them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(p: &str) -> String {
    std::fs::read_to_string(repo().join(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// Tool names as the server registers them.
fn registered() -> BTreeSet<String> {
    let src = read("src/server.rs");
    let mut out = BTreeSet::new();
    let needle = "#[tool(name = \"";
    let mut i = 0;
    while let Some(j) = src[i..].find(needle) {
        let start = i + j + needle.len();
        let end = start + src[start..].find('"').expect("unterminated tool name");
        out.insert(src[start..end].to_string());
        i = end;
    }
    out
}

/// Tool names the reference page lists, from the first column of its table.
fn documented() -> BTreeSet<String> {
    read("docs/tool-reference.md")
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix("| `")?;
            let end = rest.find('`')?;
            Some(rest[..end].to_string())
        })
        .collect()
}

#[test]
fn every_registered_tool_is_in_the_reference() {
    let (r, d) = (registered(), documented());
    let missing: Vec<_> = r.difference(&d).collect();
    assert!(
        missing.is_empty(),
        "these tools are registered and not listed in docs/tool-reference.md: {missing:?}. \
         Regenerate the page rather than deleting this assertion; an unlisted tool is one \
         nobody can find."
    );
}

#[test]
fn the_reference_invents_no_tools() {
    let (r, d) = (registered(), documented());
    let extra: Vec<_> = d.difference(&r).collect();
    assert!(
        extra.is_empty(),
        "docs/tool-reference.md lists tools the server does not register: {extra:?}. A \
         reference page that names something that does not exist is worse than a missing \
         page, because it is read as authoritative."
    );
}

/// The count on the page is a claim like any other.
#[test]
fn the_reference_states_the_right_count() {
    let n = registered().len();
    let page = read("docs/tool-reference.md");
    assert!(
        page.contains(&format!("{n} tools.")),
        "the page does not state {n} tools, which is how many are registered"
    );
}

/// The repositioning itself. The front page shows the loop and not the
/// catalogue, and this is what stops the catalogue creeping back.
#[test]
fn the_front_page_sells_the_loop_and_not_the_surface_area() {
    let readme = read("README.md");
    let fold = &readme[..readme.len().min(9000)];

    for banned in ["120 tools", "112 tools", "Terraforming MCP"] {
        assert!(
            !readme.contains(banned),
            "the README still says {banned:?}. Surface area reads as insecurity, and a \
             number where a purpose belongs tells a visitor nothing about what this is for."
        );
    }
    for beat in [
        "consequences that were not there before", // beat 1, the semantic plan
        "re-verifies months later",                // beat 2, the certificate
        "exits 1 and names the rule",              // beat 3, the refusal
    ] {
        assert!(
            fold.contains(beat),
            "the fold is missing the beat {beat:?}. The three beats are attention, \
             evidence, credibility, in that order, and dropping one leaves an argument \
             that does not close."
        );
    }
    assert!(
        fold.contains("This is not an ontology editor"),
        "the anti-positioning line is gone. It answers the first question a visitor has."
    );
    assert!(
        fold.contains("A text diff is `git diff`"),
        "the second anti-positioning line is gone. It pre-empts the first objection, which \
         is why this is not just a file in git."
    );
    assert!(
        readme.contains("docs/tool-reference.md"),
        "the breadth has to be reachable: the reference page is linked once, low on the page"
    );
}

/// `trustworthy`, and the other words the issue retired, must stay gone.
///
/// Checked by ABSENCE across every file that carries outward prose, because a
/// claim lives in more copies than anyone expects and reading the new text
/// finds only the copy you are looking at.
///
/// `docs/tool-reference.md` is exempt from the COUNT strings and only those.
/// The ban exists because a number where a purpose belongs tells a visitor
/// nothing about what this is for; the reference page is the catalogue, its job
/// is to be exhaustive, and a catalogue that will not say how long it is is
/// being coy. It is not exempt from `trustworthy`, which is a faith claim
/// wherever it appears. This exemption is written down rather than achieved by
/// narrowing the walk, so a reader can disagree with it.
#[test]
fn the_retired_words_do_not_come_back() {
    let mut hits = vec![];
    let mut walk = |p: &Path| {
        if p.extension().is_some_and(|e| e == "md")
            && let Ok(t) = std::fs::read_to_string(p)
        {
            let lower = t.to_lowercase();
            let is_catalogue = p.ends_with("tool-reference.md");
            for w in ["120 tools", "112 tools", "terraforming mcp", "trustworthy"] {
                let counts_only = w.ends_with(" tools");
                if is_catalogue && counts_only {
                    continue;
                }
                if lower.contains(w) {
                    hits.push(format!("{}: {w}", p.display()));
                }
            }
        }
    };
    walk(&repo().join("README.md"));
    walk(&repo().join("README.zh-CN.md"));
    let docs = repo().join("docs");
    let mut stack = vec![docs];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                walk(&p);
            }
        }
    }
    assert!(
        hits.is_empty(),
        "retired strings are back: {hits:?}. Verify by absence, not by reading the new text."
    );
}
