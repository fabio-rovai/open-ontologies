//! Which of YOUR axioms the rule engine can actually see.
//!
//! # The gap this closes
//!
//! `src/rulesyntax.rs` polices the Description Logic Programs boundary in one
//! direction already. A SWRL built-in atom or a RIF `External` call is refused
//! by name and counted, because a rule table that quietly lost half its rules
//! still reaches a fixpoint and still produces a certificate that checks green.
//! That is the import direction.
//!
//! Nothing policed the other direction. A user loads an ontology, runs
//! `onto_reason --profile owl-rl-ext --certificate`, and gets a certificate the
//! Lean checker accepts. Everything about that is true and none of it says how
//! much of the TBox the rule table ever looked at. An `owl:unionOf` in a
//! consequent, an existential in a head, a `owl:propertyChainAxiom`, an
//! `owl:hasKey`: each is simply not seen, no rule fires on it, the fixpoint
//! closes, and the certificate is a sound proof about the FRAGMENT of the
//! ontology the rules could read. The user is never told which fragment that
//! was. That is the same assurance-laundering shape, with the loss moved from
//! the rule table to the ontology.
//!
//! This file is that report. It classifies every schema axiom in the loaded
//! graph and says, for each one, whether the rule engine can see it — and when
//! it cannot, which of the two very different reasons applies.
//!
//! # The two dimensions, which are never merged
//!
//! **Dimension one, the FRAGMENT.** Is the axiom expressible as Horn rules over
//! triple patterns at all? `A ⊑ B ⊔ C` is not: a disjunction in the consequent
//! is not a Horn clause, and no implementation effort will make it one. This is
//! a fact about the LANGUAGE and it would hold of a perfect OWL 2 RL engine.
//!
//! **Dimension two, THIS ENGINE.** Is there a rule in the table that fires on
//! it? `owl:hasKey` is perfectly Horn — a conjunction of matching property
//! values implies `owl:sameAs` — and OWL 2 RL has a rule for it, `prp-key`.
//! This engine does not implement that rule. So the axiom is INSIDE the
//! fragment and still invisible. A different engine would see it; a different
//! language would not help.
//!
//! Merging those two into one "not covered" bucket destroys the only
//! information a reader can act on. One is a rewrite of the ontology; the other
//! is a patch to `src/reason.rs`. The report keeps them apart, and
//! `an_unimplemented_rule_is_not_reported_as_outside_the_fragment` in
//! `tests/dlp_boundary_test.rs` is the gate that they stay apart.
//!
//! # Where the line is drawn, exactly
//!
//! DLP is a family of fragments, not a single line. Grosof, Horrocks, Volz and
//! Decker's *Description Logic Programs: Combining Logic Programs with
//! Description Logic* (WWW 2003) defines the intersection; OWL 2 RL is the
//! standardised descendant of it, and OWL 2 RL is what this engine's rule table
//! targets. **So the line drawn here is OWL 2 RL's**, taken from the class
//! expression grammar in section 4.3 of the OWL 2 Profiles recommendation:
//! `subClassExpression` for what may stand in an antecedent and
//! `superClassExpression` for what may stand in a consequent.
//!
//! Two places where the abstract DLP line and the OWL 2 RL line differ are
//! named rather than smoothed over, and carry `horn_but_outside_owl2_rl`:
//! `ReflexiveObjectProperty` and `DisjointUnion` are both excluded from OWL 2
//! RL as axioms, and reflexivity in particular IS a Horn clause (`⊤(x) →
//! r(x,x)`). Saying "outside the fragment" about it without that qualification
//! would be false.
//!
//! # Where this classifier is MORE generous than the grammar, and says so
//!
//! The OWL 2 RL grammar rejects a whole axiom when any part of it is outside.
//! Two shapes split soundly, and this classifier splits them and reports
//! `partially_inside` rather than `outside`:
//!
//! * a CONJUNCTION in the consequent. `A ⊑ B ⊓ Out` is `A ⊑ B` and `A ⊑ Out`,
//!   so the `A ⊑ B` half is a Horn rule whatever `Out` is.
//! * a DISJUNCTION in the antecedent. `(A ⊔ Out) ⊑ C` is `A ⊑ C` and
//!   `Out ⊑ C`, so again one half survives.
//!
//! The asymmetry is the whole point and is not symmetric by accident. A
//! conjunction in the ANTECEDENT does not split: dropping a conjunct from a
//! rule body makes the rule fire MORE often, which is unsound, so
//! `A ⊓ Out ⊑ C` is outside as a whole. Neither does a disjunction in the
//! consequent. `a_conjunctive_body_with_one_bad_conjunct_is_outside_not_partial`
//! pins that.
//!
//! **`owl:equivalentClass` is the third and largest generosity.** OWL 2 RL has
//! a THIRD class grammar, `equivClassExpression`, tighter than either of the
//! other two: a named class, an intersection of those, `ObjectHasValue` and
//! `DataHasValue`, and nothing else. So the profile rejects
//! `EquivalentClasses(C, ObjectSomeValuesFrom(r, D))` outright at the syntax
//! level. This classifier reads it as the two subsumptions it means and reports
//! that ONE of them is evaluated, because the profile's own rules `cax-eqc1`
//! and `cax-eqc2` perform exactly that split and `cls-svf1` really does fire on
//! the surviving direction. Reporting the axiom as wholly outside would say the
//! rules see nothing of it, which is false. The generosity is written down here
//! because it is a deliberate departure from the grammar and not an oversight.
//!
//! Nothing propagates a split through `owl:complementOf`. Weakening under a
//! negation strengthens, so the partial flag is dropped when the classifier
//! crosses one.
//!
//! # What this is NOT
//!
//! It is not a consistency check, not a profile validator, and not a claim
//! about what OWL 2 RL entails. It reports what the rule table in
//! `src/reason.rs` can and cannot read, and the count of rules it does not
//! implement is derived from `reason::RULES_EVALUATED` and
//! `reason::CLASH_RULES_NOT_DETECTED` rather than typed here.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::graph::GraphStore;
use crate::tableaux::TripleIndex;

// ── Vocabulary ──────────────────────────────────────────────────────────────

macro_rules! owl {
    ($local:literal) => {
        concat!("<http://www.w3.org/2002/07/owl#", $local, ">")
    };
}
macro_rules! rdfs {
    ($local:literal) => {
        concat!("<http://www.w3.org/2000/01/rdf-schema#", $local, ">")
    };
}

const RDF_TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";

const RDFS_SUBCLASS: &str = rdfs!("subClassOf");
const RDFS_SUBPROP: &str = rdfs!("subPropertyOf");
const RDFS_DOMAIN: &str = rdfs!("domain");
const RDFS_RANGE: &str = rdfs!("range");

const OWL_THING: &str = owl!("Thing");
const OWL_NOTHING: &str = owl!("Nothing");
const OWL_EQUIV_CLASS: &str = owl!("equivalentClass");
const OWL_EQUIV_PROP: &str = owl!("equivalentProperty");
const OWL_DISJOINT_WITH: &str = owl!("disjointWith");
const OWL_ALL_DISJOINT_CLASSES: &str = owl!("AllDisjointClasses");
const OWL_ALL_DISJOINT_PROPS: &str = owl!("AllDisjointProperties");
const OWL_ALL_DIFFERENT: &str = owl!("AllDifferent");
const OWL_DISJOINT_UNION: &str = owl!("disjointUnionOf");
const OWL_COMPLEMENT_OF: &str = owl!("complementOf");
const OWL_INVERSE_OF: &str = owl!("inverseOf");
const OWL_PROPERTY_CHAIN: &str = owl!("propertyChainAxiom");
const OWL_PROP_DISJOINT_WITH: &str = owl!("propertyDisjointWith");
const OWL_HAS_KEY: &str = owl!("hasKey");
const OWL_NEGATIVE_PROP_ASSERTION: &str = owl!("NegativePropertyAssertion");
const OWL_SAME_AS: &str = owl!("sameAs");
const OWL_DIFFERENT_FROM: &str = owl!("differentFrom");

const OWL_INTERSECTION_OF: &str = owl!("intersectionOf");
const OWL_UNION_OF: &str = owl!("unionOf");
const OWL_ONE_OF: &str = owl!("oneOf");
const OWL_RESTRICTION: &str = owl!("Restriction");
const OWL_ON_PROPERTY: &str = owl!("onProperty");
const OWL_ON_CLASS: &str = owl!("onClass");
const OWL_ON_DATA_RANGE: &str = owl!("onDataRange");
const OWL_SOME_VALUES_FROM: &str = owl!("someValuesFrom");
const OWL_ALL_VALUES_FROM: &str = owl!("allValuesFrom");
const OWL_HAS_VALUE: &str = owl!("hasValue");
const OWL_HAS_SELF: &str = owl!("hasSelf");
const OWL_MIN_CARDINALITY: &str = owl!("minCardinality");
const OWL_MAX_CARDINALITY: &str = owl!("maxCardinality");
const OWL_CARDINALITY: &str = owl!("cardinality");
const OWL_MIN_QUALIFIED_CARDINALITY: &str = owl!("minQualifiedCardinality");
const OWL_MAX_QUALIFIED_CARDINALITY: &str = owl!("maxQualifiedCardinality");
const OWL_QUALIFIED_CARDINALITY: &str = owl!("qualifiedCardinality");
const OWL_ON_DATATYPE: &str = owl!("onDatatype");
const OWL_WITH_RESTRICTIONS: &str = owl!("withRestrictions");
const OWL_DATATYPE_COMPLEMENT: &str = owl!("datatypeComplementOf");

const OWL_TRANSITIVE: &str = owl!("TransitiveProperty");
const OWL_SYMMETRIC: &str = owl!("SymmetricProperty");
const OWL_ASYMMETRIC: &str = owl!("AsymmetricProperty");
const OWL_REFLEXIVE: &str = owl!("ReflexiveProperty");
const OWL_IRREFLEXIVE: &str = owl!("IrreflexiveProperty");
const OWL_FUNCTIONAL: &str = owl!("FunctionalProperty");
const OWL_INVERSE_FUNCTIONAL: &str = owl!("InverseFunctionalProperty");

// ── The OWL 2 RL rule table ─────────────────────────────────────────────────

/// One rule of the OWL 2 RL profile's inference tables.
///
/// The seventy-eight names below are transcribed from Tables 4 to 9 of the W3C
/// OWL 2 Web Ontology Language Profiles recommendation (Second Edition, 11
/// December 2012), section 4.3. A transcription is a claim, so it is checked
/// two ways rather than asserted.
///
/// **Mechanically, against the source.** The rule names were extracted from the
/// recommendation's HTML on 15 September 2026 by reading the `<span
/// class="name" id="…">` of each table's first column, and the six tables came
/// back 9 / 20 / 19 / 5 / 5 / 20 with the names and the order below. That pass
/// also found the bug this module carried in its first hour: `owl:Thing` is
/// excluded from the top-level `Class` alternative of BOTH class grammars and
/// is ADMITTED as a filler in `subObjectSomeValuesFrom` and in
/// `superObjectMaxCardinality`, which `cls-svf2`, `cls-maxqc2` and `cls-maxqc4`
/// are the rules for. Refusing it was a false positive on legal OWL 2 RL.
///
/// **Against this repository, which read the tables independently.** Seventeen
/// of these rules conclude `false` rather than a triple, `src/reason.rs` names
/// the same seventeen in a list written before this one and without reference
/// to it, and `the_seventeen_false_concluding_rules_agree_with_the_engine`
/// fails if the two ever differ. The total of 78 and the counts 29 / 10 / 7 /
/// 32 are asserted by `the_rule_table_adds_up`, so a mistyped row cannot pass.
///
/// One defect in the recommendation, noted because a reader may hit it:
/// Appendix 6.3's standalone RL grammar alternates on `superComplementOf`,
/// while the production under it and every other use of the name is
/// `superObjectComplementOf`. Section 4.2.3 is unambiguous and is what this
/// module follows.
pub struct RlRule {
    /// The W3C name.
    pub name: &'static str,
    /// Which of the W3C tables it is in.
    pub table: &'static str,
    /// True when the rule's conclusion is `false`, so a fixpoint over triples
    /// cannot hold it and the clash detector is where it would live.
    pub concludes_false: bool,
    /// What `src/reason.rs` spells it, where the two differ. Equal to `name`
    /// otherwise.
    pub engine_name: &'static str,
}

const fn r(name: &'static str, table: &'static str) -> RlRule {
    RlRule { name, table, concludes_false: false, engine_name: name }
}
const fn rf(name: &'static str, table: &'static str) -> RlRule {
    RlRule { name, table, concludes_false: true, engine_name: name }
}
const fn ra(name: &'static str, table: &'static str, engine_name: &'static str) -> RlRule {
    RlRule { name, table, concludes_false: false, engine_name }
}

/// The OWL 2 RL profile's inference rules. Seventy-eight of them.
pub const OWL2_RL_RULES: &[RlRule] = &[
    // Table 4: The Semantics of Equality.
    r("eq-ref", "eq"),
    r("eq-sym", "eq"),
    r("eq-trans", "eq"),
    r("eq-rep-s", "eq"),
    r("eq-rep-p", "eq"),
    r("eq-rep-o", "eq"),
    rf("eq-diff1", "eq"),
    rf("eq-diff2", "eq"),
    rf("eq-diff3", "eq"),
    // Table 5: The Semantics of Axioms about Properties.
    r("prp-ap", "prp"),
    ra("prp-dom", "prp", "rdfs2"),
    ra("prp-rng", "prp", "rdfs3"),
    r("prp-fp", "prp"),
    r("prp-ifp", "prp"),
    rf("prp-irp", "prp"),
    r("prp-symp", "prp"),
    rf("prp-asyp", "prp"),
    r("prp-trp", "prp"),
    ra("prp-spo1", "prp", "rdfs7"),
    r("prp-spo2", "prp"),
    r("prp-eqp1", "prp"),
    r("prp-eqp2", "prp"),
    rf("prp-pdw", "prp"),
    rf("prp-adp", "prp"),
    r("prp-inv1", "prp"),
    r("prp-inv2", "prp"),
    r("prp-key", "prp"),
    rf("prp-npa1", "prp"),
    rf("prp-npa2", "prp"),
    // Table 6: The Semantics of Classes.
    r("cls-thing", "cls"),
    r("cls-nothing1", "cls"),
    rf("cls-nothing2", "cls"),
    r("cls-int1", "cls"),
    r("cls-int2", "cls"),
    r("cls-uni", "cls"),
    rf("cls-com", "cls"),
    r("cls-svf1", "cls"),
    r("cls-svf2", "cls"),
    r("cls-avf", "cls"),
    r("cls-hv1", "cls"),
    r("cls-hv2", "cls"),
    rf("cls-maxc1", "cls"),
    r("cls-maxc2", "cls"),
    rf("cls-maxqc1", "cls"),
    rf("cls-maxqc2", "cls"),
    r("cls-maxqc3", "cls"),
    r("cls-maxqc4", "cls"),
    r("cls-oo", "cls"),
    // Table 7: The Semantics of Class Axioms.
    ra("cax-sco", "cax", "rdfs9"),
    r("cax-eqc1", "cax"),
    r("cax-eqc2", "cax"),
    rf("cax-dw", "cax"),
    rf("cax-adc", "cax"),
    // Table 8: The Semantics of Datatypes.
    r("dt-type1", "dt"),
    r("dt-type2", "dt"),
    r("dt-eq", "dt"),
    r("dt-diff", "dt"),
    rf("dt-not-type", "dt"),
    // Table 9: The Semantics of Schema Vocabulary.
    r("scm-cls", "scm"),
    ra("scm-sco", "scm", "rdfs11"),
    r("scm-eqc1", "scm"),
    r("scm-eqc2", "scm"),
    r("scm-op", "scm"),
    r("scm-dp", "scm"),
    ra("scm-spo", "scm", "rdfs5"),
    r("scm-eqp1", "scm"),
    r("scm-eqp2", "scm"),
    r("scm-dom1", "scm"),
    r("scm-dom2", "scm"),
    r("scm-rng1", "scm"),
    r("scm-rng2", "scm"),
    r("scm-hv", "scm"),
    r("scm-svf1", "scm"),
    r("scm-svf2", "scm"),
    r("scm-avf1", "scm"),
    r("scm-avf2", "scm"),
    r("scm-int", "scm"),
    r("scm-uni", "scm"),
];

/// What this engine does with an OWL 2 RL rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleStatus {
    /// Evaluated in the forward-chaining fixpoint, under the named profile.
    Evaluated(&'static str),
    /// Concludes `false`, and the clash detector looks for it.
    ClashDetected,
    /// Concludes `false`, and the clash detector does NOT look for it. The
    /// reason travels with it, from `reason::CLASH_RULES_NOT_DETECTED`.
    ClashNotDetected(&'static str),
    /// No rule in this engine fires on it, and there is no clash detector for
    /// it either.
    NotImplemented,
}

impl RuleStatus {
    pub fn word(self) -> &'static str {
        match self {
            RuleStatus::Evaluated(_) => "evaluated",
            RuleStatus::ClashDetected => "clash_detected",
            RuleStatus::ClashNotDetected(_) => "clash_not_detected",
            RuleStatus::NotImplemented => "not_implemented",
        }
    }
    /// Whether an axiom this rule would evaluate is visible to a run at all.
    pub fn is_seen(self) -> bool {
        matches!(self, RuleStatus::Evaluated(_) | RuleStatus::ClashDetected)
    }
}

/// What this engine does with the named rule. Derived from `src/reason.rs`,
/// never typed here: the two figures are then one figure.
pub fn rule_status(name: &str) -> RuleStatus {
    let Some(rule) = OWL2_RL_RULES.iter().find(|r| r.name == name) else {
        return RuleStatus::NotImplemented;
    };
    if let Some((_, profile)) = crate::reason::RULES_EVALUATED
        .iter()
        .find(|(n, _)| *n == rule.engine_name)
    {
        return RuleStatus::Evaluated(profile);
    }
    if rule.concludes_false {
        return match crate::reason::CLASH_RULES_NOT_DETECTED
            .iter()
            .find(|(n, _)| *n == rule.name)
        {
            Some((_, why)) => RuleStatus::ClashNotDetected(why),
            None => RuleStatus::ClashDetected,
        };
    }
    RuleStatus::NotImplemented
}

// ── Classifying a class expression ──────────────────────────────────────────

/// Which side of an axiom a class expression stands on.
///
/// Not cosmetic and not symmetric. `owl:someValuesFrom` is legal in an
/// antecedent (`∃r.B ⊑ C` is the Horn rule `r(x,y) ∧ B(y) → C(x)`) and illegal
/// in a consequent (`C ⊑ ∃r.B` asserts a successor into existence, which a
/// rule over a fixed set of terms cannot do). Half of this module is that one
/// distinction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Position {
    Antecedent,
    Consequent,
}

impl Position {
    fn word(self) -> &'static str {
        match self {
            Position::Antecedent => "antecedent",
            Position::Consequent => "consequent",
        }
    }
}

/// One construct that puts an axiom outside the fragment, with the reason.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Excluded {
    /// The OWL construct, spelled as the ontology spells it.
    pub construct: String,
    /// Which side of the axiom it stands on.
    pub position: &'static str,
    /// Why a Horn rule over triple patterns cannot hold it. The category a
    /// reader acts on: `disjunction in the consequent`, `existential in the
    /// head`, `cardinality restriction`, `negation in the antecedent`.
    pub reason: String,
}

/// What a class expression contributes in one position.
#[derive(Default)]
struct Verdict {
    excluded: Vec<Excluded>,
    /// True when something of the axiom still lands inside the fragment even
    /// though `excluded` is not empty. Only a conjunction in the consequent and
    /// a disjunction in the antecedent set it: those two SPLIT soundly and
    /// nothing else does.
    partial: bool,
}

impl Verdict {
    fn inside() -> Verdict {
        Verdict::default()
    }
    fn out(construct: impl Into<String>, position: Position, reason: impl Into<String>) -> Verdict {
        Verdict {
            excluded: vec![Excluded {
                construct: construct.into(),
                position: position.word(),
                reason: reason.into(),
            }],
            partial: false,
        }
    }
    fn is_inside(&self) -> bool {
        self.excluded.is_empty()
    }
}

/// How deep a chain of anonymous class expressions is followed before the
/// classifier gives up and says so.
///
/// A blank-node cycle through `owl:intersectionOf` is legal RDF and is not
/// legal OWL, and a classifier that recursed into one would hang. The visited
/// set catches the cycle; the cap catches a legal-but-absurd nesting. Either
/// way the axiom is REPORTED as unclassifiable and never passed over.
const MAX_DEPTH: usize = 40;

/// The short, shared spellings of the four reasons a reader acts on. Written
/// once so a grep for one of them finds every site that produces it.
const WHY_DISJUNCTIVE_HEAD: &str =
    "disjunction in the consequent: a Horn clause has one atom in its head, and no rule table \
     over triple patterns can conclude `A or B`";
const WHY_EXISTENTIAL_HEAD: &str =
    "existential in the head: the axiom asserts a successor into existence, and a rule over a \
     fixed set of terms can only ever conclude about terms it already has";
const WHY_NEGATIVE_BODY: &str =
    "negation in the antecedent: a Horn body is a conjunction of positive atoms, and deciding \
     that something is NOT of a class needs a closed world this engine does not assume";
const WHY_UNIVERSAL_BODY: &str =
    "a universal restriction in the antecedent: `all r . B` unfolds to a negated existential, \
     which is a disjunction once it reaches the body of a rule";

struct Classifier<'a> {
    index: &'a TripleIndex,
}

impl Classifier<'_> {
    fn has_type(&self, node: &str, ty: &str) -> bool {
        self.index.objects(node, RDF_TYPE).iter().any(|o| o == ty)
    }

    fn cardinality(&self, node: &str, predicate: &str) -> Option<u32> {
        let raw = self.index.object(node, predicate)?;
        // A literal arrives as `"1"^^<...>`; take what is between the quotes.
        let inner = raw.strip_prefix('"').and_then(|r| r.split('"').next())?;
        inner.trim().parse::<u32>().ok()
    }

    /// Classify a class expression standing in `position`.
    fn class(&self, node: &str, position: Position, seen: &mut BTreeSet<String>, depth: usize) -> Verdict {
        if depth > MAX_DEPTH {
            return Verdict::out(
                "a class expression nested deeper than this classifier follows",
                position,
                format!(
                    "the nesting passed {MAX_DEPTH} levels. Reported rather than passed over: an \
                     expression nobody classified is not an expression nobody has to worry about"
                ),
            );
        }
        if !seen.insert(node.to_string()) {
            return Verdict::out(
                "a cyclic class expression",
                position,
                "the blank-node graph of this class expression contains a cycle, which is legal \
                 RDF and is not a legal OWL class expression, so nothing here can classify it",
            );
        }

        let v = self.class_inner(node, position, seen, depth);
        seen.remove(node);
        v
    }

    fn class_inner(
        &self,
        node: &str,
        position: Position,
        seen: &mut BTreeSet<String>,
        depth: usize,
    ) -> Verdict {
        if node == OWL_THING {
            // `A subClassOf owl:Thing` is a tautology. The OWL 2 RL grammar
            // excludes owl:Thing from BOTH positions, but it excludes it in the
            // consequent for triviality and in the antecedent for expressive
            // power, and reporting a tautology as a trust gap would be a false
            // alarm.
            return match position {
                Position::Consequent => Verdict::inside(),
                Position::Antecedent => Verdict::out(
                    "owl:Thing",
                    position,
                    "owl:Thing in the antecedent asks a rule to fire on every element of the \
                     domain, including the ones no term names. OWL 2 RL excludes owl:Thing from \
                     subClassExpression for that reason",
                ),
            };
        }
        if node == OWL_NOTHING {
            return Verdict::inside();
        }
        // A named class with no class-expression machinery on it.
        if node.starts_with('<') && self.class_construct(node).is_none() {
            return Verdict::inside();
        }
        match self.class_construct(node) {
            Some(Construct::Intersection(items)) => self.boolean(items, position, seen, depth, true),
            Some(Construct::Union(items)) => self.boolean(items, position, seen, depth, false),
            Some(Construct::OneOf) => match position {
                Position::Antecedent => Verdict::inside(),
                Position::Consequent => Verdict::out(
                    "owl:oneOf",
                    position,
                    format!(
                        "{WHY_DISJUNCTIVE_HEAD}. An enumeration in the consequent is exactly a \
                         disjunction of equalities"
                    ),
                ),
            },
            Some(Construct::Complement(inner)) => match position {
                // `A subClassOf (not B)` is `A and B subClassOf Nothing`, which
                // IS Horn: the head is `false`. The negated operand moves to the
                // body, so it is classified there.
                Position::Consequent => {
                    let mut v = self.class(&inner, Position::Antecedent, seen, depth + 1);
                    // A split under a negation is not sound: weakening `B` makes
                    // `not B` stronger.
                    v.partial = false;
                    v
                }
                Position::Antecedent => {
                    Verdict::out("owl:complementOf", position, WHY_NEGATIVE_BODY)
                }
            },
            Some(Construct::Restriction) => self.restriction(node, position, seen, depth),
            Some(Construct::DataRange(what)) => Verdict::out(
                what,
                position,
                "a constructed data range is outside the OWL 2 RL data-range grammar, and this \
                 engine has no datatype value space at all: none of dt-type1, dt-type2, dt-eq, \
                 dt-diff or dt-not-type is implemented",
            ),
            None => Verdict::out(
                "an anonymous node in a class position with no recognised class construct",
                position,
                format!(
                    "the node {node} stands where a class expression must and carries none of \
                     owl:intersectionOf, owl:unionOf, owl:oneOf, owl:complementOf or \
                     owl:Restriction. It is reported rather than skipped, because an expression \
                     nobody read is not an expression nobody has to worry about"
                ),
            ),
        }
    }

    /// A conjunction or a disjunction. `intersection` picks which.
    ///
    /// The splitting asymmetry lives here and nowhere else. A conjunction
    /// splits in the CONSEQUENT (`A ⊑ B ⊓ C` is two axioms) and a disjunction
    /// splits in the ANTECEDENT (`A ⊔ B ⊑ C` is two axioms). In the other
    /// position neither splits, and dropping an operand there would be
    /// UNSOUND rather than merely weaker.
    fn boolean(
        &self,
        items: Vec<String>,
        position: Position,
        seen: &mut BTreeSet<String>,
        depth: usize,
        intersection: bool,
    ) -> Verdict {
        let splits = match position {
            Position::Consequent => intersection,
            Position::Antecedent => !intersection,
        };
        if !splits && !intersection {
            return Verdict::out("owl:unionOf", position, WHY_DISJUNCTIVE_HEAD);
        }
        let mut excluded = Vec::new();
        let mut any_inside = false;
        let mut any_partial = false;
        for item in &items {
            let v = self.class(item, position, seen, depth + 1);
            if v.is_inside() {
                any_inside = true;
            }
            any_partial |= v.partial;
            excluded.extend(v.excluded);
        }
        if excluded.is_empty() {
            return Verdict::inside();
        }
        Verdict {
            excluded,
            // A conjunctive BODY does not split: dropping a conjunct from a
            // rule body makes the rule fire more often, which is unsound, so
            // one bad conjunct takes the whole axiom out.
            partial: splits && (any_inside || any_partial),
        }
    }

    fn restriction(
        &self,
        node: &str,
        position: Position,
        seen: &mut BTreeSet<String>,
        depth: usize,
    ) -> Verdict {
        if let Some(filler) = self.index.object(node, OWL_SOME_VALUES_FROM) {
            return match position {
                // owl:Thing is excluded from the top-level `Class` alternative
                // of subClassExpression and ADMITTED here: the grammar's second
                // alternative is `'ObjectSomeValuesFrom' '(' OPE owl:Thing ')'`
                // outright. Rule cls-svf2 is that case. Refusing it here was a
                // false positive on legal OWL 2 RL, and it is the one the W3C
                // grammar check of 15 September 2026 caught.
                Position::Antecedent if filler == OWL_THING => Verdict::inside(),
                Position::Antecedent => self.class(&filler, Position::Antecedent, seen, depth + 1),
                Position::Consequent => {
                    Verdict::out("owl:someValuesFrom", position, WHY_EXISTENTIAL_HEAD)
                }
            };
        }
        if let Some(filler) = self.index.object(node, OWL_ALL_VALUES_FROM) {
            return match position {
                Position::Consequent => self.class(&filler, Position::Consequent, seen, depth + 1),
                Position::Antecedent => {
                    Verdict::out("owl:allValuesFrom", position, WHY_UNIVERSAL_BODY)
                }
            };
        }
        if self.index.object(node, OWL_HAS_VALUE).is_some() {
            return Verdict::inside();
        }
        if self.index.object(node, OWL_HAS_SELF).is_some() {
            return Verdict::out(
                "owl:hasSelf",
                position,
                "owl:hasSelf is in neither subClassExpression nor superClassExpression in the \
                 OWL 2 RL class grammar, in either position",
            );
        }
        for (pred, spelling) in [
            (OWL_MIN_CARDINALITY, "owl:minCardinality"),
            (OWL_MIN_QUALIFIED_CARDINALITY, "owl:minQualifiedCardinality"),
        ] {
            if let Some(n) = self.cardinality(node, pred) {
                if n == 0 {
                    // `min 0` constrains nothing at all.
                    return Verdict::inside();
                }
                return Verdict::out(
                    spelling,
                    position,
                    format!(
                        "cardinality restriction, and the existential half of one: a minimum of \
                         {n} asserts {n} successors into existence. {WHY_EXISTENTIAL_HEAD}"
                    ),
                );
            }
        }
        for (pred, spelling, qualified) in [
            (OWL_MAX_CARDINALITY, "owl:maxCardinality", false),
            (OWL_MAX_QUALIFIED_CARDINALITY, "owl:maxQualifiedCardinality", true),
        ] {
            if let Some(n) = self.cardinality(node, pred) {
                if position == Position::Antecedent {
                    return Verdict::out(
                        spelling,
                        position,
                        "cardinality restriction in the antecedent: OWL 2 RL admits a maximum \
                         cardinality in superClassExpression only, because counting successors \
                         in a rule BODY needs the closed world it does not assume",
                    );
                }
                if n > 1 {
                    return Verdict::out(
                        spelling,
                        position,
                        format!(
                            "cardinality restriction above 1: a maximum of {n} concludes that \
                             SOME pair among {m} successors is equal, which is a disjunction in \
                             the head. OWL 2 RL admits 0 and 1 and no more",
                            m = n + 1
                        ),
                    );
                }
                // `max 0` and `max 1` are Horn: the head is `false` and
                // `owl:sameAs` respectively. The QUALIFYING class stands in a
                // body position, because the rule reads it to decide whether a
                // successor counts, so it is classified as an antecedent.
                // `owl:onDataRange` needs no recursion: a data range is not a
                // class expression.
                return match self.index.object(node, OWL_ON_CLASS) {
                    // Admitted by the grammar's second alternative,
                    // `'ObjectMaxCardinality' '(' zeroOrOne OPE owl:Thing ')'`,
                    // and axiomatised by cls-maxqc2 and cls-maxqc4, which key
                    // on `T(?x, owl:onClass, owl:Thing)`.
                    Some(on) if qualified && on != OWL_THING => {
                        self.class(&on, Position::Antecedent, seen, depth + 1)
                    }
                    _ => Verdict::inside(),
                };
            }
        }
        for (pred, spelling) in [
            (OWL_CARDINALITY, "owl:cardinality"),
            (OWL_QUALIFIED_CARDINALITY, "owl:qualifiedCardinality"),
        ] {
            if let Some(n) = self.cardinality(node, pred) {
                return Verdict::out(
                    spelling,
                    position,
                    format!(
                        "cardinality restriction: an exact cardinality is a minimum and a maximum \
                         at once, and the minimum half is an existential. OWL 2 RL's class \
                         grammar has ObjectMaxCardinality and no exact form at all, so even \
                         `{spelling} 0`, which IS equivalent to a maximum of 0, is outside it, \
                         and this engine reads no such equivalence. Found {n}"
                    ),
                );
            }
        }
        Verdict::out(
            "an owl:Restriction with no recognised restriction predicate",
            position,
            format!(
                "the node {node} is typed owl:Restriction and carries none of \
                 owl:someValuesFrom, owl:allValuesFrom, owl:hasValue, owl:hasSelf or a \
                 cardinality predicate"
            ),
        )
    }

    fn class_construct(&self, node: &str) -> Option<Construct> {
        if let Some(head) = self.index.object(node, OWL_INTERSECTION_OF) {
            return Some(Construct::Intersection(self.index.walk_list_checked(&head).0));
        }
        if let Some(head) = self.index.object(node, OWL_UNION_OF) {
            return Some(Construct::Union(self.index.walk_list_checked(&head).0));
        }
        if self.index.object(node, OWL_ONE_OF).is_some() {
            // The members are individuals, not class expressions, so nothing
            // recurses into them and the list is not walked.
            return Some(Construct::OneOf);
        }
        if let Some(c) = self.index.object(node, OWL_COMPLEMENT_OF) {
            return Some(Construct::Complement(c));
        }
        if self.index.object(node, OWL_DATATYPE_COMPLEMENT).is_some() {
            return Some(Construct::DataRange("owl:datatypeComplementOf"));
        }
        if self.index.object(node, OWL_WITH_RESTRICTIONS).is_some()
            || self.index.object(node, OWL_ON_DATATYPE).is_some()
        {
            return Some(Construct::DataRange("a datatype facet restriction"));
        }
        if self.has_type(node, OWL_RESTRICTION) || self.index.object(node, OWL_ON_PROPERTY).is_some()
        {
            return Some(Construct::Restriction);
        }
        None
    }
}

enum Construct {
    Intersection(Vec<String>),
    Union(Vec<String>),
    OneOf,
    Complement(String),
    Restriction,
    DataRange(&'static str),
}

// ── Axioms ──────────────────────────────────────────────────────────────────

/// Where an axiom stands against the fragment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Every part of it is a Horn rule over triple patterns.
    Inside,
    /// Part of it is and part of it is not, and the parts separate soundly.
    Partial,
    /// None of it is.
    Outside,
}

impl Standing {
    pub fn word(self) -> &'static str {
        match self {
            Standing::Inside => "inside",
            Standing::Partial => "partially_inside",
            Standing::Outside => "outside",
        }
    }
}

/// One classified axiom.
pub struct Classified {
    pub kind: &'static str,
    pub subject: String,
    pub object: Option<String>,
    pub standing: Standing,
    pub excluded: Vec<Excluded>,
    /// The OWL 2 RL rules that would evaluate an axiom of this shape.
    pub rules: Vec<&'static str>,
    /// Set on the two axiom forms that ARE Horn and are still excluded from
    /// OWL 2 RL, so "outside the fragment" is never said about them without
    /// the qualification.
    pub horn_but_outside_owl2_rl: Option<&'static str>,
}

impl Classified {
    /// The rules that would evaluate this axiom and are not implemented here.
    fn unseen_rules(&self) -> Vec<(&'static str, RuleStatus)> {
        self.rules
            .iter()
            .map(|n| (*n, rule_status(n)))
            .filter(|(_, s)| !s.is_seen())
            .collect()
    }
    fn seen_rules(&self) -> Vec<&'static str> {
        self.rules.iter().copied().filter(|n| rule_status(n).is_seen()).collect()
    }
}

// ── The report ──────────────────────────────────────────────────────────────

/// The most rows of any one list that are printed individually.
///
/// The same bound `onto_defects` uses, for the same reason: one bad pattern in
/// a shared vocabulary produces thousands of identical rows and a reader facing
/// a thousand identical rows decides nothing from them. The totals are still
/// reported, so the cap hides the listing and never the scale.
pub const MAX_LISTED: usize = 50;

#[derive(Default)]
struct Counts {
    total: u64,
    inside: u64,
    partial: u64,
    outside: u64,
    fully_seen: u64,
    rule_not_implemented: u64,
}

pub struct DlpBoundary;

impl DlpBoundary {
    pub fn check(graph: &Arc<GraphStore>) -> anyhow::Result<String> {
        let triples = graph.all_triples()?;
        let total_triples = triples.len();
        let index = TripleIndex::new(&triples);
        let cl = Classifier { index: &index };

        let mut axioms: Vec<Classified> = Vec::new();
        let mut not_classified: BTreeMap<&'static str, u64> = BTreeMap::new();

        for (s, p, o) in &triples {
            match Self::axiom(&cl, s, p, o) {
                Some(a) => axioms.push(a),
                None => *not_classified.entry(Self::unclassified_kind(p, o)).or_default() += 1,
            }
        }

        // ── Fold ────────────────────────────────────────────────────────────
        let mut by_kind: BTreeMap<&'static str, Counts> = BTreeMap::new();
        let mut outside_list: Vec<serde_json::Value> = Vec::new();
        let mut partial_list: Vec<serde_json::Value> = Vec::new();
        let mut unimplemented_list: Vec<serde_json::Value> = Vec::new();
        let mut outside_total = 0u64;
        let mut partial_total = 0u64;
        let mut unimplemented_total = 0u64;
        let mut reason_census: BTreeMap<String, u64> = BTreeMap::new();

        for a in &axioms {
            let c = by_kind.entry(a.kind).or_default();
            c.total += 1;
            match a.standing {
                Standing::Inside => c.inside += 1,
                Standing::Partial => c.partial += 1,
                Standing::Outside => c.outside += 1,
            }
            let unseen = a.unseen_rules();
            let seen = a.seen_rules();
            if a.standing != Standing::Outside && !unseen.is_empty() {
                c.rule_not_implemented += 1;
                unimplemented_total += 1;
                if unimplemented_list.len() < MAX_LISTED {
                    unimplemented_list.push(serde_json::json!({
                        "axiom_type": a.kind,
                        "subject": a.subject,
                        "object": a.object,
                        "standing": a.standing.word(),
                        "rules_that_would_evaluate_it": a.rules,
                        "rules_this_engine_runs": seen,
                        "rules_this_engine_does_not": unseen.iter().map(|(n, s)| {
                            serde_json::json!({
                                "rule": n,
                                "status": s.word(),
                                "why": match s {
                                    RuleStatus::ClashNotDetected(why) => Some(*why),
                                    _ => None,
                                },
                            })
                        }).collect::<Vec<_>>(),
                    }));
                }
            } else if a.standing == Standing::Inside && unseen.is_empty() {
                c.fully_seen += 1;
            }
            // The two are listed apart. A bucket called `outside` that also
            // held the axioms half of which the rules DO evaluate would be
            // saying something false in its own name, which is the failure
            // this whole module is about.
            if a.standing != Standing::Inside {
                for e in &a.excluded {
                    *reason_census.entry(e.reason.clone()).or_default() += 1;
                }
                let entry = serde_json::json!({
                    "axiom_type": a.kind,
                    "subject": a.subject,
                    "object": a.object,
                    "standing": a.standing.word(),
                    "outside_because": a.excluded,
                    "horn_but_outside_owl2_rl": a.horn_but_outside_owl2_rl,
                });
                if a.standing == Standing::Outside {
                    outside_total += 1;
                    if outside_list.len() < MAX_LISTED {
                        outside_list.push(entry);
                    }
                } else {
                    partial_total += 1;
                    if partial_list.len() < MAX_LISTED {
                        partial_list.push(entry);
                    }
                }
            }
        }

        let classified: u64 = by_kind.values().map(|c| c.total).sum();
        let not_fully_seen = outside_total + partial_total + unimplemented_total;

        let by_kind_json: Vec<serde_json::Value> = by_kind
            .iter()
            .map(|(k, c)| {
                serde_json::json!({
                    "axiom_type": k,
                    "total": c.total,
                    "inside": c.inside,
                    "partially_inside": c.partial,
                    "outside": c.outside,
                    "inside_and_every_rule_implemented": c.fully_seen,
                    "inside_but_a_rule_is_not_implemented": c.rule_not_implemented,
                })
            })
            .collect();

        let evaluated = OWL2_RL_RULES
            .iter()
            .filter(|r| matches!(rule_status(r.name), RuleStatus::Evaluated(_)))
            .count();
        let clash_detected = OWL2_RL_RULES
            .iter()
            .filter(|r| rule_status(r.name) == RuleStatus::ClashDetected)
            .count();
        let clash_not_detected = OWL2_RL_RULES
            .iter()
            .filter(|r| matches!(rule_status(r.name), RuleStatus::ClashNotDetected(_)))
            .count();
        let not_implemented: Vec<&str> = OWL2_RL_RULES
            .iter()
            .filter(|r| rule_status(r.name) == RuleStatus::NotImplemented)
            .map(|r| r.name)
            .collect();
        // Per profile, counted rather than written down. A run at `rdfs`
        // evaluates a fraction of the table, and a report that said 29 to such
        // a run would be describing a table the run never used. The profiles
        // nest: `owl-rl` adds to `rdfs` and `owl-rl-ext` adds to `owl-rl`.
        let at_rdfs = crate::reason::RULES_EVALUATED.iter().filter(|(_, p)| *p == "rdfs").count();
        let at_owl_rl = at_rdfs
            + crate::reason::RULES_EVALUATED.iter().filter(|(_, p)| *p == "owl-rl").count();

        let report = serde_json::json!({
            "ok": true,
            "triples": total_triples,
            "axioms_classified": classified,
            "question": "Which of the axioms YOU wrote can the rule engine see? A certificate \
                         from onto_reason is sound about the axioms the rules read, and says \
                         nothing about the ones no rule fires on. This is that second thing",
            "two_dimensions_never_merged": {
                "fragment": "whether the axiom is expressible as Horn rules over triple patterns \
                             at all. `outside` here is a fact about the LANGUAGE and would hold \
                             of a perfect OWL 2 RL engine. The line drawn is OWL 2 RL's class \
                             grammar (OWL 2 Profiles section 4.3): subClassExpression for an \
                             antecedent, superClassExpression for a consequent",
                "engine": "whether a rule in THIS engine's table fires on it. \
                           `inside_but_a_rule_is_not_implemented` is an axiom that a different \
                           engine would see and that no rewriting of your ontology will help. \
                           owl:hasKey and owl:propertyChainAxiom are the two that bite most \
                           often",
                "why_they_are_separate": "one is a rewrite of the ontology and the other is a \
                                          patch to src/reason.rs. A single `not covered` bucket \
                                          destroys the only thing a reader can act on",
            },
            "by_axiom_type": by_kind_json,
            "outside_the_fragment": {
                "count": outside_total,
                "listed": outside_list.len(),
                "truncated": outside_total > outside_list.len() as u64,
                "what_this_means": "NO part of the axiom is a Horn rule over triple patterns. \
                                    This is a fact about the LANGUAGE: it would hold of a \
                                    perfect OWL 2 RL engine, and the fix, if there is one, is a \
                                    rewriting of the ontology",
                "axioms": outside_list,
            },
            "partially_inside_the_fragment": {
                "count": partial_total,
                "listed": partial_list.len(),
                "truncated": partial_total > partial_list.len() as u64,
                "why_not_outside": "part of the axiom IS evaluated and reporting it beside the \
                     wholly-outside ones would say the rules see nothing of it. Two shapes split \
                     soundly: a conjunction in the CONSEQUENT (`A subClassOf (B and Out)` keeps \
                     `A subClassOf B`) and a disjunction in the ANTECEDENT. A conjunction in the \
                     antecedent does NOT split, because dropping a conjunct from a rule body \
                     makes it fire more often, which is unsound and not merely weaker",
                "axioms": partial_list,
            },
            "reasons": reason_census.iter().map(|(r, n)| {
                serde_json::json!({"reason": r, "occurrences": n})
            }).collect::<Vec<_>>(),
            "inside_the_fragment_but_a_rule_is_not_implemented": {
                "count": unimplemented_total,
                "listed": unimplemented_list.len(),
                "truncated": unimplemented_total > unimplemented_list.len() as u64,
                "why_this_is_a_different_failure": "these axioms ARE Horn. OWL 2 RL has a rule \
                     for each of them and this engine does not run it. Rewriting the ontology \
                     will not help and the fix is in src/reason.rs",
                "axioms": unimplemented_list,
            },
            "not_fully_seen_by_the_rule_table": {
                "axioms": not_fully_seen,
                "of_classified": classified,
                "wholly_outside_the_fragment": outside_total,
                "partially_inside_the_fragment": partial_total,
                "inside_but_a_rule_is_not_implemented": unimplemented_total,
                "what_this_means": "a reasoning result over this ontology is sound about the \
                                    axioms the rules read and silent about what these carry. It \
                                    is not wrong; it is narrower than the ontology, and nothing \
                                    else in the pipeline says by how much",
                "why_the_three_are_not_added_into_one_verdict": "they are fixed in three \
                     different places. The first needs the ontology rewritten, the second is \
                     already half seen and the question is which half you needed, and the third \
                     needs a rule added to src/reason.rs and nothing changed about the ontology \
                     at all",
            },
            "rule_table": {
                "owl2_rl_rules": OWL2_RL_RULES.len(),
                "evaluated_in_the_fixpoint": evaluated,
                "detected_as_a_clash": clash_detected,
                "concludes_false_and_not_detected": clash_not_detected,
                "not_implemented": not_implemented.len(),
                "not_implemented_names": not_implemented,
                "source": "the counts are derived from reason::RULES_EVALUATED and \
                           reason::CLASH_RULES_NOT_DETECTED, not typed here, so this figure and \
                           the rule table cannot drift apart",
                "evaluated_by_profile": {
                    "rdfs": at_rdfs,
                    "owl-rl": at_owl_rl,
                    "owl-rl-ext": evaluated,
                    "why": "the profiles NEST, and a run evaluates only its own share. A report \
                            quoting the full table to an `rdfs` run would be describing rules \
                            that run never used. Counted from reason::RULES_EVALUATED, which \
                            carries the profile per rule",
                },
            },
            "not_classified": {
                "why": "this tool asks about the SCHEMA. Class and property assertions, \
                        declarations and annotations are counted here and not classified: every \
                        assertion form in OWL 2 RL is inside the fragment, and an annotation has \
                        no Direct Semantics content at all. Counted rather than passed over in \
                        silence",
                "counts": not_classified.iter().map(|(k, n)| {
                    serde_json::json!({"what": k, "occurrences": n})
                }).collect::<Vec<_>>(),
            },
            "not_a_consistency_check": "this says nothing about whether the ontology is \
                 satisfiable, and nothing about whether OWL 2 RL entails anything. Run \
                 onto_defects for self-defeating declarations and onto_dl_check for \
                 satisfiability",
        });
        Ok(serde_json::to_string(&report)?)
    }

    /// The axiom a triple states, classified, or `None` when the triple is not
    /// a schema axiom this tool reads.
    fn axiom(cl: &Classifier, s: &str, p: &str, o: &str) -> Option<Classified> {
        let mut seen = BTreeSet::new();
        let simple = |kind, rules: Vec<&'static str>| Classified {
            kind,
            subject: s.to_string(),
            object: Some(o.to_string()),
            standing: Standing::Inside,
            excluded: Vec::new(),
            rules,
            horn_but_outside_owl2_rl: None,
        };
        match p {
            RDFS_SUBCLASS => {
                let body = cl.class(s, Position::Antecedent, &mut seen, 0);
                seen.clear();
                let head = cl.class(o, Position::Consequent, &mut seen, 0);
                let mut excluded = body.excluded;
                let any_partial = body.partial || head.partial;
                excluded.extend(head.excluded);
                let standing = if excluded.is_empty() {
                    Standing::Inside
                } else if any_partial {
                    Standing::Partial
                } else {
                    Standing::Outside
                };
                let mut rules = vec!["cax-sco", "scm-sco"];
                rules.extend(Self::construct_rules(cl, s));
                rules.extend(Self::construct_rules(cl, o));
                rules.sort_unstable();
                rules.dedup();
                Some(Classified {
                    kind: "rdfs:subClassOf",
                    subject: s.to_string(),
                    object: Some(o.to_string()),
                    standing,
                    excluded,
                    rules,
                    horn_but_outside_owl2_rl: None,
                })
            }
            OWL_EQUIV_CLASS => {
                // Two axioms, and they are classified separately: `A ≡ ∃r.B`
                // is the textbook case where one direction is a Horn rule
                // (cls-svf1) and the other is an existential in the head.
                let forward = Self::direction(cl, s, o);
                let backward = Self::direction(cl, o, s);
                let standing = match (forward.is_inside(), backward.is_inside()) {
                    (true, true) => Standing::Inside,
                    (false, false) => {
                        if forward.partial || backward.partial {
                            Standing::Partial
                        } else {
                            Standing::Outside
                        }
                    }
                    _ => Standing::Partial,
                };
                let mut excluded = forward.excluded;
                excluded.extend(backward.excluded);
                let mut rules = vec!["cax-eqc1", "cax-eqc2", "scm-eqc1", "scm-eqc2"];
                rules.extend(Self::construct_rules(cl, s));
                rules.extend(Self::construct_rules(cl, o));
                rules.sort_unstable();
                rules.dedup();
                Some(Classified {
                    kind: "owl:equivalentClass",
                    subject: s.to_string(),
                    object: Some(o.to_string()),
                    standing,
                    excluded,
                    rules,
                    horn_but_outside_owl2_rl: None,
                })
            }
            OWL_DISJOINT_WITH => {
                // `A ⊓ B ⊑ ⊥`. A Horn clause with `false` in the head, so it is
                // inside; the operands stand in ANTECEDENT position.
                let mut excluded = cl.class(s, Position::Antecedent, &mut seen, 0).excluded;
                seen.clear();
                excluded.extend(cl.class(o, Position::Antecedent, &mut seen, 0).excluded);
                Some(Classified {
                    kind: "owl:disjointWith",
                    subject: s.to_string(),
                    object: Some(o.to_string()),
                    standing: if excluded.is_empty() { Standing::Inside } else { Standing::Outside },
                    excluded,
                    rules: vec!["cax-dw"],
                    horn_but_outside_owl2_rl: None,
                })
            }
            // Only on a NAMED class. A blank-node subject carrying
            // owl:complementOf is an anonymous class EXPRESSION, an operand of
            // whichever axiom uses it, and is classified there. Reading it as
            // an axiom of its own would report the same exclusion twice and
            // inflate every count in the report.
            OWL_COMPLEMENT_OF if s.starts_with('<') => Some(Classified {
                kind: "owl:complementOf",
                subject: s.to_string(),
                object: Some(o.to_string()),
                // `A ≡ ¬B` is `A ⊓ B ⊑ ⊥` (Horn, cls-com) AND `⊤ ⊑ A ⊔ B`
                // (a disjunction with no antecedent, and not Horn).
                standing: Standing::Partial,
                excluded: vec![Excluded {
                    construct: "owl:complementOf, the covering half".to_string(),
                    position: "consequent",
                    reason: format!(
                        "{WHY_DISJUNCTIVE_HEAD}. A complement says two things: nothing is in both \
                         (which IS Horn, and is what cls-com checks) and everything is in one \
                         (which is `A or B` with no antecedent at all)"
                    ),
                }],
                rules: vec!["cls-com"],
                horn_but_outside_owl2_rl: None,
            }),
            OWL_DISJOINT_UNION => Some(Classified {
                kind: "owl:disjointUnionOf",
                subject: s.to_string(),
                object: Some(o.to_string()),
                standing: Standing::Partial,
                excluded: vec![Excluded {
                    construct: "owl:disjointUnionOf".to_string(),
                    position: "consequent",
                    reason: format!(
                        "{WHY_DISJUNCTIVE_HEAD}. The pairwise disjointness half IS Horn and this \
                         engine reads it only in its owl:disjointWith spelling"
                    ),
                }],
                rules: vec!["cax-dw"],
                horn_but_outside_owl2_rl: Some(
                    "DisjointUnion is excluded from OWL 2 RL as an axiom form, so the profile \
                     rejects it whole, while the disjointness half of what it says is Horn",
                ),
            }),
            RDFS_SUBPROP => Some(simple("rdfs:subPropertyOf", vec!["prp-spo1", "scm-spo"])),
            OWL_EQUIV_PROP => Some(simple(
                "owl:equivalentProperty",
                vec!["prp-eqp1", "prp-eqp2", "scm-eqp1", "scm-eqp2"],
            )),
            RDFS_DOMAIN => {
                let mut c = simple("rdfs:domain", vec!["prp-dom", "scm-dom1", "scm-dom2"]);
                c.excluded = cl.class(o, Position::Consequent, &mut seen, 0).excluded;
                if !c.excluded.is_empty() {
                    c.standing = Standing::Outside;
                }
                Some(c)
            }
            RDFS_RANGE => {
                let mut c = simple("rdfs:range", vec!["prp-rng", "scm-rng1", "scm-rng2"]);
                c.excluded = cl.class(o, Position::Consequent, &mut seen, 0).excluded;
                if !c.excluded.is_empty() {
                    c.standing = Standing::Outside;
                }
                Some(c)
            }
            OWL_INVERSE_OF => Some(simple("owl:inverseOf", vec!["prp-inv1", "prp-inv2"])),
            OWL_PROPERTY_CHAIN => Some(simple("owl:propertyChainAxiom", vec!["prp-spo2"])),
            OWL_PROP_DISJOINT_WITH => {
                Some(simple("owl:propertyDisjointWith", vec!["prp-pdw"]))
            }
            OWL_HAS_KEY => Some(simple("owl:hasKey", vec!["prp-key"])),
            OWL_SAME_AS => Some(simple(
                "owl:sameAs",
                vec!["eq-sym", "eq-trans", "eq-rep-s", "eq-rep-p", "eq-rep-o"],
            )),
            OWL_DIFFERENT_FROM => Some(simple("owl:differentFrom", vec!["eq-diff1"])),
            RDF_TYPE => Self::typed_axiom(s, o),
            _ => None,
        }
    }

    /// The axioms stated by typing a node rather than by a predicate: the six
    /// property characteristics, and the four list-shaped forms.
    ///
    /// The four list-shaped ones are INSIDE the fragment and are exactly where
    /// this engine's rule table stops: `cax-adc`, `prp-adp`, `eq-diff2` and
    /// `eq-diff3` are all in `reason::CLASH_RULES_NOT_DETECTED` because the
    /// clash detector reads no RDF lists. The pairwise spellings of the same
    /// three axioms ARE detected, which is the difference a reader needs.
    fn typed_axiom(s: &str, o: &str) -> Option<Classified> {
        let (kind, rules): (&'static str, Vec<&'static str>) = match o {
            OWL_ALL_DISJOINT_CLASSES => ("owl:AllDisjointClasses", vec!["cax-adc"]),
            OWL_ALL_DISJOINT_PROPS => ("owl:AllDisjointProperties", vec!["prp-adp"]),
            OWL_ALL_DIFFERENT => ("owl:AllDifferent", vec!["eq-diff2", "eq-diff3"]),
            OWL_NEGATIVE_PROP_ASSERTION => {
                ("owl:NegativePropertyAssertion", vec!["prp-npa1", "prp-npa2"])
            }
            _ => return Self::characteristic(s, o),
        };
        Some(Classified {
            kind,
            subject: s.to_string(),
            object: None,
            standing: Standing::Inside,
            excluded: Vec::new(),
            rules,
            horn_but_outside_owl2_rl: None,
        })
    }

    /// One direction of an equivalence, as a subsumption.
    fn direction(cl: &Classifier, sub: &str, sup: &str) -> Verdict {
        let mut seen = BTreeSet::new();
        let body = cl.class(sub, Position::Antecedent, &mut seen, 0);
        seen.clear();
        let head = cl.class(sup, Position::Consequent, &mut seen, 0);
        let mut excluded = body.excluded;
        let partial = body.partial || head.partial;
        excluded.extend(head.excluded);
        Verdict { excluded, partial }
    }

    /// A property-characteristic declaration, as an axiom.
    fn characteristic(s: &str, o: &str) -> Option<Classified> {
        let mk = |kind, rules: Vec<&'static str>, standing, excluded, note| Classified {
            kind,
            subject: s.to_string(),
            object: None,
            standing,
            excluded,
            rules,
            horn_but_outside_owl2_rl: note,
        };
        match o {
            OWL_TRANSITIVE => Some(mk(
                "owl:TransitiveProperty",
                vec!["prp-trp"],
                Standing::Inside,
                Vec::new(),
                None,
            )),
            OWL_SYMMETRIC => Some(mk(
                "owl:SymmetricProperty",
                vec!["prp-symp"],
                Standing::Inside,
                Vec::new(),
                None,
            )),
            OWL_ASYMMETRIC => Some(mk(
                "owl:AsymmetricProperty",
                vec!["prp-asyp"],
                Standing::Inside,
                Vec::new(),
                None,
            )),
            OWL_IRREFLEXIVE => Some(mk(
                "owl:IrreflexiveProperty",
                vec!["prp-irp"],
                Standing::Inside,
                Vec::new(),
                None,
            )),
            OWL_FUNCTIONAL => Some(mk(
                "owl:FunctionalProperty",
                vec!["prp-fp"],
                Standing::Inside,
                Vec::new(),
                None,
            )),
            OWL_INVERSE_FUNCTIONAL => Some(mk(
                "owl:InverseFunctionalProperty",
                vec!["prp-ifp"],
                Standing::Inside,
                Vec::new(),
                None,
            )),
            OWL_REFLEXIVE => Some(mk(
                "owl:ReflexiveProperty",
                Vec::new(),
                Standing::Outside,
                vec![Excluded {
                    construct: "owl:ReflexiveProperty".to_string(),
                    position: "consequent",
                    reason: "ReflexiveObjectProperty is one of the two axiom forms OWL 2 RL \
                             excludes outright, so no rule of the profile evaluates it and there \
                             is nothing for an engine to implement"
                        .to_string(),
                }],
                Some(
                    "reflexivity IS a Horn clause: `thing(x) -> r(x,x)`. It is outside OWL 2 RL \
                     and not outside Horn logic, and saying `outside the fragment` without that \
                     qualification would be false. What OWL 2 RL's exclusion protects is the \
                     unnamed part of the domain, which a rule over the terms in the graph cannot \
                     reach",
                ),
            )),
            _ => None,
        }
    }

    /// The class-construct rules a node contributes, so a `subClassOf` whose
    /// consequent is `all r . B` is reported against `cls-avf` and not only
    /// against `cax-sco`.
    fn construct_rules(cl: &Classifier, node: &str) -> Vec<&'static str> {
        match cl.class_construct(node) {
            Some(Construct::Intersection(_)) => vec!["cls-int1", "cls-int2", "scm-int"],
            Some(Construct::Union(_)) => vec!["cls-uni", "scm-uni"],
            Some(Construct::OneOf) => vec!["cls-oo"],
            Some(Construct::Complement(_)) => vec!["cls-com"],
            Some(Construct::Restriction) => {
                if cl.index.object(node, OWL_SOME_VALUES_FROM).is_some() {
                    vec!["cls-svf1", "cls-svf2", "scm-svf1", "scm-svf2"]
                } else if cl.index.object(node, OWL_ALL_VALUES_FROM).is_some() {
                    vec!["cls-avf", "scm-avf1", "scm-avf2"]
                } else if cl.index.object(node, OWL_HAS_VALUE).is_some() {
                    vec!["cls-hv1", "cls-hv2", "scm-hv"]
                } else if cl.index.object(node, OWL_MAX_CARDINALITY).is_some() {
                    vec!["cls-maxc1", "cls-maxc2"]
                } else if cl.index.object(node, OWL_MAX_QUALIFIED_CARDINALITY).is_some() {
                    vec!["cls-maxqc1", "cls-maxqc2", "cls-maxqc3", "cls-maxqc4"]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    /// What a triple that is not a schema axiom is, so the count of what this
    /// tool did not look at is a NUMBER and not a silence.
    ///
    /// Every triple in the store lands in exactly one of these buckets or in
    /// `axioms`, and `every_triple_lands_somewhere` is the gate that the
    /// partition is total. A tool that reports on a fragment and leaves the
    /// rest uncounted is the shape this whole module exists to attack.
    fn unclassified_kind(p: &str, o: &str) -> &'static str {
        const OWL_NS: &str = "<http://www.w3.org/2002/07/owl#";
        const RDF_NS: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#";
        const RDFS_NS: &str = "<http://www.w3.org/2000/01/rdf-schema#";

        if p == RDF_TYPE {
            if o == OWL_RESTRICTION {
                return "a class expression node, classified with the axiom that uses it";
            }
            if o.starts_with(OWL_NS) || o.starts_with(RDFS_NS) {
                return "a declaration (owl:Class, owl:ObjectProperty and the rest)";
            }
            return "a class assertion";
        }
        // The operand of a class expression, or the RDF list carrying it. Read
        // as PART of the axiom it belongs to rather than as an axiom of its
        // own, which is why it is counted here and classified there.
        if p.starts_with(RDF_NS) {
            return "RDF list machinery inside a class expression or an axiom";
        }
        if matches!(
            p,
            OWL_INTERSECTION_OF
                | OWL_UNION_OF
                | OWL_ONE_OF
                | OWL_ON_PROPERTY
                | OWL_ON_CLASS
                | OWL_ON_DATA_RANGE
                | OWL_SOME_VALUES_FROM
                | OWL_ALL_VALUES_FROM
                | OWL_HAS_VALUE
                | OWL_HAS_SELF
                | OWL_MIN_CARDINALITY
                | OWL_MAX_CARDINALITY
                | OWL_CARDINALITY
                | OWL_MIN_QUALIFIED_CARDINALITY
                | OWL_MAX_QUALIFIED_CARDINALITY
                | OWL_QUALIFIED_CARDINALITY
                | OWL_ON_DATATYPE
                | OWL_WITH_RESTRICTIONS
                | OWL_DATATYPE_COMPLEMENT
                | OWL_COMPLEMENT_OF
        ) {
            return "the body of a class expression, classified with the axiom that uses it";
        }
        if p.starts_with(RDFS_NS) || p.starts_with("<http://www.w3.org/2004/02/skos/core#")
            || p.starts_with("<http://purl.org/dc/")
        {
            return "an annotation, which carries no Direct Semantics content";
        }
        if p.starts_with(OWL_NS) {
            return "other OWL vocabulary, neither a schema axiom this tool reads nor an assertion";
        }
        if o.starts_with('"') {
            return "a data-property assertion";
        }
        "an object-property assertion"
    }
}
