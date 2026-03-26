<!-- mcp-name: io.github.fabio-rovai/open-ontologies -->

<p align="center">
  <img src="docs/assets/logo.png" alt="Open Ontologies" width="300">
</p>

<h1 align="center">Open Ontologies</h1>

<p align="center">
  <strong>A Terraforming MCP for Knowledge Graphs</strong><br>
  Validate, classify, and govern AI-generated ontologies. Written in Rust. Ships as a single binary.
</p>

<p align="center">
  <a href="https://github.com/fabio-rovai/open-ontologies/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/fabio-rovai/open-ontologies/ci.yml?branch=main&style=for-the-badge" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge" alt="MIT"></a>
  <a href="https://openmcp.org/servers/open-ontologies"><img src="https://img.shields.io/badge/Open_MCP-open--ontologies-blue?style=for-the-badge" alt="Open MCP"></a>
  <a href="https://www.pitchhut.com/project/open-ontologies-mcp"><img src="https://img.shields.io/badge/PitchHut-open--ontologies-orange?style=for-the-badge" alt="PitchHut"></a>
  <a href="https://clawhub.ai/fabio-rovai/open-ontologies"><img src="https://img.shields.io/badge/ClawHub-open--ontologies-7c3aed?style=for-the-badge" alt="ClawHub"></a>
</p>

<p align="center">
  <a href="#quick-start-mcp--cli">Quick Start</a> ·
  <a href="#studio-desktop-app">Studio</a> ·
  <a href="#benchmarks">Benchmarks</a> ·
  <a href="#ies-support">IES</a> ·
  <a href="#tools">Tools</a> ·
  <a href="#architecture">Architecture</a> ·
  <a href="#documentation">Docs</a>
</p>

---

Open Ontologies is a **Rust MCP server** and **desktop Studio** for AI-native ontology engineering. It exposes **43 tools** that let Claude build, validate, query, diff, lint, version, reason over, align, and persist RDF/OWL ontologies using an in-memory Oxigraph triple store — with Terraform-style lifecycle management, a marketplace of 32 standard ontologies, clinical crosswalks, semantic embeddings, and a full lineage audit trail.

The **Studio** wraps the engine in a visual desktop environment: virtualized ontology tree with hierarchy lines, breadcrumb navigation, and connection explorer; AI chat panel with `/build` (IES-level deep) and `/sketch` (quick prototype) commands; Protégé-style property inspector; and lineage viewer.

No JVM. No Protégé. No GUI required.

---

## Screenshots

| Full UI | Tree View |
|---|---|
| ![Studio overview — tree view, property inspector, and AI chat panel](studio/docs/screenshots/studio-overview.png) | ![Virtualized tree with hierarchy lines and connection explorer](studio/docs/screenshots/studio-graph.png) |

*Cat ontology built from a single "build ontology about cats" command — 1,433 classes, 218 object properties, 358 individuals across 11 hierarchy levels. The tree view virtualizes rendering (only visible rows are in the DOM), with hierarchy connector lines, breadcrumb navigation, type-filtered legend, and a connections panel showing domain/range relationships as clickable pills.*

---

## Quick Start (MCP / CLI)

### Install

**Pre-built binaries:**

```bash
# macOS (Apple Silicon)
curl -LO https://github.com/fabio-rovai/open-ontologies/releases/latest/download/open-ontologies-aarch64-apple-darwin
chmod +x open-ontologies-aarch64-apple-darwin && mv open-ontologies-aarch64-apple-darwin /usr/local/bin/open-ontologies

# macOS (Intel)
curl -LO https://github.com/fabio-rovai/open-ontologies/releases/latest/download/open-ontologies-x86_64-apple-darwin
chmod +x open-ontologies-x86_64-apple-darwin && mv open-ontologies-x86_64-apple-darwin /usr/local/bin/open-ontologies

# Linux (x86_64)
curl -LO https://github.com/fabio-rovai/open-ontologies/releases/latest/download/open-ontologies-x86_64-unknown-linux-gnu
chmod +x open-ontologies-x86_64-unknown-linux-gnu && mv open-ontologies-x86_64-unknown-linux-gnu /usr/local/bin/open-ontologies
```

**Docker:**

```bash
docker pull ghcr.io/fabio-rovai/open-ontologies:latest
docker run -i ghcr.io/fabio-rovai/open-ontologies serve
```

**From source (Rust 1.85+):**

```bash
git clone https://github.com/fabio-rovai/open-ontologies.git
cd open-ontologies && cargo build --release
./target/release/open-ontologies init
```

### Connect to your MCP client

<details>
<summary><strong>Claude Code</strong></summary>

Add to `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "open-ontologies": {
      "command": "/path/to/open-ontologies/target/release/open-ontologies",
      "args": ["serve"]
    }
  }
}
```

Restart Claude Code. The `onto_*` tools are now available.
</details>

<details>
<summary><strong>Claude Desktop</strong></summary>

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "open-ontologies": {
      "command": "/path/to/open-ontologies/target/release/open-ontologies",
      "args": ["serve"]
    }
  }
}
```

</details>

<details>
<summary><strong>Cursor / Windsurf / any MCP-compatible IDE</strong></summary>

Add to `.cursor/mcp.json` or equivalent:

```json
{
  "mcpServers": {
    "open-ontologies": {
      "command": "/path/to/open-ontologies/target/release/open-ontologies",
      "args": ["serve"]
    }
  }
}
```

</details>

<details>
<summary><strong>Docker</strong></summary>

```json
{
  "mcpServers": {
    "open-ontologies": {
      "command": "docker",
      "args": ["run", "-i", "--rm", "ghcr.io/fabio-rovai/open-ontologies", "serve"]
    }
  }
}
```

</details>

### Build your first ontology

```text
Build me a Pizza ontology following the Manchester University tutorial.
Include all 49 toppings, 22 named pizzas, spiciness value partition,
and defined classes (VegetarianPizza, MeatyPizza, SpicyPizza).
Validate it, load it, and show me the stats.
```

Claude generates Turtle, then runs the full pipeline automatically:

`onto_validate` → `onto_load` → `onto_stats` → `onto_reason` → `onto_stats` → `onto_lint` → `onto_enforce` → `onto_query` → `onto_save` → `onto_version`

Every build includes OWL reasoning (materializes inferred triples), design pattern enforcement, and automatic versioning.

---

## Studio (Desktop App)

The Studio is a native desktop application that wraps the same engine in a visual environment — no browser, no server to manage. It runs entirely on your machine: the engine sidecar handles RDF/OWL operations while the UI renders the graph in real time.

Think of it as **Protege meets an AI copilot**. Type "build ontology about cats" and watch a 1,400-class ontology appear in the tree — classes, properties, individuals, and axioms built automatically across 13 pipeline steps. Click any node to inspect its triples, trace connections via clickable pills, and follow every change through the lineage panel.

### How it works

The Studio launches three processes that communicate locally:

1. **Tauri 2 shell** — native window (macOS/Linux/Windows) with a WebKit webview
2. **Engine sidecar** — the same Rust binary, running as an HTTP MCP server on `localhost:8080`
3. **Agent sidecar** — Node.js process running Claude via the Agent SDK, connected to the engine over MCP

When you type in the chat panel, your message goes to the Agent sidecar, which sends it to Claude. Claude decides which `onto_*` tools to call, the engine executes them, and the UI refreshes the graph. The entire loop — prompt to visual update — takes seconds.

### Install and run

**Prerequisites:** Rust + Cargo · Node.js 18+

```bash
# 1. Build the engine binary (from repo root)
cargo build --release

# 2. Install JS dependencies
cd studio && npm install

# 3. Run
PATH=/opt/homebrew/bin:~/.cargo/bin:$PATH npm run tauri dev
```

The first launch compiles the Tauri shell (~2 min). Subsequent launches start in seconds.

### Features

| Feature | Description |
| --- | --- |
| **Virtualized Tree** | Ontology explorer that handles 1,500+ classes without lag. Hierarchy connector lines, collapsible branches, type-filtered legend (Class/Property/Individual), search with auto-expand, breadcrumb path navigation, and a connections panel showing domain/range relationships as clickable pills. Only visible rows are in the DOM — constant memory regardless of ontology size. |
| **AI Agent Chat** | Natural language ontology engineering via Claude Opus 4.6 + Agent SDK. Two build modes: `/build` runs a 13-step pipeline producing IES-level ontologies (500-1,500+ classes, 100-200+ properties), `/sketch` runs 3 steps for quick prototyping (~80 classes). Each tool call is shown in real time. |
| **Property Inspector** | Protege-style inline triple editor. Click any node to see its `rdfs:subClassOf`, `rdfs:label`, `rdfs:domain`, `rdfs:range` and all other triples. Edit in place, hover to delete, `+ Add` for new triples. Changes are immediately reflected in the graph. |
| **Lineage Panel** | Full audit trail from SQLite: every plan, apply, enforce, drift, monitor, and align event, grouped by session with timestamps. See exactly what Claude did and in what order. |
| **Named Save** | `⌘S` to save as `~/.open-ontologies/<name>.ttl`. Auto-saves to `studio-live.ttl` after every mutation so you never lose work. |

### Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `⌘J` | Toggle AI chat panel |
| `⌘I` | Toggle property inspector |
| `⌘S` | Save ontology |
| `F` | Fit graph to viewport (tree view) |
| `R` | Reset zoom (tree view) |
| `Esc` | Deselect node |
| `Shift+click` | Collapse/expand branch (tree view) |
| `Scroll` | Zoom in/out |
| `Click + drag` | Pan |

---

## Benchmarks

### OntoAxiom — LLM Axiom Identification

[OntoAxiom](https://arxiv.org/abs/2512.05594) tests axiom identification across 9 ontologies and 3,042 ground truth axioms.

| Approach | F1 | vs o1 (paper best) |
| --- | --- | --- |
| o1 (paper's best) | 0.197 | — |
| Bare Claude Opus | 0.431 | **+119%** |
| **MCP extraction** | **0.717** | **+264%** |

### Pizza Ontology — Manchester Tutorial

One sentence input: *"Build a Pizza ontology following the Manchester tutorial specification."*

| Metric | Reference (Protégé, ~4 hours) | AI-Generated (~5 min) | Coverage |
| --- | --- | --- | --- |
| Classes | 99 | 95 | **96%** |
| Properties | 8 | 8 | **100%** |
| Toppings | 49 | 49 | **100%** |
| Named Pizzas | 24 | 24 | **100%** |

### Deep Builder v2 — IES-Level Generation from Natural Language

The Studio's `/build` command runs a 13-step pipeline within a single persistent Claude session: foundation classes → per-branch deepening → object properties (2 batches) → datatype properties → disjoints → individuals → reason → save. Each step focuses on one aspect of the ontology, staying within output token limits while building on the previous step's context.

Input: *"build ontology about cats"*

| Metric | v1 (single-pass) | v2 (13-step pipeline) | IES Common (reference) |
| --- | ---: | ---: | ---: |
| Classes | 139 | **1,433** | 511 |
| Object properties | 0 | **218** | 162 |
| Datatype properties | 0 | **101** | 44 |
| Individuals | 0 | **358** | 21 |
| Triples | 558 | **15,556** | 4,041 |
| Max hierarchy depth | 3 | **11** | 8 |
| Build time | ~2 min | ~15 min | — (hand-built) |

v1 produced only classes because a single Claude turn exhausted its output tokens on class definitions, leaving nothing for properties or axioms. v2 solves this by decomposing the build into focused steps — each step generates one type of content (classes, then properties, then axioms) within a persistent session where Claude has full context of what was loaded in previous steps.

The `/sketch` command provides a lighter 3-step alternative (~80 classes, ~25 properties) for quick prototyping.

### Mushroom Classification — OWL Reasoning vs Expert Labels

**Dataset:** UCI Mushroom Dataset — 8,124 specimens classified by mycology experts.

| Metric | Result |
| --- | --- |
| Accuracy | **98.33%** |
| Recall (poisonous) | **100%** — zero toxic mushrooms missed |
| False negatives | **0** |
| Classification rules | 6 OWL axioms |

### Ontology Marketplace — 32 Standard Ontologies

All 32 marketplace ontologies fetched, `owl:imports` resolved, loaded, and reasoned over with both RDFS and OWL-RL profiles:

| Ontology | Classes | Properties | Triples | + RDFS | + OWL-RL | Fetch | RDFS | OWL-RL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| OWL 2 | 32 | 4 | 537 | +230 | +230 | 681ms | 6ms | 3ms |
| RDF Schema | 6 | 0 | 87 | +35 | +35 | 522ms | 2ms | 1ms |
| RDF Concepts | 7 | 0 | 127 | +31 | +31 | 545ms | 2ms | 2ms |
| BFO (ISO 21838) | 35 | 0 | 1,221 | +186 | +186 | 1,141ms | 5ms | 4ms |
| DOLCE/DUL | 93 | 118 | 1,917 | +666 | +692 | 2,208ms | 13ms | 12ms |
| Schema.org | 1,009 | 0 | 17,823 | +4,031 | **+13,670** | 558ms | 57ms | 117ms |
| FOAF | 28 | 60 | 631 | +4 | +31 | 940ms | 3ms | 2ms |
| SKOS | 5 | 18 | 252 | +55 | +55 | 218ms | 2ms | 1ms |
| Dublin Core Elements | 0 | 0 | 107 | +0 | +0 | 371ms | 2ms | 1ms |
| Dublin Core Terms | 22 | 0 | 700 | +256 | +261 | 259ms | 4ms | 3ms |
| DCAT | 58 | 89 | 2,841 | +223 | +254 | 975ms | 15ms | 11ms |
| VoID | 8 | 8 | 216 | +0 | +0 | 531ms | 2ms | 2ms |
| DOAP | 17 | 0 | 741 | +0 | +0 | 727ms | 2ms | 2ms |
| PROV-O | 39 | 50 | 1,146 | +202 | +203 | 472ms | 5ms | 4ms |
| OWL-Time | 23 | 58 | 1,296 | +165 | +165 | 256ms | 5ms | 4ms |
| W3C Organization | 22 | 33 | 748 | +9 | +21 | 639ms | 4ms | 3ms |
| SSN | 35 | 38 | 1,815 | +84 | +84 | 519ms | 6ms | 4ms |
| SOSA | 29 | 23 | 396 | +0 | +0 | 1,264ms | 3ms | 2ms |
| GeoSPARQL | 12 | 54 | 796 | +4 | +12 | 733ms | 3ms | 3ms |
| LOCN | 2 | 0 | 206 | +0 | +0 | 1,031ms | 2ms | 1ms |
| SHACL | 40 | 0 | 1,128 | +268 | +268 | 662ms | 5ms | 3ms |
| vCard | 75 | 84 | 882 | +0 | +46 | 854ms | 3ms | 3ms |
| ODRL | 71 | 50 | 2,157 | +73 | +76 | 798ms | 6ms | 5ms |
| Creative Commons | 6 | 0 | 115 | +0 | +49 | 184ms | 1ms | 1ms |
| SIOC | 14 | 83 | 615 | +0 | +2 | 863ms | 3ms | 2ms |
| ADMS | 4 | 13 | 151 | +0 | +0 | 747ms | 3ms | 1ms |
| GoodRelations | 98 | 102 | 1,834 | +15 | +42 | 2,299ms | 6ms | 6ms |
| FIBO (metadata) | 0 | 0 | 45 | +0 | +0 | 1,524ms | 3ms | 1ms |
| QUDT | 73 | 175 | 2,434 | +1,574 | +1,581 | 2,934ms | 14ms | 9ms |
| **Total** | **1,863** | **1,060** | **42,964** | **+8,111** | **+17,994** | — | — | — |

32/32 ontologies loaded, imports resolved, and reasoned. RDFS adds 18% more triples. OWL-RL adds **41%** — transitive/symmetric/inverse properties and equivalentClass expansion discover significantly more implicit knowledge. Schema.org jumps from +4,031 (RDFS) to +13,670 (OWL-RL) inferred triples in 117ms.

### Reasoning Performance — vs HermiT

**LUBM Scaling (load + reason cycle)**

| Axioms | Open Ontologies | HermiT | Speedup |
| --- | --- | --- | --- |
| 1,000 | 15ms | 112ms | **7.5×** |
| 5,000 | 14ms | 410ms | **29×** |
| 10,000 | 14ms | 1,200ms | **86×** |
| 50,000 | 15ms | 24,490ms | **1,633×** |

Full benchmark writeup: [docs/benchmarks.md](docs/benchmarks.md)

---

## IES Support

[IES (Information Exchange Standard)](https://github.com/IES-Org) is the UK National Digital Twin Programme's core ontology framework. It uses a 4D extensionalist (BORO) approach for modelling entities, events, states, and relationships. Open Ontologies supports the **full IES stack** — all three layers, SHACL shapes, and 26+ example datasets.

### The IES Layers

The marketplace includes all three tiers of the IES framework:

```text
onto_marketplace install ies-top     # ToLO — BORO foundations (~22 classes)
onto_marketplace install ies-core    # Core — persons, states, events (~131 classes)
onto_marketplace install ies         # Common — full ontology (511 classes, 206 properties)
```

### Benchmark

| Metric | IES Common |
| --- | --- |
| Classes | 511 |
| Object properties | 162 |
| Datatype properties | 44 |
| Total properties | 206 |
| Triples loaded | 4,041 |
| + RDFS inferred | **+3,094 (+77%)** |
| Fetch time | 911ms |
| RDFS reasoning | 63ms |
| Lint issues | 0 |

IES is the second-largest ontology in the marketplace by class count (after Schema.org). RDFS reasoning produces the richest inference gain of any non-general ontology — State, ClassOfEntity, and Event subclasses all generating deep transitive chains.

### Example Data

Load any of the 42+ IES example datasets directly:

```text
onto_pull https://raw.githubusercontent.com/IES-Org/ont-ies/main/docs/examples/sample-data/event-participation.ttl
onto_pull https://raw.githubusercontent.com/IES-Org/ont-ies/main/docs/examples/sample-data/hospital.ttl
onto_pull https://raw.githubusercontent.com/telicent-oss/ies-examples/main/additional_examples/ship_movement.ttl
```

### SHACL Validation

```text
onto_pull https://raw.githubusercontent.com/IES-Org/ont-ies/main/docs/specification/ies-common.shacl
onto_shacl
```

### Data Mapping: EPC → IES

The repo includes a sample of real UK Energy Performance Certificates ([benchmark/epc/epc-sample.csv](benchmark/epc/epc-sample.csv)) with a mapping config that transforms tabular EPC data into IES-shaped RDF:

```text
onto_load benchmark/generated/ies-building-extension.ttl
onto_ingest benchmark/epc/epc-sample.csv --mapping benchmark/epc/epc-ies-mapping.json
onto_reason --profile rdfs
```

This mirrors NDTP's actual pipeline: CSV → IES RDF → validate → reason → query.

### IES Building Extension — Comparison with NDTP/IRIS

The repo includes an [IES Building Extension](benchmark/generated/ies-building-extension.ttl) built from the UK EPC data schema and building science fundamentals, using IES 4D patterns. It was built independently — without reference to any existing implementation — then compared against the NDTP/IRIS production building ontology used in government data pipelines.

| Metric | NDTP/IRIS (hand-built) | Open Ontologies (AI-built) |
| --- | ---: | ---: |
| **Schema** | | |
| Classes | 244 | 525 |
| Properties | 34 | 104 |
| Triples (raw) | 1,346 | 3,229 |
| Lint issues | 2 | 0 |
| **Reasoning** | | |
| RDFS inferred | 621 | 662 |
| Triples after RDFS | 1,967 | 3,891 |
| Max hierarchy depth | 7 | 10 |
| Avg hierarchy depth | 2.89 | 2.02 |
| **EPC Coverage** | | |
| EPC columns covered | 18/36 (50%) | 36/36 (100%) |
| **4D Pattern** | | |
| Complete triads (Entity+State+ClassOf) | 14 | 129 |
| Enumerated individuals | 2 | 214 |

Built blind from the 105-column EPC schema, SAP methodology, and BORO 4D extensionalism — zero reference to the IRIS implementation. The two ontologies make different trade-offs: IRIS is more tightly curated with higher average hierarchy depth (2.89 vs 2.02), reflecting deliberate grouping by domain experts. Open Ontologies covers more of the EPC data schema and applies the BORO 4D pattern more systematically across the domain.

#### How the hierarchy emerges from building science

The ontology's depth (max 10 levels) is not hand-tuned — it follows the natural classification that building scientists use. The EPC data schema describes heating systems as flat text fields (`"Condensing gas boiler with radiators"`), but the underlying domain has layered structure:

```mermaid
graph TD
    HS[Heating System] --> CH[Central Heating]
    HS --> NC[Non-Central / Room Heating]

    CH --> WET[Wet Central Heating<br/><i>hydronic distribution</i>]
    CH --> WA[Warm Air Central Heating<br/><i>ducted air</i>]
    CH --> EC[Electric Central Heating<br/><i>storage / underfloor</i>]

    WET --> BB[Boiler-Based]
    WET --> HP[Heat Pump]
    WET --> DH[Community / District]

    BB --> CB[Combustion Boiler]
    BB --> CHP[Micro-CHP]

    CB --> GAS["Gas boiler"]
    CB --> OIL["Oil boiler"]
    CB --> LPG["LPG boiler"]
    CB --> COND["Condensing boiler"]
    CB --> COMBI["Combi boiler"]
    CB --> BACK["Back boiler"]

    HP --> ASHP["Air source"]
    HP --> GSHP["Ground source"]
    HP --> WSHP["Water source"]

    EC --> STOR["Storage heaters"]
    EC --> PNL["Panel heaters"]
    EC --> UF["Underfloor electric"]

    NC --> FIX[Fixed Room Heater]
    NC --> PORT[Portable Heater]

    FIX --> GROOM["Gas room heater"]
    FIX --> EROOM["Electric room heater"]
    FIX --> SFROOM["Solid fuel room heater"]

    style HS fill:#1a1a2e,color:#fff
    style CH fill:#16213e,color:#fff
    style NC fill:#16213e,color:#fff
    style WET fill:#0f3460,color:#fff
    style WA fill:#0f3460,color:#fff
    style EC fill:#0f3460,color:#fff
    style BB fill:#533483,color:#fff
    style HP fill:#533483,color:#fff
    style DH fill:#533483,color:#fff
    style CB fill:#e94560,color:#fff
    style CHP fill:#e94560,color:#fff
```

The same pattern applies to the building fabric — heat transfer physics dictates the grouping:

```mermaid
graph TD
    TE[Building Thermal Envelope] --> OP[Opaque Elements<br/><i>conduction-dominated</i>]
    TE --> TR[Transparent Elements<br/><i>radiation + conduction</i>]

    OP --> WALL[Walls]
    OP --> ROOF[Roofs]
    OP --> FLOOR[Floors]

    TR --> WIN[Windows]
    TR --> DOOR[Doors]

    WALL --> MAS[Masonry Walls<br/><i>thermal mass</i>]
    WALL --> FRM[Framed Walls<br/><i>stud bridges</i>]

    MAS --> CAV["Cavity wall"]
    MAS --> SOL["Solid brick"]
    MAS --> SND["Sandstone"]
    MAS --> GRN["Granite"]
    MAS --> COB["Cob"]

    FRM --> TF["Timber frame"]
    FRM --> SYS["System-built"]
    FRM --> PH["Park home"]

    ROOF --> PIT[Pitched Roof]
    ROOF --> FLT[Flat Roof]

    PIT --> COLD["Cold roof<br/><i>insulation at ceiling</i>"]
    PIT --> WARM["Warm roof<br/><i>insulation at rafter</i>"]
    PIT --> THATCH["Thatched"]

    WIN --> SGL["Single glazed"]
    WIN --> DBL["Double glazed"]
    WIN --> TPL["Triple glazed"]
    WIN --> SEC["Secondary glazing"]

    style TE fill:#1a1a2e,color:#fff
    style OP fill:#16213e,color:#fff
    style TR fill:#16213e,color:#fff
    style WALL fill:#0f3460,color:#fff
    style ROOF fill:#0f3460,color:#fff
    style FLOOR fill:#0f3460,color:#fff
    style WIN fill:#0f3460,color:#fff
    style DOOR fill:#0f3460,color:#fff
    style MAS fill:#533483,color:#fff
    style FRM fill:#533483,color:#fff
    style PIT fill:#533483,color:#fff
    style FLT fill:#533483,color:#fff
```

Each level in the tree is a real building science distinction — central vs room heating, hydronic vs warm air, combustion vs electric, masonry vs framed, cavity vs solid. An independent building scientist, given the same EPC data values, produces these same intermediate groupings ([verified by clean-room reproduction](docs/ies-ecosystem.md)). RDFS reasoning traverses these chains transitively, which is why a 10-level hierarchy generates 662 inferred triples from 3,229 raw.

### EPC Column Coverage Benchmark

Both ontologies tested against 36 key EPC data columns — can each ontology receive and represent the data from that column?

| Metric | NDTP/IRIS | Open Ontologies |
| --- | ---: | ---: |
| EPC columns covered | 18/36 (50%) | 36/36 (100%) |
| Triples | 1,346 | 3,229 |

Queries derived from published DESNZ/ONS EPC statistical reports — not from either ontology's class structure. Full benchmark: [benchmark/epc/](benchmark/epc/)

Use `onto_align` to map it to other domain ontologies:

```text
onto_load benchmark/generated/ies-building-extension.ttl
onto_align <other-ontology.ttl>
```

### Hierarchy Enforcement — Automated Inference Improvement

The `hierarchy` enforce pack detects flat spots in any ontology and suggests intermediate grouping classes. This is the same process used to deepen the building extension — now codified as a repeatable tool:

```text
onto_load my-ontology.ttl
onto_enforce --pack hierarchy
# → flags classes with >5 direct children
# → reports max depth, avg depth, hierarchy density
```

Tested on IES Common (511 classes), the tool found 24 flat spots. A clean-room agent — with no prior context — proposed 38 intermediate grouping classes based solely on the domain meaning of the flagged children:

```mermaid
graph LR
    subgraph Before["IES Common — before"]
        EP1[EventParticipant] --> P1["Prosecutor"]
        EP1 --> P2["Observer"]
        EP1 --> P3["Driver"]
        EP1 --> P4["Supplier"]
        EP1 --> P5["WeaponLocation"]
        EP1 --> P6["...52 direct children"]
    end

    subgraph After["IES Common — after hierarchy enforce"]
        EP2[EventParticipant] --> R[RoleInEvent]
        EP2 --> L[LocationInEvent]
        EP2 --> A[AssetInEvent]
        R --> LR2[LegalRole]
        R --> IR[InvestigativeRole]
        R --> CR[CommercialRole]
        LR2 --> Q1["Prosecutor"]
        LR2 --> Q2["Signatory"]
        IR --> Q3["Observer"]
        IR --> Q4["Investigator"]
        CR --> Q5["Supplier"]
        CR --> Q6["Negotiator"]
        L --> Q7["WeaponLocation"]
        L --> Q8["TargetLocation"]
        A --> Q9["VehicleUsed"]
    end

    style EP1 fill:#e94560,color:#fff
    style EP2 fill:#1a1a2e,color:#fff
    style R fill:#16213e,color:#fff
    style L fill:#16213e,color:#fff
    style A fill:#16213e,color:#fff
    style LR2 fill:#0f3460,color:#fff
    style IR fill:#0f3460,color:#fff
    style CR fill:#0f3460,color:#fff
```

| Metric | Before | After | Change |
| --- | ---: | ---: | ---: |
| Classes | 511 | 549 | +38 |
| RDFS inferred | 3,094 | 3,422 | **+328 (+10.6%)** |

The same tool, applied to any ontology, produces the same kind of improvement. The intermediate classes emerge from domain knowledge — not from reference to any other implementation.

### Further Reading

| Topic | Link |
| --- | --- |
| IES Ecosystem Demo | [docs/ies-ecosystem.md](docs/ies-ecosystem.md) |
| SPARQL Examples | [docs/ies-examples.md](docs/ies-examples.md) |
| Building Alignment | [docs/ies-alignment.md](docs/ies-alignment.md) |

---

## Tools

43 tools organized by function — available as MCP tools (prefixed `onto_`) and CLI subcommands:

| Category | Tools | Purpose |
| --- | --- | --- |
| **Core** | `validate` `load` `save` `clear` `stats` `query` `diff` `lint` `convert` `status` | RDF/OWL validation, querying, and management |
| **Marketplace** | `marketplace` | Browse and install 32 standard W3C/ISO/industry ontologies |
| **Remote** | `pull` `push` `import-owl` | Fetch/push ontologies, resolve owl:imports |
| **Schema** | `import-schema` | PostgreSQL → OWL conversion |
| **Data** | `map` `ingest` `shacl` `reason` `extend` | Structured data → RDF pipeline |
| **Versioning** | `version` `history` `rollback` | Named snapshots and rollback |
| **Lifecycle** | `plan` `apply` `lock` `drift` `enforce` `monitor` `monitor-clear` `lineage` | Terraform-style change management with webhook alerts and [OpenCheir](https://github.com/fabio-rovai/opencheir) governance integration |
| **Alignment** | `align` `align-feedback` | Cross-ontology class matching with self-calibrating confidence |
| **Clinical** | `crosswalk` `enrich` `validate-clinical` | ICD-10 / SNOMED / MeSH crosswalks |
| **Feedback** | `lint-feedback` `enforce-feedback` | Self-calibrating suppression |
| **Embeddings** | `embed` `search` `similarity` | Dual-space semantic search (text + Poincaré structural) |
| **Reasoning** | `reason` `dl_explain` `dl_check` | Native OWL2-DL SHOIQ tableaux reasoner |

---

## Architecture

### Engine

```mermaid
flowchart TD
    subgraph Clients["Clients"]
        Claude["Claude / LLM\nMCP stdio"]
        CLI["CLI\nonto_* subcommands"]
        Studio["Studio\nHTTP REST"]
    end

    subgraph Server["Open Ontologies Server"]
        direction TB

        subgraph Transport["Transport Layer"]
            MCP_HTTP["MCP Streamable HTTP\n/mcp"]
            REST["REST API\n/api/query · /api/update\n/api/save · /api/load · /api/lineage"]
        end

        subgraph ToolGroups["43 Tools"]
            direction LR
            Core["Core\nvalidate · load · save · clear\nstats · query · diff · lint\nconvert · status"]
            DataPipe["Data Pipeline\nmap · ingest · shacl\nreason · extend · import-schema"]
            Lifecycle["Lifecycle\nplan · apply · lock · drift\nenforce · monitor · lineage"]
            Advanced["Alignment + Clinical\nalign · crosswalk · enrich\nenrich · embed · search · similarity\ndl_explain · dl_check"]
            Version["Versioning\nversion · history · rollback"]
        end

        subgraph Core2["Core Engine"]
            GraphStore["Oxigraph Triple Store\nRDF/OWL in-memory\nSPARQL 1.1"]
            SQLite["SQLite\nlineage events\nversion snapshots\nlint/enforce feedback\nembedding vectors"]
            Reasoner["OWL2-DL Reasoner\nSHOIQ tableaux\nRDFS · OWL-RL"]
            Embedder["Embedding Engine\ntract-onnx (ONNX)\ntext + Poincaré structural"]
        end
    end

    subgraph External["External Sources"]
        PG["PostgreSQL\nschema import"]
        SPARQL["Remote SPARQL\nendpoints"]
        OWL["OWL URLs\nowl:imports chains"]
        Parquet["Parquet / Arrow\nclinical crosswalks\nICD-10 · SNOMED · MeSH"]
        Files["Files\nCSV · JSON · XML\nYAML · XLSX · Parquet"]
    end

    Claude -->|"MCP stdio"| MCP_HTTP
    CLI -->|"subcommands"| MCP_HTTP
    Studio -->|"sessionless"| REST

    MCP_HTTP --> ToolGroups
    REST --> ToolGroups

    ToolGroups --> GraphStore
    ToolGroups --> SQLite
    ToolGroups --> Reasoner
    ToolGroups --> Embedder

    Reasoner --> GraphStore
    Embedder --> SQLite

    DataPipe --> Files
    Advanced --> Parquet
    Core --> OWL
    Core --> SPARQL
    DataPipe --> PG
```

### Studio

```mermaid
flowchart TD
    subgraph UI["React UI (Vite + Tailwind CSS)"]
        Graph["Virtualized Tree\nDOM + virtual scroll"]
        Chat["AI Chat Panel\nZustand store"]
        Inspector["Property Inspector\nInline SPARQL edit"]
        Lineage["Lineage Panel\nAudit trail"]
        Save["Named Save\n⌘S → ~/.open-ontologies/"]
    end

    subgraph Tauri["Tauri 2 Shell (Rust)"]
        IPC["Tauri IPC\ninvoke / event"]
        ChatState["ChatState\nstdin/stdout pipe"]
    end

    subgraph Engine["Engine Sidecar (Rust / Axum)"]
        MCP["/mcp — MCP Streamable HTTP\n43 onto_* tools"]
        REST2["/api/query · /api/update\n/api/save · /api/load-turtle\n/api/stats · /api/lineage"]
        Store["Arc&lt;GraphStore&gt;\nOxigraph"]
        DB["SQLite"]
    end

    subgraph Agent["Agent Sidecar (Node.js)"]
        SDK["Claude Opus 4.6\nAgent SDK"]
        Proto["stdin/stdout JSON protocol"]
    end

    Graph -->|"SPARQL SELECT/UPDATE · REST"| REST2
    Inspector -->|"SPARQL UPDATE · REST"| REST2
    Lineage -->|"GET /api/lineage"| REST2
    Save -->|"POST /api/save"| REST2
    Chat -->|"invoke send_chat_message"| IPC
    IPC --> ChatState
    ChatState -->|"stdin { type: chat }"| Proto
    Proto --> SDK
    SDK -->|"MCP tools/call"| MCP
    SDK -->|"stdout { type: text/tool_call/done }"| Proto
    Proto -->|"Tauri emit agent-message"| Chat
    MCP --> Store
    REST2 --> Store
    Store --> DB
```

### Design decisions

| Decision | Reason |
| --- | --- |
| UI reads use sessionless REST | No MCP session management needed for SPARQL queries or stats |
| UI writes use REST `/api/update` + `/api/save` | Avoids session lifecycle issues in the Tauri WebKit webview |
| Agent writes go through MCP `tools/call` | The Agent SDK manages its own MCP session; Claude needs the full tool set |
| Shared `Arc<GraphStore>` | All MCP sessions and REST handlers share the same in-memory triple store |
| Agent sidecar over stdin/stdout | Keeps Node.js isolated; Tauri manages the full lifecycle |

---

## Stack

| Layer | Tech |
| --- | --- |
| Engine language | Rust (edition 2024) — single binary, no JVM |
| Triple store | Oxigraph 0.4 — pure Rust RDF/SPARQL 1.1 engine |
| MCP protocol | rmcp — Streamable HTTP transport |
| State / lineage / feedback | SQLite (rusqlite) |
| Clinical crosswalks | Apache Arrow / Parquet |
| Embeddings runtime | tract-onnx — pure Rust ONNX (optional) |
| Desktop shell | Tauri 2 |
| Frontend | React 19, Vite 7, TypeScript 5.8, Tailwind CSS 4 |
| Tree view | Virtualized DOM tree with virtual scroll (no canvas/WebGL dependencies) |
| UI state | Zustand 5 |
| AI agent | Claude Opus 4.6 via Agent SDK (Node.js sidecar) |

---

## Documentation

| Topic | Link |
| --- | --- |
| Quickstart | [docs/quickstart.md](docs/quickstart.md) |
| Data Pipeline | [docs/data-pipeline.md](docs/data-pipeline.md) |
| Ontology Lifecycle | [docs/lifecycle.md](docs/lifecycle.md) |
| Schema Alignment | [docs/alignment.md](docs/alignment.md) |
| OWL2-DL Reasoning | [docs/reasoning.md](docs/reasoning.md) |
| Semantic Embeddings | [docs/embeddings.md](docs/embeddings.md) |
| Clinical Crosswalks | [docs/clinical.md](docs/clinical.md) |
| IES Ecosystem | [docs/ies-ecosystem.md](docs/ies-ecosystem.md) |
| IES SPARQL Examples | [docs/ies-examples.md](docs/ies-examples.md) |
| IES:Building Alignment | [docs/ies-alignment.md](docs/ies-alignment.md) |
| Benchmarks | [docs/benchmarks.md](docs/benchmarks.md) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Changelog | [CHANGELOG.md](CHANGELOG.md) |

---

## License

MIT

<a href="https://glama.ai/mcp/servers/fabio-rovai/open-ontologies"><img width="380" height="200" src="https://glama.ai/mcp/servers/fabio-rovai/open-ontologies/badge" alt="Open Ontologies on Glama"></a>
