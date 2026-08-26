//! Bi-temporal conformance corpus for `onto_temporal_*` (issue #95).
//!
//! Pins the CURRENT semantics of `Temporal::snapshot`, `query_at` and
//! `conflicts` against a small table of named-graph datasets: half-open
//! validity with an exclusive upper bound, open bounds, recorded-time
//! visibility, timeless (undescribed) graphs, snapshot-scoped querying, and
//! the overlap-versus-superseded split that is the whole point of carrying
//! valid time. Every assertion holds on the code as it stands, `temporal/2`,
//! the parsed-time semantics that #95 asked for, so these are behavioural
//! anchors, not aspirations: a later change to the temporal semantics has to
//! move a line here on purpose rather than drift silently.
//!
//! The file has three sections. The first pins semantics that are **intended**;
//! most of it uses a single lexical date form, since that is the shape those
//! assertions are about, and none of it moved when bounds started being parsed.
//! The second pinned what the code did for mixed-precision and malformed bounds
//! when they were compared as datatype-stripped text, behaviour that fell out
//! of the implementation rather than being chosen, with each test naming what
//! the parsed-time work (#95, item 4) would turn it into; that work has landed,
//! so those tests now hold the new answer on the same lines, which is what the
//! section was for. The third covers what parsing adds and could not be
//! expressed before: coarse bounds, offsets, and bounds that must be refused.
//!
//! Deliberately not feature-gated: a `#![cfg(feature = ...)]` on a test file
//! that CI runs without that feature prints `running 0 tests` and reads as
//! green (this repo lost `tests/schema_test.rs` that way for months).

use open_ontologies::graph::GraphStore;
use open_ontologies::temporal::Temporal;
use oxigraph::io::RdfFormat;
use std::collections::BTreeSet;
use std::sync::Arc;

/// Load a TriG dataset and return a `Temporal` view over it.
fn temporal(trig: &str) -> Temporal {
    let store = Arc::new(GraphStore::new());
    store
        .load_content(trig, RdfFormat::TriG)
        .expect("TriG fixture parses");
    Temporal::new(store)
}

/// The local part (after the last `/` or `#`) of every graph in one snapshot
/// bucket — `"in_scope"` or `"excluded"`.
fn graphs(snapshot: &serde_json::Value, bucket: &str) -> BTreeSet<String> {
    snapshot[bucket]
        .as_array()
        .unwrap_or_else(|| panic!("snapshot has no array `{bucket}`: {snapshot}"))
        .iter()
        .map(|row| {
            let g = row["graph"].as_str().expect("graph name is a string");
            g.rsplit(['#', '/']).next().unwrap_or(g).to_string()
        })
        .collect()
}

fn snapshot(t: &Temporal, valid_at: Option<&str>, as_of: Option<&str>) -> serde_json::Value {
    serde_json::from_str(&t.snapshot(valid_at, as_of).unwrap()).unwrap()
}

/// The reason a snapshot gives for excluding one graph, named by its local
/// part. Panics rather than returning an Option: every caller is asserting on
/// the reason, so a missing row is a failed test, not an empty string.
fn reason(snapshot: &serde_json::Value, local: &str) -> String {
    let row = snapshot["excluded"]
        .as_array()
        .unwrap_or_else(|| panic!("snapshot has no array `excluded`: {snapshot}"))
        .iter()
        .find(|row| row["graph"].as_str().is_some_and(|g| g.ends_with(local)))
        .unwrap_or_else(|| panic!("`{local}` is not excluded: {snapshot}"));
    row["reason"]
        .as_str()
        .unwrap_or_else(|| panic!("`{local}` carries no reason: {row}"))
        .to_string()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// The reason one graph was reported unreadable, or `None` if it was not.
///
/// Separate from `graphs()` on purpose: a clean answer carries no `invalid`
/// key at all, and `graphs()` panics on a bucket that is not there.
fn invalid_reason(snapshot: &serde_json::Value, graph: &str) -> Option<String> {
    snapshot
        .get("invalid")?
        .as_array()?
        .iter()
        .find(|row| row["graph"].as_str().is_some_and(|g| g.ends_with(graph)))
        .and_then(|row| row["reason"].as_str())
        .map(str::to_string)
}

/// A correction over time plus one timeless fact. `g_adherent` held for a
/// closed period, `g_suspension` from the boundary onward, and `g_species` has
/// no validity metadata at all.
const CELL_LINE: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g_adherent   { ex:HEK293 a ex:AdherentCellLine . }
ex:g_suspension { ex:HEK293 a ex:SuspensionCellLine . }
ex:g_species    { ex:HEK293 ex:species ex:Human . }

{
  ex:g_adherent   t:validFrom "2024-01-01" ; t:validTo "2026-05-01" ; t:recordedAt "2024-01-05" .
  ex:g_suspension t:validFrom "2026-05-01" ; t:recordedAt "2026-05-02" .
}
"#;

#[test]
fn no_bounds_puts_every_graph_in_scope() {
    let t = temporal(CELL_LINE);
    let snap = snapshot(&t, None, None);
    assert_eq!(
        graphs(&snap, "in_scope"),
        set(&["g_adherent", "g_suspension", "g_species"]),
        "with neither valid_at nor as_of, every graph is in scope"
    );
    assert!(graphs(&snap, "excluded").is_empty());
}

#[test]
fn undescribed_graph_is_timeless_and_always_in_scope() {
    let t = temporal(CELL_LINE);
    // Deep in the past, before either described graph begins: only the graph
    // that carries no validity survives.
    let snap = snapshot(&t, Some("2000-01-01"), None);
    assert!(graphs(&snap, "in_scope").contains("g_species"));
    assert_eq!(
        graphs(&snap, "excluded"),
        set(&["g_adherent", "g_suspension"])
    );
}

#[test]
fn valid_at_selects_the_period_that_holds() {
    let t = temporal(CELL_LINE);
    let snap = snapshot(&t, Some("2024-06-01"), None);
    assert_eq!(
        graphs(&snap, "in_scope"),
        set(&["g_adherent", "g_species"]),
        "2024-06-01 is inside [2024-01-01, 2026-05-01)"
    );
    assert_eq!(graphs(&snap, "excluded"), set(&["g_suspension"]));
}

#[test]
fn validity_interval_is_half_open_at_the_upper_bound() {
    let t = temporal(CELL_LINE);
    // The instant where one period ends and the next begins belongs to the
    // next: [from, to) excludes `to`. This is what makes the correction a
    // correction rather than a one-instant overlap.
    let snap = snapshot(&t, Some("2026-05-01"), None);
    let in_scope = graphs(&snap, "in_scope");
    assert!(
        in_scope.contains("g_suspension"),
        "validFrom is inclusive: g_suspension holds at 2026-05-01"
    );
    assert!(
        graphs(&snap, "excluded").contains("g_adherent"),
        "validTo is exclusive: g_adherent no longer holds at 2026-05-01"
    );
}

#[test]
fn as_of_hides_a_fact_recorded_after_the_cutoff() {
    let t = temporal(CELL_LINE);
    // Valid at 2024-06-01, but as known one day before it was recorded.
    let before = snapshot(&t, Some("2024-06-01"), Some("2024-01-04"));
    assert!(
        graphs(&before, "excluded").contains("g_adherent"),
        "recordedAt 2024-01-05 is not yet visible as_of 2024-01-04"
    );
    assert!(
        graphs(&before, "in_scope").contains("g_species"),
        "a timeless graph carries no recorded time and stays visible"
    );

    // Inclusive at the cutoff: a record entered exactly at the audit instant is
    // already visible (`recordedAt <= as_of`), so a slip to a strict `<` would
    // hide it.
    let at = snapshot(&t, Some("2024-06-01"), Some("2024-01-05"));
    assert!(
        graphs(&at, "in_scope").contains("g_adherent"),
        "recordedAt 2024-01-05 is visible as_of 2024-01-05"
    );

    // One day later the record has certainly arrived.
    let after = snapshot(&t, Some("2024-06-01"), Some("2024-01-06"));
    assert!(graphs(&after, "in_scope").contains("g_adherent"));
}

/// A correction on the recorded axis alone: both graphs claim the same valid
/// period, and the first was believed only until the second replaced it. This
/// is the shape `as_of` exists for, and it needs a closing bound to work — with
/// an open transaction interval both graphs answer every later audit.
const CORRECTED: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g_first { ex:HEK293 a ex:AdherentCellLine . }
ex:g_fix   { ex:HEK293 a ex:SuspensionCellLine . }

{
  ex:g_first t:validFrom "2024-01-01" ; t:recordedAt "2024-01-05" ; t:recordedUntil "2026-05-02" .
  ex:g_fix   t:validFrom "2024-01-01" ; t:recordedAt "2026-05-02" .
}
"#;

#[test]
fn as_of_before_a_correction_sees_what_was_believed_then() {
    let t = temporal(CORRECTED);
    let snap = snapshot(&t, None, Some("2025-06-01"));
    assert_eq!(graphs(&snap, "in_scope"), set(&["g_first"]));
    assert_eq!(graphs(&snap, "excluded"), set(&["g_fix"]));
    assert_eq!(
        reason(&snap, "g_fix"),
        "not yet recorded then",
        "the successor had not been recorded at that instant"
    );
}

#[test]
fn as_of_after_a_correction_drops_the_assertion_it_replaced() {
    let t = temporal(CORRECTED);
    let snap = snapshot(&t, None, Some("2026-06-01"));
    // Without recordedUntil both graphs answer here, because `as_of` only ever
    // narrowed forward: the audit would show the corpus asserting two disjoint
    // types of the same subject and no way to tell which one was believed.
    assert_eq!(graphs(&snap, "in_scope"), set(&["g_fix"]));
    assert_eq!(graphs(&snap, "excluded"), set(&["g_first"]));
    assert_eq!(
        reason(&snap, "g_first"),
        "no longer recorded then",
        "an assertion whose recorded interval has closed is not merely \
         unrecorded yet — the two exclusions are opposite facts"
    );
}

#[test]
fn recorded_interval_is_half_open_at_the_upper_bound() {
    let t = temporal(CORRECTED);
    // Exactly at the bound. Half-open on both axes, so the predecessor is
    // already out and the successor is already in, and the handover leaves no
    // instant where both are believed or neither is.
    let snap = snapshot(&t, None, Some("2026-05-02"));
    assert_eq!(graphs(&snap, "in_scope"), set(&["g_fix"]));
    assert_eq!(graphs(&snap, "excluded"), set(&["g_first"]));

    // One instant earlier, the other way round.
    let before = snapshot(&t, None, Some("2026-05-01"));
    assert_eq!(graphs(&before, "in_scope"), set(&["g_first"]));
}

/// The failure this has to catch is not a wrong answer, it is a graph quietly
/// reclassified as timeless. `validities()` builds its map from the predicates
/// it queries, and `scope()` puts anything absent from that map into `in_scope`
/// as "no validity recorded, timeless" — that is, ALWAYS TRUE. So a predicate
/// added to the vocabulary and not added to the UNION does not merely go
/// unread: it turns the assertion it describes into an eternal one, which is
/// the worst available default for a bound that exists to withdraw something.
///
/// This test fails on `recordedUntil` before the UNION carries it, and it will
/// fail the same way for the fifth predicate someone adds without wiring.
#[test]
fn every_temporal_predicate_takes_its_graph_out_of_the_timeless_bucket() {
    for predicate in ["validFrom", "validTo", "recordedAt", "recordedUntil"] {
        let trig = format!(
            "@prefix ex: <http://example.org/> .\n\
             @prefix t:  <https://open-ontologies.org/temporal#> .\n\
             ex:g_only {{ ex:HEK293 a ex:AdherentCellLine . }}\n\
             {{ ex:g_only t:{predicate} \"2024-01-01\" . }}\n"
        );
        let snap = snapshot(&temporal(&trig), None, None);
        let row = snap["in_scope"]
            .as_array()
            .unwrap()
            .iter()
            .chain(snap["excluded"].as_array().unwrap().iter())
            .find(|row| row["graph"].as_str().is_some_and(|g| g.ends_with("g_only")))
            .unwrap_or_else(|| panic!("g_only is in neither bucket for `{predicate}`: {snap}"));
        assert_ne!(
            row["reason"].as_str(),
            Some("no validity recorded, timeless"),
            "`{predicate}` does not populate the validity map, so a graph \
             described only by it reads as timeless and always in scope"
        );
    }
}

/// A described graph with an open start: no `validFrom`, only a `validTo`.
const OPEN_START: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g_early { ex:Doc ex:status ex:Draft . }

{
  ex:g_early t:validTo "2024-01-01" .
}
"#;

#[test]
fn open_start_holds_since_always_up_to_its_upper_bound() {
    let t = temporal(OPEN_START);
    // No validFrom means "since always": in scope arbitrarily far in the past.
    // This is a described graph (it carries validTo), so it takes the temporal
    // branch, not the timeless one — the absent-validFrom case a fully
    // undescribed graph never reaches.
    assert!(
        graphs(&snapshot(&t, Some("1900-01-01"), None), "in_scope").contains("g_early"),
        "an absent validFrom holds since always"
    );
    // The upper bound is still exclusive.
    assert!(
        graphs(&snapshot(&t, Some("2024-01-01"), None), "excluded").contains("g_early"),
        "validTo remains exclusive with an open start"
    );
}

#[test]
fn open_end_still_holds_in_the_far_future() {
    // g_suspension has a validFrom and no validTo: still true indefinitely.
    let t = temporal(CELL_LINE);
    assert!(
        graphs(&snapshot(&t, Some("2099-01-01"), None), "in_scope").contains("g_suspension"),
        "an absent validTo holds indefinitely"
    );
}

/// The same shape as `CELL_LINE`, but written with the `xsd:date` and
/// `xsd:dateTime` typed literals the tool documents on disk. A typed bound is
/// read against the grammar its datatype names, an untyped one by shape alone,
/// and both must land on the same instants; this is the representation a
/// regression in the typed path would break while the untyped fixtures stayed
/// green.
const TYPED_LITERALS: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_typed { ex:HEK293 a ex:AdherentCellLine . }

{
  ex:g_typed t:validFrom  "2024-01-01"^^xsd:date ;
             t:validTo    "2026-05-01"^^xsd:date ;
             t:recordedAt "2024-01-05T09:30:00"^^xsd:dateTime .
}
"#;

#[test]
fn typed_literals_behave_like_their_lexical_form() {
    let t = temporal(TYPED_LITERALS);
    // Inside the valid interval and after the (dateTime) record: in scope.
    assert!(
        graphs(
            &snapshot(&t, Some("2024-06-01"), Some("2024-01-06")),
            "in_scope"
        )
        .contains("g_typed"),
        "typed date/dateTime literals compare like their lexical form"
    );
    // The dateTime record is not yet visible one day before it was entered.
    assert!(
        graphs(
            &snapshot(&t, Some("2024-06-01"), Some("2024-01-04")),
            "excluded"
        )
        .contains("g_typed")
    );
    // Before the typed validFrom: excluded, exactly as an untyped fixture is.
    assert!(graphs(&snapshot(&t, Some("2023-01-01"), None), "excluded").contains("g_typed"));
}

#[test]
fn query_at_only_reads_graphs_in_scope() {
    let t = temporal(CELL_LINE);
    let raw = t
        .query_at("{ ?s a ?type }", Some("2024-06-01"), None)
        .unwrap();
    let out: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let body = raw.as_str();
    assert!(
        body.contains("AdherentCellLine"),
        "the in-scope type is returned: {raw}"
    );
    assert!(
        !body.contains("SuspensionCellLine"),
        "the out-of-scope graph is not queried: {raw}"
    );
    assert_eq!(
        out["graphs_in_scope"], 2,
        "g_adherent and the timeless g_species are the two graphs in scope"
    );
}

#[test]
fn query_at_reports_an_empty_scope_rather_than_the_whole_store() {
    // No timeless graph here, and both described graphs begin in 2024, so an
    // instant in 2000 leaves nothing in scope.
    let t = temporal(CONFLICT_OVERLAP);
    let raw = t
        .query_at("{ ?s a ?type }", Some("2000-01-01"), None)
        .unwrap();
    let out: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(out["results"].as_array().unwrap().len(), 0);
    assert_eq!(out["note"], "no graphs in scope at that instant");
}

/// Two disjoint types asserted for the same subject over periods that share an
/// instant.
const CONFLICT_OVERLAP: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g_a { ex:X a ex:Adherent . }
ex:g_b { ex:X a ex:Suspension . }

{
  ex:Adherent owl:disjointWith ex:Suspension .
  ex:g_a t:validFrom "2024-01-01" ; t:validTo "2026-05-01" .
  ex:g_b t:validFrom "2024-06-01" ; t:validTo "2026-12-01" .
}
"#;

/// Same two disjoint types, but the periods meet at a boundary instead of
/// overlapping — a correction, not a contradiction.
const CONFLICT_SUPERSEDED: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g_a { ex:X a ex:Adherent . }
ex:g_b { ex:X a ex:Suspension . }

{
  ex:Adherent owl:disjointWith ex:Suspension .
  ex:g_a t:validFrom "2024-01-01" ; t:validTo "2026-05-01" .
  ex:g_b t:validFrom "2026-05-01" ; t:validTo "2026-12-01" .
}
"#;

#[test]
fn overlapping_disjoint_assertions_are_a_contradiction() {
    let t = temporal(CONFLICT_OVERLAP);
    let out: serde_json::Value = serde_json::from_str(&t.conflicts().unwrap()).unwrap();
    assert_eq!(
        out["contradiction_count"], 1,
        "overlapping validity + disjoint types = a contradiction: {out}"
    );
    assert_eq!(out["superseded_count"], 0);
    assert_eq!(out["contradictions"][0]["subject"], "X");
}

#[test]
fn boundary_touching_disjoint_assertions_are_superseded_not_contradictory() {
    let t = temporal(CONFLICT_SUPERSEDED);
    let out: serde_json::Value = serde_json::from_str(&t.conflicts().unwrap()).unwrap();
    assert_eq!(
        out["contradiction_count"], 0,
        "touching half-open periods do not overlap: {out}"
    );
    assert_eq!(
        out["non_overlapping_count"], 1,
        "the two periods share no instant: {out}"
    );
    assert_eq!(
        out["superseded_count"], 1,
        "the deprecated key carries the same set until 2.0"
    );
}

/// Two disjoint types with a GAP between their periods — nothing is asserted
/// about 2025 at all. `!overlaps` is true here exactly as it is for a touching
/// pair, which is why the bucket cannot be called `superseded`: this is missing
/// coverage, not a correction.
const CONFLICT_GAP: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g_a { ex:X a ex:Adherent . }
ex:g_b { ex:X a ex:Suspension . }

{
  ex:Adherent owl:disjointWith ex:Suspension .
  ex:g_a t:validFrom "2024-01-01" ; t:validTo "2025-01-01" .
  ex:g_b t:validFrom "2026-01-01" .
}
"#;

#[test]
fn a_gap_between_periods_is_not_a_contradiction_and_is_not_a_correction() {
    let t = temporal(CONFLICT_GAP);
    let out: serde_json::Value = serde_json::from_str(&t.conflicts().unwrap()).unwrap();
    assert_eq!(
        out["contradiction_count"], 0,
        "the periods share no instant, so they do not disagree: {out}"
    );
    assert_eq!(
        out["non_overlapping_count"], 1,
        "and that is the whole of what has been established: {out}"
    );
    // The old name says a correction happened. Nothing here shows one did:
    // the whole of 2025 is undescribed, and the pair lands in exactly the same
    // bucket as a touching pair does.
    assert_eq!(out["superseded_count"], 1);
}

#[test]
fn the_deprecated_key_carries_the_same_set_until_2_0() {
    for fixture in [CONFLICT_SUPERSEDED, CONFLICT_GAP, CONFLICT_OVERLAP] {
        let t = temporal(fixture);
        let out: serde_json::Value = serde_json::from_str(&t.conflicts().unwrap()).unwrap();
        assert_eq!(
            out["superseded"], out["non_overlapping"],
            "the deprecated key is emitted unconditionally and holds the same \
             rows, so no reader has to change on the day it appears: {out}"
        );
        assert_eq!(out["superseded_count"], out["non_overlapping_count"]);
    }
}

/// The `note` is output, and output can be wrong. It once claimed "one period
/// ends where the other begins", adjacency, when the only test performed is
/// `!overlaps`, which is disjointness. A gap satisfies it too. Nothing pinned
/// that sentence, which is how it survived; this pins what the code proves:
/// disjointness judged on bounds read as instants, and a third outcome for a
/// pair whose graph could not be read at all.
#[test]
fn the_note_claims_only_what_the_code_checks() {
    let t = temporal(CONFLICT_GAP);
    let out: serde_json::Value = serde_json::from_str(&t.conflicts().unwrap()).unwrap();
    let note = out["note"].as_str().expect("note is a string");
    assert!(
        note.contains("no instant in common"),
        "the note must state the check that was performed: {note}"
    );
    assert!(
        note.contains("gap") || note.contains("GAP"),
        "and must name the case that is missing coverage rather than history: {note}"
    );
    assert!(
        note.contains("read as instants"),
        "and must name the comparison that backs the guarantee, which is on instants: {note}"
    );
    assert!(
        !note.contains("lexical") && !note.contains("LEXICAL"),
        "bounds are no longer compared as text, so the note may not say they are: {note}"
    );
    assert!(
        note.contains("undecided") && note.contains("could not be read"),
        "and must say why a pair can be in neither bucket: {note}"
    );
    assert!(
        !note.contains("one period ends where the other begins"),
        "adjacency is never established by `!overlaps`: {note}"
    );
}

/// Two periods that DO share an hour, written with different timezone offsets.
/// `2026-05-01T00:00:00-02:00` is `02:00Z`, so the first period runs past the
/// `01:00Z` start of the second.
const CONFLICT_OFFSET_OVERLAP: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_a { ex:X a ex:Adherent . }
ex:g_b { ex:X a ex:Suspension . }

{
  ex:Adherent owl:disjointWith ex:Suspension .
  ex:g_a t:validFrom "2026-04-01T00:00:00Z"^^xsd:dateTime ;
         t:validTo   "2026-05-01T00:00:00-02:00"^^xsd:dateTime .
  ex:g_b t:validFrom "2026-05-01T01:00:00Z"^^xsd:dateTime .
}
"#;

/// Pinned before the parsed-time work (#95, item 4) as an ACCIDENT of comparing
/// datatype-stripped literals as text: `"...T01:00:00Z"` sorted after
/// `"...T00:00:00-02:00"`, so the two periods read as disjoint and a pair that
/// shares an hour on the real timeline was classified `non_overlapping`. That
/// pin was written so this change would land as a diff on an assertion that
/// already existed rather than as a new one, and this is that diff: bounds
/// are instants, the offset is honoured, and the same fixture is the genuine
/// contradiction it always was.
#[test]
fn offset_bounds_are_compared_as_instants_so_a_real_overlap_is_a_contradiction() {
    let t = temporal(CONFLICT_OFFSET_OVERLAP);
    let out: serde_json::Value = serde_json::from_str(&t.conflicts().unwrap()).unwrap();
    assert_eq!(
        out["contradiction_count"], 1,
        "2026-05-01T00:00:00-02:00 is 02:00Z, an hour past the 01:00Z start of the other \
         period, so the two share an instant: {out}"
    );
    assert_eq!(
        out["non_overlapping_count"], 0,
        "and a pair that shares an instant is not in the bucket that promises none: {out}"
    );
}

// ---------------------------------------------------------------------------
// Section 2: the inputs parsed-time reading changes
//
// Everything above pins semantics that are intended, and none of it moved.
// What follows was pinned in 1.2.0 as ACCIDENTS of comparing datatype-stripped
// literals lexically rather than as decisions, each test naming what issue #95
// item 4 would turn it into. Item 4 has landed: same fixtures, same inputs, and
// the answer each one gives now, a diff on the lines that held the old answer,
// which is what this section was written for.
//
// Bounds are read as instants on the UTC timeline, accepting xsd:date,
// xsd:dateTime, xsd:gYearMonth and xsd:gYear. A less precise bound names the
// FIRST instant of the period it names. A value with no offset is UTC. A bound
// matching none of the four grammars is invalid, and so is a graph asserting
// two different instants on one axis; invalid graphs are reported in their own
// bucket and never fall into the timeless one.
// ---------------------------------------------------------------------------

/// `xsd:date` lower bound, `xsd:dateTime` upper bound, same axis.
const MIXED_PRECISION: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_mixed { ex:X a ex:A . }

{
  ex:g_mixed t:validFrom "2024-01-01"^^xsd:date ;
             t:validTo   "2026-05-01T00:00:00Z"^^xsd:dateTime .
}
"#;

#[test]
fn a_less_precise_instant_names_the_first_moment_of_its_period() {
    let t = temporal(MIXED_PRECISION);
    // WAS ACCIDENTAL: "2026-05-01" is a lexical prefix of
    // "2026-05-01T00:00:00Z", so the half-open `instant < to` test passed on
    // text and the whole day read as in scope.
    // NOW: the date names the first instant of 1 May, which is exactly `to`,
    // and a half-open interval excludes its upper bound.
    assert!(
        graphs(&snapshot(&t, Some("2026-05-01"), None), "excluded").contains("g_mixed"),
        "the date-typed instant is the upper bound itself, so it is excluded"
    );
    // Unchanged: a day inside the interval stays in scope.
    assert!(graphs(&snapshot(&t, Some("2026-04-30"), None), "in_scope").contains("g_mixed"));
}

/// A lower bound carrying a `+02:00` offset against an instant expressed in Z.
const OFFSET_BOUND: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_off { ex:X a ex:A . }

{
  ex:g_off t:validFrom "2026-05-01T00:00:00+02:00"^^xsd:dateTime ;
           t:validTo   "2026-06-01T00:00:00Z"^^xsd:dateTime .
}
"#;

#[test]
fn an_offset_bound_is_read_as_the_instant_it_denotes() {
    let t = temporal(OFFSET_BOUND);
    // 2026-05-01T00:00:00+02:00 is 2026-04-30T22:00:00Z, so 23:00Z that day is
    // one hour INSIDE the interval.
    // WAS ACCIDENTAL: compared as text, "2026-05-…" sorts after "2026-04-…",
    // so the bound read as later than it is and the instant fell outside.
    // NOW: the offset is applied and the instant is where it always was.
    assert!(
        graphs(
            &snapshot(&t, Some("2026-04-30T23:00:00Z"), None),
            "in_scope"
        )
        .contains("g_off"),
        "an hour past a +02:00 bound is inside the interval, whatever the text sorts like"
    );
    // Unchanged: an instant past both readings stays in scope.
    assert!(
        graphs(
            &snapshot(&t, Some("2026-05-01T00:00:00Z"), None),
            "in_scope"
        )
        .contains("g_off")
    );
}

/// A bound carrying a non-ISO lexical form.
///
/// It is written `^^xsd:string`, but RDF 1.1 makes that the SAME term as the
/// bare literal `"01/05/2026"` (`datatype()` answers `xsd:string` for both),
/// so nothing downstream can reject it for its datatype. It is rejected for
/// its shape, which is what makes the rule implementable at all.
const STRING_TYPED_BOUND: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_str { ex:X a ex:A . }

{
  ex:g_str t:validFrom "01/05/2026"^^xsd:string .
}
"#;

#[test]
fn a_bound_matching_no_temporal_grammar_is_invalid_rather_than_coerced() {
    let t = temporal(STRING_TYPED_BOUND);
    // WAS ACCIDENTAL: `plain()` threw the datatype away, leaving "01/05/2026",
    // which sorts before every ISO value in the store, so the graph read as
    // valid since the beginning of time, including in 1999.
    // NOW: the value matches none of the four grammars, so the graph is
    // INVALID: out of in_scope, reported with a reason, and above all NOT
    // timeless. "We hold no valid-time claim about this" and "the claim is
    // garbage" are different answers.
    for instant in ["2020-01-01", "1999-01-01"] {
        let snap = snapshot(&t, Some(instant), None);
        assert!(
            !graphs(&snap, "in_scope").contains("g_str"),
            "an unreadable bound must not put its graph in scope (at {instant})"
        );
        assert!(
            !graphs(&snap, "excluded").contains("g_str"),
            "nor in excluded, which claims we read the period and it did not hold"
        );
        let reason = invalid_reason(&snap, "g_str")
            .unwrap_or_else(|| panic!("g_str is reported unreadable: {snap}"));
        assert!(
            reason.contains("validFrom") && reason.contains("01/05/2026"),
            "the reason names the field and quotes the value: {reason}"
        );
    }
}

/// One graph carrying two distinct `validFrom` values.
const MULTI_VALUED_BOUND: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_multi { ex:X a ex:A . }

{
  ex:g_multi t:validFrom "2024-01-01"^^xsd:date , "2026-05-01"^^xsd:date .
}
"#;

#[test]
fn two_different_instants_on_one_axis_are_a_data_error_not_a_choice() {
    let t = temporal(MULTI_VALUED_BOUND);
    // WAS ACCIDENTAL: `validities()` overwrote the field on every row of the
    // validity UNION, so one value won and the other vanished: whichever the
    // UNION happened to yield last, an order the query never contracted.
    // NOW: the graph is INVALID and both values are listed. Resolving to one,
    // or to the min, or to the max, would publish an interval nobody asserted.
    let snap = snapshot(&t, Some("2026-06-01"), None);
    // 2026-06-01 is after BOTH candidate lower bounds, so under the old
    // last-row-wins reading the graph was in scope whichever row survived.
    assert!(
        !graphs(&snap, "in_scope").contains("g_multi"),
        "an interval nobody asserted must not be published: {snap}"
    );
    let reason = invalid_reason(&snap, "g_multi")
        .unwrap_or_else(|| panic!("g_multi is reported unreadable: {snap}"));
    assert!(
        reason.contains("2024-01-01") && reason.contains("2026-05-01"),
        "BOTH asserted values are named, not one of them: {reason}"
    );
}

// ---------------------------------------------------------------------------
// Section 3: what reading bounds as instants adds
//
// The cases Section 2 does not reach: values that are readable but were never
// comparable as text, and values that must be refused. Every fixture here is
// data a real store produces: a half-finished migration, a coarse register
// bound, a mislabelled datatype, a typo.
// ---------------------------------------------------------------------------

/// The same instant asserted twice on one axis, in two spellings. A bare
/// literal beside its typed twin is what a half-finished migration leaves.
const TWO_SPELLINGS: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_twin { ex:X a ex:A . }

{
  ex:g_twin t:validFrom "2024-01-01" , "2024-01-01"^^xsd:date .
}
"#;

#[test]
fn two_spellings_of_one_instant_are_one_assertion() {
    let t = temporal(TWO_SPELLINGS);
    let snap = snapshot(&t, Some("2024-06-01"), None);
    // Two terms, one instant: nothing is ambiguous and no interval has to be
    // invented, so this is not the multi-valued data error. Reading it as one
    // is what keeps a store mid-migration answering as it did in 1.2.0, where
    // both rows collapsed to the same string and last-row-wins was a no-op.
    assert!(
        graphs(&snap, "in_scope").contains("g_twin"),
        "two spellings of one instant assert one interval: {snap}"
    );
    assert!(
        invalid_reason(&snap, "g_twin").is_none(),
        "and nothing about it is unreadable: {snap}"
    );
}

/// The same instant asserted at two precisions on one axis: a year beside
/// the day that year begins on. A register that recorded "2024" and a later
/// pass that wrote out the day it stands for is the shape this leaves.
const TWO_PRECISIONS: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_coarse { ex:X a ex:A . }

{
  ex:g_coarse t:validFrom "2024"^^xsd:gYear , "2024-01-01"^^xsd:date .
}
"#;

/// `settle` compares the INSTANTS the terms name, not their lexical forms:
/// `"2024"` and `"2024-01-01"` are different strings naming one instant, so
/// they are one assertion, exactly as two spellings at one precision are.
///
/// The displayed form is pinned as well. The row shows `2024`, the lexically
/// first term after the sort, because a row carries one form per bound and
/// showing both would need a shape change. This pins the current choice so a
/// change to it is deliberate, not an endorsement of it.
#[test]
fn a_coarse_bound_and_a_fine_one_naming_the_same_instant_are_one_assertion() {
    let t = temporal(TWO_PRECISIONS);
    let snap = snapshot(&t, Some("2024-06-01"), None);
    assert!(
        graphs(&snap, "in_scope").contains("g_coarse"),
        "a year and the day it begins on are one instant, so one interval: {snap}"
    );
    assert!(
        invalid_reason(&snap, "g_coarse").is_none(),
        "and nothing about it is a data error: {snap}"
    );
    let row = snap["in_scope"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["graph"]
                .as_str()
                .is_some_and(|g| g.ends_with("g_coarse"))
        })
        .unwrap_or_else(|| panic!("g_coarse is in scope: {snap}"));
    assert_eq!(
        row["valid"], "2024 to still true",
        "the shown form is the lexically first term, `2024`: {row}"
    );
}

/// Register data is often coarser than a day.
const COARSE_BOUNDS: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_year  { ex:X a ex:A . }
ex:g_month { ex:Y a ex:A . }

{
  ex:g_year  t:validFrom "2024"^^xsd:gYear ;
             t:validTo   "2026"^^xsd:gYear .
  ex:g_month t:validFrom "2024-03"^^xsd:gYearMonth .
}
"#;

#[test]
fn a_year_or_month_bound_names_the_first_instant_of_its_period() {
    let t = temporal(COARSE_BOUNDS);
    // "2024" is 2024-01-01T00:00:00Z, so the first instant of 2024 is in.
    assert!(graphs(&snapshot(&t, Some("2024-01-01"), None), "in_scope").contains("g_year"));
    // "2026" as an upper bound is the first instant of 2026, and the interval
    // is half-open, so 2025 is the last year in scope, not 2026.
    assert!(graphs(&snapshot(&t, Some("2025-12-31"), None), "in_scope").contains("g_year"));
    assert!(graphs(&snapshot(&t, Some("2026-01-01"), None), "excluded").contains("g_year"));
    // Same rule one precision down.
    assert!(graphs(&snapshot(&t, Some("2024-03-01"), None), "in_scope").contains("g_month"));
    assert!(graphs(&snapshot(&t, Some("2024-02-29"), None), "excluded").contains("g_month"));
}

/// A value that is a perfectly good instant wearing the wrong datatype. This
/// is the shape this crate's own module doc shipped, so a store built by
/// following the documentation must keep answering.
const MISLABELLED_DATATYPE: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_doc { ex:X a ex:A . }

{
  ex:g_doc t:validFrom  "2024-01-01"^^xsd:date ;
           t:recordedAt "2024-01-05"^^xsd:dateTime .
}
"#;

#[test]
fn a_value_wearing_the_wrong_temporal_datatype_is_still_read() {
    let t = temporal(MISLABELLED_DATATYPE);
    // "2024-01-05" is not in the lexical space of xsd:dateTime. The re-read
    // stays inside the four temporal grammars, so this resolves to the date it
    // plainly is, while a value fitting none of them is still refused, which
    // is what `a_bound_matching_no_temporal_grammar_is_invalid…` pins.
    let snap = snapshot(&t, Some("2024-06-01"), Some("2024-01-05"));
    assert!(
        invalid_reason(&snap, "g_doc").is_none(),
        "the documented shape must keep answering: {snap}"
    );
    assert!(graphs(&snap, "in_scope").contains("g_doc"));
    // And the recorded-time cutoff still bites on the other side of it.
    assert!(
        graphs(
            &snapshot(&t, Some("2024-06-01"), Some("2024-01-04")),
            "excluded"
        )
        .contains("g_doc")
    );
}

/// Bounds nothing can read: a datatype outside the four, a language tag, and
/// a date that never happened.
const UNREADABLE_BOUNDS: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_token { ex:X a ex:A . }
ex:g_lang  { ex:Y a ex:A . }
ex:g_never { ex:Z a ex:A . }

{
  ex:g_token t:validFrom "2024-01-01"^^xsd:token .
  ex:g_lang  t:validFrom "2024-01-01"@en .
  ex:g_never t:validFrom "2024-02-30"^^xsd:date .
}
"#;

#[test]
fn a_foreign_datatype_a_language_tag_and_an_impossible_date_are_all_invalid() {
    let t = temporal(UNREADABLE_BOUNDS);
    let snap = snapshot(&t, Some("2024-06-01"), None);
    for graph in ["g_token", "g_lang", "g_never"] {
        assert!(
            invalid_reason(&snap, graph).is_some(),
            "{graph} carries a bound nothing can read: {snap}"
        );
        assert!(
            !graphs(&snap, "in_scope").contains(graph),
            "and it must never reach the timeless bucket: {snap}"
        );
    }
    // 30 February parses field by field and is still not a day: the calendar
    // check is what catches it, not the grammar.
    assert!(
        invalid_reason(&snap, "g_never")
            .unwrap()
            .contains("2024-02-30")
    );
}

#[test]
fn an_unreadable_argument_is_refused_rather_than_ignored() {
    let t = temporal(CELL_LINE);
    // Dropping it would answer a question nobody asked: the whole store in
    // scope, with nothing saying why.
    assert!(t.snapshot(Some("01/05/2026"), None).is_err());
    assert!(t.snapshot(None, Some("last tuesday")).is_err());
    assert!(t.query_at("{ ?s ?p ?o }", Some(""), None).is_err());
    // And a readable one still works, at every precision.
    assert!(t.snapshot(Some("2026"), None).is_ok());
    assert!(t.snapshot(Some("2026-05-01T12:00:00+02:00"), None).is_ok());
}

/// The closing bound of the recorded interval, written three ways: one that
/// nothing can read, one carrying an offset, and one at month precision.
const RECORDED_UNTIL_FORMS: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_garbage { ex:X a ex:A . }
ex:g_offset  { ex:Y a ex:A . }
ex:g_month   { ex:Z a ex:A . }

{
  ex:g_garbage t:recordedAt "2024-01-05" ; t:recordedUntil "01/05/2026"^^xsd:string .
  ex:g_offset  t:recordedAt "2024-01-05" ; t:recordedUntil "2026-05-02T00:00:00+02:00"^^xsd:dateTime .
  ex:g_month   t:recordedAt "2024-01-05" ; t:recordedUntil "2026-05"^^xsd:gYearMonth .
}
"#;

/// `recordedUntil` is the fourth parsed axis and fails the way the other three
/// do: a value matching no temporal grammar makes the WHOLE graph invalid,
/// with the reason naming the axis, rather than leaving the recorded interval
/// open or the graph timeless. Whole-graph invalidation is the rule for every
/// axis; there is no per-axis fault.
#[test]
fn an_unreadable_recorded_until_makes_the_graph_invalid_like_any_other_axis() {
    let t = temporal(RECORDED_UNTIL_FORMS);
    for as_of in [None, Some("2025-01-01")] {
        let snap = snapshot(&t, None, as_of);
        assert!(
            !graphs(&snap, "in_scope").contains("g_garbage"),
            "an unreadable closing bound must not leave the graph in scope (as_of {as_of:?})"
        );
        assert!(
            !graphs(&snap, "excluded").contains("g_garbage"),
            "nor in excluded, which claims the interval was read and had closed"
        );
        let reason = invalid_reason(&snap, "g_garbage")
            .unwrap_or_else(|| panic!("g_garbage is reported unreadable: {snap}"));
        assert!(
            reason.contains("recordedUntil") && reason.contains("01/05/2026"),
            "the reason names the axis and quotes the value: {reason}"
        );
    }
}

/// The closing bound is compared as the instant it names, not as text.
/// `2026-05-02T00:00:00+02:00` is `2026-05-01T22:00:00Z`, so an audit at
/// `23:00Z` on 1 May is already past it although the text says 2 May; and
/// `"2026-05"` closes the interval at the first instant of May, not somewhere
/// inside it.
#[test]
fn recorded_until_with_an_offset_or_at_coarse_precision_closes_at_its_instant() {
    let t = temporal(RECORDED_UNTIL_FORMS);
    let snap = snapshot(&t, None, Some("2026-05-01T23:00:00Z"));
    assert_eq!(
        reason(&snap, "g_offset"),
        "no longer recorded then",
        "22:00Z on 1 May has passed by 23:00Z, whatever the text sorts like: {snap}"
    );
    assert_eq!(
        reason(&snap, "g_month"),
        "no longer recorded then",
        "May has begun, so a month-precision bound has closed: {snap}"
    );
    // Two hours earlier the offset bound has not closed yet.
    let before = snapshot(&t, None, Some("2026-05-01T21:00:00Z"));
    assert!(
        graphs(&before, "in_scope").contains("g_offset"),
        "21:00Z on 1 May is before 22:00Z: {before}"
    );
    // And the last second of April is still inside the month-bounded interval.
    let april = snapshot(&t, None, Some("2026-04-30T23:59:59Z"));
    assert!(
        graphs(&april, "in_scope").contains("g_month"),
        "April is before the first instant of May: {april}"
    );
}

/// A disjointness pair where one side's period cannot be read.
const UNREADABLE_IN_A_PAIR: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_a { ex:X a ex:Adherent . }
ex:g_b { ex:X a ex:Suspension . }

{
  ex:Adherent owl:disjointWith ex:Suspension .
  ex:g_a t:validFrom "2024-01-01" ; t:validTo "2026-05-01" .
  ex:g_b t:validFrom "01/05/2026"^^xsd:string .
}
"#;

#[test]
fn a_pair_whose_period_cannot_be_read_is_undecided_not_a_contradiction() {
    let t = temporal(UNREADABLE_IN_A_PAIR);
    let out: serde_json::Value = serde_json::from_str(&t.conflicts().unwrap()).unwrap();
    // An unreadable period is not an open one. Treating it as timeless would
    // make it overlap everything and publish this pair as a live contradiction:
    // the false positive the tool exists to prevent, arriving through the
    // data instead of through a truncated scan. Nor is it a disjoint one, so
    // the pair is in neither bucket.
    assert_eq!(out["contradiction_count"], 0, "{out}");
    assert_eq!(out["non_overlapping_count"], 0, "{out}");
    assert_eq!(out["superseded_count"], 0, "{out}");
    assert_eq!(out["undecided_count"], 1, "{out}");
    assert!(
        out["undecided"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("could not be read"),
        "{out}"
    );
}

#[test]
fn every_answer_names_the_semantics_that_produced_it() {
    let t = temporal(CELL_LINE);
    // Two readings of the same store disagree, so an answer that does not say
    // which one produced it cannot be replayed or hashed, which is what the
    // snapshot manifest work needs from this PR.
    for raw in [
        t.snapshot(None, None).unwrap(),
        t.query_at("{ ?s ?p ?o }", None, None).unwrap(),
        t.conflicts().unwrap(),
    ] {
        let out: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(out["semantics_version"], "temporal/2", "{out}");
    }
}
