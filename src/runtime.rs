//! Runtime-tunable knobs derived from `Config`.
//!
//! Many internal modules (`tableaux`, `reason`, `cache`, `feedback`,
//! `webhook`, `server::onto_repo_list`, `server::onto_import`) historically
//! used `const` constants for safety/operational limits. To make them
//! configurable from `config.toml` (and from environment variables for the
//! most operationally critical ones) without threading a `&Config` through
//! every call site, we mirror those constants into atomic globals here.
//!
//! `init_from_config` is invoked once at server startup. Each accessor falls
//! back to the same default the original constant used, so callers that run
//! before initialisation (e.g. CLI subcommands that don't load a config)
//! observe the legacy behaviour.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};

use crate::config::{
    self, Config, FeedbackConfig, ImportsConfig, ReasonerConfig, RepoConfig, WebhookConfig,
};

// ── Defaults match the previous hardcoded constants exactly ─────────────
const DEFAULT_TABLEAUX_MAX_DEPTH: usize = 100;
const DEFAULT_TABLEAUX_MAX_NODES: usize = 10_000;
// Original reason.rs used 50; section 3 of the audit recommends 64 as a
// slightly more generous, explicitly-documented value. We adopt 64 as the
// new default since neither value affects fixpoint correctness — they only
// bound the maximum number of expansion sweeps.
const DEFAULT_REASONER_MAX_ITER: usize = 64;
const DEFAULT_CACHE_HASH_PREFIX: usize = 64 * 1024;
const DEFAULT_FB_SUPPRESS: i64 = 3;
const DEFAULT_FB_DOWNGRADE: i64 = 2;
const DEFAULT_REPO_LIST_LIMIT: usize = 1000;
const DEFAULT_IMPORTS_MAX_DEPTH: usize = 3;
const DEFAULT_IMPORTS_TIMEOUT: u64 = 30;
const DEFAULT_WEBHOOK_TIMEOUT: u64 = 10;

static TABLEAUX_MAX_DEPTH: AtomicUsize = AtomicUsize::new(DEFAULT_TABLEAUX_MAX_DEPTH);
static TABLEAUX_MAX_NODES: AtomicUsize = AtomicUsize::new(DEFAULT_TABLEAUX_MAX_NODES);
static REASONER_MAX_ITER: AtomicUsize = AtomicUsize::new(DEFAULT_REASONER_MAX_ITER);
static CACHE_HASH_PREFIX: AtomicUsize = AtomicUsize::new(DEFAULT_CACHE_HASH_PREFIX);
static FB_SUPPRESS: AtomicI64 = AtomicI64::new(DEFAULT_FB_SUPPRESS);
static FB_DOWNGRADE: AtomicI64 = AtomicI64::new(DEFAULT_FB_DOWNGRADE);
static REPO_LIST_LIMIT: AtomicUsize = AtomicUsize::new(DEFAULT_REPO_LIST_LIMIT);
static IMPORTS_MAX_DEPTH: AtomicUsize = AtomicUsize::new(DEFAULT_IMPORTS_MAX_DEPTH);
static IMPORTS_TIMEOUT: AtomicU64 = AtomicU64::new(DEFAULT_IMPORTS_TIMEOUT);
static IMPORTS_FOLLOW_REMOTE: AtomicBool = AtomicBool::new(true);
static WEBHOOK_TIMEOUT: AtomicU64 = AtomicU64::new(DEFAULT_WEBHOOK_TIMEOUT);

/// Initialise all runtime knobs from a loaded `Config`. Idempotent — calling
/// this multiple times simply overwrites the current values, which is fine
/// because all consumers re-read on every use.
pub fn init_from_config(cfg: &Config) {
    apply_reasoner(&cfg.reasoner);
    apply_cache(cfg.cache.hash_prefix_bytes);
    apply_feedback(&cfg.feedback);
    apply_repo(&cfg.repo);
    apply_imports(&cfg.imports);
    apply_webhook(&cfg.webhook);
}

fn apply_reasoner(r: &ReasonerConfig) {
    let depth = if r.tableaux_max_depth == 0 { DEFAULT_TABLEAUX_MAX_DEPTH } else { r.tableaux_max_depth };
    let nodes = if r.tableaux_max_nodes == 0 { DEFAULT_TABLEAUX_MAX_NODES } else { r.tableaux_max_nodes };
    let iters = if r.max_iterations == 0 { DEFAULT_REASONER_MAX_ITER } else { r.max_iterations };
    TABLEAUX_MAX_DEPTH.store(depth, Ordering::Relaxed);
    TABLEAUX_MAX_NODES.store(nodes, Ordering::Relaxed);
    REASONER_MAX_ITER.store(iters, Ordering::Relaxed);
}

fn apply_cache(hash_prefix: usize) {
    let v = if hash_prefix == 0 { DEFAULT_CACHE_HASH_PREFIX } else { hash_prefix };
    CACHE_HASH_PREFIX.store(v, Ordering::Relaxed);
}

fn apply_feedback(f: &FeedbackConfig) {
    FB_SUPPRESS.store(f.suppress_threshold, Ordering::Relaxed);
    FB_DOWNGRADE.store(f.downgrade_threshold, Ordering::Relaxed);
}

fn apply_repo(r: &RepoConfig) {
    let v = if r.default_list_limit == 0 { DEFAULT_REPO_LIST_LIMIT } else { r.default_list_limit };
    REPO_LIST_LIMIT.store(v, Ordering::Relaxed);
}

fn apply_imports(i: &ImportsConfig) {
    let depth = if i.max_depth == 0 { DEFAULT_IMPORTS_MAX_DEPTH } else { i.max_depth };
    IMPORTS_MAX_DEPTH.store(depth, Ordering::Relaxed);
    IMPORTS_TIMEOUT.store(config::resolve_imports_timeout_secs(i), Ordering::Relaxed);
    IMPORTS_FOLLOW_REMOTE.store(i.follow_remote, Ordering::Relaxed);
}

fn apply_webhook(w: &WebhookConfig) {
    WEBHOOK_TIMEOUT.store(config::resolve_webhook_timeout_secs(w), Ordering::Relaxed);
}

// ── Accessors ───────────────────────────────────────────────────────────

pub fn tableaux_max_depth() -> usize { TABLEAUX_MAX_DEPTH.load(Ordering::Relaxed) }
pub fn tableaux_max_nodes() -> usize { TABLEAUX_MAX_NODES.load(Ordering::Relaxed) }
pub fn reasoner_max_iterations() -> usize { REASONER_MAX_ITER.load(Ordering::Relaxed) }
pub fn cache_hash_prefix_bytes() -> usize { CACHE_HASH_PREFIX.load(Ordering::Relaxed) }
pub fn feedback_suppress_threshold() -> i64 { FB_SUPPRESS.load(Ordering::Relaxed) }
pub fn feedback_downgrade_threshold() -> i64 { FB_DOWNGRADE.load(Ordering::Relaxed) }
pub fn repo_default_list_limit() -> usize { REPO_LIST_LIMIT.load(Ordering::Relaxed) }
pub fn imports_max_depth() -> usize { IMPORTS_MAX_DEPTH.load(Ordering::Relaxed) }
pub fn imports_request_timeout_secs() -> u64 { IMPORTS_TIMEOUT.load(Ordering::Relaxed) }
pub fn imports_follow_remote() -> bool { IMPORTS_FOLLOW_REMOTE.load(Ordering::Relaxed) }
pub fn webhook_request_timeout_secs() -> u64 { WEBHOOK_TIMEOUT.load(Ordering::Relaxed) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_legacy_constants() {
        // Without calling init_from_config the accessors return the
        // original hardcoded defaults.
        assert_eq!(tableaux_max_depth(), 100);
        assert_eq!(tableaux_max_nodes(), 10_000);
        assert_eq!(cache_hash_prefix_bytes(), 64 * 1024);
        assert_eq!(repo_default_list_limit(), 1000);
        assert_eq!(imports_max_depth(), 3);
        assert!(imports_follow_remote());
    }

    #[test]
    fn init_overrides_values() {
        let mut cfg = Config::default();
        cfg.reasoner.tableaux_max_depth = 250;
        cfg.cache.hash_prefix_bytes = 128 * 1024;
        cfg.imports.follow_remote = false;
        init_from_config(&cfg);
        assert_eq!(tableaux_max_depth(), 250);
        assert_eq!(cache_hash_prefix_bytes(), 128 * 1024);
        assert!(!imports_follow_remote());

        // Restore defaults so subsequent tests in the same process aren't
        // affected.
        init_from_config(&Config::default());
    }
}
