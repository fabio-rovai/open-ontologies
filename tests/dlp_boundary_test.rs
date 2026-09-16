//! The DLP boundary report, and the one distinction it exists to keep.
//!
//! `onto_rules_import` refuses a non-Horn SWRL or RIF construct by name and
//! count, so nobody gets a certificate over a rule set they did not write.
//! `onto_dlp_boundary` is the same discipline pointed the other way: it says
//! which of the axioms a user DID write the rule table can see.
//!
//! The tests are grouped:
//!
//!   1. The rule table adds up, and its figures are the engine's own rather
//!      than a second copy that can drift.
//!   2. A disjunctive axiom is reported OUTSIDE the fragment.
//!   3. An axiom INSIDE the fragment whose rule is unimplemented is reported in
//!      the other bucket, and the two are never merged. This is the test the
//!      whole tool is for: `owl:hasKey` and `owl:propertyChainAxiom` are Horn,
//!      OWL 2 RL has `prp-key` and `prp-spo2`, and this engine runs neither.
//!   4. The splitting asymmetry: a conjunction splits in a consequent and does
//!      NOT split in an antecedent, because dropping a conjunct from a rule
//!      body makes it fire more often, which is unsound rather than weaker.
//!   5. Nothing is passed over in silence: every triple lands in a bucket.

use open_ontologies::dlp::{DlpBoundary, OWL2_RL_RULES, RuleStatus, rule_status};
use open_ontologies::graph::GraphStore;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn report(ttl: &str) -> serde_json::Value {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(ttl, None).expect("turtle parses");
    let json = DlpBoundary::check(&graph).expect("the report is built");
    serde_json::from_str(&json).expect("the report is JSON")
}

const PREFIXES: &str = r#"
@prefix : <http://e/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
"#;

/// Every axiom of a given type in the report, whatever bucket it is in.
fn kind_row<'a>(v: &'a serde_json::Value, kind: &str) -> &'a serde_json::Value {
    v["by_axiom_type"]
        .as_array()
        .expect("by_axiom_type is a list")
        .iter()
        .find(|r| r["axiom_type"] == kind)
        .unwrap_or_else(|| panic!("no row for {kind} in {}", v["by_axiom_type"]))
}

fn subjects(v: &serde_json::Value, bucket: &str) -> Vec<String> {
    v[bucket]["axioms"]
        .as_array()
        .unwrap_or_else(|| panic!("{bucket} has no axiom list in {v}"))
        .iter()
        .map(|a| a["subject"].as_str().unwrap_or("").to_string())
        .collect()
}

fn outside_axioms(v: &serde_json::Value) -> Vec<String> {
    subjects(v, "outside_the_fragment")
}

fn unimplemented_axioms(v: &serde_json::Value) -> Vec<String> {
    subjects(v, "inside_the_fragment_but_a_rule_is_not_implemented")
}

// ── 1. The rule table ───────────────────────────────────────────────────────

/// **Gate.** The transcription of the OWL 2 RL inference tables adds up.
///
/// Seventy-eight rules, and the four coverage buckets partition them exactly.
/// A mistyped or duplicated row fails here rather than quietly changing a
/// figure six other files in this repository repeat.
#[test]
fn the_rule_table_adds_up() {
    assert_eq!(
        OWL2_RL_RULES.len(),
        78,
        "the OWL 2 RL profile has 78 inference rules across Tables 4 to 9"
    );
    let names: BTreeSet<&str> = OWL2_RL_RULES.iter().map(|r| r.name).collect();
    assert_eq!(names.len(), 78, "a rule name is repeated in the table");

    let evaluated = OWL2_RL_RULES
        .iter()
        .filter(|r| matches!(rule_status(r.name), RuleStatus::Evaluated(_)))
        .count();
    let clash = OWL2_RL_RULES
        .iter()
        .filter(|r| rule_status(r.name) == RuleStatus::ClashDetected)
        .count();
    let undetected = OWL2_RL_RULES
        .iter()
        .filter(|r| matches!(rule_status(r.name), RuleStatus::ClashNotDetected(_)))
        .count();
    let unimplemented = OWL2_RL_RULES
        .iter()
        .filter(|r| rule_status(r.name) == RuleStatus::NotImplemented)
        .count();

    assert_eq!(evaluated, 29, "the figure six other files state as `29 of 78`");
    assert_eq!(clash, 10, "ten of the seventeen false-concluding rules are detected");
    assert_eq!(undetected, 7, "and seven are not: reason::CLASH_RULES_NOT_DETECTED");
    assert_eq!(unimplemented, 32);
    assert_eq!(evaluated + clash + undetected + unimplemented, 78);
}

/// **Gate.** The seventeen rules this table marks as concluding `false` are the
/// seventeen `src/reason.rs` names, in a list written independently of it.
///
/// This is the cross-check that makes the transcription above evidence rather
/// than an assertion. `src/reason.rs`'s comment says seventeen and names them;
/// that list said SIXTEEN until it was checked against the W3C source. Two
/// independent transcriptions agreeing is the only thing available here, since
/// nobody on this project holds the standard in machine-readable form.
#[test]
fn the_seventeen_false_concluding_rules_agree_with_the_engine() {
    let ours: BTreeSet<&str> = OWL2_RL_RULES
        .iter()
        .filter(|r| r.concludes_false)
        .map(|r| r.name)
        .collect();
    assert_eq!(ours.len(), 17, "got {ours:?}");

    let source = std::fs::read_to_string(repo().join("src").join("reason.rs"))
        .expect("src/reason.rs must be readable");
    let block = source
        .split("Seventeen rules of the OWL 2 RL profile")
        .nth(1)
        .expect("the comment naming the seventeen must still be there")
        .split("(Seventeen is the count")
        .next()
        .expect("the comment ends before the aside about how the count was taken");
    for name in &ours {
        assert!(
            block.contains(&format!("`{name}`")),
            "src/reason.rs's own list of the seventeen does not name {name}"
        );
    }

    // And every rule the clash detector says it does NOT look for is one of the
    // seventeen. A name there that concludes a triple would be a category error.
    for (name, _) in open_ontologies::reason::CLASH_RULES_NOT_DETECTED {
        assert!(
            ours.contains(name),
            "{name} is listed as an undetected CLASH rule and does not conclude false"
        );
    }
}

/// **Gate.** `reason::RULES_EVALUATED` is the rules `src/reason.rs` emits.
///
/// The list is the source of the figure "29 of 78". A figure next to the thing
/// it measures must be derived and never typed, and a `const` is typed, so this
/// greps the file for its own `emit` calls and refuses to let the two disagree.
/// Add a rule without a line, or a line without a rule, and this fails.
#[test]
fn the_evaluated_rule_list_is_the_rules_this_file_emits() {
    let source = std::fs::read_to_string(repo().join("src").join("reason.rs"))
        .expect("src/reason.rs must be readable");

    // Two sources, because the engine names a rule in two places. Most rows
    // live in the `BUILTIN_RULES` table and fire as `Fired::Bound`; the four
    // that chain their own premises name themselves at the call site as
    // `Fired::Chained`. Reading only one of the two would silently under-count,
    // which is the failure this gate exists to prevent.
    //
    // This used to split on `emit((`, which stopped matching anything when the
    // emit closure was changed to take a `Fired`. The assertion below caught
    // that rather than letting the list quietly become empty.
    let mut emitted: BTreeSet<String> = BTreeSet::new();
    for chunk in source.split("BuiltinRule { name: \"").skip(1) {
        let Some(close) = chunk.find('"') else { continue };
        emitted.insert(chunk[..close].to_string());
    }
    let table_rows = emitted.len();
    for chunk in source.split("Fired::Chained(\"").skip(1) {
        let Some(close) = chunk.find('"') else { continue };
        emitted.insert(chunk[..close].to_string());
    }
    assert!(
        table_rows > 0,
        "the grep itself is broken: no BUILTIN_RULES row found"
    );
    assert!(
        emitted.len() > table_rows,
        "the grep itself is broken: no Fired::Chained call site found, and the \
         engine has four of them"
    );

    let declared: BTreeSet<String> = open_ontologies::reason::RULES_EVALUATED
        .iter()
        .map(|(n, _)| n.to_string())
        .collect();
    assert_eq!(
        declared, emitted,
        "reason::RULES_EVALUATED and the emit calls in src/reason.rs disagree. The emit calls \
         are the measurement and the constant is the claim"
    );
    assert_eq!(declared.len(), 29);

    // The profile column is one of the three the reasoner accepts.
    for (name, profile) in open_ontologies::reason::RULES_EVALUATED {
        assert!(
            ["rdfs", "owl-rl", "owl-rl-ext"].contains(profile),
            "{name} is gated on a profile that does not exist: {profile}"
        );
    }
}

/// Every rule name the classifier attaches to an axiom exists in the table.
///
/// A typo would silently land in `NotImplemented` and report an axiom as
/// invisible when it is not, which is the failure mode this tool is supposed
/// to be the cure for.
#[test]
fn every_rule_the_classifier_names_is_a_real_rule() {
    let ttl = format!(
        "{PREFIXES}
:A a owl:Class ; rdfs:subClassOf :B ; owl:equivalentClass :C ; owl:disjointWith :D .
:B a owl:Class . :C a owl:Class . :D a owl:Class .
:r a owl:ObjectProperty, owl:TransitiveProperty, owl:SymmetricProperty, owl:FunctionalProperty,
     owl:InverseFunctionalProperty, owl:AsymmetricProperty, owl:IrreflexiveProperty ;
   rdfs:domain :A ; rdfs:range :B ; rdfs:subPropertyOf :s ; owl:inverseOf :s ;
   owl:propertyDisjointWith :s ; owl:equivalentProperty :s ; owl:propertyChainAxiom ( :s :s ) .
:s a owl:ObjectProperty .
:A owl:hasKey ( :r ) .
:a owl:sameAs :b ; owl:differentFrom :c .
:E a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :r ; owl:allValuesFrom :B ] .
:F a owl:Class ; owl:equivalentClass [ owl:intersectionOf ( :A :B ) ] .
:G a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :r ; owl:hasValue :a ] .
"
    );
    let v = report(&ttl);
    assert!(v["axioms_classified"].as_u64().unwrap_or(0) > 15, "{v}");

    let known: BTreeSet<&str> = OWL2_RL_RULES.iter().map(|r| r.name).collect();
    let mut checked = 0usize;
    for bucket in [
        &v["inside_the_fragment_but_a_rule_is_not_implemented"]["axioms"],
        &v["outside_the_fragment"]["axioms"],
    ] {
        for a in bucket.as_array().expect("a list") {
            for r in a["rules_that_would_evaluate_it"].as_array().unwrap_or(&Vec::new()) {
                let name = r.as_str().expect("a rule name");
                assert!(known.contains(name), "{name} is not an OWL 2 RL rule");
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "no rule name was checked, so this gate proved nothing");
}

// ── 2. Outside the fragment ─────────────────────────────────────────────────

/// **Gate.** A disjunction in the consequent is reported OUTSIDE the fragment,
/// with the reason, and it is not in the unimplemented bucket.
///
/// `Adult ⊑ Man ⊔ Woman` is the textbook non-Horn axiom: a Horn clause has one
/// atom in its head. No amount of work on `src/reason.rs` makes this visible,
/// and reporting it beside `owl:hasKey` would tell a reader to file the wrong
/// bug.
#[test]
fn a_disjunctive_consequent_is_reported_outside_the_fragment() {
    let ttl = format!(
        "{PREFIXES}
:Adult a owl:Class ; rdfs:subClassOf [ owl:unionOf ( :Man :Woman ) ] .
:Man a owl:Class . :Woman a owl:Class .
:Safe a owl:Class ; rdfs:subClassOf :Adult .
"
    );
    let v = report(&ttl);

    let row = kind_row(&v, "rdfs:subClassOf");
    assert_eq!(row["total"], 2, "two subsumptions: {row}");
    assert_eq!(row["outside"], 1, "exactly one of them is disjunctive: {row}");
    assert_eq!(row["inside"], 1, "and the other is not: {row}");
    assert_eq!(row["partially_inside"], 0, "a disjunctive HEAD does not split: {row}");

    assert_eq!(v["outside_the_fragment"]["count"], 1);
    assert_eq!(outside_axioms(&v), vec!["<http://e/Adult>"]);

    let entry = &v["outside_the_fragment"]["axioms"][0];
    assert_eq!(entry["standing"], "outside");
    let reason = entry["outside_because"][0]["reason"].as_str().unwrap_or("");
    assert!(
        reason.contains("disjunction in the consequent"),
        "the reason must name the construct a reader acts on, got {reason:?}"
    );
    assert_eq!(entry["outside_because"][0]["construct"], "owl:unionOf");
    assert_eq!(entry["outside_because"][0]["position"], "consequent");

    // And it is NOT in the other bucket. This is the whole distinction.
    assert!(
        !unimplemented_axioms(&v).contains(&"<http://e/Adult>".to_string()),
        "a non-Horn axiom is not an unimplemented rule: {}",
        v["inside_the_fragment_but_a_rule_is_not_implemented"]
    );
}

/// The other three reasons, each on an axiom built for it, each named.
#[test]
fn the_four_reasons_are_each_named_with_a_count() {
    let ttl = format!(
        "{PREFIXES}
:A a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :r ; owl:someValuesFrom :B ] .
:B a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :r ; owl:minCardinality 2 ] .
:C a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :r ; owl:maxCardinality 3 ] .
[ owl:complementOf :B ] rdfs:subClassOf :C .
:r a owl:ObjectProperty .
"
    );
    let v = report(&ttl);
    let reasons: Vec<String> = v["reasons"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|r| r["reason"].as_str().unwrap_or("").to_string())
        .collect();
    let joined = reasons.join(" || ");

    for wanted in [
        "existential in the head",
        "cardinality restriction",
        "negation in the antecedent",
    ] {
        assert!(joined.contains(wanted), "no reason mentions {wanted:?}: {joined}");
    }
    // Counted, not merely named.
    let total: u64 = v["reasons"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|r| r["occurrences"].as_u64().unwrap_or(0))
        .sum();
    assert_eq!(total, 4, "four axioms, four reasons: {:?}", reasons);
}

/// **Gate.** `owl:ReflexiveProperty` is outside OWL 2 RL and is NOT outside
/// Horn logic, and the report says both.
///
/// `⊤(x) → r(x,x)` is a Horn clause. OWL 2 RL excludes the axiom form anyway,
/// because a rule over the terms in the graph cannot reach the part of the
/// domain nothing names. Saying "outside the fragment" without that
/// qualification would be false, so the qualification is a field.
#[test]
fn a_horn_axiom_that_owl2_rl_excludes_carries_the_qualification() {
    let ttl = format!("{PREFIXES}:r a owl:ObjectProperty, owl:ReflexiveProperty .\n");
    let v = report(&ttl);
    let entry = v["outside_the_fragment"]["axioms"]
        .as_array()
        .expect("a list")
        .iter()
        .find(|a| a["axiom_type"] == "owl:ReflexiveProperty")
        .unwrap_or_else(|| panic!("no ReflexiveProperty row in {}", v["outside_the_fragment"]));
    let note = entry["horn_but_outside_owl2_rl"].as_str().unwrap_or("");
    assert!(
        note.contains("Horn clause"),
        "the qualification must say it IS Horn, got {note:?}"
    );
}

// ── 3. Inside the fragment, and still invisible ─────────────────────────────

/// **Gate.** An axiom INSIDE the fragment whose rule this engine does not
/// implement lands in its own bucket, not with the non-Horn ones.
///
/// This is the test the tool exists for. `owl:hasKey` is a conjunction of
/// property matches implying `owl:sameAs`: Horn through and through, and OWL 2
/// RL has `prp-key` for it. This engine has no `prp-key`. So the axiom is
/// inside the fragment AND invisible, and the fix is a patch to
/// `src/reason.rs` rather than a rewrite of the ontology. Merging the two
/// buckets would tell the reader to do the wrong thing.
#[test]
fn an_unimplemented_rule_is_not_reported_as_outside_the_fragment() {
    let ttl = format!(
        "{PREFIXES}
:Person a owl:Class ; owl:hasKey ( :ssn ) .
:ssn a owl:DatatypeProperty .
:ancestor a owl:ObjectProperty ; owl:propertyChainAxiom ( :parent :ancestor ) .
:parent a owl:ObjectProperty .
:A a owl:Class ; rdfs:subClassOf :B .
:B a owl:Class .
"
    );
    let v = report(&ttl);

    // Neither of the two is outside the fragment: both are Horn.
    assert_eq!(
        v["outside_the_fragment"]["count"], 0,
        "hasKey and a property chain are BOTH Horn: {}",
        v["outside_the_fragment"]
    );
    assert_eq!(kind_row(&v, "owl:hasKey")["outside"], 0);
    assert_eq!(kind_row(&v, "owl:hasKey")["inside"], 1);
    assert_eq!(kind_row(&v, "owl:propertyChainAxiom")["inside"], 1);

    // Both ARE invisible, in the other bucket, each naming the rule that would
    // have seen them.
    assert_eq!(v["inside_the_fragment_but_a_rule_is_not_implemented"]["count"], 2);
    let subjects = unimplemented_axioms(&v);
    assert!(subjects.contains(&"<http://e/Person>".to_string()), "{subjects:?}");
    assert!(subjects.contains(&"<http://e/ancestor>".to_string()), "{subjects:?}");

    let key = v["inside_the_fragment_but_a_rule_is_not_implemented"]["axioms"]
        .as_array()
        .expect("a list")
        .iter()
        .find(|a| a["axiom_type"] == "owl:hasKey")
        .expect("the hasKey row");
    assert_eq!(key["standing"], "inside", "hasKey is inside the fragment");
    assert_eq!(key["rules_this_engine_does_not"][0]["rule"], "prp-key");
    assert_eq!(key["rules_this_engine_does_not"][0]["status"], "not_implemented");

    let chain = v["inside_the_fragment_but_a_rule_is_not_implemented"]["axioms"]
        .as_array()
        .expect("a list")
        .iter()
        .find(|a| a["axiom_type"] == "owl:propertyChainAxiom")
        .expect("the chain row");
    assert_eq!(chain["rules_this_engine_does_not"][0]["rule"], "prp-spo2");

    // And the clean subsumption is in neither bucket: `cax-sco` and `scm-sco`
    // are both implemented, so the flag means something.
    assert!(!subjects.contains(&"<http://e/A>".to_string()), "{subjects:?}");
    assert_eq!(kind_row(&v, "rdfs:subClassOf")["inside_and_every_rule_implemented"], 1);

    // The headline: what the rule table does not fully see, however it came to
    // be that way, with the three causes still separately available because
    // they are fixed in three different places.
    let head = &v["not_fully_seen_by_the_rule_table"];
    assert_eq!(head["axioms"], 2);
    assert_eq!(head["wholly_outside_the_fragment"], 0);
    assert_eq!(head["partially_inside_the_fragment"], 0);
    assert_eq!(head["inside_but_a_rule_is_not_implemented"], 2);
}

/// A rule that concludes `false` and that the clash detector does not look for
/// is reported as such, with the engine's own reason, and not as
/// `not_implemented`.
///
/// `owl:AllDisjointClasses` is the case: it IS detected in its pairwise
/// `owl:disjointWith` spelling and is not detected in its list spelling,
/// because the clash detector reads no RDF lists. A reader who rewrites the
/// axiom pairwise gets it back, and only the specific reason tells them that.
#[test]
fn an_undetected_clash_rule_keeps_its_own_word_and_its_own_reason() {
    let ttl = format!(
        "{PREFIXES}
[] a owl:AllDisjointClasses ; owl:members ( :A :B :C ) .
:A a owl:Class ; owl:disjointWith :B .
:B a owl:Class . :C a owl:Class .
"
    );
    let v = report(&ttl);

    // The pairwise spelling is fully seen: cax-dw is in the clash detector.
    assert_eq!(kind_row(&v, "owl:disjointWith")["inside_and_every_rule_implemented"], 1);

    let adc = v["inside_the_fragment_but_a_rule_is_not_implemented"]["axioms"]
        .as_array()
        .expect("a list")
        .iter()
        .find(|a| a["axiom_type"] == "owl:AllDisjointClasses")
        .unwrap_or_else(|| {
            panic!("no AllDisjointClasses row: {}", v["inside_the_fragment_but_a_rule_is_not_implemented"])
        });
    assert_eq!(adc["standing"], "inside");
    assert_eq!(adc["rules_this_engine_does_not"][0]["rule"], "cax-adc");
    assert_eq!(
        adc["rules_this_engine_does_not"][0]["status"], "clash_not_detected",
        "a rule that concludes `false` and is not looked for is not the same failure as a rule \
         nobody wrote"
    );
    let why = adc["rules_this_engine_does_not"][0]["why"].as_str().unwrap_or("");
    assert!(
        why.contains("owl:disjointWith"),
        "the reason must name the spelling that IS detected, got {why:?}"
    );
}

/// **Regression.** `owl:Thing` is excluded from the top-level `Class`
/// alternative of both OWL 2 RL class grammars and is ADMITTED as a filler, and
/// this classifier refused it as a filler for its first hour.
///
/// Section 4.2.3's `subObjectSomeValuesFrom` has a second alternative that is
/// literally `'ObjectSomeValuesFrom' '(' ObjectPropertyExpression owl:Thing
/// ')'`, and `superObjectMaxCardinality` has the matching one. `cls-svf2`,
/// `cls-maxqc2` and `cls-maxqc4` are the rules for exactly those cases, and
/// `cls-maxqc2` keys on `T(?x, owl:onClass, owl:Thing)` outright. Reporting
/// such an axiom as outside the fragment is a FALSE POSITIVE, and a report
/// whose job is to tell a user what is invisible has to be trusted not to
/// invent losses.
///
/// Found by extracting the two grammars from the recommendation's HTML rather
/// than by reasoning about them, which is the only reason it was found at all.
#[test]
fn owl_thing_as_a_filler_is_inside_the_fragment() {
    let ttl = format!(
        "{PREFIXES}
[ a owl:Restriction ; owl:onProperty :r ; owl:someValuesFrom owl:Thing ] rdfs:subClassOf :A .
:B rdfs:subClassOf
  [ a owl:Restriction ; owl:onProperty :r ; owl:maxQualifiedCardinality 1 ; owl:onClass owl:Thing ] .
:A a owl:Class . :B a owl:Class . :r a owl:ObjectProperty .
"
    );
    let v = report(&ttl);
    assert_eq!(
        v["outside_the_fragment"]["count"], 0,
        "`exists r . Thing subClassOf A` is cls-svf2 and `B subClassOf max 1 r . Thing` is \
         cls-maxqc4; both are legal OWL 2 RL: {v}"
    );
    assert_eq!(v["partially_inside_the_fragment"]["count"], 0, "{v}");
    assert_eq!(kind_row(&v, "rdfs:subClassOf")["inside"], 2, "{v}");

    // And `owl:Thing` in the ANTECEDENT of a subsumption, which is the position
    // the grammar really does exclude, is still reported. Otherwise this test
    // would pass on a classifier that had stopped looking at owl:Thing at all.
    let universal = format!("{PREFIXES}owl:Thing rdfs:subClassOf :A .\n:A a owl:Class .\n");
    let v = report(&universal);
    assert_eq!(v["outside_the_fragment"]["count"], 1, "{v}");
    let why = v["outside_the_fragment"]["axioms"][0]["outside_because"][0]["reason"]
        .as_str()
        .unwrap_or("");
    assert!(why.contains("every element of the domain"), "got {why:?}");
}

// ── 4. Partial, and the splitting asymmetry ─────────────────────────────────

/// An equivalence with an existential on one side keeps the direction the rule
/// table evaluates, and is reported `partially_inside` rather than as either
/// extreme.
///
/// `A ≡ ∃r.B` is `∃r.B ⊑ A`, which is exactly `cls-svf1`, and `A ⊑ ∃r.B`,
/// which is an existential in the head. Reporting the whole axiom as outside
/// would say the engine sees nothing of it, and reporting it as inside would
/// say it sees all of it. Both are false.
#[test]
fn an_equivalence_with_one_horn_direction_is_partial() {
    let ttl = format!(
        "{PREFIXES}
:Parent a owl:Class ;
  owl:equivalentClass [ a owl:Restriction ; owl:onProperty :hasChild ; owl:someValuesFrom :Person ] .
:Person a owl:Class .
:hasChild a owl:ObjectProperty .
"
    );
    let v = report(&ttl);
    let row = kind_row(&v, "owl:equivalentClass");
    assert_eq!(row["partially_inside"], 1, "{row}");
    assert_eq!(row["inside"], 0, "{row}");
    assert_eq!(row["outside"], 0, "{row}");

    // In the PARTIAL bucket and not the outside one. A bucket called `outside`
    // holding an axiom the rules half evaluate would be false in its own name.
    assert_eq!(v["outside_the_fragment"]["count"], 0, "{v}");
    assert_eq!(v["partially_inside_the_fragment"]["count"], 1, "{v}");
    let entry = &v["partially_inside_the_fragment"]["axioms"][0];
    assert_eq!(entry["standing"], "partially_inside");
    let reason = entry["outside_because"][0]["reason"].as_str().unwrap_or("");
    assert!(reason.contains("existential in the head"), "{reason}");
    assert!(
        v["partially_inside_the_fragment"]["why_not_outside"]
            .as_str()
            .unwrap_or("")
            .contains("does NOT split"),
        "the report states the asymmetry that makes the split sound: {v}"
    );
}

/// **Gate.** The asymmetry: a conjunction splits in the consequent and does
/// NOT split in the antecedent.
///
/// `A ⊑ B ⊓ Out` keeps `A ⊑ B`, so it is `partially_inside`. `A ⊓ Out ⊑ C`
/// keeps nothing, because dropping a conjunct from a rule BODY makes the rule
/// fire more often, which is unsound rather than weaker, so it is `outside`.
/// A classifier that treated the two alike would be reporting a rule that
/// fires on cases the user never wrote.
#[test]
fn a_conjunctive_body_with_one_bad_conjunct_is_outside_not_partial() {
    let head = format!(
        "{PREFIXES}
:A a owl:Class ; rdfs:subClassOf
  [ owl:intersectionOf ( :B [ a owl:Restriction ; owl:onProperty :r ; owl:someValuesFrom :C ] ) ] .
:B a owl:Class . :C a owl:Class . :r a owl:ObjectProperty .
"
    );
    let v = report(&head);
    let row = kind_row(&v, "rdfs:subClassOf");
    assert_eq!(
        row["partially_inside"], 1,
        "`A subClassOf (B and exists r.C)` keeps its `A subClassOf B` half: {row}"
    );
    assert_eq!(row["outside"], 0, "{row}");

    let body = format!(
        "{PREFIXES}
[ owl:intersectionOf ( :B [ a owl:Restriction ; owl:onProperty :r ; owl:allValuesFrom :C ] ) ]
  rdfs:subClassOf :A .
:A a owl:Class . :B a owl:Class . :C a owl:Class . :r a owl:ObjectProperty .
"
    );
    let v = report(&body);
    let row = kind_row(&v, "rdfs:subClassOf");
    assert_eq!(
        row["outside"], 1,
        "a conjunctive BODY does not split: dropping a conjunct makes the rule fire more \
         often, which is unsound and not merely weaker. {row}"
    );
    assert_eq!(row["partially_inside"], 0, "{row}");
}

/// A disjunction in the ANTECEDENT splits, which is the mirror of the above and
/// the reason the asymmetry is about position and not about the connective.
#[test]
fn a_disjunctive_body_splits_where_a_disjunctive_head_does_not() {
    let ttl = format!(
        "{PREFIXES}
[ owl:unionOf ( :B [ a owl:Restriction ; owl:onProperty :r ; owl:allValuesFrom :C ] ) ]
  rdfs:subClassOf :A .
:A a owl:Class . :B a owl:Class . :C a owl:Class . :r a owl:ObjectProperty .
"
    );
    let v = report(&ttl);
    assert_eq!(
        kind_row(&v, "rdfs:subClassOf")["partially_inside"], 1,
        "`(B or all r.C) subClassOf A` keeps `B subClassOf A`: {v}"
    );
}

// ── 5. Nothing is passed over in silence ────────────────────────────────────

/// **Gate.** Every triple in the store is either classified as an axiom or
/// counted in a named `not_classified` bucket.
///
/// A tool that reports on a fragment and leaves the rest uncounted is the shape
/// this module exists to attack, so it must not be that shape itself.
#[test]
fn every_triple_lands_somewhere() {
    // Built here rather than loaded, and deliberately mixed: schema axioms of
    // every standing, a class expression with its RDF list, an annotation, a
    // declaration, a data assertion and an object assertion. A fixture that
    // happened to contain no schema axiom would pass the partition check while
    // proving nothing, which is how this test first went green on a file that
    // turned out to be pure instance data.
    let ttl = format!(
        "{PREFIXES}
:A a owl:Class ; rdfs:label \"A\" ; rdfs:subClassOf [ owl:unionOf ( :B :C ) ] .
:B a owl:Class ; rdfs:subClassOf :C ; owl:disjointWith :D .
:C a owl:Class ; owl:equivalentClass
  [ a owl:Restriction ; owl:onProperty :r ; owl:someValuesFrom :D ] .
:D a owl:Class ; owl:hasKey ( :r ) .
:r a owl:ObjectProperty, owl:TransitiveProperty ; rdfs:domain :A ; rdfs:range :B ;
   owl:propertyChainAxiom ( :r :r ) ; owl:inverseOf :s .
:s a owl:ObjectProperty .
:p a owl:DatatypeProperty .
:a a :A ; :r :b ; :p \"x\" ; owl:sameAs :b .
:b a :B .
"
    );
    let v = report(&ttl);

    let classified = v["axioms_classified"].as_u64().expect("a count");
    let unclassified: u64 = v["not_classified"]["counts"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|c| c["occurrences"].as_u64().unwrap_or(0))
        .sum();
    assert_eq!(
        classified + unclassified,
        v["triples"].as_u64().expect("a count"),
        "the partition is not total: {classified} + {unclassified} against {}",
        v["triples"]
    );
    assert!(classified >= 11, "the fixture has schema axioms of every standing: {classified}");
    // And each of the three standings is present, so the partition above is
    // over a real mixture rather than over one bucket.
    assert!(v["outside_the_fragment"]["count"].as_u64().unwrap_or(0) > 0, "{v}");
    assert!(
        v["inside_the_fragment_but_a_rule_is_not_implemented"]["count"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "{v}"
    );
    let partial: u64 = v["by_axiom_type"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|r| r["partially_inside"].as_u64().unwrap_or(0))
        .sum();
    assert!(partial > 0, "the equivalentClass with an existential on one side is partial: {v}");
    // The three causes are disjoint and sum to the headline.
    let head = &v["not_fully_seen_by_the_rule_table"];
    assert_eq!(
        head["wholly_outside_the_fragment"].as_u64().unwrap_or(0)
            + head["partially_inside_the_fragment"].as_u64().unwrap_or(0)
            + head["inside_but_a_rule_is_not_implemented"].as_u64().unwrap_or(0),
        head["axioms"].as_u64().unwrap_or(0),
        "{head}"
    );
    assert_eq!(head["partially_inside_the_fragment"].as_u64().unwrap_or(0), partial);
    let outside: u64 = v["by_axiom_type"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|r| r["outside"].as_u64().unwrap_or(0))
        .sum();
    assert_eq!(head["wholly_outside_the_fragment"].as_u64().unwrap_or(0), outside);
}

/// An ontology entirely inside the fragment, with every rule implemented,
/// reports nothing invisible. Without this the flag could be true always and
/// still pass every test above.
#[test]
fn an_ontology_the_rule_table_fully_sees_reports_nothing_invisible() {
    let ttl = format!(
        "{PREFIXES}
:A a owl:Class ; rdfs:subClassOf :B ; owl:disjointWith :D .
:B a owl:Class ; rdfs:subClassOf :Top .
:D a owl:Class . :Top a owl:Class .
:r a owl:ObjectProperty, owl:TransitiveProperty ; rdfs:domain :A ; rdfs:range :B .
:a a :A . :a :r :b . :b a :B .
"
    );
    let v = report(&ttl);
    assert_eq!(v["outside_the_fragment"]["count"], 0, "{v}");
    assert_eq!(
        v["inside_the_fragment_but_a_rule_is_not_implemented"]["count"], 0,
        "{}",
        v["inside_the_fragment_but_a_rule_is_not_implemented"]
    );
    assert_eq!(v["not_fully_seen_by_the_rule_table"]["axioms"], 0);
    assert_eq!(v["axioms_classified"], 6, "{v}");
}

/// A long listing is capped and the cap is reported, so it hides the listing
/// and never the scale. The same discipline `onto_defects` uses.
#[test]
fn a_long_listing_is_capped_and_says_so() {
    let mut ttl = String::from(PREFIXES);
    for i in 0..60 {
        ttl.push_str(&format!(
            ":C{i} a owl:Class ; rdfs:subClassOf [ owl:unionOf ( :X :Y ) ] .\n"
        ));
    }
    ttl.push_str(":X a owl:Class . :Y a owl:Class .\n");
    let v = report(&ttl);
    assert_eq!(v["outside_the_fragment"]["count"], 60);
    assert_eq!(v["outside_the_fragment"]["listed"], 50, "capped at MAX_LISTED");
    assert_eq!(v["outside_the_fragment"]["truncated"], true);
}

/// The report's rule-table figures are the derived ones, in the output, so a
/// reader does not have to take the prose for it.
#[test]
fn the_report_carries_the_derived_rule_counts() {
    let v = report(&format!("{PREFIXES}:A a owl:Class .\n"));
    assert_eq!(v["rule_table"]["owl2_rl_rules"], 78);
    assert_eq!(v["rule_table"]["evaluated_in_the_fixpoint"], 29);
    assert_eq!(v["rule_table"]["detected_as_a_clash"], 10);
    assert_eq!(v["rule_table"]["concludes_false_and_not_detected"], 7);
    assert_eq!(v["rule_table"]["not_implemented"], 32);
    let names = v["rule_table"]["not_implemented_names"].as_array().expect("a list");
    assert_eq!(names.len(), 32);
    // The per-profile counts are derived from reason::RULES_EVALUATED and nest.
    // A report quoting all 29 to an `rdfs` run would describe a table that run
    // never used.
    let by_profile = &v["rule_table"]["evaluated_by_profile"];
    assert_eq!(by_profile["rdfs"], 6);
    assert_eq!(by_profile["owl-rl"], 17);
    assert_eq!(by_profile["owl-rl-ext"], 29);
    assert!(names.iter().any(|n| n == "prp-key"));
    assert!(names.iter().any(|n| n == "prp-spo2"));
    assert!(names.iter().any(|n| n == "eq-trans"));
    // And the two dimensions are described in the output, not only in a doc.
    assert!(v["two_dimensions_never_merged"]["fragment"].is_string());
    assert!(v["two_dimensions_never_merged"]["engine"].is_string());
}
