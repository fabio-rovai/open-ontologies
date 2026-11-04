// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! The engine as a PRODUCER of Horn certificates.
//!
//! `lean/OOCert/Horn.lean` checks a certificate over any rule table, proved
//! once by `OOCert.horn_certificate_sound`. Until this file existed the layer
//! had a consumer and no producer: the engine could only certify its own
//! hardcoded rules, so a user with a rule table had nothing to hand the
//! checker.
//!
//! What is gated here:
//!
//!   1. A malformed rule table is refused with a reason, never guessed at.
//!   2. The table the engine writes back out is byte-identical to the one the
//!      Lean side writes, pinned against the committed built-in table.
//!   3. The emitted certificate satisfies the contract `OOCert.checkHornAll`
//!      imposes on ORDER: no step cites a premise that is neither asserted nor
//!      concluded by an earlier line, so no step can cite itself.
//!   4. Evaluating the built-in table through the generic path derives
//!      everything the hardcoded loop derives. A missing join would show here.
//!   5. `oo-horn` accepts what the engine emits, and rejects it once broken.
//!      Five forgeries, five rejections; without them the acceptance would be
//!      evidence of nothing.
//!   6. The engine states NO verdict. A certificate over the built-in table
//!      earns `entailed` and one over a user table earns
//!      `entailed_under_supplied_rules`, and `oo-horn` is the only thing that
//!      decides which. An engine that also pronounced would be a second place
//!      those two could be confused, which is the hazard decision 0003 exists
//!      to prevent.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{parse_rules, rules_tsv, InferenceTarget, Reasoner};

const PREFIXES: &str = r#"
    @prefix : <http://ex.org/> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
"#;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn lean_dir() -> PathBuf {
    repo().join("lean")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oo-horn-emit-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// One line of a rule table, assembled so no test has to type a tab.
fn rule(name: &str, body: &[[&str; 3]], head: [&str; 3]) -> String {
    let mut parts: Vec<String> = vec![name.to_string(), body.len().to_string()];
    for atom in body.iter().chain(std::iter::once(&head)) {
        for f in atom {
            parts.push((*f).to_string());
        }
    }
    parts.join("\t") + "\n"
}

fn iri(local: &str) -> String {
    format!("<http://ex.org/{local}>")
}

/// Load the Turtle, evaluate the rule table over it, and return the parsed
/// response. The rule table is written beside the certificate so a failing test
/// leaves both on disk.
fn emit(ttl: &str, rules: &str, dir: &Path) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{ttl}"), None).unwrap();
    let rules_path = dir.join("input_rules.tsv");
    std::fs::write(&rules_path, rules).unwrap();
    let out = Reasoner::run_horn(&store, &rules_path, dir)
        .unwrap_or_else(|e| panic!("run_horn failed: {e}"));
    serde_json::from_str(&out).unwrap()
}

/// Every triple in a tab-separated file, read as (s, p, o) from the given
/// field offset.
fn triples_at(line: &str, offset: usize) -> Vec<(String, String, String)> {
    let f: Vec<&str> = line.split('\t').collect();
    let mut out = Vec::new();
    let mut i = offset;
    while i + 2 < f.len() {
        out.push((f[i].to_string(), f[i + 1].to_string(), f[i + 2].to_string()));
        i += 3;
    }
    out
}

/// Split one `horn.tsv` line into (rule index, bindings, conclusion, premises).
#[allow(clippy::type_complexity)]
fn parse_horn_line(
    line: &str,
) -> (usize, Vec<(String, String)>, (String, String, String), Vec<(String, String, String)>) {
    let f: Vec<&str> = line.split('\t').collect();
    let rule: usize = f[0].parse().expect("rule index");
    let nb: usize = f[1].parse().expect("bind count");
    let mut binds = Vec::new();
    for k in 0..nb {
        binds.push((f[2 + 2 * k].to_string(), f[3 + 2 * k].to_string()));
    }
    let rest = 2 + 2 * nb;
    let conclusion = (f[rest].to_string(), f[rest + 1].to_string(), f[rest + 2].to_string());
    let premises = triples_at(line, rest + 3);
    (rule, binds, conclusion, premises)
}

fn read_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect()
}

// ── 1. A malformed table is refused, with a reason ──────────────────────────

#[test]
fn a_malformed_rule_table_is_refused_with_a_reason() {
    // Each entry is a table that must not parse, and a fragment the error has
    // to contain. Guessing at any of them would mean running a rule the user
    // did not write.
    let cases: Vec<(&str, &str, &str)> = vec![
        (
            "a body of two atoms with only one atom's worth of fields",
            "r\t2\t?s\t?p\t?o\t?s\t<http://ex.org/q>\t?o\n",
            "pattern fields",
        ),
        ("a body length that is not a number", "r\ttwo\t?s\t?p\t?o\n", "not a number"),
        (
            "a head variable the body never binds",
            "r\t1\t?s\t<http://ex.org/p>\t?o\t?s\t<http://ex.org/q>\t?z\n",
            "does not occur in the body",
        ),
        (
            "a constant that is not an N-Triples term",
            "r\t1\t?s\tex:p\t?o\t?s\t<http://ex.org/q>\t?o\n",
            "neither a variable",
        ),
        (
            "a variable with no name",
            "r\t1\t?s\t<http://ex.org/p>\t?\t?s\t<http://ex.org/q>\t?s\n",
            "no variable name",
        ),
        ("a CRLF line ending", "r\t0\t?s\t?p\t?o\r\n", "carriage return"),
        ("a line with no body length at all", "justaname\n", "expected a name"),
        ("an empty rule name", "\t0\t<http://ex.org/a>\t<http://ex.org/b>\t<http://ex.org/c>\n", "name is empty"),
    ];
    for (what, table, fragment) in cases {
        match parse_rules(table) {
            Ok(rules) => panic!("{what} parsed into {} rule(s) instead of being refused", rules.len()),
            Err(e) => assert!(
                e.to_string().contains(fragment),
                "{what}: the error must say what was wrong; wanted a mention of '{fragment}', got '{e}'"
            ),
        }
    }
}

#[test]
fn a_well_formed_table_parses() {
    // The negative test above is worthless if nothing passes.
    let table = rule(
        "grandparent",
        &[["?x", &iri("p"), "?y"], ["?y", &iri("p"), "?z"]],
        ["?x", &iri("gp"), "?z"],
    );
    let rules = parse_rules(&table).expect("a well-formed table must parse");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "grandparent");
    assert_eq!(rules[0].body.len(), 2);
}

// ── 2. The wire format is the Lean one ──────────────────────────────────────

#[test]
fn the_table_the_engine_writes_is_the_table_the_checker_reads() {
    // `tests/fixtures/horn/builtin_rules.tsv` is pinned byte for byte against
    // `oo-horn rules` by `lean_horn_certificate_test.rs`. Rendering it back out
    // through this engine and getting the same bytes is what makes the two
    // sides agree without a Lean toolchain in the loop.
    let committed = repo().join("tests").join("fixtures").join("horn").join("builtin_rules.tsv");
    if common::skip_unless(
        committed.exists(),
        "tests/fixtures/horn/builtin_rules.tsv",
        "it is committed; run `cd lean && lake exe oo-horn rules > tests/fixtures/horn/builtin_rules.tsv` to regenerate",
    ) {
        return;
    }
    let text = std::fs::read_to_string(&committed).unwrap();
    let rules = parse_rules(&text).expect("the committed built-in table must parse");
    // How many rules the built-in table holds is pinned by
    // `lean_horn_certificate_test.rs`, which owns that fixture. What matters
    // here is that this engine reads and writes the same bytes for it.
    assert!(!rules.is_empty(), "the built-in table must hold rules");
    assert_eq!(
        rules_tsv(&rules).trim_end(),
        text.trim_end(),
        "the engine's rendering of the built-in table has drifted from the committed one"
    );
}

// ── 3. Order: no step cites a premise it has not earned ─────────────────────

/// The `ancestor` rule table: recursive, so most steps cite an earlier step's
/// conclusion and the ordering contract is actually exercised.
fn ancestor_rules() -> String {
    rule("base", &[["?x", &iri("parent"), "?y"]], ["?x", &iri("anc"), "?y"])
        + &rule(
            "step",
            &[["?x", &iri("parent"), "?y"], ["?y", &iri("anc"), "?z"]],
            ["?x", &iri("anc"), "?z"],
        )
}

const CHAIN: &str = r#"
    :a :parent :b . :b :parent :c . :c :parent :d . :d :parent :e .
"#;

#[test]
fn a_recursive_table_reaches_its_fixpoint() {
    let dir = scratch("recursive");
    let r = emit(CHAIN, &ancestor_rules(), &dir);
    // Four parent edges over five nodes: the ancestor relation is every pair
    // (i, j) with i before j, which is 4 + 3 + 2 + 1 = 10.
    assert_eq!(r["derived_triples"], 10, "{r}");
    assert_eq!(r["fixpoint_reached"], true, "{r}");
    assert_eq!(r["skipped_unserialisable"], 0, "{r}");
    assert_eq!(read_lines(&dir.join("horn.tsv")).len(), 10, "one line per derived triple");
}

#[test]
fn no_step_cites_a_premise_that_is_not_already_known() {
    // This is `OOCert.checkHornAll`'s contract, checked here without Lean so a
    // break is caught by `cargo test` alone. A step whose premise is neither
    // asserted nor concluded EARLIER is rejected by the checker, and a step
    // citing its own conclusion is the special case that matters most.
    let dir = scratch("order");
    emit(CHAIN, &ancestor_rules(), &dir);
    let asserted: BTreeSet<(String, String, String)> = read_lines(&dir.join("asserted.tsv"))
        .iter()
        .map(|l| triples_at(l, 0).into_iter().next().expect("a triple per line"))
        .collect();
    // The chain is :a -> :b -> :c -> :d -> :e, so the distance of a derived
    // `anc` pair is fixed by its two ends. The engine emits round by round, and
    // round k derives exactly the distance-k pairs, so the distances down the
    // file never decrease. This is here because the premise check below can be
    // satisfied by luck: a certificate reordered by term happens to stay valid
    // when the term order agrees with the dependency order, and then a gate
    // that should have fired does not. Four subjects each have a distance-1
    // conclusion and three of them also have a longer one, so ANY grouping by
    // term forces a decrease.
    let node = |t: &str| "abcde".find(t.trim_start_matches("<http://ex.org/").trim_end_matches('>'));
    let mut previous_distance = 0usize;
    for (n, line) in read_lines(&dir.join("horn.tsv")).iter().enumerate() {
        let (_, _, (s, _, o), _) = parse_horn_line(line);
        let d = node(&o).expect("object on the chain") - node(&s).expect("subject on the chain");
        assert!(
            d >= previous_distance,
            "horn.tsv line {}: a distance-{d} conclusion after a distance-{previous_distance} one. \
             The engine emits in derivation rounds, so the file is not in the order it derived in",
            n + 1
        );
        previous_distance = d;
    }

    let mut known = asserted.clone();
    for (n, line) in read_lines(&dir.join("horn.tsv")).iter().enumerate() {
        let (_, _, conclusion, premises) = parse_horn_line(line);
        for p in &premises {
            assert!(
                known.contains(p),
                "horn.tsv line {}: premise {p:?} is neither asserted nor concluded by an earlier \
                 line, so the checker would reject the step",
                n + 1
            );
        }
        assert!(
            !premises.contains(&conclusion),
            "horn.tsv line {}: the step cites its own conclusion as a premise",
            n + 1
        );
        known.insert(conclusion);
    }
}

#[test]
fn every_binding_instantiates_the_body_and_the_head() {
    // `OOCert.checkHornStep` rejects a binding that does not instantiate the
    // body into exactly the premises listed, in the body's order. Re-running
    // the substitution here catches an emitter that writes a partial binding.
    let dir = scratch("binding");
    emit(CHAIN, &ancestor_rules(), &dir);
    let rules = parse_rules(&std::fs::read_to_string(dir.join("rules.tsv")).unwrap()).unwrap();
    for (n, line) in read_lines(&dir.join("horn.tsv")).iter().enumerate() {
        let (ri, binds, conclusion, premises) = parse_horn_line(line);
        let r = rules.get(ri).unwrap_or_else(|| panic!("line {}: rule index {ri} is off the end", n + 1));
        let lookup = |pat: &open_ontologies::reason::Pat| -> String {
            match pat {
                open_ontologies::reason::Pat::Const(c) => c.clone(),
                open_ontologies::reason::Pat::Var(v) => binds
                    .iter()
                    .find(|(name, _)| name == v)
                    .unwrap_or_else(|| panic!("line {}: variable ?{v} is unbound", n + 1))
                    .1
                    .clone(),
            }
        };
        let inst = |a: &open_ontologies::reason::AtomPat| (lookup(&a.s), lookup(&a.p), lookup(&a.o));
        assert_eq!(
            premises,
            r.body.iter().map(inst).collect::<Vec<_>>(),
            "line {}: the premises are not the body under the binding, in body order",
            n + 1
        );
        assert_eq!(conclusion, inst(&r.head), "line {}: the conclusion is not the head under the binding", n + 1);
    }
}

/// **Decision 0008, on the producer side.** No binding this engine writes has a repeated
/// key, and none omits a variable of the rule it cites. Those are the two shapes the
/// checker now refuses, so if the engine could emit either it would be a producer that
/// writes certificates its own checker rejects.
///
/// It structurally cannot, and the structure is worth naming because the run below is
/// evidence and not the argument.
///
/// * **No repeated key.** `run_horn` builds the binding from `rule.vars`, and `rule.vars`
///   is assembled with `!vars.iter().any(|x| x == v)` guarding every push. It is a
///   deduplicated list before a single binding is written, so a repeat has no way in.
/// * **No omitted variable.** The same `rule.vars` is collected over `r.atoms()`, which is
///   `body.iter().chain(once(&head))`, body AND head. So the binding is written from
///   exactly the set of variables the rule mentions, which is exactly the set the checker
///   requires, and coverage holds by construction rather than by care.
/// * **And the `expect` in between cannot fire.** `env[i].expect(..)` would panic on a
///   variable the body never bound, and `parse_rules` refuses "a head variable the body
///   never binds" before any table reaches the evaluator, and
///   `a_malformed_rule_table_is_refused_with_a_reason` is that guard, run. So every member
///   of `rule.vars` occurs in the body and `match_body` has bound it.
///
/// The run is over every rule table and graph this file has, because a structural argument
/// that nobody checks against the bytes is how a structural argument goes stale.
#[test]
fn no_emitted_binding_is_one_the_format_now_refuses() {
    let Some(builtin) = builtin_table() else { return };
    let tables: Vec<(&str, String)> = vec![
        ("ancestor", ancestor_rules()),
        ("builtin", builtin),
        ("no-body", rule("fact", &[], [&iri("a"), &iri("b"), &iri("c")])),
    ];
    let graphs: Vec<(&str, &str)> = vec![("chain", CHAIN), ("every-horn-rule", EVERY_HORN_RULE)];

    let mut lines = 0usize;
    for (tn, table) in &tables {
        for (gn, ttl) in &graphs {
            let dir = scratch(&format!("wellformed-{tn}-{gn}"));
            emit(ttl, table, &dir);
            let rules = parse_rules(&std::fs::read_to_string(dir.join("rules.tsv")).unwrap())
                .expect("the table the engine wrote must parse");
            for (n, line) in read_lines(&dir.join("horn.tsv")).iter().enumerate() {
                let (ri, binds, _conclusion, _premises) = parse_horn_line(line);
                let r = rules
                    .get(ri)
                    .unwrap_or_else(|| panic!("{tn}/{gn} line {}: rule index {ri} is off the end", n + 1));

                let keys: Vec<&str> = binds.iter().map(|(k, _)| k.as_str()).collect();
                let mut seen: BTreeSet<&str> = BTreeSet::new();
                for k in &keys {
                    assert!(
                        seen.insert(k),
                        "{tn}/{gn} line {}: the binding repeats the key '{k}'. Decision 0008 \
                         refuses that, so this engine would be writing a certificate its own \
                         checker rejects: {keys:?}",
                        n + 1
                    );
                }

                // Every variable of the rule, body AND head, exactly as `bindsCover` reads
                // `RulePattern.varList`.
                let mut wanted: BTreeSet<&str> = BTreeSet::new();
                for a in r.body.iter().chain(std::iter::once(&r.head)) {
                    for p in [&a.s, &a.p, &a.o] {
                        if let open_ontologies::reason::Pat::Var(v) = p {
                            wanted.insert(v.as_str());
                        }
                    }
                }
                let missing: Vec<&&str> = wanted.iter().filter(|v| !keys.contains(v)).collect();
                assert!(
                    missing.is_empty(),
                    "{tn}/{gn} line {}: the binding omits {missing:?}, which rule '{}' mentions. \
                     Decision 0008 refuses that",
                    n + 1,
                    r.name
                );
                lines += 1;
            }
        }
    }

    // A test that checked nothing would also pass every assertion above.
    assert!(lines >= 20, "only {lines} emitted steps were checked; the corpus has collapsed");
}

/// The other half of the producer argument, and the one a structural claim cannot make on
/// its own: a rule table whose HEAD carries a variable the body never binds is refused
/// before it is ever evaluated, so the `expect` in `run_horn`'s binding construction has
/// no reachable input. Without this, "coverage holds by construction" would rest on a
/// panic never firing rather than on a table never arriving.
#[test]
fn a_head_variable_the_body_never_binds_never_reaches_the_evaluator() {
    let unsafe_head = "r\t1\t?s\t<http://ex.org/p>\t?o\t?s\t<http://ex.org/q>\t?z\n";
    let e = parse_rules(unsafe_head).expect_err("this table must be refused, not evaluated");
    assert!(e.to_string().contains("does not occur in the body"), "{e}");

    // And it is refused through the same door `run_horn` uses, not only through the
    // function this test calls directly.
    let dir = scratch("unsafehead");
    let table = dir.join("rules.tsv");
    std::fs::write(&table, unsafe_head).unwrap();
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{CHAIN}"), None).unwrap();
    let err = Reasoner::run_horn(&store, &table, &dir.join("cert"))
        .expect_err("run_horn must refuse the table rather than run it");
    assert!(err.to_string().contains("does not occur in the body"), "{err}");
}

// ── 4. The generic path against the hardcoded one ───────────────────────────

/// One graph for every built-in rule that is a Horn rule. No `owl:intersectionOf`
/// or `owl:unionOf`: those two read an RDF list off the graph, so their premise
/// count is data rather than fixed by the rule and they are not in the Horn
/// table at all.
const EVERY_HORN_RULE: &str = r#"
    :A rdfs:subClassOf :B . :B rdfs:subClassOf :C . :a a :A .
    :p rdfs:domain :Dom ; rdfs:range :Rng . :s :p :o .
    :q rdfs:subPropertyOf :r . :r rdfs:subPropertyOf :t . :s2 :q :o2 .
    :anc a owl:TransitiveProperty . :x :anc :y . :y :anc :z .
    :near a owl:SymmetricProperty . :x2 :near :y2 .
    :parent owl:inverseOf :child . :m :parent :n . :u :child :v .
    :i owl:sameAs :j .
    :E owl:equivalentClass :F . :e a :E .
    :g owl:equivalentProperty :h . :s3 :g :o3 .
    :R owl:onProperty :pr ; owl:someValuesFrom :SV . :sv1 :pr :sv2 . :sv2 a :SV .
    :R2 owl:onProperty :pa ; owl:allValuesFrom :AV . :av1 a :R2 ; :pa :av2 .
    :R3 owl:onProperty :ph ; owl:hasValue :HV . :hv1 a :R3 . :hv2 :ph :HV .
"#;

fn builtin_table() -> Option<String> {
    let committed = repo().join("tests").join("fixtures").join("horn").join("builtin_rules.tsv");
    if common::skip_unless(
        committed.exists(),
        "tests/fixtures/horn/builtin_rules.tsv",
        "it is committed; run `cd lean && lake exe oo-horn rules > tests/fixtures/horn/builtin_rules.tsv` to regenerate",
    ) {
        return None;
    }
    Some(std::fs::read_to_string(committed).unwrap())
}

#[test]
fn the_generic_path_derives_everything_the_hardcoded_loop_derives() {
    let Some(table) = builtin_table() else { return };
    let dir = scratch("differential");

    // The hardcoded loop, with a certificate so its conclusions can be read off
    // the file rather than out of the store.
    let builtin_dir = dir.join("builtin");
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{EVERY_HORN_RULE}"), None).unwrap();
    Reasoner::run_full(&store, "owl-rl-ext", false, InferenceTarget::DefaultGraph, Some(&builtin_dir))
        .unwrap();
    let hardcoded: BTreeSet<(String, String, String)> = read_lines(&builtin_dir.join("derivations.tsv"))
        .iter()
        .map(|l| triples_at(l, 1).into_iter().next().expect("a conclusion per line"))
        .collect();

    // The same rules as data, through the generic path.
    let horn_dir = dir.join("horn");
    std::fs::create_dir_all(&horn_dir).unwrap();
    let r = emit(EVERY_HORN_RULE, &table, &horn_dir);
    let generic: BTreeSet<(String, String, String)> = read_lines(&horn_dir.join("horn.tsv"))
        .iter()
        .map(|l| parse_horn_line(l).2)
        .collect();

    assert!(!hardcoded.is_empty(), "the hardcoded loop must derive something on this graph");
    let missing: Vec<_> = hardcoded.difference(&generic).collect();
    assert!(
        missing.is_empty(),
        "the generic path missed {} triple(s) the hardcoded loop derived, which means a join is \
         wrong: {missing:?}\n{r}",
        missing.len()
    );

    // Where the two differ, the generic path derives MORE, and only in one
    // place: the hardcoded loop carries inequality guards (`a != c` in rdfs11
    // and rdfs5, `x != z` in prp-trp) that suppress a reflexive conclusion.
    // Those conclusions are licensed by the rule as written in the Lean table,
    // so the generic path draws them. Deriving more here is not unsoundness,
    // it is the guard the hardcoded loop applies and the rule does not, and
    // every one of them must be reflexive.
    for extra in generic.difference(&hardcoded) {
        assert_eq!(
            extra.0, extra.2,
            "the generic path derived {extra:?}, which the hardcoded loop did not and which is \
             not one of the reflexive conclusions its inequality guards suppress"
        );
    }
}

// ── 5. The engine states no verdict ─────────────────────────────────────────

#[test]
fn the_engine_pronounces_on_nothing() {
    // Decision 0003, rule 5. `oo-horn` decides between `entailed` and
    // `entailed_under_supplied_rules` by comparing the table it was handed
    // against the built-in one. The engine must not print either word, in any
    // field, or there are two places a reader could take a verdict from.
    let dir = scratch("verdict");
    let r = emit(CHAIN, &ancestor_rules(), &dir);
    let text = r.to_string();
    for forbidden in ["entailed", "entails", "\"verdict\"", "proved", "proven"] {
        assert!(
            !text.contains(forbidden),
            "the response contains '{forbidden}'. The engine emits a certificate and the checker \
             pronounces; a verdict word here is a second place the two could be confused:\n{text}"
        );
    }
    assert!(
        text.contains("ASSUMED and never checked"),
        "the response must say what the run is conditional on, in the response and not a \
         footnote:\n{text}"
    );
    assert_eq!(r["materialized"], false, "{r}");
}

#[test]
fn nothing_reaches_the_store() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{CHAIN}"), None).unwrap();
    let before = store.all_triples().unwrap().len();
    let dir = scratch("store");
    let rules_path = dir.join("rules_in.tsv");
    std::fs::write(&rules_path, ancestor_rules()).unwrap();
    let out = Reasoner::run_horn(&store, &rules_path, &dir).unwrap();
    let r: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(r["derived_triples"].as_u64().unwrap() > 0, "the run must do work: {r}");
    assert_eq!(
        store.all_triples().unwrap().len(),
        before,
        "a conclusion under a supplied rule table holds only in models that satisfy that table, \
         so it must not be written into the store beside the assertions"
    );
}

#[test]
fn a_head_that_cannot_be_written_is_not_derived() {
    // `?s ?p ?o` with the head putting the object in subject position: with a
    // literal object the conclusion is a triple no RDF serialiser can write.
    // The hardcoded loop guards four of its own rules this way; the generic
    // path refuses both a literal subject and a non-IRI predicate, says how
    // many it refused, and never uses one as a premise.
    let dir = scratch("literal");
    let table = rule("flip", &[["?s", &iri("p"), "?o"]], ["?o", &iri("q"), "?s"]);
    let r = emit(r#" :a :p "hello" . :a :p :b . "#, &table, &dir);
    assert_eq!(r["derived_triples"], 1, "only the IRI-object triple may be flipped: {r}");
    assert_eq!(r["skipped_unserialisable"], 1, "{r}");
    let lines = read_lines(&dir.join("horn.tsv"));
    assert_eq!(lines.len(), 1);
    assert!(
        !lines[0].contains("\"hello\"\t"),
        "a literal must never appear in subject position in the certificate: {}",
        lines[0]
    );
}

#[test]
fn a_rule_with_no_body_asserts_its_head() {
    // The boundary of the format: a body of zero atoms. `checkHornStep` maps
    // the empty body to the empty premise list and instantiates the head under
    // a binding of nothing, so the step is well formed and the head must be
    // ground. The engine emits it with a bind count of 0 and no premises, and
    // derives it once rather than once per round.
    let dir = scratch("nobody");
    let table = rule("axiom", &[], [&iri("a"), &iri("b"), &iri("c")]);
    let r = emit(CHAIN, &table, &dir);
    assert_eq!(r["derived_triples"], 1, "{r}");
    assert_eq!(r["fixpoint_reached"], true, "{r}");
    let lines = read_lines(&dir.join("horn.tsv"));
    assert_eq!(lines.len(), 1);
    let (ri, binds, conclusion, premises) = parse_horn_line(&lines[0]);
    assert_eq!(ri, 0);
    assert!(binds.is_empty(), "a rule with no body binds nothing: {:?}", binds);
    assert!(premises.is_empty(), "a rule with no body cites no premises: {premises:?}");
    assert_eq!(conclusion, (iri("a"), iri("b"), iri("c")));
    if !skip_without_lake() {
        let (code, out) = check(&dir);
        assert_eq!(code, 0, "the checker must accept a zero-body step: {out}");
    }
}

#[test]
fn a_run_that_stopped_at_the_iteration_cap_says_so() {
    // The third answer. Evaluation is naive: the transitive closure of a chain
    // of n nodes needs n rounds, and the engine stops at
    // `runtime::reasoner_max_iterations()` (64 by default). A run that stopped
    // there has NOT reached the fixpoint, and reporting `derived_triples` as
    // though it had would be a count of the closure that is not the closure.
    // Every step it did write is still a step the checker verifies.
    let dir = scratch("cap");
    let mut ttl = String::new();
    for i in 0..80 {
        ttl.push_str(&format!(":n{i} :parent :n{}.\n", i + 1));
    }
    let r = emit(&ttl, &ancestor_rules(), &dir);
    assert_eq!(r["fixpoint_reached"], false, "80 hops cannot close in 64 rounds: {r}");
    assert!(
        r["incomplete"].as_str().unwrap_or_default().contains("LOWER BOUND"),
        "a run that stopped at the cap must say the count is a lower bound: {r}"
    );
    // 81 nodes closed would be 81*80/2 = 3240 pairs; the cap stops it short.
    assert!(r["derived_triples"].as_u64().unwrap() < 3240, "{r}");
    if !skip_without_lake() {
        let (code, out) = check(&dir);
        assert_eq!(code, 0, "a truncated run still emits steps the checker accepts: {out}");
    }
}

#[test]
fn a_missing_rule_table_is_named_in_the_error() {
    let store = Arc::new(GraphStore::new());
    let dir = scratch("missing");
    let err = Reasoner::run_horn(&store, &dir.join("nowhere.tsv"), &dir).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("cannot read"), "{text}");
    assert!(text.contains("nowhere.tsv"), "the error must name the file it could not read: {text}");
}

#[test]
fn an_empty_table_is_refused() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{CHAIN}"), None).unwrap();
    let dir = scratch("empty");
    let rules_path = dir.join("empty.tsv");
    std::fs::write(&rules_path, "\n\n").unwrap();
    let err = Reasoner::run_horn(&store, &rules_path, &dir).unwrap_err();
    assert!(err.to_string().contains("no rules"), "{err}");
}

// ── 6. The CLI and the batch command ────────────────────────────────────────

fn oo() -> Command {
    Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
}

#[test]
fn the_batch_command_emits_a_certificate() {
    use std::io::Write;
    let dir = scratch("batch");
    let ttl = dir.join("chain.ttl");
    std::fs::write(&ttl, format!("{PREFIXES}{CHAIN}")).unwrap();
    let rules_path = dir.join("rules_in.tsv");
    std::fs::write(&rules_path, ancestor_rules()).unwrap();
    let cert = dir.join("cert");

    let mut child = oo()
        .args(["--no-connect", "--data-dir"])
        .arg(dir.join("store"))
        .args(["batch", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let script = format!(
        "load {}\nreason --rules {} --certificate {}\n",
        ttl.display(),
        rules_path.display(),
        cert.display()
    );
    child.stdin.take().unwrap().write_all(script.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "batch failed: {stdout}");
    assert!(stdout.contains("\"mode\":\"horn\""), "the Horn path did not run: {stdout}");
    for f in ["rules.tsv", "asserted.tsv", "horn.tsv"] {
        assert!(cert.join(f).exists(), "{f} was not written: {stdout}");
    }
    assert_eq!(read_lines(&cert.join("horn.tsv")).len(), 10, "{stdout}");
}

#[test]
fn the_cli_refuses_a_rule_table_with_nowhere_to_put_the_certificate() {
    let dir = scratch("nodir");
    let rules_path = dir.join("rules_in.tsv");
    std::fs::write(&rules_path, ancestor_rules()).unwrap();
    let out = oo()
        .args(["--data-dir"])
        .arg(dir.join("store"))
        .args(["reason", "--rules"])
        .arg(&rules_path)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !out.status.success(),
        "a run with nowhere to put the certificate must fail, not report counts nothing can \
         check: {stdout}"
    );
    assert!(stdout.contains("needs --certificate"), "{stdout}");
}

#[test]
fn the_mcp_tool_accepts_a_rule_table_and_a_certificate_directory() {
    // What this pins is the wire name of the field, which a rename would
    // otherwise break in silence: an unknown key deserialises to None and the
    // tool would quietly run the built-in profile instead. It does NOT
    // exercise the dispatch inside `onto_reason`, because the tool methods are
    // private to the crate and nothing in tests/ can call one.
    let input: open_ontologies::inputs::OntoReasonInput = serde_json::from_value(
        serde_json::json!({"rules_file": "/tmp/r.tsv", "certificate_dir": "/tmp/c"}),
    )
    .expect("the MCP input must carry a rule table");
    assert_eq!(input.rules_file.as_deref(), Some("/tmp/r.tsv"));
    assert_eq!(input.certificate_dir.as_deref(), Some("/tmp/c"));
    assert_eq!(input.materialize, None, "an unset materialize must stay unset, not default to true here");
}

// ── 7. The round trip, against the real checker ─────────────────────────────

fn lake_available() -> bool {
    Command::new("lake")
        .arg("--version")
        .current_dir(lean_dir())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn skip_without_lake() -> bool {
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

/// Run `oo-horn check` over a certificate directory. Returns (exit code, stdout).
fn check(dir: &Path) -> (i32, String) {
    let out = Command::new(checker())
        .arg("check")
        .arg(dir.join("rules.tsv"))
        .arg(dir.join("asserted.tsv"))
        .arg(dir.join("horn.tsv"))
        .output()
        .expect("run oo-horn");
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).to_string())
}

fn field<'a>(json: &'a str, key: &str) -> &'a str {
    let pat = format!("\"{key}\":\"");
    let start = json.find(&pat).unwrap_or_else(|| panic!("no {key} in {json}")) + pat.len();
    let rest = &json[start..];
    &rest[..rest.find('"').expect("unterminated")]
}

#[test]
fn the_checker_accepts_a_certificate_the_engine_emitted() {
    if skip_without_lake() {
        return;
    }
    let dir = scratch("accept");
    let r = emit(CHAIN, &ancestor_rules(), &dir);
    let (code, out) = check(&dir);
    assert_eq!(code, 0, "the checker rejected an honest certificate: {out}\n{r}");
    assert_eq!(
        field(&out, "verdict"),
        "entailed_under_supplied_rules",
        "a table the user wrote earns the relativised verdict and nothing stronger: {out}"
    );
    assert_eq!(field(&out, "theorem"), "OOCert.horn_certificate_sound", "{out}");
    assert!(field(&out, "means").contains("assumed, not checked"), "{out}");
    assert_eq!(
        out.matches("\"derivations\":10").count(),
        1,
        "the checker must see the ten steps the engine wrote: {out}"
    );
}

#[test]
fn the_built_in_table_read_back_as_data_earns_the_absolute_verdict() {
    // The strongest form of the round trip. `oo-horn rules` prints the rule
    // table `OOCert.Builtin.asHorn_sound` discharges against the semantics; the
    // engine reads it as data, evaluates it, and the certificate it emits earns
    // `entailed`, not the relativised verdict. That is the generic path
    // agreeing with the machine-checked rules, and it only works if the
    // engine's rendering of the table is byte-identical to the Lean one.
    if skip_without_lake() {
        return;
    }
    let dir = scratch("absolute");
    let out = Command::new(checker()).arg("rules").output().expect("run oo-horn rules");
    assert!(out.status.success());
    let table = String::from_utf8_lossy(&out.stdout).to_string();

    let r = emit(EVERY_HORN_RULE, &table, &dir);
    assert!(r["derived_triples"].as_u64().unwrap() > 0, "the run must derive something: {r}");
    let (code, report) = check(&dir);
    assert_eq!(code, 0, "the checker rejected the built-in-table certificate: {report}\n{r}");
    assert_eq!(
        field(&report, "verdict"),
        "entailed",
        "the built-in table is the one table that earns the absolute verdict: {report}"
    );
    assert_eq!(field(&report, "theorem"), "OOCert.entails_of_builtin_horn", "{report}");
    assert_eq!(
        field(&report, "rules_digest"),
        field(&report, "builtin_rules_digest"),
        "the table the engine wrote back out must BE the built-in table: {report}"
    );
}

#[test]
fn every_forgery_of_an_emitted_certificate_is_rejected() {
    // A gate that cannot fail is decoration. Each forgery is a different way to
    // lie about a run, applied to a certificate the checker has just accepted.
    if skip_without_lake() {
        return;
    }
    let base = scratch("forgery");
    emit(CHAIN, &ancestor_rules(), &base);
    assert_eq!(check(&base).0, 0, "the honest certificate must pass first");

    let lines = read_lines(&base.join("horn.tsv"));
    // A recursive step: rule 1, two premises, the second of them derived.
    let victim = lines
        .iter()
        .position(|l| l.starts_with("1\t"))
        .expect("the recursive rule must have fired");

    let forge = |name: &str, mutate: &dyn Fn(&mut Vec<String>)| -> (i32, String) {
        let dir = base.parent().unwrap().join(format!("forgery-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["rules.tsv", "asserted.tsv"] {
            std::fs::copy(base.join(f), dir.join(f)).unwrap();
        }
        let mut out = lines.clone();
        mutate(&mut out);
        std::fs::write(dir.join("horn.tsv"), out.join("\n") + "\n").unwrap();
        check(&dir)
    };

    type Forgery = (&'static str, Box<dyn Fn(&mut Vec<String>)>);
    let forgeries: Vec<Forgery> = vec![
        (
            // The conclusion is not the head under the binding.
            "forged conclusion",
            Box::new(move |l: &mut Vec<String>| {
                l[victim] = l[victim].replace(&iri("anc"), &iri("owns"));
            }),
        ),
        (
            // The premises are the right triples in the wrong order. Premise
            // order is part of the contract, so this is a rejection.
            "scrambled premise order",
            Box::new(move |l: &mut Vec<String>| {
                let (ri, binds, c, mut prem) = parse_horn_line(&l[victim]);
                prem.reverse();
                let mut f = vec![ri.to_string(), binds.len().to_string()];
                for (v, t) in binds {
                    f.push(v);
                    f.push(t);
                }
                for t in std::iter::once(c).chain(prem) {
                    f.push(t.0);
                    f.push(t.1);
                    f.push(t.2);
                }
                l[victim] = f.join("\t");
            }),
        ),
        (
            // A rule index off the end of the table.
            "unknown rule index",
            Box::new(move |l: &mut Vec<String>| {
                l[victim] = format!("9{}", &l[victim][1..]);
            }),
        ),
        (
            // The step that derived this step's premise is removed, so the
            // premise is cited before anything earns it.
            "premise cited before it is derived",
            Box::new(move |l: &mut Vec<String>| {
                l.remove(0);
            }),
        ),
        (
            // A binding dropped: the substitution no longer instantiates the
            // body into the premises listed.
            "incomplete binding",
            Box::new(move |l: &mut Vec<String>| {
                let (ri, binds, c, prem) = parse_horn_line(&l[victim]);
                let kept: Vec<_> = binds.into_iter().take(1).collect();
                let mut f = vec![ri.to_string(), kept.len().to_string()];
                for (v, t) in kept {
                    f.push(v);
                    f.push(t);
                }
                for t in std::iter::once(c).chain(prem) {
                    f.push(t.0);
                    f.push(t.1);
                    f.push(t.2);
                }
                l[victim] = f.join("\t");
            }),
        ),
    ];

    for (what, mutate) in forgeries {
        let (code, out) = forge(&what.replace(' ', "-"), &*mutate);
        assert_eq!(code, 1, "a certificate with a {what} must be rejected: {out}");
    }
}
