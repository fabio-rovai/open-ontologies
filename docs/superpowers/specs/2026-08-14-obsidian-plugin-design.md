# Open Ontologies for Obsidian — Design

**Date:** 2026-08-14
**Status:** Approved for planning
**Owner:** Fabio Rovai

## Summary

An official Obsidian plugin that gives Obsidian users the **full** Open Ontologies engine — all 70+ `onto_*` tools including the OWL-RL/EL reasoners, SHACL, causal layer, and planner — by managing the native release binary as a sidecar process. Obsidian becomes a first-class distribution channel of the product, alongside PyPI (`open-ontologies-lite`) and Docker. Unlike the PyPI channel, this is not a lite port: the plugin is a client of the real engine.

## Goals

1. Someone running Obsidian can install one community plugin and get every Open Ontologies function, reasoners included, with zero manual binary installation.
2. Ontology files (`.ttl`, `.owl`, `.rdf`, `.jsonld`) stored in a vault are first-class: validate, reason, classify, diff, lint, query.
3. The vault itself (notes, frontmatter, tags, wikilinks) can be compiled to RDF, reasoned over, and SHACL-validated against shapes stored in the vault.
4. Distribution through the official Obsidian community plugin store.

## Non-goals (v1)

- **Mobile / WASM.** The causal layer (Python subprocess), planner (Fast Downward), embeddings (ONNX), and SQLite persistence are native-only. `isDesktopOnly: true`. A WASM lite core is a possible later phase only if demand appears.
- **AI chat.** Obsidian users bring their own LLM plugins; the engine stays MCP-native (no embedded LLM). The plugin is UI + orchestration, not a chat host.
- **Graph visualisation beyond the tree pane.** Obsidian's own graph view covers the vault plane.

## Repos and distribution

- Plugin code lives in a sibling repo **`obsidian-open-ontologies`** under the same GitHub account.
  - Rationale: the Obsidian store resolves a plugin to a GitHub repo and downloads assets from the release whose **tag exactly matches `manifest.json` version**. Plugin releases in the engine monorepo would hijack `releases/latest`, breaking the engine's documented `releases/latest/download/...` install URLs.
- Plugin id: `open-ontologies`. Branded and documented as part of the product family.
- Main repo README gains an "Obsidian" install section beside PyPI/Docker; `docs/` cross-links both ways.
- Release pipeline: GitHub Action runs esbuild, attaches `main.js`, `manifest.json`, `styles.css` to a release tagged with bare semver. Installable via BRAT immediately; submitted to `obsidianmd/obsidian-releases` for store listing.

## Architecture

TypeScript Obsidian plugin. The engine is consumed **as released — zero Rust changes**.

### EngineManager

- Locates an engine binary in this order: user-configured path (settings) → previously downloaded copy in the plugin data dir → auto-download.
- Auto-download pulls the platform asset (macOS arm64/x64, Linux x86_64, Windows) from the engine repo's GitHub Releases, checksum-verified.
- Spawns `open-ontologies serve-http` on an ephemeral localhost port; health-checks before reporting ready.
- Lifecycle: kill on plugin unload; auto-restart on crash with capped backoff; retry on a new port if the chosen port is taken.
- Version management: the plugin declares a compatible engine semver range. On mismatch it blocks with a one-click update prompt — never silent skew.

### EngineClient

- Thin typed wrapper over the engine's existing MCP streamable-HTTP transport (`serve-http`).
- Hand-curated TypeScript types for the ~15 workbench-critical tools (`onto_load`, `onto_validate`, `onto_shacl_check`, `onto_reason`, `onto_classify_el`, `onto_diff`, `onto_query`, `onto_lint`, `onto_apply`, `onto_save`, `onto_pack`, `onto_unpack`, …).
- Generic `call(tool, args)` escape hatch exposes the remaining tools, so **every** engine function is reachable from day one.

### Vault→RDF mapper

Compiles the vault into a named graph in the engine via a sync command.

Default mapping (each rule overridable via a settings-editable YAML block):

| Vault construct | RDF |
|---|---|
| Note | Individual; IRI minted from vault-relative path, overridable with frontmatter `iri:` |
| Frontmatter `type:` | `rdf:type` |
| Other frontmatter keys | Datatype or object properties (object when the value is a wikilink) |
| Typed wikilink `[[property::Target]]` | Object property named by `property` |
| Plain wikilink | Configurable default predicate |
| Tag | SKOS concept |

Once synced, the vault graph is SHACL-validated against shapes stored in the vault and reasoned over; inferred triples and violations surface per-note.

## UI surfaces (v1)

1. **Ontology tree pane** (right sidebar): class/property hierarchy of loaded graphs, Studio-style; click navigates to the defining file or note.
2. **SPARQL console pane**: query editor with history, sortable results table, "insert IRI from tree" affordance.
3. **Validation panel**: SHACL/lint/reasoning results with severity, deep-linking to file+line or note.
4. **Command palette**: sync vault, validate current file, reason, diff, classify, pack/unpack, restart engine.
5. **File integration**: plugin registers as viewer for `.ttl`/`.owl`/`.rdf`/`.jsonld` (raw text + tree side-by-side); validate-on-save with inline diagnostics.

## Error handling

- Offline or download failure → persistent notice with the manual-binary-path escape hatch.
- Engine crash → capped auto-restart; after the cap, surfaced error with a log excerpt.
- Port in use → retry on a new ephemeral port.
- Engine/plugin version mismatch → blocking one-click update prompt.
- All engine stderr captured to a rotating log accessible from settings.

## Testing

- **Mapper unit tests** (highest-risk logic): fixture vault → expected triples.
- **EngineClient tests** against a mocked HTTP server.
- **CI integration job**: download the real binary, run sync → validate → reason end-to-end.
- **`test-vault/`** checked into the plugin repo for manual QA.

## Decisions on version and security policy

- **Engine compatibility range:** the plugin pins `^MAJOR.MINOR` of the engine version it was tested against (e.g. `^1.1`), widening only after an explicit compatibility test in CI.
- **Network exposure:** the sidecar must bind `127.0.0.1` only. Implementation includes verifying the current `serve-http` bind behaviour and, if it binds `0.0.0.0`, passing/adding a loopback-only flag before first release.
