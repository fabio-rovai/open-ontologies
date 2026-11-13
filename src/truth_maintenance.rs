// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Durable support for the existing materialisation path. The ledger and its
//! RDF consequences share ONE Oxigraph transaction, including on withdrawal.
//! Cycles are allowed in recorded rule instances, but only proofs grounded in
//! surviving assertions keep a conclusion alive. Reference counting alone
//! would incorrectly allow a cycle to support itself after its source is gone.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::Path;

use anyhow::{Context, Result, ensure};
use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::model::{GraphName, Literal, NamedNode, Quad, Term};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::reason::{DerivationGraph, Spelled};

pub(crate) mod storage;
pub(crate) mod update;

/// Reserved graph holding the durable ledger. It is excluded from ontology
/// scans and triple-format exports, and retained in dataset backups.
pub const LEDGER_GRAPH: &str = "https://open-ontologies.org/graph/derivations";
const LEDGER_PREDICATE: &str = "https://open-ontologies.org/provenance#ledger";

/// Graph identity is part of evidence identity. Equal triples in two source
/// graphs are two independent assertions and can be withdrawn separately.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceQuad {
    pub triple: Spelled,
    /// N-Quads spelling, or the empty string for the default graph.
    pub graph: String,
}

impl SourceQuad {
    pub fn from_quad(q: &Quad) -> Self {
        Self {
            triple: (
                q.subject.to_string(),
                q.predicate.to_string(),
                q.object.to_string(),
            ),
            graph: if q.graph_name.is_default_graph() {
                String::new()
            } else {
                q.graph_name.to_string()
            },
        }
    }

    pub fn to_quad(&self) -> Result<Quad> {
        let (s, p, o) = &self.triple;
        let line = format!("{s} {p} {o} {} .", self.graph);
        let quads: Vec<_> = RdfParser::from_format(RdfFormat::NQuads)
            .for_reader(line.as_bytes())
            .collect::<std::result::Result<_, _>>()?;
        ensure!(
            quads.len() == 1,
            "a provenance fact must contain exactly one quad"
        );
        quads.into_iter().next().context("empty provenance fact")
    }

    pub fn id(&self) -> String {
        hash(self)
    }
}

/// Evidence refers to an assertion in a graph, not an invented attribution to
/// a reasoner run. Source authenticity is outside the derivation proof.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Evidence {
    pub id: String,
    pub source: SourceQuad,
    pub active: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RecordedDerivation {
    pub id: String,
    pub profile: String,
    pub rule: String,
    pub rule_pack_sha256: String,
    pub conclusion: Spelled,
    /// Premise order is the existing Lean certificate rule's order.
    pub premises: Vec<Spelled>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Change {
    pub revision: u64,
    pub operation: String,
    pub withdrawn_evidence: Vec<String>,
    pub withdrawn_facts: Vec<Spelled>,
}

/// Read/export view of the per-record RDF ledger in the data transaction.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Ledger {
    pub format: String,
    pub revision: u64,
    pub evidence: BTreeMap<String, Evidence>,
    pub derivations: BTreeMap<String, RecordedDerivation>,
    pub materializations: BTreeSet<SourceQuad>,
    pub history: Vec<Change>,
    /// Completeness is relative to the latest captured profile, not full OWL.
    pub fixpoint_reached: bool,
    #[serde(default)]
    pub last_profile: Option<String>,
}

impl Default for Ledger {
    fn default() -> Self {
        Self {
            format: "oo-derivation-ledger/1".into(),
            revision: 0,
            evidence: BTreeMap::new(),
            derivations: BTreeMap::new(),
            materializations: BTreeSet::new(),
            history: Vec::new(),
            fixpoint_reached: false,
            last_profile: None,
        }
    }
}

impl Ledger {
    pub fn decode(q: &Quad) -> Result<Self> {
        ensure!(
            is_internal(q)
                && q.subject.to_string() == format!("<{LEDGER_GRAPH}>")
                && q.predicate.as_str() == LEDGER_PREDICATE,
            "invalid derivation ledger record"
        );
        let Term::Literal(literal) = &q.object else {
            anyhow::bail!("derivation ledger is not a literal");
        };
        let ledger: Self = serde_json::from_str(literal.value())?;
        ensure!(
            ledger.format == "oo-derivation-ledger/1",
            "unsupported derivation ledger format"
        );
        Ok(ledger)
    }

    pub fn as_quad(&self) -> Result<Quad> {
        Ok(Quad::new(
            NamedNode::new(LEDGER_GRAPH)?,
            NamedNode::new(LEDGER_PREDICATE)?,
            Literal::new_simple_literal(serde_json::to_string(self)?),
            NamedNode::new(LEDGER_GRAPH)?,
        ))
    }

    /// Whether this physical quad is an inference with no independent source
    /// assertion at the same location. Used by the existing certificate reader.
    pub fn derived_only(&self, q: &Quad) -> bool {
        let source = SourceQuad::from_quad(q);
        self.materializations.contains(&source)
            && !self.evidence.get(&source.id()).is_some_and(|e| e.active)
    }

    /// Reconcile evidence after a graph write, before publishing that write.
    /// Explicit loads may assert an already-materialised triple; that assertion
    /// is recorded independently and survives later rule invalidation.
    pub fn sync_sources(&mut self, quads: &[Quad], explicit: &[Quad]) -> Vec<String> {
        let previous: BTreeSet<_> = self
            .evidence
            .values()
            .filter(|e| e.active)
            .map(|e| e.source.clone())
            .collect();
        let current: BTreeSet<_> = quads
            .iter()
            .filter(|q| !is_internal(q))
            .map(SourceQuad::from_quad)
            .collect();
        let mut withdrawn = Vec::new();
        for evidence in self.evidence.values_mut() {
            if evidence.active && !current.contains(&evidence.source) {
                evidence.active = false;
                withdrawn.push(evidence.id.clone());
            }
        }
        let explicit: BTreeSet<_> = explicit
            .iter()
            .filter(|q| !is_internal(q))
            .map(SourceQuad::from_quad)
            .collect();
        for source in current {
            // Legacy inference graphs have no recoverable source attribution.
            // Never upgrade their untracked contents to asserted evidence.
            if source.graph == format!("<{}>", crate::reason::INFERRED_GRAPH)
                && !explicit.contains(&source)
            {
                continue;
            }
            if !self.materializations.contains(&source) || explicit.contains(&source) {
                let id = source.id();
                self.evidence.insert(
                    id.clone(),
                    Evidence {
                        id,
                        source,
                        active: true,
                    },
                );
            }
        }
        let active: BTreeSet<_> = self
            .evidence
            .values()
            .filter(|e| e.active)
            .map(|e| e.source.clone())
            .collect();
        if previous != active {
            self.fixpoint_reached = false;
        }
        withdrawn
    }

    pub fn record(&mut self, dag: &DerivationGraph, target: &str) {
        let pack = builtin_rule_pack_hash();
        if dag.fixpoint_reached {
            // A complete run replaces this profile's rule instances, including
            // alternative derivations that no longer apply. A partial run may
            // only add sound steps; it cannot retire unseen alternatives.
            for d in self
                .derivations
                .values_mut()
                .filter(|d| d.profile == dag.profile_used)
            {
                d.enabled = false;
            }
        }
        for instance in &dag.instances {
            let id = hash(&(
                &dag.profile_used,
                &pack,
                instance.rule,
                &instance.conclusion,
                &instance.premises,
            ));
            self.derivations.insert(
                id.clone(),
                RecordedDerivation {
                    id,
                    profile: dag.profile_used.clone(),
                    rule: instance.rule.into(),
                    rule_pack_sha256: pack.clone(),
                    conclusion: instance.conclusion.clone(),
                    premises: instance.premises.clone(),
                    enabled: true,
                },
            );
        }
        let asserted: HashSet<_> = dag.asserted.iter().cloned().collect();
        let conclusions: HashSet<_> = dag.instances.iter().map(|d| &d.conclusion).collect();
        for conclusion in &dag.closure {
            let source = SourceQuad {
                triple: conclusion.clone(),
                graph: target.into(),
            };
            // An assertion in ANY source graph can also have a proof. Retain
            // its output location even when no physical copy is needed yet,
            // so withdrawing the assertion can reveal the surviving inference.
            if !asserted.contains(conclusion) || conclusions.contains(conclusion) {
                self.materializations.insert(source);
            }
        }
        self.fixpoint_reached = dag.fixpoint_reached;
        self.last_profile = Some(dag.profile_used.clone());
    }

    /// Least fixed point of the recorded hyperedges. Seed with evidence, never
    /// with existing inferred triples; this handles ungrounded recursive SCCs.
    pub fn grounded(&self) -> BTreeSet<Spelled> {
        let mut known: BTreeSet<_> = self
            .evidence
            .values()
            .filter(|e| e.active)
            .map(|e| e.source.triple.clone())
            .collect();
        let mut queue: VecDeque<_> = known.iter().cloned().collect();
        let rules: Vec<_> = self.derivations.values().filter(|d| d.enabled).collect();
        let mut remaining = Vec::with_capacity(rules.len());
        let mut dependents: HashMap<Spelled, Vec<usize>> = HashMap::new();
        for (i, rule) in rules.iter().enumerate() {
            let premises: HashSet<_> = rule.premises.iter().cloned().collect();
            remaining.push(premises.len());
            for premise in premises {
                dependents.entry(premise).or_default().push(i);
            }
            if rule.premises.is_empty() && known.insert(rule.conclusion.clone()) {
                queue.push_back(rule.conclusion.clone());
            }
        }
        while let Some(fact) = queue.pop_front() {
            for &index in dependents.get(&fact).into_iter().flatten() {
                remaining[index] -= 1;
                if remaining[index] == 0 && known.insert(rules[index].conclusion.clone()) {
                    queue.push_back(rules[index].conclusion.clone());
                }
            }
        }
        known
    }

    /// Physical changes needed to align the RDF view with active support.
    pub fn projection(&self) -> Result<(Vec<Quad>, Vec<Quad>)> {
        let known = self.grounded();
        let asserted: HashSet<_> = self
            .evidence
            .values()
            .filter(|e| e.active)
            .map(|e| &e.source.triple)
            .collect();
        let mut insert = Vec::new();
        let mut remove = Vec::new();
        for source in &self.materializations {
            let asserted_here = self.evidence.get(&source.id()).is_some_and(|e| e.active);
            if known.contains(&source.triple) {
                // Do not create a redundant inference copy while an assertion
                // elsewhere already carries this fact. Withdrawal reveals the
                // recorded output location if an independent proof survives.
                if asserted_here || !asserted.contains(&source.triple) {
                    insert.push(source.to_quad()?);
                }
            } else if !asserted_here {
                remove.push(source.to_quad()?);
            }
        }
        Ok((insert, remove))
    }

    pub fn finish_change(
        &mut self,
        operation: &str,
        withdrawn_evidence: Vec<String>,
        before: &BTreeSet<Spelled>,
    ) -> Change {
        self.revision += 1;
        let after = self.grounded();
        let event = Change {
            revision: self.revision,
            operation: operation.into(),
            withdrawn_evidence,
            withdrawn_facts: before.difference(&after).cloned().collect(),
        };
        self.history.push(event.clone());
        event
    }

    /// Persisted explanation for a fact, including withdrawn sources. This
    /// complements the existing algebraic explanation without re-running it.
    pub fn explain(&self, target: Option<&Spelled>) -> serde_json::Value {
        let known = self.grounded();
        let mut wanted = BTreeSet::new();
        let mut queue = VecDeque::new();
        if let Some(t) = target {
            queue.push_back(t.clone());
        }
        while let Some(t) = queue.pop_front() {
            if !wanted.insert(t.clone()) {
                continue;
            }
            for d in self.derivations.values().filter(|d| d.conclusion == t) {
                queue.extend(d.premises.iter().cloned());
            }
        }
        let evidence: Vec<_> = self
            .evidence
            .values()
            .filter(|e| target.is_none() || wanted.contains(&e.source.triple))
            .collect();
        let derivations: Vec<_> = self
            .derivations
            .values()
            .filter(|d| target.is_none() || wanted.contains(&d.conclusion))
            .collect();
        let state = target.map(|t| {
            if self
                .evidence
                .values()
                .any(|e| e.active && &e.source.triple == t)
            {
                "asserted"
            } else if known.contains(t) {
                "derived"
            } else if self
                .derivations
                .values()
                .any(|d| !d.enabled && &d.conclusion == t)
            {
                "stale"
            } else {
                "retracted_or_untracked"
            }
        });
        serde_json::json!({
            "revision": self.revision, "state": state, "evidence": evidence,
            "derivations": derivations, "history": self.history,
            "grounded_fact_count": known.len(),
            "fixpoint_reached": self.fixpoint_reached,
            "completeness_profile": self.last_profile,
            "semantics": "Durable support under the recorded rule versions. Source truth is not certified."
        })
    }

    /// Export a proof forest in the EXISTING oo-cert format. Grounding order
    /// selects one valid proof per conclusion without pretending the complete
    /// dependency graph is acyclic. All alternatives remain in ledger.json.
    /// Semantic soundness is checked by oo-cert, not by hashes in this module.
    pub fn export_certificate(&self, directory: &Path) -> Result<serde_json::Value> {
        std::fs::create_dir_all(directory)?;
        let pack = builtin_rule_pack_hash();
        let mut known: BTreeSet<_> = self
            .evidence
            .values()
            .filter(|e| e.active)
            .map(|e| e.source.triple.clone())
            .collect();
        let asserted = crate::reason::asserted_bytes(&known.iter().cloned().collect::<Vec<_>>())?;
        let mut steps = Vec::new();
        loop {
            let mut progress = false;
            for rule in self.derivations.values().filter(|d| d.enabled) {
                if known.contains(&rule.conclusion)
                    || !rule.premises.iter().all(|p| known.contains(p))
                {
                    continue;
                }
                ensure!(
                    rule.rule_pack_sha256 == pack,
                    "rule pack differs from this binary; replay with the recorded engine version"
                );
                ensure!(
                    !rule.rule.contains(['\t', '\n', '\r']),
                    "invalid rule identifier"
                );
                steps.extend_from_slice(rule.rule.as_bytes());
                for (s, p, o) in std::iter::once(&rule.conclusion).chain(rule.premises.iter()) {
                    crate::boundary_core::push_triple_fields_bytes(
                        &mut steps,
                        s.as_bytes(),
                        p.as_bytes(),
                        o.as_bytes(),
                    )
                    .map_err(|position| {
                        anyhow::anyhow!("unwritable certificate term: {position:?}")
                    })?;
                }
                steps.push(b'\n');
                known.insert(rule.conclusion.clone());
                progress = true;
            }
            if !progress {
                break;
            }
        }
        let ledger = serde_json::to_vec_pretty(self)?;
        std::fs::write(directory.join("asserted.tsv"), &asserted)?;
        std::fs::write(directory.join("derivations.tsv"), &steps)?;
        std::fs::write(directory.join("ledger.json"), &ledger)?;
        let manifest = serde_json::json!({
            "format": "oo-durable-proof/1", "revision": self.revision,
            "asserted_sha256": bytes_hash(&asserted),
            "derivations_sha256": bytes_hash(&steps),
            "ledger_sha256": bytes_hash(&ledger),
            "rule_pack_sha256": builtin_rule_pack_hash(),
            "fixpoint_reached": self.fixpoint_reached,
            "completeness_profile": self.last_profile,
            "verdict": "not_checked",
            "check": "oo-cert asserted.tsv derivations.tsv",
            "limits": "Semantic soundness requires oo-cert. Hashes establish byte identity only. Source truth, completeness and absence of remaining support have no machine-checked theorem here."
        });
        std::fs::write(
            directory.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        Ok(manifest)
    }
}

pub fn is_internal(q: &Quad) -> bool {
    matches!(&q.graph_name, GraphName::NamedNode(n) if n.as_str() == LEDGER_GRAPH)
}

/// Pin to the rule implementation, including list rules and premise order.
/// A profile name alone cannot distinguish a rule upgrade.
pub fn builtin_rule_pack_hash() -> String {
    static HASH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        let mut hash = Sha256::new();
        // Git's Windows checkout conversion must not give the same rule
        // implementation a different identity after a cross-platform restore.
        hash.update(include_str!("reason.rs").replace("\r\n", "\n").as_bytes());
        hash.update(
            include_str!("reason_incremental.rs")
                .replace("\r\n", "\n")
                .as_bytes(),
        );
        format!("{:x}", hash.finalize())
    });
    HASH.clone()
}

pub fn bytes_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn hash(value: &impl Serialize) -> String {
    // These inputs contain only strings, tuples and fixed structs. Encoding is
    // unambiguous for escaped literals and independent of randomized hash maps.
    bytes_hash(
        &serde_json::to_vec(value).expect("provenance identifiers contain serializable values"),
    )
}
