#![cfg(feature = "embeddings")]

//! The embedding-configuration guard (issue #74).
//!
//! Every test here is written against a database that survives a "process
//! restart", because the failure being guarded against only appears across one:
//! the config changes, the process comes back up, and the vectors on disk are
//! from the previous model.
//!
//! The fingerprints are opaque strings — these tests do not care how one is
//! computed, only that two different configurations produce two different ones.
//! `embed_fingerprint`'s own unit tests cover the computation.

use open_ontologies::state::StateDb;
use open_ontologies::vecstore::VecStore;

const MODEL_A: &str = "fingerprint-of-model-a";
const MODEL_B: &str = "fingerprint-of-model-b";

fn temp_db_path() -> std::path::PathBuf {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    path
}

/// Vectors written under one configuration must not be loaded under another.
///
/// This is the mixing path: `load_from_db` pulls old-model vectors in, `upsert`
/// adds new-model ones, and `persist` writes the union back out as a table that
/// looks internally consistent. Refusing them at load is what breaks the chain.
#[test]
fn vectors_from_another_model_are_not_loaded() {
    let path = temp_db_path();

    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_A.to_string());
        store.upsert("http://ex.org/Dog", &[0.9, 0.1, 0.0], &[0.1, 0.0]);
        store.upsert("http://ex.org/Cat", &[0.8, 0.2, 0.0], &[0.15, 0.0]);
        store.persist().unwrap();
    }

    // Same database, different embedding configuration.
    let db = StateDb::open(&path).unwrap();
    let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_B.to_string());
    store.load_from_db().unwrap();

    assert_eq!(
        store.len(),
        0,
        "vectors produced by another model were loaded and are about to be \
         compared against queries from this one"
    );
}

/// The same configuration must keep working — the guard has to be inert when
/// nothing changed, or it just means a rebuild on every restart.
#[test]
fn vectors_from_the_same_model_are_loaded() {
    let path = temp_db_path();

    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_A.to_string());
        store.upsert("http://ex.org/Dog", &[0.9, 0.1, 0.0], &[0.1, 0.0]);
        store.persist().unwrap();
    }

    let db = StateDb::open(&path).unwrap();
    let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_A.to_string());
    store.load_from_db().unwrap();

    assert_eq!(store.len(), 1);
    let hits = store.search_cosine(&[0.9, 0.1, 0.0], 1);
    assert!(hits[0].0.contains("Dog"), "got {hits:?}");
}

/// The case `entries_hash` structurally cannot see.
///
/// **Zero new entities.** Every stored vector is old-model, every query vector
/// is new-model. The entry set is byte-identical, so `entries_hash` is
/// unchanged and the cached index is internally coherent — and the comparison
/// is meaningless anyway. No check over the stored set alone can detect this,
/// which is exactly why the fingerprint had to cover the configuration instead.
#[test]
fn an_unchanged_entry_set_is_still_invalidated_by_a_config_change() {
    let path = temp_db_path();

    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_A.to_string());
        for i in 0..12 {
            store.upsert(
                &format!("http://ex.org/E{i}"),
                &[i as f32, 1.0, 0.0],
                &[0.1, 0.0],
            );
        }
        store.persist().unwrap();
        store.persist_cosine_index().unwrap();
    }

    // Nothing is added, removed or re-embedded. Only the configuration moved.
    let db = StateDb::open(&path).unwrap();
    let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_B.to_string());
    store.load_from_db().unwrap();

    assert_eq!(
        store.len(),
        0,
        "the entry set was unchanged, so entries_hash matched — the model \
         fingerprint is the only thing that could have caught this"
    );
    assert!(
        !store.load_cosine_index().unwrap(),
        "the cached HNSW index was accepted despite being built by another model"
    );
}

/// A cached index survives a restart under the same configuration. Guards
/// against "fix it by invalidating always", which would pass the test above
/// while making every restart pay a full rebuild.
#[test]
fn a_cached_index_is_still_reused_when_the_config_is_unchanged() {
    let path = temp_db_path();

    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_A.to_string());
        for i in 0..12 {
            store.upsert(
                &format!("http://ex.org/E{i}"),
                &[i as f32, 1.0, 0.0],
                &[0.1, 0.0],
            );
        }
        store.persist().unwrap();
        store.persist_cosine_index().unwrap();
    }

    let db = StateDb::open(&path).unwrap();
    let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_A.to_string());
    store.load_from_db().unwrap();
    assert_eq!(store.len(), 12);
    assert!(
        store.load_cosine_index().unwrap(),
        "a valid cached index was rejected — the guard is too eager and every \
         restart now pays a rebuild"
    );
}

/// Rows written before the column existed carry no answer.
///
/// Treating unknown as matching would let the corruption survive precisely the
/// upgrade that adds the check. One loud rebuild is the correct price.
#[test]
fn rows_with_no_recorded_fingerprint_are_rejected_once() {
    let path = temp_db_path();

    {
        // A store with no configuration writes NULL — the pre-upgrade shape.
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db);
        store.upsert("http://ex.org/Legacy", &[1.0, 0.0, 0.0], &[0.1, 0.0]);
        store.persist().unwrap();
    }

    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_A.to_string());
        store.load_from_db().unwrap();
        assert_eq!(store.len(), 0, "unrecorded vectors were assumed compatible");

        // Re-embedding under a known configuration ends it.
        store.upsert("http://ex.org/Legacy", &[1.0, 0.0, 0.0], &[0.1, 0.0]);
        store.persist().unwrap();
    }

    let db = StateDb::open(&path).unwrap();
    let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_A.to_string());
    store.load_from_db().unwrap();
    assert_eq!(
        store.len(),
        1,
        "the rejection recurred after a clean re-embed"
    );
}

/// A store that was never told its configuration behaves exactly as it did
/// before this column existed. Every pre-existing test constructs one that way,
/// and none of them should have to change.
#[test]
fn a_store_without_a_fingerprint_is_unaffected() {
    let path = temp_db_path();

    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_A.to_string());
        store.upsert("http://ex.org/Dog", &[0.9, 0.1, 0.0], &[0.1, 0.0]);
        store.persist().unwrap();
    }

    // No fingerprint set: nothing to compare against, so nothing is refused.
    let db = StateDb::open(&path).unwrap();
    let mut store = VecStore::new(db);
    store.load_from_db().unwrap();
    assert_eq!(store.len(), 1);
    assert!(store.embeddings_fingerprint().is_none());
}

/// The Poincaré index cache carries the fingerprint too. `struct_vec`s come
/// from the structural embedder rather than the text model, but they are
/// persisted by the same store and invalidated by the same swap.
#[test]
fn the_poincare_index_cache_is_also_guarded() {
    let path = temp_db_path();

    {
        let db = StateDb::open(&path).unwrap();
        let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_A.to_string());
        for i in 0..12 {
            store.upsert(
                &format!("http://ex.org/E{i}"),
                &[1.0, 0.0, 0.0],
                &[i as f32 * 0.01, 0.02],
            );
        }
        store.persist().unwrap();
        store.persist_poincare_index().unwrap();
    }

    let db = StateDb::open(&path).unwrap();
    let mut store = VecStore::new(db).with_embeddings_fingerprint(MODEL_B.to_string());
    store.load_from_db().unwrap();
    assert!(
        !store.load_poincare_index().unwrap(),
        "the cached Poincaré index was accepted despite a configuration change"
    );
}
