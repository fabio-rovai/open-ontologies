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
//! The file has five sections. The first pins semantics that are **intended**;
//! most of it uses a single lexical date form, since that is the shape those
//! assertions are about, and none of it moved when bounds started being parsed.
//! The second pinned what the code did for mixed-precision and malformed bounds
//! when they were compared as datatype-stripped text, behaviour that fell out
//! of the implementation rather than being chosen, with each test naming what
//! the parsed-time work (#95, item 4) would turn it into; that work has landed,
//! so those tests now hold the new answer on the same lines, which is what the
//! section was for. The third covers what parsing adds and could not be
//! expressed before: coarse bounds, offsets, and bounds that must be refused.
//! The fourth covers periods that hold at no instant (#118), led by the case
//! parsing created: two readable bounds at different precisions naming one
//! instant. The fifth covers lineage that is asserted, never inferred (#109):
//! `supersedes` and `retracts`, the closing bound derived from a successor and
//! never written, the `retracted` bucket, the `lineage` reports and the
//! `corrections` bucket, led by the issue's own example.
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
/// fail the same way for the seventh predicate someone adds without wiring.
/// Each predicate gets an object of its own type: the four bounds a date,
/// the two lineage links a graph IRI, since a literal is not a legal object
/// for them and would test the report rather than the map.
#[test]
fn every_temporal_predicate_takes_its_graph_out_of_the_timeless_bucket() {
    for (predicate, object) in [
        ("validFrom", "\"2024-01-01\""),
        ("validTo", "\"2024-01-01\""),
        ("recordedAt", "\"2024-01-01\""),
        ("recordedUntil", "\"2024-01-01\""),
        ("supersedes", "ex:g_other"),
        ("retracts", "ex:g_other"),
    ] {
        let trig = format!(
            "@prefix ex: <http://example.org/> .\n\
             @prefix t:  <https://open-ontologies.org/temporal#> .\n\
             ex:g_only {{ ex:HEK293 a ex:AdherentCellLine . }}\n\
             {{ ex:g_only t:{predicate} {object} . }}\n"
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
        note.contains("holds at no instant") && note.contains("empty or inverted"),
        "and must say the bucket holds a period with no instant inside it, and name \
         both shapes, since the snapshot's reason is the only place they differ: {note}"
    );
    assert!(
        !note.contains("one period ends where the other begins"),
        "adjacency is never established by `!overlaps`: {note}"
    );
    assert!(
        note.contains("corrections") && note.contains("asserted and never inferred"),
        "and must say what puts a pair in corrections, which is an asserted link and \
         nothing about its periods: {note}"
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

// ---------------------------------------------------------------------------
// Section 4: periods that hold at no instant (#118)
//
// Two bounds that both read, and together name no instant: validFrom and
// validTo naming the same instant is an empty period, validTo before validFrom
// an inverted one. Neither is unreadable, so neither belongs in `invalid`,
// whose every reason says a bound could not be read. `valid_at` was already
// false everywhere for both; `overlaps` was not, so the snapshot excluded such
// a graph at every instant while conflicts filed it as a contradiction partner.
// The classification is now made once, where the bounds are read, and both
// tools answer from it: excluded at every instant asked about, with a reason
// naming the shape, and read like any other graph when no valid_at is given,
// since no instant was asked about. The mixed-precision fixture leads because
// it is the case parsing created: two careful assertions from two systems,
// invisible on inspection.
// ---------------------------------------------------------------------------

/// "Valid for 2024" beside "until 1 January 2024". Each bound is what a
/// careful system writes at its own precision, and the two together name one
/// instant, so the period is empty. Nothing on the page says so.
const EMPTY_MIXED_PRECISION: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_empty { ex:X a ex:Adherent . }
ex:g_live  { ex:X a ex:Suspension . }

{
  ex:Adherent owl:disjointWith ex:Suspension .
  ex:g_empty t:validFrom "2024"^^xsd:gYear ; t:validTo "2024-01-01"^^xsd:date .
  ex:g_live  t:validFrom "2023-01-01" ; t:validTo "2025-01-01" .
}
"#;

/// The same literal on both bounds: the shape that was visible before parsing.
const EMPTY_SAME_LITERAL: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g_empty { ex:X a ex:Adherent . }
ex:g_live  { ex:X a ex:Suspension . }

{
  ex:Adherent owl:disjointWith ex:Suspension .
  ex:g_empty t:validFrom "2024-06-01" ; t:validTo "2024-06-01" .
  ex:g_live  t:validFrom "2023-01-01" ; t:validTo "2025-01-01" .
}
"#;

/// validTo six years before validFrom, beside a graph still true. Nothing
/// checked `from < to`, so this read as a period ending in 2020 that a 2023
/// assertion then corrected.
const INVERTED: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g_inverted { ex:X a ex:Adherent . }
ex:g_live     { ex:X a ex:Suspension . }

{
  ex:Adherent owl:disjointWith ex:Suspension .
  ex:g_inverted t:validFrom "2026-01-01" ; t:validTo "2020-01-01" .
  ex:g_live     t:validFrom "2023-01-01" .
}
"#;

fn conflicts(t: &Temporal) -> serde_json::Value {
    serde_json::from_str(&t.conflicts().unwrap()).unwrap()
}

/// The local parts of every graph named in any row of one conflicts bucket.
fn partners(out: &serde_json::Value, bucket: &str) -> BTreeSet<String> {
    out[bucket]
        .as_array()
        .unwrap_or_else(|| panic!("conflicts has no array `{bucket}`: {out}"))
        .iter()
        .flat_map(|row| row["graphs"].as_array().unwrap().iter())
        .map(|g| {
            let g = g.as_str().unwrap();
            g.rsplit(['#', '/']).next().unwrap_or(g).to_string()
        })
        .collect()
}

#[test]
fn an_empty_period_is_excluded_at_every_instant_with_a_reason_naming_it_empty() {
    // The instant both bounds name, where a `<=` slip on either side would
    // let the period hold for exactly one instant.
    for (fixture, boundary) in [
        (EMPTY_MIXED_PRECISION, "2024-01-01"),
        (EMPTY_SAME_LITERAL, "2024-06-01"),
    ] {
        let t = temporal(fixture);
        for valid_at in [Some("2023-06-01"), Some(boundary), Some("2030-01-01")] {
            let snap = snapshot(&t, valid_at, None);
            assert!(
                !graphs(&snap, "in_scope").contains("g_empty"),
                "a period with no instant inside it is in scope at none \
                 (valid_at {valid_at:?}): {snap}"
            );
            assert!(
                invalid_reason(&snap, "g_empty").is_none(),
                "both bounds read, so nothing about it is unreadable: {snap}"
            );
            let why = reason(&snap, "g_empty");
            assert!(
                why.contains("holds at no instant") && why.contains("empty"),
                "the reason says it holds nowhere and names the shape: {why}"
            );
            assert!(
                !why.contains("inverted"),
                "and does not name the other shape: {why}"
            );
        }
        // With no valid_at no instant was asked about, and the graph is read
        // like any other: in scope, excluded nowhere, unreadable nowhere.
        let open = snapshot(&t, None, None);
        assert!(
            graphs(&open, "in_scope").contains("g_empty"),
            "no instant asked about, so nothing for it to fail: {open}"
        );
        assert!(!graphs(&open, "excluded").contains("g_empty"), "{open}");
        assert!(invalid_reason(&open, "g_empty").is_none(), "{open}");
        // The live graph is unaffected either way.
        let mid = snapshot(&t, Some("2024-01-01"), None);
        assert!(graphs(&mid, "in_scope").contains("g_live"), "{mid}");
        let late = snapshot(&t, Some("2030-01-01"), None);
        assert_eq!(
            reason(&late, "g_live"),
            "not true at that instant",
            "a sound period outside its interval keeps the generic reason: {late}"
        );
    }
}

#[test]
fn an_inverted_period_is_named_inverted_and_is_not_a_correction() {
    let t = temporal(INVERTED);
    // 2022-06-01 lies between the two bounds: a reading that silently swapped
    // them would put the graph in scope here.
    for valid_at in [Some("2020-01-01"), Some("2022-06-01"), Some("2026-01-01")] {
        let snap = snapshot(&t, valid_at, None);
        assert!(
            !graphs(&snap, "in_scope").contains("g_inverted"),
            "valid_at {valid_at:?}: {snap}"
        );
        assert!(invalid_reason(&snap, "g_inverted").is_none(), "{snap}");
        let why = reason(&snap, "g_inverted");
        assert!(
            why.contains("holds at no instant") && why.contains("inverted"),
            "the reason names the shape that is almost always a mistake: {why}"
        );
        assert!(!why.contains("empty"), "{why}");
    }
    // With no valid_at no instant was asked about, and the graph is in scope
    // like any other, the shape notwithstanding.
    let open = snapshot(&t, None, None);
    assert!(graphs(&open, "in_scope").contains("g_inverted"), "{open}");
    assert!(!graphs(&open, "excluded").contains("g_inverted"), "{open}");
    assert!(invalid_reason(&open, "g_inverted").is_none(), "{open}");
    // In conflicts the pair shares no instant, which is true, and lands in
    // the bucket that since #116 claims nothing more than that. It was filed
    // there before this fix too; what changed is that the snapshot now says
    // the graph holds nowhere instead of "not true at that instant", so the
    // row cannot be read as a period ending in 2020 that 2023 corrected.
    let out = conflicts(&t);
    assert_eq!(out["contradiction_count"], 0, "{out}");
    assert_eq!(out["non_overlapping_count"], 1, "{out}");
    assert!(
        partners(&out, "non_overlapping").contains("g_inverted"),
        "{out}"
    );
    assert!(out.get("undecided_count").is_none(), "{out}");
}

#[test]
fn a_period_that_holds_nowhere_is_never_a_contradiction_partner() {
    let t = temporal(EMPTY_MIXED_PRECISION);
    let out = conflicts(&t);
    // `[2024-01-01, 2024-01-01)` lies inside `[2023-01-01, 2025-01-01)`, so a
    // bound test alone answers "overlaps" and filed this as a live
    // contradiction. A period with no instant inside it shares none.
    assert_eq!(out["contradiction_count"], 0, "{out}");
    assert_eq!(out["non_overlapping_count"], 1, "{out}");
    assert_eq!(out["superseded_count"], 1, "{out}");
    assert!(
        out.get("undecided_count").is_none(),
        "both bounds read, so the pair is classified: {out}"
    );
    assert!(
        out["non_overlapping"][0]["periods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p == "2024 to 2024-01-01"),
        "the row shows the bounds as asserted: {out}"
    );
    let note = out["note"].as_str().unwrap();
    assert!(
        note.contains("holds at no instant") && note.contains("onto_temporal_snapshot"),
        "the note says the bucket can hold such a period, and where it is explained: {note}"
    );
}

/// The defect in one assertion: the snapshot said the graph holds nowhere
/// while conflicts said it overlapped a live one. Both outputs on the same
/// store, for all three shapes.
#[test]
fn snapshot_and_conflicts_agree_about_a_period_that_holds_nowhere() {
    for (fixture, graph, inside) in [
        (EMPTY_MIXED_PRECISION, "g_empty", "2024-01-01"),
        (EMPTY_SAME_LITERAL, "g_empty", "2024-06-01"),
        (INVERTED, "g_inverted", "2024-01-01"),
    ] {
        let t = temporal(fixture);
        let snap = snapshot(&t, Some(inside), None);
        assert!(
            reason(&snap, graph).contains("holds at no instant"),
            "{fixture}: {snap}"
        );
        assert!(graphs(&snap, "in_scope").contains("g_live"), "{snap}");
        let out = conflicts(&t);
        assert_eq!(out["contradiction_count"], 0, "{fixture}: {out}");
        assert!(
            !partners(&out, "contradictions").contains(graph),
            "a graph the snapshot excludes everywhere cannot disagree with anything: {out}"
        );
        assert!(partners(&out, "non_overlapping").contains(graph), "{out}");
    }
}

/// The guard against over-eager classification: a touching pair has one
/// bound in common ACROSS the two graphs, which is nothing like a period with
/// no instant inside it, and a sound period is still in scope inside its
/// interval and excluded outside it with the generic reason.
#[test]
fn a_sound_period_and_a_touching_pair_are_not_degenerate() {
    let t = temporal(CONFLICT_SUPERSEDED);
    let inside = snapshot(&t, Some("2025-01-01"), None);
    assert!(graphs(&inside, "in_scope").contains("g_a"), "{inside}");
    assert_eq!(
        reason(&inside, "g_b"),
        "not true at that instant",
        "a sound period outside its interval keeps the generic reason: {inside}"
    );
    let at_bound = snapshot(&t, Some("2026-05-01"), None);
    assert_eq!(reason(&at_bound, "g_a"), "not true at that instant");
    assert!(graphs(&at_bound, "in_scope").contains("g_b"), "{at_bound}");
    let out = conflicts(&t);
    assert_eq!(out["contradiction_count"], 0, "{out}");
    assert_eq!(out["non_overlapping_count"], 1, "{out}");
    // And nothing in a store with no degenerate period says one holds nowhere.
    for fixture in [
        CELL_LINE,
        CONFLICT_SUPERSEDED,
        CONFLICT_OVERLAP,
        COARSE_BOUNDS,
    ] {
        let snap = snapshot(&temporal(fixture), None, None);
        assert!(
            graphs(&snap, "excluded").is_empty(),
            "every sound period is in scope with no valid_at: {snap}"
        );
    }
}

/// `query_at` runs over the snapshot's scope. With no valid_at no instant was
/// asked about, so a graph that holds at no instant is read like any other:
/// the query's output has no key that could say it was left out, and
/// narrowing an atemporal query silently is the failure its reporting exists
/// to remove. At any valid_at the graph is out of scope, and
/// onto_temporal_snapshot at that instant is where the reason is.
#[test]
fn query_at_with_no_valid_at_reads_a_period_that_holds_nowhere_like_any_other() {
    let t = temporal(EMPTY_MIXED_PRECISION);
    let raw = t.query_at("{ ?s a ?type }", None, None).unwrap();
    let out: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(
        raw.contains("Adherent"),
        "no instant asked about, so the graph that holds nowhere is read: {raw}"
    );
    assert!(
        raw.contains("Suspension"),
        "and so is the live graph beside it: {raw}"
    );
    assert_eq!(
        out["graphs_in_scope"], 2,
        "both graphs are in scope with no valid_at: {out}"
    );
    assert!(
        out.get("invalid").is_none(),
        "both of its bounds read, so it is not listed as unreadable either: {out}"
    );
    // At an instant the live graph holds at, the boundary instant included,
    // the graph that holds nowhere is left out and the live one alone is read.
    for valid_at in ["2023-06-01", "2024-01-01"] {
        let raw = t.query_at("{ ?s a ?type }", Some(valid_at), None).unwrap();
        let out: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert!(
            !raw.contains("Adherent"),
            "valid_at {valid_at}: the graph that holds nowhere is not queried: {raw}"
        );
        assert!(raw.contains("Suspension"), "valid_at {valid_at}: {raw}");
        assert_eq!(
            out["graphs_in_scope"], 1,
            "valid_at {valid_at}: g_live alone is in scope: {out}"
        );
    }
}

/// An empty period whose graph also carries a recordedAt, beside a live graph
/// recorded at the same time. An `as_of` before that instant fails both on
/// the recorded axis, and only one of them holds nowhere.
const EMPTY_RECORDED_LATER: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g_empty { ex:X a ex:Adherent . }
ex:g_live  { ex:X a ex:Suspension . }

{
  ex:Adherent owl:disjointWith ex:Suspension .
  ex:g_empty t:validFrom "2024-06-01" ; t:validTo "2024-06-01" ; t:recordedAt "2024-07-01" .
  ex:g_live  t:validFrom "2023-01-01" ; t:validTo "2025-01-01" ; t:recordedAt "2024-07-01" .
}
"#;

/// With a valid_at given, the reason names the shape whatever `as_of` is
/// passed: validity is checked first, and "not yet recorded then" is a fact
/// about the query where "holds at no instant" is a fact about the data, the
/// one to fix, so it is not hidden behind the first when both apply. With no
/// valid_at the recorded side is the only question asked, and its reason the
/// only one that can apply. The live graph beside it shows the recorded
/// reason is still given where it is the only thing wrong.
#[test]
fn a_period_that_holds_nowhere_says_so_whatever_as_of_is_passed() {
    let t = temporal(EMPTY_RECORDED_LATER);
    // At the instant both bounds name and at one outside it, with an as_of
    // before, at and after the recordedAt of both graphs.
    for valid_at in [Some("2024-06-01"), Some("2024-01-01")] {
        for as_of in [Some("2000-01-01"), Some("2024-07-01"), Some("2030-01-01")] {
            let snap = snapshot(&t, valid_at, as_of);
            let why = reason(&snap, "g_empty");
            assert!(
                why.contains("holds at no instant") && why.contains("empty"),
                "valid_at {valid_at:?}, as_of {as_of:?}: the data fault wins over the \
                 recorded axis: {why}"
            );
            assert!(
                !why.contains("recorded"),
                "valid_at {valid_at:?}, as_of {as_of:?}: and the recorded axis is not \
                 mentioned: {why}"
            );
        }
    }
    // With no valid_at, an as_of before its recordedAt excludes it on the
    // recorded side alone, as it would a sound graph; at or after it, the
    // graph is in scope.
    let early = snapshot(&t, None, Some("2000-01-01"));
    assert_eq!(
        reason(&early, "g_empty"),
        "not yet recorded then",
        "no instant asked about, so the recorded side is the only question: {early}"
    );
    for as_of in [Some("2024-07-01"), Some("2030-01-01")] {
        let snap = snapshot(&t, None, as_of);
        assert!(
            graphs(&snap, "in_scope").contains("g_empty"),
            "as_of {as_of:?}: {snap}"
        );
    }
    assert_eq!(
        reason(&early, "g_live"),
        "not yet recorded then",
        "a sound period before its recordedAt keeps the recorded reason: {early}"
    );
    let later = snapshot(&t, None, Some("2030-01-01"));
    assert!(graphs(&later, "in_scope").contains("g_live"), "{later}");
}

/// An empty period beside a graph with no description at all.
const EMPTY_BESIDE_TIMELESS: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g_empty    { ex:X a ex:Adherent . }
ex:g_timeless { ex:X a ex:Suspension . }

{
  ex:Adherent owl:disjointWith ex:Suspension .
  ex:g_empty t:validFrom "2024-06-01" ; t:validTo "2024-06-01" .
}
"#;

/// A timeless partner is the case the bound test got most wrong: its period
/// has open ends, so `[t, t)` overlapped it unconditionally. The pair shares
/// no instant, the row shows the partner as open on both sides, and the
/// snapshot keeps the timeless graph in scope beside the excluded one.
#[test]
fn a_period_that_holds_nowhere_beside_a_timeless_graph_is_not_a_contradiction() {
    let t = temporal(EMPTY_BESIDE_TIMELESS);
    let out = conflicts(&t);
    assert_eq!(out["contradiction_count"], 0, "{out}");
    assert_eq!(out["non_overlapping_count"], 1, "{out}");
    assert!(out.get("undecided_count").is_none(), "{out}");
    assert!(
        out["non_overlapping"][0]["periods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p == "always to still true"),
        "the undescribed partner is shown open on both sides: {out}"
    );
    let snap = snapshot(&t, Some("2024-06-01"), None);
    assert!(graphs(&snap, "in_scope").contains("g_timeless"), "{snap}");
    assert!(
        reason(&snap, "g_empty").contains("holds at no instant"),
        "{snap}"
    );
    let open = snapshot(&t, None, None);
    assert_eq!(
        graphs(&open, "in_scope"),
        set(&["g_empty", "g_timeless"]),
        "no instant asked about, so both are read: {open}"
    );
}

/// An empty period beside a graph whose own bound could not be read.
const EMPTY_BESIDE_UNREADABLE: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g_empty { ex:X a ex:Adherent . }
ex:g_bad   { ex:X a ex:Suspension . }

{
  ex:Adherent owl:disjointWith ex:Suspension .
  ex:g_empty t:validFrom "2024-06-01" ; t:validTo "2024-06-01" .
  ex:g_bad   t:validFrom "01/06/2024"^^xsd:string .
}
"#;

/// Unreadable still wins: a period that holds nowhere shares no instant with
/// anything, but "shares no instant" is a verdict on two periods, and an
/// unreadable partner is not a period at all, so the pair stays undecided
/// rather than being filed as non_overlapping on the strength of one side.
#[test]
fn a_period_that_holds_nowhere_beside_an_unreadable_one_is_still_undecided() {
    let t = temporal(EMPTY_BESIDE_UNREADABLE);
    let out = conflicts(&t);
    assert_eq!(out["contradiction_count"], 0, "{out}");
    assert_eq!(out["non_overlapping_count"], 0, "{out}");
    assert_eq!(out["undecided_count"], 1, "{out}");
    assert!(
        out["undecided"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("could not be read"),
        "{out}"
    );
    let snap = snapshot(&t, Some("2024-06-01"), None);
    assert!(invalid_reason(&snap, "g_bad").is_some(), "{snap}");
    assert!(invalid_reason(&snap, "g_empty").is_none(), "{snap}");
    assert!(
        reason(&snap, "g_empty").contains("holds at no instant"),
        "{snap}"
    );
    // With no valid_at the unreadable graph stays out and the one that holds
    // nowhere is in: the two verdicts do not depend on each other.
    let open = snapshot(&t, None, None);
    assert!(invalid_reason(&open, "g_bad").is_some(), "{open}");
    assert!(graphs(&open, "in_scope").contains("g_empty"), "{open}");
}

// ---------------------------------------------------------------------------
// Section 5: lineage that is asserted, never inferred (#109)
//
// `as_of` only narrowed forward, and nothing linked one version to the next:
// a correction sharing its predecessor's valid period was filed as a
// contradiction, and with no explicit recordedUntil the predecessor stayed
// believed for ever. Two predicates on the NEWER graph carry the link:
// `supersedes` names the graph it replaces, `retracts` the graph it withdraws
// without replacing. recordedUntil alone governs scope; where a graph carries
// none, its closing bound is DERIVED from the successor's recordedAt at query
// time and never written, and where both are present and disagree the explicit
// bound governs and the disagreement is reported. The issue's own example
// leads.
// ---------------------------------------------------------------------------

/// The row for one graph in one snapshot bucket, named by its local part.
/// Panics rather than returning an Option: every caller is asserting on the
/// row, so a missing one is a failed test.
fn bucket_row(snapshot: &serde_json::Value, bucket: &str, local: &str) -> serde_json::Value {
    snapshot[bucket]
        .as_array()
        .unwrap_or_else(|| panic!("snapshot has no array `{bucket}`: {snapshot}"))
        .iter()
        .find(|row| row["graph"].as_str().is_some_and(|g| g.ends_with(local)))
        .cloned()
        .unwrap_or_else(|| panic!("`{local}` is not in `{bucket}`: {snapshot}"))
}

/// The lineage rows about one graph. Empty, not a panic, when the key is
/// absent: a clean answer carries no `lineage` at all.
fn lineage_about(snapshot: &serde_json::Value, local: &str) -> Vec<serde_json::Value> {
    snapshot
        .get("lineage")
        .and_then(|l| l.as_array())
        .map(|rows| {
            rows.iter()
                .filter(|row| row["graph"].as_str().is_some_and(|g| g.ends_with(local)))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Issue #109's own example: the correction shares its predecessor's valid
/// period, the predecessor carries no recordedUntil, and the successor says
/// which graph it replaces.
const ISSUE_109: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g1 { ex:HEK293 a ex:AdherentCellLine . }
ex:g2 { ex:HEK293 a ex:SuspensionCellLine . }

{
  ex:AdherentCellLine owl:disjointWith ex:SuspensionCellLine .
  ex:g1 t:validFrom "2024-01-01" ; t:recordedAt "2024-01-05" .
  ex:g2 t:validFrom "2024-01-01" ; t:recordedAt "2026-06-01" ; t:supersedes ex:g1 .
}
"#;

#[test]
fn a_successor_closes_its_predecessor_at_its_own_recorded_at() {
    let t = temporal(ISSUE_109);
    // After the correction was recorded: the predecessor is no longer
    // believed, and the row says which graph closed it, since it carries no
    // recordedUntil of its own that could.
    let after = snapshot(&t, None, Some("2026-07-01"));
    assert_eq!(graphs(&after, "in_scope"), set(&["g2"]), "{after}");
    assert_eq!(reason(&after, "g1"), "no longer recorded then", "{after}");
    assert!(
        bucket_row(&after, "excluded", "g1")["superseded_by"]
            .as_str()
            .is_some_and(|g| g.ends_with("g2")),
        "the derived bound names the graph it was derived from: {after}"
    );
    assert!(after.get("lineage").is_none(), "nothing to report: {after}");
    // Before it: the predecessor was what was believed, and the successor
    // had not arrived.
    let before = snapshot(&t, None, Some("2025-01-01"));
    assert_eq!(graphs(&before, "in_scope"), set(&["g1"]), "{before}");
    assert_eq!(reason(&before, "g2"), "not yet recorded then", "{before}");
    // With no as_of no instant was asked about on the recorded axis, and a
    // derived bound is read exactly as an asserted recordedUntil is: both
    // graphs are in scope, as `g_first` is in CORRECTED with no as_of. The
    // derivation composes a bound; it does not change what the axis means.
    let open = snapshot(&t, None, None);
    assert_eq!(graphs(&open, "in_scope"), set(&["g1", "g2"]), "{open}");
    assert!(graphs(&open, "excluded").is_empty(), "{open}");
}

#[test]
fn a_correction_that_shares_its_predecessors_period_is_not_a_contradiction() {
    let t = temporal(ISSUE_109);
    let out = conflicts(&t);
    // Both graphs claim [2024-01-01, open) for disjoint types. On the periods
    // alone that is a contradiction, and it was filed as one; the link says
    // it is a correction, and the link is the only thing that can.
    assert_eq!(out["contradiction_count"], 0, "{out}");
    assert_eq!(out["non_overlapping_count"], 0, "{out}");
    assert_eq!(out["corrections_count"], 1, "{out}");
    assert_eq!(partners(&out, "corrections"), set(&["g1", "g2"]), "{out}");
    let link = out["corrections"][0]["supersedes"].as_array().unwrap();
    assert!(
        link[0].as_str().unwrap().ends_with("g2") && link[1].as_str().unwrap().ends_with("g1"),
        "successor first, predecessor second: {out}"
    );
    let note = out["note"].as_str().unwrap();
    assert!(
        note.contains("corrections") && note.contains("whatever its periods"),
        "the note says the periods are not consulted for such a pair: {note}"
    );
    // And a store with no link still has no corrections key at all.
    let plain = conflicts(&temporal(CONFLICT_OVERLAP));
    assert!(plain.get("corrections").is_none(), "{plain}");
    assert!(plain.get("corrections_count").is_none(), "{plain}");
}

/// The predecessor carries its own recordedUntil, five months before the
/// successor says it was recorded.
const EXPLICIT_DISAGREES: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1 { ex:HEK293 a ex:AdherentCellLine . }
ex:g2 { ex:HEK293 a ex:SuspensionCellLine . }

{
  ex:g1 t:validFrom "2024-01-01" ; t:recordedAt "2024-01-05" ; t:recordedUntil "2026-01-01" .
  ex:g2 t:validFrom "2024-01-01" ; t:recordedAt "2026-06-01" ; t:supersedes ex:g1 .
}
"#;

#[test]
fn an_explicit_recorded_until_governs_and_its_disagreement_with_the_successor_is_reported() {
    let t = temporal(EXPLICIT_DISAGREES);
    // Between the two instants: the explicit bound has closed the interval,
    // the successor has not arrived. A derivation that yielded to the
    // successor would put g1 back in scope here.
    let between = snapshot(&t, None, Some("2026-03-01"));
    assert_eq!(
        reason(&between, "g1"),
        "no longer recorded then",
        "{between}"
    );
    assert!(
        bucket_row(&between, "excluded", "g1")
            .get("superseded_by")
            .is_none(),
        "an asserted bound was not derived from anything: {between}"
    );
    assert_eq!(reason(&between, "g2"), "not yet recorded then", "{between}");
    assert!(graphs(&between, "in_scope").is_empty(), "{between}");
    // The inconsistency survives in the report, naming both instants and
    // both graphs, and is reconciled nowhere.
    let rows = lineage_about(&between, "g1");
    assert_eq!(rows.len(), 1, "{between}");
    let row = &rows[0];
    assert!(
        row["reason"]
            .as_str()
            .unwrap()
            .contains("explicit recordedUntil disagrees"),
        "{row}"
    );
    assert!(
        row["reason"]
            .as_str()
            .unwrap()
            .contains("explicit bound governs"),
        "{row}"
    );
    assert!(row["successor"].as_str().unwrap().ends_with("g2"), "{row}");
    assert_eq!(row["recorded_until"], "2026-01-01", "{row}");
    assert_eq!(row["successor_recorded_at"], "2026-06-01", "{row}");
    // Before the explicit bound the predecessor is believed, as asserted.
    let early = snapshot(&t, None, Some("2025-06-01"));
    assert_eq!(graphs(&early, "in_scope"), set(&["g1"]), "{early}");
}

/// Out-of-order recording: the successor says it was recorded five months
/// BEFORE the predecessor it replaces.
const OUT_OF_ORDER: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g_old { ex:HEK293 a ex:AdherentCellLine . }
ex:g_new { ex:HEK293 a ex:SuspensionCellLine . }

{
  ex:g_old t:validFrom "2024-01-01" ; t:recordedAt "2024-06-01" .
  ex:g_new t:validFrom "2024-01-01" ; t:recordedAt "2024-01-01" ; t:supersedes ex:g_old .
}
"#;

#[test]
fn a_successor_recorded_before_its_predecessor_makes_an_inverted_interval_that_is_reported_not_clamped()
 {
    let t = temporal(OUT_OF_ORDER);
    // Before, between, at and after the two instants: a clamp to an empty
    // interval, or to `[recordedAt, recordedAt)`, would still exclude the
    // graph everywhere, so the assertions below pin the REASON and the
    // reported instants, which a clamp would change.
    for as_of in ["2023-01-01", "2024-03-01", "2024-06-01", "2030-01-01"] {
        let snap = snapshot(&t, None, Some(as_of));
        let why = reason(&snap, "g_old");
        assert!(
            why.contains("believed at no instant") && why.contains("closes before it opens"),
            "as_of {as_of}: the reason names the shape, not a side of the interval: {why}"
        );
        assert_eq!(
            bucket_row(&snap, "excluded", "g_old")["valid"],
            "2024-01-01 to still true",
            "as_of {as_of}: the asserted bounds are shown untouched: {snap}"
        );
        let rows = lineage_about(&snap, "g_old");
        assert_eq!(rows.len(), 1, "as_of {as_of}: {snap}");
        let row = &rows[0];
        assert!(
            row["reason"]
                .as_str()
                .unwrap()
                .contains("inverted transaction interval"),
            "{row}"
        );
        assert_eq!(row["recorded_at"], "2024-06-01", "as asserted: {row}");
        assert_eq!(
            row["recorded_until"], "2024-01-01",
            "as derived from the successor, not moved: {row}"
        );
        assert!(
            row["superseded_by"].as_str().unwrap().ends_with("g_new"),
            "{row}"
        );
    }
    // The successor itself is believed from its own recordedAt.
    let mid = snapshot(&t, None, Some("2024-03-01"));
    assert_eq!(graphs(&mid, "in_scope"), set(&["g_new"]), "{mid}");
    // With no as_of no instant was asked about, and the graph is read like
    // any other, mirroring the rule for a valid period that holds nowhere.
    let open = snapshot(&t, None, None);
    assert_eq!(
        graphs(&open, "in_scope"),
        set(&["g_old", "g_new"]),
        "{open}"
    );
    assert!(
        !lineage_about(&open, "g_old").is_empty(),
        "and the report is still there to read: {open}"
    );
}

/// A correction of a correction: three versions, each superseding the last,
/// of three pairwise disjoint types.
const CHAIN: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g1 { ex:HEK293 a ex:AdherentCellLine . }
ex:g2 { ex:HEK293 a ex:SuspensionCellLine . }
ex:g3 { ex:HEK293 a ex:HybridCellLine . }

{
  ex:AdherentCellLine   owl:disjointWith ex:SuspensionCellLine , ex:HybridCellLine .
  ex:SuspensionCellLine owl:disjointWith ex:HybridCellLine .
  ex:g1 t:validFrom "2024-01-01" ; t:recordedAt "2024-01-05" .
  ex:g2 t:validFrom "2024-01-01" ; t:recordedAt "2025-01-05" ; t:supersedes ex:g1 .
  ex:g3 t:validFrom "2024-01-01" ; t:recordedAt "2026-01-05" ; t:supersedes ex:g2 .
}
"#;

#[test]
fn a_chain_of_supersedes_links_needs_no_special_machinery() {
    let t = temporal(CHAIN);
    // After the third version only it is believed, and each predecessor
    // names its own direct successor: the bound is derived one hop at a time.
    let late = snapshot(&t, None, Some("2026-06-01"));
    assert_eq!(graphs(&late, "in_scope"), set(&["g3"]), "{late}");
    for (graph, successor) in [("g1", "g2"), ("g2", "g3")] {
        assert_eq!(reason(&late, graph), "no longer recorded then", "{late}");
        assert!(
            bucket_row(&late, "excluded", graph)["superseded_by"]
                .as_str()
                .is_some_and(|g| g.ends_with(successor)),
            "{graph} is closed by {successor}: {late}"
        );
    }
    assert!(
        late.get("lineage").is_none(),
        "a clean chain reports nothing: {late}"
    );
    // In the middle the second version is the one believed.
    let mid = snapshot(&t, None, Some("2025-06-01"));
    assert_eq!(graphs(&mid, "in_scope"), set(&["g2"]), "{mid}");
    // Every pair is disjoint and every pair shares the whole period. The
    // direct pairs are corrections by their own link, and the pair at the
    // two ends of the chain is one through it: nothing in the data says g3
    // supersedes g1, and nothing needs to.
    let out = conflicts(&t);
    assert_eq!(out["contradiction_count"], 0, "{out}");
    assert_eq!(out["corrections_count"], 3, "{out}");
    let ends = out["corrections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            let graphs: BTreeSet<String> = row["graphs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| g.as_str().unwrap().rsplit('/').next().unwrap().to_string())
                .collect();
            graphs == set(&["g1", "g3"])
        })
        .unwrap_or_else(|| panic!("the pair at the ends of the chain is a correction: {out}"));
    let link = ends["supersedes"].as_array().unwrap();
    assert!(
        link[0].as_str().unwrap().ends_with("g3") && link[1].as_str().unwrap().ends_with("g1"),
        "through the chain, g3 supersedes g1: {ends}"
    );
}

/// One graph replaced twice, by two successors a year apart.
const TWO_SUCCESSORS: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1 { ex:HEK293 a ex:AdherentCellLine . }
ex:g2 { ex:HEK293 a ex:SuspensionCellLine . }
ex:g3 { ex:HEK293 a ex:HybridCellLine . }

{
  ex:g1 t:validFrom "2024-01-01" ; t:recordedAt "2024-01-05" .
  ex:g2 t:validFrom "2024-01-01" ; t:recordedAt "2025-01-05" ; t:supersedes ex:g1 .
  ex:g3 t:validFrom "2024-01-01" ; t:recordedAt "2026-01-05" ; t:supersedes ex:g1 .
}
"#;

#[test]
fn the_earliest_of_two_successors_closes_the_graph_and_the_multiplicity_is_reported() {
    let t = temporal(TWO_SUCCESSORS);
    // Between the two replacements. Belief in g1 ended at the first, and the
    // second does not revive it; a derivation that took the latest successor
    // would put g1 back in scope here.
    let between = snapshot(&t, None, Some("2025-06-01"));
    assert_eq!(graphs(&between, "in_scope"), set(&["g2"]), "{between}");
    assert_eq!(
        reason(&between, "g1"),
        "no longer recorded then",
        "{between}"
    );
    assert!(
        bucket_row(&between, "excluded", "g1")["superseded_by"]
            .as_str()
            .is_some_and(|g| g.ends_with("g2")),
        "closed by the earliest: {between}"
    );
    let rows = lineage_about(&between, "g1");
    assert_eq!(rows.len(), 1, "{between}");
    let row = &rows[0];
    assert!(
        row["reason"]
            .as_str()
            .unwrap()
            .contains("more than one graph"),
        "{row}"
    );
    assert!(row["closed_by"].as_str().unwrap().ends_with("g2"), "{row}");
    assert_eq!(row["successors"].as_array().unwrap().len(), 2, "{row}");
}

/// One graph replaced twice at the very instant its own recordedUntil names:
/// the explicit bound agrees with both successors, so no disagreement, and
/// neither successor closed anything.
const TWO_SUCCESSORS_EXPLICIT_CLOSE: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1 { ex:HEK293 a ex:AdherentCellLine . }
ex:g2 { ex:HEK293 a ex:SuspensionCellLine . }
ex:g3 { ex:HEK293 a ex:HybridCellLine . }

{
  ex:g1 t:validFrom "2024-01-01" ; t:recordedAt "2024-01-05" ; t:recordedUntil "2026-06-01" .
  ex:g2 t:validFrom "2024-01-01" ; t:recordedAt "2026-06-01" ; t:supersedes ex:g1 .
  ex:g3 t:validFrom "2024-01-01" ; t:recordedAt "2026-06-01" ; t:supersedes ex:g1 .
}
"#;

#[test]
fn more_than_one_successor_is_reported_under_an_explicit_close_too() {
    let t = temporal(TWO_SUCCESSORS_EXPLICIT_CLOSE);
    let after = snapshot(&t, None, Some("2026-07-01"));
    // The explicit bound closed the graph, and nothing was derived.
    assert_eq!(reason(&after, "g1"), "no longer recorded then", "{after}");
    assert!(
        bucket_row(&after, "excluded", "g1")
            .get("superseded_by")
            .is_none(),
        "{after}"
    );
    // Exactly one lineage row: the multiplicity. No disagreement, since both
    // successors name the explicit instant, and a report that only spoke on
    // the derived branch would leave nothing here at all.
    let rows = lineage_about(&after, "g1");
    assert_eq!(rows.len(), 1, "{after}");
    let row = &rows[0];
    let reason = row["reason"].as_str().unwrap();
    assert!(reason.contains("more than one graph"), "{row}");
    assert!(
        reason.contains("explicit recordedUntil governs"),
        "the row says which bound closed it: {row}"
    );
    assert!(!reason.contains("disagrees"), "{row}");
    let successors: BTreeSet<String> = row["successors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().rsplit('/').next().unwrap().to_string())
        .collect();
    assert_eq!(successors, set(&["g2", "g3"]), "{row}");
    // The row names the explicit bound, not a successor, as what closed it:
    // `closed_by` is a graph on the derived branch and would be a lie here.
    assert_eq!(row["recorded_until"], "2026-06-01", "{row}");
    assert!(row.get("closed_by").is_none(), "{row}");
}

/// Two graphs each claiming to supersede the other.
const CYCLE: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g_a { ex:HEK293 a ex:AdherentCellLine . }
ex:g_b { ex:HEK293 a ex:SuspensionCellLine . }

{
  ex:AdherentCellLine owl:disjointWith ex:SuspensionCellLine .
  ex:g_a t:validFrom "2024-01-01" ; t:recordedAt "2024-01-05" ; t:supersedes ex:g_b .
  ex:g_b t:validFrom "2024-01-01" ; t:recordedAt "2025-01-05" ; t:supersedes ex:g_a .
}
"#;

#[test]
fn a_cycle_of_supersedes_links_is_reported_and_does_not_hang() {
    let t = temporal(CYCLE);
    // Returning at all is the first assertion. Each bound is still derived
    // from the direct successor: g_a closes at g_b's recordedAt, and g_b at
    // g_a's, which precedes its own and so is an inverted interval.
    let snap = snapshot(&t, None, Some("2024-06-01"));
    assert_eq!(graphs(&snap, "in_scope"), set(&["g_a"]), "{snap}");
    assert!(
        reason(&snap, "g_b").contains("believed at no instant"),
        "{snap}"
    );
    let cycle = snap["lineage"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["reason"].as_str().is_some_and(|r| r.contains("cycle")))
        .unwrap_or_else(|| panic!("the cycle is reported: {snap}"));
    assert_eq!(cycle["cycle"].as_array().unwrap().len(), 2, "{cycle}");
    // The conflict check walks the same links and stops too. Each graph
    // asserts that it supersedes the other, so the pair is a correction.
    let out = conflicts(&t);
    assert_eq!(out["contradiction_count"], 0, "{out}");
    assert_eq!(out["corrections_count"], 1, "{out}");
}

/// A withdrawal: `r` retracts `g1` a year after it was recorded, and puts
/// nothing in its place. A timeless graph sits beside them for the query.
const RETRACTED: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1        { ex:HEK293 a ex:AdherentCellLine . }
ex:r         { ex:HEK293 ex:note "withdrawn" . }
ex:g_species { ex:HEK293 ex:species ex:Human . }

{
  ex:g1 t:validFrom "2024-01-01" ; t:recordedAt "2024-01-05" .
  ex:r  t:recordedAt "2025-01-01" ; t:retracts ex:g1 .
}
"#;

#[test]
fn a_retracted_graph_leaves_scope_from_the_retraction_and_stays_visible_in_its_own_bucket() {
    let t = temporal(RETRACTED);
    // Before the retraction was recorded, g1 takes the ordinary path.
    let before = snapshot(&t, None, Some("2024-06-01"));
    assert!(graphs(&before, "in_scope").contains("g1"), "{before}");
    assert!(before.get("retracted").is_none(), "{before}");
    // From the retraction on: in neither in_scope nor excluded, and the row
    // names the graph that withdrew it and when.
    let as_of = Some("2025-06-01");
    let snap = snapshot(&t, None, as_of);
    assert!(!graphs(&snap, "in_scope").contains("g1"), "{snap}");
    assert!(
        !graphs(&snap, "excluded").contains("g1"),
        "a withdrawn claim did not fail a bound: {snap}"
    );
    let row = bucket_row(&snap, "retracted", "g1");
    assert_eq!(row["reason"], "retracted", "{row}");
    assert!(
        row["retracted_by"].as_str().unwrap().ends_with("/r"),
        "{row}"
    );
    assert_eq!(row["retracted_at"], "2025-01-01", "{row}");
    assert_eq!(row["valid"], "2024-01-01 to still true", "{row}");
    // And the query does not read its triples.
    let raw = t.query_at("{ ?s a ?type }", None, as_of).unwrap();
    assert!(
        !raw.contains("AdherentCellLine"),
        "a retracted graph is not queried: {raw}"
    );
    let out: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        bucket_row(&out, "retracted", "g1")["reason"],
        "retracted",
        "and the query says why it was left out: {out}"
    );
    // With no as_of the recorded axis is not consulted, for a retraction no
    // more than for a recordedUntil, asserted or derived: one rule for every
    // recorded-time fact. The graph takes the ordinary path and is in scope,
    // and the query reads it.
    let none = snapshot(&t, None, None);
    assert!(graphs(&none, "in_scope").contains("g1"), "{none}");
    assert!(none.get("retracted").is_none(), "{none}");
    let raw = t.query_at("{ ?s a ?type }", None, None).unwrap();
    assert!(
        raw.contains("AdherentCellLine"),
        "with no as_of the graph is read like any other: {raw}"
    );
    let out: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(out.get("retracted").is_none(), "{out}");
    // Retraction is a fact about the assertion's standing and is asked
    // first: at an instant the period does not hold at, the row is still in
    // retracted rather than excluded as not true then. With no as_of the
    // ordinary path answers, and that path excludes it as not true then.
    let early = snapshot(&t, Some("2020-01-01"), as_of);
    assert!(!graphs(&early, "excluded").contains("g1"), "{early}");
    assert_eq!(
        bucket_row(&early, "retracted", "g1")["reason"],
        "retracted",
        "{early}"
    );
    let early_none = snapshot(&t, Some("2020-01-01"), None);
    assert_eq!(
        reason(&early_none, "g1"),
        "not true at that instant",
        "{early_none}"
    );
    assert!(early_none.get("retracted").is_none(), "{early_none}");
}

/// Links that assert nothing readable: a successor naming a graph nobody
/// described, a retractor with no recordedAt, and a literal where a graph
/// IRI belongs.
const UNSETTLED_LINKS: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1    { ex:HEK293 a ex:AdherentCellLine . }
ex:g_x   { ex:HEK293 a ex:SuspensionCellLine . }
ex:r     { ex:HEK293 ex:note "withdrawn" . }
ex:g_lit { ex:HEK293 a ex:HybridCellLine . }

{
  ex:g1    t:validFrom "2024-01-01" ; t:recordedAt "2024-01-05" .
  ex:g_x   t:recordedAt "2025-01-05" ; t:supersedes ex:nowhere .
  ex:r     t:retracts ex:g1 .
  ex:g_lit t:recordedAt "2025-01-05" ; t:supersedes "g1" .
}
"#;

#[test]
fn a_link_that_asserts_nothing_readable_is_reported_and_has_no_effect() {
    let t = temporal(UNSETTLED_LINKS);
    let snap = snapshot(&t, None, Some("2026-01-01"));
    // An unknown target: reported, and nothing closed.
    let rows = lineage_about(&snap, "g_x");
    assert_eq!(rows.len(), 1, "{snap}");
    assert!(
        rows[0]["reason"]
            .as_str()
            .unwrap()
            .contains("no temporal description"),
        "{snap}"
    );
    assert!(
        rows[0]["target"].as_str().unwrap().ends_with("nowhere"),
        "{snap}"
    );
    assert!(graphs(&snap, "in_scope").contains("g_x"), "{snap}");
    // A literal where a graph IRI belongs: reported, and g1 is not closed by
    // it, which the retraction below is what takes g1 out of scope instead.
    let rows = lineage_about(&snap, "g_lit");
    assert_eq!(rows.len(), 1, "{snap}");
    assert!(
        rows[0]["reason"]
            .as_str()
            .unwrap()
            .contains("not a graph IRI"),
        "{snap}"
    );
    assert_eq!(
        rows[0]["target"], "\"g1\"",
        "the term is quoted as written: {snap}"
    );
    // An undated retractor: the withdrawal cannot be dated, is reported as
    // such, and stands at every instant asked about, including one before
    // g1 was recorded at all: a withdrawal at an unknown time is still a
    // withdrawal. With no as_of no instant is asked about on the recorded
    // axis, and the graph is in scope like one closed by a recordedUntil.
    let rows = lineage_about(&snap, "g1");
    assert_eq!(rows.len(), 1, "{snap}");
    assert!(
        rows[0]["reason"]
            .as_str()
            .unwrap()
            .contains("cannot be dated"),
        "{snap}"
    );
    for as_of in [Some("2026-01-01"), Some("2000-01-01")] {
        let snap = snapshot(&t, None, as_of);
        let row = bucket_row(&snap, "retracted", "g1");
        assert!(
            row["retracted_by"].as_str().unwrap().ends_with("/r"),
            "as_of {as_of:?}: {row}"
        );
        assert!(row["retracted_at"].is_null(), "undated: {row}");
        assert!(!graphs(&snap, "excluded").contains("g1"), "{snap}");
    }
    let none = snapshot(&t, None, None);
    assert!(graphs(&none, "in_scope").contains("g1"), "{none}");
    assert!(none.get("retracted").is_none(), "{none}");
}

/// Links asserted by graphs whose own description could not be read: a
/// successor and a retractor, each with a recordedAt that is not a date.
const UNREADABLE_LINKERS: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1 { ex:HEK293 a ex:AdherentCellLine . }
ex:g2 { ex:HEK293 a ex:SuspensionCellLine . }
ex:r  { ex:HEK293 ex:note "withdrawn" . }

{
  ex:g1 t:validFrom "2024-01-01" ; t:recordedAt "2024-01-05" .
  ex:g2 t:recordedAt "garbage" ; t:supersedes ex:g1 .
  ex:r  t:recordedAt "nope" ; t:retracts ex:g1 .
}
"#;

#[test]
fn a_link_asserted_by_an_unreadable_graph_is_reported_and_leaves_its_target_as_it_was() {
    let t = temporal(UNREADABLE_LINKERS);
    for as_of in [Some("2030-01-01"), None] {
        let snap = snapshot(&t, None, as_of);
        // The asserters are invalid for their own bounds, as before.
        assert!(invalid_reason(&snap, "g2").is_some(), "{snap}");
        assert!(invalid_reason(&snap, "/r").is_some(), "{snap}");
        // Neither link takes effect: g1 is neither closed nor withdrawn.
        assert_eq!(graphs(&snap, "in_scope"), set(&["g1"]), "{snap}");
        assert!(snap.get("retracted").is_none(), "{snap}");
        assert!(lineage_about(&snap, "g1").is_empty(), "{snap}");
        // But neither is dropped silently: each asserter's row names the
        // link and the graph it would have closed or withdrawn.
        for (local, field) in [("g2", "supersedes"), ("/r", "retracts")] {
            let rows = lineage_about(&snap, local);
            assert_eq!(rows.len(), 1, "{local}: {snap}");
            let reason = rows[0]["reason"].as_str().unwrap();
            assert!(reason.starts_with(field), "{local}: {snap}");
            assert!(reason.contains("could not be read"), "{local}: {snap}");
            assert!(reason.contains("no effect"), "{local}: {snap}");
            assert!(
                rows[0]["target"].as_str().unwrap().ends_with("g1"),
                "{local}: {snap}"
            );
        }
    }
}

/// One graph both superseded and, later, retracted.
const SUPERSEDED_AND_RETRACTED: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1 { ex:HEK293 a ex:AdherentCellLine . }
ex:g2 { ex:HEK293 a ex:SuspensionCellLine . }
ex:r  { ex:HEK293 ex:note "withdrawn" . }

{
  ex:g1 t:validFrom "2024-01-01" ; t:recordedAt "2024-01-05" .
  ex:g2 t:validFrom "2024-01-01" ; t:recordedAt "2025-01-01" ; t:supersedes ex:g1 .
  ex:r  t:recordedAt "2026-01-01" ; t:retracts ex:g1 .
}
"#;

#[test]
fn a_graph_both_superseded_and_retracted_is_retracted_once_the_retraction_is_recorded() {
    let t = temporal(SUPERSEDED_AND_RETRACTED);
    // Between the two: closed by its successor, and the row says so.
    let between = snapshot(&t, None, Some("2025-06-01"));
    assert_eq!(
        reason(&between, "g1"),
        "no longer recorded then",
        "{between}"
    );
    assert!(
        bucket_row(&between, "excluded", "g1")["superseded_by"]
            .as_str()
            .is_some_and(|g| g.ends_with("g2")),
        "{between}"
    );
    assert!(between.get("retracted").is_none(), "{between}");
    // From the retraction on: retraction is a fact about the assertion's
    // standing and beats the derived closing bound, so the graph is in
    // `retracted`, not `excluded`.
    let snap = snapshot(&t, None, Some("2027-01-01"));
    assert!(!graphs(&snap, "excluded").contains("g1"), "{snap}");
    let row = bucket_row(&snap, "retracted", "g1");
    assert!(
        row["retracted_by"].as_str().unwrap().ends_with("/r"),
        "{row}"
    );
    assert_eq!(row["retracted_at"], "2026-01-01", "{row}");
    // With no as_of neither recorded-time fact is consulted, the derived
    // bound no more than the retraction: the graph is in scope, in neither
    // `excluded` nor `retracted`, as a graph closed by an explicit
    // recordedUntil is.
    let none = snapshot(&t, None, None);
    assert!(graphs(&none, "in_scope").contains("g1"), "{none}");
    assert!(!graphs(&none, "excluded").contains("g1"), "{none}");
    assert!(none.get("retracted").is_none(), "{none}");
}

/// A graph that names itself on both predicates.
const SELF_LINKS: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1 { ex:HEK293 a ex:AdherentCellLine . }

{
  ex:g1 t:validFrom "2024-01-01" ; t:recordedAt "2024-01-05" ;
        t:supersedes ex:g1 ; t:retracts ex:g1 .
}
"#;

#[test]
fn a_link_naming_the_graph_itself_is_reported_and_neither_closes_nor_withdraws_it() {
    let t = temporal(SELF_LINKS);
    for as_of in [Some("2026-01-01"), None] {
        let snap = snapshot(&t, None, as_of);
        assert_eq!(graphs(&snap, "in_scope"), set(&["g1"]), "{snap}");
        assert!(snap.get("retracted").is_none(), "{snap}");
        let rows = lineage_about(&snap, "g1");
        assert_eq!(rows.len(), 2, "one row per predicate: {snap}");
        for row in &rows {
            assert!(
                row["reason"]
                    .as_str()
                    .unwrap()
                    .contains("names the graph itself"),
                "{snap}"
            );
            assert!(row["target"].as_str().unwrap().ends_with("g1"), "{snap}");
        }
    }
}

/// A predecessor whose valid period holds at no instant, superseded like any
/// other.
const DEGENERATE_PREDECESSOR: &str = r#"
@prefix ex: <http://example.org/> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g1 { ex:HEK293 a ex:AdherentCellLine . }
ex:g2 { ex:HEK293 a ex:SuspensionCellLine . }

{
  ex:g1 t:validFrom "2024-01-01" ; t:validTo "2024-01-01" ; t:recordedAt "2024-01-05" .
  ex:g2 t:validFrom "2024-01-01" ; t:recordedAt "2025-01-01" ; t:supersedes ex:g1 .
}
"#;

#[test]
fn a_predecessor_that_holds_at_no_instant_is_still_closed_by_its_successor() {
    let t = temporal(DEGENERATE_PREDECESSOR);
    // With no valid_at the empty period is not asked about, and before the
    // successor the graph is in scope like any other.
    let before = snapshot(&t, None, Some("2024-06-01"));
    assert!(graphs(&before, "in_scope").contains("g1"), "{before}");
    // After it, the derived bound closes the graph and the row names the
    // successor, exactly as for a sound predecessor.
    let after = snapshot(&t, None, Some("2026-01-01"));
    assert_eq!(reason(&after, "g1"), "no longer recorded then", "{after}");
    assert!(
        bucket_row(&after, "excluded", "g1")["superseded_by"]
            .as_str()
            .is_some_and(|g| g.ends_with("g2")),
        "{after}"
    );
}

/// A described graph asserts that it supersedes an undescribed one, and the
/// two disagree on a disjoint pair. The undescribed graph carries no temporal
/// description, so the link has no effect and the lineage pass says so; the
/// undescribed graph is timeless, and a timeless period overlaps everything.
const LINK_TO_AN_UNDESCRIBED_PARTNER: &str = r#"
@prefix ex:  <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix t:   <https://open-ontologies.org/temporal#> .

ex:g_b       { ex:HEK293 a ex:SuspensionCellLine . }
ex:g_nowhere { ex:HEK293 a ex:AdherentCellLine . }

{
  ex:AdherentCellLine owl:disjointWith ex:SuspensionCellLine .
  ex:g_b t:validFrom "2024-01-01" ; t:recordedAt "2026-06-01" ; t:supersedes ex:g_nowhere .
}
"#;

#[test]
fn a_link_the_lineage_pass_rejected_puts_no_pair_in_corrections() {
    let t = temporal(LINK_TO_AN_UNDESCRIBED_PARTNER);
    // The snapshot reports the link as having no effect, and the undescribed
    // graph stays in scope as timeless beside its would-be successor.
    let snap = snapshot(&t, None, Some("2027-01-01"));
    let rows = lineage_about(&snap, "g_b");
    assert_eq!(rows.len(), 1, "{snap}");
    assert!(
        rows[0]["reason"]
            .as_str()
            .unwrap()
            .contains("no temporal description"),
        "{snap}"
    );
    assert!(
        rows[0]["target"].as_str().unwrap().ends_with("g_nowhere"),
        "{snap}"
    );
    assert_eq!(
        graphs(&snap, "in_scope"),
        set(&["g_b", "g_nowhere"]),
        "{snap}"
    );
    // So the conflict check may not walk that link: the pair overlaps on its
    // periods and is a contradiction, not a correction. A walk over the raw
    // assertions would file it as a correction on a link the same answer
    // says closed nothing, and a genuine contradiction would be suppressed.
    let out = conflicts(&t);
    assert_eq!(out["contradiction_count"], 1, "{out}");
    assert_eq!(
        partners(&out, "contradictions"),
        set(&["g_b", "g_nowhere"]),
        "{out}"
    );
    assert!(out.get("corrections").is_none(), "{out}");
    assert!(out.get("corrections_count").is_none(), "{out}");
    assert_eq!(out["complete"], true, "{out}");
}
