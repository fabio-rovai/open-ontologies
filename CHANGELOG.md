# Changelog

All notable changes to Open Ontologies are documented here.

## [Unreleased]

### Fixed
- **JSON-LD is read and written like any other serialisation.** `oxrdfio` has
  supported it all along, but the engine's format handling did not: `parse_format`
  had no JSON-LD arm, and `detect_format` fell back to Turtle for any extension it
  did not recognise, so a `.jsonld` document was handed to the Turtle parser and
  died on `{ is not a valid predicate` — an error that names the wrong problem and
  sends you looking at a file that was never broken. `.jsonld` and `.json` now map
  to JSON-LD, `parse_format` accepts `jsonld`, `json-ld` and `json` (`json-ld` is
  the W3C media-type spelling and what most other tooling takes, so rejecting it
  turned a correct format name into an error), and the body sniffer recognises a
  JSON-LD document published under a misleading extension. The sniff requires an
  opening `{` or `[` *and* a `"@context"`, `"@id"` or `"@graph"` keyword: a bare
  brace proves nothing, since TriG opens its default graph block with `{` and
  Turtle admits `[` as a blank-node subject. `tests/graph_jsonld_test.rs`.
  The Python package had the mirror defect, accepting `jsonld` while rejecting
  `json-ld`; both spellings and `json` now resolve.

- **The compile cache no longer flattens named graphs** (issue #112). It was
  written as N-Triples, a format that cannot carry a graph name, so the cached
  artefact was not equivalent to the source it stood for: a dataset went in and
  a flattened graph came out. The first load parsed the source and answered
  correctly; every load after it read the cache back and returned a store with
  no named graphs at all. Measured on an unchanged TriG file, twice through
  `onto_load`: `origin: "source"` gave two named graphs, `origin: "cache"` gave
  none, and `onto_temporal_snapshot` reported `{"ok": true, "in_scope": []}` —
  a clean, confident, empty answer.

  The freshness key `(source_path, mtime, size, sha256)` was never at fault and
  is unchanged; the cache was correctly judged fresh, and what it held was
  wrong. A second path needed no second load at all: an idle-evicted ontology
  reloads through `ensure_loaded`, from that same flattened file, so a
  long-running `serve-http --idle-ttl-secs` lost its named graphs by being
  idle.

  Cache files are now N-Quads (`.nq`) — line-based and just as fast to parse,
  which was the reason N-Triples was chosen, but able to name a graph. The
  extension doubles as the format marker: an entry pointing at a `.nt` file was
  written before this fix, holds a flattened dataset, and is recompiled instead
  of read. That costs one re-parse per ontology, once, and heals warm caches
  rather than leaving them quietly wrong. Anything whose meaning lives in the
  graph name is affected — the bi-temporal tools above all, which read
  `GRAPH ?g` and saw an empty store. Three tests in `tests/registry_test.rs`,
  each loading TWICE on purpose: a test that loads once passes on the broken
  code, which is why this went unnoticed.

- **Named graphs are preserved when exporting to TriG and N-Quads.**
  `GraphStore::serialize` rendered every format with `serialize_triple`, which
  flattens a quad from a named graph into the default graph. Import and the
  persistent store keep graph names, so the loss was export-only — but it meant
  a TriG save/reload round trip dropped the named-graph structure that
  bi-temporal assertions live in (issue #95): every `validFrom`/`validTo`
  binding on `onto_save`/`onto_convert` output was silently gone. The dataset
  formats now serialize quads; the triple formats (Turtle, N-Triples, RDF/XML)
  keep flattening, which is the only thing they can represent. TriG and N-Quads
  round-trip tests in `tests/graph_named_graph_roundtrip_test.rs`.

### Added
- **Optional eviction of float32 text vectors from memory**
  (`VecStore::with_text_vectors_evicted`, `turbovec` feature). Without it the
  TurboQuant backend's compression is a smaller SQLite blob rather than less
  RAM, because the 4 bit codes and the float32 vectors they were made from are
  both resident. In eviction mode the float32 lives only in the `embeddings`
  table: upserts write through to the row before touching memory, removals
  delete it, and reads load on demand. Three new accessors carry every path
  that used to reach into the entry map directly: `fetch_text_vecs` pulls just
  the shortlist for the exact re-score (the hot path, one query rather than one
  round trip per candidate), `all_text_vecs` streams the whole set IRI-sorted
  for the paths that genuinely need every vector, and the public
  `load_text_vec` returns one owned vector in either mode. New
  `resident_text_vector_bytes()` reports what is still held.

  `entries_fingerprint` is now public and reads through `all_text_vecs`, so an
  evicted store and a resident one over the same database hash the same stream
  and can share persisted index caches; a test asserts that equality.
  `get_text_vec` still returns a borrowed slice and therefore returns `None`
  under eviction, which is why `align.rs` and `onto_compare` were moved to
  `load_text_vec`: left alone they would have silently scored every pair at
  0.0. Eviction is only coherent with the TurboQuant backend, since an
  `instant-distance` graph holds its own float32 copy of every point. 8 tests
  in `tests/vecstore_eviction_test.rs`, each asserting an evicted store returns
  exactly what a resident one built from identical data returns, plus an
  `#[ignore]`d measurement of what the mode costs per query.

  Measured at 20,000 vectors x 384 dims on an M3 Max: 30.7 MB of resident
  float32 goes to zero, `search_cosine_turbo` costs about 75 us more per query
  (312 us to 387 us) for the shortlist fetch, and the exact `search_cosine`
  scan costs 2.4x more (16.4 ms to 39.4 ms) because it reads every row. Evict
  when the workload queries through the turbo path; do not evict when it leans
  on the exact scan.

### Fixed
- **`search_cosine` and `search_product` no longer clone the entire corpus per
  query.** Routing them through the new whole-set accessor initially copied
  every float32 vector on every call, which cost 20 ms per query at 20,000 x
  384 and made a resident store measurably slower than an evicted one. The
  accessor now yields `Cow` and borrows when the vectors are resident: the
  exact scan went from 36.6 ms to 16.4 ms. Caught by the eviction measurement,
  not by the tests, which only assert results.
- **TurboQuant cosine index backend** (new optional `turbovec` feature, off by
  default, implies `embeddings`). A second index backend for the text half of
  the vector store, built on Google Research's TurboQuant quantiser
  (arXiv:2504.19874) via the `turbovec` crate, alongside the existing
  `instant-distance` HNSW graph. New `src/turbo_index.rs` with
  `TurboCosineIndex` (`build`, `upsert`, `remove`, `search`, `search_within`,
  `to_bytes`, `from_bytes`), and three new `VecStore` entry points
  (`search_cosine_turbo`, `persist_turbo_index`, `load_turbo_index`) plus the
  `turbo_index_len` accessor.

  The motivation is mutation cost, not compression. An `instant-distance`
  graph is immutable, so every `VecStore::upsert` sets `cosine_index = None`
  and the next search pays a full rebuild; an ontology whose embeddings arrive
  incrementally pays that rebuild once per class. TurboQuant has no training
  phase and no graph, so an insert is an append and a removal is O(1), and
  `upsert`/`remove` now maintain the live index rather than dropping it.

  The quantised index is a candidate generator, never the answer.
  `search_cosine_turbo` pulls a shortlist four times wider than the request
  (floor of `top_k + 32`), re-scores every candidate against the float32
  vector the store already holds, and returns that, so no approximate
  similarity number reaches a caller and the result is identical to the exact
  brute-force scan whenever the shortlist covers the true top-k. That identity
  is asserted directly against `search_cosine` in the tests rather than
  assumed.

  Scope limits, both deliberate. (1) `PoincareIndex` is untouched and stays on
  `instant-distance`: TurboQuant scores inner products, and hyperbolic
  distance is not an inner product on the ambient coordinates. (2) The store
  still holds float32 vectors in memory, because the exact re-score and
  `search_cosine` need them, so this change does not yet realise TurboQuant's
  memory win. Evicting float32 to SQLite with load-on-demand for the re-score
  is the follow-on.

  Implementation notes: vectors are zero-padded to the next multiple of 8
  (`turbovec` requires `dim % 8 == 0`; zero padding leaves every inner product
  unchanged). IRIs map to `u64` ids that are never recycled, so a stale
  allowlist entry naming a removed id fails loudly rather than resolving to
  whatever vector took its place; the id counter is serialised with the index
  so a reload cannot restart allocation at 0. A query is validated before it
  reaches the kernel: `turbovec`'s allowlist-free `search` is the panicking
  form, so a non-finite coordinate from a misbehaving embedding provider, or a
  query whose dimensionality disagrees with the index, is reported as no
  results rather than panicking the server or being silently truncated into a
  plausible-looking ranking against the wrong vector. Persistence reuses the existing
  `hnsw_index_cache` table under `kind = 'turbo_cosine'` with no schema
  change, and both load guards are unchanged: `model_fp` rejects an index
  built under a different embedding configuration, `entries_hash` rejects one
  whose entry set has moved on. 17 new tests in `tests/turbovec_index_test.rs`
  covering top-1 agreement with the exact scan, incremental add/replace/remove,
  byte round-trip, id allocation after a reload, allowlist search (including
  unknown and empty allowlists), sub-8 dimensionality padding, non-finite and
  wrong-width query rejection, the VecStore score-identity guarantee, index
  warmth across mutations, SQLite round-trip and stale-cache rejection, plus an
  `#[ignore]`d measurement against HNSW.

  Measured at 10,000 vectors x 768 dims on an M3 Max (`docs/embeddings.md` has
  the tables): build 116 s vs 178 ms, one added embedding 137 s vs 52 us, query
  4.7 ms vs 0.25 ms, serialised index 34.1 MB vs 5.2 MB against 30.7 MB of raw
  float32.

  The recall picture is worth stating carefully, because the first measurement
  overstated it. On that synthetic corpus recall@10 is 91.6% for HNSW and 100%
  for the re-scored TurboQuant shortlist, and the control shows the gap is
  structural rather than a matter of shortlist width: giving HNSW 40 candidates
  instead of 10 leaves it at exactly 91.6%, because the entries it misses are
  ones the graph walk never reaches. But isotropic random vectors are close to
  the worst case for a graph index, and on a real corpus the gap all but
  vanishes. `measure_recall_on_a_real_corpus` embeds 10,000 real ontology
  labels with the shipped local MiniLM model over two contrasting real corpora,
  a topical taxonomy (mean pairwise cosine 0.25) and a set of real-world entity
  names (0.34), against ~0 for the synthetic corpus. HNSW scores 99.9% and
  99.8% there, against 100% for TurboQuant. So recall is not a reason to switch backends; mutation cost and
  index size are.
- **Four extension surfaces** (ECOSYSTEM.md maps them). (1) **Community
  marketplace packs**: `onto_marketplace` now merges an open runtime-fetched
  registry (`community/registry.json`, override with
  `OPEN_ONTOLOGIES_COMMUNITY_REGISTRY`, `community=false` to skip) with the
  curated catalogue — entries are tagged `"source": "curated"|"community"`,
  curated IDs always shadow community IDs, and the shipped registry is
  validated in CI. Seeded with the Manchester/Stanford Pizza teaching
  ontology. (2) **Community skills**: `skills/community/` with a template —
  zero-code markdown workflow recipes. (3) **Companion servers**: the
  five-rule contract (`docs/companion-servers.md`) naming the compose-over-MCP
  pattern OpenCheir already uses (no embedded LLM, no `onto_*` squatting,
  packs/files as interchange, lineage webhook, graceful degradation).
  (4) **WASM plugins** (`--features plugins`): sandboxed community tools via
  the pure-Rust wasmi interpreter — `onto_plugin_list` / `onto_plugin_call`,
  ABI v1 (no host imports, no IO, fuel-metered, fresh instance per call,
  16 MB return cap), graph access only by caller-passed `sparql` whose rows
  are injected as `bindings`. Reference plugin in
  `examples/plugins/label-case-lint`; ABI exercised by WAT-built plugins in
  `tests/plugin_host_test.rs` (including fuel-exhaustion and oversized-return
  guards). Docs: `docs/plugins.md`.
- **fenic support.** [fenic](https://github.com/typedef-ai/fenic) (typedef-ai's
  semantic DataFrame framework) keeps its local catalog in a plain DuckDB file
  with user tables under the `typedef_default` schema. The DuckDB schema
  introspector now scans all user schemas instead of only `main` (excluding
  `information_schema`, `pg_catalog`, `__`-prefixed internals, and fenic's
  `fenic_system` telemetry schema), so `import-schema` / `onto_import_schema`
  work directly against fenic catalogs; cross-schema table-name collisions are
  disambiguated as `<schema>_<table>`. `open-ontologies-lite` gains a
  duck-typed dataframe bridge — `rows_from_dataframe`, `rows_to_turtle`, and
  `OntologyEngine.load_rows` accept fenic, polars, pandas, and pyarrow objects
  with no new dependencies. New end-to-end example
  `python/examples/fenic_pipeline.py` and a "fenic" section in
  `docs/data-pipeline.md`.

## [1.2.0] - 2026-08-15

### Added
- **`onto_pack` / `onto_unpack`.** Portable verified knowledge artifacts: sorted
  N-Triples plus a manifest (name, version, counts, timestamp, tool version,
  sha256) and the lint/enforce results recorded at pack time. What you promote
  between environments is a graph that has already passed its checks, with the
  evidence attached. `onto_unpack` refuses a pack whose checksum does not match.
- **Bi-temporal facts.** `onto_temporal_snapshot`, `onto_temporal_query` and
  `onto_temporal_conflicts` separate two independent clocks: `valid_at` asks what
  was true then, `as_of` asks what was known then. A disjointness violation only
  counts as a conflict when the two assertions claim overlapping validity, so
  superseded history stops being reported as contradiction. Graphs without
  validity metadata are timeless and always in scope, making the vocabulary
  additive to an existing store.
- **`onto_reason_incremental`.** Derives the consequences of newly added triples
  by joining the delta against the existing closure, so the cost tracks what
  changed rather than the size of the store. Schema axioms are refused with an
  explanation, because those change what the whole store entails.
- **Claim support checking.** `onto_support_check`, `onto_support_verdict` and
  `onto_support_report` add the second axis beside conformance: conformance asks
  whether a claim is expressible, support asks whether it is true to its source,
  and a claim can fail either independently.
- **`onto_communities`.** Deterministic modularity clustering that returns a
  skeleton per community (size, top members by degree, internal relations,
  bridges) so corpus-wide questions can be answered from reports instead of
  traversal from an anchor entity.
- **`onto_ossie_import`.** Compiles Apache Ossie (incubating) ontology documents
  to OWL 2 DL plus SHACL, making a vendor semantic model reasonable and
  validatable. The four constructs OWL 2 DL cannot express are reported and
  preserved as annotations rather than silently dropped.
- **SHACL `sh:inversePath` and `sh:severity`.**
- **Unauthenticated `/health` liveness route on `serve-http`.** Registered
  outside the bearer layer on purpose, so a probe does not need credentials
  while `/api` and `/mcp` stay behind them. The body is limited to status and
  version: an unauthenticated endpoint should not describe loaded state.
- **Optional PROV-O provenance emission on ingest.**
- **Embedding fingerprints.** Each vector records the configuration that
  produced it, including the tokenizer and a hash of the whole model file, so a
  changed embedder invalidates rather than silently mixes vector spaces.
- **Build-time modelling buffer, phase 1.**

### Changed
- **Loads are all-or-nothing.** A failed load no longer leaves a partially
  populated store.
- **`enforce` gained a competing-modelling-pattern rule.**

### Fixed
- **`onto_load` did not set the base IRI from the file path**, so relative IRIs
  resolved against the wrong base.
- **`serve-http` and `serve-unix` shutdown.** The cancellation token is now
  cancelled on ctrl-c and SIGTERM, the server keeps listening so a second signal
  can force the exit, and `serve-unix` unlinks its socket.
- **`plan` / `apply`.** Apply works against the ABox and stops fabricating
  bridges; plans are scoped to their owner and Windows paths are tokenised.
- **Alignment claim strength now tracks evidence strength.**

### Fixed
- **SQLite migrations discarded every error and tracked no schema version.**
  `StateDb::open` upgraded old databases with two `let _ = conn.execute_batch(...)`
  calls. Discarding the result swallowed the expected "duplicate column name" on
  an already-upgraded database, but it swallowed a locked file, an I/O error and
  a partially-applied batch with it, and `open` returned `Ok` regardless.

  `PRAGMA user_version` now records the schema version, and each migration
  applies inside a transaction that commits the DDL and the version bump
  together. Columns are checked individually with `PRAGMA table_info` before
  being added, so only the genuine already-exists case is tolerated and every
  other error propagates.

  Per-column checking also heals the state the old code could produce and not
  detect. `execute_batch` stops at its first error, so a database whose first
  `ALTER` committed and whose second did not was left with `webhook_url` and no
  `webhook_headers` — and re-running the pair would fail on the column that was
  already there, permanently. Verified against real rusqlite semantics, not
  assumed: the old idiom leaves the column missing and still returns `Ok`.

  Databases predating the tracker all report `user_version = 0` whether they
  carry the columns or not, which is why the version alone cannot decide what to
  do and the column probe is the thing that establishes truth.

  `tests/state_migration_test.rs` covers this against hand-built binary fixtures
  in `tests/fixtures/state/`: a pre-migration database, and a half-applied one.
  The fixtures are committed rather than generated, because a fixture built by
  the code under test would agree with a broken migration by construction.
  Reported in #75.

### Added
- **Versioned SQL type → XSD datatype contract.** `docs/data-pipeline.md` now
  documents what `SchemaIntrospector::sql_to_xsd` produces for every SQL type it
  recognises, alongside the declarative `datatype` mapping field it is easy to
  confuse it with. The table is marked v1 and any row that changes gets a
  CHANGELOG entry, so a `schema.rs` refactor can no longer alter the shape of
  generated ontologies invisibly. Also records the decisions the table encodes
  (parameters stripped, timezone not represented, `xsd:string` catch-all) and
  what the schema import derives beyond the datatype.
- **CI builds, lints and tests the optional features.** A `features` job adds a
  breadth leg (`cargo check` + `cargo clippy --all-targets` at `--all-features`)
  and a depth leg (`cargo test --features postgres,duckdb,embeddings,sql`), with
  `rust-cache` keyed per feature set. `default = []`, so none of `postgres`,
  `duckdb`, `sql`, `embeddings` or `causal-pywhy` — nor the `sqlx`, `duckdb`,
  `tract-onnx`, `tokenizers` and `instant-distance` trees — was compiled by
  anything in CI before this.

  The first run of the breadth leg found `clippy::large_enum_variant` in
  `src/embed.rs`, a lint that could not have fired before because nothing
  compiled `embeddings`.

  `scripts/check-test-collection.sh` fails the run when a test file whose gate
  is enabled collects zero tests. Eight files under `tests/` had never been
  collected once, 56 tests in total; a file that collects nothing reports
  "test result: ok" and is indistinguishable from a passing one. The enabled
  set is derived from the cargo flags the test step used and expanded through
  `[features]`, so `--features sql` counts as postgres + duckdb; files whose
  gate is off in a given leg are skipped, so partial-feature legs stay green.
- **Build provenance and checksums on release binaries.** The release job now
  emits a Sigstore attestation via `actions/attest-build-provenance`, binding
  each published binary to the workflow run and the commit it was built from,
  and publishes a `SHASUMS.txt` alongside the assets. The release job gains
  `id-token: write` and `attestations: write` for the signing identity.

  Verify provenance with:

  ```
  gh attestation verify <binary> --repo fabio-rovai/open-ontologies
  ```

  Or check the checksum of the binary you downloaded. `SHASUMS.txt` lists all
  four platforms, so verify the one you have rather than the whole file:

  ```
  grep open-ontologies-x86_64-unknown-linux-gnu SHASUMS.txt | sha256sum -c -
  # macOS: ... | shasum -a 256 -c -
  ```

  A bare `sha256sum -c SHASUMS.txt` expects all four binaries present and exits
  non-zero when three are missing, which is the normal case. With GNU coreutils
  you can also pass `--ignore-missing`; the form above works on macOS too.
- **Opt-in persistent triple store.** A new `[storage]` section selects the
  backend for the main graph: `mode = "memory"` (default, unchanged behaviour)
  or `mode = "persistent"`, which opens a RocksDB-backed Oxigraph store at
  `<data_dir>/triplestore` so triples survive a restart. Override with
  `OPEN_ONTOLOGIES_STORAGE_MODE` or `--storage-mode` on `serve` / `serve-http`;
  precedence is CLI, then env, then config, then the default. Unknown values
  warn and fall back to `memory` rather than failing.

  The one-shot CLI subcommands read the same setting, so `open-ontologies load
  foo.ttl` followed by `open-ontologies query ...` shares state when
  persistence is on. Sandbox stores elsewhere in the codebase stay in-memory;
  only the main graph is ever persistent.

  Note that Oxigraph permits a single read-write handle per directory, so two
  server processes pointed at the same `data_dir` will fail to open the second
  store. Contributed by Ladislav Gazo (@lgazo).

### Fixed
- **SQL floating-point columns were mapped to `xsd:decimal`.** `real`,
  `float4`, `float`, `float8`, `double` and `double precision` all resolved to
  `xsd:decimal` in `SchemaIntrospector::sql_to_xsd`, so every ontology
  generated by `onto_import_schema` / `onto_sql_ingest` from a table with a
  float column carried a wrong range. `xsd:decimal` is integers over powers of
  ten: it cannot represent `NaN`, `INF` or `-INF`, and it asserts an exactness
  IEEE 754 does not have. `real` and `float4` now map to `xsd:float`, and
  `double`, `double precision` and `float8` to `xsd:double`.

  Bare `float` is dialect-dependent (Postgres `float8`, DuckDB `float4`) and
  maps to `xsd:double`, widening rather than narrowing: every `xsd:float`
  value is exactly representable as an `xsd:double`, so the reverse choice
  would declare a range narrower than the data. `numeric` and `decimal` are
  unaffected and still map to `xsd:decimal`.

  **This changes output.** Ontologies generated before this release state
  `xsd:decimal` on float-backed properties; regenerate them, or expect
  range mismatches against ones generated after. Reported in #76.

## 1.1.1 — 2026-08-03

Correctness and reporting. No new features. Two of these change output, so
results produced with 1.1.0 or earlier are not directly comparable.

### Fixed
- **Tableaux classification was nondeterministic.** `named_classes` is a
  `HashSet` and five expansion sites collected `self.nodes.keys()` unordered, so
  hash iteration order decided which subsumption checks completed inside the
  node and depth budgets. Exhaustion correctly yields `Unknown`, so entailments
  were silently absent rather than wrong. Conformance suite went from 4/6 to
  12/12 consecutive runs passing, and that binary's wall time from 1.17-2.57s to
  0.01s. See `docs/determinism.md`.
- **Alignment was nondeterministic.** `extract_classes` returned
  `into_values()` order and the candidate comparator sorted on confidence alone,
  which is not a total order because the zero-structural-signal branch assigns
  many pairs the identical value. Five runs of the same binary produced five
  different alignments. Now sorted by IRI with ties broken on the IRI pair.
- **Security:** `tract` 0.21 -> 0.22.3 and `time` -> 0.3.55, closing
  RUSTSEC-2026-0217 and RUSTSEC-2026-0009. The 0.21 line could not satisfy both
  advisories simultaneously.
- `run_ablation_no_stable.py` had been failing on a stale source marker and had
  never run to completion.
- `score_condition_d.py --legacy` wrote to the canonical results path,
  overwriting the corrected scores with the ones they exist to refute.

### Changed
- **OAEI Anatomy result corrected to P 0.960 / R 0.730 / F1 0.829** (1,152
  correspondences). The previously recorded 0.832 was one draw from the
  nondeterministic distribution above. Rank in the OAEI 2025 field is unchanged
  at 9th of 13.
- Benchmark reporting now uses the complete OAEI 2025 field including both
  baselines, rather than four selected systems.
- Marketplace statistics regenerated; property counts were stale output from a
  counting bug fixed in March.

### Removed
- **The 1,633x OWL-RL vs HermiT speed claim is withdrawn.** It was measured on
  an empty store and inverts when measured correctly. See
  `benchmark/reasoner/README.md`. No speed claim against a Java reasoner should
  be made from this repository.

### Added
- `docs/determinism.md` recording both defects, with reproduction commands.
- `benchmark/oaei/results/ablation_no_stable.json`, the single-variable
  stable-matching ablation.

## 1.1.0 — 2026-07-27

### Added
- `claimcheck` module: compiled per-claim ontology-consistency verification.
  Token-bitset engine (0.3 µs median per claim, 11M claims/s batched), sound
  two-hop disjointness join with witness extraction, three-valued verdicts
  (`Rejected` / `Undetermined` / `Consistent`), reasoner-backed residual tier
  (`ResidualOracle`) with verdict learn-back, closed-world vocabulary checks,
  and an assumed-disjointness WARN tier for zero-disjointness ontologies.
  Correctness audited against HermiT: 0 disagreements over 78,884 exhaustive
  class pairs (13 ontologies) and 793 adversarial structural claims.
- Offline compile tooling (`benchmark/reasoner/`): `CompileOntology` with six
  sound disjointness-propagation rules (restriction, functional, union,
  data-value, counting, dueling-universal idioms), `DisjointnessMatrix`,
  `ClaimConsistency`, `PairOracle`, `VetDisjointness`, `StripDisjointness`.
- **`onto_vocab_check` — closed-world vocabulary check for generated DATA graphs.** Verifies that every predicate and every `rdf:type` class used in a Turtle data graph is actually **declared** in the loaded ontology, flagging hallucinated/undeclared terms. This is the gate open-world SHACL structurally cannot provide: SHACL silently ignores predicates it has no shape for, so a graph full of invented terms (an LLM emitting `ies:hasDeparturePort` when the ontology only defines `ies:scheduledDeparturePort`) still reports `conforms=true`. Only IRIs whose namespace belongs to the ontology (plus any passed via `namespaces`) are policed — standard `rdf`/`rdfs`/`owl`/`xsd`/`sh` vocabulary and the caller's own instance-data IRIs are never flagged. Returns `{conforms, hallucinated_terms, checked_namespaces, predicates_checked, types_checked, ontology_terms}`. **Vacuous-pass guard:** when no ontology vocabulary is present (0 declared terms and no `namespaces`), the tool returns `conforms=false` with an explanatory `warning` rather than silently passing — a closed-world check with nothing to check against must never green-light. New module `src/vocab_check.rs` (SPARQL over Oxigraph, mirroring the term-existence logic of `onto_shacl_check`) with 3 unit tests (clean conforms / hallucinated predicate caught / instance + standard namespaces never flagged). Exposed as MCP tool `onto_vocab_check` (+ `OntoVocabCheckInput`), CLI subcommand `vocab-check`, and batch command `vocab_check`. Verified end-to-end against the real IES4 ontology (714 terms): it flags the non-ontology terms IES4's *own* published sample data uses — `movement.ttl` → `isScheduledDeparturePort`/`isScheduledArrivalPort`/`seatNumber`, `events.ttl` → `happensIn` — every one of which plain SHACL reports as conforming. Companion to `onto_shacl` (open-world data validation) and `onto_shacl_check` (checks proposed shapes); this checks generated data. Follows the MCP-native validation-primitive convention.

### Changed
- OWL-DL reasoner: satisfiability is now three-valued — resource exhaustion
  reports `Unknown` instead of unsatisfiability; per-test (10s) and global
  classification (180s) wall-clock budgets; pairwise blocking; output gains
  `complete`, `undetermined_classes`, `subsumption_sweep_cut_short`, and
  `abox.undecided`.
- RDF loading content-sniffs `.owl` files, so Turtle published under `.owl`
  parses correctly.

## 1.0.0 — 2026-06-13

### Added
- **Causal (v0.5): `certify_action` × PyWhy integration.** Wires the #48 PyWhy scaffold into the live `certify_action` path. New `ActionFrame.identification_mode` field (`Structural` | `DoCalculusBackdoor`, default `Structural` for back-compat) selects which identifiability proof to attempt. New helper `build_causal_dag(graph, target_iris)` extracts a causal DAG from the loaded RDF graph: nodes = target IRIs + their one-hop structural neighbours (same slice CIVeX already hashes); edges = `rdfs:subClassOf` / `rdfs:subPropertyOf` / `rdfs:domain` / `rdfs:range` triples among slice members (treated as cause → effect); plus a synthetic `__utility__` sink downstream of every node. When `DoCalculusBackdoor` is requested AND the `causal-pywhy` Cargo feature is enabled, the verifier calls `civex_pywhy::run_pywhy_backdoor` and on success stamps the certificate's `identification_proof` with DoWhy's adjustment estimand + tags assumptions with `"do_calculus_backdoor"`. On any failure path (Python unavailable / DoWhy unavailable / DoWhy runtime error / target unidentifiable) the verifier **silently falls back to the structural proxy** and records the reason in assumptions as `"do_calculus_unavailable:<kind>"`. When the feature is off, the marker is `"do_calculus_unavailable:feature_disabled"`. The certificate's `identification_proof` field is never empty regardless of branch taken. MCP tool `onto_certify_action` gains `identification_mode: Option<String>` accepting `"structural"` (default) or `"do_calculus_backdoor"`. Two new integration tests in `tests/civex_test.rs`: `structural_mode_records_structural_only_assumption` (back-compat) and `do_calculus_mode_falls_back_to_structural_when_feature_disabled` (verifies the fallback path doesn't crash and records the marker). All 9 civex integration tests pass in both default and `causal-pywhy` build configurations; clippy clean across `--lib --tests --examples` in both configs.
- **Causal: PyWhy/DoWhy backdoor identification subprocess scaffold (#48).** Scaffolds the Causal flagship's substantive v0.5 work without pulling Python into the default build. New optional Cargo feature `causal-pywhy` (off by default) enables a `src/civex_pywhy.rs` module that wraps DoWhy v0.13 as a subprocess — same pattern as `src/plan_classical.rs` does for Fast Downward. The wrapper embeds a self-contained Python driver (`PYWHY_PYTHON_DRIVER`) that reads `{nodes, edges, treatment, outcome}` from stdin, builds a `networkx` DiGraph, runs DoWhy's `identify_effect` for backdoor adjustment, and emits `{identifiable, adjustment_set, estimand_expression}` to stdout. Per the May 2026 roadmap memo: **Pearl–Shpitser ID is not ported to Rust** — DoWhy is a 15-year-stable Python implementation, so we wrap it. Honest behaviour when Python or DoWhy is missing: structured errors with `kind = "python_unavailable"` / `"pywhy_unavailable"` / `"dowhy_runtime_failed"` that the caller (eventually `certify_action` in v0.5) dispatches on to fall back to the structural proxy. Binary resolution order: explicit `python_override` → `PYTHON_BIN` env var → `python3` on PATH. **NOT yet integrated into `certify_action`** — the structural proxy (`"structural_only"` assumption) remains the only identifier in shipped certificates; integration tracks as the v0.5 ship. 9 unit tests cover parser cases (identifiable, unidentifiable, both error kinds, invalid JSON), python resolver (override + default + env var), embedded-driver sanity check, and `python_unavailable` behaviour on missing binary. Zero additional Rust dependencies — the entire feature is embedded Python + subprocess plumbing.
- **Dynamics: non-deterministic outcomes for ActionSchema (#49).** `ActionSchema` gains an additive `outcomes: Vec<Outcome>` field. Each `Outcome` carries a categorical `probability` in `[0, 1]`, its own `effects: Vec<EffectSpec>` list, and an optional human-readable `label` (e.g. `"success"` / `"degraded"` / `"failure"`). When `outcomes` is non-empty, `apply()` samples one outcome and executes its effects; the deterministic `effects` field is ignored. When empty (the default), the schema behaves exactly as before — full back-compat with v0.4 base. Probabilities are validated: the sum must equal `1.0 ± 1e-6` or `apply()` returns an error; negative probabilities are also rejected. Sampling uses an inline xorshift64 PRNG keyed by a seed — **zero new dependencies**, no `rand`/`fastrand` pulled in. New method `ActionSchema::apply_with_seed(graph, db, bindings, seed)` exposes the seed for reproducible sampling; the default `apply()` derives a seed from `SystemTime::now()`. `ApplyResult` gains `sampled_outcome: Option<usize>` and `sampled_outcome_label: Option<String>` so callers can see which branch fired. `onto_action_apply` gains an optional `seed: Option<u64>` parameter — pass it when you need reproducible runs (CIVeX certification, replay-from-audit-log, controlled experiments). Tests (4 new): `nondeterministic_apply_with_seed_is_reproducible`, `nondeterministic_apply_distribution_matches_probabilities` (1000-call smoke check that a 70/30 split lands within `[0.60, 0.80]`), `nondeterministic_apply_rejects_invalid_probability_sum`, `deterministic_schema_still_works_when_outcomes_is_empty` (back-compat).
- **Planner: `onto_plan_classical` — Fast Downward subprocess wrap (#50).** Optional convenience tool that completes the LLM-Modulo Planner pipeline (`compile_pddl` → `classical` → IRI-bind client-side → `validate`). Per the LLM-Modulo convention, the classical solver is still client-side — this wrapper exists so a caller who *does* have Fast Downward installed locally can ask the server to run it for them rather than shelling out themselves. Honest behaviour when Fast Downward is missing: returns a structured `binary_unavailable` error with installation guidance, never falls back to a silent stub. Binary resolution order: explicit `fast_downward_bin` parameter → `FAST_DOWNWARD_BIN` env var → `fast-downward.py` on PATH. Returns the raw `sas_plan` content (preserved verbatim) plus a parsed `operators: [{name, args}]` list and the `; cost = N (...)` footer extracted separately. Search strategy is configurable (default `"lama-first"`; pass `"astar(lmcut())"` etc.). Reads the highest-numbered `sas_plan.N` variant when satisficing search emits multiple plans. New module `src/plan_classical.rs` with 7 unit tests covering parse cases (three-operator plan with cost footer, blank-line + comment skipping, empty input, zero-arg operators) and resolver / error behaviour (explicit override, default fallback, `binary_unavailable` on missing binary). New MCP tool `onto_plan_classical` + `OntoPlanClassicalInput`.
- **Planner: `onto_plan_validate` — LLM-Modulo validator primitive (#45).** Server-side companion to `onto_plan_compile_pddl`. Per the LLM-Modulo convention (Kambhampati arXiv 2402.01817), the server compiles + validates, the orchestrator solves. The validator takes a candidate plan (an ordered list of `{action_name, bindings}` operator instances — typically produced client-side by Fast Downward, LLM prompting, or any other source) and step-by-step: (a) looks up each step's registered `ActionSchema`, (b) re-evaluates its preconditions against the cumulative sandbox state under the step's bindings, (c) if applicable, executes its effects against the sandbox, (d) if not, returns immediately with the failing step index and a diagnostic. **Critically, the validator forks the loaded graph into an isolated sandbox** so the real store is never mutated — verified by a dedicated test. Multi-step plans correctly chain state through: a test exercises Step 1 establishing the precondition that Step 2 needs (declare `ex:Feline` as a class, then add `ex:Cat rdfs:subClassOf ex:Feline`). Optional `goal_facts` are checked post-plan and reported in `unsatisfied_goals` (without invalidating the plan itself — a well-formed plan that just doesn't reach the goal is still well-formed). Internal scratch `StateDb` opened as `:memory:` so per-step lineage entries don't pollute the production audit trail. New module `src/plan_validate.rs` + 6 unit tests covering empty plan, single-step success, missing action, unsatisfied precondition, multi-step state-chain, and goal-checking semantics. New MCP tool `onto_plan_validate` + `OntoPlanValidateInput` / `PlanStepInput`.
- **Dynamics: ramification via OWL-RL closure after apply (#47).** First follow-on to the Dynamics scaffold. New `ActionSchema::apply_with_ramification(graph, db, bindings, profile)` method that runs the existing `reason::Reasoner` immediately after the literal effects land, materialising downstream entailments into the same graph. `ApplyResult` gains two new fields: `derived_triples_added: usize` (count of new triples the reasoner produced beyond the literal effects) and `ramification_profile: Option<String>` (the profile actually run, or `None` when ramification was skipped). The `onto_action_apply` MCP tool gains a `ramify` parameter accepting any of `"rdfs"` / `"owl-rl"` / `"owl-rl-ext"` / `"owl-dl"`; default `None` preserves the previous literal-effects-only behaviour. Validated against the canonical acceptance case from #47: a schema that adds `?child rdfs:subClassOf ?parent`, applied with `ramify="owl-rl"` over a graph containing `ex:tigger a ex:Cat`, materialises `ex:tigger a ex:Animal` via subClassOf transitivity. Two new unit tests in `src/dynamics.rs`.
- **Dynamics layer scaffold + Planner stub (three-layer architecture, #43 + #45).** First two of the three v0.4–v0.6 layers from the May 2026 KR/UAI/ICAPS/AAMAS roadmap land as additive scaffolding on top of v0.2's primitives — no breaking changes to existing tools. **Dynamics** introduces `ActionSchema` (BC+ deterministic-single-effect subset): typed `Parameter` slots, SPARQL `ASK`/`SELECT` `preconditions` with `{param}` substitution, and KGCL-shaped `effects` (`AddTriple` / `RemoveTriple` / `AddClass`). Schemas persist by name in a new `dynamics_action_schemas` SQLite table; `apply()` runs the effects, emits the KGCL Controlled-Natural-Language patch, mints an IES4-style event IRI, and logs to `lineage`. Four new MCP tools: `onto_action_register` (persist a schema from inline JSON), `onto_action_applicable` (evaluate preconditions against the loaded graph under a binding map), `onto_action_apply` (execute effects + return patch + event IRI; re-checks preconditions by default), `onto_action_list` (enumerate registered schema names). **Causal-layer hookup**: `civex::ActionFrame` gains an optional `action_schema_name`; when set, `onto_certify_action` echoes `dynamics_action_schema:<name>` into the certificate's assumptions, so the audit trail is explicit about which Dynamics action was gated. **Planner stub** (`src/plan_pddl.rs` + `onto_plan_compile_pddl`): emits a PDDL domain from registered action schemas plus a problem instance from the loaded graph and a goal Turtle slice. Single-predicate `(triple ?s ?p ?o)` over typed sort `iri`; ASK-shaped preconditions translate cleanly, SELECT-shaped ones surface in `translation_notes` so the lossy translation is honest. Per the LLM-Modulo convention (Kambhampati arXiv 2402.01817), the actual planner (Fast Downward) is delegated to the orchestrator; this primitive only emits the PDDL. New files: `src/dynamics.rs` (~458 LOC including 7 unit tests), `src/plan_pddl.rs` (~280 LOC including 6 unit tests). Causal extension verified by a new integration test `action_schema_name_is_recorded_in_certificate_assumptions` in `tests/civex_test.rs`. Honest deferrals: ramification rules, non-deterministic dynamics, and concurrent action semantics defer to v0.4.x; OWL → PDDL rigour (Borgwardt KR 2025) defers to v0.6 proper.
- **`onto_certify_action` — CIVeX-style causal certificate for state-changing actions** (#42, [arXiv 2605.09168](https://arxiv.org/abs/2605.09168)). New MCP tool that gates any state-changing onto_* operation before execution. Maps a proposed action to a structural identifiability check + Wilson one-sided LCB on the do-effect, returns one of four auditable verdicts: **EXECUTE / REJECT / EXPERIMENT / ABSTAIN**. Each verdict carries a certificate documenting the labelled assumptions, structural-dependency identification proof, point estimate, LCB at level α, provenance SHA-256, and risk bound. Scaffold port: keeps the four-way verdict + Wilson LCB + locked-IRI hard-reject; uses a **structural-dependency proxy** for identifiability (honestly documented as `"structural_only"` in the assumptions list) rather than full do-calculus backdoor/frontdoor algorithms. EXPERIMENT degrades to ABSTAIN unless caller passes `allow_experiment=true`. New module `src/civex.rs` + 6 integration tests in `tests/civex_test.rs` covering EXECUTE, REJECT (cost > risk_threshold), REJECT (locked IRI), ABSTAIN (irreversible + ambiguous LCB), EXPERIMENT (reversible + authorised), and provenance-hash determinism.
- **`graph_projection_lossy_check` — audit projected RAG slices for information loss** (#35, IJCAI 2025). New MCP tool that compares a projected Turtle slice against the loaded ontology's full neighbourhood of seed IRIs and reports dropped predicates, dropped object IRIs, per-seed coverage ratio, and aggregate coverage. Pairs with the upcoming `onto_segment_retrieve` (#34) — the retriever produces the slice; this auditor reports what it left behind, so the calling LLM can decide whether the slice is sufficient. New module `src/projection_check.rs` + 4 inline unit tests covering full-projection-OK, dropped-predicate-flagged, parse-failure path, and missing-seed-in-source-trivially-covered.
- **HNSW polish — per-call tuning, Poincaré variant, async flush.** Three follow-on wins on the HNSW moat: (1) `onto_search` gains `use_hnsw` and `ef_search` parameters, so callers can route a single query through the HNSW cosine index and optionally trigger a rebuild with custom `ef_search` per query. Caveat documented: `instant-distance` bakes `ef_search` into the HNSW structure at build time and doesn't support per-query overrides, so a non-default `ef_search` triggers a rebuild — prefer `onto_hnsw_build` if you query frequently with the same value. (2) New `PoincareIndex` variant alongside `CosineIndex`, indexing structural embeddings (Poincaré ball) instead of text embeddings (cosine). Wired into `VecStore` with `search_poincare_hnsw`, `rebuild_poincare_index`, `persist_poincare_index`, `load_poincare_index`. The two indices coexist independently; both share the `hnsw_index_cache` SQLite table (kind = 'cosine' | 'poincare') with the same entries-fingerprint, so a mutation invalidates both at once. (3) Async background flush via `persist_cosine_index_async` / `persist_poincare_index_async` — returns a `tokio::task::JoinHandle` that resolves when the SQLite write completes. Serialisation happens synchronously (in-memory bincode, < 100ms for ontologies under ~10k classes); only the SQLite write is dispatched to `spawn_blocking`. Useful for keeping MCP tool handlers responsive when persisting large indices. Three new integration tests (Poincaré top-1 vs brute-force, coexistence with cosine, async persist round-trip) plus an additional Poincaré persistence test, taking the vecstore suite from 9 to 15 tests.
- **HNSW persistence + tuning + onto_align prefilter** (completes the moat scaffold). Three follow-on changes wire the prior HNSW scaffold into the rest of the system: (1) the built HNSW index now persists across process restarts via a new `hnsw_index_cache` SQLite table; `VecStore::load_from_db` automatically reinstates the cached index when its entries-fingerprint (deterministic FNV-1a 64-bit hash of sorted iri+text-vec bytes) matches the just-loaded vectors, and rejects stale caches when vectors changed — so a process startup over a populated DB skips the full rebuild. (2) New `onto_hnsw_build` MCP tool exposes the HNSW `ef_construction` and `ef_search` parameters so the connected orchestrator can tune index quality vs build/query time on larger ontologies; the tool optionally persists the rebuilt index. New `OntoHnswBuildInput` in `src/inputs.rs`. (3) `onto_align`'s candidate loop now transparently uses the HNSW index as a pre-filter when both source and target IRIs have embeddings in the vecstore: for each source class, a top-50 cosine shortlist of target candidates is computed once via HNSW, and the inner loop skips pairs not in the shortlist. The optimisation degrades gracefully — sources without embeddings fall back to the full target scan, preserving correctness on partially-embedded inputs. Two new persistence tests in `tests/vecstore_test.rs` cover the round-trip (vectors + index reload on a fresh `VecStore` over the same DB) and the cache-invalidation path (mutated vectors + unmutated index → cache rejected, rebuild on next search).
- **HNSW-accelerated cosine search scaffold** (the "vector-index moat"). New optional `instant-distance` dependency (gated behind the existing `embeddings` feature, no impact on default builds) and a new `src/hnsw_index.rs` module wrapping the HNSW algorithm (Malkov & Yashunin, TPAMI 2020). `VecStore` grows a `search_cosine_hnsw(query, top_k)` method that builds the index lazily on first call and rebuilds whenever the store is mutated; the existing `search_cosine` brute-force linear scan is unchanged and continues to work without HNSW (zero regression risk). Strategic context: per the May 2026 ecosystem research, no Rust knowledge-graph engine ships native HNSW alongside its triple store — the de-facto stack is `Neo4j + Qdrant + a Python adapter`. This module is the foundation for Open Ontologies to fill that gap as a Rust-native MCP server with first-class semantic search inside the same process. Scaffold scope: the core index + integration + tests; follow-up work (persistence layer for the built index, MCP-tool surface for tuning HNSW `ef_search` / `ef_construction`, wiring into `onto_align`'s embedding-similarity signal) is tracked in the project notes. New tests: 3 unit tests in `src/hnsw_index.rs` + 3 integration tests in `tests/vecstore_test.rs` (round-trip top-1 agreement with brute-force, mutation-invalidation behaviour, empty-store edge case).
- **GenOM-style description-based embedding enrichment for `onto_embed`**. New optional `descriptions: HashMap<String, String>` field on `OntoEmbedInput`. When the map is supplied, each class IRI in the map is embedded from its description text instead of from its `rdfs:label`; IRIs absent from the map fall back to the existing label-based embedding. Returns a new `enriched: <count>` field alongside the existing `embedded: <count>` so callers can see how many classes used descriptions vs. labels. This is the MCP-native form of the GenOM pattern (Mensa et al. 2025, accepted World Wide Web Journal, which showed Qwen-32B-generated descriptions lift alignment F1 substantially over raw-label embedding): the server doesn't generate descriptions, the connected orchestrator (Claude) authors them in-conversation using its own reasoning, then passes them in via this field. Net new dependencies: zero. Behaviour with no `descriptions` map is identical to before (back-compat). New inline unit tests in `src/inputs.rs` cover the deserialization both with and without the field present.
- **`ies-4.3.1` marketplace preset — frozen MIT baseline** (#25). New marketplace catalogue entry pointing at the archived `dstl/IES4` repo at tag `v4.3.1` (3 Mar 2025, MIT-licensed, last public release before the IES governance transition to DBT / IES-Org). Distinct from the existing `ies` preset (which tracks `IES-Org/ont-ies` main and shifts as upstream evolves) — use `ies-4.3.1` when you need a reproducible compliance baseline that won't drift. Source: 5,375-line Turtle artefact `ies4.ttl`, baseURI `http://ies.data.gov.uk/ontology/ies4`, the same namespace used by the existing `boro` and `ies4` enforce rule packs. Install via `onto_marketplace install ies-4.3.1`. Inline unit tests in `src/marketplace.rs` verify the URL pins to the `v4.3.1` tag (not `main`) and that the live `ies` and frozen `ies-4.3.1` presets coexist with distinct IDs and URLs.
- **RRF (Reciprocal Rank Fusion) as an opt-in fusion strategy for `onto_align`**. New `OntoAlignInput.fusion` field accepts `"weighted_sum"` (default, unchanged behaviour with self-calibrating learned weights) or `"rrf"` (Cormack et al. SIGIR 2009 at k=60, validated for ontology alignment by Agent-OM at VLDB 2025). RRF is order-based rather than score-based, so it doesn't need feedback to bootstrap; it's a sensible cold-start choice when the `align_feedback` table is empty. The per-signal scores remain on each candidate's `signals` field so downstream `onto_align_feedback` calls keep working identically. New public method `AlignmentEngine::align_with_fusion(source, target, high, low, dry_run, fusion)`; the existing `align_with_thresholds(...)` and `align(...)` entry points are preserved as thin wrappers that pass `"weighted_sum"`. New `tests/align_rrf_test.rs` (5 tests) covering normalisation to [0, 1], per-signal preservation, low-threshold post-rerank filtering, weighted_sum back-compat, and weighted_sum/RRF top-pair agreement on perfect matches.
- **`ies4` enforce rule pack** (#24). New built-in design-pattern pack for the [Information Exchange Standard](https://informationexchangestandard.org/), the UK cross-sector ontology framework custodied by Department for Business and Trade since March 2025 (canonical repo `IES-Org/ont-ies`). Three rules beyond the existing `boro` pack: (1) `ies4_particular_class_overlap` (severity: error) — a class cannot subclass both `ies:Particular` and `ies:ClassOfEntity`, as that violates the type-vs-token distinction foundational to IES4's 4D mereology; (2) `ies4_state_without_subject` (severity: warning) — a class subclassing `ies:State` must declare `ies:isStateOf` via owl:Restriction or have at least one instance using it (the state pattern is meaningless without a bearer); (3) `ies4_event_without_participant` (severity: warning) — a class subclassing `ies:Event` must have a participant pattern via `ies:isParticipantIn` / `ies:involvesParticipant` / `ies:hasParticipant` (events without participants are incomplete 4D models). Invoke via `onto_enforce` MCP tool or `enforce` CLI with `rule_pack = "ies4"`. Academic grounding: FOUST 7 paper "Comparing IES and BORO" (CEUR Vol-4176, JOWO 2024). New `tests/enforce_ies4_test.rs` (5 tests covering each rule's positive and negative cases, plus the instance-level participation accept path).

### Fixed
- **`onto_drift` now canonicalises blank nodes via RDFC 1.0 instead of filtering them out.** Replaces the temporary `_:`-prefix filter shipped in PR #14 (Jason Smith / @rustforrecess, who originally diagnosed the bnode-instability bug and shipped the surgical fix that bought time for this proper successor). New method `GraphStore::canonicalize_blank_nodes()` uses W3C RDF Dataset Canonicalization 1.0 (SHA-256) — available built-in via Oxigraph 0.5.8 — to assign deterministic `_:c14n<n>` identifiers derived from the graph structure. `DriftDetector::detect()` canonicalises each snapshot before vocabulary extraction, so reparses of the same ontology produce identical canonical bnode IDs (the reparse-stability property PR #14 achieved by exclusion), but anonymous restriction classes / quoted axioms now PARTICIPATE in the diff with stable IDs rather than being dropped. Caveat: canonical IDs are a function of the whole graph, so a quad change can shift many bnode IDs — for typical edits the existing rename-pairing logic in `detect()` re-matches shifted bnodes via the 4-signal ensemble (label / domain-range / hierarchy / individuals), so the net result is more informative than PR #14's filter. `tests/drift_blank_node_test.rs` updated to assert the new canonical-stability contract; new test `canonical_bnode_ids_are_stable_across_independent_reparses` exercises the contract directly on two restriction shapes.

### Changed
- **Oxigraph dependency bumped from 0.4 → 0.5.8** (#15). Oxigraph 0.5 ships RDF 1.2 / SPARQL 1.2 support (behind `rdf-12` / `sparql-12` feature flags), a new `SparqlEvaluator` builder-based query API, JSON-LD 1.1 by default, GeoSPARQL functions, a built-in `/sparql` HTTP server, single-pass ORDER BY, and — most relevant here — **built-in RDFC 1.0 canonicalisation** (W3C Recommendation, 21 May 2024), which gives deterministic blank-node identifiers via SHA-256 over canonical N-Quads. RDFC 1.0 is the proper successor to the bnode-filter hotfix in PR #14: a follow-up release can replace the `_:`-prefix filter in `extract_vocabulary` with canonicalisation, keeping semantic content while solving the reparse-instability problem at its root. The 0.4 → 0.5 migration was back-compatible for this codebase (the auto-migrating on-disk format means existing databases load without intervention). All six `Store::query` call sites in `graph.rs`, `shacl.rs`, and `ontology.rs` have been ported to the non-deprecated `SparqlEvaluator::new().parse_query(...).on_store(&store).execute()` chain; no deprecation warnings remain on the lib build. Full test suite (~290 tests) green on 0.5.8.

### Added
- **KGCL output format for `onto_drift`** (#17). The drift detector can now emit results in the [Knowledge Graph Change Language](https://github.com/INCATools/kgcl) (Mungall et al., Database 2025, doi:10.1093/database/baae133) alongside the existing JSON. Two new format options on the MCP tool: `format = "kgcl"` produces line-oriented CNL (`create node <iri>`, `obsolete node <iri>`, `obsolete node <iri> with replacement <iri>`) consumed by ROBOT and BioPortal; `format = "kgcl_json"` produces structured JSON-LD. High-confidence likely_renames (above `rename_threshold`, default 0.7) collapse into a `NodeObsoletion` with `has_direct_replacement` instead of plain add+remove pairs. New module `src/kgcl.rs` with 8 unit tests plus `tests/kgcl_drift_test.rs` integration suite.
- **LLM-orchestrated borderline-candidate review for `onto_align`** (#16). The alignment engine now splits its output into three buckets driven by two thresholds rather than a single `min_confidence` cliff: candidates with confidence above `high_threshold` (default 0.85) auto-apply as today, those in `[low_threshold, high_threshold)` (default low 0.4) surface in a new `borderline` array enriched with `context` (source/target labels and parent IRIs), and those below `low_threshold` are dropped. The MCP tool returns a `summary_for_review` instructing the connected LLM to inspect each borderline pair and call `onto_align_feedback` to record verdicts — those verdicts flow into the existing self-calibrating-weights loop. This is the MCP-native form of the LogMap-LLM "LLM-as-oracle" pattern (Jiménez-Ruiz et al., EACL 2026 main, top-2 in OAEI 2025 Bio-ML): no extra LLM client, no API key, no provider abstraction — the connected orchestrator does the judging via the conversation that already exists. New public method `AlignmentEngine::align_with_thresholds(source, target, high, low, dry_run)`; the legacy `align(source, target, min_confidence, dry_run)` remains and delegates with a degenerate range (empty borderline bucket) for back-compat. New `OntoAlignInput` fields `high_threshold` + `low_threshold` (both optional); `min_confidence` retained as the back-compat alias for `high_threshold`. New `tests/align_borderline_test.rs` (5 tests) covering bucket boundaries, context enrichment, summary text, and back-compat.
- **`onto_shacl_check` MCP tool — structural dry-run for proposed SHACL shapes** (#18). New `ShaclValidator::check_shapes(graph, shapes_ttl)` function and matching MCP tool that verifies (a) the shapes parse as Turtle and (b) every IRI they reference exists in the loaded ontology: `sh:targetClass` and `sh:class` must be declared as `owl:Class`/`rdfs:Class`; `sh:path` must be declared as `owl:ObjectProperty`, `owl:DatatypeProperty`, or `rdf:Property`; `sh:datatype` is prefix-checked against `xsd:`. Does NOT validate data — that's the existing `onto_shacl`. The intended workflow: the connected LLM generates candidate SHACL from a prose specification (the text2shacl paper, CiTIUS 2025, reports F1 0.904 / 0.934 / 0.699 on the EU ERA railway ontology with general-purpose LLMs), calls `onto_shacl_check` to catch missing IRIs, iterates, then runs `onto_shacl` to validate data. This is the MCP-native form of NL-to-SHACL: no LLM inside the server, no API key, the server provides the validation primitive and Claude does the authoring. Output includes per-shape diagnostic detail and an `issues` array categorised by `missing_target_class` / `missing_path` / `missing_class_constraint` / `unrecognised_datatype`. New `tests/shacl_check_test.rs` (7 tests covering well-formed shapes, each issue category, and Turtle parse failure).
- **DuckDB SQL data backbone**. New optional `duckdb` Cargo feature (and `sql` umbrella combining `postgres` + `duckdb`) wires DuckDB in alongside PostgreSQL as a *data integration* backbone — not as a SPARQL parser. DuckDB's extensions (`httpfs`, `parquet`, `csv`, `json`, `postgres_scanner`, `iceberg`, `delta`, …) let one SQL query federate over remote files, object stores, and other databases; rows then flow into the existing mapping/SHACL/reason pipeline.
- **New MCP tool `onto_sql_ingest`** — runs a SQL `SELECT` against PostgreSQL or DuckDB and ingests result rows into the triple store using the same `MappingConfig` shape as `onto_ingest`. Connection-string scheme is auto-detected (`postgres://`, `postgresql://`, `duckdb://`, `:memory:`, or a `*.duckdb` / `*.ddb` file path).
- **New CLI command `sql-ingest`** mirroring the MCP tool, with `--mapping`, `--inline-mapping`, `--base-iri`, and `-` (stdin) for the SQL.
- **`onto_import_schema` extended to DuckDB**. The same MCP tool / `import-schema` CLI now dispatches on the connection-string scheme: PostgreSQL via `sqlx`, DuckDB via the `duckdb` crate's `information_schema` + `duckdb_constraints()` introspection. The generated OWL is identical in shape (classes, datatype/object properties, NOT NULL → `owl:minCardinality 1`).
- **New `sql` tool group** in `[tools]` filter (`@sql` expands to `onto_import_schema` + `onto_sql_ingest`).
- **`SchemaIntrospector::sql_to_xsd` extended** to handle DuckDB-native types (HUGEINT, U{TINY,SMALL,}INT, DOUBLE, parameterised DECIMAL/VARCHAR, DATETIME, UUID, TIME).
- New tests: `tests/sqlsource_test.rs` (driver detection, no features required) and `tests/duckdb_test.rs` (introspection + query → row extraction, gated by the `duckdb` feature).

### Fixed
- **`onto_drift` ignores blank nodes**. Pizza-style ontologies (and any OWL with restriction classes) use anonymous blank-node restriction classes that get freshly reminted on every parse. Two snapshots of the same file would show ~40 added + ~40 removed bnodes plus a Cartesian product of confidence-scored "renames" between them, drowning real entity changes in noise. The vocabulary extractor now filters `_:`-prefixed IRIs from both class- and property-gather loops.

### Documentation
- `docs/data-pipeline.md` rewritten to cover both file-based and SQL-based ingest paths, the supported connection-string forms, federation examples (Parquet on S3 + Postgres scanner + remote CSV in one query), and a build matrix for the new feature flags.
- `SKILL.md`, `skills/ontology-engineering/SKILL.md`, `skills/ontology-engineer.md`, and `CLAUDE.md` Tool Reference tables expanded to cover the SQL backbone tools and previously-missing tools (`onto_status`, `onto_marketplace`, `onto_unload`, `onto_recompile`, `onto_cache_status`, `onto_cache_list`, `onto_cache_remove`, `onto_repo_list`, `onto_repo_load`, `onto_embed`, `onto_search`, `onto_similarity`, `onto_dl_explain`, `onto_dl_check`, `onto_import_schema`, `onto_sql_ingest`).

## [0.1.13] - 2026-05-01

### Added
- **Compile cache + TTL eviction + tool-exposure filter** (PR #1). Parsed ontologies are serialized to N-Triples on disk and reused on subsequent loads. A background evictor unloads idle ontologies after `[cache] idle_ttl_secs` (alias `unload_timeout_secs`); the on-disk cache is preserved and reloaded transparently on the next query. New `[tools]` config and `--tools-allow` / `--tools-deny` CLI flags restrict which `onto_*` tools the MCP server advertises (groups: `read_only`, `mutating`, `governance`, `remote`, `embeddings`).
- **New MCP tools**: `onto_cache_status`, `onto_cache_list`, `onto_cache_remove`, plus optional `name` parameter on `onto_unload` / `onto_recompile` for per-name cache management.
- **Ontology repository directories** (PR #2). New `[general] ontology_dirs` config (alias `data_dirs`) and `OPEN_ONTOLOGIES_ONTOLOGY_DIRS` env var let containerized deployments mount a folder of ontologies. Two new MCP tools enumerate and load from those directories with path-traversal guards: `onto_repo_list`, `onto_repo_load`.
- **OpenAI-compatible embeddings provider** (PR #3). New `[embeddings] provider = "openai"` mode targets any OpenAI-compatible gateway (official OpenAI, Azure, Ollama, vLLM, LocalAI, LM Studio, Together, …). Config fields: `api_base` (alias `base_url`), `api_key`, `model`, `dimensions`, `request_timeout_secs`. Env-var precedence: `OPEN_ONTOLOGIES_EMBEDDINGS_*` > `OPENAI_API_KEY` (for the key) > config > defaults. Remote responses are L2-normalized to remain comparable with local ONNX embeddings.
- **Surfaced operational config** (PR #4). New `[webhook]`, `[http]`, `[monitor]`, `[reasoner]`, `[feedback]`, `[imports]`, `[repo]`, `[socket]`, `[logging]` config sections expose previously hardcoded limits (tableaux depth/nodes, RDFS/OWL-RL fixpoint iterations, monitor interval, webhook timeout, import depth and remote-follow policy, feedback suppress/downgrade thresholds, etc.). A `0` value in the timeout / iteration fields is a sentinel that falls back to the documented default.
- New tests: `tests/registry_test.rs`, `tests/cache_management_test.rs`, `tests/toolfilter_test.rs`, `tests/repo_test.rs`, plus inline tests for embeddings config parsing and runtime knob initialization.

### Documentation
- New `docs/cache-and-registry.md` covering the compile cache, TTL eviction, tool-exposure filter, and ontology repository directories.
- `docs/embeddings.md` expanded with the OpenAI-compatible provider, supported gateways, config block, and env-var precedence.
- `CLAUDE.md` and `SKILL.md` Tool Reference tables updated with the seven new tools.

## [0.1.12] - 2026-03-27

### Added
- Virtualized tree view replacing D3/3D graph (handles 1500+ classes)
- Hierarchy connector lines, breadcrumb, and connections panel
- 13-step deep builder (`/build` command) producing IES-level ontologies
- `/sketch` command for quick prototyping
- `rdfs:Class` and `rdf:Property` support in Studio (not just `owl:Class`)
- Shared cargo target directory

### Fixed
- Static Linux binary via musl target (closes #2)

## [0.1.11] - 2026-03-25

### Added
- IES marketplace presets (`ies-top`, `ies-core`, `ies`)
- IES Building Extension (525 classes, clean-room)
- RDFS inference depth benchmark (662 vs 621)
- Head-to-head IRIS comparison
- Hierarchy enforce rule pack
- EPC benchmark (36/36 vs 18/36)

### Changed
- Default features off (lean build — drops tract-onnx and sqlx from default)

## [0.1.10] - 2026-03-13

### Added
- Quickstart guide (`docs/quickstart.md`)
- Server round-trip integration test (`tests/server_roundtrip_test.rs`)
- Complete architecture table in CONTRIBUTING.md (26 modules)

### Fixed
- Inconsistent CLI output: version/history/rollback/enrich/validate-clinical now respect `--pretty`
- CONTRIBUTING.md architecture table missing 10 modules (error, config, inputs, lineage, mapping, state, schema, embed, structembed)

## [0.1.9] - 2026-03-13

### Added
- Embedding similarity as alignment signal #7 (`onto_align` now uses text+structural embeddings when available)
- `onto_embed`, `onto_search`, `onto_similarity` MCP tools for semantic search
- End-to-end embedding pipeline test
- Embedding tools in architecture diagram and workflow documentation

### Fixed
- Feature gating for `tool_router` macro, clippy warnings, and tokenizer download
- Linux binary now built on ubuntu-22.04 for wider glibc compatibility

## [0.1.8] - 2026-03-12

### Added
- Poincare structural embedding trainer (Riemannian SGD for hierarchy layout)
- ONNX text embedder with tract (bge-small-en-v1.5, downloaded on init)
- Dual-space vector store with cosine + Poincare search and SQLite persistence
- Poincare ball geometry module (distance, exp_map, Riemannian SGD)

### Fixed
- Release binary naming now includes target triple
- Replaced deprecated macos-13 runner with macos-14

## [0.1.6] - 2026-03-11

### Added
- Glama server metadata and author verification

### Fixed
- Docker runtime libs and removed init from Dockerfile

## [0.1.5] - 2026-03-11

### Fixed
- Added build-essential and clang to Docker builder for oxrocksdb-sys compilation

## [0.1.4] - 2026-03-11

### Fixed
- Installed OpenSSL and libpq dev headers in Docker builder stage

## [0.1.3] - 2026-03-10

### Fixed
- Use latest Rust image in Dockerfile (dependencies need Rust 1.88+)

## [0.1.2] - 2026-03-10

### Fixed
- Free disk space in Docker workflow and optimize build
- Bumped server.json to v0.1.1

## [0.1.1] - 2026-03-09

### Added
- MCP Registry server.json, Docker publish workflow, and OCI label
- Streamable HTTP transport (`serve-http` command)
- MCP prompts (build_ontology, validate_ontology, compare_ontologies, ingest_data, explore_ontology)
- Dockerfile for containerized deployment
- OntoAxiom benchmark showdown (tool-augmented vs bare LLMs)
- Claude Code plugin package and ClawHub skill wrapper
- Bare Claude and hybrid benchmarks for three-way comparison
- Self-calibrating feedback for lint and enforce (dismiss 3x to suppress)
- Ontology alignment (`onto_align`, `onto_align_feedback`) with 6 weighted signals
- Terraform-style lifecycle: plan, apply, lock, drift, enforce, monitor, lineage
- Data pipeline: ingest, map, SHACL validate, reason, extend
- Clinical crosswalks (ICD-10, SNOMED, MeSH)
- OWL2-DL SHOIQ tableaux reasoner with parallel classification
- Design pattern enforcement (generic, BORO, value_partition)
- Version snapshots and rollback
- Core ontology tools: validate, load, save, query, stats, diff, lint, convert, clear, pull, push, import

### Fixed
- Clippy `io_other_error` warning breaking CI
- MCP benchmark scoring (camelCase normalization, pair order)
