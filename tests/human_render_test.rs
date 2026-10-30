//! `--human` renders every command's result by the command's NAME (#350, #345).
//!
//! The renderer used to be picked by sniffing which keys the JSON carried, and
//! four commands printed a WRONG answer, not merely an unformatted one: `diff`
//! reported any two files as identical, an undetermined `shacl` run read as a
//! pass, `validate` printed class counts it never computed, and `monitor`
//! printed itself as server status. Others, `defects` among them, printed raw
//! JSON. Everything here runs the real binary on tiny synthetic fixtures.
//!
//! The guard that every clap subcommand has a renderer registered lives in
//! `src/main.rs`, because the CLI struct is private to the binary crate.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How the text for a result with no renderer begins. Kept in step with
/// `output::NO_RENDERING` by hand, so this file reads only the binary.
const NO_RENDERING: &str = "No human rendering exists for";

const A: &str = r#"@prefix ex: <https://example.org/x/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
ex:A a owl:Class ; rdfs:label "A" .
ex:B a owl:Class ; rdfs:label "B" .
"#;

/// A with exactly one extra triple, which also declares one extra class.
const B: &str = r#"@prefix ex: <https://example.org/x/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
ex:A a owl:Class ; rdfs:label "A" .
ex:B a owl:Class ; rdfs:label "B" .
ex:C a owl:Class .
"#;

/// The reproduction from #345: a class under something it is disjoint from.
const DEFECTIVE: &str = r#"@prefix ex: <https://example.org/x/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
ex:A a owl:Class . ex:B a owl:Class .
ex:A rdfs:subClassOf ex:B .
ex:A owl:disjointWith ex:B .
"#;

const SHAPES: &str = r#"@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <https://example.org/x/> .
ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [ sh:path ex:name ; sh:minCount 1 ] .
"#;

/// Two people, one with no name, so the shapes above find one violation.
const PEOPLE: &str = r#"@prefix ex: <https://example.org/x/> .
ex:alice a ex:Person .
ex:bob a ex:Person ; ex:name "Bob" .
"#;

const CSV: &str = "id,name\n1,alpha\n2,beta\n";
const SSSOM: &str =
    "subject_id\tpredicate_id\tobject_id\nhttps://example.org/x/A\tskos:exactMatch\thttps://example.org/x/A\n";
const GOALS: &str = "@prefix ex: <https://example.org/x/> .\nex:A a <http://www.w3.org/2002/07/owl#Class> .\n";
const TRACE: &str = "rdfs9\t<https://example.org/x/a>\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t<https://example.org/x/B>\n";

struct Fixtures {
    dir: tempfile::TempDir,
}

impl Fixtures {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        for (name, body) in [
            ("a.ttl", A),
            ("b.ttl", B),
            ("defective.ttl", DEFECTIVE),
            ("shapes.ttl", SHAPES),
            ("people.ttl", PEOPLE),
            ("data.csv", CSV),
            ("mappings.sssom.tsv", SSSOM),
            ("goals.ttl", GOALS),
            ("trace.tsv", TRACE),
        ] {
            std::fs::write(dir.path().join(name), body).unwrap();
        }
        Fixtures { dir }
    }

    fn path(&self, name: &str) -> String {
        self.dir.path().join(name).to_string_lossy().into_owned()
    }

    fn data_dir(&self) -> PathBuf {
        self.dir.path().join("data")
    }
}

struct Run {
    code: i32,
    stdout: String,
}

/// Run the binary locally, with its own data directory and none of the
/// environment that could point it at a daemon or another store.
fn oo(data_dir: &Path, args: &[&str]) -> Run {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_open-ontologies"));
    cmd.arg("--no-connect")
        .arg("--data-dir")
        .arg(data_dir)
        .args(args)
        .env_remove("OPEN_ONTOLOGIES_TOKEN")
        .env_remove("OPEN_ONTOLOGIES_STORAGE_MODE")
        .env_remove("GOVERNANCE_WEBHOOK")
        .env("RUST_LOG", "warn")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            panic!("{args:?} timed out");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let out = child.wait_with_output().unwrap();
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
    }
}

fn human(fx: &Fixtures, args: &[&str]) -> Run {
    let mut all = vec!["--human"];
    all.extend_from_slice(args);
    oo(&fx.data_dir(), &all)
}

fn json(fx: &Fixtures, args: &[&str]) -> Value {
    let run = oo(&fx.data_dir(), args);
    serde_json::from_str(run.stdout.trim())
        .unwrap_or_else(|e| panic!("{args:?} printed no JSON ({e}): {}", run.stdout))
}

/// Every subcommand that runs offline on the fixtures, in an order in which
/// each one's prerequisites have already run (`apply` after `plan`).
///
/// Left out, each for a reason that is not "it renders badly": `init` and the
/// three server modes print no JSON at all; `batch` has its own test below;
/// `marketplace`, `pull` and `push` need the network; `sql-ingest` without the
/// `duckdb` feature fails before it prints; `daemon start` would leave a
/// process behind. Their renderers are covered by the unit tests in
/// `src/output.rs` and their registration by the guard in `src/main.rs`.
fn offline_commands(fx: &Fixtures) -> Vec<Vec<String>> {
    let p = |n: &str| fx.path(n);
    let out = |n: &str| fx.dir.path().join("out").join(n).to_string_lossy().into_owned();
    let rows: Vec<Vec<String>> = vec![
        vec!["validate".into(), p("a.ttl")],
        vec!["load".into(), p("a.ttl")],
        vec!["save".into(), out("saved.ttl")],
        vec!["clear".into()],
        vec!["stats".into()],
        vec!["query".into(), "SELECT ?s WHERE { ?s ?p ?o }".into()],
        vec!["query".into(), "ASK { ?s ?p ?o }".into()],
        vec![
            "query".into(),
            "CONSTRUCT { <https://example.org/x/s> <https://example.org/x/p> <https://example.org/x/o> } WHERE {}".into(),
        ],
        vec!["diff".into(), p("a.ttl"), p("b.ttl")],
        vec!["lint".into(), p("b.ttl")],
        vec!["defects".into(), p("defective.ttl")],
        vec!["defects".into(), p("a.ttl")],
        vec!["convert".into(), p("a.ttl"), "--to".into(), "ntriples".into(), "--output".into(), out("a.nt")],
        vec!["convert".into(), p("a.ttl"), "--to".into(), "ofn".into(), "--output".into(), out("a.ofn")],
        vec!["status".into()],
        vec!["import-owl".into()],
        vec!["version".into(), "v1".into()],
        vec!["history".into()],
        vec!["rollback".into(), "v1".into()],
        vec!["map".into(), p("data.csv")],
        vec!["ingest".into(), p("data.csv")],
        vec!["shacl".into(), p("shapes.ttl")],
        vec!["vocab-check".into(), p("b.ttl")],
        vec!["reason".into()],
        vec!["reason".into(), "--profile".into(), "owl-dl".into()],
        vec!["fol".into(), "--out".into(), out("fol")],
        vec!["rules-import".into(), "--from".into(), "swrl".into()],
        vec!["fol-model".into(), "--out".into(), out("fol-model"), "--timeout-secs".into(), "5".into()],
        vec!["fol-prove".into(), "--out".into(), out("fol-prove"), "--timeout-secs".into(), "5".into()],
        vec!["preserve".into(), "--projection".into(), p("a.ttl"), "--goals".into(), p("goals.ttl"), "--out".into(), out("preserve")],
        vec!["closure-diff".into(), "--projection".into(), p("a.ttl"), "--out".into(), out("closure-diff")],
        vec!["trace-label".into(), "--trace".into(), p("trace.tsv"), "--out".into(), out("trace-label")],
        vec!["extend".into(), p("data.csv"), "--shapes".into(), p("shapes.ttl"), "--profile".into(), "rdfs".into()],
        vec!["plan".into(), p("b.ttl")],
        vec!["apply".into()],
        vec!["lock".into(), "https://example.org/x/A".into(), "--reason".into(), "frozen".into()],
        vec!["drift".into(), p("a.ttl"), p("b.ttl")],
        vec!["enforce".into(), "generic".into()],
        vec!["unpack".into(), p("missing.oopack")],
        vec!["monitor".into()],
        vec!["monitor-clear".into()],
        vec!["lineage".into()],
        vec!["align".into(), p("a.ttl"), p("b.ttl")],
        vec!["crosswalk-probe".into(), p("a.ttl"), p("b.ttl"), p("mappings.sssom.tsv")],
        vec!["align-feedback".into(), "--source".into(), "https://example.org/x/A".into(), "--target".into(), "https://example.org/x/B".into(), "--accept".into()],
        vec!["lint-feedback".into(), "--rule-id".into(), "missing_label".into(), "--entity".into(), "https://example.org/x/C".into(), "--dismiss".into()],
        vec!["enforce-feedback".into(), "--rule-id".into(), "r1".into(), "--entity".into(), "https://example.org/x/C".into(), "--accept".into()],
        vec!["crosswalk".into(), "44054006".into(), "--system".into(), "SNOMED".into()],
        vec!["enrich".into(), "https://example.org/x/A".into(), "44054006".into(), "--system".into(), "SNOMED".into()],
        vec!["validate-clinical".into()],
        vec!["import-schema".into(), ":memory:".into()],
        vec!["daemon".into(), "status".into()],
        vec!["daemon".into(), "stop".into()],
    ];
    std::fs::create_dir_all(fx.dir.path().join("out")).unwrap();
    rows
}

#[test]
fn no_command_prints_json_under_human() {
    let fx = Fixtures::new();
    let mut failures = Vec::new();
    for row in offline_commands(&fx) {
        let args: Vec<&str> = row.iter().map(String::as_str).collect();
        let run = human(&fx, &args);
        let text = run.stdout.trim();
        let what = format!("{} (exit {})", args.join(" "), run.code);
        if text.is_empty() {
            failures.push(format!("{what}: printed nothing on stdout"));
        } else if serde_json::from_str::<Value>(text).is_ok() {
            failures.push(format!("{what}: printed JSON:\n{text}"));
        } else if text.contains(NO_RENDERING) {
            failures.push(format!("{what}: has no renderer:\n{text}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} command(s) printed JSON or nothing under --human:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[test]
fn diff_counts_the_triple_that_was_added() {
    let fx = Fixtures::new();
    let (a, b) = (fx.path("a.ttl"), fx.path("b.ttl"));
    let v = json(&fx, &["diff", &a, &b]);
    assert_eq!(v["added"], 1, "the fixture differs by one triple: {v}");
    let run = human(&fx, &["diff", &a, &b]);
    assert!(
        run.stdout.contains("+1 triple"),
        "diff must count the added triple, not print +0: {}",
        run.stdout
    );
    assert!(run.stdout.contains("https://example.org/x/C"), "{}", run.stdout);
}

#[test]
fn an_undetermined_shacl_run_says_undetermined_and_keeps_exit_2() {
    let fx = Fixtures::new();
    let shapes = fx.path("shapes.ttl");
    let run = human(&fx, &["shacl", &shapes]);
    assert_eq!(run.code, 2, "undetermined exits 2: {}", run.stdout);
    let first = run.stdout.lines().next().unwrap_or_default();
    assert_eq!(first, "conforms: undetermined", "verdict first: {}", run.stdout);
    assert!(run.stdout.contains("no focus nodes matched"), "{}", run.stdout);
    assert!(
        !run.stdout.contains("No violations found"),
        "an undetermined run must not read as a pass: {}",
        run.stdout
    );
}

#[test]
fn a_failing_shacl_run_leads_with_its_verdict_and_lists_the_violation() {
    // Persistent storage so `load` and `shacl` see one store across processes.
    let fx = Fixtures::new();
    std::fs::create_dir_all(fx.data_dir()).unwrap();
    std::fs::write(
        fx.data_dir().join("config.toml"),
        "[storage]\nmode = 'persistent'\n",
    )
    .unwrap();
    let (people, shapes) = (fx.path("people.ttl"), fx.path("shapes.ttl"));
    assert_eq!(oo(&fx.data_dir(), &["load", &people]).code, 0);
    let run = human(&fx, &["shacl", &shapes]);
    assert_eq!(run.code, 1, "a violation exits 1: {}", run.stdout);
    let first = run.stdout.lines().next().unwrap_or_default();
    assert_eq!(first, "conforms: false", "verdict first: {}", run.stdout);
    assert!(run.stdout.contains("Violations: 1"), "{}", run.stdout);
    assert!(run.stdout.contains("https://example.org/x/alice"), "{}", run.stdout);
}

#[test]
fn validate_prints_only_what_it_counted() {
    let fx = Fixtures::new();
    let a = fx.path("a.ttl");
    let run = human(&fx, &["validate", &a]);
    for never_computed in ["Classes", "Properties", "Individuals"] {
        assert!(
            !run.stdout.contains(never_computed),
            "validate computes no {never_computed} count: {}",
            run.stdout
        );
    }
    assert!(run.stdout.contains("4 distinct triple(s)"), "{}", run.stdout);
}

#[test]
fn monitor_is_not_rendered_as_server_status() {
    let fx = Fixtures::new();
    let run = human(&fx, &["monitor"]);
    assert!(!run.stdout.contains("v?"), "{}", run.stdout);
    assert!(!run.stdout.contains("triples in store"), "{}", run.stdout);
    assert!(run.stdout.starts_with("Monitor: ok."), "{}", run.stdout);
}

#[test]
fn defects_lists_each_defect_with_its_kind() {
    let fx = Fixtures::new();
    let defective = fx.path("defective.ttl");
    let run = human(&fx, &["defects", &defective]);
    let line = run
        .stdout
        .lines()
        .find(|l| l.contains("disjoint_with_ancestor"))
        .unwrap_or_else(|| panic!("no line for the defect: {}", run.stdout));
    assert!(line.trim_start().starts_with("error"), "{line}");
    assert!(line.contains("<https://example.org/x/A>"), "{line}");
    assert!(line.contains("ancestor <https://example.org/x/B>"), "{line}");
    assert!(run.stdout.starts_with("1 defect(s)"), "{}", run.stdout);
}

#[test]
fn a_clean_defects_run_says_how_many_kinds_it_checked() {
    let fx = Fixtures::new();
    let a = fx.path("a.ttl");
    let kinds = json(&fx, &["defects", &a])["kinds_checked"]
        .as_array()
        .expect("kinds_checked")
        .len();
    let run = human(&fx, &["defects", &a]);
    assert_eq!(
        run.stdout.trim(),
        format!("No defects ({kinds} kinds checked)."),
        "{}",
        run.stdout
    );
}

#[test]
fn drift_reports_terms_velocity_and_renames() {
    let fx = Fixtures::new();
    let (a, b) = (fx.path("a.ttl"), fx.path("b.ttl"));
    let v = json(&fx, &["drift", &a, &b]);
    let run = human(&fx, &["drift", &a, &b]);
    assert!(
        run.stdout.contains(&format!("Drift velocity: {}", v["drift_velocity"])),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("0 likely rename(s)"), "{}", run.stdout);
    assert!(!run.stdout.contains("triples"), "drift lists terms: {}", run.stdout);
}

#[test]
fn batch_under_human_renders_each_result_by_its_own_command() {
    let fx = Fixtures::new();
    let script = fx.dir.path().join("batch.txt");
    std::fs::write(
        &script,
        format!(
            "load {}\nshacl {}\ndiff {} {}\n",
            fx.path("people.ttl"),
            fx.path("shapes.ttl"),
            fx.path("a.ttl"),
            fx.path("b.ttl")
        ),
    )
    .unwrap();
    let run = human(&fx, &["batch", &script.to_string_lossy()]);
    for line in run.stdout.lines() {
        assert!(
            serde_json::from_str::<Value>(line).is_err(),
            "a JSON line under --human: {line}\n{}",
            run.stdout
        );
    }
    assert!(run.stdout.contains("Loaded 3 triple(s)"), "{}", run.stdout);
    assert!(run.stdout.contains("conforms: false"), "{}", run.stdout);
    assert!(run.stdout.contains("+1 triple"), "{}", run.stdout);
}

#[test]
fn json_stays_the_default_and_wins_over_human() {
    let fx = Fixtures::new();
    let (a, b) = (fx.path("a.ttl"), fx.path("b.ttl"));
    for flags in [&[][..], &["--human", "--json"][..], &["--human", "--pretty"][..]] {
        let mut args = flags.to_vec();
        args.extend(["diff", a.as_str(), b.as_str()]);
        let v = json(&fx, &args);
        assert_eq!(v["added"], 1, "{flags:?}: {v}");
    }
}
