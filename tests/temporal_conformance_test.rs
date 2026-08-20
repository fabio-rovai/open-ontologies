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
//! Dates are kept in a single lexical form (`YYYY-MM-DD`) on purpose — the
//! comparison is currently lexical, and mixing `xsd:date` with `xsd:dateTime`
//! or timezone offsets is a separate, still-open point on #95.

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
        out["superseded_count"], 1,
        "the earlier assertion is superseded history, not a live conflict"
    );
}
