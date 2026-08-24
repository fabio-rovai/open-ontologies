# Open Ontologies Studio

A desktop ontology engineering environment powered by AI. Built with Tauri 2, React 19, and the [Open Ontologies](https://github.com/fabio-rovai/open-ontologies) engine.

## Features

### 3D Graph Canvas

The graph view renders your OWL ontology as a live 3D force-directed graph powered by `3d-force-graph` (Three.js / WebGL).

**How it works:**

- On connection, the canvas fires two SPARQL queries against the engine's in-memory Oxigraph store:
  - `SELECT ?c ?label WHERE { ?c a owl:Class . OPTIONAL { ?c rdfs:label ?label } }` — fetches all named classes
  - `SELECT ?sub ?parent WHERE { ?sub rdfs:subClassOf ?parent . ?sub a owl:Class }` — fetches the subclass hierarchy
- Nodes are spheres with floating canvas-text label sprites. The selected node is highlighted in amber; others are blue.
- Edges are directed arrows (subClassOf → parent direction) with 60% opacity.
- The graph uses Three.js warm lighting (ambient + directional) on a `#1e1e2e` background.
- A `ResizeObserver` keeps the canvas sized to its container on every layout change.

**Interaction:**

- **Drag** to orbit the scene
- **Scroll** to zoom in/out
- **Click a node** — selects it, flies the camera toward it (800ms ease), and opens the Property Inspector
- **Click background** — deselects
- **Right-click** — opens the Add Class dialog at the cursor position
- **Delete / Backspace** — deletes the selected class (fires two SPARQL DELETE statements, then saves and refreshes)

**Refresh:** After any mutation (agent, inspector, delete, add), `window.__refreshGraph()` is called to re-query the store and redraw. This is also wired to the `engine-ready` Tauri event on startup.

### AI Agent Chat

Natural language ontology engineering via the Claude Agent SDK.

**How it works:**
- The agent runs as a Node.js sidecar process (`src-tauri/sidecars/agent/`), spawned by Tauri 3 seconds after the engine starts.
- The sidecar connects Claude to the Open Ontologies engine via its MCP endpoint (`http://localhost:<port>/mcp`, port 8137 by default, passed down from the Rust host), giving Claude access to all 42 ontology tools (`onto_load`, `onto_query`, `onto_validate`, `onto_lint`, `onto_reason`, `onto_enforce`, `onto_plan`, `onto_apply`, `onto_save`, `onto_diff`, `onto_align`, `onto_embed`, `onto_search`, and more).
- The Tauri Rust backend communicates with the sidecar over stdin/stdout using a simple JSON protocol (`{ type: 'chat', message }` in, `{ type: 'text' | 'tool_call' | 'done' | 'error' }` out).
- Multi-turn sessions are maintained in-memory within the sidecar process (not persisted to disk).
- After any mutation tool call (`onto_load`, `onto_apply`, `onto_reason`, etc.), the frontend detects `mutated: true` in the `done` message and triggers a graph refresh.

**Slash commands:** `/build`, `/expand`, `/validate`, `/reason`, `/query`, `/stats`, `/save`

### Property Inspector

Protégé-style property editing for any selected node.

- Click any literal or URI value to edit it inline (Enter saves, Escape cancels)
- Hover over a triple row to reveal the `×` delete button — removes that triple immediately
- The `+ Add` button at the bottom opens a form with:
  - Predicate quick-pick: `rdfs:label`, `rdfs:comment`, `rdfs:subClassOf`, `owl:equivalentClass`, `skos:definition`, `skos:altLabel`, `owl:deprecated`
  - Value type toggle: **Literal** or **URI**
- All edits call `SPARQL UPDATE` via `POST /api/update`, then `POST /api/save` to persist

### Lineage Panel

Full audit trail of every agent action, stored in SQLite and exposed via `GET /api/lineage`.

- Events are grouped by session with a count badge
- Each row shows an icon, operation badge (color-coded by event type), truncated details, and timestamp
- Event types: `P` plan, `A` apply, `E` enforce, `D` drift, `M` monitor, `AL` align
- Refresh button + Sessions grouping toggle

### Named Save

- The toolbar shows the current working file (`studio-live.ttl` by default)
- Click the `💾` button or press **⌘S** to open a rename input
- Saves to `~/.open-ontologies/<name>.ttl` via `POST /api/save` (no MCP session required)
- All auto-saves after mutations also use REST, avoiding session state issues

## Architecture

```text
React UI (Vite + Tailwind)
  └── Tauri 2 shell
        ├── Engine sidecar  →  open-ontologies (Rust/Axum/Oxigraph)
        │     ├── /mcp              MCP Streamable HTTP (tools/call, initialize…)
        │     ├── /api/stats        GET  — graph statistics (triples, classes, props, individuals)
        │     ├── /api/query        POST { query }        — SPARQL SELECT
        │     ├── /api/update       POST { query }        — SPARQL UPDATE (INSERT/DELETE)
        │     ├── /api/load         POST { path }         — load TTL file into store
        │     ├── /api/save         POST { path, format } — save store to file
        │     └── /api/lineage      GET  — lineage events from SQLite
        └── Agent sidecar   →  Node.js (Claude Agent SDK)
              ├── stdin:  { type: 'chat', message } / { type: 'reset' }
              └── stdout: { type: 'text' | 'tool_call' | 'done' | 'error' | 'session' }
```

**Key design decisions:**

| Decision | Reason |
| --- | --- |
| Reads go through sessionless REST API | No MCP session management needed for SPARQL queries or stats |
| UI writes use REST `/api/update` + `/api/save` | Avoids session lifecycle issues in the Tauri WebKit webview |
| Agent writes go through MCP `tools/call` | The Agent SDK manages its own MCP session; Claude needs the full tool set |
| Shared `Arc<GraphStore>` | All MCP sessions and all REST handlers operate on the same in-memory triple store |
| Agent sidecar over stdin/stdout | Keeps Node.js process isolated; Tauri manages lifecycle (spawns, kills on exit) |

## Persistence

The live working file is `~/.open-ontologies/studio-live.ttl`. On startup the engine loads this file. After every UI edit or agent mutation, the file is re-written via `POST /api/save`. Use **⌘S** to snapshot to a named file.

## Stack

| Layer | Tech |
| --- | --- |
| Desktop shell | Tauri 2 |
| Frontend | React 19, Vite 7, TypeScript 5.8, Tailwind CSS 4 |
| 3D graph | 3d-force-graph 1.79 (Three.js / WebGL) |
| State | Zustand 5 |
| Engine | Rust, Axum 0.8, Oxigraph 0.4 (SPARQL), SQLite |
| MCP | rmcp 1 (Streamable HTTP transport) |
| AI agent | Claude Sonnet 4.6 via Agent SDK (Node.js sidecar) |

## Running

### Prerequisites

- **Rust + Cargo** — install via `curl https://sh.rustup.rs -sSf | sh`
- **Node.js 18+** — install via Homebrew: `brew install node`
- **Tauri CLI** — installed as a dev dependency (`npm install` handles this)
- **Engine binary** — must be built once before first run (see below)

### First-time setup

```bash
# 1. Build the engine binary
cd ../open-ontologies
cargo build --release
# This produces: target/release/open-ontologies
# The studio symlink at src-tauri/binaries/open-ontologies-aarch64-apple-darwin
# already points to it — no extra step needed.

# 2. Install JS dependencies
cd ../open-ontologies-studio
npm install
```

### Start the app

```bash
PATH=/opt/homebrew/bin:/Users/fabio/.cargo/bin:$PATH npm run tauri dev
```

The `PATH` prefix is required because the macOS default shell PATH doesn't include Homebrew (`/opt/homebrew/bin`) or Cargo (`~/.cargo/bin`), which Tauri needs to find `node` and `cargo` when it spawns subprocesses.

Tauri will:

1. Compile the Rust shell (first run takes ~1 min, subsequent runs are fast)
2. Start the Vite dev server on `localhost:1420` with HMR
3. Open the app window
4. Spawn the engine sidecar (`open-ontologies serve-http --port 8137` by default, configurable via `OPEN_ONTOLOGIES_STUDIO_PORT`)
5. Spawn the agent sidecar (Node.js, after a 3s delay)

## Development

```bash
# Rebuild engine after changes to open-ontologies/
cd ../open-ontologies && /Users/fabio/.cargo/bin/cargo build --release
```

The engine sidecar binary is a symlink:

```text
src-tauri/binaries/open-ontologies-aarch64-apple-darwin
  → ../../open-ontologies/target/release/open-ontologies
```

The agent sidecar is pre-bundled at `src-tauri/sidecars/agent/dist/index.js`. Rebuild after changes to `index.ts`:

```bash
cd src-tauri/sidecars/agent && npm run build
```

## Keyboard Shortcuts

| Shortcut | Action |
| --- | --- |
| ⌘J | Toggle Chat panel |
| ⌘I | Toggle Inspector panel |
| ⌘S | Save As… |
| Delete / Backspace | Delete selected node |
