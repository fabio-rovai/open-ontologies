# Open Ontologies Studio — Design Document

**Date:** 2026-03-14
**Status:** Approved
**Author:** Fabio Rovai

## Problem

Ontology engineering today is trapped between two extremes:

1. **Protege** — a 20-year-old Java desktop app. Tree-based, single-player, no AI awareness, no lineage, heavy.
2. **Claude + Open Ontologies MCP** — powerful but invisible. The ontologist can't see what the AI is doing, can't visually explore the graph, can't point-and-click to edit.

There is no modern, visual, AI-integrated ontology editor.

## Solution

**Open Ontologies Studio** — a cross-platform visual ontology editor built on top of the Open Ontologies engine. Graph-first, AI-integrated, human-in-the-loop.

- **Engine** = `open-ontologies` (Rust CLI/MCP server, unchanged)
- **Studio** = `open-ontologies-studio` (Flutter desktop/web/mobile app, new)

## Design Principles

1. **Graph is the primary interface** — not forms, not trees. Click nodes to edit, drag to connect, pinch to zoom.
2. **AI is visible** — every action Claude takes appears in an activity feed with [approve] / [reject] controls.
3. **Live, not dead** — validation runs continuously, errors appear as halos on nodes, changes animate in real-time.
4. **Keyboard-first** — Cmd+K command palette, Tab navigation, shortcuts for everything.
5. **Offline-first** — works without a network, syncs when connected.
6. **One codebase, all platforms** — macOS, Windows, Linux, web, iPad.

## Architecture

### Stack

| Layer | Technology | Why |
|---|---|---|
| UI framework | Flutter (Dart) | One codebase → 6 platforms, custom GPU rendering, native performance |
| Graph rendering | CustomPainter (Flutter) | GPU-accelerated canvas, LOD rendering, 60fps with 10k+ nodes |
| Graph compute | Rust shared library via FFI | Force-directed layout, spatial indexing, collision detection at native speed |
| State management | Riverpod or Bloc | Reactive, testable, well-established in Flutter ecosystem |
| Local storage | Isar or Hive | Offline caching of ontology snapshots |
| Bridge to engine | gRPC with server-streaming | Typed API, real-time push from engine to UI |
| Engine | Open Ontologies (Rust) | Oxigraph triple store, SQLite state, all 42 MCP tools |
| AI integration | MCP (stdio/HTTP) | Claude connects to the same engine, changes stream to the UI |

### Component Diagram

```
┌──────────────────────────────────────────────────────────┐
│                 Open Ontologies Studio                    │
│                 (Flutter — all platforms)                 │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │              Graph Canvas                        │    │
│  │  CustomPainter + Rust FFI layout engine          │    │
│  │  ├── LOD rendering (clusters → labels → props)   │    │
│  │  ├── Spatial index (R-tree) for hit testing      │    │
│  │  ├── Force-directed / hierarchical / radial      │    │
│  │  ├── Drag-to-connect (creates triples)           │    │
│  │  ├── Multi-select, lasso, context menus          │    │
│  │  └── Animated transitions on changes             │    │
│  └─────────────────────────────────────────────────┘    │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ AI Activity  │  │ Validation   │  │ Lineage      │  │
│  │ Panel        │  │ Panel        │  │ Timeline     │  │
│  │              │  │              │  │              │  │
│  │ Shows tool   │  │ Live lint +  │  │ Version      │  │
│  │ calls from   │  │ SHACL errors │  │ snapshots    │  │
│  │ Claude with  │  │ as node      │  │ with diff    │  │
│  │ approve /    │  │ halos, auto- │  │ and revert   │  │
│  │ reject       │  │ fix suggest  │  │ controls     │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ SPARQL       │  │ Property     │  │ SHACL        │  │
│  │ Editor       │  │ Inspector    │  │ Constraints  │  │
│  │              │  │              │  │              │  │
│  │ Autocomplete │  │ Edit labels, │  │ Visual shape │  │
│  │ with loaded  │  │ comments,    │  │ indicators   │  │
│  │ class/prop   │  │ domains,     │  │ on graph     │  │
│  │ names        │  │ ranges       │  │ nodes        │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Cmd+K: command palette                          │    │
│  │  "add Vehicle subclass of Thing"                 │    │
│  └─────────────────────────────────────────────────┘    │
│                                                          │
│                    gRPC (streaming)                       │
└──────────────────────────────────────────────────────────┘
                          │
                          ▼
            ┌───────────────────────┐
            │   Open Ontologies     │
            │   (Rust engine)       │
            │                       │
            │   Oxigraph + SQLite   │
            │   42 MCP tools        │
            │                       │
            │   Interfaces:         │
            │   ├── gRPC (Studio)   │
            │   ├── MCP stdio       │
            │   ├── MCP HTTP        │
            │   └── CLI             │
            └───────────────────────┘
                     ▲
                     │ MCP (stdio/HTTP)
                     ▼
            ┌───────────────────────┐
            │   Claude / LLM        │
            └───────────────────────┘
```

### Data Flow

#### Human edits a node

1. User clicks node in graph → Property Inspector opens
2. User changes label from "Car" to "Automobile"
3. Flutter sends `UpdateTriple` via gRPC to engine
4. Engine updates Oxigraph, records in lineage
5. Engine streams `ChangeEvent` back
6. Graph canvas animates the label change

#### Claude modifies the ontology

1. Claude calls `onto_plan` via MCP → engine returns plan
2. Engine streams `PlanEvent` to Studio via gRPC
3. AI Activity Panel shows: "Claude wants to add 3 classes" with [approve] [reject]
4. User clicks [approve]
5. Studio sends approval → engine calls `onto_apply`
6. Engine streams `ChangeEvent` for each added class
7. Graph canvas animates new nodes appearing

#### Offline editing

1. Studio has a local Isar cache of the last-loaded ontology
2. User edits offline → changes queued locally
3. On reconnect → changes replayed to engine in order
4. Conflicts shown in UI for resolution

## Graph Compute Engine (Rust FFI)

The graph layout runs in Rust as a shared library, called from Flutter via FFI. This separates the computationally expensive layout from the UI thread.

### Compilation targets

```
graph_compute/
├── src/
│   ├── layout.rs        # Force-directed, hierarchical, radial
│   ├── spatial.rs       # R-tree spatial index
│   ├── clustering.rs    # LOD clustering for large graphs
│   └── lib.rs           # FFI exports
├── Cargo.toml
└── targets:
    ├── libgraph.dylib   (macOS)
    ├── graph.dll         (Windows)
    ├── libgraph.so       (Linux)
    └── graph.wasm        (web)
```

### FFI interface

```rust
#[no_mangle]
pub extern "C" fn graph_layout_tick(
    nodes: *const f64,      // [x, y, x, y, ...] positions
    edges: *const u32,      // [src, dst, src, dst, ...] indices
    node_count: u32,
    edge_count: u32,
    out_positions: *mut f64, // updated positions
) -> bool;                   // true if converged

#[no_mangle]
pub extern "C" fn graph_hit_test(
    x: f64, y: f64,
    nodes: *const f64,
    node_count: u32,
) -> i32;                    // node index or -1
```

## gRPC API

### Service definition

```protobuf
syntax = "proto3";

service OntologyEngine {
  // Core operations (map to existing onto_* tools)
  rpc LoadOntology(LoadRequest) returns (LoadResponse);
  rpc Query(SparqlRequest) returns (QueryResponse);
  rpc Validate(ValidateRequest) returns (ValidateResponse);
  rpc Stats(StatsRequest) returns (StatsResponse);
  rpc Lint(LintRequest) returns (LintResponse);
  rpc Save(SaveRequest) returns (SaveResponse);
  rpc Clear(ClearRequest) returns (ClearResponse);

  // Lifecycle
  rpc Plan(PlanRequest) returns (PlanResponse);
  rpc Apply(ApplyRequest) returns (ApplyResponse);
  rpc Enforce(EnforceRequest) returns (EnforceResponse);

  // Graph data for rendering
  rpc GetGraph(GraphRequest) returns (GraphResponse);

  // Real-time streaming — engine pushes to Studio
  rpc WatchChanges(WatchRequest) returns (stream ChangeEvent);
  rpc WatchValidation(WatchRequest) returns (stream ValidationEvent);
}

message ChangeEvent {
  enum Type {
    TRIPLE_ADDED = 0;
    TRIPLE_REMOVED = 1;
    CLASS_ADDED = 2;
    CLASS_REMOVED = 3;
    PROPERTY_ADDED = 4;
    PROPERTY_REMOVED = 5;
    PLAN_PROPOSED = 6;
    APPLY_COMPLETED = 7;
  }
  Type type = 1;
  string subject = 2;
  string predicate = 3;
  string object = 4;
  string source = 5;    // "claude" or "human"
  int64 timestamp = 6;
}

message GraphResponse {
  repeated GraphNode nodes = 1;
  repeated GraphEdge edges = 2;
}

message GraphNode {
  string iri = 1;
  string label = 2;
  string comment = 3;
  string type = 4;          // "class", "individual", "property"
  repeated string domains = 5;
  repeated string ranges = 6;
}

message GraphEdge {
  string source_iri = 1;
  string target_iri = 2;
  string predicate = 3;     // "rdfs:subClassOf", "owl:equivalentClass", etc.
}
```

## UI Components

### 1. Graph Canvas (main view)

- Full-screen GPU-rendered canvas via `CustomPainter`
- Level-of-detail rendering:
  - Galaxy view (>1000 visible): colored clusters with count labels
  - Neighborhood view (100-1000): nodes with labels, edges with type colors
  - Detail view (<100): full property display on nodes, edge labels
- Layout modes: force-directed (default), hierarchical, radial, manual
- Interactions: click, double-click edit, drag move, drag-to-connect, lasso select, pinch zoom, two-finger pan
- Visual encoding:
  - Node color = type (class=blue, individual=green, datatype property=orange, object property=purple)
  - Node border = validation status (green=ok, yellow=warning, red=error)
  - Edge style = relationship type (solid=subClassOf, dashed=equivalentClass, dotted=domain/range)

### 2. AI Activity Panel

- Chronological feed of all MCP tool calls from Claude
- Each entry shows: tool name, summary, timestamp, source
- Action buttons: [approve] [reject] [undo] [details]
- When Claude proposes a plan: shows added/removed with risk score
- Expandable to see full tool input/output

### 3. Validation Panel

- Live results from `onto_lint` and `onto_shacl`
- Grouped by severity: errors, warnings, info
- Click an issue → highlights the relevant node on the graph
- [auto-fix] button where applicable (e.g., add missing label)

### 4. Lineage Timeline

- Visual timeline of all versions (from `onto_history`)
- Click any version to load it
- [diff] button between any two versions
- Shows who/what made each change (Claude vs human)

### 5. SPARQL Editor

- Syntax-highlighted editor with autocomplete
- Autocomplete knows loaded class names, property names, prefixes
- Results displayed as table or injected into graph view
- Saved query library

### 6. Property Inspector

- Appears when a node is selected
- Edit: label, comment, equivalent classes, disjoint classes
- View: domain/range for properties, SHACL constraints
- Add: new subclass, new property, new individual

### 7. Command Palette (Cmd+K)

- Search across all actions, tools, classes, properties
- Natural language input: "add Vehicle subclass of Thing" → calls Claude
- Quick actions: "export turtle", "run SHACL", "compare versions"

## Platform Matrix

| Platform | Build | Distribution |
|---|---|---|
| macOS (arm64, x86_64) | GitHub Actions | `.dmg` on GitHub Releases |
| Windows (x86_64) | GitHub Actions | `.msix` on GitHub Releases |
| Linux (x86_64) | GitHub Actions | `.AppImage` / `.deb` on Releases |
| Web | GitHub Actions | GitHub Pages or self-hosted |
| iPad | GitHub Actions | TestFlight → App Store (later) |

All builds run in CI. Local development uses `flutter run` against a browser or connected device, with `open-ontologies serve-http` running locally.

## Repository Structure

```
open-ontologies-studio/
├── lib/                       # Flutter/Dart source
│   ├── main.dart
│   ├── app.dart
│   ├── features/
│   │   ├── graph/             # Graph canvas + rendering
│   │   ├── ai_activity/       # AI activity panel
│   │   ├── validation/        # Live validation panel
│   │   ├── lineage/           # Version timeline
│   │   ├── sparql/            # SPARQL editor
│   │   ├── inspector/         # Property inspector
│   │   └── command_palette/   # Cmd+K
│   ├── services/
│   │   ├── engine_client.dart # gRPC client to Open Ontologies
│   │   └── graph_compute.dart # FFI bridge to Rust layout engine
│   └── models/
│       ├── graph_node.dart
│       ├── graph_edge.dart
│       └── change_event.dart
├── native/                    # Rust graph compute library
│   ├── src/
│   │   ├── layout.rs
│   │   ├── spatial.rs
│   │   └── lib.rs
│   └── Cargo.toml
├── proto/                     # gRPC service definitions
│   └── ontology.proto
├── test/                      # Flutter tests
├── pubspec.yaml               # Dart dependencies
└── .github/
    └── workflows/
        └── build.yml          # CI for all platforms
```

## Build & Development

### Local development (lightweight)

```bash
# Terminal 1: run the engine
cd open-ontologies && cargo run --release -- serve-http

# Terminal 2: run the Studio in browser (hot reload)
cd open-ontologies-studio && flutter run -d chrome
```

No heavy compilation needed locally. The Rust graph compute library has a Dart fallback for development (slower but works without compiling Rust).

### CI build (full)

GitHub Actions compiles:
1. Rust graph compute library for all targets
2. Flutter app for macOS, Windows, Linux, web
3. Packages as `.dmg`, `.msix`, `.AppImage`, `.deb`
4. Uploads to GitHub Releases

## Open Questions

1. **gRPC vs HTTP+SSE** — gRPC is more typed and supports streaming natively, but adds a dependency (tonic on Rust side, grpc-dart on Flutter side). HTTP+SSE is simpler but less structured. Recommendation: start with HTTP+SSE for simplicity, migrate to gRPC if needed.

2. **Graph compute Dart fallback** — for local dev without compiling Rust, should the Dart fallback use a simple force-directed algorithm or skip layout entirely? Recommendation: simple Dart force-directed, good enough for <500 nodes during dev.

3. **Offline sync strategy** — CRDT-based merge vs last-write-wins vs manual conflict resolution? Recommendation: start with manual conflict resolution, CRDTs are overkill initially.

4. **Mobile scope** — full editing on iPad or read-only exploration? Recommendation: read-only + approve/reject AI changes initially, full editing later.

## Phase 2: Advanced Features

### 8. Poincaré Embedding Layout

Open Ontologies already computes Poincaré structural embeddings via `onto_embed`. These encode hierarchical distance in hyperbolic space — classes near each other in the taxonomy are close in the Poincaré disk.

**Implementation:**

- Call `onto_embed` to get `struct_vec` (Poincaré coordinates) for each class
- Map Poincaré disk coordinates (hyperbolic) to screen coordinates using the Beltrami-Klein projection
- Add "Poincaré" as a layout option alongside force-directed, hierarchical, radial
- Classes that are semantically similar cluster naturally — no O(n²) force simulation needed
- Combined with `onto_search`, clicking empty space and typing a concept highlights the nearest cluster

**Why this is a killer feature:** No other ontology editor uses the ontology's own embedding geometry as a layout algorithm. The graph literally organizes itself by meaning.

### 9. Virtual Graph Querying (R2RML / Direct Mapping)

Query external databases (PostgreSQL) as if they were part of the ontology graph, without materializing triples.

**Engine side (Rust):**

- New `onto_virtual_query` tool that accepts SPARQL + a database connection string
- Uses R2RML mappings (or Direct Mapping for unmapped tables) to translate SPARQL to SQL
- Returns SPARQL-compatible JSON results — Studio treats them identically to Oxigraph results

**Studio side (Flutter):**

- "Virtual Sources" panel to configure database connections
- SPARQL editor shows virtual and materialized results with different badges
- Option to materialize virtual results into the triple store via `onto_load`

### 10. Visual Mapping Editor

A split-pane editor for creating and editing ontology alignment mappings interactively.

**Engine side (Rust):**

- Extend `onto_align` to persist mapping specs (save/load/edit alignment candidates)
- New `onto_align_save` / `onto_align_load` tools for mapping spec lifecycle

**Studio side (Flutter):**

- Left pane: source ontology graph
- Right pane: target ontology graph
- Drag a class from left to right to create `owl:equivalentClass` / `skos:exactMatch` / `rdfs:subClassOf`
- Shows `onto_align` suggestions as dashed lines with confidence scores
- Click a suggestion to accept/reject (feeds `onto_align_feedback`)
- Export accepted mappings as Turtle

### 11. OWL2 Profile Selector & Reasoning Control

**Engine side (Rust):**

- Extend `onto_reason` with EL, QL, RL profile modes
- Profile detection: analyze loaded ontology and recommend the most specific applicable profile
- Profile violations highlighted as lint issues

**Studio side (Flutter):**

- Dropdown in toolbar: "Profile: OWL2 Full | EL | QL | RL | RDFS"
- When profile is selected, lint automatically flags axioms outside that profile
- Reasoner panel shows inferred triples with provenance (which axioms produced them)
- "Explain" button on each inferred triple → shows derivation tree

## Success Criteria

1. Load a 1,000-class ontology and render at 60fps
2. Human can add a class by dragging on the canvas in <3 seconds
3. Claude's changes appear on the canvas within 1 second
4. Human can approve/reject any AI change before it's applied
5. Full ontology lifecycle (plan → enforce → apply → monitor) visible in the UI
6. Works on macOS, Windows, and web from a single codebase
