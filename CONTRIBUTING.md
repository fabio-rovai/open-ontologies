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
- Rust 1.88+ (edition 2024)
- For PostgreSQL schema import: `libpq-dev`
- For embeddings: build with `--features embeddings`, then run `open-ontologies init`, which downloads a 448 MB ONNX model and a 16 MB tokenizer

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
| `graph.rs` | Oxigraph triple store wrapper |
| `ontology.rs` | Core RDF operations (validate, load, diff, lint) |
| `state.rs` | SQLite state database (versions, feedback, locks) |
| `config.rs` | TOML configuration loading |
| `tableaux.rs` | SHIQ tableaux reasoner |
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

## Contributing Without Writing Rust

Three extension surfaces accept contributions that never touch the core:

- **Community ontology packs** — add a manifest to [`community/registry.json`](community/registry.json); it becomes installable via `onto_marketplace` for every user as soon as the PR merges. Rules and acceptance criteria: [community/README.md](community/README.md).
- **Community skills** — markdown workflow recipes in [`skills/community/`](skills/community/); start from the template there.
- **Companion servers** — build your own MCP server against the [companion contract](docs/companion-servers.md) and PR a listing row to [ECOSYSTEM.md](ECOSYSTEM.md).

For in-process tools, see the **WASM plugin** surface ([docs/plugins.md](docs/plugins.md)) — plugins live in their own repos and need no PR here at all, though we happily link notable ones from ECOSYSTEM.md.

## Pull Requests

- Keep PRs focused on a single change
- Include tests for new functionality
- Run `cargo clippy -- -D warnings` before submitting
- Update CHANGELOG.md for user-facing changes
- CI must pass (build, test, clippy, audit)

## Releasing

Releases are cut from tags and their GitHub release notes are populated automatically from CHANGELOG.md. The convention:

1. Add the CHANGELOG.md entry for the new version first. Both heading formats in use are recognized by the automation: `## X.Y.Z - YYYY-MM-DD` (bare version) and `## [X.Y.Z] - YYYY-MM-DD` (bracketed version).
2. Bump `version` in `Cargo.toml` to the same `X.Y.Z` and commit both changes together.
3. Put that commit through CI: push it to `main`, or make it the head of a pull request into `main`. CI runs nowhere else, and a release will not publish a commit CI has not passed.
4. Tag that commit: `git tag vX.Y.Z` (the tag carries the leading `v`; the CHANGELOG heading does not need one).
5. Push the tag: `git push origin vX.Y.Z`. It can go up with the commit, because the release waits for CI.

Pushing the tag fires `.github/workflows/release.yml`. Before anything is built it checks that the tag, `Cargo.toml` and a CHANGELOG section agree; it then builds and checks every binary, and waits up to 90 minutes for the four required CI checks on the tagged commit. Only then does it publish the release, with that CHANGELOG section as the body, and push the ghcr image. A version mismatch, a missing CHANGELOG section, or a required check that failed or never ran stops the release before anything is published.

`open-ontologies-lite` is published to PyPI from a `py-vX.Y.Z` tag by `.github/workflows/python-release.yml`, which holds the upload back the same way until `python (open-ontologies-lite)` has passed on the tagged commit.

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
