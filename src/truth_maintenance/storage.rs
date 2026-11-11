// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Per-record persistence and dependency indexes in the data transaction.
//! Ordinary writes read the affected dependency component, never the dataset.

use super::*;
use oxigraph::store::Transaction;
use serde::de::DeserializeOwned;

const NS: &str = "https://open-ontologies.org/provenance#";

#[derive(Clone, Default, Deserialize, Serialize)]
pub(crate) struct Header {
    pub revision: u64,
    pub fixpoint_reached: bool,
    pub last_profile: Option<String>,
    pub records: usize,
}

fn node(key: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{NS}{key}"))
}

fn quad(key: &str, predicate: &str, object: impl Into<Term>) -> Quad {
    Quad::new(
        node(key),
        node(predicate),
        object,
        NamedNode::new_unchecked(LEDGER_GRAPH),
    )
}

fn json<T: Serialize>(key: &str, kind: &str, value: &T) -> Result<Quad> {
    Ok(quad(
        key,
        kind,
        Literal::new_simple_literal(serde_json::to_string(value)?),
    ))
}

fn value<T: DeserializeOwned>(q: &Quad) -> Result<T> {
    let Term::Literal(l) = &q.object else {
        anyhow::bail!("invalid ledger record")
    };
    Ok(serde_json::from_str(l.value())?)
}

fn read<T: DeserializeOwned>(txn: &Transaction<'_>, key: &str, kind: &str) -> Result<Option<T>> {
    txn.quads_for_pattern(
        Some(node(key).as_ref().into()),
        Some(node(kind).as_ref()),
        None,
        Some(NamedNode::new_unchecked(LEDGER_GRAPH).as_ref().into()),
    )
    .next()
    .transpose()?
    .map(|q| value(&q))
    .transpose()
}

pub(crate) fn header(txn: &Transaction<'_>) -> Result<Option<Header>> {
    read(txn, "state", "header")
}

pub(crate) fn decode(quads: impl IntoIterator<Item = Quad>) -> Result<Option<Ledger>> {
    let mut ledger = Ledger::default();
    let mut found = false;
    for q in quads {
        match q.predicate.as_str().strip_prefix(NS) {
            Some("header") => {
                let h: Header = value(&q)?;
                ledger.revision = h.revision;
                ledger.fixpoint_reached = h.fixpoint_reached;
                ledger.last_profile = h.last_profile;
                found = true;
            }
            Some("evidence") => {
                let e: Evidence = value(&q)?;
                ledger.evidence.insert(e.id.clone(), e);
            }
            Some("derivation") => {
                let d: RecordedDerivation = value(&q)?;
                ledger.derivations.insert(d.id.clone(), d);
            }
            Some("materialization") => {
                ledger.materializations.insert(value(&q)?);
            }
            Some("change") => ledger.history.push(value(&q)?),
            Some("ledger") => return Ok(Some(Ledger::decode(&q)?)),
            Some(
                "evidenceIndex"
                | "premiseIndex"
                | "conclusionIndex"
                | "materializationIndex"
                | "ruleIndex",
            ) => {}
            _ => anyhow::bail!("unknown derivation ledger record"),
        }
    }
    ledger.history.sort_by_key(|c| c.revision);
    Ok(found.then_some(ledger))
}

pub(crate) fn hidden_count(quads: impl IntoIterator<Item = Quad>) -> Result<usize> {
    for q in quads {
        if q.predicate == node("header") {
            return Ok(value::<Header>(&q)?.records);
        }
        if q.predicate.as_str() == LEDGER_PREDICATE {
            return Ok(1);
        }
    }
    Ok(0)
}

fn replace(txn: &mut Transaction<'_>, h: &mut Header, q: Quad) -> Result<()> {
    let old = txn
        .quads_for_pattern(
            Some(q.subject.as_ref()),
            Some(q.predicate.as_ref()),
            None,
            Some(q.graph_name.as_ref()),
        )
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if old.len() == 1 && old[0] == q {
        return Ok(());
    }
    for p in old {
        txn.remove(&p);
        h.records -= 1;
    }
    if !txn.contains(&q)? {
        txn.insert(&q);
        h.records += 1;
    }
    Ok(())
}

fn index(
    txn: &mut Transaction<'_>,
    h: &mut Header,
    fact: &Spelled,
    kind: &str,
    key: &str,
) -> Result<()> {
    let q = quad(&format!("f:{}", hash(fact)), kind, node(key));
    if !txn.contains(&q)? {
        txn.insert(&q);
        h.records += 1;
    }
    Ok(())
}

pub(crate) fn finish(txn: &mut Transaction<'_>, h: &mut Header, change: &Change) -> Result<()> {
    replace(
        txn,
        h,
        json(&format!("h:{}", change.revision), "change", change)?,
    )?;
    if header(txn)?.is_none() {
        h.records += 1;
    }
    let q = json("state", "header", h)?;
    let previous = txn
        .quads_for_pattern(
            Some(node("state").as_ref().into()),
            Some(node("header").as_ref()),
            None,
            Some(q.graph_name.as_ref()),
        )
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for q in previous {
        txn.remove(&q);
    }
    txn.insert(&q);
    Ok(())
}

pub(crate) fn publish(txn: &mut Transaction<'_>, before: &Ledger, after: &Ledger) -> Result<()> {
    let mut h = header(txn)?.unwrap_or_default();
    for (id, e) in &after.evidence {
        if before.evidence.get(id) == Some(e) {
            continue;
        }
        let key = format!("e:{id}");
        replace(txn, &mut h, json(&key, "evidence", e)?)?;
        index(txn, &mut h, &e.source.triple, "evidenceIndex", &key)?;
    }
    for (id, d) in &after.derivations {
        if before.derivations.get(id) == Some(d) {
            continue;
        }
        let key = format!("d:{id}");
        replace(txn, &mut h, json(&key, "derivation", d)?)?;
        index(txn, &mut h, &d.conclusion, "conclusionIndex", &key)?;
        index(
            txn,
            &mut h,
            &(d.rule.clone(), d.rule_pack_sha256.clone(), "rule".into()),
            "ruleIndex",
            &key,
        )?;
        for p in &d.premises {
            index(txn, &mut h, p, "premiseIndex", &key)?;
        }
    }
    for m in after.materializations.difference(&before.materializations) {
        let key = format!("m:{}", m.id());
        replace(txn, &mut h, json(&key, "materialization", m)?)?;
        index(txn, &mut h, &m.triple, "materializationIndex", &key)?;
    }
    h.revision = after.revision;
    h.fixpoint_reached = after.fixpoint_reached;
    h.last_profile.clone_from(&after.last_profile);
    for c in after
        .history
        .iter()
        .filter(|c| c.revision > before.revision)
    {
        finish(txn, &mut h, c)?;
    }
    Ok(())
}

fn indexed<T: DeserializeOwned>(
    txn: &Transaction<'_>,
    fact: &Spelled,
    index: &str,
    kind: &str,
) -> Result<Vec<T>> {
    let subject = node(&format!("f:{}", hash(fact)));
    let mut records = Vec::new();
    for q in txn.quads_for_pattern(
        Some(subject.as_ref().into()),
        Some(node(index).as_ref()),
        None,
        Some(NamedNode::new_unchecked(LEDGER_GRAPH).as_ref().into()),
    ) {
        let q = q?;
        let Term::NamedNode(key) = q.object else {
            anyhow::bail!("invalid ledger index")
        };
        let record = read(
            txn,
            key.as_str()
                .strip_prefix(NS)
                .context("invalid ledger key")?,
            kind,
        )?
        .context("missing indexed ledger record")?;
        records.push(record);
    }
    Ok(records)
}

/// Read descendants of changed facts, and ancestors needed to decide their
/// alternative proofs. Recursive components are grounded from evidence.
pub(crate) fn component(txn: &Transaction<'_>, seeds: &[Spelled]) -> Result<Option<Ledger>> {
    let Some(h) = header(txn)? else {
        return Ok(None);
    };
    let mut ledger = Ledger {
        revision: h.revision,
        fixpoint_reached: h.fixpoint_reached,
        last_profile: h.last_profile,
        ..Ledger::default()
    };
    let mut downstream = BTreeSet::new();
    let mut queue: VecDeque<_> = seeds.iter().cloned().collect();
    while let Some(f) = queue.pop_front() {
        if !downstream.insert(f.clone()) {
            continue;
        }
        for d in indexed::<RecordedDerivation>(txn, &f, "premiseIndex", "derivation")? {
            if d.enabled {
                queue.push_back(d.conclusion);
            }
        }
    }
    queue.extend(downstream.iter().cloned());
    let mut visited = BTreeSet::new();
    while let Some(f) = queue.pop_front() {
        if !visited.insert(f.clone()) {
            continue;
        }
        for e in indexed::<Evidence>(txn, &f, "evidenceIndex", "evidence")? {
            ledger.evidence.insert(e.id.clone(), e);
        }
        for d in indexed::<RecordedDerivation>(txn, &f, "conclusionIndex", "derivation")? {
            if d.enabled {
                queue.extend(d.premises.iter().cloned());
            }
            ledger.derivations.insert(d.id.clone(), d);
        }
        if downstream.contains(&f) {
            ledger.materializations.extend(indexed::<SourceQuad>(
                txn,
                &f,
                "materializationIndex",
                "materialization",
            )?);
        }
    }
    Ok(Some(ledger))
}

pub(crate) fn apply_delta(
    txn: &mut Transaction<'_>,
    inserted: &[Quad],
    deleted: &[Quad],
    cascade: bool,
    operation: &str,
) -> Result<(Option<Change>, isize)> {
    // An untracked load must not even allocate a copy of the source identities.
    if header(txn)?.is_none() {
        return Ok((None, 0));
    }
    let seeds: Vec<_> = inserted
        .iter()
        .chain(deleted)
        .map(|q| SourceQuad::from_quad(q).triple)
        .collect();
    let Some(before) = component(txn, &seeds)? else {
        return Ok((None, 0));
    };
    let mut after = before.clone();
    let known = before.grounded();
    let mut withdrawn = Vec::new();
    for q in deleted {
        let source = SourceQuad::from_quad(q);
        if let Some(e) = after.evidence.get_mut(&source.id())
            && e.active
        {
            e.active = false;
            withdrawn.push(e.id.clone());
        }
    }
    for q in inserted {
        ensure!(!is_internal(q), "the derivation ledger is reserved");
        let source = SourceQuad::from_quad(q);
        let id = source.id();
        after.evidence.insert(
            id.clone(),
            Evidence {
                id,
                source,
                active: true,
            },
        );
    }
    after.fixpoint_reached = false;
    let event = after.finish_change(operation, withdrawn, &known);
    let delta = if cascade { project(txn, &after)? } else { 0 };
    publish(txn, &before, &after)?;
    Ok((Some(event), delta))
}

pub(crate) fn project(txn: &mut Transaction<'_>, ledger: &Ledger) -> Result<isize> {
    let mut delta = 0;
    let (insert, remove) = ledger.projection()?;
    for q in remove {
        if txn.contains(&q)? {
            txn.remove(&q);
            delta -= 1;
        }
    }
    for q in insert {
        if !txn.contains(&q)? {
            txn.insert(&q);
            delta += 1;
        }
    }
    Ok(delta)
}

pub(crate) fn invalidate(txn: &mut Transaction<'_>, rule: &str, pack: &str) -> Result<Change> {
    ensure!(header(txn)?.is_some(), "no durable derivations recorded");
    let records = indexed::<RecordedDerivation>(
        txn,
        &(rule.into(), pack.into(), "rule".into()),
        "ruleIndex",
        "derivation",
    )?;
    let seeds: Vec<_> = records.iter().map(|d| d.conclusion.clone()).collect();
    let before = component(txn, &seeds)?.context("no durable derivations recorded")?;
    let mut after = before.clone();
    for record in records {
        if let Some(d) = after.derivations.get_mut(&record.id) {
            d.enabled = false;
        }
    }
    after.fixpoint_reached = false;
    let event = after.finish_change("invalidate_rule", Vec::new(), &before.grounded());
    project(txn, &after)?;
    publish(txn, &before, &after)?;
    Ok(event)
}

pub(crate) fn mark_incomplete(txn: &mut Transaction<'_>) -> Result<()> {
    if let Some(mut h) = header(txn)? {
        h.revision += 1;
        h.fixpoint_reached = false;
        let event = Change {
            revision: h.revision,
            operation: "untracked_materialization".into(),
            withdrawn_evidence: Vec::new(),
            withdrawn_facts: Vec::new(),
        };
        finish(txn, &mut h, &event)?;
    }
    Ok(())
}
