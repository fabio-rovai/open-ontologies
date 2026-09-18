//! **Issue #185.** One number, one decision, and every decision in the index.
//!
//! `docs/decisions/` numbers records in their filename and nothing checked the
//! number was unique. Two records numbered 0009 sat on `main` for days, and in
//! a single day three more collisions happened between branches running in
//! parallel: 0013 twice and 0012 once.
//!
//! **Review does not catch this.** The filenames differ, so git sees two
//! unrelated new files and merges both cleanly. There is no conflict, no
//! failing test, and nothing in either diff that looks wrong. The index gets a
//! row from each, so even the index reads plausibly. One of those collisions
//! was committed and pushed by the person who filed the issue describing it,
//! which is a fair measure of how little a filename collision announces
//! itself.
//!
//! A reused number is worse than a missing one: an old citation silently comes
//! to mean something else. That is why 0004 is a hole rather than a number
//! waiting to be filled, and why the later of the two 0009 records moved to
//! 0016 instead of the earlier one moving.

use std::collections::BTreeMap;
use std::path::PathBuf;

fn decisions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs").join("decisions")
}

/// Every record file, as (number, filename). `README.md` is the index and is
/// not a record.
fn records() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(decisions_dir()).expect("read docs/decisions") {
        let name = entry.expect("dir entry").file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") || name == "README.md" {
            continue;
        }
        let digits: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
        assert_eq!(
            digits.len(),
            4,
            "a decision record must start with four digits, and {name} does not. Without that \
             the number cannot be read, so neither this check nor a reader can tell which \
             decision it is"
        );
        out.push((digits, name));
    }
    out.sort();
    assert!(out.len() > 5, "only {} records found; the scan is not running", out.len());
    out
}

#[test]
fn no_two_decisions_take_the_same_number() {
    let mut by_number: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (n, name) in records() {
        by_number.entry(n).or_default().push(name);
    }
    let clashes: Vec<String> = by_number
        .iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(n, files)| format!("{n}: {}", files.join(" and ")))
        .collect();
    assert!(
        clashes.is_empty(),
        "{} number(s) name more than one decision:\n  {}\n\nGit will not have flagged this: the \
         filenames differ, so both merged cleanly. Renumber the LATER record to the next free \
         number and move every reference with it. Do not reuse 0004, which is a deliberate hole.",
        clashes.len(),
        clashes.join("\n  ")
    );
}

#[test]
fn every_decision_has_a_row_in_the_index_and_every_row_resolves() {
    let index = std::fs::read_to_string(decisions_dir().join("README.md")).expect("read index");

    // Rows are markdown links to a file in this directory.
    let mut linked: Vec<String> = Vec::new();
    let mut rest = index.as_str();
    while let Some(i) = rest.find("](") {
        let after = &rest[i + 2..];
        if let Some(j) = after.find(')') {
            let target = &after[..j];
            if target.ends_with(".md") && !target.contains('/') {
                linked.push(target.to_string());
            }
        }
        rest = &rest[i + 2..];
    }
    assert!(!linked.is_empty(), "the index links to no record, so this test checks nothing");

    for target in &linked {
        assert!(
            decisions_dir().join(target).is_file(),
            "the index links to {target}, which does not exist. A renumbered record leaves this \
             behind, and the row still renders, so nothing else would notice"
        );
    }

    let missing: Vec<String> = records()
        .into_iter()
        .map(|(_, name)| name)
        .filter(|name| !linked.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "{} record(s) exist with no row in the index:\n  {}\n\nAn index that looks complete and \
         is not is worse than one that admits the gap.",
        missing.len(),
        missing.join("\n  ")
    );
}
