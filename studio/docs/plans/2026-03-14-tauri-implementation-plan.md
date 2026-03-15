# Open Ontologies Studio — Tauri Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the Flutter Studio with a Tauri desktop app — React frontend (Cytoscape.js graph canvas + chat panel), Rust backend managing the engine subprocess and Agent SDK sidecar.

**Architecture:** Tauri spawns the engine binary (`open-ontologies serve-http --port 8080`) as a supervised sidecar. Frontend talks to engine via MCP HTTP for graph operations. Chat goes through a Node.js Agent SDK sidecar (stdin/stdout) that connects to the same engine. One engine instance, two clients (frontend + Agent SDK).

**Tech Stack:** Tauri v2, React 19, Vite, TypeScript, Cytoscape.js, @anthropic-ai/claude-agent-sdk, @modelcontextprotocol/sdk, Tailwind CSS, Zustand

---

## Phase 1: Project Scaffolding

### Task 1: Create Tauri + React + Vite project

**Files:**
- Create: entire project scaffold in `/Users/fabio/projects/open-ontologies-studio/` (after moving Flutter files)

**Step 1: Back up Flutter project**

```bash
cd /Users/fabio/projects
mv open-ontologies-studio open-ontologies-studio-flutter-backup
```

**Step 2: Scaffold Tauri app**

```bash
cd /Users/fabio/projects
npm create tauri-app@latest open-ontologies-studio -- --template react-ts
cd open-ontologies-studio
```

Choose: React, TypeScript, npm

**Step 3: Install frontend dependencies**

```bash
npm install cytoscape @types/cytoscape zustand tailwindcss @tailwindcss/vite
```

**Step 4: Install Tailwind — add to vite.config.ts**

```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    host: host || false,
    port: 1420,
    strictPort: true,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
}));
```

**Step 5: Add Tailwind to src/index.css**

```css
@import "tailwindcss";
```

**Step 6: Verify it runs**

```bash
npm run tauri dev
```

Expected: Tauri window opens with default React template

**Step 7: Commit**

```bash
git init && git add -A && git commit -m "feat: scaffold Tauri + React + Vite project"
```

---

### Task 2: Set up dark theme and app layout shell

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/index.css`
- Create: `src/components/Layout.tsx`

**Step 1: Set up dark theme in index.css**

```css
@import "tailwindcss";

:root {
  --bg-primary: #1e1e2e;
  --bg-secondary: #252536;
  --bg-panel: #2a2a3d;
  --text-primary: #cdd6f4;
  --text-secondary: #a6adc8;
  --accent: #89b4fa;
  --border: #45475a;
  --error: #f38ba8;
  --success: #a6e3a1;
}

body {
  margin: 0;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
}
```

**Step 2: Create Layout component**

```typescript
// src/components/Layout.tsx
import { useState } from 'react';

export function Layout() {
  const [showChat, setShowChat] = useState(true);
  const [showInspector, setShowInspector] = useState(false);

  return (
    <div className="h-screen flex flex-col" style={{ background: 'var(--bg-primary)' }}>
      {/* Toolbar */}
      <div className="h-10 flex items-center px-4 border-b"
           style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)' }}>
        <span className="text-sm font-semibold" style={{ color: 'var(--accent)' }}>
          Open Ontologies Studio
        </span>
        <div className="ml-auto flex gap-2">
          <button onClick={() => setShowChat(!showChat)}
                  className="text-xs px-2 py-1 rounded"
                  style={{ background: showChat ? 'var(--accent)' : 'var(--bg-panel)',
                           color: showChat ? 'var(--bg-primary)' : 'var(--text-secondary)' }}>
            Chat
          </button>
          <button onClick={() => setShowInspector(!showInspector)}
                  className="text-xs px-2 py-1 rounded"
                  style={{ background: showInspector ? 'var(--accent)' : 'var(--bg-panel)',
                           color: showInspector ? 'var(--bg-primary)' : 'var(--text-secondary)' }}>
            Inspector
          </button>
        </div>
      </div>

      {/* Main area */}
      <div className="flex-1 flex overflow-hidden">
        {/* Graph canvas placeholder */}
        <div className="flex-1 relative">
          <div className="absolute inset-0 flex items-center justify-center"
               style={{ color: 'var(--text-secondary)' }}>
            Graph Canvas
          </div>
        </div>

        {/* Inspector panel */}
        {showInspector && (
          <div className="w-72 border-l" style={{ borderColor: 'var(--border)', background: 'var(--bg-panel)' }}>
            <div className="p-3 text-sm" style={{ color: 'var(--text-secondary)' }}>
              Property Inspector
            </div>
          </div>
        )}

        {/* Chat panel */}
        {showChat && (
          <div className="w-96 border-l flex flex-col"
               style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)' }}>
            <div className="p-3 text-sm" style={{ color: 'var(--text-secondary)' }}>
              Chat Panel
            </div>
          </div>
        )}
      </div>

      {/* Status bar */}
      <div className="h-6 flex items-center px-4 text-xs border-t"
           style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)',
                    color: 'var(--text-secondary)' }}>
        <span>Disconnected</span>
      </div>
    </div>
  );
}
```

**Step 3: Wire up App.tsx**

```typescript
// src/App.tsx
import { Layout } from './components/Layout';

function App() {
  return <Layout />;
}

export default App;
```

**Step 4: Verify**

```bash
npm run tauri dev
```

Expected: Dark-themed window with toolbar, graph placeholder, chat panel, status bar

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: dark theme layout with graph, chat, inspector panels"
```

---

## Phase 2: Engine Process Management

### Task 3: Add Tauri shell plugin and spawn engine sidecar

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/tauri.conf.json`

**Step 1: Add shell plugin**

```bash
cd /Users/fabio/projects/open-ontologies-studio
npm install @tauri-apps/plugin-shell
```

```bash
cd src-tauri
cargo add tauri-plugin-shell
```

**Step 2: Configure sidecar in tauri.conf.json**

Add to the `bundle` section:

```json
{
  "bundle": {
    "externalBin": ["binaries/open-ontologies"]
  }
}
```

**Step 3: Create sidecar binary symlink**

The engine binary must be named with the platform triple. On macOS ARM:

```bash
mkdir -p src-tauri/binaries
ln -s /Users/fabio/projects/open-ontologies/target/release/open-ontologies \
      src-tauri/binaries/open-ontologies-aarch64-apple-darwin
```

**Step 4: Add shell permissions in capabilities/default.json**

```json
{
  "permissions": [
    "core:default",
    "shell:allow-spawn",
    "shell:allow-stdin-write",
    {
      "identifier": "shell:allow-execute",
      "allow": [{ "name": "binaries/open-ontologies", "sidecar": true }]
    }
  ]
}
```

**Step 5: Register shell plugin in lib.rs**

```rust
// src-tauri/src/lib.rs
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 6: Verify engine binary exists**

```bash
ls -la src-tauri/binaries/
```

Expected: symlink to open-ontologies binary

**Step 7: Commit**

```bash
git add -A && git commit -m "feat: configure engine as Tauri sidecar with shell plugin"
```

---

### Task 4: Spawn engine on app start and manage lifecycle

**Files:**
- Create: `src-tauri/src/engine.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Create engine manager**

```rust
// src-tauri/src/engine.rs
use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct EngineState {
    pub running: Arc<AtomicBool>,
}

pub fn spawn_engine(app: &tauri::AppHandle) -> Result<(), String> {
    let shell = app.shell();
    let cmd = shell
        .sidecar("binaries/open-ontologies")
        .map_err(|e| format!("Failed to create sidecar command: {e}"))?
        .args(["serve-http", "--port", "8080"]);

    let (mut rx, _child) = cmd.spawn().map_err(|e| format!("Failed to spawn engine: {e}"))?;

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                    let text = String::from_utf8_lossy(&line);
                    eprintln!("[engine stdout] {}", text);
                }
                tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                    let text = String::from_utf8_lossy(&line);
                    eprintln!("[engine stderr] {}", text);
                    // Emit connection status to frontend
                    if text.contains("listening") || text.contains("Listening") {
                        let _ = app_handle.emit("engine-ready", true);
                    }
                }
                tauri_plugin_shell::process::CommandEvent::Terminated(status) => {
                    eprintln!("[engine] terminated with status: {:?}", status);
                    let _ = app_handle.emit("engine-stopped", true);
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(())
}
```

**Step 2: Wire into lib.rs**

```rust
// src-tauri/src/lib.rs
mod engine;

use engine::EngineState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[tauri::command]
fn engine_status(state: tauri::State<EngineState>) -> bool {
    state.running.load(Ordering::Relaxed)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(EngineState {
            running: Arc::new(AtomicBool::new(false)),
        })
        .setup(|app| {
            let handle = app.handle().clone();
            // Spawn engine on startup
            if let Err(e) = engine::spawn_engine(&handle) {
                eprintln!("Failed to start engine: {e}");
            } else {
                app.state::<EngineState>().running.store(true, Ordering::Relaxed);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![engine_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 3: Verify**

```bash
npm run tauri dev
```

Expected: Tauri window opens, terminal shows `[engine stderr] listening on 127.0.0.1:8080` (or similar)

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: spawn engine on app start with lifecycle management"
```

---

## Phase 3: MCP Client for Frontend

### Task 5: Create TypeScript MCP client

**Files:**
- Create: `src/lib/mcp-client.ts`

**Step 1: Install MCP SDK**

```bash
npm install @modelcontextprotocol/sdk
```

**Step 2: Create MCP client**

This client talks to the engine's HTTP MCP endpoint at localhost:8080.

```typescript
// src/lib/mcp-client.ts

const ENGINE_URL = 'http://localhost:8080/mcp';

let requestId = 0;
let sessionId: string | null = null;

function nextId(): number {
  return ++requestId;
}

async function mcpRequest(method: string, params: Record<string, unknown> = {}): Promise<unknown> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    'Accept': 'text/event-stream',
  };
  if (sessionId) {
    headers['Mcp-Session-Id'] = sessionId;
  }

  const body = JSON.stringify({
    jsonrpc: '2.0',
    id: nextId(),
    method,
    params,
  });

  const resp = await fetch(ENGINE_URL, { method: 'POST', headers, body });

  // Capture session ID from response
  const sid = resp.headers.get('Mcp-Session-Id');
  if (sid) sessionId = sid;

  const contentType = resp.headers.get('Content-Type') || '';

  if (contentType.includes('text/event-stream')) {
    return parseSSE(await resp.text());
  }
  return resp.json();
}

function parseSSE(text: string): unknown {
  const lines = text.split('\n');
  for (const line of lines) {
    if (line.startsWith('data: ')) {
      const data = line.slice(6).trim();
      if (data) {
        try {
          const parsed = JSON.parse(data);
          if (parsed.result !== undefined) return parsed.result;
          if (parsed.error) throw new Error(parsed.error.message || JSON.stringify(parsed.error));
          return parsed;
        } catch {
          // Try next line
        }
      }
    }
  }
  // Fallback: try parsing the whole text as JSON
  try {
    const parsed = JSON.parse(text);
    return parsed.result ?? parsed;
  } catch {
    throw new Error(`Failed to parse engine response: ${text.slice(0, 200)}`);
  }
}

// --- Public API ---

export async function initialize(): Promise<boolean> {
  try {
    await mcpRequest('initialize', {
      protocolVersion: '2025-03-26',
      capabilities: {},
      clientInfo: { name: 'open-ontologies-studio', version: '1.0.0' },
    });
    await mcpRequest('notifications/initialized', {});
    return true;
  } catch (e) {
    console.error('MCP init failed:', e);
    return false;
  }
}

export async function callTool(name: string, args: Record<string, unknown> = {}): Promise<string> {
  const result = await mcpRequest('tools/call', { name, arguments: args }) as {
    content?: Array<{ type: string; text?: string }>;
  };
  if (result?.content) {
    return result.content
      .filter((c: { type: string }) => c.type === 'text')
      .map((c: { text?: string }) => c.text || '')
      .join('\n');
  }
  return JSON.stringify(result);
}

export async function listTools(): Promise<Array<{ name: string; description: string; inputSchema: unknown }>> {
  const result = await mcpRequest('tools/list', {}) as {
    tools?: Array<{ name: string; description: string; inputSchema: unknown }>;
  };
  return result?.tools || [];
}

export async function getStats(): Promise<{ triples: number; classes: number; properties: number; individuals: number }> {
  const text = await callTool('onto_stats');
  try {
    return JSON.parse(text);
  } catch {
    return { triples: 0, classes: 0, properties: 0, individuals: 0 };
  }
}

export async function loadTurtle(turtle: string): Promise<string> {
  return callTool('onto_load', { turtle });
}

export async function sparqlQuery(query: string): Promise<string> {
  return callTool('onto_query', { query });
}

export async function validate(turtle: string): Promise<string> {
  return callTool('onto_validate', { input: turtle, inline: true });
}

export async function lint(turtle: string): Promise<string> {
  return callTool('onto_lint', { input: turtle, inline: true });
}

export async function enforce(rulePack: string): Promise<string> {
  return callTool('onto_enforce', { rule_pack: rulePack });
}

export async function clearStore(): Promise<string> {
  return callTool('onto_clear');
}

export async function getSessionId(): string | null {
  return sessionId;
}
```

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: TypeScript MCP client for engine communication"
```

---

### Task 6: Create engine connection hook and status bar

**Files:**
- Create: `src/hooks/useEngine.ts`
- Modify: `src/components/Layout.tsx`

**Step 1: Create useEngine hook**

```typescript
// src/hooks/useEngine.ts
import { create } from 'zustand';
import { listen } from '@tauri-apps/api/event';
import * as mcp from '../lib/mcp-client';

type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

interface EngineStore {
  status: ConnectionStatus;
  stats: { triples: number; classes: number; properties: number; individuals: number } | null;
  error: string | null;
  connect: () => Promise<void>;
  refreshStats: () => Promise<void>;
}

export const useEngine = create<EngineStore>((set, get) => ({
  status: 'disconnected',
  stats: null,
  error: null,

  connect: async () => {
    set({ status: 'connecting', error: null });

    // Retry connection (engine may still be starting)
    for (let i = 0; i < 10; i++) {
      try {
        const ok = await mcp.initialize();
        if (ok) {
          set({ status: 'connected' });
          await get().refreshStats();
          return;
        }
      } catch {
        // Engine not ready yet
      }
      await new Promise(r => setTimeout(r, 1000));
    }

    set({ status: 'error', error: 'Could not connect to engine after 10 attempts' });
  },

  refreshStats: async () => {
    try {
      const stats = await mcp.getStats();
      set({ stats });
    } catch (e) {
      console.error('Failed to refresh stats:', e);
    }
  },
}));

// Listen for engine lifecycle events from Tauri
listen('engine-ready', () => {
  useEngine.getState().connect();
});

listen('engine-stopped', () => {
  useEngine.setState({ status: 'disconnected', error: 'Engine stopped' });
});
```

**Step 2: Update Layout with real status bar**

Replace the status bar `<span>Disconnected</span>` in Layout.tsx with:

```typescript
// At top of Layout.tsx
import { useEngine } from '../hooks/useEngine';
import { useEffect } from 'react';

// Inside Layout component
const { status, stats, connect } = useEngine();

useEffect(() => {
  connect();
}, [connect]);

// In the status bar div:
<div className="h-6 flex items-center px-4 text-xs border-t gap-4"
     style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)',
              color: 'var(--text-secondary)' }}>
  <span style={{ color: status === 'connected' ? 'var(--success)' :
                        status === 'error' ? 'var(--error)' : 'var(--text-secondary)' }}>
    {status === 'connected' ? 'Connected' :
     status === 'connecting' ? 'Connecting...' :
     status === 'error' ? 'Error' : 'Disconnected'}
  </span>
  {stats && (
    <span>{stats.triples} triples | {stats.classes} classes | {stats.properties} properties</span>
  )}
</div>
```

**Step 3: Verify**

```bash
npm run tauri dev
```

Expected: Status bar shows "Connecting..." then "Connected" with stats (if engine has data) or zeros

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: engine connection hook with auto-connect and status bar"
```

---

## Phase 4: Graph Canvas

### Task 7: Create Cytoscape.js graph canvas with ontology loading

**Files:**
- Create: `src/components/GraphCanvas.tsx`
- Modify: `src/components/Layout.tsx`

**Step 1: Create GraphCanvas component**

```typescript
// src/components/GraphCanvas.tsx
import { useEffect, useRef, useCallback } from 'react';
import cytoscape, { Core, NodeSingular } from 'cytoscape';
import * as mcp from '../lib/mcp-client';
import { useEngine } from '../hooks/useEngine';

interface GraphCanvasProps {
  onNodeSelect: (node: { id: string; label: string; uri: string } | null) => void;
}

export function GraphCanvas({ onNodeSelect }: GraphCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const cyRef = useRef<Core | null>(null);
  const { status, refreshStats } = useEngine();

  // Initialize Cytoscape
  useEffect(() => {
    if (!containerRef.current) return;

    const cy = cytoscape({
      container: containerRef.current,
      style: [
        {
          selector: 'node',
          style: {
            'background-color': '#89b4fa',
            'label': 'data(label)',
            'color': '#cdd6f4',
            'font-size': '11px',
            'text-valign': 'bottom',
            'text-margin-y': 5,
            'width': 30,
            'height': 30,
            'border-width': 2,
            'border-color': '#585b70',
          },
        },
        {
          selector: 'node:selected',
          style: {
            'background-color': '#f9e2af',
            'border-color': '#f9e2af',
          },
        },
        {
          selector: 'edge',
          style: {
            'width': 2,
            'line-color': '#585b70',
            'target-arrow-color': '#585b70',
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
            'label': 'data(label)',
            'font-size': '9px',
            'color': '#a6adc8',
            'text-rotation': 'autorotate',
          },
        },
      ],
      layout: { name: 'preset' },
      minZoom: 0.1,
      maxZoom: 5,
    });

    cy.on('tap', 'node', (evt) => {
      const node = evt.target;
      onNodeSelect({
        id: node.id(),
        label: node.data('label'),
        uri: node.data('uri'),
      });
    });

    cy.on('tap', (evt) => {
      if (evt.target === cy) onNodeSelect(null);
    });

    cyRef.current = cy;

    return () => { cy.destroy(); };
  }, [onNodeSelect]);

  // Load graph data when connected
  const loadGraph = useCallback(async () => {
    if (status !== 'connected' || !cyRef.current) return;

    try {
      // Query all classes
      const classesJson = await mcp.sparqlQuery(
        `PREFIX owl: <http://www.w3.org/2002/07/owl#>
         PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
         SELECT ?c ?label WHERE {
           ?c a owl:Class .
           OPTIONAL { ?c rdfs:label ?label }
           FILTER(!isBlank(?c))
         }`
      );

      // Query all subClassOf edges
      const edgesJson = await mcp.sparqlQuery(
        `PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
         PREFIX owl: <http://www.w3.org/2002/07/owl#>
         SELECT ?sub ?super WHERE {
           ?sub rdfs:subClassOf ?super .
           ?sub a owl:Class .
           ?super a owl:Class .
           FILTER(!isBlank(?sub) && !isBlank(?super))
         }`
      );

      const cy = cyRef.current;
      cy.elements().remove();

      // Parse SPARQL JSON results and add nodes
      const classes = parseSparqlResults(classesJson);
      for (const row of classes) {
        const uri = row.c;
        const label = row.label || shortUri(uri);
        cy.add({
          group: 'nodes',
          data: { id: uri, label, uri },
        });
      }

      // Add edges
      const edges = parseSparqlResults(edgesJson);
      for (const row of edges) {
        cy.add({
          group: 'edges',
          data: {
            id: `${row.sub}->${row.super}`,
            source: row.sub,
            target: row.super,
            label: 'subClassOf',
          },
        });
      }

      // Apply layout
      if (cy.nodes().length > 0) {
        cy.layout({
          name: 'breadthfirst',
          directed: true,
          spacingFactor: 1.5,
          animate: true,
          animationDuration: 500,
        }).run();
        cy.fit(undefined, 50);
      }
    } catch (e) {
      console.error('Failed to load graph:', e);
    }
  }, [status]);

  useEffect(() => { loadGraph(); }, [loadGraph]);

  // Expose refresh to parent
  useEffect(() => {
    (window as unknown as Record<string, unknown>).__refreshGraph = loadGraph;
  }, [loadGraph]);

  return (
    <div
      ref={containerRef}
      className="absolute inset-0"
      style={{ background: 'var(--bg-primary)' }}
    />
  );
}

// --- Helpers ---

function parseSparqlResults(text: string): Array<Record<string, string>> {
  try {
    const data = JSON.parse(text);
    if (data.results?.bindings) {
      return data.results.bindings.map((b: Record<string, { value: string }>) => {
        const row: Record<string, string> = {};
        for (const [key, val] of Object.entries(b)) {
          row[key] = val.value;
        }
        return row;
      });
    }
    // Engine may return pre-formatted results
    if (Array.isArray(data)) return data;
    return [];
  } catch {
    return [];
  }
}

function shortUri(uri: string): string {
  const hash = uri.lastIndexOf('#');
  if (hash >= 0) return uri.slice(hash + 1);
  const slash = uri.lastIndexOf('/');
  if (slash >= 0) return uri.slice(slash + 1);
  return uri;
}
```

**Step 2: Wire into Layout**

Replace graph placeholder div with:

```typescript
import { GraphCanvas } from './GraphCanvas';
import { useState } from 'react';

// In Layout:
const [selectedNode, setSelectedNode] = useState<{ id: string; label: string; uri: string } | null>(null);

// Replace graph placeholder:
<div className="flex-1 relative">
  <GraphCanvas onNodeSelect={setSelectedNode} />
</div>
```

**Step 3: Verify**

```bash
npm run tauri dev
```

Expected: Empty graph canvas (dark background). If engine has an ontology loaded, nodes appear with breadthfirst layout.

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: Cytoscape.js graph canvas with SPARQL-driven ontology rendering"
```

---

### Task 8: Add visual graph editing with validation

**Files:**
- Modify: `src/components/GraphCanvas.tsx`
- Create: `src/components/AddClassDialog.tsx`

**Step 1: Create AddClassDialog**

```typescript
// src/components/AddClassDialog.tsx
import { useState } from 'react';

interface Props {
  position: { x: number; y: number };
  onSubmit: (name: string) => void;
  onCancel: () => void;
}

export function AddClassDialog({ position, onSubmit, onCancel }: Props) {
  const [name, setName] = useState('');

  return (
    <div className="fixed inset-0 z-50" onClick={onCancel}>
      <div
        className="absolute p-3 rounded shadow-lg"
        style={{
          left: position.x,
          top: position.y,
          background: 'var(--bg-panel)',
          border: '1px solid var(--border)',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <input
          autoFocus
          placeholder="Class name..."
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && name.trim()) onSubmit(name.trim());
            if (e.key === 'Escape') onCancel();
          }}
          className="w-48 px-2 py-1 text-sm rounded outline-none"
          style={{
            background: 'var(--bg-primary)',
            color: 'var(--text-primary)',
            border: '1px solid var(--border)',
          }}
        />
        <div className="text-xs mt-1" style={{ color: 'var(--text-secondary)' }}>
          Enter to create, Esc to cancel
        </div>
      </div>
    </div>
  );
}
```

**Step 2: Add context menu, edge creation, and validation to GraphCanvas**

Add these to GraphCanvas.tsx:

```typescript
// Add to imports
import { useState } from 'react';
import { AddClassDialog } from './AddClassDialog';

// Add state inside GraphCanvas:
const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
const [validationError, setValidationError] = useState<string | null>(null);

// Add right-click handler after cy initialization:
cy.on('cxttap', (evt) => {
  if (evt.target === cy) {
    const pos = evt.renderedPosition || evt.position;
    setContextMenu({ x: pos.x, y: pos.y });
  }
});

// Add edge creation (shift+drag):
let edgeSource: NodeSingular | null = null;

cy.on('mousedown', 'node', (evt) => {
  if (evt.originalEvent.shiftKey) {
    edgeSource = evt.target;
    evt.target.style('border-color', '#f9e2af');
  }
});

cy.on('mouseup', 'node', async (evt) => {
  if (edgeSource && edgeSource !== evt.target) {
    const sourceUri = edgeSource.data('uri');
    const targetUri = evt.target.data('uri');
    await createEdge(sourceUri, targetUri);
  }
  if (edgeSource) {
    edgeSource.style('border-color', '#585b70');
    edgeSource = null;
  }
});

// Add class creation function:
const addClass = async (name: string) => {
  setContextMenu(null);
  const baseIri = 'http://example.org/ontology#';
  const classUri = `${baseIri}${name.replace(/\s+/g, '_')}`;
  const turtle = `
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    <${classUri}> a owl:Class ; rdfs:label "${name}" .
  `;

  try {
    // Validate first
    const validation = await mcp.validate(turtle);
    if (validation.toLowerCase().includes('error')) {
      setValidationError(`Invalid class: ${validation}`);
      setTimeout(() => setValidationError(null), 5000);
      return;
    }

    // Load into store
    await mcp.loadTurtle(turtle);
    await loadGraph();
    await refreshStats();
    setValidationError(null);
  } catch (e) {
    setValidationError(`Failed to add class: ${e}`);
    setTimeout(() => setValidationError(null), 5000);
  }
};

// Edge creation function:
const createEdge = async (sourceUri: string, targetUri: string) => {
  const turtle = `
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    <${sourceUri}> rdfs:subClassOf <${targetUri}> .
  `;

  try {
    await mcp.loadTurtle(turtle);
    await loadGraph();
    await refreshStats();
  } catch (e) {
    setValidationError(`Failed to create edge: ${e}`);
    setTimeout(() => setValidationError(null), 5000);
  }
};

// Add to the return JSX (after the container div):
{contextMenu && (
  <AddClassDialog
    position={contextMenu}
    onSubmit={addClass}
    onCancel={() => setContextMenu(null)}
  />
)}
{validationError && (
  <div className="absolute bottom-4 left-4 right-4 p-3 rounded text-sm"
       style={{ background: 'var(--error)', color: 'var(--bg-primary)' }}>
    {validationError}
  </div>
)}
```

**Step 3: Verify**

```bash
npm run tauri dev
```

Expected: Right-click on canvas → dialog to add class → node appears. Shift+drag between nodes → subClassOf edge created. Invalid edits show error toast.

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: visual graph editing with right-click add class, shift-drag edges, validation"
```

---

### Task 9: Add node deletion and label editing

**Files:**
- Modify: `src/components/GraphCanvas.tsx`

**Step 1: Add delete on Backspace/Delete key**

Add after cy initialization:

```typescript
// Delete selected node
document.addEventListener('keydown', async (e) => {
  if ((e.key === 'Delete' || e.key === 'Backspace') && !['INPUT', 'TEXTAREA'].includes((e.target as HTMLElement)?.tagName)) {
    const selected = cyRef.current?.nodes(':selected');
    if (selected && selected.length > 0) {
      const uri = selected.first().data('uri');
      try {
        // Remove all triples about this class
        await mcp.sparqlQuery(
          `DELETE WHERE { <${uri}> ?p ?o }` // Note: this uses SPARQL UPDATE
        );
        await mcp.sparqlQuery(
          `DELETE WHERE { ?s ?p <${uri}> }`
        );
        await loadGraph();
        await refreshStats();
        onNodeSelect(null);
      } catch (e) {
        setValidationError(`Failed to delete: ${e}`);
        setTimeout(() => setValidationError(null), 5000);
      }
    }
  }
});
```

**Step 2: Add double-click to edit label**

```typescript
// Double-click to rename
cy.on('dbltap', 'node', (evt) => {
  const node = evt.target;
  const currentLabel = node.data('label');
  const newLabel = prompt('Edit class label:', currentLabel);
  if (newLabel && newLabel !== currentLabel) {
    const uri = node.data('uri');
    // Update label via SPARQL
    mcp.sparqlQuery(
      `PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
       DELETE { <${uri}> rdfs:label ?old }
       INSERT { <${uri}> rdfs:label "${newLabel}" }
       WHERE { OPTIONAL { <${uri}> rdfs:label ?old } }`
    ).then(() => loadGraph());
  }
});
```

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: delete nodes with Backspace, double-click to edit labels"
```

---

## Phase 5: Agent SDK Sidecar

### Task 10: Create Agent SDK sidecar script

**Files:**
- Create: `src-tauri/sidecars/agent/package.json`
- Create: `src-tauri/sidecars/agent/tsconfig.json`
- Create: `src-tauri/sidecars/agent/index.ts`

**Step 1: Create sidecar package**

```json
// src-tauri/sidecars/agent/package.json
{
  "name": "ontology-agent-sidecar",
  "version": "1.0.0",
  "private": true,
  "type": "module",
  "main": "dist/index.js",
  "scripts": {
    "build": "tsc",
    "start": "node dist/index.js"
  },
  "dependencies": {
    "@anthropic-ai/sdk": "^0.39.0"
  },
  "devDependencies": {
    "typescript": "^5.7.0",
    "@types/node": "^22.0.0"
  }
}
```

```json
// src-tauri/sidecars/agent/tsconfig.json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "Node16",
    "moduleResolution": "Node16",
    "outDir": "dist",
    "rootDir": ".",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true
  },
  "include": ["*.ts"]
}
```

**Step 2: Create the sidecar agent script**

```typescript
// src-tauri/sidecars/agent/index.ts
import Anthropic from '@anthropic-ai/sdk';
import * as readline from 'readline';

// --- MCP Client (connects to engine HTTP) ---

const ENGINE_URL = 'http://localhost:8080/mcp';
let sessionId: string | null = null;
let reqId = 0;

async function mcpCall(method: string, params: Record<string, unknown> = {}): Promise<unknown> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    'Accept': 'text/event-stream',
  };
  if (sessionId) headers['Mcp-Session-Id'] = sessionId;

  const resp = await fetch(ENGINE_URL, {
    method: 'POST',
    headers,
    body: JSON.stringify({ jsonrpc: '2.0', id: ++reqId, method, params }),
  });

  const sid = resp.headers.get('Mcp-Session-Id');
  if (sid) sessionId = sid;

  const text = await resp.text();
  // Parse SSE
  for (const line of text.split('\n')) {
    if (line.startsWith('data: ')) {
      try {
        const parsed = JSON.parse(line.slice(6));
        if (parsed.result !== undefined) return parsed.result;
        return parsed;
      } catch { /* next line */ }
    }
  }
  try { const p = JSON.parse(text); return p.result ?? p; } catch { return text; }
}

async function initMcp(): Promise<boolean> {
  try {
    await mcpCall('initialize', {
      protocolVersion: '2025-03-26',
      capabilities: {},
      clientInfo: { name: 'ontology-agent', version: '1.0.0' },
    });
    await mcpCall('notifications/initialized');
    return true;
  } catch { return false; }
}

async function listMcpTools(): Promise<Anthropic.Tool[]> {
  const result = await mcpCall('tools/list') as {
    tools?: Array<{ name: string; description: string; inputSchema: Record<string, unknown> }>;
  };
  return (result?.tools || []).map(t => ({
    name: t.name,
    description: t.description || '',
    input_schema: t.inputSchema as Anthropic.Tool.InputSchema,
  }));
}

async function callMcpTool(name: string, input: Record<string, unknown>): Promise<string> {
  const result = await mcpCall('tools/call', { name, arguments: input }) as {
    content?: Array<{ type: string; text?: string }>;
  };
  if (result?.content) {
    return result.content.filter(c => c.type === 'text').map(c => c.text || '').join('\n');
  }
  return JSON.stringify(result);
}

// --- Agent Loop ---

const client = new Anthropic();
let conversationHistory: Anthropic.MessageParam[] = [];

const SYSTEM_PROMPT = `You are an ontology engineering assistant. You have access to the Open Ontologies engine with 42 tools for creating, validating, reasoning over, and managing OWL ontologies.

Key tools:
- onto_load: Load Turtle RDF into the store
- onto_query: Run SPARQL queries
- onto_stats: Get ontology statistics
- onto_validate: Check RDF/OWL syntax
- onto_lint: Quality checks
- onto_reason: Run OWL reasoning (rdfs, owl-rl)
- onto_enforce: Check design patterns
- onto_save: Export ontology
- onto_diff: Compare ontologies
- onto_plan: Preview changes (terraform-style)
- onto_apply: Apply planned changes
- onto_version/onto_history/onto_rollback: Version management

When asked to build an ontology, use onto_load to add Turtle RDF to the store.
When asked to validate, use onto_validate and onto_lint.
After mutations, mention what changed so the UI can refresh the graph.`;

async function handleMessage(userMessage: string): Promise<void> {
  conversationHistory.push({ role: 'user', content: userMessage });

  const tools = await listMcpTools();

  // Agent loop — keeps running until Claude stops calling tools
  while (true) {
    const response = await client.messages.create({
      model: 'claude-sonnet-4-6',
      max_tokens: 4096,
      system: SYSTEM_PROMPT,
      tools,
      messages: conversationHistory,
    });

    // Collect all content blocks
    const assistantContent = response.content;
    conversationHistory.push({ role: 'assistant', content: assistantContent });

    // Check for tool use
    const toolUses = assistantContent.filter(
      (b): b is Anthropic.ToolUseBlock => b.type === 'tool_use'
    );

    // Stream text blocks to stdout
    for (const block of assistantContent) {
      if (block.type === 'text' && block.text) {
        send({ type: 'text', content: block.text });
      }
      if (block.type === 'tool_use') {
        send({ type: 'tool_call', tool: block.name, input: block.input });
      }
    }

    if (toolUses.length === 0 || response.stop_reason === 'end_turn') {
      send({ type: 'done', mutated: toolUses.some(t =>
        ['onto_load', 'onto_clear', 'onto_apply', 'onto_reason',
         'onto_rollback', 'onto_ingest', 'onto_extend', 'onto_import',
         'onto_pull', 'onto_enrich'].includes(t.name)
      )});
      break;
    }

    // Execute tool calls and collect results
    const toolResults: Anthropic.ToolResultBlockParam[] = [];
    for (const tu of toolUses) {
      try {
        const result = await callMcpTool(tu.name, tu.input as Record<string, unknown>);
        send({ type: 'tool_result', tool: tu.name, result });
        toolResults.push({ type: 'tool_result', tool_use_id: tu.id, content: result });
      } catch (e) {
        const err = `Error calling ${tu.name}: ${e}`;
        send({ type: 'tool_result', tool: tu.name, result: err });
        toolResults.push({ type: 'tool_result', tool_use_id: tu.id, content: err, is_error: true });
      }
    }

    conversationHistory.push({ role: 'user', content: toolResults });
  }
}

// --- stdin/stdout Protocol ---

function send(msg: Record<string, unknown>): void {
  process.stdout.write(JSON.stringify(msg) + '\n');
}

async function main(): Promise<void> {
  // Wait for engine
  for (let i = 0; i < 15; i++) {
    if (await initMcp()) break;
    await new Promise(r => setTimeout(r, 1000));
  }

  send({ type: 'ready' });

  const rl = readline.createInterface({ input: process.stdin });

  rl.on('line', async (line) => {
    try {
      const msg = JSON.parse(line);
      if (msg.type === 'chat') {
        await handleMessage(msg.message);
      } else if (msg.type === 'reset') {
        conversationHistory = [];
        send({ type: 'reset_done' });
      }
    } catch (e) {
      send({ type: 'error', error: String(e) });
    }
  });
}

main().catch(e => {
  send({ type: 'error', error: String(e) });
  process.exit(1);
});
```

**Step 3: Install and build sidecar**

```bash
cd src-tauri/sidecars/agent
npm install
npm run build
```

**Step 4: Commit**

```bash
cd /Users/fabio/projects/open-ontologies-studio
git add -A && git commit -m "feat: Agent SDK sidecar with MCP tool integration and agent loop"
```

---

### Task 11: Spawn sidecar from Tauri and create chat relay commands

**Files:**
- Create: `src-tauri/src/chat.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Create chat manager**

```rust
// src-tauri/src/chat.rs
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::io::BufRead;
use tauri::Emitter;

pub struct ChatState {
    pub process: Mutex<Option<Child>>,
}

pub fn spawn_agent_sidecar(app: &tauri::AppHandle) -> Result<(), String> {
    let sidecar_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("resource dir: {e}"))?
        .join("sidecars/agent");

    // For development, use the source directory
    let sidecar_dir = if sidecar_dir.exists() {
        sidecar_dir
    } else {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sidecars/agent")
    };

    let mut child = Command::new("node")
        .arg(sidecar_dir.join("dist/index.js"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("ANTHROPIC_API_KEY", std::env::var("ANTHROPIC_API_KEY").unwrap_or_default())
        .spawn()
        .map_err(|e| format!("Failed to spawn agent sidecar: {e}"))?;

    // Read stdout in background thread, emit events to frontend
    let stdout = child.stdout.take().ok_or("No stdout")?;
    let app_handle = app.clone();

    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                let _ = app_handle.emit("agent-message", line);
            }
        }
    });

    // Read stderr for debugging
    let stderr = child.stderr.take().ok_or("No stderr")?;
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                eprintln!("[agent stderr] {}", line);
            }
        }
    });

    let state = app.state::<ChatState>();
    *state.process.lock().unwrap() = Some(child);

    Ok(())
}

#[tauri::command]
pub fn send_chat_message(message: String, state: tauri::State<ChatState>) -> Result<(), String> {
    let mut guard = state.process.lock().unwrap();
    let child = guard.as_mut().ok_or("Agent sidecar not running")?;
    let stdin = child.stdin.as_mut().ok_or("No stdin")?;

    let payload = serde_json::json!({ "type": "chat", "message": message });
    writeln!(stdin, "{}", payload).map_err(|e| format!("Write failed: {e}"))?;
    stdin.flush().map_err(|e| format!("Flush failed: {e}"))?;

    Ok(())
}

#[tauri::command]
pub fn reset_chat(state: tauri::State<ChatState>) -> Result<(), String> {
    let mut guard = state.process.lock().unwrap();
    let child = guard.as_mut().ok_or("Agent sidecar not running")?;
    let stdin = child.stdin.as_mut().ok_or("No stdin")?;

    let payload = serde_json::json!({ "type": "reset" });
    writeln!(stdin, "{}", payload).map_err(|e| format!("Write failed: {e}"))?;
    stdin.flush().map_err(|e| format!("Flush failed: {e}"))?;

    Ok(())
}
```

**Step 2: Update lib.rs to spawn sidecar and register commands**

```rust
// src-tauri/src/lib.rs
mod engine;
mod chat;

use engine::EngineState;
use chat::ChatState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[tauri::command]
fn engine_status(state: tauri::State<EngineState>) -> bool {
    state.running.load(Ordering::Relaxed)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(EngineState {
            running: Arc::new(AtomicBool::new(false)),
        })
        .manage(ChatState {
            process: Mutex::new(None),
        })
        .setup(|app| {
            let handle = app.handle().clone();

            // Spawn engine
            if let Err(e) = engine::spawn_engine(&handle) {
                eprintln!("Failed to start engine: {e}");
            } else {
                app.state::<EngineState>().running.store(true, Ordering::Relaxed);
            }

            // Spawn agent sidecar (after short delay for engine to start)
            let handle2 = handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));
                if let Err(e) = chat::spawn_agent_sidecar(&handle2) {
                    eprintln!("Failed to start agent sidecar: {e}");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_status,
            chat::send_chat_message,
            chat::reset_chat,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 3: Verify**

```bash
ANTHROPIC_API_KEY=your-key-here npm run tauri dev
```

Expected: Engine starts, then agent sidecar starts 3s later. Console shows `[agent stderr]` messages.

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: spawn Agent SDK sidecar from Tauri with chat relay commands"
```

---

## Phase 6: Chat Panel

### Task 12: Create chat panel component

**Files:**
- Create: `src/components/ChatPanel.tsx`
- Create: `src/hooks/useChat.ts`

**Step 1: Create useChat hook**

```typescript
// src/hooks/useChat.ts
import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  toolCalls?: Array<{ tool: string; input?: unknown; result?: string }>;
  timestamp: Date;
}

interface ChatStore {
  messages: ChatMessage[];
  isTyping: boolean;
  sendMessage: (text: string) => Promise<void>;
  reset: () => Promise<void>;
}

let msgCounter = 0;

export const useChat = create<ChatStore>((set, get) => ({
  messages: [{
    id: 'welcome',
    role: 'system',
    content: 'Welcome to Open Ontologies Studio. Ask me to build, expand, or validate ontologies.',
    timestamp: new Date(),
  }],
  isTyping: false,

  sendMessage: async (text: string) => {
    const userMsg: ChatMessage = {
      id: `msg-${++msgCounter}`,
      role: 'user',
      content: text,
      timestamp: new Date(),
    };

    set(s => ({ messages: [...s.messages, userMsg], isTyping: true }));

    try {
      await invoke('send_chat_message', { message: text });
    } catch (e) {
      set(s => ({
        messages: [...s.messages, {
          id: `err-${msgCounter}`,
          role: 'system',
          content: `Error: ${e}`,
          timestamp: new Date(),
        }],
        isTyping: false,
      }));
    }
  },

  reset: async () => {
    await invoke('reset_chat');
    set({ messages: [{
      id: 'welcome',
      role: 'system',
      content: 'Conversation reset. How can I help?',
      timestamp: new Date(),
    }] });
  },
}));

// Listen for agent messages
let currentAssistantMsg: ChatMessage | null = null;

listen<string>('agent-message', (event) => {
  try {
    const data = JSON.parse(event.payload);

    if (data.type === 'text') {
      if (!currentAssistantMsg) {
        currentAssistantMsg = {
          id: `msg-${++msgCounter}`,
          role: 'assistant',
          content: data.content,
          toolCalls: [],
          timestamp: new Date(),
        };
        useChat.setState(s => ({
          messages: [...s.messages, currentAssistantMsg!],
        }));
      } else {
        currentAssistantMsg.content += data.content;
        useChat.setState(s => ({
          messages: s.messages.map(m =>
            m.id === currentAssistantMsg!.id ? { ...currentAssistantMsg! } : m
          ),
        }));
      }
    }

    if (data.type === 'tool_call') {
      if (currentAssistantMsg) {
        currentAssistantMsg.toolCalls = [
          ...(currentAssistantMsg.toolCalls || []),
          { tool: data.tool, input: data.input },
        ];
        useChat.setState(s => ({
          messages: s.messages.map(m =>
            m.id === currentAssistantMsg!.id ? { ...currentAssistantMsg! } : m
          ),
        }));
      }
    }

    if (data.type === 'tool_result') {
      if (currentAssistantMsg?.toolCalls) {
        const tc = currentAssistantMsg.toolCalls.find(t => t.tool === data.tool && !t.result);
        if (tc) tc.result = data.result;
        useChat.setState(s => ({
          messages: s.messages.map(m =>
            m.id === currentAssistantMsg!.id ? { ...currentAssistantMsg! } : m
          ),
        }));
      }
    }

    if (data.type === 'done') {
      currentAssistantMsg = null;
      useChat.setState({ isTyping: false });

      // Refresh graph if mutation occurred
      if (data.mutated) {
        const refreshGraph = (window as unknown as Record<string, unknown>).__refreshGraph as (() => void) | undefined;
        if (refreshGraph) refreshGraph();
      }
    }

    if (data.type === 'error') {
      currentAssistantMsg = null;
      useChat.setState(s => ({
        isTyping: false,
        messages: [...s.messages, {
          id: `err-${++msgCounter}`,
          role: 'system',
          content: `Agent error: ${data.error}`,
          timestamp: new Date(),
        }],
      }));
    }
  } catch (e) {
    console.error('Failed to parse agent message:', e);
  }
});
```

**Step 2: Create ChatPanel component**

```typescript
// src/components/ChatPanel.tsx
import { useState, useRef, useEffect } from 'react';
import { useChat, ChatMessage } from '../hooks/useChat';

const STARTER_CHIPS = [
  { label: 'Build an ontology', prompt: 'Build me an ontology about ' },
  { label: 'Expand current', prompt: 'Expand the current ontology with ' },
  { label: 'Validate', prompt: 'Validate the current ontology and report any issues' },
  { label: 'Run reasoning', prompt: 'Run OWL reasoning on the current ontology' },
  { label: 'Explain structure', prompt: 'Explain the structure of the current ontology' },
  { label: 'Ingest data', prompt: 'Ingest data from ' },
];

export function ChatPanel() {
  const { messages, isTyping, sendMessage, reset } = useChat();
  const [input, setInput] = useState('');
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: 'smooth' });
  }, [messages, isTyping]);

  const handleSend = () => {
    const text = input.trim();
    if (!text) return;
    setInput('');
    sendMessage(text);
  };

  const showStarters = messages.length <= 1;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b"
           style={{ borderColor: 'var(--border)' }}>
        <span className="text-sm font-medium" style={{ color: 'var(--text-primary)' }}>Chat</span>
        <button onClick={reset} className="text-xs px-2 py-0.5 rounded"
                style={{ color: 'var(--text-secondary)', background: 'var(--bg-panel)' }}>
          Reset
        </button>
      </div>

      {/* Messages */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-3 space-y-3">
        {showStarters && (
          <div className="space-y-2">
            <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>
              What would you like to do?
            </p>
            <div className="flex flex-wrap gap-2">
              {STARTER_CHIPS.map((chip) => (
                <button
                  key={chip.label}
                  onClick={() => setInput(chip.prompt)}
                  className="text-xs px-3 py-1.5 rounded-full border transition-colors hover:opacity-80"
                  style={{
                    borderColor: 'var(--border)',
                    color: 'var(--accent)',
                    background: 'var(--bg-panel)',
                  }}
                >
                  {chip.label}
                </button>
              ))}
            </div>
          </div>
        )}

        {messages.map((msg) => (
          <MessageBubble key={msg.id} message={msg} />
        ))}

        {isTyping && (
          <div className="flex gap-1 px-3 py-2" style={{ color: 'var(--text-secondary)' }}>
            <span className="animate-pulse">.</span>
            <span className="animate-pulse" style={{ animationDelay: '0.2s' }}>.</span>
            <span className="animate-pulse" style={{ animationDelay: '0.4s' }}>.</span>
          </div>
        )}
      </div>

      {/* Input */}
      <div className="p-3 border-t" style={{ borderColor: 'var(--border)' }}>
        <div className="flex gap-2">
          <input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
            placeholder="Ask about ontologies..."
            className="flex-1 px-3 py-2 text-sm rounded outline-none"
            style={{
              background: 'var(--bg-panel)',
              color: 'var(--text-primary)',
              border: '1px solid var(--border)',
            }}
          />
          <button
            onClick={handleSend}
            disabled={!input.trim() || isTyping}
            className="px-3 py-2 text-sm rounded font-medium"
            style={{
              background: input.trim() && !isTyping ? 'var(--accent)' : 'var(--bg-panel)',
              color: input.trim() && !isTyping ? 'var(--bg-primary)' : 'var(--text-secondary)',
            }}
          >
            Send
          </button>
        </div>
      </div>
    </div>
  );
}

function MessageBubble({ message }: { message: ChatMessage }) {
  const isUser = message.role === 'user';
  const isSystem = message.role === 'system';

  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div
        className="max-w-[85%] px-3 py-2 rounded-lg text-sm"
        style={{
          background: isUser ? 'var(--accent)' : isSystem ? 'var(--bg-panel)' : 'var(--bg-panel)',
          color: isUser ? 'var(--bg-primary)' : 'var(--text-primary)',
          opacity: isSystem ? 0.7 : 1,
        }}
      >
        <div className="whitespace-pre-wrap">{message.content}</div>

        {message.toolCalls && message.toolCalls.length > 0 && (
          <div className="mt-2 space-y-1">
            {message.toolCalls.map((tc, i) => (
              <div key={i} className="text-xs px-2 py-1 rounded"
                   style={{ background: 'var(--bg-primary)', color: 'var(--text-secondary)' }}>
                <span style={{ color: 'var(--accent)' }}>{tc.tool}</span>
                {tc.result && (
                  <span className="ml-1" style={{ color: 'var(--success)' }}>done</span>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
```

**Step 3: Wire ChatPanel into Layout**

Replace the chat placeholder in Layout.tsx:

```typescript
import { ChatPanel } from './ChatPanel';

// Replace the chat panel placeholder div content:
{showChat && (
  <div className="w-96 border-l flex flex-col"
       style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)' }}>
    <ChatPanel />
  </div>
)}
```

**Step 4: Verify**

```bash
ANTHROPIC_API_KEY=your-key npm run tauri dev
```

Expected: Chat panel shows starter chips. Clicking one fills input. Sending a message triggers the agent, tool calls appear, response streams in. Graph refreshes after mutations.

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: chat panel with Agent SDK, starter chips, tool call visibility"
```

---

## Phase 7: Property Inspector

### Task 13: Create property inspector for selected nodes

**Files:**
- Create: `src/components/PropertyInspector.tsx`
- Modify: `src/components/Layout.tsx`

**Step 1: Create PropertyInspector**

```typescript
// src/components/PropertyInspector.tsx
import { useState, useEffect } from 'react';
import * as mcp from '../lib/mcp-client';

interface Props {
  node: { id: string; label: string; uri: string } | null;
  onGraphChanged: () => void;
}

interface NodeProperty {
  predicate: string;
  value: string;
  predicateLabel: string;
}

export function PropertyInspector({ node, onGraphChanged }: Props) {
  const [properties, setProperties] = useState<NodeProperty[]>([]);
  const [validationStatus, setValidationStatus] = useState<'valid' | 'invalid' | 'checking' | null>(null);

  useEffect(() => {
    if (!node) { setProperties([]); return; }

    // Fetch all properties of selected node
    mcp.sparqlQuery(
      `SELECT ?p ?o WHERE { <${node.uri}> ?p ?o . FILTER(!isBlank(?o)) }`
    ).then(text => {
      try {
        const data = JSON.parse(text);
        const bindings = data.results?.bindings || data || [];
        setProperties(bindings.map((b: Record<string, { value: string }>) => ({
          predicate: b.p?.value || b.p || '',
          value: b.o?.value || b.o || '',
          predicateLabel: shortUri(b.p?.value || b.p || ''),
        })));
      } catch {
        setProperties([]);
      }
    });

    // Run validation check
    setValidationStatus('checking');
    mcp.callTool('onto_lint', { input: '', inline: false }).then(result => {
      try {
        const issues = JSON.parse(result);
        const nodeIssues = Array.isArray(issues)
          ? issues.filter((i: { entity?: string }) => i.entity === node.uri)
          : [];
        setValidationStatus(nodeIssues.length === 0 ? 'valid' : 'invalid');
      } catch {
        setValidationStatus('valid');
      }
    });
  }, [node]);

  if (!node) {
    return (
      <div className="p-3 text-sm" style={{ color: 'var(--text-secondary)' }}>
        Select a node to inspect
      </div>
    );
  }

  return (
    <div className="p-3 space-y-3 overflow-y-auto h-full">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium" style={{ color: 'var(--text-primary)' }}>
          {node.label}
        </h3>
        <span
          className="text-xs px-2 py-0.5 rounded"
          style={{
            background: validationStatus === 'valid' ? 'var(--success)' :
                        validationStatus === 'invalid' ? 'var(--error)' : 'var(--bg-panel)',
            color: 'var(--bg-primary)',
          }}
        >
          {validationStatus === 'checking' ? '...' :
           validationStatus === 'valid' ? 'Valid' :
           validationStatus === 'invalid' ? 'Issues' : ''}
        </span>
      </div>

      {/* URI */}
      <div>
        <div className="text-xs mb-1" style={{ color: 'var(--text-secondary)' }}>URI</div>
        <div className="text-xs font-mono px-2 py-1 rounded break-all"
             style={{ background: 'var(--bg-primary)', color: 'var(--text-primary)' }}>
          {node.uri}
        </div>
      </div>

      {/* Properties */}
      <div>
        <div className="text-xs mb-1" style={{ color: 'var(--text-secondary)' }}>Properties</div>
        <div className="space-y-1">
          {properties.map((prop, i) => (
            <div key={i} className="text-xs px-2 py-1 rounded"
                 style={{ background: 'var(--bg-primary)' }}>
              <span style={{ color: 'var(--accent)' }}>{prop.predicateLabel}</span>
              <span style={{ color: 'var(--text-secondary)' }}> = </span>
              <span style={{ color: 'var(--text-primary)' }}>{shortUri(prop.value)}</span>
            </div>
          ))}
          {properties.length === 0 && (
            <div className="text-xs" style={{ color: 'var(--text-secondary)' }}>No properties</div>
          )}
        </div>
      </div>
    </div>
  );
}

function shortUri(uri: string): string {
  const hash = uri.lastIndexOf('#');
  if (hash >= 0) return uri.slice(hash + 1);
  const slash = uri.lastIndexOf('/');
  if (slash >= 0) return uri.slice(slash + 1);
  return uri;
}
```

**Step 2: Wire into Layout**

```typescript
import { PropertyInspector } from './PropertyInspector';

// Replace inspector placeholder:
{showInspector && (
  <div className="w-72 border-l overflow-hidden"
       style={{ borderColor: 'var(--border)', background: 'var(--bg-panel)' }}>
    <PropertyInspector
      node={selectedNode}
      onGraphChanged={() => {
        const refresh = (window as unknown as Record<string, unknown>).__refreshGraph as (() => void) | undefined;
        if (refresh) refresh();
      }}
    />
  </div>
)}
```

Also: auto-show inspector when a node is selected:

```typescript
// In Layout, add effect:
useEffect(() => {
  if (selectedNode) setShowInspector(true);
}, [selectedNode]);
```

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: property inspector showing node URI, properties, validation status"
```

---

## Phase 8: Settings and API Key Management

### Task 14: Add settings dialog for API key

**Files:**
- Create: `src/components/SettingsDialog.tsx`
- Create: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/components/Layout.tsx`

**Step 1: Create Rust settings commands**

```rust
// src-tauri/src/settings.rs
use std::fs;
use std::path::PathBuf;

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

#[tauri::command]
pub fn get_api_key(app: tauri::AppHandle) -> Result<String, String> {
    // Check env first
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() { return Ok(key); }
    }
    // Check saved settings
    let path = config_path(&app)?;
    if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
        if let Some(key) = settings.get("api_key").and_then(|v| v.as_str()) {
            return Ok(key.to_string());
        }
    }
    Ok(String::new())
}

#[tauri::command]
pub fn save_api_key(app: tauri::AppHandle, key: String) -> Result<(), String> {
    let path = config_path(&app)?;
    let settings = serde_json::json!({ "api_key": key });
    fs::write(&path, serde_json::to_string_pretty(&settings).unwrap())
        .map_err(|e| e.to_string())
}
```

**Step 2: Create SettingsDialog component**

```typescript
// src/components/SettingsDialog.tsx
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Props {
  open: boolean;
  onClose: () => void;
}

export function SettingsDialog({ open, onClose }: Props) {
  const [apiKey, setApiKey] = useState('');
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (open) {
      invoke<string>('get_api_key').then(key => setApiKey(key || ''));
    }
  }, [open]);

  if (!open) return null;

  const handleSave = async () => {
    await invoke('save_api_key', { key: apiKey });
    setSaved(true);
    setTimeout(() => { setSaved(false); onClose(); }, 1000);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center"
         style={{ background: 'rgba(0,0,0,0.5)' }}
         onClick={onClose}>
      <div className="w-96 p-4 rounded-lg shadow-xl"
           style={{ background: 'var(--bg-panel)', border: '1px solid var(--border)' }}
           onClick={(e) => e.stopPropagation()}>
        <h2 className="text-sm font-semibold mb-3" style={{ color: 'var(--text-primary)' }}>
          Settings
        </h2>
        <label className="text-xs mb-1 block" style={{ color: 'var(--text-secondary)' }}>
          Anthropic API Key
        </label>
        <input
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          className="w-full px-3 py-2 text-sm rounded mb-3 outline-none"
          style={{
            background: 'var(--bg-primary)',
            color: 'var(--text-primary)',
            border: '1px solid var(--border)',
          }}
        />
        <div className="flex justify-end gap-2">
          <button onClick={onClose} className="text-xs px-3 py-1.5 rounded"
                  style={{ color: 'var(--text-secondary)' }}>
            Cancel
          </button>
          <button onClick={handleSave} className="text-xs px-3 py-1.5 rounded"
                  style={{ background: saved ? 'var(--success)' : 'var(--accent)',
                           color: 'var(--bg-primary)' }}>
            {saved ? 'Saved!' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  );
}
```

**Step 3: Add settings button to Layout toolbar and register Rust commands**

In Layout.tsx toolbar, add:

```typescript
import { SettingsDialog } from './SettingsDialog';

// State:
const [showSettings, setShowSettings] = useState(false);

// In toolbar:
<button onClick={() => setShowSettings(true)} className="text-xs px-2 py-1 rounded"
        style={{ background: 'var(--bg-panel)', color: 'var(--text-secondary)' }}>
  Settings
</button>

// Before closing </div> of Layout:
<SettingsDialog open={showSettings} onClose={() => setShowSettings(false)} />
```

In lib.rs, register commands and add serde dependency:

```rust
// Add to Cargo.toml:
// serde_json = "1"

// Register in invoke_handler:
.invoke_handler(tauri::generate_handler![
    engine_status,
    chat::send_chat_message,
    chat::reset_chat,
    settings::get_api_key,
    settings::save_api_key,
])
```

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: settings dialog for API key management"
```

---

## Phase 9: Final Integration

### Task 15: Add keyboard shortcuts and slash command menu

**Files:**
- Modify: `src/components/Layout.tsx`
- Modify: `src/components/ChatPanel.tsx`

**Step 1: Add keyboard shortcuts to Layout**

```typescript
// In Layout, add:
useEffect(() => {
  const handler = (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'j') {
      e.preventDefault();
      setShowChat(c => !c);
    }
    if ((e.metaKey || e.ctrlKey) && e.key === 'i') {
      e.preventDefault();
      setShowInspector(i => !i);
    }
  };
  window.addEventListener('keydown', handler);
  return () => window.removeEventListener('keydown', handler);
}, []);
```

**Step 2: Add slash commands to ChatPanel**

Add after `STARTER_CHIPS` definition:

```typescript
const SLASH_COMMANDS = [
  { cmd: '/build', description: 'Build a new ontology', prompt: 'Build me an ontology about ' },
  { cmd: '/expand', description: 'Expand current ontology', prompt: 'Expand the current ontology with ' },
  { cmd: '/validate', description: 'Run full validation', prompt: 'Run onto_validate and onto_lint on the current ontology and report all issues' },
  { cmd: '/reason', description: 'Run OWL reasoning', prompt: 'Run onto_reason with profile owl-rl on the current ontology' },
  { cmd: '/enforce', description: 'Check design patterns', prompt: 'Run onto_enforce with the generic rule pack' },
  { cmd: '/query', description: 'Run SPARQL query', prompt: 'Run this SPARQL query: ' },
  { cmd: '/stats', description: 'Show statistics', prompt: 'Run onto_stats and summarize the current ontology' },
  { cmd: '/save', description: 'Export ontology', prompt: 'Save the current ontology to a Turtle file' },
];

// In ChatPanel, add slash menu state:
const [showSlash, setShowSlash] = useState(false);
const filteredCommands = SLASH_COMMANDS.filter(c =>
  input.startsWith('/') ? c.cmd.startsWith(input) : false
);

// Show slash menu when typing /
useEffect(() => {
  setShowSlash(input.startsWith('/') && filteredCommands.length > 0);
}, [input]);

// Add slash menu above input:
{showSlash && (
  <div className="mx-3 mb-1 rounded overflow-hidden"
       style={{ background: 'var(--bg-panel)', border: '1px solid var(--border)' }}>
    {filteredCommands.map((cmd) => (
      <button
        key={cmd.cmd}
        onClick={() => { setInput(cmd.prompt); setShowSlash(false); }}
        className="w-full text-left px-3 py-1.5 text-xs flex justify-between hover:opacity-80"
        style={{ color: 'var(--text-primary)' }}
      >
        <span style={{ color: 'var(--accent)' }}>{cmd.cmd}</span>
        <span style={{ color: 'var(--text-secondary)' }}>{cmd.description}</span>
      </button>
    ))}
  </div>
)}
```

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: keyboard shortcuts and slash command menu"
```

---

### Task 16: End-to-end test and polish

**Step 1: Build the engine (if not already built)**

```bash
cd /Users/fabio/projects/open-ontologies
cargo build --release
```

**Step 2: Build the agent sidecar**

```bash
cd /Users/fabio/projects/open-ontologies-studio/src-tauri/sidecars/agent
npm install && npm run build
```

**Step 3: Run the full app**

```bash
cd /Users/fabio/projects/open-ontologies-studio
ANTHROPIC_API_KEY=your-key npm run tauri dev
```

**Step 4: Test these flows:**

1. App opens → status bar shows "Connected" with stats
2. Type "Build me an ontology about animals" in chat → agent calls onto_load → graph shows nodes
3. Right-click canvas → add class "Fish" → node appears → validation passes
4. Shift+drag from Fish to Animal → subClassOf edge created
5. Select a node → inspector shows URI, properties, validation status
6. Delete key on selected node → node removed
7. Double-click node → edit label
8. Type "/validate" → slash menu appears → select → runs validation
9. Click starter chip → input filled
10. Cmd+J toggles chat, Cmd+I toggles inspector

**Step 5: Fix any issues found**

**Step 6: Final commit**

```bash
git add -A && git commit -m "feat: Open Ontologies Studio v2 — Tauri + React + Agent SDK"
```

---

## Summary

| Phase | Tasks | What it builds |
|-------|-------|---------------|
| 1: Scaffolding | 1-2 | Tauri + React + Vite project with dark theme layout |
| 2: Engine | 3-4 | Engine sidecar with lifecycle management |
| 3: MCP Client | 5-6 | TypeScript MCP client + connection hook + status bar |
| 4: Graph Canvas | 7-9 | Cytoscape.js canvas with visual editing and validation |
| 5: Agent SDK | 10-11 | Node.js sidecar with agent loop and chat relay |
| 6: Chat Panel | 12 | Chat UI with starter chips, tool call visibility |
| 7: Inspector | 13 | Property inspector for selected nodes |
| 8: Settings | 14 | API key dialog |
| 9: Integration | 15-16 | Keyboard shortcuts, slash commands, e2e test |
