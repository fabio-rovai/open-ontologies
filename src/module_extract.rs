//! Syntactic locality modules: `⊥`, `⊤`, and the iterated `⊥⊤*`.
//!
//! [`crate::segment_retrieve`] produces a SLICE, and [`crate::closure_diff`]
//! measures what a slice lost. This module produces a MODULE, which is a
//! different kind of object: a subset of the axioms carrying a coverage
//! theorem, so there is nothing left to measure afterwards.
//!
//! # The theorem, and whose it is
//!
//! For a signature `Σ`, an axiom is `⊥`-local when replacing every class and
//! property name outside `Σ` by `⊥` turns it into a tautology, and `⊤`-local
//! when the same replacement with `⊤` does. A module is extracted by repeatedly
//! taking every axiom that is NOT local with respect to `Σ ∪ sig(M)`, until the
//! set stops growing. Cuenca Grau, Horrocks, Kazakov and Sattler (JAIR 31,
//! 2008, "Modular Reuse of Ontologies: Theory and Practice") prove that the
//! result `M` satisfies `O ⊨ α iff M ⊨ α` for every axiom `α` over `Σ`, and
//! that alternating the two extractions (`⊥⊤*`) preserves that property while
//! shrinking the result.
//!
//! THAT THEOREM IS CITED, NOT CHECKED HERE. Nothing under `lean/` is about
//! syntactic locality, and this file does not pretend otherwise: the report
//! names a paper and never names a Lean theorem, and [`verify_module`] is
//! supplied so the guarantee can be EXERCISED against the engine's own closure
//! rather than asserted. `tests/module_extract_test.rs` runs that verification
//! over the shipped pizza ontology, which is the difference between "we
//! implemented an algorithm out of a paper" and "we ran it and nothing over the
//! signature was lost".
//!
//! # Conservative wherever it cannot classify
//!
//! An RDF graph is not a list of OWL axioms, so the axioms have to be recovered
//! from triples, and every recovery step has a failure mode. Each one resolves
//! the same way: an axiom this file cannot classify is NEVER local, so it is
//! INCLUDED. That direction is free, because any `M'` with `M ⊆ M' ⊆ O` still
//! has the coverage property — a superset entails everything `M` entails, and
//! `O` entails everything `M'` entails. Including too much costs size.
//! Excluding too much costs the theorem.
//!
//! The counts are in the report and the kinds are named, so "the module is
//! small" and "the module is small because half the file was unreadable" cannot
//! render the same.

use crate::graph::GraphStore;
use crate::projection_entailment::{SKOLEM_PREFIX, Spelled};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

// ───────────────────────────────────────────────────────────────────────────
// Vocabulary
// ───────────────────────────────────────────────────────────────────────────

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// Predicates OWL 2 fixes as annotation properties, plus the RDFS ones. A
/// triple over one of these carries no logical content — UNLESS the ontology
/// itself gives the property a domain, a range, a superproperty or an
/// equivalent, which is legal RDF and makes the assertion load-bearing again.
/// That case is detected in [`Index::build`] rather than assumed away.
const ANNOTATION_PREDICATES: &[&str] = &[
    "http://www.w3.org/2000/01/rdf-schema#label",
    "http://www.w3.org/2000/01/rdf-schema#comment",
    "http://www.w3.org/2000/01/rdf-schema#seeAlso",
    "http://www.w3.org/2000/01/rdf-schema#isDefinedBy",
    "http://www.w3.org/2002/07/owl#versionInfo",
    "http://www.w3.org/2002/07/owl#deprecated",
    "http://www.w3.org/2002/07/owl#priorVersion",
    "http://www.w3.org/2002/07/owl#backwardCompatibleWith",
    "http://www.w3.org/2002/07/owl#incompatibleWith",
];

/// `rdf:type` objects that DECLARE a term rather than assert membership of a
/// user class.
///
/// `owl:Thing` and `rdfs:Resource` are in this list, and the reason is measured
/// rather than aesthetic. `x rdf:type owl:Thing` is a TAUTOLOGY in the OWL 2
/// direct semantics, so the locality test says it is `⊤`-local and drops it,
/// and the paper's theorem is untouched. The OWL 2 RL rule table this engine
/// evaluates does not REGENERATE it, so dropping it removes a member of the
/// closure, and the verification then reports an entailment loss over the
/// signature that is not one. Reading it as a declaration keeps it whenever the
/// individual is in the signature, which makes the DL guarantee and the
/// measured one hold at the same time. Found on
/// `benchmark/reference/pizza-reference.owl`, where five individuals are
/// asserted this way.
const DECLARATION_TYPES: &[&str] = &[
    "http://www.w3.org/2002/07/owl#Thing",
    "http://www.w3.org/2000/01/rdf-schema#Resource",
    "http://www.w3.org/2002/07/owl#Class",
    "http://www.w3.org/2000/01/rdf-schema#Class",
    "http://www.w3.org/2002/07/owl#ObjectProperty",
    "http://www.w3.org/2002/07/owl#DatatypeProperty",
    "http://www.w3.org/2002/07/owl#AnnotationProperty",
    "http://www.w3.org/2002/07/owl#OntologyProperty",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property",
    "http://www.w3.org/2002/07/owl#NamedIndividual",
    "http://www.w3.org/2002/07/owl#Ontology",
    "http://www.w3.org/2000/01/rdf-schema#Datatype",
    "http://www.w3.org/2002/07/owl#DataRange",
];

fn iri_of(term: &str) -> Option<&str> {
    term.strip_prefix('<').and_then(|t| t.strip_suffix('>'))
}
fn is_bnode(term: &str) -> bool {
    term.starts_with("_:")
}
fn is_literal(term: &str) -> bool {
    term.starts_with('"')
}
fn is_builtin(iri: &str) -> bool {
    iri.starts_with(RDF) || iri.starts_with(RDFS) || iri.starts_with(OWL) || iri.starts_with(XSD)
}
/// Allocation-free "is this exactly `owl:<local>`".
fn owl_is(iri: &str, local: &str) -> bool {
    iri.strip_prefix(OWL) == Some(local)
}
fn rdfs_is(iri: &str, local: &str) -> bool {
    iri.strip_prefix(RDFS) == Some(local)
}
fn rdf_is(iri: &str, local: &str) -> bool {
    iri.strip_prefix(RDF) == Some(local)
}

// ───────────────────────────────────────────────────────────────────────────
// Expressions
// ───────────────────────────────────────────────────────────────────────────

/// Which symbol a name outside the signature is replaced by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// `⊥`-locality: an external class becomes `owl:Nothing`, an external
    /// property the empty property.
    Bottom,
    /// `⊤`-locality: an external class becomes `owl:Thing`, an external
    /// property the universal property.
    Top,
}

/// A class expression, as far as it could be read off the RDF.
#[derive(Clone, Debug)]
enum Ce {
    Named(String),
    Thing,
    Nothing,
    /// A datatype or data range. NOT a class name and NOT replaceable, so it is
    /// neither `⊥`-equivalent nor `⊤`-equivalent under any signature. Treating
    /// `xsd:integer` as an external class name and replacing it by `⊥` would
    /// make `∃hasAge.xsd:integer` look local and drop the axiom: unsound, and
    /// invisible, because the module still parses.
    DataRange,
    And(Vec<Ce>),
    Or(Vec<Ce>),
    Not(Box<Ce>),
    /// `owl:oneOf`, carrying only the length: `{a₁ … aₙ}` is `⊥` exactly when
    /// the list is empty, and is never `⊤`.
    OneOf(usize),
    Exists(Pe, Box<Ce>),
    Forall(Pe, Box<Ce>),
    HasValue(Pe),
    HasSelf(Pe),
    Min(u32, Pe, Box<Ce>),
    /// `≤n R.C`. The number is deliberately NOT carried: `≤n R.C` is `⊤`
    /// exactly when `R` is empty or `C` is empty, for EVERY `n`, and it is
    /// never `⊥`, so the bound contributes nothing a locality test can read.
    Max(Pe, Box<Ce>),
    Exact(u32, Pe, Box<Ce>),
    /// A term whose shape this file does not recognise. Neither `⊥` nor `⊤`,
    /// which makes every axiom mentioning it non-local.
    Unreadable,
}

/// A property expression.
#[derive(Clone, Debug)]
enum Pe {
    Named(String),
    /// `owl:inverseOf` applied to a named property. The locality of `R⁻` is the
    /// locality of `R`.
    Inverse(String),
    TopProperty,
    BottomProperty,
    Unreadable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Characteristic {
    Transitive,
    Symmetric,
    Asymmetric,
    Reflexive,
    Irreflexive,
    Functional,
    InverseFunctional,
}

impl Characteristic {
    fn name(self) -> &'static str {
        match self {
            Characteristic::Transitive => "transitive_property",
            Characteristic::Symmetric => "symmetric_property",
            Characteristic::Asymmetric => "asymmetric_property",
            Characteristic::Reflexive => "reflexive_property",
            Characteristic::Irreflexive => "irreflexive_property",
            Characteristic::Functional => "functional_property",
            Characteristic::InverseFunctional => "inverse_functional_property",
        }
    }
}

/// The OWL 2 axiom a root triple was read as.
#[derive(Clone, Debug)]
enum Form {
    SubClassOf(Ce, Ce),
    EquivalentClasses(Vec<Ce>),
    DisjointClasses(Vec<Ce>),
    DisjointUnion(Ce, Vec<Ce>),
    SubPropertyOf(Pe, Pe),
    PropertyChain(Vec<Pe>, Pe),
    EquivalentProperties(Vec<Pe>),
    DisjointProperties(Vec<Pe>),
    Domain(Pe, Ce),
    Range(Pe, Ce),
    InverseProperties(Pe, Pe),
    Characteristic(Characteristic, Pe),
    HasKey(Ce),
    ClassAssertion(Ce),
    PropertyAssertion(Pe),
    NegativePropertyAssertion(Pe),
    /// `owl:sameAs`, `owl:differentFrom`, `owl:AllDifferent`. These mention no
    /// class and no property name, so NO replacement can make them tautologies
    /// and they are never local.
    IndividualIdentity(&'static str),
    /// `X rdf:type owl:Class` and friends. Vacuous in the OWL 2 direct
    /// semantics; NOT vacuous for the OWL 2 RL rule table this engine
    /// evaluates, where `scm-cls` reads it as a premise. Kept whenever the
    /// declared term is in the working signature, which is the conservative
    /// reading of the two.
    Declaration(String),
    Annotation,
    Unclassified(String),
}

impl Form {
    fn kind(&self) -> &'static str {
        match self {
            Form::SubClassOf(..) => "sub_class_of",
            Form::EquivalentClasses(..) => "equivalent_classes",
            Form::DisjointClasses(..) => "disjoint_classes",
            Form::DisjointUnion(..) => "disjoint_union",
            Form::SubPropertyOf(..) => "sub_property_of",
            Form::PropertyChain(..) => "property_chain",
            Form::EquivalentProperties(..) => "equivalent_properties",
            Form::DisjointProperties(..) => "disjoint_properties",
            Form::Domain(..) => "property_domain",
            Form::Range(..) => "property_range",
            Form::InverseProperties(..) => "inverse_properties",
            Form::Characteristic(k, _) => k.name(),
            Form::HasKey(..) => "has_key",
            Form::ClassAssertion(..) => "class_assertion",
            Form::PropertyAssertion(..) => "property_assertion",
            Form::NegativePropertyAssertion(..) => "negative_property_assertion",
            Form::IndividualIdentity(w) => w,
            Form::Declaration(..) => "declaration",
            Form::Annotation => "annotation",
            Form::Unclassified(..) => "unclassified",
        }
    }
}

/// One axiom: the triples that spell it, the form it was read as, and the
/// non-logical names it mentions.
struct Axiom {
    triples: Vec<Spelled>,
    form: Form,
    signature: BTreeSet<String>,
    /// The root triple, rendered, for the unclassified report.
    root: String,
}

// ───────────────────────────────────────────────────────────────────────────
// Locality
// ───────────────────────────────────────────────────────────────────────────

type Sig = BTreeSet<String>;

fn c_bottom(c: &Ce, sig: &Sig, side: Side) -> bool {
    match c {
        Ce::Named(a) => side == Side::Bottom && !sig.contains(a),
        Ce::Thing => false,
        Ce::Nothing => true,
        Ce::DataRange => false,
        // An empty intersection is `owl:Thing`, so `any` is right: false on the
        // empty list.
        Ce::And(v) => v.iter().any(|x| c_bottom(x, sig, side)),
        // An empty union is `owl:Nothing`, so `all` is right: true on the empty
        // list.
        Ce::Or(v) => v.iter().all(|x| c_bottom(x, sig, side)),
        Ce::Not(x) => c_top(x, sig, side),
        Ce::OneOf(n) => *n == 0,
        Ce::Exists(p, f) => p_bottom(p, sig, side) || c_bottom(f, sig, side),
        // `∀R.⊥` is "has no R-successor", which is satisfiable, so a universal
        // restriction is never bottom-equivalent.
        Ce::Forall(..) => false,
        Ce::HasValue(p) => p_bottom(p, sig, side),
        Ce::HasSelf(p) => p_bottom(p, sig, side),
        Ce::Min(n, p, f) => *n >= 1 && (p_bottom(p, sig, side) || c_bottom(f, sig, side)),
        Ce::Max(..) => false,
        Ce::Exact(n, p, f) => *n >= 1 && (p_bottom(p, sig, side) || c_bottom(f, sig, side)),
        Ce::Unreadable => false,
    }
}

fn c_top(c: &Ce, sig: &Sig, side: Side) -> bool {
    match c {
        Ce::Named(a) => side == Side::Top && !sig.contains(a),
        Ce::Thing => true,
        Ce::Nothing => false,
        Ce::DataRange => false,
        Ce::And(v) => v.iter().all(|x| c_top(x, sig, side)),
        Ce::Or(v) => v.iter().any(|x| c_top(x, sig, side)),
        Ce::Not(x) => c_bottom(x, sig, side),
        Ce::OneOf(_) => false,
        Ce::Exists(p, f) => p_top(p, sig, side) && c_top(f, sig, side),
        Ce::Forall(p, f) => c_top(f, sig, side) || p_bottom(p, sig, side),
        Ce::HasValue(p) => p_top(p, sig, side),
        Ce::HasSelf(p) => p_top(p, sig, side),
        // `≥0 R.C` is `⊤`, and `≥1 ⊤.⊤` is `⊤` because the domain is non-empty.
        // `≥2 ⊤.⊤` is NOT, in a one-element domain, so it is left non-top.
        Ce::Min(n, p, f) => *n == 0 || (*n == 1 && p_top(p, sig, side) && c_top(f, sig, side)),
        Ce::Max(p, f) => p_bottom(p, sig, side) || c_bottom(f, sig, side),
        Ce::Exact(n, p, f) => *n == 0 && (p_bottom(p, sig, side) || c_bottom(f, sig, side)),
        Ce::Unreadable => false,
    }
}

fn p_bottom(p: &Pe, sig: &Sig, side: Side) -> bool {
    match p {
        Pe::Named(x) | Pe::Inverse(x) => side == Side::Bottom && !sig.contains(x),
        Pe::TopProperty => false,
        Pe::BottomProperty => true,
        Pe::Unreadable => false,
    }
}

fn p_top(p: &Pe, sig: &Sig, side: Side) -> bool {
    match p {
        Pe::Named(x) | Pe::Inverse(x) => side == Side::Top && !sig.contains(x),
        Pe::TopProperty => true,
        Pe::BottomProperty => false,
        Pe::Unreadable => false,
    }
}

/// Is this axiom a tautology once every name outside `sig` has been replaced?
fn is_local(a: &Axiom, sig: &Sig, side: Side) -> bool {
    let cb = |c: &Ce| c_bottom(c, sig, side);
    let ct = |c: &Ce| c_top(c, sig, side);
    let pb = |p: &Pe| p_bottom(p, sig, side);
    let pt = |p: &Pe| p_top(p, sig, side);
    match &a.form {
        Form::SubClassOf(c, d) => cb(c) || ct(d),
        Form::EquivalentClasses(v) => v.iter().all(cb) || v.iter().all(ct),
        // `Disjoint(C₁ … Cₙ)` is a tautology when at most one operand is
        // non-empty: nothing overlaps `⊥`.
        Form::DisjointClasses(v) => v.iter().filter(|c| !cb(c)).count() <= 1,
        Form::DisjointUnion(a, v) => cb(a) && v.iter().all(cb),
        Form::SubPropertyOf(p, q) => pb(p) || pt(q),
        Form::PropertyChain(chain, q) => pt(q) || chain.iter().any(pb),
        Form::EquivalentProperties(v) => v.iter().all(pb) || v.iter().all(pt),
        Form::DisjointProperties(v) => v.iter().filter(|p| !pb(p)).count() <= 1,
        Form::Domain(p, c) => pb(p) || ct(c),
        Form::Range(p, c) => pb(p) || ct(c),
        Form::InverseProperties(p, q) => (pb(p) && pb(q)) || (pt(p) && pt(q)),
        Form::Characteristic(k, p) => match k {
            // The universal property is transitive and symmetric; the empty one
            // vacuously is.
            Characteristic::Transitive | Characteristic::Symmetric => pb(p) || pt(p),
            // Only the universal property is reflexive.
            Characteristic::Reflexive => pt(p),
            // The universal property is none of these; the empty one is all of
            // them, vacuously.
            Characteristic::Asymmetric
            | Characteristic::Irreflexive
            | Characteristic::Functional
            | Characteristic::InverseFunctional => pb(p),
        },
        Form::HasKey(c) => cb(c),
        Form::ClassAssertion(c) => ct(c),
        Form::PropertyAssertion(p) => pt(p),
        Form::NegativePropertyAssertion(p) => pb(p),
        Form::IndividualIdentity(_) => false,
        // The empty name belongs to a declaration whose subject is not an IRI,
        // which is not a shape this file reads. `!sig.contains("")` would be
        // true against EVERY signature, so the empty case is spelled out rather
        // than left to fall through to "local".
        Form::Declaration(iri) => !iri.is_empty() && !sig.contains(iri),
        Form::Annotation => true,
        Form::Unclassified(_) => false,
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Reading axioms out of RDF
// ───────────────────────────────────────────────────────────────────────────

/// How deep a class expression is followed before it is declared unreadable. A
/// cyclic blank-node structure is illegal OWL and legal RDF, so this is a
/// termination guard rather than an expressiveness limit.
const MAX_DEPTH: usize = 40;

/// Longest `rdf:List` followed. Beyond it the list is unreadable, which makes
/// the axiom non-local, which includes it.
const MAX_LIST: usize = 5_000;

struct Index {
    triples: Vec<Spelled>,
    by_subject: HashMap<String, Vec<usize>>,
    object_terms: HashSet<String>,
    /// Annotation predicates this ontology has given logical force, by
    /// declaring a domain, a range, a superproperty or an equivalent for them.
    /// Assertions over these are read as property assertions, not annotations.
    loaded_annotation_predicates: BTreeSet<String>,
}

impl Index {
    fn build(store: &Arc<GraphStore>) -> anyhow::Result<Index> {
        let triples = store.all_triples()?;
        let mut by_subject: HashMap<String, Vec<usize>> = HashMap::new();
        let mut object_terms: HashSet<String> = HashSet::new();
        for (i, (s, _, o)) in triples.iter().enumerate() {
            by_subject.entry(s.clone()).or_default().push(i);
            object_terms.insert(o.clone());
        }
        // An annotation property with a domain, a range, a superproperty or an
        // equivalent is not an annotation property any more, whatever OWL 2
        // says: the rule table will read assertions over it.
        let mut loaded_annotation_predicates = BTreeSet::new();
        for (s, p, _) in &triples {
            let (Some(si), Some(pi)) = (iri_of(s), iri_of(p)) else { continue };
            let loading = rdfs_is(pi, "domain")
                || rdfs_is(pi, "range")
                || rdfs_is(pi, "subPropertyOf")
                || owl_is(pi, "equivalentProperty");
            if loading && ANNOTATION_PREDICATES.contains(&si) {
                loaded_annotation_predicates.insert(si.to_string());
            }
        }
        Ok(Index { triples, by_subject, object_terms, loaded_annotation_predicates })
    }

    /// Triple indices with this subject. Elision ties the borrow to `&self`,
    /// not to `subject`, which is what lets the results be returned.
    fn rows(&self, subject: &str) -> &[usize] {
        self.by_subject.get(subject).map(Vec::as_slice).unwrap_or(&[])
    }

    /// The first object of `subject <predicate> ?`, where `predicate` is
    /// matched by a closure so no IRI has to be allocated to look one up.
    fn object_where(&self, subject: &str, matches: impl Fn(&str) -> bool) -> Option<&str> {
        self.rows(subject)
            .iter()
            .map(|i| &self.triples[*i])
            .find(|t| iri_of(&t.1).is_some_and(&matches))
            .map(|t| t.2.as_str())
    }

    fn owl_object(&self, subject: &str, local: &'static str) -> Option<&str> {
        self.object_where(subject, move |p| owl_is(p, local))
    }

    fn has_type(&self, subject: &str, is_type: impl Fn(&str) -> bool) -> bool {
        self.rows(subject).iter().any(|i| {
            let t = &self.triples[*i];
            iri_of(&t.1).is_some_and(|p| rdf_is(p, "type"))
                && iri_of(&t.2).is_some_and(&is_type)
        })
    }

    /// Follow an `rdf:List`. `None` when the structure is not a well-formed
    /// list, which makes the containing axiom unreadable and therefore
    /// non-local.
    fn list(&self, head: &str) -> Option<Vec<String>> {
        let mut out = Vec::new();
        let mut cur = head.to_string();
        let mut seen: HashSet<String> = HashSet::new();
        while !iri_of(&cur).is_some_and(|i| rdf_is(i, "nil")) {
            if out.len() > MAX_LIST || !seen.insert(cur.clone()) {
                return None;
            }
            let f = self.object_where(&cur, |p| rdf_is(p, "first"))?.to_string();
            out.push(f);
            cur = self.object_where(&cur, |p| rdf_is(p, "rest"))?.to_string();
        }
        Some(out)
    }

    /// True when the term denotes a data range rather than a class.
    fn is_data_range(&self, term: &str) -> bool {
        if is_literal(term) {
            return true;
        }
        if let Some(i) = iri_of(term) {
            return i.starts_with(XSD) || rdfs_is(i, "Literal");
        }
        if is_bnode(term) {
            return self.has_type(term, |t| rdfs_is(t, "Datatype"))
                || self.owl_object(term, "onDatatype").is_some()
                || self.owl_object(term, "datatypeComplementOf").is_some()
                || self.owl_object(term, "withRestrictions").is_some();
        }
        false
    }

    fn property_expr(&self, term: &str) -> Pe {
        if let Some(i) = iri_of(term) {
            if owl_is(i, "topObjectProperty") || owl_is(i, "topDataProperty") {
                return Pe::TopProperty;
            }
            if owl_is(i, "bottomObjectProperty") || owl_is(i, "bottomDataProperty") {
                return Pe::BottomProperty;
            }
            return Pe::Named(i.to_string());
        }
        if is_bnode(term)
            && let Some(inner) = self.owl_object(term, "inverseOf")
            && let Some(i) = iri_of(inner)
        {
            return Pe::Inverse(i.to_string());
        }
        Pe::Unreadable
    }

    fn class_expr(&self, term: &str, depth: usize) -> Ce {
        if depth > MAX_DEPTH {
            return Ce::Unreadable;
        }
        if self.is_data_range(term) {
            return Ce::DataRange;
        }
        if let Some(i) = iri_of(term) {
            if owl_is(i, "Thing") {
                return Ce::Thing;
            }
            if owl_is(i, "Nothing") {
                return Ce::Nothing;
            }
            return Ce::Named(i.to_string());
        }
        if !is_bnode(term) {
            return Ce::Unreadable;
        }
        for (pred, wrap) in [
            ("intersectionOf", true),
            ("unionOf", false),
        ] {
            if let Some(head) = self.owl_object(term, pred) {
                let Some(items) = self.list(head) else { return Ce::Unreadable };
                let v: Vec<Ce> = items.iter().map(|t| self.class_expr(t, depth + 1)).collect();
                return if wrap { Ce::And(v) } else { Ce::Or(v) };
            }
        }
        if let Some(c) = self.owl_object(term, "complementOf") {
            return Ce::Not(Box::new(self.class_expr(c, depth + 1)));
        }
        if let Some(head) = self.owl_object(term, "oneOf") {
            return match self.list(head) {
                Some(items) => Ce::OneOf(items.len()),
                None => Ce::Unreadable,
            };
        }
        // A restriction. `owl:onProperties` is the n-ary data form and is not
        // read here, so it falls through to Unreadable and is included.
        let Some(on_property) = self.owl_object(term, "onProperty") else {
            return Ce::Unreadable;
        };
        let p = self.property_expr(on_property);
        if let Some(f) = self.owl_object(term, "someValuesFrom") {
            return Ce::Exists(p, Box::new(self.class_expr(f, depth + 1)));
        }
        if let Some(f) = self.owl_object(term, "allValuesFrom") {
            return Ce::Forall(p, Box::new(self.class_expr(f, depth + 1)));
        }
        if self.owl_object(term, "hasValue").is_some() {
            return Ce::HasValue(p);
        }
        if self.owl_object(term, "hasSelf").is_some() {
            return Ce::HasSelf(p);
        }
        let qualifier = self
            .owl_object(term, "onClass")
            .or_else(|| self.owl_object(term, "onDataRange"))
            .map(|f| self.class_expr(f, depth + 1))
            .unwrap_or(Ce::Thing);
        let card = |local: &'static str| -> Option<u32> {
            let raw = self.owl_object(term, local)?;
            raw.strip_prefix('"')?.split('"').next()?.parse::<u32>().ok()
        };
        for (unqualified, qualified, build) in [
            ("minCardinality", "minQualifiedCardinality", 0u8),
            ("maxCardinality", "maxQualifiedCardinality", 1),
            ("cardinality", "qualifiedCardinality", 2),
        ] {
            let found = card(unqualified)
                .map(|n| (n, Ce::Thing))
                .or_else(|| card(qualified).map(|n| (n, qualifier.clone())));
            if let Some((n, f)) = found {
                return match build {
                    0 => Ce::Min(n, p, Box::new(f)),
                    1 => Ce::Max(p, Box::new(f)),
                    _ => Ce::Exact(n, p, Box::new(f)),
                };
            }
        }
        Ce::Unreadable
    }

    /// Every triple the axiom rooted at `root_index` spells: the root, plus the
    /// transitive blank-node structure it reaches.
    fn axiom_triples(&self, root_index: usize) -> Vec<Spelled> {
        let mut out: BTreeSet<Spelled> = BTreeSet::new();
        let root = &self.triples[root_index];
        out.insert(root.clone());
        let mut seen: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = [&root.0, &root.2]
            .into_iter()
            .filter(|t| is_bnode(t))
            .cloned()
            .collect();
        while let Some(b) = stack.pop() {
            if !seen.insert(b.clone()) {
                continue;
            }
            for i in self.rows(&b) {
                let t = &self.triples[*i];
                out.insert(t.clone());
                if is_bnode(&t.2) {
                    stack.push(t.2.clone());
                }
            }
        }
        out.into_iter().collect()
    }
}

/// The non-logical names a set of triples mentions.
fn signature_of(triples: &[Spelled]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (s, p, o) in triples {
        for t in [s, p, o] {
            if let Some(i) = iri_of(t)
                && !is_builtin(i)
            {
                out.insert(i.to_string());
            }
        }
    }
    out
}

fn render(t: &Spelled) -> String {
    format!("{} {} {}", t.0, t.1, t.2)
}

/// Read the store as a list of axioms.
fn axioms_of(ix: &Index) -> Vec<Axiom> {
    let mut out = Vec::new();
    for i in 0..ix.triples.len() {
        let (s, p, o) = &ix.triples[i];
        // A blank node some other triple points at is part of the axiom that
        // points at it, not an axiom of its own.
        if is_bnode(s) && ix.object_terms.contains(s) {
            continue;
        }
        let Some(pi) = iri_of(p) else { continue };
        let form = if is_bnode(s) {
            bnode_rooted_form(ix, s, pi, o, i)
        } else {
            let mut f = iri_rooted_form(ix, pi, o);
            if let Some(f) = f.as_mut() {
                finish_subject(ix, f, s);
            }
            f
        };
        let Some(form) = form else { continue };
        let triples = ix.axiom_triples(i);
        let signature = signature_of(&triples);
        out.push(Axiom { triples, form, signature, root: render(&ix.triples[i]) });
    }
    out
}

/// A blank-node-rooted axiom: `owl:AllDisjointClasses` and friends.
///
/// Exactly ONE triple of the structure becomes the axiom — the `rdf:type` one —
/// so the structure is not read once per triple. A structure with no
/// `rdf:type` at all is rooted at its first triple and reported unclassified,
/// which includes it.
fn bnode_rooted_form(ix: &Index, s: &str, pi: &str, o: &str, row: usize) -> Option<Form> {
    if !rdf_is(pi, "type") {
        if ix.has_type(s, |_| true) {
            return None;
        }
        if ix.rows(s).first() != Some(&row) {
            return None;
        }
        return Some(Form::Unclassified(format!("orphan blank-node structure on <{pi}>")));
    }
    let oi = iri_of(o).unwrap_or("");
    let members = || ix.owl_object(s, "members").and_then(|h| ix.list(h));
    if owl_is(oi, "AllDisjointClasses") {
        return Some(match members() {
            Some(v) => Form::DisjointClasses(v.iter().map(|t| ix.class_expr(t, 0)).collect()),
            None => {
                Form::Unclassified("owl:AllDisjointClasses with an unreadable member list".into())
            }
        });
    }
    if owl_is(oi, "AllDisjointProperties") {
        return Some(match members() {
            Some(v) => Form::DisjointProperties(v.iter().map(|t| ix.property_expr(t)).collect()),
            None => {
                Form::Unclassified("owl:AllDisjointProperties with an unreadable member list".into())
            }
        });
    }
    if owl_is(oi, "AllDifferent") {
        return Some(Form::IndividualIdentity("different_individuals"));
    }
    if owl_is(oi, "NegativePropertyAssertion") {
        return Some(match ix.owl_object(s, "assertionProperty") {
            Some(p) => Form::NegativePropertyAssertion(ix.property_expr(p)),
            None => Form::Unclassified(
                "owl:NegativePropertyAssertion with no owl:assertionProperty".into(),
            ),
        });
    }
    if owl_is(oi, "Axiom") {
        // Axiom reification annotates an axiom that is also asserted in its own
        // right, so it carries no consequence of its own.
        return Some(Form::Annotation);
    }
    Some(Form::Unclassified(format!("blank-node-rooted structure of type <{oi}>")))
}

fn iri_rooted_form(ix: &Index, pi: &str, o: &str) -> Option<Form> {
    let ce = |t: &str| ix.class_expr(t, 0);
    let pe = |t: &str| ix.property_expr(t);
    if rdfs_is(pi, "subClassOf") {
        return Some(Form::SubClassOf(Ce::Unreadable, ce(o)));
    }
    if owl_is(pi, "equivalentClass") {
        return Some(Form::EquivalentClasses(vec![Ce::Unreadable, ce(o)]));
    }
    if owl_is(pi, "disjointWith") {
        return Some(Form::DisjointClasses(vec![Ce::Unreadable, ce(o)]));
    }
    if owl_is(pi, "disjointUnionOf") {
        return Some(match ix.list(o) {
            Some(v) => Form::DisjointUnion(Ce::Unreadable, v.iter().map(|x| ce(x)).collect()),
            None => Form::Unclassified("owl:disjointUnionOf with an unreadable list".into()),
        });
    }
    if rdfs_is(pi, "subPropertyOf") {
        return Some(Form::SubPropertyOf(Pe::Unreadable, pe(o)));
    }
    if owl_is(pi, "propertyChainAxiom") {
        return Some(match ix.list(o) {
            Some(v) => Form::PropertyChain(v.iter().map(|x| pe(x)).collect(), Pe::Unreadable),
            None => Form::Unclassified("owl:propertyChainAxiom with an unreadable list".into()),
        });
    }
    if owl_is(pi, "equivalentProperty") {
        return Some(Form::EquivalentProperties(vec![Pe::Unreadable, pe(o)]));
    }
    if owl_is(pi, "propertyDisjointWith") {
        return Some(Form::DisjointProperties(vec![Pe::Unreadable, pe(o)]));
    }
    if rdfs_is(pi, "domain") {
        return Some(Form::Domain(Pe::Unreadable, ce(o)));
    }
    if rdfs_is(pi, "range") {
        return Some(Form::Range(Pe::Unreadable, ce(o)));
    }
    if owl_is(pi, "inverseOf") {
        return Some(Form::InverseProperties(Pe::Unreadable, pe(o)));
    }
    if owl_is(pi, "hasKey") {
        return Some(Form::HasKey(Ce::Unreadable));
    }
    if owl_is(pi, "sameAs") {
        return Some(Form::IndividualIdentity("same_individual"));
    }
    if owl_is(pi, "differentFrom") {
        return Some(Form::IndividualIdentity("different_individuals"));
    }
    if rdf_is(pi, "type") {
        return Some(type_form(ix, o));
    }
    if ANNOTATION_PREDICATES.contains(&pi) && !ix.loaded_annotation_predicates.contains(pi) {
        return Some(Form::Annotation);
    }
    // Anything left in the RDF, RDFS, OWL or XSD namespaces is OWL vocabulary
    // this file does not classify. It is NOT read as a property assertion,
    // because an unrecognised `owl:` predicate could carry logical force, and
    // reading it as an assertion would make it `⊤`-local and drop it.
    if is_builtin(pi) {
        return Some(Form::Unclassified(format!("unclassified built-in predicate <{pi}>")));
    }
    Some(Form::PropertyAssertion(Pe::Named(pi.to_string())))
}

/// `X rdf:type Y`: a declaration, a property characteristic, or a class
/// assertion.
fn type_form(ix: &Index, o: &str) -> Form {
    let Some(oi) = iri_of(o) else {
        return Form::ClassAssertion(ix.class_expr(o, 0));
    };
    if DECLARATION_TYPES.contains(&oi) {
        // The declared term is the subject, filled in by `finish_subject`.
        return Form::Declaration(String::new());
    }
    for (local, k) in [
        ("TransitiveProperty", Characteristic::Transitive),
        ("SymmetricProperty", Characteristic::Symmetric),
        ("AsymmetricProperty", Characteristic::Asymmetric),
        ("ReflexiveProperty", Characteristic::Reflexive),
        ("IrreflexiveProperty", Characteristic::Irreflexive),
        ("FunctionalProperty", Characteristic::Functional),
        ("InverseFunctionalProperty", Characteristic::InverseFunctional),
    ] {
        if owl_is(oi, local) {
            return Form::Characteristic(k, Pe::Unreadable);
        }
    }
    // `owl:Restriction` and the other structural types reached at a ROOT mean a
    // blank node nothing points at, or an IRI used as one: not a shape this
    // file reads, so it is included rather than guessed at.
    if oi.starts_with(OWL) && !owl_is(oi, "Thing") && !owl_is(oi, "Nothing") {
        return Form::Unclassified(format!("rdf:type <{oi}> at an axiom root"));
    }
    Form::ClassAssertion(ix.class_expr(o, 0))
}

/// Fill in the subject side of a form parsed from the object side only.
/// Splitting it this way keeps ONE place where the subject is read.
fn finish_subject(ix: &Index, form: &mut Form, subject: &str) {
    match form {
        Form::SubClassOf(c, _) => *c = ix.class_expr(subject, 0),
        Form::EquivalentClasses(v) | Form::DisjointClasses(v) => v[0] = ix.class_expr(subject, 0),
        Form::DisjointUnion(c, _) | Form::HasKey(c) => *c = ix.class_expr(subject, 0),
        Form::SubPropertyOf(p, _) => *p = ix.property_expr(subject),
        Form::PropertyChain(_, q) => *q = ix.property_expr(subject),
        Form::EquivalentProperties(v) | Form::DisjointProperties(v) => {
            v[0] = ix.property_expr(subject)
        }
        Form::Domain(p, _) | Form::Range(p, _) => *p = ix.property_expr(subject),
        Form::InverseProperties(p, _) => *p = ix.property_expr(subject),
        Form::Characteristic(_, p) => *p = ix.property_expr(subject),
        Form::Declaration(name) => *name = iri_of(subject).unwrap_or_default().to_string(),
        _ => {}
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Extraction
// ───────────────────────────────────────────────────────────────────────────

/// Which module to extract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Locality {
    Bottom,
    Top,
    /// The iterated `⊥⊤*`: alternate the two until the set stops shrinking.
    /// Never larger than either, and usually smaller than both.
    Star,
}

impl Locality {
    pub fn parse(s: &str) -> anyhow::Result<Locality> {
        match s {
            "bottom" | "⊥" => Ok(Locality::Bottom),
            "top" | "⊤" => Ok(Locality::Top),
            "star" | "bottom-top-star" | "⊥⊤*" => Ok(Locality::Star),
            other => anyhow::bail!(
                "unknown locality {other:?}: pass \"bottom\", \"top\" or \"star\" (the iterated ⊥⊤*)"
            ),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Locality::Bottom => "bottom",
            Locality::Top => "top",
            Locality::Star => "star",
        }
    }
    pub fn symbol(self) -> &'static str {
        match self {
            Locality::Bottom => "⊥",
            Locality::Top => "⊤",
            Locality::Star => "⊥⊤*",
        }
    }
}

/// One extraction pass, run to a fixpoint.
///
/// The fixpoint is not an optimisation, it is the SOUNDNESS CONDITION. The
/// working signature grows with every axiom taken, and an axiom that looked
/// local against `Σ` stops being local once a name it mentions has been pulled
/// in. Stopping after one sweep would drop `a p₁ b` from a module that kept
/// `p₁ ⊑ p₂`, and so lose `a p₂ b` over a signature containing `p₂`, `a` and
/// `b`. `the_top_pass_iterates_rather_than_sweeping_once` pins the case.
/// An axiom's locality depends only on which of the names IT MENTIONS are in
/// the signature, so it can only stop being local when one of those names is
/// added. That is what makes the index below correct and the naive rescan
/// unnecessary: re-sweeping every axiom after every addition is quadratic, and
/// an ontology of a hundred thousand axioms is not a hypothetical.
fn extract_once(axioms: &[Axiom], over: &[usize], sigma: &Sig, side: Side) -> Vec<usize> {
    let candidates: HashSet<usize> = over.iter().copied().collect();
    let mut by_symbol: HashMap<&str, Vec<usize>> = HashMap::new();
    for &i in over {
        for s in &axioms[i].signature {
            by_symbol.entry(s.as_str()).or_default().push(i);
        }
    }
    let mut work = sigma.clone();
    let mut taken: HashSet<usize> = HashSet::new();
    // Everything is tested once; after that, only what a newly admitted name
    // could have changed.
    let mut queue: Vec<usize> = over.to_vec();
    while let Some(i) = queue.pop() {
        if !candidates.contains(&i) || taken.contains(&i) || is_local(&axioms[i], &work, side) {
            continue;
        }
        taken.insert(i);
        for s in &axioms[i].signature {
            if work.insert(s.clone())
                && let Some(v) = by_symbol.get(s.as_str())
            {
                queue.extend(v.iter().copied());
            }
        }
    }
    let mut out: Vec<usize> = taken.into_iter().collect();
    out.sort_unstable();
    out
}

// ───────────────────────────────────────────────────────────────────────────
// Options and report
// ───────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ModuleOptions {
    /// The IRIs the module must cover.
    pub signature: Vec<String>,
    pub locality: Locality,
    /// Re-attach `rdfs:label`, `rdfs:comment` and the other annotation
    /// assertions for terms the module keeps. They are NOT part of the logical
    /// module and are counted separately, so the reported module size stays the
    /// logical one.
    pub include_annotations: bool,
    /// Rows rendered per list. Totals are always exact.
    pub max_rows: usize,
}

impl Default for ModuleOptions {
    fn default() -> Self {
        ModuleOptions {
            signature: Vec::new(),
            locality: Locality::Star,
            include_annotations: false,
            max_rows: 200,
        }
    }
}

/// An axiom taken into the module because this file could not classify it.
#[derive(Clone, Debug, Serialize)]
pub struct UnclassifiedAxiom {
    pub root_triple: String,
    pub why: String,
    pub triples: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModuleReport {
    pub format: &'static str,
    /// One sentence the reader cannot miss, first after `format`.
    pub headline: String,
    pub locality: &'static str,
    pub locality_symbol: &'static str,

    /// What the module guarantees, and whose theorem it is.
    pub guarantee: &'static str,
    /// The word that is NOT used, spelled out so it cannot be inferred.
    pub guarantee_is_not_machine_checked: &'static str,
    /// Axiom kinds whose locality was decided by a test written for that kind.
    pub guarantee_covers: Vec<&'static str>,
    /// Axiom kinds taken into the module WITHOUT a locality test, because no
    /// replacement could make them tautologies or because they could not be
    /// classified. Including is the safe direction; the counts are here so that
    /// a small module and an unreadable one cannot render the same.
    pub included_conservatively: BTreeMap<String, usize>,
    pub unclassified_axioms: Vec<UnclassifiedAxiom>,
    pub unclassified_total: usize,

    pub signature_requested: Vec<String>,
    /// Names in `signature_requested` that occur nowhere in the ontology. A
    /// module over a misspelled IRI is empty and correct, which is the most
    /// expensive way to learn about a typo.
    pub signature_not_in_ontology: Vec<String>,
    /// `Σ ∪ sig(M)`: everything the module's guarantee actually ranges over.
    pub signature_closure: Vec<String>,
    pub signature_closure_size: usize,

    pub ontology_axioms: usize,
    pub module_axioms: usize,
    pub ontology_triples: usize,
    pub module_triples: usize,
    /// `module_triples / ontology_triples`, for reading only. It is NOT a
    /// quality score: a module is not better for being smaller, it is correct
    /// or it is not, and the size is what the signature costs.
    pub module_fraction: f64,
    pub axioms_by_kind: BTreeMap<String, usize>,
    pub annotation_triples_added: usize,
    /// The predicates this run treated as carrying no logical content, so a
    /// reader can see exactly what "annotation" meant here. An annotation
    /// predicate the ontology gave a domain, a range, a superproperty or an
    /// equivalent is NOT in this list: it was read as a property assertion
    /// instead, because the rule table will read it as one.
    pub annotation_predicates_treated_as_vacuous: Vec<String>,
    /// `⊥⊤` rounds. One for a plain `⊥` or `⊤` extraction.
    pub rounds: usize,

    pub module_ttl: String,
    pub seconds: f64,
}

pub const GUARANTEE: &str =
    "every axiom over the requested signature that the whole ontology entails, the module entails, \
     and nothing else: M ⊨ α iff O ⊨ α for every α over Σ. That is the locality theorem of Cuenca \
     Grau, Horrocks, Kazakov and Sattler (JAIR 31, 2008, 'Modular Reuse of Ontologies: Theory and \
     Practice'), which holds for OWL 2 DL and therefore for every profile inside it. It is a \
     COVERAGE guarantee and not a MINIMALITY one: a locality module is the smallest set the \
     syntactic test can justify, not the smallest set that would do.";

pub const NOT_MACHINE_CHECKED: &str =
    "the locality theorem is CITED, not checked: nothing under lean/ is about syntactic locality, \
     so this report names a paper and never names a Lean theorem. What can be checked here is the \
     consequence rather than the theorem, and that is what verify_out_dir does — it reasons the \
     whole ontology and the module to a fixpoint under one rule table and reports every conclusion \
     over the signature the module does not reach, which must be none.";

/// Axiom kinds with a locality test written for them.
pub const CLASSIFIED_KINDS: &[&str] = &[
    "sub_class_of",
    "equivalent_classes",
    "disjoint_classes",
    "disjoint_union",
    "sub_property_of",
    "property_chain",
    "equivalent_properties",
    "disjoint_properties",
    "property_domain",
    "property_range",
    "inverse_properties",
    "transitive_property",
    "symmetric_property",
    "asymmetric_property",
    "reflexive_property",
    "irreflexive_property",
    "functional_property",
    "inverse_functional_property",
    "has_key",
    "class_assertion",
    "property_assertion",
    "negative_property_assertion",
    "declaration",
    "annotation",
];

/// Axiom kinds that are never local, so they are always in the module.
pub const ALWAYS_INCLUDED_KINDS: &[&str] =
    &["same_individual", "different_individuals", "unclassified"];

// ───────────────────────────────────────────────────────────────────────────
// The entry point
// ───────────────────────────────────────────────────────────────────────────

pub fn extract_module(
    store: &Arc<GraphStore>,
    opts: &ModuleOptions,
) -> anyhow::Result<ModuleReport> {
    let started = std::time::Instant::now();
    if opts.signature.is_empty() {
        anyhow::bail!(
            "the signature is empty. The module over the empty signature is the set of axioms no \
             replacement can turn into a tautology, which is a real answer and never the one \
             anybody wanted: pass the IRIs the module has to cover"
        );
    }
    let ix = Index::build(store)?;
    let axioms = axioms_of(&ix);

    let sigma: Sig = opts.signature.iter().cloned().collect();
    let present: BTreeSet<String> = signature_of(&ix.triples);
    let signature_not_in_ontology: Vec<String> =
        sigma.iter().filter(|s| !present.contains(*s)).cloned().collect();

    let all: Vec<usize> = (0..axioms.len()).collect();
    let (module, rounds) = match opts.locality {
        Locality::Bottom => (extract_once(&axioms, &all, &sigma, Side::Bottom), 1),
        Locality::Top => (extract_once(&axioms, &all, &sigma, Side::Top), 1),
        Locality::Star => {
            let mut current = all;
            let mut rounds = 0usize;
            loop {
                rounds += 1;
                let b = extract_once(&axioms, &current, &sigma, Side::Bottom);
                let t = extract_once(&axioms, &b, &sigma, Side::Top);
                // Each pass only ever removes, so the length strictly decreases
                // until the fixpoint and this terminates.
                let done = t.len() == current.len();
                current = t;
                if done {
                    break;
                }
            }
            (current, rounds)
        }
    };

    let mut module_triples: BTreeSet<Spelled> = BTreeSet::new();
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut conservative: BTreeMap<String, usize> = BTreeMap::new();
    let mut unclassified: Vec<UnclassifiedAxiom> = Vec::new();
    let mut unclassified_total = 0usize;
    let mut closure: BTreeSet<String> = sigma.clone();
    for &i in &module {
        let a = &axioms[i];
        module_triples.extend(a.triples.iter().cloned());
        closure.extend(a.signature.iter().cloned());
        let kind = a.form.kind();
        *by_kind.entry(kind.to_string()).or_default() += 1;
        if ALWAYS_INCLUDED_KINDS.contains(&kind) {
            *conservative.entry(kind.to_string()).or_default() += 1;
        }
        if let Form::Unclassified(why) = &a.form {
            unclassified_total += 1;
            if unclassified.len() < opts.max_rows {
                unclassified.push(UnclassifiedAxiom {
                    root_triple: a.root.clone(),
                    why: why.clone(),
                    triples: a.triples.len(),
                });
            }
        }
    }

    let mut annotation_triples_added = 0usize;
    if opts.include_annotations {
        for a in &axioms {
            if !matches!(a.form, Form::Annotation) {
                continue;
            }
            let kept = a
                .triples
                .first()
                .and_then(|t| iri_of(&t.0))
                .is_some_and(|s| closure.contains(s));
            if kept {
                for t in &a.triples {
                    if module_triples.insert(t.clone()) {
                        annotation_triples_added += 1;
                    }
                }
            }
        }
    }

    let module_ttl: String = module_triples.iter().map(|t| format!("{} .\n", render(t))).collect();
    let ontology_triples = ix.triples.len();
    let module_fraction = if ontology_triples == 0 {
        0.0
    } else {
        module_triples.len() as f64 / ontology_triples as f64
    };
    let headline = format!(
        "the {} module over {} requested name(s) is {} of {} axioms and {} of {} triples. {} \
         axiom(s) were taken in WITHOUT a locality test because they could not be classified. {}",
        opts.locality.symbol(),
        opts.signature.len(),
        module.len(),
        axioms.len(),
        module_triples.len(),
        ontology_triples,
        unclassified_total,
        GUARANTEE,
    );

    Ok(ModuleReport {
        format: "oo-locality-module/1",
        headline,
        locality: opts.locality.name(),
        locality_symbol: opts.locality.symbol(),
        guarantee: GUARANTEE,
        guarantee_is_not_machine_checked: NOT_MACHINE_CHECKED,
        guarantee_covers: CLASSIFIED_KINDS.to_vec(),
        included_conservatively: conservative,
        unclassified_axioms: unclassified,
        unclassified_total,
        signature_requested: opts.signature.clone(),
        signature_not_in_ontology,
        signature_closure_size: closure.len(),
        signature_closure: closure.into_iter().collect(),
        ontology_axioms: axioms.len(),
        module_axioms: module.len(),
        ontology_triples,
        module_triples: module_triples.len(),
        module_fraction,
        axioms_by_kind: by_kind,
        annotation_triples_added,
        annotation_predicates_treated_as_vacuous: ANNOTATION_PREDICATES
            .iter()
            .filter(|p| !ix.loaded_annotation_predicates.contains(**p))
            .map(|p| p.to_string())
            .collect(),
        rounds,
        module_ttl,
        seconds: started.elapsed().as_secs_f64(),
    })
}

// ───────────────────────────────────────────────────────────────────────────
// Exercising the guarantee instead of asserting it
// ───────────────────────────────────────────────────────────────────────────

/// What the closure diff said about a module, filtered to the signature the
/// module is supposed to cover.
#[derive(Clone, Debug, Serialize)]
pub struct ModuleVerification {
    pub format: &'static str,
    pub headline: String,
    pub profile: String,
    /// The one number that matters. A locality module may not lose ANY of
    /// these, so a non-zero value is a defect in the extractor rather than a
    /// property of the ontology.
    pub lost_over_signature_total: usize,
    pub lost_over_signature: Vec<crate::closure_diff::LostEntailment>,
    /// Triples over the signature whose predicate is one of
    /// `annotation_predicates_treated_as_vacuous`. They ARE gone from the
    /// module and they are NOT entailment losses: the extractor drops them
    /// deliberately, and it only calls a predicate vacuous after checking that
    /// the ontology gives it no domain, range, superproperty or equivalent, so
    /// no rule in the table can read one as a premise. Counted separately
    /// rather than filtered out, because a silent filter is how a real loss
    /// would eventually hide here.
    pub lost_over_signature_annotation_only: usize,
    pub annotation_losses_are_not_entailment_losses: &'static str,
    /// Conclusions the module does not reach whose terms are NOT all in the
    /// signature closure. Expected and unremarkable: that is what a module is
    /// for.
    pub lost_outside_signature_total: usize,
    /// The BLIND SPOT, counted rather than left to be discovered. A subset of
    /// `lost_outside_signature_total`, and one that could not have landed
    /// anywhere else: a conclusion carrying a blank node or a Skolem IRI has no
    /// term whose name means the same thing on both sides, so the signature
    /// question cannot be ASKED of it, let alone answered. A large number here
    /// means a `lost_over_signature_total: 0` covers less of the ontology than
    /// it looks.
    pub not_decided_blank_node_bearing: usize,
    pub blind_spot_means: &'static str,
    /// Conclusions the diff did not look at, because it renders at most
    /// `max_rows` of them. Non-zero means this verification is a SAMPLE and
    /// `lost_over_signature_total: 0` does not mean "nothing was lost".
    pub not_examined: usize,
    /// The engine's own soundness gate, run for free by the closure diff.
    pub engine_soundness_violations: usize,
    pub source_certificate: crate::closure_diff::CertificateVerdict,
    pub means: &'static str,
    pub seconds: f64,
}

pub const ANNOTATION_LOSSES_MEAN: &str =
    "an rdfs:label or rdfs:comment the module drops is not a lost entailment: no rule in the table \
     reads one as a premise, which is checked rather than assumed — a predicate the ontology gives \
     a domain, a range, a superproperty or an equivalent is NOT counted here and is kept as a \
     property assertion instead. Pass include_annotations to carry them into the module for \
     reading; the logical module is the same either way.";

pub const BLIND_SPOT_MEANS: &str =
    "the module is serialised with the source store's own blank node labels, and the closure diff \
     skolemises the source before comparing, so a conclusion carrying a blank node or a Skolem IRI \
     has no name that means the same thing on both sides and is NOT decided against the signature. \
     That is the same complexity decision 0007 documents and declines to close: deciding subgraph \
     containment up to blank-node renaming is simple entailment, NP-complete in general. \
     Skolemising the store before extracting the module empties this bucket, and until something \
     does, the number is here so the coverage of a clean result can be read.";

pub const VERIFICATION_MEANS: &str =
    "this is the CONSEQUENCE of the locality theorem, measured; it is not the theorem. It reasons \
     the whole ontology and the module to a fixpoint under one rule table and subtracts, so it can \
     only ever speak about that rule table: a clean run says the module lost nothing this engine \
     could derive over the signature, and says nothing about entailments the engine cannot reach. \
     A non-zero lost_over_signature_total is a defect in the extractor.";

/// Run the closure diff between the whole ontology and the module, and report
/// the part the module's guarantee is about.
///
/// This is deliberately the same machinery `onto_closure_diff` runs. A second
/// implementation of "what did this subset lose" would be a second place for
/// the answer to be wrong.
pub fn verify_module(
    store: &Arc<GraphStore>,
    module: &ModuleReport,
    opts: &crate::closure_diff::DiffOptions,
    scan_rows: usize,
) -> anyhow::Result<ModuleVerification> {
    let started = std::time::Instant::now();
    let module_ttl = module.module_ttl.as_str();
    let sig: BTreeSet<&str> = module.signature_closure.iter().map(String::as_str).collect();
    let vacuous: BTreeSet<&str> =
        module.annotation_predicates_treated_as_vacuous.iter().map(String::as_str).collect();
    // The diff renders `max_rows` of the difference and counts the rest, but
    // the signature partition has to see EVERY row: a verification computed
    // from the first two hundred conclusions is a sample, and a clean sample
    // is not a clean result. `scan_rows` bounds it so a pathological graph
    // cannot exhaust memory, and `not_examined` says when the bound bit.
    let scan = crate::closure_diff::DiffOptions { max_rows: scan_rows, ..opts.clone() };
    let report = crate::closure_diff::closure_diff(store, module_ttl, &scan)?;
    let over_signature = |t: &Spelled| -> bool {
        [&t.0, &t.1, &t.2].into_iter().all(|term| match iri_of(term) {
            Some(i) => is_builtin(i) || sig.contains(i),
            // A literal is a value, not a name, so it never takes a triple out
            // of the signature. A blank node has no name at all and cannot be
            // decided, so it does.
            None => is_literal(term),
        })
    };
    let over: Vec<&crate::closure_diff::LostEntailment> =
        report.entailments_lost.iter().filter(|l| over_signature(&l.triple)).collect();
    let annotation_only = over
        .iter()
        .filter(|l| iri_of(&l.triple.1).is_some_and(|p| vacuous.contains(p)))
        .count();
    let lost: Vec<crate::closure_diff::LostEntailment> = over
        .iter()
        .filter(|l| !iri_of(&l.triple.1).is_some_and(|p| vacuous.contains(p)))
        .map(|l| (*l).clone())
        .collect();
    let unnameable = |t: &Spelled| -> bool {
        [&t.0, &t.1, &t.2].into_iter().any(|term| {
            is_bnode(term)
                || iri_of(term).is_some_and(|i| i.starts_with(SKOLEM_PREFIX))
        })
    };
    let blind = report.entailments_lost.iter().filter(|l| unnameable(&l.triple)).count();
    let scanned = report.entailments_lost.len();
    let lost_over = lost.len();
    let headline = format!(
        "{lost_over} conclusion(s) over the signature closure are reachable from the whole \
         ontology and not from the module, out of {scanned} examined of {} lost in total, plus \
         {annotation_only} annotation triple(s) the module drops by design and {blind} carrying a \
         blank node or a Skolem IRI, which cannot be decided against the signature at all. A \
         locality module may not lose any of the first kind.",
        report.lost_total
    );
    Ok(ModuleVerification {
        format: "oo-module-verification/1",
        headline,
        profile: report.profile.clone(),
        lost_over_signature_total: lost_over,
        lost_over_signature: lost.into_iter().take(opts.max_rows).collect(),
        lost_over_signature_annotation_only: annotation_only,
        annotation_losses_are_not_entailment_losses: ANNOTATION_LOSSES_MEAN,
        lost_outside_signature_total: scanned - lost_over - annotation_only,
        not_decided_blank_node_bearing: blind,
        blind_spot_means: BLIND_SPOT_MEANS,
        not_examined: report.lost_total.saturating_sub(scanned),
        engine_soundness_violations: report.monotonicity_violations.len(),
        source_certificate: report.source_certificate.clone(),
        means: VERIFICATION_MEANS,
        seconds: started.elapsed().as_secs_f64(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(ttl: &str) -> Arc<GraphStore> {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(ttl, None).unwrap();
        g
    }

    const PREFIXES: &str = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         @prefix ex: <http://ex.org/> .\n";

    fn chain() -> Arc<GraphStore> {
        loaded(&format!(
            "{PREFIXES}
             ex:Cat rdfs:subClassOf ex:Mammal .
             ex:Mammal rdfs:subClassOf ex:Animal .
             ex:Animal rdfs:subClassOf ex:LivingThing .
             ex:Plant rdfs:subClassOf ex:LivingThing ."
        ))
    }

    fn opts(sig: &[&str], locality: Locality) -> ModuleOptions {
        ModuleOptions {
            signature: sig.iter().map(|s| s.to_string()).collect(),
            locality,
            ..Default::default()
        }
    }

    #[test]
    fn a_bottom_module_keeps_the_chain_and_drops_the_sibling() {
        let r = extract_module(
            &chain(),
            &opts(&["http://ex.org/Cat", "http://ex.org/LivingThing"], Locality::Bottom),
        )
        .unwrap();
        assert_eq!(r.module_axioms, 3, "{}", r.module_ttl);
        assert!(r.module_ttl.contains("Cat"));
        assert!(r.module_ttl.contains("Animal"));
        assert!(!r.module_ttl.contains("Plant"), "the sibling is ⊥-local: {}", r.module_ttl);
    }

    /// The fixpoint IS the soundness condition, and this is the case that shows
    /// it: the first sweep of the ⊤ pass finds `Cat ⊑ Mammal` and `Mammal ⊑
    /// Animal` local, and only the signature growth from `Animal ⊑ LivingThing`
    /// pulls them back in.
    #[test]
    fn the_top_pass_iterates_rather_than_sweeping_once() {
        let r = extract_module(
            &chain(),
            &opts(&["http://ex.org/Cat", "http://ex.org/LivingThing"], Locality::Star),
        )
        .unwrap();
        assert_eq!(r.module_axioms, 3, "{}", r.module_ttl);
        for term in ["Cat", "Mammal", "Animal", "LivingThing"] {
            assert!(r.module_ttl.contains(term), "{term} missing from {}", r.module_ttl);
        }
    }

    /// `⊥⊤*` is strictly smaller than `⊥` here: the `⊥` pass keeps an ABox
    /// assertion over an external property, and the `⊤` pass removes it.
    #[test]
    fn star_is_strictly_smaller_than_bottom_on_an_external_assertion() {
        let g = loaded(&format!(
            "{PREFIXES}
             ex:Cat rdfs:subClassOf ex:LivingThing .
             ex:tom ex:eats ex:fish ."
        ));
        let sig = ["http://ex.org/Cat", "http://ex.org/LivingThing"];
        let bottom = extract_module(&g, &opts(&sig, Locality::Bottom)).unwrap();
        let star = extract_module(&g, &opts(&sig, Locality::Star)).unwrap();
        assert_eq!(bottom.module_axioms, 2, "{}", bottom.module_ttl);
        assert_eq!(star.module_axioms, 1, "{}", star.module_ttl);
        assert!(!star.module_ttl.contains("eats"));
    }

    #[test]
    fn an_unclassified_builtin_predicate_is_included_not_dropped() {
        let g = loaded(&format!(
            "{PREFIXES}
             ex:Cat rdfs:subClassOf ex:LivingThing .
             ex:Cat owl:members ex:Nonsense ."
        ));
        let r = extract_module(
            &g,
            &opts(&["http://ex.org/Cat", "http://ex.org/LivingThing"], Locality::Star),
        )
        .unwrap();
        assert_eq!(r.unclassified_total, 1, "{:?}", r.axioms_by_kind);
        assert!(r.module_ttl.contains("members"), "{}", r.module_ttl);
    }

    #[test]
    fn same_as_is_never_local() {
        let g = loaded(&format!("{PREFIXES}\nex:a owl:sameAs ex:b ."));
        let r = extract_module(&g, &opts(&["http://ex.org/Unrelated"], Locality::Star)).unwrap();
        assert_eq!(r.module_axioms, 1);
        assert_eq!(r.included_conservatively.get("same_individual"), Some(&1));
    }

    #[test]
    fn a_datatype_filler_is_not_treated_as_an_external_class_name() {
        let g = loaded(&format!(
            "{PREFIXES}
             @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
             ex:Person rdfs:subClassOf
                 [ a owl:Restriction ; owl:onProperty ex:age ; owl:someValuesFrom xsd:integer ] ."
        ));
        // `xsd:integer` is not a class name, so `∃age.xsd:integer` is not
        // ⊥-equivalent and the axiom is not ⊥-local over {Person}.
        let r = extract_module(&g, &opts(&["http://ex.org/Person"], Locality::Bottom)).unwrap();
        assert_eq!(r.module_axioms, 1, "{}", r.module_ttl);
        assert!(r.module_ttl.contains("someValuesFrom"));
    }

    #[test]
    fn an_empty_signature_is_an_error_not_an_empty_module() {
        let e = extract_module(&chain(), &opts(&[], Locality::Star)).unwrap_err();
        assert!(e.to_string().contains("signature is empty"), "{e}");
    }

    #[test]
    fn a_signature_name_absent_from_the_ontology_is_named() {
        let r = extract_module(&chain(), &opts(&["http://ex.org/Typo"], Locality::Star)).unwrap();
        assert_eq!(r.signature_not_in_ontology, vec!["http://ex.org/Typo".to_string()]);
    }

    #[test]
    fn a_declaration_survives_for_a_term_in_the_signature() {
        let g = loaded(&format!("{PREFIXES}\nex:Cat a owl:Class .\nex:Dog a owl:Class ."));
        let r = extract_module(&g, &opts(&["http://ex.org/Cat"], Locality::Star)).unwrap();
        assert_eq!(r.module_axioms, 1, "{}", r.module_ttl);
        assert!(r.module_ttl.contains("Cat"));
        assert!(!r.module_ttl.contains("Dog"));
    }

    /// An annotation property the ontology has given a domain is not an
    /// annotation property any more, and its assertions carry consequences.
    #[test]
    fn a_loaded_annotation_predicate_stops_being_vacuous() {
        let g = loaded(&format!(
            "{PREFIXES}
             rdfs:label rdfs:domain ex:Named .
             ex:thing rdfs:label \"x\" ."
        ));
        let r = extract_module(&g, &opts(&["http://ex.org/Named"], Locality::Bottom)).unwrap();
        assert!(
            r.module_ttl.contains("label"),
            "a label assertion under a declared domain is a property assertion: {}",
            r.module_ttl
        );
    }

    #[test]
    fn locality_names_round_trip() {
        for (word, l) in
            [("bottom", Locality::Bottom), ("top", Locality::Top), ("star", Locality::Star)]
        {
            assert_eq!(Locality::parse(word).unwrap(), l);
            assert_eq!(l.name(), word);
        }
        assert!(Locality::parse("sideways").is_err());
    }
}
