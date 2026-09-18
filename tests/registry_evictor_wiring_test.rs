//! The evictor the HTTP server spawns is wired to a registry nothing loads into, so it
//! can never evict anything. This file pins that.
//!
//! `src/main.rs:1913-1921` builds a registry for the sole purpose of driving eviction,
//! over the graph every session shares, and hands it to `spawn_evictor`. The comment
//! above it says "the shared one drives memory cleanup". It drives nothing.
//! `OntologyRegistry::new` starts with `active: None` (`src/registry.rs:82`), only
//! `load_file` ever fills that slot (`src/registry.rs:158`), nothing calls `load_file` on
//! this registry, and `evictor_tick` returns `Ok(false)` on its third line when the slot
//! is empty (`src/registry.rs:284`). So `[cache] idle_ttl_secs` is silently inert in
//! `serve-http` and `daemon` mode however it is configured.
//!
//! This is the shape `docs/ci-gates.md` exists to attack, one level down: not a test that
//! skips and reports `ok`, but a background task that runs, does nothing, and reports
//! nothing. A green process is not a working evictor.
//!
//! These tests are characterisation tests. They assert what the code does TODAY, which is
//! wrong, so that the wrongness is visible in the suite rather than in a reader's
//! inference from two files. They are written so that either direction of change breaks
//! them: closing the defect properly makes `an_unloaded_registry_never_evicts` fail, and
//! "fixing" it by pointing the evictor at the shared graph regardless of the active slot
//! makes `the_dead_evictor_does_not_touch_the_shared_store` fail, which is the fix that
//! must not be made (eviction state is per registry, so clearing a store no session's
//! registry has flagged leaves every session believing its ontology is resident, with no
//! path back through `ensure_loaded`).
//!
//! Full analysis in `docs/concurrency-inventory.md`, defect A.
//!
//! Nothing here can skip: no external toolchain, no fixture, no network. It needs only a
//! temporary directory.

use std::path::PathBuf;
use std::sync::Arc;

use open_ontologies::config::CacheConfig;
use open_ontologies::graph::GraphStore;
use open_ontologies::registry::OntologyRegistry;
use open_ontologies::state::StateDb;

const TTL_SECS: u64 = 1;

/// A scratch directory unique to one test, removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "oo-evictor-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Exactly what `src/main.rs:1913-1921` builds: a registry over a shared graph, with
/// eviction enabled and a TTL that has already elapsed by the time the tick runs, and
/// with nothing ever loaded through it.
fn evictor_registry(scratch: &Scratch, graph: Arc<GraphStore>) -> Arc<OntologyRegistry> {
    let db = StateDb::open(&scratch.0.join("state.db")).expect("open state db");
    let cfg = CacheConfig {
        enabled: true,
        dir: scratch.0.to_string_lossy().into_owned(),
        idle_ttl_secs: TTL_SECS,
        evictor_interval_secs: 1,
        auto_refresh: false,
        ..CacheConfig::default()
    };
    Arc::new(OntologyRegistry::new(graph, db, cfg).expect("build registry"))
}

fn some_triples(graph: &GraphStore) -> usize {
    graph
        .load_turtle(
            "@prefix : <https://example.org/> .\n\
             :A a <http://www.w3.org/2002/07/owl#Class> .\n\
             :B a <http://www.w3.org/2002/07/owl#Class> .\n\
             :A <http://www.w3.org/2000/01/rdf-schema#subClassOf> :B .\n",
            None,
        )
        .expect("load turtle")
}

/// The defect itself. The store is full, the TTL has elapsed, eviction is enabled, and
/// the tick still reports that it evicted nothing, because the active slot of THIS
/// registry was never filled.
///
/// When defect A is closed this assertion must be inverted, and that is the point of
/// writing it down: the fix cannot land silently.
#[test]
fn an_unloaded_registry_never_evicts() {
    let scratch = Scratch::new("never-evicts");
    let graph = Arc::new(GraphStore::new());
    let loaded = some_triples(&graph);
    assert!(loaded > 0, "the fixture must put something in the store");

    let registry = evictor_registry(&scratch, graph.clone());

    // Well past `idle_ttl_secs`, so nothing below can be explained by the entry still
    // being fresh. There is no entry at all.
    std::thread::sleep(std::time::Duration::from_millis(1_100 * TTL_SECS));

    let evicted = registry.evictor_tick().expect("tick must not error");
    assert!(
        !evicted,
        "DEFECT A IS FIXED: the evictor now reports an eviction. Invert this test and \
         update docs/concurrency-inventory.md, which records it as live."
    );
}

/// The other half, and the one that guards against the wrong fix. Because the tick does
/// nothing, the shared store is untouched, so an operator who set `idle_ttl_secs` gets
/// unbounded growth rather than the eviction they configured.
///
/// This assertion is what must NOT change. Making the evictor clear a store whose active
/// slot it does not own is the tempting one-line repair and it is worse than the defect:
/// `ensure_loaded` reloads only when its OWN registry's `evicted` flag is set
/// (`src/registry.rs:243`), and this registry is not the one any session consults.
#[test]
fn the_dead_evictor_does_not_touch_the_shared_store() {
    let scratch = Scratch::new("untouched");
    let graph = Arc::new(GraphStore::new());
    let loaded = some_triples(&graph);

    let registry = evictor_registry(&scratch, graph.clone());
    std::thread::sleep(std::time::Duration::from_millis(1_100 * TTL_SECS));
    let _ = registry.evictor_tick().expect("tick must not error");

    assert_eq!(
        graph.triple_count(),
        loaded,
        "a registry whose active slot is empty must never clear the shared store: \
         eviction state is per registry, so clearing a store no session's registry has \
         flagged strands every session on a store it believes is resident"
    );
}

/// The control, and the reason the two tests above are about wiring rather than about
/// eviction being broken in general. Give a registry an entry of its own and the same
/// tick evicts, which is what the stdio arm gets at `src/main.rs:1764` because there
/// `spawn_evictor` is handed the very registry `onto_load` fills.
#[test]
fn a_registry_with_its_own_entry_does_evict() {
    let scratch = Scratch::new("does-evict");
    let graph = Arc::new(GraphStore::new());
    let registry = evictor_registry(&scratch, graph.clone());

    let source = scratch.0.join("ontology.ttl");
    std::fs::write(
        &source,
        "@prefix : <https://example.org/> .\n\
         :A a <http://www.w3.org/2002/07/owl#Class> .\n",
    )
    .expect("write source");

    registry
        .load_file(&source.to_string_lossy(), Default::default())
        .expect("load through the registry");
    assert!(graph.triple_count() > 0, "the load must populate the store");

    std::thread::sleep(std::time::Duration::from_millis(1_100 * TTL_SECS));

    let evicted = registry.evictor_tick().expect("tick must not error");
    assert!(
        evicted,
        "a registry that owns an idle entry must evict it; if this fails the defect is \
         not the wiring and the other two tests in this file are measuring the wrong thing"
    );
    assert_eq!(
        graph.triple_count(),
        0,
        "eviction must actually clear the in-memory store"
    );
}
