//! An RDF graph written as OWL 2 Functional-Style Syntax (OFN), issue #322.
//!
//! # What this inverts, and why only that
//!
//! [`crate::ofn`] reads OFN into the triples the W3C "OWL 2 Mapping to RDF
//! Graphs" Table 1, Table 2 and Section 2.3 assign it. This module recognises
//! exactly the image of that function and writes the axiom each piece of it
//! came from. It does not implement the full reverse mapping of Section 3,
//! which also accepts OWL 1 spellings (`owl:distinctMembers`), unqualified
//! `rdf:List` typing and other habits of other writers: each of those would
//! come back through the reader as a DIFFERENT graph, so writing it would be a
//! conversion that changes the ontology without saying so. A structure outside
//! the image is reported instead.
//!
//! The acceptance test is therefore `ofn::parse(write_ofn(G))` isomorphic to
//! `G`, measured in `tests/ofn_writer_test.rs`.
//!
//! # Nothing is dropped silently
//!
//! Every triple of the input is either consumed by an axiom in the output or
//! counted in [`OfnOutput::not_exported`] under its predicate (`rdf:type X` for
//! a type triple) with the reason the axiom it belonged to could not be
//! written. That is the rule `constructs_not_exported` states for the
//! first-order export, applied to the one writer that had no report. Two
//! further lists say where the output is NOT faithful even though nothing was
//! left out: an anonymous expression the graph shares between axioms, which
//! OFN can only write twice, and properties whose kind the graph never
//! declared.
//!
//! # Determinism
//!
//! Each axiom is rendered to a string with every anonymous class expression,
//! data range and list written inline, so their blank-node labels never reach
//! the output, and the axioms are then sorted. The only labels that are
//! printed are those of anonymous INDIVIDUALS, and those are RDFC-1.0
//! canonical labels ([`crate::canon`]) computed over the blank-node
//! components the individuals belong to, so two loads of one file, whatever
//! labels the parser handed out, give byte-identical text. A graph whose
//! components exhaust the RDFC budget (scaled with their size, see
//! `label_budget`) is refused by name, because a label chosen any other way
//! would not be canonical and the output would stop being reproducible.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use oxigraph::model::{BlankNode, GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term, Triple};

use crate::canon;

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const SWRL: &str = "http://www.w3.org/2003/11/swrl#";

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const RDF_PLAIN_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral";
const RDFS_DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const XSD_NON_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";

/// Nested annotations deeper than this are refused. The reader never nests
/// more than the source did, and a graph whose `owl:Annotation` nodes point
/// at each other in a loop would otherwise recurse without end.
const MAX_ANNOTATION_DEPTH: usize = 64;

/// See [`label_budget`].
const LABEL_CALLS_PER_NODE: usize = 8;

/// The annotation properties OWL 2 builds in (Section 5.5 of the structural
/// specification). Every other IRI in a reserved namespace is vocabulary, and
/// a triple using it outside a recognised structure is reported, not written
/// as an annotation.
const BUILT_IN_ANNOTATION_PROPERTIES: &[&str] = &[
    "http://www.w3.org/2000/01/rdf-schema#label",
    "http://www.w3.org/2000/01/rdf-schema#comment",
    "http://www.w3.org/2000/01/rdf-schema#seeAlso",
    "http://www.w3.org/2000/01/rdf-schema#isDefinedBy",
    "http://www.w3.org/2002/07/owl#deprecated",
    "http://www.w3.org/2002/07/owl#versionInfo",
    "http://www.w3.org/2002/07/owl#priorVersion",
    "http://www.w3.org/2002/07/owl#backwardCompatibleWith",
    "http://www.w3.org/2002/07/owl#incompatibleWith",
];

/// Predicates whose triple is an AXIOM about its subject, never part of the
/// subject's own structure. An anonymous expression is the subject of one of
/// these whenever it is the left operand of a GCI or an n-ary chain, so they
/// are left out when a node's shape is matched against Table 1.
const AXIOM_PREDICATES: &[&str] = &[
    "http://www.w3.org/2000/01/rdf-schema#subClassOf",
    "http://www.w3.org/2000/01/rdf-schema#subPropertyOf",
    "http://www.w3.org/2000/01/rdf-schema#domain",
    "http://www.w3.org/2000/01/rdf-schema#range",
    "http://www.w3.org/2002/07/owl#equivalentClass",
    "http://www.w3.org/2002/07/owl#disjointWith",
    "http://www.w3.org/2002/07/owl#hasKey",
    "http://www.w3.org/2002/07/owl#equivalentProperty",
    "http://www.w3.org/2002/07/owl#propertyDisjointWith",
    "http://www.w3.org/2002/07/owl#propertyChainAxiom",
];

/// The property characteristics, which are `rdf:type` triples on a property
/// expression and so may sit on an `ObjectInverseOf` node.
const CHARACTERISTICS: &[&str] = &[
    "FunctionalProperty",
    "InverseFunctionalProperty",
    "ReflexiveProperty",
    "IrreflexiveProperty",
    "SymmetricProperty",
    "AsymmetricProperty",
    "TransitiveProperty",
];

/// Datatypes OWL 2 knows without a declaration (Section 4 of the structural
/// specification), besides everything in the `xsd:` namespace. Used only to
/// tell a data restriction from an object restriction, which the RDF does not
/// say when the property is undeclared.
const BUILT_IN_DATATYPES: &[&str] = &[
    "http://www.w3.org/2000/01/rdf-schema#Literal",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString",
    "http://www.w3.org/2002/07/owl#real",
    "http://www.w3.org/2002/07/owl#rational",
];

/// One construct the output does not carry, or carries unfaithfully.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct NotExported {
    pub construct: String,
    pub occurrences: usize,
    pub why: String,
}

/// The document and everything a caller needs to know about what it left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfnOutput {
    pub text: String,
    pub axioms_written: usize,
    pub triples_read: usize,
    pub triples_exported: usize,
    /// Triples no axiom in `text` accounts for, by construct and reason.
    pub not_exported: Vec<NotExported>,
    /// Written, but the reader will not give back the same graph.
    pub not_round_trippable: Vec<NotExported>,
    /// The graph had no `owl:Ontology` node, so the header `Ontology(` adds
    /// one triple the input did not have.
    pub header_synthesised: bool,
    /// Assertions on a property the graph never declared, written as
    /// `AnnotationAssertion` because that is the one reading that commits the
    /// ontology to nothing. The RDF round trip is exact either way; an OWL
    /// tool reading the output sees no object or data property there.
    pub undeclared_property_assertions: usize,
}

impl OfnOutput {
    /// The report as JSON, without the text, under the names the first-order
    /// export uses for the same idea.
    pub fn report_json(&self) -> serde_json::Value {
        serde_json::json!({
            "format": "ofn",
            "axioms_written": self.axioms_written,
            "triples_read": self.triples_read,
            "triples_exported": self.triples_exported,
            "exports_fewer_triples_than_read": !self.not_exported.is_empty(),
            "constructs_not_exported": self.not_exported,
            "constructs_not_round_trippable": self.not_round_trippable,
            "header_synthesised": self.header_synthesised,
            "undeclared_property_assertions_written_as_annotations":
                self.undeclared_property_assertions,
        })
    }
}

/// Write `triples` as OFN, abbreviating IRIs with `prefixes` (plus the five
/// the specification predefines).
///
/// The only error is an RDFC budget refusal while labelling anonymous
/// individuals; see the module notes.
pub fn write_ofn(
    triples: &[Triple],
    prefixes: &[(String, String)],
) -> Result<OfnOutput, canon::CanonError> {
    let ix = Index::new(triples);
    let names = Names::new(prefixes);
    let first = Writer { ix: &ix, names: &names, labels: None }.run();
    if first.anons.is_empty() {
        return Ok(first.finish(&ix, &names));
    }
    // Label every anonymous individual canonically, then render again with
    // those labels. Rendering is a pure function of the graph and the labels,
    // so the second run makes the same decisions as the first.
    //
    // The labels are computed over every triple of the blank-node components
    // the individuals belong to, not only the triples that mention one. With
    // the narrower set, `ObjectHasValue(:diameter _:a)` under `:PM1` and the
    // same restriction under `:PM2point5` gave two individuals that looked
    // alike, because the restriction node linking each to its class was left
    // out, and RDFC swapped their labels between runs (measured on ORE file
    // ore_ont_15184, one of 141 files from 1.2 MB up that failed that way).
    // Within a component, two nodes RDFC cannot tell apart are automorphic,
    // and swapping automorphic nodes does not change the text.
    let component = blank_component(&ix.triples, &first.anons);
    let quads: Vec<Quad> = ix
        .triples
        .iter()
        .filter(|t| mentions_any(t, &component))
        .map(|t| t.clone().in_graph(GraphName::DefaultGraph))
        .collect();
    let labels: HashMap<String, String> =
        canon::canonical_labels(&quads, label_budget(&component), canon::HashAlgorithm::Sha256)?
            .into_iter()
            .collect();
    Ok(Writer { ix: &ix, names: &names, labels: Some(&labels) }
        .run()
        .finish(&ix, &names))
}

/// Whether a `to` format name asks for this writer rather than an RDF one.
pub fn is_ofn_format(name: &str) -> bool {
    matches!(name.to_ascii_lowercase().as_str(), "ofn" | "functional")
}

/// Load `path`, in any format `onto_load` reads, and write it as OFN with the
/// prefixes the document declared. This is the one function behind
/// `onto_convert`, the CLI `convert` and `batch`'s `convert`, so the three
/// cannot disagree about what was left out.
///
/// The document's triples are handed over as parsed, without passing through
/// a store, so nothing between the file and the writer can respell a term.
/// That was first done because Oxigraph 0.5.9's store re-typed an
/// `xsd:nonNegativeInteger` literal as `xsd:integer` (measured on
/// `owl:minCardinality 2`), and every such cardinality would have been refused
/// as outside Table 1. The pinned store keeps literals as written since #335
/// (`tests/datatype_preservation_test.rs`); reading without one is kept
/// because it is the shorter path, not because a store would now be wrong.
pub fn convert_file(path: &str) -> anyhow::Result<OfnOutput> {
    let (quads, prefixes) = crate::graph::GraphStore::read_file_with_prefixes(path)?;
    let triples: Vec<Triple> = quads.into_iter().map(Triple::from).collect();
    Ok(write_ofn(&triples, &prefixes)?)
}

/// Every blank node connected to one of `seeds` through triples whose subject
/// and object are both blank nodes.
fn blank_component(triples: &[Triple], seeds: &HashSet<BlankNode>) -> HashSet<BlankNode> {
    let mut next: HashMap<&BlankNode, Vec<&BlankNode>> = HashMap::new();
    for t in triples {
        if let (NamedOrBlankNode::BlankNode(s), Term::BlankNode(o)) = (&t.subject, &t.object) {
            next.entry(s).or_default().push(o);
            next.entry(o).or_default().push(s);
        }
    }
    let mut seen: HashSet<BlankNode> = seeds.clone();
    let mut stack: Vec<&BlankNode> = seeds.iter().collect();
    while let Some(b) = stack.pop() {
        for &n in next.get(b).map(Vec::as_slice).unwrap_or(&[]) {
            if seen.insert(n.clone()) {
                stack.push(n);
            }
        }
    }
    seen
}

/// The RDFC call budget for labelling the nodes of `component`.
///
/// `canon::DEFAULT_CALL_BUDGET` (4000) is sized for one dataset of the W3C
/// suite, and an ABox with thousands of anonymous individuals that look alike
/// spends at least one `hash_n_degree_quads` call on every one of them before
/// any symmetry is involved: seven ORE 2015 files under 1 MB (4,155 to 6,800
/// anonymous individuals each) were refused at 4000. Scaling the budget with
/// the number of nodes keeps it a bound on SUPERLINEAR work, which is what
/// RDFC-1.0 section 4.4 asks to be bounded, without refusing linear work.
fn label_budget(component: &HashSet<BlankNode>) -> usize {
    canon::DEFAULT_CALL_BUDGET.max(LABEL_CALLS_PER_NODE * component.len())
}

fn mentions_any(t: &Triple, set: &HashSet<BlankNode>) -> bool {
    matches!(&t.subject, NamedOrBlankNode::BlankNode(b) if set.contains(b))
        || matches!(&t.object, Term::BlankNode(b) if set.contains(b))
}

// --------------------------------------------------------------------------- //
// Vocabulary tests
// --------------------------------------------------------------------------- //

fn local<'a>(n: &'a NamedNode, ns: &str) -> Option<&'a str> {
    n.as_str().strip_prefix(ns)
}

fn is_reserved(n: &NamedNode) -> bool {
    [RDF, RDFS, OWL, XSD, SWRL].iter().any(|ns| n.as_str().starts_with(ns))
}

fn is_annotation_predicate(p: &NamedNode) -> bool {
    !is_reserved(p) || BUILT_IN_ANNOTATION_PROPERTIES.contains(&p.as_str())
}

fn term_iri(t: &Term) -> Option<&NamedNode> {
    match t {
        Term::NamedNode(n) => Some(n),
        _ => None,
    }
}

fn term_blank(t: &Term) -> Option<&BlankNode> {
    match t {
        Term::BlankNode(b) => Some(b),
        _ => None,
    }
}

fn subject_term(s: &NamedOrBlankNode) -> Term {
    match s {
        NamedOrBlankNode::NamedNode(n) => Term::NamedNode(n.clone()),
        NamedOrBlankNode::BlankNode(b) => Term::BlankNode(b.clone()),
    }
}

fn term_subject(t: &Term) -> Option<NamedOrBlankNode> {
    match t {
        Term::NamedNode(n) => Some(n.clone().into()),
        Term::BlankNode(b) => Some(b.clone().into()),
        _ => None,
    }
}

// --------------------------------------------------------------------------- //
// The graph, indexed
// --------------------------------------------------------------------------- //

/// Declared entity kinds, from `Declaration` triples. They decide between
/// OFN keywords that share one RDF image (`SubObjectPropertyOf` and
/// `SubDataPropertyOf` are both `rdfs:subPropertyOf`), so they change what an
/// OWL tool reads but never the round-tripped graph.
#[derive(Default)]
struct Kinds {
    object: HashSet<NamedNode>,
    data: HashSet<NamedNode>,
    annotation: HashSet<NamedNode>,
    datatype: HashSet<NamedNode>,
    variable: HashSet<NamedNode>,
}

struct Index {
    triples: Vec<Triple>,
    by_subject: HashMap<NamedOrBlankNode, Vec<usize>>,
    /// `owl:Axiom` nodes by the triple they reify (Section 2.3.1).
    axiom_reifications: HashMap<Triple, Vec<BlankNode>>,
    /// `owl:Annotation` nodes by the annotation triple they reify (Table 2).
    annotation_reifications: HashMap<Triple, Vec<BlankNode>>,
    kinds: Kinds,
}

impl Index {
    fn new(input: &[Triple]) -> Self {
        let mut seen = HashSet::new();
        let triples: Vec<Triple> = input.iter().filter(|t| seen.insert(*t)).cloned().collect();
        let mut by_subject: HashMap<NamedOrBlankNode, Vec<usize>> = HashMap::new();
        for (i, t) in triples.iter().enumerate() {
            by_subject.entry(t.subject.clone()).or_default().push(i);
        }
        let mut ix = Self {
            triples,
            by_subject,
            axiom_reifications: HashMap::new(),
            annotation_reifications: HashMap::new(),
            kinds: Kinds::default(),
        };
        for t in &ix.triples {
            if t.predicate.as_str() != RDF_TYPE {
                continue;
            }
            let Some(class) = term_iri(&t.object) else { continue };
            match (&t.subject, class.as_str()) {
                (NamedOrBlankNode::NamedNode(n), c) => {
                    let set = match c.strip_prefix(OWL) {
                        Some("ObjectProperty") => &mut ix.kinds.object,
                        Some("DatatypeProperty") => &mut ix.kinds.data,
                        Some("AnnotationProperty") => &mut ix.kinds.annotation,
                        _ if c == RDFS_DATATYPE => &mut ix.kinds.datatype,
                        _ if c == "http://www.w3.org/2003/11/swrl#Variable" => &mut ix.kinds.variable,
                        _ => continue,
                    };
                    set.insert(n.clone());
                }
                (NamedOrBlankNode::BlankNode(_), _) => {}
            }
        }
        let mut axioms = HashMap::<Triple, Vec<BlankNode>>::new();
        let mut annotations = HashMap::<Triple, Vec<BlankNode>>::new();
        for (subject, ids) in &ix.by_subject {
            let NamedOrBlankNode::BlankNode(x) = subject else { continue };
            let node = ix.node_of(ids);
            let target = match node.one_iri(RDF_TYPE).and_then(|c| local(c, OWL)) {
                Some("Axiom") => &mut axioms,
                Some("Annotation") => &mut annotations,
                _ => continue,
            };
            let source = node.one(&format!("{OWL}annotatedSource")).map(|(_, t)| t);
            let property = node.one(&format!("{OWL}annotatedProperty")).map(|(_, t)| t);
            let value = node.one(&format!("{OWL}annotatedTarget")).map(|(_, t)| t);
            if let (Some(s), Some(Term::NamedNode(p)), Some(o)) = (source, property, value)
                && let Some(s) = term_subject(s)
            {
                target
                    .entry(Triple::new(s, p.clone(), o.clone()))
                    .or_default()
                    .push(x.clone());
            }
        }
        ix.axiom_reifications = axioms;
        ix.annotation_reifications = annotations;
        ix
    }

    fn ids(&self, s: &NamedOrBlankNode) -> &[usize] {
        self.by_subject.get(s).map(Vec::as_slice).unwrap_or(&[])
    }

    fn node_of(&self, ids: &[usize]) -> Node<'_> {
        let mut reserved: BTreeMap<&str, Vec<(usize, &Term)>> = BTreeMap::new();
        for &i in ids {
            let t = &self.triples[i];
            if !is_annotation_predicate(&t.predicate) && !AXIOM_PREDICATES.contains(&t.predicate.as_str()) {
                reserved.entry(t.predicate.as_str()).or_default().push((i, &t.object));
            }
        }
        Node { reserved }
    }

    fn node(&self, b: &BlankNode) -> Node<'_> {
        self.node_of(self.ids(&b.clone().into()))
    }

    fn is_datatype_iri(&self, n: &NamedNode) -> bool {
        n.as_str().starts_with(XSD)
            || BUILT_IN_DATATYPES.contains(&n.as_str())
            || self.kinds.datatype.contains(n)
    }

    /// Whether `t` reads as a data range rather than a class expression.
    fn is_data_range(&self, t: &Term) -> bool {
        match t {
            Term::NamedNode(n) => self.is_datatype_iri(n),
            Term::BlankNode(b) => self.node(b).one_iri(RDF_TYPE).map(NamedNode::as_str) == Some(RDFS_DATATYPE),
            _ => false,
        }
    }

    fn is_data_property(&self, t: &Term) -> bool {
        matches!(t, Term::NamedNode(n) if self.kinds.data.contains(n) && !self.kinds.object.contains(n))
    }

    fn is_annotation_property(&self, t: &Term) -> bool {
        matches!(t, Term::NamedNode(n) if self.kinds.annotation.contains(n))
    }
}

/// The reserved-vocabulary triples of one subject, by predicate. Annotation
/// triples are left out: on the nodes that may carry them they are read
/// separately, and on the nodes that may not they are reported.
struct Node<'a> {
    reserved: BTreeMap<&'a str, Vec<(usize, &'a Term)>>,
}

impl<'a> Node<'a> {
    fn one(&self, p: &str) -> Option<(usize, &'a Term)> {
        match self.reserved.get(p).map(Vec::as_slice) {
            Some([only]) => Some(*only),
            _ => None,
        }
    }

    fn one_iri(&self, p: &str) -> Option<&'a NamedNode> {
        self.one(p).and_then(|(_, t)| term_iri(t))
    }

    /// The reserved predicates present, besides `rdf:type`.
    fn shape(&self) -> BTreeSet<&'a str> {
        self.reserved.keys().copied().filter(|p| *p != RDF_TYPE).collect()
    }
}

// --------------------------------------------------------------------------- //
// Prefixes
// --------------------------------------------------------------------------- //

struct Names {
    /// Name to namespace, as `Prefix(...)` lines are written.
    declared: BTreeMap<String, String>,
    /// The same, longest namespace first, for abbreviation.
    by_length: Vec<(String, String)>,
}

/// A prefix name the reader's lexer reads back as one: no character that ends
/// a word, no colon, and not `_`, which would make every name a blank node.
fn safe_prefix_name(name: &str) -> bool {
    name.is_empty()
        || (name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
            && !name.ends_with('.'))
}

/// A local name that survives the lexer and means one thing to every OFN
/// reader: the OWL API follows SPARQL's PN_LOCAL, which this is a subset of.
fn safe_local(local: &str) -> bool {
    !local.is_empty()
        && local.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        && !local.starts_with(['-', '.'])
        && !local.ends_with('.')
}

impl Names {
    fn new(prefixes: &[(String, String)]) -> Self {
        let mut declared: BTreeMap<String, String> = crate::ofn::STANDARD_PREFIXES
            .iter()
            .map(|(n, i)| ((*n).to_string(), (*i).to_string()))
            .collect();
        for (name, iri) in prefixes {
            // A later declaration of a name wins, as it does in the reader.
            if safe_prefix_name(name) && oxigraph::model::NamedNode::new(iri.as_str()).is_ok() {
                declared.insert(name.clone(), iri.clone());
            }
        }
        let mut by_length: Vec<(String, String)> =
            declared.iter().map(|(n, i)| (n.clone(), i.clone())).collect();
        by_length.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
        Self { declared, by_length }
    }

    fn iri(&self, n: &NamedNode) -> String {
        let s = n.as_str();
        for (name, ns) in &self.by_length {
            if let Some(rest) = s.strip_prefix(ns.as_str())
                && safe_local(rest)
            {
                return format!("{name}:{rest}");
            }
        }
        format!("<{s}>")
    }
}

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        if matches!(c, '"' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

// --------------------------------------------------------------------------- //
// Rendering
// --------------------------------------------------------------------------- //

type R<T> = Result<T, String>;

/// What one axiom used, committed only if the whole axiom renders.
#[derive(Default)]
struct Tx {
    used: Vec<usize>,
    /// Anonymous expressions, data ranges, lists and facets written inline.
    inline: Vec<BlankNode>,
    anons: Vec<BlankNode>,
}

#[derive(Default)]
struct Run {
    lines: BTreeSet<String>,
    header: BTreeSet<String>,
    header_line: String,
    header_synthesised: bool,
    consumed: Vec<bool>,
    why: HashMap<usize, String>,
    inline_uses: HashMap<BlankNode, usize>,
    anons: HashSet<BlankNode>,
    undeclared_property_assertions: usize,
}

struct Writer<'a> {
    ix: &'a Index,
    names: &'a Names,
    labels: Option<&'a HashMap<String, String>>,
}

impl<'a> Writer<'a> {
    fn t(&self, i: usize) -> &'a Triple {
        &self.ix.triples[i]
    }

    // ---- terminals ------------------------------------------------------ //

    fn iri(&self, n: &NamedNode) -> String {
        self.names.iri(n)
    }

    fn literal(&self, l: &Literal) -> R<String> {
        let dt = l.datatype();
        if dt.as_str() == RDF_PLAIN_LITERAL {
            // The reader turns `"x@"^^rdf:PlainLiteral` into the plain literal
            // it spells (Table 1, first two rows), so writing one would come
            // back as a different term.
            return Err("an rdf:PlainLiteral-typed literal, which the reader maps to a plain literal".into());
        }
        let v = escape(l.value());
        Ok(match l.language() {
            Some(tag) => format!("{v}@{tag}"),
            None if dt.as_str() == XSD_STRING => v,
            None => format!("{v}^^{}", self.iri(&dt.into_owned())),
        })
    }

    fn anon(&self, b: &BlankNode, tx: &mut Tx) -> String {
        tx.anons.push(b.clone());
        match self.labels.and_then(|m| m.get(b.as_str())) {
            Some(label) => format!("_:{label}"),
            None => format!("_:{}", b.as_str()),
        }
    }

    fn individual(&self, t: &Term, tx: &mut Tx) -> R<String> {
        match t {
            Term::NamedNode(n) => Ok(self.iri(n)),
            Term::BlankNode(b) => Ok(self.anon(b, tx)),
            _ => Err("a literal where an individual belongs".into()),
        }
    }

    fn annotation_value(&self, t: &Term, tx: &mut Tx) -> R<String> {
        match t {
            Term::Literal(l) => self.literal(l),
            other => self.individual(other, tx),
        }
    }

    fn property_iri(&self, t: &Term, what: &str) -> R<String> {
        term_iri(t)
            .map(|n| self.iri(n))
            .ok_or_else(|| format!("a {what} that is not an IRI"))
    }

    fn cardinality(&self, t: &Term) -> R<String> {
        // The reader writes the cardinality as the canonical decimal of a u64
        // typed xsd:nonNegativeInteger, so only that spelling comes back as
        // the same literal.
        let Term::Literal(l) = t else {
            return Err("a cardinality that is not a literal".into());
        };
        let v = l.value();
        if l.datatype().as_str() == XSD_NON_NEGATIVE_INTEGER
            && v.parse::<u64>().is_ok_and(|n| n.to_string() == v)
        {
            Ok(v.to_string())
        } else {
            Err("a cardinality that is not a canonical xsd:nonNegativeInteger".into())
        }
    }

    // ---- lists ---------------------------------------------------------- //

    /// `T(SEQ ...)` read back: the items of an `rdf:List`, each cell carrying
    /// `rdf:first` and `rdf:rest` and nothing else, or also `rdf:type
    /// swrl:AtomList` when `atom_list`, which is how the reader writes the
    /// body and head of a rule.
    fn list(&self, head: &Term, atom_list: bool, tx: &mut Tx) -> R<Vec<&'a Term>> {
        let mut items = Vec::new();
        let mut seen = HashSet::new();
        let mut cur = head.clone();
        loop {
            match &cur {
                Term::NamedNode(n) if n.as_str() == RDF_NIL => return Ok(items),
                Term::BlankNode(b) => {
                    if !seen.insert(b.clone()) {
                        return Err("a cyclic rdf:List".into());
                    }
                    let node = self.ix.node(b);
                    // The cell's triples are touched whether or not it is
                    // well formed, so a malformed one is reported with the
                    // reason rather than as an unexplained rdf:first.
                    tx.used.extend(self.ix.ids(&b.clone().into()).iter().filter(|&&i| !is_annotation_predicate(&self.t(i).predicate)));
                    let mut want: BTreeSet<&str> = [RDF_FIRST, RDF_REST].into();
                    if atom_list {
                        want.insert(RDF_TYPE);
                    }
                    let have: BTreeSet<&str> = node.reserved.keys().copied().collect();
                    if have != want {
                        return Err("a malformed rdf:List cell".into());
                    }
                    if atom_list && node.one_iri(RDF_TYPE).and_then(|c| local(c, SWRL)) != Some("AtomList") {
                        return Err("a SWRL atom list cell not typed swrl:AtomList".into());
                    }
                    let (Some((_, first)), Some((_, rest))) = (node.one(RDF_FIRST), node.one(RDF_REST)) else {
                        return Err("an rdf:List cell without exactly one rdf:first and rdf:rest".into());
                    };
                    tx.inline.push(b.clone());
                    items.push(first);
                    cur = rest.clone();
                }
                _ => return Err("a malformed rdf:List: a tail that is neither rdf:nil nor a cell".into()),
            }
        }
    }

    fn list_min(&self, head: &Term, min: usize, what: &str, tx: &mut Tx) -> R<Vec<&'a Term>> {
        let items = self.list(head, false, tx)?;
        if items.len() < min {
            // The reader writes this construct only with at least `min`
            // operands (fewer is its syntax error), so a shorter list is not
            // the image of anything it reads.
            return Err(format!("{what} over fewer than {min} operand(s)"));
        }
        Ok(items)
    }

    fn join<T>(&self, items: &[T], tx: &mut Tx, mut f: impl FnMut(&Self, &T, &mut Tx) -> R<String>) -> R<String> {
        let mut parts = Vec::with_capacity(items.len());
        for it in items {
            parts.push(f(self, it, tx)?);
        }
        Ok(parts.join(" "))
    }

    // ---- expressions ---------------------------------------------------- //

    fn ope(&self, t: &Term, tx: &mut Tx) -> R<String> {
        match t {
            Term::NamedNode(n) => Ok(self.iri(n)),
            Term::BlankNode(b) => {
                let node = self.ix.node(b);
                let characteristics_only = node.reserved.get(RDF_TYPE).is_none_or(|types| {
                    types.iter().all(|(_, c)| {
                        term_iri(c).and_then(|c| local(c, OWL)).is_some_and(|l| CHARACTERISTICS.contains(&l))
                    })
                });
                let inverse = format!("{OWL}inverseOf");
                match node.one(&inverse) {
                    Some((i, Term::NamedNode(p))) if node.shape().len() == 1 && characteristics_only => {
                        tx.used.push(i);
                        tx.inline.push(b.clone());
                        Ok(format!("ObjectInverseOf({})", self.iri(p)))
                    }
                    _ => Err("an anonymous object property that is not ObjectInverseOf of a named one".into()),
                }
            }
            _ => Err("a literal where an object property belongs".into()),
        }
    }

    fn ce(&self, t: &Term, tx: &mut Tx) -> R<String> {
        match t {
            Term::NamedNode(n) => Ok(self.iri(n)),
            Term::BlankNode(b) => self.class_node(b, tx),
            _ => Err("a literal where a class expression belongs".into()),
        }
    }

    fn class_node(&self, b: &BlankNode, tx: &mut Tx) -> R<String> {
        let node = self.ix.node(b);
        let Some((ti, ty)) = node.one(RDF_TYPE) else {
            return Err("an anonymous class expression without exactly one rdf:type".into());
        };
        tx.used.push(ti);
        let shape = node.shape();
        let ty = term_iri(ty).and_then(|c| local(c, OWL));
        let out = match ty {
            Some("Class") => {
                let [p] = shape.iter().copied().collect::<Vec<_>>()[..] else {
                    return Err("an owl:Class node without exactly one constructor".into());
                };
                let (i, arg) = node.one(p).ok_or("a class constructor given more than once")?;
                tx.used.push(i);
                match p.strip_prefix(OWL) {
                    Some(k @ ("intersectionOf" | "unionOf")) => {
                        let items = self.list_min(arg, 2, k, tx)?;
                        let kw = if k == "intersectionOf" { "ObjectIntersectionOf" } else { "ObjectUnionOf" };
                        format!("{kw}({})", self.join(&items, tx, |w, t, tx| w.ce(t, tx))?)
                    }
                    Some("complementOf") => format!("ObjectComplementOf({})", self.ce(arg, tx)?),
                    Some("oneOf") => {
                        let items = self.list_min(arg, 1, "oneOf", tx)?;
                        format!("ObjectOneOf({})", self.join(&items, tx, |w, t, tx| w.individual(t, tx))?)
                    }
                    _ => return Err("an owl:Class node whose constructor is not a class constructor".into()),
                }
            }
            Some("Restriction") => self.restriction(&node, &shape, tx)?,
            _ => return Err("an anonymous class expression typed neither owl:Class nor owl:Restriction".into()),
        };
        tx.inline.push(b.clone());
        Ok(out)
    }

    fn restriction(&self, node: &Node<'a>, shape: &BTreeSet<&'a str>, tx: &mut Tx) -> R<String> {
        let owl = |l: &str| format!("{OWL}{l}");
        let rest: Vec<&str> = shape
            .iter()
            .filter_map(|p| p.strip_prefix(OWL))
            .filter(|p| *p != "onProperty" && *p != "onProperties")
            .collect();
        if rest.len() + usize::from(shape.contains(owl("onProperty").as_str()))
            + usize::from(shape.contains(owl("onProperties").as_str()))
            != shape.len()
        {
            return Err("an owl:Restriction with a predicate outside the OWL vocabulary of restrictions".into());
        }
        if let Some((li, list)) = node.one(&owl("onProperties")) {
            // Table 1 writes owl:onProperties only for n-ary data
            // restrictions, n >= 2; one property gets owl:onProperty.
            let ["allValuesFrom" | "someValuesFrom"] = rest[..] else {
                return Err("an owl:onProperties restriction that is not DataSomeValuesFrom or DataAllValuesFrom".into());
            };
            if shape.contains(owl("onProperty").as_str()) {
                return Err("an owl:Restriction with both owl:onProperty and owl:onProperties".into());
            }
            tx.used.push(li);
            let props = self.list_min(list, 2, "owl:onProperties", tx)?;
            let props = self.join(&props, tx, |w, t, _| w.property_iri(t, "data property"))?;
            let (fi, filler) = node.one(&owl(rest[0])).ok_or("a restriction filler given more than once")?;
            tx.used.push(fi);
            let kw = if rest[0] == "someValuesFrom" { "DataSomeValuesFrom" } else { "DataAllValuesFrom" };
            return Ok(format!("{kw}({props} {})", self.dr(filler, tx)?));
        }
        let Some((pi, p)) = node.one(&owl("onProperty")) else {
            return Err("an owl:Restriction without exactly one owl:onProperty".into());
        };
        tx.used.push(pi);
        let take = |l: &str, tx: &mut Tx| -> R<&'a Term> {
            let (i, t) = node.one(&owl(l)).ok_or("a restriction component given more than once")?;
            tx.used.push(i);
            Ok(t)
        };
        let data_p = self.ix.is_data_property(p);
        Ok(match rest[..] {
            [k @ ("someValuesFrom" | "allValuesFrom")] => {
                let f = take(k, tx)?;
                let some = k == "someValuesFrom";
                if data_p || (term_iri(p).is_some() && self.ix.is_data_range(f)) {
                    let kw = if some { "DataSomeValuesFrom" } else { "DataAllValuesFrom" };
                    format!("{kw}({} {})", self.property_iri(p, "data property")?, self.dr(f, tx)?)
                } else {
                    let kw = if some { "ObjectSomeValuesFrom" } else { "ObjectAllValuesFrom" };
                    format!("{kw}({} {})", self.ope(p, tx)?, self.ce(f, tx)?)
                }
            }
            ["hasValue"] => match take("hasValue", tx)? {
                Term::Literal(l) => {
                    format!("DataHasValue({} {})", self.property_iri(p, "data property")?, self.literal(l)?)
                }
                v => format!("ObjectHasValue({} {})", self.ope(p, tx)?, self.individual(v, tx)?),
            },
            ["hasSelf"] => match take("hasSelf", tx)? {
                Term::Literal(l) if l.value() == "true" && l.datatype().as_str() == XSD_BOOLEAN => {
                    format!("ObjectHasSelf({})", self.ope(p, tx)?)
                }
                _ => return Err("an owl:hasSelf that is not \"true\"^^xsd:boolean".into()),
            },
            [k @ ("minCardinality" | "maxCardinality" | "cardinality")] => {
                let n = self.cardinality(take(k, tx)?)?;
                let kind = cardinality_kind(k);
                if data_p {
                    format!("Data{kind}({n} {})", self.property_iri(p, "data property")?)
                } else {
                    format!("Object{kind}({n} {})", self.ope(p, tx)?)
                }
            }
            [a, b] => {
                let (k, on) = if a.starts_with("on") { (b, a) } else { (a, b) };
                let kind = match k {
                    "minQualifiedCardinality" => "MinCardinality",
                    "maxQualifiedCardinality" => "MaxCardinality",
                    "qualifiedCardinality" => "ExactCardinality",
                    _ => return Err("an owl:Restriction whose components match no row of Table 1".into()),
                };
                let n = self.cardinality(take(k, tx)?)?;
                match on {
                    "onClass" => format!("Object{kind}({n} {} {})", self.ope(p, tx)?, self.ce(take(on, tx)?, tx)?),
                    "onDataRange" => format!(
                        "Data{kind}({n} {} {})",
                        self.property_iri(p, "data property")?,
                        self.dr(take(on, tx)?, tx)?
                    ),
                    _ => return Err("an owl:Restriction whose components match no row of Table 1".into()),
                }
            }
            _ => return Err("an owl:Restriction whose components match no row of Table 1".into()),
        })
    }

    fn dr(&self, t: &Term, tx: &mut Tx) -> R<String> {
        match t {
            Term::NamedNode(n) => Ok(self.iri(n)),
            Term::BlankNode(b) => self.data_range_node(b, tx),
            _ => Err("a literal where a data range belongs".into()),
        }
    }

    fn data_range_node(&self, b: &BlankNode, tx: &mut Tx) -> R<String> {
        let node = self.ix.node(b);
        let Some((ti, ty)) = node.one(RDF_TYPE) else {
            return Err("an anonymous data range without exactly one rdf:type".into());
        };
        if term_iri(ty).map(NamedNode::as_str) != Some(RDFS_DATATYPE) {
            return Err("an anonymous data range not typed rdfs:Datatype".into());
        }
        tx.used.push(ti);
        let shape: Vec<&str> = node.shape().into_iter().collect();
        let take = |p: &str, tx: &mut Tx| -> R<&'a Term> {
            let (i, t) = node.one(p).ok_or("a data range component given more than once")?;
            tx.used.push(i);
            Ok(t)
        };
        let owl = |l: &str| format!("{OWL}{l}");
        let out = match shape.iter().map(|p| p.strip_prefix(OWL).unwrap_or(p)).collect::<Vec<_>>()[..] {
            [k @ ("intersectionOf" | "unionOf")] => {
                let items = self.list_min(take(&owl(k), tx)?, 2, k, tx)?;
                let kw = if k == "intersectionOf" { "DataIntersectionOf" } else { "DataUnionOf" };
                format!("{kw}({})", self.join(&items, tx, |w, t, tx| w.dr(t, tx))?)
            }
            ["datatypeComplementOf"] => format!("DataComplementOf({})", self.dr(take(&owl("datatypeComplementOf"), tx)?, tx)?),
            ["oneOf"] => {
                let items = self.list_min(take(&owl("oneOf"), tx)?, 1, "oneOf", tx)?;
                let lits = self.join(&items, tx, |w, t, _| match t {
                    Term::Literal(l) => w.literal(l),
                    _ => Err("a DataOneOf member that is not a literal".into()),
                })?;
                format!("DataOneOf({lits})")
            }
            ["onDatatype", "withRestrictions"] => {
                let dt = self.property_iri(take(&owl("onDatatype"), tx)?, "datatype")?;
                let facets = self.list_min(take(&owl("withRestrictions"), tx)?, 1, "owl:withRestrictions", tx)?;
                let facets = self.join(&facets, tx, |w, t, tx| w.facet(t, tx))?;
                format!("DatatypeRestriction({dt} {facets})")
            }
            _ => return Err("an rdfs:Datatype node whose components match no row of Table 1".into()),
        };
        tx.inline.push(b.clone());
        Ok(out)
    }

    fn facet(&self, t: &Term, tx: &mut Tx) -> R<String> {
        let Some(b) = term_blank(t) else {
            return Err("a facet restriction that is not a blank node".into());
        };
        let ids = self.ix.ids(&b.clone().into());
        let [i] = ids[..] else {
            return Err("a facet restriction node without exactly one triple".into());
        };
        let tr = self.t(i);
        let Term::Literal(l) = &tr.object else {
            return Err("a facet restriction whose value is not a literal".into());
        };
        tx.used.push(i);
        tx.inline.push(b.clone());
        Ok(format!("{} {}", self.iri(&tr.predicate), self.literal(l)?))
    }

    // ---- annotations ---------------------------------------------------- //

    /// Every annotation on `subject` (Table 2), nested annotations included,
    /// sorted. `TANN` writes one triple per annotation and, when that
    /// annotation is itself annotated, an `owl:Annotation` node reifying it.
    fn annotations(&self, subject: &NamedOrBlankNode, depth: usize, tx: &mut Tx) -> R<Vec<String>> {
        if depth > MAX_ANNOTATION_DEPTH {
            return Err(format!("annotations nested more than {MAX_ANNOTATION_DEPTH} deep"));
        }
        let mut out = Vec::new();
        for &i in self.ix.ids(subject) {
            if is_annotation_predicate(&self.t(i).predicate) {
                out.extend(self.annotation(i, depth, tx)?);
            }
        }
        out.sort();
        Ok(out)
    }

    /// The annotation triple `i`, once per `owl:Annotation` node that annotates
    /// it, or once bare. Two annotations that differ only in their own
    /// annotations share the triple and differ in the reifications, which is
    /// exactly what writing each of them gives back.
    fn annotation(&self, i: usize, depth: usize, tx: &mut Tx) -> R<Vec<String>> {
        let t = self.t(i);
        let body = format!("{} {}", self.iri(&t.predicate), self.annotation_value(&t.object, tx)?);
        tx.used.push(i);
        let mut out = Vec::new();
        for y in self.ix.annotation_reifications.get(t).map(Vec::as_slice).unwrap_or(&[]) {
            if let Some(nested) = self.reification(y, depth, tx) {
                out.push(format!("Annotation({} {body})", nested.join(" ")));
            }
        }
        if out.is_empty() {
            out.push(format!("Annotation({body})"));
        }
        Ok(out)
    }

    /// The annotations an `owl:Axiom` or `owl:Annotation` node carries, with
    /// its four structural triples consumed, or `None` when it carries none or
    /// does not render: the reader reifies only annotated things, so such a
    /// node is not in its image and is left to be reported.
    fn reification(&self, x: &BlankNode, depth: usize, tx: &mut Tx) -> Option<Vec<String>> {
        let node = self.ix.node(x);
        let structural = [
            RDF_TYPE.to_string(),
            format!("{OWL}annotatedSource"),
            format!("{OWL}annotatedProperty"),
            format!("{OWL}annotatedTarget"),
        ];
        let mut sub = Tx::default();
        for p in &structural {
            sub.used.push(node.one(p)?.0);
        }
        let anns = self.annotations(&x.clone().into(), depth + 1, &mut sub).ok()?;
        if anns.is_empty() {
            return None;
        }
        tx.used.extend(sub.used);
        tx.inline.extend(sub.inline);
        tx.anons.extend(sub.anons);
        Some(anns)
    }

    /// The annotation sets of the axiom whose main triple is `main`: one per
    /// `owl:Axiom` node reifying it, or a single empty set.
    fn axiom_annotations(&self, main: &Triple, tx: &mut Tx) -> Vec<Vec<String>> {
        let mut sets: Vec<Vec<String>> = self
            .ix
            .axiom_reifications
            .get(main)
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .iter()
            .filter_map(|x| self.reification(x, 0, tx))
            .collect();
        sets.sort();
        if sets.is_empty() {
            sets.push(Vec::new());
        }
        sets
    }

    // ---- axioms --------------------------------------------------------- //

    fn emit(kw: &str, sets: &[Vec<String>], body: &str) -> Vec<String> {
        sets.iter()
            .map(|anns| {
                if anns.is_empty() {
                    format!("{kw}({body})")
                } else {
                    format!("{kw}({} {body})", anns.join(" "))
                }
            })
            .collect()
    }

    /// An axiom with a main triple: the body and the annotation sets.
    fn main(&self, i: usize, kw: &str, body: String, tx: &mut Tx) -> Vec<String> {
        let sets = self.axiom_annotations(self.t(i), tx);
        Self::emit(kw, &sets, &body)
    }

    /// The axiom triple `i` begins, if its predicate begins one. `None` means
    /// the triple is not an axiom trigger at all and is left to later passes.
    fn axiom(&self, i: usize, tx: &mut Tx) -> Option<R<Vec<String>>> {
        let t = self.t(i);
        let s = subject_term(&t.subject);
        let o = &t.object;
        let p = t.predicate.as_str();
        let ix = self.ix;
        let r = (|| -> R<Vec<String>> {
            if p == RDF_TYPE {
                return self.typed_axiom(i, tx).unwrap_or_else(|| Err(String::new()));
            }
            let (kw, body) = match (p.strip_prefix(RDFS), p.strip_prefix(OWL)) {
                (Some("subClassOf"), _) => ("SubClassOf", format!("{} {}", self.ce(&s, tx)?, self.ce(o, tx)?)),
                (_, Some("disjointWith")) => ("DisjointClasses", format!("{} {}", self.ce(&s, tx)?, self.ce(o, tx)?)),
                (_, Some("disjointUnionOf")) => {
                    let c = self.property_iri(&s, "class")?;
                    let items = self.list_min(o, 2, "owl:disjointUnionOf", tx)?;
                    ("DisjointUnion", format!("{c} {}", self.join(&items, tx, |w, t, tx| w.ce(t, tx))?))
                }
                (Some("subPropertyOf"), _) => {
                    if ix.is_annotation_property(&s) || ix.is_annotation_property(o) {
                        ("SubAnnotationPropertyOf", self.iri_pair(&s, o)?)
                    } else if ix.is_data_property(&s) || ix.is_data_property(o) {
                        ("SubDataPropertyOf", self.iri_pair(&s, o)?)
                    } else {
                        ("SubObjectPropertyOf", format!("{} {}", self.ope(&s, tx)?, self.ope(o, tx)?))
                    }
                }
                (_, Some("propertyChainAxiom")) => {
                    let chain = self.list_min(o, 2, "ObjectPropertyChain", tx)?;
                    let chain = self.join(&chain, tx, |w, t, tx| w.ope(t, tx))?;
                    ("SubObjectPropertyOf", format!("ObjectPropertyChain({chain}) {}", self.ope(&s, tx)?))
                }
                (_, Some("propertyDisjointWith")) => {
                    if term_iri(o).is_some() && (ix.is_data_property(&s) || ix.is_data_property(o)) {
                        ("DisjointDataProperties", self.iri_pair(&s, o)?)
                    } else {
                        ("DisjointObjectProperties", format!("{} {}", self.ope(&s, tx)?, self.ope(o, tx)?))
                    }
                }
                (Some("domain"), _) => {
                    if ix.is_annotation_property(&s) {
                        ("AnnotationPropertyDomain", self.iri_pair(&s, o)?)
                    } else if ix.is_data_property(&s) {
                        ("DataPropertyDomain", format!("{} {}", self.property_iri(&s, "data property")?, self.ce(o, tx)?))
                    } else {
                        ("ObjectPropertyDomain", format!("{} {}", self.ope(&s, tx)?, self.ce(o, tx)?))
                    }
                }
                (Some("range"), _) => {
                    if ix.is_annotation_property(&s) {
                        ("AnnotationPropertyRange", self.iri_pair(&s, o)?)
                    } else if ix.is_data_property(&s) || (term_iri(&s).is_some() && ix.is_data_range(o)) {
                        ("DataPropertyRange", format!("{} {}", self.property_iri(&s, "data property")?, self.dr(o, tx)?))
                    } else {
                        ("ObjectPropertyRange", format!("{} {}", self.ope(&s, tx)?, self.ce(o, tx)?))
                    }
                }
                // A blank subject of owl:inverseOf is an ObjectInverseOf
                // expression, written where it is used. Only a named subject
                // is the main triple of InverseObjectProperties.
                (_, Some("inverseOf")) if term_iri(&s).is_some() => {
                    ("InverseObjectProperties", format!("{} {}", self.iri_of(&s), self.ope(o, tx)?))
                }
                (_, Some("hasKey")) => ("HasKey", format!("{} {}", self.ce(&s, tx)?, self.has_key(o, tx)?)),
                (_, Some("sameAs")) => ("SameIndividual", self.individual_pair(&s, o, tx)?),
                (_, Some("differentFrom")) => ("DifferentIndividuals", self.individual_pair(&s, o, tx)?),
                _ => return Err(String::new()),
            };
            Ok(self.main(i, kw, body, tx))
        })();
        match r {
            Err(e) if e.is_empty() => None,
            other => Some(other),
        }
    }

    fn iri_of(&self, t: &Term) -> String {
        term_iri(t).map(|n| self.iri(n)).unwrap_or_default()
    }

    fn iri_pair(&self, a: &Term, b: &Term) -> R<String> {
        Ok(format!("{} {}", self.property_iri(a, "property")?, self.property_iri(b, "property or IRI")?))
    }

    fn individual_pair(&self, a: &Term, b: &Term, tx: &mut Tx) -> R<String> {
        Ok(format!("{} {}", self.individual(a, tx)?, self.individual(b, tx)?))
    }

    /// `HasKey`'s single list, split back into its object and data parts. The
    /// reader writes the object properties first, so the split is at the
    /// first declared data property, which keeps the list order exact.
    fn has_key(&self, list: &Term, tx: &mut Tx) -> R<String> {
        let keys = self.list(list, false, tx)?;
        let split = keys.iter().position(|k| self.ix.is_data_property(k)).unwrap_or(keys.len());
        let objects = self.join(&keys[..split], tx, |w, t, tx| w.ope(t, tx))?;
        let data = self.join(&keys[split..], tx, |w, t, _| w.property_iri(t, "data property"))?;
        Ok(format!("({objects}) ({data})"))
    }

    /// The `rdf:type` axioms that pass 1 owns: property characteristics and
    /// the axioms rooted at a typed blank node (Section 2.3.3).
    fn typed_axiom(&self, i: usize, tx: &mut Tx) -> Option<R<Vec<String>>> {
        let t = self.t(i);
        let class = term_iri(&t.object)?;
        let s = subject_term(&t.subject);
        if let Some(local) = local(class, OWL) {
            let characteristic = match local {
                "FunctionalProperty" if self.ix.is_data_property(&s) => Some("FunctionalDataProperty"),
                "FunctionalProperty" => Some("FunctionalObjectProperty"),
                "InverseFunctionalProperty" => Some("InverseFunctionalObjectProperty"),
                "ReflexiveProperty" => Some("ReflexiveObjectProperty"),
                "IrreflexiveProperty" => Some("IrreflexiveObjectProperty"),
                "SymmetricProperty" => Some("SymmetricObjectProperty"),
                "AsymmetricProperty" => Some("AsymmetricObjectProperty"),
                "TransitiveProperty" => Some("TransitiveObjectProperty"),
                _ => None,
            };
            if let Some(kw) = characteristic {
                return Some((|| {
                    let body = if kw == "FunctionalDataProperty" {
                        self.property_iri(&s, "data property")?
                    } else {
                        self.ope(&s, tx)?
                    };
                    Ok(self.main(i, kw, body, tx))
                })());
            }
        }
        let NamedOrBlankNode::BlankNode(x) = &t.subject else {
            return None;
        };
        let kind = local(class, OWL)
            .filter(|l| matches!(*l, "AllDisjointClasses" | "AllDisjointProperties" | "AllDifferent" | "NegativePropertyAssertion"))
            .or_else(|| local(class, SWRL).filter(|l| *l == "Imp"))?;
        Some(self.rooted_axiom(i, x, kind, tx))
    }

    /// Section 2.3.3: an axiom whose translation is a typed blank node, with
    /// its annotations on that node.
    fn rooted_axiom(&self, ti: usize, x: &BlankNode, kind: &str, tx: &mut Tx) -> R<Vec<String>> {
        let node = self.ix.node(x);
        let shape: Vec<&str> = node.shape().into_iter().collect();
        let take = |p: &str, tx: &mut Tx| -> R<&'a Term> {
            let (i, t) = node.one(p).ok_or_else(|| format!("{p} given more than once"))?;
            tx.used.push(i);
            Ok(t)
        };
        let members = format!("{OWL}members");
        let (kw, body) = match kind {
            "AllDisjointClasses" | "AllDisjointProperties" | "AllDifferent" => {
                if shape != [members.as_str()] {
                    return Err(format!("an owl:{kind} node with components other than owl:members"));
                }
                // The binary case is owl:disjointWith (or its kin) in Table 1,
                // so a two-member node is not the image of anything.
                let items = self.list_min(take(&members, tx)?, 3, &format!("owl:{kind}"), tx)?;
                match kind {
                    "AllDisjointClasses" => ("DisjointClasses", self.join(&items, tx, |w, t, tx| w.ce(t, tx))?),
                    "AllDifferent" => ("DifferentIndividuals", self.join(&items, tx, |w, t, tx| w.individual(t, tx))?),
                    _ if items.iter().all(|t| term_iri(t).is_some()) && items.iter().any(|t| self.ix.is_data_property(t)) => {
                        ("DisjointDataProperties", self.join(&items, tx, |w, t, _| w.property_iri(t, "data property"))?)
                    }
                    _ => ("DisjointObjectProperties", self.join(&items, tx, |w, t, tx| w.ope(t, tx))?),
                }
            }
            "NegativePropertyAssertion" => {
                let source = format!("{OWL}sourceIndividual");
                let prop = format!("{OWL}assertionProperty");
                let target_i = format!("{OWL}targetIndividual");
                let target_v = format!("{OWL}targetValue");
                let mut want = vec![prop.as_str(), source.as_str()];
                let target = if node.reserved.contains_key(target_v.as_str()) { &target_v } else { &target_i };
                want.push(target.as_str());
                want.sort();
                if shape != want {
                    return Err("an owl:NegativePropertyAssertion whose components match no row of Table 1".into());
                }
                let a = self.individual(take(&source, tx)?, tx)?;
                let p = take(&prop, tx)?;
                let v = take(target, tx)?;
                match v {
                    Term::Literal(l) if target == &target_v => {
                        ("NegativeDataPropertyAssertion", format!("{} {a} {}", self.property_iri(p, "data property")?, self.literal(l)?))
                    }
                    v if target == &target_i => {
                        ("NegativeObjectPropertyAssertion", format!("{} {a} {}", self.ope(p, tx)?, self.individual(v, tx)?))
                    }
                    _ => return Err("an owl:NegativePropertyAssertion whose target has the wrong kind".into()),
                }
            }
            _ => ("DLSafeRule", self.rule(&node, &shape, tx)?),
        };
        tx.used.push(ti);
        tx.inline.push(x.clone());
        let anns = self.annotations(&x.clone().into(), 0, tx)?;
        Ok(Self::emit(kw, &[anns], &body))
    }

    // ---- SWRL ----------------------------------------------------------- //

    fn rule(&self, node: &Node<'a>, shape: &[&str], tx: &mut Tx) -> R<String> {
        let body = format!("{SWRL}body");
        let head = format!("{SWRL}head");
        if shape != [body.as_str(), head.as_str()] {
            return Err("a swrl:Imp without exactly one swrl:body and swrl:head".into());
        }
        let mut parts = Vec::new();
        for (kw, p) in [("Body", &body), ("Head", &head)] {
            let (i, list) = node.one(p).ok_or("a swrl:Imp body or head given more than once")?;
            tx.used.push(i);
            let atoms = self.list(list, true, tx)?;
            parts.push(format!("{kw}({})", self.join(&atoms, tx, |w, t, tx| w.atom(t, tx))?));
        }
        Ok(parts.join(" "))
    }

    fn variable(&self, t: &Term, tx: &mut Tx) -> Option<String> {
        let n = term_iri(t)?;
        if !self.ix.kinds.variable.contains(n) {
            return None;
        }
        let ty = Triple::new(n.clone(), NamedNode::new_unchecked(RDF_TYPE), NamedNode::new_unchecked(format!("{SWRL}Variable")));
        if let Some(&i) = self.ix.ids(&n.clone().into()).iter().find(|&&i| self.t(i) == &ty) {
            tx.used.push(i);
        }
        Some(format!("Variable({})", self.iri(n)))
    }

    fn i_arg(&self, t: &Term, tx: &mut Tx) -> R<String> {
        match self.variable(t, tx) {
            Some(v) => Ok(v),
            None => self.individual(t, tx),
        }
    }

    fn d_arg(&self, t: &Term, tx: &mut Tx) -> R<String> {
        match (self.variable(t, tx), t) {
            (Some(v), _) => Ok(v),
            (None, Term::Literal(l)) => self.literal(l),
            _ => Err("a SWRL data argument that is neither a variable nor a literal".into()),
        }
    }

    fn atom(&self, t: &Term, tx: &mut Tx) -> R<String> {
        let Some(b) = term_blank(t) else {
            return Err("a SWRL atom that is not a blank node".into());
        };
        let node = self.ix.node(b);
        let Some((ti, ty)) = node.one(RDF_TYPE) else {
            return Err("a SWRL atom without exactly one rdf:type".into());
        };
        let shape: Vec<&str> = node.shape().into_iter().filter_map(|p| p.strip_prefix(SWRL)).collect();
        if shape.len() != node.shape().len() {
            return Err("a SWRL atom with a predicate outside the swrl: vocabulary".into());
        }
        let take = |l: &str, tx: &mut Tx| -> R<&'a Term> {
            let (i, t) = node.one(&format!("{SWRL}{l}")).ok_or("a SWRL atom component given more than once")?;
            tx.used.push(i);
            Ok(t)
        };
        let kind = term_iri(ty).and_then(|c| local(c, SWRL)).unwrap_or("");
        let out = match (kind, &shape[..]) {
            ("ClassAtom", ["argument1", "classPredicate"]) => {
                let c = self.ce(take("classPredicate", tx)?, tx)?;
                format!("ClassAtom({c} {})", self.i_arg(take("argument1", tx)?, tx)?)
            }
            ("DataRangeAtom", ["argument1", "dataRange"]) => {
                let d = self.dr(take("dataRange", tx)?, tx)?;
                format!("DataRangeAtom({d} {})", self.d_arg(take("argument1", tx)?, tx)?)
            }
            ("IndividualPropertyAtom", ["argument1", "argument2", "propertyPredicate"]) => {
                let p = self.ope(take("propertyPredicate", tx)?, tx)?;
                let a = self.i_arg(take("argument1", tx)?, tx)?;
                format!("ObjectPropertyAtom({p} {a} {})", self.i_arg(take("argument2", tx)?, tx)?)
            }
            ("DatavaluedPropertyAtom", ["argument1", "argument2", "propertyPredicate"]) => {
                let p = self.property_iri(take("propertyPredicate", tx)?, "data property")?;
                let a = self.i_arg(take("argument1", tx)?, tx)?;
                format!("DataPropertyAtom({p} {a} {})", self.d_arg(take("argument2", tx)?, tx)?)
            }
            ("BuiltinAtom", ["arguments", "builtin"]) => {
                let f = self.property_iri(take("builtin", tx)?, "built-in")?;
                let args = self.list(take("arguments", tx)?, false, tx)?;
                let args = self.join(&args, tx, |w, t, tx| w.d_arg(t, tx))?;
                if args.is_empty() { format!("BuiltInAtom({f})") } else { format!("BuiltInAtom({f} {args})") }
            }
            (k @ ("SameIndividualAtom" | "DifferentIndividualsAtom"), ["argument1", "argument2"]) => {
                let a = self.i_arg(take("argument1", tx)?, tx)?;
                format!("{k}({a} {})", self.i_arg(take("argument2", tx)?, tx)?)
            }
            _ => return Err("a SWRL atom whose type and arguments match no atom the reader writes".into()),
        };
        tx.used.push(ti);
        tx.inline.push(b.clone());
        Ok(out)
    }

    // ---- the n-ary chains ----------------------------------------------- //

    /// `EquivalentClasses` and `EquivalentObjectProperties` over n operands
    /// are n-1 pairwise triples (Section 2.3.2). Written back pairwise they
    /// read back the same, EXCEPT where an operand is anonymous: two axioms
    /// would write it twice and the reader would make two nodes of it. So an
    /// anonymous node with exactly one incoming and one outgoing link joins
    /// its two links into one axiom.
    fn chains(&self, predicate: &str, run: &mut Run) {
        let edges: Vec<usize> = (0..self.ix.triples.len())
            .filter(|&i| self.t(i).predicate.as_str() == predicate)
            .collect();
        let mut out: HashMap<Term, Vec<usize>> = HashMap::new();
        let mut inc: HashMap<&Term, usize> = HashMap::new();
        for &e in &edges {
            out.entry(subject_term(&self.t(e).subject)).or_default().push(e);
            *inc.entry(&self.t(e).object).or_default() += 1;
        }
        let joint = |t: &Term| {
            term_blank(t).is_some() && inc.get(t) == Some(&1) && out.get(t).is_some_and(|v| v.len() == 1)
        };
        for &start in &edges {
            if joint(&subject_term(&self.t(start).subject)) {
                continue;
            }
            let mut path = vec![start];
            let mut cur = &self.t(start).object;
            while joint(cur) {
                let next = out[cur][0];
                if path.contains(&next) {
                    break;
                }
                path.push(next);
                cur = &self.t(next).object;
            }
            self.attempt(run, &path, |tx| self.chain(&path, predicate, tx));
        }
    }

    fn chain(&self, path: &[usize], predicate: &str, tx: &mut Tx) -> R<Vec<String>> {
        let mut operands: Vec<Term> = vec![subject_term(&self.t(path[0]).subject)];
        operands.extend(path.iter().map(|&e| self.t(e).object.clone()));
        let mut sets = None;
        for &e in path {
            let s = self.axiom_annotations(self.t(e), tx);
            match &sets {
                None => sets = Some(s),
                Some(prev) if *prev == s => {}
                Some(_) => return Err("the links of one n-ary axiom carry different annotations".into()),
            }
        }
        let sets = sets.unwrap_or_default();
        let (kw, body) = if predicate.ends_with("equivalentClass") {
            let (a, b) = (&operands[0], &operands[1]);
            let named_datatype = term_iri(a).is_some_and(|n| self.ix.kinds.datatype.contains(n));
            if operands.len() == 2 && term_iri(a).is_some() && (named_datatype || (term_blank(b).is_some() && self.ix.is_data_range(b))) {
                ("DatatypeDefinition", format!("{} {}", self.iri_of(a), self.dr(b, tx)?))
            } else {
                ("EquivalentClasses", self.join(&operands, tx, |w, t, tx| w.ce(t, tx))?)
            }
        } else if operands.iter().all(|t| term_iri(t).is_some()) && operands.iter().any(|t| self.ix.is_data_property(t)) {
            ("EquivalentDataProperties", self.join(&operands, tx, |w, t, _| w.property_iri(t, "data property"))?)
        } else {
            ("EquivalentObjectProperties", self.join(&operands, tx, |w, t, tx| w.ope(t, tx))?)
        };
        Ok(Self::emit(kw, &sets, &body))
    }

    // ---- passes --------------------------------------------------------- //

    /// Render one axiom transactionally: on success its triples are consumed
    /// and its lines kept; on failure nothing is consumed and every triple it
    /// touched carries the reason, so the report says why and not only what.
    fn attempt(&self, run: &mut Run, trigger: &[usize], f: impl FnOnce(&mut Tx) -> R<Vec<String>>) -> bool {
        let mut tx = Tx::default();
        tx.used.extend(trigger);
        match f(&mut tx) {
            Ok(lines) => {
                for &i in &tx.used {
                    run.consumed[i] = true;
                }
                for b in tx.inline {
                    *run.inline_uses.entry(b).or_default() += lines.len();
                }
                run.anons.extend(tx.anons);
                run.lines.extend(lines);
                true
            }
            // The empty reason is "this triple begins no axiom", which is not
            // a failure and must not displace a real reason.
            Err(why) if why.is_empty() => false,
            Err(why) => {
                for &i in trigger.iter().chain(&tx.used) {
                    let slot = run.why.entry(i).or_insert_with(|| why.clone());
                    if why < *slot {
                        *slot = why.clone();
                    }
                }
                false
            }
        }
    }

    fn header(&self, run: &mut Run) {
        let ontology_type = format!("{OWL}Ontology");
        let mut heads: Vec<usize> = (0..self.ix.triples.len())
            .filter(|&i| {
                let t = self.t(i);
                t.predicate.as_str() == RDF_TYPE && term_iri(&t.object).is_some_and(|c| c.as_str() == ontology_type)
            })
            .collect();
        // One header: the named ontology with the smallest IRI if there is
        // one, else an anonymous one. Any other owl:Ontology node is left in
        // the report, because OFN has one ontology per document.
        heads.sort_by_key(|&i| match &self.t(i).subject {
            NamedOrBlankNode::NamedNode(n) => (0, n.as_str().to_string()),
            NamedOrBlankNode::BlankNode(_) => (1, String::new()),
        });
        let Some(&head) = heads.first() else {
            run.header_synthesised = true;
            run.header_line = "Ontology(".into();
            return;
        };
        if heads.len() > 1 && matches!(self.t(heads[1]).subject, NamedOrBlankNode::BlankNode(_)) && matches!(self.t(head).subject, NamedOrBlankNode::BlankNode(_)) {
            // Two anonymous ontologies: which one is "the" header would depend
            // on blank-node labels, so neither is chosen.
            run.header_synthesised = true;
            run.header_line = "Ontology(".into();
            return;
        }
        run.consumed[head] = true;
        let subject = self.t(head).subject.clone();
        let mut line = "Ontology(".to_string();
        if let NamedOrBlankNode::NamedNode(o) = &subject {
            line.push_str(&self.iri(o));
            let version: Vec<usize> = self.ix.ids(&subject).iter().copied()
                .filter(|&i| self.t(i).predicate.as_str() == format!("{OWL}versionIRI") && term_iri(&self.t(i).object).is_some())
                .collect();
            if let [v] = version[..] {
                run.consumed[v] = true;
                line.push(' ');
                line.push_str(&self.iri_of(&self.t(v).object));
            }
        }
        run.header_line = line;
        for &i in self.ix.ids(&subject) {
            let t = self.t(i);
            if t.predicate.as_str() == format!("{OWL}imports") {
                if let Some(n) = term_iri(&t.object) {
                    run.consumed[i] = true;
                    run.header.insert(format!("Import({})", self.iri(n)));
                }
            } else if is_annotation_predicate(&t.predicate) {
                let mut tx = Tx::default();
                if let Ok(lines) = self.annotation(i, 0, &mut tx) {
                    for &u in &tx.used {
                        run.consumed[u] = true;
                    }
                    run.anons.extend(tx.anons);
                    run.header.extend(lines);
                }
            }
        }
    }

    fn run(&self) -> Run {
        let n = self.ix.triples.len();
        let mut run = Run { consumed: vec![false; n], ..Run::default() };
        self.header(&mut run);

        // Pass 1: every axiom that has a trigger triple of its own. Rendering
        // here never looks at what another axiom consumed, so the outcome does
        // not depend on the order triples are visited in, and that order is
        // the one thing that follows blank-node labels.
        for p in ["equivalentClass", "equivalentProperty"] {
            self.chains(&format!("{OWL}{p}"), &mut run);
        }
        for i in 0..n {
            self.attempt(&mut run, &[i], |tx| self.axiom(i, tx).unwrap_or_else(|| Err(String::new())));
        }

        // A reification or an axiom-rooted node that did not render is still
        // not an individual: writing its leftover triples as assertions about
        // `_:x` would put half of a failed axiom into the output.
        for x in self.ix.axiom_reifications.values().chain(self.ix.annotation_reifications.values()).flatten() {
            run.inline_uses.entry(x.clone()).or_insert(0);
        }
        for t in &self.ix.triples {
            if let (NamedOrBlankNode::BlankNode(x), true) = (&t.subject, t.predicate.as_str() == RDF_TYPE) {
                let rooted = term_iri(&t.object).is_some_and(|c| {
                    matches!(local(c, OWL), Some("AllDisjointClasses" | "AllDisjointProperties" | "AllDifferent" | "NegativePropertyAssertion"))
                        || local(c, SWRL) == Some("Imp")
                });
                if rooted {
                    run.inline_uses.entry(x.clone()).or_insert(0);
                }
            }
        }

        // Pass 2a: ClassAssertion of an ANONYMOUS class, before anything else
        // decides what a blank node is, so the class expressions it writes
        // inline are known when 2b asks.
        let inline_after_pass1: HashSet<BlankNode> = run.inline_uses.keys().cloned().collect();
        for i in 0..n {
            let t = self.t(i);
            if run.consumed[i] || t.predicate.as_str() != RDF_TYPE || term_blank(&t.object).is_none() {
                continue;
            }
            self.attempt(&mut run, &[i], |tx| {
                self.not_inline(&t.subject, &inline_after_pass1)?;
                let body = format!("{} {}", self.ce(&t.object, tx)?, self.individual(&subject_term(&t.subject), tx)?);
                Ok(self.main(i, "ClassAssertion", body, tx))
            });
        }

        // Pass 2b: declarations, named class assertions and the assertions a
        // plain triple is.
        let inline: HashSet<BlankNode> = run.inline_uses.keys().cloned().collect();
        for i in 0..n {
            if run.consumed[i] {
                continue;
            }
            let t = self.t(i);
            if t.predicate.as_str() == RDF_TYPE {
                let Some(class) = term_iri(&t.object) else { continue };
                if let (NamedOrBlankNode::NamedNode(e), Some(kind)) = (&t.subject, declaration_kind(class)) {
                    self.attempt(&mut run, &[i], |tx| {
                        Ok(self.main(i, "Declaration", format!("{kind}({})", self.iri(e)), tx))
                    });
                    continue;
                }
                if is_reserved(class) && !matches!(local(class, OWL), Some("Thing" | "Nothing")) {
                    continue;
                }
                self.attempt(&mut run, &[i], |tx| {
                    self.not_inline(&t.subject, &inline)?;
                    let body = format!("{} {}", self.iri(class), self.individual(&subject_term(&t.subject), tx)?);
                    Ok(self.main(i, "ClassAssertion", body, tx))
                });
                continue;
            }
            // A `swrl:` predicate is vocabulary inside a rule node, but OWL 2
            // does not reserve the namespace, so on an individual it is an
            // ordinary property: ORE file ore_ont_15753 asserts
            // `ObjectPropertyAssertion(swrl:argument2 _:genid71 ...)`.
            if !is_annotation_predicate(&t.predicate) && !t.predicate.as_str().starts_with(SWRL) {
                continue;
            }
            let mut undeclared = false;
            let ok = self.attempt(&mut run, &[i], |tx| {
                self.not_inline(&t.subject, &inline)?;
                if let Term::BlankNode(b) = &t.object {
                    self.not_inline(&b.clone().into(), &inline)?;
                }
                let s = self.individual(&subject_term(&t.subject), tx)?;
                let p = &t.predicate;
                let declared_ap = self.ix.kinds.annotation.contains(p)
                    || BUILT_IN_ANNOTATION_PROPERTIES.contains(&p.as_str());
                let (kw, body) = match &t.object {
                    Term::Literal(l) if !declared_ap && self.ix.kinds.data.contains(p) => {
                        ("DataPropertyAssertion", format!("{} {s} {}", self.iri(p), self.literal(l)?))
                    }
                    o @ (Term::NamedNode(_) | Term::BlankNode(_)) if !declared_ap && self.ix.kinds.object.contains(p) => {
                        ("ObjectPropertyAssertion", format!("{} {s} {}", self.iri(p), self.individual(o, tx)?))
                    }
                    o => {
                        undeclared = !declared_ap;
                        ("AnnotationAssertion", format!("{} {s} {}", self.iri(p), self.annotation_value(o, tx)?))
                    }
                };
                Ok(self.main(i, kw, body, tx))
            });
            if ok && undeclared {
                run.undeclared_property_assertions += 1;
            }
        }
        run
    }

    fn not_inline(&self, s: &NamedOrBlankNode, inline: &HashSet<BlankNode>) -> R<()> {
        match s {
            NamedOrBlankNode::BlankNode(b) if inline.contains(b) => {
                Err("a triple about an anonymous class expression, data range or list, which no OWL 2 axiom states".into())
            }
            _ => Ok(()),
        }
    }
}

fn cardinality_kind(k: &str) -> &'static str {
    match k {
        "minCardinality" => "MinCardinality",
        "maxCardinality" => "MaxCardinality",
        _ => "ExactCardinality",
    }
}

fn declaration_kind(class: &NamedNode) -> Option<&'static str> {
    if class.as_str() == RDFS_DATATYPE {
        return Some("Datatype");
    }
    match local(class, OWL)? {
        "Class" => Some("Class"),
        "ObjectProperty" => Some("ObjectProperty"),
        "DatatypeProperty" => Some("DataProperty"),
        "AnnotationProperty" => Some("AnnotationProperty"),
        "NamedIndividual" => Some("NamedIndividual"),
        _ => None,
    }
}

impl Run {
    fn finish(self, ix: &Index, names: &Names) -> OfnOutput {
        let mut not_exported: BTreeMap<(String, String), usize> = BTreeMap::new();
        let generic = "no OWL 2 axiom maps to this triple under the W3C mapping (Table 1)";
        let mut exported = 0;
        for (i, t) in ix.triples.iter().enumerate() {
            if self.consumed[i] {
                exported += 1;
                continue;
            }
            let construct = if t.predicate.as_str() == RDF_TYPE {
                match &t.object {
                    Term::NamedNode(c) => format!("rdf:type {}", names.iri(c)),
                    _ => "rdf:type (anonymous class)".into(),
                }
            } else {
                names.iri(&t.predicate)
            };
            let why = self.why.get(&i).filter(|w| !w.is_empty()).cloned().unwrap_or_else(|| generic.into());
            *not_exported.entry((construct, why)).or_default() += 1;
        }
        let mut unfaithful = Vec::new();
        let shared = self.inline_uses.values().filter(|&&n| n > 1).count();
        let inline: HashSet<&BlankNode> =
            self.inline_uses.iter().filter(|(_, n)| **n > 0).map(|(b, _)| b).collect();
        if shared > 0 {
            unfaithful.push(NotExported {
                construct: "anonymous expression shared by several axioms".into(),
                occurrences: shared,
                why: "OFN writes an anonymous expression inline, so a node the graph shares is written once per \
                      axiom and reads back as that many nodes"
                    .into(),
            });
        }
        let both = self.anons.iter().filter(|b| inline.contains(b)).count();
        if both > 0 {
            unfaithful.push(NotExported {
                construct: "blank node used both as an anonymous individual and as an anonymous expression".into(),
                occurrences: both,
                why: "OFN keeps the two apart, so the reader gives back two nodes".into(),
            });
        }
        let mut text = String::new();
        for (name, iri) in &names.declared {
            text.push_str(&format!("Prefix({name}:=<{iri}>)\n"));
        }
        text.push('\n');
        text.push_str(&self.header_line);
        text.push('\n');
        for l in &self.header {
            text.push_str(l);
            text.push('\n');
        }
        text.push('\n');
        // Declarations first, as the OWL API writes them, then every other
        // axiom; each group in byte order.
        let (declarations, axioms): (Vec<&String>, Vec<&String>) =
            self.lines.iter().partition(|l| l.starts_with("Declaration("));
        for l in declarations.into_iter().chain(axioms) {
            text.push_str(l);
            text.push('\n');
        }
        text.push_str(")\n");
        OfnOutput {
            text,
            axioms_written: self.lines.len(),
            triples_read: ix.triples.len(),
            triples_exported: exported,
            not_exported: not_exported
                .into_iter()
                .map(|((construct, why), occurrences)| NotExported { construct, occurrences, why })
                .collect(),
            not_round_trippable: unfaithful,
            header_synthesised: self.header_synthesised,
            undeclared_property_assertions: self.undeclared_property_assertions,
        }
    }
}
