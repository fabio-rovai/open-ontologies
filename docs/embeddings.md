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

### Notes

- API responses are L2-normalised in-process so cosine scores remain comparable with the local ONNX path.
- The local and remote paths share the same downstream `onto_embed` / `onto_search` / `onto_similarity` tools — switching providers requires no code changes.
- See `[embeddings]` block in the default config emitted by `open-ontologies init` (`src/main.rs::DEFAULT_CONFIG`) for the full set of fields with comments.
