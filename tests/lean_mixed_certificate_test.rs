// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! One certificate carrying both rule families.
//!
//! Until `lean/OOCert/Mixed.lean` landed, a certificate was either all built-in
//! arms (`OOCert.certificate_sound`) or all citations of a user-supplied Horn
//! rule (`OOCert.horn_certificate_sound`). That split is not cosmetic.
//! `cls-int1` and `cls-uni` read an RDF list off the graph, so their premise
//! count is the length of that list rather than something the rule fixes, and
//! they are never going to become Horn rules. A user with one rule of their own
//! therefore could not cite them at all.
//!
//! `OOCert.checkCertM` accepts both step kinds and `OOCert.mixed_certificate_sound`
//! covers both in one induction. The certificate exercised below is the smallest
//! one that needs the feature: a `cls-int1` step concludes that acme is a
//! `Supplier`, and a step citing a rule the checker has never heard of turns that
//! into a compliance claim.
//!
//! # The distinction these tests exist to protect
//!
//! A mixed certificate over a table the user supplied proves the RELATIVE
//! guarantee: every conclusion is true in every model of the asserted graph that
//! ALSO satisfies those rules. Nothing checks the rules. The rule used here is
//! decision 0003's own example, "every supplier is compliant", and
//! `OOCert.mix_not_absolutely_entailed` is a machine-checked proof that its
//! conclusion does NOT follow from the asserted graph alone: there is a model of
//! that graph in which acme has no status at all.
//!
//! So a run over this table must print `entailed_under_supplied_rules` and must
//! never print `entailed`. `a_user_rule_never_earns_the_absolute_verdict` is the
//! test that fails if it ever does, and `OOCert.absolute_verdict_is_earned` is
//! the theorem that says the report function cannot do it in the first place.
//!
//! # How these tests run Lean
//!
//! There is no `oo-mixed` binary. Adding one means editing `lean/lakefile.toml`,
//! which this change does not own, so each case is a small generated Lean file
//! run through `lake env lean --run`. It imports the same library `lake build`
//! proved and calls the same `checkCertM`. What it does NOT exercise is a file
//! format: there is no parser for a mixed certificate yet, and the certificates
//! below are Lean terms rather than TSV. That is a real gap, stated here rather
//! than left to be discovered.

mod common;

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn lean_dir() -> PathBuf {
    repo().join("lean")
}

fn lake_available() -> bool {
    // `.current_dir(lean_dir())`: elan resolves the toolchain from the working
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

/// Build once per test binary. The proofs are part of the build, so a failure
/// here is a failure and never a skip.
fn built() -> bool {
    static BUILT: OnceLock<bool> = OnceLock::new();
    *BUILT.get_or_init(|| {
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
        true
    })
}

const TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const FIRST: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>";
const REST: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>";
const NIL: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>";
const SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const INTERSECTION: &str = "<http://www.w3.org/2002/07/owl#intersectionOf>";

const ACME: &str = "<u:acme>";
const VENDOR: &str = "<u:Vendor>";
const APPROVED: &str = "<u:Approved>";
const SUPPLIER: &str = "<u:Supplier>";
const TRADER: &str = "<u:Trader>";
const STATUS: &str = "<u:status>";
const COMPLIANT: &str = "<u:Compliant>";
const L0: &str = "_:l0";
const L1: &str = "_:l1";

/// A Lean string literal. Every term here is ASCII with no quote or backslash,
/// so Rust's debug spelling is also Lean's.
fn q(s: &str) -> String {
    format!("{s:?}")
}

fn triple(s: &str, p: &str, o: &str) -> String {
    format!("Triple.mk {} {} {}", q(s), q(p), q(o))
}

fn lean_list(items: &[String]) -> String {
    format!("[{}]", items.join(", "))
}

/// The asserted graph: `Supplier` is the intersection of `Vendor` and
/// `Approved`, acme is both, and acme is a `Trader` with `Trader` a subclass of
/// `Vendor` so that a built-in Horn rule has something to do as well.
fn asserted() -> String {
    lean_list(&[
        triple(SUPPLIER, INTERSECTION, L0),
        triple(L0, FIRST, VENDOR),
        triple(L0, REST, L1),
        triple(L1, FIRST, APPROVED),
        triple(L1, REST, NIL),
        triple(ACME, TYPE, VENDOR),
        triple(ACME, TYPE, APPROVED),
        triple(ACME, TYPE, TRADER),
        triple(TRADER, SUBCLASS, VENDOR),
    ])
}

/// "Every supplier is compliant." Nothing checks it, which is the point.
fn compliance_table() -> String {
    format!(
        "[RulePattern.mk \"every-supplier-is-compliant\" \
         [AtomPat.mk (Pat.var \"s\") (Pat.const {}) (Pat.const {})] \
         (AtomPat.mk (Pat.var \"s\") (Pat.const {}) (Pat.const {}))]",
        q(TYPE),
        q(SUPPLIER),
        q(STATUS),
        q(COMPLIANT)
    )
}

/// The `cls-int1` step: a hardcoded arm, because the premise count is the length
/// of the RDF list.
fn int_step(conclusion: &str) -> String {
    let premises = lean_list(&[
        triple(SUPPLIER, INTERSECTION, L0),
        triple(L0, FIRST, VENDOR),
        triple(L0, REST, L1),
        triple(L1, FIRST, APPROVED),
        triple(L1, REST, NIL),
        triple(ACME, TYPE, VENDOR),
        triple(ACME, TYPE, APPROVED),
    ]);
    format!(
        "MStep.builtin (Step.mk Rule.clsInt1 {premises} ({}))",
        triple(ACME, TYPE, conclusion)
    )
}

/// The step citing the supplied rule, taking the built-in step's conclusion as
/// its only premise.
fn horn_step(rule: usize, bind: &str, premise_subject: &str, conclusion: &str) -> String {
    format!(
        "MStep.horn (HornStep.mk {rule} [(\"s\", {})] [{}] ({}))",
        q(bind),
        triple(premise_subject, TYPE, SUPPLIER),
        conclusion
    )
}

fn compliance_conclusion(subject: &str) -> String {
    triple(subject, STATUS, COMPLIANT)
}

const DRIVER: &str = r#"import OOCert
open OOCert

def G : List Triple := __G__

def R : List RulePattern := __R__

def steps : List MStep := __STEPS__

def main : IO UInt32 := do
  let ok := checkCertM G R steps
  let w := warrantOf R
  let verdict := if ok then w.verdict else "rejected"
  IO.println ("ok\t" ++ toString ok)
  IO.println ("verdict\t" ++ verdict)
  IO.println ("theorem\t" ++ w.theoremName)
  IO.println ("means\t" ++ w.means)
  return (if ok then 0 else 1)
"#;

/// Returns (exit code, stdout). The Lean file is regenerated per case, so a
/// stale artifact cannot make a rejection look like an acceptance.
fn run(name: &str, rules: &str, graph: &str, steps: &[String]) -> (i32, String) {
    assert!(built());
    let dir = std::env::temp_dir().join(format!("oo-mixed-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src = DRIVER
        .replace("__G__", graph)
        .replace("__R__", rules)
        .replace("__STEPS__", &lean_list(steps));
    let path = dir.join("Driver.lean");
    std::fs::write(&path, src).unwrap();

    let out = Command::new("lake")
        .arg("env")
        .arg("lean")
        .arg("--run")
        .arg(&path)
        .current_dir(lean_dir())
        .output()
        .expect("run lake env lean --run");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // Lean exits 1 both for a rejected certificate and for a file that does not
    // elaborate. The two must never be confused, so an elaboration failure is a
    // panic rather than a rejection.
    assert!(
        !text.contains("error:"),
        "the generated Lean file did not elaborate, which is not the same as a rejected \
         certificate:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.code().unwrap_or(-1), text)
}

fn field<'a>(out: &'a str, key: &str) -> &'a str {
    out.lines()
        .find_map(|l| l.strip_prefix(&format!("{key}\t")))
        .unwrap_or_else(|| panic!("no {key} line in {out}"))
}

/// The certificate that could not exist before `Mixed.lean`: a hardcoded arm
/// feeding a step that cites a rule the checker has never heard of.
fn good_mixed_steps() -> Vec<String> {
    vec![
        int_step(SUPPLIER),
        horn_step(0, ACME, ACME, &compliance_conclusion(ACME)),
    ]
}

#[test]
fn a_mixed_certificate_carries_both_step_kinds() {
    if skip() {
        return;
    }
    let (code, out) = run(
        "good",
        &compliance_table(),
        &asserted(),
        &good_mixed_steps(),
    );
    assert_eq!(code, 0, "the honest mixed certificate must be accepted: {out}");
    assert_eq!(field(&out, "ok"), "true", "{out}");
}

#[test]
fn a_user_rule_never_earns_the_absolute_verdict() {
    // The laundering guard. `OOCert.mix_not_absolutely_entailed` proves that the
    // conclusion of this very certificate is not entailed by the asserted graph,
    // so reporting `entailed` here would be a false statement about a
    // machine-checked fact.
    if skip() {
        return;
    }
    let (code, out) = run(
        "relative",
        &compliance_table(),
        &asserted(),
        &good_mixed_steps(),
    );
    assert_eq!(code, 0, "{out}");
    assert_ne!(
        field(&out, "verdict"),
        "entailed",
        "a run over user-supplied rules must NOT report absolute entailment: {out}"
    );
    assert_eq!(field(&out, "verdict"), "entailed_under_supplied_rules", "{out}");
    assert_eq!(field(&out, "theorem"), "OOCert.mixed_certificate_sound", "{out}");
    assert!(
        field(&out, "means").contains("assumed, not checked"),
        "the report must say the rules are assumed: {out}"
    );
}

#[test]
fn a_table_of_built_in_rules_earns_the_absolute_verdict() {
    // Index 4 of `Builtin.asHorn` is rdfs9. The certificate mixes it with the
    // `cls-int1` arm, which is exactly the combination `Mixed.lean` exists for,
    // and every rule in the table is discharged against the semantics by
    // `Builtin.asHorn_sound`, so this run earns `Entails`.
    if skip() {
        return;
    }
    let rdfs9 = format!(
        "MStep.horn (HornStep.mk 4 [(\"x\", {}), (\"a\", {}), (\"b\", {})] [{}, {}] ({}))",
        q(ACME),
        q(TRADER),
        q(VENDOR),
        triple(ACME, TYPE, TRADER),
        triple(TRADER, SUBCLASS, VENDOR),
        triple(ACME, TYPE, VENDOR)
    );
    let steps = vec![int_step(SUPPLIER), rdfs9];
    let (code, out) = run("builtin", "Builtin.asHorn", &asserted(), &steps);
    assert_eq!(code, 0, "{out}");
    assert_eq!(field(&out, "verdict"), "entailed", "{out}");
    assert_eq!(
        field(&out, "theorem"),
        "OOCert.mixed_certificate_sound_builtin",
        "{out}"
    );
}

#[test]
fn an_empty_table_is_the_old_certificate_unchanged() {
    if skip() {
        return;
    }
    let (code, out) = run("norules", "[]", &asserted(), &[int_step(SUPPLIER)]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(field(&out, "verdict"), "entailed", "{out}");
    assert_eq!(
        field(&out, "theorem"),
        "OOCert.mixed_certificate_sound_no_rules",
        "{out}"
    );
}

#[test]
fn every_forgery_is_rejected() {
    if skip() {
        return;
    }
    // A list the graph does not carry: `Supplier` would become the intersection
    // of `Approved` alone, so being approved would be enough. The chain triples
    // are absent from the asserted graph.
    let forged_chain = {
        let premises = lean_list(&[
            triple(SUPPLIER, INTERSECTION, L0),
            triple(L0, FIRST, APPROVED),
            triple(L0, REST, NIL),
            triple(ACME, TYPE, APPROVED),
        ]);
        format!(
            "MStep.builtin (Step.mk Rule.clsInt1 {premises} ({}))",
            triple(ACME, TYPE, SUPPLIER)
        )
    };

    let forgeries: Vec<(&str, Vec<String>, &str)> = vec![
        (
            "builtin_conclusion",
            vec![int_step(VENDOR), horn_step(0, ACME, ACME, &compliance_conclusion(ACME))],
            "the built-in step concludes membership in a class that is not the intersection",
        ),
        (
            "unasserted_list",
            vec![forged_chain, horn_step(0, ACME, ACME, &compliance_conclusion(ACME))],
            "the RDF list the built-in step walks is not in the asserted graph",
        ),
        (
            "horn_conclusion",
            vec![
                int_step(SUPPLIER),
                horn_step(0, ACME, ACME, &triple(ACME, STATUS, SUPPLIER)),
            ],
            "the cited rule's head under that binding is not the claimed conclusion",
        ),
        (
            "horn_index",
            vec![
                int_step(SUPPLIER),
                horn_step(7, ACME, ACME, &compliance_conclusion(ACME)),
            ],
            "the cited rule index is outside the supplied table",
        ),
        (
            "horn_binding",
            vec![
                int_step(SUPPLIER),
                horn_step(0, VENDOR, ACME, &compliance_conclusion(ACME)),
            ],
            "the binding does not instantiate the rule's body into the cited premises",
        ),
        (
            "horn_premise",
            vec![
                int_step(SUPPLIER),
                horn_step(0, VENDOR, VENDOR, &compliance_conclusion(VENDOR)),
            ],
            "the premise is neither asserted nor concluded by an earlier step",
        ),
        (
            "out_of_order",
            vec![
                horn_step(0, ACME, ACME, &compliance_conclusion(ACME)),
                int_step(SUPPLIER),
            ],
            "the step that derives the Horn step's premise comes after it, and a step may only \
             use what earlier steps concluded",
        ),
    ];

    for (name, steps, why) in forgeries {
        let (code, out) = run(name, &compliance_table(), &asserted(), &steps);
        assert_eq!(code, 1, "{name} must be rejected because {why}: {out}");
        assert_eq!(field(&out, "ok"), "false", "{name}: {out}");
        assert_eq!(
            field(&out, "verdict"),
            "rejected",
            "{name}: a rejected certificate must not carry either entailment verdict: {out}"
        );
    }
}
