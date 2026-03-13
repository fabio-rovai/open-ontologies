# Quickstart

Get from zero to a validated, queryable ontology in under 2 minutes.

## Install

```bash
# macOS (Apple Silicon)
curl -LO https://github.com/fabio-rovai/open-ontologies/releases/latest/download/open-ontologies-aarch64-apple-darwin
chmod +x open-ontologies-aarch64-apple-darwin
mv open-ontologies-aarch64-apple-darwin /usr/local/bin/open-ontologies

# Initialize (creates ~/.open-ontologies, downloads embedding model)
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

Every MCP tool also works as a CLI subcommand:

```bash
# Validate a Turtle file
open-ontologies validate pizza.ttl

# Load and query
open-ontologies load pizza.ttl
open-ontologies query "SELECT ?class WHERE { ?class a owl:Class }" --pretty

# Run OWL2-DL reasoning
open-ontologies reason --profile owl-dl

# Semantic search (requires embeddings)
open-ontologies init  # downloads model if needed
# then via MCP: onto_embed -> onto_search "domestic animal"
```

## What's next

- [Data Pipeline](data-pipeline.md) -- ingest CSV/JSON/Parquet into your ontology
- [Ontology Lifecycle](lifecycle.md) -- plan, enforce, apply, monitor changes
- [OWL2-DL Reasoning](reasoning.md) -- native Rust SHOIQ tableaux
- [Semantic Embeddings](embeddings.md) -- dual-space search (text + Poincare)
- [Benchmarks](benchmarks.md) -- performance numbers and comparisons
