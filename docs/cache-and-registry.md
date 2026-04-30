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
| `onto_unload` | Unload from memory. With `name`: targets that named entry (clears in-memory store if it is the active slot). Without `name`: operates on the active ontology. `delete_cache=true` also removes the on-disk file. |
| `onto_recompile` | Re-parse the source. With `name`: rebuilds that cached entry — if it is not the active slot, the in-memory store is left untouched (safe background refresh). Without `name`: recompiles the active ontology and reloads it. |
| `onto_cache_status` | Active slot, all cache rows, and effective config. |
| `onto_cache_list` | Lighter alternative to `onto_cache_status` — returns just the array of cached ontologies with metadata and `is_active`/`in_memory` flags. |
| `onto_cache_remove` | Remove a cached ontology by name. If it is the active slot, the in-memory store is unloaded first. By default the on-disk N-Triples file is also deleted; pass `delete_file=false` to keep it. |
| `onto_repo_list` | List RDF/OWL files in the configured `[general] ontology_dirs` directories. Returns `path`, `name`, `size`, `mtime`, `is_cached`, `is_active` for each entry. Optional `dir` (must be inside a configured repo), `recursive`, `glob`, `limit`, `offset`. |
| `onto_repo_load` | Load an ontology from a configured repo by bare name (file stem), relative path, or absolute path inside a repo. Reuses the same compile-cache / TTL-eviction path as `onto_load`. |

## Ontology repository directories

In addition to single-file `onto_load`, the server can be pointed at one or
more host directories that act as on-disk repositories of ontologies. This
is the recommended pattern for containerized deployments: mount a host
folder of `.ttl` files and the server enumerates them on demand.

```toml
[general]
data_dir = "~/.open-ontologies"
# Either name works; `data_dirs` is accepted as an alias.
ontology_dirs = ["./ttl_data", "/srv/ontologies"]
```

The `OPEN_ONTOLOGIES_ONTOLOGY_DIRS` environment variable overrides the
config (`:` separated on Unix, `;` on Windows; either accepted on both):

```sh
OPEN_ONTOLOGIES_ONTOLOGY_DIRS=/srv/ontologies:/data/extra open-ontologies serve
```

Use the new MCP tools to interact with the repo:

- `onto_repo_list` returns the full catalogue with `is_cached` / `is_active`
  flags so a client can show which entries are already compiled.
- `onto_repo_load` resolves a bare stem (e.g. `pizza`), a relative path
  (e.g. `domain/pizza.ttl`), or an absolute path that must lie inside one
  of the configured repos. Paths outside the configured directories are
  rejected (path-traversal guard).


### Managing multiple cached ontologies

Although the registry holds a single *active* slot at a time, the on-disk
compile cache is keyed by `name` and supports many entries. The combination
of `onto_load`, `onto_cache_list`, `onto_cache_remove`, and per-name
`onto_recompile`/`onto_unload` lets operators:

- maintain a set of pre-compiled N-Triples caches for many ontologies,
- swap which one is currently active by calling `onto_load` with the
  matching path (cache will be reused since the source is unchanged),
- refresh background ontologies' caches *without* disturbing the active
  in-memory ontology by calling `onto_recompile { name: "other" }`,
- clean up obsolete entries with `onto_cache_remove`.
