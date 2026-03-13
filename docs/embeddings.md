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
