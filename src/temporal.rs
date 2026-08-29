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
//!       t:recordedAt "2024-01-05T09:00:00Z"^^xsd:dateTime ;
//!       t:recordedUntil "2026-05-02T09:00:00Z"^^xsd:dateTime .
//!   :g2 t:validFrom "2026-05-01"^^xsd:date ;
//!       t:recordedAt "2026-05-02T09:00:00Z"^^xsd:dateTime .
//! }
//! ```
//!
//! An absent `validFrom` means "since always", an absent `validTo` means
//! "still true", an absent `recordedUntil` means "still believed", and a graph
//! with no temporal description at all is timeless: it is in scope for every
//! snapshot, so adding this vocabulary to an existing store changes nothing
//! until it is used.
//!
//! Intervals are half-open on both axes, `[validFrom, validTo)` and
//! `[recordedAt, recordedUntil)`. Two facts that meet at a boundary do not
//! overlap, which is what makes "adherent until May, suspension from May" a
//! correction rather than a contradiction.
//!
//! Closing the recorded interval is what lets `as_of` answer "what did we
//! believe THEN" instead of "everything we had ever recorded by then". With no
//! upper bound the transaction axis only narrows forward, so a corrected
//! assertion keeps turning up in every later snapshot beside the correction
//! that replaced it, and the audit question this module exists to answer stops
//! being answerable after the first correction.
//!
//! ## Lineage is asserted, never inferred
//!
//! Two graphs claiming the same period can be a correction (one authority
//! replaced its own assertion), a disagreement (two sources) or a retraction
//! (withdrawn, nothing put in its place), and nothing in the periods tells
//! the three apart. Two predicates carry the link, both written on the NEWER
//! graph: `temporal:supersedes` names the graph it replaces, and
//! `temporal:retracts` names the graph it withdraws without replacing. They
//! answer different questions from `recordedUntil` and do not compete with
//! it: `recordedUntil` alone governs scope, and an explicit bound is
//! authoritative. Where a graph carries none, the bound is DERIVED from the
//! link: belief in it ended at the `recordedAt` of the graph that supersedes
//! it, the earliest one where there are several. Where both are present and
//! disagree, the explicit value governs and the disagreement is reported
//! under `lineage`, never reconciled. Nothing is written: the derivation is a
//! join made when the validity map is read, so the snapshot, the query and
//! the conflict check share it by construction.
//!
//! A retracted graph stays visible, in its own `retracted` bucket, from the
//! instant the retraction was recorded. With no `as_of` the recorded axis is
//! not consulted, for a retraction no more than for a `recordedUntil`, and
//! the graph takes the ordinary path: one rule for every recorded-time fact,
//! a closing bound asserted or derived and a withdrawal alike. In
//! `conflicts` a disjointness
//! pair where one graph supersedes the other, directly or through a chain, is
//! a correction whatever its periods, and lands in `corrections` rather than
//! `contradictions`. A successor recorded before its predecessor produces a
//! transaction interval that closes before it opens: it is reported as
//! inverted and believed at no instant, never clamped.
//!
//! ## Bounds are instants
//!
//! Every bound is read as an instant on the UTC timeline, from `xsd:date`,
//! `xsd:dateTime`, `xsd:gYearMonth` or `xsd:gYear`. A less precise bound names
//! the FIRST instant of the period it names, so `"2026-05-01"^^xsd:date` as a
//! `validTo` excludes the whole of 1 May, and `"2026"^^xsd:gYear` as a
//! `validFrom` starts at midnight on 1 January. A value with no timezone
//! offset is UTC: XSD leaves such a value only partially ordered against one
//! carrying an offset, and "indeterminate" is not an answer a register query
//! can return.
//!
//! A bound that matches none of those grammars is not coerced into one, and a
//! graph asserting two different instants on the same axis is not resolved to
//! either of them: both make the graph INVALID. An invalid graph is reported
//! with its reason and is never in scope; above all it is never timeless,
//! because "we hold no valid-time claim about this" and "the claim is garbage"
//! are different answers.
//!
//! Two readable bounds can still name no instant: `validFrom` and `validTo`
//! naming the same instant is an empty period, `validTo` before `validFrom`
//! an inverted one. Neither is unreadable, and neither is a period something
//! can overlap. The classification is made once, when the bounds are read,
//! and every comparison answers from it, so a graph the snapshot excludes at
//! every instant asked about is never a contradiction partner in `conflicts`.
//! It is excluded with a reason naming which of the two it is: an empty period
//! is sometimes intended, and an inverted one almost never is, and the text is
//! the only place that distinction can survive. With no `valid_at` it is read
//! like any other graph, because no instant was asked about.
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
use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub const NS: &str = "https://open-ontologies.org/temporal#";

/// Which comparison semantics produced an answer.
///
/// `temporal/1` was lexical string comparison of the bounds as written.
/// `temporal/2` reads every bound as an instant on the UTC timeline, and a
/// period whose two bounds name no instant holds nowhere under it, in scope
/// at no instant and overlapping nothing, where it used to overlap. The same
/// store answers differently under the two, so anything that replays or
/// records an answer, a snapshot manifest above all, has to carry this, or
/// two answers that cannot both be right hash identically.
///
/// The lineage predicates, `supersedes` and `retracts`, are read under
/// `temporal/2` as well: they ship in the same release as the parsed bounds
/// and a store that does not write them answers exactly as it did without
/// them, so they do not move the version on their own.
pub const SEMANTICS_VERSION: &str = "temporal/2";

pub struct Temporal {
    graph: Arc<GraphStore>,
    limits: Limits,
}

/// Validity ROWS, not graphs. The query is a six-way UNION, so a graph
/// carrying validFrom, validTo, recordedAt and recordedUntil costs four rows,
/// and one that also asserts a `supersedes` link and a `retracts` link costs
/// six: the cap is reached at roughly 5,000 fully bounded graphs, at roughly
/// 4,000 where each also carries one lineage link, and at roughly 3,300 where
/// each carries both, where a three-predicate store still reaches it at
/// roughly 6,700. A store that never writes recordedUntil or a lineage link
/// pays nothing for them; the cap counts rows, not graphs.
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
    /// Graphs whose validity metadata could not be read. In neither of the
    /// two buckets above: an unreadable claim is not a claim we do not hold.
    invalid: Vec<serde_json::Value>,
    /// Graphs withdrawn by a `retracts` link recorded by the audit instant.
    /// Not `excluded`: a retracted claim did not fail a bound, it was taken
    /// back, and the row names the graph that took it back.
    retracted: Vec<serde_json::Value>,
    /// What the lineage links could not settle or settled with a remark:
    /// an explicit bound that disagrees with its successor, a transaction
    /// interval that closes before it opens, a link naming nothing readable.
    lineage: Vec<serde_json::Value>,
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
                 period excludes them, and one whose metadata is unreadable is published as \
                 timeless instead of invalid; a supersedes or retracts row that fell past the \
                 limit while its asserter's bounds did not was never seen, so the graph it \
                 named stays open, never closed, or in scope, never withdrawn, although it is \
                 described",
            ),
            self.graph_scan.report(
                "all_graphs",
                "graphs past the limit are missing from in_scope, excluded and invalid alike",
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
                 opposite of the truth for any graph whose period had ended, unreadable \
                 ones were published as timeless rather than invalid, and a supersedes or \
                 retracts row that fell past the limit was never seen, leaving the graph it \
                 named open or in scope although it is described",
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

#[derive(Clone, Debug, Default)]
pub struct Validity {
    pub graph: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub recorded_at: Option<String>,
    pub recorded_until: Option<String>,
}

impl Validity {
    /// The ASSERTED lexical forms, not the instants they parse to: an answer
    /// shows what the store says. What `"2024"` means as a bound is a rule,
    /// and a rule belongs in the tool description, not on every row.
    fn describe(&self) -> String {
        let from = self.valid_from.as_deref().unwrap_or("always");
        let to = self.valid_to.as_deref().unwrap_or("still true");
        format!("{from} to {to}")
    }
}

/// One graph's validity, read as instants on the UTC timeline.
///
/// The asserted lexical forms travel alongside so an answer keeps showing
/// what the store says, while every comparison runs on the parsed instant.
#[derive(Clone, Debug, Default)]
struct Period {
    shown: Validity,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    recorded: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
    /// `Some` when the valid interval holds at no instant. Set once, where
    /// the bounds are read, and consulted by every comparison: `overlaps`
    /// compares two periods without seeing the `GraphValidity` each came
    /// from, so the verdict has to travel on the period itself.
    degenerate: Option<Degenerate>,
    /// The links this graph asserts, and what the lineage pass derived from
    /// the links that name it. Carried on the period so `scope`, `query_at`
    /// and `conflicts` read one derivation rather than three.
    lineage: Lineage,
}

/// The lineage a graph asserts, and what was derived from the lineage
/// asserted about it. Asserted, never inferred: every field here traces to a
/// `supersedes` or `retracts` triple somebody wrote.
#[derive(Clone, Debug, Default)]
struct Lineage {
    /// The graphs this one asserts it replaces, as plain IRIs. As read, every
    /// asserted target; once `derive_lineage` has run, the EFFECTIVE ones
    /// only, a link it rejected surviving in its report row and nowhere
    /// else, so no consumer can walk a link the report says had no effect.
    supersedes: Vec<String>,
    /// The graphs this one asserts it withdraws, as plain IRIs, pruned to
    /// the effective ones by the same pass.
    retracts: Vec<String>,
    /// Terms on either predicate that are not IRIs, with the predicate each
    /// sat on. Kept so the report can quote them; nothing else reads them.
    not_iris: Vec<(&'static str, String)>,
    /// The successor whose `recordedAt` closed this graph's transaction
    /// interval, when the closing bound was derived rather than asserted.
    /// `None` for an asserted bound and for an open interval alike.
    until_derived_from: Option<String>,
    /// The graphs that retract this one, earliest recorded first and the
    /// undated ones last, so the first that qualifies at an instant is the
    /// one to name.
    retracted_by: Vec<Retractor>,
}

/// One graph that withdraws another, and when the withdrawal was recorded.
#[derive(Clone, Debug)]
struct Retractor {
    graph: String,
    recorded: Option<DateTime<Utc>>,
    /// The asserted lexical form, for the row; `None` when the retracting
    /// graph carries no `recordedAt`.
    recorded_lexical: Option<String>,
}

/// On which side of the recorded interval an instant fell, when it fell
/// outside. Three variants rather than a bool: "recorded later" and "no
/// longer believed" are opposite facts about an assertion, and an interval
/// that closes before it opens is a third, a fact about the data rather
/// than about the instant asked about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecordedMiss {
    NotYet,
    NoLonger,
    /// `recordedUntil` precedes `recordedAt`, asserted or derived, so the
    /// interval holds at no instant.
    Nowhere,
}

impl RecordedMiss {
    fn reason(self) -> &'static str {
        match self {
            RecordedMiss::NotYet => "not yet recorded then",
            RecordedMiss::NoLonger => "no longer recorded then",
            RecordedMiss::Nowhere => {
                "believed at no instant: the transaction interval closes before it opens"
            }
        }
    }

    /// Whether the miss is on the closing side, where a derived bound is the
    /// thing that closed it and the row should name the successor.
    fn closed(self) -> bool {
        !matches!(self, RecordedMiss::NotYet)
    }
}

/// Why a period with two readable bounds holds at no instant.
///
/// Two variants rather than one, because the two are different facts about
/// the data: an empty period is sometimes written on purpose, an inverted one
/// almost never is, and the reason text is the only place that survives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Degenerate {
    /// `validFrom` and `validTo` name the same instant: `[t, t)` is empty.
    Empty,
    /// `validTo` names an instant before `validFrom`.
    Inverted,
}

impl Degenerate {
    /// The exclusion reason. Deliberately not the generic "not true at that
    /// instant": that is accurate for a period that holds nowhere and useless,
    /// because it reads as a period that holds somewhere else.
    fn reason(self) -> &'static str {
        match self {
            Degenerate::Empty => {
                "holds at no instant: validFrom and validTo name the same instant, an empty period"
            }
            Degenerate::Inverted => {
                "holds at no instant: validTo precedes validFrom, an inverted period"
            }
        }
    }
}

impl Period {
    /// Was this true at `instant`, on the half-open interval.
    fn valid_at(&self, instant: DateTime<Utc>) -> bool {
        self.from.is_none_or(|f| f <= instant) && self.to.is_none_or(|t| instant < t)
    }

    /// Was this believed at `instant`, on the half-open recorded interval
    /// `[recordedAt, recordedUntil)`, and if not, on which side it fell.
    ///
    /// This is the two-sided form of what used to be `recorded_by`. Adding the
    /// closing bound gives the predicate a second way to fail, and the two are
    /// opposite facts about the assertion: one has not been recorded yet, the
    /// other is no longer believed. A bool would flatten them back together at
    /// the only place that has to tell them apart, so it returns the reason.
    /// Both bounds are instants like the valid-time pair: a closing bound
    /// written with an offset or at coarse precision closes the interval where
    /// that instant falls, not where its text sorts.
    ///
    /// An interval that closes before it opens is asked first. The two checks
    /// below already fail every instant for it, so the answer would be right
    /// either way, and wrong to read: "not yet recorded then" for a graph
    /// that was never believed at any instant sends the reader looking for
    /// a later instant that does not exist.
    fn not_recorded_at(&self, instant: DateTime<Utc>) -> Option<RecordedMiss> {
        if self.inverted_recording() {
            return Some(RecordedMiss::Nowhere);
        }
        if self.recorded.is_some_and(|r| instant < r) {
            return Some(RecordedMiss::NotYet);
        }
        if self.until.is_some_and(|u| u <= instant) {
            return Some(RecordedMiss::NoLonger);
        }
        None
    }

    /// Whether the transaction interval closes before it opens. Judged on the
    /// composed bound, so a derived `until` counts exactly as an asserted one.
    fn inverted_recording(&self) -> bool {
        match (self.recorded, self.until) {
            (Some(r), Some(u)) => u < r,
            _ => false,
        }
    }

    /// The retraction that stands at `as_of`, if any: the earliest one
    /// recorded by then, or an undated one, which stands at every instant
    /// asked about. With no `as_of` there is none: no instant was asked about
    /// on the recorded axis, so no recorded-time fact is consulted, a
    /// retraction no more than a `recordedUntil`, asserted or derived. One
    /// rule for every fact on that axis; reading an absent `as_of` as "now"
    /// would have to change the explicit bound too.
    fn retracted_as_of(&self, as_of: Option<DateTime<Utc>>) -> Option<&Retractor> {
        let t = as_of?;
        self.lineage
            .retracted_by
            .iter()
            .find(|r| r.recorded.is_none_or(|at| at <= t))
    }

    /// Do two validity periods share any instant. Half-open, so touching
    /// intervals do not overlap, and a period that holds at no instant shares
    /// none with anything: the bound test below would answer true for `[t, t)`
    /// against any period containing `t`, and unconditionally against an open
    /// one, while `valid_at` is false everywhere for it. The classification is
    /// made when the bounds are read: `scope` answers from the variant, this
    /// answers from the flag the period carries, and `valid_at` is never asked
    /// about a period that holds nowhere, so the two cannot disagree.
    fn overlaps(&self, other: &Period) -> bool {
        if self.degenerate.is_some() || other.degenerate.is_some() {
            return false;
        }
        let start_before_end = |a: Option<DateTime<Utc>>, b: Option<DateTime<Utc>>| match (a, b) {
            (Some(start), Some(end)) => start < end,
            _ => true, // an open end never closes the interval
        };
        start_before_end(self.from, other.to) && start_before_end(other.from, self.to)
    }
}

/// A graph's validity metadata as the tools read it.
///
/// An enum rather than a struct with a flag, because every consumer has to
/// answer the question: a graph whose metadata cannot be read must never be
/// treated as timeless, and a `None` that silently means "open period" is
/// exactly how that happens.
#[derive(Clone, Debug)]
enum GraphValidity {
    /// Every bound the graph asserts was readable, and the period holds
    /// somewhere.
    Sound(Period),
    /// Every bound was readable, and together the valid bounds name no
    /// instant. Not `Unreadable`: every reason in that bucket says a bound
    /// could not be read, and a readable claim there would give the bucket a
    /// second meaning. The period is kept so a row still shows the asserted
    /// bounds; `kind` says which of the two shapes it is.
    Degenerate { period: Period, kind: Degenerate },
    /// At least one bound was not. "We hold no valid-time claim about this"
    /// and "the claim is garbage" are different answers, and only the first
    /// one is timeless. The links the graph asserted travel with the faults,
    /// as `(predicate, target)` pairs, so the lineage pass can report that
    /// each had no effect: a link dropped with its asserter would leave the
    /// graph it names open with nothing saying why.
    Unreadable {
        faults: Vec<Fault>,
        asserted: Vec<(&'static str, String)>,
    },
}

/// Why one bound could not be read.
#[derive(Clone, Debug)]
struct Fault {
    field: &'static str,
    reason: String,
}

impl GraphValidity {
    /// One sentence naming every bound that could not be read.
    fn fault_reason(faults: &[Fault]) -> String {
        faults
            .iter()
            .map(|f| format!("{}: {}", f.field, f.reason))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// One readable bound: the instant every comparison uses, and the lexical
/// form the answer shows.
struct Bound {
    lexical: String,
    instant: DateTime<Utc>,
}

/// The raw terms one graph asserts on each axis, before they are read, and
/// the raw lineage terms beside them.
#[derive(Default)]
struct Bounds {
    from: Vec<String>,
    to: Vec<String>,
    recorded: Vec<String>,
    until: Vec<String>,
    supersedes: Vec<String>,
    retracts: Vec<String>,
}

/// Split one predicate's raw terms into the graph IRIs it names and the
/// terms that name no graph. A literal on `supersedes` is a data error, and
/// one that was silently dropped would leave a predecessor open for ever,
/// which is the defect the predicate exists to close.
fn links(
    field: &'static str,
    terms: Vec<String>,
    not_iris: &mut Vec<(&'static str, String)>,
) -> Vec<String> {
    let mut iris = Vec::new();
    for term in terms {
        let t = term.trim();
        if t.starts_with('<') && t.ends_with('>') {
            iris.push(plain(t));
        } else {
            not_iris.push((field, term));
        }
    }
    iris
}

impl Bounds {
    /// Read all four axes, and refuse the graph if any one of them cannot be
    /// read. `recordedUntil` is settled by the same rule as the other three:
    /// it is a bound, and a bound that stayed a string beside three parsed
    /// ones would close the interval where its text sorts rather than where
    /// its instant falls.
    ///
    /// Then classify the valid interval, here and nowhere else. Two readable
    /// bounds that name no instant between them make the graph `Degenerate`,
    /// and every consumer answers from that verdict rather than rediscovering
    /// it: a guard inside `overlaps` would be the same `None`-means-open
    /// mistake the enum exists to prevent, one precision down. The recorded
    /// axis is NOT classified this way: `recordedUntil` before `recordedAt`
    /// is out-of-order recording (#109), which is a different question, and
    /// the lineage pass reports it once the derived bound is known.
    ///
    /// The lineage terms ride on the period as plain IRIs, for the sound and
    /// the degenerate verdict alike. An unreadable graph keeps its faults and
    /// the links it asserted, as raw pairs: it is invalid already, and a link
    /// asserted by a graph whose own description cannot be read closes
    /// nothing, but the lineage pass reports it rather than losing it.
    fn resolve(self, graph: String) -> GraphValidity {
        let mut faults = Vec::new();
        let from = settle("validFrom", self.from, &mut faults);
        let to = settle("validTo", self.to, &mut faults);
        let recorded = settle("recordedAt", self.recorded, &mut faults);
        let until = settle("recordedUntil", self.until, &mut faults);
        let mut not_iris = Vec::new();
        let supersedes = links("supersedes", self.supersedes, &mut not_iris);
        let retracts = links("retracts", self.retracts, &mut not_iris);
        if !faults.is_empty() {
            let asserted = supersedes
                .into_iter()
                .map(|t| ("supersedes", t))
                .chain(retracts.into_iter().map(|t| ("retracts", t)))
                .chain(not_iris)
                .collect();
            return GraphValidity::Unreadable { faults, asserted };
        }
        let lineage = Lineage {
            supersedes,
            retracts,
            not_iris,
            ..Lineage::default()
        };
        let degenerate = match (&from, &to) {
            (Some(f), Some(t)) => match f.instant.cmp(&t.instant) {
                Ordering::Equal => Some(Degenerate::Empty),
                Ordering::Greater => Some(Degenerate::Inverted),
                Ordering::Less => None,
            },
            _ => None, // an open end always leaves instants inside
        };
        let period = Period {
            shown: Validity {
                graph,
                valid_from: from.as_ref().map(|b| b.lexical.clone()),
                valid_to: to.as_ref().map(|b| b.lexical.clone()),
                recorded_at: recorded.as_ref().map(|b| b.lexical.clone()),
                recorded_until: until.as_ref().map(|b| b.lexical.clone()),
            },
            from: from.map(|b| b.instant),
            to: to.map(|b| b.instant),
            recorded: recorded.map(|b| b.instant),
            until: until.map(|b| b.instant),
            degenerate,
            lineage,
        };
        match degenerate {
            Some(kind) => GraphValidity::Degenerate { period, kind },
            None => GraphValidity::Sound(period),
        }
    }
}

/// Read every term asserted on one axis, and insist they agree.
///
/// Agreement is judged on the INSTANTS the terms name, not on their lexical
/// forms. Two terms denoting the SAME instant are two spellings of one
/// assertion (a bare `"2024-01-01"` beside `"2024-01-01"^^xsd:date` is what a
/// half-finished migration leaves behind) and they invent no interval, so they
/// resolve. So do a coarse bound and a fine one naming the same instant:
/// `"2024"^^xsd:gYear` beside `"2024-01-01"^^xsd:date` is one assertion,
/// because both name midnight on 1 January. Two terms denoting DIFFERENT
/// instants resolve to nothing: picking one, or the min, or the max, would
/// invent an interval nobody asserted. The form shown is the lexically first
/// term after the sort; a row shows one form per bound.
fn settle(field: &'static str, mut terms: Vec<String>, faults: &mut Vec<Fault>) -> Option<Bound> {
    terms.sort(); // the query contracts no row order, so neither does the answer
    let mut read: Vec<(String, DateTime<Utc>)> = Vec::new();
    for term in &terms {
        match instant::bound(term) {
            Ok(instant) => read.push((plain(term), instant)),
            Err(reason) => {
                faults.push(Fault { field, reason });
                return None;
            }
        }
    }
    let (lexical, instant) = read.first()?.clone();
    let mut distinct: Vec<DateTime<Utc>> = read.iter().map(|(_, i)| *i).collect();
    distinct.sort_unstable();
    distinct.dedup();
    if distinct.len() > 1 {
        faults.push(Fault {
            field,
            reason: format!(
                "{} distinct instants asserted ({}); which interval was meant is not \
                 recoverable, and choosing one would invent an interval nobody asserted",
                distinct.len(),
                terms.join(", ")
            ),
        });
        return None;
    }
    Some(Bound { lexical, instant })
}

/// Reading a temporal bound as an instant on the UTC timeline.
///
/// XSD leaves a timezone-less value only partially ordered against one that
/// carries an offset. "Indeterminate" is not an answer a register query can
/// return, so an absent offset is read as UTC and the order is total; the
/// tool descriptions say so. A less precise bound names the FIRST instant of
/// the period it names, which is what lexical comparison already gave for
/// same-precision data, so a well-formed 1.2.0 dataset answers as it did.
mod instant {
    use chrono::{DateTime, NaiveDate, TimeDelta, Utc};

    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Form {
        DateTime,
        Date,
        GYearMonth,
        GYear,
    }

    /// The four grammars are pairwise disjoint, so at most one can match and
    /// the order here is only a short-circuit.
    const FORMS: [Form; 4] = [Form::DateTime, Form::Date, Form::GYearMonth, Form::GYear];

    fn name(form: Form) -> &'static str {
        match form {
            Form::DateTime => "xsd:dateTime",
            Form::Date => "xsd:date",
            Form::GYearMonth => "xsd:gYearMonth",
            Form::GYear => "xsd:gYear",
        }
    }

    fn form_of(datatype: &str) -> Option<Form> {
        match datatype.strip_prefix(XSD)? {
            "date" => Some(Form::Date),
            "dateTime" => Some(Form::DateTime),
            "gYearMonth" => Some(Form::GYearMonth),
            "gYear" => Some(Form::GYear),
            _ => None,
        }
    }

    /// A term as `sparql_select` renders it, split into lexical form and
    /// datatype IRI. Nothing in the four grammars contains a character
    /// N-Triples escapes, so an escaped lexical form fails the grammar rather
    /// than needing to be unescaped.
    fn split(term: &str) -> Result<(&str, Option<&str>), String> {
        let trimmed = term.trim();
        let Some(body) = trimmed.strip_prefix('"') else {
            return Err(format!("{trimmed} is not a literal"));
        };
        if let Some(rest) = body.strip_suffix('>') {
            let Some((lex, datatype)) = rest.rsplit_once("\"^^<") else {
                return Err(format!("{trimmed} is not a literal"));
            };
            return Ok((lex, Some(datatype)));
        }
        if let Some((_, tag)) = body.rsplit_once("\"@") {
            return Err(format!(
                "a language-tagged literal (@{tag}) is not a temporal bound"
            ));
        }
        let Some(lex) = body.strip_suffix('"') else {
            return Err(format!("{trimmed} is not a literal"));
        };
        Ok((lex, None))
    }

    /// A bound as the store holds it.
    ///
    /// RDF 1.1 makes a simple literal and an `xsd:string`-typed literal with
    /// the same lexical form THE SAME TERM (`datatype()` answers `xsd:string`
    /// for both), so a bound cannot be rejected for "carrying" `xsd:string`:
    /// the store holds no such fact to report. An untyped bound is therefore
    /// read by SHAPE against all four grammars. `"01/05/2026"` matches none
    /// and is rejected, by shape rather than by datatype, but rejected.
    pub(super) fn bound(term: &str) -> Result<DateTime<Utc>, String> {
        let (lex, datatype) = split(term)?;
        let lex = lex.trim(); // XSD whiteSpace on these types is `collapse`
        match datatype {
            Some(iri) => match form_of(iri) {
                // A declared temporal datatype whose value does not match it
                // is mislabelled rather than unreadable: the crate's own
                // module doc shipped `"2024-01-05"^^xsd:dateTime`, and reading
                // it as the date it plainly is keeps every answer such a store
                // already gave. The re-read stays inside the four grammars, so
                // `"01/05/2026"` is still rejected whatever it claims to be.
                Some(form) => at(lex, form).map(Ok).unwrap_or_else(|| {
                    shape(lex).map_err(|_| {
                        format!(
                            "\"{lex}\" is not a valid {} and matches no other temporal type",
                            name(form)
                        )
                    })
                }),
                None => Err(format!(
                    "datatype <{iri}> is outside xsd:date, xsd:dateTime, xsd:gYearMonth and xsd:gYear"
                )),
            },
            None => shape(lex),
        }
    }

    /// A tool argument. It arrives as text rather than as an RDF term, so
    /// there is no datatype to consult, and it goes through the same
    /// grammars a bound does, so an argument and a bound can never disagree
    /// about what an instant is.
    pub(super) fn argument(text: &str) -> Result<DateTime<Utc>, String> {
        shape(text.trim())
    }

    fn shape(lex: &str) -> Result<DateTime<Utc>, String> {
        FORMS.iter().find_map(|&form| at(lex, form)).ok_or_else(|| {
            format!(
                "\"{lex}\" is not an xsd:date, xsd:dateTime, xsd:gYearMonth or xsd:gYear instant"
            )
        })
    }

    /// The first instant of the period `lex` names, on the UTC timeline.
    /// `None` when the lexical form does not match the grammar, when the
    /// fields are not a real date, or when the year is not representable.
    fn at(lex: &str, form: Form) -> Option<DateTime<Utc>> {
        let b = lex.as_bytes();
        let (year, mut i) = year_frag(b)?;
        let (mut month, mut day) = (1u32, 1u32);
        let (mut hour, mut minute, mut second, mut nano) = (0u32, 0u32, 0u32, 0u32);
        let mut next_day = false;

        if !matches!(form, Form::GYear) {
            i = byte(b, i, b'-')?;
            let (m, j) = two(b, i, 1, 12)?;
            month = m;
            i = j;
        }
        if matches!(form, Form::Date | Form::DateTime) {
            i = byte(b, i, b'-')?;
            let (d, j) = two(b, i, 1, 31)?; // from_ymd_opt rejects 30 February
            day = d;
            i = j;
        }
        if matches!(form, Form::DateTime) {
            i = byte(b, i, b'T')?;
            // XSD's end-of-day form: 24:00:00 is the first instant of the next
            // day. The store canonicalises it away, but an argument can carry
            // it, and an argument must not be refused where a bound would not.
            if b.get(i..).is_some_and(|rest| rest.starts_with(b"24:00:00")) {
                next_day = true;
                i = zero_fraction(b, i + 8)?;
            } else {
                let (h, j) = two(b, i, 0, 23)?;
                hour = h;
                i = byte(b, j, b':')?;
                let (m, j) = two(b, i, 0, 59)?;
                minute = m;
                i = byte(b, j, b':')?;
                let (s, j) = two(b, i, 0, 59)?; // :60 is not in the XSD lexical space
                second = s;
                let (n, j) = fraction(b, j)?;
                nano = n;
                i = j;
            }
        }
        let offset = tz(b, i)?; // 0 when absent: no offset means UTC
        let date = NaiveDate::from_ymd_opt(year, month, day)?;
        let date = if next_day { date.succ_opt()? } else { date };
        date.and_hms_nano_opt(hour, minute, second, nano)?
            .and_utc()
            .checked_sub_signed(TimeDelta::try_seconds(i64::from(offset))?)
    }

    /// XSD yearFrag: `'-'? ( [1-9][0-9]{3,} | '0'[0-9]{3} )`. No leading `+`,
    /// which is why `"+2024"` is not a gYear, and is not one to the store
    /// either, which passes it through unchanged.
    fn year_frag(b: &[u8]) -> Option<(i32, usize)> {
        let negative = b.first() == Some(&b'-');
        let start = usize::from(negative);
        let mut i = start;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        let digits = i - start;
        if !(4..=9).contains(&digits) {
            return None; // 9 keeps the parse inside i32 before from_ymd_opt sees it
        }
        if b[start] == b'0' && digits != 4 {
            return None;
        }
        let magnitude: i64 = std::str::from_utf8(b.get(start..i)?).ok()?.parse().ok()?;
        if negative && magnitude == 0 {
            return None; // XSD 1.1 forbids -0000
        }
        let year = if negative { -magnitude } else { magnitude };
        Some((i32::try_from(year).ok()?, i)) // from_ymd_opt caps the range
    }

    /// XSD allows unbounded fractional digits; chrono holds nanoseconds. A
    /// value carrying more PRECISION than that is REFUSED rather than
    /// truncated. Truncating would silently merge two instants a well-formed
    /// store distinguishes, and these intervals are half-open, so the instants
    /// that merge are exactly the ones a boundary test is asking about.
    ///
    /// Digits past the ninth are only extra precision when one of them is
    /// NONZERO. A zero tail adds nothing: `.1234567890` is 123,456,789
    /// nanoseconds exactly, the same instant `.123456789` names, and refusing
    /// it would contradict `settle`, which resolves two spellings of one
    /// instant precisely because they invent no interval. Fixed-width
    /// formatters pad to a fixed digit count, so the tail is common in stores
    /// nobody wrote by hand.
    fn fraction(b: &[u8], i: usize) -> Option<(u32, usize)> {
        if b.get(i) != Some(&b'.') {
            return Some((0, i));
        }
        let (mut j, mut nano, mut scale) = (i + 1, 0u32, 100_000_000u32);
        while let Some(d) = b.get(j).and_then(|&c| char::from(c).to_digit(10)) {
            if scale == 0 {
                if d != 0 {
                    return None; // more precision asserted than can be compared
                }
            } else {
                nano += d * scale;
                scale /= 10;
            }
            j += 1;
        }
        if j == i + 1 {
            return None; // a lone '.'
        }
        Some((nano, j))
    }

    /// The `('.' '0'+)?` XSD allows after `24:00:00`.
    fn zero_fraction(b: &[u8], i: usize) -> Option<usize> {
        let (nano, j) = fraction(b, i)?;
        (nano == 0).then_some(j)
    }

    /// A trailing `Z` or `±hh:mm`, in seconds east of UTC. Absent means 0.
    fn tz(b: &[u8], i: usize) -> Option<i32> {
        let rest = b.get(i..)?;
        if rest.is_empty() || rest == b"Z" {
            return Some(0);
        }
        if rest.len() != 6 || rest[3] != b':' {
            return None;
        }
        let sign = match rest[0] {
            b'+' => 1,
            b'-' => -1,
            _ => return None,
        };
        let hours = two_digits(&rest[1..3])?;
        let minutes = two_digits(&rest[4..6])?;
        if minutes > 59 {
            return None;
        }
        let total = i32::try_from(hours * 60 + minutes).ok()?;
        if total > 14 * 60 {
            return None; // XSD caps the offset at ±14:00
        }
        Some(sign * total * 60)
    }

    fn byte(b: &[u8], i: usize, want: u8) -> Option<usize> {
        (b.get(i) == Some(&want)).then_some(i + 1)
    }

    fn two(b: &[u8], i: usize, lo: u32, hi: u32) -> Option<(u32, usize)> {
        let value = two_digits(b.get(i..i + 2)?)?;
        (lo..=hi).contains(&value).then_some((value, i + 2))
    }

    fn two_digits(pair: &[u8]) -> Option<u32> {
        let tens = char::from(*pair.first()?).to_digit(10)?;
        let units = char::from(*pair.get(1)?).to_digit(10)?;
        Some(tens * 10 + units)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The instant a lexical form names, as `YYYY-MM-DDTHH:MM:SSZ`, or the
        /// word "invalid". One row per rule, so a rule that changes moves one
        /// line rather than a paragraph of prose.
        fn read(text: &str) -> String {
            match argument(text) {
                Ok(i) => i.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
                Err(_) => "invalid".to_string(),
            }
        }

        #[test]
        fn a_less_precise_form_names_the_first_instant_of_its_period() {
            for (lexical, instant) in [
                ("2024", "2024-01-01T00:00:00Z"),
                ("2024-03", "2024-03-01T00:00:00Z"),
                ("2024-03-05", "2024-03-05T00:00:00Z"),
                ("2024-03-05T06:07:08", "2024-03-05T06:07:08Z"),
                ("2024-03-05T06:07:08.25", "2024-03-05T06:07:08.250Z"),
            ] {
                assert_eq!(read(lexical), instant, "{lexical}");
            }
        }

        #[test]
        fn an_absent_offset_is_utc_and_a_present_one_is_applied() {
            for (lexical, instant) in [
                ("2024-03-05T06:00:00", "2024-03-05T06:00:00Z"),
                ("2024-03-05T06:00:00Z", "2024-03-05T06:00:00Z"),
                ("2024-03-05T06:00:00+02:00", "2024-03-05T04:00:00Z"),
                ("2024-03-05T06:00:00-05:00", "2024-03-05T11:00:00Z"),
                // The offset applies to the coarser forms too, and can move
                // the instant into the previous day, month or year.
                ("2024-03-05+02:00", "2024-03-04T22:00:00Z"),
                ("2024-01+02:00", "2023-12-31T22:00:00Z"),
                ("2024+02:00", "2023-12-31T22:00:00Z"),
                // XSD's end-of-day form: the first instant of the next day.
                ("2024-03-05T24:00:00Z", "2024-03-06T00:00:00Z"),
            ] {
                assert_eq!(read(lexical), instant, "{lexical}");
            }
        }

        #[test]
        fn everything_outside_the_four_grammars_is_refused() {
            for lexical in [
                "01/05/2026",                // the shape a spreadsheet exports
                "2024-1-1",                  // XSD requires two digits
                "20240101",                  // and the separators
                "2024-13-01",                // month out of range
                "2024-02-30",                // parses field by field, never happened
                "2024-03-05T25:00:00Z",      // hour out of range
                "2024-03-05T06:07",          // xsd:dateTime requires seconds
                "2024-03-05T06:07:60Z",      // leap seconds are not in the lexical space
                "2024-03-05T06:00:00+00:99", // minutes out of range
                "2024-03-05T06:00:00+15:00", // XSD caps the offset at 14:00
                "+2024",                     // yearFrag has no leading plus
                "-0000",                     // XSD 1.1 forbids negative zero
                "024",                       // fewer than four digits
                "02024",                     // a leading zero is only for four
                "",
                "   ",
                "tomorrow",
                // Finer than a nanosecond: refused rather than truncated, so
                // two instants a store distinguishes never silently merge.
                "2024-03-05T06:00:00.0000000001Z",
            ] {
                assert_eq!(read(lexical), "invalid", "{lexical}");
            }
        }

        #[test]
        fn a_datatype_outside_the_four_is_refused_whatever_its_value_looks_like() {
            let date = "\"2024-03-05\"^^<http://www.w3.org/2001/XMLSchema#date>";
            assert!(bound(date).is_ok());
            for term in [
                "\"2024-03-05\"^^<http://www.w3.org/2001/XMLSchema#token>",
                "\"2024-03-05\"^^<http://example.org/JulianDay>",
                "\"2024-03-05\"@en",
                "<http://example.org/not-a-literal>",
            ] {
                assert!(bound(term).is_err(), "{term}");
            }
        }

        /// The crate's own module doc shipped `"2024-01-05"^^xsd:dateTime`,
        /// which is a date wearing a dateTime datatype. Reading it as the date
        /// it plainly is keeps every answer such a store already gave; the
        /// re-read stays inside the four temporal grammars, so a value fitting
        /// none of them is still refused whatever it claims to be.
        #[test]
        fn a_value_mislabelled_as_another_temporal_type_is_re_read_by_shape() {
            let mislabelled = "\"2024-01-05\"^^<http://www.w3.org/2001/XMLSchema#dateTime>";
            assert_eq!(
                bound(mislabelled)
                    .unwrap()
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
                "2024-01-05T00:00:00Z"
            );
            let garbage = "\"01/05/2026\"^^<http://www.w3.org/2001/XMLSchema#dateTime>";
            assert!(bound(garbage).is_err());
        }

        /// XSD's whiteSpace facet on these types is `collapse`, and a store
        /// that keeps the padding must not answer differently from one that
        /// does not, on either path.
        #[test]
        fn padding_is_collapsed_on_both_the_typed_and_the_bare_path() {
            assert_eq!(read("  2024-03-05  "), "2024-03-05T00:00:00Z");
            let padded = "\"  2024-03-05  \"^^<http://www.w3.org/2001/XMLSchema#date>";
            assert!(bound(padded).is_ok());
        }

        /// A tenth fractional digit is only unrepresentable when it carries a
        /// value. `.1234567890` is 123,456,789 nanoseconds exactly, the same
        /// instant `.123456789` names, so refusing it made two spellings of
        /// one instant answer differently, which is the thing `settle` exists
        /// to prevent. Fixed-width formatters pad to a fixed digit count, so
        /// the zero tail arrives from machines, not from typos.
        #[test]
        fn a_zero_tail_past_nanoseconds_is_the_same_instant_not_more_precision() {
            let nine = read("2024-03-05T06:07:08.123456789Z");
            assert_eq!(read("2024-03-05T06:07:08.1234567890Z"), nine);
            assert_eq!(read("2024-03-05T06:07:08.12345678900000Z"), nine);
            // The zero tail must not disturb a value that is entirely zeroes,
            // nor the `('.' '0'+)?` XSD allows after 24:00:00.
            assert_eq!(
                read("2024-03-05T06:07:08.0000000000Z"),
                read("2024-03-05T06:07:08Z")
            );
            assert!(bound("\"2024-03-05T24:00:00.0000000000Z\"").is_ok());
        }

        /// The negative control for the test above: a NONZERO digit past the
        /// ninth is real precision this crate cannot compare, and truncating
        /// it would merge two instants a well-formed store distinguishes.
        /// Without this case the fix above would read as "accept any tail".
        #[test]
        fn a_nonzero_digit_past_nanoseconds_is_still_refused() {
            assert!(bound("\"2024-03-05T06:07:08.1234567891Z\"").is_err());
            assert!(bound("\"2024-03-05T06:00:00.0000000001Z\"").is_err());
            // A nonzero digit further out still counts, however long the tail.
            assert!(bound("\"2024-03-05T06:07:08.12345678900001Z\"").is_err());
            // And 24:00:00 still admits only zeroes.
            assert!(bound("\"2024-03-05T24:00:00.0000000001Z\"").is_err());
        }
    }
}

/// The readable period behind a verdict, if there is one.
fn period_of(v: &GraphValidity) -> Option<&Period> {
    match v {
        GraphValidity::Sound(p) | GraphValidity::Degenerate { period: p, .. } => Some(p),
        GraphValidity::Unreadable { .. } => None,
    }
}

fn period_mut(v: &mut GraphValidity) -> Option<&mut Period> {
    match v {
        GraphValidity::Sound(p) | GraphValidity::Degenerate { period: p, .. } => Some(p),
        GraphValidity::Unreadable { .. } => None,
    }
}

/// The second pass over the validity map: what the lineage links imply.
///
/// A join made when the map is read, and never written back. Every consumer
/// runs on the map this returns, so `scope`, `query_at` and `conflicts`
/// cannot disagree about which bound closed a graph. The rules, in the order
/// they run:
///
///   - A link whose target is the graph itself, a graph with no temporal
///     description, a graph whose description could not be read, or not a
///     graph IRI at all, has no effect and is reported. So does a link
///     asserted BY a graph whose own description could not be read: the
///     graph it names is neither closed nor withdrawn, and the row is the
///     only place that says so, since the asserter's `invalid` row is about
///     its bounds. Lineage is asserted, and a link that asserts nothing
///     readable closes nothing. Such a link is then PRUNED from the map:
///     the report row is the only place it survives, so the cycle walk here
///     and the chain walk in `conflicts` follow effective links only and
///     cannot file a pair as a correction on a link this pass rejected.
///   - A graph with no explicit `recordedUntil` and at least one dated
///     successor is closed at the EARLIEST successor's `recordedAt`. Belief
///     in it ended at the first replacement; a second successor does not
///     revive it. A successor with no `recordedAt` cannot date the
///     replacement and is reported, leaving the bound as it was.
///   - A graph WITH an explicit `recordedUntil` keeps it. A successor whose
///     `recordedAt` names a different instant is reported as a disagreement,
///     never reconciled: the explicit bound is the authority, the report is
///     the place the inconsistency survives.
///   - More than one dated successor is reported on either branch, in one
///     row shape naming them all: `closed_by` names the successor whose
///     `recordedAt` became the bound where it was derived, and
///     `recorded_until` the explicit bound where it was asserted, since no
///     successor closed a graph that closed itself.
///   - A retracted graph carries its retractors, earliest first. One with no
///     `recordedAt` is reported as undated, and stands at every instant
///     asked about; with no `as_of` none is, and no retractor stands.
///   - A cycle of `supersedes` links is reported once, and the derivation
///     above is one hop per graph, so a cycle cannot loop it.
///   - A transaction interval that closes before it opens, from an asserted
///     bound or a derived one, is reported as inverted. Nothing is clamped:
///     the row shows the asserted instants, and `not_recorded_at` answers
///     that the graph was believed at no instant.
///
/// Reports are rows of `{graph, reason, ...}`; a row may carry the IRIs and
/// instants it is about as fields, because each row is about one graph.
fn derive_lineage(map: &mut BTreeMap<String, GraphValidity>) -> Vec<serde_json::Value> {
    let mut reports: Vec<serde_json::Value> = Vec::new();

    // The inbound index: which readable graphs name each graph, per predicate.
    // A link from an unreadable graph joins no index and is reported here,
    // on the asserter's side like every other link that has no effect.
    //
    // A link that joins no index is also PRUNED from the asserter's lineage,
    // below, once this loop has finished reading the map. The report row is
    // the only place a rejected link survives: `conflicts` used to walk the
    // raw `supersedes` list and file a pair as a correction on a link this
    // pass had just said had no effect, a contradiction suppressed by a
    // link that closed nothing. Every consumer now walks effective links by
    // construction, so none can disagree with the report.
    let mut successors: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut retractors: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // (asserter, effective supersedes, effective retracts), for asserters
    // that lost at least one link.
    let mut pruned: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
    for (g, v) in map.iter() {
        let p = match v {
            GraphValidity::Sound(p) | GraphValidity::Degenerate { period: p, .. } => p,
            GraphValidity::Unreadable { asserted, .. } => {
                for (field, target) in asserted {
                    reports.push(serde_json::json!({
                        "graph": g,
                        "reason": format!(
                            "{field} is asserted by a graph whose own temporal description \
                             could not be read; the link has no effect, and the graph it \
                             names is neither closed nor withdrawn by it"
                        ),
                        "target": target,
                    }));
                }
                continue;
            }
        };
        let mut kept_supersedes: Vec<String> = Vec::new();
        let mut kept_retracts: Vec<String> = Vec::new();
        for (field, targets, index, kept) in [
            (
                "supersedes",
                &p.lineage.supersedes,
                &mut successors,
                &mut kept_supersedes,
            ),
            (
                "retracts",
                &p.lineage.retracts,
                &mut retractors,
                &mut kept_retracts,
            ),
        ] {
            for target in targets {
                let fault = if target == g {
                    Some("names the graph itself")
                } else {
                    match map.get(target) {
                        None => Some(
                            "names a graph that carries no temporal description, whether or not \
                             it exists, so there is nothing to close or withdraw",
                        ),
                        Some(GraphValidity::Unreadable { .. }) => Some(
                            "names a graph whose temporal description could not be read, which \
                             is invalid already",
                        ),
                        Some(_) => None,
                    }
                };
                match fault {
                    Some(why) => reports.push(serde_json::json!({
                        "graph": g,
                        "reason": format!("{field} {why}; the link has no effect"),
                        "target": target,
                    })),
                    None => {
                        index.entry(target.clone()).or_default().push(g.clone());
                        kept.push(target.clone());
                    }
                }
            }
        }
        if kept_supersedes.len() != p.lineage.supersedes.len()
            || kept_retracts.len() != p.lineage.retracts.len()
        {
            pruned.push((g.clone(), kept_supersedes, kept_retracts));
        }
        for (field, term) in &p.lineage.not_iris {
            reports.push(serde_json::json!({
                "graph": g,
                "reason": format!("{field} names a term that is not a graph IRI; the link has no effect"),
                "target": term,
            }));
        }
    }
    // From here on the map carries effective links only: a rejected one is
    // in `reports` and nowhere else, so the cycle walk below and the chain
    // walk in `conflicts` cannot follow it.
    for (g, supersedes, retracts) in pruned {
        if let Some(p) = map.get_mut(&g).and_then(period_mut) {
            p.lineage.supersedes = supersedes;
            p.lineage.retracts = retracts;
        }
    }

    // Close each superseded graph from its successors. Collected first and
    // applied after, since a successor is read from the same map.
    let mut closings: Vec<(String, DateTime<Utc>, String)> = Vec::new();
    for (x, succs) in &successors {
        let Some(p) = map.get(x).and_then(period_of) else {
            continue;
        };
        // (instant, lexical form, successor graph): sorted, the first is the
        // earliest, ties broken by the text and then the IRI so the answer
        // does not depend on row order.
        let mut dated: Vec<(DateTime<Utc>, String, String)> = Vec::new();
        for s in succs {
            let Some(sp) = map.get(s).and_then(period_of) else {
                continue;
            };
            match (sp.recorded, sp.shown.recorded_at.as_ref()) {
                (Some(at), Some(lexical)) => dated.push((at, lexical.clone(), s.clone())),
                _ => reports.push(serde_json::json!({
                    "graph": x,
                    "reason": "the graph that supersedes it carries no recordedAt, so the \
                               replacement cannot be dated and this link leaves the \
                               transaction interval as it was",
                    "successor": s,
                })),
            }
        }
        dated.sort();
        let Some((earliest, _, closer)) = dated.first().cloned() else {
            continue;
        };
        let explicit = match (p.until, p.shown.recorded_until.as_ref()) {
            (Some(explicit), Some(explicit_lexical)) => {
                for (at, lexical, s) in &dated {
                    if *at != explicit {
                        reports.push(serde_json::json!({
                            "graph": x,
                            "reason": "explicit recordedUntil disagrees with the recordedAt of \
                                       the graph that supersedes it; the explicit bound governs",
                            "successor": s,
                            "recorded_until": explicit_lexical,
                            "successor_recorded_at": lexical,
                        }));
                    }
                }
                Some(explicit_lexical.clone())
            }
            _ => {
                closings.push((x.clone(), earliest, closer.clone()));
                None
            }
        };
        // Multiplicity is a fact about the links, not about which bound
        // closed the graph, so it is reported on both branches in one row
        // shape: `closed_by` names the successor where the bound was derived,
        // and `recorded_until` the explicit bound where it was asserted, so
        // the row never names a successor as having closed what it did not.
        if dated.len() > 1 {
            let mut row = serde_json::json!({
                "graph": x,
                "reason": match explicit {
                    Some(_) => "superseded by more than one graph; the explicit recordedUntil \
                                governs, so none of them closed it, and a later successor does \
                                not revive it",
                    None => "superseded by more than one graph; belief in it ended at the \
                             earliest recordedAt among them, and a later successor does not \
                             revive it",
                },
                "successors": dated.iter().map(|(_, _, s)| s).collect::<Vec<_>>(),
            });
            match explicit {
                Some(lexical) => row["recorded_until"] = serde_json::Value::from(lexical),
                None => row["closed_by"] = serde_json::Value::from(closer.as_str()),
            }
            reports.push(row);
        }
    }

    // Retractors, earliest recorded first and the undated ones last.
    let mut withdrawals: Vec<(String, Vec<Retractor>)> = Vec::new();
    for (x, rets) in &retractors {
        let mut list: Vec<Retractor> = Vec::new();
        for r in rets {
            let Some(rp) = map.get(r).and_then(period_of) else {
                continue;
            };
            if rp.recorded.is_none() {
                reports.push(serde_json::json!({
                    "graph": x,
                    "reason": "retracted by a graph with no recordedAt, so the withdrawal cannot \
                               be dated and stands at every as_of",
                    "retracted_by": r,
                }));
            }
            list.push(Retractor {
                graph: r.clone(),
                recorded: rp.recorded,
                recorded_lexical: rp.shown.recorded_at.clone(),
            });
        }
        list.sort_by(|a, b| match (a.recorded, b.recorded) {
            (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.graph.cmp(&b.graph)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.graph.cmp(&b.graph),
        });
        withdrawals.push((x.clone(), list));
    }

    // Cycles. Iterative rather than recursive: a chain can be as long as the
    // map, and a worker thread's stack is not. The links are effective by
    // construction, pruned above: every child is a readable graph in the
    // map other than its parent, so the walk needs no guard against a
    // self-link or a target that is not there.
    fn children<'a>(map: &'a BTreeMap<String, GraphValidity>, g: &str) -> &'a [String] {
        map.get(g)
            .and_then(period_of)
            .map(|p| p.lineage.supersedes.as_slice())
            .unwrap_or(&[])
    }
    let mut done: BTreeSet<&str> = BTreeSet::new();
    let mut cycles: BTreeSet<Vec<String>> = BTreeSet::new();
    for start in map.keys() {
        if done.contains(start.as_str()) {
            continue;
        }
        let mut path: Vec<(&str, usize)> = vec![(start.as_str(), 0)];
        while let Some(&(g, i)) = path.last() {
            let Some(child) = children(map, g).get(i) else {
                path.pop();
                done.insert(g);
                continue;
            };
            if let Some(frame) = path.last_mut() {
                frame.1 = i + 1;
            }
            let c = child.as_str();
            if done.contains(c) {
                continue;
            }
            if let Some(k) = path.iter().position(|&(on_path, _)| on_path == c) {
                cycles.insert(
                    path[k..]
                        .iter()
                        .map(|(on_path, _)| on_path.to_string())
                        .collect(),
                );
                continue;
            }
            path.push((c, 0));
        }
    }
    for cycle in cycles {
        reports.push(serde_json::json!({
            "graph": cycle[0],
            "reason": "supersedes links form a cycle, so no graph in it is the latest version; \
                       each bound is still derived from the direct successor and the walk \
                       stops here",
            "cycle": cycle,
        }));
    }

    for (x, at, closer) in closings {
        if let Some(p) = map.get_mut(&x).and_then(period_mut) {
            p.until = Some(at);
            p.lineage.until_derived_from = Some(closer);
        }
    }
    for (x, list) in withdrawals {
        if let Some(p) = map.get_mut(&x).and_then(period_mut) {
            p.lineage.retracted_by = list;
        }
    }

    // Inverted transaction intervals, judged on the composed bound.
    let mut inverted: Vec<serde_json::Value> = Vec::new();
    for (g, v) in map.iter() {
        let Some(p) = period_of(v) else { continue };
        if !p.inverted_recording() {
            continue;
        }
        let mut row = serde_json::json!({
            "graph": g,
            "reason": "the transaction interval closes before it opens: recordedUntil precedes \
                       recordedAt, an inverted transaction interval; nothing was clamped, and \
                       the graph is believed at no instant",
            "recorded_at": p.shown.recorded_at,
        });
        match &p.lineage.until_derived_from {
            Some(s) => {
                row["superseded_by"] = serde_json::Value::from(s.as_str());
                row["recorded_until"] = serde_json::Value::from(
                    map.get(s)
                        .and_then(period_of)
                        .and_then(|sp| sp.shown.recorded_at.clone()),
                );
            }
            None => row["recorded_until"] = serde_json::Value::from(p.shown.recorded_until.clone()),
        }
        inverted.push(row);
    }
    reports.extend(inverted);

    // One graph's rows together, whatever rule produced them.
    reports.sort_by(|a, b| a["graph"].as_str().cmp(&b["graph"].as_str()));
    reports
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

/// Read a tool argument as an instant.
///
/// An argument that cannot be read is refused rather than dropped: treating it
/// as "no bound" would answer a question nobody asked, with the whole store in
/// scope and nothing saying why.
///
/// The message deliberately does not quote the argument back. The tool
/// handlers render an `Err` into JSON by hand (`src/server.rs`, quote
/// substitution only), so caller text reaching that path could emit a
/// malformed response, and the caller has the value already. Store-side
/// reasons do quote the offending bound, because those travel in the `invalid`
/// array, which is built by serde_json and escaped properly.
fn argument(name: &str, text: Option<&str>) -> anyhow::Result<Option<DateTime<Utc>>> {
    text.map(|t| {
        instant::argument(t).map_err(|_| {
            anyhow::anyhow!(
                "{name} is not readable as an instant: it must be an xsd:date, xsd:dateTime, \
                 xsd:gYearMonth or xsd:gYear value, and a value carrying no timezone offset \
                 is read as UTC"
            )
        })
    })
    .transpose()
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
        self.rows_with(query, cap, false)
    }

    /// As `rows`, but evaluates against the union of all graphs so that a query's
    /// UNGUARDED triple patterns (those not inside a `GRAPH` block) see every named
    /// graph rather than the default graph alone. `GRAPH ?g` patterns still range
    /// over named graphs under the union default, so a query that mixes graph-scoped
    /// and schema-level patterns keeps the first and widens the second. Used by
    /// `conflicts`, whose disjointWith/subClassOf* schema half is unguarded and may
    /// live in a named graph.
    fn rows_union(&self, query: &str, cap: usize) -> anyhow::Result<Scan> {
        self.rows_with(query, cap, true)
    }

    fn rows_with(&self, query: &str, cap: usize, union: bool) -> anyhow::Result<Scan> {
        let probe = cap.saturating_add(1);
        let limited = format!("{query} LIMIT {probe}");
        let raw = if union {
            self.graph.sparql_select_union(&limited)?
        } else {
            self.graph.sparql_select(&limited)?
        };
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

    /// Every graph that carries validity or lineage metadata, whether the scan
    /// that found them was complete, and what the lineage pass reported.
    ///
    /// The lineage derivation runs here, on the map every consumer reads, so
    /// a graph closed by its successor is closed for the snapshot, the query
    /// and the conflict check alike.
    fn validities(
        &self,
    ) -> anyhow::Result<(
        BTreeMap<String, GraphValidity>,
        Capped,
        Vec<serde_json::Value>,
    )> {
        let query = format!(
            "SELECT ?g ?from ?to ?rec ?until ?sup ?ret WHERE {{ \
             {{ ?g <{NS}validFrom> ?from }} UNION {{ ?g <{NS}validTo> ?to }} \
             UNION {{ ?g <{NS}recordedAt> ?rec }} \
             UNION {{ ?g <{NS}recordedUntil> ?until }} \
             UNION {{ ?g <{NS}supersedes> ?sup }} \
             UNION {{ ?g <{NS}retracts> ?ret }} }} ORDER BY ?g"
        );
        let scan = self.rows(&query, self.limits.validity_scan)?;
        // 1.2.0 ASSIGNED each field here, so the last row of the UNION won
        // and any other value vanished. Collect instead: a graph asserting
        // two different instants on one axis is a data error, and resolving
        // it by min, max or row order would invent an interval nobody
        // asserted.
        let mut terms: BTreeMap<String, Bounds> = BTreeMap::new();
        for row in &scan.rows {
            let Some(g) = row.get("g").and_then(|v| v.as_str()).map(plain) else {
                continue;
            };
            let entry = terms.entry(g).or_default();
            for (key, slot) in [
                ("from", &mut entry.from),
                ("to", &mut entry.to),
                ("rec", &mut entry.recorded),
                ("until", &mut entry.until),
                ("sup", &mut entry.supersedes),
                ("ret", &mut entry.retracts),
            ] {
                if let Some(v) = row.get(key).and_then(|v| v.as_str()) {
                    let term = v.trim().to_string();
                    if !slot.contains(&term) {
                        slot.push(term);
                    }
                }
            }
        }
        let mut out: BTreeMap<String, GraphValidity> = terms
            .into_iter()
            .map(|(graph, bounds)| (graph.clone(), bounds.resolve(graph)))
            .collect();
        let lineage = derive_lineage(&mut out);
        Ok((out, scan.capped, lineage))
    }

    /// Named graphs holding assertions, whether or not they are described.
    fn all_graphs(&self) -> anyhow::Result<(BTreeSet<String>, Capped)> {
        // ORDER BY so that, if the scan is truncated at the cap, which graphs are
        // dropped is deterministic rather than dependent on hash iteration order.
        let scan = self.rows(
            "SELECT DISTINCT ?g WHERE { GRAPH ?g { ?s ?p ?o } } ORDER BY ?g",
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
    fn scope(
        &self,
        valid_at: Option<DateTime<Utc>>,
        as_of: Option<DateTime<Utc>>,
    ) -> anyhow::Result<Scope> {
        let (validities, validity_scan, lineage) = self.validities()?;
        let (graphs, graph_scan) = self.all_graphs()?;

        let mut in_scope = Vec::new();
        let mut excluded = Vec::new();
        let mut invalid = Vec::new();
        let mut retracted = Vec::new();
        let mut names = Vec::new();
        for g in &graphs {
            // The readable period, and which shape it is when it holds at no
            // instant. The two verdicts that are not a period answer here.
            let (p, kind) = match validities.get(g) {
                // Undescribed graphs are timeless and always in scope, so
                // this vocabulary is additive to an existing store.
                None => {
                    in_scope.push(
                        serde_json::json!({"graph": g, "reason": "no validity recorded, timeless"}),
                    );
                    names.push(g.clone());
                    continue;
                }
                // Described, but the description could not be read. Never in
                // scope and never timeless: "we hold no valid-time claim about
                // this" and "the claim is garbage" are different answers.
                Some(GraphValidity::Unreadable { faults, .. }) => {
                    invalid.push(serde_json::json!({
                        "graph": g,
                        "reason": GraphValidity::fault_reason(faults),
                    }));
                    continue;
                }
                Some(GraphValidity::Sound(p)) => (p, None),
                // Readable, and holds at no instant. With no valid_at it is
                // read like any other graph: no instant was asked about, and
                // narrowing an atemporal query silently is worse than
                // answering it. At any instant asked about it is excluded,
                // with a reason naming which of the two shapes it is, since
                // the row's `valid` shows bounds that can look perfectly
                // ordinary at a glance; so onto_temporal_snapshot at any
                // valid_at reports it once. From here on it takes the sound
                // graph's path, validity first and the recorded side second:
                // with a valid_at given, "holds at no instant" beats "not yet
                // recorded then", because the first is a fact about the data
                // and the second a fact about the query, and only the first
                // is the thing to fix; with none given, the recorded side is
                // the only question asked, and its reason the only one that
                // can apply. `conflicts` is unaffected: `overlaps` answers
                // false for such a period regardless.
                Some(GraphValidity::Degenerate { period, kind }) => (period, Some(*kind)),
            };
            // Retraction is asked first, once the graph is readable at all: an
            // unreadable one answered above and stays invalid. It is a fact
            // about the assertion's standing, not about the instant asked
            // about, so it beats "not true at that instant" and both
            // recorded-side reasons; a graph that is both superseded and
            // retracted lands here. Before the retraction was recorded, and
            // with no `as_of` at all, the graph takes the ordinary path: with
            // no `as_of` the recorded axis is consulted for no fact, a
            // retraction no more than a `recordedUntil`, asserted or derived.
            if let Some(r) = p.retracted_as_of(as_of) {
                retracted.push(serde_json::json!({
                    "graph": g,
                    "valid": p.shown.describe(),
                    "reason": "retracted",
                    "retracted_by": r.graph,
                    "retracted_at": r.recorded_lexical,
                }));
                continue;
            }
            // A period that holds at no instant is never valid at one, and
            // `valid_at` is not asked about it: the half-open test already
            // answers false for `from >= to`, and the guard keeps that so
            // even if a `<=` slip there let such a period hold for one
            // instant.
            let valid_ok = valid_at.is_none_or(|t| kind.is_none() && p.valid_at(t));
            // Which side of the recorded interval `as_of` fell on, not merely
            // that it fell outside: "recorded later" and "no longer believed"
            // are opposite facts about the assertion.
            let recorded_miss = as_of.and_then(|t| p.not_recorded_at(t));
            if valid_ok && recorded_miss.is_none() {
                in_scope.push(serde_json::json!({"graph": g, "valid": p.shown.describe()}));
                names.push(g.clone());
            } else {
                let mut row = serde_json::json!({
                    "graph": g,
                    "valid": p.shown.describe(),
                    "reason": if !valid_ok {
                        kind.map_or("not true at that instant", Degenerate::reason)
                    } else {
                        recorded_miss.map_or("not yet recorded then", RecordedMiss::reason)
                    },
                });
                // A closing bound that was derived names the graph it was
                // derived from: the row would otherwise say the interval had
                // closed and show no recordedUntil that closed it.
                let closed_by_successor = p
                    .lineage
                    .until_derived_from
                    .as_deref()
                    .filter(|_| valid_ok && recorded_miss.is_some_and(RecordedMiss::closed));
                if let Some(s) = closed_by_successor {
                    row["superseded_by"] = serde_json::Value::from(s);
                }
                excluded.push(row);
            }
        }

        Ok(Scope {
            in_scope,
            excluded,
            invalid,
            retracted,
            lineage,
            graphs: names,
            validity_scan,
            graph_scan,
        })
    }

    /// Which graphs are in scope for a snapshot, and why.
    pub fn snapshot(&self, valid_at: Option<&str>, as_of: Option<&str>) -> anyhow::Result<String> {
        let at = argument("valid_at", valid_at)?;
        let of = argument("as_of", as_of)?;
        let scope = self.scope(at, of)?;

        let mut out = serde_json::json!({
            "ok": true,
            "valid_at": valid_at,
            "as_of": as_of,
            "in_scope": scope.in_scope,
            "excluded": scope.excluded,
            "complete": scope.complete(),
            "semantics_version": SEMANTICS_VERSION,
            "note": "Graphs without validity metadata are timeless and always in scope.",
        });
        if !scope.invalid.is_empty() {
            out["invalid"] = serde_json::Value::Array(scope.invalid.clone());
        }
        if !scope.retracted.is_empty() {
            out["retracted"] = serde_json::Value::Array(scope.retracted.clone());
        }
        if !scope.lineage.is_empty() {
            out["lineage"] = serde_json::Value::Array(scope.lineage.clone());
        }
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
        let at = argument("valid_at", valid_at)?;
        let of = argument("as_of", as_of)?;
        let scope = self.scope(at, of)?;
        // The scope this query runs over is the snapshot's scope, so the
        // snapshot's truncation is this query's truncation. Saying nothing
        // here would hide a wrong scope behind a tool that never mentions
        // scans at all. Same for the graphs left out as unreadable: a query
        // that silently narrows what it read is the failure this reporting
        // exists to remove.
        let mut cuts = scope.cuts();
        let scope_warning = scope.warning();
        let scope_complete = scope.complete();
        let invalid = scope.invalid;
        let retracted = scope.retracted;
        let lineage = scope.lineage;
        let graphs = scope.graphs;

        if graphs.is_empty() {
            let mut out = serde_json::json!({
                "ok": true,
                "results": [],
                "complete": scope_complete,
                "semantics_version": SEMANTICS_VERSION,
                "note": "no graphs in scope at that instant",
            });
            if !invalid.is_empty() {
                out["invalid"] = serde_json::Value::Array(invalid);
            }
            if !retracted.is_empty() {
                out["retracted"] = serde_json::Value::Array(retracted);
            }
            if !lineage.is_empty() {
                out["lineage"] = serde_json::Value::Array(lineage);
            }
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
        // FROM NAMED confines the dataset to exactly the in-scope graphs. The body is
        // the caller-supplied pattern, spliced as text; without this a crafted pattern
        // could close the GRAPH block and open its own `GRAPH <out-of-scope>` (or a
        // top-level group over the default graph), defeating temporal scope. With only
        // FROM NAMED and no FROM, a graph the caller names that is not in scope has no
        // triples in the dataset and the default graph is empty, so neither escape
        // reads anything. The pattern is wrapped in its own braces rather than having a
        // level stripped, so a legitimate top-level `{…} UNION {…}` stays well-formed.
        let from_named = graphs
            .iter()
            .map(|g| format!("FROM NAMED <{g}>"))
            .collect::<Vec<_>>()
            .join(" ");
        let body = pattern.trim();
        let wrapped = format!(
            "SELECT * {from_named} WHERE {{ VALUES ?__g {{ {values} }} GRAPH ?__g {{ {body} }} }}"
        );
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
            "semantics_version": SEMANTICS_VERSION,
        });
        if !invalid.is_empty() {
            out["invalid"] = serde_json::Value::Array(invalid);
        }
        if !retracted.is_empty() {
            out["retracted"] = serde_json::Value::Array(retracted);
        }
        if !lineage.is_empty() {
            out["lineage"] = serde_json::Value::Array(lineage);
        }
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
    /// the finding is noise. With it, only genuine disagreement about the same
    /// period survives.
    ///
    /// What the other bucket contains is narrower than it used to claim. The
    /// test is `!overlaps`, which proves the two periods share no instant —
    /// nothing more. It does NOT prove one replaced the other, and the data
    /// carries no link that would. Three different situations land there:
    /// periods that touch at a boundary, periods separated by a GAP, and
    /// periods that never met. The gap is the one that is not benign: it is
    /// missing coverage, not history, and reading it as a correction invents a
    /// continuity nobody asserted. Hence `non_overlapping`, and hence
    /// `superseded` being deprecated rather than redefined — see #110.
    ///
    /// The link the data did not carry, it now can. A pair where one graph
    /// asserts that it supersedes the other, directly or through a chain of
    /// `supersedes` links, is a correction whatever its periods: the issue's
    /// own example is a correction that shares its predecessor's valid period
    /// and was filed as a contradiction. Such a pair lands in `corrections`,
    /// checked before the overlap test, and lineage that is asserted is the
    /// only thing that puts it there. A retracted graph is not treated
    /// specially here.
    pub fn conflicts(&self) -> anyhow::Result<String> {
        let (validities, validity_scan, _lineage) = self.validities()?;
        // Union scope: the ABox halves stay graph-scoped via GRAPH ?ga/?gb, while
        // the disjointWith/subClassOf* schema half is unguarded and must see every
        // graph. Under the plain default dataset it read the default graph alone,
        // so a store whose schema sits in a named graph (the normal per-version
        // temporal layout, or any TriG/N-Quads schema graph) formed no pair and
        // returned a clean zero contradictions.
        let scan = self.rows_union(
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

        // An undescribed graph is timeless, which is a real period with two
        // open ends. An UNREADABLE one is not a period at all, and handing it
        // this value would make it overlap everything: the false
        // contradiction the truncation warning below is about, arriving
        // through the data instead of through a cap. A DEGENERATE one is a
        // real period that holds at no instant: it is compared, and
        // `overlaps` answers false for it, so the pair lands in
        // non_overlapping, which is true and since #116 claims no correction.
        let timeless = Period::default();
        let period = |graph: &str| match validities.get(graph) {
            None => Some(&timeless),
            Some(v) => period_of(v),
        };
        // Does `from` supersede `to`, directly or through a chain. Walks only
        // readable graphs in the map, each once, so a cycle ends the walk
        // rather than the process. The links it follows are EFFECTIVE by
        // construction: `derive_lineage` pruned every link it rejected (a
        // self-link, an undescribed or unreadable target) from the map
        // before handing it on, so a pair can land in `corrections` only on
        // a link the snapshot's `lineage` does not report as having no
        // effect. Reading the raw assertions here would let a link that
        // closed nothing suppress a genuine contradiction.
        let supersedes = |from: &str, to: &str| -> bool {
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            let mut stack: Vec<&str> = vec![from];
            while let Some(g) = stack.pop() {
                if !seen.insert(g) {
                    continue;
                }
                let Some(p) = validities.get(g).and_then(period_of) else {
                    continue;
                };
                for s in &p.lineage.supersedes {
                    if s == to {
                        return true;
                    }
                    stack.push(s.as_str());
                }
            }
            false
        };

        let mut conflicts = Vec::new();
        let mut corrections = Vec::new();
        let mut non_overlapping = Vec::new();
        let mut undecided = Vec::new();
        for row in &scan.rows {
            let get = |k: &str| row.get(k).and_then(|v| v.as_str()).map(plain);
            let (Some(s), Some(a), Some(b), Some(ga), Some(gb)) =
                (get("s"), get("a"), get("b"), get("ga"), get("gb"))
            else {
                continue;
            };
            if ga == gb {
                continue;
            }

            let (Some(va), Some(vb)) = (period(&ga), period(&gb)) else {
                undecided.push(serde_json::json!({
                    "subject": local(&s),
                    "types": [local(&a), local(&b)],
                    "graphs": [ga, gb],
                    "reason": "one of these graphs has temporal metadata that could not be read \
                               on at least one axis, so it is invalid and the pair was not \
                               classified; see invalid in onto_temporal_snapshot",
                }));
                continue;
            };

            let mut entry = serde_json::json!({
                "subject": local(&s),
                "types": [local(&a), local(&b)],
                "periods": [va.shown.describe(), vb.shown.describe()],
                "graphs": [&ga, &gb],
            });
            // Asserted lineage first: a pair one side of which replaces the
            // other is a correction whatever the periods say, and the
            // overlap test is never reached for it.
            let link = if supersedes(&ga, &gb) {
                Some([&ga, &gb])
            } else if supersedes(&gb, &ga) {
                Some([&gb, &ga])
            } else {
                None
            };
            if let Some([successor, predecessor]) = link {
                entry["supersedes"] = serde_json::json!([successor, predecessor]);
                corrections.push(entry);
            } else if va.overlaps(vb) {
                conflicts.push(entry);
            } else {
                non_overlapping.push(entry);
            }
        }

        // `superseded` is the same set under its old, wrong name. Emitted
        // unconditionally until 2.0 and behind no flag: an opt-out would be a
        // third response shape to document and test, and it would let a client
        // code against a shape no release guarantees. It is deprecated, not
        // renamed: lineage-backed supersession arrived as `corrections`, a
        // NEW key, so that no key ever names a different set across a major
        // version. See #110.
        let mut out = serde_json::json!({
            "ok": true,
            "contradictions": conflicts,
            "contradiction_count": conflicts.len(),
            "non_overlapping": non_overlapping,
            "non_overlapping_count": non_overlapping.len(),
            "superseded": non_overlapping,
            "superseded_count": non_overlapping.len(),
            "complete": !validity_scan.hit && !scan.capped.hit,
            "semantics_version": SEMANTICS_VERSION,
            "note": "contradictions claim overlapping validity and genuinely disagree. \
                     non_overlapping pairs have no instant in common, judged on their bounds \
                     read as instants on the UTC timeline, so a timezone offset is honoured; \
                     that is all that has been checked: it is not evidence that one replaced \
                     the other, and the bucket also holds pairs separated by a GAP (missing \
                     coverage rather than history) and pairs where one period holds at no \
                     instant, empty or inverted, which onto_temporal_snapshot reports once \
                     with a reason naming which. undecided holds pairs where a graph's \
                     temporal metadata could not be read: an unreadable period is never \
                     treated as an open one, so such a pair is classified neither way. \
                     corrections are pairs where one graph asserts that it supersedes the \
                     other, directly or through a chain of supersedes links, lineage that is \
                     asserted and never inferred, and such a pair is never a contradiction \
                     whatever its periods. \
                     superseded is the same set as non_overlapping under a name that claimed \
                     more than was proven; it is deprecated and will be dropped at 2.0.",
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
                 as contradictions; a supersedes row that fell past the limit while its \
                 asserter's bounds did not was never seen, so the pair it links was compared \
                 on its periods and can appear here as a contradiction too",
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

        if !corrections.is_empty() {
            out["corrections"] = serde_json::Value::Array(corrections.clone());
            out["corrections_count"] = serde_json::Value::from(corrections.len());
        }
        if !undecided.is_empty() {
            out["undecided"] = serde_json::Value::Array(undecided.clone());
            out["undecided_count"] = serde_json::Value::from(undecided.len());
        }
        if !cuts.is_empty() {
            out["truncated"] = serde_json::Value::Array(cuts);
        }
        if validity_scan.hit {
            out["warning"] = serde_json::Value::String(
                "UNSOUND CLASSIFICATION: the validity scan hit its row limit, so some pairs \
                 were compared without their periods, or without the supersedes link that \
                 would have pre-empted the comparison. A correction can be reported here as \
                 a contradiction, which is the one thing this tool exists to prevent; \
                 contradiction_count is an upper bound over the pairs that were examined, and \
                 says nothing about any pair the candidate scan did not reach. See truncated."
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

    /// One graph carrying all four predicates. The scan cap is denominated in
    /// ROWS, so this is what a fully described graph costs.
    const FULLY_DESCRIBED: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1 { ex:X ex:p ex:one . }

{
  ex:g1 t:validFrom "2024-01-01" ; t:validTo "2026-01-01" ;
        t:recordedAt "2024-01-05" ; t:recordedUntil "2026-01-06" .
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

    /// A crafted query_at pattern must not escape temporal scope. g_live is valid
    /// at the query instant; g_expired stopped being valid in 2020. A pattern that
    /// closes the GRAPH block and opens `GRAPH <g_expired>` (or a top-level group)
    /// must read nothing from out-of-scope graphs, and a legitimate top-level UNION
    /// must still parse.
    const SCOPE_ESCAPE_FIXTURE: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .
ex:g_live    { ex:live_secret ex:says "IN_SCOPE" . }
ex:g_expired { ex:expired_secret ex:says "OUT_OF_SCOPE" . }
{
  ex:g_live    t:validFrom "2020-01-01" .
  ex:g_expired t:validFrom "2010-01-01" ; t:validTo "2020-01-01" .
}
"#;

    #[test]
    fn query_at_pattern_cannot_escape_temporal_scope() {
        let t = Temporal::new(store(SCOPE_ESCAPE_FIXTURE));

        let benign = json(t.query_at("{ ?s ?p ?o }", Some("2025-01-01"), None).unwrap());
        let benign_str = benign.to_string();
        assert!(benign_str.contains("IN_SCOPE"), "control: g_live must be in scope: {benign}");
        assert!(!benign_str.contains("OUT_OF_SCOPE"), "control: g_expired excluded at 2025: {benign}");

        // Escape attempt: close GRAPH ?__g, open an explicit out-of-scope GRAPH.
        let escaped = json(
            t.query_at(
                "{ ?s ?p ?o } GRAPH <http://example.org/g_expired> { ?a ?b ?c }",
                Some("2025-01-01"),
                None,
            )
            .unwrap(),
        );
        assert!(
            !escaped.to_string().contains("OUT_OF_SCOPE"),
            "SCOPE ESCAPE: an out-of-scope graph was read through a crafted pattern: {escaped}"
        );

        // A legitimate top-level UNION must remain valid (no one-level brace strip).
        let unioned = json(
            t.query_at(
                "{ ?s ?p ?o } UNION { ?s ?p ?o }",
                Some("2025-01-01"),
                None,
            )
            .unwrap(),
        );
        assert_eq!(unioned["ok"], true, "a top-level UNION must parse: {unioned}");
        assert!(unioned.to_string().contains("IN_SCOPE"), "union must still return in-scope rows: {unioned}");
    }

    /// Two raw valid bounds, as the validity scan would hand them over.
    fn valid(from: &str, to: &str) -> Bounds {
        Bounds {
            from: vec![format!("\"{from}\"")],
            to: vec![format!("\"{to}\"")],
            ..Bounds::default()
        }
    }

    /// The classification is made where the bounds are read, and nowhere
    /// else: equal instants are an empty period, a `validTo` before its
    /// `validFrom` an inverted one, and everything with an instant inside it
    /// is sound. The period travels with the verdict, and carries it too, so
    /// `overlaps` can answer from it without seeing the enum.
    #[test]
    fn resolve_classifies_a_period_that_holds_nowhere_once_when_it_is_read() {
        for (from, to, kind) in [
            ("2024-01-01", "2024-01-01", Degenerate::Empty),
            // Two precisions naming one instant: the case parsing created.
            ("2024", "2024-01-01", Degenerate::Empty),
            ("2026-01-01", "2020-01-01", Degenerate::Inverted),
        ] {
            match valid(from, to).resolve("g".into()) {
                GraphValidity::Degenerate { period, kind: k } => {
                    assert_eq!(k, kind, "[{from}, {to})");
                    assert_eq!(period.degenerate, Some(kind), "[{from}, {to})");
                }
                other => panic!("[{from}, {to}) should hold nowhere: {other:?}"),
            }
        }
        for (from, to) in [("2024-01-01", "2025-01-01"), ("2024-01-01", "2024-01-02")] {
            match valid(from, to).resolve("g".into()) {
                GraphValidity::Sound(period) => assert_eq!(period.degenerate, None),
                other => panic!("[{from}, {to}) holds somewhere: {other:?}"),
            }
        }
        // The recorded axis is not classified here: a recordedUntil before
        // its recordedAt is out-of-order recording (#109), a different
        // question, and pinning it as sound makes that change deliberate.
        let recorded = Bounds {
            recorded: vec!["\"2026-01-01\"".to_string()],
            until: vec!["\"2020-01-01\"".to_string()],
            ..Bounds::default()
        };
        assert!(matches!(
            recorded.resolve("g".into()),
            GraphValidity::Sound(_)
        ));
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
    /// `complete`, and no report of a cut that did not happen. "Nothing else"
    /// is about the cut. `semantics_version` is always present, and `invalid`
    /// and `undecided` appear whenever they are non-empty; those keys say
    /// nothing about caps and the conformance corpus pins them separately.
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

    /// The row cost of the fourth predicate, proved rather than documented.
    /// A fully described graph used to fit in three rows and now needs four,
    /// which is the whole of what `recordedUntil` does to the cap: 20,000 rows
    /// covers roughly 5,000 such graphs instead of roughly 6,700. A store that
    /// never writes the predicate is untouched — nothing here counts graphs.
    #[test]
    fn a_fully_described_graph_costs_four_validity_rows() {
        let cut = Temporal::with_limits(
            store(FULLY_DESCRIBED),
            Limits {
                validity_scan: 3,
                ..Limits::default()
            },
        );
        let snap = json(cut.snapshot(None, None).unwrap());
        assert_eq!(
            snap["complete"], false,
            "three rows no longer cover one four-predicate graph: {snap}"
        );

        let whole = Temporal::with_limits(
            store(FULLY_DESCRIBED),
            Limits {
                validity_scan: 4,
                ..Limits::default()
            },
        );
        let snap = json(whole.snapshot(None, None).unwrap());
        assert_eq!(snap["complete"], true, "four rows do: {snap}");
    }

    /// The same graph, plus one `supersedes` link.
    const FULLY_DESCRIBED_WITH_LINK: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1 { ex:X ex:p ex:one . }

{
  ex:g1 t:validFrom "2024-01-01" ; t:validTo "2026-01-01" ;
        t:recordedAt "2024-01-05" ; t:recordedUntil "2026-01-06" ;
        t:supersedes ex:g0 .
}
"#;

    /// The row cost of a lineage link, proved the same way. The link is a
    /// fifth UNION branch, so a graph carrying the four bounds and one
    /// `supersedes` costs five rows and 20,000 rows cover roughly 4,000 such
    /// graphs; with a `retracts` beside it, six rows and roughly 3,300. The
    /// test above is untouched: a graph without a link still costs four.
    #[test]
    fn a_fully_described_graph_with_a_supersedes_link_costs_five_validity_rows() {
        let cut = Temporal::with_limits(
            store(FULLY_DESCRIBED_WITH_LINK),
            Limits {
                validity_scan: 4,
                ..Limits::default()
            },
        );
        let snap = json(cut.snapshot(None, None).unwrap());
        assert_eq!(
            snap["complete"], false,
            "four rows no longer cover a graph that also asserts a link: {snap}"
        );

        let whole = Temporal::with_limits(
            store(FULLY_DESCRIBED_WITH_LINK),
            Limits {
                validity_scan: 5,
                ..Limits::default()
            },
        );
        let snap = json(whole.snapshot(None, None).unwrap());
        assert_eq!(snap["complete"], true, "five rows do: {snap}");
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

        // The validity UNION carries the lineage rows under the same cap, so
        // both texts must name the failure that costs: a graph whose own
        // bounds survived the cut but whose supersedes or retracts row did
        // not closed or withdrew nothing, and the graph it named stays open
        // or in scope although it is described. Neither text names a graph.
        for text in [
            snap["warning"].as_str().unwrap(),
            snap["truncated"][0]["consequence"].as_str().unwrap(),
        ] {
            assert!(
                text.contains("supersedes or retracts row"),
                "a cut lineage row is a third way the scope is wrong: {text}"
            );
            assert!(
                text.contains("described"),
                "and it happens to a described graph, which is the point: {text}"
            );
            assert!(
                !text.contains("http"),
                "no graph IRI belongs in a cap text: {text}"
            );
        }

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
        // The supersedes rows share the cap with the bounds, so a link can be
        // cut while both periods survive: the pair is then compared on
        // periods the link was meant to pre-empt. Both texts say so, and
        // neither names a graph.
        let consequence = cut["truncated"][0]["consequence"].as_str().unwrap();
        assert!(
            consequence.contains("supersedes row") && consequence.contains("never seen"),
            "a cut link is a second way a correction reads as a contradiction: {consequence}"
        );
        let warning = cut["warning"].as_str().unwrap();
        assert!(warning.contains("without the supersedes link"), "{warning}");
        assert!(
            !consequence.contains("http") && !warning.contains("http"),
            "{cut}"
        );
    }

    /// A live contradiction whose disjointWith/subClassOf schema lives in a NAMED
    /// graph, the normal layout for per-version temporal data. The ABox halves of
    /// the conflicts query are GRAPH-scoped, but the TBox halves were unguarded and
    /// so read the default graph alone: with the schema in a named graph the pair
    /// never formed and the tool returned a clean zero. Overlapping periods, disjoint
    /// types, no supersedes link, so exactly one contradiction.
    const NAMED_SCHEMA_CONFLICT: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g_before { ex:X a ex:Adherent . ex:Adherent owl:disjointWith ex:Suspension . }
ex:g_after  { ex:X a ex:Suspension . }

{
  ex:g_before t:validFrom "2024-01-01" ; t:validTo "2026-05-01" .
  ex:g_after  t:validFrom "2025-01-01" .
}
"#;

    #[test]
    fn conflicts_reads_disjointness_schema_from_a_named_graph() {
        let out = json(Temporal::new(store(NAMED_SCHEMA_CONFLICT)).conflicts().unwrap());
        assert_eq!(
            out["contradiction_count"], 1,
            "a live contradiction with named-graph schema must be found, not silently zero: {out}"
        );
        assert_eq!(out["superseded_count"], 0, "{out}");
        assert_eq!(out["complete"], true, "{out}");
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

    /// Both caps at once: every cut scan is listed, and the warning may not
    /// call the count an upper bound full stop — unexamined pairs can hold
    /// contradictions the count never saw, so the claim is bounded to the
    /// pairs that were actually compared.
    #[test]
    fn both_conflict_scans_cut_report_both_and_bound_the_claim() {
        let t = Temporal::with_limits(
            store(CORRECTION),
            Limits {
                validity_scan: 1,
                conflict_pairs: 0,
                ..Limits::default()
            },
        );
        let out = json(t.conflicts().unwrap());
        let scans: Vec<&str> = out["truncated"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["scan"].as_str().unwrap())
            .collect();
        assert_eq!(scans, ["validities", "conflict_pairs"], "{out}");
        assert_eq!(out["complete"], false);
        let warning = out["warning"].as_str().unwrap();
        assert!(
            warning.contains("over the pairs that were examined"),
            "with the candidate scan cut too, the count bounds nothing about the store: {out}"
        );
    }
}
