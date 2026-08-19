#![cfg(feature = "turbovec")]

//! Tests for the TurboQuant-backed cosine index.
//!
//! The reference for every ranking assertion is the brute-force exact cosine
//! scan already in `VecStore::search_cosine`: the quantised index is a
//! candidate generator, and it is only useful if its shortlist contains what
//! the exact scan would have returned.

use open_ontologies::poincare::l2_normalize;
use open_ontologies::turbo_index::TurboCosineIndex;

/// Deterministic pseudo-random unit vectors, so a failure is reproducible.
fn synth_vectors(n: usize, dim: usize) -> Vec<(String, Vec<f32>)> {
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

fn exact_top1(entries: &[(String, Vec<f32>)], query: &[f32]) -> String {
    entries
        .iter()
        .map(|(iri, v)| {
            let dot: f32 = query.iter().zip(v.iter()).map(|(a, b)| a * b).sum();
            (iri.clone(), dot)
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap()
        .0
}

#[test]
fn top1_agrees_with_brute_force_cosine() {
    let entries = synth_vectors(500, 64);
    let index = TurboCosineIndex::build(entries.clone()).unwrap();

    // Query near a known entry so there is an unambiguous winner.
    let query = entries[42].1.clone();

    let results = index.search(&query, 5);
    assert_eq!(results.len(), 5, "expected 5 candidates, got {}", results.len());
    assert_eq!(
        results[0].0,
        exact_top1(&entries, &query),
        "quantised top-1 disagrees with the exact cosine scan"
    );
}

#[test]
fn upsert_is_searchable_without_a_rebuild() {
    let entries = synth_vectors(200, 64);
    let mut index = TurboCosineIndex::build(entries.clone()).unwrap();

    // A direction no existing entry occupies: the first basis vector.
    let mut newcomer = vec![0.0f32; 64];
    newcomer[0] = 1.0;
    index.upsert("http://ex.org/Newcomer", &newcomer).unwrap();

    let results = index.search(&newcomer, 1);
    assert_eq!(
        results[0].0, "http://ex.org/Newcomer",
        "an upserted vector must be searchable without rebuilding the index"
    );
    assert_eq!(index.len(), 201);
}

#[test]
fn upsert_of_an_existing_iri_replaces_its_vector() {
    let entries = synth_vectors(50, 64);
    let mut index = TurboCosineIndex::build(entries.clone()).unwrap();

    let mut moved = vec![0.0f32; 64];
    moved[0] = 1.0;
    index.upsert("http://ex.org/C7", &moved).unwrap();

    assert_eq!(index.len(), 50, "replacing an IRI must not grow the index");
    let results = index.search(&moved, 1);
    assert_eq!(results[0].0, "http://ex.org/C7");
}

#[test]
fn remove_drops_the_entry() {
    let entries = synth_vectors(50, 64);
    let mut index = TurboCosineIndex::build(entries.clone()).unwrap();

    let target = entries[7].1.clone();
    assert_eq!(index.search(&target, 1)[0].0, "http://ex.org/C7");

    assert!(index.remove("http://ex.org/C7"));
    assert_eq!(index.len(), 49);
    assert!(
        index.search(&target, 10).iter().all(|(iri, _)| iri != "http://ex.org/C7"),
        "a removed IRI must not come back in results"
    );
    assert!(!index.remove("http://ex.org/C7"), "removing twice reports false");
}

#[test]
fn round_trips_through_bytes() {
    let entries = synth_vectors(300, 64);
    let mut index = TurboCosineIndex::build(entries.clone()).unwrap();
    index.remove("http://ex.org/C3");

    let bytes = index.to_bytes().unwrap();
    let reloaded = TurboCosineIndex::from_bytes(&bytes).unwrap();

    assert_eq!(reloaded.len(), index.len());
    let query = entries[11].1.clone();
    assert_eq!(
        reloaded.search(&query, 10),
        index.search(&query, 10),
        "a reloaded index must rank identically to the one it was serialised from"
    );
}

#[test]
fn upsert_after_a_reload_does_not_reuse_a_stale_id() {
    let entries = synth_vectors(20, 64);
    let index = TurboCosineIndex::build(entries.clone()).unwrap();
    let mut reloaded = TurboCosineIndex::from_bytes(&index.to_bytes().unwrap()).unwrap();

    let mut newcomer = vec![0.0f32; 64];
    newcomer[0] = 1.0;
    reloaded.upsert("http://ex.org/Newcomer", &newcomer).unwrap();

    assert_eq!(reloaded.len(), 21);
    assert_eq!(reloaded.search(&newcomer, 1)[0].0, "http://ex.org/Newcomer");
}

#[test]
fn search_within_restricts_results_to_the_allowlist() {
    let entries = synth_vectors(200, 64);
    let index = TurboCosineIndex::build(entries.clone()).unwrap();

    let query = entries[0].1.clone();
    let allowed = vec![
        "http://ex.org/C150".to_string(),
        "http://ex.org/C151".to_string(),
        "http://ex.org/C152".to_string(),
    ];

    let results = index.search_within(&query, 10, &allowed).unwrap();
    assert_eq!(results.len(), 3, "effective k is capped by the allowlist size");
    assert!(
        results.iter().all(|(iri, _)| allowed.contains(iri)),
        "results escaped the allowlist: {results:?}"
    );
}

#[test]
fn search_within_ignores_unknown_iris_rather_than_failing() {
    let entries = synth_vectors(20, 64);
    let index = TurboCosineIndex::build(entries.clone()).unwrap();

    let allowed = vec!["http://ex.org/C1".to_string(), "http://ex.org/Ghost".to_string()];
    let results = index.search_within(&entries[1].1, 10, &allowed).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "http://ex.org/C1");
}

#[test]
fn search_within_an_empty_allowlist_returns_nothing() {
    let entries = synth_vectors(20, 64);
    let index = TurboCosineIndex::build(entries.clone()).unwrap();
    assert!(index.search_within(&entries[0].1, 10, &[]).unwrap().is_empty());
}

#[test]
fn a_dim_that_is_not_a_multiple_of_eight_is_padded() {
    // turbovec requires dim % 8 == 0; the vector store accepts any width.
    let entries = vec![
        ("http://ex.org/Dog".to_string(), l2_normalize(&[0.9, 0.1, 0.0])),
        ("http://ex.org/Cat".to_string(), l2_normalize(&[0.8, 0.2, 0.0])),
        ("http://ex.org/Car".to_string(), l2_normalize(&[0.0, 0.0, 1.0])),
    ];
    let index = TurboCosineIndex::build(entries).unwrap();

    let results = index.search(&l2_normalize(&[0.0, 0.0, 1.0]), 1);
    assert_eq!(results[0].0, "http://ex.org/Car");
}

#[test]
fn a_non_finite_query_returns_nothing_rather_than_panicking() {
    // turbovec's allowlist-free `search` is the panicking form, and it panics
    // on a non-finite query coordinate. A broken embedding provider must not
    // be able to take down the MCP server through this path.
    let entries = synth_vectors(20, 64);
    let index = TurboCosineIndex::build(entries).unwrap();

    let mut nan_query = vec![0.1f32; 64];
    nan_query[3] = f32::NAN;
    assert!(index.search(&nan_query, 5).is_empty());

    let mut inf_query = vec![0.1f32; 64];
    inf_query[3] = f32::INFINITY;
    assert!(index.search(&inf_query, 5).is_empty());
    assert!(index
        .search_within(&inf_query, 5, &["http://ex.org/C1".to_string()])
        .unwrap()
        .is_empty());
}

#[test]
fn a_query_of_the_wrong_dimensionality_returns_nothing_rather_than_being_truncated() {
    let entries = synth_vectors(20, 64);
    let index = TurboCosineIndex::build(entries).unwrap();

    // Silently truncating or padding a mismatched query would return a
    // plausible-looking ranking computed against the wrong vector.
    assert!(index.search(&[0.1f32; 128], 5).is_empty());
    assert!(index.search(&[0.1f32; 32], 5).is_empty());
    assert!(index
        .search_within(&[0.1f32; 128], 5, &["http://ex.org/C1".to_string()])
        .unwrap()
        .is_empty());
}

// ---------------------------------------------------------------------------
// VecStore integration
// ---------------------------------------------------------------------------

use open_ontologies::state::StateDb;
use open_ontologies::vecstore::VecStore;

fn test_db() -> StateDb {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    StateDb::open(&path).unwrap()
}

fn populated_store(n: usize, dim: usize) -> (VecStore, Vec<(String, Vec<f32>)>) {
    let entries = synth_vectors(n, dim);
    let mut store = VecStore::new(test_db());
    for (iri, v) in &entries {
        store.upsert(iri, v, &[0.1, 0.1]);
    }
    (store, entries)
}

#[test]
fn turbo_search_returns_the_same_ranking_and_scores_as_the_exact_scan() {
    let (mut store, entries) = populated_store(400, 64);
    let query = entries[99].1.clone();

    let exact = store.search_cosine(&query, 10);
    let turbo = store.search_cosine_turbo(&query, 10);

    assert_eq!(
        turbo, exact,
        "the turbo path re-scores its candidates exactly, so it must not \
         differ from the brute-force scan in either order or score"
    );
}

#[test]
fn upsert_keeps_the_turbo_index_warm_instead_of_dropping_it() {
    let (mut store, _entries) = populated_store(100, 64);
    // Warm the index.
    let _ = store.search_cosine_turbo(&[0.1; 64], 5);
    assert_eq!(store.turbo_index_len(), Some(100));

    let mut newcomer = vec![0.0f32; 64];
    newcomer[0] = 1.0;
    store.upsert("http://ex.org/Newcomer", &newcomer, &[0.1, 0.1]);

    assert_eq!(
        store.turbo_index_len(),
        Some(101),
        "upsert must extend the live turbo index, not invalidate it"
    );
    assert_eq!(
        store.search_cosine_turbo(&newcomer, 1)[0].0,
        "http://ex.org/Newcomer"
    );
}

#[test]
fn remove_updates_the_turbo_index_in_place() {
    let (mut store, entries) = populated_store(100, 64);
    let _ = store.search_cosine_turbo(&entries[0].1, 5);

    store.remove("http://ex.org/C5");

    assert_eq!(store.turbo_index_len(), Some(99));
    assert!(store
        .search_cosine_turbo(&entries[5].1, 10)
        .iter()
        .all(|(iri, _)| iri != "http://ex.org/C5"));
}

#[test]
fn the_turbo_index_survives_a_process_restart_through_sqlite() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);

    let entries = synth_vectors(200, 64);
    {
        let mut store = VecStore::new(StateDb::open(&path).unwrap());
        for (iri, v) in &entries {
            store.upsert(iri, v, &[0.1, 0.1]);
        }
        let _ = store.search_cosine_turbo(&entries[0].1, 5);
        store.persist().unwrap();
        store.persist_turbo_index().unwrap();
    }

    let mut reloaded = VecStore::new(StateDb::open(&path).unwrap());
    reloaded.load_from_db().unwrap();
    assert!(
        reloaded.load_turbo_index().unwrap(),
        "a persisted turbo index should reload rather than rebuild"
    );
    assert_eq!(reloaded.turbo_index_len(), Some(200));
}

#[test]
fn a_stale_turbo_index_cache_is_rejected() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);

    let entries = synth_vectors(50, 64);
    {
        let mut store = VecStore::new(StateDb::open(&path).unwrap());
        for (iri, v) in &entries {
            store.upsert(iri, v, &[0.1, 0.1]);
        }
        let _ = store.search_cosine_turbo(&entries[0].1, 5);
        store.persist_turbo_index().unwrap();
        // Entries move on without the cache being refreshed.
        store.upsert("http://ex.org/Extra", &[0.5; 64], &[0.1, 0.1]);
        store.persist().unwrap();
    }

    let mut reloaded = VecStore::new(StateDb::open(&path).unwrap());
    reloaded.load_from_db().unwrap();
    assert!(
        !reloaded.load_turbo_index().unwrap(),
        "an index cache whose entry set has moved on must be refused"
    );
}

// ---------------------------------------------------------------------------
// Measurement. `cargo test --release --features turbovec -- --ignored --nocapture`
// ---------------------------------------------------------------------------

#[test]
#[ignore = "measurement, not an assertion; run explicitly in release"]
fn measure_turbo_against_hnsw() {
    use open_ontologies::hnsw_index::CosineIndex;
    use std::time::Instant;

    const N: usize = 10_000;
    const DIM: usize = 768;
    const TOP_K: usize = 10;

    let entries = synth_vectors(N, DIM);
    println!("\n{N} vectors x {DIM} dims\n");

    let t = Instant::now();
    let mut hnsw = CosineIndex::build(entries.clone());
    let hnsw_build = t.elapsed();

    let t = Instant::now();
    let mut turbo = TurboCosineIndex::build(entries.clone()).unwrap();
    let turbo_build = t.elapsed();
    println!("build         hnsw {hnsw_build:>10.2?}   turbo {turbo_build:>10.2?}");

    // The cost of one added embedding: instant-distance is immutable, so the
    // whole graph is rebuilt; turbovec appends.
    let mut newcomer = vec![0.0f32; DIM];
    newcomer[0] = 1.0;
    let t = Instant::now();
    let _rebuilt = CosineIndex::build(entries.clone());
    let hnsw_upsert = t.elapsed();
    let t = Instant::now();
    turbo.upsert("http://ex.org/Newcomer", &newcomer).unwrap();
    let turbo_upsert = t.elapsed();
    println!(
        "one upsert    hnsw {hnsw_upsert:>10.2?}   turbo {turbo_upsert:>10.2?}   ({:.0}x)",
        hnsw_upsert.as_secs_f64() / turbo_upsert.as_secs_f64()
    );

    let queries: Vec<Vec<f32>> = (0..200).map(|i| entries[i * 97 % N].1.clone()).collect();

    let t = Instant::now();
    for q in &queries {
        let _ = hnsw.search(q, TOP_K);
    }
    let hnsw_q = t.elapsed() / queries.len() as u32;
    let t = Instant::now();
    for q in &queries {
        let _ = turbo.search(q, TOP_K * 4);
    }
    let turbo_q = t.elapsed() / queries.len() as u32;
    println!("query         hnsw {hnsw_q:>10.2?}   turbo {turbo_q:>10.2?}  (turbo pulls 4x candidates)");

    // Recall of the exact top-k inside each index's returned set.
    let mut hnsw_hits = 0usize;
    let mut hnsw_wide_hits = 0usize;
    let mut turbo_hits = 0usize;
    let mut total = 0usize;
    for q in &queries {
        let mut exact: Vec<(String, f32)> = entries
            .iter()
            .map(|(iri, v)| (iri.clone(), q.iter().zip(v).map(|(a, b)| a * b).sum::<f32>()))
            .collect();
        exact.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let truth: Vec<&String> = exact.iter().take(TOP_K).map(|(i, _)| i).collect();

        let h: Vec<String> = hnsw.search(q, TOP_K).into_iter().map(|(i, _)| i).collect();
        let h_wide: Vec<String> = hnsw
            .search(q, TOP_K * 4)
            .into_iter()
            .map(|(i, _)| i)
            .collect();
        let tv: Vec<String> = turbo
            .search(q, TOP_K * 4)
            .into_iter()
            .map(|(i, _)| i)
            .collect();
        for iri in &truth {
            total += 1;
            if h.contains(iri) {
                hnsw_hits += 1;
            }
            if h_wide.contains(iri) {
                hnsw_wide_hits += 1;
            }
            if tv.contains(iri) {
                turbo_hits += 1;
            }
        }
    }
    // Three numbers, because the interesting question is whether an accuracy
    // difference comes from TurboQuant or merely from over-querying and
    // re-scoring, a pattern the HNSW backend could adopt too.
    println!(
        "recall@{TOP_K}     hnsw {:>9.1}%  (as wired: {TOP_K} returned, {TOP_K} kept)",
        100.0 * hnsw_hits as f64 / total as f64
    );
    println!(
        "              hnsw {:>9.1}%  (control: {} returned, re-scored to {TOP_K})",
        100.0 * hnsw_wide_hits as f64 / total as f64,
        TOP_K * 4
    );
    println!(
        "             turbo {:>9.1}%  (as wired: {} returned, re-scored to {TOP_K})",
        100.0 * turbo_hits as f64 / total as f64,
        TOP_K * 4
    );

    let hnsw_bytes = hnsw.to_bytes().unwrap().len();
    let turbo_bytes = turbo.to_bytes().unwrap().len();
    let f32_bytes = N * DIM * 4;
    println!(
        "serialised    hnsw {:>7.1} MB   turbo {:>7.1} MB   raw float32 {:>7.1} MB",
        hnsw_bytes as f64 / 1e6,
        turbo_bytes as f64 / 1e6,
        f32_bytes as f64 / 1e6
    );
    println!();
}

/// Recall of both backends over a *real* embedded corpus.
///
/// `measure_turbo_against_hnsw` runs on isotropic random vectors, which is
/// close to the worst case for a graph index: every pairwise cosine sits near
/// zero, so ranks 10 and 11 are separated by noise and there is no cluster
/// structure for the walk to exploit. This runs the same comparison over real
/// ontology labels embedded with the shipped local ONNX model, which is the
/// only way to know whether that 8 point gap is an artefact of the corpus.
///
/// ```text
/// OO_BENCH_LABELS=/tmp/labels_lso.txt \
///   cargo test --release --features turbovec -- --ignored --nocapture real_corpus
/// ```
///
/// Embeddings are cached next to the labels file, so re-runs skip the model.
#[test]
#[ignore = "measurement; needs OO_BENCH_LABELS and the local ONNX model"]
fn measure_recall_on_a_real_corpus() {
    use open_ontologies::embed::{TextEmbedder, DEFAULT_MODEL_FILENAME};
    use open_ontologies::hnsw_index::CosineIndex;
    use std::io::Write;
    use std::path::PathBuf;

    const TOP_K: usize = 10;
    const WIDE: usize = TOP_K * 4;

    let labels_path = match std::env::var("OO_BENCH_LABELS") {
        Ok(p) => PathBuf::from(p),
        Err(_) => {
            println!("set OO_BENCH_LABELS to a newline-delimited label file");
            return;
        }
    };
    let limit: usize = std::env::var("OO_BENCH_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);

    let labels: Vec<String> = std::fs::read_to_string(&labels_path)
        .expect("labels file")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(limit)
        .map(|l| l.to_string())
        .collect();
    let n = labels.len();

    // Embed once, cache to disk. The cache is keyed by the labels file and the
    // count, so a different slice does not silently reuse the wrong vectors.
    let cache_path = labels_path.with_extension(format!("{n}.f32"));
    let flat: Vec<f32> = if cache_path.exists() {
        let raw = std::fs::read(&cache_path).unwrap();
        raw.chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    } else {
        let models = PathBuf::from(std::env::var("HOME").unwrap()).join(".open-ontologies/models");
        let embedder = TextEmbedder::load(
            &models.join(DEFAULT_MODEL_FILENAME),
            &models.join("tokenizer.json"),
        )
        .expect("local ONNX model + tokenizer");
        let started = std::time::Instant::now();
        let mut flat = Vec::with_capacity(n * embedder.dim());
        for (i, chunk) in labels.chunks(64).enumerate() {
            let refs: Vec<&str> = chunk.iter().map(|s| s.as_str()).collect();
            for v in embedder.embed_batch(&refs).unwrap() {
                flat.extend_from_slice(&v);
            }
            if i % 20 == 0 {
                println!("  embedded {}/{n} ({:.0?})", (i + 1) * 64, started.elapsed());
            }
        }
        let mut fh = std::fs::File::create(&cache_path).unwrap();
        for v in &flat {
            fh.write_all(&v.to_le_bytes()).unwrap();
        }
        flat
    };

    let dim = flat.len() / n;
    let entries: Vec<(String, Vec<f32>)> = (0..n)
        .map(|i| {
            (
                format!("http://ex.org/L{i}"),
                l2_normalize(&flat[i * dim..(i + 1) * dim]),
            )
        })
        .collect();
    println!("\n{n} real labels x {dim} dims  ({})\n", labels_path.display());

    let mut hnsw = CosineIndex::build(entries.clone());
    let turbo = TurboCosineIndex::build(entries.clone()).unwrap();

    // Anisotropy of the corpus, as the one number that says how far it is from
    // the synthetic case: the mean cosine between random distinct pairs. Near
    // zero is isotropic; real embeddings sit well above it.
    let mut acc = 0.0f64;
    let pairs = 20_000;
    for p in 0..pairs {
        let a = &entries[(p * 7919) % n].1;
        let b = &entries[(p * 104729 + 13) % n].1;
        acc += a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>() as f64;
    }
    println!("mean pairwise cosine  {:.4}  (isotropic random ~= 0)", acc / pairs as f64);

    let queries: Vec<usize> = (0..300).map(|i| (i * 7919) % n).collect();
    let (mut h_hits, mut h_wide_hits, mut t_hits, mut total) = (0usize, 0usize, 0usize, 0usize);
    let mut self_match_failures = 0usize;

    for &qi in &queries {
        let q = &entries[qi].1;
        let mut exact: Vec<(usize, f32)> = entries
            .iter()
            .enumerate()
            .map(|(i, (_, v))| (i, q.iter().zip(v).map(|(a, b)| a * b).sum::<f32>()))
            .collect();
        exact.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let truth: Vec<String> = exact
            .iter()
            .take(TOP_K)
            .map(|(i, _)| entries[*i].0.clone())
            .collect();

        let h: Vec<String> = hnsw.search(q, TOP_K).into_iter().map(|(i, _)| i).collect();
        let h_wide: Vec<String> = hnsw.search(q, WIDE).into_iter().map(|(i, _)| i).collect();
        let t: Vec<String> = turbo.search(q, WIDE).into_iter().map(|(i, _)| i).collect();

        // Sanity: a query that IS an indexed vector must retrieve itself.
        if t.first().map(|s| s.as_str()) != Some(entries[qi].0.as_str()) {
            self_match_failures += 1;
        }

        for iri in &truth {
            total += 1;
            if h.contains(iri) {
                h_hits += 1;
            }
            if h_wide.contains(iri) {
                h_wide_hits += 1;
            }
            if t.contains(iri) {
                t_hits += 1;
            }
        }
    }

    println!(
        "recall@{TOP_K}     hnsw {:>9.1}%  (as wired: {TOP_K} returned, {TOP_K} kept)",
        100.0 * h_hits as f64 / total as f64
    );
    println!(
        "              hnsw {:>9.1}%  (control: {WIDE} returned, re-scored to {TOP_K})",
        100.0 * h_wide_hits as f64 / total as f64
    );
    println!(
        "             turbo {:>9.1}%  (as wired: {WIDE} returned, re-scored to {TOP_K})",
        100.0 * t_hits as f64 / total as f64
    );
    println!("turbo self-match failures: {self_match_failures}/{}\n", queries.len());
}
