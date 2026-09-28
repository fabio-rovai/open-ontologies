#![cfg(feature = "embeddings")]
//! The embedding provider is built once per process, not once per HTTP session
//! (#262).
//!
//! # Why every assertion here needs two sessions
//!
//! `OpenOntologiesServer::new_with_repo_options` is what the HTTP arm hands
//! `StreamableHttpService::new`, and that is a factory the transport calls again
//! for every MCP session that connects. Under the defect, session two ran a
//! fresh tract optimize pass and kept its own copy of a 470 MB model; under the
//! fix it clones an `Arc`. A test that opens ONE session sees a build either
//! way, because the first session builds under both arrangements, so it would
//! have passed before the change and proved nothing. The signal is therefore
//! always a comparison across two sessions: how far
//! `shared_embeddings_builds()` moved for the second one, and whether the two
//! sessions' providers are the same allocation.

use std::sync::Arc;

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::embed::shared_embeddings_builds;
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;

/// The build counter is process-wide, so two of these tests running at once
/// would each see the other's builds inside their own delta and the deltas
/// would be wrong in whichever direction the scheduler chose. Clearing the
/// environment is process-wide for the same reason. Every test here takes this
/// first, which costs nothing: there are four of them and none is slow.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Take [`ONE_AT_A_TIME`] and put the embedding environment in a known state.
///
/// Every resolver in `config` reads its environment variable before the config
/// field, so a developer with `OPEN_ONTOLOGIES_EMBEDDINGS_PROVIDER=local`
/// exported would otherwise get a different provider than the one the case
/// under test asked for. Removed rather than saved and restored, because this
/// is a dedicated test binary whose own process nobody else is using.
fn serialised_with_a_clean_environment() -> std::sync::MutexGuard<'static, ()> {
    let guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    for key in [
        "OPEN_ONTOLOGIES_EMBEDDINGS_PROVIDER",
        "OPEN_ONTOLOGIES_EMBEDDINGS_API_BASE",
        "OPEN_ONTOLOGIES_EMBEDDINGS_MODEL",
        "OPEN_ONTOLOGIES_EMBEDDINGS_API_KEY",
        "OPENAI_API_KEY",
    ] {
        unsafe { std::env::remove_var(key) };
    }
    guard
}

/// A configuration whose provider builds with no model file and no network.
///
/// `OpenAIEmbedder::new` builds a `reqwest::Client` and formats a URL. It sends
/// nothing, so this yields a real `Some(provider)` on a machine with no ONNX
/// model and no connectivity, which is what lets the identity assertion below
/// run everywhere rather than only where the 470 MB model happens to be
/// downloaded. `label` keeps each case on its own cache key.
fn reachable_without_a_model(label: &str) -> EmbeddingsConfig {
    EmbeddingsConfig {
        provider: Some("openai".to_string()),
        api_base: Some(format!("http://127.0.0.1:9/{label}")),
        model: Some(format!("oo-262-{label}")),
        ..Default::default()
    }
}

/// One MCP session, built exactly the way the HTTP factory builds one: a fresh
/// `StateDb` handle over the same file, the shared graph, and the same
/// configuration values every time.
fn session(dir: &std::path::Path, cfg: &EmbeddingsConfig, graph: &Arc<GraphStore>) -> OpenOntologiesServer {
    let db = StateDb::open(&dir.join("state.db")).unwrap();
    let cache = CacheConfig {
        enabled: true,
        dir: dir.join("cache").to_string_lossy().into_owned(),
        idle_ttl_secs: 0,
        evictor_interval_secs: 30,
        auto_refresh: false,
        hash_prefix_bytes: 64 * 1024,
    };
    OpenOntologiesServer::new_with_repo_options(
        db,
        graph.clone(),
        None,
        cfg.clone(),
        cache,
        ToolFilter::default(),
        Vec::new(),
    )
}

/// The claim of #262, stated as a count: two sessions, one build.
#[test]
fn a_second_session_does_not_build_a_second_model() {
    let _serial = serialised_with_a_clean_environment();
    let tmp = tempfile::tempdir().unwrap();
    let graph = Arc::new(GraphStore::new());
    let cfg = reachable_without_a_model("two-sessions");

    let before = shared_embeddings_builds();
    let first = session(tmp.path(), &cfg, &graph);
    let after_first = shared_embeddings_builds();

    // Without this the counter could be one that never moves, and the
    // interesting assertion below would hold for the most boring reason there
    // is. The first session must pay, under the defect and under the fix.
    assert_eq!(
        after_first - before,
        1,
        "the first session did not build a provider at all, so the counter \
         below proves nothing"
    );

    let second = session(tmp.path(), &cfg, &graph);
    let after_second = shared_embeddings_builds();

    assert_eq!(
        after_second - after_first,
        0,
        "the second session built its own provider: {} builds for 2 sessions, \
         which is the per-session model load this test exists to forbid",
        after_second - before
    );

    // The count says the work happened once. This says the second session is
    // actually served BY the first session's model rather than by a second copy
    // that merely happened to be cheap, which is the resident-memory half of
    // the defect and the half a counter cannot see.
    let a = first
        .text_embedder()
        .expect("first session has a provider for this configuration");
    let b = second
        .text_embedder()
        .expect("second session has a provider for this configuration");
    assert!(
        Arc::ptr_eq(&a, &b),
        "two sessions hold two separate allocations of the model"
    );
}

/// Ten sessions is where the defect was severe, so state it at ten as well.
/// Under the defect this is ten optimize passes and ten resident copies.
#[test]
fn ten_sessions_still_build_one_model() {
    let _serial = serialised_with_a_clean_environment();
    let tmp = tempfile::tempdir().unwrap();
    let graph = Arc::new(GraphStore::new());
    let cfg = reachable_without_a_model("ten-sessions");

    let before = shared_embeddings_builds();
    let sessions: Vec<_> = (0..10).map(|_| session(tmp.path(), &cfg, &graph)).collect();
    let builds = shared_embeddings_builds() - before;

    assert_eq!(builds, 1, "10 sessions caused {builds} model builds");

    let first = sessions[0].text_embedder().expect("provider");
    for (i, s) in sessions.iter().enumerate().skip(1) {
        let other = s.text_embedder().expect("provider");
        assert!(
            Arc::ptr_eq(&first, &other),
            "session {i} holds its own copy of the model"
        );
    }
}

/// Two different configurations must NOT share, which is the hazard in the
/// obvious implementation of this fix. A single process-wide cell would hand
/// the second server the first server's model and report it as that server's
/// own: vectors from one model served under another model's name, silently.
#[test]
fn a_different_configuration_gets_its_own_provider() {
    let _serial = serialised_with_a_clean_environment();
    let tmp = tempfile::tempdir().unwrap();
    let graph = Arc::new(GraphStore::new());

    let before = shared_embeddings_builds();
    let one = session(tmp.path(), &reachable_without_a_model("config-one"), &graph);
    let two = session(tmp.path(), &reachable_without_a_model("config-two"), &graph);
    let builds = shared_embeddings_builds() - before;

    assert_eq!(builds, 2, "two configurations produced {builds} builds");
    let a = one.text_embedder().expect("provider");
    let b = two.text_embedder().expect("provider");
    assert!(
        !Arc::ptr_eq(&a, &b),
        "a server configured for one model was handed another server's model"
    );
}

/// The absent-model path is unchanged: no provider, no error, and the server
/// still starts. It is also cached, so a deployment with no model does not
/// re-probe the filesystem once per session.
#[test]
fn an_absent_local_model_still_yields_no_provider() {
    let _serial = serialised_with_a_clean_environment();
    let tmp = tempfile::tempdir().unwrap();
    let graph = Arc::new(GraphStore::new());
    let cfg = EmbeddingsConfig {
        provider: Some("local".to_string()),
        model_path: Some(tmp.path().join("absent.onnx").to_string_lossy().into_owned()),
        tokenizer_path: Some(tmp.path().join("absent.json").to_string_lossy().into_owned()),
        ..Default::default()
    };

    let before = shared_embeddings_builds();
    let first = session(tmp.path(), &cfg, &graph);
    let second = session(tmp.path(), &cfg, &graph);
    let builds = shared_embeddings_builds() - before;

    assert!(
        first.text_embedder().is_none(),
        "a missing model file produced a provider"
    );
    assert!(
        second.text_embedder().is_none(),
        "a missing model file produced a provider on the second session"
    );
    assert_eq!(builds, 1, "the absent-model outcome was re-derived per session");
}
