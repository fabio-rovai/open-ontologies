//! Bi-temporal facts: when something was true, and when we learned it.
//!
//! Two clocks, deliberately independent:
//!
//!   - VALID time is when a statement holds in the world. A cell line was
//!     adherent until May and suspension after; both statements are true, of
//!     different periods.
//!   - RECORDED time is when the store came to hold it. A correction entered
//!     today about last year changes what we know, not what happened.
//!
//! Collapsing them loses the two questions people actually ask: what was true
//! then, and what did we believe then. An audit needs the second; analysis
//! needs the first; a contradiction check needs both, because two statements
//! only conflict if they claim the same period.
//!
//! ## Shape on disk
//!
//! RDF-star would be the elegant carrier and the parser does not accept it,
//! so assertions live in NAMED GRAPHS and their validity is described in the
//! default graph, which is ordinary TriG that any store can read:
//!
//! ```turtle
//! :g1 { :HEK293 a :AdherentCellLine . }
//! :g2 { :HEK293 a :SuspensionCellLine . }
//! {
//!   :g1 t:validFrom "2024-01-01"^^xsd:date ;
//!       t:validTo   "2026-05-01"^^xsd:date ;
//!       t:recordedAt "2024-01-05"^^xsd:dateTime .
//!   :g2 t:validFrom "2026-05-01"^^xsd:date ;
//!       t:recordedAt "2026-05-02"^^xsd:dateTime .
//! }
//! ```
//!
//! An absent `validFrom` means "since always", an absent `validTo` means
//! "still true", and a graph with no temporal description at all is timeless:
//! it is in scope for every snapshot, so adding this vocabulary to an
//! existing store changes nothing until it is used.
//!
//! Intervals are half-open, `[validFrom, validTo)`. Two facts that meet at a
//! boundary do not overlap, which is what makes "adherent until May,
//! suspension from May" a correction rather than a contradiction.
//!
//! ## Bounded scans
//!
//! Every query behind these tools is capped so a pathological store cannot
//! pull the whole dataset into memory. A cap that quietly changes the answer
//! is worse than a slow query, so each response carries `complete`, and a run
//! that was cut short says which scan was cut and at what limit.
//!
//! The two failures are not equally bad. Truncating a result list gives you
//! fewer rows. Truncating the validity scan gives you a WRONG scope: a graph
//! whose description fell past the cap reads as having no description at all,
//! and an undescribed graph is timeless and always in scope. Those responses
//! also carry a `warning`.

use crate::graph::GraphStore;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub const NS: &str = "https://open-ontologies.org/temporal#";

pub struct Temporal {
    graph: Arc<GraphStore>,
    limits: Limits,
}

/// Validity ROWS, not graphs. The query is a three-way UNION, so a graph
/// carrying validFrom, validTo and recordedAt costs three rows: the cap is
/// reached at roughly 6,700 fully described graphs.
const VALIDITY_SCAN_LIMIT: usize = 20_000;
/// Distinct named graphs holding assertions.
const GRAPH_SCAN_LIMIT: usize = 20_000;
/// Result rows returned by a snapshot-scoped query.
const QUERY_ROW_LIMIT: usize = 10_000;
/// Candidate disjointness pairs examined for overlap.
const CONFLICT_PAIR_LIMIT: usize = 5_000;

/// The four scan caps. Only `Limits::default` ships; the field-by-field form
/// exists so a test can reach a truncated scan with three graphs instead of
/// twenty thousand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Limits {
    validity_scan: usize,
    graph_scan: usize,
    query_rows: usize,
    conflict_pairs: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            validity_scan: VALIDITY_SCAN_LIMIT,
            graph_scan: GRAPH_SCAN_LIMIT,
            query_rows: QUERY_ROW_LIMIT,
            conflict_pairs: CONFLICT_PAIR_LIMIT,
        }
    }
}

/// Whether one capped scan was cut short, and at what cap. Carried apart from
/// the rows, so a caller that has already folded the rows into a map can still
/// say what the map was built from.
#[derive(Clone, Copy, Debug)]
struct Capped {
    hit: bool,
    cap: usize,
}

impl Capped {
    /// A machine-readable report, or `None` when the scan was complete: a
    /// response that was never cut gains no `truncated` entry at all.
    fn report(self, scan: &str, consequence: &str) -> Option<serde_json::Value> {
        self.hit.then(|| {
            serde_json::json!({
                "scan": scan,
                "limit": self.cap,
                "consequence": consequence,
            })
        })
    }
}

/// The rows one capped scan returned, and whether the cap cut it short.
struct Scan {
    rows: Vec<serde_json::Value>,
    capped: Capped,
}

/// The graphs a snapshot puts in and out of scope, and how trustworthy that
/// partition is. `snapshot` renders it; `query_at` runs against it.
struct Scope {
    in_scope: Vec<serde_json::Value>,
    excluded: Vec<serde_json::Value>,
    /// The in-scope graph IRIs, in the same order as `in_scope`.
    graphs: Vec<String>,
    validity_scan: Capped,
    graph_scan: Capped,
}

impl Scope {
    fn complete(&self) -> bool {
        !self.validity_scan.hit && !self.graph_scan.hit
    }

    /// One report per scan that was cut. Both scans decide the partition, so
    /// either one being cut makes it wrong rather than merely short.
    fn cuts(&self) -> Vec<serde_json::Value> {
        [
            self.validity_scan.report(
                "validities",
                "graphs whose validity rows fell past the limit were read as having no \
                 validity at all, so they are listed in scope as timeless even where their \
                 period excludes them",
            ),
            self.graph_scan.report(
                "all_graphs",
                "graphs past the limit are missing from both in_scope and excluded",
            ),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// The loud half, in SHACL's register (`src/shacl.rs`): a sentence that
    /// refuses to let a degraded answer read as a clean one. Callers add what
    /// it means for their own answer and the pointer to `truncated`.
    fn warning(&self) -> Option<String> {
        let mut parts: Vec<&str> = Vec::new();
        if self.validity_scan.hit {
            parts.push(
                "described graphs were read as timeless and put in scope, which is the \
                 opposite of the truth for any graph whose period had ended",
            );
        }
        if self.graph_scan.hit {
            parts.push("graphs are missing from both in_scope and excluded");
        }
        if parts.is_empty() {
            return None;
        }
        Some(format!(
            "INCOMPLETE SCOPE: a scan hit its row limit, so {}. The scope is not merely short, \
             part of it is wrong.",
            parts.join(", and ")
        ))
    }
}

#[derive(Clone, Debug)]
pub struct Validity {
    pub graph: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub recorded_at: Option<String>,
}

impl Validity {
    /// Was this true at `instant`, on the half-open interval.
    fn valid_at(&self, instant: &str) -> bool {
        self.valid_from.as_deref().is_none_or(|f| f <= instant)
            && self.valid_to.as_deref().is_none_or(|t| instant < t)
    }

    /// Had we recorded it by `instant`.
    fn recorded_by(&self, instant: &str) -> bool {
        self.recorded_at.as_deref().is_none_or(|r| r <= instant)
    }

    /// Do two validity periods share any instant. Half-open, so touching
    /// intervals do not overlap.
    fn overlaps(&self, other: &Validity) -> bool {
        let start_before_end = |a: &Option<String>, b: &Option<String>| match (a, b) {
            (Some(start), Some(end)) => start.as_str() < end.as_str(),
            _ => true, // an open end never closes the interval
        };
        start_before_end(&self.valid_from, &other.valid_to)
            && start_before_end(&other.valid_from, &self.valid_to)
    }

    fn describe(&self) -> String {
        let from = self.valid_from.as_deref().unwrap_or("always");
        let to = self.valid_to.as_deref().unwrap_or("still true");
        format!("{from} to {to}")
    }
}

/// Literal or IRI as SPARQL returns it, without its wrapping.
fn plain(value: &str) -> String {
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

fn local(iri: &str) -> &str {
    iri.rsplit(['#', '/']).next().unwrap_or(iri)
}

impl Temporal {
    pub fn new(graph: Arc<GraphStore>) -> Self {
        Self::with_limits(graph, Limits::default())
    }

    /// The same view over the same store with different scan caps.
    ///
    /// Private, and the only caller outside this module's tests is `new` with
    /// the shipped defaults: the caps are a safety bound, not a knob.
    fn with_limits(graph: Arc<GraphStore>, limits: Limits) -> Self {
        Self { graph, limits }
    }

    /// Run one capped scan.
    ///
    /// `query` must NOT carry its own LIMIT: the cap is appended here as
    /// `cap + 1`. That extra row is a probe rather than data — if it comes
    /// back, more rows exist, which is proof instead of the guess you get from
    /// comparing a returned count against the cap (a store holding exactly
    /// `cap` rows is complete and must not be reported as cut). The probe is
    /// dropped before returning, so an untruncated scan yields exactly the
    /// rows it yielded in 1.2.0.
    fn rows(&self, query: &str, cap: usize) -> anyhow::Result<Scan> {
        let probe = cap.saturating_add(1);
        let raw = self
            .graph
            .sparql_select(&format!("{query} LIMIT {probe}"))?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)?;
        let mut rows: Vec<serde_json::Value> = parsed
            .get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();
        let hit = rows.len() > cap;
        rows.truncate(cap);
        Ok(Scan {
            rows,
            capped: Capped { hit, cap },
        })
    }

    /// Every graph that carries validity metadata, and whether the scan that
    /// found them was complete.
    fn validities(&self) -> anyhow::Result<(BTreeMap<String, Validity>, Capped)> {
        let query = format!(
            "SELECT ?g ?from ?to ?rec WHERE {{ \
             {{ ?g <{NS}validFrom> ?from }} UNION {{ ?g <{NS}validTo> ?to }} \
             UNION {{ ?g <{NS}recordedAt> ?rec }} }}"
        );
        let scan = self.rows(&query, self.limits.validity_scan)?;
        let mut out: BTreeMap<String, Validity> = BTreeMap::new();
        for row in &scan.rows {
            let Some(g) = row.get("g").and_then(|v| v.as_str()).map(plain) else {
                continue;
            };
            let entry = out.entry(g.clone()).or_insert(Validity {
                graph: g,
                valid_from: None,
                valid_to: None,
                recorded_at: None,
            });
            if let Some(v) = row.get("from").and_then(|v| v.as_str()) {
                entry.valid_from = Some(plain(v));
            }
            if let Some(v) = row.get("to").and_then(|v| v.as_str()) {
                entry.valid_to = Some(plain(v));
            }
            if let Some(v) = row.get("rec").and_then(|v| v.as_str()) {
                entry.recorded_at = Some(plain(v));
            }
        }
        Ok((out, scan.capped))
    }

    /// Named graphs holding assertions, whether or not they are described.
    fn all_graphs(&self) -> anyhow::Result<(BTreeSet<String>, Capped)> {
        let scan = self.rows(
            "SELECT DISTINCT ?g WHERE { GRAPH ?g { ?s ?p ?o } }",
            self.limits.graph_scan,
        )?;
        let graphs = scan
            .rows
            .iter()
            .filter_map(|r| r.get("g").and_then(|v| v.as_str()).map(plain))
            .collect();
        Ok((graphs, scan.capped))
    }

    /// Partition the store's named graphs into in scope and excluded.
    ///
    /// `snapshot` renders this and `query_at` runs against it, so the two
    /// tools agree by construction and the truncation verdict reaches
    /// `query_at` as a value rather than through its own JSON output.
    fn scope(&self, valid_at: Option<&str>, as_of: Option<&str>) -> anyhow::Result<Scope> {
        let (validities, validity_scan) = self.validities()?;
        let (graphs, graph_scan) = self.all_graphs()?;

        let mut in_scope = Vec::new();
        let mut excluded = Vec::new();
        let mut names = Vec::new();
        for g in &graphs {
            match validities.get(g) {
                // Undescribed graphs are timeless and always in scope, so
                // this vocabulary is additive to an existing store.
                None => {
                    in_scope.push(
                        serde_json::json!({"graph": g, "reason": "no validity recorded, timeless"}),
                    );
                    names.push(g.clone());
                }
                Some(v) => {
                    let valid_ok = valid_at.is_none_or(|t| v.valid_at(t));
                    let recorded_ok = as_of.is_none_or(|t| v.recorded_by(t));
                    if valid_ok && recorded_ok {
                        in_scope.push(serde_json::json!({"graph": g, "valid": v.describe()}));
                        names.push(g.clone());
                    } else {
                        excluded.push(serde_json::json!({
                            "graph": g,
                            "valid": v.describe(),
                            "reason": if !valid_ok { "not true at that instant" } else { "not yet recorded then" },
                        }));
                    }
                }
            }
        }

        Ok(Scope {
            in_scope,
            excluded,
            graphs: names,
            validity_scan,
            graph_scan,
        })
    }

    /// Which graphs are in scope for a snapshot, and why.
    pub fn snapshot(&self, valid_at: Option<&str>, as_of: Option<&str>) -> anyhow::Result<String> {
        let scope = self.scope(valid_at, as_of)?;

        let mut out = serde_json::json!({
            "ok": true,
            "valid_at": valid_at,
            "as_of": as_of,
            "in_scope": scope.in_scope,
            "excluded": scope.excluded,
            "complete": scope.complete(),
            "note": "Graphs without validity metadata are timeless and always in scope.",
        });
        if let Some(warning) = scope.warning() {
            out["warning"] = serde_json::Value::String(format!("{warning} See truncated."));
            out["truncated"] = serde_json::Value::Array(scope.cuts());
        }
        Ok(out.to_string())
    }

    /// Run a query against only the graphs in temporal scope.
    ///
    /// The query is wrapped rather than rewritten: its pattern is evaluated
    /// inside a GRAPH block restricted to the snapshot, which keeps arbitrary
    /// SPARQL working without parsing it.
    pub fn query_at(
        &self,
        pattern: &str,
        valid_at: Option<&str>,
        as_of: Option<&str>,
    ) -> anyhow::Result<String> {
        let scope = self.scope(valid_at, as_of)?;
        // The scope this query runs over is the snapshot's scope, so the
        // snapshot's truncation is this query's truncation. Saying nothing
        // here would hide a wrong scope behind a tool that never mentions
        // scans at all.
        let mut cuts = scope.cuts();
        let scope_warning = scope.warning();
        let scope_complete = scope.complete();
        let graphs = scope.graphs;

        if graphs.is_empty() {
            let mut out = serde_json::json!({
                "ok": true,
                "results": [],
                "complete": scope_complete,
                "note": "no graphs in scope at that instant",
            });
            if let Some(warning) = scope_warning {
                out["warning"] = serde_json::Value::String(format!(
                    "{warning} An empty scope here may mean \"nothing among the graphs that \
                     were read\" rather than \"nothing\". See truncated."
                ));
                out["truncated"] = serde_json::Value::Array(cuts);
            }
            return Ok(out.to_string());
        }

        let values = graphs
            .iter()
            .map(|g| format!("<{g}>"))
            .collect::<Vec<_>>()
            .join(" ");
        let body = pattern.trim();
        let body = body
            .strip_prefix('{')
            .and_then(|b| b.strip_suffix('}'))
            .unwrap_or(body);
        let wrapped =
            format!("SELECT * WHERE {{ VALUES ?__g {{ {values} }} GRAPH ?__g {{ {body} }} }}");
        let scan = self.rows(&wrapped, self.limits.query_rows)?;
        cuts.extend(scan.capped.report(
            "query_rows",
            "the result list is the first page only; rows past the limit were not returned",
        ));

        let mut out = serde_json::json!({
            "ok": true,
            "valid_at": valid_at,
            "as_of": as_of,
            "graphs_in_scope": graphs.len(),
            "results": scan.rows,
            "complete": scope_complete && !scan.capped.hit,
        });
        if !cuts.is_empty() {
            out["truncated"] = serde_json::Value::Array(cuts);
        }
        // A short result list is fewer rows and stays a quiet key. A cut scope
        // is a wrong answer, so it keeps the warning it arrived with.
        if let Some(warning) = scope_warning {
            out["warning"] = serde_json::Value::String(format!(
                "{warning} These results were drawn from that scope, so they come from the \
                 wrong set of graphs. See truncated."
            ));
        }
        Ok(out.to_string())
    }

    /// Disjointness violations, but only where the two assertions claim
    /// OVERLAPPING validity.
    ///
    /// This is the point of carrying valid time at all. Without it, a
    /// correction reads as a contradiction: an entity recorded as one thing
    /// until May and another thereafter trips every disjointness check, and
    /// the finding is noise. With it, a superseded statement is superseded,
    /// and only genuine disagreement about the same period survives.
    pub fn conflicts(&self) -> anyhow::Result<String> {
        let (validities, validity_scan) = self.validities()?;
        let scan = self.rows(
            "PREFIX owl: <http://www.w3.org/2002/07/owl#> \
             PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> \
             SELECT DISTINCT ?s ?a ?b ?ga ?gb WHERE { \
               GRAPH ?ga { ?s a ?a } GRAPH ?gb { ?s a ?b } \
               FILTER(STR(?a) < STR(?b)) \
               ?a rdfs:subClassOf* ?da . ?b rdfs:subClassOf* ?db . \
               { ?da owl:disjointWith ?db } UNION { ?db owl:disjointWith ?da } \
             }",
            self.limits.conflict_pairs,
        )?;

        let mut conflicts = Vec::new();
        let mut superseded = Vec::new();
        for row in &scan.rows {
            let get = |k: &str| row.get(k).and_then(|v| v.as_str()).map(plain);
            let (Some(s), Some(a), Some(b), Some(ga), Some(gb)) = (
                get("s"), get("a"), get("b"), get("ga"), get("gb"),
            ) else {
                continue;
            };
            if ga == gb {
                continue;
            }

            let timeless = Validity {
                graph: String::new(),
                valid_from: None,
                valid_to: None,
                recorded_at: None,
            };
            let va = validities.get(&ga).unwrap_or(&timeless);
            let vb = validities.get(&gb).unwrap_or(&timeless);

            let entry = serde_json::json!({
                "subject": local(&s),
                "types": [local(&a), local(&b)],
                "periods": [va.describe(), vb.describe()],
                "graphs": [ga, gb],
            });
            if va.overlaps(vb) {
                conflicts.push(entry);
            } else {
                superseded.push(entry);
            }
        }

        let mut out = serde_json::json!({
            "ok": true,
            "contradictions": conflicts,
            "contradiction_count": conflicts.len(),
            "superseded": superseded,
            "superseded_count": superseded.len(),
            "complete": !validity_scan.hit && !scan.capped.hit,
            "note": "contradictions claim overlapping validity and genuinely disagree. \
                     superseded pairs are corrections: one period ends where the other begins, \
                     which is a history rather than a conflict.",
        });

        // A cut validity scan is not a smaller answer here, it is a wrong one:
        // a graph whose rows fell past the cap takes the `timeless` fallback
        // above, a timeless period has open ends, so `overlaps` returns true
        // against everything and a correction that should read as superseded
        // history is published as a live contradiction.
        let cuts: Vec<serde_json::Value> = [
            validity_scan.report(
                "validities",
                "pairs whose validity rows fell past the limit were compared as timeless, and \
                 a timeless period overlaps everything: superseded corrections can appear here \
                 as contradictions",
            ),
            scan.capped.report(
                "conflict_pairs",
                "candidate pairs past the limit were never examined, so the absence of a \
                 contradiction is not evidence that there is none",
            ),
        ]
        .into_iter()
        .flatten()
        .collect();

        if !cuts.is_empty() {
            out["truncated"] = serde_json::Value::Array(cuts);
        }
        if validity_scan.hit {
            out["warning"] = serde_json::Value::String(
                "UNSOUND CLASSIFICATION: the validity scan hit its row limit, so some pairs \
                 were compared without their periods. A correction can be reported here as a \
                 contradiction, which is the one thing this tool exists to prevent; \
                 contradiction_count is an upper bound. See truncated."
                    .to_string(),
            );
        }
        Ok(out.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxigraph::io::RdfFormat;

    /// Three graphs, one validity row each: a cap of 2 cuts a scan with a
    /// dataset that fits on a screen.
    const THREE: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1 { ex:X ex:p ex:one . }
ex:g2 { ex:X ex:p ex:two . }
ex:g3 { ex:X ex:p ex:three . }

{
  ex:g1 t:validFrom "2024-01-01" .
  ex:g2 t:validFrom "2025-01-01" .
  ex:g3 t:validFrom "2026-01-01" .
}
"#;

    /// A correction: one type up to a boundary, a disjoint one from it.
    /// Half-open, so this is superseded history and not a contradiction.
    const CORRECTION: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g_before { ex:X a ex:Adherent . }
ex:g_after  { ex:X a ex:Suspension . }

{
  ex:Adherent owl:disjointWith ex:Suspension .
  ex:g_before t:validFrom "2024-01-01" ; t:validTo "2026-05-01" .
  ex:g_after  t:validFrom "2026-05-01" .
}
"#;

    fn store(trig: &str) -> Arc<GraphStore> {
        let store = Arc::new(GraphStore::new());
        store
            .load_content(trig, RdfFormat::TriG)
            .expect("TriG fixture parses");
        store
    }

    fn json(raw: String) -> serde_json::Value {
        serde_json::from_str(&raw).expect("tool output is JSON")
    }

    /// The overrides below are for tests. If one ever became the shipped
    /// value, this is the line that says so.
    #[test]
    fn shipped_caps_are_the_defaults() {
        let l = Limits::default();
        assert_eq!(
            (
                l.validity_scan,
                l.graph_scan,
                l.query_rows,
                l.conflict_pairs
            ),
            (
                VALIDITY_SCAN_LIMIT,
                GRAPH_SCAN_LIMIT,
                QUERY_ROW_LIMIT,
                CONFLICT_PAIR_LIMIT
            )
        );
    }

    /// The contract for every store that never reaches a cap: one honest
    /// `complete`, and not a key more than 1.2.0 emitted.
    #[test]
    fn a_run_that_was_never_cut_says_so_and_adds_nothing_else() {
        let t = Temporal::new(store(THREE));
        for out in [
            json(t.snapshot(None, None).unwrap()),
            json(t.query_at("{ ?s ?p ?o }", None, None).unwrap()),
            json(t.conflicts().unwrap()),
        ] {
            assert_eq!(out["complete"], true, "{out}");
            assert!(out.get("truncated").is_none(), "{out}");
            assert!(out.get("warning").is_none(), "{out}");
        }
    }

    /// Why the probe row exists. Three graphs, three validity rows, both caps
    /// set to exactly three: the pages are full and nothing is missing. A
    /// `len() == cap` detector reports two false truncations here.
    #[test]
    fn an_exactly_full_scan_is_not_reported_as_truncated() {
        let t = Temporal::with_limits(
            store(THREE),
            Limits {
                validity_scan: 3,
                graph_scan: 3,
                ..Limits::default()
            },
        );
        let snap = json(t.snapshot(None, None).unwrap());
        assert_eq!(snap["in_scope"].as_array().unwrap().len(), 3);
        assert_eq!(
            snap["complete"], true,
            "a full page is not a cut one: {snap}"
        );
        assert!(snap.get("truncated").is_none(), "{snap}");
    }

    /// The case that has to be loud. Every graph in the fixture is described,
    /// so a graph the snapshot calls timeless is provably a misclassification.
    #[test]
    fn a_cut_validity_scan_makes_the_snapshot_loudly_incomplete() {
        let t = Temporal::with_limits(
            store(THREE),
            Limits {
                validity_scan: 2,
                ..Limits::default()
            },
        );
        let snap = json(t.snapshot(Some("2024-06-01"), None).unwrap());

        assert_eq!(snap["complete"], false, "{snap}");
        assert!(
            snap["warning"]
                .as_str()
                .expect("a cut scope carries a warning")
                .contains("INCOMPLETE SCOPE"),
            "loud, not a quiet key: {snap}"
        );
        assert_eq!(snap["truncated"][0]["scan"], "validities");
        assert_eq!(snap["truncated"][0]["limit"], 2);

        // SPARQL does not order an unordered LIMIT, so WHICH graph lost its
        // description is not fixed — but exactly one did, whichever it was.
        let misclassified = snap["in_scope"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row.get("reason").is_some())
            .count();
        assert_eq!(
            misclassified, 1,
            "a described graph was published as timeless and in scope, which is what the \
             warning is for: {snap}"
        );
    }

    #[test]
    fn a_cut_graph_scan_drops_graphs_from_both_buckets() {
        let t = Temporal::with_limits(
            store(THREE),
            Limits {
                graph_scan: 2,
                ..Limits::default()
            },
        );
        let snap = json(t.snapshot(None, None).unwrap());
        let seen =
            snap["in_scope"].as_array().unwrap().len() + snap["excluded"].as_array().unwrap().len();
        assert_eq!(seen, 2, "the third graph is in neither bucket: {snap}");
        assert_eq!(snap["complete"], false);
        assert_eq!(snap["truncated"][0]["scan"], "all_graphs");
    }

    #[test]
    fn query_at_inherits_the_scope_truncation() {
        let t = Temporal::with_limits(
            store(THREE),
            Limits {
                validity_scan: 2,
                ..Limits::default()
            },
        );
        let out = json(
            t.query_at("{ ?s ?p ?o }", Some("2024-06-01"), None)
                .unwrap(),
        );
        assert_eq!(
            out["complete"], false,
            "a query over a wrong scope is a wrong query: {out}"
        );
        assert!(
            out["warning"]
                .as_str()
                .unwrap()
                .contains("INCOMPLETE SCOPE"),
            "{out}"
        );
        assert!(
            out["truncated"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["scan"] == "validities"),
            "{out}"
        );
    }

    /// The other half of the distinction: a short result list is fewer rows,
    /// not a wrong answer, so it is reported without the warning.
    #[test]
    fn query_at_reports_its_own_row_cap_without_calling_the_scope_wrong() {
        let t = Temporal::with_limits(
            store(THREE),
            Limits {
                query_rows: 2,
                ..Limits::default()
            },
        );
        let out = json(t.query_at("{ ?s ?p ?o }", None, None).unwrap());
        assert_eq!(
            out["results"].as_array().unwrap().len(),
            2,
            "the probe row is dropped, not returned: {out}"
        );
        assert_eq!(out["complete"], false);
        assert_eq!(out["truncated"][0]["scan"], "query_rows");
        assert_eq!(out["truncated"][0]["limit"], 2);
        assert!(
            out.get("warning").is_none(),
            "a short result list is not a wrong scope: {out}"
        );
    }

    /// Truncation here produces a false POSITIVE, which is why conflicts is
    /// loud too. Cap 1: whichever validity row survives, one graph is left
    /// undescribed, an undescribed period is timeless, and a timeless period
    /// overlaps everything.
    #[test]
    fn a_cut_validity_scan_turns_a_correction_into_a_false_contradiction() {
        let sound = json(Temporal::new(store(CORRECTION)).conflicts().unwrap());
        assert_eq!(sound["contradiction_count"], 0);
        assert_eq!(sound["superseded_count"], 1);
        assert_eq!(sound["complete"], true, "{sound}");

        let cut = json(
            Temporal::with_limits(
                store(CORRECTION),
                Limits {
                    validity_scan: 1,
                    ..Limits::default()
                },
            )
            .conflicts()
            .unwrap(),
        );
        assert_eq!(
            cut["contradiction_count"], 1,
            "the same correction now reads as a live conflict: {cut}"
        );
        assert_eq!(cut["superseded_count"], 0);
        assert_eq!(cut["complete"], false, "and it is labelled: {cut}");
        assert!(
            cut["warning"]
                .as_str()
                .unwrap()
                .contains("UNSOUND CLASSIFICATION"),
            "{cut}"
        );
        assert_eq!(cut["truncated"][0]["scan"], "validities");
        assert_eq!(cut["truncated"][0]["limit"], 1);
    }

    /// The pair scan is the mild cap: fewer pairs examined, no claim turned
    /// upside down, so it is reported without the warning.
    #[test]
    fn a_cut_pair_scan_is_reported_quietly() {
        let t = Temporal::with_limits(
            store(CORRECTION),
            Limits {
                conflict_pairs: 0,
                ..Limits::default()
            },
        );
        let out = json(t.conflicts().unwrap());
        assert_eq!(out["contradiction_count"], 0);
        assert_eq!(out["superseded_count"], 0);
        assert_eq!(out["complete"], false, "{out}");
        assert_eq!(out["truncated"][0]["scan"], "conflict_pairs");
        assert!(
            out.get("warning").is_none(),
            "an unexamined pair is a gap, not a wrong classification: {out}"
        );
    }
}
