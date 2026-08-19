# Semantic Embeddings (Poincare Vector Store)

Open Ontologies includes a built-in dual-space vector store for semantic search and alignment:

- **Text embeddings** via ONNX model (bge-small-en-v1.5) — captures label/definition similarity
- **Structural embeddings** via Poincare ball — captures hierarchy position (root classes near center, leaves near boundary)
- **Product search** — combines both spaces for best results

```text
onto_load → onto_embed → onto_search "domestic animal"
```

The embedding model (~33MB) is downloaded on `open-ontologies init`. All inference runs locally via tract (pure Rust ONNX runtime) — no API keys or external services needed.

## Tools

| Tool | Purpose |
| ---- | ------- |
| `onto_embed` | Generate embeddings for all classes in the loaded ontology |
| `onto_search` | Semantic search by natural language query |
| `onto_similarity` | Compare two IRIs by embedding similarity |

## Search Modes

| Mode | What it uses |
| ---- | ------------ |
| `text` | Cosine similarity on text embeddings only |
| `structure` | Poincare distance on structural embeddings only |
| `product` | Weighted combination of both (default, alpha=0.5) |

## Cosine index backends

The text (cosine) half of the store has two interchangeable index backends.
The structural (Poincare) half has one, and will keep having one: TurboQuant
scores inner products, and hyperbolic distance is not an inner product on the
ambient coordinates.

| | `embeddings` (default) | `turbovec` |
| --- | --- | --- |
| Algorithm | HNSW graph (`instant-distance`) | TurboQuant quantiser (`turbovec`, arXiv:2504.19874) |
| Entry point | `VecStore::search_cosine_hnsw` | `VecStore::search_cosine_turbo` |
| Cost of one added embedding | Full graph rebuild, the graph is immutable | One append |
| Cost of one removal | Full graph rebuild | O(1) |
| Storage | float32 in the graph | 4 bit codes plus a per-vector scale |
| Scores returned | Exact for whichever entries the graph walk reaches | Exact, the shortlist is re-scored against float32 |
| Approximation lives in | Which entries the walk reaches | Which entries the quantised shortlist contains |

Build it with `cargo build --features turbovec`. The feature implies
`embeddings`; it replaces one backend rather than the whole subsystem, and the
exact brute-force `search_cosine` stays available under both.

`search_cosine_turbo` treats the quantised index as a candidate generator
only. It pulls a shortlist four times wider than the request (floor of
`top_k + 32`), re-scores every candidate against the float32 vector the store
already holds, and returns that. So a quantised similarity number never
reaches a caller, and the result is identical to the exact scan whenever the
shortlist covers the true top-k.

Both backends persist into `hnsw_index_cache` under their own `kind`
(`cosine`, `poincare`, `turbo_cosine`), and both are gated by the same two
checks on load: `model_fp` rejects an index built under a different embedding
configuration, and `entries_hash` rejects one whose entry set has moved on.

### Measured

10,000 vectors x 768 dims, Apple M3 Max, release build,
`cargo test --release --features turbovec -- --ignored --nocapture`
(`measure_turbo_against_hnsw` in `tests/turbovec_index_test.rs`).

| | HNSW | TurboQuant |
| --- | --- | --- |
| Build | 116 s | 178 ms |
| One added embedding | 137 s (full rebuild) | 52 us |
| Query, top-10 | 4.7 ms | 0.25 ms (pulling 40 candidates) |
| recall@10 as wired | 91.6% | 100% |
| recall@10, 40 candidates re-scored to 10 | 91.6% | 100% |
| Serialised index | 34.1 MB | 5.2 MB |

Raw float32 for that corpus is 30.7 MB, so the HNSW blob is larger than the
vectors it indexes and the TurboQuant one is a sixth of them. Build and
rebuild times move by a factor of several between runs depending on machine
load; the query, recall and size figures are stable.

Read the two recall rows together, because that is the control. Asking HNSW
for 40 candidates instead of 10 and re-scoring changes nothing: the entries it
misses are ones its graph walk never reaches, so no amount of over-fetching
recovers them, and `ef_search` (default 100) is the only lever. The flat
quantised scan has no such failure mode by construction, because it scores
every vector. That is a structural difference, not a tuning one.

One caveat on those recall numbers: the corpus is isotropic random vectors,
which is close to the worst case for a graph index. Every pairwise cosine sits
near zero, so ranks 10 and 11 are separated by noise and there is no cluster
structure for the walk to exploit. Real text embeddings are kinder to HNSW.
The point is not that HNSW loses 8% on your ontology; it is that its recall is
data-dependent while the flat index's is not.

**When to pick which.** HNSW if the ontology is embedded once and then only
queried. TurboQuant if embeddings arrive incrementally (the rebuild is what
you are paying for, not the query), or if the corpus is large enough that
float32 storage is the constraint: at 768 dims a 4 bit code is 388 bytes
against 3,072.

## Providers

The text-embedding side is pluggable via `[embeddings] provider`:

| Provider | When to use |
| -------- | ----------- |
| `local` (default) | Offline / air-gapped; no API keys; bge-small-en-v1.5 ONNX runs in-process via tract |
| `openai` | Any OpenAI-compatible HTTP gateway: official OpenAI, Azure OpenAI, Ollama, vLLM, LocalAI, LM Studio, Together, Mistral, etc. |

### Configuring the OpenAI-compatible provider

```toml
[embeddings]
provider = "openai"
api_base = "https://api.openai.com/v1"   # alias: base_url
api_key  = "sk-..."                       # optional — env vars take precedence
model    = "text-embedding-3-small"       # any model your gateway serves
dimensions = 1536                         # optional — only sent when set
request_timeout_secs = 30
```

Trailing slashes on `api_base` are stripped automatically. The gateway must accept `POST {api_base}/embeddings` with the OpenAI request shape.

### Environment variables (override config)

| Variable | Purpose | Precedence |
| -------- | ------- | ---------- |
| `OPEN_ONTOLOGIES_EMBEDDINGS_PROVIDER` | Force `local` or `openai` | Highest |
| `OPEN_ONTOLOGIES_EMBEDDINGS_API_BASE` | Override gateway URL | Highest |
| `OPEN_ONTOLOGIES_EMBEDDINGS_API_KEY`  | Override bearer token | Highest |
| `OPENAI_API_KEY` | Bearer token fallback when the dedicated var is unset | Higher than config |
| `OPEN_ONTOLOGIES_EMBEDDINGS_MODEL` | Override model name | Highest |

Auth is optional — many local gateways (Ollama, LocalAI) accept unauthenticated requests, so the resolver returns `None` rather than failing when no key is configured.

### Example: Ollama (local OpenAI-compatible gateway)

```toml
[embeddings]
provider = "openai"
api_base = "http://localhost:11434/v1"
model    = "nomic-embed-text"
# api_key is unnecessary for local Ollama
```

## Changing the model or provider

**Stored vectors are only comparable against queries from the same
configuration.** Two models of the same dimension produce numbers of the same
shape and no shared meaning, so mixing them does not fail — retrieval quality
just quietly drops.

Every stored vector and every cached HNSW index therefore carries `model_fp`, a
composite hash of `(provider, model, revision)`:

| provider | `model` | `revision` |
|---|---|---|
| `local` | model file name | sha256 of the **whole file** |
| `openai`-compatible | resolved model name, plus `api_base` and `dimensions` | none — the API exposes no revision |

The fingerprint describes the **effective** configuration, so an environment
override (`OPEN_ONTOLOGIES_EMBEDDINGS_MODEL`, …) counts as a change. For the
local provider it hashes the file *contents*: replacing the `.onnx` in place
leaves the path and filename identical, and that is the most likely way a local
model gets swapped.

The whole file, not a head prefix — two fine-tunes of one architecture have
identical sizes and identical ONNX graph protos, and `cp -p` preserves mtime, so
`(size, mtime, head)` cannot tell them apart. Cost, measured on the 470 MB
default model: **1.4 s**, once per process, and only when the local provider is
in use — against a model load that already reads and optimises the same file.

On a mismatch, the affected vectors and index caches are discarded and rebuilt,
with a `warn` naming what happened. What to expect:

- **After changing `[embeddings] model`, `provider`, `api_base` or
  `dimensions`** — everything must be re-embedded. Discarding is not optional:
  the old vectors cannot be compared against new queries.
- **The first start after upgrading** to a build that has this column — vectors
  written earlier carry no fingerprint. Nothing recorded which model produced
  them, and assuming it was the current one is the failure this prevents, so
  they are discarded once. Re-embed and it does not recur.
- **No change** — nothing is discarded and cached indices are reused, exactly as
  before.

`entries_hash` is unchanged and still checked. It catches a changed entry set;
`model_fp` catches an unchanged entry set queried by a different model — which
`entries_hash` structurally cannot see, since the stored bytes are identical.

### Notes

- API responses are L2-normalized in-process so cosine scores remain comparable with the local ONNX path.
- The local and remote paths share the same downstream `onto_embed` / `onto_search` / `onto_similarity` tools — switching providers requires no code changes.
- See `[embeddings]` block in the default config emitted by `open-ontologies init` (`src/main.rs::DEFAULT_CONFIG`) for the full set of fields with comments.
- Changing the *tokenizer* alone (same `.onnx`, different `tokenizer.json`) **is** covered: its sha256 is folded into the local arm's revision alongside the model's, because a different tokenizer produces different vectors from the same model.
