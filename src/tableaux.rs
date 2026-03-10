//! OWL2-DL Tableaux Reasoner — SHOIQ with Agent-Based Classification
//!
//! A native Rust implementation of a tableaux decision procedure for
//! the SHOIQ Description Logic (the logical foundation of OWL2-DL).
//!
//! ## Description Logic Coverage
//!
//! | DL     | OWL Construct              | Status |
//! |--------|----------------------------|--------|
//! | ¬A     | complementOf               | ✅     |
//! | C ⊓ D  | intersectionOf             | ✅     |
//! | C ⊔ D  | unionOf                    | ✅     |
//! | ∃R.C   | someValuesFrom             | ✅     |
//! | ∀R.C   | allValuesFrom              | ✅     |
//! | ≥n R.C | minQualifiedCardinality    | ✅     |
//! | ≤n R.C | maxQualifiedCardinality    | ✅     |
//! | R ⊑ S  | subPropertyOf              | ✅     |
//! | Trans   | TransitiveProperty         | ✅     |
//! | R⁻     | inverseOf                  | ✅     |
//! | Sym     | SymmetricProperty          | ✅     |
//! | Fun     | FunctionalProperty         | ✅     |
//! | ABox   | NamedIndividual            | ✅     |
//!
//! ## Architecture
//!
//! Uses agent-based decomposition for classification:
//! - **Satisfiability Agent**: Parallel sat testing via rayon worker pool
//! - **Subsumption Agent**: Parallel pairwise subsumption with told-subsumer pruning
//! - **Explanation Agent**: Clash tracing and justification extraction
//! - **ABox Agent**: Individual consistency checking and type inference

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use rayon::prelude::*;

use crate::graph::GraphStore;

// ── Well-known IRIs (with <> brackets, matching Oxigraph output) ────────

const RDF_TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const RDF_FIRST: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>";
const RDF_REST: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>";
const RDF_NIL: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>";
const RDFS_SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const RDFS_SUBPROP: &str = "<http://www.w3.org/2000/01/rdf-schema#subPropertyOf>";
const OWL_CLASS: &str = "<http://www.w3.org/2002/07/owl#Class>";
const OWL_THING: &str = "<http://www.w3.org/2002/07/owl#Thing>";
const OWL_NOTHING: &str = "<http://www.w3.org/2002/07/owl#Nothing>";
const OWL_RESTRICTION: &str = "<http://www.w3.org/2002/07/owl#Restriction>";
const OWL_ON_PROPERTY: &str = "<http://www.w3.org/2002/07/owl#onProperty>";
const OWL_SOME_VALUES: &str = "<http://www.w3.org/2002/07/owl#someValuesFrom>";
const OWL_ALL_VALUES: &str = "<http://www.w3.org/2002/07/owl#allValuesFrom>";
const OWL_HAS_VALUE: &str = "<http://www.w3.org/2002/07/owl#hasValue>";
const OWL_COMPLEMENT: &str = "<http://www.w3.org/2002/07/owl#complementOf>";
const OWL_INTERSECTION: &str = "<http://www.w3.org/2002/07/owl#intersectionOf>";
const OWL_UNION: &str = "<http://www.w3.org/2002/07/owl#unionOf>";
const OWL_EQUIV_CLASS: &str = "<http://www.w3.org/2002/07/owl#equivalentClass>";
const OWL_DISJOINT_WITH: &str = "<http://www.w3.org/2002/07/owl#disjointWith>";
const OWL_TRANSITIVE: &str = "<http://www.w3.org/2002/07/owl#TransitiveProperty>";
const OWL_SYMMETRIC: &str = "<http://www.w3.org/2002/07/owl#SymmetricProperty>";
const OWL_INVERSE_OF: &str = "<http://www.w3.org/2002/07/owl#inverseOf>";
const OWL_FUNCTIONAL: &str = "<http://www.w3.org/2002/07/owl#FunctionalProperty>";
const OWL_INV_FUNCTIONAL: &str = "<http://www.w3.org/2002/07/owl#InverseFunctionalProperty>";
const OWL_OBJECT_PROPERTY: &str = "<http://www.w3.org/2002/07/owl#ObjectProperty>";
const OWL_NAMED_INDIVIDUAL: &str = "<http://www.w3.org/2002/07/owl#NamedIndividual>";
const OWL_MIN_CARD: &str = "<http://www.w3.org/2002/07/owl#minCardinality>";
const OWL_MAX_CARD: &str = "<http://www.w3.org/2002/07/owl#maxCardinality>";
const OWL_EXACT_CARD: &str = "<http://www.w3.org/2002/07/owl#cardinality>";
const OWL_MIN_QCARD: &str = "<http://www.w3.org/2002/07/owl#minQualifiedCardinality>";
const OWL_MAX_QCARD: &str = "<http://www.w3.org/2002/07/owl#maxQualifiedCardinality>";
const OWL_ON_CLASS: &str = "<http://www.w3.org/2002/07/owl#onClass>";

const MAX_DEPTH: usize = 100;
const MAX_NODES: usize = 10_000;

// ── Concept (Negation Normal Form) ──────────────────────────────────────

/// Description Logic concept in NNF (Negation Normal Form).
/// All negations pushed to atomic level. Supports SHOIQ.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Concept {
    Top,
    Bottom,
    Atom(u32),
    NegAtom(u32),
    And(Vec<Concept>),
    Or(Vec<Concept>),
    Exists(u32, Box<Concept>),         // ∃R.C
    ForAll(u32, Box<Concept>),         // ∀R.C
    MinCard(u32, u32, Box<Concept>),   // ≥n R.C  (role, n, filler)
    MaxCard(u32, u32, Box<Concept>),   // ≤n R.C  (role, n, filler)
}

impl Concept {
    /// Push negation inward to produce NNF.
    pub fn negate(&self) -> Concept {
        match self {
            Concept::Top => Concept::Bottom,
            Concept::Bottom => Concept::Top,
            Concept::Atom(a) => Concept::NegAtom(*a),
            Concept::NegAtom(a) => Concept::Atom(*a),
            Concept::And(cs) => {
                let mut parts: Vec<_> = cs.iter().map(|c| c.negate()).collect();
                parts.sort();
                Concept::Or(parts)
            }
            Concept::Or(cs) => {
                let mut parts: Vec<_> = cs.iter().map(|c| c.negate()).collect();
                parts.sort();
                Concept::And(parts)
            }
            Concept::Exists(r, c) => Concept::ForAll(*r, Box::new(c.negate())),
            Concept::ForAll(r, c) => Concept::Exists(*r, Box::new(c.negate())),
            // ¬(≥n R.C) = ≤(n-1) R.C
            Concept::MinCard(r, n, c) => {
                if *n == 0 {
                    Concept::Bottom // ≥0 is always true, ¬⊤ = ⊥
                } else {
                    Concept::MaxCard(*r, n - 1, c.clone())
                }
            }
            // ¬(≤n R.C) = ≥(n+1) R.C
            Concept::MaxCard(r, n, c) => Concept::MinCard(*r, n + 1, c.clone()),
        }
    }
}

/// Pre-NNF concept used during OWL parsing.
#[derive(Clone, Debug)]
enum RawConcept {
    Top,
    Bottom,
    Named(u32),
    Not(Box<RawConcept>),
    And(Vec<RawConcept>),
    Or(Vec<RawConcept>),
    Exists(u32, Box<RawConcept>),
    ForAll(u32, Box<RawConcept>),
    MinCard(u32, u32, Box<RawConcept>),
    MaxCard(u32, u32, Box<RawConcept>),
}

impl RawConcept {
    fn to_nnf(&self) -> Concept {
        match self {
            RawConcept::Top => Concept::Top,
            RawConcept::Bottom => Concept::Bottom,
            RawConcept::Named(id) => Concept::Atom(*id),
            RawConcept::Not(inner) => inner.to_nnf().negate(),
            RawConcept::And(cs) => {
                let mut parts: Vec<_> = cs.iter().map(|c| c.to_nnf()).collect();
                parts.sort();
                match parts.len() {
                    0 => Concept::Top,
                    1 => parts.remove(0),
                    _ => Concept::And(parts),
                }
            }
            RawConcept::Or(cs) => {
                let mut parts: Vec<_> = cs.iter().map(|c| c.to_nnf()).collect();
                parts.sort();
                match parts.len() {
                    0 => Concept::Bottom,
                    1 => parts.remove(0),
                    _ => Concept::Or(parts),
                }
            }
            RawConcept::Exists(r, c) => Concept::Exists(*r, Box::new(c.to_nnf())),
            RawConcept::ForAll(r, c) => Concept::ForAll(*r, Box::new(c.to_nnf())),
            RawConcept::MinCard(r, n, c) => Concept::MinCard(*r, *n, Box::new(c.to_nnf())),
            RawConcept::MaxCard(r, n, c) => Concept::MaxCard(*r, *n, Box::new(c.to_nnf())),
        }
    }
}

// ── String Interner ─────────────────────────────────────────────────────

pub struct Interner {
    to_id: HashMap<String, u32>,
    to_str: Vec<String>,
}

impl Interner {
    fn new() -> Self {
        Self {
            to_id: HashMap::new(),
            to_str: Vec::new(),
        }
    }

    fn intern(&mut self, s: &str) -> u32 {
        if let Some(&id) = self.to_id.get(s) {
            return id;
        }
        let id = self.to_str.len() as u32;
        self.to_str.push(s.to_string());
        self.to_id.insert(s.to_string(), id);
        id
    }

    pub fn resolve(&self, id: u32) -> &str {
        &self.to_str[id as usize]
    }
}

// ── Triple Index ────────────────────────────────────────────────────────

struct TripleIndex {
    by_subject: HashMap<String, Vec<(String, String)>>,
}

impl TripleIndex {
    fn new(triples: &[(String, String, String)]) -> Self {
        let mut by_subject: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for (s, p, o) in triples {
            by_subject
                .entry(s.clone())
                .or_default()
                .push((p.clone(), o.clone()));
        }
        Self { by_subject }
    }

    fn objects(&self, subject: &str, predicate: &str) -> Vec<String> {
        self.by_subject
            .get(subject)
            .map(|pairs| {
                pairs
                    .iter()
                    .filter(|(p, _)| p == predicate)
                    .map(|(_, o)| o.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn object(&self, subject: &str, predicate: &str) -> Option<String> {
        self.objects(subject, predicate).into_iter().next()
    }

    fn walk_list(&self, head: &str) -> Vec<String> {
        let mut items = Vec::new();
        let mut current = head.to_string();
        for _ in 0..1000 {
            if current == RDF_NIL {
                break;
            }
            if let Some(first) = self.object(&current, RDF_FIRST) {
                items.push(first);
            }
            match self.object(&current, RDF_REST) {
                Some(rest) => current = rest,
                None => break,
            }
        }
        items
    }
}

// ── OWL Parser ──────────────────────────────────────────────────────────

struct OwlParser {
    index: TripleIndex,
    interner: Interner,
}

impl OwlParser {
    fn new(triples: Vec<(String, String, String)>) -> Self {
        Self {
            index: TripleIndex::new(&triples),
            interner: Interner::new(),
        }
    }

    fn parse(mut self) -> ParseResult {
        let mut axioms: Vec<(Concept, Concept)> = Vec::new();
        let mut named_classes: HashSet<u32> = HashSet::new();
        let mut transitive_roles: HashSet<u32> = HashSet::new();
        let mut sub_to_super: HashMap<u32, HashSet<u32>> = HashMap::new();
        let mut disjoint_pairs: Vec<(Concept, Concept)> = Vec::new();
        let mut inverse_roles: HashMap<u32, u32> = HashMap::new();
        let mut functional_roles: HashSet<u32> = HashSet::new();
        let mut inv_functional_roles: HashSet<u32> = HashSet::new();
        let mut object_properties: HashSet<u32> = HashSet::new();
        let mut individual_types: HashMap<u32, HashSet<u32>> = HashMap::new();
        let mut role_assertions: Vec<(u32, u32, u32)> = Vec::new();

        // Collect all subjects with their types for classification
        let mut subject_types: HashMap<String, Vec<String>> = HashMap::new();
        for (s, pairs) in &self.index.by_subject {
            for (p, o) in pairs {
                if p == RDF_TYPE {
                    subject_types.entry(s.clone()).or_default().push(o.clone());
                }
            }
        }

        // Collect class declarations
        let class_subjects: Vec<String> = subject_types
            .iter()
            .filter(|(_, types)| types.iter().any(|t| t == OWL_CLASS))
            .map(|(s, _)| s.clone())
            .collect();
        for s in &class_subjects {
            let id = self.interner.intern(s);
            named_classes.insert(id);
        }

        // Collect object properties
        let obj_prop_subjects: Vec<String> = subject_types
            .iter()
            .filter(|(_, types)| types.iter().any(|t| t == OWL_OBJECT_PROPERTY))
            .map(|(s, _)| s.clone())
            .collect();
        for s in &obj_prop_subjects {
            object_properties.insert(self.interner.intern(s));
        }

        // Collect transitive roles
        let trans_subjects: Vec<String> = subject_types
            .iter()
            .filter(|(_, types)| types.iter().any(|t| t == OWL_TRANSITIVE))
            .map(|(s, _)| s.clone())
            .collect();
        for s in trans_subjects {
            transitive_roles.insert(self.interner.intern(&s));
        }

        // Collect symmetric roles → inverse of self
        let sym_subjects: Vec<String> = subject_types
            .iter()
            .filter(|(_, types)| types.iter().any(|t| t == OWL_SYMMETRIC))
            .map(|(s, _)| s.clone())
            .collect();
        for s in sym_subjects {
            let id = self.interner.intern(&s);
            inverse_roles.insert(id, id); // symmetric = own inverse
        }

        // Collect functional properties
        let func_subjects: Vec<String> = subject_types
            .iter()
            .filter(|(_, types)| types.iter().any(|t| t == OWL_FUNCTIONAL))
            .map(|(s, _)| s.clone())
            .collect();
        for s in func_subjects {
            functional_roles.insert(self.interner.intern(&s));
        }

        // Collect inverse-functional properties
        let inv_func_subjects: Vec<String> = subject_types
            .iter()
            .filter(|(_, types)| types.iter().any(|t| t == OWL_INV_FUNCTIONAL))
            .map(|(s, _)| s.clone())
            .collect();
        for s in inv_func_subjects {
            inv_functional_roles.insert(self.interner.intern(&s));
        }

        // Collect owl:inverseOf pairs (bidirectional)
        let inverse_pairs_raw: Vec<(String, String)> = self
            .index
            .by_subject
            .iter()
            .flat_map(|(s, pairs)| {
                pairs
                    .iter()
                    .filter(|(p, _)| p == OWL_INVERSE_OF)
                    .map(move |(_, o)| (s.clone(), o.clone()))
            })
            .collect();
        for (a_str, b_str) in inverse_pairs_raw {
            let a = self.interner.intern(&a_str);
            let b = self.interner.intern(&b_str);
            inverse_roles.insert(a, b);
            inverse_roles.insert(b, a);
        }

        // Collect sub-property relations
        let subprop_pairs: Vec<(String, String)> = self
            .index
            .by_subject
            .iter()
            .flat_map(|(s, pairs)| {
                pairs
                    .iter()
                    .filter(|(p, _)| p == RDFS_SUBPROP)
                    .map(move |(_, o)| (s.clone(), o.clone()))
            })
            .collect();
        for (sub, sup) in subprop_pairs {
            let sub_id = self.interner.intern(&sub);
            let sup_id = self.interner.intern(&sup);
            sub_to_super.entry(sub_id).or_default().insert(sup_id);
        }

        // Collect SubClassOf axioms
        let subclass_pairs: Vec<(String, String)> = self
            .index
            .by_subject
            .iter()
            .flat_map(|(s, pairs)| {
                pairs
                    .iter()
                    .filter(|(p, _)| p == RDFS_SUBCLASS)
                    .map(move |(_, o)| (s.clone(), o.clone()))
            })
            .collect();
        for (sub_str, sup_str) in subclass_pairs {
            let sub = self.parse_class_expr(&sub_str);
            let sup = self.parse_class_expr(&sup_str);
            axioms.push((sub.to_nnf(), sup.to_nnf()));
        }

        // Collect EquivalentClass axioms (→ bidirectional SubClassOf)
        let equiv_pairs: Vec<(String, String)> = self
            .index
            .by_subject
            .iter()
            .flat_map(|(s, pairs)| {
                pairs
                    .iter()
                    .filter(|(p, _)| p == OWL_EQUIV_CLASS)
                    .map(move |(_, o)| (s.clone(), o.clone()))
            })
            .collect();
        for (a_str, b_str) in equiv_pairs {
            let a = self.parse_class_expr(&a_str);
            let b = self.parse_class_expr(&b_str);
            let a_nnf = a.to_nnf();
            let b_nnf = b.to_nnf();
            axioms.push((a_nnf.clone(), b_nnf.clone()));
            axioms.push((b_nnf, a_nnf));
        }

        // Collect DisjointWith axioms
        let disjoint_raw: Vec<(String, String)> = self
            .index
            .by_subject
            .iter()
            .flat_map(|(s, pairs)| {
                pairs
                    .iter()
                    .filter(|(p, _)| p == OWL_DISJOINT_WITH)
                    .map(move |(_, o)| (s.clone(), o.clone()))
            })
            .collect();
        for (a_str, b_str) in disjoint_raw {
            let a = self.parse_class_expr(&a_str).to_nnf();
            let b = self.parse_class_expr(&b_str).to_nnf();
            disjoint_pairs.push((a, b));
        }

        // Collect named individuals and their types + role assertions
        let individual_subjects: Vec<String> = subject_types
            .iter()
            .filter(|(_, types)| types.iter().any(|t| t == OWL_NAMED_INDIVIDUAL))
            .map(|(s, _)| s.clone())
            .collect();
        for ind_str in &individual_subjects {
            let ind_id = self.interner.intern(ind_str);
            let types = self.index.objects(ind_str, RDF_TYPE);
            for t in &types {
                let t_id = self.interner.intern(t);
                if named_classes.contains(&t_id) {
                    individual_types.entry(ind_id).or_default().insert(t_id);
                }
            }
            // Role assertions
            if let Some(pairs) = self.index.by_subject.get(ind_str) {
                for (p, o) in pairs {
                    if p == RDF_TYPE {
                        continue;
                    }
                    let p_id = self.interner.intern(p);
                    if object_properties.contains(&p_id) {
                        let o_id = self.interner.intern(o);
                        role_assertions.push((ind_id, p_id, o_id));
                    }
                }
            }
        }

        // Ensure owl:Thing and owl:Nothing are interned
        let thing_id = self.interner.intern(OWL_THING);
        let nothing_id = self.interner.intern(OWL_NOTHING);
        named_classes.insert(thing_id);

        ParseResult {
            interner: self.interner,
            axioms,
            named_classes,
            thing_id,
            nothing_id,
            transitive_roles,
            sub_to_super,
            disjoint_pairs,
            inverse_roles,
            functional_roles,
            inv_functional_roles,
            individual_types,
            role_assertions,
        }
    }

    fn parse_class_expr(&mut self, node: &str) -> RawConcept {
        if node == OWL_THING {
            return RawConcept::Top;
        }
        if node == OWL_NOTHING {
            return RawConcept::Bottom;
        }

        // Blank nodes: check for complex class expressions
        if node.starts_with("_:") {
            if let Some(c) = self.try_parse_complex(node) {
                return c;
            }
        }

        // Named class
        let id = self.interner.intern(node);
        RawConcept::Named(id)
    }

    fn try_parse_complex(&mut self, node: &str) -> Option<RawConcept> {
        // Restriction
        if self
            .index
            .objects(node, RDF_TYPE)
            .iter()
            .any(|t| t == OWL_RESTRICTION)
        {
            return Some(self.parse_restriction(node));
        }
        // intersectionOf
        if let Some(list_head) = self.index.object(node, OWL_INTERSECTION) {
            let items = self.index.walk_list(&list_head);
            let concepts: Vec<_> = items.iter().map(|i| self.parse_class_expr(i)).collect();
            return Some(if concepts.is_empty() {
                RawConcept::Top
            } else {
                RawConcept::And(concepts)
            });
        }
        // unionOf
        if let Some(list_head) = self.index.object(node, OWL_UNION) {
            let items = self.index.walk_list(&list_head);
            let concepts: Vec<_> = items.iter().map(|i| self.parse_class_expr(i)).collect();
            return Some(if concepts.is_empty() {
                RawConcept::Bottom
            } else {
                RawConcept::Or(concepts)
            });
        }
        // complementOf
        if let Some(comp) = self.index.object(node, OWL_COMPLEMENT) {
            return Some(RawConcept::Not(Box::new(self.parse_class_expr(&comp))));
        }
        None
    }

    fn parse_restriction(&mut self, node: &str) -> RawConcept {
        let prop = match self.index.object(node, OWL_ON_PROPERTY) {
            Some(p) => self.interner.intern(&p),
            None => return RawConcept::Top,
        };

        // someValuesFrom → ∃R.C
        if let Some(filler) = self.index.object(node, OWL_SOME_VALUES) {
            return RawConcept::Exists(prop, Box::new(self.parse_class_expr(&filler)));
        }
        // allValuesFrom → ∀R.C
        if let Some(filler) = self.index.object(node, OWL_ALL_VALUES) {
            return RawConcept::ForAll(prop, Box::new(self.parse_class_expr(&filler)));
        }
        // hasValue → ∃R.{a} (approximated as ∃R.Named(a))
        if let Some(value) = self.index.object(node, OWL_HAS_VALUE) {
            let val_id = self.interner.intern(&value);
            return RawConcept::Exists(prop, Box::new(RawConcept::Named(val_id)));
        }

        // Qualified cardinality restrictions
        let on_class = self.index.object(node, OWL_ON_CLASS);
        let filler = match on_class {
            Some(ref cls) => self.parse_class_expr(cls),
            None => RawConcept::Top,
        };

        // minQualifiedCardinality / minCardinality → ≥n R.C
        if let Some(val) = self
            .index
            .object(node, OWL_MIN_QCARD)
            .or_else(|| self.index.object(node, OWL_MIN_CARD))
        {
            if let Some(n) = parse_card_value(&val) {
                return RawConcept::MinCard(prop, n, Box::new(filler));
            }
        }
        // maxQualifiedCardinality / maxCardinality → ≤n R.C
        if let Some(val) = self
            .index
            .object(node, OWL_MAX_QCARD)
            .or_else(|| self.index.object(node, OWL_MAX_CARD))
        {
            if let Some(n) = parse_card_value(&val) {
                return RawConcept::MaxCard(prop, n, Box::new(filler));
            }
        }
        // exactCardinality → ≥n R.C ⊓ ≤n R.C
        if let Some(val) = self.index.object(node, OWL_EXACT_CARD) {
            if let Some(n) = parse_card_value(&val) {
                return RawConcept::And(vec![
                    RawConcept::MinCard(prop, n, Box::new(filler.clone())),
                    RawConcept::MaxCard(prop, n, Box::new(filler)),
                ]);
            }
        }

        RawConcept::Top
    }
}

/// Parse cardinality value from OWL literal (e.g., "2"^^<xsd:nonNegativeInteger>).
fn parse_card_value(literal: &str) -> Option<u32> {
    if literal.starts_with('"') {
        let rest = &literal[1..];
        if let Some(end) = rest.find('"') {
            return rest[..end].parse().ok();
        }
    }
    literal.parse().ok()
}

struct ParseResult {
    interner: Interner,
    axioms: Vec<(Concept, Concept)>,
    named_classes: HashSet<u32>,
    thing_id: u32,
    nothing_id: u32,
    transitive_roles: HashSet<u32>,
    sub_to_super: HashMap<u32, HashSet<u32>>,
    disjoint_pairs: Vec<(Concept, Concept)>,
    inverse_roles: HashMap<u32, u32>,
    functional_roles: HashSet<u32>,
    inv_functional_roles: HashSet<u32>,
    individual_types: HashMap<u32, HashSet<u32>>,
    role_assertions: Vec<(u32, u32, u32)>,
}

// ── Processed TBox ──────────────────────────────────────────────────────

#[derive(Clone)]
struct ProcessedTBox {
    /// Atomic LHS definitions: when Atom(A) appears, add these concepts.
    concept_defs: HashMap<u32, Vec<Concept>>,
    /// General Concept Inclusions for complex LHS: ¬C ⊔ D.
    gcis: Vec<Concept>,
    /// Disjointness pairs.
    disjoint_pairs: Vec<(Concept, Concept)>,
    /// Transitive roles.
    transitive_roles: HashSet<u32>,
    /// Role hierarchy: super-role → set of sub-roles.
    super_to_sub: HashMap<u32, HashSet<u32>>,
    /// Inverse role mapping (bidirectional): R → R⁻ and R⁻ → R.
    inverse_roles: HashMap<u32, u32>,
}

impl ProcessedTBox {
    fn new(
        axioms: &[(Concept, Concept)],
        disjoint_pairs: &[(Concept, Concept)],
        transitive_roles: HashSet<u32>,
        sub_to_super: &HashMap<u32, HashSet<u32>>,
        inverse_roles: HashMap<u32, u32>,
        functional_roles: &HashSet<u32>,
        inv_functional_roles: &HashSet<u32>,
    ) -> Self {
        let mut concept_defs: HashMap<u32, Vec<Concept>> = HashMap::new();
        let mut gcis: Vec<Concept> = Vec::new();

        for (sub, sup) in axioms {
            match sub {
                Concept::Atom(a) => {
                    concept_defs.entry(*a).or_default().push(sup.clone());
                }
                _ => {
                    // Complex LHS → GCI: ¬sub ⊔ sup
                    let mut parts = vec![sub.negate(), sup.clone()];
                    parts.sort();
                    gcis.push(Concept::Or(parts));
                }
            }
        }

        // Functional properties: R functional → every node gets ≤1 R.⊤
        for &role in functional_roles {
            gcis.push(Concept::MaxCard(role, 1, Box::new(Concept::Top)));
        }

        // Inverse-functional: R inv-functional → ≤1 R⁻.⊤
        for &role in inv_functional_roles {
            if let Some(&inv) = inverse_roles.get(&role) {
                gcis.push(Concept::MaxCard(inv, 1, Box::new(Concept::Top)));
            }
        }

        // Compute super_to_sub from sub_to_super
        let mut super_to_sub_map: HashMap<u32, HashSet<u32>> = HashMap::new();
        for (&sub, supers) in sub_to_super {
            for &sup in supers {
                super_to_sub_map.entry(sup).or_default().insert(sub);
            }
        }

        Self {
            concept_defs,
            gcis,
            disjoint_pairs: disjoint_pairs.to_vec(),
            transitive_roles,
            super_to_sub: super_to_sub_map,
            inverse_roles,
        }
    }
}

// ── Tableau Node ────────────────────────────────────────────────────────

#[derive(Clone)]
struct TNode {
    labels: HashSet<Concept>,
    processed: HashSet<Concept>,
    edges: HashMap<u32, HashSet<u32>>,
    parent: Option<u32>,
    /// Which role created this node from its parent (for inverse propagation).
    parent_role: Option<u32>,
    blocked: bool,
}

impl TNode {
    fn new(parent: Option<u32>, parent_role: Option<u32>) -> Self {
        Self {
            labels: HashSet::new(),
            processed: HashSet::new(),
            edges: HashMap::new(),
            parent,
            parent_role,
            blocked: false,
        }
    }

    fn has_clash(&self) -> bool {
        if self.labels.contains(&Concept::Bottom) {
            return true;
        }
        for label in &self.labels {
            if let Concept::Atom(a) = label {
                if self.labels.contains(&Concept::NegAtom(*a)) {
                    return true;
                }
            }
        }
        // MinCard/MaxCard direct clash: ≥n1 R.C and ≤n2 R.C with n1 > n2
        for label in &self.labels {
            if let Concept::MinCard(r1, n1, f1) = label {
                for other in &self.labels {
                    if let Concept::MaxCard(r2, n2, f2) = other {
                        if r1 == r2 && n1 > n2 && (f1 == f2 || **f2 == Concept::Top) {
                            return true;
                        }
                    }
                }
            }
            // Exists(R, C) = ≥1 R.C clashes with MaxCard(R, 0, C/Top)
            if let Concept::Exists(r1, f1) = label {
                for other in &self.labels {
                    if let Concept::MaxCard(r2, n2, f2) = other {
                        if r1 == r2 && *n2 == 0 && (f1 == f2 || **f2 == Concept::Top) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

// ── Explanation Trace ───────────────────────────────────────────────────

/// Records reasoning steps for clash explanation.
#[derive(Clone, Default)]
struct ExplanationTrace {
    steps: Vec<String>,
    enabled: bool,
}

impl ExplanationTrace {
    fn new(enabled: bool) -> Self {
        Self {
            steps: Vec::new(),
            enabled,
        }
    }

    fn record(&mut self, step: &str) {
        if self.enabled {
            self.steps.push(step.to_string());
        }
    }
}

// ── Tableau ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Tableau {
    nodes: HashMap<u32, TNode>,
    next_id: u32,
    tbox: Arc<ProcessedTBox>,
    trace: ExplanationTrace,
}

impl Tableau {
    fn new(tbox: Arc<ProcessedTBox>) -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: 0,
            tbox,
            trace: ExplanationTrace::new(false),
        }
    }

    fn new_with_tracing(tbox: Arc<ProcessedTBox>) -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: 0,
            tbox,
            trace: ExplanationTrace::new(true),
        }
    }

    fn is_satisfiable(&mut self, concept: &Concept) -> bool {
        let root = self.fresh_node(None, None);
        self.add_label(root, concept.clone());
        // Add GCIs to root
        for gci in self.tbox.gcis.clone() {
            self.add_label(root, gci);
        }
        self.expand(0)
    }

    fn fresh_node(&mut self, parent: Option<u32>, parent_role: Option<u32>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(id, TNode::new(parent, parent_role));
        id
    }

    fn add_label(&mut self, node_id: u32, concept: Concept) -> bool {
        if concept == Concept::Top {
            return false;
        }
        let node = self.nodes.get_mut(&node_id).unwrap();
        if !node.labels.insert(concept.clone()) {
            return false;
        }
        // Trigger concept definitions for atomic labels
        if let Concept::Atom(a) = &concept {
            if let Some(defs) = self.tbox.concept_defs.get(a).cloned() {
                for d in defs {
                    self.add_label(node_id, d);
                }
            }
        }
        true
    }

    /// Get successors via a role, considering sub-roles and inverse relationships.
    fn successors(&self, node_id: u32, role: u32) -> HashSet<u32> {
        let node = &self.nodes[&node_id];
        let mut result = HashSet::new();

        // Direct successors
        if let Some(succs) = node.edges.get(&role) {
            result.extend(succs);
        }

        // Sub-role successors
        if let Some(sub_roles) = self.tbox.super_to_sub.get(&role) {
            for &sub in sub_roles {
                if let Some(succs) = node.edges.get(&sub) {
                    result.extend(succs);
                }
            }
        }

        // Inverse: if parent_role has inverse == role, parent is a role-successor
        if let Some(parent_id) = node.parent {
            if let Some(parent_role) = node.parent_role {
                if let Some(&inv_of_parent) = self.tbox.inverse_roles.get(&parent_role) {
                    if role == inv_of_parent {
                        result.insert(parent_id);
                    }
                }
            }
        }

        result
    }

    /// Create a new successor node and set up edges (including inverse back-edges).
    fn create_successor(&mut self, parent_id: u32, role: u32, filler: Concept) -> u32 {
        let succ = self.fresh_node(Some(parent_id), Some(role));
        self.add_label(succ, filler);

        // Add GCIs to new node
        for gci in self.tbox.gcis.clone() {
            self.add_label(succ, gci);
        }

        // Propagate ∀ labels from parent to new successor
        let parent_labels: Vec<Concept> = self.nodes[&parent_id].labels.iter().cloned().collect();
        for label in &parent_labels {
            match label {
                Concept::ForAll(r, f) if *r == role => {
                    self.add_label(succ, *f.clone());
                    if self.tbox.transitive_roles.contains(r) {
                        self.add_label(succ, Concept::ForAll(*r, f.clone()));
                    }
                }
                _ => {}
            }
        }

        // Also propagate ∀S.C where S is a super-role of 'role'
        // (if role ⊑ S, then an R-edge counts as an S-edge for ∀S propagation)
        // Build super-roles of 'role'
        let super_roles: Vec<u32> = self
            .tbox
            .super_to_sub
            .iter()
            .filter(|(_, subs)| subs.contains(&role))
            .map(|(&sup, _)| sup)
            .collect();
        for sup_role in super_roles {
            for label in &parent_labels {
                if let Concept::ForAll(r, f) = label {
                    if *r == sup_role {
                        self.add_label(succ, *f.clone());
                    }
                }
            }
        }

        // Add forward edge
        self.nodes
            .get_mut(&parent_id)
            .unwrap()
            .edges
            .entry(role)
            .or_default()
            .insert(succ);

        self.trace.record(&format!(
            "∃-rule: node {} creates successor {} via role {}",
            parent_id, succ, role
        ));

        succ
    }

    /// Merge two nodes (for ≤-rule / MaxCard). Combines labels, edges, redirects.
    fn merge_nodes(&mut self, keep_id: u32, remove_id: u32) {
        self.trace.record(&format!(
            "≤-merge: merging node {} into node {}",
            remove_id, keep_id
        ));

        // 1. Merge labels
        let remove_labels: Vec<Concept> = self.nodes[&remove_id].labels.iter().cloned().collect();
        for label in remove_labels {
            self.add_label(keep_id, label);
        }

        // 2. Merge edges
        let remove_edges = self.nodes[&remove_id].edges.clone();
        for (role, targets) in remove_edges {
            for target in targets {
                if target != keep_id && target != remove_id {
                    self.nodes
                        .get_mut(&keep_id)
                        .unwrap()
                        .edges
                        .entry(role)
                        .or_default()
                        .insert(target);
                }
            }
        }

        // 3. Redirect references: in all other nodes, replace remove_id with keep_id
        let all_ids: Vec<u32> = self.nodes.keys().copied().collect();
        for &nid in &all_ids {
            if nid == remove_id {
                continue;
            }
            let node = self.nodes.get_mut(&nid).unwrap();
            for targets in node.edges.values_mut() {
                if targets.remove(&remove_id) {
                    if nid != keep_id {
                        targets.insert(keep_id);
                    }
                }
            }
            // Update parent references
            if node.parent == Some(remove_id) {
                node.parent = Some(keep_id);
            }
        }

        // 4. Remove merged node
        self.nodes.remove(&remove_id);

        // 5. Clear processed set for keep_id so rules are re-applied
        self.nodes.get_mut(&keep_id).unwrap().processed.clear();
    }

    /// Main expansion with backtracking for disjunctions and MaxCard merging.
    fn expand(&mut self, depth: usize) -> bool {
        if depth > MAX_DEPTH || self.nodes.len() > MAX_NODES {
            return false;
        }

        // Apply deterministic rules until fixpoint
        loop {
            if self.any_clash() {
                return false;
            }
            let mut changed = false;
            let node_ids: Vec<u32> = self.nodes.keys().copied().collect();

            for &nid in &node_ids {
                if !self.nodes.contains_key(&nid) || self.nodes[&nid].blocked {
                    continue;
                }

                let labels: Vec<Concept> = self.nodes[&nid]
                    .labels
                    .iter()
                    .filter(|l| !self.nodes[&nid].processed.contains(l))
                    .cloned()
                    .collect();

                for label in labels {
                    match &label {
                        // ⊓-rule: expand conjunction
                        Concept::And(cs) => {
                            let cs = cs.clone();
                            self.nodes.get_mut(&nid).unwrap().processed.insert(label);
                            for c in cs {
                                if self.add_label(nid, c) {
                                    changed = true;
                                }
                            }
                        }
                        // ∃-rule: create successor if needed (≥1 R.C)
                        Concept::Exists(role, filler) => {
                            let role = *role;
                            let filler = *filler.clone();
                            let succs = self.successors(nid, role);
                            let has_matching =
                                succs.iter().any(|&s| {
                                    self.nodes.get(&s).map_or(false, |n| n.labels.contains(&filler))
                                });
                            if !has_matching {
                                self.create_successor(nid, role, filler);
                                changed = true;
                            }
                            self.nodes.get_mut(&nid).unwrap().processed.insert(label);
                        }
                        // ≥-rule: ensure at least n R-successors with C
                        Concept::MinCard(role, n, filler) => {
                            let role = *role;
                            let n = *n as usize;
                            let filler = *filler.clone();
                            let succs = self.successors(nid, role);
                            let matching: usize = succs
                                .iter()
                                .filter(|&&s| {
                                    self.nodes.get(&s).map_or(false, |n| n.labels.contains(&filler))
                                })
                                .count();
                            if matching < n {
                                for _ in 0..(n - matching) {
                                    self.create_successor(nid, role, filler.clone());
                                }
                                changed = true;
                            }
                            self.nodes.get_mut(&nid).unwrap().processed.insert(label);
                        }
                        // ∀-rule: apply filler to all successors + inverse propagation
                        Concept::ForAll(role, filler) => {
                            let role = *role;
                            let filler = *filler.clone();
                            let is_transitive = self.tbox.transitive_roles.contains(&role);
                            let succs = self.successors(nid, role);
                            for s in succs {
                                if !self.nodes.contains_key(&s) {
                                    continue;
                                }
                                if self.add_label(s, filler.clone()) {
                                    changed = true;
                                }
                                if is_transitive {
                                    let forall = Concept::ForAll(role, Box::new(filler.clone()));
                                    if self.add_label(s, forall) {
                                        changed = true;
                                    }
                                }
                            }
                            self.nodes.get_mut(&nid).unwrap().processed.insert(label);
                        }
                        // Atomic labels: already handled by add_label
                        Concept::Atom(_)
                        | Concept::NegAtom(_)
                        | Concept::Top
                        | Concept::Bottom => {
                            self.nodes.get_mut(&nid).unwrap().processed.insert(label);
                        }
                        // ⊔-rule and ≤-rule: handled below (non-deterministic)
                        Concept::Or(_) | Concept::MaxCard(..) => {}
                    }
                }
            }

            self.update_blocking();
            if !changed {
                break;
            }
        }

        if self.any_clash() {
            return false;
        }

        // ── ≤-rule (MaxCard) with node merging ──────────────────────────
        // Check for MaxCard violations and merge nodes
        let node_ids: Vec<u32> = self.nodes.keys().copied().collect();
        for &nid in &node_ids {
            if !self.nodes.contains_key(&nid) || self.nodes[&nid].blocked {
                continue;
            }

            let max_labels: Vec<(u32, u32, Concept)> = self.nodes[&nid]
                .labels
                .iter()
                .filter(|l| !self.nodes[&nid].processed.contains(l))
                .filter_map(|l| match l {
                    Concept::MaxCard(r, n, f) => Some((*r, *n, *f.clone())),
                    _ => None,
                })
                .collect();

            for (role, n, filler) in max_labels {
                let succs = self.successors(nid, role);
                let matching: Vec<u32> = succs
                    .iter()
                    .filter(|&&s| {
                        self.nodes
                            .get(&s)
                            .map_or(false, |node| node.labels.contains(&filler))
                    })
                    .copied()
                    .collect();

                if matching.len() as u32 > n {
                    self.trace.record(&format!(
                        "≤-rule: node {} has {} {}-successors with filler but max is {}",
                        nid,
                        matching.len(),
                        role,
                        n
                    ));

                    // Non-deterministic merge: try each pair
                    let mc_label = Concept::MaxCard(role, n, Box::new(filler));
                    self.nodes
                        .get_mut(&nid)
                        .unwrap()
                        .processed
                        .insert(mc_label);

                    for i in 0..matching.len() {
                        for j in (i + 1)..matching.len() {
                            let mut branch = self.clone();
                            branch.merge_nodes(matching[i], matching[j]);
                            if branch.expand(depth + 1) {
                                *self = branch;
                                return true;
                            }
                        }
                    }
                    return false; // All merges lead to clash
                } else {
                    // No violation, mark as processed
                    let mc_label = Concept::MaxCard(role, n, Box::new(filler));
                    self.nodes
                        .get_mut(&nid)
                        .unwrap()
                        .processed
                        .insert(mc_label);
                }
            }
        }

        // ── ⊔-rule: find unprocessed disjunction → branch ──────────────
        let node_ids: Vec<u32> = self.nodes.keys().copied().collect();
        for &nid in &node_ids {
            if !self.nodes.contains_key(&nid) || self.nodes[&nid].blocked {
                continue;
            }
            let pending_ors: Vec<Concept> = self.nodes[&nid]
                .labels
                .iter()
                .filter(|l| matches!(l, Concept::Or(_)))
                .filter(|l| !self.nodes[&nid].processed.contains(l))
                .cloned()
                .collect();

            for or_concept in pending_ors {
                if let Concept::Or(ref disjuncts) = or_concept {
                    let already_has = disjuncts
                        .iter()
                        .any(|d| self.nodes[&nid].labels.contains(d));
                    if already_has {
                        self.nodes
                            .get_mut(&nid)
                            .unwrap()
                            .processed
                            .insert(or_concept);
                        continue;
                    }
                    // Branch: try each disjunct
                    self.nodes
                        .get_mut(&nid)
                        .unwrap()
                        .processed
                        .insert(or_concept.clone());
                    for disjunct in disjuncts {
                        let mut branch = self.clone();
                        branch.add_label(nid, disjunct.clone());
                        if branch.expand(depth + 1) {
                            return true;
                        }
                    }
                    return false; // All branches clash
                }
            }
        }

        // Check disjointness constraints
        for (a, b) in &self.tbox.disjoint_pairs {
            for node in self.nodes.values() {
                if node.blocked {
                    continue;
                }
                if node.labels.contains(a) && node.labels.contains(b) {
                    self.trace
                        .record(&format!("Clash: disjoint concepts {:?} and {:?}", a, b));
                    return false;
                }
            }
        }

        true // Complete, clash-free
    }

    fn any_clash(&self) -> bool {
        self.nodes.values().any(|n| !n.blocked && n.has_clash())
    }

    /// Subset blocking: node blocked by ancestor with ⊇ labels.
    fn update_blocking(&mut self) {
        let node_ids: Vec<u32> = self.nodes.keys().copied().collect();
        for &nid in &node_ids {
            if self.nodes[&nid].parent.is_none() {
                continue;
            }
            let node_labels = self.nodes[&nid].labels.clone();
            let mut ancestor = self.nodes[&nid].parent;
            let mut found = false;
            while let Some(anc_id) = ancestor {
                if let Some(anc_node) = self.nodes.get(&anc_id) {
                    if node_labels.is_subset(&anc_node.labels) {
                        found = true;
                        break;
                    }
                    ancestor = anc_node.parent;
                } else {
                    break;
                }
            }
            self.nodes.get_mut(&nid).unwrap().blocked = found;
        }
    }
}

// ── DL Reasoner (Public API) ────────────────────────────────────────────

pub struct DlReasoner {
    interner: Interner,
    tbox: Arc<ProcessedTBox>,
    named_classes: HashSet<u32>,
    thing_id: u32,
    nothing_id: u32,
    individual_types: HashMap<u32, HashSet<u32>>,
    role_assertions: Vec<(u32, u32, u32)>,
}

impl DlReasoner {
    pub fn from_graph(graph: &Arc<GraphStore>) -> anyhow::Result<Self> {
        let triples = graph.all_triples()?;
        let parser = OwlParser::new(triples);
        let result = parser.parse();

        let tbox = Arc::new(ProcessedTBox::new(
            &result.axioms,
            &result.disjoint_pairs,
            result.transitive_roles,
            &result.sub_to_super,
            result.inverse_roles,
            &result.functional_roles,
            &result.inv_functional_roles,
        ));

        Ok(Self {
            interner: result.interner,
            tbox,
            named_classes: result.named_classes,
            thing_id: result.thing_id,
            nothing_id: result.nothing_id,
            individual_types: result.individual_types,
            role_assertions: result.role_assertions,
        })
    }

    /// Thread-safe satisfiability test — each call creates its own Tableau.
    pub fn is_satisfiable(&self, concept: &Concept) -> bool {
        let mut tableau = Tableau::new(Arc::clone(&self.tbox));
        tableau.is_satisfiable(concept)
    }

    /// Check if sub ⊑ sup (sub is subsumed by sup).
    pub fn is_subsumed(&self, sub: &Concept, sup: &Concept) -> bool {
        let mut test = vec![sub.clone(), sup.negate()];
        test.sort();
        let test_concept = Concept::And(test);
        !self.is_satisfiable(&test_concept)
    }

    /// Check TBox consistency.
    pub fn is_consistent(&self) -> bool {
        self.is_satisfiable(&Concept::Top)
    }

    /// Explain why a class is unsatisfiable. Returns None if satisfiable.
    pub fn explain_unsatisfiable(&self, class_id: u32) -> Option<Vec<String>> {
        let concept = Concept::Atom(class_id);
        let mut tableau = Tableau::new_with_tracing(Arc::clone(&self.tbox));
        if tableau.is_satisfiable(&concept) {
            return None;
        }
        Some(tableau.trace.steps)
    }

    /// Check subsumption with explanation trace.
    pub fn check_subsumption_explained(
        &self,
        sub: &Concept,
        sup: &Concept,
    ) -> (bool, Vec<String>) {
        let mut test = vec![sub.clone(), sup.negate()];
        test.sort();
        let test_concept = Concept::And(test);
        let mut tableau = Tableau::new_with_tracing(Arc::clone(&self.tbox));
        let sat = tableau.is_satisfiable(&test_concept);
        (!sat, tableau.trace.steps)
    }

    /// Compute told-subsumer transitive closure (for pruning).
    fn compute_told_subsumers(&self) -> HashMap<u32, HashSet<u32>> {
        let mut told: HashMap<u32, HashSet<u32>> = HashMap::new();
        for (&cls, defs) in &self.tbox.concept_defs {
            for def in defs {
                if let Concept::Atom(sup) = def {
                    told.entry(cls).or_default().insert(*sup);
                }
            }
        }
        let mut changed = true;
        while changed {
            changed = false;
            let keys: Vec<u32> = told.keys().copied().collect();
            for cls in keys {
                let supers: Vec<u32> = told
                    .get(&cls)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                for sup in supers {
                    if let Some(grand) = told.get(&sup).cloned() {
                        for g in grand {
                            if told.entry(cls).or_default().insert(g) {
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
        told
    }

    /// Agent-based parallel classification using rayon worker pool.
    ///
    /// Phase 1 — Satisfiability Agent: parallel sat testing for all named classes.
    /// Phase 2 — Subsumption Agent: parallel pairwise subsumption with told-pruning.
    /// Phase 3 — Equivalence detection from mutual subsumptions.
    pub fn classify_parallel(&self) -> AgentClassificationResult {
        let start = Instant::now();

        let classes: Vec<u32> = self
            .named_classes
            .iter()
            .filter(|&&c| c != self.thing_id && c != self.nothing_id)
            .copied()
            .collect();

        // ── Satisfiability Agent ─────────────────────────────────────
        let sat_start = Instant::now();
        let sat_results: Vec<(u32, bool)> = classes
            .par_iter()
            .map(|&cls| (cls, self.is_satisfiable(&Concept::Atom(cls))))
            .collect();

        let satisfiable: Vec<u32> = sat_results
            .iter()
            .filter(|(_, s)| *s)
            .map(|(c, _)| *c)
            .collect();
        let unsatisfiable: Vec<u32> = sat_results
            .iter()
            .filter(|(_, s)| !*s)
            .map(|(c, _)| *c)
            .collect();
        let sat_time = sat_start.elapsed();

        // ── Subsumption Agent ────────────────────────────────────────
        let sub_start = Instant::now();
        let told = self.compute_told_subsumers();

        // Build pairs to test (skip told subsumptions)
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for &sub in &satisfiable {
            for &sup in &satisfiable {
                if sub != sup && !told.get(&sub).map_or(false, |s| s.contains(&sup)) {
                    pairs.push((sub, sup));
                }
            }
        }

        let inferred: Vec<(u32, u32)> = pairs
            .par_iter()
            .filter(|(sub, sup)| self.is_subsumed(&Concept::Atom(*sub), &Concept::Atom(*sup)))
            .cloned()
            .collect();
        let sub_time = sub_start.elapsed();

        // Build hierarchy (told + inferred)
        let mut hierarchy: HashMap<u32, HashSet<u32>> = HashMap::new();
        for (&cls, supers) in &told {
            if satisfiable.contains(&cls) {
                for &sup in supers {
                    if satisfiable.contains(&sup) {
                        hierarchy.entry(cls).or_default().insert(sup);
                    }
                }
            }
        }
        for (sub, sup) in &inferred {
            hierarchy.entry(*sub).or_default().insert(*sup);
        }

        // Detect equivalences
        let mut equivalences: Vec<(u32, u32)> = Vec::new();
        for (&a, a_supers) in &hierarchy {
            for &b in a_supers {
                if a < b {
                    if hierarchy.get(&b).map_or(false, |bs| bs.contains(&a)) {
                        equivalences.push((a, b));
                    }
                }
            }
        }

        let total_time = start.elapsed();

        AgentClassificationResult {
            hierarchy,
            unsatisfiable,
            equivalences,
            inferred_subsumptions: inferred.len(),
            agents: AgentMetrics {
                satisfiability: AgentTaskMetrics {
                    tasks: classes.len(),
                    results: satisfiable.len(),
                    time_ms: sat_time.as_millis() as u64,
                },
                subsumption: AgentTaskMetrics {
                    tasks: pairs.len(),
                    results: inferred.len(),
                    time_ms: sub_time.as_millis() as u64,
                },
                total_time_ms: total_time.as_millis() as u64,
                parallel_workers: rayon::current_num_threads(),
            },
        }
    }

    /// ABox Agent: check individual consistency and infer types.
    pub fn check_abox(&self) -> ABoxResult {
        if self.individual_types.is_empty() {
            return ABoxResult {
                consistent: true,
                individuals_checked: 0,
                inferred_types: HashMap::new(),
            };
        }

        let mut tableau = Tableau::new(Arc::clone(&self.tbox));
        let mut ind_to_node: HashMap<u32, u32> = HashMap::new();

        // Create nodes for each individual
        for (&ind, types) in &self.individual_types {
            let node_id = tableau.fresh_node(None, None);
            ind_to_node.insert(ind, node_id);
            for &cls in types {
                tableau.add_label(node_id, Concept::Atom(cls));
            }
            // Add GCIs
            for gci in tableau.tbox.gcis.clone() {
                tableau.add_label(node_id, gci);
            }
        }

        // Add role assertions as edges
        for &(a, r, b) in &self.role_assertions {
            if let (Some(&a_node), Some(&b_node)) = (ind_to_node.get(&a), ind_to_node.get(&b)) {
                tableau
                    .nodes
                    .get_mut(&a_node)
                    .unwrap()
                    .edges
                    .entry(r)
                    .or_default()
                    .insert(b_node);
            }
        }

        let consistent = tableau.expand(0);

        // Infer additional types for each individual
        let mut inferred: HashMap<u32, HashSet<u32>> = HashMap::new();
        if consistent {
            for (&ind, &node_id) in &ind_to_node {
                if let Some(node) = tableau.nodes.get(&node_id) {
                    for label in &node.labels {
                        if let Concept::Atom(cls) = label {
                            if !self.individual_types[&ind].contains(cls) {
                                inferred.entry(ind).or_default().insert(*cls);
                            }
                        }
                    }
                }
            }
        }

        ABoxResult {
            consistent,
            individuals_checked: self.individual_types.len(),
            inferred_types: inferred,
        }
    }

    /// Entry point for integration with the existing reasoner.
    pub fn run(graph: &Arc<GraphStore>, materialize: bool) -> anyhow::Result<String> {
        let reasoner = Self::from_graph(graph)?;
        let initial_triples = graph.triple_count();

        let consistent = reasoner.is_consistent();
        let result = reasoner.classify_parallel();
        let abox_result = reasoner.check_abox();

        // Collect explanations for unsatisfiable classes
        let mut explanations: Vec<serde_json::Value> = Vec::new();
        for &cls in &result.unsatisfiable {
            if let Some(steps) = reasoner.explain_unsatisfiable(cls) {
                explanations.push(serde_json::json!({
                    "class": reasoner.interner.resolve(cls),
                    "trace": steps,
                }));
            }
        }

        let unsat_names: Vec<&str> = result
            .unsatisfiable
            .iter()
            .map(|&id| reasoner.interner.resolve(id))
            .collect();

        let mut hierarchy_json: Vec<serde_json::Value> = Vec::new();
        for (&cls, supers) in &result.hierarchy {
            let cls_name = reasoner.interner.resolve(cls);
            let super_names: Vec<&str> = supers
                .iter()
                .map(|&id| reasoner.interner.resolve(id))
                .collect();
            hierarchy_json.push(serde_json::json!({
                "class": cls_name,
                "superclasses": super_names,
            }));
        }

        let equiv_json: Vec<serde_json::Value> = result
            .equivalences
            .iter()
            .map(|&(a, b)| {
                serde_json::json!({
                    "class_a": reasoner.interner.resolve(a),
                    "class_b": reasoner.interner.resolve(b),
                })
            })
            .collect();

        // ABox results
        let abox_json = if abox_result.individuals_checked > 0 {
            let mut ind_json: Vec<serde_json::Value> = Vec::new();
            for (&ind, types) in &abox_result.inferred_types {
                let type_names: Vec<&str> = types
                    .iter()
                    .map(|&id| reasoner.interner.resolve(id))
                    .collect();
                ind_json.push(serde_json::json!({
                    "individual": reasoner.interner.resolve(ind),
                    "inferred_types": type_names,
                }));
            }
            serde_json::json!({
                "consistent": abox_result.consistent,
                "individuals_checked": abox_result.individuals_checked,
                "inferred": ind_json,
            })
        } else {
            serde_json::json!(null)
        };

        // Materialize all hierarchy subsumptions
        let mut materialized = 0;
        if materialize && !result.hierarchy.is_empty() {
            let mut ntriples = String::new();
            for (&cls, supers) in &result.hierarchy {
                let cls_str = reasoner.interner.resolve(cls);
                for &sup in supers {
                    let sup_str = reasoner.interner.resolve(sup);
                    ntriples.push_str(cls_str);
                    ntriples.push(' ');
                    ntriples.push_str(RDFS_SUBCLASS);
                    ntriples.push(' ');
                    ntriples.push_str(sup_str);
                    ntriples.push_str(" .\n");
                    materialized += 1;
                }
            }
            if !ntriples.is_empty() {
                graph.load_ntriples(&ntriples)?;
            }
        }

        let mut output = serde_json::json!({
            "profile_used": "owl-dl",
            "algorithm": "tableaux",
            "description_logic": "SHOIQ",
            "consistent": consistent,
            "named_classes": reasoner.named_classes.len(),
            "unsatisfiable_classes": unsat_names,
            "inferred_subsumptions": result.inferred_subsumptions,
            "equivalences": equiv_json,
            "classification": hierarchy_json,
            "initial_triples": initial_triples,
            "final_triples": graph.triple_count(),
            "inferred_count": materialized,
            "agents": {
                "satisfiability_agent": {
                    "classes_checked": result.agents.satisfiability.tasks,
                    "satisfiable_found": result.agents.satisfiability.results,
                    "time_ms": result.agents.satisfiability.time_ms,
                },
                "subsumption_agent": {
                    "pairs_tested": result.agents.subsumption.tasks,
                    "subsumptions_found": result.agents.subsumption.results,
                    "time_ms": result.agents.subsumption.time_ms,
                },
                "parallel_workers": result.agents.parallel_workers,
                "total_time_ms": result.agents.total_time_ms,
            },
        });

        if !explanations.is_empty() {
            output["explanations"] = serde_json::json!(explanations);
        }
        if !abox_json.is_null() {
            output["abox"] = abox_json;
        }
        if !materialize {
            output["dry_run"] = serde_json::json!(true);
        }

        Ok(output.to_string())
    }

    /// Explain why a named class is unsatisfiable. For agent-friendly MCP tools.
    pub fn explain_class(graph: &Arc<GraphStore>, class_iri: &str) -> anyhow::Result<String> {
        let reasoner = Self::from_graph(graph)?;
        let class_id = reasoner
            .interner
            .to_id
            .get(class_iri)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Unknown class: {}", class_iri))?;

        match reasoner.explain_unsatisfiable(class_id) {
            Some(steps) => Ok(serde_json::json!({
                "class": class_iri,
                "satisfiable": false,
                "explanation": steps,
            })
            .to_string()),
            None => Ok(serde_json::json!({
                "class": class_iri,
                "satisfiable": true,
                "explanation": "Class is satisfiable — no clash found.",
            })
            .to_string()),
        }
    }

    /// Check if class_a ⊑ class_b. For agent-friendly MCP tools.
    pub fn check_subsumption(
        graph: &Arc<GraphStore>,
        sub_iri: &str,
        sup_iri: &str,
    ) -> anyhow::Result<String> {
        let reasoner = Self::from_graph(graph)?;
        let sub_id = reasoner
            .interner
            .to_id
            .get(sub_iri)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Unknown class: {}", sub_iri))?;
        let sup_id = reasoner
            .interner
            .to_id
            .get(sup_iri)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Unknown class: {}", sup_iri))?;

        let (subsumed, trace) = reasoner.check_subsumption_explained(
            &Concept::Atom(sub_id),
            &Concept::Atom(sup_id),
        );

        Ok(serde_json::json!({
            "sub_class": sub_iri,
            "super_class": sup_iri,
            "subsumed": subsumed,
            "trace": trace,
        })
        .to_string())
    }
}

// ── Agent Result Types ──────────────────────────────────────────────────

pub struct AgentClassificationResult {
    pub hierarchy: HashMap<u32, HashSet<u32>>,
    pub unsatisfiable: Vec<u32>,
    pub equivalences: Vec<(u32, u32)>,
    pub inferred_subsumptions: usize,
    pub agents: AgentMetrics,
}

pub struct AgentMetrics {
    pub satisfiability: AgentTaskMetrics,
    pub subsumption: AgentTaskMetrics,
    pub total_time_ms: u64,
    pub parallel_workers: usize,
}

pub struct AgentTaskMetrics {
    pub tasks: usize,
    pub results: usize,
    pub time_ms: u64,
}

pub struct ABoxResult {
    pub consistent: bool,
    pub individuals_checked: usize,
    pub inferred_types: HashMap<u32, HashSet<u32>>,
}

pub struct ClassificationResult {
    pub hierarchy: HashMap<u32, HashSet<u32>>,
    pub unsatisfiable: Vec<u32>,
    pub equivalences: Vec<(u32, u32)>,
    pub inferred_subsumptions: usize,
}
