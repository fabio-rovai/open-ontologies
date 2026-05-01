//! Integration tests for the compile cache (`src/cache.rs`) and the
//! ontology registry (`src/registry.rs`).
//!
//! These exercise features 1–4 from the task statement:
//!  1. compile/cache of loaded ontologies
//!  2. TTL-based eviction from memory
//!  3. transparent reload-on-query
//!  4. auto-refresh when source file changes

use std::path::PathBuf;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use open_ontologies::cache::{CacheManager, SourceFingerprint};
use open_ontologies::config::CacheConfig;
use open_ontologies::graph::GraphStore;
use open_ontologies::registry::{LoadOptions, OntologyRegistry};
use open_ontologies::state::StateDb;

const SAMPLE_TTL: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/test#> .

ex:Animal a owl:Class ;
    rdfs:label "Animal" .

ex:Dog a owl:Class ;
    rdfs:subClassOf ex:Animal ;
    rdfs:label "Dog" .
"#;

const SAMPLE_TTL_V2: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/test#> .

ex:Animal a owl:Class ;
    rdfs:label "Animal" .

ex:Dog a owl:Class ;
    rdfs:subClassOf ex:Animal ;
    rdfs:label "Dog" .

ex:Cat a owl:Class ;
    rdfs:subClassOf ex:Animal ;
    rdfs:label "Cat" .

ex:Bird a owl:Class ;
    rdfs:subClassOf ex:Animal ;
    rdfs:label "Bird" .
"#;

struct Harness {
    _tmp: tempfile::TempDir,
    pub source_path: PathBuf,
    pub cache_dir: PathBuf,
    pub registry: Arc<OntologyRegistry>,
    pub graph: Arc<GraphStore>,
}

fn setup(idle_ttl_secs: u64) -> Harness {
    let tmp = tempfile::tempdir().unwrap();
    let source_path = tmp.path().join("sample.ttl");
    std::fs::write(&source_path, SAMPLE_TTL).unwrap();
    let cache_dir = tmp.path().join("cache");
    let db_path = tmp.path().join("state.db");
    let db = StateDb::open(&db_path).unwrap();

    let graph = Arc::new(GraphStore::new());
    let cfg = CacheConfig {
        enabled: true,
        dir: cache_dir.to_string_lossy().into_owned(),
        idle_ttl_secs,
        evictor_interval_secs: 1,
        auto_refresh: false,
        hash_prefix_bytes: 64 * 1024,
    };
    let registry = Arc::new(OntologyRegistry::new(graph.clone(), db, cfg).unwrap());
    Harness {
        _tmp: tmp,
        source_path,
        cache_dir,
        registry,
        graph,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Feature 1 — compile cache
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn first_load_writes_cache_file() {
    let h = setup(0);
    let res = h
        .registry
        .load_file(
            h.source_path.to_str().unwrap(),
            LoadOptions::default(),
        )
        .unwrap();
    assert_eq!(res.origin, "source", "first load should be from source");
    assert!(res.triple_count > 0);
    assert!(
        std::path::Path::new(&res.cache_path).exists(),
        "cache file should exist on disk"
    );
    // Cache directory should contain at least one .nt file.
    let entries: Vec<_> = std::fs::read_dir(&h.cache_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "nt")
                .unwrap_or(false)
        })
        .collect();
    assert!(!entries.is_empty(), "expected an .nt file in cache dir");
}

#[test]
fn second_load_uses_cache_when_unchanged() {
    let h = setup(0);
    let r1 = h
        .registry
        .load_file(h.source_path.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    assert_eq!(r1.origin, "source");

    // Re-load: the source file is unchanged so we should hit the cache.
    let r2 = h
        .registry
        .load_file(h.source_path.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    assert_eq!(r2.origin, "cache");
    assert_eq!(r1.triple_count, r2.triple_count);
}

#[test]
fn force_recompile_bypasses_cache() {
    let h = setup(0);
    h.registry
        .load_file(h.source_path.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    // Even with a fresh cache, force_recompile=true must re-parse from source.
    let r = h
        .registry
        .load_file(
            h.source_path.to_str().unwrap(),
            LoadOptions {
                force_recompile: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(r.origin, "source");
}

#[test]
fn changing_source_invalidates_cache() {
    let h = setup(0);
    let r1 = h
        .registry
        .load_file(h.source_path.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    let initial = r1.triple_count;

    // Modify source. Sleep > 1s so mtime resolution doesn't mask the change.
    sleep(Duration::from_millis(1100));
    std::fs::write(&h.source_path, SAMPLE_TTL_V2).unwrap();

    let r2 = h
        .registry
        .load_file(h.source_path.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    assert_eq!(r2.origin, "source", "modified source should bypass cache");
    assert!(r2.triple_count > initial, "v2 has more triples than v1");
}

#[test]
fn cache_disabled_skips_ondisk_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let source_path = tmp.path().join("sample.ttl");
    std::fs::write(&source_path, SAMPLE_TTL).unwrap();
    let db_path = tmp.path().join("state.db");
    let db = StateDb::open(&db_path).unwrap();
    let graph = Arc::new(GraphStore::new());
    let cfg = CacheConfig {
        enabled: false,
        dir: tmp.path().join("cache").to_string_lossy().into_owned(),
        idle_ttl_secs: 0,
        evictor_interval_secs: 30,
        auto_refresh: false,
        hash_prefix_bytes: 64 * 1024,
    };
    let registry = OntologyRegistry::new(graph, db, cfg).unwrap();
    let r = registry
        .load_file(source_path.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    assert_eq!(r.origin, "source");
    assert!(r.cache_path.is_empty(), "no cache path when disabled");
}

#[test]
fn fingerprint_round_trips_through_cache_manager() {
    let tmp = tempfile::tempdir().unwrap();
    let f = tmp.path().join("f.txt");
    std::fs::write(&f, b"hello").unwrap();
    let fp = SourceFingerprint::from_path(&f).unwrap();
    assert_eq!(fp.size, 5);
    assert!(!fp.sha_prefix.is_empty());
}

#[test]
fn cache_manager_upsert_and_get_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("s.db")).unwrap();
    let cm = CacheManager::new(tmp.path().to_path_buf(), db).unwrap();
    let f = tmp.path().join("a.ttl");
    std::fs::write(&f, b"ignored").unwrap();
    let fp = SourceFingerprint::from_path(&f).unwrap();
    let cp = cm.cache_path_for("a", &fp.sha_prefix);
    CacheManager::atomic_write(&cp, "<a> <b> <c> .\n").unwrap();
    cm.upsert("a", f.to_str().unwrap(), &fp, &cp, 1).unwrap();

    let got = cm.get("a").unwrap().expect("entry present");
    assert_eq!(got.name, "a");
    assert_eq!(got.triple_count, 1);
    assert!(cm.is_fresh(&got).unwrap());

    // Modifying the source invalidates freshness.
    sleep(Duration::from_millis(1100));
    std::fs::write(&f, b"different content").unwrap();
    let got2 = cm.get("a").unwrap().unwrap();
    assert!(!cm.is_fresh(&got2).unwrap());

    // Removal cleans up disk + row.
    cm.remove("a").unwrap();
    assert!(cm.get("a").unwrap().is_none());
    assert!(!cp.exists());
}

// ────────────────────────────────────────────────────────────────────────────
// Feature 2 + 3 — TTL eviction + auto-reload-on-query
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn evictor_unloads_idle_ontology_and_keeps_cache() {
    let h = setup(/* idle_ttl_secs */ 1);
    let _ = h
        .registry
        .load_file(h.source_path.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    assert!(h.graph.triple_count() > 0);

    // Wait past the TTL, then invoke the evictor.
    sleep(Duration::from_millis(1100));
    let evicted = h.registry.evictor_tick().unwrap();
    assert!(evicted, "evictor should have unloaded the idle ontology");
    assert_eq!(h.graph.triple_count(), 0, "graph should be empty");

    // Status should report `evicted: true`.
    let status = h.registry.status();
    assert_eq!(status["active"]["evicted"], true);
}

#[test]
fn ensure_loaded_reloads_after_eviction() {
    let h = setup(1);
    let r1 = h
        .registry
        .load_file(h.source_path.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    let original = r1.triple_count;

    // Evict.
    sleep(Duration::from_millis(1100));
    assert!(h.registry.evictor_tick().unwrap());
    assert_eq!(h.graph.triple_count(), 0);

    // Simulate a query: ensure_loaded must transparently reload.
    h.registry.ensure_loaded().unwrap();
    assert_eq!(
        h.graph.triple_count(),
        original,
        "store should be reloaded with the same triple count"
    );

    // Active entry should be marked not-evicted again.
    let status = h.registry.status();
    assert_eq!(status["active"]["evicted"], false);
}

#[test]
fn touch_postpones_eviction() {
    let h = setup(2);
    h.registry
        .load_file(h.source_path.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    // Less than TTL: touch resets the access timestamp.
    sleep(Duration::from_millis(800));
    h.registry.touch();
    sleep(Duration::from_millis(800));
    let evicted = h.registry.evictor_tick().unwrap();
    assert!(!evicted, "touch should keep the entry alive");
    assert!(h.graph.triple_count() > 0);
}

#[test]
fn unload_drops_active_entry() {
    let h = setup(0);
    h.registry
        .load_file(h.source_path.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    let name = h.registry.unload(false).unwrap();
    assert!(name.is_some());
    assert_eq!(h.graph.triple_count(), 0);

    // ensure_loaded with no active entry is a no-op.
    h.registry.ensure_loaded().unwrap();
    assert_eq!(h.graph.triple_count(), 0);
}

#[test]
fn unload_with_delete_cache_removes_file() {
    let h = setup(0);
    let r = h
        .registry
        .load_file(h.source_path.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    let cache_path = std::path::PathBuf::from(&r.cache_path);
    assert!(cache_path.exists());
    h.registry.unload(true).unwrap();
    assert!(!cache_path.exists(), "delete_cache should remove the .nt file");
}

// ────────────────────────────────────────────────────────────────────────────
// Feature 4 — auto-refresh when source file changes
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn auto_refresh_picks_up_changes_on_ensure_loaded() {
    let h = setup(0);
    let r1 = h
        .registry
        .load_file(
            h.source_path.to_str().unwrap(),
            LoadOptions {
                auto_refresh: true,
                ..Default::default()
            },
        )
        .unwrap();
    let v1 = r1.triple_count;

    // Modify source.
    sleep(Duration::from_millis(1100));
    std::fs::write(&h.source_path, SAMPLE_TTL_V2).unwrap();

    // ensure_loaded should detect the change and recompile.
    h.registry.ensure_loaded().unwrap();
    let v2 = h.graph.triple_count();
    assert!(
        v2 > v1,
        "auto_refresh should pick up the larger v2 ontology (v1={}, v2={})",
        v1,
        v2
    );
}

#[test]
fn no_auto_refresh_keeps_old_data_until_explicit_recompile() {
    let h = setup(0);
    let r1 = h
        .registry
        .load_file(
            h.source_path.to_str().unwrap(),
            LoadOptions {
                auto_refresh: false,
                ..Default::default()
            },
        )
        .unwrap();
    let v1 = r1.triple_count;

    sleep(Duration::from_millis(1100));
    std::fs::write(&h.source_path, SAMPLE_TTL_V2).unwrap();

    h.registry.ensure_loaded().unwrap();
    assert_eq!(
        h.graph.triple_count(),
        v1,
        "without auto_refresh, ensure_loaded must NOT reload from changed source"
    );

    // Manual recompile picks up the change.
    let r2 = h.registry.recompile().unwrap();
    assert!(r2.triple_count > v1);
    assert_eq!(r2.origin, "source");
}

#[test]
fn auto_refresh_after_eviction_uses_new_source() {
    let h = setup(1);
    h.registry
        .load_file(
            h.source_path.to_str().unwrap(),
            LoadOptions {
                auto_refresh: true,
                ..Default::default()
            },
        )
        .unwrap();
    let v1 = h.graph.triple_count();

    // Evict, then change source.
    sleep(Duration::from_millis(1100));
    assert!(h.registry.evictor_tick().unwrap());
    std::fs::write(&h.source_path, SAMPLE_TTL_V2).unwrap();

    // ensure_loaded should:
    //  - notice that the source changed (auto_refresh)
    //  - recompile from the new source rather than reload the stale cache
    h.registry.ensure_loaded().unwrap();
    let v2 = h.graph.triple_count();
    assert!(v2 > v1, "expected refreshed (larger) ontology after change");
}
