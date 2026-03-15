# Open Ontologies Studio — Tauri Redesign

**Date:** 2026-03-14
**Status:** Approved
**Supersedes:** 2026-03-14-studio-architecture-redesign.md

## Problem

The Flutter-based Studio had the wrong architecture: it either tried to embed Claude API directly (bypassing the engine) or reduced the chat to a structured command interface. The user wants:

1. A graph canvas (Protégé-like) where manual edits are validated instantly
2. A chat interface powered by Claude via Agent SDK (subagent pattern, no raw API calls)
3. The engine running "under the hood" — not a separate process the user manages
4. Rust-level robustness for the critical path (edit → validate → accept/reject)

## Architecture

**Single Tauri binary** with three layers:

```
Tauri App (single distributable binary)
│
├── Rust Backend (tauri core)
│   ├── Engine (embedded as Rust library)
│   │   └── 42 MCP tools, 5 prompts, Oxigraph store, SQLite state
│   ├── Graph edit validation (direct function calls, microsecond latency)
│   ├── Tauri commands for frontend IPC
│   └── Sidecar supervisor (manages Agent SDK process)
│
├── Agent SDK Sidecar (Node.js, managed by Tauri)
│   ├── Connects to engine via MCP (stdio or HTTP loopback)
│   ├── Creates Claude subagents with all 42 engine tools
│   ├── Handles chat conversation state
│   └── Auto-restarted by Rust if it crashes
│
└── Web Frontend (React + Cytoscape.js)
    ├── Graph canvas — visual ontology editor
    ├── Chat panel — Claude-powered with starter prompts
    └── Property inspector — selected node details
```

## Two Data Paths

### Path 1: Visual Edits (fast, robust, no Claude)

```
User edits graph → Frontend → Tauri IPC → Rust backend
  → engine.onto_validate()  // syntax check
  → engine.onto_lint()      // quality check
  → engine.onto_enforce()   // pattern check
  → engine.onto_reason()    // consistency check (optional, heavier)
  → Legal? → apply to store, update frontend
  → Illegal? → reject, show error with explanation
```

All Rust. No subprocess, no network. Direct library calls.

### Path 2: Chat (Claude via Agent SDK)

```
User types message → Frontend → Tauri IPC → Rust backend
  → relay to Agent SDK sidecar → Claude subagent
  → Claude decides which engine tools to call
  → Agent SDK calls engine MCP tools
  → Results flow back → Frontend updates chat + graph
```

Only chat goes through the sidecar. If sidecar crashes, graph editing still works.

## Frontend Components

### Graph Canvas (Cytoscape.js)

- Visualize ontology as interactive graph (classes, properties, relationships)
- Click to select nodes → property inspector opens
- Double-click to edit labels
- Drag to rearrange layout
- Right-click context menu: add class, add property, delete
- Shift+drag between nodes to create relationships (subClassOf, etc.)
- Every edit validated before commit — illegal moves blocked with error message
- Auto-refresh after chat mutations
- Layout algorithms: force-directed, hierarchical, circular

### Chat Panel

- Claude-powered via Agent SDK subagent
- Starter chips on empty state:
  - "Build an ontology about..."
  - "Expand current ontology with..."
  - "Validate this graph"
  - "Explain why X is inconsistent"
  - "Ingest data from CSV/JSON"
  - "Compare with another ontology"
- Contextual suggestions based on graph state:
  - Empty graph → build/create prompts
  - Graph with content → expand/validate/reason prompts
  - After errors → explain/fix prompts
- Slash command menu (/build, /expand, /validate, /reason, /enforce, /query)
- Streaming responses
- Tool call visibility (show which engine tools Claude is calling)

### Property Inspector

- Shows details of selected node (URI, label, type, properties)
- Editable fields — changes validated and synced to engine store
- Shows validation status (green check / red X)

## Tech Stack

| Layer | Technology | Why |
|-------|-----------|-----|
| Desktop shell | Tauri v2 | Rust backend, web frontend, sidecar support |
| Engine | open-ontologies (Rust library) | Embedded, no subprocess needed |
| Chat backend | Claude Agent SDK (TypeScript) | Subagent pattern, MCP tool support |
| Frontend framework | React 19 | Standard, large ecosystem |
| Graph visualization | Cytoscape.js | Purpose-built for ontology/graph editing |
| Styling | Tailwind CSS | Fast, utility-first |
| State management | Zustand or Jotai | Lightweight, React-native |
| Build | Vite | Fast dev server, good Tauri integration |

## Engine Integration

The engine (`open-ontologies`) is added as a Rust dependency in `src-tauri/Cargo.toml`:

```toml
[dependencies]
open-ontologies = { path = "../../open-ontologies" }
```

Tauri commands expose engine functions to the frontend:

```rust
#[tauri::command]
fn validate_edit(state: State<Engine>, turtle: String) -> Result<ValidationResult, String> {
    state.graph.load_turtle(&turtle, None)?;
    let lint = state.graph.lint()?;
    let enforce = state.graph.enforce("generic")?;
    Ok(ValidationResult { lint, enforce })
}
```

## Agent SDK Sidecar

A small Node.js script (~100 lines) bundled as a Tauri sidecar:

```typescript
import { Agent } from '@anthropic-ai/agent-sdk';
import { McpServerStdio } from '@anthropic-ai/agent-sdk/mcp';

const engine = new McpServerStdio('open-ontologies', ['serve']);
const agent = new Agent({
  model: 'claude-sonnet-4-6',
  mcpServers: [engine],
  instructions: 'You are an ontology engineering assistant...'
});

// Listen for messages from Tauri, relay to agent, return responses
```

Tauri manages the sidecar lifecycle — starts it on app launch, restarts on crash.

## API Key Management

- Stored locally in the Tauri app's config directory
- Settings dialog in the UI for entering/updating the key
- Key never touches the frontend — Rust backend passes it to the sidecar via environment variable
- Sidecar reads `ANTHROPIC_API_KEY` from env

## Project Structure

```
open-ontologies-studio/
├── src/                          # React frontend
│   ├── App.tsx
│   ├── components/
│   │   ├── GraphCanvas.tsx       # Cytoscape.js graph editor
│   │   ├── ChatPanel.tsx         # Chat with starter prompts
│   │   ├── PropertyInspector.tsx  # Node details editor
│   │   ├── StarterChips.tsx      # Example prompt buttons
│   │   └── SlashMenu.tsx         # Command menu
│   ├── hooks/
│   │   ├── useEngine.ts          # Tauri IPC to engine
│   │   ├── useChat.ts            # Tauri IPC to Agent SDK
│   │   └── useGraph.ts           # Graph state management
│   └── lib/
│       └── tauri.ts              # Tauri command bindings
├── src-tauri/                    # Rust backend
│   ├── Cargo.toml                # Depends on open-ontologies
│   ├── src/
│   │   ├── main.rs               # Tauri app setup
│   │   ├── commands.rs           # Tauri commands (validate, query, etc.)
│   │   └── sidecar.rs            # Agent SDK sidecar management
│   ├── tauri.conf.json
│   └── sidecars/
│       └── agent/                # Agent SDK Node.js sidecar
│           ├── package.json
│           ├── index.ts
│           └── tsconfig.json
├── package.json                  # Frontend deps
├── vite.config.ts
├── tailwind.config.js
└── tsconfig.json
```

## What Gets Deleted

The entire Flutter project is replaced. Key decisions:
- No Flutter (Dart) — replaced by React (TypeScript) + Tauri (Rust)
- No custom MCP HTTP client — engine embedded as library
- No command_processor.dart — chat handled by Agent SDK
- No polling timer — Tauri commands are synchronous, graph refreshes on mutation

## Success Criteria

1. One binary — user double-clicks app, everything starts
2. Graph canvas shows ontology, supports visual editing with instant validation
3. Chat creates/expands/validates ontologies via Claude subagent
4. Illegal graph edits are blocked with clear error messages
5. Engine runs embedded — no separate process to manage
6. API key stored securely, never exposed to frontend
