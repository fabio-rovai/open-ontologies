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
use std::sync::RwLock;

use crate::config::{
    self, Config, FeedbackConfig, ImportsConfig, LanguageConfig, ReasonerConfig, RepoConfig,
    WebhookConfig,
};

// ── Defaults match the previous hardcoded constants exactly ─────────────
const DEFAULT_TABLEAUX_MAX_DEPTH: usize = 100;
const DEFAULT_TABLEAUX_MAX_NODES: usize = 10_000;
const DEFAULT_TABLEAUX_MAX_STEPS: usize = 5_000_000;
/// 10s per satisfiability test. Generous for well-behaved ontologies, and the
/// difference between a reported Unknown and an unbounded hang for the rest.
const DEFAULT_TABLEAUX_TEST_TIMEOUT_MS: usize = 10_000;
/// 180s for a whole classification, matching the ORE competition timeout.
const DEFAULT_CLASSIFY_TIMEOUT_MS: usize = 180_000;
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
/// Generous by design. It is a backstop against exponential backtracking, not a
/// working limit: every ontology in the benchmark set finishes far inside it,
/// and a run that hits it should be read as "this needs more budget" rather
/// than as an answer.
static TABLEAUX_MAX_STEPS: AtomicUsize = AtomicUsize::new(DEFAULT_TABLEAUX_MAX_STEPS);
static TABLEAUX_TEST_TIMEOUT_MS: AtomicUsize =
    AtomicUsize::new(DEFAULT_TABLEAUX_TEST_TIMEOUT_MS);
static CLASSIFY_TIMEOUT_MS: AtomicUsize = AtomicUsize::new(DEFAULT_CLASSIFY_TIMEOUT_MS);
static REASONER_MAX_ITER: AtomicUsize = AtomicUsize::new(DEFAULT_REASONER_MAX_ITER);
static CACHE_HASH_PREFIX: AtomicUsize = AtomicUsize::new(DEFAULT_CACHE_HASH_PREFIX);
static FB_SUPPRESS: AtomicI64 = AtomicI64::new(DEFAULT_FB_SUPPRESS);
static FB_DOWNGRADE: AtomicI64 = AtomicI64::new(DEFAULT_FB_DOWNGRADE);
static REPO_LIST_LIMIT: AtomicUsize = AtomicUsize::new(DEFAULT_REPO_LIST_LIMIT);
static IMPORTS_MAX_DEPTH: AtomicUsize = AtomicUsize::new(DEFAULT_IMPORTS_MAX_DEPTH);
static IMPORTS_TIMEOUT: AtomicU64 = AtomicU64::new(DEFAULT_IMPORTS_TIMEOUT);
static IMPORTS_FOLLOW_REMOTE: AtomicBool = AtomicBool::new(true);
static WEBHOOK_TIMEOUT: AtomicU64 = AtomicU64::new(DEFAULT_WEBHOOK_TIMEOUT);
/// Preferred natural-language tags for label matching. Empty (the default)
/// means "keep all languages" — fully multilingual. Populated from
/// `[language] preferred` / `OPEN_ONTOLOGIES_LANGUAGES` at startup.
static PREFERRED_LANGUAGES: RwLock<Vec<String>> = RwLock::new(Vec::new());

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
    apply_language(&cfg.language);
}

fn apply_language(l: &LanguageConfig) {
    let resolved = config::resolve_languages(l);
    if let Ok(mut guard) = PREFERRED_LANGUAGES.write() {
        *guard = resolved;
    }
}

fn apply_reasoner(r: &ReasonerConfig) {
    let depth = if r.tableaux_max_depth == 0 { DEFAULT_TABLEAUX_MAX_DEPTH } else { r.tableaux_max_depth };
    let nodes = if r.tableaux_max_nodes == 0 { DEFAULT_TABLEAUX_MAX_NODES } else { r.tableaux_max_nodes };
    let iters = if r.max_iterations == 0 { DEFAULT_REASONER_MAX_ITER } else { r.max_iterations };
    TABLEAUX_MAX_DEPTH.store(depth, Ordering::Relaxed);
    TABLEAUX_MAX_NODES.store(nodes, Ordering::Relaxed);
    REASONER_MAX_ITER.store(iters, Ordering::Relaxed);
    // Stored VERBATIM, with no `if x == 0 { DEFAULT }` rewrite, because for a
    // clock zero is a configuration and not an omission: `0` means no limit, and
    // both accessors read it that way. An omitted key does not arrive here as 0
    // — `#[serde(default)]` fills it from `ReasonerConfig::default`, which is
    // 10 000 and 180 000 — so the two cases are distinguishable, which is
    // exactly what the three caps above cannot do.
    TABLEAUX_TEST_TIMEOUT_MS.store(r.tableaux_test_timeout_ms, Ordering::Relaxed);
    CLASSIFY_TIMEOUT_MS.store(r.classify_timeout_ms, Ordering::Relaxed);
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

/// Deterministic ceiling on the WORK one satisfiability test may do, counted in
/// expansion steps. `None` (value 0) means no step bound.
///
/// The node and depth budgets do not bound the number of BRANCHES explored, so
/// before this existed the only thing that did was a wall clock. A clock makes
/// the verdict depend on how fast the machine is, which is how a soundness test
/// came to fail about three runs in five against an unmodified baseline (#161).
/// This bounds the same work deterministically: the same ontology takes the
/// same number of steps on every machine, so a run either finishes or does not,
/// everywhere, and a budget that stops a run says which budget it was.
pub fn tableaux_max_steps() -> Option<u64> {
    match TABLEAUX_MAX_STEPS.load(Ordering::Relaxed) {
        0 => None,
        n => Some(n as u64),
    }
}

/// Wall-clock cut-off for a single tableau satisfiability test, in
/// milliseconds. `None` (value 0) means no time limit.
///
/// This exists because the node and depth budgets do not bound the number of
/// BRANCHES explored. A tableau can stay small and shallow while backtracking
/// through exponentially many disjunction and merge choices, which is exactly
/// how nominal-bearing ontologies hang the reasoner. A clock is the only
/// budget that catches that.
pub fn tableaux_test_timeout_ms() -> Option<u64> {
    match TABLEAUX_TEST_TIMEOUT_MS.load(Ordering::Relaxed) {
        0 => None,
        ms => Some(ms as u64),
    }
}

/// Override the per-phase timeout, for tests.
///
/// This used to say "used by the CLI `--reason-timeout-ms` flag and by tests".
/// There is no such flag anywhere in `src/`; grep for it and the only hit is
/// the sentence that claimed it. With `[reasoner] tableaux_test_timeout_ms`
/// also absent from `ReasonerConfig` until it was added beside
/// `classify_timeout_ms`, neither reasoner clock had ANY user-reachable
/// setting at all, and both were documented as though they did.
pub fn set_tableaux_test_timeout_ms(ms: usize) {
    TABLEAUX_TEST_TIMEOUT_MS.store(ms, Ordering::Relaxed);
}

/// Wall-clock CEILING for an entire reasoning run, in milliseconds. `None`
/// (value 0) means no ceiling.
///
/// Distinct from the per-test timeout, which bounds one satisfiability check.
/// Classification performs one check per class plus one per ordered pair, so
/// the per-test budget multiplied by the pair count is the enormous number this
/// exists to cut off.
///
/// READ THIS BEFORE QUOTING IT AS THE BOUND. It is a ceiling over every phase of
/// a run — consistency, satisfiability, subsumption, ABox, explanation — and it
/// is enforced: `DlReasoner::phase_deadline_within` intersects it with each
/// phase's own deadline, so no phase can outlive it. At the SHIPPED DEFAULTS it
/// nonetheless cannot be the bound that fires, because those five phases each
/// open a `tableaux_test_timeout_ms` budget of 10 000 ms and five times ten is
/// fifty, which is less than a hundred and eighty. Making it the binding bound
/// means minting a deadline per tableau rather than per phase, which was
/// measured: a 20-ontology corpus went from 67s to over 600s with no verdict
/// changing. That number is UNREPRODUCIBLE — its corpus is written down nowhere
/// and matches neither corpus in `tests/reasoner_budget_corpus_bench.rs`; see
/// `DlReasoner::phase_deadline`, which is the one place that says so, and the
/// bench, which is the one place that measures. The engine therefore reports, in
/// `budget.binding_bound` and
/// `budget.note` on every `owl-dl` run, which of the two bounds actually stops
/// the run — including "neither" — rather than leaving a reader to do this
/// arithmetic from two settings and a phase count.
///
/// Default matches the ORE competition convention of 180s.
pub fn classify_timeout_ms() -> Option<u64> {
    match CLASSIFY_TIMEOUT_MS.load(Ordering::Relaxed) {
        0 => None,
        ms => Some(ms as u64),
    }
}

pub fn set_classify_timeout_ms(ms: usize) {
    CLASSIFY_TIMEOUT_MS.store(ms, Ordering::Relaxed);
}
pub fn reasoner_max_iterations() -> usize { REASONER_MAX_ITER.load(Ordering::Relaxed) }

/// Set the forward-chaining iteration cap directly.
///
/// `set_classify_timeout_ms` above is the same shape and exists for the same
/// reason: a knob that only `init_from_config` can move cannot be exercised by
/// a test that has no config file. The cap matters here because a run that
/// stops at it has a closure that is a LOWER BOUND, and a consumer comparing
/// two closures has to be shown behaving correctly when one of them is
/// truncated. A test that forces the cap must restore it, since this is a
/// process-wide global.
pub fn set_reasoner_max_iterations(n: usize) {
    REASONER_MAX_ITER.store(n.max(1), Ordering::Relaxed);
}
pub fn cache_hash_prefix_bytes() -> usize { CACHE_HASH_PREFIX.load(Ordering::Relaxed) }
pub fn feedback_suppress_threshold() -> i64 { FB_SUPPRESS.load(Ordering::Relaxed) }
pub fn feedback_downgrade_threshold() -> i64 { FB_DOWNGRADE.load(Ordering::Relaxed) }
pub fn repo_default_list_limit() -> usize { REPO_LIST_LIMIT.load(Ordering::Relaxed) }
pub fn imports_max_depth() -> usize { IMPORTS_MAX_DEPTH.load(Ordering::Relaxed) }
pub fn imports_request_timeout_secs() -> u64 { IMPORTS_TIMEOUT.load(Ordering::Relaxed) }
pub fn imports_follow_remote() -> bool { IMPORTS_FOLLOW_REMOTE.load(Ordering::Relaxed) }
pub fn webhook_request_timeout_secs() -> u64 { WEBHOOK_TIMEOUT.load(Ordering::Relaxed) }

/// Preferred natural-language tags for label matching. An empty vector means
/// "keep all languages" (multilingual mode). Cloned per call so callers hold no
/// lock; alignment runs are infrequent relative to this cost.
pub fn preferred_languages() -> Vec<String> {
    PREFERRED_LANGUAGES
        .read()
        .map(|g| g.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Both assertions live in ONE test so they run sequentially. They share
    // global atomic state (`init_from_config` mutates the static accessors),
    // and cargo runs tests in parallel by default — so if the two were
    // separate `#[test]` functions, `init_overrides_values` could set the
    // tableaux_max_depth to 250 mid-flight and cause `defaults_match_legacy_constants`
    // to observe 250 instead of 100. Combining them serialises the race.
    #[test]
    fn defaults_then_init_overrides_then_restore() {
        // Phase 1: without calling init_from_config the accessors return the
        // original hardcoded defaults.
        assert_eq!(tableaux_max_depth(), 100);
        assert_eq!(tableaux_max_nodes(), 10_000);
        assert_eq!(cache_hash_prefix_bytes(), 64 * 1024);
        assert_eq!(repo_default_list_limit(), 1000);
        assert_eq!(imports_max_depth(), 3);
        assert!(imports_follow_remote());

        // Phase 2: init_from_config overrides the values.
        let mut cfg = Config::default();
        cfg.reasoner.tableaux_max_depth = 250;
        cfg.cache.hash_prefix_bytes = 128 * 1024;
        cfg.imports.follow_remote = false;
        init_from_config(&cfg);
        assert_eq!(tableaux_max_depth(), 250);
        assert_eq!(cache_hash_prefix_bytes(), 128 * 1024);
        assert!(!imports_follow_remote());

        // Phase 3: restore defaults so subsequent tests in the same process
        // (any test that reads these accessors) aren't affected.
        init_from_config(&Config::default());
        assert_eq!(tableaux_max_depth(), 100);
        assert!(imports_follow_remote());
    }
}
