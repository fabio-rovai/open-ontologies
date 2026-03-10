//! OWL2-DL Tableaux Reasoner
//!
//! Implements a tableau-based decision procedure for ALCH description logic:
//! - Atomic concepts and negation (complement)
//! - Conjunction (intersectionOf) and Disjunction (unionOf)
//! - Existential restriction (someValuesFrom)
//! - Universal restriction (allValuesFrom)
//! - Role hierarchy (subPropertyOf) and transitive roles
//!
//! Provides: satisfiability testing, subsumption, classification,
//! consistency checking, unsatisfiable class detection.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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

const MAX_DEPTH: usize = 100;
const MAX_NODES: usize = 10_000;

// ── Concept (Negation Normal Form) ──────────────────────────────────────

/// Description Logic concept in NNF. All negations pushed to atomic level.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Concept {
    Top,
    Bottom,
    Atom(u32),
    NegAtom(u32),
    And(Vec<Concept>),
    Or(Vec<Concept>),
    Exists(u32, Box<Concept>),
    ForAll(u32, Box<Concept>),
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
        }
    }
}

// ── String Interner ─────────────────────────────────────────────────────

struct Interner {
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

    fn resolve(&self, id: u32) -> &str {
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

        // Collect class declarations
        let class_subjects: Vec<String> = self
            .index
            .by_subject
            .iter()
            .filter(|(_, pairs)| pairs.iter().any(|(p, o)| p == RDF_TYPE && o == OWL_CLASS))
            .map(|(s, _)| s.clone())
            .collect();
        for s in &class_subjects {
            let id = self.interner.intern(s);
            named_classes.insert(id);
        }

        // Collect transitive roles
        let trans_subjects: Vec<String> = self
            .index
            .by_subject
            .iter()
            .filter(|(_, pairs)| pairs.iter().any(|(p, o)| p == RDF_TYPE && o == OWL_TRANSITIVE))
            .map(|(s, _)| s.clone())
            .collect();
        for s in trans_subjects {
            transitive_roles.insert(self.interner.intern(&s));
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

        // someValuesFrom
        if let Some(filler) = self.index.object(node, OWL_SOME_VALUES) {
            return RawConcept::Exists(prop, Box::new(self.parse_class_expr(&filler)));
        }
        // allValuesFrom
        if let Some(filler) = self.index.object(node, OWL_ALL_VALUES) {
            return RawConcept::ForAll(prop, Box::new(self.parse_class_expr(&filler)));
        }
        // hasValue (approximated as ∃R.{a})
        if let Some(value) = self.index.object(node, OWL_HAS_VALUE) {
            let val_id = self.interner.intern(&value);
            return RawConcept::Exists(prop, Box::new(RawConcept::Named(val_id)));
        }

        RawConcept::Top
    }
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
}

impl ProcessedTBox {
    fn new(
        axioms: &[(Concept, Concept)],
        disjoint_pairs: &[(Concept, Concept)],
        transitive_roles: HashSet<u32>,
        sub_to_super: &HashMap<u32, HashSet<u32>>,
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

        // Compute super_to_sub from sub_to_super
        let mut super_to_sub: HashMap<u32, HashSet<u32>> = HashMap::new();
        for (&sub, supers) in sub_to_super {
            for &sup in supers {
                super_to_sub.entry(sup).or_default().insert(sub);
            }
        }

        Self {
            concept_defs,
            gcis,
            disjoint_pairs: disjoint_pairs.to_vec(),
            transitive_roles,
            super_to_sub,
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
    blocked: bool,
}

impl TNode {
    fn new(parent: Option<u32>) -> Self {
        Self {
            labels: HashSet::new(),
            processed: HashSet::new(),
            edges: HashMap::new(),
            parent,
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
        false
    }
}

// ── Tableau ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Tableau {
    nodes: HashMap<u32, TNode>,
    next_id: u32,
    tbox: Arc<ProcessedTBox>,
}

impl Tableau {
    fn new(tbox: Arc<ProcessedTBox>) -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: 0,
            tbox,
        }
    }

    fn is_satisfiable(&mut self, concept: &Concept) -> bool {
        let root = self.fresh_node(None);
        self.add_label(root, concept.clone());
        // Add GCIs to root
        for gci in self.tbox.gcis.clone() {
            self.add_label(root, gci);
        }
        self.expand(0)
    }

    fn fresh_node(&mut self, parent: Option<u32>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(id, TNode::new(parent));
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

    /// Get successors via a role, considering sub-roles.
    fn successors(&self, node_id: u32, role: u32) -> HashSet<u32> {
        let node = &self.nodes[&node_id];
        let mut result = HashSet::new();
        if let Some(succs) = node.edges.get(&role) {
            result.extend(succs);
        }
        if let Some(sub_roles) = self.tbox.super_to_sub.get(&role) {
            for &sub in sub_roles {
                if let Some(succs) = node.edges.get(&sub) {
                    result.extend(succs);
                }
            }
        }
        result
    }

    /// Main expansion with backtracking for disjunctions.
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
                if self.nodes[&nid].blocked {
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
                        // ∃-rule: create successor if needed
                        Concept::Exists(role, filler) => {
                            let role = *role;
                            let filler = *filler.clone();
                            let succs = self.successors(nid, role);
                            let has_matching =
                                succs.iter().any(|&s| self.nodes[&s].labels.contains(&filler));
                            if !has_matching {
                                let succ = self.fresh_node(Some(nid));
                                self.add_label(succ, filler);
                                // Add GCIs to new node
                                for gci in self.tbox.gcis.clone() {
                                    self.add_label(succ, gci);
                                }
                                // Propagate ∀ labels from parent to new successor
                                let parent_foralls: Vec<(u32, Concept)> = self.nodes[&nid]
                                    .labels
                                    .iter()
                                    .filter_map(|l| match l {
                                        Concept::ForAll(r, f) => Some((*r, *f.clone())),
                                        _ => None,
                                    })
                                    .collect();
                                for (r, f) in parent_foralls {
                                    if r == role {
                                        self.add_label(succ, f.clone());
                                        if self.tbox.transitive_roles.contains(&r) {
                                            self.add_label(
                                                succ,
                                                Concept::ForAll(r, Box::new(f)),
                                            );
                                        }
                                    }
                                }
                                self.nodes
                                    .get_mut(&nid)
                                    .unwrap()
                                    .edges
                                    .entry(role)
                                    .or_default()
                                    .insert(succ);
                                changed = true;
                            }
                            self.nodes.get_mut(&nid).unwrap().processed.insert(label);
                        }
                        // ∀-rule: apply filler to all successors
                        Concept::ForAll(role, filler) => {
                            let role = *role;
                            let filler = *filler.clone();
                            let is_transitive = self.tbox.transitive_roles.contains(&role);
                            let succs = self.successors(nid, role);
                            for s in succs {
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
                        // ⊔-rule: handled below (non-deterministic)
                        Concept::Or(_) => {}
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

        // Find unprocessed disjunction → branch with backtracking
        let node_ids: Vec<u32> = self.nodes.keys().copied().collect();
        for &nid in &node_ids {
            if self.nodes[&nid].blocked {
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
            // Only block non-root nodes
            if self.nodes[&nid].parent.is_none() {
                continue;
            }
            let node_labels = self.nodes[&nid].labels.clone();
            let mut ancestor = self.nodes[&nid].parent;
            let mut found = false;
            while let Some(anc_id) = ancestor {
                if node_labels.is_subset(&self.nodes[&anc_id].labels) {
                    found = true;
                    break;
                }
                ancestor = self.nodes[&anc_id].parent;
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
        ));

        Ok(Self {
            interner: result.interner,
            tbox,
            named_classes: result.named_classes,
            thing_id: result.thing_id,
            nothing_id: result.nothing_id,
        })
    }

    pub fn is_satisfiable(&self, concept: &Concept) -> bool {
        let mut tableau = Tableau::new(Arc::clone(&self.tbox));
        tableau.is_satisfiable(concept)
    }

    pub fn is_subsumed(&self, sub: &Concept, sup: &Concept) -> bool {
        let mut test = vec![sub.clone(), sup.negate()];
        test.sort();
        let test_concept = Concept::And(test);
        !self.is_satisfiable(&test_concept)
    }

    pub fn is_consistent(&self) -> bool {
        self.is_satisfiable(&Concept::Top)
    }

    pub fn classify(&self) -> ClassificationResult {
        let classes: Vec<u32> = self
            .named_classes
            .iter()
            .filter(|&&c| c != self.thing_id && c != self.nothing_id)
            .copied()
            .collect();

        let mut unsatisfiable: Vec<u32> = Vec::new();
        for &cls in &classes {
            if !self.is_satisfiable(&Concept::Atom(cls)) {
                unsatisfiable.push(cls);
            }
        }

        let satisfiable: Vec<u32> = classes
            .iter()
            .filter(|c| !unsatisfiable.contains(c))
            .copied()
            .collect();

        // Compute told subsumers (transitive closure)
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

        let mut hierarchy: HashMap<u32, HashSet<u32>> = HashMap::new();
        let mut inferred_count: usize = 0;

        // Include told subsumptions in hierarchy
        for (&cls, supers) in &told {
            if satisfiable.contains(&cls) {
                for &sup in supers {
                    if satisfiable.contains(&sup) {
                        hierarchy.entry(cls).or_default().insert(sup);
                    }
                }
            }
        }

        // Test non-told subsumptions via tableau
        for &sub in &satisfiable {
            for &sup in &satisfiable {
                if sub == sup {
                    continue;
                }
                if told.get(&sub).map_or(false, |s| s.contains(&sup)) {
                    continue; // Already known
                }
                if self.is_subsumed(&Concept::Atom(sub), &Concept::Atom(sup)) {
                    hierarchy.entry(sub).or_default().insert(sup);
                    inferred_count += 1;
                }
            }
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

        ClassificationResult {
            hierarchy,
            unsatisfiable,
            equivalences,
            inferred_subsumptions: inferred_count,
        }
    }

    /// Entry point for integration with the existing reasoner.
    pub fn run(graph: &Arc<GraphStore>, materialize: bool) -> anyhow::Result<String> {
        let reasoner = Self::from_graph(graph)?;
        let initial_triples = graph.triple_count();

        let consistent = reasoner.is_consistent();
        let result = reasoner.classify();

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

        // Materialize all hierarchy subsumptions (told transitive + inferred)
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
            "consistent": consistent,
            "named_classes": reasoner.named_classes.len(),
            "unsatisfiable_classes": unsat_names,
            "inferred_subsumptions": result.inferred_subsumptions,
            "equivalences": equiv_json,
            "classification": hierarchy_json,
            "initial_triples": initial_triples,
            "final_triples": graph.triple_count(),
            "inferred_count": materialized,
        });
        if !materialize {
            output["dry_run"] = serde_json::json!(true);
        }
        Ok(output.to_string())
    }
}

pub struct ClassificationResult {
    hierarchy: HashMap<u32, HashSet<u32>>,
    unsatisfiable: Vec<u32>,
    equivalences: Vec<(u32, u32)>,
    inferred_subsumptions: usize,
}
