# Open Ontologies: A Terraforming MCP for Knowledge Graphs: validate, classify, and govern AI-generated ontologies.

[![CI](https://github.com/fabio-rovai/open-ontologies/actions/workflows/ci.yml/badge.svg)](https://github.com/fabio-rovai/open-ontologies/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)


Open Ontologies is a standalone MCP server and CLI for AI-native ontology engineering. It exposes 35 tools that let Claude validate, query, diff, lint, version, and persist RDF/OWL ontologies using an in-memory Oxigraph triple store; plus plan changes, detect drift, enforce design patterns, monitor health, and track lineage.

Written in Rust, ships as a single binary. No JVM, no Protege, no GUI.

## Quick Start

### 1. Build

Requires Rust 1.85+ (edition 2024).

```bash
git clone https://github.com/fabio-rovai/open-ontologies.git
cd open-ontologies
cargo build --release
./target/release/open-ontologies init
```

### 2. Connect to Claude Code

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

### 3. Build your first ontology

```text
Build me a Pizza ontology following the Manchester University tutorial.
Include all 49 toppings, 22 named pizzas, spiciness value partition,
and defined classes (VegetarianPizza, MeatyPizza, SpicyPizza).
Validate it, load it, and show me the stats.
```

Claude generates Turtle, then automatically calls `onto_validate` → `onto_load` → `onto_stats` → `onto_lint` → `onto_query`, fixing errors along the way.

### Demo: Database → Ontology in 3 commands

```bash
# Import a PostgreSQL schema as OWL
open-ontologies import-schema postgres://demo:demo@localhost/shop

# Classify with native OWL2-DL reasoner
open-ontologies reason --profile owl-dl

# Query the result
open-ontologies query "SELECT ?c ?label WHERE { ?c a owl:Class . ?c rdfs:label ?label }"
```

## Why This Exists

You can ask Claude to generate an ontology in a single prompt — and it will. But single-shot generation has real problems:

| Problem | What goes wrong |
| ------- | --------------- |
| No validation | Invalid Turtle — wrong prefixes, unclosed brackets, bad URIs |
| No verification | No way to check counts or structure without SPARQL |
| No iteration | Can't diff versions, lint for missing labels, or run competency questions |
| No persistence | Ontology lives only in chat context — no versioning, no rollback |
| No scale | Context window holds ~2,000 triples; real ontologies need a triple store |
| No integration | Can't push to SPARQL endpoints or resolve owl:imports chains |

Open Ontologies solves all of these. It's a proper RDF/SPARQL engine (Oxigraph) exposed as MCP tools that Claude calls automatically.

| Layer | Tool | What it does |
| ----- | ---- | ------------ |
| Generation | Claude / GPT / LLaMA | Generates OWL/RDF from natural language |
| **Validation** | **Open Ontologies** | **Validates, classifies, enforces, monitors** |
| Storage | SPARQL endpoint / triplestore | Persists the production ontology |
| Consumption | Your app / API / pipeline | Queries the knowledge graph |

## How It Works

You provide domain requirements in natural language. Claude generates Turtle/OWL, then **dynamically decides** which MCP tools to call based on what each tool returns — validating, fixing, re-loading, querying, iterating until the ontology is correct.

```mermaid
flowchart TD
    You["You — 'Build me a Pizza ontology'"]
    Claude["Claude generates Turtle"]
    Validate["onto_validate"]
    Fix["Claude fixes errors"]
    Load["onto_load"]
    Stats["onto_stats"]
    Lint["onto_lint"]
    Query["onto_query — SPARQL"]
    Save["onto_save"]
    Version["onto_version"]

    You --> Claude
    Claude --> Validate
    Validate -->|"syntax errors"| Fix
    Fix --> Validate
    Validate -->|"ok"| Load
    Load --> Stats
    Stats -->|"wrong counts"| Claude
    Stats -->|"ok"| Lint
    Lint -->|"issues"| Fix
    Lint -->|"clean"| Query
    Query -->|"gaps found"| Claude
    Query -->|"all correct"| Version
    Version --> Save
```

This is not a fixed pipeline. Claude is the orchestrator — it decides what to call next based on results.

## Tools

35 tools organized by function:

| Category | Tools | Purpose |
| -------- | ----- | ------- |
| **Core** | `validate`, `load`, `save`, `clear`, `stats`, `query`, `diff`, `lint`, `convert`, `status` | RDF/OWL validation, querying, and management |
| **Remote** | `pull`, `push`, `import-owl` | Fetch/push ontologies, resolve owl:imports |
| **Schema** | `import-schema` | PostgreSQL → OWL conversion |
| **Data** | `map`, `ingest`, `shacl`, `reason`, `extend` | Structured data → RDF pipeline |
| **Versioning** | `version`, `history`, `rollback` | Named snapshots and rollback |
| **Lifecycle** | `plan`, `apply`, `lock`, `drift`, `enforce`, `monitor`, `monitor-clear`, `lineage` | Terraform-style change management |
| **Clinical** | `crosswalk`, `enrich`, `validate-clinical` | ICD-10 / SNOMED / MeSH crosswalks |
| **Reasoning** | `reason` (rdfs, owl-rl, owl-rl-ext, owl-dl), `dl_explain`, `dl_check` | Native SHOIQ tableaux reasoner |

All tools are available both as MCP tools (prefixed `onto_`) and as CLI subcommands.

```bash
open-ontologies <command> [args] [--pretty] [--data-dir ~/.open-ontologies]
```

## Data Pipeline

Take any structured data — CSV, JSON, Parquet, XLSX, XML, YAML — and terraform it into a validated, reasoned knowledge graph.

```mermaid
flowchart LR
    Data["CSV / JSON / XLSX / ..."]
    Map["onto_map — generate mapping"]
    Ingest["onto_ingest — parse to RDF"]
    Validate["onto_shacl — check constraints"]
    Reason["onto_reason — infer new facts"]
    Query["onto_query — ask questions"]

    Data --> Map
    Map --> Ingest
    Ingest --> Validate
    Validate -->|violations| Map
    Validate -->|ok| Reason
    Reason --> Query
```

| Manual process | Open Ontologies equivalent |
| -------------- | ------------------------- |
| Domain expert defines classes by hand | `import-schema` or Claude generates OWL |
| Analyst maps spreadsheet columns to ontology | `map` auto-generates mapping config |
| Data engineer writes ETL to RDF | `ingest` parses CSV/JSON/Parquet/XLSX → RDF |
| Ontologist validates data constraints | `shacl` checks cardinality, datatypes, classes |
| Reasoner classifies instances (Protege + HermiT) | `reason` runs native OWL2-DL classification |
| Quality reviewer checks consistency | `enforce` + `lint` + `monitor` |

### Supported formats

| Format | Extension |
| ------ | --------- |
| CSV | `.csv` |
| JSON | `.json` |
| NDJSON | `.ndjson` |
| XML | `.xml` |
| YAML | `.yaml` |
| Excel | `.xlsx` |
| Parquet | `.parquet` |

### Mapping config

The mapping bridges tabular data and RDF:

```json
{
  "base_iri": "http://www.co-ode.org/ontologies/pizza/pizza.owl#",
  "id_field": "name",
  "class": "http://www.co-ode.org/ontologies/pizza/pizza.owl#NamedPizza",
  "mappings": [
    { "field": "base", "predicate": "pizza:hasBase", "lookup": true },
    { "field": "topping1", "predicate": "pizza:hasTopping", "lookup": true },
    { "field": "price", "predicate": "pizza:hasPrice", "datatype": "xsd:decimal" }
  ]
}
```

- **`lookup: true`** — IRI reference (links to another entity)
- **`datatype`** — typed literal (decimal, integer, date)
- **Neither** — plain string literal

## Ontology Lifecycle

Production ontologies change over time. Open Ontologies provides Terraform-style lifecycle management.

```mermaid
flowchart LR
    Plan["onto_plan"]
    Enforce["onto_enforce"]
    Apply["onto_apply"]
    Monitor["onto_monitor"]
    Drift["onto_drift"]

    Plan -->|"risk score"| Enforce
    Enforce -->|"compliance"| Apply
    Apply -->|"safe / migrate"| Monitor
    Monitor -->|"watchers"| Drift
    Drift -->|"velocity"| Plan
```

**Plan** — Diffs current vs proposed ontology. Reports added/removed classes, blast radius, risk score (`low`/`medium`/`high`). Locked IRIs (`onto_lock`) prevent accidental removal.

**Enforce** — Design pattern checks. Built-in packs: `generic` (orphan classes, missing labels), `boro` (IES4/BORO compliance), `value_partition` (disjointness). Custom SPARQL rules supported.

**Apply** — Two modes: `safe` (clear + reload) or `migrate` (add owl:equivalentClass/Property bridges for consumers).

**Monitor** — SPARQL watchers with threshold alerts. Actions: `notify`, `block_next_apply`, `auto_rollback`, `log`.

**Drift** — Compares versions, detects renames via Jaro-Winkler similarity, computes drift velocity. Self-calibrating confidence via SQLite feedback loop.

**Lineage** — Append-only audit trail of all lifecycle operations.

### Clinical Crosswalks

For healthcare ontologies, three tools bridge clinical coding systems:

- `onto_crosswalk` — Look up mappings between ICD-10 (diagnoses), SNOMED CT (clinical terms), and MeSH (medical literature) from a Parquet-backed crosswalk file
- `onto_enrich` — Insert `skos:exactMatch` triples linking ontology classes to clinical codes
- `onto_validate_clinical` — Check that class labels align with standard clinical terminology

## OWL2-DL Reasoning

Native Rust SHOIQ tableaux reasoner — no JVM required.

| DL Feature | Symbol | OWL Construct |
| ---------- | ------ | ------------- |
| Atomic negation | ¬A | complementOf |
| Conjunction | C ⊓ D | intersectionOf |
| Disjunction | C ⊔ D | unionOf |
| Existential | ∃R.C | someValuesFrom |
| Universal | ∀R.C | allValuesFrom |
| Min cardinality | ≥n R.C | minQualifiedCardinality |
| Max cardinality | ≤n R.C | maxQualifiedCardinality |
| Role hierarchy | R ⊑ S | subPropertyOf |
| Transitive roles | Trans(R) | TransitiveProperty |
| Inverse roles | R⁻ | inverseOf |
| Symmetric roles | Sym(R) | SymmetricProperty |
| Functional | Fun(R) | FunctionalProperty |
| ABox reasoning | a:C | NamedIndividual |

**Agent-based parallel classification:**

1. **Satisfiability Agent** — Tests each class in parallel using rayon
2. **Subsumption Agent** — Pairwise subsumption tests, pruned by told-subsumer closure
3. **Explanation Agent** — Traces clash derivations for unsatisfiable classes
4. **ABox Agent** — Individual consistency and type inference

| Reasoner | Language | JVM | Parallel | SHOIQ |
| -------- | -------- | --- | -------- | ----- |
| **Open Ontologies** | Rust | No | Yes (rayon) | Yes |
| HermiT | Java | Yes | No | Yes |
| Pellet | Java | Yes | No | Yes |

## Benchmarks

### Ontology Generation

#### Pizza Ontology — Manchester Tutorial

The [Manchester Pizza Tutorial](https://www.michaeldebellis.com/post/new-protege-pizza-tutorial) is the most widely used OWL teaching material. Students build a Pizza ontology in Protege over ~4 hours.

**Input:** One sentence — "Build a Pizza ontology following the Manchester tutorial specification."

| Metric | Reference (Protege) | AI-Generated | Coverage |
| ------ | ------------------- | ------------ | -------- |
| Classes | 99 | 95 | **96%** |
| Properties | 8 | 8 | **100%** |
| Toppings | 49 | 49 | **100%** |
| Named Pizzas | 24 | 24 | **100%** |
| Time | ~4 hours | ~5 minutes | |

The 4 missing classes are teaching artifacts (e.g., `UnclosedPizza`) that exist only to demonstrate OWL syntax variants. Files: [`benchmark/`](benchmark/)

#### IES4 Building Domain — BORO/4D

The [IES4 standard](https://github.com/dstl/IES4) is the UK government's Information Exchange Standard for defence/intelligence. Built on BORO methodology and 4D perdurantist modeling.

**Input:** Three context documents — BORO/4D methodology, structural requirements, and a domain brief with 9 competency questions.

| Metric | Value |
| ------ | ----- |
| Compliance checks | **86/86 passed (100%)** |
| Triples | 318 |
| Classes | 36 |
| Properties | 12 |
| Generation | One pass — valid Turtle directly |

Files: [`benchmark/`](benchmark/)

### Ontology Extension — Pizza Menu Mapping

Given the Manchester Pizza OWL (expert-crafted reference) and a 13-row restaurant CSV, map the data into the ontology.

| Metric | Value |
| ------ | ----- |
| Topping coverage vs reference | **94%** (62/66 matched) |
| IRI accuracy (naive) | 5% |
| IRI accuracy (Claude-refined) | **94–100%** |
| Vegetarian classification | **92%** (100% with refined mapping) |

The 4 mismatches are naming convention gaps (e.g., "Anchovy" vs `AnchoviesTopping`). Files: [`benchmark/`](benchmark/)

### Mushroom Classification — OWL Reasoning vs Expert Labels

**Dataset:** UCI Mushroom Dataset — 8,124 specimens classified by mycology experts.

| Metric | Value |
| ------ | ----- |
| Accuracy | **98.33%** |
| Recall (poisonous) | **100%** — zero toxic mushrooms missed |
| False positives | 136 (1.67%) — conservative by design |
| False negatives | **0** |
| Classification rules | 6 OWL axioms |

The reasoner is conservative — it flags safe mushrooms as suspicious before ever classifying a toxic one as edible. Files: [`benchmark/mushroom/`](benchmark/mushroom/)

### Vision Benchmark — Image to Knowledge Graph

**Dataset:** 10 real photographs with manually annotated ground truth. RDF pipeline data extracted directly from TTL files; triple counts from `onto_validate`.

| Metric | Manual | Pure Claude | RDF Pipeline |
| ------ | ------ | ----------- | ------------ |
| Object Recall | 100% | 89% | **95%** |
| Category Recall | 100% | 79% | 32% |
| Total RDF Triples | 0 | 0 | **2,540** |
| skos:altLabel Synonyms | 0 | 0 | **612** |
| SPARQL Queryable | No | No | **Yes** |
| Confidence Scores | No | No | **Yes** |
| Effort per image | ~2 min | ~8 sec | ~8 sec |

Category recall is lower because TTL files use fine-grained categories ("animal body part", "vehicle part") while ground truth uses broad labels ("animal", "vehicle"). The value is in queryability — you can't ask "find all images containing animals near water" with flat text labels.

The benchmark runs the full MCP pipeline: `onto_clear` → `onto_validate` (×10) → `onto_load` (×10) → `onto_stats` → `onto_lint` (×10) → `onto_query` (×6) via the real MCP server using the official MCP Python SDK over JSON-RPC 2.0 stdio. Files: [`benchmark/vision/`](benchmark/vision/)

## Architecture

```mermaid
flowchart TD
    Claude["Claude"]
    MCP["Open Ontologies MCP Server"]
    OntologyService["OntologyService"]
    GraphStore["GraphStore — Oxigraph"]
    StateDb["StateDb — SQLite"]
    Planner["Planner — plan / apply / migrate"]
    DriftDetector["DriftDetector — version comparison"]
    Enforcer["Enforcer — design pattern rules"]
    Monitor["Monitor — SPARQL watchers"]
    Lineage["LineageLog — audit trail"]
    Clinical["ClinicalCrosswalks — Parquet"]

    Claude --> MCP
    MCP --> OntologyService
    MCP --> Planner
    MCP --> DriftDetector
    MCP --> Enforcer
    MCP --> Monitor
    MCP --> Lineage
    MCP --> Clinical
    OntologyService --> GraphStore
    OntologyService --> StateDb
    Planner --> GraphStore
    Planner --> StateDb
    DriftDetector --> StateDb
    Enforcer --> GraphStore
    Enforcer --> StateDb
    Monitor --> GraphStore
    Monitor --> StateDb
    Lineage --> StateDb
```

## Stack

- **Rust** (edition 2024) — single binary, no JVM
- **Oxigraph 0.4** — pure Rust RDF/SPARQL engine
- **rmcp** — MCP protocol implementation
- **SQLite** (rusqlite) — state, versions, lineage, monitor, enforcer rules, drift feedback
- **Apache Arrow/Parquet** — clinical crosswalk file format

**Optional companion:** [OpenCheir](https://github.com/fabio-rovai/opencheir) adds workflow enforcement, audit trails, and multi-agent orchestration.

## License

MIT
