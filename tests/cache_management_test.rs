//! Tests for per-name cache management:
//!  - `OntologyRegistry::list_cached`
//!  - `OntologyRegistry::unload_named`
//!  - `OntologyRegistry::recompile_named`
//!
//! These complement `tests/registry_test.rs` (which exercises the active-slot
//! lifecycle) by covering operations on cache entries that are not the
//! currently-active ontology.

use std::path::PathBuf;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use open_ontologies::config::CacheConfig;
use open_ontologies::graph::GraphStore;
use open_ontologies::registry::{LoadOptions, OntologyRegistry};
use open_ontologies::state::StateDb;

const TTL_A: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/a#> .
ex:A1 a owl:Class ; rdfs:label "A1" .
ex:A2 a owl:Class ; rdfs:label "A2" .
"#;

const TTL_B: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/b#> .
ex:B1 a owl:Class ; rdfs:label "B1" .
ex:B2 a owl:Class ; rdfs:label "B2" .
ex:B3 a owl:Class ; rdfs:label "B3" .
"#;

const TTL_B_V2: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/b#> .
ex:B1 a owl:Class ; rdfs:label "B1" .
ex:B2 a owl:Class ; rdfs:label "B2" .
ex:B3 a owl:Class ; rdfs:label "B3" .
ex:B4 a owl:Class ; rdfs:label "B4" .
ex:B5 a owl:Class ; rdfs:label "B5" .
"#;

struct H {
    _tmp: tempfile::TempDir,
    pub path_a: PathBuf,
    pub path_b: PathBuf,
    pub registry: Arc<OntologyRegistry>,
    pub graph: Arc<GraphStore>,
}

fn setup() -> H {
    let tmp = tempfile::tempdir().unwrap();
    let path_a = tmp.path().join("a.ttl");
    let path_b = tmp.path().join("b.ttl");
    std::fs::write(&path_a, TTL_A).unwrap();
    std::fs::write(&path_b, TTL_B).unwrap();
    let db = StateDb::open(&tmp.path().join("s.db")).unwrap();
    let graph = Arc::new(GraphStore::new());
    let cfg = CacheConfig {
        enabled: true,
        dir: tmp.path().join("cache").to_string_lossy().into_owned(),
        idle_ttl_secs: 0,
        evictor_interval_secs: 30,
        auto_refresh: false,
    };
    let registry = Arc::new(OntologyRegistry::new(graph.clone(), db, cfg).unwrap());
    H { _tmp: tmp, path_a, path_b, registry, graph }
}

/// Load A then B so B is active, A is cached but not active.
fn populate_two(h: &H) {
    h.registry
        .load_file(h.path_a.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    h.registry
        .load_file(h.path_b.to_str().unwrap(), LoadOptions::default())
        .unwrap();
}

// ────────────────────────────────────────────────────────────────────────────
// list_cached
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn list_cached_empty_when_nothing_loaded() {
    let h = setup();
    let list = h.registry.list_cached().unwrap();
    assert!(list.is_empty());
}

#[test]
fn list_cached_includes_all_loaded_ontologies() {
    let h = setup();
    populate_two(&h);
    let list = h.registry.list_cached().unwrap();
    assert_eq!(list.len(), 2, "both A and B should be cached");
    let names: Vec<&str> = list
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"a"));
    assert!(names.contains(&"b"));
}

#[test]
fn list_cached_flags_active_and_in_memory() {
    let h = setup();
    populate_two(&h);
    let list = h.registry.list_cached().unwrap();
    let b = list.iter().find(|e| e["name"] == "b").unwrap();
    let a = list.iter().find(|e| e["name"] == "a").unwrap();
    assert_eq!(b["is_active"], true, "B is the most-recently-loaded -> active");
    assert_eq!(b["in_memory"], true);
    assert_eq!(a["is_active"], false, "A is cached but not active");
    assert_eq!(a["in_memory"], false);
}

#[test]
fn list_cached_marks_in_memory_false_after_eviction() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("a.ttl");
    std::fs::write(&path, TTL_A).unwrap();
    let db = StateDb::open(&tmp.path().join("s.db")).unwrap();
    let graph = Arc::new(GraphStore::new());
    let cfg = CacheConfig {
        enabled: true,
        dir: tmp.path().join("cache").to_string_lossy().into_owned(),
        idle_ttl_secs: 1,
        evictor_interval_secs: 30,
        auto_refresh: false,
    };
    let reg = OntologyRegistry::new(graph, db, cfg).unwrap();
    reg.load_file(path.to_str().unwrap(), LoadOptions::default()).unwrap();
    sleep(Duration::from_millis(1100));
    assert!(reg.evictor_tick().unwrap());
    let list = reg.list_cached().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["is_active"], true, "still the active slot");
    assert_eq!(list[0]["in_memory"], false, "but evicted from memory");
}

// ────────────────────────────────────────────────────────────────────────────
// unload_named
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn unload_named_unknown_returns_error() {
    let h = setup();
    populate_two(&h);
    let err = h.registry.unload_named("does-not-exist", false).unwrap_err();
    assert!(
        err.to_string().contains("does-not-exist"),
        "error message should mention the name; got: {}",
        err
    );
}

#[test]
fn unload_named_non_active_with_delete_removes_only_that_entry() {
    let h = setup();
    populate_two(&h);
    let active_triples = h.graph.triple_count();
    assert!(active_triples > 0);

    // A is non-active. Delete it; B (active) must be unaffected.
    let changed = h.registry.unload_named("a", true).unwrap();
    assert!(changed);

    assert_eq!(
        h.graph.triple_count(),
        active_triples,
        "removing a non-active entry must NOT touch the in-memory active store"
    );

    let list = h.registry.list_cached().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["name"], "b");
}

#[test]
fn unload_named_non_active_without_delete_is_noop_but_validates_name() {
    let h = setup();
    populate_two(&h);
    // A is non-active. With delete_cache=false there is nothing to do
    // (it's not in memory and we are told not to touch the cache file).
    let changed = h.registry.unload_named("a", false).unwrap();
    assert!(!changed);
    // But the entry is still present.
    let list = h.registry.list_cached().unwrap();
    assert_eq!(list.len(), 2);
}

#[test]
fn unload_named_active_unloads_and_optionally_deletes() {
    let h = setup();
    populate_two(&h);
    // B is active. Unload it WITHOUT deleting cache.
    let changed = h.registry.unload_named("b", false).unwrap();
    assert!(changed);
    assert_eq!(h.graph.triple_count(), 0);

    let list = h.registry.list_cached().unwrap();
    assert_eq!(list.len(), 2, "cache file for B should still exist");
    let b = list.iter().find(|e| e["name"] == "b").unwrap();
    assert_eq!(b["is_active"], false);
}

#[test]
fn unload_named_active_with_delete_removes_db_row_and_file() {
    let h = setup();
    populate_two(&h);
    let b_cache = h
        .registry
        .list_cached()
        .unwrap()
        .into_iter()
        .find(|e| e["name"] == "b")
        .unwrap()["cache_path"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(std::path::Path::new(&b_cache).exists());

    h.registry.unload_named("b", true).unwrap();
    assert_eq!(h.graph.triple_count(), 0);
    assert!(!std::path::Path::new(&b_cache).exists(), "cache file deleted");

    let list = h.registry.list_cached().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["name"], "a");
}

// ────────────────────────────────────────────────────────────────────────────
// recompile_named
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn recompile_named_unknown_returns_error() {
    let h = setup();
    let err = h.registry.recompile_named("nope").unwrap_err();
    assert!(err.to_string().contains("nope"));
}

#[test]
fn recompile_named_active_behaves_like_recompile() {
    let h = setup();
    populate_two(&h);
    // B is active. Modify its source.
    sleep(Duration::from_millis(1100));
    std::fs::write(&h.path_b, TTL_B_V2).unwrap();
    let res = h.registry.recompile_named("b").unwrap();
    assert_eq!(res.origin, "source");
    assert_eq!(res.name, "b");
    // Active in-memory store reflects v2.
    assert!(h.graph.triple_count() > 0);
    let list = h.registry.list_cached().unwrap();
    let b = list.iter().find(|e| e["name"] == "b").unwrap();
    assert!(b["triple_count"].as_u64().unwrap() > 5);
}

#[test]
fn recompile_named_non_active_does_not_touch_active_memory() {
    let h = setup();
    populate_two(&h);
    let active_count_before = h.graph.triple_count();

    // Modify A (non-active) and recompile by name.
    sleep(Duration::from_millis(1100));
    std::fs::write(&h.path_a, TTL_B_V2).unwrap(); // give A more triples
    let res = h.registry.recompile_named("a").unwrap();
    assert_eq!(res.origin, "source");
    assert_eq!(res.name, "a");

    // The active in-memory store (B) must be untouched.
    assert_eq!(
        h.graph.triple_count(),
        active_count_before,
        "active in-memory store must not be disturbed by recompiling a non-active entry"
    );
    // A's cache row reflects the new content.
    let list = h.registry.list_cached().unwrap();
    let a = list.iter().find(|e| e["name"] == "a").unwrap();
    assert_eq!(a["is_active"], false);
    // v2 has more classes than the original A (which had 2).
    assert!(
        a["triple_count"].as_u64().unwrap() > 2,
        "A's cached triple count should have grown after recompile"
    );
}

#[test]
fn recompile_named_missing_source_returns_clear_error() {
    let h = setup();
    populate_two(&h);
    // Delete A's source file.
    std::fs::remove_file(&h.path_a).unwrap();
    let err = h.registry.recompile_named("a").unwrap_err();
    assert!(
        err.to_string().contains("missing") || err.to_string().contains("source"),
        "error should mention the missing source; got: {}",
        err
    );
    // Active store still untouched.
    assert!(h.graph.triple_count() > 0);
}

#[test]
fn recompile_named_garbage_collects_old_cache_file_when_sha_changes() {
    let h = setup();
    populate_two(&h);
    let old_a_cache = h
        .registry
        .list_cached()
        .unwrap()
        .into_iter()
        .find(|e| e["name"] == "a")
        .unwrap()["cache_path"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(std::path::Path::new(&old_a_cache).exists());

    sleep(Duration::from_millis(1100));
    std::fs::write(&h.path_a, TTL_B_V2).unwrap(); // very different content -> different sha
    h.registry.recompile_named("a").unwrap();

    let new_a_cache = h
        .registry
        .list_cached()
        .unwrap()
        .into_iter()
        .find(|e| e["name"] == "a")
        .unwrap()["cache_path"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(new_a_cache, old_a_cache, "new cache file path expected");
    assert!(!std::path::Path::new(&old_a_cache).exists(),
        "old cache file should be cleaned up");
    assert!(std::path::Path::new(&new_a_cache).exists());
}
