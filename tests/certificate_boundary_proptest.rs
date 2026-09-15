//! Property tests for the trusted computing base of the Lean certificate layer.
//!
//! `lean/` proves a conditional: IF the asserted graph is `G` and IF these steps
//! check, THEN the conclusions hold in every model of `G`. Everything to the
//! left of that "if" is the Rust writing down the truth. If `asserted.tsv` is
//! not the graph the engine reasoned over, or a step is not a step it took, a
//! valid Lean proof certifies a false claim.
//!
//! That boundary is small and it is enumerated in
//! `docs/trusted-computing-base.md` as `TCB-1` to `TCB-29`. This file
//! property-tests it. The identifiers in the test names are the ones in that
//! document, and a reader should be able to go from either to the other.
//!
//! # Why properties and not examples
//!
//! The certificate format is tab-separated, has no escaping layer of its own,
//! and carries arbitrary RDF literals. `lean/OOCert/Parse.lean` says so in a
//! comment:
//!
//! > Tabs and newlines cannot occur inside an N-Triples term (they are
//! > escaped), so splitting on them is exact.
//!
//! That is the entire escaping argument, it is a claim about `oxrdf`, and
//! nothing re-checks it. A hand-written test uses the terms its author thought
//! of. The generators here are built to forge a derivation step: lexical forms
//! holding tabs, newlines, carriage returns, quotes and backslashes; a literal
//! spelled exactly like an IRI; a literal spelled exactly like a whole extra
//! TSV line; combining characters against their precomposed form; the empty
//! string; NUL.
//!
//! # What needs a Lean toolchain and what does not
//!
//! The properties do not. They are about the side of the boundary the proofs
//! assume, so they must hold whether or not `lake` is installed, and they
//! re-derive what the checker would demand instead of asking it. A Lean process
//! per generated case would not be a property test in any event.
//!
//! One test at the end is the exception, and it is there because re-deriving
//! what the checker demands means transcribing it, and a transcription is an
//! assumption at exactly the boundary this file exists to check.
//! `tcb_19_21_the_real_lean_checker_accepts_an_adversarial_table` puts the
//! adversarial shapes through `oo-horn check` itself. It skips loudly without a
//! toolchain, and `OO_REQUIRE_FIXTURES=1` turns that skip into a failure.
//!
//! `tests/lean_certificate_test.rs` and `tests/lean_horn_certificate_test.rs`
//! remain the corpus-wide end-to-end gates.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{parse_rules, rules_tsv, AtomPat, InferenceTarget, Pat, Reasoner, RulePattern};
use proptest::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Generators
//
// Everything here is chosen to break a tab-separated format, not to look like
// realistic data. Realistic data is what the rest of the suite already covers.
// ─────────────────────────────────────────────────────────────────────────────

/// Lexical forms that attack the serialisation.
fn adversarial_lexical() -> impl Strategy<Value = String> {
    prop_oneof![
        // The separators themselves. If any of these reaches a certificate
        // file unescaped, a line gains a field or splits in two.
        Just("\t".to_string()),
        Just("\n".to_string()),
        Just("\r".to_string()),
        Just("\r\n".to_string()),
        // A whole forged line. If the escaping fails, this one literal becomes
        // a second `asserted.tsv` triple that the store never held.
        Just("x\n<http://e/forged>\t<http://e/p>\t<http://e/o>".to_string()),
        // A forged field. Splits one triple into four fields.
        Just("a\tb".to_string()),
        // Quote and backslash: the escape machinery's own characters.
        Just("\"".to_string()),
        Just("\\".to_string()),
        Just("\\\\".to_string()),
        // Already-escaped spellings. `"a\tb"` as six characters must not be
        // confused with `"a<TAB>b"`, in either direction.
        Just("\\t".to_string()),
        Just("\\n".to_string()),
        Just("\\u0009".to_string()),
        // A literal whose lexical form is exactly an IRI. TCB-5.
        Just("<http://e/s>".to_string()),
        // A literal that looks like a blank node, and one that looks like a
        // rule-table variable.
        Just("_:b0".to_string()),
        Just("?x".to_string()),
        // Unicode: a combining sequence against its precomposed form. Both
        // sides compare bytes, so these must stay distinct all the way through.
        Just("e\u{0301}".to_string()),
        Just("\u{00e9}".to_string()),
        // Control characters and the empty string.
        Just("\u{0000}".to_string()),
        Just("\u{007f}".to_string()),
        Just("".to_string()),
        Just(" ".to_string()),
        Just(" .".to_string()),
        Just(" . \n".to_string()),
        // Ordinary text, so the graph is not pathological in every position.
        "[a-z]{1,5}",
    ]
}

/// The path segment of a generated IRI. Restricted to characters `oxiri`
/// accepts, because `TCB_4_separators_cannot_enter_an_iri` covers the rejection
/// of the rest and a graph that never loads tests nothing.
fn iri_local() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("A".to_string()),
        Just("B".to_string()),
        Just("C".to_string()),
        Just("p".to_string()),
        Just("q".to_string()),
        Just("x".to_string()),
        // Percent-encoded tab. Legal in an IRI, and a reader that decoded
        // percent-escapes on the way into a certificate would be forging.
        Just("a%09b".to_string()),
        Just("a%0Ab".to_string()),
        // A quote and a bracket inside an IRI path.
        Just("a'b".to_string()),
        Just("~a".to_string()),
    ]
}

fn bnode_label() -> impl Strategy<Value = String> {
    prop_oneof![Just("b0".to_string()), Just("b1".to_string()), Just("x".to_string())]
}

#[derive(Clone, Debug)]
enum Term {
    Iri(String),
    Blank(String),
    Lit(String),
    LitLang(String, String),
    LitTyped(String, String),
}

/// Escape a lexical form into the N-Triples `STRING_LITERAL_QUOTE` production.
/// This is the test's own escaper, deliberately not the engine's, so a bug in
/// the engine's cannot hide behind it.
fn nt_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04X}", c as u32))
            }
            c => out.push(c),
        }
    }
    out
}

impl Term {
    fn to_nt(&self) -> String {
        match self {
            Term::Iri(l) => format!("<http://e/{l}>"),
            Term::Blank(b) => format!("_:{b}"),
            Term::Lit(s) => format!("\"{}\"", nt_escape(s)),
            Term::LitLang(s, l) => format!("\"{}\"@{}", nt_escape(s), l),
            Term::LitTyped(s, d) => format!("\"{}\"^^<http://e/{}>", nt_escape(s), d),
        }
    }
}

fn arb_object() -> impl Strategy<Value = Term> {
    prop_oneof![
        iri_local().prop_map(Term::Iri),
        bnode_label().prop_map(Term::Blank),
        adversarial_lexical().prop_map(Term::Lit),
        adversarial_lexical().prop_map(|s| Term::LitLang(s, "en-gb".to_string())),
        (adversarial_lexical(), iri_local()).prop_map(|(s, d)| Term::LitTyped(s, d)),
    ]
}

fn arb_subject() -> impl Strategy<Value = Term> {
    prop_oneof![iri_local().prop_map(Term::Iri), bnode_label().prop_map(Term::Blank)]
}

const RDF_TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const RDFS_SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const RDFS_SUBPROP: &str = "<http://www.w3.org/2000/01/rdf-schema#subPropertyOf>";
const RDFS_DOMAIN: &str = "<http://www.w3.org/2000/01/rdf-schema#domain>";
const RDFS_RANGE: &str = "<http://www.w3.org/2000/01/rdf-schema#range>";

/// Predicates are biased hard towards the vocabulary the reasoner fires on.
/// A generated graph with no schema triples produces no derivations, and a
/// certificate with no steps tests nothing about steps.
fn arb_predicate() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => prop_oneof![
            Just(RDF_TYPE.to_string()),
            Just(RDFS_SUBCLASS.to_string()),
            Just(RDFS_SUBPROP.to_string()),
            Just(RDFS_DOMAIN.to_string()),
            Just(RDFS_RANGE.to_string()),
        ],
        1 => iri_local().prop_map(|l| format!("<http://e/{l}>")),
    ]
}

/// An N-Triples document. Returned as text because loading it is the only way
/// to get terms into the store through the path a user's data takes.
fn arb_graph() -> impl Strategy<Value = String> {
    prop::collection::vec((arb_subject(), arb_predicate(), arb_object()), 0..8).prop_map(|ts| {
        let mut out = String::new();
        for (s, p, o) in ts {
            out.push_str(&s.to_nt());
            out.push(' ');
            out.push_str(&p);
            out.push(' ');
            out.push_str(&o.to_nt());
            out.push_str(" .\n");
        }
        out
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A scratch directory unique to the calling process and to a counter, because
/// proptest runs hundreds of cases and cargo runs test binaries in parallel.
fn scratch(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "oo-tcb-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The triples of a store, as the N-Triples spellings the certificate uses.
fn store_set(g: &GraphStore) -> BTreeSet<(String, String, String)> {
    g.all_triples().unwrap().into_iter().collect()
}

/// Split a TSV line, refusing to guess. Returns the fields.
fn fields(line: &str) -> Vec<&str> {
    line.split('\t').collect()
}

/// Every rule name `OOCert.Rule.name` gives, read out of the Lean source.
///
/// Read rather than duplicated: a rule renamed on one side and not the other
/// is exactly the drift this is here to catch, and a hardcoded list in this
/// file would drift with it.
fn lean_rule_names() -> BTreeSet<String> {
    let src = std::fs::read_to_string(repo().join("lean/OOCert/Rules.lean"))
        .expect("lean/OOCert/Rules.lean is tracked by this repository");
    let start = src.find("def Rule.name").expect("Rule.name is defined");
    let end = src[start..].find("def Rule.all").expect("Rule.all follows Rule.name") + start;
    let body = &src[start..end];
    let mut out = BTreeSet::new();
    let mut rest = body;
    while let Some(i) = rest.find('"') {
        rest = &rest[i + 1..];
        let Some(j) = rest.find('"') else { break };
        out.insert(rest[..j].to_string());
        rest = &rest[j + 1..];
    }
    assert!(out.len() >= 20, "parsed {} rule names out of Rules.lean", out.len());
    out
}

/// Run a certified reason over `nt`, returning the store's before and after
/// triple sets and the two certificate files.
struct Run {
    before: BTreeSet<(String, String, String)>,
    after: BTreeSet<(String, String, String)>,
    asserted: String,
    derivations: String,
    reported_inferred: usize,
    reported_asserted: usize,
    reported_derivations: usize,
}

fn certified_run(nt: &str) -> Option<Run> {
    let g = Arc::new(GraphStore::new());
    // A generated document that will not parse is not a counterexample to
    // anything here: the injection properties are about what happens AFTER a
    // term is in the store.
    g.load_ntriples(nt).ok()?;
    let before = store_set(&g);
    let dir = scratch("run");
    let out = Reasoner::run_full(
        &g,
        "owl-rl-ext",
        true,
        InferenceTarget::DefaultGraph,
        Some(&dir),
    )
    .expect("a certified run over a loaded graph does not fail");
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    let after = store_set(&g);
    let r = Run {
        before,
        after,
        asserted: std::fs::read_to_string(dir.join("asserted.tsv")).unwrap(),
        derivations: std::fs::read_to_string(dir.join("derivations.tsv")).unwrap(),
        reported_inferred: json["inferred_count"].as_u64().unwrap() as usize,
        reported_asserted: json["certificate"]["asserted"].as_u64().unwrap() as usize,
        reported_derivations: json["certificate"]["derivations"].as_u64().unwrap() as usize,
    };
    let _ = std::fs::remove_dir_all(&dir);
    Some(r)
}

// ─────────────────────────────────────────────────────────────────────────────
// TCB-1, TCB-4, TCB-5: the serialisation cannot be forged from inside a term
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// TCB-1. Every line of `asserted.tsv` is exactly three tab-separated
    /// fields, and there are as many lines as the run started with triples.
    ///
    /// This is the property a forged literal breaks. A term carrying a tab
    /// gives a line four fields; a term carrying a newline turns one line into
    /// two, and the second one is a triple nobody asserted.
    #[test]
    fn tcb_1_asserted_lines_are_exactly_three_fields(nt in arb_graph()) {
        let Some(run) = certified_run(&nt) else { return Ok(()) };
        let lines: Vec<&str> = run.asserted.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            prop_assert_eq!(
                fields(line).len(), 3,
                "asserted.tsv line {} is not three fields: {:?}", i, line
            );
        }
        prop_assert_eq!(lines.len(), run.reported_asserted);
        prop_assert_eq!(lines.len(), run.before.len());
        // A file that does not end in a newline would merge with anything
        // appended, and the Lean parser splits on "\n".
        prop_assert!(run.asserted.is_empty() || run.asserted.ends_with('\n'));
    }

    /// TCB-4. No term written to a certificate file contains a field or record
    /// separator. TCB-1 to TCB-3 detect the consequence; this states the cause,
    /// because it is the one that has to survive an `oxrdf` upgrade.
    #[test]
    fn tcb_4_no_term_carries_a_separator(nt in arb_graph()) {
        let Some(run) = certified_run(&nt) else { return Ok(()) };
        for line in run.asserted.lines().chain(run.derivations.lines()) {
            for f in fields(line) {
                prop_assert!(
                    !f.contains('\t') && !f.contains('\n') && !f.contains('\r'),
                    "field carries a separator: {:?}", f
                );
            }
        }
    }

    /// TCB-5. A literal and an IRI never have the same spelling, so the
    /// checkers, which compare terms as opaque strings, cannot confuse them.
    #[test]
    fn tcb_5_literal_and_iri_spellings_are_disjoint(nt in arb_graph()) {
        let Some(run) = certified_run(&nt) else { return Ok(()) };
        for line in run.asserted.lines() {
            for f in fields(line) {
                let looks_iri = f.starts_with('<') && f.ends_with('>');
                let looks_lit = f.starts_with('"');
                prop_assert!(
                    !(looks_iri && looks_lit),
                    "term is both an IRI and a literal: {:?}", f
                );
                prop_assert!(
                    looks_iri || looks_lit || f.starts_with("_:"),
                    "term is in no N-Triples shape: {:?}", f
                );
            }
        }
    }
}

/// TCB-4, the other half. A separator cannot get INTO an IRI, because the
/// parser refuses one. Not a property test: the point is the exact rejection,
/// and there are only a handful of ways to write the attack.
///
/// This pins a guarantee `oxiri` provides and this repository relies on without
/// re-checking. If an upgrade ever accepts one of these, `NamedNodeRef`'s
/// `Display` is `write!(f, "<{}>", self.as_str())` with no escaping, and the
/// character goes straight into `asserted.tsv`.
#[test]
fn tcb_4_separators_cannot_enter_an_iri() {
    let attacks = [
        ("raw tab", "<http://e/s> <http://e/p> <http://e/a\tb> .\n"),
        ("escaped tab", "<http://e/s> <http://e/p> <http://e/a\\u0009b> .\n"),
        ("escaped newline", "<http://e/s> <http://e/p> <http://e/a\\u000Ab> .\n"),
        ("escaped cr", "<http://e/s> <http://e/p> <http://e/a\\u000Db> .\n"),
        ("escaped space", "<http://e/s> <http://e/p> <http://e/a\\u0020b> .\n"),
        ("raw space", "<http://e/s> <http://e/p> <http://e/a b> .\n"),
        (
            "tab in the datatype IRI",
            "<http://e/s> <http://e/p> \"v\"^^<http://e/a\\u0009b> .\n",
        ),
    ];
    for (what, nt) in attacks {
        let g = GraphStore::new();
        let r = g.load_ntriples(nt);
        assert!(
            r.is_err(),
            "the parser accepted {what}, so a separator can reach an IRI and \
             `asserted.tsv` can be forged from inside a term"
        );
    }
}

/// TCB-4, the third way in. Turtle's long-string form carries RAW control
/// characters, so it is the realistic route to a tab inside a literal: no
/// escape sequence is involved on the way in, and the engine's own serialiser
/// is the only thing that puts one back.
#[test]
fn tcb_4_raw_control_characters_from_turtle_are_escaped_on_the_way_out() {
    let ttl = "<http://e/s> <http://e/p> \"\"\"a\tb\nc\rd\"\"\" .\n";
    let g = Arc::new(GraphStore::new());
    g.load_turtle(ttl, None).expect("Turtle long strings carry raw control characters");
    let dir = scratch("rawctl");
    Reasoner::run_full(&g, "owl-rl-ext", false, InferenceTarget::DefaultGraph, Some(&dir)).unwrap();
    let asserted = std::fs::read_to_string(dir.join("asserted.tsv")).unwrap();
    assert_eq!(
        asserted.lines().count(),
        1,
        "a raw tab and newline in a literal split the line: {asserted:?}"
    );
    assert_eq!(fields(asserted.lines().next().unwrap()).len(), 3, "{asserted:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

// ─────────────────────────────────────────────────────────────────────────────
// TCB-6, TCB-7: `asserted.tsv` is the graph
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

    /// TCB-6 and TCB-7. `asserted.tsv` round-trips: reading it back into a
    /// fresh store gives exactly the graph the run started from. Nothing
    /// dropped, nothing added.
    ///
    /// The re-read goes through the N-Triples parser, a different code path
    /// from the one that wrote the file, so a term that serialises to something
    /// unreadable fails here rather than at the checker.
    #[test]
    fn tcb_6_7_asserted_round_trips_to_the_same_graph(nt in arb_graph()) {
        let Some(run) = certified_run(&nt) else { return Ok(()) };
        let mut back = String::new();
        for line in run.asserted.lines() {
            let f = fields(line);
            prop_assert_eq!(f.len(), 3, "{:?}", line);
            back.push_str(f[0]);
            back.push(' ');
            back.push_str(f[1]);
            back.push(' ');
            back.push_str(f[2]);
            back.push_str(" .\n");
        }
        let fresh = GraphStore::new();
        fresh.load_ntriples(&back).map_err(|e| TestCaseError::fail(format!(
            "asserted.tsv does not read back as N-Triples: {e}"
        )))?;
        prop_assert_eq!(store_set(&fresh), run.before.clone());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TCB-2, TCB-8, TCB-10, TCB-11, TCB-12, TCB-13: the derivation record
// ─────────────────────────────────────────────────────────────────────────────

/// One parsed `derivations.tsv` line.
struct Step {
    rule: String,
    conclusion: (String, String, String),
    premises: Vec<(String, String, String)>,
}

fn parse_derivations(text: &str) -> Result<Vec<Step>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let f = fields(line);
        if f.len() < 7 || !(f.len() - 1).is_multiple_of(3) {
            return Err(format!("line {n}: {} fields: {line:?}", f.len()));
        }
        let mut triples = Vec::new();
        for c in f[1..].chunks(3) {
            triples.push((c[0].to_string(), c[1].to_string(), c[2].to_string()));
        }
        let conclusion = triples.remove(0);
        out.push(Step { rule: f[0].to_string(), conclusion, premises: triples });
    }
    Ok(out)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

    /// TCB-2. `derivations.tsv` lines are `1 + 3 * (1 + premises)` fields with
    /// at least one premise. This is the shape `OOCert.Parse.parseSteps`
    /// demands, re-derived here rather than asked of the checker.
    #[test]
    fn tcb_2_derivation_lines_have_the_shape_the_parser_demands(nt in arb_graph()) {
        let Some(run) = certified_run(&nt) else { return Ok(()) };
        let steps = parse_derivations(&run.derivations)
            .map_err(TestCaseError::fail)?;
        for s in &steps {
            prop_assert!(!s.premises.is_empty(), "rule {} emitted no premise", s.rule);
        }
        prop_assert!(run.derivations.is_empty() || run.derivations.ends_with('\n'));
    }

    /// TCB-10 and TCB-11. The conclusions are exactly the triples the run added
    /// to the store, one line each.
    ///
    /// The unsafe direction is a triple in the store with no line covering it:
    /// an uncertified inference sitting under the certificate's cover, which a
    /// consumer reading `{"ok": true}` has no way to see.
    #[test]
    fn tcb_10_11_every_inference_is_certified_exactly_once(nt in arb_graph()) {
        let Some(run) = certified_run(&nt) else { return Ok(()) };
        let steps = parse_derivations(&run.derivations).map_err(TestCaseError::fail)?;
        let inferred: BTreeSet<_> = run.after.difference(&run.before).cloned().collect();
        let concluded: BTreeSet<_> = steps.iter().map(|s| s.conclusion.clone()).collect();
        prop_assert_eq!(&concluded, &inferred, "certified set is not the inferred set");
        // One LINE per inference, not merely one distinct conclusion.
        prop_assert_eq!(steps.len(), inferred.len(), "a conclusion was certified twice");
        prop_assert_eq!(steps.len(), run.reported_derivations);
        prop_assert_eq!(inferred.len(), run.reported_inferred);
    }

    /// TCB-8. No conclusion appears in `asserted.tsv`. Within a run the engine
    /// captures the assertions before it materialises, so an inference cannot
    /// be read back as an axiom.
    ///
    /// ACROSS runs this does not hold, and no test here can make it: see
    /// `docs/trusted-computing-base.md` TCB-8 and decision 0001.
    #[test]
    fn tcb_8_no_inference_appears_among_the_assertions(nt in arb_graph()) {
        let Some(run) = certified_run(&nt) else { return Ok(()) };
        let steps = parse_derivations(&run.derivations).map_err(TestCaseError::fail)?;
        let mut asserted = BTreeSet::new();
        for line in run.asserted.lines() {
            let f = fields(line);
            asserted.insert((f[0].to_string(), f[1].to_string(), f[2].to_string()));
        }
        for s in &steps {
            prop_assert!(
                !asserted.contains(&s.conclusion),
                "rule {} concluded a triple that asserted.tsv also claims: {:?}",
                s.rule, s.conclusion
            );
        }
    }

    /// TCB-12. Every premise is asserted or was concluded by an EARLIER line,
    /// so no step cites itself and no step cites a triple from the future.
    /// `OOCert.checkAll` demands this; the emitter has to supply it.
    #[test]
    fn tcb_12_premises_precede_their_conclusions(nt in arb_graph()) {
        let Some(run) = certified_run(&nt) else { return Ok(()) };
        let steps = parse_derivations(&run.derivations).map_err(TestCaseError::fail)?;
        let mut known: BTreeSet<(String, String, String)> = BTreeSet::new();
        for line in run.asserted.lines() {
            let f = fields(line);
            known.insert((f[0].to_string(), f[1].to_string(), f[2].to_string()));
        }
        for (i, s) in steps.iter().enumerate() {
            for p in &s.premises {
                prop_assert!(
                    known.contains(p),
                    "step {i} ({}) cites a premise that is neither asserted nor concluded \
                     earlier: {:?}", s.rule, p
                );
            }
            known.insert(s.conclusion.clone());
        }
    }

    /// TCB-13. Every rule name is one `OOCert.Rule.ofName?` accepts. The list is
    /// read out of `lean/OOCert/Rules.lean` at test time, so a rename on one
    /// side fails here.
    #[test]
    fn tcb_13_rule_names_are_names_the_checker_knows(nt in arb_graph()) {
        let Some(run) = certified_run(&nt) else { return Ok(()) };
        let known = lean_rule_names();
        let steps = parse_derivations(&run.derivations).map_err(TestCaseError::fail)?;
        for s in &steps {
            prop_assert!(known.contains(&s.rule), "unknown rule name {:?}", s.rule);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TCB-19, TCB-20, TCB-21, TCB-22: the rule table
// ─────────────────────────────────────────────────────────────────────────────

/// A faithful transcription of `lean/OOCert/HornParse.lean`.
///
/// This is a model, not the checker. It exists so a generated table can be run
/// through BOTH parsers in a property test without a Lean toolchain in the
/// loop. Its fidelity is the assumption; `tests/lean_horn_certificate_test.rs`
/// pins the real thing on the built-in table.
mod lean_model {
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Pat {
        Const(String),
        Var(String),
    }

    /// `patOf`: any field starting with `?` is a variable, everything else a
    /// constant. Note it is strictly more permissive than the Rust parser.
    pub fn pat_of(s: &str) -> Pat {
        if s.starts_with('?') {
            Pat::Var(s.chars().skip(1).collect())
        } else {
            Pat::Const(s.to_string())
        }
    }

    pub fn pat_str(p: &Pat) -> String {
        match p {
            Pat::Const(c) => c.clone(),
            Pat::Var(v) => format!("?{v}"),
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Rule {
        pub name: String,
        pub body: Vec<[Pat; 3]>,
        pub head: [Pat; 3],
    }

    /// `ruleStr`: `String.intercalate "\t" ([name, len] ++ body ++ head)`.
    pub fn rule_str(r: &Rule) -> String {
        let mut parts: Vec<String> = vec![r.name.clone(), r.body.len().to_string()];
        for a in r.body.iter().chain(std::iter::once(&r.head)) {
            for p in a {
                parts.push(pat_str(p));
            }
        }
        parts.join("\t")
    }

    fn group_atoms(fs: &[&str]) -> Option<Vec<[Pat; 3]>> {
        if !fs.len().is_multiple_of(3) {
            return None;
        }
        Some(fs.chunks(3).map(|c| [pat_of(c[0]), pat_of(c[1]), pat_of(c[2])]).collect())
    }

    /// `parseRules`. `k.toNat?` is Lean's arbitrary-precision Nat parse, so a
    /// number too large for `usize` still parses there and then fails the field
    /// count; that is modelled with `u128` and a saturating comparison.
    pub fn parse_rules(content: &str) -> Result<Vec<Rule>, String> {
        let mut out = Vec::new();
        for (i, line) in content.split('\n').enumerate() {
            let n = i + 1;
            if line.is_empty() {
                continue;
            }
            let fs: Vec<&str> = line.split('\t').collect();
            if fs.len() < 2 {
                return Err(format!("rules line {n}: expected name, body length, patterns"));
            }
            let (name, k, rest) = (fs[0], fs[1], &fs[2..]);
            if k.is_empty() || !k.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!("rules line {n}: body length is not a number"));
            }
            let m: u128 = k.parse().map_err(|_| format!("rules line {n}: unrepresentable"))?;
            let want = m.saturating_mul(3).saturating_add(3);
            if rest.len() as u128 != want {
                return Err(format!("rules line {n}: expected {want} pattern fields"));
            }
            let m = m as usize;
            let (Some(body), Some(head)) = (group_atoms(&rest[..3 * m]), group_atoms(&rest[3 * m..]))
            else {
                return Err(format!("rules line {n}: malformed patterns"));
            };
            let [head] = <[_; 1]>::try_from(head).map_err(|_| "head".to_string())?;
            out.push(Rule { name: name.to_string(), body, head });
        }
        Ok(out)
    }
}

fn to_model(r: &RulePattern) -> lean_model::Rule {
    fn p(p: &Pat) -> lean_model::Pat {
        match p {
            Pat::Const(c) => lean_model::Pat::Const(c.clone()),
            Pat::Var(v) => lean_model::Pat::Var(v.clone()),
        }
    }
    fn a(a: &AtomPat) -> [lean_model::Pat; 3] {
        [p(&a.s), p(&a.p), p(&a.o)]
    }
    lean_model::Rule {
        name: r.name.clone(),
        body: r.body.iter().map(a).collect(),
        head: a(&r.head),
    }
}

/// Fields for a rule-table position. Adversarial in the directions that matter:
/// a constant that could be read as a variable, a variable whose name starts
/// with `?`, a term that is not in N-Triples spelling at all.
fn arb_rule_field() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("?x".to_string()),
        Just("?y".to_string()),
        Just("??x".to_string()),
        Just("?".to_string()),
        Just("<http://e/A>".to_string()),
        Just("<http://e/B>".to_string()),
        Just(RDF_TYPE.to_string()),
        Just(RDFS_SUBCLASS.to_string()),
        Just("_:b0".to_string()),
        Just("\"lit\"".to_string()),
        Just("\"?x\"".to_string()),
        Just("\"<http://e/A>\"".to_string()),
        Just("".to_string()),
        Just("bare".to_string()),
        Just("5".to_string()),
    ]
}

fn arb_rule_line() -> impl Strategy<Value = String> {
    (
        prop_oneof![Just("r".to_string()), Just("5".to_string()), Just("".to_string()), Just("r\u{00e9}".to_string())],
        0usize..3,
        prop::collection::vec(arb_rule_field(), 3..10),
    )
        .prop_map(|(name, m, mut fs)| {
            // Usually make the field count agree with the declared body length,
            // so the parser gets past its arity check often enough to test the
            // interesting part.
            fs.resize(3 * m + 3, "?x".to_string());
            let mut parts = vec![name, m.to_string()];
            parts.extend(fs);
            parts.join("\t")
        })
}

fn arb_rule_table() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_rule_line(), 0..4).prop_map(|ls| {
        let mut s = ls.join("\n");
        if !s.is_empty() {
            s.push('\n');
        }
        s
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    /// TCB-20. `parse_rules` and `rules_tsv` are inverse on everything
    /// `parse_rules` accepts. In particular a constant never renders to
    /// something that reads back as a variable.
    ///
    /// This is what makes the re-parse guard inside `run_horn` meaningful: the
    /// engine writes `rules_tsv(&rules)` and checks it reads back as `rules`,
    /// and that check is only as good as the round trip being total rather than
    /// accidental on the tables anyone has tried.
    #[test]
    fn tcb_20_render_and_parse_are_inverse(text in arb_rule_table()) {
        let Ok(rules) = parse_rules(&text) else { return Ok(()) };
        let rendered = rules_tsv(&rules);
        let reparsed = parse_rules(&rendered)
            .map_err(|e| TestCaseError::fail(format!("rendered table does not parse: {e}")))?;
        prop_assert_eq!(reparsed, rules, "render/parse is not the identity on {:?}", rendered);
        // Idempotent, so a table that has been through the engine once is a
        // fixed point and its SHA-256 is stable.
        prop_assert_eq!(rules_tsv(&parse_rules(&rendered).unwrap()), rendered);
    }

    /// TCB-21. `rules_tsv` is byte-identical, per line, to
    /// `OOCert.HornParse.ruleStr` as transcribed in `lean_model`. The engine
    /// publishes a SHA-256 of its own rendering and `oo-horn` decides `entailed`
    /// against `entailed_under_supplied_rules` by comparing tables, so a
    /// difference of one byte changes a verdict.
    #[test]
    fn tcb_21_rust_and_lean_render_a_table_identically(text in arb_rule_table()) {
        let Ok(rules) = parse_rules(&text) else { return Ok(()) };
        let rust = rules_tsv(&rules);
        let lean: String = rules
            .iter()
            .map(|r| lean_model::rule_str(&to_model(r)) + "\n")
            .collect();
        prop_assert_eq!(rust, lean);
    }

    /// TCB-19. Whatever the Rust accepts, the Lean parser reads as the same
    /// table. The Rust is strictly stricter, and the strictness is all in the
    /// refusing direction, so the two can only disagree by the Rust accepting
    /// something the Lean reads differently. That is the failure this looks for.
    #[test]
    fn tcb_19_rust_and_lean_parse_a_table_the_same_way(text in arb_rule_table()) {
        let Ok(rules) = parse_rules(&text) else { return Ok(()) };
        let lean = lean_model::parse_rules(&text)
            .map_err(|e| TestCaseError::fail(format!(
                "the Rust accepted a table the Lean parser refuses ({e}): {text:?}"
            )))?;
        let mine: Vec<lean_model::Rule> = rules.iter().map(to_model).collect();
        prop_assert_eq!(lean, mine);
    }

    /// TCB-22, the parsing half. Both parsers skip empty lines, so a rule's
    /// index is its position in the parsed list on both sides, and
    /// `rules_tsv` never emits an empty line for that index to slip past.
    #[test]
    fn tcb_22_rendered_tables_have_no_empty_lines(text in arb_rule_table()) {
        let Ok(rules) = parse_rules(&text) else { return Ok(()) };
        let rendered = rules_tsv(&rules);
        prop_assert_eq!(rendered.lines().count(), rules.len());
        for line in rendered.lines() {
            prop_assert!(!line.is_empty());
            prop_assert!(!line.contains('\r'));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TCB-3, TCB-22, TCB-23, TCB-24: the Horn certificate
// ─────────────────────────────────────────────────────────────────────────────

/// A rule table that `parse_rules` will accept, built rather than filtered:
/// every head position is a constant or a variable the body binds.
fn arb_valid_rule_table() -> impl Strategy<Value = String> {
    let atom = prop_oneof![
        Just(["?x".to_string(), RDF_TYPE.to_string(), "?c".to_string()]),
        Just(["?c".to_string(), RDFS_SUBCLASS.to_string(), "?d".to_string()]),
        Just(["?x".to_string(), "?p".to_string(), "?y".to_string()]),
        Just(["?p".to_string(), RDFS_DOMAIN.to_string(), "?c".to_string()]),
        Just(["?x".to_string(), RDF_TYPE.to_string(), "<http://e/A>".to_string()]),
    ];
    let head = prop_oneof![
        Just(["?x".to_string(), RDF_TYPE.to_string(), "<http://e/B>".to_string()]),
        Just(["<http://e/k>".to_string(), RDF_TYPE.to_string(), "<http://e/B>".to_string()]),
        // A head that puts a literal in subject position when `?c` binds one.
        // `run_horn` must refuse to write it rather than emit an unserialisable
        // triple, and must not use it as a premise for anything.
        Just(["?c".to_string(), RDF_TYPE.to_string(), "<http://e/C>".to_string()]),
    ];
    prop::collection::vec((prop::collection::vec(atom, 1..3), head), 1..3).prop_map(|rs| {
        let mut out = String::new();
        for (i, (body, head)) in rs.into_iter().enumerate() {
            // Keep only the head variables the body binds: the engine refuses a
            // table that does not, and a generator that produced them would
            // spend every case in the error path.
            let bound: BTreeSet<String> = body
                .iter()
                .flatten()
                .filter(|f| f.starts_with('?'))
                .cloned()
                .collect();
            if head.iter().any(|f| f.starts_with('?') && !bound.contains(f)) {
                continue;
            }
            let mut parts = vec![format!("r{i}"), body.len().to_string()];
            for a in body.iter().chain(std::iter::once(&head)) {
                parts.extend(a.iter().cloned());
            }
            out.push_str(&parts.join("\t"));
            out.push('\n');
        }
        if out.is_empty() {
            out.push_str(&format!("r0\t1\t?x\t{RDF_TYPE}\t?c\t?x\t{RDF_TYPE}\t<http://e/B>\n"));
        }
        out
    })
}

/// A `horn.tsv` line, parsed against the table it cites.
struct HornStep {
    rule: usize,
    binds: Vec<(String, String)>,
    conclusion: (String, String, String),
    premises: Vec<(String, String, String)>,
}

fn parse_horn(text: &str) -> Result<Vec<HornStep>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let f = fields(line);
        if f.len() < 2 {
            return Err(format!("horn line {n}: {} fields", f.len()));
        }
        let rule: usize = f[0].parse().map_err(|_| format!("horn line {n}: rule index"))?;
        let k: usize = f[1].parse().map_err(|_| format!("horn line {n}: bind count"))?;
        if f.len() < 2 + 2 * k + 3 {
            return Err(format!("horn line {n}: too short for {k} bindings and a conclusion"));
        }
        let binds: Vec<(String, String)> = f[2..2 + 2 * k]
            .chunks(2)
            .map(|c| (c[0].to_string(), c[1].to_string()))
            .collect();
        let rest = &f[2 + 2 * k..];
        if !rest.len().is_multiple_of(3) {
            return Err(format!("horn line {n}: {} trailing fields", rest.len()));
        }
        let mut triples: Vec<(String, String, String)> = rest
            .chunks(3)
            .map(|c| (c[0].to_string(), c[1].to_string(), c[2].to_string()))
            .collect();
        let conclusion = triples.remove(0);
        out.push(HornStep { rule, binds, conclusion, premises: triples });
    }
    Ok(out)
}

/// Apply a written binding to a pattern, the way `OOCert.AtomPat.inst` does.
fn inst(p: &Pat, binds: &[(String, String)]) -> Option<String> {
    match p {
        Pat::Const(c) => Some(c.clone()),
        Pat::Var(v) => binds.iter().find(|(n, _)| n == v).map(|(_, t)| t.clone()),
    }
}

fn inst_atom(a: &AtomPat, binds: &[(String, String)]) -> Option<(String, String, String)> {
    Some((inst(&a.s, binds)?, inst(&a.p, binds)?, inst(&a.o, binds)?))
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// TCB-3, TCB-22, TCB-23 and TCB-24 in one pass over a real `run_horn`.
    ///
    /// The substitution is re-applied here from `rules.tsv` and the written
    /// binding, independently of the engine. That is the whole point: if the
    /// emitter's premises are not the body under its own binding, the checker
    /// rejects and nobody learns why, and if they agree by construction in both
    /// places the agreement is worth nothing.
    #[test]
    fn tcb_3_22_23_24_horn_certificate_is_self_consistent(
        nt in arb_graph(),
        table in arb_valid_rule_table(),
    ) {
        let g = Arc::new(GraphStore::new());
        let Ok(_) = g.load_ntriples(&nt) else { return Ok(()) };
        let before = store_set(&g);

        let dir = scratch("horn");
        let rules_path = dir.join("input-rules.tsv");
        std::fs::write(&rules_path, &table).unwrap();
        let cert = dir.join("cert");
        let Ok(_) = Reasoner::run_horn(&g, &rules_path, &cert) else {
            let _ = std::fs::remove_dir_all(&dir);
            return Ok(());
        };

        let written = std::fs::read_to_string(cert.join("rules.tsv")).unwrap();
        let asserted = std::fs::read_to_string(cert.join("asserted.tsv")).unwrap();
        let horn = std::fs::read_to_string(cert.join("horn.tsv")).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        // TCB-24: nothing is materialised under a supplied table.
        prop_assert_eq!(store_set(&g), before.clone(), "run_horn wrote into the store");

        // TCB-18/20: what is checked is what was evaluated.
        let rules = parse_rules(&written).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(rules_tsv(&rules), written.clone());

        let steps = parse_horn(&horn).map_err(TestCaseError::fail)?;

        // TCB-3: field shape, checked against the cited rule's arity.
        for (i, line) in horn.lines().enumerate() {
            let s = &steps[i];
            // TCB-22: the index names a rule in the table that was written.
            prop_assert!(s.rule < rules.len(), "step {i} cites rule {} of {}", s.rule, rules.len());
            let r = &rules[s.rule];
            let want = 2 + 2 * s.binds.len() + 3 * (1 + r.body.len());
            prop_assert_eq!(fields(line).len(), want, "step {} shape: {:?}", i, line);
        }

        // TCB-4 again, on the Horn files.
        for line in asserted.lines().chain(horn.lines()).chain(written.lines()) {
            for f in fields(line) {
                prop_assert!(!f.contains('\n') && !f.contains('\r'), "separator in {:?}", f);
            }
        }

        let mut known: BTreeSet<(String, String, String)> = BTreeSet::new();
        for line in asserted.lines() {
            let f = fields(line);
            prop_assert_eq!(f.len(), 3, "{:?}", line);
            known.insert((f[0].to_string(), f[1].to_string(), f[2].to_string()));
        }
        prop_assert_eq!(known.len(), before.len());
        prop_assert_eq!(&known, &before);

        for (i, s) in steps.iter().enumerate() {
            let r = &rules[s.rule];
            // TCB-23: the binding names every variable of the rule, once.
            let names: BTreeSet<&String> = s.binds.iter().map(|(n, _)| n).collect();
            prop_assert_eq!(names.len(), s.binds.len(), "step {} repeats a variable", i);

            // TCB-23: premises are the body under the written binding, in order.
            let want: Option<Vec<_>> = r.body.iter().map(|a| inst_atom(a, &s.binds)).collect();
            let want = want.ok_or_else(|| TestCaseError::fail(format!(
                "step {i} leaves a body variable unbound"
            )))?;
            prop_assert_eq!(&s.premises, &want, "step {} premises are not the body instance", i);

            // TCB-23: the conclusion is the head under the same binding.
            let head = inst_atom(&r.head, &s.binds).ok_or_else(|| TestCaseError::fail(
                format!("step {i} leaves a head variable unbound")
            ))?;
            prop_assert_eq!(&s.conclusion, &head, "step {} conclusion is not the head instance", i);

            // TCB-12 for the Horn path.
            for p in &s.premises {
                prop_assert!(known.contains(p), "step {i} cites an unknown premise {p:?}");
            }
            known.insert(s.conclusion.clone());
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TCB-9: the store is a set and the file is a list
// ─────────────────────────────────────────────────────────────────────────────

/// A triple asserted in two named graphs is written to `asserted.tsv` twice,
/// because `all_triples` flattens quads. Harmless to soundness, because the
/// Lean side builds a `HashSet`, and stated so that `asserted` in the JSON is
/// read as a line count and not a triple count.
#[test]
fn tcb_9_a_quad_in_two_graphs_is_two_lines() {
    let g = Arc::new(GraphStore::new());
    g.load_nquads(
        "<http://e/s> <http://e/p> <http://e/o> <http://e/g1> .\n\
         <http://e/s> <http://e/p> <http://e/o> <http://e/g2> .\n",
    )
    .unwrap();
    let dir = scratch("quads");
    let out =
        Reasoner::run_full(&g, "rdfs", false, InferenceTarget::DefaultGraph, Some(&dir)).unwrap();
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    let asserted = std::fs::read_to_string(dir.join("asserted.tsv")).unwrap();
    assert_eq!(asserted.lines().count(), 2, "{asserted:?}");
    assert_eq!(json["certificate"]["asserted"], 2);
    let distinct: BTreeSet<&str> = asserted.lines().collect();
    assert_eq!(distinct.len(), 1, "the two lines are the same triple");
    let _ = std::fs::remove_dir_all(&dir);
}

// ─────────────────────────────────────────────────────────────────────────────
// TCB-8 across runs: what was fixed, and what is irreducible
// ─────────────────────────────────────────────────────────────────────────────

/// A second certified run does not read the first run's conclusions back as
/// assertions, when the caller asked for them to be kept apart.
///
/// **This test used to assert the DEFECT.** `GraphStore::all_triples` read
/// every named graph, so the inferences `inference_graph: true` parked in
/// `https://open-ontologies.org/graph/inferred` came back in the next run's
/// `asserted.tsv` as axioms, with no column saying they were derived. The
/// separation protected `save` and not the certificate, and the second half of
/// this test asserted that it did not work, so that the day it changed the
/// documentation would be forced to change with it. It changed on 15 September
/// 2026: the certified paths read `triples_outside(&[INFERRED_GRAPH])` and the
/// certificate reports the graphs it read. The assertion is now the property.
///
/// The FIRST half still asserts the leak, because that half is not a defect and
/// cannot be fixed here. `InferenceTarget::DefaultGraph` merges conclusions
/// into the default graph beside the assertions, which is what the caller asked
/// for, and after that nothing distinguishes them. That is the irreducible
/// half of TCB-8 and it is why the fix is a fix of the named-graph path.
#[test]
fn tcb_8_across_runs_only_the_default_graph_leaks() {
    let ttl = "<http://e/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://e/A> .\n\
               <http://e/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://e/B> .\n";

    // Default graph: the inference comes back as an assertion, because the
    // caller asked for it to be merged and the merge is lossy.
    let g = Arc::new(GraphStore::new());
    g.load_ntriples(ttl).unwrap();
    let d1 = scratch("leak1");
    Reasoner::run_full(&g, "rdfs", true, InferenceTarget::DefaultGraph, Some(&d1)).unwrap();
    let concluded: BTreeSet<String> = std::fs::read_to_string(d1.join("derivations.tsv"))
        .unwrap()
        .lines()
        .map(|l| fields(l)[1..4].join("\t"))
        .collect();
    assert!(!concluded.is_empty(), "the fixture must infer something");
    let d2 = scratch("leak2");
    Reasoner::run_full(&g, "rdfs", true, InferenceTarget::DefaultGraph, Some(&d2)).unwrap();
    let asserted2: BTreeSet<String> =
        std::fs::read_to_string(d2.join("asserted.tsv")).unwrap().lines().map(str::to_string).collect();
    assert!(
        concluded.iter().any(|c| asserted2.contains(c)),
        "the default-graph limitation is gone; docs/lean-certificates.md and \
         docs/trusted-computing-base.md TCB-8 must be corrected"
    );

    // The named inference graph DOES fix it. Not one conclusion of the first
    // run appears among the second run's assertions, and the certificate says
    // which graphs it read.
    let h = Arc::new(GraphStore::new());
    h.load_ntriples(ttl).unwrap();
    let d3 = scratch("leak3");
    let r3 = Reasoner::run_full(&h, "rdfs", true, InferenceTarget::Inferred, Some(&d3)).unwrap();
    let j3: serde_json::Value = serde_json::from_str(&r3).unwrap();
    assert!(j3["inferred_count"].as_u64().unwrap() > 0, "run 1 must infer something");
    let d4 = scratch("leak4");
    let r4 = Reasoner::run_full(&h, "rdfs", true, InferenceTarget::Inferred, Some(&d4)).unwrap();
    let j4: serde_json::Value = serde_json::from_str(&r4).unwrap();
    let asserted4: BTreeSet<String> =
        std::fs::read_to_string(d4.join("asserted.tsv")).unwrap().lines().map(str::to_string).collect();
    for c in &concluded {
        assert!(
            !asserted4.contains(c),
            "the conclusion {c:?} of an earlier run is listed as an assertion of a later one; \
             TCB-8 has regressed"
        );
    }
    // The assertions of run 2 are exactly the assertions of run 1: the store
    // grew, the asserted graph did not.
    assert_eq!(
        j4["certificate"]["asserted"], 2,
        "the second run asserted more than the two triples that were loaded"
    );
    assert_eq!(
        j4["certificate"]["graphs_excluded"][0],
        "https://open-ontologies.org/graph/inferred"
    );
    assert_eq!(j4["certificate"]["graphs_read"][0], "<default>");
    assert_eq!(
        j4["certificate"]["graphs_read"].as_array().unwrap().len(),
        1,
        "run 2 read a graph run 1 did not"
    );

    for d in [&d1, &d2, &d3, &d4] {
        let _ = std::fs::remove_dir_all(d);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TCB-13: the rule-name list is readable at all
// ─────────────────────────────────────────────────────────────────────────────

/// The extractor above is itself a parser, and a parser that silently returns
/// the empty set would make `tcb_13` vacuous.
#[test]
fn tcb_13_the_lean_rule_name_list_is_recovered() {
    let names = lean_rule_names();
    for expect in ["rdfs2", "rdfs9", "rdfs11", "prp-trp", "eq-sym", "cls-svf1", "scm-rng2"] {
        assert!(names.contains(expect), "{expect} missing from {names:?}");
    }
    assert_eq!(names.len(), 29, "the engine documents twenty-nine rule ids: {names:?}");
}

// ─────────────────────────────────────────────────────────────────────────────
// TCB-19 and TCB-21 against the REAL Lean parsers
// ─────────────────────────────────────────────────────────────────────────────

/// The property tests above run a generated table through a TRANSCRIPTION of
/// `OOCert.HornParse` written in this file, because a Lean process per case is
/// not a property test. The transcription's fidelity is then an assumption, and
/// an assumption at the trusted boundary is what this whole exercise is about.
///
/// This closes it for the adversarial shapes specifically: a literal whose
/// lexical form is an IRI, a variable name beginning with `?`, a term that is a
/// bare number, a rule name that is a number, a non-ASCII rule name, an
/// empty-bodied rule. Each is put through the engine and then through
/// `oo-horn check`, which is the real parser and the real proof.
///
/// Skips loudly without a Lean toolchain; `OO_REQUIRE_FIXTURES=1` makes the
/// skip a failure, as everywhere else in this suite.
#[test]
fn tcb_19_21_the_real_lean_checker_accepts_an_adversarial_table() {
    let lean = repo().join("lean");
    let lake_ok = std::process::Command::new("lake")
        .arg("--version")
        .current_dir(&lean)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if common::skip_unless(
        lake_ok,
        "lake (the Lean 4 build tool), to run the real oo-horn parser",
        "install elan from https://github.com/leanprover/elan",
    ) {
        return;
    }

    let build = std::process::Command::new("lake")
        .arg("build")
        .current_dir(&lean)
        .output()
        .expect("run lake build");
    assert!(
        build.status.success(),
        "lake build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let exe = lean.join(".lake").join("build").join("bin").join("oo-horn");
    assert!(exe.exists(), "oo-horn missing at {}", exe.display());

    // Every field here is one the Rust parser accepts and that could be read
    // differently by a parser that guessed.
    let table = format!(
        "{}{}{}{}",
        // A rule name that is a number, and a literal constant spelled exactly
        // like an IRI in the body.
        format_args!("5\t1\t?x\t{RDF_TYPE}\t\"<http://e/A>\"\t?x\t{RDF_TYPE}\t<http://e/N>\n"),
        // A variable whose name begins with `?`, so the field is `??x`.
        format_args!("r\u{e9}\t1\t??x\t{RDF_TYPE}\t<http://e/A>\t??x\t{RDF_TYPE}\t<http://e/B>\n"),
        // A blank node and a plain literal as constants.
        format_args!("b\t1\t?x\t{RDFS_SUBCLASS}\t\"5\"\t?x\t{RDF_TYPE}\t<http://e/C>\n"),
        // An empty body: the rule asserts its head unconditionally.
        format_args!("e\t0\t<http://e/k>\t{RDF_TYPE}\t<http://e/D>\n"),
    );

    let nt = format!(
        "<http://e/s> {RDF_TYPE} \"<http://e/A>\" .\n\
         <http://e/s> {RDF_TYPE} <http://e/A> .\n\
         <http://e/s> {RDFS_SUBCLASS} \"5\" .\n"
    );

    let g = Arc::new(GraphStore::new());
    g.load_ntriples(&nt).unwrap();
    let dir = scratch("leanhorn");
    let rules_path = dir.join("in.tsv");
    std::fs::write(&rules_path, &table).unwrap();
    let cert = dir.join("cert");
    let out = Reasoner::run_horn(&g, &rules_path, &cert)
        .expect("the engine accepts this table, so it must be able to certify over it");
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(json["derived_triples"].as_u64().unwrap() > 0, "{out}");

    let run = std::process::Command::new(&exe)
        .arg("check")
        .arg(cert.join("rules.tsv"))
        .arg(cert.join("asserted.tsv"))
        .arg(cert.join("horn.tsv"))
        .output()
        .expect("run oo-horn");
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    assert_eq!(
        run.status.code(),
        Some(0),
        "oo-horn rejected a certificate over an adversarial but legal table. \
         The Rust and the Lean parsers disagree, which is TCB-19.\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    // A user table is not the built-in one, so the verdict must be the
    // conditional one. Decision 0003.
    assert!(
        stdout.contains("entailed_under_supplied_rules"),
        "expected the conditional verdict, got {stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `docs/trusted-computing-base.md` is the deliverable the properties above are
/// named after. A property renamed without the document being updated leaves a
/// reader with no way to look one up from the other.
#[test]
fn the_trusted_base_document_names_every_property_tested_here() {
    let doc = std::fs::read_to_string(repo().join("docs/trusted-computing-base.md")).unwrap();
    let src = std::fs::read_to_string(Path::new(file!()))
        .or_else(|_| std::fs::read_to_string(repo().join("tests/certificate_boundary_proptest.rs")))
        .unwrap();
    let mut seen = BTreeSet::new();
    for fname in src.split("fn tcb_").skip(1) {
        let ids: String = fname.chars().take_while(|c| c.is_ascii_digit() || *c == '_').collect();
        for part in ids.split('_').filter(|p| !p.is_empty()) {
            if part.chars().all(|c| c.is_ascii_digit()) {
                seen.insert(part.to_string());
            }
        }
    }
    assert!(!seen.is_empty(), "no TCB identifiers found in this file");
    for id in &seen {
        assert!(
            doc.contains(&format!("TCB-{id} ")) || doc.contains(&format!("TCB-{id}.")),
            "TCB-{id} is tested but not described in docs/trusted-computing-base.md"
        );
    }
}
