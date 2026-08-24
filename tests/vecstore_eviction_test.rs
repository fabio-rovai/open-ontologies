#![cfg(feature = "turbovec")]

//! Evicting float32 text vectors from memory.
//!
//! The TurboQuant backend stores 4 bit codes, but until the store stops
//! holding the float32 vectors those codes were made from, the compression is
//! a smaller SQLite blob rather than less RAM. In eviction mode the vectors
//! live only in the `embeddings` table and are loaded on demand: a shortlist
//! of them per query for the exact re-score, all of them for the paths that
//! genuinely need every vector.
//!
//! Every test here asserts against a resident store built from identical
//! data. Eviction is a memory strategy, not a semantic one, so anything an
//! evicted store returns must be what the resident store would have returned.

use open_ontologies::poincare::l2_normalize;
use open_ontologies::state::StateDb;
use open_ontologies::vecstore::VecStore;

fn db_path() -> std::path::PathBuf {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    path
}

fn synth(n: usize, dim: usize) -> Vec<(String, Vec<f32>)> {
    let mut out = Vec::with_capacity(n);
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for i in 0..n {
        let v: Vec<f32> = (0..dim)
            .map(|_| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((state >> 33) as f32 / (1u64 << 31) as f32) - 0.5
            })
            .collect();
        out.push((format!("http://ex.org/C{i}"), l2_normalize(&v)));
    }
    out
}

fn resident(entries: &[(String, Vec<f32>)]) -> VecStore {
    let mut s = VecStore::new(StateDb::open(&db_path()).unwrap());
    for (iri, v) in entries {
        s.upsert(iri, v, &[0.1, 0.2]);
    }
    s
}

fn evicted(entries: &[(String, Vec<f32>)]) -> VecStore {
    let mut s = VecStore::new(StateDb::open(&db_path()).unwrap()).with_text_vectors_evicted();
    for (iri, v) in entries {
        s.upsert(iri, v, &[0.1, 0.2]);
    }
    s
}

#[test]
fn an_evicted_store_holds_no_float32_text_vectors() {
    let entries = synth(200, 64);
    let res = resident(&entries);
    let ev = evicted(&entries);

    assert_eq!(res.resident_text_vector_bytes(), 200 * 64 * 4);
    assert_eq!(
        ev.resident_text_vector_bytes(),
        0,
        "eviction mode must not retain float32 text vectors"
    );
    assert_eq!(ev.len(), 200, "the entry set itself is still known");
}

#[test]
fn turbo_search_is_identical_under_eviction() {
    let entries = synth(400, 64);
    let mut res = resident(&entries);
    let mut ev = evicted(&entries);

    for qi in [0usize, 17, 199, 399] {
        let q = &entries[qi].1;
        assert_eq!(
            ev.search_cosine_turbo(q, 10),
            res.search_cosine_turbo(q, 10),
            "eviction changed the turbo result for query {qi}"
        );
    }
}

#[test]
fn the_exact_scan_is_identical_under_eviction() {
    let entries = synth(300, 64);
    let res = resident(&entries);
    let ev = evicted(&entries);

    let q = &entries[42].1;
    assert_eq!(ev.search_cosine(q, 10), res.search_cosine(q, 10));
}

#[test]
fn the_product_search_is_identical_under_eviction() {
    let entries = synth(200, 64);
    let res = resident(&entries);
    let ev = evicted(&entries);

    let q = &entries[7].1;
    assert_eq!(
        ev.search_product(q, &[0.1, 0.2], 10, 0.5),
        res.search_product(q, &[0.1, 0.2], 10, 0.5)
    );
}

#[test]
fn an_upsert_under_eviction_is_immediately_searchable() {
    let entries = synth(100, 64);
    let mut ev = evicted(&entries);
    let _ = ev.search_cosine_turbo(&entries[0].1, 5);

    let mut newcomer = vec![0.0f32; 64];
    newcomer[0] = 1.0;
    ev.upsert("http://ex.org/Newcomer", &newcomer, &[0.1, 0.2]);

    // Write-through matters: the re-score reads the vector back out of SQLite,
    // so an upsert that only touched memory would score the new entry as
    // missing and drop it from its own top-1.
    assert_eq!(
        ev.search_cosine_turbo(&newcomer, 1)[0].0,
        "http://ex.org/Newcomer"
    );
    assert_eq!(ev.resident_text_vector_bytes(), 0);
}

#[test]
fn a_removal_under_eviction_drops_the_stored_vector() {
    let entries = synth(100, 64);
    let mut ev = evicted(&entries);
    let _ = ev.search_cosine_turbo(&entries[0].1, 5);

    ev.remove("http://ex.org/C5");

    assert_eq!(ev.len(), 99);
    assert!(ev
        .search_cosine_turbo(&entries[5].1, 10)
        .iter()
        .all(|(iri, _)| iri != "http://ex.org/C5"));
    assert!(ev.load_text_vec("http://ex.org/C5").is_none());
}

#[test]
fn the_index_cache_fingerprint_is_the_same_under_eviction() {
    // The fingerprint gates every persisted index. If eviction changed it, an
    // evicted process would reject a cache a resident one wrote, and the two
    // modes could not share a database.
    let entries = synth(150, 64);
    let mut res = resident(&entries);
    let ev = evicted(&entries);

    res.persist().unwrap();
    res.persist_turbo_index().unwrap();

    let reloaded = VecStore::new(StateDb::open(&db_path()).unwrap());
    let _ = reloaded;

    assert_eq!(
        ev.entries_fingerprint(),
        res.entries_fingerprint(),
        "eviction must not change the entry-set fingerprint"
    );
}

#[test]
fn load_text_vec_works_in_both_modes() {
    let entries = synth(50, 64);
    let res = resident(&entries);
    let ev = evicted(&entries);

    assert_eq!(
        ev.load_text_vec("http://ex.org/C9").unwrap(),
        res.load_text_vec("http://ex.org/C9").unwrap()
    );
    assert!(ev.load_text_vec("http://ex.org/Ghost").is_none());
}

/// What eviction costs and what it buys.
/// `cargo test --release --features turbovec -- --ignored --nocapture eviction_cost`
#[test]
#[ignore = "measurement, not an assertion; run explicitly in release"]
fn measure_eviction_cost() {
    use std::time::Instant;

    const N: usize = 20_000;
    const DIM: usize = 384;

    let entries = synth(N, DIM);
    let mut res = resident(&entries);
    let mut ev = evicted(&entries);

    // Warm both indices so the build is not counted in the query timing.
    let _ = res.search_cosine_turbo(&entries[0].1, 10);
    let _ = ev.search_cosine_turbo(&entries[0].1, 10);

    println!("\n{N} vectors x {DIM} dims\n");
    println!(
        "resident float32   {:>8.1} MB   evicted {:>8.1} MB",
        res.resident_text_vector_bytes() as f64 / 1e6,
        ev.resident_text_vector_bytes() as f64 / 1e6
    );

    let queries: Vec<&Vec<f32>> = (0..200).map(|i| &entries[i * 97 % N].1).collect();
    let t = Instant::now();
    for q in &queries {
        let _ = res.search_cosine_turbo(q, 10);
    }
    let res_q = t.elapsed() / queries.len() as u32;
    let t = Instant::now();
    for q in &queries {
        let _ = ev.search_cosine_turbo(q, 10);
    }
    let ev_q = t.elapsed() / queries.len() as u32;
    println!(
        "turbo query        {res_q:>11.2?}   evicted {ev_q:>11.2?}   ({:.1}x)",
        ev_q.as_secs_f64() / res_q.as_secs_f64()
    );

    let t = Instant::now();
    for (i, q) in queries.iter().enumerate().take(20) {
        let _ = res.search_cosine(q, 10);
        let _ = i;
    }
    let res_exact = t.elapsed() / 20;
    let t = Instant::now();
    for q in queries.iter().take(20) {
        let _ = ev.search_cosine(q, 10);
    }
    let ev_exact = t.elapsed() / 20;
    println!(
        "exact scan         {res_exact:>11.2?}   evicted {ev_exact:>11.2?}   ({:.1}x)",
        ev_exact.as_secs_f64() / res_exact.as_secs_f64()
    );
    println!();
}
