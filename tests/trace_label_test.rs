//! **A reasoning trace is labelled step by step, and the third answer is the point.**
//!
//! `src/trace_label.rs` reads a trace that CLAIMS inference steps and returns a
//! label per step. This suite is the gate on the three things that make that
//! worth having.
//!
//! 1. THE THIRD FAMILY EXISTS AND IS NOT THE SECOND. A step that fails because
//!    the graph says nothing and a step that fails because the rule table is
//!    blind to the axiom that would license it are opposite findings, fixed in
//!    two different places. `three_labels_over_one_trace` runs all three in one
//!    call and the mutation that collapses the third into the second is written
//!    down beside it.
//! 2. THE LOCAL AND THE STORE ANSWER ARE TWO ANSWERS. A step can be
//!    machine-checked sound over the premises it cited while the graph holds
//!    none of them. `a_locally_sound_step_over_premises_the_store_lacks_is_not_entailed`
//!    is that case, and it must never read as an entailed family.
//! 3. A SUPPLIED TABLE CANNOT EARN THE ABSOLUTE WORD.
//!    `a_supplied_table_cannot_earn_the_absolute_word` asserts it over the
//!    SERIALISED report and not over the enum, for the reason
//!    `src/projection_entailment.rs` gives about its own suite: the enum is
//!    where the discipline is easy and the serialisation is where it leaks.
//!
//! Every test that needs a verdict needs the Lean checker, and skips loudly
//! through `common::skip_unless` when it is not built, so a CI job that sets
//! `OO_REQUIRE_FIXTURES=1` turns the skip into a failure rather than a green run
//! over nothing.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{
    CHAINED_RULES, InferenceTarget, Kw, Reasoner, RulePattern, parse_rules, rules_tsv,
};
use open_ontologies::trace_label::{
    Opts, Warrant, label_trace, parse_trace, rules_whose_head_can_conclude, warrant_of_theorem,
};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ── Vocabulary, spelled the way the store spells it ──────────────────────────

const TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const HAS_KEY: &str = "<http://www.w3.org/2002/07/owl#hasKey>";
const INTERSECTION_OF: &str = "<http://www.w3.org/2002/07/owl#intersectionOf>";

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(name: &str) -> PathBuf {
    repo().join("tests").join("fixtures").join("trace").join(name)
}

fn checker_path(name: &str) -> PathBuf {
    repo().join("lean").join(".lake").join("build").join("bin").join(name)
}

/// True when the caller should skip. The binaries are what pronounce, so a run
/// without them proves nothing about a label word.
fn skip_without(name: &str) -> bool {
    common::skip_unless(
        checker_path(name).exists(),
        &format!("the Lean checker {name}"),
        "build it with `cd lean && lake build` (elan from https://github.com/leanprover/elan)",
    )
}

/// A scratch directory per test, so a failure leaves its certificates behind.
fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-trace-label-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("create the scratch directory");
    d
}

fn store() -> Arc<GraphStore> {
    let ttl = std::fs::read_to_string(fixture("boundary.ttl")).expect("read boundary.ttl");
    let g = Arc::new(GraphStore::new());
    g.load_turtle(&ttl, None).expect("load boundary.ttl");
    g
}

fn opts(out: &Path) -> Opts {
    Opts {
        profile: "owl-rl".to_string(),
        rules: None,
        work_dir: out.to_path_buf(),
        checker: None,
        require_checker: false,
    }
}

/// A trace line, assembled the way `oo-trace/1` spells one: the rule, then the
/// conclusion, then each premise, three tab-separated N-Triples terms each.
fn line(rule: &str, conclusion: [&str; 3], premises: &[[&str; 3]]) -> String {
    let mut s = String::from(rule);
    for t in std::iter::once(&conclusion).chain(premises.iter()) {
        for term in t {
            s.push('\t');
            s.push_str(term);
        }
    }
    s.push('\n');
    s
}

/// The object of `:Adult rdfs:subClassOf _:x`, which is the restriction node
/// carrying the `owl:unionOf`.
///
/// Read out of the STORE rather than typed, because a blank node is relabelled
/// on parse and a hardcoded label would be a fixture that silently stops
/// matching anything. A step citing a triple the store does not hold is a
/// different test.
fn the_disjunctive_axiom(g: &Arc<GraphStore>) -> (String, String, String) {
    g.all_triples()
        .expect("read the store")
        .into_iter()
        .find(|(s, p, o)| s == "<http://e/Adult>" && p == SUBCLASS && o.starts_with("_:"))
        .expect("boundary.ttl states :Adult rdfs:subClassOf [ owl:unionOf ( :Man :Woman ) ]")
}

fn step_of(report: &Value, i: usize) -> &Value {
    &report["steps"][i]
}

fn as_json(report: &open_ontologies::trace_label::TraceLabelReport) -> (String, Value) {
    let text = serde_json::to_string(report).expect("the report serialises");
    let v: Value = serde_json::from_str(&text).expect("and parses back");
    (text, v)
}

// ── 1. The three families, over one trace ────────────────────────────────────

/// **Gate.** One call produces `entailed`, `not_entailed` and
/// `outside_the_fragment`, and the third is not the second.
///
/// MUTATION THAT BREAKS IT, run before landing: delete the
/// `_ if fragment.is_some()` arm of the `Membership::NotDerivable` match in
/// `label_trace`. Step (c) then comes back `not_entailed_under_this_table` and
/// this test fails on it, which is the whole point of the arm: a reader told to
/// patch `src/reason.rs` for an axiom no Horn rule could ever express has been
/// sent to the wrong place.
#[test]
fn three_labels_over_one_trace() {
    if skip_without("oo-cert") {
        return;
    }
    let g = store();
    let (ds, dp, dobj) = the_disjunctive_axiom(&g);
    let out = scratch("three");

    let mut trace = String::new();
    // (a) genuinely entailed: rdfs9 over two axioms the table reads.
    trace.push_str(&line(
        "rdfs9",
        ["<http://e/a>", TYPE, "<http://e/C>"],
        &[
            ["<http://e/a>", TYPE, "<http://e/B>"],
            ["<http://e/B>", SUBCLASS, "<http://e/C>"],
        ],
    ));
    // (b) bogus: the same premises, a conclusion nothing supports. `:D` occurs
    //     nowhere in the fixture.
    trace.push_str(&line(
        "rdfs9",
        ["<http://e/a>", TYPE, "<http://e/D>"],
        &[
            ["<http://e/a>", TYPE, "<http://e/B>"],
            ["<http://e/B>", SUBCLASS, "<http://e/C>"],
        ],
    ));
    // (c) outside the fragment: the step cites the disjunctive subsumption. No
    //     rule is claimed, so the label is decided on the conclusion alone and
    //     the fragment finding is what separates it from (b).
    trace.push_str(&line(
        "",
        ["<http://e/s>", TYPE, "<http://e/Man>"],
        &[[&ds, &dp, &dobj], ["<http://e/s>", TYPE, "<http://e/Adult>"]],
    ));

    let steps = parse_trace(&trace).expect("the trace parses");
    let report = label_trace(&g, &steps, &opts(&out)).expect("labelling runs");
    let (text, v) = as_json(&report);

    let a = step_of(&v, 0);
    assert_eq!(a["label"], "entailed_checked", "step (a): {a}");
    assert_eq!(a["family"], "entailed", "step (a): {a}");
    assert_eq!(a["theorem"], "OOCert.certificate_sound", "step (a): {a}");
    assert_eq!(a["rule_claim"], "licensed", "step (a): {a}");
    assert_eq!(a["conclusion_in_store"], "derived", "step (a): {a}");
    assert_eq!(a["premises_in_store"], true, "step (a): {a}");
    assert_eq!(a["local"]["verdict"], "locally_sound_checked", "step (a): {a}");
    assert!(
        a["subject_sha256"].as_str().is_some_and(|s| s.len() == 64),
        "an accepted step names the digest of what the checker was handed: {a}"
    );

    let b = step_of(&v, 1);
    assert_eq!(b["family"], "not_entailed", "step (b): {b}");
    assert_eq!(b["label"], "not_entailed_under_this_table", "step (b): {b}");
    assert_eq!(b["rule_claim"], "rule_in_table_but_did_not_fire_here", "step (b): {b}");
    assert_eq!(b["local"]["verdict"], "not_licensed_by_the_cited_rule", "step (b): {b}");
    assert_eq!(b["conclusion_in_store"], "not_derivable", "step (b): {b}");
    assert!(
        b["theorem"].is_null(),
        "a step nothing checked must not name a theorem: {b}"
    );
    // The non-empty head list is exactly what separates this label from the
    // third family. Without it the reader cannot tell "this graph does not
    // support it" from "no data ever could".
    assert!(
        !b["heads_that_could_conclude_it"].as_array().expect("an array").is_empty(),
        "step (b) must have rules whose head could conclude a triple of its shape: {b}"
    );

    let c = step_of(&v, 2);
    assert_eq!(c["family"], "outside_the_fragment", "step (c): {c}");
    assert_eq!(c["label"], "outside_the_fragment", "step (c): {c}");
    assert_eq!(c["rule_claim"], "no_rule_claimed", "step (c): {c}");
    let row = &c["fragment"]["premises_outside_the_fragment"][0];
    assert_eq!(row["standing"], "outside", "step (c) fragment row: {row}");
    let reason = row["outside_because"][0]["reason"].as_str().unwrap_or("");
    assert!(
        reason.contains("disjunction in the consequent"),
        "the fragment finding must carry src/dlp.rs's own reason, got {reason:?} in {c}"
    );

    assert_eq!(v["by_family"]["entailed"], 1, "{text}");
    assert_eq!(v["by_family"]["not_entailed"], 1, "{text}");
    assert_eq!(v["by_family"]["outside_the_fragment"], 1, "{text}");
    assert_eq!(v["exit_code"], 1, "a trace with a step outside the entailed family exits 1");
    assert_eq!(v["format"], "oo-trace-label/1");

    // The dataset is the product, so it is checked here rather than trusted.
    let jsonl = std::fs::read_to_string(out.join("labels.jsonl")).expect("labels.jsonl is written");
    let rows: Vec<Value> = jsonl
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("each line is one JSON object"))
        .collect();
    assert_eq!(rows.len(), 3, "one record per step: {jsonl}");
    assert_eq!(rows[0]["label"], "entailed_checked", "{jsonl}");
    assert_eq!(rows[2]["label"], "outside_the_fragment", "{jsonl}");
    let cmd = rows[0]["reproduce"].as_str().expect("a reproduce command");
    assert!(
        cmd.contains("oo-cert") && cmd.contains("asserted.tsv") && cmd.contains("derivations.tsv"),
        "the record must name the command a third party runs, got {cmd:?}"
    );
}

// ── 2. Dimension two, and what it actually measures ──────────────────────────

/// **Gate.** The built-in head list is NEVER empty, and that is a fact about
/// the table rather than a weakness of the reachability test.
///
/// The design note this module was written from claimed the opposite: that a
/// conclusion of `owl:hasKey` between two IRIs would come back with an empty
/// head list, because no row of `BUILTIN_RULES` fixes that predicate. It does
/// not. `rdfs7`'s head is three variables, so from `s p o` and
/// `p rdfs:subPropertyOf q` it concludes `s q o` for ANY `q`, and `prp-trp`,
/// `prp-symp`, `prp-inv1`, `prp-inv2` and `cls-hv1` have all-variable heads
/// too. So `no_rule_in_this_table_concludes_this_shape` is unreachable under
/// the built-in table and reachable only for a supplied table whose heads fix a
/// predicate, which is what
/// `no_supplied_head_concludes_this_shape` covers.
///
/// A test asserting the note's claim would have been a gate that cannot fail
/// in the worst way: it would have failed immediately and been "fixed" by
/// widening the reachability test until it lied.
///
/// MUTATION: make `rules_whose_head_can_conclude` return `Vec::new()` for the
/// built-in branch and this fails.
#[test]
fn the_builtin_head_list_is_never_empty_because_rdfs7_concludes_anything() {
    let key = (
        "<http://e/Person>".to_string(),
        HAS_KEY.to_string(),
        "<http://e/ssn>".to_string(),
    );
    let heads = rules_whose_head_can_conclude(&key, &[]);
    assert!(
        heads.contains(&"rdfs7".to_string()),
        "rdfs7's head is three variables, so some data makes it conclude any triple: {heads:?}"
    );
    assert!(
        !heads.contains(&"rdfs9".to_string()),
        "rdfs9's head fixes rdf:type in the predicate, so it cannot conclude an owl:hasKey \
         triple: {heads:?}"
    );
    // Every shape a caller might ask about, over the built-in table.
    for t in [
        key,
        ("<http://e/a>".to_string(), TYPE.to_string(), "<http://e/C>".to_string()),
        ("<http://e/B>".to_string(), SUBCLASS.to_string(), "<http://e/C>".to_string()),
    ] {
        assert!(
            !rules_whose_head_can_conclude(&t, &[]).is_empty(),
            "the built-in table has all-variable heads, so no triple shape is unreachable: {t:?}"
        );
    }
}

/// **Gate.** The four list rules are the one piece of rule-shape data this
/// module cannot read off `BUILTIN_RULES`, so a fifth chained rule with a
/// different head shape must fail here rather than silently widen the head list.
///
/// MUTATION: add a fifth name to `reason::CHAINED_RULES` and this fails.
#[test]
fn the_four_chained_rules_all_conclude_a_type_triple() {
    assert_eq!(
        CHAINED_RULES,
        &["cls-int1", "cls-int2", "cls-uni", "cls-oo"],
        "the head shape of every chained rule is hardcoded in \
         trace_label::rules_whose_head_can_conclude; a fifth rule needs that reviewed"
    );
    let a_type = ("<http://e/a>".to_string(), Kw::Type.iri().to_string(), "<http://e/C>".to_string());
    let heads = rules_whose_head_can_conclude(&a_type, &[]);
    for r in CHAINED_RULES {
        assert!(heads.contains(&r.to_string()), "{r} concludes a type triple: {heads:?}");
    }
    let a_subclass =
        ("<http://e/B>".to_string(), SUBCLASS.to_string(), "<http://e/C>".to_string());
    let heads = rules_whose_head_can_conclude(&a_subclass, &[]);
    for r in CHAINED_RULES {
        assert!(
            !heads.contains(&r.to_string()),
            "{r} concludes rdf:type and nothing else, so it must not be listed for a \
             subClassOf conclusion: {heads:?}"
        );
    }
}

/// **Gate.** The empty head list is reachable, and it is what the third family's
/// second label rests on.
///
/// A supplied table whose head fixes `<http://e/ancestor>` in the predicate
/// cannot conclude a triple over any other predicate, whatever data arrives.
/// That is decided from the table with no reference to the store.
///
/// MUTATION: make `rules_whose_head_can_conclude` push every supplied rule name
/// unconditionally and this fails on both halves.
#[test]
fn no_supplied_head_concludes_this_shape() {
    let supplied: Vec<RulePattern> =
        parse_rules(&std::fs::read_to_string(fixture("supplied_rules.tsv")).expect("read the table"))
            .expect("the table parses");
    let unrelated = (
        "<http://e/p1>".to_string(),
        "<http://e/unrelated>".to_string(),
        "<http://e/p2>".to_string(),
    );
    assert!(
        rules_whose_head_can_conclude(&unrelated, &supplied).is_empty(),
        "no head in the supplied table fixes <http://e/unrelated>, so no data could make it \
         conclude such a triple"
    );
    let ancestor = (
        "<http://e/p1>".to_string(),
        "<http://e/ancestor>".to_string(),
        "<http://e/p2>".to_string(),
    );
    assert_eq!(
        rules_whose_head_can_conclude(&ancestor, &supplied),
        vec!["anc".to_string()],
        "and the one head that does fix it is listed"
    );
}

// ── 3. The hard rule ─────────────────────────────────────────────────────────

/// **Gate, the mapping.** The one function that decides the word maps each
/// theorem name to the sentence it stands for, and anything else to nothing.
///
/// `Certified` has no public constructor, so this is the only way to exercise
/// the mapping directly. It is a SEPARATE test from the one over the serialised
/// report, deliberately: when both lived in one function this assertion ran
/// first and panicked, so the serialisation assertion, which is the one that
/// matters, was never reached under the mutation that was supposed to prove it
/// could fail. A gate that a prior gate short-circuits is not a gate.
///
/// MUTATION: change the `"OOCert.horn_certificate_sound"` arm of
/// `warrant_of_theorem` to `Some(Warrant::Absolute)` and this fails.
#[test]
fn the_theorem_name_decides_the_word_and_nothing_else_does() {
    assert_eq!(
        warrant_of_theorem("OOCert.horn_certificate_sound"),
        Some(Warrant::UnderSuppliedRules)
    );
    assert_eq!(warrant_of_theorem("OOCert.certificate_sound"), Some(Warrant::Absolute));
    assert_eq!(warrant_of_theorem("OOCert.entails_of_builtin_horn"), Some(Warrant::Absolute));
    assert_eq!(
        warrant_of_theorem("OOCert.something_weaker"),
        None,
        "a renamed or weakened theorem earns nothing at all"
    );
}

/// **Gate.** A run over a rule table the user wrote cannot print the absolute
/// word, in any field, anywhere in the SERIALISED report.
///
/// Asserted over the serialisation and not over the enum, for the reason
/// `src/projection_entailment.rs` gives about its own suite: the enum is where
/// the discipline is easy and the serialisation is where it leaks.
///
/// MUTATION THAT BREAKS IT: change the `"OOCert.horn_certificate_sound"` arm of
/// `warrant_of_theorem` to `Some(Warrant::Absolute)`. Assertion (i) then fails
/// on the serialised report. This is the assertion the whole workstream rests
/// on and the mutation was run before landing.
#[test]
fn a_supplied_table_cannot_earn_the_absolute_word() {
    if skip_without("oo-horn") {
        return;
    }
    let g = store();
    let out = scratch("supplied");
    let rules = fixture("supplied_rules.tsv");
    let mut trace = String::new();
    trace.push_str(&line(
        "anc",
        ["<http://e/p1>", "<http://e/ancestor>", "<http://e/p2>"],
        &[["<http://e/p1>", "<http://e/parent>", "<http://e/p2>"]],
    ));
    // A second step whose conclusion no head in this table can reach, so the
    // report carries the third family too and every step is either
    // `entailed_under_supplied_rules` or outside. No step is asserted, which
    // matters: `asserted_not_derived` is a lookup in asserted.tsv, is absolute
    // whatever table is in force, and would legitimately read family
    // `entailed`, so a trace containing one could not carry assertion (ii).
    trace.push_str(&line(
        "",
        ["<http://e/p1>", "<http://e/unrelated>", "<http://e/p2>"],
        &[["<http://e/p1>", "<http://e/parent>", "<http://e/p2>"]],
    ));

    let steps = parse_trace(&trace).expect("the trace parses");
    let o = Opts { rules: Some(rules.clone()), ..opts(&out) };
    let report = label_trace(&g, &steps, &o).expect("labelling runs");
    let (text, v) = as_json(&report);

    // (i) The absolute word is unreachable, in the label and in the family and
    //     in the theorem.
    for forbidden in [
        "\"label\":\"entailed_checked\"",
        "\"label\":\"entailed_unchecked\"",
        "\"family\":\"entailed\"",
        "\"theorem\":\"OOCert.certificate_sound\"",
        "\"theorem\":\"OOCert.entails_of_builtin_horn\"",
        "\"verdict\":\"locally_sound_checked\"",
    ] {
        assert!(
            !text.contains(forbidden),
            "the report over a SUPPLIED table contains {forbidden}. A certificate over a rule \
             table nobody discharged is entailed_under_supplied_rules and never entailed; \
             printing the absolute word here is assurance laundering with a per-step \
             interface:\n{text}"
        );
    }
    // (ii) And the relativised one IS reached, so (i) is not passing for want
    //      of a checked step.
    assert_eq!(
        step_of(&v, 0)["label"],
        "entailed_under_supplied_rules_checked",
        "{text}"
    );
    assert_eq!(step_of(&v, 0)["family"], "entailed_under_supplied_rules", "{text}");
    assert_eq!(
        step_of(&v, 0)["local"]["verdict"],
        "locally_sound_under_supplied_rules_checked",
        "{text}"
    );
    assert_eq!(
        step_of(&v, 1)["label"],
        "no_rule_in_this_table_concludes_this_shape",
        "{text}"
    );

    // (iii) The digest of the table that was in force, and it is the digest of
    //       the table AS THE RUN EVALUATED IT rather than of the user's bytes.
    let digest = v["rules_tsv_sha256"].as_str().expect("a digest over a supplied table");
    assert_eq!(digest.len(), 64, "a sha256 is 64 hex characters, got {digest:?}");
    assert!(
        digest.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "lower-case hex, got {digest:?}"
    );
    let expected = {
        use sha2::{Digest, Sha256};
        let table = parse_rules(&std::fs::read_to_string(&rules).unwrap()).unwrap();
        format!("{:x}", Sha256::digest(rules_tsv(&table).as_bytes()))
    };
    assert_eq!(digest, expected, "the digest must be of the canonical rendering of the table");

    // (iv) Every theorem named anywhere is the relativised one.
    for s in v["steps"].as_array().expect("steps") {
        for key in [&s["theorem"], &s["local"]["theorem"]] {
            if let Some(t) = key.as_str() {
                assert_eq!(t, "OOCert.horn_certificate_sound", "{text}");
            }
        }
    }

    // (v) And the report says what it is conditional on, in the report and not
    //     in a footnote.
    let cond = v["conditional_on"].as_str().expect("conditional_on over a supplied table");
    assert!(
        cond.contains("ASSUMED and never checked") && cond.contains("supplied_rules.tsv"),
        "the sentence must name the file and say it was assumed, got {cond:?}"
    );
}

// ── 4. The two answers ───────────────────────────────────────────────────────

/// **Gate.** A step whose own reasoning is machine-checked sound over premises
/// the graph does not hold is NOT an entailed family.
///
/// This is the half of a per-step labeller that a graph-only labeller cannot
/// do, and the half a local-only labeller gets actively wrong.
/// `OOCert.certificate_sound` really does hold over the cited premises, and
/// that is not the whole truth: the store holds none of them, so the conclusion
/// is not entailed by the store. Both answers are in the report and neither is
/// reported as the other.
///
/// MUTATION THAT BREAKS IT: delete the `Some(c) if !premises_in_store` arm of
/// the `Membership::NotDerivable` match. The step then reads
/// `not_entailed_under_this_table` and the assertion on the label fails, which
/// is the milder failure; the serious one is the version of this module that
/// asked the local question first, where the step read `entailed_checked`.
#[test]
fn a_locally_sound_step_over_premises_the_store_lacks_is_not_entailed() {
    if skip_without("oo-cert") {
        return;
    }
    let g = store();
    let out = scratch("invented");
    // None of :q, :X or :Y occurs in boundary.ttl. The step is a correct
    // application of rdfs9 over premises it invented.
    let trace = line(
        "rdfs9",
        ["<http://e/q>", TYPE, "<http://e/Y>"],
        &[
            ["<http://e/q>", TYPE, "<http://e/X>"],
            ["<http://e/X>", SUBCLASS, "<http://e/Y>"],
        ],
    );
    let steps = parse_trace(&trace).expect("the trace parses");
    let report = label_trace(&g, &steps, &opts(&out)).expect("labelling runs");
    let (text, v) = as_json(&report);
    let s = step_of(&v, 0);

    assert_eq!(
        s["local"]["verdict"], "locally_sound_checked",
        "the cited premises DO entail the conclusion, and the report must say so: {text}"
    );
    assert_eq!(s["local"]["theorem"], "OOCert.certificate_sound", "{text}");
    assert_eq!(s["rule_claim"], "licensed", "the citation is correct: {text}");

    assert_eq!(s["premises_in_store"], false, "{text}");
    assert_eq!(
        s["premises_not_in_store"].as_array().expect("an array").len(),
        2,
        "both invented premises must be named, not counted: {text}"
    );
    assert_eq!(s["conclusion_in_store"], "not_derivable", "{text}");

    assert_eq!(s["label"], "locally_sound_but_premises_not_in_the_store", "{text}");
    assert!(
        s["theorem"].is_null() && s["subject_sha256"].is_null(),
        "the step-level theorem is the warrant for the step-level LABEL, which is about the \
         store. The token this step earned is about its own cited premises and is reported under \
         `local`; naming it here would invite the reading this label exists to prevent: {text}"
    );
    assert_eq!(
        s["family"], "not_entailed",
        "a step reasoning from premises the graph does not hold is not entailed BY THE GRAPH, \
         whatever its own reasoning was worth: {text}"
    );
    assert!(
        !text.contains("\"family\":\"entailed\""),
        "no step may read family entailed on the strength of invented premises:\n{text}"
    );
    assert_eq!(v["exit_code"], 1, "{text}");
}

// ── 5. The format justification, made executable ─────────────────────────────

/// **Gate.** A trace this engine itself emitted labels every step entailed.
///
/// The whole argument for `oo-trace/1` is that it IS a line of
/// `derivations.tsv`, so the engine's own certificate is a valid trace. That is
/// an argument until it is run: this fails the moment the trace parser drifts
/// from what `src/reason.rs` writes.
///
/// MUTATION: change `parse_trace` to read the rule name last and this fails at
/// parse, before any label is produced.
#[test]
fn a_trace_this_engine_emitted_labels_every_step_entailed() {
    if skip_without("oo-cert") {
        return;
    }
    let g = store();
    let emitted = scratch("emitted");
    Reasoner::run_full(
        &g,
        "owl-rl-ext",
        false,
        InferenceTarget::DefaultGraph,
        Some(&emitted.join("cert")),
    )
    .expect("the engine writes a certificate");
    let derivations = emitted.join("cert").join("derivations.tsv");
    let text = std::fs::read_to_string(&derivations).expect("read derivations.tsv");
    assert!(!text.trim().is_empty(), "boundary.ttl must derive something at all");

    let steps = parse_trace(&text).expect("a derivations.tsv is a valid oo-trace/1");
    let out = scratch("selfcheck");
    let o = Opts { profile: "owl-rl-ext".to_string(), ..opts(&out) };
    let report = label_trace(&g, &steps, &o).expect("labelling runs");
    let (text, v) = as_json(&report);

    for s in v["steps"].as_array().expect("steps") {
        assert_eq!(
            s["family"], "entailed",
            "every step of the engine's own certificate must be entailed: {s}"
        );
        assert_eq!(s["rule_claim"], "licensed", "and correctly cited: {s}");
        assert_eq!(s["premises_in_store"], true, "and read only triples the store holds: {s}");
    }
    assert_eq!(v["exit_code"], 0, "a self-certificate exits clean: {text}");
}

// ── 6. A bad citation is a citation finding, never a label ───────────────────

/// **Gate.** An unknown rule name is reported against the CITATION and the step
/// is still labelled on its conclusion.
///
/// `OOCert.Parse.parseSteps` fails on an unknown rule name and `lean/Main.lean`
/// exits 2 for that, which is "a file could not be parsed" and not "the step
/// was rejected". `label_trace` bails on exit 2, so if such a step ever reached
/// the checker this test would error rather than assert, which is how it also
/// pins that it never does.
///
/// MUTATION: drop the `!known_rule(...)` guard so a bad name reaches
/// `write_one_step_certificate`, and this fails with the bail message.
#[test]
fn an_unknown_rule_name_is_a_claim_finding_and_not_a_label() {
    if skip_without("oo-cert") {
        return;
    }
    let g = store();
    let out = scratch("unknown");
    let trace = line(
        "rdfs99",
        ["<http://e/a>", TYPE, "<http://e/C>"],
        &[
            ["<http://e/a>", TYPE, "<http://e/B>"],
            ["<http://e/B>", SUBCLASS, "<http://e/C>"],
        ],
    );
    let steps = parse_trace(&trace).expect("the trace parses");
    let report = label_trace(&g, &steps, &opts(&out)).expect("labelling runs");
    let (text, v) = as_json(&report);
    let s = step_of(&v, 0);
    assert_eq!(s["rule_claim"], "rule_not_in_table", "{text}");
    assert_eq!(s["local"]["verdict"], "not_asked", "{text}");
    assert_eq!(
        s["family"], "entailed",
        "the conclusion IS entailed, and saying not_entailed for a mistyped rule name would be a \
         false statement about the ontology: {text}"
    );
    assert!(
        !text.contains("certificate_rejected"),
        "no certificate was rejected, because none was written:\n{text}"
    );
}

/// **Gate.** A chained rule cited without the RDF list it reads is a citation
/// finding, not a rejection.
///
/// `cls-int1` reads its `owl:intersectionOf` list off the asserted graph, so a
/// one-step certificate over premises carrying no `rdf:rest` would be rejected
/// for a reason that is about the citation and not about the inference.
#[test]
fn a_chained_rule_cited_without_its_list_is_a_claim_finding() {
    if skip_without("oo-cert") {
        return;
    }
    let g = store();
    let out = scratch("chained");
    let trace = line(
        "cls-int1",
        ["<http://e/a>", TYPE, "<http://e/Both>"],
        &[["<http://e/Both>", INTERSECTION_OF, "<http://e/list>"]],
    );
    let steps = parse_trace(&trace).expect("the trace parses");
    let report = label_trace(&g, &steps, &opts(&out)).expect("labelling runs");
    let (text, v) = as_json(&report);
    let s = step_of(&v, 0);
    assert_eq!(s["rule_claim"], "chained_rule_premises_incomplete", "{text}");
    assert_eq!(s["local"]["verdict"], "not_asked", "{text}");
    assert!(
        !text.contains("certificate_rejected"),
        "a citation with no list is not a rejected inference:\n{text}"
    );
}

// ── 7. The parser refuses rather than guesses ────────────────────────────────

/// **Gate.** Every malformed trace is an error naming the line, never a guess.
///
/// A step half-read still produces a label, and a label nobody wrote is worse
/// than no label.
#[test]
fn the_parser_refuses_what_it_cannot_read() {
    let good = line(
        "rdfs9",
        ["<http://e/a>", TYPE, "<http://e/C>"],
        &[["<http://e/a>", TYPE, "<http://e/B>"]],
    );
    assert_eq!(parse_trace(&good).expect("the good line parses").len(), 1);

    for (bad, wanted) in [
        ("rdfs9\t<http://e/a>\r\n", "carriage return"),
        ("rdfs9\t<http://e/a>\t<http://e/b>\n", "multiple of three"),
        ("rdfs9\t<http://e/a>\t<http://e/b>\t<http://e/c>\t<http://e/d>\n", "multiple of three"),
        ("rdfs9\t<http://e/a>\ttype\t<http://e/C>\n", "not a term in the certificate format"),
        ("", "holds no steps"),
        ("\n\n", "holds no steps"),
    ] {
        let e = parse_trace(bad).expect_err(&format!("{bad:?} must be refused")).to_string();
        assert!(e.contains(wanted), "the refusal must say {wanted:?}, got {e:?} for {bad:?}");
    }
}
