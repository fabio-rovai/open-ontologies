//! Failed source/cache replacements must preserve the last successful dataset.
use std::sync::Arc;

use open_ontologies::config::CacheConfig;
use open_ontologies::graph::GraphStore;
use open_ontologies::registry::{LoadOptions, OntologyRegistry};
use open_ontologies::state::StateDb;
use oxigraph::io::RdfFormat;

const OLD: &str =
    "<https://example.org/old> <https://example.org/p> \"old\" <https://example.org/g> .\n";
const EXTRA: &str = "<https://example.org/extra> <https://example.org/p> \"mutation\" .\n";
const NEW: &str = "<https://example.org/new> <https://example.org/p> \"new\" .\n";
const BROKEN: &str =
    "<https://example.org/partial> <https://example.org/p> \"partial\" .\nthis is not RDF\n";

struct Harness {
    tmp: tempfile::TempDir,
    graph: Arc<GraphStore>,
    registry: OntologyRegistry,
    source: std::path::PathBuf,
}

impl Harness {
    fn new(ttl: u64) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let graph = Arc::new(GraphStore::new());
        let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
        let config = CacheConfig {
            enabled: true,
            dir: tmp.path().join("cache").to_string_lossy().into_owned(),
            idle_ttl_secs: ttl,
            ..CacheConfig::default()
        };
        let registry = OntologyRegistry::new(graph.clone(), db, config).unwrap();
        let source = tmp.path().join("old.nq");
        std::fs::write(&source, OLD).unwrap();
        Self {
            tmp,
            graph,
            registry,
            source,
        }
    }

    fn load(&self, refresh: bool) -> open_ontologies::registry::LoadResult {
        self.registry
            .load_file(
                self.source.to_str().unwrap(),
                LoadOptions {
                    auto_refresh: refresh,
                    ..LoadOptions::default()
                },
            )
            .unwrap()
    }

    fn snapshot(&self) -> String {
        self.graph.serialize("nquads").unwrap()
    }
}

#[test]
fn malformed_replacement_preserves_active_dataset_and_mutations() {
    let h = Harness::new(0);
    h.load(false);
    h.graph.load_nquads(EXTRA).unwrap();
    let before = h.snapshot();
    let broken = h.tmp.path().join("broken.ttl");
    std::fs::write(&broken, BROKEN).unwrap();
    assert!(
        h.registry
            .load_file(broken.to_str().unwrap(), LoadOptions::default())
            .is_err()
    );
    assert_eq!(h.snapshot(), before);
    h.registry.ensure_loaded().unwrap();
    assert_eq!(h.snapshot(), before);
}

#[test]
fn malformed_warm_cache_does_not_clear_or_publish_partial_quads() {
    let h = Harness::new(0);
    let loaded = h.load(false);
    h.graph.load_nquads(EXTRA).unwrap();
    let before = h.snapshot();
    std::fs::write(&loaded.cache_path, BROKEN).unwrap();
    assert!(
        h.registry
            .load_file(h.source.to_str().unwrap(), LoadOptions::default())
            .is_err()
    );
    assert_eq!(h.snapshot(), before);
}

#[test]
fn malformed_auto_refresh_keeps_live_dataset_until_a_valid_refresh() {
    let h = Harness::new(0);
    h.load(true);
    h.graph.load_nquads(EXTRA).unwrap();
    let before = h.snapshot();
    std::fs::write(&h.source, BROKEN).unwrap();
    assert!(h.registry.ensure_loaded().is_err());
    assert_eq!(h.snapshot(), before);
    std::fs::write(&h.source, NEW).unwrap();
    h.registry.ensure_loaded().unwrap();
    assert_eq!(h.snapshot(), NEW);
}

#[test]
fn malformed_eviction_snapshot_does_not_publish_a_partial_restore() {
    let h = Harness::new(1);
    let loaded = h.load(false);
    std::thread::sleep(std::time::Duration::from_millis(1100));
    assert!(h.registry.evictor_tick().unwrap());
    let mut snapshot = std::path::PathBuf::from(loaded.cache_path);
    snapshot.set_extension("nq.evicted");
    assert!(snapshot.exists());
    std::fs::write(&snapshot, BROKEN).unwrap();
    h.graph.load_nquads(EXTRA).unwrap();
    let before = h.snapshot();
    assert!(h.registry.ensure_loaded().is_err());
    assert_eq!(h.snapshot(), before);
    assert!(snapshot.exists(), "a failed restore must keep its snapshot");
}

#[test]
fn successful_replacement_removes_old_quads_but_append_apis_still_append() {
    let h = Harness::new(0);
    h.load(false);
    let new = h.tmp.path().join("new.ttl");
    std::fs::write(&new, format!("{NEW}{NEW}")).unwrap();
    let result = h
        .registry
        .load_file(new.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    assert_eq!(result.triple_count, 1);
    assert_eq!(h.snapshot(), NEW);
    let warm = h
        .registry
        .load_file(new.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    assert_eq!(warm.origin, "cache");
    assert_eq!(warm.triple_count, 1);
    assert_eq!(h.snapshot(), NEW);
    assert_eq!(h.graph.load_file(h.source.to_str().unwrap()).unwrap(), 1);
    assert_eq!(h.graph.load_content(EXTRA, RdfFormat::NQuads).unwrap(), 1);
    let contents = h.snapshot();
    assert!(contents.contains("\"new\""));
    assert!(contents.contains("\"old\" <https://example.org/g>"));
    assert!(contents.contains("\"mutation\""));
    assert_eq!(h.graph.load_content(EXTRA, RdfFormat::NQuads).unwrap(), 0);
}
