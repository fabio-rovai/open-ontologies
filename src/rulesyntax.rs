//! Standard rule syntaxes read into the Horn rule table the Lean checker takes.
//!
//! # The gap this closes
//!
//! `lean/OOCert/Horn.lean` accepts a certificate over ANY rule table and
//! `OOCert.horn_certificate_sound` is proved once for all of them, so decision
//! 0003 says the logic-programming family — Datalog, RIF Core, SWRL — is
//! covered. `src/reason.rs` then evaluates a table and writes the certificate.
//! What neither of them had was a way IN: the only rule syntax anything here
//! could read was the tab-separated `rules.tsv` the Lean parser happens to use,
//! which is an internal encoding and not a language anybody writes. A user
//! holding a SWRL ontology or a RIF Core document could not use any of it.
//!
//! This file is the way in. It reads SWRL out of a loaded RDF graph and RIF
//! Core out of its normative XML syntax, and produces `crate::reason::
//! RulePattern` values: the same type `reason::run_horn` evaluates and the same
//! table `oo-horn check` is given.
//!
//! # What a front end may never do, and what it therefore does instead
//!
//! Both languages say more than a Horn table over triple patterns can. SWRL has
//! built-in atoms over the data domain, same-individual and different-individual
//! atoms, and data ranges. RIF Core has equality, external functions and
//! predicates, local constants and list terms.
//!
//! A rule using any of those is REFUSED BY NAME AND COUNTED. It is never
//! dropped, never approximated, and never turned into the nearest thing that
//! fits. The reason is the failure this project exists to attack: a rule set
//! that quietly lost half its rules, evaluated to a fixpoint, and then reported
//! a certificate that checks green is assurance laundering with a proof
//! attached. The certificate would be perfectly sound — about a rule set nobody
//! wrote.
//!
//! So the default is that ONE refused rule fails the whole import and no table
//! is written. `allow_partial` is the opt-in, and a partial import carries
//! `certifies_a_weaker_rule_set: true`, a per-construct census and a per-rule
//! list of what went, in the vocabulary `src/tableaux.rs` already uses for the
//! constructs its model certificates do not cover.
//!
//! # The verdict, which is why this is safe to build at all
//!
//! Every rule this file produces is a rule a USER wrote. Nothing discharges it
//! against the RDF semantics, so a certificate over such a table earns
//! `entailed_under_supplied_rules` and names `OOCert.horn_certificate_sound`:
//! true in every model of the asserted graph THAT ALSO SATISFIES THOSE RULES. A
//! SWRL rule reading "every supplier is compliant" makes certificates that check
//! green for ever, and the certificate certifies the inference and never the
//! premises.
//!
//! `oo-horn` decides that by comparing the table it is given against
//! `Builtin.asHorn`, line for line, names included. Every rule emitted here is
//! named `swrl/…` or `rif/…` and no built-in rule is, so a table out of this
//! file can never BE the built-in table and can never earn the absolute
//! verdict. That is a structural property of the naming and not an observation
//! about the tables anyone has tried; `a_swrl_rule_never_earns_the_absolute_verdict`
//! and `a_rif_rule_never_earns_the_absolute_verdict` pin it end to end, and
//! `no_front_end_can_name_a_rule_the_way_a_built_in_is_named` pins the naming
//! itself without needing Lean present.
//!
//! This file states no verdict of its own, for the reason `run_horn` gives: a
//! second place that pronounces is a second place the two verdicts can be
//! confused. What it reports is which of the two a table of its making is
//! ELIGIBLE for, which is a fact about provenance and not a judgement on a
//! certificate.

use crate::reason::{AtomPat, Pat, RulePattern, parse_rules, rules_tsv};
use crate::tableaux::TripleIndex;
use std::collections::BTreeMap;
use std::sync::Arc;

// ── Vocabulary ──────────────────────────────────────────────────────────────

const RDF_TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const RDFS_SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const RDF_NIL: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>";
const RDF_PLAIN_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral";

macro_rules! swrl {
    ($local:literal) => {
        concat!("<http://www.w3.org/2003/11/swrl#", $local, ">")
    };
}

const SWRL_IMP: &str = swrl!("Imp");
const SWRL_BODY: &str = swrl!("body");
const SWRL_HEAD: &str = swrl!("head");
const SWRL_VARIABLE: &str = swrl!("Variable");
const SWRL_CLASS_ATOM: &str = swrl!("ClassAtom");
const SWRL_INDIVIDUAL_PROPERTY_ATOM: &str = swrl!("IndividualPropertyAtom");
const SWRL_DATAVALUED_PROPERTY_ATOM: &str = swrl!("DatavaluedPropertyAtom");
const SWRL_SAME_INDIVIDUAL_ATOM: &str = swrl!("SameIndividualAtom");
const SWRL_DIFFERENT_INDIVIDUALS_ATOM: &str = swrl!("DifferentIndividualsAtom");
const SWRL_BUILTIN_ATOM: &str = swrl!("BuiltinAtom");
const SWRL_DATA_RANGE_ATOM: &str = swrl!("DataRangeAtom");
const SWRL_CLASS_PREDICATE: &str = swrl!("classPredicate");
const SWRL_PROPERTY_PREDICATE: &str = swrl!("propertyPredicate");
const SWRL_ARGUMENT1: &str = swrl!("argument1");
const SWRL_ARGUMENT2: &str = swrl!("argument2");
const SWRL_BUILTIN: &str = swrl!("builtin");

const RIF_NS: &str = "http://www.w3.org/2007/rif#";
const RIF_IRI_TYPE: &str = "http://www.w3.org/2007/rif#iri";
const RIF_LOCAL_TYPE: &str = "http://www.w3.org/2007/rif#local";

/// The prose both the docs and every tool response quote, so the supported
/// fragment is stated in exactly one place and cannot drift between them.
pub const SWRL_FRAGMENT: &str = "SWRL rules encoded in RDF (swrl:Imp with swrl:body and swrl:head \
     as rdf:List atom lists), restricted to the three atom forms that ARE triple patterns: \
     swrl:ClassAtom over a NAMED class (arg1 rdf:type C), swrl:IndividualPropertyAtom (arg1 P \
     arg2) and swrl:DatavaluedPropertyAtom (arg1 P literal-or-variable), with arguments that are \
     swrl:Variable instances, IRIs or literals. A head of n atoms becomes n rules, which is the \
     standard split of a conjunctive consequent and is reported. Everything else is refused";

pub const RIF_FRAGMENT: &str = "RIF Core in the normative XML syntax (the presentation syntax is \
     NOT read), restricted to Forall/Implies rules and ground facts whose conditions and \
     conclusions are Frame, Member, Subclass, or Atom of arity 1 or 2. Frame(o p v) is the \
     triple (o p v); Member(i c) is (i rdf:type c); Subclass(a b) is (a rdfs:subClassOf b); a \
     binary Atom p(a b) is (a p b) and a unary Atom p(a) is (a rdf:type p), which reads a RIF \
     predicate as an RDF property or class and is a CONVENTION, not part of RIF. Exists is \
     accepted in the condition only, where dropping the quantifier is equivalent. And in the \
     conclusion, and a multi-slot Frame in it, become several rules. Everything else is refused";

// ── Results ─────────────────────────────────────────────────────────────────

/// One source rule, or one whole document, that could not be represented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The rule as the source identifies it.
    pub rule: String,
    /// The construct responsible, spelled as the source language spells it.
    pub construct: String,
    /// Why a Horn table over triple patterns cannot hold it.
    pub why: String,
}

/// What a front end made of a source document.
#[derive(Debug, Clone, Default)]
pub struct Import {
    pub syntax: &'static str,
    pub fragment: &'static str,
    /// Rules found in the source, refused ones included.
    pub source_rules: usize,
    pub rules: Vec<RulePattern>,
    pub refused: Vec<Refusal>,
    /// Source rules whose conclusion was a conjunction and so became several
    /// rules. Reported because it makes `rules.len()` exceed `source_rules`
    /// legitimately, and a reader comparing the two deserves to know why.
    pub head_splits: usize,
    /// Things true of this import that are neither a rule nor a refusal.
    pub notes: Vec<String>,
}

impl Import {
    fn new(syntax: &'static str, fragment: &'static str) -> Self {
        Self { syntax, fragment, ..Default::default() }
    }

    fn refuse(&mut self, rule: &str, construct: &str, why: impl Into<String>) {
        self.refused.push(Refusal {
            rule: rule.to_string(),
            construct: construct.to_string(),
            why: why.into(),
        });
    }

    /// Accept one rule, but only after it has survived the exact round trip the
    /// certificate depends on: rendered to a `rules.tsv` line and read back by
    /// `crate::reason::parse_rules`, which is the parser `run_horn` will use on
    /// the file this import writes. A rule that does not come back identical is
    /// refused here rather than written out to fail, or worse to succeed as
    /// something else, later.
    ///
    /// This is also what enforces the two conditions the emitter cannot check
    /// structurally: a head variable must occur in the body, because the engine
    /// has nothing to bind it to otherwise, and every constant must be in
    /// N-Triples spelling, because a constant in any other spelling matches
    /// nothing and the rule silently never fires.
    fn accept(&mut self, id: &str, name: String, body: Vec<AtomPat>, head: AtomPat) {
        let rule = RulePattern { name, body, head };
        let line = rules_tsv(std::slice::from_ref(&rule));
        match parse_rules(&line) {
            Ok(back) if back.len() == 1 && back[0] == rule => self.rules.push(rule),
            Ok(_) => self.refuse(
                id,
                "horn-table",
                "the rule does not read back as itself from the rules.tsv line this would write, \
                 so the checker would verify steps against a different rule than the one imported",
            ),
            Err(e) => self.refuse(id, "horn-table", e.to_string()),
        }
    }

    /// Accept a source rule whose conclusion is a conjunction of `heads`, as
    /// `heads.len()` rules sharing one body. `B -> H1 and H2` and
    /// `(B -> H1) and (B -> H2)` are the same sentence, so nothing is lost and
    /// nothing is invented; the count changes, and the count is reported.
    fn accept_split(&mut self, id: &str, body: Vec<AtomPat>, heads: Vec<AtomPat>) {
        if heads.len() > 1 {
            self.head_splits += 1;
        }
        let many = heads.len() > 1;
        for (i, head) in heads.into_iter().enumerate() {
            let name = if many { format!("{id}#{}", i + 1) } else { id.to_string() };
            self.accept(id, name, body.clone(), head);
        }
    }

    /// A census of the constructs that cost rules, most frequent first, in the
    /// shape `constructs_not_modelled` uses in the DL model-certificate block.
    fn construct_census(&self) -> Vec<serde_json::Value> {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for r in &self.refused {
            *counts.entry(r.construct.as_str()).or_default() += 1;
        }
        let mut rows: Vec<(&str, usize)> = counts.into_iter().collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        rows.into_iter()
            .map(|(k, n)| serde_json::json!({"construct": k, "occurrences": n}))
            .collect()
    }

    /// The part of a response that is the same whatever the caller does next.
    fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "source_syntax": self.syntax,
            "supported_fragment": self.fragment,
            "source_rules": self.source_rules,
            "rules_emitted": self.rules.len(),
            "rules_refused": self.refused.len(),
            "conjunctive_heads_split": self.head_splits,
            // The name the DL layer uses for the same idea: the artefact is
            // about FEWER axioms than the source states, and the verdict must
            // be read against this flag rather than on its own.
            "certifies_a_weaker_rule_set": !self.refused.is_empty(),
            "constructs_not_supported": self.construct_census(),
            "refused": self.refused.iter().map(|r| serde_json::json!({
                "rule": r.rule,
                "construct": r.construct,
                "why": r.why,
            })).collect::<Vec<_>>(),
            "not_covered":
                "a rule outside the fragment above is refused and counted, never approximated. \
                 Where rules_refused is non-zero the emitted table is a WEAKER rule set than the \
                 source document states, and a certificate over it is about that weaker set",
            "notes": self.notes,
            // Provenance, not a verdict. Every rule here was written by a user
            // and discharged by nobody, so the only warrant a certificate over
            // this table can earn is the relativised one.
            //
            // This is the ONE place in the crate that names a checker-owned
            // word outside an echo of the checker's own bytes, and it is not a
            // verdict: the key says "is eligible for", `pronounced_by` says
            // lean/ pronounces, and the constant lives in `verdict.rs` beside
            // the list of words that belong to a checker so a reader who greps
            // for the string lands on the paragraph that explains it.
            "verdict_this_table_is_eligible_for": crate::verdict::ELIGIBLE_UNDER_SUPPLIED_RULES,
            "theorem": "OOCert.horn_certificate_sound",
            "means":
                "a checked certificate over this table says every conclusion is true in every \
                 model of the asserted graph THAT ALSO SATISFIES THESE RULES. The rules are \
                 assumed and never checked: a rule saying every supplier is compliant produces \
                 certificates that check green for ever",
            "why_not_the_absolute_verdict":
                "`entailed` is earned only by a certificate over the BUILT-IN table, which \
                 OOCert.Builtin.asHorn_sound discharges against the RDF semantics. Every rule \
                 emitted here is named swrl/… or rif/… and no built-in rule is, so a table out \
                 of this importer is never that table",
            "pronounced_by":
                "lean/, through `oo-horn check`. This importer states no verdict of its own",
        })
    }
}

// ── SWRL ────────────────────────────────────────────────────────────────────

/// Read every `swrl:Imp` out of a loaded graph.
pub fn swrl_from_graph(graph: &Arc<crate::graph::GraphStore>) -> anyhow::Result<Import> {
    swrl_from_triples(&graph.all_triples()?)
}

/// The same, over triples already in hand. Terms are in N-Triples spelling,
/// which is what `GraphStore::all_triples` produces and what a rule table
/// holds, so no term is respelled anywhere in this path.
pub fn swrl_from_triples(triples: &[(String, String, String)]) -> anyhow::Result<Import> {
    let idx = TripleIndex::new(triples);
    let mut imp = Import::new("swrl", SWRL_FRAGMENT);

    let mut imps: Vec<&str> = triples
        .iter()
        .filter(|(_, p, o)| p == RDF_TYPE && o == SWRL_IMP)
        .map(|(s, _, _)| s.as_str())
        .collect();
    imps.sort_unstable();
    imps.dedup();
    imp.source_rules = imps.len();

    for node in imps {
        let id = format!("swrl/{}", strip_iri(node));
        // One rule, read in three passes: the atom lists, then the variables
        // they mention, then the atoms themselves. Variables have to be known
        // before any atom is built, because what a variable is CALLED depends
        // on which other variables the same rule uses.
        let body_nodes = match swrl_atom_list(&idx, node, SWRL_BODY, "swrl:body", &id, &mut imp) {
            Some(v) => v,
            None => continue,
        };
        let head_nodes = match swrl_atom_list(&idx, node, SWRL_HEAD, "swrl:head", &id, &mut imp) {
            Some(v) => v,
            None => continue,
        };
        if head_nodes.is_empty() {
            imp.refuse(
                &id,
                "swrl:head of length zero",
                "a rule with an empty consequent concludes falsity, which is an integrity \
                 constraint and not a Horn rule: it has no head atom to derive",
            );
            continue;
        }

        let mut var_iris: Vec<String> = Vec::new();
        for n in body_nodes.iter().chain(head_nodes.iter()) {
            for arg in [SWRL_ARGUMENT1, SWRL_ARGUMENT2] {
                if let Some(v) = idx.object(n, arg)
                    && is_swrl_variable(&idx, &v)
                    && !var_iris.contains(&v)
                {
                    var_iris.push(v);
                }
            }
        }
        let names = variable_names(&var_iris);

        let mut body: Vec<AtomPat> = Vec::new();
        let mut heads: Vec<AtomPat> = Vec::new();
        let mut failed = false;
        for (nodes, target) in [(&body_nodes, &mut body), (&head_nodes, &mut heads)] {
            for n in nodes {
                match swrl_atom(&idx, n, &names) {
                    Ok(a) => target.push(a),
                    Err((construct, why)) => {
                        imp.refuse(&id, &construct, why);
                        failed = true;
                        break;
                    }
                }
            }
            if failed {
                break;
            }
        }
        if failed {
            continue;
        }

        if names.is_empty() {
            // Not an error: a ground rule is a legal SWRL rule. It is also what
            // a rule whose variables were never typed `swrl:Variable` looks
            // like, and that one matches nothing and fires never. Saying so
            // costs a line and catches a whole class of silent no-op.
            imp.notes.push(format!(
                "{id} has no variables. That is legal, and it is also what a rule looks like when \
                 its variables were not declared `rdf:type swrl:Variable`, in which case they \
                 were read as individuals and the rule will never fire"
            ));
        }
        imp.accept_split(&id, body, heads);
    }

    Ok(imp)
}

/// Read one `rdf:List` of atoms hanging off a `swrl:Imp`. A list that does not
/// reach `rdf:nil` cleanly refuses the rule: a truncated body is a rule that
/// fires MORE often than the one the author wrote, which is the one direction
/// an importer may never take quietly.
fn swrl_atom_list(
    idx: &TripleIndex,
    node: &str,
    predicate: &str,
    spelling: &str,
    id: &str,
    imp: &mut Import,
) -> Option<Vec<String>> {
    let Some(head) = idx.object(node, predicate) else {
        imp.refuse(id, format!("missing {spelling}").as_str(), format!(
            "a swrl:Imp needs both {spelling} and its counterpart; this one has no {spelling}, so \
             there is no rule to read"
        ));
        return None;
    };
    if head == RDF_NIL {
        return Some(Vec::new());
    }
    let (items, defect) = idx.walk_list_checked(&head);
    if let Some(d) = defect {
        imp.refuse(
            id,
            format!("malformed {spelling} list").as_str(),
            format!(
                "{}. A truncated atom list is a DIFFERENT rule, and in the body it is a weaker \
                 condition that fires more often, so the rule is refused rather than read short",
                d.describe()
            ),
        );
        return None;
    }
    Some(items)
}

fn is_swrl_variable(idx: &TripleIndex, term: &str) -> bool {
    idx.objects(term, RDF_TYPE).iter().any(|t| t == SWRL_VARIABLE)
}

/// One SWRL atom as a triple pattern, or the construct that stopped it.
fn swrl_atom(
    idx: &TripleIndex,
    node: &str,
    names: &BTreeMap<String, String>,
) -> Result<AtomPat, (String, String)> {
    let types = idx.objects(node, RDF_TYPE);
    let ty = types
        .iter()
        .find(|t| {
            matches!(
                t.as_str(),
                SWRL_CLASS_ATOM
                    | SWRL_INDIVIDUAL_PROPERTY_ATOM
                    | SWRL_DATAVALUED_PROPERTY_ATOM
                    | SWRL_SAME_INDIVIDUAL_ATOM
                    | SWRL_DIFFERENT_INDIVIDUALS_ATOM
                    | SWRL_BUILTIN_ATOM
                    | SWRL_DATA_RANGE_ATOM
            )
        })
        .map(String::as_str);

    let arg = |which: &str, spelling: &str| -> Result<Pat, (String, String)> {
        let Some(v) = idx.object(node, which) else {
            return Err((
                format!("missing {spelling}"),
                format!("the atom {node} has no {spelling}, so one of its positions is unknown"),
            ));
        };
        if is_swrl_variable(idx, &v) {
            let name = names.get(&v).ok_or_else(|| {
                (
                    "swrl:Variable".to_string(),
                    format!("the variable {v} was not collected for this rule"),
                )
            })?;
            Ok(Pat::Var(name.clone()))
        } else {
            Ok(Pat::Const(v))
        }
    };

    match ty {
        Some(SWRL_CLASS_ATOM) => {
            let c = named_predicate(idx, node, SWRL_CLASS_PREDICATE, "swrl:classPredicate", "class")?;
            Ok(AtomPat {
                s: arg(SWRL_ARGUMENT1, "swrl:argument1")?,
                p: Pat::Const(RDF_TYPE.to_string()),
                o: Pat::Const(c),
            })
        }
        Some(SWRL_INDIVIDUAL_PROPERTY_ATOM) | Some(SWRL_DATAVALUED_PROPERTY_ATOM) => {
            let p = named_predicate(
                idx,
                node,
                SWRL_PROPERTY_PREDICATE,
                "swrl:propertyPredicate",
                "property",
            )?;
            Ok(AtomPat {
                s: arg(SWRL_ARGUMENT1, "swrl:argument1")?,
                p: Pat::Const(p),
                o: arg(SWRL_ARGUMENT2, "swrl:argument2")?,
            })
        }
        Some(SWRL_BUILTIN_ATOM) => {
            let b = idx.object(node, SWRL_BUILTIN).unwrap_or_else(|| "?".into());
            Err((
                format!("swrl:BuiltinAtom {}", strip_iri(&b)),
                "a built-in atom is a predicate over the data domain (swrlb:greaterThan, \
                 swrlb:add, swrlb:stringConcat and the rest), computed rather than matched. A \
                 Horn table over triple patterns has no computed predicates: its atoms match \
                 triples that are in the graph. Rewriting the built-in as a triple pattern would \
                 invent a property the graph does not hold and would fire on nothing"
                    .to_string(),
            ))
        }
        Some(SWRL_SAME_INDIVIDUAL_ATOM) => Err((
            "swrl:SameIndividualAtom".to_string(),
            "the atom asserts EQUALITY OF INDIVIDUALS in the domain, not the presence of a \
             triple. Writing it as (a owl:sameAs b) would be a different claim: the checker \
             matches triples and has no congruence, so nothing downstream would substitute equals \
             for equals, and in the head it would licence a conclusion the rule does not make"
                .to_string(),
        )),
        Some(SWRL_DIFFERENT_INDIVIDUALS_ATOM) => Err((
            "swrl:DifferentIndividualsAtom".to_string(),
            "the atom asserts that two individuals are DISTINCT, which is a negative fact. A Horn \
             rule's head is one positive atom and its body is a conjunction of positive atoms, so \
             there is no position in the table for it"
                .to_string(),
        )),
        Some(SWRL_DATA_RANGE_ATOM) => Err((
            "swrl:DataRangeAtom".to_string(),
            "the atom tests a value against a data range (a datatype or an enumeration), which is \
             a condition on the VALUE a literal denotes. The checker compares RDF terms and does \
             no datatype reasoning, so the test has no triple pattern"
                .to_string(),
        )),
        _ => Err((
            "unrecognised swrl atom".to_string(),
            format!(
                "the node {node} in an atom list is typed {} and is none of swrl:ClassAtom, \
                 swrl:IndividualPropertyAtom or swrl:DatavaluedPropertyAtom. An atom this \
                 importer cannot name is not guessed at",
                if types.is_empty() { "nothing".to_string() } else { types.join(", ") }
            ),
        )),
    }
}

/// A `swrl:classPredicate` or `swrl:propertyPredicate` that is a named IRI.
fn named_predicate(
    idx: &TripleIndex,
    node: &str,
    predicate: &str,
    spelling: &str,
    what: &str,
) -> Result<String, (String, String)> {
    let Some(v) = idx.object(node, predicate) else {
        return Err((
            format!("missing {spelling}"),
            format!("the atom {node} has no {spelling}, so its {what} is unknown"),
        ));
    };
    if is_swrl_variable(idx, &v) {
        return Err((
            format!("variable {spelling}"),
            format!(
                "{spelling} is the variable {v}. SWRL requires an IRI there, and a rule that \
                 quantifies over {what}es is not a SWRL rule this importer will invent a reading \
                 for"
            ),
        ));
    }
    if v.starts_with("_:") {
        return Err((
            format!("anonymous {what} expression"),
            format!(
                "{spelling} is the blank node {v}, which is an anonymous {what} expression (a \
                 restriction, an intersection, a union). A triple pattern can hold a NAME; it \
                 cannot hold a class expression, and unfolding one would be a different rule"
            ),
        ));
    }
    if !v.starts_with('<') {
        return Err((
            format!("non-IRI {spelling}"),
            format!("{spelling} is {v}, which is not an IRI"),
        ));
    }
    Ok(v)
}

/// What to call each variable of one rule.
///
/// The local name is what a reader wants to see, and it is only safe when it
/// identifies the variable. Two variable IRIs in one rule with the same local
/// name would become ONE variable in the table, which silently joins two
/// positions the author kept apart: a strictly stronger rule, derived from a
/// cosmetic choice. So a collision, or a local name that is empty or holds
/// whitespace, drops the whole rule to full IRIs, which cannot collide.
fn variable_names(iris: &[String]) -> BTreeMap<String, String> {
    let locals: Vec<String> = iris.iter().map(|i| local_name(i)).collect();
    let usable = locals.iter().all(|l| {
        !l.is_empty() && !l.chars().any(|c| c.is_whitespace()) && locals.iter().filter(|x| *x == l).count() == 1
    });
    iris.iter()
        .enumerate()
        .map(|(i, iri)| {
            let name = if usable { locals[i].clone() } else { strip_iri(iri).to_string() };
            (iri.clone(), name)
        })
        .collect()
}

fn strip_iri(term: &str) -> &str {
    term.strip_prefix('<').and_then(|t| t.strip_suffix('>')).unwrap_or(term)
}

fn local_name(term: &str) -> String {
    let body = strip_iri(term);
    match body.rfind(['#', '/']) {
        Some(i) => body[i + 1..].to_string(),
        None => body.to_string(),
    }
}

// ── RIF Core, XML syntax ────────────────────────────────────────────────────

/// A parsed XML element, namespace resolved.
#[derive(Debug, Default)]
struct El {
    /// Local name, the prefix or default namespace stripped.
    name: String,
    /// The namespace it resolved to, empty when the document declared none.
    ns: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<El>,
}

impl El {
    fn child(&self, name: &str) -> Option<&El> {
        self.children.iter().find(|c| c.name == name)
    }
    fn children_named(&self, name: &str) -> Vec<&El> {
        self.children.iter().filter(|c| c.name == name).collect()
    }
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
    /// The one element a wrapper such as `<formula>` or `<if>` contains.
    fn only_child(&self) -> Option<&El> {
        match self.children.len() {
            1 => self.children.first(),
            _ => None,
        }
    }
    fn in_rif(&self) -> bool {
        self.ns.is_empty() || self.ns == RIF_NS
    }
}

/// Read RIF Core out of its XML syntax.
pub fn rif_core_from_xml(xml: &str) -> anyhow::Result<Import> {
    let trimmed = xml.trim_start_matches(['\u{feff}', ' ', '\t', '\r', '\n']);
    if !trimmed.starts_with('<') {
        anyhow::bail!(
            "this is not RIF XML. Only the XML syntax is read, because it is the normative \
             interchange form; the presentation syntax (`Document( Prefix(...) Group( Forall ?x \
             ( ... :- ... ) ) )`) is a different grammar and is NOT parsed here. A half-written \
             parser for it would mis-read rules rather than refuse them, which is the one failure \
             this importer exists to avoid. Serialise the document to RIF XML and pass that"
        );
    }
    let root = parse_xml(trimmed)?;
    let mut imp = Import::new("rif-core", RIF_FRAGMENT);
    if root.ns.is_empty() {
        imp.notes.push(format!(
            "the document declares no namespace, so its elements were read by local name alone. \
             RIF XML puts them in {RIF_NS}"
        ));
    }
    if !root.in_rif() {
        anyhow::bail!(
            "the document element <{}> is in the namespace {}, not the RIF namespace {RIF_NS}",
            root.name,
            root.ns
        );
    }
    if root.name != "Document" {
        anyhow::bail!(
            "the document element is <{}>; a RIF document element is <Document>",
            root.name
        );
    }

    for d in root.children_named("directive") {
        if let Some(i) = d.child("Import") {
            let loc = i.child("location").map(|l| l.text.clone()).unwrap_or_default();
            imp.source_rules += 1;
            imp.refuse(
                "rif/document",
                "rif:Import",
                format!(
                    "the document imports {} and this importer does not fetch it, so the rule set \
                     it defines is larger than the rules read here",
                    if loc.is_empty() { "another document".into() } else { loc }
                ),
            );
        }
    }

    let mut sentences: Vec<&El> = Vec::new();
    for p in root.children_named("payload") {
        collect_sentences(p, &mut sentences);
    }
    imp.source_rules += sentences.len();

    for (i, s) in sentences.into_iter().enumerate() {
        let id = rif_sentence_id(s, i);
        match rif_rule(s) {
            Ok((body, heads)) => {
                if heads.is_empty() {
                    imp.refuse(
                        &id,
                        "empty conclusion",
                        "the conclusion holds no atom, so there is nothing to derive",
                    );
                } else {
                    imp.accept_split(&id, body, heads);
                }
            }
            Err((construct, why)) => imp.refuse(&id, &construct, why),
        }
    }

    Ok(imp)
}

/// `payload` holds a `Group`; a `Group` holds `sentence`s, each of which may
/// hold another `Group`. Flatten to the sentences that are not groups.
fn collect_sentences<'a>(el: &'a El, out: &mut Vec<&'a El>) {
    for child in &el.children {
        match child.name.as_str() {
            "Group" | "payload" | "sentence" | "formula" => {
                if child.name == "sentence" || child.name == "formula" {
                    match child.only_child() {
                        Some(inner) if inner.name == "Group" => collect_sentences(inner, out),
                        Some(inner) => out.push(inner),
                        None => out.push(child),
                    }
                } else {
                    collect_sentences(child, out);
                }
            }
            "id" | "meta" => {}
            _ => out.push(child),
        }
    }
}

/// A RIF sentence's own name, when it has one.
///
/// `<id>` hangs off whichever element carries it, and in practice that is the
/// `Implies` rather than the `Forall` wrapped around it, so the search walks
/// down through the wrappers instead of looking only at the top. A refusal list
/// naming the author's own rule identifiers is worth a few lines here.
fn rif_sentence_id(s: &El, index: usize) -> String {
    fn find(el: &El, depth: usize) -> Option<String> {
        if let Some(id) = el.child("id")
            && let Some(c) = id.child("Const")
            && !c.text.is_empty()
        {
            return Some(c.text.clone());
        }
        if depth == 0 {
            return None;
        }
        el.child("formula").and_then(|f| f.only_child()).and_then(|inner| find(inner, depth - 1))
    }
    match find(s, 4) {
        Some(name) => format!("rif/{name}"),
        None => format!("rif/sentence{}", index + 1),
    }
}

type RifRule = (Vec<AtomPat>, Vec<AtomPat>);

/// One RIF sentence as a body and a list of head atoms.
fn rif_rule(s: &El) -> Result<RifRule, (String, String)> {
    if !s.in_rif() {
        return Err((
            format!("<{}> in {}", s.name, s.ns),
            format!("the element is not in the RIF namespace {RIF_NS}"),
        ));
    }
    match s.name.as_str() {
        // The declared variables are not carried across: a RulePattern
        // quantifies every variable it mentions, and the condition that
        // actually matters — a head variable must occur in the body — is
        // enforced by `parse_rules` when the rule is accepted.
        "Forall" => {
            let f = s.child("formula").and_then(|f| f.only_child()).ok_or_else(|| {
                (
                    "malformed Forall".to_string(),
                    "a Forall needs exactly one <formula> holding one element".to_string(),
                )
            })?;
            rif_rule(f)
        }
        "Implies" => {
            let cond = s.child("if").and_then(|f| f.only_child()).ok_or_else(|| {
                (
                    "malformed Implies".to_string(),
                    "an Implies needs an <if> holding exactly one element".to_string(),
                )
            })?;
            let conc = s.child("then").and_then(|f| f.only_child()).ok_or_else(|| {
                (
                    "malformed Implies".to_string(),
                    "an Implies needs a <then> holding exactly one element".to_string(),
                )
            })?;
            Ok((rif_formula(cond, true)?, rif_formula(conc, false)?))
        }
        // A bare atomic sentence is a fact: an empty body and a ground head.
        // `parse_rules` refuses it if the head turns out to hold a variable,
        // which it should, since there would be nothing to bind it to.
        "Atom" | "Frame" | "Member" | "Subclass" => Ok((Vec::new(), rif_formula(s, false)?)),
        other => Err((
            format!("rif:{other} as a sentence"),
            format!(
                "a RIF Core sentence is a Forall, an Implies, or a ground fact; <{other}> is none \
                 of those"
            ),
        )),
    }
}

/// A RIF condition (`in_condition`) or conclusion, as a conjunction of triple
/// patterns. The two differ in what they are allowed to hold, so the flag is a
/// parameter rather than two near-identical functions.
fn rif_formula(el: &El, in_condition: bool) -> Result<Vec<AtomPat>, (String, String)> {
    if !el.in_rif() {
        return Err((
            format!("<{}> in {}", el.name, el.ns),
            format!("the element is not in the RIF namespace {RIF_NS}"),
        ));
    }
    match el.name.as_str() {
        "And" => {
            let mut out = Vec::new();
            for f in el.children_named("formula") {
                let inner = f.only_child().ok_or_else(|| {
                    (
                        "malformed And".to_string(),
                        "each <formula> of an And holds exactly one element".to_string(),
                    )
                })?;
                out.extend(rif_formula(inner, in_condition)?);
            }
            Ok(out)
        }
        "Exists" if in_condition => {
            // `(exists y. B(x,y)) -> H(x)` and `forall y. B(x,y) -> H(x)` are
            // the same sentence, so the quantifier is dropped and the variable
            // simply does not occur in the head. That equivalence holds ONLY on
            // this side of the arrow.
            let inner = el.child("formula").and_then(|f| f.only_child()).ok_or_else(|| {
                (
                    "malformed Exists".to_string(),
                    "an Exists needs one <formula> holding one element".to_string(),
                )
            })?;
            rif_formula(inner, in_condition)
        }
        "Exists" => Err((
            "rif:Exists in the conclusion".to_string(),
            "an existential conclusion asserts that something exists without naming it. \
             Representing it needs a Skolem term, which is a NEW constant this importer would \
             have to invent, and a rule with an invented constant is not the rule that was written"
                .to_string(),
        )),
        "Atom" => rif_atom(el),
        "Frame" => rif_frame(el),
        "Member" => {
            let i = rif_term(el.child("instance").and_then(|c| c.only_child()), "Member/instance")?;
            let c = rif_term(el.child("class").and_then(|c| c.only_child()), "Member/class")?;
            Ok(vec![AtomPat { s: i, p: Pat::Const(RDF_TYPE.to_string()), o: c }])
        }
        "Subclass" => {
            let a = rif_term(el.child("sub").and_then(|c| c.only_child()), "Subclass/sub")?;
            let b = rif_term(el.child("super").and_then(|c| c.only_child()), "Subclass/super")?;
            Ok(vec![AtomPat { s: a, p: Pat::Const(RDFS_SUBCLASS.to_string()), o: b }])
        }
        "Equal" => Err((
            if in_condition { "rif:Equal in the condition" } else { "rif:Equal in the conclusion" }
                .to_string(),
            if in_condition {
                "an equality in the condition is a guard that two terms denote the same thing. The \
                 checker matches RDF TERMS and has no congruence, so it cannot decide one"
            } else {
                "an equality in the conclusion asserts that two terms denote the same thing, which \
                 is not a triple. Deriving it would require every later step to substitute equals \
                 for equals, and the Horn checker does not: a rule table has no equality"
            }
            .to_string(),
        )),
        "External" => Err((
            "rif:External".to_string(),
            "an external term or predicate is computed by a function outside the rule set \
             (func:numeric-add, pred:literal-not-identical and the rest). A Horn table matches \
             triples that are in the graph and computes nothing"
                .to_string(),
        )),
        "Or" | "Neg" | "INeg" | "Naf" => Err((
            format!("rif:{}", el.name),
            format!(
                "<{}> is outside RIF Core, which is Horn: a conjunction of positive atoms \
                 implying one positive atom. Disjunction and negation have no place in the table",
                el.name
            ),
        )),
        other => Err((
            format!("rif:{other}"),
            format!("<{other}> is not a RIF Core condition or conclusion this importer reads"),
        )),
    }
}

fn rif_atom(el: &El) -> Result<Vec<AtomPat>, (String, String)> {
    let op = el.child("op").and_then(|c| c.only_child());
    let Some(op) = op else {
        return Err((
            "malformed Atom".to_string(),
            "an Atom needs an <op> holding one element".to_string(),
        ));
    };
    if op.name != "Const" {
        return Err((
            format!("rif:Atom with a {} predicate", op.name),
            "the predicate of an Atom must be a Const naming an IRI. A variable or an expression \
             in predicate position is outside RIF Core"
                .to_string(),
        ));
    }
    let pred = rif_term(Some(op), "Atom/op")?;
    let args: Vec<&El> = el.children_named("args").into_iter().flat_map(|a| a.children.iter()).collect();
    match args.len() {
        1 => {
            let a = rif_term(Some(args[0]), "Atom/args")?;
            Ok(vec![AtomPat { s: a, p: Pat::Const(RDF_TYPE.to_string()), o: pred }])
        }
        2 => {
            let a = rif_term(Some(args[0]), "Atom/args")?;
            let b = rif_term(Some(args[1]), "Atom/args")?;
            Ok(vec![AtomPat { s: a, p: pred, o: b }])
        }
        n => Err((
            format!("rif:Atom of arity {n}"),
            format!(
                "a triple has three positions, so a predicate of arity {n} has no triple pattern. \
                 Arity 1 is read as a class and arity 2 as a property; arity 0 says nothing about \
                 any subject, and arity 3 or more would have to be reified, which is a modelling \
                 decision this importer will not take on the author's behalf"
            ),
        )),
    }
}

fn rif_frame(el: &El) -> Result<Vec<AtomPat>, (String, String)> {
    let obj = rif_term(el.child("object").and_then(|c| c.only_child()), "Frame/object")?;
    let slots = el.children_named("slot");
    if slots.is_empty() {
        return Err((
            "rif:Frame with no slot".to_string(),
            "a frame with no slot states nothing".to_string(),
        ));
    }
    let mut out = Vec::new();
    for slot in slots {
        if slot.children.len() != 2 {
            return Err((
                "malformed rif:Frame slot".to_string(),
                format!(
                    "a <slot> holds exactly two terms, a key and a value; this one holds {}",
                    slot.children.len()
                ),
            ));
        }
        let k = rif_term(Some(&slot.children[0]), "Frame/slot key")?;
        let v = rif_term(Some(&slot.children[1]), "Frame/slot value")?;
        out.push(AtomPat { s: obj.clone(), p: k, o: v });
    }
    Ok(out)
}

/// One RIF term as a pattern position.
fn rif_term(el: Option<&El>, where_: &str) -> Result<Pat, (String, String)> {
    let Some(el) = el else {
        return Err((
            format!("missing {where_}"),
            format!("{where_} holds no term"),
        ));
    };
    if !el.in_rif() {
        return Err((
            format!("<{}> in {}", el.name, el.ns),
            format!("the term at {where_} is not in the RIF namespace {RIF_NS}"),
        ));
    }
    match el.name.as_str() {
        "Var" => {
            if el.text.is_empty() || el.text.chars().any(|c| c.is_whitespace()) {
                return Err((
                    "malformed rif:Var".to_string(),
                    format!("the variable name at {where_} is empty or holds whitespace"),
                ));
            }
            Ok(Pat::Var(el.text.clone()))
        }
        "Const" => {
            let Some(ty) = el.attr("type") else {
                return Err((
                    "rif:Const with no type".to_string(),
                    format!(
                        "the constant at {where_} has no type attribute, so whether it names an \
                         IRI or a literal is unknown"
                    ),
                ));
            };
            match ty {
                RIF_IRI_TYPE => Ok(Pat::Const(format!("<{}>", el.text))),
                RIF_LOCAL_TYPE => Err((
                    "rif:local constant".to_string(),
                    format!(
                        "the constant \"{}\" at {where_} is rif:local, which is scoped to the \
                         document and denotes nothing outside it. Giving it an IRI would be \
                         minting a name the document does not have",
                        el.text
                    ),
                )),
                RDF_PLAIN_LITERAL => Ok(Pat::Const(plain_literal(&el.text))),
                dt => Ok(Pat::Const(format!("\"{}\"^^<{dt}>", escape_literal(&el.text)))),
            }
        }
        "Expr" => Err((
            "rif:Expr".to_string(),
            format!(
                "the term at {where_} is a function application. RIF Core is function-free by \
                 definition, and a triple pattern position holds a name or a variable, never a \
                 term to be evaluated"
            ),
        )),
        "External" => Err((
            "rif:External term".to_string(),
            format!("the term at {where_} is computed by an external function, and nothing here computes"),
        )),
        "List" => Err((
            "rif:List term".to_string(),
            format!(
                "the term at {where_} is a list. A triple position holds one term; an RDF \
                 collection would be several triples and a different graph shape"
            ),
        )),
        other => Err((
            format!("rif:{other} as a term"),
            format!("<{other}> at {where_} is not a RIF Core term this importer reads"),
        )),
    }
}

/// `rdf:PlainLiteral` carries its language tag inside the lexical form, after
/// the last `@`, and an empty tag means no tag at all.
fn plain_literal(text: &str) -> String {
    match text.rfind('@') {
        Some(i) if i + 1 < text.len() => {
            format!("\"{}\"@{}", escape_literal(&text[..i]), &text[i + 1..])
        }
        Some(i) => format!("\"{}\"", escape_literal(&text[..i])),
        None => format!("\"{}\"", escape_literal(text)),
    }
}

/// N-Triples literal escaping. The tab matters twice over: a raw tab in a
/// literal would split a `rules.tsv` line into extra fields and make the rule
/// unreadable, and a raw newline would make it two rules.
fn escape_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// Read the document into `El`s, namespaces resolved.
///
/// Entities beyond the five XML predefines and numeric character references are
/// an error rather than a guess. RIF documents in the wild declare `&rif;` and
/// `&xs;` in a DOCTYPE, which is not processed here, and reading `&rif;iri` as
/// the literal six characters would silently turn every IRI constant into a
/// local one.
fn parse_xml(xml: &str) -> anyhow::Result<El> {
    use quick_xml::events::Event;
    use quick_xml::name::ResolveResult;

    let mut reader = quick_xml::NsReader::from_str(xml);
    reader.config_mut().trim_text(true);
    let decoder = reader.decoder();

    let mut stack: Vec<El> = Vec::new();
    let mut root: Option<El> = None;

    let finish = |stack: &mut Vec<El>, root: &mut Option<El>, el: El| {
        match stack.last_mut() {
            Some(parent) => parent.children.push(el),
            None => *root = Some(el),
        }
    };

    loop {
        let (ns, ev) = reader
            .read_resolved_event()
            .map_err(|e| anyhow::anyhow!("the document is not well-formed XML: {e}"))?;
        let ns = match ns {
            ResolveResult::Bound(n) => String::from_utf8_lossy(n.as_ref()).into_owned(),
            _ => String::new(),
        };
        match ev {
            Event::Start(e) => stack.push(start_element(&ns, &e, decoder)?),
            Event::Empty(e) => {
                let el = start_element(&ns, &e, decoder)?;
                finish(&mut stack, &mut root, el);
            }
            Event::End(_) => {
                let el = stack
                    .pop()
                    .ok_or_else(|| anyhow::anyhow!("the document has an unmatched closing tag"))?;
                finish(&mut stack, &mut root, el);
            }
            Event::Text(e) => {
                let text = e.unescape().map_err(|err| {
                    anyhow::anyhow!(
                        "text content could not be unescaped ({err}). Entities other than the \
                         five XML predefines are not expanded here: a RIF document written \
                         against a DOCTYPE that declares &rif; or &xs; must be serialised with \
                         those expanded"
                    )
                })?;
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(text.as_ref());
                }
            }
            Event::CData(e) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&String::from_utf8_lossy(e.as_ref()));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    if !stack.is_empty() {
        anyhow::bail!("the document has {} unclosed element(s)", stack.len());
    }
    root.ok_or_else(|| anyhow::anyhow!("the document holds no elements"))
}

fn start_element(
    ns: &str,
    e: &quick_xml::events::BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
) -> anyhow::Result<El> {
    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
    let mut attrs = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(|err| anyhow::anyhow!("attribute of <{name}> is malformed: {err}"))?;
        let key = String::from_utf8_lossy(a.key.local_name().as_ref()).into_owned();
        let value = a.decode_and_unescape_value(decoder).map_err(|err| {
            anyhow::anyhow!(
                "the {key} attribute of <{name}> could not be unescaped ({err}). Entities other \
                 than the five XML predefines are not expanded here, so a type=\"&rif;iri\" must \
                 be serialised as the full IRI"
            )
        })?;
        attrs.push((key, value.into_owned()));
    }
    Ok(El { name, ns: ns.to_string(), attrs, text: String::new(), children: Vec::new() })
}

// ── The one entry point the CLI and the MCP tool share ──────────────────────

/// Which front end to run.
pub fn run_import(
    graph: &Arc<crate::graph::GraphStore>,
    from: &str,
    file: Option<&std::path::Path>,
    out: Option<&std::path::Path>,
    allow_partial: bool,
) -> anyhow::Result<String> {
    let imp = match from {
        "swrl" => match file {
            // A rules file that is not the data: loaded into a store of its
            // own so importing rules never changes what is loaded.
            Some(p) => {
                let scratch = Arc::new(crate::graph::GraphStore::new());
                scratch.load_file(&p.display().to_string()).map_err(|e| {
                    anyhow::anyhow!("cannot read {} as RDF: {e}", p.display())
                })?;
                swrl_from_graph(&scratch)?
            }
            None => swrl_from_graph(graph)?,
        },
        "rif" | "rif-core" => {
            let Some(p) = file else {
                anyhow::bail!(
                    "rif needs a file: RIF Core is an XML document, not something held in the RDF \
                     store"
                );
            };
            let xml = std::fs::read_to_string(p)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", p.display()))?;
            rif_core_from_xml(&xml)?
        }
        other => anyhow::bail!(
            "unknown rule syntax '{other}'. This reads 'swrl' (out of an RDF graph) and 'rif' \
             (RIF Core, XML syntax). Datalog has no single standard concrete syntax and is not \
             offered under that name: a Datalog program IS a rules.tsv table, which \
             `reason --rules` already takes"
        ),
    };

    let mut report = imp.report();

    // The loud half. A refusal fails the import unless the caller has said, in
    // as many words, that a weaker rule set is what they want. Silence here is
    // the whole failure mode: a table that lost a rule, evaluated to a
    // fixpoint, and produced a certificate that checks green is a sound proof
    // about a rule set nobody wrote.
    if !imp.refused.is_empty() && !allow_partial {
        report["error"] = serde_json::json!(format!(
            "{} of {} rule(s) in this {} document cannot be represented as Horn rules over triple \
             patterns, so no table was written. Each one is listed under `refused` with the \
             construct that stopped it. Pass allow_partial to import the rest ANYWAY, which \
             produces a table that is a WEAKER rule set than the document states and is flagged \
             as such",
            imp.refused.len(),
            imp.source_rules,
            imp.syntax
        ));
        report["rules_written"] = serde_json::json!(false);
        return Ok(report.to_string());
    }

    if imp.rules.is_empty() {
        report["error"] = serde_json::json!(format!(
            "no rules were imported from this {} document. An empty table derives nothing, so \
             there is no rule set to write and nothing for `reason --rules` to evaluate",
            imp.syntax
        ));
        report["rules_written"] = serde_json::json!(false);
        return Ok(report.to_string());
    }

    let tsv = rules_tsv(&imp.rules);
    // The same round trip `run_horn` does before it writes a certificate: what
    // the checker will read must be what this importer built. Each rule already
    // survived it alone; this catches anything that only goes wrong in company.
    match parse_rules(&tsv) {
        Ok(back) if back == imp.rules => {}
        Ok(_) => anyhow::bail!(
            "internal: the rule table this importer writes does not read back as the table it \
             built, so `reason --rules` would evaluate different rules. Refusing to write it"
        ),
        Err(e) => anyhow::bail!(
            "internal: the rule table this importer writes does not parse ({e}). Refusing to \
             write it"
        ),
    }

    let digest = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(tsv.as_bytes()))
    };
    report["rules_tsv_sha256"] = serde_json::json!(digest);

    match out {
        Some(path) => {
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, &tsv)?;
            report["rules_written"] = serde_json::json!(true);
            report["rules_file"] = serde_json::json!(path.display().to_string());
            report["next"] = serde_json::json!(format!(
                "open-ontologies reason --rules {p} --certificate DIR, then cd lean && lake exe \
                 oo-horn check DIR/rules.tsv DIR/asserted.tsv DIR/horn.tsv",
                p = path.display()
            ));
        }
        None => {
            report["rules_written"] = serde_json::json!(false);
            report["rules_tsv"] = serde_json::json!(tsv);
            report["next"] = serde_json::json!(
                "write rules_tsv to a file, then `reason --rules FILE --certificate DIR` and \
                 `lake exe oo-horn check` on what that writes"
            );
        }
    }
    Ok(report.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triples(ttl: &str) -> Vec<(String, String, String)> {
        let g = Arc::new(crate::graph::GraphStore::new());
        g.load_turtle(ttl, None).unwrap();
        g.all_triples().unwrap()
    }

    const PREFIXES: &str = r#"
        @prefix : <http://ex.org/> .
        @prefix swrl: <http://www.w3.org/2003/11/swrl#> .
        @prefix var: <urn:swrl#> .
    "#;

    #[test]
    fn a_class_atom_becomes_an_rdf_type_pattern() {
        let imp = swrl_from_triples(&triples(&format!(
            r#"{PREFIXES}
            var:x a swrl:Variable .
            [] a swrl:Imp ;
               swrl:body ( [ a swrl:ClassAtom ; swrl:classPredicate :Supplier ; swrl:argument1 var:x ] ) ;
               swrl:head ( [ a swrl:ClassAtom ; swrl:classPredicate :Compliant ; swrl:argument1 var:x ] ) ."#
        )))
        .unwrap();
        assert!(imp.refused.is_empty(), "{:?}", imp.refused);
        assert_eq!(imp.rules.len(), 1);
        let r = &imp.rules[0];
        assert_eq!(r.body.len(), 1);
        assert_eq!(r.body[0].p, Pat::Const(RDF_TYPE.to_string()));
        assert_eq!(r.body[0].s, Pat::Var("x".into()));
        assert_eq!(r.head.o, Pat::Const("<http://ex.org/Compliant>".into()));
    }

    #[test]
    fn a_built_in_atom_is_refused_by_name() {
        let imp = swrl_from_triples(&triples(&format!(
            r#"{PREFIXES}
            @prefix swrlb: <http://www.w3.org/2003/11/swrlb#> .
            var:x a swrl:Variable . var:n a swrl:Variable .
            [] a swrl:Imp ;
               swrl:body ( [ a swrl:BuiltinAtom ; swrl:builtin swrlb:greaterThan ] ) ;
               swrl:head ( [ a swrl:ClassAtom ; swrl:classPredicate :Big ; swrl:argument1 var:x ] ) ."#
        )))
        .unwrap();
        assert_eq!(imp.rules.len(), 0);
        assert_eq!(imp.refused.len(), 1);
        assert!(imp.refused[0].construct.contains("swrlb#greaterThan"), "{:?}", imp.refused[0]);
    }

    #[test]
    fn a_conjunctive_head_becomes_several_rules_and_says_so() {
        let imp = swrl_from_triples(&triples(&format!(
            r#"{PREFIXES}
            var:x a swrl:Variable .
            [] a swrl:Imp ;
               swrl:body ( [ a swrl:ClassAtom ; swrl:classPredicate :A ; swrl:argument1 var:x ] ) ;
               swrl:head ( [ a swrl:ClassAtom ; swrl:classPredicate :B ; swrl:argument1 var:x ]
                           [ a swrl:ClassAtom ; swrl:classPredicate :C ; swrl:argument1 var:x ] ) ."#
        )))
        .unwrap();
        assert!(imp.refused.is_empty(), "{:?}", imp.refused);
        assert_eq!(imp.rules.len(), 2);
        assert_eq!(imp.head_splits, 1);
        assert_eq!(imp.source_rules, 1);
    }

    #[test]
    fn colliding_local_names_fall_back_to_full_iris_rather_than_join_two_variables() {
        // `urn:a#x` and `urn:b#x` are different variables with the same local
        // name. Calling both `?x` would make them ONE variable and silently
        // strengthen the rule into one that joins two positions the author kept
        // apart.
        let t = triples(&format!(
            r#"{PREFIXES}
            @prefix a: <urn:a#> . @prefix b: <urn:b#> .
            a:x a swrl:Variable . b:x a swrl:Variable .
            [] a swrl:Imp ;
               swrl:body ( [ a swrl:IndividualPropertyAtom ; swrl:propertyPredicate :p ;
                             swrl:argument1 a:x ; swrl:argument2 b:x ] ) ;
               swrl:head ( [ a swrl:IndividualPropertyAtom ; swrl:propertyPredicate :q ;
                             swrl:argument1 a:x ; swrl:argument2 b:x ] ) ."#
        ));
        let imp = swrl_from_triples(&t).unwrap();
        assert!(imp.refused.is_empty(), "{:?}", imp.refused);
        let r = &imp.rules[0];
        assert_eq!(r.body[0].s, Pat::Var("urn:a#x".into()));
        assert_eq!(r.body[0].o, Pat::Var("urn:b#x".into()));
    }

    #[test]
    fn a_literal_holding_a_tab_is_escaped_rather_than_splitting_the_line() {
        // A raw tab inside a rule would add fields to the `rules.tsv` line and
        // a raw newline would make it two rules.
        assert_eq!(escape_literal("a\tb\nc\"d\\e"), "a\\tb\\nc\\\"d\\\\e");
    }

    #[test]
    fn the_presentation_syntax_is_refused_rather_than_guessed_at() {
        let e = rif_core_from_xml("Document( Group( Forall ?x ( p(?x) :- q(?x) ) ) )").unwrap_err();
        assert!(e.to_string().contains("presentation syntax"), "{e}");
    }

    #[test]
    fn a_rif_frame_is_a_triple_and_equality_in_the_head_is_refused() {
        let xml = r#"<Document xmlns="http://www.w3.org/2007/rif#"><payload><Group>
          <sentence><Forall><declare><Var>x</Var></declare><formula><Implies>
            <if><Frame><object><Var>x</Var></object>
              <slot><Const type="http://www.w3.org/2007/rif#iri">http://ex.org/p</Const>
                    <Const type="http://www.w3.org/2007/rif#iri">http://ex.org/o</Const></slot>
            </Frame></if>
            <then><Frame><object><Var>x</Var></object>
              <slot><Const type="http://www.w3.org/2007/rif#iri">http://ex.org/q</Const>
                    <Const type="http://www.w3.org/2007/rif#iri">http://ex.org/o</Const></slot>
            </Frame></then>
          </Implies></formula></Forall></sentence>
          <sentence><Forall><declare><Var>y</Var></declare><formula><Implies>
            <if><Member><instance><Var>y</Var></instance>
                <class><Const type="http://www.w3.org/2007/rif#iri">http://ex.org/C</Const></class></Member></if>
            <then><Equal><left><Var>y</Var></left>
                  <right><Const type="http://www.w3.org/2007/rif#iri">http://ex.org/a</Const></right></Equal></then>
          </Implies></formula></Forall></sentence>
        </Group></payload></Document>"#;
        let imp = rif_core_from_xml(xml).unwrap();
        assert_eq!(imp.source_rules, 2);
        assert_eq!(imp.rules.len(), 1);
        assert_eq!(imp.rules[0].body[0].p, Pat::Const("<http://ex.org/p>".into()));
        assert_eq!(imp.refused.len(), 1);
        assert_eq!(imp.refused[0].construct, "rif:Equal in the conclusion");
    }
}
