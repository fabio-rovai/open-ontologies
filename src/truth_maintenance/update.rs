// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Stores without a ledger use Oxigraph's native update executor. Tracked
//! stores journal template instantiations to reconcile affected support.

use super::*;
use oxigraph::model::{BlankNode, NamedOrBlankNode};
use oxigraph::sparql::SparqlEvaluator;
use oxigraph::store::Transaction;
use spargebra::algebra::GraphTarget;
use spargebra::term::GraphNamePattern;
use spargebra::{GraphUpdateOperation as Op, SparqlParser, Update};

pub(crate) fn execute(txn: &mut Transaction<'_>, update: &str, cascade: bool) -> Result<isize> {
    if storage::header(txn)?.is_none() {
        // Preserve the original executor and its before/after delta count.
        // No provenance journal or template rewriting is needed without a ledger.
        let before = txn.len()?;
        SparqlEvaluator::new()
            .parse_update(update)?
            .on_transaction(txn)
            .execute()?;
        ensure!(
            !txn.contains_named_graph(&NamedNode::new_unchecked(LEDGER_GRAPH))?,
            "the derivation ledger is reserved; use dataset restore to replace it"
        );
        return Ok(txn.len()? as isize - before as isize);
    }
    // Parse all operations before changing either data or metadata.
    let parsed = SparqlParser::new().parse_update(update)?;
    let mut total = 0;
    for op in parsed.operations {
        let (inserted, deleted, delta) =
            operation(txn, op, parsed.base_iri.as_ref().map(|i| i.as_str()))?;
        let (_, projection_delta) =
            storage::apply_delta(txn, &inserted, &deleted, cascade, "sparql_update")?;
        total += delta + projection_delta;
    }
    Ok(total)
}

fn graph_name(name: spargebra::term::GraphName) -> GraphName {
    match name {
        spargebra::term::GraphName::NamedNode(n) => n.into(),
        spargebra::term::GraphName::DefaultGraph => GraphName::DefaultGraph,
    }
}

fn check(q: &Quad) -> Result<()> {
    ensure!(
        !is_internal(q),
        "the derivation ledger is reserved; use dataset restore to replace it"
    );
    Ok(())
}

fn operation(
    txn: &mut Transaction<'_>,
    op: Op,
    base: Option<&str>,
) -> Result<(Vec<Quad>, Vec<Quad>, isize)> {
    let mut delta = 0;
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();
    let drop_graph = matches!(&op, Op::Drop { .. });
    match op {
        Op::InsertData { data } => {
            let mut blanks = HashMap::new();
            let mut fresh =
                |b: BlankNode| blanks.entry(b).or_insert_with(BlankNode::default).clone();
            for q in data {
                let subject = match q.subject {
                    NamedOrBlankNode::BlankNode(b) => NamedOrBlankNode::BlankNode(fresh(b)),
                    s => s,
                };
                let object = match q.object {
                    Term::BlankNode(b) => Term::BlankNode(fresh(b)),
                    o => o,
                };
                let q = Quad::new(subject, q.predicate, object, graph_name(q.graph_name));
                check(&q)?;
                if !txn.contains(&q)? {
                    txn.insert(&q);
                    delta += 1;
                }
                inserted.push(q);
            }
        }
        Op::DeleteData { data } => {
            for q in data {
                let q = Quad::new(
                    q.subject,
                    q.predicate,
                    Term::from(q.object),
                    graph_name(q.graph_name),
                );
                check(&q)?;
                if txn.contains(&q)? {
                    txn.remove(&q);
                    delta -= 1;
                }
                deleted.push(q);
            }
        }
        Op::DeleteInsert {
            delete,
            insert,
            using,
            pattern,
        } => {
            let journal = NamedNode::new_unchecked(format!(
                "urn:oo:update:{}",
                BlankNode::default().as_str()
            ));
            let mut templates = String::new();
            let prefix = BlankNode::default().to_string();
            let mut add = |s: String,
                           p: String,
                           o: String,
                           g: &GraphNamePattern,
                           kind: &str,
                           i: usize| {
                let graph = if *g == GraphNamePattern::DefaultGraph {
                    "\"\"".into()
                } else {
                    g.to_string()
                };
                let default = *g == GraphNamePattern::DefaultGraph;
                templates.push_str(&format!("{prefix}{kind}{i} <urn:oo:s> {s}; <urn:oo:p> {p}; <urn:oo:o> {o}; <urn:oo:g> {graph}; <urn:oo:kind> \"{kind}\"; <urn:oo:default> {default} . "));
            };
            for (i, q) in delete.iter().enumerate() {
                add(
                    q.subject.to_string(),
                    q.predicate.to_string(),
                    q.object.to_string(),
                    &q.graph_name,
                    "d",
                    i,
                );
            }
            for (i, q) in insert.iter().enumerate() {
                add(
                    q.subject.to_string(),
                    q.predicate.to_string(),
                    q.object.to_string(),
                    &q.graph_name,
                    "i",
                    i,
                );
            }
            let journal_update = SparqlParser::new().parse_update(&format!(
                "INSERT {{ GRAPH {journal} {{ {templates} }} }} WHERE {{}}"
            ))?;
            let Op::DeleteInsert {
                insert: journal_templates,
                ..
            } = journal_update
                .operations
                .into_iter()
                .next()
                .context("missing journal template")?
            else {
                unreachable!()
            };
            // Evaluate WHERE once into the journal. Apply those instantiations
            // below, so membership checks can count the actual data delta.
            // This also preserves blank-node identity within each solution.
            let update = Update {
                base_iri: base.map(|b| b.parse()).transpose()?,
                operations: vec![Op::DeleteInsert {
                    delete: Vec::new(),
                    insert: journal_templates,
                    using,
                    pattern,
                }],
            };
            let mut prepared = SparqlEvaluator::new().parse_update(&update.to_string())?;
            let names = txn
                .named_graphs()
                .filter_map(|n| match n {
                    Ok(NamedOrBlankNode::NamedNode(n)) if n.as_str() == LEDGER_GRAPH => None,
                    other => Some(other),
                })
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for dataset in prepared.using_datasets_mut() {
                let allowed = dataset.available_named_graphs().map_or_else(|| names.clone(), |ns| ns.iter()
                    .filter(|n| !matches!(n, NamedOrBlankNode::NamedNode(n) if n.as_str() == LEDGER_GRAPH)).cloned().collect());
                dataset.set_available_named_graphs(allowed);
                if let Some(default) = dataset.default_graph_graphs() {
                    dataset.set_default_graph(default.iter().filter(|n| !matches!(n, GraphName::NamedNode(n) if n.as_str() == LEDGER_GRAPH)).cloned().collect());
                }
            }
            prepared.on_transaction(txn).execute()?;
            let mut rows: BTreeMap<String, BTreeMap<String, Term>> = BTreeMap::new();
            for q in txn.quads_for_pattern(None, None, None, Some(journal.as_ref().into())) {
                let q = q?;
                rows.entry(q.subject.to_string())
                    .or_default()
                    .insert(q.predicate.as_str().into(), q.object);
            }
            for row in rows.values() {
                let (Some(s), Some(p), Some(o), Some(g), Some(kind)) = (
                    row.get("urn:oo:s"),
                    row.get("urn:oo:p"),
                    row.get("urn:oo:o"),
                    row.get("urn:oo:g"),
                    row.get("urn:oo:kind"),
                ) else {
                    continue;
                };
                let Ok(s) = NamedOrBlankNode::try_from(s.clone()) else {
                    continue;
                };
                let Term::NamedNode(p) = p else { continue };
                let graph = match g {
                    Term::Literal(l)
                        if l.value().is_empty()
                            && matches!(row.get("urn:oo:default"), Some(Term::Literal(flag)) if flag.value() == "true") =>
                    {
                        GraphName::DefaultGraph
                    }
                    Term::NamedNode(n) => n.clone().into(),
                    Term::BlankNode(b) => b.clone().into(),
                    _ => continue,
                };
                let q = Quad::new(s, p.clone(), o.clone(), graph);
                check(&q)?;
                if matches!(kind, Term::Literal(l) if l.value() == "i") {
                    inserted.push(q);
                } else {
                    deleted.push(q);
                }
            }
            txn.remove_named_graph(&journal)?;
            for q in &deleted {
                if txn.contains(q)? {
                    txn.remove(q);
                    delta -= 1;
                }
            }
            for q in &inserted {
                if !txn.contains(q)? {
                    txn.insert(q);
                    delta += 1;
                }
            }
        }
        Op::Clear { silent, graph } | Op::Drop { silent, graph } => {
            let drop = drop_graph;
            let names: Vec<GraphName> = match graph {
                GraphTarget::NamedNode(n) => {
                    ensure!(
                        n.as_str() != LEDGER_GRAPH,
                        "the derivation ledger is reserved"
                    );
                    if !silent {
                        ensure!(
                            txn.contains_named_graph(&n)?,
                            "the named graph does not exist"
                        );
                    }
                    vec![n.into()]
                }
                GraphTarget::DefaultGraph => vec![GraphName::DefaultGraph],
                GraphTarget::NamedGraphs | GraphTarget::AllGraphs => {
                    let mut names = txn.named_graphs().collect::<std::result::Result<Vec<_>, _>>()?
                        .into_iter().filter(|n| !matches!(n, NamedOrBlankNode::NamedNode(n) if n.as_str() == LEDGER_GRAPH))
                        .map(Into::into).collect::<Vec<_>>();
                    if graph == GraphTarget::AllGraphs {
                        names.push(GraphName::DefaultGraph);
                    }
                    names
                }
            };
            for name in names {
                let affected = txn
                    .quads_for_pattern(None, None, None, Some(name.as_ref()))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                delta -= affected.len() as isize;
                deleted.extend(affected);
                if drop && !name.is_default_graph() {
                    match &name {
                        GraphName::NamedNode(n) => txn.remove_named_graph(n)?,
                        GraphName::BlankNode(b) => txn.remove_named_graph(b)?,
                        GraphName::DefaultGraph => unreachable!(),
                    }
                } else {
                    txn.clear_graph(&name)?;
                }
            }
        }
        Op::Create { graph, silent } => {
            ensure!(
                graph.as_str() != LEDGER_GRAPH,
                "the derivation ledger is reserved"
            );
            if !silent {
                ensure!(
                    !txn.contains_named_graph(&graph)?,
                    "the named graph already exists"
                );
            }
            txn.insert_named_graph(&graph);
        }
        Op::Load {
            silent,
            source,
            destination,
        } => {
            ensure!(
                destination.to_string() != format!("<{LEDGER_GRAPH}>"),
                "the derivation ledger is reserved"
            );
            // LOAD's input is new data, not a copy of the destination dataset.
            let store = oxigraph::store::Store::new()?;
            let load = Op::Load {
                silent,
                source,
                destination,
            };
            SparqlEvaluator::new()
                .parse_update(&load.to_string())?
                .on_store(&store)
                .execute()?;
            for name in store.named_graphs() {
                txn.insert_named_graph(&name?);
            }
            for q in store.iter() {
                let q = q?;
                check(&q)?;
                if !txn.contains(&q)? {
                    txn.insert(&q);
                    delta += 1;
                }
                inserted.push(q);
            }
        }
    }
    Ok((inserted, deleted, delta))
}
