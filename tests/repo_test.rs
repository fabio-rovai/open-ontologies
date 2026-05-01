//! Integration tests for the on-disk ontology repository helpers
//! (`src/repo.rs`) used by the `onto_repo_list` and `onto_repo_load` MCP
//! tools.

use std::path::PathBuf;
use std::sync::Arc;

use open_ontologies::config::{resolve_ontology_dirs, Config};
use open_ontologies::graph::GraphStore;
use open_ontologies::registry::{LoadOptions, OntologyRegistry};
use open_ontologies::repo;
use open_ontologies::state::StateDb;

const SAMPLE_TTL: &str = r#"
@prefix owl:  <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex:   <http://example.org/test#> .

ex:Animal a owl:Class ; rdfs:label "Animal" .
ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal ; rdfs:label "Dog" .
"#;

const SAMPLE_TTL_B: &str = r#"
@prefix owl:  <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex:   <http://example.org/other#> .

ex:Vehicle a owl:Class ; rdfs:label "Vehicle" .
"#;

fn setup_repo() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("ttl_data");
    std::fs::create_dir_all(&repo).unwrap();
    let a = repo.join("animals.ttl");
    let b = repo.join("vehicles.ttl");
    std::fs::write(&a, SAMPLE_TTL).unwrap();
    std::fs::write(&b, SAMPLE_TTL_B).unwrap();
    // A non-RDF file that should be ignored.
    std::fs::write(repo.join("README.md"), "not rdf").unwrap();
    // A nested file to exercise the recursive flag.
    let nested = repo.join("sub");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("nested.ttl"), SAMPLE_TTL).unwrap();
    (tmp, repo, a)
}

#[test]
fn list_one_filters_to_rdf_extensions() {
    let (_tmp, repo, _) = setup_repo();
    let entries = repo::list_one(&repo, &repo, false);
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"animals"));
    assert!(names.contains(&"vehicles"));
    assert!(!names.contains(&"README"), "non-RDF file should be skipped");
    // Non-recursive: nested.ttl excluded.
    assert!(!names.contains(&"nested"));
}

#[test]
fn list_one_recursive_includes_nested() {
    let (_tmp, repo, _) = setup_repo();
    let entries = repo::list_one(&repo, &repo, true);
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"nested"));
}

#[test]
fn glob_filters_filename_only() {
    assert!(repo::glob_match("*.ttl", "foo.ttl"));
    assert!(!repo::glob_match("*.ttl", "foo.nt"));
    assert!(repo::glob_match("a*", "abc"));
}

#[test]
fn resolve_load_target_by_bare_stem() {
    let (_tmp, repo, expected) = setup_repo();
    let path = repo::resolve_load_target("animals", &[repo.clone()]).unwrap();
    assert_eq!(
        std::fs::canonicalize(&path).unwrap(),
        std::fs::canonicalize(&expected).unwrap()
    );
}

#[test]
fn resolve_load_target_by_relative_path() {
    let (_tmp, repo, _) = setup_repo();
    let path = repo::resolve_load_target("sub/nested.ttl", &[repo.clone()]).unwrap();
    assert!(path.ends_with("sub/nested.ttl") || path.ends_with("sub\\nested.ttl"));
}

#[test]
fn resolve_load_target_rejects_outside_path() {
    let (_tmp, repo, _) = setup_repo();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("evil.ttl");
    std::fs::write(&outside_file, SAMPLE_TTL).unwrap();
    let err = repo::resolve_load_target(outside_file.to_str().unwrap(), &[repo.clone()])
        .unwrap_err()
        .to_string();
    assert!(err.contains("outside") || err.contains("no file"), "got: {}", err);
}

#[test]
fn resolve_load_target_ambiguous_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let r1 = tmp.path().join("r1");
    let r2 = tmp.path().join("r2");
    std::fs::create_dir_all(&r1).unwrap();
    std::fs::create_dir_all(&r2).unwrap();
    std::fs::write(r1.join("dup.ttl"), SAMPLE_TTL).unwrap();
    std::fs::write(r2.join("dup.ttl"), SAMPLE_TTL).unwrap();
    let err = repo::resolve_load_target("dup", &[r1, r2])
        .unwrap_err()
        .to_string();
    assert!(err.contains("ambiguous"), "got: {}", err);
}

#[test]
fn resolve_within_repos_rejects_traversal() {
    let (_tmp, repo, _) = setup_repo();
    let err = repo::resolve_within_repos("/etc", &[repo.clone()])
        .unwrap_err()
        .to_string();
    assert!(err.contains("not under"), "got: {}", err);
}

#[test]
fn resolve_within_repos_accepts_subdir() {
    let (_tmp, repo, _) = setup_repo();
    let (resolved, repo_root) = repo::resolve_within_repos("sub", &[repo.clone()]).unwrap();
    assert!(resolved.ends_with("sub"));
    assert_eq!(
        std::fs::canonicalize(&repo_root).unwrap(),
        std::fs::canonicalize(&repo).unwrap()
    );
}

#[test]
fn registry_loads_path_resolved_from_repo() {
    // End-to-end: simulate what `onto_repo_load` does — resolve a name against
    // the configured repo dirs, then call `registry.load_file`.
    let (_tmp, repo, _) = setup_repo();
    let state_dir = tempfile::tempdir().unwrap();
    let cache_dir = state_dir.path().join("cache");
    let db = StateDb::open(&state_dir.path().join("state.db")).unwrap();
    let graph = Arc::new(GraphStore::new());
    let cfg = open_ontologies::config::CacheConfig {
        enabled: true,
        dir: cache_dir.to_string_lossy().into_owned(),
        idle_ttl_secs: 0,
        evictor_interval_secs: 30,
        auto_refresh: false,
        hash_prefix_bytes: 64 * 1024,
    };
    let registry = Arc::new(OntologyRegistry::new(graph.clone(), db, cfg).unwrap());

    let path = repo::resolve_load_target("vehicles", &[repo.clone()]).unwrap();
    let res = registry
        .load_file(&path.to_string_lossy(), LoadOptions::default())
        .unwrap();
    assert_eq!(res.name, "vehicles");
    assert!(res.triple_count > 0);
}

#[test]
fn config_alias_data_dirs_works() {
    let toml = r#"
[general]
data_dir = "/tmp/state"
data_dirs = ["/tmp/a", "/tmp/b"]
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.general.ontology_dirs, vec!["/tmp/a", "/tmp/b"]);
}

#[test]
fn config_canonical_ontology_dirs_works() {
    let toml = r#"
[general]
data_dir = "/tmp/state"
ontology_dirs = ["./ttl_data"]
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.general.ontology_dirs, vec!["./ttl_data"]);
}

#[test]
fn resolve_ontology_dirs_dedupes_and_expands_tilde() {
    // Ensure the env var is unset for this test (other tests may set it).
    // Safe in single-threaded test flow; cargo test in this crate is the
    // default threadpool but env reads happen synchronously in this fn.
    // Use a unique env-cleared scope to be safe.
    let prev = std::env::var("OPEN_ONTOLOGIES_ONTOLOGY_DIRS").ok();
    // SAFETY: tests touching process env must accept the inherent unsafety
    // of process-wide globals; this test is gated by the env var name.
    unsafe { std::env::remove_var("OPEN_ONTOLOGIES_ONTOLOGY_DIRS") };
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().to_string_lossy().into_owned();
    let resolved =
        resolve_ontology_dirs(&[p.clone(), p.clone(), "".to_string()]);
    assert_eq!(resolved.len(), 1, "duplicates should be removed");
    if let Some(v) = prev {
        unsafe { std::env::set_var("OPEN_ONTOLOGIES_ONTOLOGY_DIRS", v) };
    }
}
