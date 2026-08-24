//! Bi-temporal conformance corpus for `onto_temporal_*` (issue #95).
//!
//! Pins the CURRENT semantics of `Temporal::snapshot`, `query_at` and
//! `conflicts` against a small table of named-graph datasets: half-open
//! validity with an exclusive upper bound, open bounds, recorded-time
//! visibility, timeless (undescribed) graphs, snapshot-scoped querying, and
//! the overlap-versus-superseded split that is the whole point of carrying
//! valid time. Every assertion holds on the code as shipped in 1.2.0, so these
//! are behavioural anchors, not aspirations: a later change to the temporal
//! semantics has to move a line here on purpose rather than drift silently.
//!
//! The file has two sections. The first pins semantics that are **intended**;
//! most of it uses a single lexical date form, since that is the shape those
//! assertions are about. The second pins what the code does **today** for
//! mixed-precision and malformed bounds — behaviour that falls out of comparing
//! datatype-stripped literals lexically rather than being chosen. Each test
//! there names what the planned parsed-time work (#95, item 4) turns it into,
//! so that change lands as a diff on assertions that already exist instead of
//! arriving as new ones the corpus could never have failed on.
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
/// `xsd:dateTime` typed literals the tool documents on disk. `Temporal::plain`
/// strips the datatype, so these must behave exactly like their lexical forms;
/// this is the representation a regression in that stripping would break while
/// the untyped fixtures stayed green.
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
        graphs(&snapshot(&t, Some("2024-06-01"), Some("2024-01-06")), "in_scope")
            .contains("g_typed"),
        "typed date/dateTime literals compare like their lexical form"
    );
    // The dateTime record is not yet visible one day before it was entered.
    assert!(
        graphs(&snapshot(&t, Some("2024-06-01"), Some("2024-01-04")), "excluded")
            .contains("g_typed")
    );
    // Before the typed validFrom: excluded, exactly as an untyped fixture is.
    assert!(
        graphs(&snapshot(&t, Some("2023-01-01"), None), "excluded").contains("g_typed")
    );
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
    let raw = t.query_at("{ ?s a ?type }", Some("2000-01-01"), None).unwrap();
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

/// The `note` is output, and output can be wrong. It claimed "one period ends
/// where the other begins" — adjacency — when the only test performed is
/// `!overlaps`, which is disjointness. A gap satisfies it too. Nothing pinned
/// that sentence, which is how it survived; this pins what the code proves.
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
        note.contains("LEXICAL COMPARISON") || note.contains("lexical"),
        "and must qualify the guarantee by the comparison that backs it: {note}"
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

/// ACCIDENTAL, pinned on purpose so the parsed-time work (#95, item 4) lands as
/// a diff on an assertion that already exists rather than as a new one.
///
/// `overlaps()` compares datatype-stripped literals as TEXT. `"…T01:00:00Z"`
/// sorts after `"…T00:00:00-02:00"`, so the two periods read as disjoint and
/// the pair is classified `non_overlapping` — while on the real timeline they
/// share an hour and are a genuine contradiction.
///
/// This is why the `note` qualifies its guarantee with the comparison that
/// backs it: the key must not promise more than `overlaps()` can establish.
///
/// AFTER item 4: `contradiction_count` is 1 and `non_overlapping_count` is 0.
#[test]
fn accidental_offset_bounds_can_hide_a_real_overlap_from_the_disjointness_check() {
    let t = temporal(CONFLICT_OFFSET_OVERLAP);
    let out: serde_json::Value = serde_json::from_str(&t.conflicts().unwrap()).unwrap();
    assert_eq!(
        out["contradiction_count"], 0,
        "today the offsets are compared as text, so the real overlap is invisible: {out}"
    );
    assert_eq!(
        out["non_overlapping_count"], 1,
        "and the pair lands in the bucket whose guarantee it violates: {out}"
    );
}

// ---------------------------------------------------------------------------
// Section 2 — accidental behaviour, pinned so it can be flipped on purpose
//
// Everything above pins semantics that are intended. What follows pins what the
// code does TODAY for mixed-precision and malformed bounds, which is a side
// effect of comparing datatype-stripped literals lexically (`plain()` at
// temporal.rs:229-242, `validities()` at :309-317) rather than a decision.
//
// These are the exact inputs the planned parsed-time work (issue #95, item 4)
// changes: bounds parsed as instants on the UTC timeline accepting xsd:date,
// xsd:dateTime, xsd:gYearMonth and xsd:gYear; a less precise bound mapping to
// the first instant of the period it names; timezone-less values read as UTC;
// any other datatype rejected rather than coerced; and multi-valued bounds
// marked invalid instead of resolved. Each test below says what it will become,
// so that work lands as a diff on existing assertions rather than as new ones.
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
fn accidental_mixed_precision_upper_bound_admits_its_own_day() {
    let t = temporal(MIXED_PRECISION);
    // ACCIDENTAL: "2026-05-01" is a lexical prefix of "2026-05-01T00:00:00Z",
    // so the half-open `instant < to` test passes and the day is in scope.
    // Parsed, the date maps to 2026-05-01T00:00:00Z, which is exactly `to`,
    // so the same instant is excluded.
    // AFTER item 4: this graph is EXCLUDED at 2026-05-01.
    assert!(
        graphs(&snapshot(&t, Some("2026-05-01"), None), "in_scope").contains("g_mixed"),
        "today the dateTime upper bound does not exclude its own date-typed instant"
    );
    // Unchanged by item 4: a day inside the interval stays in scope.
    assert!(
        graphs(&snapshot(&t, Some("2026-04-30"), None), "in_scope").contains("g_mixed")
    );
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
fn accidental_offset_bound_compares_as_text_not_as_an_instant() {
    let t = temporal(OFFSET_BOUND);
    // 2026-05-01T00:00:00+02:00 is 2026-04-30T22:00:00Z, so 23:00Z that day is
    // one hour INSIDE the interval.
    // ACCIDENTAL: compared as text, "2026-05-…" sorts after "2026-04-…", so the
    // bound reads as later than it is and the instant falls outside.
    // AFTER item 4: this graph is IN SCOPE at 2026-04-30T23:00:00Z.
    assert!(
        graphs(&snapshot(&t, Some("2026-04-30T23:00:00Z"), None), "excluded").contains("g_off"),
        "today an offset bound is compared lexically, not as an instant"
    );
    // Unchanged by item 4: an instant past both readings stays in scope.
    assert!(
        graphs(&snapshot(&t, Some("2026-05-01T00:00:00Z"), None), "in_scope").contains("g_off")
    );
}

/// A bound whose datatype is `xsd:string`, carrying a non-ISO lexical form.
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
fn accidental_string_typed_bound_is_coerced_instead_of_rejected() {
    let t = temporal(STRING_TYPED_BOUND);
    // ACCIDENTAL: `plain()` throws the datatype away, leaving "01/05/2026",
    // which sorts before every ISO value in the store. The graph therefore
    // reads as valid since the beginning of time — including in 1999.
    // AFTER item 4: the datatype is outside the accepted set, so the graph is
    // INVALID: excluded from in_scope and reported with a reason, never
    // silently in scope and never in the timeless bucket.
    for instant in ["2020-01-01", "1999-01-01"] {
        assert!(
            graphs(&snapshot(&t, Some(instant), None), "in_scope").contains("g_str"),
            "today an xsd:string bound is coerced and sorts before ISO values (at {instant})"
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
fn accidental_multi_valued_bound_resolves_to_one_row_instead_of_failing() {
    let t = temporal(MULTI_VALUED_BOUND);
    // ACCIDENTAL: `validities()` overwrites the field on every row of the
    // 4-way UNION, so one of the two values wins and the other vanishes. Which
    // one is whichever the UNION yields last — today that is 2024-01-01, but
    // nothing in the query contracts that order, which is the defect.
    // AFTER item 4: the graph is INVALID, both values are listed in the reason,
    // and it is neither resolved to one of them nor treated as timeless.
    let snap = snapshot(&t, Some("2026-06-01"), None);
    // 2026-06-01 is after BOTH candidate lower bounds, so this assertion holds
    // whichever row wins — it pins "resolved to a single interval and admitted"
    // without depending on the iteration order.
    assert!(
        graphs(&snap, "in_scope").contains("g_multi"),
        "today a multi-valued bound still yields one usable interval"
    );
    // Exactly one of the two values is reported, never both.
    let described = snap["in_scope"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["graph"].as_str().unwrap().ends_with("g_multi"))
        .and_then(|r| r["valid"].as_str())
        .unwrap_or_default()
        .to_string();
    let mentions_early = described.contains("2024-01-01");
    let mentions_late = described.contains("2026-05-01");
    assert!(
        mentions_early ^ mentions_late,
        "exactly one validFrom survives today (last row of the UNION wins): {described}"
    );
    // And nothing anywhere says the graph is malformed.
    assert!(
        snap.get("invalid").is_none(),
        "today there is no invalid bucket to report this in: {snap}"
    );
}
