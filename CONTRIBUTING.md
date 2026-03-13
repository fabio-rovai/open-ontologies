# Contributing to Open Ontologies

Thanks for your interest in contributing! This document explains how to get started.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/open-ontologies.git`
3. Create a feature branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run checks: `cargo build && cargo test && cargo clippy -- -D warnings`
6. Push and open a PR

## Development Setup

**Requirements:**
- Rust 1.85+ (edition 2024)
- For PostgreSQL schema import: `libpq-dev`
- For embeddings: models are downloaded automatically via `open-ontologies init`

**Build:**
```bash
cargo build --release
```

**Test:**
```bash
cargo test
```

**Lint:**
```bash
cargo clippy -- -D warnings
```

**Audit:**
```bash
cargo install cargo-audit
cargo audit
```

## Architecture

The codebase is organized into domain modules under `src/`:

| Module | Purpose |
|--------|---------|
| `server.rs` | MCP server + tool/prompt implementations |
| `inputs.rs` | Tool and prompt input structs (JsonSchema) |
| `error.rs` | Typed error enum (`OntologyError`) |
| `graph.rs` | Oxigraph triple store wrapper |
| `ontology.rs` | Core RDF operations (validate, load, diff, lint) |
| `state.rs` | SQLite state database (versions, feedback, locks) |
| `config.rs` | TOML configuration loading |
| `tableaux.rs` | OWL2-DL SHOIQ tableaux reasoner |
| `align.rs` | Cross-ontology alignment (7 weighted signals) |
| `reason.rs` | RDFS/OWL-RL inference |
| `ingest.rs` | Data format parsing (CSV, JSON, XLSX, Parquet, etc.) |
| `mapping.rs` | Tabular-to-RDF mapping config |
| `schema.rs` | PostgreSQL schema introspection → OWL |
| `drift.rs` | Version comparison & rename detection |
| `enforce.rs` | Design pattern checking |
| `plan.rs` | Terraform-style planning |
| `shacl.rs` | SHACL shape validation |
| `monitor.rs` | Lifecycle monitoring |
| `lineage.rs` | Append-only audit trail |
| `clinical.rs` | ICD-10/SNOMED/MeSH crosswalks |
| `feedback.rs` | Self-calibrating feedback |
| `poincare.rs` | Hyperbolic geometry (embeddings feature) |
| `vecstore.rs` | Dual-space vector store (embeddings feature) |
| `embed.rs` | ONNX text embedder (embeddings feature) |
| `structembed.rs` | Poincaré structural embedding trainer (embeddings feature) |

## Pull Requests

- Keep PRs focused on a single change
- Include tests for new functionality
- Run `cargo clippy -- -D warnings` before submitting
- Update CHANGELOG.md for user-facing changes
- CI must pass (build, test, clippy, audit)

## Code Style

- Follow standard Rust conventions (rustfmt defaults)
- Use `anyhow::Result` for fallible operations
- All MCP tools use the `onto_` prefix
- Feature-gate optional dependencies with `#[cfg(feature = "...")]`
- Integration tests go in `tests/`, not inline

## Reporting Issues

Open an issue on GitHub with:
- What you expected to happen
- What actually happened
- Steps to reproduce
- Relevant error output

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
