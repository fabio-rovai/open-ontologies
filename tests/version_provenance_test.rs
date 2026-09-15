//! The crate version must be the version that was released.
//!
//! It was not. v1.4.0 was tagged from a tree whose `Cargo.toml` still said
//! `1.3.0`, so the binary reported 1.3.0, the MCP server advertised 1.3.0, and
//! every pack written by that build stamped `tool_version: "1.3.0"`. Four call
//! sites read `CARGO_PKG_VERSION` and all four were wrong at once.
//!
//! That is worse here than in most projects. A pack is an artefact somebody
//! keeps, and this repository's whole argument is that you should be able to
//! check what produced a thing rather than take its word. A release that
//! misreports its own identity attacks exactly that.
//!
//! The lesson is the one that keeps recurring: a number typed by hand next to
//! the thing it describes will drift. Derive it, or test it. A tag is derived
//! from a human decision, so this is the test.

use std::path::PathBuf;
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The version compiled into the binary, which is what every runtime call site
/// reports and what a pack records.
fn compiled_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The newest `vX.Y.Z` tag reachable from HEAD, or `None` when git is absent or
/// there are no tags, which is the case in a source tarball.
fn newest_release_tag() -> Option<String> {
    let out = Command::new("git")
        .args(["tag", "--list", "v[0-9]*", "--sort=-v:refname"])
        .current_dir(repo())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout).lines().next().map(|s| s.trim().to_string())
}

#[test]
fn the_compiled_version_is_not_behind_the_newest_tag() {
    let Some(tag) = newest_release_tag() else {
        eprintln!("SKIPPED: no git tags reachable, so there is no release to compare against");
        return;
    };
    let tagged = tag.trim_start_matches('v');

    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        let mut it = v.split('.').map(|p| p.split(['-', '+']).next().unwrap_or(p));
        Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?))
    };

    let (Some(c), Some(t)) = (parse(compiled_version()), parse(tagged)) else {
        panic!("cannot compare versions: compiled {:?}, tag {:?}", compiled_version(), tag);
    };

    assert!(
        c >= t,
        "Cargo.toml says {} and the newest release tag is {}. A build from this tree would \
         report a version older than a release that already exists, which is what happened to \
         v1.4.0: it was tagged from a tree still saying 1.3.0, so the binary, the MCP server \
         and every pack it wrote all misreported their own identity.\n\n\
         Bump the version BEFORE tagging, not after.",
        compiled_version(),
        tag
    );
}

/// The other half. The first test allows the working tree to be ahead of the
/// newest tag, which is normal between releases. This one catches the case that
/// actually shipped: standing exactly ON a release tag while declaring a
/// different version.
#[test]
fn a_tagged_commit_declares_the_version_of_its_tag() {
    let out = Command::new("git")
        .args(["tag", "--points-at", "HEAD", "--list", "v[0-9]*"])
        .current_dir(repo())
        .output();
    let Ok(out) = out else {
        eprintln!("SKIPPED: git unavailable");
        return;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let Some(tag) = text.lines().next().map(str::trim).filter(|t| !t.is_empty()) else {
        // Not on a release tag. Nothing to check, and that is the normal case.
        return;
    };

    assert_eq!(
        compiled_version(),
        tag.trim_start_matches('v'),
        "HEAD carries tag {tag} and Cargo.toml says {}. A binary built from this exact commit \
         would report a version that is not the release it came from.",
        compiled_version()
    );
}
