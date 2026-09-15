//! SHIQ Tableaux Reasoner with Agent-Based Classification
//!
//! A native Rust implementation of a tableaux decision procedure for
//! the SHIQ Description Logic: ALC extended with transitive roles, role
//! hierarchies, inverse roles and qualified number restrictions. Nominals are
//! not implemented, so this is a strict fragment of OWL 2 DL. `owl:oneOf` is
//! not parsed, `owl:hasValue` is approximated as an atomic concept named after
//! the individual, and datatype ranges are skipped.
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
//! | InvFun | InverseFunctionalProperty  | ✅     |
//! | ABox   | NamedIndividual            | ✅     |
//! |        | AsymmetricProperty         | ❌     |
//! |        | ReflexiveProperty          | ❌     |
//! |        | IrreflexiveProperty        | ❌     |
//!
//! The ❌ rows are constraints on an EDGE or a pair of edges rather than on a
//! node's concept membership, and SHIQ without nominals has no label that says
//! them. They are reported by `DlReasoner::unmodelled_constructs`, which is how
//! a reader learns that an answer did not take them into account. That reporting
//! is the whole point: `owl:AsymmetricProperty` used to be in neither column, so
//! an ontology using it got a clean verdict and no warning.
//!
//! A ✅ in this table is a claim about behaviour and has been wrong. `Fun` and
//! `InvFun` carried one for a long time while the rule that enforces them could
//! not fire: functionality is encoded as `≤1 R.⊤`, `add_label` refuses to store
//! `⊤`, and the ≤-rule counted successors by testing whether the label set
//! contains the filler, which for `⊤` is false on every node. Both now go
//! through the `MaxCard` path with `Top` special-cased at the counting step, and
//! `tests/tableaux_role_characteristics_test.rs` pins the negative — an
//! inconsistent ontology that used to come back consistent — rather than only a
//! positive.
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
/// All negations pushed to atomic level. Covers SHIQ. There is no nominal
/// constructor in this enum, so `owl:oneOf` has no representation and
/// `owl:hasValue` is carried as an atomic concept named after the individual.
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

/// A by-subject view of a graph, with the one RDF list reader this crate has.
///
/// `pub(crate)` rather than private because `rulesyntax.rs` reads SWRL atom
/// lists, which are `rdf:List`s exactly as `owl:intersectionOf`'s operand list
/// is. A second traversal of `rdf:first`/`rdf:rest` would be a second thing to
/// keep right; there is one, and `walk_list_checked` is it.
pub(crate) struct TripleIndex {
    by_subject: HashMap<String, Vec<(String, String)>>,
}

/// Why a traversal of an `rdf:List` did not reach `rdf:nil`.
///
/// `walk_list` drops this, which is right for the DL parser: a malformed
/// `owl:intersectionOf` list yields a shorter conjunction, the reasoner derives
/// less, and deriving less is the sound direction. It is NOT right for a rule
/// importer, where a truncated body is a DIFFERENT RULE that fires more often
/// than the one the user wrote. So the defect is returned and the caller
/// decides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ListDefect {
    /// A cell with no `rdf:first`. Its position in the list is lost.
    NoFirst(String),
    /// A cell with no `rdf:rest`, so the list never reaches `rdf:nil`.
    Unterminated(String),
    /// Still walking after `MAX_LIST_CELLS` cells: a cycle, or a list longer
    /// than anything this reads.
    TooLong,
}

impl ListDefect {
    pub(crate) fn describe(&self) -> String {
        match self {
            ListDefect::NoFirst(cell) => {
                format!("the list cell {cell} has no rdf:first, so an element is missing")
            }
            ListDefect::Unterminated(cell) => format!(
                "the list cell {cell} has no rdf:rest, so the list never reaches rdf:nil and its \
                 remaining elements are unknown"
            ),
            ListDefect::TooLong => format!(
                "the list did not reach rdf:nil within {MAX_LIST_CELLS} cells, which means a \
                 cycle through rdf:rest or a list longer than this reads"
            ),
        }
    }
}

/// The cell budget both list readers share.
pub(crate) const MAX_LIST_CELLS: usize = 1000;

impl TripleIndex {
    pub(crate) fn new(triples: &[(String, String, String)]) -> Self {
        let mut by_subject: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for (s, p, o) in triples {
            by_subject
                .entry(s.clone())
                .or_default()
                .push((p.clone(), o.clone()));
        }
        Self { by_subject }
    }

    pub(crate) fn objects(&self, subject: &str, predicate: &str) -> Vec<String> {
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

    pub(crate) fn object(&self, subject: &str, predicate: &str) -> Option<String> {
        self.objects(subject, predicate).into_iter().next()
    }

    /// Walk an `rdf:List`, reporting the FIRST thing that stopped it reaching
    /// `rdf:nil` cleanly. The items are gathered exactly as `walk_list` has
    /// always gathered them, so the two readings differ only in what they SAY,
    /// never in what they collect.
    pub(crate) fn walk_list_checked(&self, head: &str) -> (Vec<String>, Option<ListDefect>) {
        let mut items = Vec::new();
        let mut defect: Option<ListDefect> = None;
        let mut current = head.to_string();
        for _ in 0..MAX_LIST_CELLS {
            if current == RDF_NIL {
                return (items, defect);
            }
            match self.object(&current, RDF_FIRST) {
                Some(first) => items.push(first),
                // Skipped, not fatal: that is what this reader has always done.
                None => {
                    defect.get_or_insert_with(|| ListDefect::NoFirst(current.clone()));
                }
            }
            match self.object(&current, RDF_REST) {
                Some(rest) => current = rest,
                None => {
                    defect.get_or_insert(ListDefect::Unterminated(current));
                    return (items, defect);
                }
            }
        }
        (items, Some(defect.unwrap_or(ListDefect::TooLong)))
    }

    /// The DL parser's reading: take what there is and say nothing about a
    /// malformed tail. A short `owl:intersectionOf` list yields a smaller
    /// conjunction and the reasoner derives less, which is the sound direction
    /// there. Behaviour is unchanged.
    fn walk_list(&self, head: &str) -> Vec<String> {
        self.walk_list_checked(head).0
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

        // Close the role hierarchy under inverses: `r ⊑ s` entails `r⁻ ⊑ s⁻`.
        //
        // In every model, `(x,y) ∈ r` implies `(x,y) ∈ s`, and `(y,x) ∈ r⁻` iff
        // `(x,y) ∈ r`, so `(y,x) ∈ r⁻` implies `(y,x) ∈ s⁻`. That is the whole
        // proof; the entailment is not subtle, it was simply never computed.
        //
        // What it was costing: each role's `rdfs:domain` and `rdfs:range` lists
        // are folded over its transitive SUPER-roles once, in `ProcessedTBox::new`,
        // and consulted per edge. A domain stated directly on `s` therefore already
        // reached an `r` edge. A domain stated on `s⁻` did not, because `r⁻` was
        // not known to be a sub-role of `s⁻`, so `s⁻`'s domain never folded into
        // `r⁻`'s list and the constraint never reached the two individuals the
        // edge actually binds. The same gap closed `successors()` off from
        // sub-role edges reached through an inverse.
        //
        // ONE pass suffices, and that is not an optimisation, it is the closure.
        // `inverse_roles` holds both directions of every `owl:inverseOf` pair and
        // maps a symmetric role to itself, so on a well-formed ontology it is an
        // involution: for any pair `(inv a, inv b)` this loop adds, applying the
        // rule again yields `(inv inv a, inv inv b) = (a, b)`, already present.
        // The result is closed after a single sweep, with no fixpoint to iterate
        // and no chance of divergence.
        //
        // The map holds ONE inverse per role, so an ontology declaring two
        // different inverses for the same property keeps only the last and the
        // involution does not hold there. That under-closes: some entailed
        // `r⁻ ⊑ s⁻` is missed. It never OVER-closes, because every pair added
        // here is entailed by the pair it came from whatever else the map says,
        // so the failure mode is a missed constraint and never a fabricated one.
        // `onto_defects` reports that ontology shape as `inverse_not_mutual`.
        //
        // Transitivity is deliberately left to the existing downstream closure:
        // it runs over this enlarged DIRECT relation, so `r ⊑ s ⊑ t` still yields
        // `r⁻ ⊑ t⁻` through `s⁻`.
        let inverse_subprops: Vec<(u32, u32)> = sub_to_super
            .iter()
            .flat_map(|(sub, sups)| sups.iter().map(move |sup| (*sub, *sup)))
            .filter_map(|(sub, sup)| {
                match (inverse_roles.get(&sub), inverse_roles.get(&sup)) {
                    (Some(&sub_inv), Some(&sup_inv)) if sub_inv != sup_inv => {
                        Some((sub_inv, sup_inv))
                    }
                    _ => None,
                }
            })
            .collect();
        for (sub_inv, sup_inv) in inverse_subprops {
            sub_to_super.entry(sub_inv).or_default().insert(sup_inv);
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

        // A subject that makes role assertions is an individual whether or not
        // anybody typed it.
        //
        // The loop above walks `subject_types`, which only has an entry for a
        // subject carrying an `rdf:type` at all, and then keeps it only if that
        // type is recognisable as a class. So an IRI that is never typed, or is
        // typed only by a vocabulary this reasoner does not treat as a class (a
        // `skos:Concept`, say), contributed NOTHING: not its edges, not itself.
        // Now that `add_role_edge` applies `rdfs:domain` and `rdfs:range` to
        // asserted edges, that is a false clean with a very short witness — an
        // individual with two role assertions whose domains are disjoint classes
        // is inconsistent on those two triples alone, and the check never saw
        // them. The object side of exactly this hole was already closed, in
        // `build_abox_tableau`, which builds a node for any individual named as
        // the OBJECT of a role assertion. This is the subject side.
        //
        // The filter is deliberately narrow, because the cost of being wrong
        // here is sweeping schema into the ABox:
        //   - IRIs only, matching the rule used for class discovery. A blank node
        //     subject is a parsed class expression, not an individual.
        //   - Not already an individual, and not a declared class or property, so
        //     `:p a owl:ObjectProperty` can never double as an instance.
        //   - At least one predicate that is a DECLARED object property. That is
        //     the only reason this individual is wanted: it is the thing that
        //     produces an edge. A subject carrying nothing but `rdfs:label` or a
        //     `rdfs:subClassOf` is schema and stays out.
        // `object_properties` is complete by this point: it is filled from the
        // explicit typing, from the four characteristic types and from both sides
        // of every `owl:inverseOf` pair, all above.
        let already: HashSet<&str> = individual_subjects.iter().map(|s| s.as_str()).collect();
        let mut untyped_role_subjects: Vec<String> = Vec::new();
        for (subject, pairs) in &self.index.by_subject {
            if !subject.starts_with('<') || already.contains(subject.as_str()) {
                continue;
            }
            let id = self.interner.intern(subject);
            if named_classes.contains(&id) || object_properties.contains(&id) {
                continue;
            }
            if subject_types
                .get(subject)
                .is_some_and(|ts| ts.iter().any(|t| schema_markers.contains(&t.as_str())))
            {
                continue;
            }
            let asserts_a_role = pairs.iter().any(|(p, _)| {
                p != RDF_TYPE && object_properties.contains(&self.interner.intern(p))
            });
            if asserts_a_role {
                untyped_role_subjects.push(subject.clone());
            }
        }
        individual_subjects.extend(untyped_role_subjects);
        individual_subjects.sort_unstable();
        individual_subjects.dedup();
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
    /// Keep the completion graph of the branch that succeeded.
    ///
    /// The ⊔-rule explores a disjunct in a CLONE and returns `true` from the
    /// clone's expansion without copying it back, so after a successful run
    /// `self.nodes` is the state as it was BEFORE the last disjunction, not the
    /// model that was found. Every existing caller reads only the boolean, so
    /// this stays off by default and nothing changes; the model emitter turns it
    /// on because it has to hand over the graph, not the verdict.
    capture: bool,
}

impl Tableau {
    fn new(tbox: Arc<ProcessedTBox>) -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: 0,
            tbox,
            trace: ExplanationTrace::new(false),
            budget: Budget::default(),
            capture: false,
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
            capture: false,
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

    /// Does the node `s` satisfy the filler concept `filler`?
    ///
    /// This is the test every role-counting rule needs, and it is one function
    /// because it was three copies of `labels.contains(&filler)` and all three
    /// were wrong in the same way.
    ///
    /// `⊤` is satisfied by EVERY element of the domain. `add_label` refuses to
    /// store `Concept::Top` — correctly, since there is nothing to propagate from
    /// it and carrying it would bloat every label set in the tableau — so
    /// `labels.contains(&Top)` is false on every node that will ever exist. The
    /// ≤-rule, the ≥-rule and the ∃-rule all counted successors with a bare
    /// `contains`, so on a `⊤` filler the ≤-rule counted ZERO matching successors
    /// and its bound could not be violated, while the ≥- and ∃-rules also counted
    /// zero and manufactured successors they already had.
    ///
    /// `owl:FunctionalProperty` is encoded as the GCI `≤1 R.⊤` and
    /// `owl:InverseFunctionalProperty` as `≤1 R⁻.⊤`, so BOTH were completely
    /// inert: no merge could ever fire, and an ABox with two distinct fillers of
    /// a functional property in disjoint classes came back `consistent: true,
    /// undecided: false`. The module documentation claimed both as supported.
    /// `has_clash` already special-cased `⊤` when comparing two cardinality
    /// LABELS on one node; the successor-counting sites were simply missed.
    fn node_satisfies(&self, s: u32, filler: &Concept) -> bool {
        match self.nodes.get(&s) {
            Some(n) => *filler == Concept::Top || n.labels.contains(filler),
            None => false,
        }
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

    /// Add a role edge, and apply everything the edge itself entails.
    ///
    /// `rdfs:domain` binds the SOURCE and `rdfs:range` binds the TARGET. The
    /// inverse edge `to inv(role) from` holds in every model in which `from role
    /// to` does, so the inverse role's domain and range bind the same two nodes
    /// the other way round. A symmetric role is its own inverse in
    /// `inverse_roles`, so that clause covers it too.
    ///
    /// This is the ONLY sanctioned way to add an edge, and that is the whole
    /// point of it existing. `create_successor` used to be the only place
    /// `role_domain` and `role_range` were consulted, while `build_abox_tableau`
    /// wrote asserted role assertions straight into the edge map. A GENERATED
    /// successor therefore carried its domain and its range and an ASSERTED edge
    /// carried neither, so an ABox that is inconsistent precisely because an
    /// asserted edge forces its subject into a class disjoint from one it
    /// already has came back `consistent: true, undecided: false` — the
    /// strongest answer this checker can give, and wrong. The defect was the
    /// split between the two edge-creating paths, not either path on its own, so
    /// the repair is a single primitive both of them go through.
    ///
    /// Termination is unaffected. This adds labels to nodes that already exist
    /// and creates none, and the labels are drawn from the same finite
    /// sub-concept closure every other rule draws on, so no node's label set can
    /// grow without bound and blocking still fires on the same condition.
    fn add_role_edge(&mut self, from: u32, role: u32, to: u32) {
        if !self.nodes.contains_key(&from) || !self.nodes.contains_key(&to) {
            return;
        }
        self.nodes
            .get_mut(&from)
            .unwrap()
            .edges
            .entry(role)
            .or_default()
            .insert(to);

        // Keeping domain and range out of the GCI list is deliberate: as GCIs
        // they are `∃p.⊤ ⊑ D` and `⊤ ⊑ ∀p.R`, both with a non-atomic left-hand
        // side, so both land a disjunction on every node in the tableau. Applied
        // here they cost one label each on the two nodes actually involved.
        if let Some(ds) = self.tbox.role_domain.get(&role).cloned() {
            for c in ds {
                self.add_label(from, c);
            }
        }
        if let Some(rs) = self.tbox.role_range.get(&role).cloned() {
            for c in rs {
                self.add_label(to, c);
            }
        }

        if let Some(&inv) = self.tbox.inverse_roles.get(&role) {
            if let Some(ds) = self.tbox.role_domain.get(&inv).cloned() {
                for c in ds {
                    self.add_label(to, c);
                }
            }
            if let Some(rs) = self.tbox.role_range.get(&inv).cloned() {
                for c in rs {
                    self.add_label(from, c);
                }
            }
        }
    }

    /// Create a new successor node and set up edges (including inverse back-edges).
    fn create_successor(&mut self, parent_id: u32, role: u32, filler: Concept) -> u32 {
        let succ = self.fresh_node(Some(parent_id), Some(role));
        // The edge goes in through the shared primitive, which is what applies
        // rdfs:domain to this node and rdfs:range to the new one.
        self.add_role_edge(parent_id, role, succ);
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
                if let Concept::ForAll(r, f) = label
                    && *r == sup_role {
                        self.add_label(succ, *f.clone());
                    }
            }
        }

        // The forward edge was added by `add_role_edge` above, together with the
        // domain and range it entails.

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
                                succs.iter().any(|&s| self.node_satisfies(s, &filler));
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
                                .filter(|&&s| self.node_satisfies(s, &filler))
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
                    .filter(|&&s| self.node_satisfies(s, &filler))
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

                    // The bound is NOT marked processed here, and that is the
                    // second half of making functional properties work.
                    //
                    // This used to insert the `MaxCard` label into `nid.processed`
                    // before cloning, so every branch below inherited the mark and
                    // the rule could fire at most ONCE per node. One firing is one
                    // merge, which removes one successor. With three successors
                    // under `≤1` that leaves two, the bound is still violated, and
                    // the rule is already spent — so the branch completes and is
                    // returned as a MODEL of a constraint it visibly breaks. While
                    // `≤1 R.⊤` could never fire at all that was unreachable; the
                    // moment functionality started firing, three fillers of a
                    // functional property became an ordinary input.
                    //
                    // ── TERMINATION ──
                    //
                    // The measure that decreases is the recursion depth remaining,
                    // and it decreases by one on every merge with nothing that can
                    // give it back. Each firing of this rule performs exactly one
                    // `merge_nodes` and then recurses as `expand(depth + 1)`;
                    // `expand` returns `false` on entry once `depth >
                    // tableaux_max_depth`. So a chain of merges cannot be infinite,
                    // whatever the merges do to the graph. That is the guarantee,
                    // and it does not depend on the argument below being right.
                    //
                    // The argument below is why the bound is not reached in
                    // practice. `merge_nodes` removes a node, so each merge
                    // strictly decreases `self.nodes.len()`; the only rule that
                    // increases it is `create_successor`, driven by the ∃- and
                    // ≥-rules. Those cannot form a yo-yo with this one, because
                    // `merge_nodes` UNIONS the two label sets into the survivor: if
                    // `∃R.C` and `∃R.D` forced the two successors that were just
                    // identified, the survivor carries both `C` and `D`, so the
                    // ∃-rule's `has_matching` test is satisfied for both and it
                    // creates nothing. A new successor appears only for an
                    // existential that is genuinely unsatisfied, and each
                    // existential marks itself processed on its node, so it fires
                    // at most once there.
                    //
                    // ── WHY BLOCKING STILL FIRES ──
                    //
                    // `update_blocking()` runs at the end of every pass of the
                    // deterministic fixpoint loop, and the recursive
                    // `expand(depth + 1)` on the merged branch re-enters that loop
                    // from the top. So blocking is recomputed against the
                    // POST-MERGE graph before this rule looks at the graph again,
                    // and the `blocked` test at the head of this loop sees the
                    // fresh answer. The condition is pairwise ancestor blocking
                    // (Horrocks and Sattler, JAR 39), which is the variant that
                    // stays sound with inverse roles and number restrictions
                    // together — which is exactly the combination merging puts in
                    // play, since an inverse-functional bound is a bound on `R⁻`.
                    // Merging changes node labels and edges, which is precisely
                    // what that condition is computed from, so recomputing it after
                    // every merge is not an optimisation, it is required: a node
                    // blocked before a merge may not be blocked after it.
                    //
                    // ── AND IF THE ARGUMENT IS WRONG ANYWAY ──
                    //
                    // Depth, node count and a wall clock are all checked, the clock
                    // inside the inner fixpoint loop. Exhausting any of them sets
                    // `budget.exhausted`, which callers turn into `Verdict::Unknown`
                    // and `undecided: true`. The degraded answer is "I did not
                    // finish", never a verdict.
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
                            // Only the model emitter asks for the graph back. See
                            // `Tableau::capture`.
                            if self.capture {
                                *self = branch;
                            }
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

    /// Applies pairwise ancestor blocking to every node. See
    /// `is_pairwise_blocked` for the condition.
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
    /// SHIQ this reasoner implements.
    ///
    /// This is deliberately the classical ancestor variant rather than HermiT's
    /// "anywhere" blocking. Anywhere blocking yields smaller models but is a
    /// further optimisation on top of a correct base; get the base right first.
    fn is_pairwise_blocked(&self, s: u32) -> bool {
        self.blocker_of(s).is_some()
    }

    /// The ancestor that blocks `s`, under the condition `is_pairwise_blocked`
    /// documents. Split out so that the model emitter can fold a blocked node
    /// into the node that blocks it, which is the only way a completion graph
    /// with blocking becomes a finite interpretation. One condition, one place.
    fn blocker_of(&self, s: u32) -> Option<u32> {
        // Only blockable successors can be blocked. A node with no parent is
        // a root and is never blocked.
        let s_parent = self.nodes.get(&s).and_then(|n| n.parent)?;
        let s_node = self.nodes.get(&s)?;
        let s_parent_node = self.nodes.get(&s_parent)?;

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
                    return Some(t);
                }
            }
            cur = t_node.parent;
        }
        None
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
    /// How LONG a phase of the run may take, in milliseconds. `None` means no
    /// time limit (the node/depth budgets still apply).
    ///
    /// A DURATION, not an instant, and the difference was a defect. This used to
    /// hold one `Instant` computed when the reasoner was CONSTRUCTED, shared by
    /// the satisfiability sweep, the subsumption sweep and the ABox check. One
    /// shared instant is not a per-test budget and it is not a per-phase budget:
    /// it is a budget for the whole run, handed out oldest-first. A
    /// classification that used the clock up left the ABox check starting already
    /// expired, and it reported `undecided` on an ABox it decides in
    /// microseconds. Honest, but a loaded machine silently degraded an answer
    /// that was there for the taking, and the output gave a reader no way to see
    /// that classification was what spent it.
    ///
    /// Each phase now opens its own from this duration, and no cap was raised:
    /// the number is unchanged and the node and depth caps are untouched. Which
    /// phase ran out is reported in `budget_exhausted_in`. See `phase_deadline`
    /// for why the unit is a phase rather than a single test, which is what the
    /// setting's own name says, and `phase_deadline_within` for how
    /// `classify_timeout_ms` is intersected into it so that no phase can outlive
    /// the run's ceiling.
    budget_ms: Option<u64>,
    /// The axioms as the OWL parser read them, before `ProcessedTBox` absorbed,
    /// rewrote and folded them. A model certificate is written against these, so
    /// that what the checker verifies is the ontology's own axioms rather than
    /// the reasoner's internal encoding of them.
    source: Arc<AxiomSource>,
}

/// The parsed axioms, kept whole for the model emitter.
///
/// `ProcessedTBox` is a compilation target: it absorbs atomic left-hand sides
/// into `concept_defs`, turns the rest into `¬C ⊔ D`, splits disjunctive
/// left-hand sides and lifts domain and range out of the axiom list entirely.
/// Certifying a model against THAT would certify the reasoner's encoding
/// against itself. This is the input to that compilation, kept so the checker
/// can be pointed at the axioms instead.
struct AxiomSource {
    axioms: Vec<(Concept, Concept)>,
    disjoint_pairs: Vec<(Concept, Concept)>,
    role_domains: Vec<(u32, Concept)>,
    role_ranges: Vec<(u32, Concept)>,
    sub_to_super: HashMap<u32, HashSet<u32>>,
    transitive_roles: HashSet<u32>,
    inverse_roles: HashMap<u32, u32>,
    functional_roles: HashSet<u32>,
    inv_functional_roles: HashSet<u32>,
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

        let source = Arc::new(AxiomSource {
            axioms: result.axioms.clone(),
            disjoint_pairs: result.disjoint_pairs.clone(),
            role_domains: result.role_domains.clone(),
            role_ranges: result.role_ranges.clone(),
            sub_to_super: result.sub_to_super.clone(),
            transitive_roles: result.transitive_roles.clone(),
            inverse_roles: result.inverse_roles.clone(),
            functional_roles: result.functional_roles.clone(),
            inv_functional_roles: result.inv_functional_roles.clone(),
        });

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
            budget_ms: crate::runtime::tableaux_test_timeout_ms(),
            source,
        })
    }

    /// A fresh cut-off for a PHASE that is about to start, counted from now.
    ///
    /// Call this once when a phase begins and hand the result to every tableau
    /// in that phase. Never store it on the reasoner: storing it is exactly what
    /// made the ABox check start on whatever the classification had left.
    ///
    /// A phase, not a test, and the distinction is a deliberate cost decision
    /// rather than a reading of the setting's name. `tableaux_test_timeout_ms`
    /// does say "a single tableau satisfiability test", and minting it per
    /// tableau is the literal reading — but the two sweeps run one tableau per
    /// class and one per ORDERED PAIR of classes, and the shape that bounds them
    /// as a whole is a budget per sweep rather than one per test. Minting per
    /// tableau is also what would make `classify_timeout_ms` the bound that
    /// fires, since with 10s per tableau a sweep runs until the 180s ceiling
    /// stops it — and it costs what it says
    /// it costs: MEASURED on this machine, a 20-ontology corpus went from 67s to
    /// over 600s, with the five ontologies that hit the cap moving from ~10s each
    /// to ~180s each and NO change in any verdict — the same classes stayed
    /// undetermined, just after eighteen times the work. Per phase fixes the
    /// defect that was reported (the ABox check inheriting a spent clock) and
    /// bounds the change at 3x the old worst case instead of 18x. The cost of
    /// that choice is that the ceiling cannot be the bound that fires, and every
    /// `owl-dl` run now says so in its `budget` block rather than leaving a
    /// reader to infer it from two settings and a phase count.
    ///
    /// THAT 67s NUMBER CANNOT BE REPRODUCED FROM THIS REPOSITORY and is kept only
    /// as the reason a route was abandoned. The corpus it was taken over is not
    /// written down anywhere: it is not the twenty case-study ontologies, which
    /// run in 0.10s and reach no budget at all, and it is not the ten hard ones,
    /// of which two reach a budget rather than five. Both of those ARE written
    /// down, in `tests/reasoner_budget_corpus_bench.rs`, which exists so that the
    /// next person to change a budget can measure rather than cite this
    /// paragraph. Cite the bench.
    ///
    /// `classify_timeout_ms` is now a CEILING over every phase, not a fifth
    /// budget beside them: see `phase_deadline_within`, which is what every
    /// phase inside a run actually calls. A bare `phase_deadline` is for a
    /// caller running one phase on its own with no run around it.
    fn phase_deadline(&self) -> Option<Instant> {
        self.budget_ms
            .map(|ms| Instant::now() + std::time::Duration::from_millis(ms))
    }

    /// A phase budget under a run's global ceiling: whichever expires first.
    ///
    /// This is the whole of the cheap half of making `classify_timeout_ms` real.
    /// A tableau already checks ONE deadline inside its expansion loop, so
    /// handing it the earlier of the two costs nothing at all — no second clock
    /// read, no extra branch — while making it impossible for any phase to
    /// outlive the global budget. The expensive half, minting a fresh budget per
    /// TABLEAU so that the global one is what eventually stops the sweep, is what
    /// took the corpus from 67s to over 600s (a number `phase_deadline` flags as
    /// unreproducible) and is not what this does.
    ///
    /// `None` on either side means "that one imposes nothing", so with the global
    /// budget switched off this is the phase budget unchanged, and with the phase
    /// budget switched off it is the global deadline, which is the case that used
    /// to leave a sweep with no clock at all.
    fn phase_deadline_within(&self, global: Option<Instant>) -> Option<Instant> {
        match (self.phase_deadline(), global) {
            (Some(phase), Some(global)) => Some(phase.min(global)),
            (Some(phase), None) => Some(phase),
            (None, global) => global,
        }
    }

    /// The ceiling for one whole run, opened now from `classify_timeout_ms`.
    ///
    /// Open it ONCE per run and pass it down. Opening it per phase is what made
    /// the setting vacuous: four phases each opening a fresh 180s meant a run
    /// could take 720s under a budget that says 180.
    fn global_deadline() -> Option<Instant> {
        crate::runtime::classify_timeout_ms()
            .map(|ms| Instant::now() + std::time::Duration::from_millis(ms))
    }

    /// Three-valued satisfiability test — each call creates its own Tableau.
    ///
    /// Returns `Unknown` when a resource budget was hit before a decision was
    /// reached. Callers MUST NOT read `Unknown` as either answer.
    ///
    /// A bare call is its own phase and gets its own budget. Inside a sweep, use
    /// `decide_satisfiable_within` and pass the deadline the sweep opened with,
    /// so the thousands of tableaux in one sweep share one budget rather than
    /// taking one each.
    pub fn decide_satisfiable(&self, concept: &Concept) -> Verdict {
        self.decide_satisfiable_within(concept, self.phase_deadline())
    }

    /// `decide_satisfiable` against a deadline the caller already opened.
    fn decide_satisfiable_within(&self, concept: &Concept, deadline: Option<Instant>) -> Verdict {
        let mut tableau = Tableau::with_deadline(Arc::clone(&self.tbox), deadline);
        tableau.decide(concept)
    }

    /// `decide_subsumption` against a deadline the caller already opened.
    fn decide_subsumption_within(
        &self,
        sub: &Concept,
        sup: &Concept,
        deadline: Option<Instant>,
    ) -> Verdict {
        let mut test = vec![sub.clone(), sup.negate()];
        test.sort();
        self.decide_satisfiable_within(&Concept::And(test), deadline)
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
        self.is_consistent_within(Self::global_deadline())
    }

    /// `is_consistent` under a ceiling the caller already opened. This is the
    /// first phase of a run and used to be outside the global budget entirely.
    pub fn is_consistent_within(&self, global: Option<Instant>) -> bool {
        !self
            .decide_satisfiable_within(&Concept::Top, self.phase_deadline_within(global))
            .is_unsat()
    }

    /// Explain why a class is unsatisfiable. Returns None if satisfiable.
    pub fn explain_unsatisfiable(&self, class_id: u32) -> Option<Vec<String>> {
        self.explain_unsatisfiable_within(class_id, self.phase_deadline())
    }

    /// `explain_unsatisfiable` against a deadline the caller already opened.
    ///
    /// `Tableau::new_with_tracing` left `Budget::deadline` at `None`, so an
    /// explanation was the one tableau in this engine that ran under no
    /// wall-clock budget at all — and `run` performs one per unsatisfiable
    /// class, in a loop, immediately after a classification that may have been
    /// cut short for want of exactly that budget. The node and depth caps still
    /// applied; nothing stopped the branching.
    ///
    /// An expired deadline yields `None`, which is the honest degradation: the
    /// tableau returns `Unknown`, and `Unknown` is not a proof of
    /// unsatisfiability, so there is no clash trace to report.
    pub fn explain_unsatisfiable_within(
        &self,
        class_id: u32,
        deadline: Option<Instant>,
    ) -> Option<Vec<String>> {
        let concept = Concept::Atom(class_id);
        let mut tableau = Tableau::new_with_tracing(Arc::clone(&self.tbox));
        tableau.budget.deadline = deadline;
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
        self.classify_parallel_within(Self::global_deadline())
    }

    /// `classify_parallel` under a ceiling the caller already opened, so that
    /// classification and the phases around it share ONE global budget instead
    /// of opening one each.
    pub fn classify_parallel_within(
        &self,
        global_deadline: Option<Instant>,
    ) -> AgentClassificationResult {
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

        // The ceiling for the whole run, opened by the caller.
        //
        // The per-test deadline bounds one satisfiability check. It does not
        // bound classification, which runs one check per class plus one per
        // ordered pair of classes. On the Pizza ontology that is ~100 classes
        // and ~10,000 pairs; at the 10s per-test budget the worst case is over
        // a day. A global deadline is what actually stops that.
        //
        // Checked here at the head of each task, which is the cheap place, AND
        // folded into every tableau's own deadline below, which is the place
        // that makes it a ceiling rather than a suggestion: without the fold, a
        // single tableau that started inside the budget could run for a further
        // phase-budget's worth of time past it.
        let out_of_time = || global_deadline.is_some_and(|d| Instant::now() >= d);

        // ── Satisfiability Agent ─────────────────────────────────────
        let sat_start = Instant::now();
        // One budget for this phase, opened here. The subsumption sweep below
        // opens its own, and `check_abox` opens a third when it runs. Before
        // that, all three drew on a single instant fixed when the reasoner was
        // constructed, so whichever phase ran first spent the clock and the ABox
        // check — which runs last and is the one a user reads for their own data
        // — routinely reported `undecided` on an ABox it decides in microseconds.
        let sat_deadline = self.phase_deadline_within(global_deadline);
        let sat_results: Vec<(u32, Verdict)> = classes
            .par_iter()
            .map(|&cls| {
                if out_of_time() {
                    // Budget gone: report Unknown rather than guessing.
                    return (cls, Verdict::Unknown);
                }
                (
                    cls,
                    self.decide_satisfiable_within(&Concept::Atom(cls), sat_deadline),
                )
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
        // This phase's own budget, opened now rather than inherited from the
        // satisfiability sweep that just finished.
        let sub_deadline = self.phase_deadline_within(global_deadline);
        let inferred: Vec<(u32, u32)> = pairs
            .par_iter()
            .filter(|(sub, sup)| {
                if out_of_time() {
                    // Untested pairs are NOT non-subsumptions. Flag the run as
                    // incomplete so no caller reads absence as refutation.
                    subsumption_cut_short.store(true, std::sync::atomic::Ordering::Relaxed);
                    return false;
                }
                self.decide_subsumption_within(
                    &Concept::Atom(*sub),
                    &Concept::Atom(*sup),
                    sub_deadline,
                )
                .is_unsat()
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

        // Every individual, not only the ones carrying a NAMED rdf:type.
        //
        // This used to iterate `individual_types`, which is keyed by exactly
        // those. An individual with no rdf:type at all — named only by its own
        // role assertions — and an individual typed ONLY by an anonymous class
        // expression both have no entry there, so neither was ever realized into
        // a defined class however completely it met the definition.
        // `build_abox_tableau` has given both a node for some time, because a
        // domain constraint binds them and a disjointness axiom can refute them:
        // the consistency check saw them and the realizer did not.
        //
        // The universe is built the same way `build_abox_tableau` builds its
        // node set, so the individuals realization considers and the individuals
        // the consistency check ran on are one set rather than two that can
        // drift. Widening it cannot widen what counts as a match: `satisfies`
        // declines everything outside its fragment, so an empty named-type set
        // can only fail to match.
        let mut individuals: Vec<u32> = self.individual_types.keys().copied().collect();
        for &ind in self.individual_anon_types.keys() {
            if !self.individual_types.contains_key(&ind) {
                individuals.push(ind);
            }
        }
        for &(a, _, b) in &self.role_assertions {
            individuals.push(a);
            individuals.push(b);
        }
        individuals.sort_unstable();
        individuals.dedup();

        let empty: HashSet<u32> = HashSet::new();
        for ind in individuals {
            // Conjuncts of an anonymous rdf:type are assertions in their own
            // right, and the atomic ones are named types this individual has
            // exactly as if they had been written as an ordinary rdf:type.
            let asserted = self.asserted_concepts(ind);
            let declared = self.individual_types.get(&ind).unwrap_or(&empty);
            // Borrowed unless an anonymous type actually contributes a named
            // one. Widening the universe to every individual means this runs
            // once per individual rather than once per TYPED individual, and a
            // SKOS vocabulary is tens of thousands of individuals with no
            // anonymous type at all; a HashSet clone each would be a real cost
            // for nothing.
            let types: std::borrow::Cow<'_, HashSet<u32>> =
                if asserted.iter().any(|c| matches!(c, Concept::Atom(_))) {
                    std::borrow::Cow::Owned(
                        declared
                            .iter()
                            .copied()
                            .chain(asserted.iter().filter_map(|c| match c {
                                Concept::Atom(a) => Some(*a),
                                _ => None,
                            }))
                            .collect(),
                    )
                } else {
                    std::borrow::Cow::Borrowed(declared)
                };
            for (&cls, def) in &self.definitions {
                if types.contains(&cls) {
                    continue;
                }
                if self.satisfies(ind, &types, &asserted, def, &by_subject) {
                    out.entry(ind).or_default().insert(cls);
                }
            }
        }
        out
    }

    /// Every concept `ind` is DIRECTLY asserted to belong to by an anonymous
    /// `rdf:type`, flattened through conjunction.
    ///
    /// `x rdf:type C ⊓ D` entails `x rdf:type C` and `x rdf:type D`, so each
    /// conjunct is an assertion on its own. Nothing else is unfolded: no axiom is
    /// applied and no subsumption is followed here, so this stays a reading of
    /// what the graph SAYS rather than an inference from it, which is the same
    /// discipline the rest of this function keeps.
    fn asserted_concepts(&self, ind: u32) -> Vec<Concept> {
        let mut out = Vec::new();
        let Some(anon) = self.individual_anon_types.get(&ind) else {
            return out;
        };
        let mut stack: Vec<Concept> = anon.clone();
        while let Some(c) = stack.pop() {
            match c {
                Concept::And(parts) => stack.extend(parts),
                other => out.push(other),
            }
        }
        out
    }

    /// True when the asserted facts about `ind` entail `concept`, for the accepted fragment.
    /// Returns false for anything outside it, which keeps unsupported definitions empty
    /// rather than unsound.
    ///
    /// `types` is the individual's named types, `asserted` the conjuncts of its
    /// anonymous ones. Either may be empty; an individual with neither is decided
    /// entirely on its role assertions, which is the whole point of considering it.
    fn satisfies(
        &self,
        ind: u32,
        types: &HashSet<u32>,
        asserted: &[Concept],
        concept: &Concept,
        by_subject: &HashMap<u32, Vec<(u32, u32)>>,
    ) -> bool {
        // An assertion discharges the conjunct it is spelled the same as. Both
        // sides are in NNF and come from the same interner, so structural
        // equality here is concept identity, and `x : C` entails `C` for any `C`
        // whatever — including the shapes the match below declines.
        if asserted.iter().any(|a| a == concept) {
            return true;
        }
        match concept {
            Concept::Top => true,
            Concept::Atom(c) => self.has_type(types, *c),
            Concept::And(parts) => parts
                .iter()
                .all(|p| self.satisfies(ind, types, asserted, p, by_subject)),
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

    /// Build the single tableau the ABox check runs on, with one node per named
    /// individual and the asserted role assertions as edges.
    ///
    /// Extracted verbatim out of `check_abox` so the model emitter runs on the
    /// SAME graph the consistency answer came from. Two copies of this setup
    /// would let the certificate drift away from the verdict it is supposed to
    /// certify, which is precisely the failure this whole layer exists to catch.
    ///
    /// Returns the tableau, the individual-to-node map, and the number of
    /// individuals the check actually ran on, which is one per node in that map.
    ///
    /// That count USED TO BE the number of individuals carrying a type, taken
    /// before the nodes for role-assertion endpoints were added, so it undercounted
    /// by exactly the individuals this reasoner had the least information about. It
    /// is a reported number a reader uses to sanity-check that the check saw their
    /// data, and a node that can carry a label, a GCI and a clash has been checked
    /// whatever its `rdf:type` says. It matters more now: an untyped subject of a
    /// role assertion is a node that a domain constraint binds and a disjointness
    /// axiom can refute, and reporting `individuals_checked: 0` for an ABox made
    /// entirely of those also suppressed the whole `abox` block from the output.
    fn build_abox_tableau(&self, global: Option<Instant>) -> (Tableau, HashMap<u32, u32>, usize) {
        // The ABox check builds ONE tableau containing every named individual,
        // so it is the largest single expansion the reasoner ever performs and it
        // must be under a budget, or it is an unbounded hole in one. It opens its
        // OWN, which is the point of `phase_deadline`: sharing an instant with
        // classification is what had this phase reporting `undecided` on ABoxes
        // it decides in microseconds, because classification had already spent it.
        let mut tableau =
            Tableau::with_deadline(Arc::clone(&self.tbox), self.phase_deadline_within(global));
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
        // Counted at the end, off `ind_to_node`, not here off `all_individuals`.
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

        // An individual may be named only by a role assertion, typed by a
        // vocabulary this reasoner does not treat as a class (a skos:Concept, say) and never
        // declared owl:NamedIndividual. It still has to exist as a node, or the edge that
        // names it leads nowhere and the restriction that depends on it silently fails.
        //
        // BOTH endpoints, and the subject half is the newer one. The object half
        // was here already; the subject half was unreachable, because an untyped
        // subject's assertions were dropped during parsing and never became
        // `role_assertions` at all. Now that they survive, the subject needs a
        // node for the same reason the object does — and it needs one MORE than
        // the object does, because `rdfs:domain` binds the source of the edge, so
        // the subject is where a domain clash actually lands.
        let referenced: Vec<u32> = self
            .role_assertions
            .iter()
            .flat_map(|&(a, _, b)| [a, b])
            .filter(|x| !ind_to_node.contains_key(x))
            .collect();
        for ind in referenced {
            // `referenced` can name the same individual twice, once as a subject
            // and once as an object, so this is not redundant with the filter.
            if ind_to_node.contains_key(&ind) {
                continue;
            }
            let node_id = tableau.fresh_node(None, None);
            ind_to_node.insert(ind, node_id);
            tableau.add_label(node_id, Concept::Atom(ind));
            // A GCI holds of EVERY element of the domain, so a node that does not
            // carry the GCIs is a hole this check cannot see into. These nodes were
            // the only ones in the tableau created without them — the loop above
            // adds them to every typed individual and `create_successor` adds them
            // to every generated successor — and the omission was a second false
            // clean of the same shape as the domain/range one: an ABox whose only
            // contradiction lands on an individual named just by a role assertion
            // was reported consistent.
            for gci in tableau.tbox.gcis.clone() {
                tableau.add_label(node_id, gci);
            }
        }

        // Add role assertions as edges. An asserted edge a R b also means b has an
        // R-inverse edge to a: without it, a ForAll or cardinality constraint on b
        // never propagates backward across the edge, and an inconsistency reachable
        // only through the inverse (or, since a symmetric role is its own inverse in
        // `inverse_roles`, a symmetric) role is silently missed. Unlike the tree
        // tableau, ABox individuals form an arbitrary graph, so the parent back-link
        // successors() uses cannot stand in for the inverse neighbour. Materialise it.
        //
        // Both edges go in through `add_role_edge`, which is what applies rdfs:domain
        // and rdfs:range. Writing them into the edge map directly, as this used to,
        // is what made an asserted edge weaker than a generated one.
        for &(a, r, b) in &self.role_assertions {
            if let (Some(&a_node), Some(&b_node)) = (ind_to_node.get(&a), ind_to_node.get(&b)) {
                let r_inv = tableau.tbox.inverse_roles.get(&r).copied();
                tableau.add_role_edge(a_node, r, b_node);
                if let Some(r_inv) = r_inv {
                    tableau.add_role_edge(b_node, r_inv, a_node);
                }
            }
        }


        let individuals_checked = ind_to_node.len();
        (tableau, ind_to_node, individuals_checked)
    }

    pub fn check_abox(&self) -> ABoxResult {
        self.check_abox_within(Self::global_deadline())
    }

    /// `check_abox` under a ceiling the caller already opened.
    ///
    /// This phase used to be outside `classify_timeout_ms` entirely: the setting
    /// was read inside `classify_parallel` and nowhere else, so a run that spent
    /// the whole global budget classifying went on to open a fresh phase budget
    /// here. A knob a run can exceed is not a ceiling.
    pub fn check_abox_within(&self, global: Option<Instant>) -> ABoxResult {
        // "No ABox" has to mean no ABox, and a role assertion is ABox. An
        // ontology whose entire instance data is untyped individuals related by
        // a property with a domain still states something refutable, and this
        // guard used to return `consistent: true` on it without building
        // anything, which is the same false clean one level up from the one
        // `add_role_edge` closed.
        if self.individual_types.is_empty()
            && self.individual_anon_types.is_empty()
            && self.role_assertions.is_empty()
        {
            return ABoxResult {
                consistent: true,
                undecided: false,
                individuals_checked: 0,
                inferred_types: HashMap::new(),
            };
        }

        let (mut tableau, ind_to_node, individuals_checked) = self.build_abox_tableau(global);

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

        // ONE ceiling for the whole run, opened here and passed to every phase.
        //
        // `classify_timeout_ms` used to be read inside `classify_parallel` and
        // nowhere else, so the three phases around it — the consistency check
        // before, the ABox check after, the explanation loop after that — each
        // opened a budget of their own and none of them was under it. A run
        // could exceed the ceiling it was given by three further phase budgets,
        // and the explanation loop had no wall clock at all.
        let global_deadline = Self::global_deadline();

        let tbox_consistent = reasoner.is_consistent_within(global_deadline);
        let result = reasoner.classify_parallel_within(global_deadline);
        let abox_result = reasoner.check_abox_within(global_deadline);

        // The headline flag answers for the whole knowledge base. Reporting the TBox alone
        // while the ABox check has PROVEN an inconsistency in the same output is a false
        // claim; the three-valued discipline is preserved because an undecided ABox already
        // defaults to consistent inside check_abox.
        let consistent = tbox_consistent && abox_result.consistent;

        // Collect explanations for unsatisfiable classes. One phase, one budget,
        // under the same ceiling as the three before it.
        let explain_deadline = reasoner.phase_deadline_within(global_deadline);
        let mut explanations: Vec<serde_json::Value> = Vec::new();
        for &cls in &result.unsatisfiable {
            if let Some(steps) = reasoner.explain_unsatisfiable_within(cls, explain_deadline) {
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

        // WHICH phase ran out, not merely THAT something did.
        //
        // `complete: false` says the run is not a proof. It does not say which of
        // the three phases to give more room. Every phase now draws on the same
        // two settings — its own `tableaux_test_timeout_ms` budget, under the
        // `[reasoner] classify_timeout_ms` ceiling for the whole run — and the
        // `budget` block below says which of the two is the one that fires. The
        // ABox check used to be under the phase budget ALONE, outside the ceiling
        // entirely. A reader who is told only "incomplete" cannot act, and before
        // each phase got its own budget
        // the phase that hit the wall was usually not the phase that spent the
        // time — classification would eat the clock and the ABox check would be
        // the one reporting `undecided`. Naming the phases makes that visible
        // rather than leaving it to be inferred.
        let mut budget_exhausted_in: Vec<&str> = Vec::new();
        if !undetermined_names.is_empty() {
            budget_exhausted_in.push("satisfiability");
        }
        if result.subsumption_cut_short {
            budget_exhausted_in.push("subsumption");
        }
        if abox_result.undecided {
            budget_exhausted_in.push("abox");
        }

        // WHICH budget was in force, in words, in the output.
        //
        // `classify_timeout_ms` defaults to 180 000 ms and its own documentation
        // called it "the budget that actually bounds the run". It had never
        // bounded one. Each phase opens its own deadline from
        // `tableaux_test_timeout_ms`, which defaults to 10 000 ms; five phases is
        // a worst case of 50 000 ms, so the 180 000 ms ceiling is dead
        // arithmetic and cannot fire. Handing the global budget the run instead
        // means minting a deadline per TABLEAU, which was measured and took a
        // 20-ontology corpus from 67s to over 600s, and was abandoned. That
        // number is unreproducible from this tree; `phase_deadline` says why and
        // names the bench that is.
        //
        // What is left is the thing that was missing: SAYING SO. A knob that
        // reads as a safety limit may not enforce nothing, and the arithmetic
        // that decides whether it can fire is two settings and a phase count,
        // which is not something a reader of the output can be expected to do.
        // The ceiling is now genuinely enforced over every phase — see
        // `phase_deadline_within` — and this block says which of the two bounds
        // is the one that actually stops the run, including when the answer is
        // neither.
        let phase_names = ["consistency", "satisfiability", "subsumption", "abox", "explanation"];
        let global_ms = crate::runtime::classify_timeout_ms();
        let phase_ms = crate::runtime::tableaux_test_timeout_ms();
        let phase_worst_case_ms = phase_ms.map(|ms| ms * phase_names.len() as u64);
        let (binding_bound, note) = match (global_ms, phase_worst_case_ms) {
            (None, None) => (
                "none",
                format!(
                    "no wall-clock bound of any kind is in force: classify_timeout_ms and \
                     tableaux_test_timeout_ms are both 0. The only limits left are the node \
                     cap ({}) and the depth cap ({}), and neither bounds the number of \
                     BRANCHES a tableau explores.",
                    crate::runtime::tableaux_max_nodes(),
                    crate::runtime::tableaux_max_depth()
                ),
            ),
            (None, Some(worst)) => (
                "phase",
                format!(
                    "no global bound is in force: classify_timeout_ms is 0. Each of the {} \
                     phases opens its own {} ms budget, so the worst case for this run is \
                     {worst} ms.",
                    phase_names.len(),
                    phase_ms.unwrap_or(0)
                ),
            ),
            (Some(g), None) => (
                "global",
                format!(
                    "classify_timeout_ms ({g} ms) is the only wall-clock bound: \
                     tableaux_test_timeout_ms is 0, so no phase budget can expire before it \
                     and every tableau runs to the global deadline."
                ),
            ),
            (Some(g), Some(worst)) if g <= worst => (
                "global",
                format!(
                    "classify_timeout_ms ({g} ms) is at or below the worst case the phase \
                     budgets permit ({} phases x {} ms = {worst} ms), so it is the bound that \
                     stops this run.",
                    phase_names.len(),
                    phase_ms.unwrap_or(0)
                ),
            ),
            (Some(g), Some(worst)) => (
                "phase",
                format!(
                    "classify_timeout_ms ({g} ms) cannot be the bound that stops this run: \
                     {} phases at {} ms each cap it at {worst} ms. It is enforced as a ceiling \
                     over every phase and will not fire at this setting. The bound in force is \
                     the phase budget; lower classify_timeout_ms below {worst} to make it the \
                     binding one.",
                    phase_names.len(),
                    phase_ms.unwrap_or(0)
                ),
            ),
        };

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
            "description_logic": "SHIQ",
            "consistent": consistent,
            "tbox_consistent": tbox_consistent,
            "named_classes": reasoner.named_classes.len(),
            "unsatisfiable_classes": unsat_names,
            "complete": complete,
            "budget_exhausted_in": budget_exhausted_in,
            "budget": {
                "classify_timeout_ms": global_ms,
                "phase_timeout_ms": phase_ms,
                "phases": phase_names,
                "phase_worst_case_ms": phase_worst_case_ms,
                "binding_bound": binding_bound,
                "note": note,
            },
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

        // Opt-in model certificates. Off unless `OO_DL_MODEL_DIR` names a
        // directory, so the default behaviour of `owl-dl` is exactly what it
        // was. Only the POSITIVE answers are certified: an inconsistent TBox or
        // an unsatisfiable class produces `Refuted` here, which means "no
        // certificate", not "certified unsatisfiable".
        if let Ok(root) = std::env::var("OO_DL_MODEL_DIR") {
            let root = std::path::PathBuf::from(root);
            let describe = |r: anyhow::Result<ModelOutcome>| match r {
                Ok(o) => o.describe(),
                Err(e) => format!("no certificate: {e}"),
            };
            // Per class as well as per ontology. TBox consistency is honestly
            // witnessed by a single point with empty extensions, which really
            // is what consistency means and is a weak-looking artefact; the
            // per-class certificates are the informative ones, and the sweep
            // that validated this layer used them.
            let mut classes = serde_json::Map::new();
            for (i, class) in reasoner.named_class_names().iter().enumerate() {
                let sub = root.join("classes").join(i.to_string());
                classes.insert(
                    class.clone(),
                    serde_json::json!(describe(reasoner.certify_class_satisfiable(class, &sub))),
                );
            }
            let unmodelled = DlReasoner::unmodelled_constructs(graph);
            output["model_certificate"] = serde_json::json!({
                "dir": root.display().to_string(),
                "tbox": describe(reasoner.certify_tbox_consistent(&root.join("tbox"))),
                "abox": describe(reasoner.certify_abox_consistent(&root.join("abox"))),
                "classes": classes,
                "checker": "oo-dlmodel",
                "theorem": "Dl.satisfiable_of_checkModel",
                // The constructs below are IN THE GRAPH and NOT in the axioms
                // the certificate is about, so where this list is non-empty the
                // certificate describes a weaker axiom set than the ontology
                // states. Read the verdict against this list, not on its own.
                "certifies_a_weaker_axiom_set": !unmodelled.is_empty(),
                "constructs_not_modelled": unmodelled
                    .iter()
                    .map(|(k, n)| serde_json::json!({"construct": k, "occurrences": n}))
                    .collect::<Vec<_>>(),
                "not_covered": "unsatisfiability and inconsistency carry no certificate",
            });
        }

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

// ── Model certificates ──────────────────────────────────────────────────
//
// Every verdict above is a bare verdict. A consumer who wants to know whether
// the reasoner was right has to trust the reasoner. This section closes half of
// that gap, and only half, deliberately.
//
// A tableaux run gives two kinds of answer and they are not equally hard to
// certify. SATISFIABLE is easy: the procedure builds a completion graph, and a
// completion graph either is a finite model or folds into one. A finite model is
// a finite object, and checking that a finite interpretation satisfies a set of
// axioms is decidable and provable. UNSATISFIABLE is hard: it needs the closed
// tableau, every branch, every clash and the blocking argument, which is a
// proof-checking problem and is not attempted here.
//
// So the positive answers carry a certificate and the negative ones do not. When
// the reasoner reports a class unsatisfiable or an ontology inconsistent, it
// still just says so, and the report says that plainly rather than blurring the
// two.
//
// The certificate is two tab-separated files, `axioms.tsv` and `model.tsv`, read
// by `oo-dlmodel` (`lean/DlMain.lean`). `lean/Dl/Syntax.lean` documents the
// format and is the normative description of it. `Dl.satisfiable_of_checkModel`
// is the machine-checked statement the exit code stands for: an interpretation
// the checker accepts is a model, so the axiom set is satisfiable.
//
// Three things this does NOT do, stated here so no reader has to discover them:
//
//  1. The OWL parser is outside the theorem. The axioms written out are the
//     axioms `OwlParser` read, not the ontology. A triple the parser ignores is
//     absent from the certificate and the checker never sees it.
//  2. Nothing is emitted for an answer that is not positive, and nothing is
//     emitted when the completion graph cannot be turned into a finite
//     interpretation. `ModelOutcome::Refused` says which, and no file is
//     written. Emitting something that will not check would be worse than
//     emitting nothing.
//  3. The self-check below decides whether to emit. It is the same semantics
//     `lean/Dl/Check.lean` implements, written twice on purpose: the Rust copy
//     is a gate on the emitter, and the Lean copy is the one with the proof. A
//     disagreement between them shows up as `oo-dlmodel` rejecting a certificate
//     this side thought was fine, which is a failure, not a shrug.

use std::path::Path;

/// An axiom in the fragment `lean/Dl/` covers, over interned identifiers.
#[derive(Clone, Debug)]
enum DlAxiom {
    Sub(Concept, Concept),
    Disjoint(Concept, Concept),
    Domain(u32, Concept),
    Range(u32, Concept),
    SubRole(u32, u32),
    Trans(u32),
    Sym(u32),
    Inv(u32, u32),
    InvFunc(u32),
    Inst(u32, Concept),
    Rel(u32, u32, u32),
    Indiv(u32),
    NonEmpty(Concept),
}

/// A folded completion graph: the node identifiers that survived, and the edges
/// between them, each written `(source, role, target)`.
type FoldedGraph = (Vec<u32>, Vec<(u32, u32, u32)>);

/// A finite interpretation whose domain elements are tableau node identifiers.
#[derive(Default)]
struct FiniteModel {
    dom: Vec<u32>,
    domset: HashSet<u32>,
    /// Atomic concept identifier → the nodes in its extension. Closed world on
    /// atoms: a node is in `A` exactly when `Atom(A)` is in its label. That is
    /// the canonical reading of a completion graph, and it is what makes the
    /// negated atoms in the labels mean anything.
    cext: HashMap<u32, HashSet<u32>>,
    /// (role, source) → deduplicated targets.
    rext: HashMap<(u32, u32), Vec<u32>>,
    /// Individual identifier → the node that denotes it.
    ind: HashMap<u32, u32>,
}

/// What `DlReasoner::certify_*` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelOutcome {
    /// `axioms.tsv` and `model.tsv` were written and pass the emitter's own
    /// check. `oo-dlmodel` is what turns that into a verified statement.
    Certified {
        axioms: usize,
        domain: usize,
        edges: usize,
    },
    /// The reasoner PROVED the negative answer. There is no model to hand over,
    /// and this layer says nothing about whether the proof is right.
    Refuted,
    /// A resource budget was hit. Nothing was proven either way.
    Undetermined,
    /// A positive answer was reached but the completion graph could not be
    /// turned into a finite interpretation that satisfies the axioms. No file
    /// was written. The string says why.
    Refused(String),
}

impl ModelOutcome {
    /// True only when files were written.
    pub fn is_certified(&self) -> bool {
        matches!(self, ModelOutcome::Certified { .. })
    }

    /// A one-line description, for logs and for the `owl-dl` JSON output.
    pub fn describe(&self) -> String {
        match self {
            ModelOutcome::Certified {
                axioms,
                domain,
                edges,
            } => format!("certified: {axioms} axioms, {domain} elements, {edges} edges"),
            ModelOutcome::Refuted => "no certificate: the answer was negative".to_string(),
            ModelOutcome::Undetermined => "no certificate: the run was undetermined".to_string(),
            ModelOutcome::Refused(why) => format!("no certificate: {why}"),
        }
    }
}

/// A name that survives the round trip through the two files.
///
/// The concept grammar is space-separated and the files are tab-separated, so a
/// term carrying either would be read back as a different term. N-Triples IRIs
/// and blank node labels never do. A literal can, and `owl:hasValue` is
/// approximated here by an atomic concept named after the individual, which may
/// be a literal. Refusing is the only honest answer for one of those.
fn name_is_safe(s: &str) -> bool {
    !s.is_empty() && !s.contains([' ', '\t', '\n', '\r'])
}

// ── Serialisation ───────────────────────────────────────────────────────

/// The ONE place a name enters either file, and therefore the one place the
/// guard has to be.
///
/// TCB-26 used to read: "the `names` vector is built by walking every axiom
/// variant, every role in `model.rext` and every class in `model.cext`.
/// Individuals in `model.ind` are covered only because `DlAxiom::Indiv(i)` is
/// emitted for every individual that reaches the model. That is an argument
/// about two separate loops agreeing, not a check." The separate loop is gone.
/// A name is checked as it is written, so "the guard covers every name that is
/// written" is true by construction: there is no other way to write one.
///
/// `bad` records the FIRST refused name and the caller refuses the whole
/// certificate. It is a sink rather than a `Result` so the serialisers stay
/// total and keep their shape; a certificate is written only after the caller
/// has looked at it.
fn push_name(out: &mut String, interner: &Interner, id: u32, bad: &mut Option<String>) {
    let s = interner.resolve(id);
    if !name_is_safe(s) && bad.is_none() {
        *bad = Some(s.to_string());
    }
    out.push_str(s);
}

fn write_concept(out: &mut String, interner: &Interner, c: &Concept, bad: &mut Option<String>) {
    match c {
        Concept::Top => out.push_str("top"),
        Concept::Bottom => out.push_str("bot"),
        Concept::Atom(a) => {
            out.push_str("atom ");
            push_name(out, interner, *a, bad);
        }
        Concept::NegAtom(a) => {
            out.push_str("not atom ");
            push_name(out, interner, *a, bad);
        }
        // `lean/Dl` has binary conjunction and disjunction, so the n-ary OWL
        // constructors are folded right-associatively here. An empty list is the
        // unit of the connective, which is how `RawConcept::to_nnf` already
        // reads it.
        Concept::And(cs) => write_nary(out, interner, cs, "and", &Concept::Top, bad),
        Concept::Or(cs) => write_nary(out, interner, cs, "or", &Concept::Bottom, bad),
        Concept::Exists(r, f) => {
            out.push_str("some ");
            push_name(out, interner, *r, bad);
            out.push(' ');
            write_concept(out, interner, f, bad);
        }
        Concept::ForAll(r, f) => {
            out.push_str("all ");
            push_name(out, interner, *r, bad);
            out.push(' ');
            write_concept(out, interner, f, bad);
        }
        Concept::MinCard(r, n, f) => {
            out.push_str("min ");
            out.push_str(&n.to_string());
            out.push(' ');
            push_name(out, interner, *r, bad);
            out.push(' ');
            write_concept(out, interner, f, bad);
        }
        Concept::MaxCard(r, n, f) => {
            out.push_str("max ");
            out.push_str(&n.to_string());
            out.push(' ');
            push_name(out, interner, *r, bad);
            out.push(' ');
            write_concept(out, interner, f, bad);
        }
    }
}

fn write_nary(
    out: &mut String,
    interner: &Interner,
    cs: &[Concept],
    op: &str,
    unit: &Concept,
    bad: &mut Option<String>,
) {
    match cs.split_first() {
        None => write_concept(out, interner, unit, bad),
        Some((head, [])) => write_concept(out, interner, head, bad),
        Some((head, rest)) => {
            out.push_str(op);
            out.push(' ');
            write_concept(out, interner, head, bad);
            out.push(' ');
            write_nary(out, interner, rest, op, unit, bad);
        }
    }
}

fn concept_string(interner: &Interner, c: &Concept, bad: &mut Option<String>) -> String {
    let mut s = String::new();
    write_concept(&mut s, interner, c, bad);
    s
}

fn axiom_line(interner: &Interner, a: &DlAxiom, bad: &mut Option<String>) -> String {
    // Every name in the line goes through `push_name`, including the ones that
    // are not inside a concept, so `bad` is set by the time the line exists.
    let cs = |c: &Concept, bad: &mut Option<String>| concept_string(interner, c, bad);
    let r = |id: &u32, bad: &mut Option<String>| {
        let mut s = String::new();
        push_name(&mut s, interner, *id, bad);
        s
    };
    match a {
        DlAxiom::Sub(c, d) => format!("sub\t{}\t{}", cs(c, bad), cs(d, bad)),
        DlAxiom::Disjoint(c, d) => format!("disjoint\t{}\t{}", cs(c, bad), cs(d, bad)),
        DlAxiom::Domain(role, c) => format!("domain\t{}\t{}", r(role, bad), cs(c, bad)),
        DlAxiom::Range(role, c) => format!("range\t{}\t{}", r(role, bad), cs(c, bad)),
        DlAxiom::SubRole(a1, b1) => format!("subrole\t{}\t{}", r(a1, bad), r(b1, bad)),
        DlAxiom::Trans(role) => format!("trans\t{}", r(role, bad)),
        DlAxiom::Sym(role) => format!("sym\t{}", r(role, bad)),
        DlAxiom::Inv(a1, b1) => format!("inv\t{}\t{}", r(a1, bad), r(b1, bad)),
        DlAxiom::InvFunc(role) => format!("invfunc\t{}", r(role, bad)),
        DlAxiom::Inst(i, c) => format!("inst\t{}\t{}", r(i, bad), cs(c, bad)),
        DlAxiom::Rel(a1, role, b1) => {
            format!("rel\t{}\t{}\t{}", r(a1, bad), r(role, bad), r(b1, bad))
        }
        DlAxiom::Indiv(i) => format!("indiv\t{}", r(i, bad)),
        DlAxiom::NonEmpty(c) => format!("nonempty\t{}", cs(c, bad)),
    }
}

// ── The finite interpretation, and the check on it ──────────────────────

impl FiniteModel {
    fn successors(&self, role: u32, x: u32) -> &[u32] {
        self.rext
            .get(&(role, x))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    fn in_class(&self, class: u32, x: u32) -> bool {
        self.cext.get(&class).is_some_and(|s| s.contains(&x))
    }

    /// The same clauses as `Dl.sat` in `lean/Dl/Check.lean`. `rext` entries are
    /// deduplicated when the model is built, so the counting cases compare
    /// against a count of DISTINCT successors, which is what `Dl.AtLeast` and
    /// `Dl.AtMost` mean.
    fn sat(&self, c: &Concept, x: u32) -> bool {
        match c {
            Concept::Top => true,
            Concept::Bottom => false,
            Concept::Atom(a) => self.in_class(*a, x),
            Concept::NegAtom(a) => !self.in_class(*a, x),
            Concept::And(cs) => cs.iter().all(|d| self.sat(d, x)),
            Concept::Or(cs) => cs.iter().any(|d| self.sat(d, x)),
            Concept::Exists(r, f) => self.successors(*r, x).iter().any(|&y| self.sat(f, y)),
            Concept::ForAll(r, f) => self.successors(*r, x).iter().all(|&y| self.sat(f, y)),
            Concept::MinCard(r, n, f) => {
                self.successors(*r, x).iter().filter(|&&y| self.sat(f, y)).count() >= *n as usize
            }
            Concept::MaxCard(r, n, f) => {
                self.successors(*r, x).iter().filter(|&&y| self.sat(f, y)).count() <= *n as usize
            }
        }
    }

    /// The same clauses as `Dl.holds`.
    fn holds(&self, a: &DlAxiom) -> bool {
        match a {
            DlAxiom::Sub(c, d) => self.dom.iter().all(|&x| !self.sat(c, x) || self.sat(d, x)),
            DlAxiom::Disjoint(c, d) => {
                self.dom.iter().all(|&x| !(self.sat(c, x) && self.sat(d, x)))
            }
            DlAxiom::Domain(r, c) => self
                .dom
                .iter()
                .all(|&x| self.successors(*r, x).is_empty() || self.sat(c, x)),
            DlAxiom::Range(r, c) => self
                .dom
                .iter()
                .all(|&x| self.successors(*r, x).iter().all(|&y| self.sat(c, y))),
            DlAxiom::SubRole(r, s) => self.dom.iter().all(|&x| {
                self.successors(*r, x)
                    .iter()
                    .all(|y| self.successors(*s, x).contains(y))
            }),
            DlAxiom::Trans(r) => self.dom.iter().all(|&x| {
                self.successors(*r, x).iter().all(|&y| {
                    self.successors(*r, y)
                        .iter()
                        .all(|z| self.successors(*r, x).contains(z))
                })
            }),
            DlAxiom::Sym(r) => self.dom.iter().all(|&x| {
                self.successors(*r, x)
                    .iter()
                    .all(|&y| self.successors(*r, y).contains(&x))
            }),
            DlAxiom::Inv(r, s) => {
                self.dom.iter().all(|&x| {
                    self.successors(*r, x)
                        .iter()
                        .all(|&y| self.successors(*s, y).contains(&x))
                }) && self.dom.iter().all(|&x| {
                    self.successors(*s, x)
                        .iter()
                        .all(|&y| self.successors(*r, y).contains(&x))
                })
            }
            DlAxiom::InvFunc(r) => self.dom.iter().all(|&y| {
                self.dom
                    .iter()
                    .filter(|&&x| self.successors(*r, x).contains(&y))
                    .count()
                    <= 1
            }),
            DlAxiom::Inst(i, c) => self.ind.get(i).is_some_and(|&x| self.sat(c, x)),
            DlAxiom::Rel(a1, r, b1) => match (self.ind.get(a1), self.ind.get(b1)) {
                (Some(&x), Some(&y)) => self.successors(*r, x).contains(&y),
                _ => false,
            },
            DlAxiom::Indiv(i) => self.ind.get(i).is_some_and(|x| self.domset.contains(x)),
            DlAxiom::NonEmpty(c) => self.dom.iter().any(|&x| self.sat(c, x)),
        }
    }

    /// The same clauses as `Dl.checkWF`. Returns the first failure.
    fn well_formed(&self, axioms: &[DlAxiom]) -> Result<(), String> {
        if self.dom.is_empty() {
            return Err("the domain is empty".to_string());
        }
        for (class, members) in &self.cext {
            if let Some(bad) = members.iter().find(|m| !self.domset.contains(m)) {
                return Err(format!(
                    "class {class} has member n{bad} outside the domain"
                ));
            }
        }
        for (&(role, from), targets) in &self.rext {
            if !self.domset.contains(&from) {
                return Err(format!("role {role} has an edge from n{from}, outside the domain"));
            }
            if let Some(bad) = targets.iter().find(|t| !self.domset.contains(t)) {
                return Err(format!("role {role} has an edge to n{bad}, outside the domain"));
            }
        }
        for a in axioms {
            let inds: Vec<u32> = match a {
                DlAxiom::Inst(i, _) | DlAxiom::Indiv(i) => vec![*i],
                DlAxiom::Rel(x, _, y) => vec![*x, *y],
                _ => vec![],
            };
            for i in inds {
                match self.ind.get(&i) {
                    Some(x) if self.domset.contains(x) => {}
                    _ => {
                        return Err(format!(
                            "individual {i} has no denotation inside the domain"
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

// ── Reading a completion graph off a finished tableau ────────────────────

impl Tableau {
    /// Fold blocked nodes into the nodes that block them, and return the
    /// surviving nodes with their labels and edges.
    ///
    /// A blocked node is never expanded, so its label can carry an `∃R.C` with
    /// no `R`-successor: the completion graph as it stands is NOT an
    /// interpretation. The standard repair is to send the edge that reaches the
    /// blocked node to its blocker instead. Pairwise blocking requires the two
    /// labels to be EQUAL, not merely included, so the redirect moves the edge
    /// to a node carrying exactly the same constraints.
    ///
    /// That argument is why the fold is worth doing at all. It is not why the
    /// result is trusted: the caller checks the folded interpretation against
    /// the axioms and refuses to emit if it does not hold. SHIQ has no finite
    /// model property, so for some inputs no fold can work, and refusing is the
    /// correct outcome there rather than a bug.
    fn folded_graph(&self) -> Result<FoldedGraph, String> {
        let resolve = |mut n: u32| -> Result<u32, String> {
            let mut steps = 0usize;
            while let Some(b) = self.blocker_of(n) {
                n = b;
                steps += 1;
                if steps > self.nodes.len() + 1 {
                    return Err("a blocking chain did not terminate".to_string());
                }
            }
            Ok(n)
        };

        let mut roots: Vec<u32> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.parent.is_none())
            .map(|(&id, _)| id)
            .collect();
        roots.sort_unstable();
        if roots.is_empty() {
            return Err("the completion graph has no root node".to_string());
        }

        let mut kept: Vec<u32> = Vec::new();
        let mut seen: HashSet<u32> = HashSet::new();
        let mut edges: Vec<(u32, u32, u32)> = Vec::new();
        let mut queue: Vec<u32> = Vec::new();
        for r in roots {
            let r = resolve(r)?;
            if seen.insert(r) {
                kept.push(r);
                queue.push(r);
            }
        }
        while let Some(x) = queue.pop() {
            let Some(node) = self.nodes.get(&x) else {
                return Err(format!("node {x} is referenced but absent"));
            };
            let mut roles: Vec<u32> = node.edges.keys().copied().collect();
            roles.sort_unstable();
            for role in roles {
                let mut targets: Vec<u32> = node.edges[&role].iter().copied().collect();
                targets.sort_unstable();
                for t in targets {
                    let t = resolve(t)?;
                    edges.push((x, role, t));
                    if seen.insert(t) {
                        kept.push(t);
                        queue.push(t);
                    }
                }
            }
        }
        kept.sort_unstable();
        edges.sort_unstable();
        edges.dedup();
        Ok((kept, edges))
    }
}

// ── Closing the role extensions ─────────────────────────────────────────

/// Add every edge the role axioms force: super-roles, inverses (a symmetric role
/// is its own inverse here) and transitive closure.
///
/// The completion graph carries only the edges the tableau rules created, and
/// `Tableau::successors` reads the rest off the role hierarchy and the parent
/// back-link as it goes. A finite interpretation has to carry them, because
/// `subrole`, `inv`, `sym` and `trans` are axioms the checker will evaluate.
///
/// The cap is not a nicety. Closing a transitive role over a completion graph
/// can square the edge count, and a certificate nobody can read is no better
/// than none.
fn close_roles(
    src: &AxiomSource,
    raw: &[(u32, u32, u32)],
    cap: usize,
) -> Result<HashSet<(u32, u32, u32)>, String> {
    let mut super_of: HashMap<u32, HashSet<u32>> = HashMap::new();
    for (&sub, sups) in &src.sub_to_super {
        let mut seen: HashSet<u32> = HashSet::new();
        let mut stack: Vec<u32> = sups.iter().copied().collect();
        while let Some(x) = stack.pop() {
            if seen.insert(x)
                && let Some(next) = src.sub_to_super.get(&x)
            {
                stack.extend(next.iter().copied());
            }
        }
        seen.remove(&sub);
        super_of.insert(sub, seen);
    }

    let mut all: HashSet<(u32, u32, u32)> = raw.iter().copied().collect();
    let mut work: Vec<(u32, u32, u32)> = all.iter().copied().collect();
    while let Some((x, r, y)) = work.pop() {
        if all.len() > cap {
            return Err(format!(
                "closing the role extensions passed {cap} edges; no finite interpretation \
                 small enough to certify was produced"
            ));
        }
        let mut add: Vec<(u32, u32, u32)> = Vec::new();
        if let Some(sups) = super_of.get(&r) {
            for &s in sups {
                add.push((x, s, y));
            }
        }
        if let Some(&inv) = src.inverse_roles.get(&r) {
            add.push((y, inv, x));
        }
        if src.transitive_roles.contains(&r) {
            for &(a, rr, b) in all.iter() {
                if rr != r {
                    continue;
                }
                if b == x {
                    add.push((a, r, y));
                }
                if a == y {
                    add.push((x, r, b));
                }
            }
        }
        for e in add {
            if all.insert(e) {
                work.push(e);
            }
        }
    }
    Ok(all)
}

// ── The public entry points ─────────────────────────────────────────────

impl DlReasoner {
    /// The TBox axioms, as the OWL parser read them, in the `lean/Dl` fragment.
    fn tbox_axioms(&self) -> Vec<DlAxiom> {
        let s = &self.source;
        let mut out: Vec<DlAxiom> = Vec::new();
        for (sub, sup) in &s.axioms {
            out.push(DlAxiom::Sub(sub.clone(), sup.clone()));
        }
        for (a, b) in &s.disjoint_pairs {
            out.push(DlAxiom::Disjoint(a.clone(), b.clone()));
        }
        for (r, c) in &s.role_domains {
            out.push(DlAxiom::Domain(*r, c.clone()));
        }
        for (r, c) in &s.role_ranges {
            out.push(DlAxiom::Range(*r, c.clone()));
        }
        let mut subrole: Vec<(u32, u32)> = Vec::new();
        for (&sub, sups) in &s.sub_to_super {
            for &sup in sups {
                if sub != sup {
                    subrole.push((sub, sup));
                }
            }
        }
        subrole.sort_unstable();
        for (sub, sup) in subrole {
            out.push(DlAxiom::SubRole(sub, sup));
        }
        let mut trans: Vec<u32> = s.transitive_roles.iter().copied().collect();
        trans.sort_unstable();
        for r in trans {
            out.push(DlAxiom::Trans(r));
        }
        // `inverse_roles` holds both directions, and a symmetric role appears as
        // its own inverse. Emit each pair once, and a symmetric role as `sym`,
        // which is the same condition stated in the form a reader expects.
        let mut invs: Vec<(u32, u32)> = s.inverse_roles.iter().map(|(&a, &b)| (a, b)).collect();
        invs.sort_unstable();
        for (a, b) in invs {
            if a == b {
                out.push(DlAxiom::Sym(a));
            } else if a < b {
                out.push(DlAxiom::Inv(a, b));
            }
        }
        // A functional role is exactly `⊤ ⊑ ≤1 R.⊤`, so it needs no constructor
        // of its own. Inverse functionality does: `≤1 R⁻.⊤` is not expressible
        // as a concept unless `R` happens to have a named inverse.
        let mut func: Vec<u32> = s.functional_roles.iter().copied().collect();
        func.sort_unstable();
        for r in func {
            out.push(DlAxiom::Sub(
                Concept::Top,
                Concept::MaxCard(r, 1, Box::new(Concept::Top)),
            ));
        }
        let mut invfunc: Vec<u32> = s.inv_functional_roles.iter().copied().collect();
        invfunc.sort_unstable();
        for r in invfunc {
            out.push(DlAxiom::InvFunc(r));
        }
        out
    }

    /// The TBox axioms plus the ABox: class assertions, role assertions, and one
    /// `indiv` line per named individual so that a model is forced to contain it.
    fn abox_axioms(&self) -> Vec<DlAxiom> {
        let mut out = self.tbox_axioms();
        let mut inds: Vec<u32> = self.individual_types.keys().copied().collect();
        for &i in self.individual_anon_types.keys() {
            inds.push(i);
        }
        for &(a, _, b) in &self.role_assertions {
            inds.push(a);
            inds.push(b);
        }
        inds.sort_unstable();
        inds.dedup();
        for i in inds {
            out.push(DlAxiom::Indiv(i));
        }
        let mut typed: Vec<(u32, Vec<u32>)> = self
            .individual_types
            .iter()
            .map(|(&i, ts)| {
                let mut v: Vec<u32> = ts.iter().copied().collect();
                v.sort_unstable();
                (i, v)
            })
            .collect();
        typed.sort_unstable();
        for (i, ts) in typed {
            for t in ts {
                out.push(DlAxiom::Inst(i, Concept::Atom(t)));
            }
        }
        let mut anon: Vec<(u32, Vec<Concept>)> = self
            .individual_anon_types
            .iter()
            .map(|(&i, cs)| (i, cs.clone()))
            .collect();
        anon.sort_unstable_by_key(|(i, _)| *i);
        for (i, cs) in anon {
            for c in cs {
                out.push(DlAxiom::Inst(i, c));
            }
        }
        let mut rels: Vec<(u32, u32, u32)> = self.role_assertions.clone();
        rels.sort_unstable();
        rels.dedup();
        for (a, r, b) in rels {
            out.push(DlAxiom::Rel(a, r, b));
        }
        out
    }

    /// Build the finite interpretation, check it here, and write the two files.
    /// Nothing is written unless the check passes.
    fn emit(
        &self,
        tableau: &Tableau,
        ind_to_node: &HashMap<u32, u32>,
        axioms: Vec<DlAxiom>,
        dir: &Path,
    ) -> anyhow::Result<ModelOutcome> {
        let (kept, raw_edges) = match tableau.folded_graph() {
            Ok(v) => v,
            Err(e) => return Ok(ModelOutcome::Refused(e)),
        };
        if kept.is_empty() {
            return Ok(ModelOutcome::Refused(
                "the completion graph is empty, so there is no interpretation to hand over"
                    .to_string(),
            ));
        }

        let domset: HashSet<u32> = kept.iter().copied().collect();
        let closed = match close_roles(&self.source, &raw_edges, 200_000) {
            Ok(v) => v,
            Err(e) => return Ok(ModelOutcome::Refused(e)),
        };

        let mut rext: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
        for (x, r, y) in closed {
            if !domset.contains(&x) || !domset.contains(&y) {
                return Ok(ModelOutcome::Refused(format!(
                    "closing the role extensions produced an edge outside the folded domain \
                     (n{x} to n{y}); the completion graph does not fold into a finite model"
                )));
            }
            rext.entry((r, x)).or_default().push(y);
        }
        for v in rext.values_mut() {
            v.sort_unstable();
            v.dedup();
        }

        let mut cext: HashMap<u32, HashSet<u32>> = HashMap::new();
        for &n in &kept {
            let Some(node) = tableau.nodes.get(&n) else {
                return Ok(ModelOutcome::Refused(format!("node {n} vanished")));
            };
            for label in &node.labels {
                if let Concept::Atom(a) = label {
                    cext.entry(*a).or_default().insert(n);
                }
            }
        }

        let mut ind: HashMap<u32, u32> = HashMap::new();
        for (&i, &n) in ind_to_node {
            // An individual whose node was folded away denotes the node it was
            // folded into. Roots are never blocked, and every individual is a
            // root, so in practice this is the identity; it is written out
            // rather than assumed.
            let mut cur = n;
            let mut steps = 0usize;
            while let Some(b) = tableau.blocker_of(cur) {
                cur = b;
                steps += 1;
                if steps > tableau.nodes.len() + 1 {
                    return Ok(ModelOutcome::Refused(
                        "a blocking chain under a named individual did not terminate".to_string(),
                    ));
                }
            }
            if !domset.contains(&cur) {
                return Ok(ModelOutcome::Refused(format!(
                    "individual {} has no node in the folded domain",
                    self.interner.resolve(i)
                )));
            }
            ind.insert(i, cur);
        }

        let model = FiniteModel {
            dom: kept.clone(),
            domset,
            cext,
            rext,
            ind,
        };

        // Every name that appears in either file has to survive the round trip.
        // `owl:hasValue` is approximated by an atom named after the individual,
        // and that individual can be a literal with a space in it.
        //
        // The check is AT THE POINT OF WRITING. It used to be a separate walk
        // over every axiom variant plus `model.rext` and `model.cext`, with
        // individuals in `model.ind` covered only because `DlAxiom::Indiv(i)`
        // happens to be emitted for every individual that reaches the model:
        // TCB-26, an argument about two loops agreeing rather than a check.
        // Building the text first and letting `push_name` record the first
        // refusal makes the coverage question disappear, because `push_name` is
        // the only way a name reaches either buffer.
        let mut bad: Option<String> = None;

        let mut axiom_text = String::new();
        for a in &axioms {
            axiom_text.push_str(&axiom_line(&self.interner, a, &mut bad));
            axiom_text.push('\n');
        }

        let mut model_text = String::new();
        for &n in &model.dom {
            model_text.push_str(&format!("domain\tn{n}\n"));
        }
        let mut classes: Vec<(u32, Vec<u32>)> = model
            .cext
            .iter()
            .map(|(&c, ms)| {
                let mut v: Vec<u32> = ms.iter().copied().collect();
                v.sort_unstable();
                (c, v)
            })
            .collect();
        classes.sort_unstable();
        for (c, ms) in classes {
            for m in ms {
                model_text.push_str("class\t");
                push_name(&mut model_text, &self.interner, c, &mut bad);
                model_text.push_str(&format!("\tn{m}\n"));
            }
        }
        let mut edge_lines: Vec<(u32, u32, u32)> = Vec::new();
        for (&(r, x), ys) in &model.rext {
            for &y in ys {
                edge_lines.push((x, r, y));
            }
        }
        edge_lines.sort_unstable();
        let edge_count = edge_lines.len();
        for (x, r, y) in edge_lines {
            model_text.push_str(&format!("edge\tn{x}\t"));
            push_name(&mut model_text, &self.interner, r, &mut bad);
            model_text.push_str(&format!("\tn{y}\n"));
        }
        let mut ind_lines: Vec<(u32, u32)> = model.ind.iter().map(|(&i, &n)| (i, n)).collect();
        ind_lines.sort_unstable();
        for (i, n) in ind_lines {
            model_text.push_str("ind\t");
            push_name(&mut model_text, &self.interner, i, &mut bad);
            model_text.push_str(&format!("\tn{n}\n"));
        }

        // Nothing has been written yet, so a refusal here writes nothing. The
        // order is the order it always was: the name guard decides before the
        // semantics gate does.
        if let Some(bad) = bad {
            return Ok(ModelOutcome::Refused(format!(
                "the name {bad:?} carries whitespace, so it would not survive the \
                 tab-and-space separated format; nothing was written"
            )));
        }

        // The gate. A certificate that will not check is worse than no
        // certificate, so the emitter runs the same semantics the checker does
        // and refuses when it does not hold.
        if let Err(why) = model.well_formed(&axioms) {
            return Ok(ModelOutcome::Refused(format!(
                "the folded completion graph is not a finite interpretation: {why}"
            )));
        }
        if let Some(bad) = axioms.iter().find(|a| !model.holds(a)) {
            return Ok(ModelOutcome::Refused(format!(
                "the folded completion graph does not satisfy the axiom `{}`",
                axiom_line(&self.interner, bad, &mut None).replace('\t', " ")
            )));
        }

        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join("axioms.tsv"), axiom_text)?;
        std::fs::write(dir.join("model.tsv"), model_text)?;

        Ok(ModelOutcome::Certified {
            axioms: axioms.len(),
            domain: model.dom.len(),
            edges: edge_count,
        })
    }

    /// Certify that `class_iri` is satisfiable with respect to the TBox.
    ///
    /// The claim is written into the axiom set as `nonempty`, so a checker that
    /// accepts the certificate has verified the reasoner's actual answer and not
    /// merely that the TBox has some model. The ABox is NOT part of this
    /// certificate: classification in this reasoner is a TBox question, the
    /// completion graph has no named individuals in it, and pretending otherwise
    /// would produce a certificate that cannot check.
    pub fn certify_class_satisfiable(
        &self,
        class_iri: &str,
        dir: &Path,
    ) -> anyhow::Result<ModelOutcome> {
        let Some(&cid) = self.interner.to_id.get(class_iri) else {
            anyhow::bail!("no class named {class_iri} in this ontology");
        };
        let concept = Concept::Atom(cid);
        let mut tableau = Tableau::with_deadline(Arc::clone(&self.tbox), self.phase_deadline());
        tableau.capture = true;
        match tableau.decide(&concept) {
            Verdict::Unsatisfiable => return Ok(ModelOutcome::Refuted),
            Verdict::Unknown => return Ok(ModelOutcome::Undetermined),
            Verdict::Satisfiable => {}
        }
        let mut axioms = self.tbox_axioms();
        axioms.push(DlAxiom::NonEmpty(concept));
        self.emit(&tableau, &HashMap::new(), axioms, dir)
    }

    /// The internal id of a named class, by IRI, for callers that hold a name
    /// and need the id `explain_unsatisfiable` takes. Matches the spelling
    /// `named_class_names` returns, angle brackets and all, and also the bare
    /// IRI, because a caller that has one rarely has the other.
    pub fn named_class_id(&self, iri: &str) -> Option<u32> {
        let bare = iri.trim_start_matches('<').trim_end_matches('>');
        self.named_classes
            .iter()
            .copied()
            .find(|&id| self.interner.resolve(id).trim_start_matches('<').trim_end_matches('>') == bare)
    }

    /// The named classes, in the interner's order, so a caller can certify each
    /// one. `run` uses it because TBox consistency alone is witnessed by a
    /// single point with empty extensions, which is honest and uninformative.
    pub fn named_class_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .named_classes
            .iter()
            .map(|id| self.interner.resolve(*id).to_string())
            .collect();
        v.sort();
        v
    }

    /// The OWL constructs present in the graph that this certificate layer does
    /// not model, with a count of each.
    ///
    /// This matters more than it looks. The emitter certifies that a finite
    /// interpretation satisfies THE AXIOMS IT EMITTED, and a construct the
    /// parser does not recognise is absent from both sides. So an ontology
    /// leaning on nominals, role chains, `owl:sameAs` or datatypes gets a
    /// perfectly valid certificate about a WEAKER axiom set than the one it
    /// actually states, and a reader who is not told that will draw a stronger
    /// conclusion than the proof supports. Naming them in a source comment is
    /// not enough: it has to be in the report, next to the verdict.
    pub fn unmodelled_constructs(graph: &Arc<GraphStore>) -> Vec<(String, u64)> {
        // `owl:AsymmetricProperty` sits here rather than in the tableau, and the
        // choice is worth stating. Asymmetry is a constraint on a PAIR of edges
        // (`a r b` forbids `b r a`), not a concept membership, so unlike every
        // characteristic the tableau does model it cannot be expressed as a label
        // on a node in SHIQ without nominals — the ALCOIQ encoding needs `{a}` to
        // say "the r-successor that is this individual". Modelling it would mean
        // a new edge-level clash rule and a new termination argument. Declaring it
        // costs a line and is honest, which is what this list is for. Until it was
        // added here it was in NEITHER place: not implemented, and not declared,
        // so an ontology leaning on asymmetry got a clean verdict with no sign
        // that a constraint had been dropped. That is the one outcome this list
        // exists to prevent.
        const NOT_MODELLED: [(&str, &str); 9] = [
            ("owl:sameAs", "http://www.w3.org/2002/07/owl#sameAs"),
            ("owl:differentFrom", "http://www.w3.org/2002/07/owl#differentFrom"),
            ("owl:oneOf", "http://www.w3.org/2002/07/owl#oneOf"),
            ("owl:hasValue", "http://www.w3.org/2002/07/owl#hasValue"),
            ("owl:propertyChainAxiom", "http://www.w3.org/2002/07/owl#propertyChainAxiom"),
            ("owl:hasKey", "http://www.w3.org/2002/07/owl#hasKey"),
            ("owl:ReflexiveProperty", "http://www.w3.org/2002/07/owl#ReflexiveProperty"),
            ("owl:IrreflexiveProperty", "http://www.w3.org/2002/07/owl#IrreflexiveProperty"),
            ("owl:AsymmetricProperty", "http://www.w3.org/2002/07/owl#AsymmetricProperty"),
        ];
        let mut found = Vec::new();
        for (label, iri) in NOT_MODELLED {
            let q = format!(
                "SELECT (COUNT(*) AS ?n) WHERE {{ {{ ?s <{iri}> ?o }} UNION {{ ?s ?p <{iri}> }} }}"
            );
            let Ok(raw) = graph.sparql_select_union(&q) else { continue };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else { continue };
            let n = v["results"][0]["n"]
                .as_str()
                .and_then(|s| s.trim_matches('"').split('"').next())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            if n > 0 {
                found.push((label.to_string(), n));
            }
        }
        found
    }

    /// Certify that the TBox is consistent.
    ///
    /// No `nonempty` line is needed: `Dl.WellFormed` already requires the domain
    /// to be non-empty, so a model of the axiom set alone is exactly what TBox
    /// consistency asserts.
    ///
    /// This paragraph was stranded: it sat above `named_class_names`, four
    /// hundred lines from the function it describes and joined to that
    /// function's own doc comment with no blank line, so `cargo doc` printed a
    /// block about certification to a reader of a method that returns a list of
    /// names, and this method was documented nowhere. It is back where it
    /// belongs.
    pub fn certify_tbox_consistent(&self, dir: &Path) -> anyhow::Result<ModelOutcome> {
        let mut tableau = Tableau::with_deadline(Arc::clone(&self.tbox), self.phase_deadline());
        tableau.capture = true;
        match tableau.decide(&Concept::Top) {
            Verdict::Unsatisfiable => return Ok(ModelOutcome::Refuted),
            Verdict::Unknown => return Ok(ModelOutcome::Undetermined),
            Verdict::Satisfiable => {}
        }
        self.emit(&tableau, &HashMap::new(), self.tbox_axioms(), dir)
    }

    /// Certify that the ABox is consistent with the TBox.
    pub fn certify_abox_consistent(&self, dir: &Path) -> anyhow::Result<ModelOutcome> {
        // Same three-part test as `check_abox`, and it has to be the same or the
        // certificate layer silently declines to certify an ABox the reasoner
        // decided. `abox_axioms` already emits one `indiv` line per role-assertion
        // endpoint, so refusing here left those axioms with nothing to certify.
        if self.individual_types.is_empty()
            && self.individual_anon_types.is_empty()
            && self.role_assertions.is_empty()
        {
            return Ok(ModelOutcome::Refused(
                "this ontology has no ABox, so there is nothing to certify beyond the TBox"
                    .to_string(),
            ));
        }
        let (mut tableau, ind_to_node, _) = self.build_abox_tableau(Self::global_deadline());
        tableau.capture = true;
        if !tableau.expand(0) {
            return Ok(if tableau.budget.exhausted {
                ModelOutcome::Undetermined
            } else {
                ModelOutcome::Refuted
            });
        }
        self.emit(&tableau, &ind_to_node, self.abox_axioms(), dir)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The model certificate's serialisation boundary
//
// `axioms.tsv` is tab separated at the top level and SPACE separated inside a
// concept, and it carries IRIs and literals. `docs/trusted-computing-base.md`
// calls that TCB-25 and TCB-26: `name_is_safe` is the only explicit injection
// guard in this codebase, and the argument that it covers every name that gets
// written is an argument about two loops agreeing rather than a check.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod certificate_boundary_tests {
    use super::*;
    use proptest::prelude::*;

    /// Names drawn from exactly the two sides of `name_is_safe`: some that must
    /// pass, some that must not, and the grammar's own keywords, which must be
    /// harmless because the encoding is prefix and positional rather than
    /// delimited.
    fn name() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("<http://e/A>".to_string()),
            Just("\"lit\"".to_string()),
            Just("_:b0".to_string()),
            // Keywords of the concept grammar.
            Just("atom".to_string()),
            Just("top".to_string()),
            Just("not".to_string()),
            Just("some".to_string()),
            Just("min".to_string()),
            Just("2".to_string()),
            // Must be refused.
            Just("a b".to_string()),
            Just("a\tb".to_string()),
            Just("a\nb".to_string()),
            Just("a\rb".to_string()),
            Just(String::new()),
            "[^ \t\n\r]{1,5}",
        ]
    }

    /// A concept over name SLOTS rather than interned ids, so the shape can be
    /// generated once and instantiated against whatever names the case drew.
    #[derive(Clone, Debug)]
    enum Shape {
        Top,
        Bot,
        Atom(usize),
        NegAtom(usize),
        And(Vec<Shape>),
        Or(Vec<Shape>),
        Exists(usize, Box<Shape>),
        ForAll(usize, Box<Shape>),
        Min(usize, u32, Box<Shape>),
        Max(usize, u32, Box<Shape>),
    }

    fn shape() -> impl Strategy<Value = Shape> {
        let leaf = prop_oneof![
            Just(Shape::Top),
            Just(Shape::Bot),
            (0usize..6).prop_map(Shape::Atom),
            (0usize..6).prop_map(Shape::NegAtom),
        ];
        leaf.prop_recursive(3, 12, 3, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..3).prop_map(Shape::And),
                prop::collection::vec(inner.clone(), 0..3).prop_map(Shape::Or),
                (0usize..6, inner.clone()).prop_map(|(r, f)| Shape::Exists(r, Box::new(f))),
                (0usize..6, inner.clone()).prop_map(|(r, f)| Shape::ForAll(r, Box::new(f))),
                (0usize..6, 0u32..3, inner.clone())
                    .prop_map(|(r, n, f)| Shape::Min(r, n, Box::new(f))),
                (0usize..6, 0u32..3, inner).prop_map(|(r, n, f)| Shape::Max(r, n, Box::new(f))),
            ]
        })
    }

    fn build(s: &Shape, ids: &[u32]) -> Concept {
        let pick = |i: usize| ids[i % ids.len()];
        match s {
            Shape::Top => Concept::Top,
            Shape::Bot => Concept::Bottom,
            Shape::Atom(i) => Concept::Atom(pick(*i)),
            Shape::NegAtom(i) => Concept::NegAtom(pick(*i)),
            Shape::And(cs) => Concept::And(cs.iter().map(|c| build(c, ids)).collect()),
            Shape::Or(cs) => Concept::Or(cs.iter().map(|c| build(c, ids)).collect()),
            Shape::Exists(r, f) => Concept::Exists(pick(*r), Box::new(build(f, ids))),
            Shape::ForAll(r, f) => Concept::ForAll(pick(*r), Box::new(build(f, ids))),
            Shape::Min(r, n, f) => Concept::MinCard(pick(*r), *n, Box::new(build(f, ids))),
            Shape::Max(r, n, f) => Concept::MaxCard(pick(*r), *n, Box::new(build(f, ids))),
        }
    }

    /// How many space-separated tokens the encoding in `write_concept` must
    /// produce for a concept, derived from the shape alone. Written out here so
    /// it is an independent statement of the grammar rather than a second call
    /// to the same code.
    fn tokens(c: &Concept) -> usize {
        match c {
            Concept::Top | Concept::Bottom => 1,
            Concept::Atom(_) => 2,
            Concept::NegAtom(_) => 3,
            Concept::And(cs) => nary_tokens(cs, &Concept::Top),
            Concept::Or(cs) => nary_tokens(cs, &Concept::Bottom),
            Concept::Exists(_, f) | Concept::ForAll(_, f) => 2 + tokens(f),
            Concept::MinCard(_, _, f) | Concept::MaxCard(_, _, f) => 3 + tokens(f),
        }
    }

    fn nary_tokens(cs: &[Concept], unit: &Concept) -> usize {
        match cs.split_first() {
            None => tokens(unit),
            Some((head, [])) => tokens(head),
            Some((head, rest)) => 1 + tokens(head) + nary_tokens(rest, unit),
        }
    }

    /// TCB-26, as a statement about this file rather than about a run.
    ///
    /// The guard covers every name that is written because `push_name` is the
    /// only thing that writes one. This reads the source of the two writers and
    /// requires it: no `interner.resolve` may append to a certificate buffer.
    /// It is a crude check and it is the one that matches the claim. The
    /// previous arrangement — a `names` vector built by one loop and the files
    /// written by another — could not be checked this way at all, which is why
    /// TCB-26 was listed as trusted rather than tested.
    #[test]
    fn tcb_26_only_push_name_writes_a_name() {
        // Normalise line endings before any of the anchors below are matched.
        // git on Windows checks this file out with CRLF by default, and the
        // `fn emit(\n` anchor wants a newline immediately after the paren, so
        // without this the anchor misses, `expect` fires, and the test fails on
        // Windows alone while passing on Linux and macOS. It did exactly that.
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/tableaux.rs"))
            .expect("this file is readable")
            .replace("\r\n", "\n");

        // The two regions that build certificate text: the serialisers, and the
        // emitter that assembles `axioms.tsv` and `model.tsv` out of them.
        let ser_start = src.find("// ── Serialisation ──").expect("the serialisation section");
        let ser_end = src[ser_start..]
            .find("\n// ── The finite interpretation")
            .expect("the section after it")
            + ser_start;
        let emit_start = src.find("    fn emit(\n").expect("the emitter");
        let emit_end = src[emit_start..]
            .find("\n    /// Certify that `class_iri` is satisfiable")
            .expect("the method after it")
            + emit_start;
        let regions = [&src[ser_start..ser_end], &src[emit_start..emit_end]];
        assert!(regions[0].contains("fn push_name("), "the guard left the serialisers");
        assert!(regions[1].contains("axioms.tsv"), "the emitter region is not the emitter");

        let mut offenders: Vec<&str> = Vec::new();
        for region in regions {
            for line in region.lines() {
                let t = line.trim();
                if !t.contains("resolve(") || t.starts_with("//") {
                    continue;
                }
                // The one inside `push_name`, and the one that names an
                // individual in an ERROR MESSAGE rather than in a line.
                if t == "let s = interner.resolve(id);" || t == "self.interner.resolve(i)" {
                    continue;
                }
                offenders.push(line);
            }
        }
        assert!(
            offenders.is_empty(),
            "a name reaches a certificate buffer without passing `push_name`:\n{}",
            offenders.join("\n")
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

        /// TCB-25. `name_is_safe` refuses exactly the names that would break the
        /// format: empty, or carrying a space, tab, carriage return or newline.
        #[test]
        fn tcb_25_name_is_safe_refuses_every_separator(n in name()) {
            let safe = name_is_safe(&n);
            prop_assert_eq!(
                safe,
                !n.is_empty() && !n.contains([' ', '\t', '\n', '\r'])
            );
        }

        /// TCB-25. Given names the guard accepts, a concept serialises to
        /// exactly the number of space-separated tokens its shape determines.
        ///
        /// This is the property that makes the encoding readable back: the
        /// grammar is prefix and positional, so a name that happens to be a
        /// keyword (`atom`, `top`, `min`) is harmless, and a name carrying a
        /// space is not. If the guard ever stopped refusing one, the token count
        /// would drift from the shape and the checker would read a different
        /// concept from the one the reasoner decided about.
        #[test]
        fn tcb_25_a_safe_concept_serialises_to_the_tokens_its_shape_demands(
            ns in prop::collection::vec(name(), 1..6),
            sh in shape(),
        ) {
            let mut interner = Interner::new();
            let ids: Vec<u32> = ns.iter().map(|n| interner.intern(n)).collect();
            let c = build(&sh, &ids);
            let mut bad = None;
            let s = concept_string(&interner, &c, &mut bad);
            // The condition is now the GUARD's verdict, not the test author's
            // recollection of which names the concept happens to mention. That
            // is TCB-26: `push_name` is the only way a name reaches the buffer,
            // so `bad` is set exactly when the buffer carries an unsafe one.
            if bad.is_none() {
                prop_assert_eq!(
                    s.split(' ').count(), tokens(&c),
                    "token count drifted from the shape: {:?}", s
                );
                prop_assert!(!s.contains('\t') && !s.contains('\n') && !s.contains('\r'));
            } else {
                prop_assert!(
                    ns.iter().any(|n| !name_is_safe(n)),
                    "the guard refused {:?} but every name is safe", s
                );
            }
        }

        /// TCB-25 for `axiom_line`. Every axiom is a tab-separated record whose
        /// field count its variant fixes, so a name carrying a tab would add a
        /// field and the checker would read a different axiom.
        #[test]
        fn tcb_25_an_axiom_line_has_the_fields_its_variant_fixes(n in name()) {
            let mut interner = Interner::new();
            let id = interner.intern(&n);
            let c = Concept::Atom(id);
            let cases: Vec<(DlAxiom, usize)> = vec![
                (DlAxiom::Sub(c.clone(), c.clone()), 3),
                (DlAxiom::Disjoint(c.clone(), c.clone()), 3),
                (DlAxiom::Domain(id, c.clone()), 3),
                (DlAxiom::Range(id, c.clone()), 3),
                (DlAxiom::SubRole(id, id), 3),
                (DlAxiom::Trans(id), 2),
                (DlAxiom::Sym(id), 2),
                (DlAxiom::Inv(id, id), 3),
                (DlAxiom::InvFunc(id), 2),
                (DlAxiom::Inst(id, c.clone()), 3),
                (DlAxiom::Rel(id, id, id), 4),
                (DlAxiom::Indiv(id), 2),
                (DlAxiom::NonEmpty(c), 2),
            ];
            for (a, want) in cases {
                let mut bad = None;
                let line = axiom_line(&interner, &a, &mut bad);
                // Every one of these variants mentions the generated name, so
                // the guard's verdict and the name's safety have to agree. That
                // is the TCB-26 claim: no variant writes a name the guard did
                // not see, and none reports one it did not write.
                prop_assert_eq!(bad.is_none(), name_is_safe(&n), "{:?}", line);
                if bad.is_none() {
                    prop_assert_eq!(
                        line.split('\t').count(), want,
                        "{:?} is not {} tab-separated fields", line, want
                    );
                    prop_assert!(!line.contains('\n') && !line.contains('\r'));
                }
            }
        }
    }
}
