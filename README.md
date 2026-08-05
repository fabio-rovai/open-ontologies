[![MCP Toplist](https://mcptoplist.com/badge/glama%2Ffabio-rovai%2Fopen-ontologies.svg)](https://mcptoplist.com/server/glama%2Ffabio-rovai%2Fopen-ontologies)

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

Open Ontologies is a **Rust MCP server** and **desktop Studio** for AI-native ontology engineering. It exposes **70+ tools** that let Claude build, validate, query, diff, lint, version, reason over, align, plan, certify, and govern RDF/OWL ontologies using an in-memory Oxigraph triple store — with a full three-layer Dynamics → Causal → Planner architecture, a marketplace of 33 standard ontologies, clinical crosswalks, semantic embeddings, and a full lineage audit trail.

The **Studio** wraps the engine in a visual desktop environment: virtualized ontology tree with hierarchy lines, breadcrumb navigation, and connection explorer; AI chat panel with `/build` (IES-level deep) and `/sketch` (quick prototype) commands; Protégé-style property inspector; and lineage viewer.

No JVM. No Protégé.

---

## What's New (three-layer architecture + 13 new primitives)

The full **Dynamics → Causal → Planner** stack plus 13 new primitives. Every piece holds the **MCP-native** convention: the server provides validation and scaffolding, the connected LLM (Claude over MCP) does the intelligence. No internal LLM clients, no API keys, no provider abstractions.

### Three-layer architecture

| Layer | What it ships |
|---|---|
| **Dynamics** | `ActionSchema` + 4 MCP tools: `onto_action_register` / `_applicable` / `_apply` / `_list`. Concurrent atomic ticks, static causal laws (invariants), default-value laws, ramification via OWL-RL closure, non-deterministic outcomes with reproducible seed. |
| **Causal** | `onto_certify_action` with optional PyWhy backdoor identification (opt-in via `causal-pywhy` feature). Structural-proxy default + do-calculus opt-in + graceful fallback. |
| **Planner** | `onto_plan_compile_pddl` + `onto_plan_classical` (Fast Downward subprocess) + `onto_plan_validate` (sandbox-simulate). Solver stays client-side; server compiles + validates. |

### 13 new primitives

- **`onto_owl_shacl_coevolve_check`** + **`onto_owl_shacl_coevolve_incremental`** — SHACL validation against the OWL-RL closure, with dependency-graph routing so only shapes touching changed IRIs revalidate.
- **`onto_segment_retrieve`** — TBox-slice retrieval for ontology-grounded RAG.
- **`onto_extract_scaffold`** + **`onto_extract_validate`** — schema-guided structured extraction with typed datatype validation + conformance scoring.
- **`onto_cq_run`** + **`onto_verify_cq`** + **`onto_cq_verdicts_list`** — competency-question runner with pitfall hints + LLM-judgement loop.
- **`onto_classify_el`** — OWL-EL classification (transitive subsumption table, trivial pairs excluded).
- **`onto_eval_alignment`** — P/R/F1 over reference + computed alignment sets.
- **`onto_shape_combinatorics`** + **`onto_shape_induce`** — property-combination lattice + data-driven SHACL shape induction with support × confidence ranking.
- **`borderline_partition`** + **`borderline_record_verdict`** — generalised two-threshold review loop for any candidate set.
- **`onto_align_fuzzy`** — embedding-free fuzzy-logic adjudication with 10-rule Mamdani inference; HNSW is demoted to a candidate generator.
- **`onto_align_flora`** — end-to-end alignment pipeline pairing the signal extractor to the fuzzy adjudicator.
- **`onto_policy_register`** + **`onto_policy_list`** + **`onto_policy_check`** — authorisation gate that composes with `onto_certify_action` (Causal = risk; policy = authorisation).
- **`eval_rag`** + **`eval_rag_mmrag`** — Hit@k / MRR / faithfulness / token-Jaccard / ROUGE-1 scoring for retriever pipelines, with a dataset adapter.
- **`graph_projection_lossy_check`** — the auditor that pairs with `onto_segment_retrieve`.

### Validating end-to-end

```bash
cargo run --example three_layer_pipeline
```

Walks Dynamics register → PDDL compile → Fast-Downward-shaped sas_plan parse → orchestrator-side IRI bind → sandbox validate → CIVeX certify → apply with OWL-RL ramification → final state inspection. Every layer through its public API, no external dependencies (Python, DoWhy, Fast Downward) required.

Zero new external Rust dependencies; everything optional gates behind Cargo features. Full test suite (160+ tests) green on default build; `cargo clippy --lib --tests --examples -- -D warnings` clean across both default and `causal-pywhy` configurations.

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

> `serve` starts an **MCP server that speaks JSON-RPC over stdin/stdout** — it is not an interactive CLI, so on launch it will appear to "hang" while it waits for an MCP client to connect. That is expected. To try the tools directly from a terminal instead, use the CLI subcommands (e.g. `open-ontologies validate <file.ttl>`); to use it with an LLM, wire it into an MCP client as shown under [Connect to your MCP client](#connect-to-your-mcp-client).

**From source (Rust 1.85+):**

```bash
git clone https://github.com/fabio-rovai/open-ontologies.git
cd open-ontologies && cargo build --release
./target/release/open-ontologies init
```

For native Windows builds, see [docs/windows.md](docs/windows.md).

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
Include all 49 toppings, 24 named pizzas, spiciness value partition,
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

### Why virtualized tree (not 3D graph)

Prior to v0.1.12, the Studio used a D3.js horizontal tree and a 3D force-directed graph (Three.js / WebGL). Both worked for small ontologies (~100 classes) but became unusable at IES-level depth: the D3 tree couldn't handle 500+ nodes without layout thrashing, and the 3D graph froze the WebKit webview above 1,000 nodes.

The v2 deep builder changed the equation — a single `/build` command now produces 1,400+ classes. We replaced both views with a virtualized DOM tree: only visible rows exist in the DOM (constant memory regardless of ontology size), with hierarchy connector lines, type-filtered legend, search, breadcrumb navigation, and a connections panel. This handles the full IES Common (511 classes) and deep-built ontologies (1,400+ classes) without lag.

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
| **AI Agent Chat** | Natural language ontology engineering via Claude Opus 4.8 + Agent SDK. Two build modes: `/build` runs a 13-step pipeline producing IES-level ontologies (500-1,500+ classes, 100-200+ properties), `/sketch` runs 3 steps for quick prototyping (~80 classes). Each tool call is shown in real time. |
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

## Research Questions

The benchmarks below are not a feature tour. Each one exists to answer a specific question, and the answers include the unflattering ones.

| # | Question | Where it is measured | Answer as measured |
| --- | --- | --- | --- |
| **RQ1** | Does structured tool access give an LLM a materially different mode of access to an ontology than handing it the raw OWL file, or than giving it nothing but class and property names? | [OntoAxiom](#ontoaxiom--llm-axiom-identification), 3 conditions, 2 models | **Partly.** Both beat name lists decisively. Tool extraction and raw-file reading are at **parity** with each other: raw OWL wins the macro average, extraction wins the micro. The tools' remaining advantage is that every pair traces to a query against real triples. |
| **RQ2** | When two evaluation conditions disagree, how much of the gap is the method and how much is the scorer? | [OntoAxiom](#ontoaxiom--llm-axiom-identification), legacy vs unified evaluator | **Enough to invert the finding.** Three scorer asymmetries, all pointing one way, produced a reported result whose sign reverses under a shared evaluator, on both models and under both averages. |
| **RQ3** | In ontology alignment, which carries the result: how the similarity signals are weighted, or the constraint that the matching be 1-to-1? | [OAEI Anatomy](#oaei-ontology-alignment--anatomy-track), 5 weight configurations vs stable-matching ablation | **The constraint, overwhelmingly.** Removing stable matching as the only variable drops F1 from 0.829 to 0.728; the five-weight-configuration spread is 0.0033, and even that overstates it (the zero-structural-signal branch bypasses the weights entirely). |
| **RQ4** | How far does an alignment system get on a biomedical track with **no** domain background knowledge (no UMLS, no BioPortal, no LLM oracle)? | [OAEI Anatomy](#oaei-ontology-alignment--anatomy-track) and [Conference](#oaei-ontology-alignment--conference-track), full 2025 field | **Not far enough.** 9th of 13 on Anatomy, level with the lightweight lexical matcher and +0.063 F1 over a string-equality baseline. Below every system and both baselines on Conference. Precision is competitive; recall is the failure. |
| **RQ5** | Does a closed-world vocabulary check catch generated terms that open-world SHACL validation silently accepts? | [`onto-correctness-bench`](case-studies/onto-correctness-bench/): 3 vocabularies, 418 fabricated terms, 300 graphs | **Yes, completely.** SHACL returned `conforms=true` on **300/300** graphs containing a fabricated term. The closed-world gate flagged **300/300**, with zero false positives on clean graphs. Open-world semantics treat an undeclared predicate as merely unknown, so SHACL is structurally unable to see it. |

RQ2 and RQ4 are the ones worth reading if you are deciding whether to trust this repo. Both are negative results about work done here.

## Benchmarks

> **How to read these numbers.** Unless stated, LLM results are **single-run** (not averaged over seeds) and use **Claude Opus 4.8**. Several benchmark ontologies (Pizza, FOAF, Schema.org, OWL-Time) are widely published and may appear in an LLM's pretraining data, so a *bare-LLM* score is a **contamination-inclusive baseline**, not a clean measure of reasoning — the contribution is the **delta** the MCP tools add on top of that baseline, and whether that delta reproduces across models. To check exactly that, the repo ships a **cross-model ablation** driving the same tasks with a local **Qwen3-Coder-30B** as well as Claude — see [`benchmark/ontoaxiom/`](benchmark/ontoaxiom/). If the tool-augmented gain holds on a second, open model, the gain is a property of the tooling, not of one vendor's model.

### OntoAxiom — LLM Axiom Identification

[OntoAxiom](https://arxiv.org/abs/2512.05594) tests axiom identification across 9 ontologies and 3,042 ground truth axioms.

All conditions below are scored by a **single evaluator** (`benchmark/ontoaxiom/score_all_conditions.py`) with one shared normalizer, and both averages are reported, because the original scripts disagreed on all of that. `macro` = mean of per-(ontology, axiom) F1; `micro` = F1 over pooled TP/FP/FN.

| Approach | Input | macro F1 | micro F1 |
| --- | --- | --- | --- |
| o1 (paper's best) | Name lists | — | 0.197 |
| Bare Claude Opus | Name lists | 0.451 | 0.397 |
| Bare Qwen3-Coder-30B | Name lists | 0.223 | 0.176 |
| Claude Opus, raw OWL file | Full Turtle | **0.768** | 0.686 |
| Qwen3-Coder-30B, raw OWL file | Full Turtle | 0.673 | 0.667 |
| **MCP extraction** | **Full OWL** | 0.713 | **0.717** |

**The paper's "raw OWL hurts" result is a scoring artifact.** OntoAxiom reports that an LLM given the full OWL file (F1 = 0.323) does *worse* than one given only class/property name lists (0.431). Those two numbers came from scripts that disagreed on three axes: the name-list scorer splits camelCase and the raw-OWL scorer only lowercases; the first reports a **macro** mean and the second a **micro** F1; and they flip pair order on different axiom types. Every one of those differences penalizes the raw-OWL condition, because that is the one where the model reads real Turtle and therefore answers in QNames (`foaf:Person`) and `rdfs:label` text (`"personal mailbox"` for `mbox`) rather than bare local names. **0.431 and 0.323 were never the same statistic.**

Rescoring the *same stored predictions* under one evaluator flips the sign on both models and under both averages: Claude 0.451 → **0.768** macro (0.397 → 0.686 micro), Qwen 0.246 → **0.673** macro, winning 33/43 and 33/38 cells respectively. `score_condition_d.py --legacy` reproduces the broken 0.323 exactly, so the bug is demonstrated rather than asserted. The correction moves 0 of 5,083 name-list pairs, so it cannot flatter the baseline, and it still under-credits raw OWL: 51.8% of Claude's pairs are label text no normalizer here can match. Full analysis and reproduction: [`benchmark/ontoaxiom/ONTOAXIOM_SHOWDOWN.md`](benchmark/ontoaxiom/ONTOAXIOM_SHOWDOWN.md).

Corrected, reading the ontology and SPARQL-extracting it are **at parity** — raw OWL wins macro (0.768 vs 0.713), extraction wins micro (0.717 vs 0.686). So the tools' edge is **auditability, not F1**: every MCP pair traces to a query against real triples, whereas an LLM reading a file can still hallucinate a plausible pair and no F1 score will say which.

### Pizza Ontology — Manchester Tutorial

One sentence input: *"Build a Pizza ontology following the Manchester tutorial specification."*

| Metric | Reference (Protégé, ~4 hours) | AI-Generated (~5 min) | Coverage |
| --- | --- | --- | --- |
| Classes | 99 | 95 | **96%** |
| Properties | 8 | 8 | **100%** |
| Toppings | 49 | 49 | **100%** |
| Named Pizzas | 24 | 24 | **100%** |

### `/sketch` vs `/build` — Two Build Modes

The Studio provides two build commands for different use cases. Both take the same input — *"build ontology about cats"* — but produce very different results:

| Metric | `/sketch` (3 steps, ~2 min) | `/build` (13 steps, ~15 min) | IES Common (reference) |
| --- | ---: | ---: | ---: |
| Classes | 95 | **1,433** | 511 |
| Object properties | 15 | **218** | 162 |
| Datatype properties | 5 | **101** | 44 |
| Individuals | 3 | **358** | 21 |
| Disjoints | 6 | **60+** | — |
| Max hierarchy depth | 5 | **11** | 8 |
| Build time | ~2 min | ~15 min | — (hand-built) |

**`/sketch`** runs 3 steps: classes + properties in one Turtle block, axioms + individuals, then save. Good for quick domain exploration or demo prototyping. Produces a complete ontology with hierarchy, properties, and individuals — but at a fraction of the depth.

**`/build`** runs a 13-step pipeline within a single persistent Claude session: foundation classes → per-branch deepening (4 passes) → gap filling → object properties (2 batches) → datatype properties → disjoints → individuals → reason → save. Each step focuses on one aspect of the ontology, staying within output token limits while building on the previous step's context. The result exceeds IES Common on every metric.

`/sketch` is comparable to the Pizza benchmark (95 classes, 8 properties). `/build` produces IES-level ontologies — deep enough for production use.

### Mushroom Classification — OWL Reasoning vs Expert Labels

**Dataset:** UCI Mushroom Dataset — 8,124 specimens classified by mycology experts.

| Metric | Result |
| --- | --- |
| Accuracy | **98.33%** |
| Recall (poisonous) | **100%** — zero toxic mushrooms missed |
| False negatives | **0** |
| Classification rules | 6 OWL axioms |

### Ontology Marketplace — 33 Standard Ontologies

The 29 general-purpose marketplace ontologies (the four IES entries are covered separately under [IES Support](#ies-support)) fetched, `owl:imports` resolved, loaded, and reasoned over with both RDFS and OWL-RL profiles. Regenerate with `python3 benchmark/marketplace_benchmark.py`:

| Ontology | Classes | Properties | Triples | + RDFS | + OWL-RL | Fetch | RDFS | OWL-RL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| OWL 2 | 29 | 0 | 537 | +230 | +230 | 276ms | 1ms | 1ms |
| RDF Schema | 4 | 0 | 87 | +35 | +35 | 268ms | 0ms | 0ms |
| RDF Concepts | 13 | 0 | 127 | +31 | +31 | 175ms | 0ms | 0ms |
| BFO (ISO 21838) | 35 | 2 | 1,221 | +186 | +186 | 254ms | 2ms | 1ms |
| DOLCE/DUL | 79 | 118 | 1,917 | +666 | +692 | 181ms | 7ms | 8ms |
| Schema.org | 1,032 | 1,674 | 17,949 | +4,082 | **+14,236** | 679ms | 56ms | 136ms |
| FOAF | 15 | 62 | 631 | +4 | +31 | 656ms | 1ms | 1ms |
| SKOS | 5 | 28 | 252 | +55 | +55 | 81ms | 1ms | 1ms |
| Dublin Core Elements | 0 | 15 | 107 | +0 | +0 | 315ms | 1ms | 0ms |
| Dublin Core Terms | 23 | 70 | 700 | +256 | +261 | 129ms | 2ms | 2ms |
| DCAT | 49 | 110 | 2,841 | +223 | +254 | 356ms | 7ms | 8ms |
| VoID | 7 | 27 | 216 | +0 | +0 | 333ms | 0ms | 0ms |
| DOAP | 22 | 45 | 741 | +0 | +0 | 352ms | 1ms | 1ms |
| PROV-O | 31 | 59 | 1,146 | +202 | +203 | 204ms | 3ms | 2ms |
| OWL-Time | 29 | 58 | 1,296 | +165 | +165 | 206ms | 3ms | 2ms |
| W3C Organization | 17 | 38 | 748 | +9 | +21 | 259ms | 2ms | 2ms |
| SSN | 23 | 38 | 1,815 | +84 | +84 | 262ms | 2ms | 2ms |
| SOSA | 17 | 23 | 396 | +0 | +0 | 323ms | 1ms | 1ms |
| GeoSPARQL | 15 | 54 | 796 | +4 | +12 | 227ms | 1ms | 1ms |
| LOCN | 3 | 15 | 206 | +0 | +0 | 363ms | 1ms | 1ms |
| SHACL | 48 | 101 | 1,128 | +268 | +268 | 277ms | 2ms | 2ms |
| vCard | 64 | 84 | 882 | +0 | +46 | 316ms | 1ms | 1ms |
| ODRL | 31 | 58 | 2,157 | +73 | +76 | 212ms | 3ms | 3ms |
| Creative Commons | 21 | 13 | 115 | +0 | +49 | 564ms | 1ms | 1ms |
| SIOC | 17 | 86 | 615 | +0 | +2 | 364ms | 1ms | 1ms |
| ADMS | 8 | 16 | 151 | +0 | +0 | 377ms | 1ms | 1ms |
| GoodRelations | 43 | 102 | 1,834 | +15 | +42 | 303ms | 3ms | 4ms |
| FIBO (metadata) | 0 | 0 | 48 | +0 | +0 | 505ms | 4ms | 4ms |
| QUDT | 99 | 196 | 2,434 | +1,574 | +1,581 | 2,100ms | 50ms | 49ms |
| **Total** | **1,779** | **3,092** | **43,093** | **+8,162** | **+18,560** | — | — | — |

29/29 ontologies loaded, imports resolved, and reasoned. RDFS adds 18% more triples. OWL-RL adds **43%** — transitive/symmetric/inverse properties and equivalentClass expansion discover significantly more implicit knowledge. Schema.org jumps from +4,082 (RDFS) to +14,236 (OWL-RL) inferred triples in 136ms.

Class and property counts are structural, not declaration-only: a term counts if it is typed (`owl:Class`, `rdfs:Class`, `owl:ObjectProperty`, `owl:DatatypeProperty`, `rdf:Property`) **or** used as one (`rdfs:subClassOf`/`subPropertyOf`/`domain`/`range` position). Vocabularies that never issue OWL type declarations, such as Schema.org, are therefore counted rather than reported as empty. Terms in the `rdf:`, `rdfs:` and `owl:` namespaces are excluded so a vocabulary is not credited with the meta-vocabulary it is written in. That exclusion is why OWL 2, RDF Schema and RDF Concepts report **0 properties**: every property they define is, by definition, in an excluded namespace. FIBO's marketplace entry is the metadata module only (48 triples), which declares no terms of its own.

### Compiled Claim Verification — measured vs HermiT

The `claimcheck` module compiles an ontology **once** (inferred hierarchy +
disjointness, including pairs derived by sound propagation rules) into
per-class token bitsets, then verifies candidate claims — the "is this set of
triples consistent with the ontology?" question — with **no reasoning at query
time**: two 64-bit ANDs per class pair, witness axiom extracted for the
explanation.

Same ontology (canonical `pizza.owl`), same task, same machine, verdicts
cross-checked against HermiT 1.4.3.456:

| Per-claim consistency check | median | p95 | throughput |
| --- | --- | --- | --- |
| HermiT (warm JVM, ontology pre-loaded) | 4,936 µs | — | ~200/s |
| **open-ontologies compiled check** | **0.3 µs** | **0.4 µs** | **3.1M/s (11.2M/s batched)** |

Correctness before speed:

- **0 disagreements with HermiT** across 78,884 exhaustively-audited class
  pairs (13 ontologies) and 793 structurally adversarial claims.
- **Sound by construction**: a `Rejected` verdict is backed by a derivable
  contradiction and names the witnessing axiom. 100% contradiction recall on
  both fully-audited ontologies.
- **Explicit incompleteness envelope**: anything the compiled surface cannot
  decide returns `Undetermined` and routes to a reasoner-backed residual tier
  — it is never guessed.
- Closed-world vocabulary checks catch hallucinated classes/properties that
  open-world OWL semantics structurally cannot flag.

Offline compile: one classification pass (~120 ms for Pizza) via any complete
OWL reasoner; the shipped hot path is pure Rust with no JVM dependency.
Reproduction scripts: [benchmark/layer3-prototype/](benchmark/layer3-prototype/).

Design, measurements and envelope: [docs/layer3-compiled-reasoning.md](docs/layer3-compiled-reasoning.md).
Full benchmark methodology: [docs/benchmarks.md](docs/benchmarks.md)

### OAEI Ontology Alignment — Anatomy Track

[OAEI](https://oaei.ontologymatching.org/) is the standard benchmark for ontology alignment systems. The Anatomy track aligns the mouse anatomy ontology (2,744 classes) to the human anatomy fragment of the NCI Thesaurus (3,304 classes) against 1,516 reference mappings.

The comparison below is the **complete** OAEI 2025 Anatomy field, reproduced from Table 9 of the official results paper ([Vol-4144, om2025-oaei-paper0](https://ceur-ws.org/Vol-4144/om2025-oaei-paper0.pdf)), including both baselines. Open Ontologies is inserted at its measured rank. Nothing is filtered.

| System | Precision | Recall | F1 |
| --- | ---: | ---: | ---: |
| Matcha | 0.951 | 0.931 | **0.941** |
| Agent-OM | 0.959 | 0.883 | **0.920** |
| ALIN | 0.942 | 0.884 | **0.912** |
| LogMapLLM | 0.964 | 0.842 | **0.899** |
| LogMap-Bio | 0.885 | 0.911 | **0.898** |
| MDMapper | 0.899 | 0.879 | **0.889** |
| LogMap | 0.917 | 0.848 | **0.881** |
| LogMapKG | 0.917 | 0.848 | **0.881** |
| **Open Ontologies** | **0.960** | **0.730** | **0.829** |
| DRAL-OA | 0.830 | 0.827 | **0.828** |
| LogMapLt | 0.962 | 0.728 | **0.828** |
| *StringEquiv (baseline)* | 0.997 | 0.622 | **0.766** |
| LSMatch | 0.952 | 0.634 | **0.761** |

**Read this honestly.** Open Ontologies ranks **9th of 13** on F1. Its precision (0.960) is third in the field, but its recall (0.730) is second-from-bottom among non-baseline systems, and the resulting F1 sits **level with LogMapLt**, the deliberately lightweight lexical matcher, and 0.112 below the leader. The `StringEquiv` baseline reaches 0.766 by normalised string equality alone, so the margin this system earns over pure string matching is **+0.063 F1**. That is the number to beat, and it is not yet a competitive result.

The gap is recall, and its cause is identifiable: this system carries **no domain background knowledge**. Anatomy is a track where UMLS and BioPortal lookups are what separate the 0.88+ band from the rest, and every system above Open Ontologies in the table either uses biomedical background knowledge, an LLM oracle, or both. Alignment here uses 7 weighted signals (label similarity, property/parent/instance/restriction/neighbourhood overlap, embedding similarity), stable 1-to-1 matching, and a label penalty when no structural evidence is available.

The defensible finding from this track is therefore **not** the headline F1. It is the ablation: stable 1-to-1 matching is what makes the difference, and the signal weights are nearly irrelevant once it is applied.

| Configuration | Precision | Recall | F1 |
| --- | ---: | ---: | ---: |
| With stable matching | 0.960 | 0.730 | 0.829 |
| Without stable matching | 0.102 | 0.846 | 0.182 |

Removing stable matching produces 12,557 candidates against a 1,516-mapping reference. See issues [#8](https://github.com/fabio-rovai/open-ontologies/issues/8), [#9](https://github.com/fabio-rovai/open-ontologies/issues/9), [#10](https://github.com/fabio-rovai/open-ontologies/issues/10); background-knowledge integration is the open work.

### OAEI Ontology Alignment — Conference Track

15 of the 21 conference-track pairs, micro-averaged:

| System | Precision | Recall | F1 |
| --- | ---: | ---: | ---: |
| ALIN | 0.62 | 0.68 | **0.65** |
| LogMap | 0.76 | 0.56 | **0.64** |
| Matcha | 0.77 | 0.53 | **0.63** |
| Agent-OM | 0.64 | 0.59 | **0.61** |
| MDMapper | 0.69 | 0.50 | **0.58** |
| *edna (baseline)* | 0.74 | 0.45 | **0.56** |
| LogMapLt | 0.68 | 0.47 | **0.56** |
| LSMatch | 0.83 | 0.41 | **0.55** |
| *StringEquiv (baseline)* | 0.76 | 0.41 | **0.53** |
| **Open Ontologies** | **0.693** | **0.320** | **0.438** |

**Not comparable like-for-like, and worse than it looks.** The OAEI rows are Table 10 of the 2025 results paper, evaluated over all 21 pairs against the `rar2` reference at each system's F1-optimal threshold. The Open Ontologies row covers 15 pairs at a fixed confidence threshold. With that caveat stated, the result is unambiguous: **below every participating system and below both baselines**. Best pairs are `ekaw-iasted` (0.588) and `ekaw-sigkdd` (0.533); worst is `edas-sigkdd` (0.211). Conference is a track where the same recall failure that costs Anatomy its rank costs more, because there is no lexical redundancy to fall back on.

---

## IES Support

[IES (Information Exchange Standard)](https://github.com/IES-Org) is the UK National Digital Twin Programme's core ontology framework. It uses a 4D extensionalist (BORO) approach for modelling entities, events, states, and relationships. Open Ontologies supports the **full IES stack** — all three layers, SHACL shapes, and example datasets from the IES-Org GitHub repositories.

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

Load IES example datasets directly from the official repositories:

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

70+ tools organized by function — available as MCP tools (prefixed `onto_`) and CLI subcommands:

| Category | Tools | Purpose |
| --- | --- | --- |
| **Core** | `validate` `load` `save` `clear` `stats` `query` `diff` `lint` `convert` `status` | RDF/OWL validation, querying, and management |
| **Repository** | `repo_list` `repo_load` | Browse and load ontologies from configured `[general] ontology_dirs` directories |
| **Cache** | `cache_status` `cache_list` `cache_remove` `unload` `recompile` | On-disk N-Triples compile cache, idle-TTL eviction, per-name management ([details](docs/cache-and-registry.md)) |
| **Marketplace** | `marketplace` | Browse and install 33 standard W3C/ISO/industry ontologies |
| **Remote** | `pull` `push` `import` | Fetch/push ontologies, resolve owl:imports |
| **Schema** | `import-schema` `sql-ingest` | Postgres + DuckDB → OWL + SQL → RDF ingest |
| **Data** | `map` `ingest` `shacl` `shacl_check` `vocab_check` `reason` `extend` | Structured data → RDF pipeline; `vocab_check` = closed-world check that generated data uses only ontology-declared terms (catches what open-world SHACL misses) |
| **Versioning** | `version` `history` `rollback` | Named snapshots and rollback |
| **Lifecycle** | `plan` `apply` `lock` `drift` `enforce` `monitor` `monitor-clear` `lineage` | Terraform-style change management with webhook alerts and [OpenCheir](https://github.com/fabio-rovai/opencheir) governance integration |
| **Alignment** | `align` `align_feedback` `align_fuzzy` `align_flora` | Cross-ontology class matching with self-calibrating weights + fuzzy-logic adjudication and end-to-end signal-driven pipeline |
| **HNSW** | `hnsw_build` | Persisted HNSW indices (cosine + Poincaré) over class embeddings |
| **Clinical** | `crosswalk` `enrich` `validate_clinical` | ICD-10 / SNOMED / MeSH crosswalks (93-row sample ships in `data/crosswalks.parquet`; run `python scripts/build_crosswalks.py` to rebuild or extend) |
| **Feedback** | `lint_feedback` `enforce_feedback` | Self-calibrating suppression |
| **Embeddings** | `embed` `search` `similarity` | Dual-space semantic search (text + Poincaré structural) |
| **Reasoning** | `reason` `dl_explain` `dl_check` `classify_el` | Native OWL2-DL SHOIQ tableaux + OWL-EL classification |
| **Dynamics** | `action_register` `action_applicable` `action_apply` `action_list` `action_apply_concurrent` `invariant_register` `invariant_list` `invariant_remove` `invariant_check` `default_register` `default_apply` | Action schemas + concurrent atomic ticks + static causal laws + default values |
| **Causal** | `certify_action` | Four-verdict causal certificate (EXECUTE / REJECT / EXPERIMENT / ABSTAIN); optional `causal-pywhy` feature enables backdoor identification |
| **Planner** | `plan_compile_pddl` `plan_classical` `plan_validate` | Compile + validate on the server; solver (Fast Downward) is a client-side subprocess |
| **Governance** | `policy_register` `policy_list` `policy_check` | Authorisation rules; composes with `certify_action` |
| **RAG** | `segment_retrieve` `graph_projection_lossy_check` | TBox-slice retrieval + projection-loss auditor |
| **Extraction** | `extract_scaffold` `extract_validate` | Schema-guided structured-extraction prompt + validator |
| **CQs** | `cq_run` `verify_cq` `cq_verdicts_list` | Competency-question runner with pitfall hints + judgement loop |
| **Shape induction** | `shape_combinatorics` `shape_induce` | Property-combination lattice + data-driven SHACL induction |
| **Borderline loop** | `borderline_partition` `borderline_record_verdict` | Generalised two-threshold review pattern for any candidate set |
| **SQL sync** | `sql_sync_state` `sql_sync_reset` `sql_sync_states_list` | CDC watermark tracking for incremental SQL ingest |
| **Evaluation** | `eval_alignment` `eval_rag` `eval_rag_mmrag` | Alignment P/R/F1 + RAG Hit@k / MRR / faithfulness + dataset adapter |

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

        subgraph ToolGroups["70+ Tools"]
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
        MCP["/mcp — MCP Streamable HTTP\nonto_* tools"]
        REST2["/api/query · /api/update\n/api/save · /api/load-turtle\n/api/stats · /api/lineage"]
        Store["Arc&lt;GraphStore&gt;\nOxigraph"]
        DB["SQLite"]
    end

    subgraph Agent["Agent Sidecar (Node.js)"]
        SDK["Claude Opus 4.8\nAgent SDK"]
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
| AI agent | Claude Opus 4.8 via Agent SDK (Node.js sidecar) |

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
| Determinism & corrected results | [docs/determinism.md](docs/determinism.md) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Changelog | [CHANGELOG.md](CHANGELOG.md) |

---

## Citation

Open Ontologies is described in a preprint. The alignment engine implements the
stable-matching method introduced there; the Causal layer builds on the
intervention-verification framework of CIVeX.

- **Open Ontologies: Tool-Augmented Ontology Engineering with Stable Matching Alignment.** Fabio Rovai, 2026. [arXiv:2605.09184](https://arxiv.org/abs/2605.09184)
- **CIVeX: Causal Intervention Verification for Language Agents.** Fabio Rovai, 2026. [arXiv:2605.09168](https://arxiv.org/abs/2605.09168)

```bibtex
@article{rovai2026openontologies,
  title   = {Open Ontologies: Tool-Augmented Ontology Engineering with Stable Matching Alignment},
  author  = {Rovai, Fabio},
  journal = {arXiv preprint arXiv:2605.09184},
  year    = {2026},
  doi     = {10.48550/arXiv.2605.09184},
  url     = {https://arxiv.org/abs/2605.09184}
}
```

See [`CITATION.cff`](CITATION.cff) for machine-readable metadata. It powers GitHub's "Cite this repository" button.

---

## Maintainer

Maintained by [The Tesseract Academy](https://gov.tesseract.academy) (Kampakis and Co Ltd), a UK research and data-science practice. Applied case studies built with this toolkit are published in the [Tesseract Foundational Research](https://gov.tesseract.academy/research) programme.

## License

MIT

<a href="https://glama.ai/mcp/servers/fabio-rovai/open-ontologies"><img width="380" height="200" src="https://glama.ai/mcp/servers/fabio-rovai/open-ontologies/badge" alt="Open Ontologies on Glama"></a>
