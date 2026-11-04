// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Standard rule syntaxes into the Horn certificate layer, end to end.
//!
//! `lean/OOCert/Horn.lean` proves one soundness theorem good for every rule
//! table, and `reason --rules` evaluates a table and writes a certificate. What
//! neither had was a way IN from a language anybody writes: the only rule syntax
//! the repository could read was the tab-separated internal encoding. These
//! tests take a real SWRL ontology and a real RIF Core XML document, turn each
//! into a table, run the engine over it, and hand the certificate to the Lean
//! checker.
//!
//! # The distinction these tests exist to protect
//!
//! Exactly one table earns `entailed`: the BUILT-IN one, because
//! `OOCert.Builtin.asHorn_sound` discharges its rules against the RDF
//! semantics. Every rule a front end imports was written by a user and is
//! discharged by nobody, so a certificate over it earns
//! `entailed_under_supplied_rules` under `OOCert.horn_certificate_sound`: true
//! in every model of the asserted graph THAT ALSO SATISFIES THOSE RULES.
//!
//! `family_swrl.ttl` contains the rule that makes the point, spelled in SWRL:
//! every supplier is compliant. It is a valid SWRL rule, it is assumed and never
//! checked, and a certificate over a table containing it checks green for ever.
//! `a_swrl_rule_never_earns_the_absolute_verdict` and its RIF twin are the
//! `a_user_rule_never_earns_the_absolute_verdict` of the front ends, and
//! `no_front_end_can_name_a_rule_the_way_a_built_in_is_named` pins the
//! structural reason without needing Lean present.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{Reasoner, RulePattern, rules_tsv};
use open_ontologies::rulesyntax::{Import, rif_core_from_xml, run_import, swrl_from_graph};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn lean_dir() -> PathBuf {
    repo().join("lean")
}

fn fixture(name: &str) -> PathBuf {
    repo().join("tests").join("fixtures").join("rules").join(name)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oo-rulesyntax-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A store holding only the data, never the rules: the SWRL encoding triples
/// are not facts the rules should reason over.
fn data_graph() -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_file(&fixture("family.ttl").display().to_string()).unwrap();
    g
}

fn import_swrl(path: &str) -> Import {
    let g = Arc::new(GraphStore::new());
    g.load_file(&fixture(path).display().to_string()).unwrap();
    swrl_from_graph(&g).unwrap()
}

fn import_rif(path: &str) -> Import {
    rif_core_from_xml(&std::fs::read_to_string(fixture(path)).unwrap()).unwrap()
}

fn json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap_or_else(|e| panic!("not JSON ({e}): {s}"))
}

// ── Lean plumbing, the same shape lean_horn_certificate_test.rs uses ─────────

fn lake_available() -> bool {
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

/// Build once per test binary. The proofs are part of the build, so a failure
/// here is a failure and never a skip.
fn checker() -> &'static Path {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
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
    })
}

fn field<'a>(json: &'a str, key: &str) -> &'a str {
    let pat = format!("\"{key}\":\"");
    let start = json.find(&pat).unwrap_or_else(|| panic!("no {key} in {json}")) + pat.len();
    let rest = &json[start..];
    &rest[..rest.find('"').expect("unterminated")]
}

/// Import → evaluate → certify → check. Returns the engine's response and the
/// checker's, so a test can assert on both halves of the pipeline.
fn round_trip(name: &str, rules: &[RulePattern]) -> (serde_json::Value, i32, String) {
    let dir = scratch(name);
    let table = dir.join("imported.tsv");
    std::fs::write(&table, rules_tsv(rules)).unwrap();

    let cert = dir.join("cert");
    let engine = json(&Reasoner::run_horn(&data_graph(), &table, &cert).unwrap());

    let out = Command::new(checker())
        .arg("check")
        .arg(cert.join("rules.tsv"))
        .arg(cert.join("asserted.tsv"))
        .arg(cert.join("horn.tsv"))
        .output()
        .expect("run oo-horn");
    (engine, out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).to_string())
}

// ── The laundering guards ───────────────────────────────────────────────────

#[test]
fn a_swrl_rule_never_earns_the_absolute_verdict() {
    // `family_swrl.ttl` holds "every supplier is compliant" as a SWRL rule. The
    // certificate still checks, because the inference is still valid under the
    // rule; the verdict must say what it is relative to.
    if skip() {
        return;
    }
    let imp = import_swrl("family_swrl.ttl");
    assert!(imp.refused.is_empty(), "{:?}", imp.refused);
    let (engine, code, out) = round_trip("swrl", &imp.rules);

    assert_eq!(code, 0, "{out}");
    assert_ne!(
        field(&out, "verdict"),
        "entailed",
        "a run over rules imported from SWRL must NOT report absolute entailment: {out}"
    );
    assert_eq!(field(&out, "verdict"), "entailed_under_supplied_rules", "{out}");
    assert_eq!(field(&out, "theorem"), "OOCert.horn_certificate_sound", "{out}");
    assert_ne!(
        field(&out, "rules_digest"),
        field(&out, "builtin_rules_digest"),
        "the digest must show the imported table differed from the built-ins: {out}"
    );
    assert!(
        field(&out, "means").contains("assumed, not checked"),
        "the report must say the rules are assumed: {out}"
    );

    // The rules really did fire, so the verdict is about a certificate with
    // something in it rather than an empty one.
    assert_eq!(engine["derived_triples"], 2, "{engine}");
    let derived: Vec<&str> =
        engine["sample_derivations"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert!(
        derived.contains(
            &"<http://example.org/family#john> <http://example.org/family#hasUncle> <http://example.org/family#bob>"
        ),
        "{derived:?}"
    );
    assert!(
        derived.contains(
            &"<http://example.org/family#acme> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/family#Compliant>"
        ),
        "{derived:?}"
    );
}

#[test]
fn a_rif_rule_never_earns_the_absolute_verdict() {
    if skip() {
        return;
    }
    let imp = import_rif("family_rif.xml");
    assert!(imp.refused.is_empty(), "{:?}", imp.refused);
    let (engine, code, out) = round_trip("rif", &imp.rules);

    assert_eq!(code, 0, "{out}");
    assert_ne!(
        field(&out, "verdict"),
        "entailed",
        "a run over rules imported from RIF Core must NOT report absolute entailment: {out}"
    );
    assert_eq!(field(&out, "verdict"), "entailed_under_supplied_rules", "{out}");
    assert_eq!(field(&out, "theorem"), "OOCert.horn_certificate_sound", "{out}");
    assert_ne!(field(&out, "rules_digest"), field(&out, "builtin_rules_digest"), "{out}");
    assert_eq!(engine["derived_triples"], 2, "{engine}");
}

#[test]
fn no_front_end_can_name_a_rule_the_way_a_built_in_is_named() {
    // Why the two tests above cannot come out the other way, without needing
    // Lean present to see it. `oo-horn` awards the absolute verdict only when
    // the table it is given renders IDENTICALLY to the built-in one, names
    // included. Every imported rule is named `swrl/…` or `rif/…`; no built-in
    // rule is; so the tables can never coincide whatever the rules say.
    let builtin =
        std::fs::read_to_string(repo().join("tests/fixtures/horn/builtin_rules.tsv")).unwrap();
    let builtin_names: Vec<&str> =
        builtin.lines().filter(|l| !l.is_empty()).map(|l| l.split('\t').next().unwrap()).collect();
    assert_eq!(builtin_names.len(), 27, "the built-in table changed shape");
    for n in &builtin_names {
        assert!(
            !n.starts_with("swrl/") && !n.starts_with("rif/"),
            "a built-in rule is named {n}, which an imported rule could now collide with"
        );
    }

    for (what, rules) in [
        ("swrl", import_swrl("family_swrl.ttl").rules),
        ("rif", import_rif("family_rif.xml").rules),
    ] {
        assert!(!rules.is_empty(), "{what} imported nothing");
        for r in &rules {
            assert!(
                r.name.starts_with("swrl/") || r.name.starts_with("rif/"),
                "{what} emitted the rule name {}, which is not provenance-marked",
                r.name
            );
        }
        assert_ne!(rules_tsv(&rules), builtin, "{what} produced the built-in table");
    }
}

#[test]
fn the_two_front_ends_agree_on_the_same_two_rules() {
    // family_swrl.ttl and family_rif.xml say the same thing in two languages.
    // Two independently written parsers landing on the same patterns is the
    // strongest check available that neither is quietly mis-reading its input.
    // Only the NAMES differ, and they must, because the provenance is different.
    // Matched by the local part of the rule's name rather than by position:
    // SWRL rules come out in the order of their subject IRIs, RIF rules in
    // document order, and neither ordering is the other's.
    let swrl = import_swrl("family_swrl.ttl").rules;
    let rif = import_rif("family_rif.xml").rules;
    assert_eq!(swrl.len(), 2);
    assert_eq!(rif.len(), 2);
    let tail = |n: &str| n.rsplit(['#', '/']).next().unwrap().to_string();
    for a in &swrl {
        let b = rif
            .iter()
            .find(|b| tail(&b.name) == tail(&a.name))
            .unwrap_or_else(|| panic!("no RIF counterpart for {}", a.name));
        assert_eq!(a.body, b.body, "bodies differ for {} and {}", a.name, b.name);
        assert_eq!(a.head, b.head, "heads differ for {} and {}", a.name, b.name);
        assert!(a.name.starts_with("swrl/") && b.name.starts_with("rif/"));
    }
}

// ── Every refusal, and the flag that says a table lost something ────────────

#[test]
fn a_built_in_atom_refuses_the_import_and_writes_no_table() {
    let dir = scratch("builtin-refused");
    let out = dir.join("rules.tsv");
    let r = json(
        &run_import(
            &data_graph(),
            "swrl",
            Some(&fixture("family_swrl_builtin.ttl")),
            Some(&out),
            false,
        )
        .unwrap(),
    );
    assert!(r["error"].is_string(), "a refused rule must fail the import: {r}");
    assert_eq!(r["rules_written"], false);
    assert!(!out.exists(), "no table may be written when a rule was refused");
    assert_eq!(r["rules_refused"], 1);
    assert_eq!(r["certifies_a_weaker_rule_set"], true);
    assert!(
        r["refused"][0]["construct"].as_str().unwrap().contains("swrlb#greaterThan"),
        "the refusal must name the built-in: {r}"
    );
}

#[test]
fn allow_partial_writes_the_rest_and_flags_it_as_a_weaker_rule_set() {
    // The opt-in half. A table that lost a rule still reaches a fixpoint and
    // still produces a certificate that checks green, so the loss has to be
    // visible in the artefact and not only in a log line.
    let dir = scratch("builtin-partial");
    let out = dir.join("rules.tsv");
    let r = json(
        &run_import(
            &data_graph(),
            "swrl",
            Some(&fixture("family_swrl_builtin.ttl")),
            Some(&out),
            true,
        )
        .unwrap(),
    );
    assert!(r["error"].is_null(), "{r}");
    assert_eq!(r["rules_written"], true);
    assert_eq!(r["source_rules"], 2);
    assert_eq!(r["rules_emitted"], 1);
    assert_eq!(r["rules_refused"], 1);
    assert_eq!(
        r["certifies_a_weaker_rule_set"], true,
        "a partial import must SAY the rule set is weaker than the source: {r}"
    );
    assert_eq!(r["constructs_not_supported"][0]["occurrences"], 1);
    assert_eq!(r["verdict_this_table_is_eligible_for"], "entailed_under_supplied_rules");
    assert!(out.exists());
}

#[test]
fn a_partial_import_can_never_be_silent() {
    // The invariant behind the flag, stated so a later edit cannot lose it:
    // there is no combination of arguments that writes a table with a refusal
    // recorded and `certifies_a_weaker_rule_set` false.
    for (syntax, file) in [
        ("swrl", "family_swrl_builtin.ttl"),
        ("swrl", "family_swrl_truncated.ttl"),
        ("swrl", "family_swrl_empty_head.ttl"),
        ("rif", "family_rif_head_equality.xml"),
        ("rif", "family_rif_external.xml"),
    ] {
        for allow_partial in [false, true] {
            let dir = scratch("silent");
            let out = dir.join("rules.tsv");
            let r = json(
                &run_import(
                    &data_graph(),
                    syntax,
                    Some(&fixture(file)),
                    Some(&out),
                    allow_partial,
                )
                .unwrap(),
            );
            assert!(r["rules_refused"].as_u64().unwrap() > 0, "{file}: {r}");
            assert_eq!(
                r["certifies_a_weaker_rule_set"], true,
                "{file} (allow_partial={allow_partial}) wrote a report that does not admit it \
                 lost a rule: {r}"
            );
            assert!(
                !r["refused"].as_array().unwrap().is_empty(),
                "{file}: the lost rule must be named, not just counted: {r}"
            );
            if !allow_partial {
                assert!(r["error"].is_string(), "{file}: {r}");
                assert!(!out.exists(), "{file}: a refused import wrote a table anyway");
            }
        }
    }
}

#[test]
fn a_truncated_atom_list_is_refused_rather_than_read_short() {
    let imp = import_swrl("family_swrl_truncated.ttl");
    assert!(imp.rules.is_empty());
    assert_eq!(imp.refused.len(), 1);
    assert!(imp.refused[0].construct.contains("swrl:body"), "{:?}", imp.refused[0]);
    assert!(imp.refused[0].why.contains("rdf:rest"), "{:?}", imp.refused[0]);
    assert!(
        imp.refused[0].why.contains("fires more often"),
        "the refusal must say WHY a short read is unsafe: {:?}",
        imp.refused[0]
    );
}

#[test]
fn an_empty_head_is_refused_because_there_is_nothing_to_derive() {
    let imp = import_swrl("family_swrl_empty_head.ttl");
    assert!(imp.rules.is_empty());
    assert_eq!(imp.refused.len(), 1);
    assert_eq!(imp.refused[0].construct, "swrl:head of length zero");
}

#[test]
fn equality_in_the_conclusion_is_refused_and_the_rest_is_kept_only_on_request() {
    let imp = import_rif("family_rif_head_equality.xml");
    assert_eq!(imp.source_rules, 2);
    assert_eq!(imp.rules.len(), 1);
    assert_eq!(imp.refused.len(), 1);
    assert_eq!(imp.refused[0].construct, "rif:Equal in the conclusion");
    assert!(imp.refused[0].why.contains("substitute equals for equals"), "{:?}", imp.refused[0]);
}

#[test]
fn an_external_predicate_is_refused() {
    let imp = import_rif("family_rif_external.xml");
    assert!(imp.rules.is_empty());
    assert_eq!(imp.refused.len(), 1);
    assert_eq!(imp.refused[0].construct, "rif:External");
}

#[test]
fn the_presentation_syntax_is_refused_by_name_rather_than_half_read() {
    let r = run_import(
        &data_graph(),
        "rif",
        Some(&fixture("family_rif_presentation.rifps")),
        None,
        true,
    )
    .unwrap_err()
    .to_string();
    assert!(r.contains("presentation syntax"), "{r}");
    assert!(r.contains("XML"), "{r}");
}

#[test]
fn datalog_is_not_offered_as_a_syntax_it_does_not_have() {
    // The family decision 0003 names is Datalog, RIF Core and SWRL. Datalog has
    // no single standard concrete syntax, and claiming one would be exactly the
    // "covered in architecture" move this work exists to undo.
    let e = run_import(&data_graph(), "datalog", None, None, false).unwrap_err().to_string();
    assert!(e.contains("Datalog"), "{e}");
    assert!(e.contains("rules.tsv"), "the error must say where a Datalog program already goes: {e}");
}

#[test]
fn the_reported_fragment_names_what_is_not_supported() {
    // The response is what a user reads, so the fragment has to be IN it and
    // has to be specific. "SWRL is supported" would be the false claim.
    let r = json(&run_import(&data_graph(), "swrl", Some(&fixture("family_swrl.ttl")), None, false).unwrap());
    let frag = r["supported_fragment"].as_str().unwrap();
    for must in ["swrl:ClassAtom", "swrl:IndividualPropertyAtom", "Everything else is refused"] {
        assert!(frag.contains(must), "the SWRL fragment does not mention {must}: {frag}");
    }
    assert!(r["rules_tsv"].is_string(), "with no --out the table comes back inline: {r}");

    let r = json(&run_import(&data_graph(), "rif", Some(&fixture("family_rif.xml")), None, false).unwrap());
    let frag = r["supported_fragment"].as_str().unwrap();
    for must in ["presentation syntax is NOT read", "Frame", "CONVENTION"] {
        assert!(frag.contains(must), "the RIF fragment does not mention {must}: {frag}");
    }
}
