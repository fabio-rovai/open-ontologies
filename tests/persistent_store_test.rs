use open_ontologies::config::{resolve_storage_mode_from, StorageConfig, StorageMode};
use open_ontologies::graph::GraphStore;
use tempfile::TempDir;

#[test]
fn persistent_store_survives_drop_and_reopen() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("triplestore");

    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
        ex:Bob   a ex:Person .
    "#;

    {
        let store = GraphStore::open_persistent(&path).unwrap();
        assert_eq!(store.triple_count(), 0);
        store.load_turtle(ttl, None).unwrap();
        assert_eq!(store.triple_count(), 2);
    }

    {
        let store = GraphStore::open_persistent(&path).unwrap();
        assert_eq!(store.triple_count(), 2);
        let json = store
            .sparql_select("SELECT ?s WHERE { ?s a <http://example.org/Person> }")
            .unwrap();
        assert!(json.contains("Alice"));
        assert!(json.contains("Bob"));
    }
}

// These exercise `resolve_storage_mode_from` rather than `resolve_storage_mode`
// so that no test has to touch the process environment. `remove_var` is unsafe
// under edition 2024 for a real reason: cargo runs these test functions on
// parallel threads, so mutating the environment while another thread reads it
// is a data race regardless of whether the assertion happens to hold.
// Passing the override in explicitly tests the same logic with the hazard gone.

#[test]
fn storage_mode_resolves_default_to_memory() {
    let cfg = StorageConfig::default();
    assert_eq!(
        resolve_storage_mode_from(None, &cfg),
        StorageMode::Memory
    );
}

#[test]
fn storage_mode_parses_persistent() {
    let cfg = StorageConfig {
        mode: "persistent".to_string(),
    };
    assert_eq!(
        resolve_storage_mode_from(None, &cfg),
        StorageMode::Persistent
    );
}

#[test]
fn storage_mode_override_beats_config() {
    let cfg = StorageConfig {
        mode: "persistent".to_string(),
    };
    assert_eq!(
        resolve_storage_mode_from(Some("memory"), &cfg),
        StorageMode::Memory
    );
}

#[test]
fn storage_mode_blank_override_falls_through_to_config() {
    let cfg = StorageConfig {
        mode: "persistent".to_string(),
    };
    assert_eq!(
        resolve_storage_mode_from(Some("   "), &cfg),
        StorageMode::Persistent
    );
}

#[test]
fn storage_mode_accepts_aliases_case_insensitively() {
    let cfg = StorageConfig::default();
    for alias in ["persistent", "disk", "rocksdb", "RocksDB", "  DISK  "] {
        assert_eq!(
            resolve_storage_mode_from(Some(alias), &cfg),
            StorageMode::Persistent,
            "alias {alias:?} should select the persistent backend"
        );
    }
    for alias in ["memory", "mem", "in-memory", "IN-MEMORY"] {
        assert_eq!(
            resolve_storage_mode_from(Some(alias), &cfg),
            StorageMode::Memory,
            "alias {alias:?} should select the in-memory backend"
        );
    }
}

#[test]
fn storage_mode_unknown_value_falls_back_to_memory() {
    let cfg = StorageConfig {
        mode: "persistent".to_string(),
    };
    // A typo must not silently promote an in-memory deployment to a
    // persistent one, nor hard-fail: it warns and degrades to memory.
    assert_eq!(
        resolve_storage_mode_from(Some("persistant"), &cfg),
        StorageMode::Memory
    );
}
