// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! The same store gives the same `derivations.tsv`, byte for byte (#427).
//!
//! `reason --certificate DIR` records one step per inferred triple: the rule,
//! the conclusion and the premises it read. Where a conclusion can be reached
//! more than one way (`A subClassOf C` through `B1` or through `B2`), the step
//! recorded is the first the fixpoint reached, and the fixpoint used to walk
//! its closure in hash order. The conclusions were the same set on every run;
//! which intermediate justified a step, and the order of the lines, were not.
//! A certificate could then not be pinned by digest or compared between two
//! machines, although `asserted.tsv` beside it could.
//!
//! Rust draws its hash keys once per thread and steps them for each new map,
//! so the seeds inside one process are related. Every run here is its own
//! process: the built binary is spawned, loads the file and writes the
//! certificate, and the bytes are compared across processes.
//!
//! The synthetic ontology is built so that almost every conclusion has more
//! than one derivation in the round that first reaches it: class and property
//! diamonds (rdfs11, rdfs5, rdfs7, rdfs9, rdfs2), a transitive chain with
//! several middles (prp-trp), restriction and union memberships with two
//! witnesses each (cls-svf1, cls-uni), and an intersection every copy reaches
//! in the same round (cls-int1). With sixteen copies of each shape, two
//! hash-ordered runs agreeing by chance is not a possibility worth discussing.
//!
//! Every IRI is named. A parser mints a fresh blank node label on each load, so
//! a file with blank nodes gives two different stores, and this is about two
//! runs over the same one.
//!
//! Each certificate is also checked for the ordering `oo-cert` requires:
//! every premise of a step is asserted or concluded by an EARLIER line. That
//! is what a deterministic order must not give up.

use std::collections::HashSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Copies of each shape. Every copy multiplies the number of distinct
/// certificates a hash-ordered run could write.
const COPIES: usize = 16;

fn synthetic_ontology() -> String {
    let mut t = String::from(
        "@prefix : <http://example.org/det#> .\n\
         @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
         :t a owl:TransitiveProperty .\n\
         :hasPart a owl:ObjectProperty .\n",
    );
    for i in 0..COPIES {
        // A class diamond three wide, with a tail: A ⊑ B0|B1|B2 ⊑ C ⊑ D.
        for j in 0..3 {
            t.push_str(&format!(":A{i} rdfs:subClassOf :B{i}_{j} .\n"));
            t.push_str(&format!(":B{i}_{j} rdfs:subClassOf :C{i} .\n"));
        }
        t.push_str(&format!(":C{i} rdfs:subClassOf :D{i} .\n"));
        t.push_str(&format!(":E{i} owl:equivalentClass :A{i} .\n"));
        t.push_str(&format!(":x{i} a :A{i} .\n"));

        // A property diamond: p ⊑ q0|q1 ⊑ r, with r carrying a domain.
        for j in 0..2 {
            t.push_str(&format!(":p{i} rdfs:subPropertyOf :q{i}_{j} .\n"));
            t.push_str(&format!(":q{i}_{j} rdfs:subPropertyOf :r{i} .\n"));
        }
        t.push_str(&format!(":r{i} rdfs:domain :Dom{i} .\n"));
        t.push_str(&format!(":x{i} :p{i} :y{i} .\n"));

        // A transitive chain of five nodes: n0 t n4 has three middles.
        for k in 0..4 {
            t.push_str(&format!(":n{i}_{k} :t :n{i}_{} .\n", k + 1));
        }

        // An existential with two witnesses, each typed through the diamond.
        t.push_str(&format!(
            ":R{i} a owl:Restriction ; owl:onProperty :hasPart ; owl:someValuesFrom :C{i} .\n"
        ));
        for j in 0..2 {
            t.push_str(&format!(":x{i} :hasPart :w{i}_{j} .\n"));
            t.push_str(&format!(":w{i}_{j} a :B{i}_{j} .\n"));
        }

        // A union x reaches through two of its members in the same round. The
        // list nodes are named: a parser mints a fresh blank node label on
        // every load, which would make the two stores differ, and this test is
        // about two runs over the SAME store.
        t.push_str(&format!(":U{i} owl:unionOf :L{i}_0 .\n"));
        t.push_str(&format!(":L{i}_0 rdf:first :B{i}_0 ; rdf:rest :L{i}_1 .\n"));
        t.push_str(&format!(":L{i}_1 rdf:first :B{i}_1 ; rdf:rest rdf:nil .\n"));

        // An intersection x reaches once it holds both members, so cls-int1
        // fires for every copy in the same round.
        t.push_str(&format!(":I{i} owl:intersectionOf :M{i}_0 .\n"));
        t.push_str(&format!(":M{i}_0 rdf:first :B{i}_0 ; rdf:rest :M{i}_1 .\n"));
        t.push_str(&format!(":M{i}_1 rdf:first :B{i}_1 ; rdf:rest rdf:nil .\n"));
    }
    t
}

/// The rules each profile has to fire over the synthetic ontology, so that a
/// later change cannot quietly turn this into a test of nothing.
fn expected_rules(profile: &str) -> Vec<&'static str> {
    let mut r = vec!["rdfs2", "rdfs5", "rdfs7", "rdfs9", "rdfs11"];
    if profile != "rdfs" {
        r.extend(["prp-trp", "scm-eqc1", "scm-dom2"]);
    }
    if profile == "owl-rl-ext" {
        r.extend(["cls-svf1", "cls-uni", "cls-int1"]);
    }
    r
}

/// One process: load `file`, reason under `profile`, write the certificate
/// into `cert`. The data directory is the run's own, so nothing is shared
/// between two runs but the binary.
fn certify(scratch: &Path, run: &str, file: &Path, profile: &str) -> PathBuf {
    let data = scratch.join(format!("data-{run}"));
    let cert = scratch.join(format!("cert-{run}"));
    let lines = format!(
        "load {}\nreason --profile {profile} --certificate {}\n",
        file.display(),
        cert.display()
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .arg("--data-dir")
        .arg(&data)
        .args(["batch", "-", "--no-connect"])
        .env_remove("OPEN_ONTOLOGIES_TOKEN")
        .env_remove("OPEN_ONTOLOGIES_STORAGE_MODE")
        .env_remove("GOVERNANCE_WEBHOOK")
        .env("RUST_LOG", "warn")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary should run");
    child.stdin.take().unwrap().write_all(lines.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && !stdout.contains("\"error\""),
        "batch failed for {profile} ({run}):\n{stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        cert.join("derivations.tsv").is_file(),
        "no derivations.tsv for {profile} ({run}):\n{stdout}"
    );
    cert
}

/// Every premise of every step is asserted or concluded by an earlier line,
/// which is the order `OOCert.checkAll` reads the file in.
fn assert_premises_precede(cert: &Path, what: &str) {
    let asserted = std::fs::read_to_string(cert.join("asserted.tsv")).unwrap();
    let mut known: HashSet<String> = asserted.lines().map(str::to_string).collect();
    let steps = std::fs::read_to_string(cert.join("derivations.tsv")).unwrap();
    for (n, line) in steps.lines().enumerate() {
        let fields: Vec<&str> = line.split('\t').collect();
        assert!(
            fields.len() >= 4 && (fields.len() - 1).is_multiple_of(3),
            "{what}: line {} is not rule + triples: {line}",
            n + 1
        );
        for premise in fields[4..].chunks(3) {
            let p = premise.join("\t");
            assert!(
                known.contains(&p),
                "{what}: line {} cites a premise no earlier line asserts or concludes: {p}",
                n + 1
            );
        }
        known.insert(fields[1..4].join("\t"));
    }
}

fn first_difference(a: &str, b: &str) -> String {
    for (n, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}:\n  first:  {x}\n  second: {y}", n + 1);
        }
    }
    format!(
        "one file is a prefix of the other ({} lines against {})",
        a.lines().count(),
        b.lines().count()
    )
}

/// Three runs, three processes, one set of bytes.
fn assert_deterministic(file: &Path, profile: &str, rules: &[&str]) {
    let scratch = tempfile::tempdir().unwrap();
    let certs: Vec<PathBuf> = (0..3)
        .map(|r| certify(scratch.path(), &format!("{profile}-{r}"), file, profile))
        .collect();
    let what = format!("{} under {profile}", file.display());
    let bytes: Vec<String> = certs
        .iter()
        .map(|c| std::fs::read_to_string(c.join("derivations.tsv")).unwrap())
        .collect();
    assert!(!bytes[0].is_empty(), "{what}: the run derived nothing, so it tests nothing");
    let fired: HashSet<&str> = bytes[0].lines().filter_map(|l| l.split('\t').next()).collect();
    for rule in rules {
        assert!(fired.contains(rule), "{what}: {rule} never fired, so its order is not tested");
    }
    for c in &certs {
        assert_premises_precede(c, &what);
    }
    for (r, b) in bytes.iter().enumerate().skip(1) {
        assert!(
            *b == bytes[0],
            "{what}: run {r} wrote a different derivations.tsv from run 0, {}",
            first_difference(&bytes[0], b)
        );
    }
    // The asserted side was already stable; it stays so.
    let asserted: Vec<Vec<u8>> =
        certs.iter().map(|c| std::fs::read(c.join("asserted.tsv")).unwrap()).collect();
    assert!(asserted.iter().all(|a| *a == asserted[0]), "{what}: asserted.tsv differs between runs");
}

#[test]
fn every_profile_writes_the_same_derivations_over_a_store_with_many_paths() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("diamonds.ttl");
    std::fs::write(&file, synthetic_ontology()).unwrap();
    for profile in ["rdfs", "owl-rl", "owl-rl-ext"] {
        assert_deterministic(&file, profile, &expected_rules(profile));
    }
}

#[test]
fn ies_core_writes_the_same_derivations_on_every_run() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmark/reference/ies-core.ttl");
    assert_deterministic(&file, "rdfs", &["rdfs11"]);
}
