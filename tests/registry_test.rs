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
    pub db_path: PathBuf,
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
        db_path,
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
    // The cache directory should hold a file in the cache format. Asserted
    // through the constant rather than a literal: the extension IS the format
    // marker (a file in any other one is treated as stale), so the two must
    // not be able to drift apart.
    let entries: Vec<_> = std::fs::read_dir(&h.cache_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == open_ontologies::cache::CACHE_EXT)
                .unwrap_or(false)
        })
        .collect();
    assert!(
        !entries.is_empty(),
        "expected a .{} file in cache dir",
        open_ontologies::cache::CACHE_EXT
    );
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

// ────────────────────────────────────────────────────────────────────────────
// Named graphs survive the cache
//
// The cache is only useful if what comes out equals what went in. It was
// written as N-Triples, which cannot carry a graph name, so a dataset went in
// and a flattened graph came out — on every load after the first, and on every
// reload after an idle eviction. The source never changed, so the freshness
// key was right and nothing anywhere reported a loss.
//
// Every test below loads TWICE on purpose. A test that loads once passes on
// the broken code, which is why this went unnoticed.
// ────────────────────────────────────────────────────────────────────────────

/// Two named graphs plus a description of them in the default graph — the
/// shape the bi-temporal tools read (`src/temporal.rs`).
const SAMPLE_TRIG: &str = r#"
@prefix ex: <http://example.org/test#> .
@prefix t:  <https://open-ontologies.org/temporal#> .

ex:g_v1 { ex:Doc ex:status ex:Draft . }
ex:g_v2 { ex:Doc ex:status ex:Published . }

{
  ex:g_v1 t:validFrom "2024-01-01" ; t:validTo "2026-05-01" .
  ex:g_v2 t:validFrom "2026-05-01" .
}
"#;

fn setup_trig(idle_ttl_secs: u64) -> Harness {
    let h = setup(idle_ttl_secs);
    let trig_path = h._tmp.path().join("sample.trig");
    std::fs::write(&trig_path, SAMPLE_TRIG).unwrap();
    Harness {
        source_path: trig_path,
        ..h
    }
}

/// The graph names the store currently holds, as reported by SPARQL rather
/// than by the loader that just claimed to have loaded them.
fn named_graphs(h: &Harness) -> Vec<String> {
    let raw = h
        .graph
        .sparql_select("SELECT DISTINCT ?g WHERE { GRAPH ?g { ?s ?p ?o } } ORDER BY ?g")
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    parsed["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["g"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn a_second_load_from_cache_keeps_the_named_graphs() {
    let h = setup_trig(0);
    let path = h.source_path.to_str().unwrap();

    let first = h.registry.load_file(path, LoadOptions::default()).unwrap();
    assert_eq!(first.origin, "source");
    let from_source = named_graphs(&h);
    assert_eq!(from_source.len(), 2, "fixture has two named graphs: {from_source:?}");

    // Same file, untouched: the cache is legitimately fresh and IS used. That
    // is the point — the bug was never in the freshness key.
    let second = h.registry.load_file(path, LoadOptions::default()).unwrap();
    assert_eq!(second.origin, "cache", "second load should hit the cache");
    assert_eq!(
        named_graphs(&h),
        from_source,
        "a cache round trip must return the dataset it was given, graph names included"
    );
    assert_eq!(second.triple_count, first.triple_count);
}

#[test]
fn a_reload_after_eviction_keeps_the_named_graphs() {
    // The path that needs no second load and no user action at all: the entry
    // goes idle, the evictor drops it, and the next read reloads from cache.
    let h = setup_trig(1);
    let path = h.source_path.to_str().unwrap();
    h.registry.load_file(path, LoadOptions::default()).unwrap();
    let before = named_graphs(&h);

    sleep(Duration::from_millis(1100));
    assert!(h.registry.evictor_tick().unwrap(), "entry should be evicted");
    h.registry.ensure_loaded().unwrap();

    assert_eq!(
        named_graphs(&h),
        before,
        "an eviction is a memory event, not a data change"
    );
}

#[test]
fn a_cache_file_left_by_an_older_build_is_recompiled_not_read() {
    // A `.nt` cache holds a flattened dataset. The source has not changed, so
    // every other freshness check passes: without the format check it would be
    // read back as authoritative and the graph names would be gone.
    let h = setup_trig(0);
    let path = h.source_path.to_str().unwrap();
    let first = h.registry.load_file(path, LoadOptions::default()).unwrap();
    let expected = named_graphs(&h);

    // Rewrite history: flatten the cache to N-Triples under the old name, and
    // point the metadata at it, exactly as an older build would have left it.
    let legacy = std::path::Path::new(&first.cache_path).with_extension("nt");
    let flattened = h.graph.serialize("ntriples").unwrap();
    std::fs::write(&legacy, &flattened).unwrap();
    let fp = SourceFingerprint::from_path(std::path::Path::new(path)).unwrap();
    let cm = CacheManager::new(
        h.cache_dir.clone(),
        StateDb::open(&h.db_path).unwrap(),
    )
    .unwrap();
    cm.upsert("sample", path, &fp, &legacy, first.triple_count)
        .unwrap();

    let second = h.registry.load_file(path, LoadOptions::default()).unwrap();
    assert_eq!(
        second.origin, "source",
        "a legacy cache file must be recompiled, not trusted"
    );
    assert_eq!(named_graphs(&h), expected);
    assert!(
        second.cache_path.ends_with(".nq"),
        "the rewritten cache carries the format that can hold graph names: {}",
        second.cache_path
    );
}
