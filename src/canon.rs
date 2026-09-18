//! **RDFC-1.0, the W3C RDF Dataset Canonicalization algorithm.**
//!
//! A certified run needs to say WHICH graph it was about, and a graph is only
//! nameable by its content if two serialisations of the same dataset hash to
//! the same value. Blank nodes are what make that hard: their labels are
//! process-local, so hashing them directly gives the same dataset a different
//! address after a reload, and a content-addressed certificate whose address
//! moves for no semantic reason is worse than no address at all.
//!
//! Decision 0010 records the three options and shipped the most conservative,
//! which is to REFUSE blank nodes in certified input. That is not viable:
//! 168 of the 288 tracked `.ttl` files in this repository use blank-node
//! syntax, and a SHACL property shape is a blank node almost by construction.
//! Refusing them would refuse most of the corpus.
//!
//! ## Why this is written here rather than taken from a crate
//!
//! `rdf-canon` 0.15.3 implements RDFC-1.0 and is pinned to `oxrdf ^0.2.4`.
//! This tree is on `oxrdf 0.3`, and there has been no release in fifteen
//! months, so adopting it means a SECOND copy of oxrdf plus a conversion layer
//! that is itself untrusted. `sophia_c14n` means an entire second RDF stack.
//!
//! A canonicaliser is a trusted component: if it gives two different datasets
//! one address, a certificate vouches for an input nobody supplied. Paying
//! thousands of unverified lines for that is the trade decision 0014 declined
//! for Dafny. This module is a few hundred lines measured against the W3C
//! suite in `tests/w3c-rdfc10/`, which is the standard this repository holds
//! its own gates to.
//!
//! ## The poison budget is part of the algorithm, not a safety valve
//!
//! Section 4.4 of the Recommendation documents dataset poisoning: blank-node
//! symmetry drives `hash_n_degree_quads` superlinear, and the specification
//! tells implementers to bound the work. The bound here is a call budget, and
//! exceeding it is a REFUSAL with a named reason rather than a truncated
//! answer, because a canonical form computed under a cut-off is not canonical.
//! The W3C suite carries a negative test for exactly this and it is run.

use oxigraph::model::{BlankNode, GraphName, NamedOrBlankNode, Quad, Term};
use sha2::{Digest, Sha256, Sha384};
use std::collections::BTreeMap;

/// How many times `hash_n_degree_quads` may be entered for one dataset.
///
/// 4000 is what the reference implementations use. It is high enough that no
/// test in the W3C suite marked `low` or `medium` complexity comes near it,
/// and low enough that a poisoned dataset is refused in well under a second.
pub const DEFAULT_CALL_BUDGET: usize = 4000;

#[derive(Debug)]
pub enum CanonError {
    /// The work budget was exhausted. Named separately from every other error
    /// because it is not a statement about the dataset being malformed: it is
    /// a refusal to spend unbounded time on a dataset shaped to cost it.
    BudgetExhausted { budget: usize },
}

impl std::fmt::Display for CanonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BudgetExhausted { budget } => write!(
                f,
                "canonicalisation exceeded its budget of {budget} hash_n_degree_quads calls. \
                 RDFC-1.0 section 4.4 documents this as dataset poisoning: blank-node symmetry \
                 can drive the algorithm superlinear, and a canonical form computed under a \
                 cut-off is not canonical, so this is refused rather than answered"
            ),
        }
    }
}

impl std::error::Error for CanonError {}

/// Assigns `<prefix>0`, `<prefix>1`, ... in the order identifiers are first
/// seen.
///
/// The PREFIX is not cosmetic. RDFC-1.0 uses `c14n` for the canonical issuer
/// and `b` for the temporary issuers that explore permutations, and the
/// issued identifier is concatenated into the path string that
/// `hash_n_degree_quads` compares to pick a permutation. Issuing `c14n0`
/// where the algorithm says `b0` changes those comparisons and therefore the
/// labels, which is exactly how this implementation first failed W3C tests
/// 044, 045, 046 and 075 while passing the other sixty.
#[derive(Clone, Debug)]
struct Issuer {
    prefix: &'static str,
    issued: BTreeMap<String, String>,
    order: Vec<String>,
}

impl Issuer {
    fn new(prefix: &'static str) -> Self {
        Self { prefix, issued: BTreeMap::new(), order: Vec::new() }
    }

    fn issue(&mut self, existing: &str) -> String {
        if let Some(id) = self.issued.get(existing) {
            return id.clone();
        }
        let id = format!("{}{}", self.prefix, self.order.len());
        self.issued.insert(existing.to_string(), id.clone());
        self.order.push(existing.to_string());
        id
    }

    fn get(&self, existing: &str) -> Option<&String> {
        self.issued.get(existing)
    }
}

/// The hash RDFC-1.0 is parameterised over.
///
/// The Recommendation fixes SHA-256 as the default and permits others; the
/// W3C suite carries a SHA-384 variant of the diamond test, which is how this
/// parameter earns its place rather than being generality for its own sake.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HashAlgorithm {
    #[default]
    Sha256,
    Sha384,
}

impl HashAlgorithm {
    fn hex(self, bytes: impl AsRef<[u8]>) -> String {
        match self {
            Self::Sha256 => format!("{:x}", Sha256::digest(bytes.as_ref())),
            Self::Sha384 => format!("{:x}", Sha384::digest(bytes.as_ref())),
        }
    }
}

/// One quad as canonical N-Quads, terminated. `Display` gives everything but
/// the full stop.
fn nquad(q: &Quad) -> String {
    format!("{q} .\n")
}

fn blank_of_subject(s: &NamedOrBlankNode) -> Option<&str> {
    match s {
        NamedOrBlankNode::BlankNode(b) => Some(b.as_str()),
        _ => None,
    }
}

fn blank_of_term(t: &Term) -> Option<&str> {
    match t {
        Term::BlankNode(b) => Some(b.as_str()),
        _ => None,
    }
}

fn blank_of_graph(g: &GraphName) -> Option<&str> {
    match g {
        GraphName::BlankNode(b) => Some(b.as_str()),
        _ => None,
    }
}

/// Every blank node mentioned by a quad, by position.
fn blanks(q: &Quad) -> Vec<&str> {
    let mut out = Vec::new();
    if let Some(b) = blank_of_subject(&q.subject) {
        out.push(b);
    }
    if let Some(b) = blank_of_term(&q.object) {
        out.push(b);
    }
    if let Some(b) = blank_of_graph(&q.graph_name) {
        out.push(b);
    }
    out
}

/// Rewrite a quad's blank nodes through `f`.
fn relabel(q: &Quad, f: &mut impl FnMut(&str) -> String) -> Quad {
    let subject = match &q.subject {
        NamedOrBlankNode::BlankNode(b) => {
            NamedOrBlankNode::BlankNode(BlankNode::new_unchecked(f(b.as_str())))
        }
        other => other.clone(),
    };
    let object = match &q.object {
        Term::BlankNode(b) => Term::BlankNode(BlankNode::new_unchecked(f(b.as_str()))),
        other => other.clone(),
    };
    let graph_name = match &q.graph_name {
        GraphName::BlankNode(b) => GraphName::BlankNode(BlankNode::new_unchecked(f(b.as_str()))),
        other => other.clone(),
    };
    Quad {
        subject,
        predicate: q.predicate.clone(),
        object,
        graph_name,
    }
}

struct State<'a> {
    /// Blank node identifier to the quads mentioning it.
    to_quads: BTreeMap<&'a str, Vec<&'a Quad>>,
    canonical: Issuer,
    budget: usize,
    spent: usize,
    hash: HashAlgorithm,
}

impl<'a> State<'a> {
    /// RDFC-1.0 section 4.6, Hash First Degree Quads.
    ///
    /// The reference blank node becomes `_:a` and every other blank node
    /// becomes `_:z`, so the hash describes the node's immediate shape without
    /// depending on any label.
    fn hash_first_degree(&self, reference: &str) -> String {
        let mut lines: Vec<String> = self
            .to_quads
            .get(reference)
            .map(|qs| {
                qs.iter()
                    .map(|q| {
                        nquad(&relabel(q, &mut |b| {
                            if b == reference { "a".to_string() } else { "z".to_string() }
                        }))
                    })
                    .collect()
            })
            .unwrap_or_default();
        lines.sort();
        self.hash.hex(lines.concat())
    }

    /// RDFC-1.0 section 4.7, Hash Related Blank Node.
    fn hash_related(
        &self,
        related: &str,
        quad: &Quad,
        issuer: &Issuer,
        position: char,
    ) -> String {
        let identifier = if let Some(id) = self.canonical.get(related) {
            format!("_:{id}")
        } else if let Some(id) = issuer.get(related) {
            format!("_:{id}")
        } else {
            self.hash_first_degree(related)
        };
        let mut input = String::new();
        input.push(position);
        if position != 'g' {
            input.push('<');
            input.push_str(quad.predicate.as_str());
            input.push('>');
        }
        input.push_str(&identifier);
        self.hash.hex(input)
    }

    /// RDFC-1.0 section 4.8, Hash N-Degree Quads.
    fn hash_n_degree(&mut self, identifier: &str, issuer: &Issuer) -> Result<(String, Issuer), CanonError> {
        self.spent += 1;
        if self.spent > self.budget {
            return Err(CanonError::BudgetExhausted { budget: self.budget });
        }

        // Hash to the related blank nodes that produced it.
        let mut hn: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let quads: Vec<&Quad> = self.to_quads.get(identifier).cloned().unwrap_or_default();
        for q in &quads {
            for (position, related) in [
                ('s', blank_of_subject(&q.subject)),
                ('o', blank_of_term(&q.object)),
                ('g', blank_of_graph(&q.graph_name)),
            ] {
                let Some(related) = related else { continue };
                if related == identifier {
                    continue;
                }
                let h = self.hash_related(related, q, issuer, position);
                hn.entry(h).or_default().push(related.to_string());
            }
        }

        let mut data_to_hash = String::new();
        let mut issuer = issuer.clone();
        for (related_hash, mut nodes) in hn {
            data_to_hash.push_str(&related_hash);
            let mut chosen_path = String::new();
            let mut chosen_issuer: Option<Issuer> = None;

            nodes.sort();
            nodes.dedup_by(|a, b| a == b && false); // keep duplicates: they are distinct positions
            for permutation in permutations(&nodes) {
                let mut issuer_copy = issuer.clone();
                let mut path = String::new();
                let mut recursion: Vec<String> = Vec::new();
                let mut abandoned = false;

                for related in &permutation {
                    if let Some(id) = self.canonical.get(related) {
                        path.push_str("_:");
                        path.push_str(id);
                    } else {
                        if issuer_copy.get(related).is_none() {
                            recursion.push(related.clone());
                        }
                        path.push_str("_:");
                        path.push_str(&issuer_copy.issue(related));
                    }
                    if !chosen_path.is_empty() && path.len() >= chosen_path.len() && path > chosen_path {
                        abandoned = true;
                        break;
                    }
                }
                if abandoned {
                    continue;
                }

                for related in &recursion {
                    let (hash, next) = self.hash_n_degree(related, &issuer_copy)?;
                    path.push_str("_:");
                    path.push_str(&issuer_copy.issue(related));
                    path.push('<');
                    path.push_str(&hash);
                    path.push('>');
                    issuer_copy = next;
                    if !chosen_path.is_empty() && path.len() >= chosen_path.len() && path > chosen_path {
                        abandoned = true;
                        break;
                    }
                }
                if abandoned {
                    continue;
                }

                if chosen_path.is_empty() || path < chosen_path {
                    chosen_path = path;
                    chosen_issuer = Some(issuer_copy);
                }
            }

            data_to_hash.push_str(&chosen_path);
            if let Some(c) = chosen_issuer {
                issuer = c;
            }
        }
        Ok((self.hash.hex(&data_to_hash), issuer))
    }
}

/// Every permutation of `items`, in a deterministic order.
fn permutations(items: &[String]) -> Vec<Vec<String>> {
    if items.is_empty() {
        return vec![Vec::new()];
    }
    let mut out = Vec::new();
    for i in 0..items.len() {
        let mut rest = items.to_vec();
        let head = rest.remove(i);
        for mut tail in permutations(&rest) {
            let mut one = vec![head.clone()];
            one.append(&mut tail);
            out.push(one);
        }
    }
    out
}

/// Canonical blank-node labels for a dataset, as a map from the input label.
pub fn canonical_labels(
    quads: &[Quad],
    budget: usize,
    hash: HashAlgorithm,
) -> Result<BTreeMap<String, String>, CanonError> {
    let mut to_quads: BTreeMap<&str, Vec<&Quad>> = BTreeMap::new();
    for q in quads {
        for b in blanks(q) {
            to_quads.entry(b).or_default().push(q);
        }
    }
    let mut state = State {
        to_quads,
        canonical: Issuer::new("c14n"),
        budget,
        spent: 0,
        hash,
    };

    // 4.4.3 step 3: first-degree hashes, and anything uniquely hashed is
    // settled without recursion.
    let mut by_hash: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let ids: Vec<String> = state.to_quads.keys().map(|s| (*s).to_string()).collect();
    for id in &ids {
        by_hash.entry(state.hash_first_degree(id)).or_default().push(id.to_string());
    }

    let mut ambiguous: Vec<(String, Vec<String>)> = Vec::new();
    for (hash, mut nodes) in by_hash {
        if nodes.len() == 1 {
            state.canonical.issue(&nodes[0]);
        } else {
            nodes.sort();
            ambiguous.push((hash, nodes));
        }
    }

    // 4.4.3 step 5: the rest, in hash order, each explored with a temporary
    // issuer so the choice of label cannot depend on input order.
    for (_, nodes) in ambiguous {
        let mut hash_path: Vec<(String, Issuer)> = Vec::new();
        for n in &nodes {
            if state.canonical.get(n).is_some() {
                continue;
            }
            let mut temp = Issuer::new("b");
            temp.issue(n);
            hash_path.push(state.hash_n_degree(n, &temp)?);
        }
        hash_path.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, issuer) in hash_path {
            for existing in &issuer.order {
                state.canonical.issue(existing);
            }
        }
    }

    Ok(state
        .canonical
        .issued
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect())
}

/// The dataset in canonical N-Quads: blank nodes relabelled `c14n0`, `c14n1`,
/// ..., every quad serialised, sorted, and concatenated.
///
/// This is the form to hash. Two serialisations of the same dataset produce
/// the same string whatever their blank-node labels were.
pub fn canonical_nquads(
    quads: &[Quad],
    budget: usize,
    hash: HashAlgorithm,
) -> Result<String, CanonError> {
    let labels = canonical_labels(quads, budget, hash)?;
    let mut lines: Vec<String> = quads
        .iter()
        .map(|q| {
            nquad(&relabel(q, &mut |b| {
                labels.get(b).cloned().unwrap_or_else(|| b.to_string())
            }))
        })
        .collect();
    lines.sort();
    lines.dedup();
    Ok(lines.concat())
}

/// SHA-256 of the canonical form. This is the address of a dataset.
pub fn digest(
    quads: &[Quad],
    budget: usize,
    hash: HashAlgorithm,
) -> Result<String, CanonError> {
    Ok(hash.hex(canonical_nquads(quads, budget, hash)?))
}
