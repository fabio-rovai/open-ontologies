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

use std::collections::{BTreeSet, HashMap, HashSet};
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
const RDFS_DOMAIN: &str = "<http://www.w3.org/2000/01/rdf-schema#domain>";
const RDFS_RANGE: &str = "<http://www.w3.org/2000/01/rdf-schema#range>";
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
const OWL_EXACT_QCARD: &str = "<http://www.w3.org/2002/07/owl#qualifiedCardinality>";
const OWL_MIN_QCARD: &str = "<http://www.w3.org/2002/07/owl#minQualifiedCardinality>";
const OWL_MAX_QCARD: &str = "<http://www.w3.org/2002/07/owl#maxQualifiedCardinality>";
const OWL_ON_CLASS: &str = "<http://www.w3.org/2002/07/owl#onClass>";

// Tableaux safety limits live in `crate::runtime` (initialised from
// `[reasoner] tableaux_max_depth` / `tableaux_max_nodes` in config.toml).

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
        let mut individual_anon_types: HashMap<u32, Vec<Concept>> = HashMap::new();
        let mut role_assertions: Vec<(u32, u32, u32)> = Vec::new();
        let mut data_assertions: Vec<(u32, u32, u32)> = Vec::new();
        let mut role_domains: Vec<(u32, Concept)> = Vec::new();
        let mut role_ranges: Vec<(u32, Concept)> = Vec::new();

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

        // owl:{Transitive,Symmetric,Functional,InverseFunctional}Property and any
        // property named by owl:inverseOf are object properties by OWL semantics,
        // whether or not they are also explicitly typed owl:ObjectProperty. Without
        // this, a role declared only by its characteristic has its ABox role
        // assertions dropped at edge-building time and the characteristic is inert.
        for &r in transitive_roles
            .iter()
            .chain(functional_roles.iter())
            .chain(inv_functional_roles.iter())
        {
            object_properties.insert(r);
        }
        for (&a, &b) in &inverse_roles {
            object_properties.insert(a);
            object_properties.insert(b);
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
        // A class is a class because axioms treat it as one, not because someone remembered
        // to type it `owl:Class`. Real ontologies routinely declare a class only through
        // rdfs:subClassOf / owl:equivalentClass / owl:disjointWith; gating discovery on the
        // explicit typing silently dropped such classes from the satisfiability sweep, so an
        // unsatisfiable class could be reported satisfiable by omission. Blank-node class
        // expressions are excluded: only IRIs name checkable classes.
        for (a, b) in &subclass_pairs {
            for side in [a, b] {
                if side.starts_with('<') && side != OWL_THING && side != OWL_NOTHING {
                    let id = self.interner.intern(side);
                    named_classes.insert(id);
                }
            }
        }
        for (sub_str, sup_str) in subclass_pairs {
            let sub = self.parse_class_expr(&sub_str);
            let sup = self.parse_class_expr(&sup_str);
            axioms.push((sub.to_nnf(), sup.to_nnf()));
        }

        // Collect EquivalentClass axioms (→ bidirectional SubClassOf)
        let mut definitions: HashMap<u32, Concept> = HashMap::new();
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
        for (a, b) in &equiv_pairs {
            for side in [a, b] {
                if side.starts_with('<') && side != OWL_THING && side != OWL_NOTHING {
                    let id = self.interner.intern(side);
                    named_classes.insert(id);
                }
            }
        }
        for (a_str, b_str) in equiv_pairs {
            let a = self.parse_class_expr(&a_str);
            let b = self.parse_class_expr(&b_str);
            let a_nnf = a.to_nnf();
            let b_nnf = b.to_nnf();
            // Keep the definition in structural form. Realization needs to know that a
            // named class IS a particular conjunction, which cannot be recovered from the
            // GCI encoding once negation has been pushed through it.
            if let Concept::Atom(id) = a_nnf {
                definitions.insert(id, b_nnf.clone());
            } else if let Concept::Atom(id) = b_nnf {
                definitions.insert(id, a_nnf.clone());
            }
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
        for (a, b) in &disjoint_raw {
            for side in [a, b] {
                if side.starts_with('<') && side != OWL_THING && side != OWL_NOTHING {
                    let id = self.interner.intern(side);
                    named_classes.insert(id);
                }
            }
        }
        for (a_str, b_str) in disjoint_raw {
            let a = self.parse_class_expr(&a_str).to_nnf();
            let b = self.parse_class_expr(&b_str).to_nnf();
            disjoint_pairs.push((a, b));
        }

        // Collect rdfs:domain / rdfs:range as GCIs.
        //
        // These are ordinary DL axioms and were previously not read at all, so every
        // incoherence arising from a property's domain or range colliding with a
        // disjointness axiom was reported satisfiable. Measured on the GCHQ-published
        // hqdm.owl: HermiT finds 39 unsatisfiable classes, this reasoner found 0,
        // because that file carries 265 domain and 260 range declarations and nothing
        // consumed them. The failure direction is the dangerous one - it answers
        // "satisfiable" when it cannot tell - so a coherence gate built on it passes
        // everything.
        //
        //   domain(p, D)  =>  exists p.Top  [=  D
        //   range(p, R)   =>  Top  [=  forall p.R
        //
        // Datatype ranges are skipped: xsd:* and rdfs:Literal are not concepts, and
        // asserting Top [= forall p.xsd:string would be a type error, not an axiom.
        let domain_range_raw: Vec<(String, String, bool)> = self
            .index
            .by_subject
            .iter()
            .flat_map(|(s, pairs)| {
                pairs.iter().filter_map(move |(p, o)| {
                    if p == RDFS_DOMAIN {
                        Some((s.clone(), o.clone(), true))
                    } else if p == RDFS_RANGE {
                        Some((s.clone(), o.clone(), false))
                    } else {
                        None
                    }
                })
            })
            .collect();
        for (prop_str, cls_str, is_domain) in domain_range_raw {
            if cls_str.starts_with("<http://www.w3.org/2001/XMLSchema#")
                || cls_str == "<http://www.w3.org/2000/01/rdf-schema#Literal>"
                || cls_str == OWL_THING
            {
                continue;
            }
            let role = self.interner.intern(&prop_str);
            let cls = self.parse_class_expr(&cls_str).to_nnf();
            if cls_str.starts_with('<') && cls_str != OWL_NOTHING {
                let id = self.interner.intern(&cls_str);
                named_classes.insert(id);
            }
            if is_domain {
                role_domains.push((role, cls));
            } else {
                role_ranges.push((role, cls));
            }
        }

        // Collect individuals and their types + role assertions.
        //
        // An individual is discovered two ways: the explicit `owl:NamedIndividual` typing,
        // or an rdf:type pointing at a known class. Instance data in the wild almost never
        // carries the explicit typing, and gating on it meant the ABox check received no
        // individuals at all, so a textbook inconsistency - one individual typed into two
        // disjoint classes - sailed through as consistent. Subjects that are themselves
        // schema declarations are excluded, so a class or property never doubles as an
        // individual by accident.
        let schema_markers: [&str; 10] = [
            OWL_CLASS,
            OWL_OBJECT_PROPERTY,
            OWL_TRANSITIVE,
            "<http://www.w3.org/2002/07/owl#DatatypeProperty>",
            "<http://www.w3.org/2002/07/owl#AnnotationProperty>",
            "<http://www.w3.org/2002/07/owl#FunctionalProperty>",
            "<http://www.w3.org/2002/07/owl#InverseFunctionalProperty>",
            "<http://www.w3.org/2002/07/owl#SymmetricProperty>",
            "<http://www.w3.org/2002/07/owl#Ontology>",
            "<http://www.w3.org/2000/01/rdf-schema#Class>",
        ];
        let mut individual_subjects: Vec<String> = Vec::new();
        for (subject, types) in &subject_types {
            if types.iter().any(|t| schema_markers.contains(&t.as_str())) {
                continue;
            }
            let explicit = types.iter().any(|t| t == OWL_NAMED_INDIVIDUAL);
            let typed_by_known_class = types.iter().any(|t| {
                t.starts_with('<') && {
                    let id = self.interner.intern(t);
                    named_classes.contains(&id)
                }
            });
            // An individual typed only by an anonymous class expression is still an
            // individual, and its constraint still has to be checked.
            let typed_by_anon_class = types.iter().any(|t| t.starts_with("_:"));
            if explicit || typed_by_known_class || typed_by_anon_class {
                individual_subjects.push(subject.clone());
            }
        }
        individual_subjects.sort_unstable();
        for ind_str in &individual_subjects {
            let ind_id = self.interner.intern(ind_str);
            let types = self.index.objects(ind_str, RDF_TYPE);
            for t in &types {
                if t.starts_with("_:") {
                    // Anonymous class expression: parse it and attach the concept
                    // so the constraint reaches the tableau. A blank node is never a
                    // named class, so this is disjoint from the branch below.
                    let concept = self.parse_class_expr(t).to_nnf();
                    individual_anon_types.entry(ind_id).or_default().push(concept);
                    continue;
                }
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
                    } else {
                        // Datatype assertions are kept apart from role assertions on purpose.
                        // They must not become tableau edges, since a literal is not a node,
                        // but realization needs them: owl:hasValue on a datatype property is
                        // how a defined class picks out individuals by a flag, and dropping
                        // these made every such class permanently empty.
                        let o_id = self.interner.intern(o);
                        data_assertions.push((ind_id, p_id, o_id));
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
            role_domains,
            role_ranges,
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
            individual_anon_types,
            role_assertions,
            data_assertions,
            definitions,
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
        if node.starts_with("_:")
            && let Some(c) = self.try_parse_complex(node) {
                return c;
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
            && let Some(n) = parse_card_value(&val) {
                return RawConcept::MinCard(prop, n, Box::new(filler));
            }
        // maxQualifiedCardinality / maxCardinality → ≤n R.C
        if let Some(val) = self
            .index
            .object(node, OWL_MAX_QCARD)
            .or_else(|| self.index.object(node, OWL_MAX_CARD))
            && let Some(n) = parse_card_value(&val) {
                return RawConcept::MaxCard(prop, n, Box::new(filler));
            }
        // qualifiedCardinality / cardinality → ≥n R.C ⊓ ≤n R.C
        //
        // The qualified form was missing while min and max both had it, so an
        // `owl:qualifiedCardinality` restriction fell through to Top: no
        // successor was ever generated and nothing downstream of it could
        // clash. That is how hqdm.owl's `defined_relationship` reads as
        // satisfiable — its =1 member_of_kind successor, which rdfs:range and
        // owl:onClass place in two disjoint classes, was never created.
        if let Some(val) = self
            .index
            .object(node, OWL_EXACT_QCARD)
            .or_else(|| self.index.object(node, OWL_EXACT_CARD))
            && let Some(n) = parse_card_value(&val) {
                return RawConcept::And(vec![
                    RawConcept::MinCard(prop, n, Box::new(filler.clone())),
                    RawConcept::MaxCard(prop, n, Box::new(filler)),
                ]);
            }

        RawConcept::Top
    }
}

/// Parse cardinality value from OWL literal (e.g., "2"^^<xsd:nonNegativeInteger>).
fn parse_card_value(literal: &str) -> Option<u32> {
    if let Some(rest) = literal.strip_prefix('"')
        && let Some(end) = rest.find('"')
    {
        return rest[..end].parse().ok();
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
    /// rdfs:domain / rdfs:range, kept as ROLE METADATA rather than GCIs.
    /// Encoding them as axioms puts a non-atomic left-hand side into the GCI
    /// list, which lands a disjunction on every node and multiplies branching.
    role_domains: Vec<(u32, Concept)>,
    role_ranges: Vec<(u32, Concept)>,
    individual_types: HashMap<u32, HashSet<u32>>,
    /// Individuals typed directly by an ANONYMOUS class expression
    /// (`:a rdf:type [ owl:Restriction … ]`, an intersection/union/complement).
    /// These carry no named-class id, so they cannot live in `individual_types`,
    /// but the constraint they impose is real and an inconsistency arising from
    /// it must be found rather than silently dropped.
    individual_anon_types: HashMap<u32, Vec<Concept>>,
    role_assertions: Vec<(u32, u32, u32)>,
    /// Datatype-property assertions, for realization only; never tableau edges.
    data_assertions: Vec<(u32, u32, u32)>,
    /// Named class → the concept it is defined as being equivalent to.
    definitions: HashMap<u32, Concept>,
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
    /// Transitive closure of asserted Atom ⊑ Atom subsumption, used to decide
    /// cardinality clashes whose fillers are related by subsumption rather than
    /// syntactically equal.
    class_closure: HashMap<u32, HashSet<u32>>,
    /// Effective rdfs:domain per role, super-role constraints already folded in.
    /// Applied to the SOURCE node when an edge is created.
    role_domain: HashMap<u32, Vec<Concept>>,
    /// Effective rdfs:range per role, likewise. Applied to the TARGET node.
    role_range: HashMap<u32, Vec<Concept>>,
}

/// Role-level axioms, grouped so `ProcessedTBox::new` keeps a workable
/// argument count as more role metadata (domains, ranges) is absorbed out of
/// the GCI list.
struct RoleAxioms<'a> {
    transitive_roles: HashSet<u32>,
    sub_to_super: &'a HashMap<u32, HashSet<u32>>,
    inverse_roles: HashMap<u32, u32>,
    functional_roles: &'a HashSet<u32>,
    inv_functional_roles: &'a HashSet<u32>,
    role_domains: &'a [(u32, Concept)],
    role_ranges: &'a [(u32, Concept)],
}

impl ProcessedTBox {
    fn new(
        axioms: &[(Concept, Concept)],
        disjoint_pairs: &[(Concept, Concept)],
        roles: RoleAxioms<'_>,
    ) -> Self {
        let RoleAxioms {
            transitive_roles,
            sub_to_super,
            inverse_roles,
            functional_roles,
            inv_functional_roles,
            role_domains,
            role_ranges,
        } = roles;
        let mut concept_defs: HashMap<u32, Vec<Concept>> = HashMap::new();
        let mut gcis: Vec<Concept> = Vec::new();

        // ── GCI absorption ──────────────────────────────────────────────
        //
        // An axiom with a non-atomic left-hand side becomes ¬sub ⊔ sup and is
        // added to EVERY node, so each one doubles the branching factor of the
        // whole tableau. Two standard rewrites turn most of them into atomic
        // left-hand sides, which absorb into concept_defs and fire only when
        // the class actually appears in a label:
        //
        //   (C₁ ⊔ C₂) ⊑ D   ==>   C₁ ⊑ D  and  C₂ ⊑ D
        //   C ⊑ (D₁ ⊓ D₂)   ==>   C ⊑ D₁  and  C ⊑ D₂
        //
        // The first is what hqdm.owl needs: `abstract_object ≡ class ⊔
        // relationship` produces a disjunctive left-hand side that kept the
        // whole ontology undecidable within budget.
        let mut work: Vec<(Concept, Concept)> = axioms.to_vec();
        let mut absorbed: Vec<(Concept, Concept)> = Vec::new();
        let mut guard = 0usize;
        while let Some((sub, sup)) = work.pop() {
            guard += 1;
            if guard > 100_000 {
                absorbed.push((sub, sup));
                continue;
            }
            match (&sub, &sup) {
                (Concept::Or(parts), _) if !parts.is_empty() => {
                    for part in parts {
                        work.push((part.clone(), sup.clone()));
                    }
                }
                (_, Concept::And(parts)) if !parts.is_empty() => {
                    for part in parts {
                        work.push((sub.clone(), part.clone()));
                    }
                }
                _ => absorbed.push((sub, sup)),
            }
        }
        let axioms: &[(Concept, Concept)] = &absorbed;

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

        // Transitive closure of asserted Atom ⊑ Atom subsumption. Needed because a
        // cardinality bound stated on a superclass constrains its subclasses too,
        // which a syntactic filler comparison cannot see.
        let mut direct: HashMap<u32, HashSet<u32>> = HashMap::new();
        for (sub, sup) in axioms {
            if let (Concept::Atom(a), Concept::Atom(b)) = (sub, sup)
                && a != b
            {
                direct.entry(*a).or_default().insert(*b);
            }
        }
        let mut class_closure: HashMap<u32, HashSet<u32>> = HashMap::new();
        for sub in direct.keys().copied().collect::<Vec<_>>() {
            let mut seen: HashSet<u32> = HashSet::new();
            let mut stack: Vec<u32> = direct
                .get(&sub)
                .map(|v| v.iter().copied().collect())
                .unwrap_or_default();
            while let Some(x) = stack.pop() {
                if seen.insert(x)
                    && let Some(next) = direct.get(&x)
                {
                    stack.extend(next.iter().copied());
                }
            }
            class_closure.insert(sub, seen);
        }

        // Domain and range are role metadata, applied when an edge is created,
        // NOT axioms. As GCIs they are `∃p.⊤ ⊑ D` and `⊤ ⊑ ∀p.R`, both with a
        // non-atomic left-hand side, so both land a disjunction on every node
        // and branching grows with the number of property declarations. hqdm.owl
        // alone carries 525 of them.
        //
        // A constraint on a super-role binds its sub-roles, so each role's list
        // is closed over its transitive super-roles once, here, rather than
        // walked on every edge.
        let mut super_of: HashMap<u32, HashSet<u32>> = HashMap::new();
        for (&sub, sups) in sub_to_super {
            let mut seen: HashSet<u32> = HashSet::new();
            let mut stack: Vec<u32> = sups.iter().copied().collect();
            while let Some(x) = stack.pop() {
                if seen.insert(x)
                    && let Some(next) = sub_to_super.get(&x)
                {
                    stack.extend(next.iter().copied());
                }
            }
            super_of.insert(sub, seen);
        }
        let fold = |src: &[(u32, Concept)]| -> HashMap<u32, Vec<Concept>> {
            let mut direct: HashMap<u32, Vec<Concept>> = HashMap::new();
            for (r, c) in src {
                direct.entry(*r).or_default().push(c.clone());
            }
            let mut out = direct.clone();
            for (role, sups) in &super_of {
                for sup in sups {
                    if let Some(cs) = direct.get(sup) {
                        out.entry(*role).or_default().extend(cs.iter().cloned());
                    }
                }
            }
            for v in out.values_mut() {
                v.sort();
                v.dedup();
            }
            out
        };
        let role_domain = fold(role_domains);
        let role_range = fold(role_ranges);

        Self {
            concept_defs,
            gcis,
            disjoint_pairs: disjoint_pairs.to_vec(),
            transitive_roles,
            super_to_sub: super_to_sub_map,
            inverse_roles,
            class_closure,
            role_domain,
            role_range,
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
            if let Concept::Atom(a) = label
                && self.labels.contains(&Concept::NegAtom(*a)) {
                    return true;
                }
        }
        // MinCard/MaxCard direct clash: ≥n1 R.C and ≤n2 R.C with n1 > n2
        for label in &self.labels {
            if let Concept::MinCard(r1, n1, f1) = label {
                for other in &self.labels {
                    if let Concept::MaxCard(r2, n2, f2) = other
                        && r1 == r2 && n1 > n2 && (f1 == f2 || **f2 == Concept::Top) {
                            return true;
                        }
                }
            }
            // Exists(R, C) = ≥1 R.C clashes with MaxCard(R, 0, C/Top)
            if let Concept::Exists(r1, f1) = label {
                for other in &self.labels {
                    if let Concept::MaxCard(r2, n2, f2) = other
                        && r1 == r2 && *n2 == 0 && (f1 == f2 || **f2 == Concept::Top) {
                            return true;
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

/// Outcome of a satisfiability test.
///
/// A two-valued `bool` cannot express the difference between "I constructed a
/// clash, this concept is impossible" and "I ran out of budget before I could
/// decide". Conflating them is a soundness bug: the caller turns exhaustion
/// into an asserted `owl:Nothing` subsumption. This enum makes the third
/// outcome unavoidable at the type level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// A complete, clash-free model was constructed.
    Satisfiable,
    /// Every branch clashed. This is a proof.
    Unsatisfiable,
    /// A resource budget was hit first. Nothing is proven either way.
    Unknown,
}

impl Verdict {
    /// True only when the concept is PROVEN satisfiable. `Unknown` is not.
    ///
    /// Note that this is NOT the negation of [`Verdict::is_unsat`]: both return
    /// false for `Unknown`. Callers must handle the third case explicitly,
    /// which is the whole point of the type.
    pub fn is_sat(self) -> bool {
        matches!(self, Verdict::Satisfiable)
    }

    /// True only when unsatisfiability is PROVEN. `Unknown` is not.
    pub fn is_unsat(self) -> bool {
        matches!(self, Verdict::Unsatisfiable)
    }

    /// True when no decision was reached. Callers that treat this as either
    /// answer are reintroducing the bug this enum exists to prevent.
    pub fn is_unknown(self) -> bool {
        matches!(self, Verdict::Unknown)
    }
}

/// Why a tableau run stopped short of a decision.
#[derive(Debug, Clone, Copy, Default)]
struct Budget {
    /// Wall-clock cut-off for a single satisfiability test.
    deadline: Option<Instant>,
    /// Set when any budget was hit during expansion.
    exhausted: bool,
}

impl Budget {
    fn expired(&self) -> bool {
        self.deadline.is_some_and(|d| Instant::now() >= d)
    }
}

#[derive(Clone)]
struct Tableau {
    nodes: HashMap<u32, TNode>,
    next_id: u32,
    tbox: Arc<ProcessedTBox>,
    trace: ExplanationTrace,
    budget: Budget,
}

impl Tableau {
    fn new(tbox: Arc<ProcessedTBox>) -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: 0,
            tbox,
            trace: ExplanationTrace::new(false),
            budget: Budget::default(),
        }
    }

    fn with_deadline(tbox: Arc<ProcessedTBox>, deadline: Option<Instant>) -> Self {
        let mut t = Self::new(tbox);
        t.budget.deadline = deadline;
        t
    }

    fn new_with_tracing(tbox: Arc<ProcessedTBox>) -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: 0,
            tbox,
            trace: ExplanationTrace::new(true),
            budget: Budget::default(),
        }
    }

    fn decide(&mut self, concept: &Concept) -> Verdict {
        let root = self.fresh_node(None, None);
        self.add_label(root, concept.clone());
        // Add GCIs to root
        for gci in self.tbox.gcis.clone() {
            self.add_label(root, gci);
        }
        if self.expand(0) {
            Verdict::Satisfiable
        } else if self.budget.exhausted {
            // We never completed the search, so we have proven nothing.
            Verdict::Unknown
        } else {
            Verdict::Unsatisfiable
        }
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
        if let Concept::Atom(a) = &concept
            && let Some(defs) = self.tbox.concept_defs.get(a).cloned() {
                for d in defs {
                    self.add_label(node_id, d);
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
        if let Some(parent_id) = node.parent
            && let Some(parent_role) = node.parent_role
                && let Some(&inv_of_parent) = self.tbox.inverse_roles.get(&parent_role)
                    && role == inv_of_parent {
                        result.insert(parent_id);
                    }

        result
    }

    /// Create a new successor node and set up edges (including inverse back-edges).
    fn create_successor(&mut self, parent_id: u32, role: u32, filler: Concept) -> u32 {
        let succ = self.fresh_node(Some(parent_id), Some(role));
        self.add_label(succ, filler);

        // rdfs:range binds the target, rdfs:domain binds the source. Doing it
        // here keeps both out of the GCI list, so they cost one label each on
        // the two nodes actually involved instead of a disjunction on every
        // node in the tableau.
        if let Some(rs) = self.tbox.role_range.get(&role).cloned() {
            for c in rs {
                self.add_label(succ, c);
            }
        }
        if let Some(ds) = self.tbox.role_domain.get(&role).cloned() {
            for c in ds {
                self.add_label(parent_id, c);
            }
        }

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
                if let Concept::ForAll(r, f) = label
                    && *r == sup_role {
                        self.add_label(succ, *f.clone());
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
        let mut all_ids: Vec<u32> = self.nodes.keys().copied().collect();
        // Sorted: HashMap iteration order is seeded per process, so an unsorted
        // traversal makes expansion order (and therefore which checks finish
        // inside budget) vary run to run. Node ids are assigned sequentially, so
        // sorting by id is creation order.
        all_ids.sort_unstable();
        for &nid in &all_ids {
            if nid == remove_id {
                continue;
            }
            let node = self.nodes.get_mut(&nid).unwrap();
            for targets in node.edges.values_mut() {
                if targets.remove(&remove_id)
                    && nid != keep_id {
                        targets.insert(keep_id);
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
        let max_depth = crate::runtime::tableaux_max_depth();
        let max_nodes = crate::runtime::tableaux_max_nodes();
        // Hitting a budget is NOT a clash. Record it so the caller can report
        // Unknown instead of asserting unsatisfiability, then unwind. The
        // `false` return here means only "this branch produced no model".
        if depth > max_depth || self.nodes.len() > max_nodes || self.budget.expired() {
            self.budget.exhausted = true;
            return false;
        }

        // Apply deterministic rules until fixpoint
        loop {
            // The deadline must be checked HERE, not only on entry to expand().
            // This fixpoint loop can run for a very long time on a single call
            // (every iteration re-runs blocking over every node), so a check
            // only at function entry lets one expansion overrun the budget
            // without bound.
            if self.budget.expired() {
                self.budget.exhausted = true;
                return false;
            }
            if self.any_clash() {
                return false;
            }
            let mut changed = false;
            let mut node_ids: Vec<u32> = self.nodes.keys().copied().collect();
            // Sorted: HashMap iteration order is seeded per process, so an unsorted
            // traversal makes expansion order (and therefore which checks finish
            // inside budget) vary run to run. Node ids are assigned sequentially, so
            // sorting by id is creation order.
            node_ids.sort_unstable();

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
                                    self.nodes.get(&s).is_some_and(|n| n.labels.contains(&filler))
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
                                    self.nodes.get(&s).is_some_and(|n| n.labels.contains(&filler))
                                })
                                .count();
                            if matching < n {
                                // Guard EVERY iteration: n comes from an owl:minCardinality
                                // literal with no magnitude cap, so an ingested value in the
                                // billions would allocate that many nodes before the outer
                                // fixpoint check (which only runs between passes) ever sees
                                // them. Hitting the node budget is not a clash; record it and
                                // unwind so the caller reports Unknown, not a fabricated answer.
                                let max_nodes = crate::runtime::tableaux_max_nodes();
                                for _ in 0..(n - matching) {
                                    if self.nodes.len() > max_nodes || self.budget.expired() {
                                        self.budget.exhausted = true;
                                        return false;
                                    }
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
        let mut node_ids: Vec<u32> = self.nodes.keys().copied().collect();
        // Sorted: HashMap iteration order is seeded per process, so an unsorted
        // traversal makes expansion order (and therefore which checks finish
        // inside budget) vary run to run. Node ids are assigned sequentially, so
        // sorting by id is creation order.
        node_ids.sort_unstable();
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
                            .is_some_and(|node| node.labels.contains(&filler))
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
                            // Carry the budget flag back: a branch that ran out
                            // of room does not license "all merges clash".
                            self.budget.exhausted |= branch.budget.exhausted;
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
        let mut node_ids: Vec<u32> = self.nodes.keys().copied().collect();
        // Sorted: HashMap iteration order is seeded per process, so an unsorted
        // traversal makes expansion order (and therefore which checks finish
        // inside budget) vary run to run. Node ids are assigned sequentially, so
        // sorting by id is creation order.
        node_ids.sort_unstable();
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
                        // Same here: an exhausted disjunct is not a refuted one.
                        self.budget.exhausted |= branch.budget.exhausted;
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
        self.nodes
            .values()
            .any(|n| !n.blocked && (n.has_clash() || self.card_clash_via_hierarchy(n)))
    }

    /// ≥n1 R.C against ≤n2 R.D where n1 > n2 and C ⊑ D.
    ///
    /// `TNode::has_clash` only fires when the two fillers are syntactically equal
    /// or the max's filler is ⊤. That misses the common shape where a class
    /// asserts ≥2 R.C while inheriting ≤1 R.D from an ancestor with C ⊑ D: every
    /// C-successor is also a D-successor, so the bound is violated and the class
    /// is unsatisfiable.
    ///
    /// Measured on the GCHQ-published hqdm.owl, whose 39 unsatisfiable classes
    /// are all of this shape. Ablating its cardinality axioms takes the count to
    /// 0, while ablating rdfs:domain, rdfs:range or owl:disjointWith each leave
    /// all 39 standing.
    fn card_clash_via_hierarchy(&self, n: &TNode) -> bool {
        let subsumed = |a: &Concept, b: &Concept| -> bool {
            match (a, b) {
                (Concept::Atom(x), Concept::Atom(y)) => {
                    x == y
                        || self
                            .tbox
                            .class_closure
                            .get(x)
                            .is_some_and(|anc| anc.contains(y))
                }
                _ => false,
            }
        };
        for label in &n.labels {
            let (r1, n1, f1) = match label {
                Concept::MinCard(r, k, f) => (*r, *k, f.as_ref()),
                Concept::Exists(r, f) => (*r, 1u32, f.as_ref()),
                _ => continue,
            };
            for other in &n.labels {
                if let Concept::MaxCard(r2, n2, f2) = other
                    && r1 == *r2
                    && n1 > *n2
                    && subsumed(f1, f2)
                {
                    return true;
                }
            }
        }
        false
    }

    /// Subset blocking: node blocked by ancestor with ⊇ labels.
    fn update_blocking(&mut self) {
        let mut node_ids: Vec<u32> = self.nodes.keys().copied().collect();
        // Sorted: HashMap iteration order is seeded per process, so an unsorted
        // traversal makes expansion order (and therefore which checks finish
        // inside budget) vary run to run. Node ids are assigned sequentially, so
        // sorting by id is creation order.
        node_ids.sort_unstable();
        for &nid in &node_ids {
            let blocked = self.is_pairwise_blocked(nid);
            self.nodes.get_mut(&nid).unwrap().blocked = blocked;
        }
    }

    /// The set of roles labelling the edge from `from` to `to`.
    fn roles_between(&self, from: u32, to: u32) -> BTreeSet<u32> {
        match self.nodes.get(&from) {
            Some(n) => n
                .edges
                .iter()
                .filter(|(_, targets)| targets.contains(&to))
                .map(|(r, _)| *r)
                .collect(),
            None => BTreeSet::new(),
        }
    }

    /// Pairwise (double) ancestor blocking.
    ///
    /// Horrocks and Sattler, *A Tableau Decision Procedure for SHOIQ* (JAR 39,
    /// 2007): once a logic has BOTH inverse roles and number restrictions,
    /// single-node blocking is not sound. The reason is unravelling. To build a
    /// model from a completion graph you replicate the fragment between the
    /// blocked node and its blocker infinitely often, and that only yields a
    /// model if the blocked node behaves like its blocker *in its context*.
    /// With inverse roles a node constrains its predecessor, so the contexts
    /// have to match too.
    ///
    /// The condition, for `s` a blockable successor of `s'` and `t` a blockable
    /// successor of `t'`, is that `t` blocks `s` iff
    ///
    ///   t ≺ s,  L(s) = L(t),  L(s') = L(t'),  L(s,s') = L(t,t'),  L(s',s) = L(t',t)
    ///
    /// Note `=`, not `⊆`. This replaces the previous implementation, which used
    /// ancestor SUBSET blocking on the node label alone and ignored parents and
    /// edge labels entirely. That was adequate for ALC but unsound for the
    /// SHOIQ this reasoner advertises.
    ///
    /// This is deliberately the classical ancestor variant rather than HermiT's
    /// "anywhere" blocking. Anywhere blocking yields smaller models but is a
    /// further optimisation on top of a correct base; get the base right first.
    fn is_pairwise_blocked(&self, s: u32) -> bool {
        // Only blockable successors can be blocked. A node with no parent is
        // a root and is never blocked.
        let Some(s_parent) = self.nodes.get(&s).and_then(|n| n.parent) else {
            return false;
        };
        let Some(s_node) = self.nodes.get(&s) else {
            return false;
        };
        let Some(s_parent_node) = self.nodes.get(&s_parent) else {
            return false;
        };

        let s_labels = &s_node.labels;
        let s_parent_labels = &s_parent_node.labels;
        let s_down = self.roles_between(s_parent, s);
        let s_up = self.roles_between(s, s_parent);

        // Walk strict ancestors, looking for a blocker.
        let mut cur = s_node.parent;
        while let Some(t) = cur {
            let Some(t_node) = self.nodes.get(&t) else {
                break;
            };
            if let Some(t_parent) = t_node.parent {
                // Node ids are allocated monotonically, so `t < s` is the
                // strict ordering the calculus requires. It also guarantees we
                // never create a blocking cycle.
                if t < s
                    && t_node.labels == *s_labels
                    && self
                        .nodes
                        .get(&t_parent)
                        .is_some_and(|tp| tp.labels == *s_parent_labels)
                    && self.roles_between(t_parent, t) == s_down
                    && self.roles_between(t, t_parent) == s_up
                {
                    return true;
                }
            }
            cur = t_node.parent;
        }
        false
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
    individual_anon_types: HashMap<u32, Vec<Concept>>,
    role_assertions: Vec<(u32, u32, u32)>,
    data_assertions: Vec<(u32, u32, u32)>,
    definitions: HashMap<u32, Concept>,
    /// Sub-class closure over asserted rdfs:subClassOf, used by realization.
    subclass_closure: HashMap<u32, HashSet<u32>>,
    /// Wall-clock cut-off applied to each individual satisfiability test.
    /// `None` means no time limit (the node/depth budgets still apply).
    deadline: Option<Instant>,
}

impl DlReasoner {
    pub fn from_graph(graph: &Arc<GraphStore>) -> anyhow::Result<Self> {
        let triples = graph.all_triples()?;
        let parser = OwlParser::new(triples);
        let result = parser.parse();

        // Transitive closure of asserted CLASS subsumption, so an individual typed
        // EnumerationTerm satisfies a conjunct requiring ClassTerm.
        //
        // Built from the axiom list, not from `sub_to_super`: that map is the ROLE hierarchy
        // (rdfs:subPropertyOf). Using it here silently produced an empty class closure and
        // no individual ever matched a definition naming a superclass.
        let mut direct: HashMap<u32, HashSet<u32>> = HashMap::new();
        for (sub, sup) in &result.axioms {
            if let (Concept::Atom(a), Concept::Atom(b)) = (sub, sup)
                && a != b {
                    direct.entry(*a).or_default().insert(*b);
                }
        }
        let mut closure: HashMap<u32, HashSet<u32>> = HashMap::new();
        for sub in direct.keys().copied().collect::<Vec<_>>() {
            let mut seen: HashSet<u32> = HashSet::new();
            let mut stack: Vec<u32> = direct.get(&sub).map(|v| v.iter().copied().collect()).unwrap_or_default();
            while let Some(x) = stack.pop() {
                if seen.insert(x)
                    && let Some(next) = direct.get(&x) {
                        stack.extend(next.iter().copied());
                    }
            }
            closure.insert(sub, seen);
        }

        let tbox = Arc::new(ProcessedTBox::new(
            &result.axioms,
            &result.disjoint_pairs,
            RoleAxioms {
                transitive_roles: result.transitive_roles,
                sub_to_super: &result.sub_to_super,
                inverse_roles: result.inverse_roles,
                functional_roles: &result.functional_roles,
                inv_functional_roles: &result.inv_functional_roles,
                role_domains: &result.role_domains,
                role_ranges: &result.role_ranges,
            },
        ));

        Ok(Self {
            interner: result.interner,
            tbox,
            named_classes: result.named_classes,
            thing_id: result.thing_id,
            nothing_id: result.nothing_id,
            individual_types: result.individual_types,
            individual_anon_types: result.individual_anon_types,
            role_assertions: result.role_assertions,
            data_assertions: result.data_assertions,
            definitions: result.definitions,
            subclass_closure: closure,
            deadline: crate::runtime::tableaux_test_timeout_ms()
                .map(|ms| Instant::now() + std::time::Duration::from_millis(ms)),
        })
    }

    /// Three-valued satisfiability test — each call creates its own Tableau.
    ///
    /// Returns `Unknown` when a resource budget was hit before a decision was
    /// reached. Callers MUST NOT read `Unknown` as either answer.
    pub fn decide_satisfiable(&self, concept: &Concept) -> Verdict {
        let mut tableau = Tableau::with_deadline(Arc::clone(&self.tbox), self.deadline);
        tableau.decide(concept)
    }

    /// Thread-safe satisfiability test.
    ///
    /// Collapses `Unknown` to `true`, which is the SAFE direction: an
    /// undecided class is left in the hierarchy rather than being declared
    /// impossible. Prefer `decide_satisfiable` where the distinction matters.
    pub fn is_satisfiable(&self, concept: &Concept) -> bool {
        !self.decide_satisfiable(concept).is_unsat()
    }

    /// Check if sub ⊑ sup (sub is subsumed by sup).
    ///
    /// A subsumption is asserted ONLY when the negated test concept is proven
    /// unsatisfiable. `Unknown` yields `false`: we decline to claim a
    /// subsumption we could not establish.
    pub fn is_subsumed(&self, sub: &Concept, sup: &Concept) -> bool {
        self.decide_subsumption(sub, sup).is_unsat()
    }

    /// Subsumption as a three-valued verdict. `Unsatisfiable` on the negated
    /// test concept means the subsumption holds.
    pub fn decide_subsumption(&self, sub: &Concept, sup: &Concept) -> Verdict {
        let mut test = vec![sub.clone(), sup.negate()];
        test.sort();
        self.decide_satisfiable(&Concept::And(test))
    }

    /// Check TBox consistency. An undecided run reports consistent, which is
    /// the safe direction: we do not condemn an ontology we failed to refute.
    pub fn is_consistent(&self) -> bool {
        !self.decide_satisfiable(&Concept::Top).is_unsat()
    }

    /// Explain why a class is unsatisfiable. Returns None if satisfiable.
    pub fn explain_unsatisfiable(&self, class_id: u32) -> Option<Vec<String>> {
        let concept = Concept::Atom(class_id);
        let mut tableau = Tableau::new_with_tracing(Arc::clone(&self.tbox));
        if !tableau.decide(&concept).is_unsat() {
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
        let verdict = tableau.decide(&test_concept);
        (verdict.is_unsat(), tableau.trace.steps)
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

        // Sorted for the same reason as the node traversals: `named_classes` is a
        // HashSet, so its iteration order is seeded per process. That order fixes
        // the order of `satisfiable`, which fixes the order of `pairs`, which
        // decides which subsumption checks are reached before a budget runs out.
        // Unsorted, classification is not a function of the ontology alone.
        let mut classes: Vec<u32> = self
            .named_classes
            .iter()
            .filter(|&&c| c != self.thing_id && c != self.nothing_id)
            .copied()
            .collect();
        classes.sort_unstable();

        // Global budget for the WHOLE classification.
        //
        // The per-test deadline bounds one satisfiability check. It does not
        // bound classification, which runs one check per class plus one per
        // ordered pair of classes. On the Pizza ontology that is ~100 classes
        // and ~10,000 pairs; at the 10s per-test budget the worst case is over
        // a day. A global deadline is what actually stops that.
        let global_deadline = crate::runtime::classify_timeout_ms()
            .map(|ms| Instant::now() + std::time::Duration::from_millis(ms));
        let out_of_time = || global_deadline.is_some_and(|d| Instant::now() >= d);

        // ── Satisfiability Agent ─────────────────────────────────────
        let sat_start = Instant::now();
        let sat_results: Vec<(u32, Verdict)> = classes
            .par_iter()
            .map(|&cls| {
                if out_of_time() {
                    // Budget gone: report Unknown rather than guessing.
                    return (cls, Verdict::Unknown);
                }
                (cls, self.decide_satisfiable(&Concept::Atom(cls)))
            })
            .collect();

        // Only PROVEN unsatisfiable classes go in the unsatisfiable list.
        // Classes we ran out of budget on are undetermined, and are kept in
        // the satisfiable set for downstream subsumption testing so they stay
        // in the hierarchy rather than silently vanishing.
        let satisfiable: Vec<u32> = sat_results
            .iter()
            .filter(|(_, v)| !v.is_unsat())
            .map(|(c, _)| *c)
            .collect();
        // Reported separately from `satisfiable`. That vector deliberately carries
        // the undetermined classes so they stay in the hierarchy for the
        // subsumption sweep, but counting them as "satisfiable_found" in the
        // output claims a proof that was never constructed. On a 234-class
        // ontology where every class is undetermined, the old field read
        // "satisfiable_found: 234", which is indistinguishable from a coherent
        // result.
        let proven_satisfiable = sat_results
            .iter()
            .filter(|(_, v)| matches!(v, Verdict::Satisfiable))
            .count();
        let unsatisfiable: Vec<u32> = sat_results
            .iter()
            .filter(|(_, v)| v.is_unsat())
            .map(|(c, _)| *c)
            .collect();
        let undetermined: Vec<u32> = sat_results
            .iter()
            .filter(|(_, v)| matches!(v, Verdict::Unknown))
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
                if sub != sup && !told.get(&sub).is_some_and(|s| s.contains(&sup)) {
                    pairs.push((sub, sup));
                }
            }
        }

        let subsumption_cut_short = std::sync::atomic::AtomicBool::new(false);
        let inferred: Vec<(u32, u32)> = pairs
            .par_iter()
            .filter(|(sub, sup)| {
                if out_of_time() {
                    // Untested pairs are NOT non-subsumptions. Flag the run as
                    // incomplete so no caller reads absence as refutation.
                    subsumption_cut_short.store(true, std::sync::atomic::Ordering::Relaxed);
                    return false;
                }
                self.is_subsumed(&Concept::Atom(*sub), &Concept::Atom(*sup))
            })
            .cloned()
            .collect();
        let subsumption_cut_short =
            subsumption_cut_short.load(std::sync::atomic::Ordering::Relaxed);
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
                if a < b
                    && hierarchy.get(&b).is_some_and(|bs| bs.contains(&a)) {
                        equivalences.push((a, b));
                    }
            }
        }

        let total_time = start.elapsed();

        AgentClassificationResult {
            hierarchy,
            unsatisfiable,
            undetermined,
            subsumption_cut_short,
            equivalences,
            inferred_subsumptions: inferred.len(),
            agents: AgentMetrics {
                satisfiability: AgentTaskMetrics {
                    tasks: classes.len(),
                    results: proven_satisfiable,
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
    /// Realize individuals into classes that are DEFINED by an owl:equivalentClass axiom.
    ///
    /// `check_abox` reads concept labels off one completed tableau. That is sound for
    /// detecting inconsistency but it is not realization: a concept satisfied in the single
    /// model the expansion happened to find is not thereby entailed, and the branch that
    /// would have added a defined class is usually not the branch recorded. The practical
    /// consequence was that a class defined by owl:equivalentClass stayed permanently empty,
    /// so the whole idiom of defining a class and letting the reasoner find its members did
    /// not work at all.
    ///
    /// Full realization would test, for every individual and every class, whether asserting
    /// the negated class makes the ABox inconsistent. That is one tableau expansion over the
    /// entire ABox per pair, which is not affordable: a few thousand individuals against a
    /// few dozen classes is tens of thousands of full expansions.
    ///
    /// This instead matches definitions structurally against asserted facts, which is linear
    /// in individuals and is complete for the fragment it accepts: a conjunction whose
    /// conjuncts are named classes and existential restrictions whose filler is a named class
    /// or a nominal. That covers `owl:hasValue` and `owl:someValuesFrom` inside an
    /// `owl:intersectionOf`, which is how defined classes are written in practice. Anything
    /// outside the fragment is declined rather than guessed at, so a definition using
    /// negation, cardinality or a nested anonymous class yields no members here rather than
    /// wrong ones. Cardinality-based definitions in particular are closed-world questions and
    /// belong in SHACL.
    fn realize_definitions(&self) -> HashMap<u32, HashSet<u32>> {
        let mut out: HashMap<u32, HashSet<u32>> = HashMap::new();
        if std::env::var("OO_DEBUG_REALIZE").is_ok() {
            eprintln!("[realize] definitions={} individuals={} roles={}",
                self.definitions.len(), self.individual_types.len(), self.role_assertions.len());
            for (c, d) in &self.definitions {
                eprintln!("[realize]   {} := {:?}", self.interner.resolve(*c), d);
            }
        }
        if self.definitions.is_empty() {
            return out;
        }

        // role assertions indexed by subject for a linear scan per individual
        let mut by_subject: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
        for &(a, r, b) in &self.role_assertions {
            by_subject.entry(a).or_default().push((r, b));
        }
        for &(a, r, b) in &self.data_assertions {
            by_subject.entry(a).or_default().push((r, b));
        }

        for (&ind, types) in &self.individual_types {
            for (&cls, def) in &self.definitions {
                if types.contains(&cls) {
                    continue;
                }
                if self.satisfies(ind, types, def, &by_subject) {
                    out.entry(ind).or_default().insert(cls);
                }
            }
        }
        out
    }

    /// True when the asserted facts about `ind` entail `concept`, for the accepted fragment.
    /// Returns false for anything outside it, which keeps unsupported definitions empty
    /// rather than unsound.
    fn satisfies(
        &self,
        ind: u32,
        types: &HashSet<u32>,
        concept: &Concept,
        by_subject: &HashMap<u32, Vec<(u32, u32)>>,
    ) -> bool {
        match concept {
            Concept::Top => true,
            Concept::Atom(c) => self.has_type(types, *c),
            Concept::And(parts) => parts.iter().all(|p| self.satisfies(ind, types, p, by_subject)),
            Concept::Exists(role, filler) => {
                let Concept::Atom(target) = **filler else {
                    return false;
                };
                by_subject.get(&ind).is_some_and(|edges| {
                    edges.iter().any(|&(r, o)| {
                        if !self.role_matches(r, *role) {
                            return false;
                        }
                        // nominal: the filler names the object itself
                        // class filler: the object is typed by it
                        o == target
                            || self
                                .individual_types
                                .get(&o)
                                .is_some_and(|ot| self.has_type(ot, target))
                    })
                })
            }
            _ => false,
        }
    }

    fn has_type(&self, types: &HashSet<u32>, target: u32) -> bool {
        types.contains(&target)
            || types.iter().any(|t| {
                self.subclass_closure
                    .get(t)
                    .is_some_and(|sups| sups.contains(&target))
            })
    }

    fn role_matches(&self, asserted: u32, required: u32) -> bool {
        asserted == required
            || self
                .tbox
                .super_to_sub
                .get(&required)
                .is_some_and(|subs| subs.contains(&asserted))
    }

    pub fn check_abox(&self) -> ABoxResult {
        if self.individual_types.is_empty() && self.individual_anon_types.is_empty() {
            return ABoxResult {
                consistent: true,
                undecided: false,
                individuals_checked: 0,
                inferred_types: HashMap::new(),
            };
        }

        // The ABox check builds ONE tableau containing every named individual,
        // so it is the largest single expansion the reasoner ever performs. It
        // must carry the same deadline as every other test, otherwise it is an
        // unbounded hole in the budget.
        let mut tableau = Tableau::with_deadline(Arc::clone(&self.tbox), self.deadline);
        let mut ind_to_node: HashMap<u32, u32> = HashMap::new();

        // Create nodes for each individual carrying a named-class OR an anonymous
        // class type. An individual typed only by an anonymous class expression
        // (`:a rdf:type [ owl:Restriction … ]`) has no entry in individual_types,
        // and dropping it here is how a restriction-borne inconsistency was missed.
        let mut all_individuals: Vec<u32> = self.individual_types.keys().copied().collect();
        for &ind in self.individual_anon_types.keys() {
            if !self.individual_types.contains_key(&ind) {
                all_individuals.push(ind);
            }
        }
        all_individuals.sort_unstable();
        let individuals_checked = all_individuals.len();
        for ind in all_individuals {
            let node_id = tableau.fresh_node(None, None);
            ind_to_node.insert(ind, node_id);
            if let Some(types) = self.individual_types.get(&ind) {
                for &cls in types {
                    tableau.add_label(node_id, Concept::Atom(cls));
                }
            }
            // Anonymous class expressions the individual is directly typed by.
            if let Some(anon) = self.individual_anon_types.get(&ind) {
                for concept in anon {
                    tableau.add_label(node_id, concept.clone());
                }
            }
            // The nominal {a} is approximated as the atomic concept named by a's own IRI
            // (see RawConcept::Named). That approximation only works if a's node actually
            // carries that atom: without it an owl:hasValue restriction can never be
            // satisfied, so every class DEFINED by one stays empty and no individual is
            // ever realized into it.
            tableau.add_label(node_id, Concept::Atom(ind));
            // Add GCIs
            for gci in tableau.tbox.gcis.clone() {
                tableau.add_label(node_id, gci);
            }
        }

        // An individual may be named only as the object of a role assertion, typed by a
        // vocabulary this reasoner does not treat as a class (a skos:Concept, say) and never
        // declared owl:NamedIndividual. It still has to exist as a node, or the edge that
        // points at it leads nowhere and the restriction that depends on it silently fails.
        let referenced: Vec<u32> = self
            .role_assertions
            .iter()
            .map(|&(_, _, b)| b)
            .filter(|b| !ind_to_node.contains_key(b))
            .collect();
        for b in referenced {
            if ind_to_node.contains_key(&b) {
                continue;
            }
            let node_id = tableau.fresh_node(None, None);
            ind_to_node.insert(b, node_id);
            tableau.add_label(node_id, Concept::Atom(b));
        }

        // Add role assertions as edges. An asserted edge a R b also means b has an
        // R-inverse edge to a: without it, a ForAll or cardinality constraint on b
        // never propagates backward across the edge, and an inconsistency reachable
        // only through the inverse (or, since a symmetric role is its own inverse in
        // `inverse_roles`, a symmetric) role is silently missed. Unlike the tree
        // tableau, ABox individuals form an arbitrary graph, so the parent back-link
        // successors() uses cannot stand in for the inverse neighbour. Materialise it.
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
                if let Some(&r_inv) = tableau.tbox.inverse_roles.get(&r) {
                    tableau
                        .nodes
                        .get_mut(&b_node)
                        .unwrap()
                        .edges
                        .entry(r_inv)
                        .or_default()
                        .insert(a_node);
                }
            }
        }

        // Same three-valued discipline as everywhere else: exhausting the
        // budget is not a proof of inconsistency. Declaring an ABox
        // inconsistent is a strong, user-visible claim and must never be the
        // by-product of giving up. Default to consistent when undecided.
        let expanded = tableau.expand(0);
        let abox_undecided = !expanded && tableau.budget.exhausted;
        let consistent = expanded || abox_undecided;

        // Infer additional types for each individual. Only when the expansion
        // genuinely completed: a partial tableau's labels are not conclusions.
        let mut inferred: HashMap<u32, HashSet<u32>> = HashMap::new();
        if expanded {
            for (&ind, &node_id) in &ind_to_node {
                if let Some(node) = tableau.nodes.get(&node_id) {
                    for label in &node.labels {
                        if let Concept::Atom(cls) = label
                            && *cls != ind
                            && !self.individual_types.get(&ind).is_some_and(|t| t.contains(cls)) {
                                inferred.entry(ind).or_default().insert(*cls);
                            }
                    }
                }
            }
        }

        // Structural realization runs regardless of which model the expansion found, and is
        // what actually populates classes defined by owl:equivalentClass.
        for (ind, classes) in self.realize_definitions() {
            inferred.entry(ind).or_default().extend(classes);
        }

        ABoxResult {
            consistent,
            undecided: abox_undecided,
            individuals_checked,
            inferred_types: inferred,
        }
    }

    /// Entry point for integration with the existing reasoner.
    pub fn run(graph: &Arc<GraphStore>, materialize: bool) -> anyhow::Result<String> {
        let reasoner = Self::from_graph(graph)?;
        let initial_triples = graph.triple_count();

        let tbox_consistent = reasoner.is_consistent();
        let result = reasoner.classify_parallel();
        let abox_result = reasoner.check_abox();

        // The headline flag answers for the whole knowledge base. Reporting the TBox alone
        // while the ABox check has PROVEN an inconsistency in the same output is a false
        // claim; the three-valued discipline is preserved because an undecided ABox already
        // defaults to consistent inside check_abox.
        let consistent = tbox_consistent && abox_result.consistent;

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

        // Completeness signal. A caller must be able to tell a proof from a
        // run that gave up; without this the two are indistinguishable in the
        // output and every consumer silently over-trusts the result.
        let undetermined_names: Vec<&str> = result
            .undetermined
            .iter()
            .map(|&id| reasoner.interner.resolve(id))
            .collect();
        let complete = undetermined_names.is_empty()
            && !result.subsumption_cut_short
            && !abox_result.undecided;

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
                "undecided": abox_result.undecided,
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
            "tbox_consistent": tbox_consistent,
            "named_classes": reasoner.named_classes.len(),
            "unsatisfiable_classes": unsat_names,
            "complete": complete,
            "undetermined_classes": undetermined_names,
            "subsumption_sweep_cut_short": result.subsumption_cut_short,
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
                    "undetermined": result.undetermined.len(),
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
    /// Classes PROVEN to have no instances.
    pub unsatisfiable: Vec<u32>,
    /// Classes on which a resource budget was hit before a decision. Neither
    /// proven satisfiable nor proven unsatisfiable. Their presence means the
    /// classification as a whole is incomplete.
    pub undetermined: Vec<u32>,
    /// True when the subsumption sweep ran out of global budget, so some pairs
    /// were never tested. Absence of a subsumption in the output is then not
    /// evidence that it does not hold.
    pub subsumption_cut_short: bool,
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
    /// Reported consistent unless inconsistency was PROVEN. See `undecided`.
    pub consistent: bool,
    /// True when the ABox expansion ran out of budget. `consistent` is then a
    /// default, not a finding.
    pub undecided: bool,
    pub individuals_checked: usize,
    pub inferred_types: HashMap<u32, HashSet<u32>>,
}

pub struct ClassificationResult {
    pub hierarchy: HashMap<u32, HashSet<u32>>,
    pub unsatisfiable: Vec<u32>,
    pub equivalences: Vec<(u32, u32)>,
    pub inferred_subsumptions: usize,
}
