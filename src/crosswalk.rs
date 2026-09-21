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
//! * The two disagree in both directions, or an ancestor has no image in the
//!   target at all: `relatedMatch`, which is the weakest thing SKOS lets you
//!   say and the honest answer when the vocabularies do not line up.
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
    /// something the source says.
    pub untranslatable: BTreeSet<String>,
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
fn ancestors(store: &Arc<GraphStore>) -> anyhow::Result<BTreeMap<String, BTreeSet<String>>> {
    Reasoner::run_full(store, "owl-rl", true, InferenceTarget::DefaultGraph, None)?;
    let q = "SELECT ?c ?p WHERE { ?c <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?p . \
             FILTER(!isBlank(?c) && !isBlank(?p) && ?c != ?p) }";
    let js: serde_json::Value = serde_json::from_str(&store.sparql_select(q)?)?;
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in js["results"].as_array().cloned().unwrap_or_default() {
        let (Some(c), Some(p)) = (row["c"].as_str(), row["p"].as_str()) else { continue };
        let (c, p) = (strip(c), strip(p));
        // owl:Thing is entailed above everything and distinguishes nothing.
        if p.ends_with("#Thing") {
            continue;
        }
        out.entry(c).or_default().insert(p);
    }
    Ok(out)
}

/// The tightest SKOS predicate two ancestor sets support.
///
/// Returns the predicate and the sentence explaining it. Superset means MORE
/// ancestors, which means the more specific concept: a concept under more
/// things is lower in the hierarchy.
fn tightest(
    sub_in_target: &BTreeSet<String>,
    obj: &BTreeSet<String>,
    untranslatable: &BTreeSet<String>,
) -> (&'static str, String) {
    if !untranslatable.is_empty() {
        let n = untranslatable.len();
        return (
            "relatedMatch",
            format!(
                "{n} entailed ancestor(s) of the subject have no image in the target \
                 vocabulary, so the two cannot be compared where it matters most. This is \
                 the case a crosswalk hides: the target does not carry the distinction."
            ),
        );
    }
    if sub_in_target == obj {
        return (
            "exactMatch",
            "both sides entail the same ancestors once translated, and nothing was lost in \
             translation."
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
    let sa = ancestors(source)?;
    let ta = ancestors(target)?;

    // The mapping is its own translation table, which is the point: a bridge
    // is checked against itself rather than against a second, unstated one.
    let fwd: BTreeMap<&str, &str> =
        mappings.iter().map(|m| (m.subject.as_str(), m.object.as_str())).collect();

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

        let (supported, because) = tightest(&translated, &o_anc, &untranslatable);
        let claimed = m.predicate.rsplit(['#', '/']).next().unwrap_or("").to_string();
        let rank = |p: &str| TIGHTEST_FIRST.iter().position(|q| *q == p).unwrap_or(usize::MAX);
        rows.push(Certified {
            mapping: m.clone(),
            supported,
            // Stronger than the evidence, by the SKOS ordering. A claim that was
            // already weaker than the evidence is not "upgraded" here: the file
            // said less than it could, which is not a defect.
            downgraded: rank(&claimed) < rank(supported),
            subject_ancestors: s_anc,
            object_ancestors: o_anc,
            untranslatable,
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
            "because": r.because,
            "subject_ancestors": r.subject_ancestors.iter().collect::<Vec<_>>(),
            "object_ancestors": r.object_ancestors.iter().collect::<Vec<_>>(),
            "untranslatable": r.untranslatable.iter().collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "unmapped": gaps.iter().map(|g| serde_json::json!({
            "term": g.term, "side": g.side, "reason": g.reason,
        })).collect::<Vec<_>>(),
        "verdict": "checked_by_this_engine",
        "means": "each side was reasoned INDEPENDENTLY under OWL 2 RL and the entailed named \
                  ancestors were compared through the mapping itself. A supported predicate \
                  means nothing this engine can see refutes it. It is NOT a proof: the \
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
/// anything. `predicate_id` carries the SUPPORTED predicate, because a file
/// whose predicate column still said `exactMatch` after this ran would be
/// worse than no check at all.
pub fn to_sssom(rows: &[Certified], gaps: &[Gap]) -> String {
    let mut out = String::from(
        "subject_id\tpredicate_id\tobject_id\tmapping_justification\t\
         claimed_predicate_id\tcertification\tcomment\n",
    );
    for r in rows {
        out.push_str(&format!(
            "{}\tskos:{}\t{}\tsemapv:LogicalReasoning\t{}\t{}\t{}\n",
            r.mapping.subject,
            r.supported,
            r.mapping.object,
            r.mapping.predicate,
            if r.downgraded { "downgraded" } else { "upheld" },
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
    for (c, anc) in &sa {
        let Some(ci) = fwd.get(c.as_str()) else { continue };
        for a in anc {
            let Some(ai) = fwd.get(a.as_str()) else { continue };
            nt.push_str(&format!(
                "<{ci}> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <{ai}> .\n"
            ));
            carried.push(serde_json::json!({"subject": ci, "ancestor": ai, "from": c}));
        }
    }

    let probe = Arc::new(GraphStore::new());
    probe.load_turtle(target_ttl, None)?;
    probe.load_turtle(&nt, None)?;
    let out = Reasoner::run_full(&probe, "owl-rl", false, InferenceTarget::DefaultGraph, None)?;
    let r: serde_json::Value = serde_json::from_str(&out)?;
    let found = r["inconsistency"]["found"] == serde_json::Value::Bool(true);

    Ok(serde_json::json!({
        "carried": carried.len(),
        "carried_claims": carried,
        "contradicted": found,
        "clashes": r["inconsistency"]["clashes"],
        "by_rule": r["inconsistency"]["by_rule"],
        "verdict": if found { "the_target_denies_what_the_mapping_carried" }
                   else { "no_contradiction_among_the_rules_this_engine_looks_for" },
        "means": "the target's own axioms, plus the source's entailed subsumptions translated \
                  through the mapping, reasoned together. A contradiction here is a DISAGREEMENT \
                  and not a granularity gap: either the mapping row is wrong or the two \
                  ontologies encode incompatible claims, and a person has to decide which. A \
                  clean probe is NOT proof of compatibility: ten of the seventeen OWL 2 RL \
                  rules that conclude false are looked for, and nothing was checked about the \
                  other seven.",
    }))
}
