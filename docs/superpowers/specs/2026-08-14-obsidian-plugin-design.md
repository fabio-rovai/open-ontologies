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
4. **An LLM agent (Claude Code, Claude Desktop, or any MCP client) can query the reasoned vault graph over MCP**, so the "AI second brain" pattern gets entailment and validation instead of grep over Markdown.
5. Distribution through the official Obsidian community plugin store.

## Non-goals (v1)

- **Mobile / WASM.** The causal layer (Python subprocess), planner (Fast Downward), embeddings (ONNX), and SQLite persistence are native-only. `isDesktopOnly: true`. A WASM lite core is a possible later phase only if demand appears.
- **AI chat.** Obsidian users bring their own LLM plugins; the engine stays MCP-native (no embedded LLM). The plugin is UI + orchestration, not a chat host.
- **File access for agents.** We do not expose read/write/search over vault files to an LLM. `mcp-obsidian` plus the Local REST API plugin already does that well, and the two compose: an agent can hold both connections. Duplicating it would trade our only defensible advantage (reasoning) for a commodity.
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
- Spawns `open-ontologies serve-http --host 127.0.0.1 --port <configured> --token <generated>`; health-checks before reporting ready.
- Port is the configured stable one (default 27125) so an external MCP client can be pointed at it once; falls back to an ephemeral port with a notice if it is occupied.
- The bearer token is generated on first run, stored in plugin settings, and passed to every request. It is mandatory — see the CORS finding under "Agent-facing layer".
- Lifecycle: kill on plugin unload; auto-restart on crash with capped backoff.
- Version management: the plugin declares a compatible engine semver range. On mismatch it blocks with a one-click update prompt — never silent skew.

### EngineClient

- Thin typed wrapper over the engine's existing MCP streamable-HTTP transport (`serve-http`), sending `Authorization: Bearer <token>` on every request.
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

## Agent-facing layer: the second-brain use case

The dominant Obsidian-plus-Claude pattern (Karpathy's LLM Wiki, popularised April 2026) wires Claude to the vault through the Local REST API community plugin and the `mcp-obsidian` MCP server. That stack gives an agent file read, file write, and text search. It does not give it entailment, typed structure, or validation.

We do not compete with it, and we must not reimplement it. `mcp-obsidian` stays the file-access layer; Open Ontologies is the **reasoning layer next to it**. The claim is specific and defensible: an agent talking to this plugin gets a typed, entailed, SHACL-validated graph of the vault, and can ask questions no text search answers ("which notes assert a claim that contradicts a shape", "what is transitively part of this project", "which people are two hops from this topic").

Four components make that real.

**1. Stable, authenticated MCP endpoint.** The sidecar already *is* an MCP server, so no new server is needed. What it needs is an address an agent can be configured against once: a fixed loopback port (default 27125, chosen to sit beside Local REST API's 27124) instead of an ephemeral one, plus a bearer token the plugin generates on first run and passes as `--token`. Settings offers a one-click "copy MCP client config" that emits the correct JSON with the URL and header filled in.

The token is **required, not optional**, because of a specific finding in the engine source: `serve-http` applies `tower_http::cors::CorsLayer::permissive()` to the router (`src/main.rs`, immediately before the `TcpListener::bind`). Permissive CORS on an ephemeral port that nothing knows about is a limited exposure; permissive CORS on a documented fixed port is not. Without a token, any web page open in the user's browser could issue cross-origin `POST http://127.0.0.1:27125/mcp` and read or mutate the vault graph. The token closes that: the bearer layer already wraps `/api` and `/mcp` (only `/health` sits outside it), so an unauthenticated cross-origin request is rejected before it reaches a tool. No Rust change is required — the capability exists and we must use it.

The config the settings pane copies to the clipboard (verified against current Claude Code documentation; the same shape works in Claude Code's `.mcp.json`/`~/.claude.json` and in Claude Desktop's `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "open-ontologies": {
      "type": "http",
      "url": "http://127.0.0.1:27125/mcp",
      "headers": { "Authorization": "Bearer <generated-token>" }
    }
  }
}
```

The equivalent CLI form is `claude mcp add --transport http --header "Authorization: Bearer <token>" open-ontologies http://127.0.0.1:27125/mcp`. Both are offered in settings; the README documents the JSON form as primary because it is unambiguous across clients.

**2. Auto-sync, so the graph an agent queries is current.** A stale graph is worse than no graph, because the agent cannot tell. On markdown create, modify, delete, or rename, the plugin schedules a debounced full vault re-sync (fires 10 seconds after the last change) in addition to the manual command. Full re-sync rather than per-note patching is deliberate for v1: deleting a note's prior triples correctly requires tracking what was asserted for it, and getting that wrong silently corrupts the graph. The engine handles graphs far larger than any vault (the repo's own LUBM benchmark runs at 1.29M triples), so a whole-vault reload is affordable. Incremental sync via `onto_reason_incremental` is a v2 optimisation, not a v1 requirement.

**3. A starter vault ontology, because entailment over an untyped graph is vacuous.** This is the single biggest risk to the whole agent-facing story and it deserves stating plainly: most vaults have no `type:` frontmatter and no typed links. Mapped to RDF, such a vault is a bag of `vault:linksTo` triples, and OWL-RL over it infers nothing worth querying. The reasoning advantage is real only once notes carry types and relations.

So the plugin ships a starter ontology and SHACL shape file, installed into the vault by a command, defining the vocabulary a second brain actually uses: `Note`, `Person`, `Project`, `Task`, `Source`, `Idea`, `Topic`, with `partOf` (transitive), `relatesTo` (symmetric), `authoredBy`, `references`, and `about`. Transitivity and symmetry are what give the reasoner something to derive. The same command seeds shapes that catch the structural mistakes a growing vault makes (a `Task` with no `partOf`, a `Source` with no URL).

**4. Inferred connections, the visible payoff.** Obsidian shows explicit backlinks. This plugin surfaces **entailed** ones: triples true in the materialised closure that were never asserted. A command computes them for the active note (closure minus asserted set), lists them in the validation panel, and optionally writes them into the note under an "Inferred connections" heading. This is what "gets smarter every day" means concretely, and it is the feature no file-access MCP server can offer.

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
- **Network exposure:** the sidecar binds `127.0.0.1` explicitly. Verified in the engine source: `serve-http` already defaults its `--host` to `127.0.0.1`, so this needs no Rust change; the plugin passes the flag anyway rather than depending on a default.
- **Authentication is mandatory.** Because the router applies permissive CORS and the port is now fixed and documented, an unauthenticated endpoint would be reachable cross-origin from any page in the user's browser. The plugin generates a 32-byte hex token on first run and never starts the engine without one. There is no setting to disable it.
- **Token handling:** stored in the plugin's own settings file inside the vault's `.obsidian` directory, displayed in settings only behind a reveal control, and regenerable with one click (which restarts the engine and invalidates any MCP client config the user copied earlier — the settings copy explains this).

## Honest risks

1. **Entailment over an untyped vault is vacuous.** Highest-severity risk to the agent-facing value proposition, and the reason the starter ontology is a v1 requirement rather than a nicety. A vault whose notes carry no types yields a graph of undifferentiated link triples, and the reasoning story collapses to marketing. Mitigation: ship the starter ontology, make typing a documented first step, and be honest in the README that the payoff scales with how typed the vault is.
2. **Full re-sync cost.** Debounced whole-vault re-sync is O(vault) per edit burst. Comfortable for thousands of notes, not free for tens of thousands. Mitigation: 10-second debounce in v1, incremental sync in v2. Measure on a large vault before claiming a bound.
3. **A second local endpoint is a second attack surface.** Addressed by the mandatory token above, but the honest framing is that we are adding a listening socket to the user's machine and that obliges us to default it shut.
