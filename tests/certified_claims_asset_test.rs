//! The figure says what a fresh run says, or it is not a figure.
//!
//! `docs/assets/certified-claims.svg` shows counts, predicates, module names
//! and a theorem. Every one of those is read out of a real run by the
//! generator and none is typed into it, but that is a property of the script
//! and not of the committed file: the script could be right and the SVG stale.
//!
//! So these re-run the tools against the same committed fixtures and compare
//! with what the drawing says. A figure that drifts from the code under it is
//! worse than no figure, because it carries the authority of having been
//! generated.

mod common;

use std::path::PathBuf;
use std::sync::Arc;

use open_ontologies::crosswalk::{certify, parse_sssom};
use open_ontologies::graph::GraphStore;
use open_ontologies::modules::{distributed, Module};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn claims() -> PathBuf {
    repo().join("docs").join("assets").join("claims")
}

fn svg(lang: &str) -> String {
    let name = if lang == "zh" { "certified-claims.zh-CN.svg" } else { "certified-claims.svg" };
    std::fs::read_to_string(repo().join("docs").join("assets").join(name))
        .unwrap_or_else(|e| panic!("read {name}: {e}"))
}

fn store(rel: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_file(&claims().join(rel).display().to_string()).expect("fixture loads");
    g
}

#[test]
fn the_crosswalk_panel_matches_a_fresh_run() {
    let maps = parse_sssom(&std::fs::read_to_string(claims().join("mapping.tsv")).unwrap())
        .expect("the fixture is valid SSSOM");
    let (rows, gaps) = certify(&store("ies-side.ttl"), &store("hqdm-side.ttl"), &maps)
        .expect("certifies");

    let s = svg("en");
    // Every subject, with the predicate the run supports, must appear as the
    // drawing shows it.
    for r in &rows {
        let subject = r.mapping.subject.rsplit('/').next().unwrap();
        assert!(s.contains(subject), "the figure does not name {subject}");
        assert!(
            s.contains(&format!("exactMatch → {}", r.supported)),
            "a fresh run supports {} for {subject} and the figure does not show it",
            r.supported
        );
    }
    let downgraded = rows.iter().filter(|r| r.downgraded).count();
    assert_eq!(downgraded, 2, "the fixture is built to produce two downgrades");
    assert!(
        s.contains(&format!("{} term the crosswalk does not carry", gaps.len()))
            || s.contains(&format!("{} terms the crosswalk does not carry", gaps.len())),
        "the gap count in the figure disagrees with a fresh run ({})",
        gaps.len()
    );
}

#[test]
fn the_modules_panel_matches_a_fresh_run() {
    let mods: Vec<Module> = [("birds", "mod-birds.ttl"), ("penguins", "mod-penguins.ttl"),
                             ("field_guide", "mod-guide.ttl")]
        .iter()
        .map(|(n, f)| Module {
            name: n.to_string(),
            ttl: std::fs::read_to_string(claims().join(f)).expect("fixture"),
        })
        .collect();
    let js = distributed(&mods, 2).expect("runs");
    let s = svg("en");

    for m in js["modules"].as_array().unwrap() {
        let name = m["name"].as_str().unwrap();
        let n = m["entails"].as_u64().unwrap();
        assert!(s.contains(name), "the figure does not name the module {name}");
        assert!(
            s.contains(&format!("entails {n}")),
            "a fresh run has {name} entailing {n}, which the figure does not show"
        );
    }
    let promoted = js["promoted"].as_u64().unwrap();
    assert!(
        s.contains(&format!("{promoted} fact entailed in 2 of 3 modules")),
        "the promoted count drifted from a fresh run ({promoted})"
    );
    let contested = js["contested"].as_u64().unwrap();
    assert!(
        s.contains(&format!("{contested} held by fewer")),
        "the contested count drifted from a fresh run ({contested})"
    );
}

#[test]
fn the_numeric_panel_names_the_theorem_the_checker_printed() {
    if common::skip_unless(
        open_ontologies::matcert::find_checker().is_some(),
        "the numeric checker (oo-matcert)",
        "run `lake build oo-matcert` in lean/",
    ) {
        return;
    }
    // Run the committed certificate through the real binary.
    let bin = open_ontologies::matcert::find_checker().unwrap();
    let out = std::process::Command::new(bin)
        .arg(claims().join("product.matcert"))
        .output()
        .expect("runs");
    assert_eq!(out.status.code(), Some(0), "the committed certificate must check");
    let js: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).expect("JSON");
    let theorem = js["theorem"].as_str().expect("an accepted run names its theorem");

    let s = svg("en");
    assert!(
        s.contains(theorem),
        "the figure shows a theorem the checker did not print. The name in a drawing must \
         come from a run: {theorem}"
    );
    // And only there. The other two rows are opinions and the figure says so.
    assert_eq!(
        s.matches(theorem).count(),
        1,
        "the theorem name appears more than once, so the figure is lending it to a row that \
         did not earn it"
    );
}

#[test]
fn both_languages_draw_the_same_numbers() {
    let en = svg("en");
    let zh = svg("zh");
    // The LABEL is translated and the NUMBER is not, which is the whole point:
    // a figure that shared its prose would be untranslated, and one that
    // differed in its counts would be two different claims.
    for n in [3u32, 4] {
        assert!(en.contains(&format!("entails {n}")), "the English figure lost 'entails {n}'");
        assert!(zh.contains(&format!("蕴涵 {n}")), "the Chinese figure lost its count {n}");
    }
    assert!(zh.contains("MatCert.mul_of_check"), "the theorem is a name, not a translation");
    // A figure that lost its Chinese text would still pass the numbers, so
    // check it is actually translated.
    assert!(zh.contains("对照表"), "the Chinese figure is not in Chinese");
}

#[test]
fn the_figure_is_well_formed_and_carries_no_placeholders() {
    for lang in ["en", "zh"] {
        let s = svg(lang);
        assert!(s.starts_with("<svg"), "{lang}: not an SVG");
        assert!(s.trim_end().ends_with("</svg>"), "{lang}: truncated");
        assert!(
            !s.contains("{n}") && !s.contains("{k}") && !s.contains("{t}"),
            "{lang}: an unsubstituted placeholder reached the file"
        );
    }
}
