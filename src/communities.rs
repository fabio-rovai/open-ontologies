//! Community detection over the loaded graph, and the skeletons a language
//! model needs to summarise each community.
//!
//! Microsoft GraphRAG established the shape: cluster the entity graph, have a
//! model write a report per community, and answer corpus-wide questions by
//! map-reduce over those reports. Entity traversal alone cannot answer "what
//! are the themes here", because such a question has no anchor to traverse
//! from.
//!
//! This module takes the half that is deterministic and puts it in the
//! engine, following the project convention that the server computes what a
//! model cannot and never embeds a model of its own:
//!
//!   - detection is label propagation, seeded and iterated in a fixed order,
//!     so the same graph always yields the same communities;
//!   - each community is returned as a SKELETON: its size, its top members by
//!     degree, the relations inside it, and the bridges leaving it;
//!   - the connected orchestrator writes the summaries and does the
//!     map-reduce, in the conversation, with its own model.
//!
//! The expensive part of GraphRAG indexing is therefore under the caller's
//! control rather than buried in a pipeline, and the cheap part is exact.

use crate::graph::GraphStore;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

pub struct Communities {
    graph: Arc<GraphStore>,
}

/// One community, described well enough for a model to name and summarise it.
#[derive(serde::Serialize)]
pub struct CommunitySkeleton {
    pub id: usize,
    pub size: usize,
    /// Members, most connected first, with their labels.
    pub members: Vec<Member>,
    /// Relations whose ends are both inside this community.
    pub internal_relations: Vec<Relation>,
    /// Relations crossing to another community: how this one connects out.
    pub bridges: Vec<Bridge>,
}

#[derive(serde::Serialize)]
pub struct Member {
    pub iri: String,
    pub label: String,
    pub degree: usize,
}

#[derive(serde::Serialize)]
pub struct Relation {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

#[derive(serde::Serialize)]
pub struct Bridge {
    pub from: String,
    pub predicate: String,
    pub to: String,
    pub to_community: usize,
}

const SUMMARY_INSTRUCTION: &str = concat!(
    "Each community is a cluster of entities that refer to one another more ",
    "than to the rest of the graph. Write one short report per community: a ",
    "title, what it is about, and what its bridges say about how it connects ",
    "to the others. For a corpus-wide question, answer from the reports ",
    "first and only traverse into a community when the question needs a ",
    "specific fact. Nothing here is generated: membership, degrees, ",
    "relations and bridges are computed from the graph."
);

impl Communities {
    pub fn new(graph: Arc<GraphStore>) -> Self {
        Self { graph }
    }

    /// Detect communities and return their skeletons as JSON.
    ///
    /// `min_size` drops clusters too small to be worth a report; `top_members`
    /// bounds how many members are described per community, because a model
    /// needs the shape of a community, not a dump of it.
    pub fn detect(&self, min_size: usize, top_members: usize) -> anyhow::Result<String> {
        let edges = self.edges()?;
        if edges.is_empty() {
            return Ok(serde_json::json!({
                "ok": true,
                "communities": [],
                "note": "No relations between named subjects and objects: nothing to cluster."
            })
            .to_string());
        }

        // Adjacency over an undirected view: community structure is about
        // who refers to whom, not the direction of the reference.
        let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (s, _, o) in &edges {
            adjacency.entry(s.clone()).or_default().insert(o.clone());
            adjacency.entry(o.clone()).or_default().insert(s.clone());
        }

        let labels = self.propagate(&adjacency);

        // Group, order communities by size, and renumber so ids are stable
        // for a given graph rather than reflecting iteration accidents.
        let mut by_label: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        for (node, label) in &labels {
            by_label.entry(*label).or_default().push(node.clone());
        }
        let mut groups: Vec<Vec<String>> = by_label
            .into_values()
            .filter(|members| members.len() >= min_size)
            .collect();
        groups.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a[0].cmp(&b[0])));

        let mut community_of: HashMap<&str, usize> = HashMap::new();
        for (id, members) in groups.iter().enumerate() {
            for m in members {
                community_of.insert(m.as_str(), id);
            }
        }

        let display = self.labels_of(&adjacency.keys().cloned().collect::<Vec<_>>())?;
        let short = |iri: &str| -> String {
            display
                .get(iri)
                .cloned()
                .unwrap_or_else(|| local_name(iri).to_string())
        };

        let mut skeletons = Vec::new();
        for (id, members) in groups.iter().enumerate() {
            let mut ranked: Vec<Member> = members
                .iter()
                .map(|iri| Member {
                    degree: adjacency.get(iri).map(|n| n.len()).unwrap_or(0),
                    label: short(iri),
                    iri: iri.clone(),
                })
                .collect();
            ranked.sort_by(|a, b| b.degree.cmp(&a.degree).then_with(|| a.iri.cmp(&b.iri)));
            ranked.truncate(top_members);

            let member_set: BTreeSet<&str> = members.iter().map(|s| s.as_str()).collect();
            let mut internal = Vec::new();
            let mut bridges = Vec::new();
            for (s, p, o) in &edges {
                let (si, oi) = (member_set.contains(s.as_str()), member_set.contains(o.as_str()));
                if si && oi {
                    if internal.len() < top_members * 2 {
                        internal.push(Relation {
                            subject: short(s),
                            predicate: local_name(p).to_string(),
                            object: short(o),
                        });
                    }
                } else if si || oi {
                    let (from, to) = if si { (s, o) } else { (o, s) };
                    let other = community_of.get(to.as_str()).copied();
                    if let Some(other) = other.filter(|&c| c != id && bridges.len() < top_members) {
                        {
                            bridges.push(Bridge {
                                from: short(from),
                                predicate: local_name(p).to_string(),
                                to: short(to),
                                to_community: other,
                            });
                        }
                    }
                }
            }

            skeletons.push(CommunitySkeleton {
                id,
                size: members.len(),
                members: ranked,
                internal_relations: internal,
                bridges,
            });
        }

        Ok(serde_json::json!({
            "ok": true,
            "communities": skeletons,
            "community_count": skeletons.len(),
            "clustered_nodes": labels.len(),
            "summary_instruction": SUMMARY_INSTRUCTION,
        })
        .to_string())
    }

    /// Greedy modularity optimisation, the first phase of Louvain, which
    /// Leiden (used by GraphRAG) descends from.
    ///
    /// Label propagation was tried first and rejected: on small graphs a
    /// single edge between two otherwise separate clusters is enough for one
    /// label to sweep both, and two triangles joined by one link came back as
    /// one community. Modularity asks a better question, whether a grouping
    /// has more internal edges than chance would give it, so that same link
    /// leaves the triangles apart.
    ///
    /// Nodes are visited in sorted order and ties go to the smallest
    /// community, so the result is reproducible rather than seed-dependent.
    fn propagate(&self, adjacency: &BTreeMap<String, BTreeSet<String>>) -> BTreeMap<String, usize> {
        let nodes: Vec<&String> = adjacency.keys().collect();
        let degree = |n: &str| adjacency.get(n).map(|s| s.len()).unwrap_or(0) as f64;
        let two_m: f64 = nodes.iter().map(|n| degree(n)).sum();
        if two_m == 0.0 {
            return nodes.iter().enumerate().map(|(i, n)| ((*n).clone(), i)).collect();
        }

        let mut community: BTreeMap<String, usize> =
            nodes.iter().enumerate().map(|(i, n)| ((*n).clone(), i)).collect();
        // Sum of degrees of the members of each community.
        let mut tot: BTreeMap<usize, f64> =
            nodes.iter().enumerate().map(|(i, n)| (i, degree(n))).collect();

        for _ in 0..20 {
            let mut moved = false;
            for node in &nodes {
                let node = node.as_str();
                let k_i = degree(node);
                let current = community[node];
                *tot.entry(current).or_insert(0.0) -= k_i;

                // Edges from this node into each candidate community.
                let mut links: BTreeMap<usize, f64> = BTreeMap::new();
                if let Some(neigh) = adjacency.get(node) {
                    for n in neigh {
                        if let Some(&c) = community.get(n) {
                            *links.entry(c).or_insert(0.0) += 1.0;
                        }
                    }
                }

                // Modularity gain of joining c: internal links gained, less
                // what random attachment would already predict.
                let gain = |c: usize| -> f64 {
                    links.get(&c).copied().unwrap_or(0.0)
                        - tot.get(&c).copied().unwrap_or(0.0) * k_i / two_m
                };
                let mut best = current;
                let mut best_gain = gain(current);
                for &c in links.keys() {
                    let g = gain(c);
                    if g > best_gain + 1e-12 || (g > best_gain - 1e-12 && c < best) {
                        best = c;
                        best_gain = g;
                    }
                }

                *tot.entry(best).or_insert(0.0) += k_i;
                if best != current {
                    community.insert(node.to_string(), best);
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
        community
    }

    /// Relations between named things, excluding schema-level triples: the
    /// entity graph, which is what communities are about.
    fn edges(&self) -> anyhow::Result<Vec<(String, String, String)>> {
        let query = "SELECT ?s ?p ?o WHERE { \
            ?s ?p ?o . \
            FILTER(isIRI(?s) && isIRI(?o)) \
            FILTER(?p != <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>) \
            FILTER(?p != <http://www.w3.org/2000/01/rdf-schema#subClassOf>) \
            FILTER(?p != <http://www.w3.org/2000/01/rdf-schema#subPropertyOf>) \
            FILTER(?p != <http://www.w3.org/2000/01/rdf-schema#domain>) \
            FILTER(?p != <http://www.w3.org/2000/01/rdf-schema#range>) \
            FILTER(?p != <http://www.w3.org/2002/07/owl#disjointWith>) \
            FILTER(?p != <http://www.w3.org/2002/07/owl#equivalentClass>) \
        } LIMIT 20000";
        let raw = self.graph.sparql_select(query)?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)?;
        let rows = parsed
            .get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(rows
            .iter()
            .filter_map(|row| {
                Some((
                    strip(row.get("s")?.as_str()?),
                    strip(row.get("p")?.as_str()?),
                    strip(row.get("o")?.as_str()?),
                ))
            })
            .collect())
    }

    /// rdfs:label per IRI, for skeletons a human can read.
    fn labels_of(&self, iris: &[String]) -> anyhow::Result<HashMap<String, String>> {
        let query = "SELECT ?s ?l WHERE { ?s <http://www.w3.org/2000/01/rdf-schema#label> ?l } LIMIT 20000";
        let raw = self.graph.sparql_select(query)?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)?;
        let wanted: BTreeSet<&str> = iris.iter().map(|s| s.as_str()).collect();
        let mut out = HashMap::new();
        if let Some(rows) = parsed.get("results").and_then(|r| r.as_array()) {
            for row in rows {
                if let (Some(s), Some(l)) = (
                    row.get("s").and_then(|v| v.as_str()),
                    row.get("l").and_then(|v| v.as_str()),
                ) {
                    let s = strip(s);
                    if wanted.contains(s.as_str()) {
                        out.entry(s).or_insert_with(|| strip(l));
                    }
                }
            }
        }
        Ok(out)
    }
}

/// SPARQL results arrive as `<iri>` or `"literal"`; skeletons want neither.
fn strip(value: &str) -> String {
    let v = value.trim();
    if v.starts_with('<') && v.ends_with('>') {
        return v[1..v.len() - 1].to_string();
    }
    if let Some(body) = v.strip_prefix('"') {
        for cut in ["\"^^", "\"@", "\""] {
            if let Some(i) = body.find(cut) {
                return body[..i].to_string();
            }
        }
    }
    v.to_string()
}

fn local_name(iri: &str) -> &str {
    iri.rsplit(['#', '/']).next().unwrap_or(iri)
}
