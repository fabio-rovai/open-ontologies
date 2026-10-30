# Quickstart

Get from zero to a validated, queryable ontology in under 2 minutes.

## Install

```bash
# macOS (Apple Silicon)
curl -LO https://github.com/fabio-rovai/open-ontologies/releases/latest/download/open-ontologies-aarch64-apple-darwin
chmod +x open-ontologies-aarch64-apple-darwin
mv open-ontologies-aarch64-apple-darwin /usr/local/bin/open-ontologies

# Initialize (creates ~/.open-ontologies with a database and a default config)
open-ontologies init
```

## Connect

Add to your MCP client config (Claude Code: `~/.claude/settings.json`):

```json
{
  "mcpServers": {
    "open-ontologies": {
      "command": "open-ontologies",
      "args": ["serve"]
    }
  }
}
```

## Use

Ask Claude:

```text
Build me a Pizza ontology with 5 toppings and 3 named pizzas.
Validate it, load it, and show me the stats.
```

Claude will call `onto_validate` -> `onto_load` -> `onto_stats` -> `onto_lint` automatically.

## CLI mode

Most MCP tools also work as CLI subcommands. The embedding tools (`onto_embed`, `onto_search`, `onto_similarity`) are MCP only and have no CLI equivalent in any build.

```bash
# Validate a Turtle file
open-ontologies validate pizza.ttl

# Load and query
open-ontologies load pizza.ttl
open-ontologies query "SELECT ?class WHERE { ?class a owl:Class }" --pretty

# Run SHIQ tableaux reasoning
open-ontologies reason --profile owl-dl
# ...with a 30 s budget per phase for this run (overrides [reasoner] in config.toml)
open-ontologies reason --profile owl-dl --phase-timeout-ms 30000

# Semantic search (requires a build with --features embeddings)
open-ontologies init  # on an embeddings build, downloads a 448 MB ONNX model and a 16 MB tokenizer
# then via MCP: onto_embed -> onto_search "domestic animal"
```

## What's next

- [Data Pipeline](data-pipeline.md) -- ingest CSV/JSON/Parquet into your ontology
- [Ontology Lifecycle](lifecycle.md) -- plan, enforce, apply, monitor changes
- [SHIQ Reasoning](reasoning.md) -- native Rust SHIQ tableaux, no nominals
- [Semantic Embeddings](embeddings.md) -- dual-space search (text + Poincare)
- [Benchmarks](benchmarks.md) -- performance numbers and comparisons
