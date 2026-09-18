# The concurrent state in this codebase, measured

Every piece of shared mutable state in the Rust engine, where it lives, who touches it
concurrently, and whether anything can go wrong. It was written to answer a narrower question
(whether Iris has any application here, which is
[decision 0012](decisions/0012-concurrency-lives-below-the-certificate.md)), and it outlived that
question, because two of the things it found are defects and one of them is live.

The method was to read the code rather than the design notes. Every claim below carries a file and
a line. Counts are as of commit `32029c5`.

## Is this program actually concurrent?

Yes, and more so than the stdio transport suggests.

`rmcp` spawns every incoming JSON-RPC request as its own tokio task
(`rmcp-1.4.0/src/service.rs:963`, `spawn_service_task(async move { service.handle_request(...) })`,
where `spawn_service_task` is `tokio::spawn` without the `local` feature, which this crate does not
enable). Tool calls therefore run concurrently on a multi-threaded runtime **in stdio mode too**,
not only over HTTP. No tool method in `src/server.rs` takes a lock at its entry, so any two tools
can be inside the engine at the same time.

`serve-http` adds a second and stronger form of it. One `Arc<GraphStore>` is shared across every
MCP session and every REST route (`src/main.rs:1932`, `src/main.rs:1941-1947`), and axum serves
those requests concurrently.

So "the MCP server is async" understates the position. The server is genuinely parallel, over a
store every request can write to.

## The inventory

### 1. Process-global configuration atomics

`src/runtime.rs:44-61`. Fourteen atomics and one `RwLock<Vec<String>>`, all `static`, all written by
`init_from_config` (`src/runtime.rs:66`) and read by accessors on every use.

| global | line | written by | read by |
|---|---|---|---|
| `TABLEAUX_MAX_DEPTH`, `TABLEAUX_MAX_NODES` | 44, 45 | `init_from_config` | `src/tableaux.rs:1728, 1729, 1814, 3313, 3314` |
| `TABLEAUX_TEST_TIMEOUT_MS` | 46 | `init_from_config`, `set_tableaux_test_timeout_ms:154` | `src/tableaux.rs` phase deadlines |
| `CLASSIFY_TIMEOUT_MS` | 48 | `init_from_config`, `set_classify_timeout_ms:192` | `classify_timeout_ms():185` |
| `REASONER_MAX_ITER` | 49 | `init_from_config`, `set_reasoner_max_iterations:206` | `src/reason.rs` |
| `CACHE_HASH_PREFIX` | 50 | `init_from_config` | `src/cache.rs:64` |
| `FB_SUPPRESS`, `FB_DOWNGRADE` | 51, 52 | `init_from_config` | `src/feedback.rs` |
| `REPO_LIST_LIMIT` | 53 | `init_from_config` | `src/server.rs:326` |
| `IMPORTS_MAX_DEPTH`, `IMPORTS_TIMEOUT`, `IMPORTS_FOLLOW_REMOTE` | 54-56 | `init_from_config` | `src/server.rs:648, 650` |
| `WEBHOOK_TIMEOUT` | 57 | `init_from_config` | `src/webhook.rs` |
| `PREFERRED_LANGUAGES` | 61 | `apply_language:78` | `preferred_languages():221` |

Every one of them uses `Ordering::Relaxed`, and that is correct rather than sloppy. The fourteen
values are independent scalars. No reader derives an invariant from two of them jointly, and no
write publishes a data structure whose initialisation a reader must not observe out of order.
Relaxed is the right ordering for a knob, and there is no happens-before edge for anyone to prove.

In production the write happens once, at startup (`src/main.rs:1702`, `1801`, `2160`), before any
request is served. The interesting case is therefore tests, which is where this inventory started.

**The test-side hazard is real in principle and handled in practice.** `cargo test` runs each
integration file as its own binary and runs the tests within a binary on several threads, so a test
that stores into one of these globals is mutating state its siblings read. Every live instance is
already guarded, by three different mechanisms, and the guard is a different one each time:

- `tests/reasoner_global_budget_test.rs` stores into `CLASSIFY_TIMEOUT_MS` and
  `TABLEAUX_TEST_TIMEOUT_MS` nine times (lines 97, 98, 131, 132, 148, 149, 162, 163, 178, 179, 267,
  268) and contains exactly one `#[test]` (line 90). Nothing races because nothing else is running.
- `tests/owl2_conformance_test.rs:37-40` stores `0` into both clocks from a helper every one of its
  21 tests calls, and never restores. Idempotent, so concurrent stores of the same value are benign,
  and the comment at line 35 says so.
- `tests/closure_diff_test.rs`, `tests/lean_projection_entailment_test.rs` and
  `tests/gate_demonstration_test.rs` mutate `REASONER_MAX_ITER` and each carry a `serial()` helper
  over a file-static `Mutex` (`closure_diff_test.rs:24-27`, `lean_projection_entailment_test.rs:59`,
  `gate_demonstration_test.rs:28-31`). Coverage is total rather than partial: all 14 `#[test]`
  functions in `closure_diff_test.rs` take the lock, and all 17 in
  `lean_projection_entailment_test.rs` do.

The one unguarded mutation left is the unit test in `src/runtime.rs:239-264`, which stores 250 into
`TABLEAUX_MAX_DEPTH`, 128 KiB into `CACHE_HASH_PREFIX` and `false` into `IMPORTS_FOLLOW_REMOTE`, in
the lib test binary, with no lock. Its own comment (lines 232-237) explains that the two assertions
were merged into one test for exactly this reason, which handles the race against itself and not
the race against the rest of the binary. The readers in that binary are `src/cache.rs:64` and
`src/tableaux.rs:1728`. It is unobservable today, for a reason that is luck rather than design:
`src/cache.rs:65` computes `vec![0u8; prefix_len.min(size as usize)]`, and every fixture the
`cache.rs` unit tests hash is far smaller than 64 KiB, so the prefix length is never the binding
term and moving it from 64 KiB to 128 KiB changes no digest. A fixture larger than 64 KiB would
make this flaky. It is a latent hazard, not a live defect, and the cheap fix is the `serial()`
pattern the three integration files already use.

`src/config.rs:340-346` is worth reading next to this. It records the same class of hazard for
environment variables, notes that `set_var` is `unsafe` in edition 2024 precisely because the
environment is unsynchronised shared mutable state, and resolves it by splitting a pure
`resolve_storage_mode_from` out of the reader. `src/embed_fingerprint.rs:176-221` resolves the same
hazard the other way, with an `ENV_LOCK` taken before the save so a sibling cannot have its scratch
state restored as if it were the guard's. The repository already understands this category.

### 2. The shared graph store

`src/graph.rs:81-83`. `GraphStore` holds an `oxigraph::store::Store` directly, not behind a lock,
and the comment at lines 74-80 is explicit that this is deliberate: Oxigraph's `Store` is
`Send + Sync` and synchronises internally, so an added mutex bought nothing but serialisation of
non-conflicting reads and fourteen poisoning sites.

This is the most important line in the inventory for the Iris question. **All synchronisation of the
one genuinely shared, genuinely mutable object in this program happens inside a dependency.** The
Rust here holds `&self` and calls `store.insert`, `store.clear`, `store.update`, `store.iter`.

Two consequences follow, and they point in opposite directions:

- There is no ordering or ownership property in *our* code to prove, because we never wrote one.
- There is no isolation either. `src/graph.rs:630-637` (`load_lines`) inserts quad by quad in a
  loop with no transaction, so a concurrent reader observes a partially loaded graph; and
  `src/graph.rs:607-611` (`clear`) can empty the store under an in-flight query.

The `Arc<GraphStore>` is cloned into: the per-session server (`src/server.rs:26`), every registry
(`src/registry.rs:64`), the seven REST handlers (`src/main.rs:1941-1947`), and the batch runner.

### 3. The ontology registry

`src/registry.rs`. Four locks, and the only hand-written lock protocol in the codebase:
`active: Mutex<Option<ActiveEntry>>` (line 69), `reload_lock: Mutex<()>` (line 71),
`last_access: Mutex<Instant>` (line 56) and `evicted: Mutex<bool>` (line 59), the last two nested
inside the first.

The protocol is not uniform, and the gaps are where the defects are.

- `load_file` calls `self.graph.clear()` at line 128 and loads at 134 or 139, and does not take
  `active` until line 158 or `reload_lock` at all.
- `ensure_loaded` takes `reload_lock` first (line 190), snapshots the fields it needs under `active`
  (lines 199-212), then releases `active` before touching the graph.
- `evictor_tick` takes `active` (line 283) and holds it across the serialise and the clear (lines
  298-313). It never takes `reload_lock`.
- `unload`, `unload_named`, `recompile_named`, `list_cached` and `status` take `active` only.

So `load_file` and `evictor_tick` can both be inside `self.graph` at once, and neither excludes the
other. See defect B below.

### 4. Parallel classification

`src/tableaux.rs:2648`, `2712` and `src/claimcheck.rs:554`. Three rayon `par_iter` fan-outs.

All three are read-only over immutable data, and this is worth stating precisely because the brief
suspected otherwise. `DlReasoner` contains no `Mutex`, `RwLock`, `RefCell` or `Cell`: the grep for
interior mutability across `src/` returns no hit in `tableaux.rs` at all. `decide_satisfiable_within`
and `decide_subsumption_within` take `&self` and allocate their own tableau. Results are returned by
value and `collect`ed. `CompiledOntology::check_batch` fans out over an `Arc<Index>` that is built
once and never mutated (`src/claimcheck.rs:308-310` says so, and it is true).

The single piece of cross-thread mutable state in the parallel phases is
`subsumption_cut_short: AtomicBool` at `src/tableaux.rs:2707`, a monotone latch that is only ever
stored `true` (line 2716) and is read after the join. A one-way flag needs no ordering argument.

This is embarrassingly parallel work over frozen data. There is no invariant here that a type system
does not already enforce.

### 5. Lazy index with double-checked locking

`src/claimcheck.rs:339-351`. `CompiledOntology::index()` reads under `RwLock::read`, drops, takes
`RwLock::write`, re-checks, builds, stores. Textbook double-checked locking, correct because the
re-check is inside the write lock and because `Arc<Index>` is what escapes rather than a reference
into the lock.

There is a benign inefficiency rather than a bug: `invalidate()` (line 336) can run between another
thread's build and its store, so a stale index can be published and the next reader rebuilds. The
index is a cache of a pure function of `raw`, so a stale read is a lost invalidation, not a wrong
answer, and `known_compatible` is deliberately kept outside the index (lines 313-315) so tier-2
learning does not force a rebuild.

### 6. The rest

- `src/state.rs:424`, `conn: Arc<Mutex<Connection>>`. One SQLite connection behind one mutex, with
  WAL on (`src/state.rs:428`) and a busy retry (`is_busy:412`, backoff at `src/state.rs:398`).
  Coarse, and correct by coarseness.
- `src/align.rs:13`, `src/server.rs:37`, `vecstore: Arc<Mutex<VecStore>>`. Same shape.
- `src/registry.rs:526`, `spawn_evictor`, a detached tokio task. `src/monitor.rs:261`, the same.
- `src/vecstore.rs:585, 622`, `src/sqlsource.rs:230`, `src/server.rs:2590`, `src/main.rs:3219`:
  `spawn_blocking` for CPU-bound or blocking work. Owned data moved in, nothing shared.
- `src/main.rs:1259`, `JSON_MODE: OnceLock<bool>`, set once at `src/main.rs:1930` and read
  thereafter. Write-once is the one shape that needs no argument at all.
- `src/cache.rs:353` `TMP_SEQ: AtomicU64` and `src/lineage.rs:90` `COUNTER: AtomicU64`, both
  `fetch_add` unique-name counters. Relaxed is right; only uniqueness is wanted, not order.
- `src/main.rs:1548`, an explicit 8 MiB thread for the tokio root because Windows gives the main
  thread 1 MiB. Not concurrency, stack sizing.

## The defects

### Defect A: in `serve-http` and `daemon` mode the evictor can never evict. LIVE.

`src/main.rs:1913-1921` constructs a registry solely to drive eviction:

    let shared_registry = Arc::new(OntologyRegistry::new(shared_graph.clone(), evictor_db, ...)?);
    let _evictor = spawn_evictor(shared_registry);

`OntologyRegistry::new` sets `active: Mutex::new(None)` (`src/registry.rs:82`). Nothing ever calls
`load_file` on `shared_registry`: the binding is created inside a block, moved into `spawn_evictor`,
and the block ends at line 1921. The per-session registries that `load_file` does populate are
different objects, constructed by the service factory at `src/main.rs:1928-1936`.

`evictor_tick` bails on the third line of its body:

    let Some(entry) = active.as_ref() else { return Ok(false) };    // src/registry.rs:284

So the task at `src/registry.rs:528` wakes every `evictor_interval_secs`, calls `evictor_tick`, gets
`Ok(false)`, and sleeps again, forever. The comment above it (`src/main.rs:1910-1912`) claims "the
shared one drives memory cleanup". It drives nothing.

The consequence is that `[cache] idle_ttl_secs` is silently inert in HTTP and daemon mode. An
operator who sets it to 120, as `src/config.rs:780` shows a config doing, gets no eviction and
unbounded in-memory growth, with no warning and no error. The stdio arm is fine, because there
`spawn_evictor` is given `server.registry()` (`src/main.rs:1764`), the same object `onto_load`
populates.

This is a gate that cannot fail, in a repository whose stated culture is that a gate which cannot
fail is worse than no gate. `tests/registry_evictor_wiring_test.rs` pins it.

The fix is not to point the evictor at the shared graph. Eviction state (`evicted`, at
`src/registry.rs:59`) is per registry, and `ensure_loaded` reloads only when its own registry's flag
is set (`src/registry.rs:243`). Clearing the shared store from a registry no session consults would
leave every session believing its ontology is resident, with no path back. The honest options are to
say plainly that the TTL is not honoured in HTTP mode, or to move the active slot and its eviction
flag to where the store is, which is the shared object. Both are larger than this document.

### Defect B: concurrent `load_file` over a shared store interleaves. LATENT.

`src/registry.rs:128` clears the graph, lines 130 to 155 load into it, and `active` is not taken
until line 158. Under `serve-http` every session's registry wraps the same `Arc<GraphStore>`, and
rmcp runs each request in its own task. Two concurrent `onto_load` calls can interleave as:

    session 1: clear()                  store = {}
    session 2: clear()                  store = {}
    session 1: load A                   store = A
    session 2: load B                   store = A union B
    session 2: active := B, reports |B|
    session 1: active := A, reports |A|

The store then holds the union of two ontologies while both registries report their own triple
count, and `onto_cache_status` (`src/registry.rs:489`) reports `in_memory_triples` from the store,
which agrees with neither. The reasoner would then classify A union B and certify the result against
an `asserted.tsv` drawn from the same union, so the certificate would be internally consistent and
about a graph no user asked for.

The REST surface reaches the same state without any concurrency at all: `/api/load`
(`src/main.rs:1981-1992`) calls `g.load_file(&path)` directly on the shared graph, with no clear and
no registry update, so it appends into a store a session's registry believes holds only its own
ontology.

Marked latent rather than live only because defect A means the evictor is not also in there, and
because the multi-session HTTP path is not the common deployment. The missing lock is real.

### Defect C: there is no read lease, so a long tool can be evicted under itself. LATENT.

`touch()` (`src/registry.rs:179`) stamps `last_access` when a tool STARTS. `ensure_loaded` releases
`active` at line 212 and returns, and the caller then uses the graph with nothing held. The TTL clock
runs during the tool's execution. A run longer than `idle_ttl_secs` can therefore have
`evictor_tick` clear the store under it at `src/registry.rs:312`.

The default `classify_timeout_ms` is 180 000 (`src/runtime.rs:30`), so a classification is allowed to
run for three minutes; a TTL below that is an ordinary setting. The mitigation is accidental:
`idle_ttl_secs` defaults to 0 (`src/config.rs:282`), which disables eviction, and in HTTP mode defect
A disables it again.

### Defect D: the store can drift between hashing and reasoning. KNOWN, DOCUMENTED, OPEN.

`Reasoner::run_horn` takes `&Arc<GraphStore>` (`src/reason.rs:2915`) and reads the store at line 2935.
This is already
[decision 0010](decisions/0010-the-input-is-a-value-and-not-a-store.md), which names it, calls the
window a race in as many words, and specifies `CertifiedInput` to dissolve it by copying once. It is
accepted and not yet implemented. Recording it here so the four sit in one list.

## What would have caught these

None of A, B or C needs a program logic, and it is worth being concrete about what each does need,
because the answer is the same cheap answer three times.

A is not a race. The code is perfectly synchronised and does nothing; a two-line test that calls
`evictor_tick` on a registry nothing loaded into catches it, and now does.

B and C are missing critical sections. `loom` explores every interleaving of `std::sync` primitives
exhaustively and is the standard tool for exactly this; a two-thread test with a barrier finds B in
an afternoon without it. Neither needs a proof, because the property being violated is not subtle:
two writers are inside one object with nothing between them.

D is the one with real depth, and decision 0010 already solves it by removing the shared state
rather than by reasoning about it.
