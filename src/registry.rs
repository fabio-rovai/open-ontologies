//! Ontology registry with TTL-based eviction and on-demand reload.
//!
//! The registry tracks the *currently loaded* ontology (a single active slot
//! at a time) along with the metadata needed to reload it cheaply from the
//! N-Triples compile cache.
//!
//! Lifecycle:
//!   1. `load_file(path, name, opts)` — parses (or reads cache), populates
//!      `Arc<GraphStore>`, records active entry.
//!   2. `touch()` — called by every read tool (`onto_query`, `onto_stats`, ...)
//!      to keep the active entry alive.
//!   3. Background `evictor_tick()` clears the in-memory store if the active
//!      entry has been idle for longer than `idle_ttl_secs`.
//!   4. `ensure_loaded()` — called by every read tool *before* using the graph;
//!      if the store was evicted, it reloads from the cache (or from source if
//!      `auto_refresh` detected a change).
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::cache::{CacheManager, SourceFingerprint};
use crate::config::CacheConfig;
use crate::graph::GraphStore;
use crate::state::StateDb;

/// Options accepted by `OntologyRegistry::load_file`.
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    pub name: Option<String>,
    /// Re-parse the source on every `ensure_loaded` if its mtime/sha changed.
    pub auto_refresh: bool,
    /// Bypass the cache and re-parse from source unconditionally.
    pub force_recompile: bool,
}

/// Outcome of a load operation, returned to callers (and surfaced in tool JSON).
#[derive(Debug, Clone)]
pub struct LoadResult {
    pub name: String,
    pub source_path: String,
    pub triple_count: usize,
    /// "cache" if loaded from N-Triples cache, "source" if re-parsed.
    pub origin: &'static str,
    pub cache_path: String,
}

#[derive(Debug)]
struct ActiveEntry {
    name: String,
    source_path: String,
    fingerprint: SourceFingerprint,
    cache_path: PathBuf,
    auto_refresh: bool,
    /// `Mutex<Instant>` to allow `&self`-touch from many threads.
    last_access: Mutex<Instant>,
    /// True after the in-memory store has been cleared by the evictor.
    /// Read tools must reload before using the graph.
    evicted: Mutex<bool>,
}

/// Registry of the active ontology. Wrapped in `Arc` and shared with the server.
pub struct OntologyRegistry {
    graph: Arc<GraphStore>,
    cache: CacheManager,
    config: CacheConfig,
    /// `Mutex<Option<ActiveEntry>>` — at most one entry is "active" at a time
    /// in this minimal model. Multi-slot extension is future work.
    active: Mutex<Option<ActiveEntry>>,
    /// Per-name reload mutex to prevent thundering-herd reloads.
    reload_lock: Mutex<()>,
}

impl OntologyRegistry {
    pub fn new(graph: Arc<GraphStore>, db: StateDb, config: CacheConfig) -> Result<Self> {
        let cache_dir = PathBuf::from(crate::config::expand_tilde(&config.dir));
        let cache = CacheManager::new(cache_dir, db)?;
        Ok(Self {
            graph,
            cache,
            config,
            active: Mutex::new(None),
            reload_lock: Mutex::new(()),
        })
    }

    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    pub fn cache(&self) -> &CacheManager {
        &self.cache
    }

    /// Load an RDF file into the graph, using the compile cache when valid.
    pub fn load_file(&self, path: &str, opts: LoadOptions) -> Result<LoadResult> {
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(anyhow!("file not found: {}", path));
        }
        let name = opts.name.clone().unwrap_or_else(|| crate::cache::derive_name(path));
        let fp = SourceFingerprint::from_path(path_obj)?;

        // Decide load strategy: cache vs source.
        let existing = self.cache.get(&name)?;
        let cache_is_fresh = !opts.force_recompile
            && self.config.enabled
            && existing
                .as_ref()
                .map(|e| {
                    e.source_path == path
                        && e.source_mtime == fp.mtime_secs
                        && e.source_size == fp.size
                        && e.source_sha == fp.sha_prefix
                        && Path::new(&e.cache_path).exists()
                })
                .unwrap_or(false);

        // Always start from a clean store for a fresh load.
        self.graph.clear()?;

        let (triple_count, origin, cache_path) = if cache_is_fresh {
            let entry = existing.unwrap();
            let nt = std::fs::read_to_string(&entry.cache_path)
                .with_context(|| format!("read cache {}", entry.cache_path))?;
            let count = self.graph.load_ntriples(&nt)?;
            self.cache.touch(&name)?;
            (count, "cache", PathBuf::from(entry.cache_path))
        } else {
            let count = self
                .graph
                .load_file(path)
                .with_context(|| format!("parse source {}", path))?;
            let cache_path = if self.config.enabled {
                let cp = self.cache.cache_path_for(&name, &fp.sha_prefix);
                let nt = self.graph.serialize("ntriples")?;
                CacheManager::atomic_write(&cp, &nt)?;
                self.cache.upsert(&name, path, &fp, &cp, count)?;
                cp
            } else {
                PathBuf::new()
            };
            (count, "source", cache_path)
        };

        // Record active entry.
        let mut active = self.active.lock().unwrap();
        *active = Some(ActiveEntry {
            name: name.clone(),
            source_path: path.to_string(),
            fingerprint: fp,
            cache_path: cache_path.clone(),
            auto_refresh: opts.auto_refresh,
            last_access: Mutex::new(Instant::now()),
            evicted: Mutex::new(false),
        });

        Ok(LoadResult {
            name,
            source_path: path.to_string(),
            triple_count,
            origin,
            cache_path: cache_path.to_string_lossy().into_owned(),
        })
    }

    /// Update `last_access` if there is an active entry.
    pub fn touch(&self) {
        if let Some(entry) = &*self.active.lock().unwrap() {
            *entry.last_access.lock().unwrap() = Instant::now();
        }
    }

    /// Make sure the in-memory store reflects the active entry.
    /// If the store was evicted, reload from cache (or source on refresh).
    /// No-op if no active entry exists.
    pub fn ensure_loaded(&self) -> Result<()> {
        // Single-flight guard.
        let _g = self.reload_lock.lock().unwrap();

        // Snapshot needed fields (avoid holding the active lock across reload).
        let needs_reload;
        let auto_refresh;
        let source_path;
        let stored_fp;
        let cache_path;
        let name;
        {
            let active_guard = self.active.lock().unwrap();
            let Some(entry) = active_guard.as_ref() else {
                return Ok(());
            };
            // Touch first.
            *entry.last_access.lock().unwrap() = Instant::now();
            needs_reload = *entry.evicted.lock().unwrap();
            auto_refresh = entry.auto_refresh;
            source_path = entry.source_path.clone();
            stored_fp = entry.fingerprint.clone();
            cache_path = entry.cache_path.clone();
            name = entry.name.clone();
        }

        // Auto-refresh: if the source file changed, recompile (even if not evicted).
        let mut refreshed = false;
        if auto_refresh && Path::new(&source_path).exists() {
            let cur = SourceFingerprint::from_path(Path::new(&source_path))?;
            if cur != stored_fp {
                // Source changed — re-parse and rewrite cache.
                self.graph.clear()?;
                let count = self.graph.load_file(&source_path)?;
                let new_cache = self.cache.cache_path_for(&name, &cur.sha_prefix);
                let nt = self.graph.serialize("ntriples")?;
                CacheManager::atomic_write(&new_cache, &nt)?;
                self.cache.upsert(&name, &source_path, &cur, &new_cache, count)?;

                let mut active_guard = self.active.lock().unwrap();
                if let Some(entry) = active_guard.as_mut() {
                    entry.fingerprint = cur;
                    entry.cache_path = new_cache;
                    *entry.evicted.lock().unwrap() = false;
                }
                refreshed = true;
            }
        }

        if needs_reload && !refreshed {
            // Reload from N-Triples cache; fall back to source if cache file
            // is missing for some reason.
            if cache_path.exists() {
                let nt = std::fs::read_to_string(&cache_path)?;
                self.graph.clear()?;
                self.graph.load_ntriples(&nt)?;
            } else if Path::new(&source_path).exists() {
                self.graph.clear()?;
                self.graph.load_file(&source_path)?;
            } else {
                return Err(anyhow!(
                    "ontology '{}' was evicted but neither cache file '{}' nor source '{}' exists",
                    name, cache_path.display(), source_path
                ));
            }
            let active_guard = self.active.lock().unwrap();
            if let Some(entry) = active_guard.as_ref() {
                *entry.evicted.lock().unwrap() = false;
            }
            self.cache.touch(&name)?;
        }

        Ok(())
    }

    /// Evict the active entry if idle longer than `idle_ttl_secs`.
    /// Returns `true` when an eviction took place.
    pub fn evictor_tick(&self) -> Result<bool> {
        if !self.config.enabled || self.config.idle_ttl_secs == 0 {
            return Ok(false);
        }
        let ttl = Duration::from_secs(self.config.idle_ttl_secs);
        let active = self.active.lock().unwrap();
        let Some(entry) = active.as_ref() else { return Ok(false) };
        let already = *entry.evicted.lock().unwrap();
        if already {
            return Ok(false);
        }
        let elapsed = entry.last_access.lock().unwrap().elapsed();
        if elapsed >= ttl {
            // Clear the in-memory store to release memory; cache file remains.
            self.graph.clear()?;
            *entry.evicted.lock().unwrap() = true;
            return Ok(true);
        }
        Ok(false)
    }

    /// Manually unload the active ontology (clear graph + drop active slot).
    /// The cache file is preserved unless `delete_cache` is true.
    pub fn unload(&self, delete_cache: bool) -> Result<Option<String>> {
        let mut active = self.active.lock().unwrap();
        let Some(entry) = active.take() else { return Ok(None) };
        self.graph.clear()?;
        if delete_cache {
            self.cache.remove(&entry.name)?;
        }
        Ok(Some(entry.name))
    }

    /// Force recompile the active ontology from source (used by `onto_recompile`).
    pub fn recompile(&self) -> Result<LoadResult> {
        let (path, name, auto_refresh) = {
            let active = self.active.lock().unwrap();
            let entry = active
                .as_ref()
                .ok_or_else(|| anyhow!("no active ontology to recompile"))?;
            (entry.source_path.clone(), entry.name.clone(), entry.auto_refresh)
        };
        self.load_file(
            &path,
            LoadOptions {
                name: Some(name),
                auto_refresh,
                force_recompile: true,
            },
        )
    }

    /// Status snapshot for `onto_cache_status`.
    pub fn status(&self) -> serde_json::Value {
        let active = self.active.lock().unwrap();
        let active_json = if let Some(entry) = active.as_ref() {
            let evicted = *entry.evicted.lock().unwrap();
            let last_access_secs = entry.last_access.lock().unwrap().elapsed().as_secs();
            serde_json::json!({
                "name": entry.name,
                "source_path": entry.source_path,
                "cache_path": entry.cache_path.to_string_lossy(),
                "auto_refresh": entry.auto_refresh,
                "evicted": evicted,
                "idle_seconds": last_access_secs,
                "in_memory_triples": self.graph.triple_count(),
            })
        } else {
            serde_json::Value::Null
        };
        let entries: Vec<_> = self
            .cache
            .list()
            .unwrap_or_default()
            .into_iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name,
                    "source_path": e.source_path,
                    "cache_path": e.cache_path,
                    "triple_count": e.triple_count,
                    "compiled_at": e.compiled_at,
                    "last_access_at": e.last_access_at,
                })
            })
            .collect();
        serde_json::json!({
            "active": active_json,
            "cache_entries": entries,
            "config": {
                "enabled": self.config.enabled,
                "dir": self.config.dir,
                "idle_ttl_secs": self.config.idle_ttl_secs,
                "auto_refresh": self.config.auto_refresh,
            }
        })
    }
}

/// Spawn the background evictor task. Returns a `JoinHandle` that callers can
/// keep alive; dropping it does NOT abort (the task is detached but will exit
/// with the runtime).
pub fn spawn_evictor(registry: Arc<OntologyRegistry>) -> tokio::task::JoinHandle<()> {
    let interval_secs = registry.config().evictor_interval_secs.max(1);
    tokio::spawn(async move {
        let mut ticker =
            tokio::time::interval(Duration::from_secs(interval_secs));
        // Skip the immediate tick so we don't evict before any access happens.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            if let Err(e) = registry.evictor_tick() {
                tracing::warn!("ontology evictor tick failed: {}", e);
            }
        }
    })
}
