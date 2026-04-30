# Compile cache, dynamic load/unload, and tool exposure control

This document describes three operational features added to make Open
Ontologies suitable for serving many ontologies on a memory-constrained host.

## 1. Compile cache (parsed-graph reuse)

When `onto_load` is called with a file path, the parsed graph is serialized
to N-Triples and written to `[cache] dir`. Subsequent loads of the same
source file (unchanged mtime/size/sha) read the N-Triples cache directly,
which is significantly faster than re-parsing Turtle / RDF-XML / etc.

Configuration (`config.toml`):

```toml
[cache]
enabled = true
dir = "~/.open-ontologies/cache"
```

Inspect with `onto_cache_status`. Bypass the cache for one call with
`onto_load { force_recompile: true }`. Force-recompile the active
ontology with `onto_recompile`.

## 2. Idle TTL eviction (memory-saving)

The registry tracks a `last_access` timestamp for the active ontology.
A background task running every `evictor_interval_secs` seconds clears
the in-memory store when `now - last_access >= idle_ttl_secs`. The
on-disk N-Triples cache is preserved.

```toml
[cache]
idle_ttl_secs = 600          # unload after 10 minutes idle
evictor_interval_secs = 30   # check every 30 seconds
```

Set `idle_ttl_secs = 0` to disable eviction.

## 3. Auto-load on query

Every read tool (`onto_query`, `onto_stats`, `onto_save`, ...) calls
`registry.ensure_loaded()` before touching the graph. If the ontology
was evicted, it is reloaded from the N-Triples cache. The MCP client
sees a slightly slower first query after eviction; subsequent queries
hit warm memory.

## 4. Auto-refresh on file change

When `onto_load` was called with `auto_refresh: true` (or `--auto-refresh`
was passed to the server), `ensure_loaded()` additionally checks the
source file's mtime/size/sha on every call. If it changed, the source
is re-parsed and the cache is rewritten before the query runs.

This is opt-in for predictability — without it, running `onto_recompile`
is the explicit way to pick up source-file edits.

## 5. MCP tool exposure filter

Operators can restrict which `onto_*` tools the MCP server advertises.

CLI:

```sh
open-ontologies serve --tools-allow "onto_status,onto_query,@read_only"
open-ontologies serve --tools-deny  "onto_clear,onto_apply"
```

Config:

```toml
[tools]
mode = "allow"
list = ["onto_status", "onto_query", "onto_save"]
groups = ["read_only"]
```

Modes: `all` (default), `allow` (only listed tools exposed), `deny`
(all tools except listed). Groups are expanded to curated sets:

- `read_only` — `onto_status`, `onto_validate`, `onto_query`, `onto_stats`,
  `onto_diff`, `onto_lint`, `onto_history`, `onto_lineage`,
  `onto_cache_status`, `onto_dl_check`, `onto_dl_explain`, `onto_search`,
  `onto_similarity`
- `mutating` — `onto_load`, `onto_clear`, `onto_save`, `onto_convert`,
  `onto_pull`, `onto_import`, `onto_marketplace`, `onto_version`,
  `onto_rollback`, `onto_ingest`, `onto_map`, `onto_shacl`, `onto_reason`,
  `onto_extend`, `onto_unload`, `onto_recompile`
- `governance` — `onto_plan`, `onto_apply`, `onto_lock`, `onto_drift`,
  `onto_enforce`, `onto_monitor`, `onto_monitor_clear`, `onto_align`,
  `onto_align_feedback`, `onto_lint_feedback`, `onto_enforce_feedback`
- `remote` — `onto_pull`, `onto_push`, `onto_marketplace`, `onto_import`
- `embeddings` — `onto_embed`, `onto_search`, `onto_similarity`

Removed tools are not advertised via `tools/list` and cannot be invoked
via `tools/call`.

## New tools added by this feature

| Tool | Description |
| ---- | ----------- |
| `onto_unload` | Drop the active ontology from memory (cache file kept by default). |
| `onto_recompile` | Re-parse the active ontology's source file, ignoring the cache. |
| `onto_cache_status` | Inspect the registry: active entry, all cache rows, and config. |
