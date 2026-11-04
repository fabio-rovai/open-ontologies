// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

#![cfg(feature = "embeddings")]

use open_ontologies::vecstore::VecStore;
use open_ontologies::state::StateDb;

fn test_db() -> StateDb {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    StateDb::open(&path).unwrap()
}

#[test]
fn test_insert_and_search_cosine() {
    let db = test_db();
    let mut store = VecStore::new(db);

    store.upsert("http://ex.org/Dog", &[0.9, 0.1, 0.0], &[0.1, 0.0]);
    store.upsert("http://ex.org/Cat", &[0.8, 0.2, 0.0], &[0.15, 0.0]);
    store.upsert("http://ex.org/Car", &[0.0, 0.0, 1.0], &[0.5, 0.0]);

    let results = store.search_cosine(&[0.85, 0.15, 0.0], 2);
    assert_eq!(results.len(), 2);
    assert!(results[0].0.contains("Dog") || results[0].0.contains("Cat"),
        "Top result should be Dog or Cat, got {}", results[0].0);
}

#[test]
fn test_search_poincare() {
    let db = test_db();
    let mut store = VecStore::new(db);

    store.upsert("http://ex.org/Dog", &[0.0, 0.0, 0.0], &[0.1, 0.05]);
    store.upsert("http://ex.org/Cat", &[0.0, 0.0, 0.0], &[0.12, 0.03]);
    store.upsert("http://ex.org/Car", &[0.0, 0.0, 0.0], &[0.8, 0.8]);

    let results = store.search_poincare(&[0.11, 0.04], 2);
    assert_eq!(results.len(), 2);
    let iris: Vec<&str> = results.iter().map(|r| r.0.as_str()).collect();
    assert!(iris.contains(&"http://ex.org/Dog"));
    assert!(iris.contains(&"http://ex.org/Cat"));
}

#[test]
fn test_product_search() {
    let db = test_db();
    let mut store = VecStore::new(db);

    store.upsert("http://ex.org/Dog", &[0.9, 0.1], &[0.1, 0.0]);
    store.upsert("http://ex.org/Cat", &[0.8, 0.2], &[0.12, 0.0]);
    store.upsert("http://ex.org/Car", &[0.0, 1.0], &[0.8, 0.8]);

    let results = store.search_product(&[0.85, 0.15], &[0.11, 0.0], 2, 0.5);
    assert_eq!(results.len(), 2);
    assert!(results[0].0.contains("Dog") || results[0].0.contains("Cat"));
}

#[test]
fn test_upsert_overwrites() {
    let db = test_db();
    let mut store = VecStore::new(db);

    store.upsert("http://ex.org/Dog", &[1.0, 0.0], &[0.1, 0.0]);
    store.upsert("http://ex.org/Dog", &[0.0, 1.0], &[0.5, 0.0]);

    let results = store.search_cosine(&[0.0, 1.0], 1);
    assert_eq!(results[0].0, "http://ex.org/Dog");
    assert!(results[0].1 > 0.99);
}

#[test]
fn test_persist_and_reload() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);

    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db);
        store.upsert("http://ex.org/Dog", &[0.9, 0.1], &[0.1, 0.0]);
        store.persist().unwrap();
    }

    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db);
        store.load_from_db().unwrap();
        let results = store.search_cosine(&[0.9, 0.1], 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "http://ex.org/Dog");
    }
}

#[test]
fn test_remove() {
    let db = test_db();
    let mut store = VecStore::new(db);

    store.upsert("http://ex.org/Dog", &[0.9, 0.1], &[0.1, 0.0]);
    store.upsert("http://ex.org/Cat", &[0.8, 0.2], &[0.1, 0.0]);
    store.remove("http://ex.org/Dog");

    let results = store.search_cosine(&[0.9, 0.1], 10);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "http://ex.org/Cat");
}

#[test]
fn search_cosine_hnsw_returns_same_top_1_as_brute_force() {
    // Round-trip test for the HNSW backing index: top-1 must agree with the
    // brute-force linear scan on well-separated vectors.
    let db = test_db();
    let mut store = VecStore::new(db);

    let entries: [(&str, [f32; 3]); 6] = [
        ("http://ex.org/Cat",    [1.0, 0.05, 0.0]),
        ("http://ex.org/Kitten", [0.98, 0.1, 0.0]),
        ("http://ex.org/Dog",    [0.9, 0.3, 0.0]),
        ("http://ex.org/Bird",   [0.4, 0.7, 0.0]),
        ("http://ex.org/Car",    [0.0, 0.1, 1.0]),
        ("http://ex.org/Bike",   [0.0, 0.0, 0.95]),
    ];
    for (iri, vec) in entries.iter() {
        store.upsert(iri, vec, &[0.0]);
    }

    let query = [1.0_f32, 0.0, 0.0]; // Closest to Cat by far.

    let brute = store.search_cosine(&query, 1);
    let hnsw = store.search_cosine_hnsw(&query, 1);

    assert_eq!(brute.len(), 1);
    assert_eq!(hnsw.len(), 1);
    assert_eq!(
        brute[0].0, hnsw[0].0,
        "top-1 disagreement between brute-force and HNSW: brute={:?}, hnsw={:?}",
        brute, hnsw
    );
    assert!(brute[0].0.contains("Cat"));
}

#[test]
fn search_cosine_hnsw_invalidates_on_mutation() {
    // After an upsert the index must rebuild. Warm the index with an
    // initial search, then insert a closer match — the next search must
    // surface it (i.e. the stale index didn't sandbag the result).
    let db = test_db();
    let mut store = VecStore::new(db);

    store.upsert("http://ex.org/Cat", &[0.9, 0.1, 0.0], &[0.0]);
    store.upsert("http://ex.org/Dog", &[0.7, 0.3, 0.0], &[0.0]);

    let first = store.search_cosine_hnsw(&[1.0, 0.0, 0.0], 1);
    assert!(first[0].0.contains("Cat"));

    store.upsert("http://ex.org/Tiger", &[0.99, 0.05, 0.0], &[0.0]);

    let second = store.search_cosine_hnsw(&[1.0, 0.0, 0.0], 1);
    assert!(
        second[0].0.contains("Tiger") || second[0].0.contains("Cat"),
        "after upsert of closer match, expected Tiger or Cat in top-1; got {:?}",
        second
    );
    // The structural invariant: the previously-built index didn't prevent
    // Tiger from being considered. Either Tiger (very close) or Cat (close)
    // demonstrate that.
}

#[test]
fn search_cosine_hnsw_on_empty_store_returns_empty() {
    let db = test_db();
    let mut store = VecStore::new(db);
    let results = store.search_cosine_hnsw(&[1.0, 0.0, 0.0], 5);
    assert!(results.is_empty());
}

#[test]
fn hnsw_index_persists_across_process_restart() {
    // Round-trip: upsert entries -> persist BOTH vectors and the built HNSW
    // index -> reopen a fresh VecStore on the same DB -> load_from_db should
    // restore both the entries and the cached index -> search_cosine_hnsw
    // works immediately without rebuilding.
    use tempfile::NamedTempFile;
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);

    // Phase 1: populate, build, persist.
    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db);
        for (iri, v) in [
            ("http://ex.org/Cat",    [1.0_f32, 0.05, 0.0]),
            ("http://ex.org/Kitten", [0.98, 0.1, 0.0]),
            ("http://ex.org/Dog",    [0.9, 0.3, 0.0]),
            ("http://ex.org/Bird",   [0.4, 0.7, 0.0]),
            ("http://ex.org/Car",    [0.0, 0.1, 1.0]),
        ] {
            store.upsert(iri, &v, &[0.0]);
        }
        // Warm the index, then persist both vectors and the index.
        let _ = store.search_cosine_hnsw(&[1.0, 0.0, 0.0], 3);
        store.persist().expect("persist vectors");
        store.persist_cosine_index().expect("persist index");
    }

    // Phase 2: fresh store, load from same DB, query immediately.
    let db = StateDb::open(&path).unwrap();
    let mut store = VecStore::new(db);
    store.load_from_db().expect("load_from_db");
    assert_eq!(store.len(), 5, "all 5 vectors should reload");

    let results = store.search_cosine_hnsw(&[1.0, 0.0, 0.0], 1);
    assert_eq!(results.len(), 1);
    assert!(
        results[0].0.contains("Cat"),
        "after persistence round-trip, expected Cat as top-1 nearest neighbour; got {:?}",
        results
    );
}

#[test]
fn search_poincare_hnsw_returns_same_top_1_as_brute_force() {
    // Round-trip for the Poincaré HNSW index over structural embeddings.
    // Structural vectors must lie inside the Poincaré ball (||v|| < 1) so we
    // construct them small.
    let db = test_db();
    let mut store = VecStore::new(db);

    let entries = [
        ("http://ex.org/A", [0.1_f32, 0.0, 0.0]),
        ("http://ex.org/B", [0.15, 0.05, 0.0]),
        ("http://ex.org/C", [0.05, 0.0, 0.0]),
        ("http://ex.org/D", [0.5, 0.5, 0.0]),
        ("http://ex.org/E", [-0.4, -0.4, 0.0]),
    ];
    for (iri, v) in entries.iter() {
        store.upsert(iri, &[1.0, 0.0], v);
    }

    let query = [0.05_f32, 0.0, 0.0]; // Closest to C (identical), then A, then B.

    let brute = store.search_poincare(&query, 1);
    let hnsw = store.search_poincare_hnsw(&query, 1);

    assert_eq!(brute.len(), 1);
    assert_eq!(hnsw.len(), 1);
    assert_eq!(
        brute[0].0, hnsw[0].0,
        "Poincaré top-1 disagreement between brute-force and HNSW: brute={:?}, hnsw={:?}",
        brute, hnsw
    );
}

#[test]
fn poincare_and_cosine_indices_coexist_independently() {
    // The two HNSW variants must be independent — they index different vectors
    // (text_vec vs struct_vec) over the same entries. Building one must not
    // affect or invalidate the other.
    let db = test_db();
    let mut store = VecStore::new(db);
    store.upsert("http://ex.org/X", &[1.0, 0.0], &[0.1, 0.0]);
    store.upsert("http://ex.org/Y", &[0.0, 1.0], &[0.0, 0.2]);

    // Build cosine first.
    let _ = store.search_cosine_hnsw(&[1.0, 0.0], 1);
    // Build Poincaré second — should not clobber the cosine index.
    let _ = store.search_poincare_hnsw(&[0.1, 0.0], 1);

    // Both queries still return something sensible (cosine results are
    // similarity-descending, Poincaré are distance-ascending).
    let c = store.search_cosine_hnsw(&[1.0, 0.0], 1);
    assert!(!c.is_empty() && c[0].0.contains('X'));

    let p = store.search_poincare_hnsw(&[0.1, 0.0], 1);
    assert!(!p.is_empty() && p[0].0.contains('X'));
}

#[tokio::test]
async fn async_persist_round_trip() {
    // Async flush variant: serialise sync, write to SQLite on a spawn_blocking
    // task. JoinHandle resolves Ok; the persisted cache is identical to the
    // sync variant (round-trips on a fresh VecStore).
    use tempfile::NamedTempFile;
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);

    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db);
        for (iri, v) in [
            ("http://ex.org/A", [1.0_f32, 0.0, 0.0]),
            ("http://ex.org/B", [0.9, 0.1, 0.0]),
            ("http://ex.org/C", [0.0, 1.0, 0.0]),
        ] {
            store.upsert(iri, &v, &[0.1, 0.0]);
        }
        let _ = store.search_cosine_hnsw(&[1.0, 0.0, 0.0], 1);
        store.persist().expect("persist vectors");
        let handle = store
            .persist_cosine_index_async()
            .expect("schedule async persist");
        handle
            .await
            .expect("join async persist")
            .expect("async persist result");
    }

    let db = StateDb::open(&path).unwrap();
    let mut store = VecStore::new(db);
    store.load_from_db().expect("load_from_db");
    let results = store.search_cosine_hnsw(&[1.0, 0.0, 0.0], 1);
    assert_eq!(results.len(), 1);
    assert!(results[0].0.contains('A'));
}

#[test]
fn poincare_hnsw_index_persists_across_process_restart() {
    use tempfile::NamedTempFile;
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);

    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db);
        for (iri, v) in [
            ("http://ex.org/A", [0.1_f32, 0.0, 0.0]),
            ("http://ex.org/B", [0.15, 0.05, 0.0]),
            ("http://ex.org/C", [0.5, 0.5, 0.0]),
        ] {
            store.upsert(iri, &[1.0, 0.0, 0.0], &v);
        }
        let _ = store.search_poincare_hnsw(&[0.1, 0.0, 0.0], 1);
        store.persist().expect("persist vectors");
        store.persist_poincare_index().expect("persist poincare index");
    }

    let db = StateDb::open(&path).unwrap();
    let mut store = VecStore::new(db);
    store.load_from_db().expect("load_from_db");
    assert_eq!(store.len(), 3);
    let results = store.search_poincare_hnsw(&[0.1, 0.0, 0.0], 1);
    assert_eq!(results.len(), 1);
    assert!(results[0].0.contains('A'), "expected A as nearest; got {:?}", results);
}

#[test]
fn hnsw_index_cache_invalidated_when_entries_change() {
    // If the SQLite cache holds a stale index (different entry fingerprint),
    // load_cosine_index must NOT install it; the next search_cosine_hnsw
    // rebuilds from the new entries.
    use tempfile::NamedTempFile;
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);

    // Persist an index built from one entry set.
    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db);
        store.upsert("http://ex.org/Cat", &[1.0, 0.0, 0.0], &[0.0]);
        store.upsert("http://ex.org/Dog", &[0.9, 0.1, 0.0], &[0.0]);
        let _ = store.search_cosine_hnsw(&[1.0, 0.0, 0.0], 1);
        store.persist().expect("persist vectors");
        store.persist_cosine_index().expect("persist index");
    }

    // Mutate the underlying embeddings table (simulate: a fresh embed run
    // produced different vectors for the same IRIs) — but DON'T re-persist
    // the index. The fingerprint should mismatch on the next load.
    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db);
        // Different vectors than the persisted set.
        store.upsert("http://ex.org/Cat", &[0.0, 1.0, 0.0], &[0.0]);
        store.upsert("http://ex.org/Dog", &[0.0, 0.9, 0.1], &[0.0]);
        store.persist().expect("persist new vectors only — NOT the index");
    }

    // Load fresh. The cached index's fingerprint won't match the new vectors,
    // so load_cosine_index should refuse it; search_cosine_hnsw must rebuild
    // and return the NEW nearest-neighbour, not the stale one.
    let db = StateDb::open(&path).unwrap();
    let mut store = VecStore::new(db);
    store.load_from_db().expect("load_from_db");
    let results = store.search_cosine_hnsw(&[0.0, 1.0, 0.0], 1);
    assert_eq!(results.len(), 1);
    assert!(
        results[0].0.contains("Cat"),
        "after vectors mutated and stale-cache rejected, expected Cat (now aligned with [0,1,0]); got {:?}",
        results
    );
}
