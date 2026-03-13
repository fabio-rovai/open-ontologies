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
