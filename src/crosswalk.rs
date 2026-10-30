//! Certify what a crosswalk CLAIMS against what the two ontologies ENTAIL.
//!
//! A mapping file says `ies:Person skos:exactMatch hqdm:Person`. Nothing checks
//! it. `src/align.rs` chooses that predicate from label similarity and property
//! overlap, which is a guess from surface features, and once written down the
//! guess is indistinguishable from a fact. Every crosswalk export in this space
//! has the same property: the match TYPE is asserted and never verified.
//!
//! That matters because the type is the whole claim. `exactMatch` and
//! `broadMatch` licence different downstream reasoning, and the case this
//! repository has already measured is the expensive one: the Foundry crosswalk
//! found IES 5.0.3 losing 202 `rdfs:subPropertyOf` axioms against HQDM. A
//! mapping asserting `exactMatch` across that gap is not slightly wrong, it is
//! claiming two concepts are interchangeable when one side cannot express what
//! the other says.
//!
//! # What is checked
//!
//! For a pair `(a, b)`, each side is reasoned INDEPENDENTLY under OWL 2 RL, in
//! its own store. Then the entailed named superclasses of `a` are translated
//! into the target vocabulary through the mapping itself, and compared with
//! those of `b`.
//!
//! * The two translated ancestor sets agree, and nothing was lost in
//!   translation: `exactMatch` stands.
//! * `a` is under everything `b` is under and more: `a` is the more specific
//!   concept, so `a ⊑ b`, so the tightest true predicate is `broadMatch` —
//!   `skos:broadMatch` is a sub-property of `skos:broader`, and `A broader B`
//!   means B is the BROADER concept. Getting that direction backwards is the
//!   exact silent error this module exists to catch, so
//!   `tests/crosswalk_certify_test.rs` pins it with a worked example rather
//!   than trusting this paragraph.
//! * `b` is the more specific one: `narrowMatch`.
//! * The two disagree in both directions: `relatedMatch`, which is the
//!   weakest thing SKOS lets you say and the honest answer when the
//!   vocabularies do not line up.
//!
//! # Only what can be compared is compared
//!
//! An ancestor of the subject that the mapping carries no row for has no
//! image in the target, so nothing over there can agree or disagree with it.
//! The same holds the other way for an ancestor of the object that no row
//! maps onto. Both are COVERAGE gaps: they are named on the row
//! (`untranslatable`, `object_uncovered`), the unmapped source terms are
//! listed under `unmapped`, and the row's `certification` word says the
//! comparison was partial. Neither is evidence that the two terms differ, and
//! neither downgrades the claim. This used to be otherwise: an untranslatable
//! ancestor forced `relatedMatch`, so one missing row for a section heading
//! downgraded every term beneath it, and a target-side section with no
//! source counterpart turned every correct `exactMatch` from a flat list into
//! `narrowMatch`. A feasibility run on a food-waste taxonomy downgraded every
//! row that way, which made the check carry no information at all.
//!
//! # What a supported predicate establishes, and what it cannot
//!
//! The comparison is between POSITIONS in two hierarchies, read through the
//! mapping. It can refute a claim (the subject sits under a mapped concept
//! the object does not), and it can fail to refute one. It cannot confirm
//! that two terms mean the same thing: two siblings under one mapped parent
//! have identical translated ancestors, so a row pairing the wrong sibling is
//! upheld exactly as the right one is. That is why the output never
//! STRENGTHENS a claim: a row that claimed `closeMatch` keeps `closeMatch` in
//! the SSSOM predicate column even when the hierarchy is compatible with
//! `exactMatch`, because compatibility is not evidence of identity.
//!
//! # What is NOT checked, and is said rather than hidden
//!
//! This is not a proof. The comparison is over NAMED superclasses entailed by
//! this engine's OWL 2 RL rule table, which is 29 of the profile's 78 rules,
//! and a class whose relationship is expressible only through a construct
//! those rules do not evaluate contributes nothing to either side. So a
//! `certified_exact` here means "nothing this engine can see refutes it", not
//! "the two concepts are identical". The report says so in its own `means`
//! field, and `onto_dlp_boundary` is the tool that says which of your axioms
//! were invisible.
//!
//! Nothing here mints a `verdict::Certified`. No Lean checker has a theorem
//! about SKOS match types, so the word `certified` in this module is this
//! engine's own and is spelled `checked_by_this_engine` on the wire, exactly
//! as `EngineRefutation` spells its own opinions.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::graph::GraphStore;
use crate::reason::{InferenceTarget, Reasoner};

/// The SKOS mapping predicates, tightest first.
///
/// Order matters: the certifier reports the TIGHTEST predicate the evidence
/// supports, so a scan that stopped at the first supported one would report
/// `relatedMatch` for everything.
pub const TIGHTEST_FIRST: [&str; 4] =
    ["exactMatch", "broadMatch", "narrowMatch", "relatedMatch"];

const SKOS: &str = "http://www.w3.org/2004/02/skos/core#";

/// One row of a crosswalk: a claim about two terms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mapping {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// What the evidence supports for one pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certified {
    pub mapping: Mapping,
    /// The tightest predicate the entailments support.
    pub supported: &'static str,
    /// True when the claim was stronger than the evidence.
    pub downgraded: bool,
    /// Ancestors of the subject, in the subject's own vocabulary.
    pub subject_ancestors: BTreeSet<String>,
    /// Ancestors of the object.
    pub object_ancestors: BTreeSet<String>,
    /// Subject ancestors the mapping cannot carry into the target vocabulary.
    /// A non-empty set here is the Foundry case: the target cannot express
    /// something the source says. A coverage gap, named rather than scored.
    pub untranslatable: BTreeSet<String>,
    /// Object ancestors that no mapping row maps onto, so nothing on the
    /// source side can be compared with them. The same gap from the other
    /// side.
    pub object_uncovered: BTreeSet<String>,
    /// One word for what the check did: `downgraded`, `upheld` (compared,
    /// nothing missing), `upheld_with_coverage_gap` (compared, some
    /// ancestors could not be), or `not_refuted_nothing_compared` (no
    /// ancestor on either side could be compared, whether because there is
    /// no hierarchy or because the mapping carries none of it, so the row is
    /// neither refuted nor supported by anything and is not certified).
    pub certification: &'static str,
    /// Why, in one sentence a reader can act on.
    pub because: String,
}

/// A term the crosswalk cannot map, and the reason.
///
/// This is the half that vanishes from every crosswalk export. A reviewer
/// needs the list of what CANNOT map at least as much as the list of what
/// does, and today that information is simply absent from the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    pub term: String,
    pub side: &'static str,
    pub reason: String,
}

fn strip(t: &str) -> String {
    t.trim().trim_start_matches('<').trim_end_matches('>').to_string()
}

/// Entailed named superclasses of every class, under OWL 2 RL.
///
/// `materialize` is true because the comparison is about what each ontology
/// ENTAILS, not about what its author happened to write down. That is the
/// whole point: a crosswalk that looks right against asserted triples can be
/// wrong against entailed ones.
///
/// This is the hierarchy the contradiction probe carries into the target as
/// `rdfs:subClassOf`, so it holds subsumption and nothing else.
fn ancestors(store: &Arc<GraphStore>) -> anyhow::Result<BTreeMap<String, BTreeSet<String>>> {
    Reasoner::run_full(store, "owl-rl", true, InferenceTarget::DefaultGraph, None)?;
    let q = "SELECT ?c ?p WHERE { ?c <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?p . \
             FILTER(!isBlank(?c) && !isBlank(?p) && ?c != ?p) }";
    let mut out = collect_pairs(store, q)?;
    for v in out.values_mut() {
        // owl:Thing is entailed above everything and distinguishes nothing.
        v.retain(|p| !p.ends_with("#Thing"));
    }
    out.retain(|_, v| !v.is_empty());
    Ok(out)
}

/// The hierarchy the match-type check compares: entailed superclasses, and
/// for a SKOS vocabulary its broader concepts.
///
/// A SKOS scheme states its hierarchy with `skos:broader` and never with
/// `rdfs:subClassOf`. Read through subclass axioms alone, every concept of a
/// pure SKOS scheme had no ancestors, every row compared two empty sets, and
/// every row was upheld: a deliberately wrong crosswalk certified clean, so
/// the check could not fail on SKOS input at all. `skos:broaderTransitive` is
/// transitive and `skos:broader` is its sub-property (SKOS reference, 8.6.4),
/// so the transitive closure over both is what a SKOS scheme entails. The
/// closure reaches each concept's top concept, which is the axis
/// `skos:topConceptOf` marks, so the axis takes part in the comparison
/// whenever the mapping carries a row for it. Scheme membership
/// (`skos:inScheme`) is not read as an ancestor: every concept of a scheme
/// shares it, so it cannot tell two concepts of one scheme apart, and an
/// unmapped scheme would only add the same coverage gap to every row.
///
/// `broader` is not subsumption, which is why this set stays out of the
/// contradiction probe.
fn hierarchy(store: &Arc<GraphStore>) -> anyhow::Result<BTreeMap<String, BTreeSet<String>>> {
    let mut out = ancestors(store)?;
    let q = "SELECT DISTINCT ?c ?p WHERE { \
             ?c (<http://www.w3.org/2004/02/skos/core#broader>|<http://www.w3.org/2004/02/skos/core#broaderTransitive>)+ ?p . \
             FILTER(isIRI(?c) && isIRI(?p) && ?c != ?p) }";
    for (c, ps) in collect_pairs(store, q)? {
        out.entry(c).or_default().extend(ps);
    }
    Ok(out)
}

fn collect_pairs(
    store: &Arc<GraphStore>,
    query: &str,
) -> anyhow::Result<BTreeMap<String, BTreeSet<String>>> {
    let js: serde_json::Value = serde_json::from_str(&store.sparql_select(query)?)?;
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in js["results"].as_array().cloned().unwrap_or_default() {
        let (Some(c), Some(p)) = (row["c"].as_str(), row["p"].as_str()) else { continue };
        out.entry(strip(c)).or_default().insert(strip(p));
    }
    Ok(out)
}

/// The tightest SKOS predicate two ancestor sets support.
///
/// Both sets are already restricted to what can be compared: the subject's
/// ancestors that have an image, translated, and the object's ancestors that
/// are an image. Returns the predicate and the sentence explaining it.
/// Superset means MORE ancestors, which means the more specific concept: a
/// concept under more things is lower in the hierarchy.
fn tightest(sub_in_target: &BTreeSet<String>, obj: &BTreeSet<String>) -> (&'static str, String) {
    if sub_in_target == obj {
        return (
            "exactMatch",
            "both sides entail the same ancestors once translated, over every ancestor the \
             mapping lets the two compare."
                .to_string(),
        );
    }
    if sub_in_target.is_superset(obj) {
        return (
            "broadMatch",
            "the subject is under every ancestor the object is under, and more, so the \
             subject is the MORE SPECIFIC concept and the object is the broader one."
                .to_string(),
        );
    }
    if obj.is_superset(sub_in_target) {
        return (
            "narrowMatch",
            "the object is under every ancestor the subject is under, and more, so the \
             object is the more specific concept."
                .to_string(),
        );
    }
    (
        "relatedMatch",
        "each side entails an ancestor the other does not, so neither subsumes the other \
         and only a related-match is defensible."
            .to_string(),
    )
}

/// Certify a crosswalk against the two ontologies it claims to connect.
pub fn certify(
    source: &Arc<GraphStore>,
    target: &Arc<GraphStore>,
    mappings: &[Mapping],
) -> anyhow::Result<(Vec<Certified>, Vec<Gap>)> {
    let sa = hierarchy(source)?;
    let ta = hierarchy(target)?;

    // The mapping is its own translation table, which is the point: a bridge
    // is checked against itself rather than against a second, unstated one.
    let fwd: BTreeMap<&str, &str> =
        mappings.iter().map(|m| (m.subject.as_str(), m.object.as_str())).collect();
    // Every target term some row maps onto. An object ancestor outside this
    // set has no counterpart the source could be compared with.
    let images: BTreeSet<&str> = mappings.iter().map(|m| m.object.as_str()).collect();

    let mut rows = Vec::new();
    for m in mappings {
        let empty = BTreeSet::new();
        let s_anc = sa.get(&m.subject).unwrap_or(&empty).clone();
        let o_anc = ta.get(&m.object).unwrap_or(&empty).clone();

        let mut translated = BTreeSet::new();
        let mut untranslatable = BTreeSet::new();
        for c in &s_anc {
            match fwd.get(c.as_str()) {
                Some(img) => {
                    translated.insert((*img).to_string());
                }
                None => {
                    untranslatable.insert(c.clone());
                }
            }
        }
        let (o_comparable, object_uncovered): (BTreeSet<String>, BTreeSet<String>) =
            o_anc.iter().cloned().partition(|c| images.contains(c.as_str()));

        let (supported, mut because) = tightest(&translated, &o_comparable);
        let claimed = m.predicate.rsplit(['#', '/']).next().unwrap_or("").to_string();
        let rank = |p: &str| TIGHTEST_FIRST.iter().position(|q| *q == p).unwrap_or(usize::MAX);
        // Stronger than the evidence, by the SKOS ordering. A claim that was
        // already weaker than the evidence is not "upgraded" here: the file
        // said less than it could, which is not a defect.
        let downgraded = rank(&claimed) < rank(supported);
        let gap = !untranslatable.is_empty() || !object_uncovered.is_empty();
        let compared = !translated.is_empty() || !o_comparable.is_empty();
        if gap {
            because.push_str(&format!(
                " {} ancestor(s) of the subject have no image in the target and {} ancestor(s) \
                 of the object have no counterpart in the source; they could not be compared, \
                 which is a gap in the mapping's coverage and not evidence that the two terms \
                 differ.",
                untranslatable.len(),
                object_uncovered.len()
            ));
        }
        if !compared {
            because = format!(
                "nothing to compare: neither term has an ancestor the mapping carries across, \
                 so the hierarchy can neither support nor refute this row and it is NOT \
                 certified.{}",
                if gap { " The ancestors that exist have no counterpart through the mapping." } else { "" }
            );
        }
        let certification = if downgraded {
            "downgraded"
        } else if !compared {
            "not_refuted_nothing_compared"
        } else if gap {
            "upheld_with_coverage_gap"
        } else {
            "upheld"
        };
        rows.push(Certified {
            mapping: m.clone(),
            supported,
            downgraded,
            subject_ancestors: s_anc,
            object_ancestors: o_anc,
            untranslatable,
            object_uncovered,
            certification,
            because,
        });
    }

    // Every source class with no row at all. Silence in a crosswalk reads as
    // "nothing to say"; it should read as "not mapped, and here is why".
    let mapped: BTreeSet<&str> = mappings.iter().map(|m| m.subject.as_str()).collect();
    let mut gaps = Vec::new();
    for c in sa.keys() {
        if !mapped.contains(c.as_str()) {
            gaps.push(Gap {
                term: c.clone(),
                side: "source",
                reason: "the crosswalk carries no row for this term, so a reader cannot tell \
                         whether it was considered and rejected or never looked at"
                    .to_string(),
            });
        }
    }
    Ok((rows, gaps))
}

/// The report, as the JSON every other tool here returns.
pub fn report(rows: &[Certified], gaps: &[Gap]) -> serde_json::Value {
    let downgrades = rows.iter().filter(|r| r.downgraded).count();
    serde_json::json!({
        "checked": rows.len(),
        "downgraded": downgrades,
        "gaps": gaps.len(),
        "rows": rows.iter().map(|r| serde_json::json!({
            "subject": r.mapping.subject,
            "object": r.mapping.object,
            "claimed": r.mapping.predicate,
            "supported": format!("{SKOS}{}", r.supported),
            "downgraded": r.downgraded,
            "certification": r.certification,
            "because": r.because,
            "subject_ancestors": r.subject_ancestors.iter().collect::<Vec<_>>(),
            "object_ancestors": r.object_ancestors.iter().collect::<Vec<_>>(),
            "untranslatable": r.untranslatable.iter().collect::<Vec<_>>(),
            "object_uncovered": r.object_uncovered.iter().collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "unmapped": gaps.iter().map(|g| serde_json::json!({
            "term": g.term, "side": g.side, "reason": g.reason,
        })).collect::<Vec<_>>(),
        "verdict": "checked_by_this_engine",
        "means": "each side was reasoned INDEPENDENTLY under OWL 2 RL and the entailed named \
                  ancestors were compared through the mapping itself. What this checks is the \
                  POSITION of the two terms in their hierarchies, not what they mean: a \
                  downgrade is a refutation (the subject sits under a mapped concept the \
                  object does not, or the reverse), and an upheld row means only that nothing \
                  this engine can see refutes it. Two siblings under one mapped parent have \
                  identical translated ancestors, so a row pairing the wrong sibling is upheld \
                  exactly as the right one is, which is why the predicate column never \
                  strengthens a claim. Only ancestors both sides can name are compared: an \
                  ancestor with no counterpart through the mapping is a coverage gap, named on \
                  the row and never counted as a disagreement, and a row with nothing \
                  comparable, including a term with no hierarchy at all, is \
                  not_refuted_nothing_compared and is NOT certified. A SKOS scheme's hierarchy is \
                  its skos:broader closure. It is NOT a proof: the \
                  comparison is over 29 of OWL 2 RL's 78 rules, and an axiom no rule fires on \
                  contributes to neither side. Run onto_dlp_boundary to see which of your \
                  axioms were invisible. No Lean checker has a theorem about SKOS match types, \
                  so nothing here is certified in this repository's sense of that word.",
    })
}

/// A valid SSSOM mapping set, with the certification as extra columns.
///
/// Extra columns rather than a new format, so a reader who already has SSSOM
/// tooling gets this as a strict upgrade and nobody is asked to adopt
/// anything. `predicate_id` carries the SUPPORTED predicate when the claim was
/// downgraded, because a file whose predicate column still said `exactMatch`
/// after this ran would be worse than no check at all. Otherwise it keeps the
/// CLAIMED predicate: a hierarchy that is compatible with `exactMatch` cannot
/// tell two siblings apart, so writing `exactMatch` over a claimed
/// `closeMatch` would manufacture a certainty nobody supplied.
pub fn to_sssom(rows: &[Certified], gaps: &[Gap]) -> String {
    let mut out = String::from(
        "subject_id\tpredicate_id\tobject_id\tmapping_justification\t\
         claimed_predicate_id\tcertification\tcomment\n",
    );
    for r in rows {
        let written = if r.downgraded {
            format!("skos:{}", r.supported)
        } else {
            match r.mapping.predicate.strip_prefix(SKOS) {
                Some(local) => format!("skos:{local}"),
                None => r.mapping.predicate.clone(),
            }
        };
        out.push_str(&format!(
            "{}\t{}\t{}\tsemapv:LogicalReasoning\t{}\t{}\t{}\n",
            r.mapping.subject,
            written,
            r.mapping.object,
            r.mapping.predicate,
            r.certification,
            r.because.replace('\t', " "),
        ));
    }
    for g in gaps {
        out.push_str(&format!(
            "{}\tsssom:NoMapping\t\tsemapv:LogicalReasoning\t\tgap\t{}\n",
            g.term,
            g.reason.replace('\t', " ")
        ));
    }
    out
}

/// Read an SSSOM mapping set.
///
/// Tab separated with a header naming the columns, which is the format the
/// standard specifies and the one anybody already holding a crosswalk has.
/// Comment lines beginning `#` carry SSSOM's YAML metadata block and are
/// skipped rather than parsed: nothing here reads it, and pretending to would
/// be worse than ignoring it.
///
/// A row missing any of the three required columns is an ERROR naming the
/// line, not a row quietly dropped. A crosswalk that silently loses rows
/// during certification is the failure this whole module is about.
pub fn parse_sssom(text: &str) -> anyhow::Result<Vec<Mapping>> {
    let mut lines = text.lines().filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty());
    let header = lines.next().ok_or_else(|| anyhow::anyhow!("the mapping file is empty"))?;
    let cols: Vec<&str> = header.split('\t').map(|c| c.trim()).collect();
    let find = |name: &str| {
        cols.iter().position(|c| *c == name).ok_or_else(|| {
            anyhow::anyhow!("SSSOM requires a {name} column; this file has {cols:?}")
        })
    };
    let (si, pi, oi) = (find("subject_id")?, find("predicate_id")?, find("object_id")?);
    let mut out = Vec::new();
    for (n, line) in lines.enumerate() {
        let f: Vec<&str> = line.split('\t').collect();
        let get = |i: usize| f.get(i).map(|s| s.trim()).unwrap_or("");
        let (s, p, o) = (get(si), get(pi), get(oi));
        if s.is_empty() || p.is_empty() || o.is_empty() {
            anyhow::bail!(
                "row {} is missing one of subject_id, predicate_id, object_id: {line:?}. \
                 Refused rather than dropped, because a crosswalk that loses rows while \
                 being certified is the defect this tool exists to find.",
                n + 2
            );
        }
        out.push(Mapping {
            subject: strip(s),
            predicate: expand(p),
            object: strip(o),
        });
    }
    Ok(out)
}

/// `skos:exactMatch` and the full IRI are the same predicate and must compare
/// equal, or a file using the customary prefix would certify as something else.
fn expand(p: &str) -> String {
    let p = strip(p);
    match p.strip_prefix("skos:") {
        Some(local) => format!("{SKOS}{local}"),
        None => p,
    }
}

/// Certify a crosswalk from three files, as the command line hands them over.
pub fn certify_files(
    source_path: &str,
    target_path: &str,
    mapping_path: &str,
) -> anyhow::Result<(serde_json::Value, String)> {
    let source = Arc::new(GraphStore::new());
    source.load_file(source_path)?;
    let target = Arc::new(GraphStore::new());
    target.load_file(target_path)?;
    let mappings = parse_sssom(&std::fs::read_to_string(mapping_path)?)?;
    let (rows, gaps) = certify(&source, &target, &mappings)?;
    Ok((report(&rows, &gaps), to_sssom(&rows, &gaps)))
}

// ───────────────────────────────────────────────────────────────────────────
// The contradiction probe
// ───────────────────────────────────────────────────────────────────────────

/// Does translating the source's claims into the target CONTRADICT the target?
///
/// A different question from the match-type check above, and a sharper one. A
/// downgrade says the two sides differ in granularity. A contradiction says
/// they DISAGREE: the target's own axioms actively deny something the source
/// asserts, once the mapping has carried it across. That is either a wrong
/// mapping row or two ontologies that genuinely encode incompatible claims
/// about the world, and a person has to decide which.
///
/// The probe is mechanical: build a scratch store holding the TARGET's axioms
/// plus the translated source subsumptions, and reason over it. A clash means
/// the target denies something the mapping carried in. This reuses the ten
/// clash detectors in `reason::find_clashes` rather than inventing a second
/// notion of contradiction, so what counts as a disagreement here is exactly
/// what counts as one everywhere else in this engine.
///
/// Two things make the answer mean what it says.
///
/// * WITNESSES. Those detectors need an individual: `cax-dw` fires on a thing
///   typed with two disjoint classes, and a class made unsatisfiable by a
///   carried subsumption is invisible while nothing is typed with it. A
///   taxonomy or a crosswalk is all TBox and has no individuals, so without
///   help every probe of one would come back clean by construction. Each
///   target class that receives a carried claim therefore gets one fresh
///   witness individual, which turns "this class can have no members" into a
///   clash the detectors see.
/// * A BASELINE. The witnesses are also typed into the target ALONE, without
///   the carried claims, and reasoned. A target that clashes on its own, or
///   whose own classes are unsatisfiable, is reported under `target_alone`
///   and is not blamed on the mapping: only clashes the carried claims ADD
///   count. Reasoning here is monotone, so the probe can only add clashes,
///   and a higher count is exactly the mapping's contribution.
///
/// The limit travels with the answer: those ten are the rules this engine
/// LOOKS FOR, out of the seventeen in OWL 2 RL that conclude false. A clean
/// probe is not a proof of compatibility.
pub fn contradiction_probe(
    source: &Arc<GraphStore>,
    target_ttl: &str,
    mappings: &[Mapping],
) -> anyhow::Result<serde_json::Value> {
    let sa = ancestors(source)?;
    let fwd: BTreeMap<&str, &str> =
        mappings.iter().map(|m| (m.subject.as_str(), m.object.as_str())).collect();

    let mut carried = Vec::new();
    let mut nt = String::new();
    let mut receivers: BTreeSet<String> = BTreeSet::new();
    for (c, anc) in &sa {
        let Some(ci) = fwd.get(c.as_str()) else { continue };
        for a in anc {
            let Some(ai) = fwd.get(a.as_str()) else { continue };
            nt.push_str(&format!(
                "<{ci}> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <{ai}> .\n"
            ));
            carried.push(serde_json::json!({"subject": ci, "ancestor": ai, "from": c}));
            receivers.insert((*ci).to_string());
        }
    }

    let mut witnesses_nt = String::new();
    let mut witnesses = Vec::new();
    for (n, class) in receivers.iter().enumerate() {
        let w = format!("urn:open-ontologies:crosswalk-probe:witness:{n}");
        witnesses_nt.push_str(&format!(
            "<{w}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{class}> .\n"
        ));
        witnesses.push(serde_json::json!({"witness": w, "typed": class}));
    }

    let reason = |with_carried: bool| -> anyhow::Result<serde_json::Value> {
        let store = Arc::new(GraphStore::new());
        store.load_turtle(target_ttl, None)?;
        store.load_turtle(&witnesses_nt, None)?;
        if with_carried {
            store.load_turtle(&nt, None)?;
        }
        let out = Reasoner::run_full(&store, "owl-rl", false, InferenceTarget::DefaultGraph, None)?;
        Ok(serde_json::from_str(&out)?)
    };
    let alone = reason(false)?;
    let probed = reason(true)?;
    let count = |r: &serde_json::Value| {
        if r["inconsistency"]["found"] == serde_json::Value::Bool(true) {
            r["inconsistency"]["clash_count"].as_u64().unwrap_or(1)
        } else {
            0
        }
    };
    let (before, after) = (count(&alone), count(&probed));
    let found = after > before;
    let verdict = if found {
        "the_target_denies_what_the_mapping_carried"
    } else if before > 0 {
        "no_contradiction_added_by_the_mapping_but_the_target_clashes_on_its_own"
    } else {
        "no_contradiction_among_the_rules_this_engine_looks_for"
    };

    Ok(serde_json::json!({
        "carried": carried.len(),
        "carried_claims": carried,
        "witnesses": witnesses,
        "contradicted": found,
        "clashes_added_by_the_mapping": after - before,
        "clashes": probed["inconsistency"]["clashes"],
        "by_rule": probed["inconsistency"]["by_rule"],
        "target_alone": {
            "clash_count": before,
            "by_rule": alone["inconsistency"]["by_rule"],
        },
        "verdict": verdict,
        // A contradiction is a finding a script must not walk past.
        "exit_code": if found { 1 } else { 0 },
        "means": "the target's own axioms, plus the source's entailed subsumptions translated \
                  through the mapping, reasoned together, with one witness individual typed \
                  into every target class that received a carried claim so that a class the \
                  claims make unsatisfiable shows up as a clash. Only clashes the carried \
                  claims ADD are counted; clashes the target has with the witnesses alone are \
                  reported under target_alone and are not blamed on the mapping. A \
                  contradiction here is a DISAGREEMENT and not a granularity gap: either the \
                  mapping row is wrong or the two ontologies encode incompatible claims, and a \
                  person has to decide which. A clean probe is NOT proof of compatibility: ten \
                  of the seventeen OWL 2 RL rules that conclude false are looked for, and \
                  nothing was checked about the other seven.",
    }))
}

/// Run the contradiction probe from three files, as the command line hands
/// them over. The target may be in any RDF format the engine reads.
pub fn probe_files(
    source_path: &str,
    target_path: &str,
    mapping_path: &str,
) -> anyhow::Result<serde_json::Value> {
    let source = Arc::new(GraphStore::new());
    source.load_file(source_path)?;
    let target_ttl = GraphStore::read_as_turtle(target_path)?;
    let mappings = parse_sssom(&std::fs::read_to_string(mapping_path)?)?;
    contradiction_probe(&source, &target_ttl, &mappings)
}
