# Open Ontologies

[![CI](https://github.com/fabio-rovai/open-ontologies/actions/workflows/ci.yml/badge.svg)](https://github.com/fabio-rovai/open-ontologies/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Terraform for Knowledge Graphs — validate, classify, and govern AI-generated ontologies.

## Why not just ask Claude directly?

You can ask Claude to generate an ontology in a single prompt — and it will. Claude knows OWL, RDF, BORO, 4D modeling, and every methodology from its training data. No fine-tuning, no plugins.

**But a single-shot generation has real problems:**

| Problem | What goes wrong |
| ------- | --------------- |
| No validation | Claude sometimes generates invalid Turtle — wrong prefixes, unclosed brackets, bad URIs. You won't know until you try to load it. |
| No verification | Did Claude actually produce 49 toppings or did it skip some? Without SPARQL, you're counting by hand. |
| No iteration | You can't diff what changed between versions. You can't lint for missing labels. You can't run competency questions as SPARQL. |
| No persistence | The ontology only lives in the chat context. Close the window, it's gone. No versioning, no rollback. |
| No scale | Claude's context window can hold ~2,000 triples. Real ontologies (IES4: 10,000+ triples) need an actual triple store. |
| No integration | You can't push to a SPARQL endpoint, pull from a remote ontology, or resolve owl:imports chains. |

**Open Ontologies solves every one of these.** It's a proper RDF/SPARQL engine (Oxigraph) exposed as 35 MCP tools that Claude calls automatically. Generate → validate → load → query → iterate → persist — plus a full Terraform-style lifecycle for managing ontology evolution in production.

## The problem: AI generates knowledge, but can't guarantee consistency

LLMs can extract entities, build schemas, and generate ontologies. But they can't guarantee the result is logically consistent, structurally valid, or safe to deploy.

**Open Ontologies is the safety and automation layer.** Like Terraform validates infrastructure before applying it, Open Ontologies validates ontologies before they go live. Plan changes, detect drift, enforce design patterns, monitor health — with a full audit trail.

Position it in your stack:

| Layer | Tool | What it does |
| ----- | ---- | ------------ |
| Generation | Claude / GPT / LLaMA | Generates OWL/RDF from natural language |
| **Validation** | **Open Ontologies** | **Validates, classifies, enforces, monitors** |
| Storage | SPARQL endpoint / triplestore | Persists the production ontology |
| Consumption | Your app / API / pipeline | Queries the knowledge graph |

## Demo: Database → Ontology in 3 commands

```bash
# Import a PostgreSQL schema as OWL
open-ontologies import-schema postgres://demo:demo@localhost/shop

# Classify with native OWL2-DL reasoner
open-ontologies reason --profile owl-dl

# Query the result
open-ontologies query "SELECT ?c ?label WHERE { ?c a owl:Class . ?c rdfs:label ?label }"
```

See [`benchmark/demo/`](benchmark/demo/) for a full Docker-based example.

## Terraforming: Any Data → Ontology → Reasoning

Open Ontologies replaces manual ontology annotation with an automated pipeline. Take any structured data — CSV, JSON, Parquet, XLSX, XML — and terraform it into a validated, reasoned knowledge graph.

```text
Raw Data → map → ingest → reason → query → validated knowledge
```

The pipeline replaces what traditionally takes weeks of manual work:

| Manual process | Open Ontologies equivalent |
| -------------- | ------------------------- |
| Domain expert defines classes by hand | `import-schema` or Claude generates OWL from requirements |
| Analyst maps spreadsheet columns to ontology | `map` auto-generates mapping config from schema |
| Data engineer writes ETL to RDF | `ingest` parses CSV/JSON/Parquet/XLSX → RDF triples |
| Ontologist validates data constraints | `shacl` checks cardinality, datatypes, classes |
| Reasoner classifies instances (Protege + HermiT) | `reason` runs native OWL2-DL classification |
| Quality reviewer checks consistency | `enforce` + `lint` + `monitor` |

### Proof: Mushroom Classification

**Dataset:** UCI Mushroom Dataset — 8,124 mushroom specimens manually classified as edible or poisonous by mycology experts for the Audubon Society Field Guide (1981). This dataset has never been mapped to a formal ontology.

**What the experts did:** Examined each specimen's physical features (odor, gill color, spore print, habitat, etc.) and classified it as edible or poisonous. This manual annotation took years of fieldwork.

**What the pipeline does:**

```bash
# 1. Load domain ontology with classification rules
open-ontologies load mushroom-ontology.ttl

# 2. Ingest 8,124 specimens from CSV
open-ontologies ingest mushrooms.csv --mapping mushroom-mapping.json

# 3. Run OWL reasoning to classify
open-ontologies reason --profile owl-rl

# 4. Query for poisonous mushrooms
open-ontologies query "SELECT ?m WHERE { ?m a mush:PoisonousMushroom }"
```

### Results: OWL reasoning vs expert manual labels

| Metric | Value |
| ------ | ----- |
| Total specimens | 8,124 |
| Accuracy | **98.33%** |
| Recall (poisonous) | **100%** — zero poisonous mushrooms missed |
| False positives | 136 (1.67%) — edible conservatively marked as poisonous |
| False negatives | **0** — never misclassifies a toxic mushroom as safe |
| Classification rules | 6 OWL axioms encoding expert mycological knowledge |

The reasoner is **conservative by design** — it will flag a safe mushroom as suspicious before it ever tells you a poisonous one is edible. The 6 OWL axioms encode the same domain knowledge the field guide experts used: odor type, spore print color, gill characteristics, and habitat.

**Files:** [`benchmark/mushroom/`](benchmark/mushroom/)

### Proof: Image → Knowledge Graph (Vision Benchmark)

**Dataset:** 10 real photographs (landscapes, animals, vehicles, architecture) with manually annotated ground truth labels.

**The question:** Can you convert unstructured images into a queryable knowledge graph? We compared three approaches:

1. **Manual annotation** — a human labels objects and categories (~2 min/image)
2. **Pure Claude** — Claude vision returns flat JSON text labels (~8 sec/image)
3. **RDF Pipeline** — Claude vision generates structured Turtle with `rdfs:label`, `skos:altLabel` synonyms, `schema:category`, confidence scores, and spatial relationships (~8 sec/image)

**Method:** 20 parallel Claude agents processed 10 images simultaneously — 10 for pure text labeling, 10 for structured Turtle generation. Ground truth was manually annotated before the agents ran.

| Metric | Manual | Pure Claude | RDF Pipeline |
| ------ | ------ | ----------- | ------------ |
| Object Recall | 100% | 89% | **95%** |
| Category Recall | 100% | 79% | 32% |
| Total RDF Triples | 0 | 0 | **2,540** |
| skos:altLabel Synonyms | 0 | 0 | **612** |
| SPARQL Queryable | No | No | **Yes** |
| Confidence Scores | No | No | **Yes** |
| Effort per image | ~2 min | ~8 sec | ~8 sec |
| Scales to 1000+ images | No | Yes | Yes |

**Object recall:** Both approaches detect most ground truth objects. The RDF pipeline uses `skos:altLabel` synonyms (612 total) to expand matching surface, achieving 95% recall. Category recall is lower (32%) because the TTL files use fine-grained categories ("animal body part", "vehicle part") while ground truth uses broad labels ("animal", "vehicle").

**Why this matters:** Pure Claude gives you flat text — you can't query "find all images containing animals near water." The RDF pipeline produces a queryable knowledge graph with 2,540 triples, 204 spatial relationships, and 612 synonyms across 10 images.

**Full MCP pipeline:** The benchmark runs the complete `onto_clear` → `onto_validate` (×10) → `onto_load` (×10) → `onto_stats` → `onto_lint` (×10) → `onto_query` (×6) tool chain via the real MCP server (`open-ontologies serve`), using the official MCP Python SDK over JSON-RPC 2.0 stdio. All 10 images validated, loaded into Oxigraph, linted clean, and queried with SPARQL — the same protocol Claude uses.

**Files:** [`benchmark/vision/`](benchmark/vision/)

### Supported input formats

| Format | Extension | Notes |
| ------ | --------- | ----- |
| CSV | `.csv` | Comma/tab delimited |
| JSON | `.json` | Array of objects |
| NDJSON | `.ndjson` | Newline-delimited JSON |
| XML | `.xml` | Record-oriented XML |
| YAML | `.yaml` | Document or array |
| Excel | `.xlsx` | First sheet |
| Parquet | `.parquet` | Columnar, recommended for large datasets |

For any format: `open-ontologies ingest data.parquet --mapping mapping.json`

## What is it?

Open Ontologies is a standalone MCP server for AI-native ontology engineering. It exposes 35 tools that let Claude validate, query, diff, lint, version, and persist RDF/OWL ontologies using an in-memory Oxigraph triple store — plus plan changes, detect drift, enforce design patterns, monitor health, manage clinical crosswalks, and track lineage.

Written in Rust, ships as a single binary. No JVM, no Protege, no GUI.

**Optional companion:** [OpenCheir](https://github.com/fabio-rovai/opencheir) adds workflow enforcement, audit trails, and multi-agent orchestration. Its enforcer rules can govern ontology workflows (e.g., warn if saving without validating). But Open Ontologies works perfectly on its own.

## How it works

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

This is not a fixed pipeline. The MCP server exposes 35 ontology tools — **Claude is the orchestrator** that decides what to call next based on results. If `onto_validate` fails, Claude fixes the Turtle and retries. If `onto_stats` shows wrong counts, Claude regenerates. If `onto_lint` finds missing labels, Claude adds them.

No Protege. No GUI. No manual class creation. Claude is the ontology engineer, Open Ontologies is the runtime.

### Extending ontologies with data

An ontology without data is a schema without rows. Open Ontologies lets you feed structured datasets — CSV, JSON, XML, YAML, XLSX, Parquet — into a loaded ontology. The data becomes RDF instances (ABox) governed by the ontology's classes and properties (TBox).

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

**How it works:**

1. **Map** — `onto_map` reads your data file's column headers and the loaded ontology's classes/properties. It generates a JSON mapping config that links each field to an RDF predicate. Claude reviews and refines the mapping — e.g., linking a `base` column to `pizza:hasBase` as a lookup (IRI reference) rather than a string literal.

2. **Ingest** — `onto_ingest` parses the data file, applies the mapping, converts each row to N-Triples, and loads them into the Oxigraph store. A 13-row CSV produces ~100 triples (one `rdf:type` + one per field per row).

3. **Validate** — `onto_shacl` checks the loaded data against SHACL shapes: does every pizza have at least one topping? Exactly one base? A label? Violations are reported as JSON so Claude can fix the mapping and re-ingest.

4. **Reason** — `onto_reason` runs inference over the ontology. Four profiles available:
   - `rdfs` — subclass, domain/range, subproperty closure
   - `owl-rl` — adds transitive/symmetric/inverse, sameAs, equivalentClass
   - `owl-rl-ext` — adds someValuesFrom, allValuesFrom, hasValue, intersectionOf, unionOf
   - `owl-dl` — native SHOIQ tableaux reasoner: satisfiability testing, parallel agent-based classification, qualified number restrictions with node merging, inverse/symmetric roles, functional properties, complement/disjunction with backtracking, ABox consistency checking, explanation traces, unsatisfiable class detection

5. **Query** — `onto_query` runs SPARQL against the combined ontology + data + inferred triples. You can now ask "which pizzas are vegetarian?" and get answers derived from the ontology's class hierarchy, not from a hardcoded column.

**The mapping config** is the key bridge between tabular data and RDF:

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

- **`lookup: true`** — the value is an IRI reference (links to another entity), not a string literal
- **`datatype`** — typed literal (decimal, integer, date, etc.)
- **Neither** — plain string literal

You can also skip the step-by-step and use `onto_extend` to run the full pipeline (ingest → SHACL → reason) in one call.

The workflow is codified in two places so Claude follows it consistently:

- **[`CLAUDE.md`](CLAUDE.md)** — loaded automatically when you open Claude Code in this repo. Describes the generate → validate → verify → iterate → persist workflow.
- **[`/ontology-engineer` skill](skills/ontology-engineer.md)** — portable skill you can use from any project. Invoke it with `/ontology-engineer` in Claude Code.

### Architecture

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

## Tools

| Tool | Purpose |
| ---- | ------- |
| `onto_validate` | Validate RDF/OWL syntax (file or inline Turtle) |
| `onto_convert` | Convert between formats (Turtle, N-Triples, RDF/XML, N-Quads, TriG) |
| `onto_load` | Load RDF into in-memory graph store |
| `onto_query` | Run SPARQL queries against loaded ontology |
| `onto_save` | Save ontology store to file |
| `onto_stats` | Triple count, classes, properties, individuals |
| `onto_diff` | Compare two ontology files (added/removed triples) |
| `onto_lint` | Check for missing labels, comments, domains |
| `onto_clear` | Clear in-memory store |
| `onto_pull` | Fetch ontology from remote URL or SPARQL endpoint |
| `onto_push` | Push ontology to a SPARQL endpoint |
| `onto_import` | Resolve and load owl:imports chain |
| `onto_version` | Save a named snapshot of the current store |
| `onto_history` | List saved version snapshots |
| `onto_rollback` | Restore a previous version |
| `onto_status` | Server health and loaded triple count |
| `onto_ingest` | Parse structured data (CSV/JSON/XML/YAML/XLSX/Parquet) into RDF |
| `onto_map` | Generate mapping config from data schema + ontology |
| `onto_shacl` | Validate data against SHACL shapes |
| `onto_reason` | Run inference: rdfs, owl-rl, owl-rl-ext, or owl-dl (native SHOIQ tableaux with parallel agent classification) |
| `onto_extend` | Full pipeline: ingest → validate → reason |
| **Lifecycle** | |
| `onto_plan` | Terraform-style plan — diff current vs proposed, show added/removed classes/properties, blast radius, risk score |
| `onto_apply` | Apply a planned change: `safe` (clear + reload) or `migrate` (add owl:equivalentClass/Property bridges) |
| `onto_lock` | Lock an IRI to prevent removal (e.g., production classes) |
| `onto_drift` | Detect drift between two ontology versions — renames, velocity, self-calibrating confidence |
| `onto_enforce` | Run design pattern enforcement: `generic`, `boro`, `value_partition`, or custom SPARQL rules |
| `onto_monitor` | Run active SPARQL watchers with threshold comparison — notify, block, or auto-rollback |
| `onto_monitor_clear` | Clear monitor block state after resolving alerts |
| `onto_crosswalk` | Look up clinical terminology crosswalks (ICD-10 ↔ SNOMED ↔ MeSH) from Parquet file |
| `onto_enrich` | Add skos:exactMatch triples linking ontology classes to clinical codes |
| `onto_validate_clinical` | Validate ontology class labels against clinical crosswalk terminology |
| `onto_lineage` | View compressed lineage trail for the current session (plan → enforce → apply → monitor → drift) |

## CLI

Open Ontologies ships as a single binary with subcommands mirroring every MCP tool. Use it standalone or as an MCP server.

```bash
open-ontologies <command> [args] [--pretty] [--data-dir ~/.open-ontologies]
```

| Category | Commands |
| -------- | -------- |
| Core | `validate`, `load`, `save`, `clear`, `stats`, `query`, `diff`, `lint`, `convert`, `status` |
| Remote | `pull`, `push`, `import-owl` |
| Schema | `import-schema` (PostgreSQL → OWL) |
| Data | `map`, `ingest`, `shacl`, `reason`, `extend` |
| Versioning | `version`, `history`, `rollback` |
| Lifecycle | `plan`, `apply`, `lock`, `drift`, `enforce`, `monitor`, `monitor-clear`, `lineage` |
| Clinical | `crosswalk`, `enrich`, `validate-clinical` |
| Server | `init`, `serve` (MCP server mode) |

Run `open-ontologies --help` for full usage.

## Benchmarks

### Reasoner Performance

Open Ontologies includes a native Rust OWL2-DL reasoner (SHOIQ tableaux). Benchmark it against Java reasoners:

| Reasoner | Language | JVM required | Parallel | SHOIQ |
| -------- | -------- | ------------ | -------- | ----- |
| **Open Ontologies** | Rust | No | Yes (rayon) | Yes |
| HermiT | Java | Yes | No | Yes |
| Pellet | Java | Yes | No | Yes |

Run the benchmarks yourself:

```bash
# Correctness: compare subsumptions against HermiT/Pellet on Pizza ontology
cd benchmark/reasoner && bash run_pizza_correctness.sh

# Performance: LUBM-style ontologies at 1K–50K axioms
cd benchmark/reasoner && bash run_lubm_performance.sh
```

See [`benchmark/reasoner/README.md`](benchmark/reasoner/README.md) for Java setup instructions.

### Ontology Generation & Extension

Open Ontologies covers two distinct capabilities, each benchmarked separately:

- **Ontology generation** — building an ontology schema (TBox) from requirements
- **Ontology extension** — mapping real-world data (ABox) into an existing ontology

### Ontology Generation

#### Pizza Ontology — Manchester University Tutorial

**Source:** The [Manchester Pizza Tutorial](https://www.michaeldebellis.com/post/new-protege-pizza-tutorial) is the most widely used OWL teaching material. Students build a Pizza ontology step-by-step in Protege over ~4 hours. The reference OWL file is [published on GitHub](https://github.com/owlcs/pizza-ontology).

**What the tutorial teaches (traditional approach):**

| Step | What you do in Protege | Time |
| ---- | ---------------------- | ---- |
| 1 | Create blank ontology, set IRI | 5 min |
| 2 | Add `Pizza`, `PizzaTopping`, `PizzaBase` classes via GUI | 10 min |
| 3 | Create `hasTopping`, `hasBase` object properties, set domains/ranges | 15 min |
| 4 | Add 49 topping subclasses (`MozzarellaTopping`, `HamTopping`, ...) one by one | 30 min |
| 5 | Add `hasSpiciness` property, create `Spiciness` value partition (`Hot`/`Medium`/`Mild`) | 15 min |
| 6 | Add spiciness restrictions to each topping class individually | 20 min |
| 7 | Make all sibling classes disjoint (click "Make siblings disjoint" per group) | 10 min |
| 8 | Create 24 named pizzas (`Margherita`, `American`, ...) with `someValuesFrom` restrictions | 45 min |
| 9 | Add closure axioms (`allValuesFrom`) to each named pizza | 30 min |
| 10 | Define `VegetarianPizza`, `MeatyPizza`, `SpicyPizza` as equivalent classes | 20 min |
| 11 | Run reasoner, debug, iterate | 20 min |

**Result:** 99 classes, 8 properties, 2,332 triples.

**What we did (AI-native approach):**

**Input to Claude:** One sentence — "Build a Pizza ontology following the Manchester tutorial specification." No custom prompts, no background documents, no examples. Claude knows the Pizza tutorial from its training data.

| Step | What you tell Claude | Tool used | Time |
| ---- | -------------------- | --------- | ---- |
| 1 | "Build a Pizza ontology following the Manchester tutorial spec" | Claude generates Turtle | 2 min |
| 2 | "Validate it" | `onto_validate` | 5 sec |
| 3 | "Load and check stats" | `onto_load` → `onto_stats` | 5 sec |
| 4 | "Lint it" | `onto_lint` | 5 sec |
| 5 | "Diff against the reference" | `onto_diff` | 5 sec |

**Result:** 95 classes, 8 properties, 1,168 triples. ~5 minutes total.

**Comparison:**

| Metric | Reference | AI-Generated | Coverage |
| ------ | --------- | ------------ | -------- |
| Classes | 99 | 95 | **96%** |
| Properties | 8 | 8 | **100%** |
| Toppings | 49 | 49 | **100%** |
| Named Pizzas | 24 | 24 | **100%** |
| Total triples | 2,332 | 1,168 | 50% size |

The 4 missing classes (`UnclosedPizza`, `SpicyPizzaEquivalent`, `VegetarianPizzaEquivalent1`, `VegetarianPizzaEquivalent2`) are teaching artifacts — they exist only to demonstrate OWL syntax variants in the tutorial, not actual domain concepts.

The AI produces 50% fewer triples because it uses compact `owl:AllDisjointClasses` declarations instead of exhaustive pairwise `owl:disjointWith` axioms (398 pairs in reference vs 101 in AI) — same semantics, fewer triples.

**Files:**

- Reference: [`benchmark/reference/pizza-reference.owl`](benchmark/reference/pizza-reference.owl) — the original Manchester OWL file (6,858 lines)
- AI-generated: [`benchmark/generated/pizza-ai.ttl`](benchmark/generated/pizza-ai.ttl) — Claude's Turtle output
- Comparison script: [`benchmark/pizza_compare.py`](benchmark/pizza_compare.py)
- Full comparison: [`benchmark/PIZZA_COMPARISON.md`](benchmark/PIZZA_COMPARISON.md)

#### IES4 Building Domain — BORO/4D Methodology

**Source:** The [IES4 standard](https://github.com/dstl/IES4) is the UK government's Information Exchange Standard, built on BORO (Business Objects Reference Ontology) and 4D perdurantist modeling. It's a real-world upper ontology used in defence and intelligence.

**What an ontology engineer does (traditional approach):**

| Step | What you do | Time |
| ---- | ----------- | ---- |
| 1 | Read the IES4 spec (200+ pages), understand BORO/4D patterns | 2-3 days |
| 2 | Import IES4 upper ontology into Protege | 30 min |
| 3 | Create Entity+State pairs for each domain concept | 2-3 hours |
| 4 | Add BoundingStates, ClassOf hierarchies | 1-2 hours |
| 5 | Define properties linking states to entities | 1 hour |
| 6 | Write SHACL shapes for validation | 2-3 hours |
| 7 | Run validation against IES4 SHACL shapes | 30 min |
| 8 | Debug and iterate until compliant | 1-2 days |

**What we did (AI-native approach):**

**Input to Claude:** Three documents providing context (all included in [`benchmark/reference/`](benchmark/reference/)):

- [`background_prompt.txt`](benchmark/reference/background_prompt.txt) — explains BORO/4D methodology, perdurantism, mereotopology
- [`instructions.txt`](benchmark/reference/instructions.txt) — structural requirements: 4D Entity+State pattern, ClassOf hierarchies, property conventions
- [`custom_instructions.txt`](benchmark/reference/custom_instructions.txt) — the actual domain brief: UK building/energy performance, 9 competency questions

Claude reads these, then generates the complete Turtle file in one pass. Tools validate and verify.

**Result:**

- **100% compliance** — 86/86 checks passed
- 318 triples, 36 classes, 12 properties
- Full 4D/BORO patterns: Entity+State pairs, BoundingStates, ClassOf
- All 9 competency questions covered
- One-pass generation — Claude produced valid Turtle directly, tools validated afterward

**Files:**

- Reference upper ontology: [`benchmark/reference/ies4.ttl`](benchmark/reference/ies4.ttl) — the full IES4 standard (249K)
- Reference SHACL shapes: [`benchmark/reference/ies4.shacl`](benchmark/reference/ies4.shacl) — validation rules (106K)
- Reference instructions: [`benchmark/reference/instructions.txt`](benchmark/reference/instructions.txt) — the domain brief given to Claude
- AI-generated extension: [`benchmark/generated/ies-building-extension.ttl`](benchmark/generated/ies-building-extension.ttl) — Claude's output
- BORO handcrafted: [`benchmark/reference/boro-building-handcrafted.ttl`](benchmark/reference/boro-building-handcrafted.ttl) — traditional BORO for comparison
- BORO AI-generated: [`benchmark/generated/boro-building-ai.ttl`](benchmark/generated/boro-building-ai.ttl) — Claude's BORO version
- Compliance script: [`benchmark/compare.py`](benchmark/compare.py) — runs 86 checks
- BORO comparison script: [`benchmark/boro_compare.py`](benchmark/boro_compare.py)
- Full comparison: [`benchmark/BORO_COMPARISON.md`](benchmark/BORO_COMPARISON.md)

### Ontology Extension

Ontology extension is a different task from ontology generation. Given an **existing** ontology (the rules/schema) and a **new dataset** (CSV, JSON, etc.), the goal is to map the data into the ontology's structure — creating RDF instances (ABox) that conform to the ontology's classes and properties (TBox). The ontology then enables reasoning over the data: inferring facts that aren't explicitly stated.

#### Pizza Extension — Mapping Data to the Manchester Ontology

**What this tests:** The Manchester Pizza OWL was built by domain experts in Protege over 20+ years. It defines 22 named pizzas with exact OWL restrictions specifying their toppings — e.g., `Margherita rdfs:subClassOf (hasTopping some MozzarellaTopping)`. This IS the handcrafted reference.

The AI pipeline takes a restaurant menu CSV and maps it to the same ontology using `onto_map` + `onto_ingest`. How closely does the auto-mapped data match what the experts defined?

**Input data:** A 13-row CSV ([`benchmark/data/pizza-menu.csv`](benchmark/data/pizza-menu.csv)) with topping names from the Manchester definitions:

```csv
name,topping1,topping2,topping3,topping4,topping5,topping6,topping7
Margherita,Mozzarella,Tomato,,,,,
American,Mozzarella,Tomato,PeperoniSausage,,,,
Fiorentina,Mozzarella,Tomato,Spinach,Olive,Parmesan,Garlic,
SloppyGiuseppe,Mozzarella,Tomato,GreenPepper,Onion,HotSpicedBeef,,
...
```

**Topping coverage — AI pipeline vs Manchester reference:**

| Pizza | Ref toppings | Matched | Missing | Coverage |
| ----- | ------------ | ------- | ------- | -------- |
| Margherita | 2 | 2 | 0 | 100% |
| American | 3 | 3 | 0 | 100% |
| AmericanHot | 5 | 5 | 0 | 100% |
| Capricciosa | 7 | 6 | 1 | 86% |
| Fiorentina | 6 | 6 | 0 | 100% |
| FourSeasons | 7 | 6 | 1 | 86% |
| LaReine | 5 | 5 | 0 | 100% |
| Mushroom | 3 | 3 | 0 | 100% |
| Napoletana | 5 | 4 | 1 | 80% |
| Parmense | 5 | 5 | 0 | 100% |
| SloppyGiuseppe | 5 | 5 | 0 | 100% |
| Soho | 6 | 6 | 0 | 100% |
| Veneziana | 7 | 6 | 1 | 86% |
| **Total** | **66** | **62** | **4** | **94%** |

The 4 mismatches are naming convention gaps — the CSV says "Anchovy" but the ontology class is `AnchoviesTopping` (pluralised). Claude resolves these during mapping review.

**IRI mapping accuracy — the key challenge:**

The CSV contains short names (`Mozzarella`, `Anchovy`). The ontology uses full class names (`MozzarellaTopping`, `AnchoviesTopping`). How well does the pipeline bridge this gap?

| Mapping approach | IRI accuracy |
| ---------------- | ------------ |
| Naive auto-mapping (raw CSV values) | **5%** (3/66) |
| With `Topping` suffix convention | **94%** (62/66) |
| After Claude resolves naming variants | **100%** (66/66) |

The naive auto-mapper produces `pizza:Mozzarella` — which doesn't exist as an ontology class. Adding the `Topping` suffix convention (which Claude applies when reviewing the mapping) fixes 94%. The remaining 2 naming variants (`Anchovy` → `AnchoviesTopping`, `PineKernel` → `PineKernels`) require Claude's domain knowledge.

**Reasoning accuracy — vegetarian classification:**

After ingesting the CSV and running RDFS reasoning (`onto_reason`), the reasoner classifies pizzas as vegetarian by traversing the `rdfs:subClassOf` hierarchy: `PeperoniSausageTopping` → `MeatTopping`, `AnchoviesTopping` → `FishTopping`, etc. Ground truth comes from the Manchester reference itself.

| Pizza | Reference | Inferred | Match |
| ----- | --------- | -------- | ----- |
| American | Non-veg | Non-veg | YES |
| AmericanHot | Non-veg | Non-veg | YES |
| Capricciosa | Non-veg | Non-veg | YES |
| Fiorentina | Vegetarian | Vegetarian | YES |
| FourSeasons | Non-veg | Non-veg | YES |
| LaReine | Non-veg | Non-veg | YES |
| Margherita | Vegetarian | Vegetarian | YES |
| Mushroom | Vegetarian | Vegetarian | YES |
| Napoletana | Non-veg | Vegetarian | NO |
| Parmense | Non-veg | Non-veg | YES |
| SloppyGiuseppe | Non-veg | Non-veg | YES |
| Soho | Vegetarian | Vegetarian | YES |
| Veneziana | Vegetarian | Vegetarian | YES |

Accuracy: 12/13 (92%). The one miss (Napoletana) is caused by the `Anchovy` → `AnchoviesTopping` naming gap — the IRI `pizza:AnchovyTopping` doesn't exist in the fish class hierarchy, so the reasoner can't classify it as non-vegetarian. With Claude-refined mapping this becomes 100%.

**Summary:**

| Metric | Handcrafted (Manchester) | AI Pipeline |
| ------ | ------------------------ | ----------- |
| Source | Protege GUI, 20+ years | CSV + `onto_ingest`, 30 sec |
| Named pizzas | 22 | 13 |
| Avg toppings/pizza | 5.1 | 5.1 |
| Topping coverage | 100% (IS reference) | **94%** |
| IRI accuracy (naive) | 100% | 5% |
| IRI accuracy (Claude-refined) | 100% | **94–100%** |
| Reasoning accuracy | 100% | **92%** (100% with refined mapping) |

**Files:**

- Menu CSV: [`benchmark/data/pizza-menu.csv`](benchmark/data/pizza-menu.csv) — 13 pizzas with toppings from the Manchester reference
- Mapping config: [`benchmark/data/pizza-mapping.json`](benchmark/data/pizza-mapping.json)
- SHACL shapes: [`benchmark/data/pizza-shapes.ttl`](benchmark/data/pizza-shapes.ttl)
- Extension comparison: [`benchmark/extension_compare.py`](benchmark/extension_compare.py) — topping coverage + IRI accuracy
- Reasoning benchmark: [`benchmark/pizza_extend_compare.py`](benchmark/pizza_extend_compare.py) — vegetarian classification

### OWL2-DL Reasoning

#### Native SHOIQ Tableaux Engine

Open Ontologies includes a native Rust implementation of a tableaux decision procedure for the SHOIQ Description Logic — the logical foundation of OWL2-DL.

**Why this matters:** Most OWL reasoners (HermiT, Pellet, FaCT++) are Java-based and require a JVM. ELK is fast but limited to EL++. There is no production-quality native DL reasoner in Rust — until now.

**Description Logic coverage:**

| DL Feature | Symbol | OWL Construct | Status |
| --------- | ------ | ------------- | ------ |
| Atomic negation | ¬A | complementOf | ✅ |
| Conjunction | C ⊓ D | intersectionOf | ✅ |
| Disjunction | C ⊔ D | unionOf | ✅ |
| Existential | ∃R.C | someValuesFrom | ✅ |
| Universal | ∀R.C | allValuesFrom | ✅ |
| Min cardinality | ≥n R.C | minQualifiedCardinality | ✅ |
| Max cardinality | ≤n R.C | maxQualifiedCardinality | ✅ |
| Role hierarchy | R ⊑ S | subPropertyOf | ✅ |
| Transitive roles | Trans(R) | TransitiveProperty | ✅ |
| Inverse roles | R⁻ | inverseOf | ✅ |
| Symmetric roles | Sym(R) | SymmetricProperty | ✅ |
| Functional | Fun(R) | FunctionalProperty | ✅ |
| ABox reasoning | a:C | NamedIndividual | ✅ |

**Architecture — Agent-based parallel classification:**

The reasoner uses an agent-based decomposition pattern for classification:

1. **Satisfiability Agent** — Tests each named class for satisfiability in parallel using rayon worker threads
2. **Subsumption Agent** — Tests pairwise subsumptions in parallel, pruned by told-subsumer transitive closure
3. **Explanation Agent** — Traces clash derivations to explain why classes are unsatisfiable
4. **ABox Agent** — Checks individual consistency and infers additional types

Each agent operates independently, enabling the O(n²) classification to scale across all available CPU cores.

**Key implementation details:**

- **Node merging for MaxCard (≤n R.C)** — When a node has more role-successors than allowed, the tableau non-deterministically merges successor nodes, combining their labels and edges. This is the "complex route" that distinguishes full SHOIQ from simpler clash-based cardinality handling.
- **Inverse role propagation** — When a node is created via ∃R.C, the parent is accessible as an R⁻-successor. Universal restrictions ∀R⁻.C propagate backward through these inverse edges.
- **Subset blocking** — Ensures termination: a node is blocked when an ancestor has a superset of its labels.
- **Dependency-aware backtracking** — Disjunction branches (⊔-rule) and MaxCard merges (≤-rule) use full tableau cloning for correct backtracking.
- **String interning** — All IRIs compressed to u32 for cache-friendly set operations.

**MCP tools for agent orchestration:**

| Tool | Purpose |
| ---- | ------- |
| `onto_reason` (profile: `owl-dl`) | Full classification with all agents |
| `onto_dl_explain` | Explain why a specific class is unsatisfiable |
| `onto_dl_check` | Check if class A ⊑ class B with justification trace |

The JSON output includes agent metrics showing task distribution and timing:

```json
{
  "agents": {
    "satisfiability_agent": {"classes_checked": 15, "satisfiable_found": 14, "time_ms": 2},
    "subsumption_agent": {"pairs_tested": 182, "subsumptions_found": 8, "time_ms": 5},
    "parallel_workers": 8,
    "total_time_ms": 7
  }
}
```

### Ontology Lifecycle

Production ontologies change over time — classes are added, properties renamed, hierarchies refactored. Open Ontologies provides a Terraform-style lifecycle to manage these changes safely.

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

**Plan** — `onto_plan` diffs the current ontology against a proposed Turtle file. It reports added/removed classes and properties, counts affected triples (blast radius), and assigns a risk score: `low` (additions only), `medium` (modifications), `high` (removals with dependents). IRIs can be locked (`onto_lock`) to prevent accidental removal of production classes.

**Enforce** — `onto_enforce` runs design pattern checks against the loaded ontology. Three built-in packs:

- `generic` — orphan classes, missing domains/ranges/labels
- `boro` — IES4/BORO compliance: entities must have State subclasses
- `value_partition` — sibling classes must be declared disjoint

Custom rules can be added as ASK/SELECT SPARQL queries.

**Apply** — `onto_apply` executes the planned changes. Two modes:

- `safe` — clears the store and reloads with the new version
- `migrate` — adds `owl:equivalentClass` and `owl:equivalentProperty` bridge triples so consumers can follow renames

**Monitor** — `onto_monitor` runs configurable SPARQL watchers with threshold comparison. Each watcher has an action: `notify`, `block_next_apply`, `auto_rollback`, or `log`. Blocked state prevents `onto_apply` until cleared with `onto_monitor_clear`.

**Drift** — `onto_drift` compares two ontology versions and computes drift velocity (proportion of vocabulary that changed). It detects renames using Jaro-Winkler string similarity, domain/range matching, and hierarchy position. Confidence scores self-calibrate via a SQLite-backed feedback loop — record correct/incorrect rename detections with `onto_drift` feedback, and the weights adjust over time.

**Lineage** — Every lifecycle operation is recorded to an append-only lineage log. `onto_lineage` returns a compressed event trail: `session:seq:timestamp:type:operation:details`. This provides a complete audit trail of who changed what and when.

**Clinical crosswalks** — `onto_crosswalk` looks up ICD-10, SNOMED CT, and MeSH mappings from a Parquet-backed crosswalk file. `onto_enrich` inserts `skos:exactMatch` triples linking ontology classes to clinical codes. `onto_validate_clinical` checks that class labels align with standard clinical terminology.

## Replicate it yourself

### 1. Build Open Ontologies

You need Rust 1.85+ (edition 2024).

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

Restart Claude Code. You should see the `onto_*` tools available.

### 3. (Optional) Add OpenCheir for governance

For workflow enforcement, audit trails, and multi-agent orchestration:

```json
{
  "mcpServers": {
    "open-ontologies": {
      "command": "/path/to/open-ontologies/target/release/open-ontologies",
      "args": ["serve"]
    },
    "opencheir": {
      "command": "/path/to/opencheir/target/release/opencheir",
      "args": ["serve"]
    }
  }
}
```

### 4. Build your first ontology

Open Claude Code and type:

```text
Build me a Pizza ontology following the Manchester University tutorial.
Include all 49 toppings, 22 named pizzas, spiciness value partition,
and defined classes (VegetarianPizza, MeatyPizza, SpicyPizza).
Validate it, load it, and show me the stats.
```

### 5. Try with your own domain

For a simple ontology (like Pizza), one sentence is enough. For complex methodologies (like BORO/4D), give Claude context:

```text
I need a building domain ontology following IES4/BORO patterns.
Here are the requirements: [paste your competency questions]
Use 4D Entity+State pairs, ClassOf hierarchies, and alphabetical ordering.
Validate it and run compliance checks.
```

### 6. Run the benchmarks

```bash
cd benchmark
pip install rdflib
python3 pizza_compare.py          # Pizza: 96% class coverage, 100% properties
python3 extension_compare.py       # Extension: 94% topping coverage vs Manchester
python3 pizza_extend_compare.py   # Reasoning: 92% vegetarian classification
python3 compare.py                # IES4: 86/86 compliance checks passed
```

## Stack

- **Rust** (edition 2024) — single binary, no JVM
- **Oxigraph 0.4** — pure Rust RDF/SPARQL engine
- **rmcp** — MCP protocol implementation
- **SQLite** (rusqlite) — state, versions, lineage, monitor, enforcer rules, drift feedback
- **Apache Arrow/Parquet** — clinical crosswalk file format

## License

MIT
