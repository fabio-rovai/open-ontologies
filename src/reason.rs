use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::graph::GraphStore;

// Well-known IRIs
const RDF_TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const RDFS_SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const RDFS_SUBPROP: &str = "<http://www.w3.org/2000/01/rdf-schema#subPropertyOf>";
const RDFS_DOMAIN: &str = "<http://www.w3.org/2000/01/rdf-schema#domain>";
const RDFS_RANGE: &str = "<http://www.w3.org/2000/01/rdf-schema#range>";
const OWL_TRANSITIVE: &str = "<http://www.w3.org/2002/07/owl#TransitiveProperty>";
const OWL_SYMMETRIC: &str = "<http://www.w3.org/2002/07/owl#SymmetricProperty>";
const OWL_INVERSE: &str = "<http://www.w3.org/2002/07/owl#inverseOf>";
const OWL_SAMEAS: &str = "<http://www.w3.org/2002/07/owl#sameAs>";
const OWL_EQUIV_CLASS: &str = "<http://www.w3.org/2002/07/owl#equivalentClass>";
const OWL_EQUIV_PROP: &str = "<http://www.w3.org/2002/07/owl#equivalentProperty>";
const OWL_SOME_VALUES: &str = "<http://www.w3.org/2002/07/owl#someValuesFrom>";
const OWL_ALL_VALUES: &str = "<http://www.w3.org/2002/07/owl#allValuesFrom>";
const OWL_HAS_VALUE: &str = "<http://www.w3.org/2002/07/owl#hasValue>";
const OWL_ON_PROPERTY: &str = "<http://www.w3.org/2002/07/owl#onProperty>";
const OWL_INTERSECTION: &str = "<http://www.w3.org/2002/07/owl#intersectionOf>";
const OWL_UNION: &str = "<http://www.w3.org/2002/07/owl#unionOf>";
const OWL_ONEOF: &str = "<http://www.w3.org/2002/07/owl#oneOf>";
const RDF_FIRST: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>";
const RDF_REST: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>";
const RDF_NIL: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>";

// The vocabulary the clash detector reads. None of it drives a rule: nothing in
// the table above concludes a triple from any of these terms. They are read
// once, after the fixpoint, by `find_clashes`.
const OWL_DISJOINT_WITH: &str = "<http://www.w3.org/2002/07/owl#disjointWith>";
const OWL_NOTHING: &str = "<http://www.w3.org/2002/07/owl#Nothing>";
const OWL_COMPLEMENT_OF: &str = "<http://www.w3.org/2002/07/owl#complementOf>";
const OWL_IRREFLEXIVE: &str = "<http://www.w3.org/2002/07/owl#IrreflexiveProperty>";
const OWL_ASYMMETRIC: &str = "<http://www.w3.org/2002/07/owl#AsymmetricProperty>";
const OWL_PROP_DISJOINT_WITH: &str = "<http://www.w3.org/2002/07/owl#propertyDisjointWith>";
const OWL_DIFFERENT_FROM: &str = "<http://www.w3.org/2002/07/owl#differentFrom>";
const OWL_MAX_CARDINALITY: &str = "<http://www.w3.org/2002/07/owl#maxCardinality>";
const OWL_SOURCE_INDIVIDUAL: &str = "<http://www.w3.org/2002/07/owl#sourceIndividual>";
const OWL_ASSERTION_PROPERTY: &str = "<http://www.w3.org/2002/07/owl#assertionProperty>";
const OWL_TARGET_INDIVIDUAL: &str = "<http://www.w3.org/2002/07/owl#targetIndividual>";
const OWL_TARGET_VALUE: &str = "<http://www.w3.org/2002/07/owl#targetValue>";

/// A triple over interned ids.
type Fact = (u32, u32, u32);

/// One line of a certificate: the rule, what it concluded, and the premises
/// it read, in the order `lean/OOCert/Rules.lean` documents for that rule.
struct Derivation {
    rule: &'static str,
    conclusion: Fact,
    premises: Vec<Fact>,
}


// ── The rules that conclude `false` ─────────────────────────────────────────
//
// Seventeen rules of the OWL 2 RL profile conclude `false` rather than a
// triple: `eq-diff1`, `eq-diff2`, `eq-diff3`, `prp-irp`, `prp-asyp`,
// `prp-pdw`, `prp-adp`, `prp-npa1`, `prp-npa2`, `cls-nothing2`, `cls-com`,
// `cls-maxc1`, `cls-maxqc1`, `cls-maxqc2`, `cax-dw`, `cax-adc` and
// `dt-not-type`. (Seventeen is the count extracted mechanically from the W3C
// tables, and `lean/OOCert/Refute.lean` now agrees; it said sixteen until the
// list was checked against the source. `dt-not-type` is the one that is not
// merely absent from the Lean but INEXPRESSIBLE there, because that semantics
// has no datatype value space and a literal is its spelling like any other
// term.)
//
// None of them is in the fixpoint's rule table, and none of them could be:
// `Derivation.conclusion` is a triple. They are looked for HERE, once, over
// the closure the fixpoint reached, and reported in the separate `oo-refute/1`
// format.
//
// **Only `cax-dw` can be certified**, and the reason is not this file's.
// `OOCert.RefuteConditions` in `lean/OOCert/Refute.lean` carries exactly one
// semantic field, for `cax-dw`, and `oo-refute` refuses a refutation naming any
// other rule with exit 2 rather than judging it. So a clash of any other rule
// found here is an engine opinion with nothing behind it, it is reported under
// a different word from the certified one, and no refutation file is written
// for it.

/// One way the closure contradicts itself: the OWL 2 RL rule that concludes
/// `false` from it, and the premises that rule reads, in the order the W3C
/// table writes them (which is also the order `OOCert.checkRefuteStep` matches
/// on, and a refutation with the right triples in the wrong order is rejected).
struct Clash {
    rule: &'static str,
    premises: Vec<Fact>,
}

/// The clash rules `lean/` holds a semantic condition for, so that a refutation
/// naming one can be CHECKED rather than merely asserted.
///
/// One rule, and the list is not this file's to extend: adding a name here
/// without a matching field in `OOCert.RefuteConditions` and a matching arm in
/// `OOCert.checkRefuteStep` would make the engine write a file the checker
/// exits 2 on, which is the one failure mode this layer exists to prevent.
///
/// One constant, read both by the emitter and by the report, so the two cannot
/// drift into disagreeing about what was certified.
const CLASH_RULES_CERTIFIABLE: &[&str] = &["cax-dw"];

fn clash_is_certifiable(rule: &str) -> bool {
    CLASH_RULES_CERTIFIABLE.contains(&rule)
}

/// The clash rules this engine does NOT look for, each with the reason. Listed
/// so that "no clash found" is never read as "consistent": the honest reading
/// of a clean run is that none of the rules below was even tried.
const CLASH_RULES_NOT_DETECTED: &[(&str, &str)] = &[
    ("cax-adc", "owl:AllDisjointClasses states its members in an RDF list, and this detector reads \
                 no lists. The pairwise owl:disjointWith spelling of the same axiom IS detected."),
    ("prp-adp", "owl:AllDisjointProperties states its members in an RDF list, and this detector \
                 reads no lists. The pairwise owl:propertyDisjointWith spelling IS detected."),
    ("eq-diff2", "owl:AllDifferent with owl:members is an RDF list. The pairwise owl:differentFrom \
                  spelling IS detected, by eq-diff1."),
    ("eq-diff3", "owl:AllDifferent with owl:distinctMembers is an RDF list, as above."),
    ("cls-maxqc1", "owl:maxQualifiedCardinality needs the qualifying owl:onClass and a type check \
                    on the value. Not implemented; owl:maxCardinality 0 IS detected, by cls-maxc1."),
    ("cls-maxqc2", "the owl:Thing case of the same rule, and not implemented for the same reason."),
    ("dt-not-type", "an ill-typed literal needs a datatype VALUE space. This engine, and the Lean \
                     semantics under it, read a literal as its N-Triples spelling and compare \
                     nothing by value, so there is no notion here of a literal outside its type."),
];

/// The interned vocabulary [`find_clashes`] reads. Gathered into one value so
/// the detector takes three arguments rather than eighteen.
struct ClashVocab {
    rdf_type: u32,
    disjoint_with: u32,
    nothing: u32,
    complement_of: u32,
    irreflexive: u32,
    asymmetric: u32,
    prop_disjoint_with: u32,
    same_as: u32,
    different_from: u32,
    max_cardinality: u32,
    on_property: u32,
    source_individual: u32,
    assertion_property: u32,
    target_individual: u32,
    target_value: u32,
}

/// Zero, in whichever spelling the store hands back for a
/// `xsd:nonNegativeInteger` zero. Only the lexical form is read, because
/// nothing else in this engine compares a literal by value and a detector that
/// did would be claiming a datatype semantics the checker underneath it does
/// not have.
fn literal_is_zero(term: &str) -> bool {
    let Some(rest) = term.strip_prefix('"') else {
        return false;
    };
    let Some(end) = rest.find('"') else {
        return false;
    };
    rest[..end].trim().parse::<i128>().map(|n| n == 0).unwrap_or(false)
}

/// Look for a contradiction in the closure the fixpoint reached.
///
/// Deterministic: the result is sorted on the N-Triples spelling of the
/// premises and duplicates are dropped, so two runs over one store produce the
/// same refutation file byte for byte.
fn find_clashes(closure: &HashSet<Fact>, interner: &Interner, v: &ClashVocab) -> Vec<Clash> {
    // One pass for every index, because the closure is the large thing here.
    let mut by_class: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut disjoint: Vec<Fact> = Vec::new();
    let mut complement: Vec<Fact> = Vec::new();
    let mut irreflexive: HashSet<u32> = HashSet::new();
    let mut asymmetric: HashSet<u32> = HashSet::new();
    let mut prop_disjoint: Vec<Fact> = Vec::new();
    let mut different: Vec<Fact> = Vec::new();
    let mut on_property: HashMap<u32, u32> = HashMap::new();
    let mut max_zero: Vec<(u32, u32)> = Vec::new();
    let mut npa_source: HashMap<u32, u32> = HashMap::new();
    let mut npa_prop: HashMap<u32, u32> = HashMap::new();
    let mut npa_target_ind: HashMap<u32, u32> = HashMap::new();
    let mut npa_target_val: HashMap<u32, u32> = HashMap::new();
    for &(s, p, o) in closure.iter() {
        if p == v.rdf_type {
            by_class.entry(o).or_default().push(s);
            if o == v.irreflexive {
                irreflexive.insert(s);
            }
            if o == v.asymmetric {
                asymmetric.insert(s);
            }
        } else if p == v.disjoint_with {
            disjoint.push((s, p, o));
        } else if p == v.complement_of {
            complement.push((s, p, o));
        } else if p == v.prop_disjoint_with {
            prop_disjoint.push((s, p, o));
        } else if p == v.different_from {
            different.push((s, p, o));
        } else if p == v.on_property {
            on_property.insert(s, o);
        } else if p == v.max_cardinality && literal_is_zero(interner.resolve(o)) {
            max_zero.push((s, o));
        } else if p == v.source_individual {
            npa_source.insert(s, o);
        } else if p == v.assertion_property {
            npa_prop.insert(s, o);
        } else if p == v.target_individual {
            npa_target_ind.insert(s, o);
        } else if p == v.target_value {
            npa_target_val.insert(s, o);
        }
    }

    let mut out: Vec<Clash> = Vec::new();

    // cax-dw: c1 owl:disjointWith c2, x rdf:type c1, x rdf:type c2.
    // The one rule a refutation can be written for.
    for &(c1, dw, c2) in &disjoint {
        let Some(xs) = by_class.get(&c1) else { continue };
        for &x in xs {
            if closure.contains(&(x, v.rdf_type, c2)) {
                out.push(Clash {
                    rule: "cax-dw",
                    premises: vec![(c1, dw, c2), (x, v.rdf_type, c1), (x, v.rdf_type, c2)],
                });
            }
        }
    }

    // cls-com: the same shape, over owl:complementOf.
    for &(c1, co, c2) in &complement {
        let Some(xs) = by_class.get(&c1) else { continue };
        for &x in xs {
            if closure.contains(&(x, v.rdf_type, c2)) {
                out.push(Clash {
                    rule: "cls-com",
                    premises: vec![(c1, co, c2), (x, v.rdf_type, c1), (x, v.rdf_type, c2)],
                });
            }
        }
    }

    // cls-nothing2: x rdf:type owl:Nothing.
    if let Some(xs) = by_class.get(&v.nothing) {
        for &x in xs {
            out.push(Clash {
                rule: "cls-nothing2",
                premises: vec![(x, v.rdf_type, v.nothing)],
            });
        }
    }

    // eq-diff1: x owl:sameAs y, x owl:differentFrom y. The spec's pattern
    // exactly, with both premises on the same subject.
    for &(x, df, y) in &different {
        if closure.contains(&(x, v.same_as, y)) {
            out.push(Clash {
                rule: "eq-diff1",
                premises: vec![(x, v.same_as, y), (x, df, y)],
            });
        }
    }

    // prp-irp, prp-asyp and prp-pdw each need a second scan over the closure,
    // and the scan is skipped entirely when none of their vocabulary is
    // present, which is the case for every ontology this repository ships.
    if !irreflexive.is_empty() || !asymmetric.is_empty() || !prop_disjoint.is_empty() {
        for &(x, p, y) in closure.iter() {
            // prp-irp: p rdf:type owl:IrreflexiveProperty, x p x.
            if x == y && irreflexive.contains(&p) {
                out.push(Clash {
                    rule: "prp-irp",
                    premises: vec![(p, v.rdf_type, v.irreflexive), (x, p, y)],
                });
            }
            // prp-asyp: p rdf:type owl:AsymmetricProperty, x p y, y p x.
            if asymmetric.contains(&p) && closure.contains(&(y, p, x)) {
                out.push(Clash {
                    rule: "prp-asyp",
                    premises: vec![(p, v.rdf_type, v.asymmetric), (x, p, y), (y, p, x)],
                });
            }
            // prp-pdw: p1 owl:propertyDisjointWith p2, x p1 y, x p2 y.
            for &(p1, pdw, p2) in &prop_disjoint {
                if p == p1 && closure.contains(&(x, p2, y)) {
                    out.push(Clash {
                        rule: "prp-pdw",
                        premises: vec![(p1, pdw, p2), (x, p1, y), (x, p2, y)],
                    });
                }
            }
        }
    }

    // cls-maxc1: x owl:maxCardinality 0, x owl:onProperty p, u rdf:type x, u p y.
    // The (subject, predicate) index is built only when the vocabulary is
    // present, so an ontology with no cardinality-zero restriction pays for
    // none of it.
    if !max_zero.is_empty() {
        let mut values: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
        for &(s, p, o) in closure.iter() {
            values.entry((s, p)).or_default().push(o);
        }
        for &(r, zero) in &max_zero {
            let Some(&prop) = on_property.get(&r) else { continue };
            let Some(us) = by_class.get(&r) else { continue };
            for &u in us {
                let Some(ys) = values.get(&(u, prop)) else { continue };
                for &y in ys {
                    out.push(Clash {
                        rule: "cls-maxc1",
                        premises: vec![
                            (r, v.max_cardinality, zero),
                            (r, v.on_property, prop),
                            (u, v.rdf_type, r),
                            (u, prop, y),
                        ],
                    });
                }
            }
        }
    }

    // prp-npa1 / prp-npa2: a negative property assertion its own graph
    // contradicts. One assertion per node is read; a node carrying two
    // owl:sourceIndividual triples is malformed and the last one seen wins.
    for (&node, &i) in &npa_source {
        let Some(&p) = npa_prop.get(&node) else { continue };
        if let Some(&j) = npa_target_ind.get(&node)
            && closure.contains(&(i, p, j))
        {
            out.push(Clash {
                rule: "prp-npa1",
                premises: vec![
                    (node, v.source_individual, i),
                    (node, v.assertion_property, p),
                    (node, v.target_individual, j),
                    (i, p, j),
                ],
            });
        }
        if let Some(&lt) = npa_target_val.get(&node)
            && closure.contains(&(i, p, lt))
        {
            out.push(Clash {
                rule: "prp-npa2",
                premises: vec![
                    (node, v.source_individual, i),
                    (node, v.assertion_property, p),
                    (node, v.target_value, lt),
                    (i, p, lt),
                ],
            });
        }
    }

    // Deterministic order, so the refutation file does not depend on hash
    // iteration order and two runs over one store agree byte for byte.
    let mut keyed: Vec<(String, Clash)> = out
        .into_iter()
        .map(|c| {
            let mut k = String::from(c.rule);
            for &(s, p, o) in &c.premises {
                k.push('\t');
                k.push_str(interner.resolve(s));
                k.push('\t');
                k.push_str(interner.resolve(p));
                k.push('\t');
                k.push_str(interner.resolve(o));
            }
            (k, c)
        })
        .collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    keyed.dedup_by(|a, b| a.0 == b.0);
    keyed.into_iter().map(|(_, c)| c).collect()
}

/// The derivation steps a refutation must carry so that every premise it cites
/// is either asserted or concluded by a step BEFORE it.
///
/// Returns indices into `derivations`, ascending, which is a valid order:
/// `derivations` records each conclusion the first time the fixpoint reached
/// it, and every premise of that step was in the closure of an earlier
/// iteration, so the recorded order is already a topological one.
///
/// `None` when a premise is neither asserted nor derived, which cannot happen
/// for a clash found in the closure of a certified run and would be a bug here
/// rather than a refutation.
fn refutation_prefix(
    premises: &[Fact],
    derivations: &[Derivation],
    asserted: &HashSet<Fact>,
) -> Option<Vec<usize>> {
    let mut first: HashMap<Fact, usize> = HashMap::new();
    for (i, d) in derivations.iter().enumerate() {
        first.entry(d.conclusion).or_insert(i);
    }
    let mut needed: HashSet<usize> = HashSet::new();
    let mut work: Vec<Fact> = premises.to_vec();
    while let Some(t) = work.pop() {
        if asserted.contains(&t) {
            continue;
        }
        let &i = first.get(&t)?;
        if !needed.insert(i) {
            continue;
        }
        work.extend(derivations[i].premises.iter().copied());
    }
    let mut ids: Vec<usize> = needed.into_iter().collect();
    ids.sort_unstable();
    Some(ids)
}

/// Can this triple be written by an RDF serialiser?
///
/// A literal cannot be a subject and only an IRI can be a predicate, in every
/// RDF 1.1 serialisation. A rule that concludes otherwise has produced
/// something the store cannot hold, and the materialiser does not find out
/// until it is halfway through inserting the batch.
///
/// Both positions, and ONE definition, because the previous attempt was four
/// targeted guards and it missed two things.
///
/// The subject case was found on 30 August 2026 and guarded at `prp-symp`,
/// `prp-inv1`, `prp-inv2` and `eq-sym`. It was still open at `cls-avf`, which
/// concludes `y rdf:type c` from `x type ∀P.c` and `x P y`, so an
/// `owl:allValuesFrom` restriction on a property with a literal value derived
/// `"x" rdf:type :D`. That needs no unusual modelling at all.
///
/// The predicate case was never guarded anywhere outside `run_horn`. It is
/// reachable from `rdfs7`, `prp-inv1`, `prp-inv2` and `cls-hv1`, each of which
/// takes its conclusion's predicate from an object position where RDF permits a
/// blank node or a literal. `:p rdfs:subPropertyOf [ owl:inverseOf :q ]` is
/// enough, and that is what the OWL 2 mapping to RDF produces for
/// `SubObjectPropertyOf(:p ObjectInverseOf(:q))`. Found by
/// `tests/certificate_boundary_proptest.rs` on 14 September 2026 and pinned by
/// `tests/reason_unwritable_predicate_test.rs`, which also pins that `prp-symp`
/// and `prp-trp` CANNOT reach it, because their property must already be the
/// predicate of a stored triple and is therefore an IRI.
///
/// Terms arrive in their N-Triples spelling, so a literal begins with `"` and
/// an IRI with `<`.
fn writable_triple(subject: &str, predicate: &str) -> bool {
    !subject.starts_with('"') && predicate.starts_with('<')
}

/// Which position of a certificate line refused to be written.
///
/// Deliberately a bare enum and not an `anyhow::Error`. The writers below are
/// the functions the Kani harnesses at the bottom of this file prove things
/// about, and a formatted error inside a function is what put CBMC inside the
/// formatting machinery and killed the first `parse_pat` harness. The message a
/// user reads is built by the caller, from this and the term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Subject,
    Predicate,
    Object,
}

impl Position {
    fn name(self) -> &'static str {
        match self {
            Position::Subject => "subject",
            Position::Predicate => "predicate",
            Position::Object => "object",
        }
    }
}

/// Whether a term can be written to a certificate file without forging it.
///
/// This is TCB-4 and TCB-5, enforced here rather than observed of `oxrdf`.
///
/// The files are tab separated with one record per line and have no escaping
/// layer of their own, so a term carrying a tab gains a field, a term carrying a
/// newline splits a record in two, and either forges a triple the store never
/// held. Until 15 September 2026 the only thing standing between a tab and
/// `asserted.tsv` was `oxrdf`'s `print_quoted_str` for a literal and `oxiri`
/// refusing to parse for an IRI: `NamedNodeRef`'s `Display` is
/// `write!(f, "<{}>", self.as_str())` and escapes nothing at all. That is a
/// dependency's behaviour, pinned by a test that an upgrade would break loudly
/// but that cannot make it hold. This function makes it hold, in the refusing
/// direction: a term that does not fit the format means no certificate, not a
/// certificate a reader cannot trust.
///
/// The leading-byte condition is TCB-5. A literal is quoted and an IRI is
/// bracketed, so a literal whose lexical form is spelled exactly like an IRI
/// cannot collide with that IRI in a file where the two checkers compare terms
/// as opaque strings. Every term `GraphStore::all_triples` yields satisfies it
/// (`oxrdf`'s `Display` brackets a `NamedNode`, prefixes a `BlankNode` with
/// `_:` and quotes a `Literal`), and so does every constant `parse_pat` accepts,
/// so this refuses nothing the engine has any business writing.
fn term_fits_the_format(t: &str) -> bool {
    if !field_fits_the_format(t) {
        return false;
    }
    let b = t.as_bytes();
    b[0] == b'<' || b[0] == b'"' || (b[0] == b'_' && b.len() > 1 && b[1] == b':')
}

/// The half of [`term_fits_the_format`] that is about the FORMAT and not about
/// N-Triples: a field is non-empty and carries no separator.
///
/// `horn.tsv` also writes variable names, which are not terms and have no
/// N-Triples spelling, and this is what they have to satisfy.
fn field_fits_the_format(f: &str) -> bool {
    let b = f.as_bytes();
    if b.is_empty() {
        return false;
    }
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\t' || b[i] == b'\n' || b[i] == b'\r' {
            return false;
        }
        i += 1;
    }
    true
}

/// Append one line of `asserted.tsv`: `s TAB p TAB o NEWLINE`.
///
/// This is the whole of that file's grammar, and it is the narrowest point of
/// the trusted computing base: `OOCert.certificate_sound` is conditional on the
/// asserted graph, `OOCert.Parse.parseTriples` recovers it by splitting on
/// those two characters, and nothing between the two escapes anything. It lives
/// in one function so that the claim "a line is exactly three fields" is a
/// claim about one place, verified in `kani_harnesses` below rather than only
/// sampled.
///
/// Every term is checked BEFORE anything is appended, so a refusal leaves `out`
/// byte for byte as it was. A writer that appended a subject and then refused
/// the object would leave a half-line in the buffer, which is the same defect
/// the non-atomic materialiser had.
fn push_asserted_line(out: &mut String, s: &str, p: &str, o: &str) -> Result<(), Position> {
    if !term_fits_the_format(s) {
        return Err(Position::Subject);
    }
    if !term_fits_the_format(p) {
        return Err(Position::Predicate);
    }
    if !term_fits_the_format(o) {
        return Err(Position::Object);
    }
    out.push_str(s);
    out.push('\t');
    out.push_str(p);
    out.push('\t');
    out.push_str(o);
    out.push('\n');
    Ok(())
}

/// Append a triple as three further fields of a line already begun, the shape
/// `derivations.tsv` and `horn.tsv` use after their header fields. Same guard
/// and same all-or-nothing discipline as [`push_asserted_line`].
fn push_triple_fields(out: &mut String, s: &str, p: &str, o: &str) -> Result<(), Position> {
    if !term_fits_the_format(s) {
        return Err(Position::Subject);
    }
    if !term_fits_the_format(p) {
        return Err(Position::Predicate);
    }
    if !term_fits_the_format(o) {
        return Err(Position::Object);
    }
    out.push('\t');
    out.push_str(s);
    out.push('\t');
    out.push_str(p);
    out.push('\t');
    out.push_str(o);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// The built-in rules, as data
//
// TCB-14 used to read: "`OOCert.checkStep` pattern-matches the premises
// positionally per rule. The emitter passes them in a hand-written order at
// each of the thirty-one `emit` call sites. A wrong order is a rejection, not a
// false pass, and `tests/lean_certificate_test.rs` covers every rule, but
// nothing derives the order from a single source."
//
// This is the single source. Each rule is a pattern over numbered variables,
// in the order `lean/OOCert/Rules.lean` matches them, and the emitter computes
// both the premise list AND the conclusion from it. A call site supplies the
// BINDING and nothing else, so it has no order to get wrong: the twenty-seven
// fixed-arity rules cannot drift from the checker by a mistake at the site.
// `tests/lean_certificate_test.rs` still runs the real checker over the corpus,
// and `tests/premise_order_test.rs` parses the arms out of `Rules.lean` and
// compares them with this table, so a change to either side that the other does
// not follow fails at `cargo test` rather than at a user's certificate.
//
// The four list rules (`cls-int1`, `cls-int2`, `cls-uni`, `cls-oo`) are not
// here. Their premises include an RDF list chain whose length is the length of
// the list, so they are not a fixed pattern and the checker matches them with
// `takeChain` rather than positionally. They keep an explicit premise vector
// and say so at the site.
// ─────────────────────────────────────────────────────────────────────────────

/// A vocabulary term a built-in rule's pattern fixes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kw {
    Type,
    SubClassOf,
    SubPropertyOf,
    Domain,
    Range,
    SameAs,
    InverseOf,
    TransitiveProperty,
    SymmetricProperty,
    EquivalentClass,
    EquivalentProperty,
    OnProperty,
    SomeValuesFrom,
    AllValuesFrom,
    HasValue,
}

impl Kw {
    /// The IRI, in the N-Triples spelling the store uses. Reading this out is
    /// how `tests/premise_order_test.rs` maps a `V.foo` in the Lean source onto
    /// a row of this table.
    pub fn iri(self) -> &'static str {
        match self {
            Kw::Type => RDF_TYPE,
            Kw::SubClassOf => RDFS_SUBCLASS,
            Kw::SubPropertyOf => RDFS_SUBPROP,
            Kw::Domain => RDFS_DOMAIN,
            Kw::Range => RDFS_RANGE,
            Kw::SameAs => OWL_SAMEAS,
            Kw::InverseOf => OWL_INVERSE,
            Kw::TransitiveProperty => OWL_TRANSITIVE,
            Kw::SymmetricProperty => OWL_SYMMETRIC,
            Kw::EquivalentClass => OWL_EQUIV_CLASS,
            Kw::EquivalentProperty => OWL_EQUIV_PROP,
            Kw::OnProperty => OWL_ON_PROPERTY,
            Kw::SomeValuesFrom => OWL_SOME_VALUES,
            Kw::AllValuesFrom => OWL_ALL_VALUES,
            Kw::HasValue => OWL_HAS_VALUE,
        }
    }

    /// The name `lean/OOCert/Semantics.lean` gives it, without the `V.`.
    pub fn lean_name(self) -> &'static str {
        match self {
            Kw::Type => "type",
            Kw::SubClassOf => "subClassOf",
            Kw::SubPropertyOf => "subPropertyOf",
            Kw::Domain => "domain",
            Kw::Range => "range",
            Kw::SameAs => "sameAs",
            Kw::InverseOf => "inverseOf",
            Kw::TransitiveProperty => "transitiveProperty",
            Kw::SymmetricProperty => "symmetricProperty",
            Kw::EquivalentClass => "equivalentClass",
            Kw::EquivalentProperty => "equivalentProperty",
            Kw::OnProperty => "onProperty",
            Kw::SomeValuesFrom => "someValuesFrom",
            Kw::AllValuesFrom => "allValuesFrom",
            Kw::HasValue => "hasValue",
        }
    }
}

/// One position of a built-in rule's pattern: a slot in the binding, or a
/// vocabulary term the rule fixes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Var(usize),
    Fixed(Kw),
}

const V0: Slot = Slot::Var(0);
const V1: Slot = Slot::Var(1);
const V2: Slot = Slot::Var(2);
const V3: Slot = Slot::Var(3);
const V4: Slot = Slot::Var(4);
const TYPE: Slot = Slot::Fixed(Kw::Type);
const SUBCLASS: Slot = Slot::Fixed(Kw::SubClassOf);
const SUBPROP: Slot = Slot::Fixed(Kw::SubPropertyOf);
const DOMAIN: Slot = Slot::Fixed(Kw::Domain);
const RANGE: Slot = Slot::Fixed(Kw::Range);
const SAMEAS: Slot = Slot::Fixed(Kw::SameAs);
const INVERSE: Slot = Slot::Fixed(Kw::InverseOf);
const TRANSITIVE: Slot = Slot::Fixed(Kw::TransitiveProperty);
const SYMMETRIC: Slot = Slot::Fixed(Kw::SymmetricProperty);
const EQ_CLASS: Slot = Slot::Fixed(Kw::EquivalentClass);
const EQ_PROP: Slot = Slot::Fixed(Kw::EquivalentProperty);
const ON_PROP: Slot = Slot::Fixed(Kw::OnProperty);
const SVF: Slot = Slot::Fixed(Kw::SomeValuesFrom);
const AVF: Slot = Slot::Fixed(Kw::AllValuesFrom);
const HAS_VALUE: Slot = Slot::Fixed(Kw::HasValue);

/// A built-in rule with a fixed premise arity, as a pattern.
pub struct BuiltinRule {
    /// The rule id written into `derivations.tsv`, which `OOCert.Rule.ofName?`
    /// has to accept. Two rows may share a name: `scm-eqc1` and `scm-eqp1` each
    /// license two conclusions from one premise, and the checker accepts
    /// either.
    pub name: &'static str,
    /// Names for the binding slots, for readability and for the error a
    /// mis-sized binding produces. The LENGTH is the arity.
    pub vars: &'static [&'static str],
    /// The premises, in the order the checker matches them.
    pub body: &'static [[Slot; 3]],
    pub head: [Slot; 3],
}

/// The rows of [`BUILTIN_RULES`], by name, so a call site names a rule rather
/// than an index. `builtin_rule_table_is_indexed_by_its_enum` pins the
/// correspondence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rl {
    Rdfs2,
    Rdfs3,
    Rdfs5,
    Rdfs7,
    Rdfs9,
    Rdfs11,
    PrpTrp,
    PrpSymp,
    PrpInv1,
    PrpInv2,
    EqSym,
    ScmEqc1Fwd,
    ScmEqc1Rev,
    ScmEqp1Fwd,
    ScmEqp1Rev,
    ClsSvf1,
    ClsAvf,
    ClsHv1,
    ClsHv2,
    ScmSvf1,
    ScmSvf2,
    ScmAvf1,
    ScmAvf2,
    ScmDom1,
    ScmDom2,
    ScmRng1,
    ScmRng2,
}

impl Rl {
    /// Every row, in table order. `each_row_is_at_its_own_index` pins that
    /// `Rl::X as usize` really is the index of `X`'s row, which is the whole
    /// basis of `BUILTIN_RULES[r as usize]`.
    pub const ALL: &'static [Rl] = &[
        Rl::Rdfs2, Rl::Rdfs3, Rl::Rdfs5, Rl::Rdfs7, Rl::Rdfs9, Rl::Rdfs11,
        Rl::PrpTrp, Rl::PrpSymp, Rl::PrpInv1, Rl::PrpInv2, Rl::EqSym,
        Rl::ScmEqc1Fwd, Rl::ScmEqc1Rev, Rl::ScmEqp1Fwd, Rl::ScmEqp1Rev,
        Rl::ClsSvf1, Rl::ClsAvf, Rl::ClsHv1, Rl::ClsHv2,
        Rl::ScmSvf1, Rl::ScmSvf2, Rl::ScmAvf1, Rl::ScmAvf2,
        Rl::ScmDom1, Rl::ScmDom2, Rl::ScmRng1, Rl::ScmRng2,
    ];
}

/// Every fixed-arity built-in rule, in `Rl` order.
///
/// Read this against the table in the module docstring of
/// `lean/OOCert/Rules.lean`. They are the same table; `tests/premise_order_test.rs`
/// checks that mechanically against the `checkStep` arms rather than against
/// the prose.
pub const BUILTIN_RULES: &[BuiltinRule] = &[
    BuiltinRule { name: "rdfs2", vars: &["s", "p", "o", "c"],
        body: &[[V0, V1, V2], [V1, DOMAIN, V3]], head: [V0, TYPE, V3] },
    BuiltinRule { name: "rdfs3", vars: &["s", "p", "o", "c"],
        body: &[[V0, V1, V2], [V1, RANGE, V3]], head: [V2, TYPE, V3] },
    BuiltinRule { name: "rdfs5", vars: &["a", "b", "c"],
        body: &[[V0, SUBPROP, V1], [V1, SUBPROP, V2]], head: [V0, SUBPROP, V2] },
    BuiltinRule { name: "rdfs7", vars: &["s", "p", "o", "q"],
        body: &[[V0, V1, V2], [V1, SUBPROP, V3]], head: [V0, V3, V2] },
    BuiltinRule { name: "rdfs9", vars: &["x", "a", "b"],
        body: &[[V0, TYPE, V1], [V1, SUBCLASS, V2]], head: [V0, TYPE, V2] },
    BuiltinRule { name: "rdfs11", vars: &["a", "b", "c"],
        body: &[[V0, SUBCLASS, V1], [V1, SUBCLASS, V2]], head: [V0, SUBCLASS, V2] },
    BuiltinRule { name: "prp-trp", vars: &["p", "x", "y", "z"],
        body: &[[V0, TYPE, TRANSITIVE], [V1, V0, V2], [V2, V0, V3]], head: [V1, V0, V3] },
    BuiltinRule { name: "prp-symp", vars: &["p", "x", "y"],
        body: &[[V0, TYPE, SYMMETRIC], [V1, V0, V2]], head: [V2, V0, V1] },
    BuiltinRule { name: "prp-inv1", vars: &["p", "q", "x", "y"],
        body: &[[V0, INVERSE, V1], [V2, V0, V3]], head: [V3, V1, V2] },
    BuiltinRule { name: "prp-inv2", vars: &["p", "q", "x", "y"],
        body: &[[V0, INVERSE, V1], [V2, V1, V3]], head: [V3, V0, V2] },
    BuiltinRule { name: "eq-sym", vars: &["a", "b"],
        body: &[[V0, SAMEAS, V1]], head: [V1, SAMEAS, V0] },
    BuiltinRule { name: "scm-eqc1", vars: &["a", "b"],
        body: &[[V0, EQ_CLASS, V1]], head: [V0, SUBCLASS, V1] },
    BuiltinRule { name: "scm-eqc1", vars: &["a", "b"],
        body: &[[V0, EQ_CLASS, V1]], head: [V1, SUBCLASS, V0] },
    BuiltinRule { name: "scm-eqp1", vars: &["a", "b"],
        body: &[[V0, EQ_PROP, V1]], head: [V0, SUBPROP, V1] },
    BuiltinRule { name: "scm-eqp1", vars: &["a", "b"],
        body: &[[V0, EQ_PROP, V1]], head: [V1, SUBPROP, V0] },
    BuiltinRule { name: "cls-svf1", vars: &["r", "p", "c", "x", "y"],
        body: &[[V0, ON_PROP, V1], [V0, SVF, V2], [V3, V1, V4], [V4, TYPE, V2]],
        head: [V3, TYPE, V0] },
    BuiltinRule { name: "cls-avf", vars: &["r", "p", "c", "x", "y"],
        body: &[[V0, ON_PROP, V1], [V0, AVF, V2], [V3, TYPE, V0], [V3, V1, V4]],
        head: [V4, TYPE, V2] },
    BuiltinRule { name: "cls-hv1", vars: &["r", "p", "v", "x"],
        body: &[[V0, ON_PROP, V1], [V0, HAS_VALUE, V2], [V3, TYPE, V0]],
        head: [V3, V1, V2] },
    BuiltinRule { name: "cls-hv2", vars: &["r", "p", "v", "x"],
        body: &[[V0, ON_PROP, V1], [V0, HAS_VALUE, V2], [V3, V1, V2]],
        head: [V3, TYPE, V0] },
    BuiltinRule { name: "scm-svf1", vars: &["c1", "y1", "p", "c2", "y2"],
        body: &[[V0, SVF, V1], [V0, ON_PROP, V2], [V3, SVF, V4], [V3, ON_PROP, V2],
                [V1, SUBCLASS, V4]],
        head: [V0, SUBCLASS, V3] },
    BuiltinRule { name: "scm-svf2", vars: &["c1", "y", "p1", "c2", "p2"],
        body: &[[V0, SVF, V1], [V0, ON_PROP, V2], [V3, SVF, V1], [V3, ON_PROP, V4],
                [V2, SUBPROP, V4]],
        head: [V0, SUBCLASS, V3] },
    BuiltinRule { name: "scm-avf1", vars: &["c1", "y1", "p", "c2", "y2"],
        body: &[[V0, AVF, V1], [V0, ON_PROP, V2], [V3, AVF, V4], [V3, ON_PROP, V2],
                [V1, SUBCLASS, V4]],
        head: [V0, SUBCLASS, V3] },
    // The conclusion is the other way round, and it is the one place in this
    // table where that is true. A universal restriction is antitone in its
    // property, so `all p2 y` is the SMALLER class; writing this row the way
    // `scm-svf2` is written gives a step no model supports, and
    // `OOCert.the_natural_avf2_direction_is_not_entailed` is the refutation.
    BuiltinRule { name: "scm-avf2", vars: &["c1", "y", "p1", "c2", "p2"],
        body: &[[V0, AVF, V1], [V0, ON_PROP, V2], [V3, AVF, V1], [V3, ON_PROP, V4],
                [V2, SUBPROP, V4]],
        head: [V3, SUBCLASS, V0] },
    BuiltinRule { name: "scm-dom1", vars: &["p", "c1", "c2"],
        body: &[[V0, DOMAIN, V1], [V1, SUBCLASS, V2]], head: [V0, DOMAIN, V2] },
    BuiltinRule { name: "scm-dom2", vars: &["p2", "c", "p1"],
        body: &[[V0, DOMAIN, V1], [V2, SUBPROP, V0]], head: [V2, DOMAIN, V1] },
    BuiltinRule { name: "scm-rng1", vars: &["p", "c1", "c2"],
        body: &[[V0, RANGE, V1], [V1, SUBCLASS, V2]], head: [V0, RANGE, V2] },
    BuiltinRule { name: "scm-rng2", vars: &["p2", "c", "p1"],
        body: &[[V0, RANGE, V1], [V2, SUBPROP, V0]], head: [V2, RANGE, V1] },
];

/// The rule names the four list rules use. Not in [`BUILTIN_RULES`] because
/// their premises are a variable-length chain, listed here so a test can assert
/// that the two sets together are exactly the checker's rule set and that
/// nothing fell between them.
pub const CHAINED_RULES: &[&str] = &["cls-int1", "cls-int2", "cls-uni", "cls-oo"];

/// The interned id of each vocabulary term a rule pattern can fix.
struct RuleVocab {
    type_: u32,
    subclass: u32,
    subprop: u32,
    domain: u32,
    range: u32,
    sameas: u32,
    inverse: u32,
    transitive: u32,
    symmetric: u32,
    equiv_class: u32,
    equiv_prop: u32,
    on_property: u32,
    svf: u32,
    avf: u32,
    has_value: u32,
}

/// What a rule site hands the emitter.
///
/// `Bound` is a row of [`BUILTIN_RULES`] plus the binding for its variables:
/// the site chooses the terms, the table chooses the order and the conclusion.
/// `Chained` is one of the four list rules, whose premises are a constructor
/// triple, an RDF list chain of the list's own length, and the typings, and
/// which therefore has no fixed pattern to be a row of.
enum Fired<'a> {
    Bound(Rl, &'a [u32]),
    Chained(&'static str, Fact, &'a [Fact]),
}

impl RuleVocab {
    fn id(&self, k: Kw) -> u32 {
        match k {
            Kw::Type => self.type_,
            Kw::SubClassOf => self.subclass,
            Kw::SubPropertyOf => self.subprop,
            Kw::Domain => self.domain,
            Kw::Range => self.range,
            Kw::SameAs => self.sameas,
            Kw::InverseOf => self.inverse,
            Kw::TransitiveProperty => self.transitive,
            Kw::SymmetricProperty => self.symmetric,
            Kw::EquivalentClass => self.equiv_class,
            Kw::EquivalentProperty => self.equiv_prop,
            Kw::OnProperty => self.on_property,
            Kw::SomeValuesFrom => self.svf,
            Kw::AllValuesFrom => self.avf,
            Kw::HasValue => self.has_value,
        }
    }

    fn fill(&self, a: &[Slot; 3], b: &[u32]) -> Fact {
        let one = |s: Slot| match s {
            Slot::Var(i) => b[i],
            Slot::Fixed(k) => self.id(k),
        };
        (one(a[0]), one(a[1]), one(a[2]))
    }
}

/// The message a refused term produces. One place, so the four writers say the
/// same thing, and outside the functions Kani reasons about.
fn unwritable_term(file: &str, pos: Position, term: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "internal: the {} of a triple bound for {file} is {term:?}, which does not fit the \
         certificate format: a term must be non-empty, must carry no tab, newline or carriage \
         return, and must be in N-Triples spelling (<iri>, _:blank or a quoted literal). Writing \
         it would let the term forge a field or a record, so no certificate was written. See \
         TCB-4 and TCB-5 in docs/trusted-computing-base.md",
        pos.name()
    )
}

/// Intern strings to u32 IDs for efficient reasoning.
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

/// OWL2-RL reasoner using interned u32 triples and fixpoint iteration.
///
/// Profiles:
///   "rdfs"       — RDFS rules (subclass, domain/range, subproperty)
///   "owl-rl"     — RDFS + core OWL-RL (transitive, symmetric, inverse,
///                  sameAs, equivalentClass/Property) + the schema rules that
///                  move a domain or a range along the class and property
///                  hierarchies (scm-dom1, scm-dom2, scm-rng1, scm-rng2)
///   "owl-rl-ext" — All above + someValuesFrom, allValuesFrom, hasValue,
///                  intersectionOf (both directions), unionOf, oneOf, and the
///                  four rules that order two restrictions by their filler or
///                  by their property (scm-svf1, scm-svf2, scm-avf1, scm-avf2)
///
/// Twenty-nine rule ids in all, every one of them with an arm in
/// `lean/OOCert/Rules.lean` and a lemma in `lean/OOCert/Soundness.lean`. A rule
/// the checker cannot prove sound is a rule this file does not run: that is the
/// contract, and `tests/lean_certificate_test.rs` walks the whole corpus to
/// enforce it.
///
/// What is deliberately NOT here, with the reason, because "not implemented" and
/// "not sound" are different statements and a reader is owed which one applies:
///
///   * `eq-ref` is sound and useless. It asserts `owl:sameAs` reflexivity for
///     every term in every position. Measured on the largest file this
///     repository ships, `benchmark/oaei/data/anatomy/human.owl`, that is
///     17,194 triples of no consequence added to a graph of 35,354. It would
///     also put a derivation into the empty-graph case that
///     `OOCert.not_everything_is_entailed` depends on staying empty.
///   * The `eq-rep-s`, `eq-rep-p`, `eq-rep-o` family is sound and quadratic in
///     the size of a `owl:sameAs` clique. Counted by SPARQL over every RDF file
///     this repository tracks that parses and is under the size cap, the corpus
///     contains ZERO `owl:sameAs` triples, so the cost is certain and the
///     benefit here is nothing at all. Three files mention `sameAs` inside an
///     `rdfs:comment` and none of them asserts one.
///   * `scm-cls`, `scm-op` and `scm-dp` are sound and emit reflexive trivia
///     (`c rdfs:subClassOf c`, `c owl:equivalentClass c`) of exactly the kind
///     the `a != b` guards elsewhere in this file exist to suppress.
///   * The `dt-*` family needs a datatype VALUE space. `OOCert.Semantics`
///     reads a literal as its N-Triples spelling and says so, so there is no
///     condition here for those rules to be sound against.
///   * Nothing concludes `false`. Every rule above concludes a triple, which
///     is what the `oo-cert/1` format can carry. The seventeen rules of the
///     profile that conclude `false` are handled AFTER the fixpoint instead,
///     by [`find_clashes`], and written in the separate `oo-refute/1` format
///     that `lean/OOCert/Refute.lean` checks. Only `cax-dw` earns a
///     certificate there, because it is the only clash rule the Lean checker
///     has a semantic condition for; the rest are reported as found and
///     uncertified, and the two must never share a word.
///
/// The graph materialised inferences are written to when the caller asks for
/// them to be kept apart from what was asserted.
pub const INFERRED_GRAPH: &str = "https://open-ontologies.org/graph/inferred";

/// Where `reason` puts the triples it materialises.
///
/// The choice is a failure-direction one, not a matter of taste. A marker
/// triple on statements sitting in the default graph obliges every consumer to
/// filter, and the one that forgets publishes an inference as an assertion. A
/// separate graph fails the other way: a consumer that forgets sees fewer
/// triples, never wrong ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferenceTarget {
    /// Merge into the default graph beside the asserted statements. The
    /// historical behaviour, kept so existing callers keep their contract.
    DefaultGraph,
    /// Keep them in [`INFERRED_GRAPH`], where nothing downstream can mistake
    /// an inference for an assertion.
    Inferred,
}

pub struct Reasoner;

impl Reasoner {
    pub fn run(
        graph: &Arc<GraphStore>,
        profile: &str,
        materialize: bool,
    ) -> anyhow::Result<String> {
        Self::run_with_target(graph, profile, materialize, InferenceTarget::DefaultGraph)
    }

    pub fn run_with_target(
        graph: &Arc<GraphStore>,
        profile: &str,
        materialize: bool,
        target: InferenceTarget,
    ) -> anyhow::Result<String> {
        Self::run_full(graph, profile, materialize, target, None)
    }

    /// Run the forward-chaining reasoner and, when `certificate_dir` is given,
    /// write a derivation certificate beside the result.
    ///
    /// The certificate is two tab-separated files: `asserted.tsv`, every
    /// triple the run started from, and `derivations.tsv`, one line per
    /// inferred triple naming the rule that produced it and the premises the
    /// rule read. `lean/` holds a checker for that format whose soundness is a
    /// machine-checked theorem (`OOCert.certificate_sound`): a certificate it
    /// accepts contains only triples entailed by the asserted graph under the
    /// RDF-based semantics of the vocabulary the rules use. The engine's own
    /// correctness is therefore not the thing a consumer has to trust; the
    /// checker's is, and the checker is a few hundred lines with a proof.
    ///
    /// The first thing the checker caught was in this file: `cls-svf1` used
    /// to derive membership in a subclass from membership in its restriction
    /// superclass, the converse of the axiom
    /// (tests/reason_rl_ext_soundness_test.rs).
    ///
    /// Not available for `owl-dl`: the tableaux path has no rule trace.
    pub fn run_full(
        graph: &Arc<GraphStore>,
        profile: &str,
        materialize: bool,
        target: InferenceTarget,
        certificate_dir: Option<&std::path::Path>,
    ) -> anyhow::Result<String> {
        // Delegate OWL-DL to tableaux reasoner
        if profile == "owl-dl" {
            if certificate_dir.is_some() {
                anyhow::bail!(
                    "the owl-dl tableaux path emits no derivation certificate; \
                     run rdfs, owl-rl or owl-rl-ext for a certified run"
                );
            }
            if target == InferenceTarget::Inferred {
                // Say so rather than materialise into the default graph while
                // the caller believes the inferences were kept apart.
                anyhow::bail!(
                    "the owl-dl tableaux path does not yet write to {INFERRED_GRAPH}; \
                     run it with the default target, or use owl-rl / owl-rl-ext"
                );
            }
            return crate::tableaux::DlReasoner::run(graph, materialize);
        }

        let profile_used = match profile {
            "owl-rl" => "owl-rl",
            "owl-rl-ext" => "owl-rl-ext",
            _ => "rdfs",
        };
        let include_owl = profile_used == "owl-rl" || profile_used == "owl-rl-ext";
        let include_ext = profile_used == "owl-rl-ext";

        // Extract and intern the ASSERTED triples.
        //
        // Every graph except [`INFERRED_GRAPH`], which is where this same
        // function parks its own conclusions when the caller asks for them to
        // be kept apart. Reading them back would make run N's conclusions run
        // N+1's axioms, and `asserted.tsv` has no column that says "derived",
        // so the certificate would be conditional on a graph that was never
        // asserted. Until 15 September 2026 it did exactly that and the defect
        // was pinned by a test rather than fixed; TCB-8 in
        // `docs/trusted-computing-base.md` has the history.
        let (raw_triples, graphs_read) = graph.triples_outside(&[INFERRED_GRAPH])?;
        let mut interner = Interner::new();
        let mut facts: Vec<(u32, u32, u32)> = Vec::with_capacity(raw_triples.len());
        for (s, p, o) in &raw_triples {
            facts.push((interner.intern(s), interner.intern(p), interner.intern(o)));
        }

        // Intern well-known IRIs
        let rdf_type = interner.intern(RDF_TYPE);
        let rdfs_subclass = interner.intern(RDFS_SUBCLASS);
        let rdfs_subprop = interner.intern(RDFS_SUBPROP);
        let owl_sameas = interner.intern(OWL_SAMEAS);
        // Every well-known id is interned once, before the loop, so the
        // schema indices below can be rebuilt from the closure each iteration
        // without borrowing the interner mutably inside it.
        let rdfs_domain = interner.intern(RDFS_DOMAIN);
        let rdfs_range = interner.intern(RDFS_RANGE);
        let owl_transitive = interner.intern(OWL_TRANSITIVE);
        let owl_symmetric = interner.intern(OWL_SYMMETRIC);
        let owl_inverse = interner.intern(OWL_INVERSE);
        let owl_equiv_class = interner.intern(OWL_EQUIV_CLASS);
        let owl_equiv_prop = interner.intern(OWL_EQUIV_PROP);
        let owl_on_property = interner.intern(OWL_ON_PROPERTY);
        let owl_some_values = interner.intern(OWL_SOME_VALUES);
        let owl_all_values = interner.intern(OWL_ALL_VALUES);
        let owl_has_value = interner.intern(OWL_HAS_VALUE);
        let owl_intersection = interner.intern(OWL_INTERSECTION);
        let owl_union = interner.intern(OWL_UNION);
        let owl_oneof = interner.intern(OWL_ONEOF);
        let rdf_first = interner.intern(RDF_FIRST);
        let rdf_rest = interner.intern(RDF_REST);
        let rdf_nil = interner.intern(RDF_NIL);

        // The same well-known ids, in the shape `BUILTIN_RULES` reads them.
        // Every fixed term in every built-in rule pattern resolves through
        // this, so a rule's vocabulary is not restated at its call site either.
        let rule_vocab = RuleVocab {
            type_: rdf_type,
            subclass: rdfs_subclass,
            subprop: rdfs_subprop,
            domain: rdfs_domain,
            range: rdfs_range,
            sameas: owl_sameas,
            inverse: owl_inverse,
            transitive: owl_transitive,
            symmetric: owl_symmetric,
            equiv_class: owl_equiv_class,
            equiv_prop: owl_equiv_prop,
            on_property: owl_on_property,
            svf: owl_some_values,
            avf: owl_all_values,
            has_value: owl_has_value,
        };

        // The clash detector's vocabulary. Nothing below drives a rule in the
        // fixpoint: it is read once, after it, by `find_clashes`. Interning a
        // term the graph never mentions costs one entry and makes every lookup
        // a u32 comparison, the same as every other well-known id here.
        let clash_vocab = ClashVocab {
            rdf_type,
            disjoint_with: interner.intern(OWL_DISJOINT_WITH),
            nothing: interner.intern(OWL_NOTHING),
            complement_of: interner.intern(OWL_COMPLEMENT_OF),
            irreflexive: interner.intern(OWL_IRREFLEXIVE),
            asymmetric: interner.intern(OWL_ASYMMETRIC),
            prop_disjoint_with: interner.intern(OWL_PROP_DISJOINT_WITH),
            same_as: owl_sameas,
            different_from: interner.intern(OWL_DIFFERENT_FROM),
            max_cardinality: interner.intern(OWL_MAX_CARDINALITY),
            on_property: owl_on_property,
            source_individual: interner.intern(OWL_SOURCE_INDIVIDUAL),
            assertion_property: interner.intern(OWL_ASSERTION_PROPERTY),
            target_individual: interner.intern(OWL_TARGET_INDIVIDUAL),
            target_value: interner.intern(OWL_TARGET_VALUE),
        };

        // ── Fixpoint iteration ──────────────────────────────────────
        let mut triple_set: HashSet<Fact> = facts.iter().copied().collect();
        let initial_size = triple_set.len();
        let mut iterations = 0;
        // Which of the two break conditions ended the loop. A run that stopped
        // at the cap has a closure that is a LOWER BOUND, and every consumer
        // that compares two closures has to be able to see that: a truncated
        // closure(G) missing a conclusion closure(P) reached looks exactly like
        // an unsoundness in the engine. `run_horn` already reports the same
        // field, so this is consistency rather than novelty.
        let mut fixpoint_reached = false;

        // Certificate bookkeeping. A conclusion is recorded the first time it
        // is derived and never again, so the certificate has exactly one line
        // per inferred triple and `derivations.len() == inferred_count` is an
        // invariant the tests pin. Nothing here runs unless a certificate was
        // asked for: the hot path pays one branch per candidate triple.
        let certify = certificate_dir.is_some();
        let mut derivations: Vec<Derivation> = Vec::new();
        let mut recorded: HashSet<Fact> = HashSet::new();

        // Conclusions refused for being unwritable, and up to three of them to
        // show. A set rather than a counter, for the reason `run_horn` gives:
        // a refused conclusion never enters `triple_set`, so every later round
        // derives it again and a counter would report attempts.
        let mut refused: HashSet<Fact> = HashSet::new();
        let mut skipped_samples: Vec<String> = Vec::new();

        loop {
            iterations += 1;
            let before = triple_set.len();
            let mut new: Vec<Fact> = Vec::new();

            // Schema indices, rebuilt from the closure on every iteration.
            //
            // These used to be filtered ONCE out of the pre-loop snapshot while
            // only the three data indices below were rebuilt, so the run was
            // not a fixpoint of its own rule set: a `rdfs:domain` triple that
            // the reasoner itself derived, by rdfs7 over a subproperty of
            // rdfs:domain or by scm-eqp, was never used, and running `reason` a
            // second time derived more than running it once. The number of runs
            // needed was the length of the longest chain of such rules, not two.
            //
            // For certificates that mattered more than for query answers.
            // Materialising turns run N's conclusions into run N+1's premises,
            // so `asserted.tsv` could list the reasoner's own output as an
            // axiom with nothing marking it as derived, and the soundness
            // theorem is conditional on the assertions. Reaching the fixpoint
            // in one run is what makes a single certificate the whole story.
            //
            // The cost is a constant factor on a scan the loop already does.
            let domain_map: Vec<(u32, u32)> = triple_set.iter()
                .filter(|&&(_, p, _)| p == rdfs_domain)
                .map(|&(s, _, o)| (s, o)).collect();
            let range_map: Vec<(u32, u32)> = triple_set.iter()
                .filter(|&&(_, p, _)| p == rdfs_range)
                .map(|&(s, _, o)| (s, o)).collect();
            let transitive_set: HashSet<u32> = triple_set.iter()
                .filter(|&&(_, p, o)| p == rdf_type && o == owl_transitive)
                .map(|&(s, _, _)| s).collect();
            let symmetric_set: HashSet<u32> = triple_set.iter()
                .filter(|&&(_, p, o)| p == rdf_type && o == owl_symmetric)
                .map(|&(s, _, _)| s).collect();
            let inverse_pairs: Vec<(u32, u32)> = triple_set.iter()
                .filter(|&&(_, p, _)| p == owl_inverse)
                .map(|&(s, _, o)| (s, o)).collect();
            let equiv_class: Vec<(u32, u32)> = triple_set.iter()
                .filter(|&&(_, p, _)| p == owl_equiv_class)
                .map(|&(s, _, o)| (s, o)).collect();
            let equiv_prop: Vec<(u32, u32)> = triple_set.iter()
                .filter(|&&(_, p, _)| p == owl_equiv_prop)
                .map(|&(s, _, o)| (s, o)).collect();

            // OWL restriction structures and RDF lists (owl-rl-ext only).
            let mut restr_prop: HashMap<u32, u32> = HashMap::new();
            let mut restr_svf: HashMap<u32, u32> = HashMap::new();
            let mut restr_hv: HashMap<u32, u32> = HashMap::new();
            let mut intersection_classes: Vec<(u32, u32, Vec<Fact>, Vec<u32>)> = Vec::new();
            let mut union_classes: Vec<(u32, u32, Vec<Fact>, Vec<u32>)> = Vec::new();
            let mut oneof_classes: Vec<(u32, u32, Vec<Fact>, Vec<u32>)> = Vec::new();
            let mut svf_rules: Vec<(u32, u32, u32)> = Vec::new();
            let mut hv_rules: Vec<(u32, u32, u32)> = Vec::new();
            let mut avf_rules: Vec<(u32, u32, u32)> = Vec::new();
            if include_ext {
                let mut restr_avf: HashMap<u32, u32> = HashMap::new();
                for &(s, p, o) in triple_set.iter() {
                    if p == owl_on_property { restr_prop.insert(s, o); }
                    if p == owl_some_values { restr_svf.insert(s, o); }
                    if p == owl_all_values { restr_avf.insert(s, o); }
                    if p == owl_has_value { restr_hv.insert(s, o); }
                }
                avf_rules = restr_avf.iter()
                    .filter_map(|(&r, &filler)| restr_prop.get(&r).map(|&prop| (prop, filler, r)))
                    .collect();
                svf_rules = restr_svf.iter()
                    .filter_map(|(&r, &filler)| restr_prop.get(&r).map(|&prop| (prop, filler, r)))
                    .collect();
                hv_rules = restr_hv.iter()
                    .filter_map(|(&r, &val)| restr_prop.get(&r).map(|&prop| (prop, val, r)))
                    .collect();

                // A list is read only when it is well formed: every node carries
                // the rdf:first and rdf:rest the certificate checker will look
                // for, and the chain reaches rdf:nil. It used to be read
                // leniently and the class rules fired on whatever came back. The
                // checker has no rule for a list it cannot walk, so the reasoner
                // no longer derives from one either; deriving less from
                // malformed input is the sound direction. Each entry keeps the
                // head node and the chain triples so a certificate can cite them.
                let first_map: HashMap<u32, u32> = triple_set.iter()
                    .filter(|&&(_, p, _)| p == rdf_first)
                    .map(|&(s, _, o)| (s, o)).collect();
                let rest_map: HashMap<u32, u32> = triple_set.iter()
                    .filter(|&&(_, p, _)| p == rdf_rest)
                    .map(|&(s, _, o)| (s, o)).collect();
                let walk_list = |head: u32| -> Option<(Vec<Fact>, Vec<u32>)> {
                    let mut chain = Vec::new();
                    let mut items = Vec::new();
                    let mut cur = head;
                    // Bounded so that a cyclic rdf:rest cannot spin.
                    for _ in 0..100_000 {
                        if cur == rdf_nil {
                            return Some((chain, items));
                        }
                        let item = *first_map.get(&cur)?;
                        let next = *rest_map.get(&cur)?;
                        chain.push((cur, rdf_first, item));
                        chain.push((cur, rdf_rest, next));
                        items.push(item);
                        cur = next;
                    }
                    None
                };
                for &(s, p, o) in triple_set.iter() {
                    if p == owl_intersection
                        && let Some((chain, items)) = walk_list(o)
                        && !items.is_empty()
                    {
                        intersection_classes.push((s, o, chain, items));
                    }
                    if p == owl_union
                        && let Some((chain, items)) = walk_list(o)
                        && !items.is_empty()
                    {
                        union_classes.push((s, o, chain, items));
                    }
                    if p == owl_oneof
                        && let Some((chain, items)) = walk_list(o)
                        && !items.is_empty()
                    {
                        oneof_classes.push((s, o, chain, items));
                    }
                }
            }

            // Build per-iteration indices
            let type_idx: Vec<(u32, u32)> = triple_set.iter()
                .filter(|&&(_, p, _)| p == rdf_type)
                .map(|&(s, _, o)| (s, o)).collect();
            let subclass_idx: Vec<(u32, u32)> = triple_set.iter()
                .filter(|&&(_, p, _)| p == rdfs_subclass)
                .map(|&(s, _, o)| (s, o)).collect();
            let subprop_idx: Vec<(u32, u32)> = triple_set.iter()
                .filter(|&&(_, p, _)| p == rdfs_subprop)
                .map(|&(s, _, o)| (s, o)).collect();

            // Build a subclass lookup: sub → [super]
            let mut sub_to_super: HashMap<u32, Vec<u32>> = HashMap::new();
            for &(sub, sup) in &subclass_idx {
                sub_to_super.entry(sub).or_default().push(sup);
            }

            // Every rule goes through this. It pushes the candidate and, when a
            // certificate was asked for, records the first derivation of each
            // triple not already in the closure, with the premises in the
            // order the checker expects for that rule.
            //
            // THE ORDER IS NOT THE CALL SITE'S. A fixed-arity rule fires as
            // `Fired::Bound(rule, binding)`, and the conclusion and the premise
            // list are both computed from that rule's row in `BUILTIN_RULES`,
            // which is the table `tests/premise_order_test.rs` checks against
            // the `checkStep` arms in `lean/OOCert/Rules.lean`. A site that got
            // the order wrong used to produce a certificate the checker
            // rejected for a reason no user could act on; there is now no order
            // at a site to get wrong. The four list rules keep an explicit
            // premise vector, because an RDF list chain is not a fixed pattern,
            // and they say so where they fire.
            //
            // It is also the one place that refuses a conclusion no RDF
            // serialiser can write. Four rules below guard the subject position
            // themselves and are left alone; this guard is central because the
            // per-rule approach is what left the PREDICATE position open, and a
            // rule added later gets the guard without its author remembering.
            // Deriving less is the sound direction, and the count is reported
            // rather than swallowed.
            let interner_ref = &interner;
            let refused_ref = &mut refused;
            let samples_ref = &mut skipped_samples;
            let rv = &rule_vocab;
            let mut emit = |f: Fired| {
                let (rule, t) = match &f {
                    Fired::Bound(r, b) => {
                        let row = &BUILTIN_RULES[*r as usize];
                        // A real assert and not a `debug_assert`. The cost is
                        // one comparison beside two interner lookups the next
                        // lines already do, and the alternative in a release
                        // build is an index panic inside `fill` with nothing
                        // naming the rule.
                        assert_eq!(
                            row.vars.len(),
                            b.len(),
                            "{} takes a binding of {} terms",
                            row.name,
                            row.vars.len()
                        );
                        (row.name, rv.fill(&row.head, b))
                    }
                    Fired::Chained(name, t, _) => (*name, *t),
                };
                let (s, p, _) = t;
                if !writable_triple(interner_ref.resolve(s), interner_ref.resolve(p)) {
                    if refused_ref.insert(t) && samples_ref.len() < 3 {
                        samples_ref.push(format!(
                            "{} {} {} (by {rule})",
                            interner_ref.resolve(t.0),
                            interner_ref.resolve(t.1),
                            interner_ref.resolve(t.2)
                        ));
                    }
                    return;
                }
                if certify && !triple_set.contains(&t) && recorded.insert(t) {
                    let premises: Vec<Fact> = match &f {
                        Fired::Bound(r, b) => BUILTIN_RULES[*r as usize]
                            .body
                            .iter()
                            .map(|a| rv.fill(a, b))
                            .collect(),
                        Fired::Chained(_, _, ps) => ps.to_vec(),
                    };
                    derivations.push(Derivation { rule, conclusion: t, premises });
                }
                new.push(t);
            };

            // ── RDFS rules ──────────────────────────────────────────

            // rdfs9: x type sub, sub subClassOf super → x type super
            for &(x, sub) in &type_idx {
                if let Some(supers) = sub_to_super.get(&sub) {
                    for &sup in supers {
                        if sub != sup {
                            emit(Fired::Bound(Rl::Rdfs9, &[x, sub, sup]));
                        }
                    }
                }
            }

            // rdfs11: a subClassOf b, b subClassOf c → a subClassOf c
            for &(a, b) in &subclass_idx {
                if let Some(cs) = sub_to_super.get(&b) {
                    for &c in cs {
                        if a != b && b != c && a != c {
                            emit(Fired::Bound(Rl::Rdfs11, &[a, b, c]));
                        }
                    }
                }
            }

            // rdfs2: s p o, p domain class → s type class
            for &(prop, cls) in &domain_map {
                for &(s, p, o) in triple_set.iter() {
                    if p == prop {
                        emit(Fired::Bound(Rl::Rdfs2, &[s, p, o, cls]));
                    }
                }
            }

            // rdfs3: s p o, p range class → o type class (IRI only)
            for &(prop, cls) in &range_map {
                for &(s, p, o) in triple_set.iter() {
                    // The guard has to exclude LITERALS, which cannot be the
                    // subject of the conclusion, and nothing else. Requiring an
                    // IRI also dropped every range inference onto a blank node,
                    // so a blank-node value never got typed, rdfs9 starved
                    // behind it, and a SHACL shape targeting that class found
                    // no focus nodes. rdfs2 twelve lines up has no such guard.
                    if p == prop && !interner.resolve(o).starts_with('"') {
                        emit(Fired::Bound(Rl::Rdfs3, &[s, p, o, cls]));
                    }
                }
            }

            // rdfs5: subproperty transitivity
            let mut subp_to_super: HashMap<u32, Vec<u32>> = HashMap::new();
            for &(sub, sup) in &subprop_idx {
                subp_to_super.entry(sub).or_default().push(sup);
            }
            for &(a, b) in &subprop_idx {
                if let Some(cs) = subp_to_super.get(&b) {
                    for &c in cs {
                        if a != b && b != c && a != c {
                            emit(Fired::Bound(Rl::Rdfs5, &[a, b, c]));
                        }
                    }
                }
            }

            // rdfs7: s sub o, sub subPropertyOf super → s super o
            for &(sub, sup) in &subprop_idx {
                if sub != sup {
                    for &(s, p, o) in triple_set.iter() {
                        if p == sub {
                            emit(Fired::Bound(Rl::Rdfs7, &[s, sub, o, sup]));
                        }
                    }
                }
            }

            // Four rules below conclude a triple whose SUBJECT comes from an
            // object position, so a literal object would produce a triple no
            // RDF serialisation can express. The materialiser then failed on
            // the whole batch with "The subject of a triple must be an IRI or a
            // blank node", at a line number that moved between runs because it
            // depends on hash iteration order, and every inference from that
            // run was lost. OWL 2 RL scopes prp-symp, prp-inv1, prp-inv2 and
            // eq-sym to what can legally appear as a subject; this is that
            // scope, made explicit. Pinned by `tests/reason_literal_subject_test.rs`.
            let is_literal = |id: u32| interner.resolve(id).starts_with('"');

            // ── OWL-RL rules ────────────────────────────────────────
            if include_owl {
                // prp-trp: x P y, y P z → x P z
                for &tp in &transitive_set {
                    let pairs: Vec<(u32, u32)> = triple_set.iter()
                        .filter(|&&(_, p, _)| p == tp)
                        .map(|&(s, _, o)| (s, o)).collect();
                    let mut by_subj: HashMap<u32, Vec<u32>> = HashMap::new();
                    for &(s, o) in &pairs {
                        by_subj.entry(s).or_default().push(o);
                    }
                    for &(x, y) in &pairs {
                        if let Some(zs) = by_subj.get(&y) {
                            for &z in zs {
                                if x != z {
                                    emit(Fired::Bound(Rl::PrpTrp, &[tp, x, y, z]));
                                }
                            }
                        }
                    }
                }

                // prp-symp: s P o → o P s
                for &sp in &symmetric_set {
                    for &(s, p, o) in triple_set.iter() {
                        if p == sp && !is_literal(o) {
                            emit(Fired::Bound(Rl::PrpSymp, &[sp, s, o]));
                        }
                    }
                }

                // prp-inv1, prp-inv2: s P o, P inverseOf Q → o Q s (both directions)
                for &(p, q) in &inverse_pairs {
                    for &(s, pred, o) in triple_set.iter() {
                        if is_literal(o) {
                            continue;
                        }
                        if pred == p {
                            emit(Fired::Bound(Rl::PrpInv1, &[p, q, s, o]));
                        }
                        if pred == q {
                            emit(Fired::Bound(Rl::PrpInv2, &[p, q, s, o]));
                        }
                    }
                }

                // eq-sym: sameAs symmetry
                for &(s, p, o) in triple_set.iter() {
                    if p == owl_sameas && !is_literal(o) {
                        emit(Fired::Bound(Rl::EqSym, &[s, o]));
                    }
                }

                // scm-eqc1, scm-eqc2: equivalentClass → bidirectional subClassOf
                // Both conclusions are W3C scm-eqc1, which licenses two of them
                // from one premise. The second used to be emitted as "scm-eqc2",
                // which is a DIFFERENT W3C rule: it concludes owl:equivalentClass
                // from two subClassOf triples, the opposite direction. An auditor
                // reading that id and looking it up found the wrong rule, which
                // is precisely what the cls-hv1 comment below forbids. Emitting
                // both steps under the rule that licenses them also frees the
                // name for the real scm-eqc2 when it is implemented.
                for &(a, b) in &equiv_class {
                    emit(Fired::Bound(Rl::ScmEqc1Fwd, &[a, b]));
                    emit(Fired::Bound(Rl::ScmEqc1Rev, &[a, b]));
                }

                // scm-eqp1, scm-eqp2: equivalentProperty → bidirectional subPropertyOf
                // Same for scm-eqp1 and the name scm-eqp2.
                for &(a, b) in &equiv_prop {
                    emit(Fired::Bound(Rl::ScmEqp1Fwd, &[a, b]));
                    emit(Fired::Bound(Rl::ScmEqp1Rev, &[a, b]));
                }

                // scm-dom1, scm-dom2, scm-rng1, scm-rng2: a declared domain or
                // range moved along the class and the property hierarchy.
                //
                // These four add no ANSWERS. Everything they license at the
                // instance level is already reachable: rdfs2 over the original
                // domain gives `s rdf:type c1` and rdfs9 carries it up to `c2`,
                // which is what scm-dom1 followed by rdfs2 gives, and scm-dom2
                // is rdfs7 up to the superproperty followed by rdfs2. What they
                // add is the SCHEMA those answers are consequences of. A
                // consumer reading the materialised graph for "what is the
                // domain of this property" saw the declaration and not its
                // consequences, and a second reasoner run over the output
                // derived them, so the output was not a fixpoint of the
                // profile.
                //
                // They also fire on schema alone, so unlike almost everything
                // else in the extended profile they fire on a schema-only file
                // with no instances at all, which is most of this repository's
                // corpus.

                // scm-dom1: p rdfs:domain c1, c1 subClassOf c2 → p rdfs:domain c2
                for &(p, c1) in &domain_map {
                    if let Some(supers) = sub_to_super.get(&c1) {
                        for &c2 in supers {
                            if c1 != c2 {
                                emit(Fired::Bound(Rl::ScmDom1, &[p, c1, c2]));
                            }
                        }
                    }
                }

                // scm-dom2: p2 rdfs:domain c, p1 subPropertyOf p2 → p1 rdfs:domain c
                for &(p1, p2) in &subprop_idx {
                    if p1 == p2 {
                        continue;
                    }
                    for &(pd, c) in &domain_map {
                        if pd == p2 {
                            emit(Fired::Bound(Rl::ScmDom2, &[p2, c, p1]));
                        }
                    }
                }

                // scm-rng1: p rdfs:range c1, c1 subClassOf c2 → p rdfs:range c2
                for &(p, c1) in &range_map {
                    if let Some(supers) = sub_to_super.get(&c1) {
                        for &c2 in supers {
                            if c1 != c2 {
                                emit(Fired::Bound(Rl::ScmRng1, &[p, c1, c2]));
                            }
                        }
                    }
                }

                // scm-rng2: p2 rdfs:range c, p1 subPropertyOf p2 → p1 rdfs:range c
                for &(p1, p2) in &subprop_idx {
                    if p1 == p2 {
                        continue;
                    }
                    for &(pr, c) in &range_map {
                        if pr == p2 {
                            emit(Fired::Bound(Rl::ScmRng2, &[p2, c, p1]));
                        }
                    }
                }
            }

            // ── OWL-RL extended (someValuesFrom, hasValue, intersection, union)
            if include_ext {
                // Build type lookup: instance → set of classes
                let mut inst_types: HashMap<u32, HashSet<u32>> = HashMap::new();
                for &(x, cls) in &type_idx {
                    inst_types.entry(x).or_default().insert(cls);
                }

                // cls-svf1: x P y, y type filler, restriction(P, svf=filler)
                //           → x type restriction
                //
                // Two derivations this rule used to make are gone, both found
                // when every rule had to correspond to one the Lean checker
                // can prove sound (tests/reason_rl_ext_soundness_test.rs):
                //   * `x type C` for every `C rdfs:subClassOf restriction`.
                //     That is the converse of the axiom. Membership in a
                //     superclass never gives membership in a subclass; the
                //     equivalentClass case that made it look right is carried
                //     by rdfs9 over the subClassOf triple scm-eqc emits.
                //   * `x type restriction` from `x P filler`, where the
                //     object is the filler class IRI itself. A class in
                //     object position is a resource, not an instance of
                //     itself.
                for &(prop, filler, restr) in &svf_rules {
                    let prop_pairs: Vec<(u32, u32)> = triple_set.iter()
                        .filter(|&&(_, p, _)| p == prop)
                        .map(|&(s, _, o)| (s, o)).collect();

                    let filler_insts: HashSet<u32> = type_idx.iter()
                        .filter(|&&(_, cls)| cls == filler)
                        .map(|&(inst, _)| inst).collect();

                    for &(x, y) in &prop_pairs {
                        if filler_insts.contains(&y) {
                            emit(Fired::Bound(Rl::ClsSvf1, &[restr, prop, filler, x, y]));
                        }
                    }
                }

                // cls-avf: x type restriction(P, allValuesFrom c), x P y
                //          → y type c
                //
                // The restriction was parsed and the parse was thrown away, so
                // 123 owl:allValuesFrom axioms across six shipped files licensed
                // nothing. It is the cheapest missing rule in the OWL 2 RL
                // profile on both axes: the Rust is the cls-svf1 loop with the
                // premises the other way round, and the semantic condition is
                // the mirror of `svf`.
                for &(prop, filler, restr) in &avf_rules {
                    let in_restr: HashSet<u32> = type_idx.iter()
                        .filter(|&&(_, cls)| cls == restr)
                        .map(|&(inst, _)| inst).collect();
                    if in_restr.is_empty() {
                        continue;
                    }
                    for &(x, p, y) in triple_set.iter() {
                        if p == prop && in_restr.contains(&x) {
                            emit(Fired::Bound(Rl::ClsAvf, &[restr, prop, filler, x, y]));
                        }
                    }
                }

                // cls-hv1: x type restriction(P, hasValue v) → x P v
                // cls-hv2: x P v, restriction(P, hasValue v) → x type restriction
                //
                // The rule emitted under the name `cls-hv1` used to be the
                // composite of `cax-sco` and W3C `cls-hv1`: it demanded an
                // explicit `k rdfs:subClassOf r` hop and fired on `x type k`,
                // so an individual typed with the restriction DIRECTLY derived
                // nothing. The engine reached the restriction class by its own
                // rdfs9 and then refused to use it, and a certificate could
                // carry `rdfs9  <k> rdf:type <R>` right next to a cls-hv1 step
                // that ignored it. Putting a W3C rule name in front of an
                // auditor obliges the rule to be that rule. The W3C form is
                // used here and loses nothing: the composite case is rdfs9
                // followed by this.
                for &(prop, val, restr) in &hv_rules {
                    for &(x, c) in &type_idx {
                        if c == restr {
                            emit(Fired::Bound(Rl::ClsHv1, &[restr, prop, val, x]));
                        }
                    }
                    for &(s, p, o) in triple_set.iter() {
                        if p == prop && o == val {
                            emit(Fired::Bound(Rl::ClsHv2, &[restr, prop, val, s]));
                        }
                    }
                }

                // scm-svf1, scm-svf2, scm-avf1, scm-avf2: two restrictions
                // ordered by their fillers or by their properties.
                //
                // These are the rules that reach subsumptions no instance-level
                // rule can. `cls-svf1` needs an individual with a witness before
                // it says anything, and a schema-only ontology has none, so a
                // pizza-style file full of restrictions licensed nothing at all
                // from them. These four run on the schema.
                //
                // Two indices per restriction kind: (property, filler) → the
                // restrictions with both, and property → its (filler,
                // restriction) pairs. The first is the lookup and the second is
                // the scan, which keeps each rule linear in the hierarchy it
                // walks rather than quadratic in the restriction count.
                let mut svf_by_pf: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
                let mut svf_by_filler: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
                let mut svf_by_prop: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
                for &(prop, filler, restr) in &svf_rules {
                    svf_by_pf.entry((prop, filler)).or_default().push(restr);
                    svf_by_filler.entry(filler).or_default().push((prop, restr));
                    svf_by_prop.entry(prop).or_default().push((filler, restr));
                }
                let mut avf_by_pf: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
                let mut avf_by_filler: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
                let mut avf_by_prop: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
                for &(prop, filler, restr) in &avf_rules {
                    avf_by_pf.entry((prop, filler)).or_default().push(restr);
                    avf_by_filler.entry(filler).or_default().push((prop, restr));
                    avf_by_prop.entry(prop).or_default().push((filler, restr));
                }

                // scm-svf1: c1 svf y1, c1 onProperty p, c2 svf y2,
                //           c2 onProperty p, y1 subClassOf y2 → c1 subClassOf c2
                for &(y1, y2) in &subclass_idx {
                    let Some(ones) = svf_by_filler.get(&y1) else { continue };
                    for &(p, r1) in ones {
                        let Some(twos) = svf_by_pf.get(&(p, y2)) else { continue };
                        for &r2 in twos {
                            if r1 != r2 {
                                emit(Fired::Bound(Rl::ScmSvf1, &[r1, y1, p, r2, y2]));
                            }
                        }
                    }
                }

                // scm-svf2: c1 svf y, c1 onProperty p1, c2 svf y,
                //           c2 onProperty p2, p1 subPropertyOf p2 → c1 subClassOf c2
                for &(p1, p2) in &subprop_idx {
                    let Some(ones) = svf_by_prop.get(&p1) else { continue };
                    for &(y, r1) in ones {
                        let Some(twos) = svf_by_pf.get(&(p2, y)) else { continue };
                        for &r2 in twos {
                            if r1 != r2 {
                                emit(Fired::Bound(Rl::ScmSvf2, &[r1, y, p1, r2, p2]));
                            }
                        }
                    }
                }

                // scm-avf1: the same shape with owl:allValuesFrom, and the same
                // direction: a wider filler makes a wider class.
                for &(y1, y2) in &subclass_idx {
                    let Some(ones) = avf_by_filler.get(&y1) else { continue };
                    for &(p, r1) in ones {
                        let Some(twos) = avf_by_pf.get(&(p, y2)) else { continue };
                        for &r2 in twos {
                            if r1 != r2 {
                                emit(Fired::Bound(Rl::ScmAvf1, &[r1, y1, p, r2, y2]));
                            }
                        }
                    }
                }

                // scm-avf2: c1 avf y, c1 onProperty p1, c2 avf y,
                //           c2 onProperty p2, p1 subPropertyOf p2
                //           → c2 subClassOf c1
                //
                // THE CONCLUSION IS THE OTHER WAY ROUND. The W3C table reads
                // `T(?c2, rdfs:subClassOf, ?c1)` where scm-svf2 reads
                // `T(?c1, rdfs:subClassOf, ?c2)`, because a universal
                // restriction is antitone in its property: `all p2 y` has more
                // values to constrain than `all p1 y`, so it is the smaller
                // class. Writing it the way scm-svf2 is written gives a step no
                // model supports, and `OOCert.the_natural_avf2_direction_is_not_entailed`
                // is the refutation.
                for &(p1, p2) in &subprop_idx {
                    let Some(ones) = avf_by_prop.get(&p1) else { continue };
                    for &(y, r1) in ones {
                        let Some(twos) = avf_by_pf.get(&(p2, y)) else { continue };
                        for &r2 in twos {
                            if r1 != r2 {
                                emit(Fired::Bound(Rl::ScmAvf2, &[r1, y, p1, r2, p2]));
                            }
                        }
                    }
                }

                // cls-int1: x type ALL members → x type intersection class
                for (cls, head, chain, members) in &intersection_classes {
                    for (&x, x_types) in &inst_types {
                        if members.iter().all(|m| x_types.contains(m)) {
                            let premises: Vec<Fact> = if certify {
                                let mut v = vec![(*cls, owl_intersection, *head)];
                                v.extend(chain.iter().copied());
                                v.extend(members.iter().map(|&m| (x, rdf_type, m)));
                                v
                            } else {
                                Vec::new()
                            };
                            emit(Fired::Chained("cls-int1", (x, rdf_type, *cls), &premises));
                        }
                    }
                }

                // cls-int2: x type intersection class → x type EVERY member
                //
                // The mirror of cls-int1, and the only high-count rule in the
                // measured gap that produces genuinely new instance typings
                // rather than schema. One step per member, each repeating the
                // constructor triple and the chain, because a certificate step
                // carries one conclusion.
                for (cls, head, chain, members) in &intersection_classes {
                    for &(x, c) in &type_idx {
                        if c != *cls {
                            continue;
                        }
                        let premises: Vec<Fact> = if certify {
                            let mut v = vec![(*cls, owl_intersection, *head)];
                            v.extend(chain.iter().copied());
                            v.push((x, rdf_type, *cls));
                            v
                        } else {
                            Vec::new()
                        };
                        for &m in members {
                            emit(Fired::Chained("cls-int2", (x, rdf_type, m), &premises));
                        }
                    }
                }

                // cls-oo: every member of an owl:oneOf list is an instance of
                // the enumerated class. The only rule here with no instance
                // premise at all: an enumeration types its members on schema
                // alone. A literal member is skipped for the same reason the
                // four rules above skip one, since it would be the subject of
                // the conclusion.
                for (cls, head, chain, members) in &oneof_classes {
                    let premises: Vec<Fact> = if certify {
                        let mut v = vec![(*cls, owl_oneof, *head)];
                        v.extend(chain.iter().copied());
                        v
                    } else {
                        Vec::new()
                    };
                    for &m in members {
                        if !is_literal(m) {
                            emit(Fired::Chained("cls-oo", (m, rdf_type, *cls), &premises));
                        }
                    }
                }

                // cls-uni: x type ANY member → x type union class
                for (cls, head, chain, members) in &union_classes {
                    for &(x, c) in &type_idx {
                        if members.contains(&c) {
                            let premises: Vec<Fact> = if certify {
                                let mut v = vec![(*cls, owl_union, *head)];
                                v.extend(chain.iter().copied());
                                v.push((x, rdf_type, c));
                                v
                            } else {
                                Vec::new()
                            };
                            emit(Fired::Chained("cls-uni", (x, rdf_type, *cls), &premises));
                        }
                    }
                }
            }

            // Insert new triples (dedup against existing)
            for t in new {
                triple_set.insert(t);
            }

            if triple_set.len() == before {
                fixpoint_reached = true;
                break;
            }
            if iterations >= crate::runtime::reasoner_max_iterations() {
                break;
            }
        }

        let inferred_count = triple_set.len() - initial_size;

        // Materialize inferred triples
        if materialize && inferred_count > 0 {
            let original: HashSet<Fact> = facts.iter().copied().collect();
            let mut lines = String::new();
            for &(s, p, o) in &triple_set {
                if !original.contains(&(s, p, o)) {
                    lines.push_str(interner.resolve(s));
                    lines.push(' ');
                    lines.push_str(interner.resolve(p));
                    lines.push(' ');
                    lines.push_str(interner.resolve(o));
                    if target == InferenceTarget::Inferred {
                        lines.push_str(" <");
                        lines.push_str(INFERRED_GRAPH);
                        lines.push('>');
                    }
                    lines.push_str(" .\n");
                }
            }
            match target {
                InferenceTarget::DefaultGraph => graph.load_ntriples(&lines)?,
                InferenceTarget::Inferred => graph.load_nquads(&lines)?,
            };
        }

        // Sample
        let original: HashSet<Fact> = facts.iter().copied().collect();
        let sample: Vec<String> = triple_set.iter()
            .filter(|t| !original.contains(t))
            .filter(|&&(_, p, _)| p == rdf_type)
            .take(10)
            .map(|&(s, _, o)| format!("{} a {}", interner.resolve(s), interner.resolve(o)))
            .collect();

        let mut result = serde_json::json!({
            "profile_used": profile_used,
            "inferred_count": inferred_count,
            "iterations": iterations,
            "fixpoint_reached": fixpoint_reached,
            "initial_triples": initial_size,
            "final_triples": triple_set.len(),
            "sample_inferences": sample
        });
        if !materialize {
            result["dry_run"] = serde_json::json!(true);
        }
        if target == InferenceTarget::Inferred {
            // A caller cannot ask the inferences back unless it is told where
            // they were put.
            result["inference_graph"] = serde_json::json!(INFERRED_GRAPH);
        }
        if !refused.is_empty() {
            // Said out loud, because this run derived LESS than its rule set
            // licenses and a reader comparing two runs' counts is entitled to
            // know why.
            result["skipped_unserialisable"] = serde_json::json!(refused.len());
            result["skipped_examples"] = serde_json::json!(skipped_samples);
            result["skipped_reason"] = serde_json::json!(
                "the rule concluded a triple no RDF serialiser can write (a literal in subject \
                 position, or a non-IRI in predicate position). Such conclusions are neither \
                 materialised nor certified nor used as premises. Before this was guarded the \
                 materialiser failed partway through inserting the batch, which left the store \
                 holding an arbitrary, run-dependent prefix of the inferences with no certificate \
                 covering any of them"
            );
        }

        if let Some(dir) = certificate_dir {
            std::fs::create_dir_all(dir)?;
            let mut asserted = String::with_capacity(facts.len() * 96);
            for &(s, p, o) in &facts {
                push_asserted_line(
                    &mut asserted,
                    interner.resolve(s),
                    interner.resolve(p),
                    interner.resolve(o),
                )
                .map_err(|pos| unwritable_term("asserted.tsv", pos, interner.resolve(match pos {
                    Position::Subject => s,
                    Position::Predicate => p,
                    Position::Object => o,
                })))?;
            }
            std::fs::write(dir.join("asserted.tsv"), asserted)?;

            let mut by_rule: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
            let mut lines = String::with_capacity(derivations.len() * 256);
            for d in &derivations {
                *by_rule.entry(d.rule).or_default() += 1;
                lines.push_str(d.rule);
                for &(s, p, o) in std::iter::once(&d.conclusion).chain(d.premises.iter()) {
                    push_triple_fields(
                        &mut lines,
                        interner.resolve(s),
                        interner.resolve(p),
                        interner.resolve(o),
                    )
                    .map_err(|pos| unwritable_term("derivations.tsv", pos, interner.resolve(match pos {
                        Position::Subject => s,
                        Position::Predicate => p,
                        Position::Object => o,
                    })))?;
                }
                lines.push('\n');
            }
            std::fs::write(dir.join("derivations.tsv"), lines)?;

            // A refutation left behind by an earlier run into this directory
            // would be checked against THIS run's asserted.tsv, which is a
            // different graph. It would almost certainly be rejected, and
            // "almost certainly" is not the standard here: the stale file goes
            // before the clash detector below decides whether to write a new
            // one. Ignoring the error is correct; the usual case is that there
            // is no such file.
            let _ = std::fs::remove_file(dir.join("refutation.tsv"));

            result["certificate"] = serde_json::json!({
                "dir": dir.display().to_string(),
                "format": "oo-cert/1",
                "asserted": facts.len(),
                "derivations": derivations.len(),
                "by_rule": by_rule,
                // Which graphs the assertions came from, and which were held
                // back. A certificate that does not say this cannot be checked
                // against the store it claims to be about.
                "graphs_read": graphs_read,
                "graphs_excluded": [INFERRED_GRAPH],
                "check_with": "cd lean && lake exe oo-cert <dir>/asserted.tsv <dir>/derivations.tsv",
            });
        }

        // ── Inconsistency ───────────────────────────────────────────────────
        //
        // The clash rules, looked for over the closure the fixpoint reached.
        // The key appears only when something was found, so a run over a graph
        // with no clash is byte for byte the run it always was.
        let clashes = find_clashes(&triple_set, &interner, &clash_vocab);
        if !clashes.is_empty() {
            let show = |t: &Fact| {
                serde_json::json!([
                    interner.resolve(t.0),
                    interner.resolve(t.1),
                    interner.resolve(t.2)
                ])
            };
            let mut by_rule: std::collections::BTreeMap<&str, usize> =
                std::collections::BTreeMap::new();
            for c in &clashes {
                *by_rule.entry(c.rule).or_default() += 1;
            }
            let listed: Vec<serde_json::Value> = clashes
                .iter()
                .take(10)
                .map(|c| {
                    serde_json::json!({
                        "rule": c.rule,
                        "certifiable": clash_is_certifiable(c.rule),
                        "premises": c.premises.iter().map(show).collect::<Vec<_>>(),
                    })
                })
                .collect();
            let uncertifiable: Vec<&str> = by_rule
                .keys()
                .copied()
                .filter(|r| !clash_is_certifiable(r))
                .collect();
            let mut inconsistency = serde_json::json!({
                "found": true,
                // THIS ENGINE'S WORD, and deliberately not the checker's.
                // `oo-refute` says `unsatisfiable_under_disjointness` when it
                // has accepted a refutation. Nothing here may say that, or an
                // engine opinion and a machine-checked result share a string
                // and a consumer cannot tell them apart.
                "verdict": "clash_found_by_this_engine",
                "checked_by_lean": false,
                "clash_count": clashes.len(),
                "by_rule": by_rule,
                "clashes_listed": listed.len(),
                "clashes": listed,
                "certifiable_rules": CLASH_RULES_CERTIFIABLE,
                "uncertifiable_rules_found": uncertifiable,
                "rules_not_detected": CLASH_RULES_NOT_DETECTED
                    .iter()
                    .map(|(r, why)| serde_json::json!({"rule": r, "why": why}))
                    .collect::<Vec<_>>(),
                "means": "this engine found a contradiction in the closure it computed, and \
                          NOTHING HAS CHECKED THAT. A refutation the Lean checker accepted is a \
                          different kind of result with a different word for it: run `oo-refute \
                          check` and read its verdict, which is `unsatisfiable_under_disjointness` \
                          and is never stated here.",
                "limits": "cax-dw needs an INDIVIDUAL in two disjoint classes, so a TBox that is \
                           unsatisfiable with no individual asserted is invisible to this route \
                           entirely: it is a boundary of rule-based reasoning, not a bug. The SHIQ \
                           tableau in src/tableaux.rs (--profile owl-dl) does see that case, and \
                           its unsatisfiability finding carries NO certificate, which the owl-dl \
                           report states as `unsatisfiability and inconsistency carry no \
                           certificate`. Nothing found by the rules listed under \
                           rules_not_detected was looked for at all, so a clean run is not a \
                           consistency result.",
            });

            if let Some(dir) = certificate_dir {
                // A refutation file is written for the one rule the checker can
                // judge, and for nothing else. The engine's own detection of
                // any other clash rule stays a report, because a file naming a
                // rule `OOCert.RefuteConditions` has no field for is refused by
                // `oo-refute` with exit 2 and would be a certificate-shaped
                // object that certifies nothing.
                let certifiable = clashes.iter().find(|c| clash_is_certifiable(c.rule));
                match certifiable {
                    Some(c) => match refutation_prefix(&c.premises, &derivations, &original) {
                        Some(ids) => {
                            let mut text = String::from("oo-refute/1\n");
                            for &i in &ids {
                                let d = &derivations[i];
                                text.push_str(d.rule);
                                for &(s, p, o) in
                                    std::iter::once(&d.conclusion).chain(d.premises.iter())
                                {
                                    text.push('\t');
                                    text.push_str(interner.resolve(s));
                                    text.push('\t');
                                    text.push_str(interner.resolve(p));
                                    text.push('\t');
                                    text.push_str(interner.resolve(o));
                                }
                                text.push('\n');
                            }
                            text.push_str("refute\t");
                            text.push_str(c.rule);
                            for &(s, p, o) in &c.premises {
                                text.push('\t');
                                text.push_str(interner.resolve(s));
                                text.push('\t');
                                text.push_str(interner.resolve(p));
                                text.push('\t');
                                text.push_str(interner.resolve(o));
                            }
                            text.push('\n');
                            std::fs::write(dir.join("refutation.tsv"), text)?;
                            inconsistency["refutation"] = serde_json::json!({
                                "written": true,
                                "file": dir.join("refutation.tsv").display().to_string(),
                                "format": "oo-refute/1",
                                "rule": c.rule,
                                "prefix": ids.len(),
                                "premises": c.premises.iter().map(show).collect::<Vec<_>>(),
                                // Written, not checked. The word for a checked
                                // one is `oo-refute`'s to say.
                                "verdict": "refutation_written_not_yet_checked",
                                "check_with": format!(
                                    "cd lean && lake exe oo-refute check {a} {r}",
                                    a = dir.join("asserted.tsv").display(),
                                    r = dir.join("refutation.tsv").display()
                                ),
                                "guard_with": format!(
                                    "cd lean && lake exe oo-refute guard {a} {d} {r}",
                                    a = dir.join("asserted.tsv").display(),
                                    d = dir.join("derivations.tsv").display(),
                                    r = dir.join("refutation.tsv").display()
                                ),
                                "means": "a file in the format OOCert.RefuteParse.parseRefutation \
                                          reads. Exit 0 from `oo-refute check` is the result that \
                                          means anything; this engine writing the file means only \
                                          that it wrote it.",
                            });
                        }
                        None => {
                            inconsistency["refutation"] = serde_json::json!({
                                "written": false,
                                "why": "a premise of the clash is neither asserted nor recorded in \
                                        the derivation log, so no checkable prefix could be built. \
                                        That is a defect in this producer, not a property of the \
                                        ontology; please report it.",
                            });
                        }
                    },
                    None => {
                        inconsistency["refutation"] = serde_json::json!({
                            "written": false,
                            "why": "every clash found is of a rule the Lean checker has no \
                                    semantic condition for. `OOCert.RefuteConditions` carries one \
                                    field, for cax-dw, and `oo-refute` refuses a refutation naming \
                                    any other rule with exit 2 rather than judging it. Writing one \
                                    anyway would produce a certificate-shaped file that certifies \
                                    nothing.",
                        });
                    }
                }
            } else {
                inconsistency["refutation"] = serde_json::json!({
                    "written": false,
                    "why": "no certificate directory was given. Pass --certificate DIR (or \
                            certificate_dir over MCP) and a refutation is written there for the \
                            one clash rule the Lean checker can judge.",
                });
            }

            // A derivation certificate over a graph with a clash in it is worth
            // nothing under the disjointness reading, and the certificate block
            // must not be read without that.
            // `OOCert.a_certificate_adds_nothing_when_the_graph_is_refuted`.
            if let Some(cert) = result.get_mut("certificate")
                && let Some(obj) = cert.as_object_mut()
            {
                obj.insert("graph_has_a_clash".into(), serde_json::json!(true));
                obj.insert(
                    "read_with".into(),
                    serde_json::json!(
                        "this graph contradicts itself, so under the disjointness reading EVERY \
                         triple is entailed and this certificate carries no information. \
                         `oo-cert` will still accept it and will still be telling the truth, \
                         because its verdict quantifies over a model class that ignores \
                         disjointness. Run `oo-refute guard` instead of `oo-cert`."
                    ),
                );
            }

            result["inconsistency"] = inconsistency;
        }

        Ok(result.to_string())
    }
}

// ── User-supplied Horn rules ────────────────────────────────────────────────
//
// Everything above this line applies a rule set this file hardcodes. The
// checker in `lean/OOCert/Horn.lean` accepts a certificate over ANY rule table,
// proved once by `OOCert.horn_certificate_sound`, and until now nothing could
// produce one: the generic layer had a consumer and no producer, and a user
// with a rule table had nothing to hand it.
//
// What follows reads a rule table in the `rules.tsv` format
// `OOCert.HornParse.parseRules` accepts, evaluates it to a fixpoint over the
// loaded graph, and writes a certificate in the `horn.tsv` format
// `OOCert.HornParse.parseHornSteps` accepts.
//
// # This path states no verdict, and that is deliberate
//
// A certificate over the built-in table earns `entailed`: true in every model
// of the asserted graph, because `OOCert.Builtin.asHorn_sound` discharges those
// rules against the semantics. A certificate over a table a user wrote earns
// `entailed_under_supplied_rules`: true in every model of the graph THAT ALSO
// SATISFIES THOSE RULES. The rules are assumed and never checked, so a rule
// reading "every supplier is compliant" produces steps that check green for
// ever.
//
// `oo-horn` decides which of the two a run earned, by comparing the table it
// was given against the built-in one. This engine does not repeat that
// judgement in its own output and does not print either verdict word, because a
// second place where the two could be confused is exactly the hazard decision
// 0003 exists to prevent. What the engine reports is the table it used, a
// hash of it, and the command that pronounces.

/// One position of a triple pattern: a fixed term in its N-Triples spelling, or
/// a variable. Mirrors `OOCert.Pat`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pat {
    Const(String),
    Var(String),
}

impl Pat {
    /// The `rules.tsv` spelling: `?x` for a variable, the term itself for a
    /// constant. Inverse of the parse below, and byte-identical to
    /// `OOCert.HornParse.patStr`.
    ///
    /// The variable arm builds the string rather than `format!`-ing it, and the
    /// reason is verification and not speed: `format!` drags the whole
    /// `core::fmt` machinery into any function that calls it, and CBMC flattens
    /// a function before it solves. With `format!` here,
    /// `kani_harnesses::pat_of_and_render_are_inverse` measures the formatter.
    /// The bytes it produces are the same either way.
    fn render(&self) -> String {
        match self {
            Pat::Const(c) => c.clone(),
            Pat::Var(v) => {
                let mut s = String::with_capacity(v.len() + 1);
                s.push('?');
                s.push_str(v);
                s
            }
        }
    }
}

/// A triple pattern. Any of the three positions may be a variable, predicate
/// position included: `Interp` has one ternary extension, so a property is a
/// domain element like any other and the language stays first-order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtomPat {
    pub s: Pat,
    pub p: Pat,
    pub o: Pat,
}

/// `forall vars. body -> head`, the quantifier left implicit. Mirrors
/// `OOCert.RulePattern`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulePattern {
    pub name: String,
    pub body: Vec<AtomPat>,
    pub head: AtomPat,
}

impl RulePattern {
    fn atoms(&self) -> impl Iterator<Item = &AtomPat> {
        self.body.iter().chain(std::iter::once(&self.head))
    }
}

fn pat_vars<'a>(a: &'a AtomPat, out: &mut Vec<&'a str>) {
    for p in [&a.s, &a.p, &a.o] {
        if let Pat::Var(v) = p
            && !out.contains(&v.as_str())
        {
            out.push(v.as_str());
        }
    }
}

/// Parse one field of a rule table.
///
/// A field beginning with `?` is a variable, anything else a term, which is the
/// encoding `HornParse` documents: N-Triples terms begin with `<`, `_:` or `"`,
/// so nothing is ambiguous. This is stricter than `HornParse` on exactly one
/// point, and the strictness is the "reject rather than guess" direction: a
/// constant that is not in N-Triples spelling can never equal a term the store
/// holds, so a rule carrying one silently never fires. That is a typo, not a
/// rule, and it is refused rather than run.
/// The classification half of [`parse_pat`], with no error value and no
/// formatting.
///
/// Split out on 15 September 2026 for one reason, and it is a verification
/// reason rather than a style one. `parse_pat` returns `anyhow::Result` and
/// every refusal formats a message naming the line and the position, so the
/// function body carries the whole formatting machinery; CBMC flattens a
/// function before it solves, so a Kani harness on `parse_pat` was measuring
/// `format!` and never returned a verdict (14 minutes, 9.5 GB, and worse when
/// the input was constrained). This function is pure, total and allocates only
/// the `Pat` it returns, and `kani_harnesses::pat_of_and_render_are_inverse`
/// proves TCB-20 over every byte pattern at a bound. The messages stayed where
/// they were.
fn pat_of(field: &str) -> Option<Pat> {
    if let Some(name) = field.strip_prefix('?') {
        if name.is_empty() {
            return None;
        }
        return Some(Pat::Var(name.to_string()));
    }
    if field.starts_with('<') || field.starts_with("_:") || field.starts_with('"') {
        return Some(Pat::Const(field.to_string()));
    }
    None
}

fn parse_pat(field: &str, line: usize, which: &str) -> anyhow::Result<Pat> {
    if let Some(p) = pat_of(field) {
        return Ok(p);
    }
    if field == "?" {
        anyhow::bail!("rules line {line}: {which} is '?' with no variable name");
    }
    if field.is_empty() {
        anyhow::bail!("rules line {line}: {which} is empty");
    }
    anyhow::bail!(
        "rules line {line}: {which} is '{field}', which is neither a variable (?x) nor an \
         N-Triples term (<iri>, _:blank, or a quoted literal). The store spells every term in \
         N-Triples, so a constant in any other spelling would match nothing and the rule would \
         silently never fire"
    )
}

/// Read a rule table in the `rules.tsv` format `OOCert.HornParse.parseRules`
/// accepts: `name TAB bodyLength TAB (s TAB p TAB o)* TAB hs TAB hp TAB ho`.
///
/// Malformed input is an error naming the line and what was wrong, never a
/// guess. Two rejections go beyond what the Lean parser refuses, both because
/// this side has to PRODUCE bindings rather than check them:
///
/// * A head variable that does not occur in the body. The checker is right to
///   accept a certificate over such a rule, because `SatRule` quantifies over
///   every substitution, but the engine would have to invent a term to bind it
///   to. Refusing is the only honest option.
/// * A carriage return. `HornParse` splits on `\n` alone, so a CRLF file would
///   put a `\r` inside the last term of every line. Stripping it is a guess
///   about what the user meant; the error says to convert the file.
pub fn parse_rules(content: &str) -> anyhow::Result<Vec<RulePattern>> {
    let mut out: Vec<RulePattern> = Vec::new();
    for (i, line) in content.split('\n').enumerate() {
        let n = i + 1;
        if line.is_empty() {
            continue;
        }
        if line.contains('\r') {
            anyhow::bail!(
                "rules line {n}: the line contains a carriage return. The format is tab \
                 separated with LF endings, and a CR would become part of a term; convert the \
                 file rather than have it stripped silently"
            );
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 2 {
            anyhow::bail!(
                "rules line {n}: expected a name, a body length and then the patterns, got \
                 {} tab-separated field(s)",
                fields.len()
            );
        }
        let name = fields[0];
        if name.is_empty() {
            anyhow::bail!("rules line {n}: the rule name is empty");
        }
        let len_field = fields[1];
        if len_field.is_empty() || !len_field.bytes().all(|b| b.is_ascii_digit()) {
            anyhow::bail!("rules line {n}: body length '{len_field}' is not a number");
        }
        let m: usize = len_field
            .parse()
            .map_err(|_| anyhow::anyhow!("rules line {n}: body length '{len_field}' is too large"))?;
        let rest = &fields[2..];
        let want = m
            .checked_mul(3)
            .and_then(|x| x.checked_add(3))
            .ok_or_else(|| anyhow::anyhow!("rules line {n}: body length '{len_field}' is too large"))?;
        if rest.len() != want {
            anyhow::bail!(
                "rules line {n}: a body of {m} atom(s) plus a head needs {want} pattern fields, \
                 got {}",
                rest.len()
            );
        }
        let mut pats: Vec<Pat> = Vec::with_capacity(want);
        for (k, f) in rest.iter().enumerate() {
            let which = if k < 3 * m {
                format!("body atom {} position {}", k / 3 + 1, k % 3 + 1)
            } else {
                format!("head position {}", k - 3 * m + 1)
            };
            pats.push(parse_pat(f, n, &which)?);
        }
        let head = AtomPat {
            o: pats.pop().expect("head object"),
            p: pats.pop().expect("head predicate"),
            s: pats.pop().expect("head subject"),
        };
        let mut body: Vec<AtomPat> = Vec::with_capacity(m);
        let mut it = pats.into_iter();
        for _ in 0..m {
            let s = it.next().expect("body subject");
            let p = it.next().expect("body predicate");
            let o = it.next().expect("body object");
            body.push(AtomPat { s, p, o });
        }
        let rule = RulePattern { name: name.to_string(), body, head };

        let mut body_vars: Vec<&str> = Vec::new();
        for a in &rule.body {
            pat_vars(a, &mut body_vars);
        }
        let mut head_vars: Vec<&str> = Vec::new();
        pat_vars(&rule.head, &mut head_vars);
        for v in head_vars {
            if !body_vars.contains(&v) {
                anyhow::bail!(
                    "rules line {n}: rule '{}' has head variable ?{v}, which does not occur in \
                     the body. The engine has nothing to bind it to, so the table is refused \
                     rather than run with a term invented for it",
                    rule.name
                );
            }
        }
        out.push(rule);
    }
    Ok(out)
}

/// Render a rule table back to `rules.tsv`. Byte-identical to
/// `OOCert.HornParse.ruleStr` per line, so `oo-horn rules` and this agree and a
/// table read here and written back out digests to the same value it came in
/// with.
pub fn rules_tsv(rules: &[RulePattern]) -> String {
    let mut out = String::new();
    for r in rules {
        out.push_str(&r.name);
        out.push('\t');
        out.push_str(&r.body.len().to_string());
        for a in r.atoms() {
            for p in [&a.s, &a.p, &a.o] {
                out.push('\t');
                out.push_str(&p.render());
            }
        }
        out.push('\n');
    }
    out
}

/// A rule pattern position over interned ids, with variables resolved to a slot
/// in the rule's environment.
enum CPat {
    Const(u32),
    Var(usize),
}

struct CAtom {
    s: CPat,
    p: CPat,
    o: CPat,
}

struct CRule {
    vars: Vec<String>,
    body: Vec<CAtom>,
    head: CAtom,
}

/// One line of a Horn certificate: the rule index, the binding the engine used,
/// the premises it matched in the rule's body order, and what it concluded.
struct HornStepOut {
    rule: usize,
    binds: Vec<(String, u32)>,
    premises: Vec<Fact>,
    conclusion: Fact,
}

fn resolve_pat(p: &CPat, env: &[Option<u32>]) -> Option<u32> {
    match p {
        CPat::Const(id) => Some(*id),
        CPat::Var(i) => env[*i],
    }
}

/// Match one position against one term, binding a fresh variable and recording
/// the slot so the caller can undo it on backtracking.
fn unify_pat(p: &CPat, val: u32, env: &mut [Option<u32>], bound: &mut [usize; 3], nb: &mut usize) -> bool {
    match p {
        CPat::Const(id) => *id == val,
        CPat::Var(i) => match env[*i] {
            Some(existing) => existing == val,
            None => {
                env[*i] = Some(val);
                bound[*nb] = *i;
                *nb += 1;
                true
            }
        },
    }
}

/// Enumerate every way to match the body against the known facts, calling
/// `on_match` once per complete binding.
///
/// Plain backtracking, one atom at a time, left to right. An atom whose
/// predicate is already fixed (a constant, or a variable bound by an earlier
/// atom) is matched against the facts with that predicate; otherwise every fact
/// is a candidate. That is the whole optimisation, and it is not semi-naive
/// evaluation: each iteration re-derives what the last one did, and the
/// duplicate is dropped when its conclusion is found to be known. Correctness
/// first, as the task said. The cost is a constant factor per iteration on a
/// scan the loop does anyway, and `derived_triples` is unaffected.
fn match_body(
    atoms: &[CAtom],
    i: usize,
    env: &mut Vec<Option<u32>>,
    all: &[Fact],
    by_pred: &HashMap<u32, Vec<Fact>>,
    on_match: &mut dyn FnMut(&[Option<u32>]),
) {
    if i == atoms.len() {
        on_match(env);
        return;
    }
    let a = &atoms[i];
    let candidates: &[Fact] = match resolve_pat(&a.p, env) {
        Some(pid) => by_pred.get(&pid).map(Vec::as_slice).unwrap_or(&[]),
        None => all,
    };
    for &(s, p, o) in candidates {
        let mut bound = [0usize; 3];
        let mut nb = 0usize;
        let ok = unify_pat(&a.s, s, env, &mut bound, &mut nb)
            && unify_pat(&a.p, p, env, &mut bound, &mut nb)
            && unify_pat(&a.o, o, env, &mut bound, &mut nb);
        if ok {
            match_body(atoms, i + 1, env, all, by_pred, on_match);
        }
        for slot in bound.iter().take(nb) {
            env[*slot] = None;
        }
    }
}

fn inst_atom(a: &CAtom, env: &[Option<u32>]) -> Fact {
    const WHY: &str = "every variable of a matched rule is bound: the body is matched in full and \
                       parse_rules refuses a head variable that does not occur in the body";
    (
        resolve_pat(&a.s, env).expect(WHY),
        resolve_pat(&a.p, env).expect(WHY),
        resolve_pat(&a.o, env).expect(WHY),
    )
}

impl Reasoner {
    /// Evaluate a SUPPLIED Horn rule table over the loaded graph and write a
    /// certificate `lean`'s `oo-horn` can check.
    ///
    /// Three files land in `certificate_dir`:
    ///
    /// * `rules.tsv`, the table as the engine parsed it. The checker is given
    ///   this rather than the user's file, so what it checks and what the run
    ///   used are the same table.
    /// * `asserted.tsv`, every triple the run started from.
    /// * `horn.tsv`, one line per derived triple:
    ///   `ruleIndex TAB bindCount TAB (var TAB term)* TAB cs TAB cp TAB co TAB (ps TAB pp TAB po)*`.
    ///   The binding is written out in full and the premises are the body
    ///   instantiated, in the body's order, because that is what
    ///   `OOCert.checkHornStep` demands: a binding that does not instantiate
    ///   the body, or premises in the wrong order, is rejected.
    ///
    /// Steps are written in derivation order, and a step's premises are always
    /// facts that were known before the round that derived it, so every premise
    /// is asserted or concluded by an EARLIER line. That is the ordering
    /// `OOCert.checkHornAll` requires, and no step can cite itself.
    ///
    /// Nothing is materialised into the store. A conclusion under a supplied
    /// table holds only in models that satisfy that table, and writing it in
    /// beside the assertions would lose exactly the distinction decision 0003
    /// is about.
    pub fn run_horn(
        graph: &Arc<GraphStore>,
        rules_path: &std::path::Path,
        certificate_dir: &std::path::Path,
    ) -> anyhow::Result<String> {
        let rules_text = std::fs::read_to_string(rules_path)
            .map_err(|e| anyhow::anyhow!("cannot read rule table {}: {e}", rules_path.display()))?;
        let rules = parse_rules(&rules_text)
            .map_err(|e| anyhow::anyhow!("{} is not a rule table: {e}", rules_path.display()))?;
        if rules.is_empty() {
            anyhow::bail!(
                "{} holds no rules. An empty table derives nothing, so there is no certificate \
                 to write",
                rules_path.display()
            );
        }

        // The asserted triples, every graph except the one the built-in path
        // parks its own conclusions in. See TCB-8; this path materialises
        // nothing itself (decision 0003) but it reads the same store.
        let (raw_triples, graphs_read) = graph.triples_outside(&[INFERRED_GRAPH])?;
        let mut interner = Interner::new();
        let mut facts: Vec<Fact> = Vec::with_capacity(raw_triples.len());
        for (s, p, o) in &raw_triples {
            facts.push((interner.intern(s), interner.intern(p), interner.intern(o)));
        }

        // Compile the table over the interner. A constant the graph never
        // mentions still gets an id; it simply matches nothing, which is what
        // it should do.
        let mut crules: Vec<CRule> = Vec::with_capacity(rules.len());
        for r in &rules {
            let mut vars: Vec<String> = Vec::new();
            for a in r.atoms() {
                for p in [&a.s, &a.p, &a.o] {
                    if let Pat::Var(v) = p
                        && !vars.iter().any(|x| x == v)
                    {
                        vars.push(v.clone());
                    }
                }
            }
            let mut compile = |p: &Pat| match p {
                Pat::Const(c) => CPat::Const(interner.intern(c)),
                Pat::Var(v) => CPat::Var(vars.iter().position(|x| x == v).expect("var listed")),
            };
            let body: Vec<CAtom> = r
                .body
                .iter()
                .map(|a| CAtom { s: compile(&a.s), p: compile(&a.p), o: compile(&a.o) })
                .collect();
            let head = CAtom { s: compile(&r.head.s), p: compile(&r.head.p), o: compile(&r.head.o) };
            crules.push(CRule { vars, body, head });
        }

        // A rule whose head puts a literal in subject position, or anything but
        // an IRI in predicate position, produces something no RDF serialiser
        // can write. Both are refused, the count is reported, and the
        // conclusion is not used as a premise for anything else. Deriving less
        // is the sound direction, but it means the emitted set is the fixpoint
        // of the table over WRITABLE triples, which is what
        // `skipped_unserialisable` in the response is there to say.
        //
        // `run_full` now shares the predicate (`writable_triple`). It used to
        // have its own guard on the subject position only, at four rule sites,
        // and this path was the only one that refused a non-IRI predicate.
        let serialisable = |interner: &Interner, (s, p, _o): Fact| -> bool {
            writable_triple(interner.resolve(s), interner.resolve(p))
        };

        let mut known: HashSet<Fact> = facts.iter().copied().collect();
        let asserted_count = known.len();
        let mut steps: Vec<HornStepOut> = Vec::new();
        // Distinct conclusions refused for being unwritable. A set rather than
        // a counter because a refused conclusion is never added to `known`, so
        // every later round matches the same body again and re-derives it: a
        // counter would report attempts and grow with the iteration count,
        // which is not what a reader takes "skipped 2" to mean.
        let mut refused: HashSet<Fact> = HashSet::new();
        let mut skipped_samples: Vec<String> = Vec::new();
        let mut iterations = 0usize;
        let mut fixpoint = false;
        let max_iterations = crate::runtime::reasoner_max_iterations();

        while iterations < max_iterations {
            iterations += 1;

            // Sorted so the candidate order, and therefore the order of the
            // lines in `horn.tsv`, does not depend on hash iteration order.
            let mut all: Vec<Fact> = known.iter().copied().collect();
            all.sort_unstable();
            let mut by_pred: HashMap<u32, Vec<Fact>> = HashMap::new();
            for &f in &all {
                by_pred.entry(f.1).or_default().push(f);
            }

            let mut round: Vec<HornStepOut> = Vec::new();
            let mut pending: HashSet<Fact> = HashSet::new();
            for (ri, rule) in crules.iter().enumerate() {
                let mut env: Vec<Option<u32>> = vec![None; rule.vars.len()];
                let known_ref = &known;
                let interner_ref = &interner;
                let round_ref = &mut round;
                let pending_ref = &mut pending;
                let refused_ref = &mut refused;
                let samples_ref = &mut skipped_samples;
                match_body(&rule.body, 0, &mut env, &all, &by_pred, &mut |env| {
                    let conclusion = inst_atom(&rule.head, env);
                    if known_ref.contains(&conclusion) || pending_ref.contains(&conclusion) {
                        return;
                    }
                    if !serialisable(interner_ref, conclusion) {
                        if refused_ref.insert(conclusion) && samples_ref.len() < 3 {
                            samples_ref.push(format!(
                                "{} {} {}",
                                interner_ref.resolve(conclusion.0),
                                interner_ref.resolve(conclusion.1),
                                interner_ref.resolve(conclusion.2)
                            ));
                        }
                        return;
                    }
                    pending_ref.insert(conclusion);
                    round_ref.push(HornStepOut {
                        rule: ri,
                        binds: rule
                            .vars
                            .iter()
                            .enumerate()
                            .map(|(i, v)| (v.clone(), env[i].expect("matched body binds every body variable")))
                            .collect(),
                        premises: rule.body.iter().map(|a| inst_atom(a, env)).collect(),
                        conclusion,
                    });
                });
            }

            if round.is_empty() {
                fixpoint = true;
                break;
            }
            for st in round {
                known.insert(st.conclusion);
                steps.push(st);
            }
        }

        // Write the three files.
        std::fs::create_dir_all(certificate_dir)?;
        let canonical_rules = rules_tsv(&rules);
        // The checker verifies the steps against the FILE, and the run
        // evaluated the table in memory. If the rendering lost or changed
        // anything the two would be different rule sets, and a step would be
        // checked against a rule nobody ran. Reading the file back and
        // comparing costs nothing and closes that gap; a mismatch is a bug in
        // this file, so it stops the run rather than writing a certificate
        // whose meaning is not the run's.
        match parse_rules(&canonical_rules) {
            Ok(reparsed) if reparsed == rules => {}
            Ok(_) => anyhow::bail!(
                "internal: the rule table this engine writes does not read back as the table it \
                 evaluated, so the certificate would be checked against different rules. Refusing \
                 to write it"
            ),
            Err(e) => anyhow::bail!(
                "internal: the rule table this engine writes does not parse ({e}), so the checker \
                 could not read it. Refusing to write it"
            ),
        }
        std::fs::write(certificate_dir.join("rules.tsv"), &canonical_rules)?;

        let mut asserted = String::with_capacity(facts.len() * 96);
        for &(s, p, o) in &facts {
            push_asserted_line(
                &mut asserted,
                interner.resolve(s),
                interner.resolve(p),
                interner.resolve(o),
            )
            .map_err(|pos| unwritable_term("asserted.tsv", pos, interner.resolve(match pos {
                Position::Subject => s,
                Position::Predicate => p,
                Position::Object => o,
            })))?;
        }
        std::fs::write(certificate_dir.join("asserted.tsv"), asserted)?;

        let mut horn = String::with_capacity(steps.len() * 256);
        let mut by_rule: Vec<usize> = vec![0; rules.len()];
        for st in &steps {
            by_rule[st.rule] += 1;
            horn.push_str(&st.rule.to_string());
            horn.push('\t');
            horn.push_str(&st.binds.len().to_string());
            for (v, term) in &st.binds {
                // A variable name is not a term and has no N-Triples spelling,
                // so it gets the format half of the guard. `parse_rules` cannot
                // produce one carrying a separator, which is exactly why this
                // is cheap: it enforces here what is argued there.
                if !field_fits_the_format(v) {
                    anyhow::bail!(
                        "internal: the rule variable name {v:?} does not fit the certificate \
                         format, so no certificate was written"
                    );
                }
                let bound = interner.resolve(*term);
                if !term_fits_the_format(bound) {
                    return Err(unwritable_term("horn.tsv", Position::Object, bound));
                }
                horn.push('\t');
                horn.push_str(v);
                horn.push('\t');
                horn.push_str(bound);
            }
            for &(s, p, o) in std::iter::once(&st.conclusion).chain(st.premises.iter()) {
                push_triple_fields(
                    &mut horn,
                    interner.resolve(s),
                    interner.resolve(p),
                    interner.resolve(o),
                )
                .map_err(|pos| unwritable_term("horn.tsv", pos, interner.resolve(match pos {
                    Position::Subject => s,
                    Position::Predicate => p,
                    Position::Object => o,
                })))?;
            }
            horn.push('\n');
        }
        std::fs::write(certificate_dir.join("horn.tsv"), horn)?;

        let digest = {
            use sha2::{Digest, Sha256};
            format!("{:x}", Sha256::digest(canonical_rules.as_bytes()))
        };
        let per_rule: Vec<serde_json::Value> = rules
            .iter()
            .enumerate()
            .map(|(i, r)| {
                serde_json::json!({"index": i, "name": r.name, "derivations": by_rule[i]})
            })
            .collect();
        let sample: Vec<String> = steps
            .iter()
            .take(10)
            .map(|st| {
                format!(
                    "{} {} {}",
                    interner.resolve(st.conclusion.0),
                    interner.resolve(st.conclusion.1),
                    interner.resolve(st.conclusion.2)
                )
            })
            .collect();
        let dir = certificate_dir.display().to_string();

        let mut result = serde_json::json!({
            "mode": "horn",
            "rules_file": rules_path.display().to_string(),
            "rules": rules.len(),
            "asserted_triples": facts.len(),
            "distinct_asserted_triples": asserted_count,
            "graphs_read": graphs_read,
            "graphs_excluded": [INFERRED_GRAPH],
            "derived_triples": steps.len(),
            "iterations": iterations,
            "fixpoint_reached": fixpoint,
            "skipped_unserialisable": refused.len(),
            "materialized": false,
            "why_not_materialized":
                "a conclusion drawn under a supplied rule table holds only in models that satisfy \
                 that table, so it is not written into the store beside the assertions. The \
                 certificate is the output of this run",
            "conditional_on":
                format!("the rules in {}, which this run ASSUMED and never checked. A certificate \
                         is only as good as the table it cites: a rule saying every supplier is \
                         compliant produces steps that check green for ever",
                        rules_path.display()),
            "sample_derivations": sample,
            "certificate": {
                "dir": dir,
                "format": "oo-horn/1",
                "rules": rules.len(),
                "rules_tsv_sha256": digest,
                "asserted": facts.len(),
                "derivations": steps.len(),
                "by_rule": per_rule,
                "check_with": format!(
                    "cd lean && lake exe oo-horn check {d}/rules.tsv {d}/asserted.tsv {d}/horn.tsv",
                    d = certificate_dir.display()
                ),
                "pronounced_by":
                    "lean/, through `oo-horn check`, which decides what this run earned by \
                     comparing the table against the built-in one. This engine emits the \
                     certificate and states no verdict of its own",
            }
        });
        if !fixpoint {
            // An iteration cap reached with work still to do is not a fixpoint,
            // and a count from such a run is a lower bound. Say it in the
            // result rather than let the caller read `derived_triples` as the
            // closure.
            result["incomplete"] = serde_json::json!(format!(
                "the run stopped at the {max_iterations}-iteration cap with rules still firing. \
                 Every step in the certificate is still a step the checker can verify, but \
                 derived_triples is a LOWER BOUND on the closure of this table, not the closure"
            ));
        }
        if !refused.is_empty() {
            result["skipped_examples"] = serde_json::json!(skipped_samples);
            result["skipped_reason"] = serde_json::json!(
                "the rule head instantiated to a triple no RDF serialiser can write (a literal in \
                 subject position, or a non-IRI in predicate position). Such conclusions are \
                 neither certified nor used as premises, so this run derives LESS than the table \
                 licenses"
            );
        }
        Ok(result.to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The trusted boundary, verified
//
// `docs/trusted-computing-base.md` enumerates what `lean/` assumes about this
// file. `tests/certificate_boundary_proptest.rs` property-tests the parts that
// need a store. What is left is a handful of pure functions, and those are
// small enough to do better than sample: the Kani harnesses below prove the
// same statements over EVERY input up to a length bound, rather than over the
// inputs a generator happened to draw.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod boundary_tests {
    use super::*;
    use proptest::prelude::*;

    /// `BUILTIN_RULES[r as usize]` is `r`'s row. Everything the emitter does
    /// with the table rests on this one line of arithmetic, and a row inserted
    /// in the middle without a matching enum variant would silently give every
    /// rule after it the next rule's premises.
    #[test]
    fn each_row_is_at_its_own_index() {
        assert_eq!(Rl::ALL.len(), BUILTIN_RULES.len());
        for (i, r) in Rl::ALL.iter().enumerate() {
            assert_eq!(*r as usize, i, "{r:?} is not at index {i}");
        }
        // The two rules that license two conclusions from one premise are the
        // only names that appear twice.
        let mut names: Vec<&str> = BUILTIN_RULES.iter().map(|r| r.name).collect();
        names.sort_unstable();
        let mut dups: Vec<&str> = names.windows(2).filter(|w| w[0] == w[1]).map(|w| w[0]).collect();
        dups.dedup();
        assert_eq!(dups, vec!["scm-eqc1", "scm-eqp1"]);
    }

    /// Terms that carry no separator, so the writer's precondition holds and
    /// the round trip is the thing under test.
    fn term() -> impl Strategy<Value = String> {
        prop_oneof![
            Just(String::new()),
            Just("<http://e/a>".to_string()),
            Just("\"lit\"".to_string()),
            Just("\"a\\tb\"".to_string()),
            Just("_:b0".to_string()),
            Just("e\u{0301}".to_string()),
            Just("\u{0000}".to_string()),
            "[^\t\n\r]{0,6}",
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 1024, ..ProptestConfig::default() })]

        /// Every slot a rule pattern mentions is within the arity it declares,
        /// and every variable it declares is actually used. An unused slot is a
        /// binding a call site has to invent a term for; an out-of-range slot
        /// is a panic on a graph nobody happened to test.
        #[test]
        fn every_rule_slot_is_within_its_arity(i in 0usize..BUILTIN_RULES.len()) {
            let r = &BUILTIN_RULES[i];
            let mut seen = vec![false; r.vars.len()];
            for a in r.body.iter().chain(std::iter::once(&r.head)) {
                for s in a {
                    if let Slot::Var(k) = s {
                        prop_assert!(*k < r.vars.len(), "{} slot {k} of {}", r.name, r.vars.len());
                        seen[*k] = true;
                    }
                }
            }
            for (k, used) in seen.iter().enumerate() {
                prop_assert!(*used, "{} declares {} and never uses it", r.name, r.vars[k]);
            }
        }

        /// TCB-15 and TCB-16. The interner is a bijection between the strings it
        /// has seen and the ids it has issued. Every term in every certificate
        /// file passes through it, so if it were not, two different terms would
        /// print the same and a step would be checked against a triple the
        /// engine never used.
        #[test]
        fn tcb_15_16_the_interner_is_a_bijection(ss in prop::collection::vec(term(), 0..16)) {
            let mut i = Interner::new();
            let ids: Vec<u32> = ss.iter().map(|s| i.intern(s)).collect();
            for (s, &id) in ss.iter().zip(&ids) {
                prop_assert_eq!(i.resolve(id), s.as_str());
            }
            for a in 0..ss.len() {
                for b in 0..ss.len() {
                    prop_assert_eq!(ids[a] == ids[b], ss[a] == ss[b]);
                }
            }
        }

        /// TCB-17. An id keeps its meaning as more strings arrive. The
        /// certificate writers resolve ids long after the reasoning that
        /// created them.
        #[test]
        fn tcb_17_interning_more_does_not_move_an_id(
            early in prop::collection::vec(term(), 1..8),
            late in prop::collection::vec(term(), 0..8),
        ) {
            let mut i = Interner::new();
            let ids: Vec<u32> = early.iter().map(|s| i.intern(s)).collect();
            for s in &late {
                i.intern(s);
            }
            for (s, &id) in early.iter().zip(&ids) {
                prop_assert_eq!(i.resolve(id), s.as_str());
            }
        }

        /// TCB-1, as a statement about the writer alone. The writer accepts
        /// exactly the terms that fit the format, and what it accepts splits
        /// back into exactly those three terms. A refusal leaves the buffer
        /// untouched.
        ///
        /// This used to be a statement about terms the generator kept
        /// separator-free, because the writer took anything. The writer now
        /// decides, so the test covers both verdicts. The Kani harnesses below
        /// prove the same statement over every byte pattern up to a bound; this
        /// one runs in CI without a model checker installed and samples the
        /// lengths, including the empty string.
        #[test]
        fn tcb_1_an_asserted_line_splits_back_into_its_three_terms(
            s in term(), p in term(), o in term(),
        ) {
            let mut out = String::from("<a>\t<b>\t<c>\n");
            let before = out.clone();
            let r = push_asserted_line(&mut out, &s, &p, &o);
            let fits = term_fits_the_format(&s)
                && term_fits_the_format(&p)
                && term_fits_the_format(&o);
            prop_assert_eq!(r.is_ok(), fits);
            if r.is_err() {
                prop_assert_eq!(out, before, "a refused line left a fragment behind");
                return Ok(());
            }
            prop_assert!(out.ends_with('\n'));
            let line = out.strip_prefix(&before).expect("the line was appended");
            let body = &line[..line.len() - 1];
            prop_assert!(!body.contains('\n'));
            let f: Vec<&str> = body.split('\t').collect();
            prop_assert_eq!(f, vec![s.as_str(), p.as_str(), o.as_str()]);
        }

        /// TCB-2 and TCB-3, as a statement about the writer alone.
        #[test]
        fn tcb_2_3_triple_fields_append_exactly_three_fields(
            head in "[a-z0-9-]{1,8}",
            ts in prop::collection::vec((term(), term(), term()), 1..4),
        ) {
            let mut out = String::new();
            out.push_str(&head);
            let mut written = 0usize;
            for (s, p, o) in &ts {
                let fits = term_fits_the_format(s)
                    && term_fits_the_format(p)
                    && term_fits_the_format(o);
                let before = out.clone();
                let r = push_triple_fields(&mut out, s, p, o);
                prop_assert_eq!(r.is_ok(), fits);
                if r.is_err() {
                    prop_assert_eq!(out.clone(), before);
                } else {
                    written += 1;
                }
            }
            let f: Vec<&str> = out.split('\t').collect();
            prop_assert_eq!(f.len(), 1 + 3 * written);
            prop_assert_eq!(f[0], head.as_str());
            let mut i = 0usize;
            for (s, p, o) in ts.iter() {
                if !(term_fits_the_format(s) && term_fits_the_format(p) && term_fits_the_format(o)) {
                    continue;
                }
                prop_assert_eq!(f[1 + 3 * i], s.as_str());
                prop_assert_eq!(f[2 + 3 * i], p.as_str());
                prop_assert_eq!(f[3 + 3 * i], o.as_str());
                i += 1;
            }
        }

        /// TCB-4 and TCB-5, as a statement about the guard alone. A term the
        /// guard accepts carries no separator and is in one of the three
        /// N-Triples spellings, and two accepted terms of different spellings
        /// are different strings, which is what lets the two checkers compare
        /// terms as opaque strings.
        ///
        /// The Kani harness `term_guard_is_exact` proves this over every byte
        /// pattern at a bound rather than over a sample.
        #[test]
        fn tcb_4_5_the_guard_decides_separators_and_spelling(a in term(), b in term()) {
            for t in [&a, &b] {
                if term_fits_the_format(t) {
                    prop_assert!(!t.contains(['\t', '\n', '\r']));
                    prop_assert!(!t.is_empty());
                    prop_assert!(t.starts_with('<') || t.starts_with('"') || t.starts_with("_:"));
                }
            }
            if term_fits_the_format(&a) && term_fits_the_format(&b) {
                let kind = |t: &str| t.as_bytes()[0];
                if kind(&a) != kind(&b) {
                    prop_assert_ne!(&a, &b);
                }
            }
        }

        /// TCB-20 at one rule position. `parse_pat` and `Pat::render` are
        /// inverse, so a constant never renders to something that reads back as
        /// a variable. `run_horn`'s re-parse guard rests on this.
        ///
        /// Both directions, and `pat_of` is the classifier `parse_pat` is now
        /// built on, so this also pins that the refactor did not move the
        /// boundary between accept and refuse.
        #[test]
        fn tcb_20_parse_pat_and_render_are_inverse(field in "[^\t\n\r]{0,8}") {
            let via_pat_of = pat_of(&field);
            prop_assert_eq!(
                parse_pat(&field, 1, "position").ok(),
                via_pat_of.clone(),
                "parse_pat and pat_of disagree on {:?}", field
            );
            if let Some(p) = via_pat_of {
                prop_assert_eq!(p.render(), field);
            }
        }

        /// TCB-20, the other direction: a pattern renders to a field that reads
        /// back as the same pattern. This is the direction `rules_tsv` needs,
        /// because it renders the table the checker then parses.
        #[test]
        fn tcb_20_render_then_parse_returns_the_pattern(
            name in "[^\t\n\r]{1,8}",
            c in prop_oneof![
                Just("<http://e/a>".to_string()),
                Just("\"lit\"".to_string()),
                Just("_:b0".to_string()),
            ],
            is_var in any::<bool>(),
        ) {
            let p = if is_var { Pat::Var(name) } else { Pat::Const(c) };
            prop_assert_eq!(pat_of(&p.render()), Some(p));
        }
    }
}

/// Bounded model checking of the pure functions on the trusted boundary.
///
/// Run with `cargo kani --harness <name>`. Not part of `cargo test`: Kani
/// compiles the crate with its own toolchain, and a `#[cfg(kani)]` block is
/// invisible to a normal build. What each harness proves and what it does NOT
/// prove is written on it, because a bounded proof read as an unbounded one is
/// the same defect this whole layer exists to attack.
#[cfg(kani)]
mod kani_harnesses {
    use super::*;

    /// An ASCII string of EXACTLY `N` bytes, every byte unconstrained below
    /// 0x80. `kani::any` cannot produce a `String`; ASCII because every byte
    /// below 0x80 is valid UTF-8 on its own.
    ///
    /// Two decisions here are what make these harnesses terminate, and both are
    /// worth recording because the first attempts did not.
    ///
    /// `from_utf8_unchecked` rather than `from_utf8`. With the checked version
    /// CBMC symbolically executes `core::str::validations::run_utf8_validation`
    /// once per buffer, and that loop, not the function under test, becomes
    /// what the harness measures: seventeen minutes and three gigabytes with no
    /// verdict, the log a wall of `Unwinding loop ... run_utf8_validation`. The
    /// `assume` below already establishes exactly what `from_utf8` would check.
    ///
    /// A FIXED length rather than a symbolic one. A symbolic `n` makes every
    /// offset in the produced line symbolic, every slice bound a case split,
    /// and the cost compounds across three terms: two bytes per term with a
    /// symbolic length reached fifteen gigabytes without a verdict. The length
    /// dimension is covered instead by INSTANTIATING the generic harnesses
    /// below at several lengths, `0` included, so the claim is "every byte
    /// pattern at each of these lengths" rather than "every byte pattern at one
    /// length". That is still a bound and it is stated on each harness.
    fn any_ascii<const N: usize>(buf: &mut [u8; N]) -> &str {
        *buf = kani::any();
        for b in buf.iter() {
            kani::assume(*b < 0x80);
        }
        // SAFETY: every byte is assumed below 0x80, so the buffer is ASCII and
        // therefore valid UTF-8. The assumption is the model checker's, so it
        // holds on every path it explores.
        unsafe { core::str::from_utf8_unchecked(&buf[..]) }
    }

    /// TCB-1 and TCB-4, over every byte pattern at length `N` rather than over
    /// a sample, and with no assumption that the terms are separator-free.
    ///
    /// The writer now DECIDES that, so the harness proves the decision as well
    /// as the layout: `push_asserted_line` returns `Ok` exactly when all three
    /// terms fit the format, and when it does, the line it wrote carries
    /// exactly two tabs and one newline, the newline last, with the three terms
    /// at the offsets those separators imply. When it refuses, the buffer is
    /// byte for byte what it was, so a refusal cannot leave half a line behind.
    ///
    /// Until 15 September 2026 this harness ASSUMED separator-freedom, because
    /// nothing in this repository enforced it: it was a property of `oxrdf`'s
    /// `Display` and of `oxiri` refusing to parse. The assumption is now a
    /// branch of the function under test.
    ///
    /// The claim is about the BYTES, not about Rust's `split`. The reader is
    /// `OOCert.Parse.parseTriples`, which is Lean's `String.splitOn`, so a proof
    /// about `str::split` would be a proof about the wrong splitter. Asserting
    /// on `body.split('\t')` also put CBMC inside `CharSearcher` and ran ten
    /// minutes to two gigabytes without a verdict. The `split` formulation is
    /// kept as a property test in `boundary_tests`, where it is free.
    fn asserted_line_layout<const N: usize>() {
        let (mut a, mut b, mut c) = ([0u8; N], [0u8; N], [0u8; N]);
        let s = any_ascii(&mut a);
        let p = any_ascii(&mut b);
        let o = any_ascii(&mut c);

        // A non-empty buffer, so "nothing was appended" is a real claim and not
        // "the buffer is still empty".
        let mut out = String::with_capacity(4 * N + 8);
        out.push('x');
        let r = push_asserted_line(&mut out, s, p, o);

        let fits =
            term_fits_the_format(s) && term_fits_the_format(p) && term_fits_the_format(o);
        assert!(r.is_ok() == fits);
        if !fits {
            assert!(out.len() == 1);
            assert!(out.as_bytes()[0] == b'x');
            return;
        }

        let w = &out.as_bytes()[1..];

        // Nothing added and nothing lost: three terms, two tabs, one newline.
        assert!(w.len() == 3 * N + 3);

        // The separators are where the grammar says.
        assert!(w[N] == b'\t');
        assert!(w[2 * N + 1] == b'\t');
        assert!(w[3 * N + 2] == b'\n');

        // And nowhere else, so any correct splitter finds exactly three fields.
        let mut tabs = 0usize;
        let mut newlines = 0usize;
        let mut i = 0usize;
        while i < w.len() {
            if w[i] == b'\t' {
                tabs += 1;
            }
            if w[i] == b'\n' {
                newlines += 1;
            }
            i += 1;
        }
        assert!(tabs == 2);
        assert!(newlines == 1);

        // The three terms are at the offsets those separators imply, byte for
        // byte, so no term was altered on the way out.
        let mut k = 0usize;
        while k < N {
            assert!(w[k] == s.as_bytes()[k]);
            assert!(w[N + 1 + k] == p.as_bytes()[k]);
            assert!(w[2 * N + 2 + k] == o.as_bytes()[k]);
            k += 1;
        }
    }

    /// The empty term. Every position is refused and nothing is written.
    #[kani::proof]
    #[kani::unwind(24)]
    fn asserted_line_round_trips_at_0() {
        asserted_line_layout::<0>();
    }

    #[kani::proof]
    #[kani::unwind(24)]
    fn asserted_line_round_trips_at_1() {
        asserted_line_layout::<1>();
    }

    #[kani::proof]
    #[kani::unwind(24)]
    fn asserted_line_round_trips_at_2() {
        asserted_line_layout::<2>();
    }

    /// The length the original harness was fixed at. Renamed from
    /// `asserted_line_round_trips` because `cargo kani --harness NAME` selects
    /// by SUBSTRING: the bare name matched all five of these and ran them
    /// together, which is how `_at_4` first failed inside a run reported under
    /// another harness's heading. Every harness name here is now a full name.
    #[kani::proof]
    #[kani::unwind(24)]
    fn asserted_line_round_trips_at_3() {
        asserted_line_layout::<3>();
    }

    #[kani::proof]
    #[kani::unwind(24)]
    fn asserted_line_round_trips_at_4() {
        asserted_line_layout::<4>();
    }

    /// TCB-2, TCB-3 and TCB-4. Three fields appended to a line already begun
    /// come back as three fields, so a step's conclusion and its premises
    /// cannot run into one another, and the writer refuses exactly the terms
    /// that do not fit the format.
    fn triple_fields_layout<const N: usize>() {
        let (mut a, mut b, mut c) = ([0u8; N], [0u8; N], [0u8; N]);
        let s = any_ascii(&mut a);
        let p = any_ascii(&mut b);
        let o = any_ascii(&mut c);

        let mut out = String::with_capacity(4 * N + 8);
        out.push('r');
        let r = push_triple_fields(&mut out, s, p, o);

        let fits =
            term_fits_the_format(s) && term_fits_the_format(p) && term_fits_the_format(o);
        assert!(r.is_ok() == fits);
        if !fits {
            assert!(out.len() == 1);
            assert!(out.as_bytes()[0] == b'r');
            return;
        }

        let w = out.as_bytes();
        assert!(w.len() == 1 + 3 * N + 3);
        assert!(w[0] == b'r');
        assert!(w[1] == b'\t');
        assert!(w[2 + N] == b'\t');
        assert!(w[3 + 2 * N] == b'\t');

        // Exactly three tabs are added and no newline, so the header field
        // keeps its position and the triple occupies exactly the next three.
        let mut tabs = 0usize;
        let mut i = 0usize;
        while i < w.len() {
            if w[i] == b'\t' {
                tabs += 1;
            }
            assert!(w[i] != b'\n');
            i += 1;
        }
        assert!(tabs == 3);

        let mut k = 0usize;
        while k < N {
            assert!(w[2 + k] == s.as_bytes()[k]);
            assert!(w[3 + N + k] == p.as_bytes()[k]);
            assert!(w[4 + 2 * N + k] == o.as_bytes()[k]);
            k += 1;
        }
    }

    #[kani::proof]
    #[kani::unwind(24)]
    fn triple_fields_append_exactly_three_at_0() {
        triple_fields_layout::<0>();
    }

    #[kani::proof]
    #[kani::unwind(24)]
    fn triple_fields_append_exactly_three_at_2() {
        triple_fields_layout::<2>();
    }

    #[kani::proof]
    #[kani::unwind(24)]
    fn triple_fields_append_exactly_three_at_3() {
        triple_fields_layout::<3>();
    }

    #[kani::proof]
    #[kani::unwind(24)]
    fn triple_fields_append_exactly_three_at_4() {
        triple_fields_layout::<4>();
    }

    /// TCB-4. A term the guard accepts carries NO separator, anywhere.
    ///
    /// Stated over a symbolic index rather than as a loop, so it is the
    /// universally quantified claim the Lean parser needs and not a
    /// restatement of the implementation's own scan. This is the property that
    /// used to be an observation about `oxrdf`: the certificate writers are now
    /// the place it is decided, and this is the proof that the decision is the
    /// right one.
    fn term_guard_admits_no_separator<const N: usize>() {
        let mut buf = [0u8; N];
        let t = any_ascii(&mut buf);
        if !term_fits_the_format(t) {
            return;
        }
        assert!(N > 0);
        let i: usize = kani::any();
        kani::assume(i < N);
        let b = t.as_bytes()[i];
        assert!(b != b'\t' && b != b'\n' && b != b'\r');
    }

    #[kani::proof]
    #[kani::unwind(16)]
    fn term_guard_admits_no_separator_at_3() {
        term_guard_admits_no_separator::<3>();
    }

    #[kani::proof]
    #[kani::unwind(24)]
    fn term_guard_admits_no_separator_at_6() {
        term_guard_admits_no_separator::<6>();
    }

    /// TCB-5. The spellings do not collide.
    ///
    /// An accepted term is an IRI, a blank node or a literal and never two of
    /// them, the kind is decided by the leading byte, and two accepted terms of
    /// different kinds are therefore different strings. That is what entitles
    /// `oo-cert` and `oo-horn` to compare terms as opaque strings: a literal
    /// whose lexical form is spelled exactly like an IRI cannot be that IRI,
    /// because the literal is quoted and the IRI is bracketed.
    #[kani::proof]
    #[kani::unwind(16)]
    fn term_guard_separates_the_three_spellings() {
        let (mut x, mut y) = ([0u8; 3], [0u8; 3]);
        let a = any_ascii(&mut x);
        let b = any_ascii(&mut y);
        kani::assume(term_fits_the_format(a));
        kani::assume(term_fits_the_format(b));

        let kinds = |t: &str| -> u8 {
            (t.starts_with('<') as u8)
                + (t.starts_with('"') as u8)
                + (t.starts_with("_:") as u8)
        };
        // Exactly one kind, so the partition is a partition.
        assert!(kinds(a) == 1);
        assert!(kinds(b) == 1);

        if a.as_bytes()[0] != b.as_bytes()[0] {
            assert!(a != b);
        }
    }

    /// The guard that fixed the unwritable-predicate defect. For ANY pair of
    /// strings it decides both positions and never panics. The statement is
    /// small because the function is; its value is that the engine now has
    /// exactly one of these, so this is the only place the question is decided.
    #[kani::proof]
    #[kani::unwind(12)]
    fn writable_triple_decides_both_positions() {
        let (mut a, mut b) = ([0u8; 4], [0u8; 4]);
        let s = any_ascii(&mut a);
        let p = any_ascii(&mut b);
        let ok = writable_triple(s, p);
        assert!(ok == (s.as_bytes()[0] != b'"' && p.as_bytes()[0] == b'<'));
    }

    /// TCB-20. `pat_of` and `Pat::render` are inverse, so a constant never
    /// renders to something that reads back as a variable and a variable name
    /// never renders to something that reads back as a constant. `run_horn`'s
    /// re-parse guard, and therefore the claim that the checker reads the table
    /// the engine evaluated, rests on this.
    ///
    /// **This is the harness the previous version of this file reported as NOT
    /// TERMINATING.** It targeted `parse_pat`, which returns `anyhow::Result`
    /// and formats a message naming the line and the position on every refusal,
    /// so the function body carried the whole formatting machinery. CBMC
    /// flattens a function before it solves, so an `assume` that makes the
    /// refusal paths infeasible is a constraint for the solver and not a cut in
    /// the program: constraining the input made it worse, not better. Measured
    /// then, with nothing else competing: four unconstrained ASCII bytes, no
    /// verdict at 7 minutes and 3.0 GB; two bytes, no verdict at 8 minutes and
    /// 2.8 GB; two bytes with the first constrained to the three spellings the
    /// function accepts, no verdict at 14 minutes and 9.5 GB.
    ///
    /// What that harness said would close it was "lifting the classification
    /// out of `parse_pat` into a pure `Option<Pat>` function, leaving the
    /// messages where they are". That is `pat_of`, and this is the harness. The
    /// input is unconstrained, both verdicts are explored, and the refusal
    /// branch is proved to be exactly the fields that are neither a named
    /// variable nor an N-Triples term.
    fn pat_of_and_render_are_inverse<const N: usize>() {
        let mut buf = [0u8; N];
        let field = any_ascii(&mut buf);
        let b = field.as_bytes();
        let named_var = N > 1 && b[0] == b'?';
        let nt_term = N > 0
            && (b[0] == b'<' || b[0] == b'"' || (b[0] == b'_' && N > 1 && b[1] == b':'));
        match pat_of(field) {
            Some(p) => {
                assert!(named_var || nt_term);
                assert!(p.render() == field);
            }
            None => assert!(!named_var && !nt_term),
        }
    }

    #[kani::proof]
    #[kani::unwind(12)]
    fn pat_of_and_render_are_inverse_at_2() {
        pat_of_and_render_are_inverse::<2>();
    }

    #[kani::proof]
    #[kani::unwind(12)]
    fn pat_of_and_render_are_inverse_at_3() {
        pat_of_and_render_are_inverse::<3>();
    }
}
