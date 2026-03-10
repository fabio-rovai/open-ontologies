# Open Ontologies

AI-native ontology engine — build production ontologies in minutes instead of months.

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

**Open Ontologies solves every one of these.** It's a proper RDF/SPARQL engine (Oxigraph) exposed as 21 MCP tools that Claude calls automatically. Generate → validate → load → query → iterate → persist.

## What is it?

Open Ontologies is a standalone MCP server for AI-native ontology engineering. It exposes 21 tools that let Claude validate, query, diff, lint, version, and persist RDF/OWL ontologies using an in-memory Oxigraph triple store.

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

This is not a fixed pipeline. The MCP server exposes 21 ontology tools — **Claude is the orchestrator** that decides what to call next based on results. If `onto_validate` fails, Claude fixes the Turtle and retries. If `onto_stats` shows wrong counts, Claude regenerates. If `onto_lint` finds missing labels, Claude adds them.

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

4. **Reason** — `onto_reason` runs RDFS or OWL-RL inference. Subclass hierarchies propagate — if `PepperoniSausageTopping` is a `MeatTopping`, the reasoner materialises that relationship. This enables downstream classification: a pizza with only non-meat, non-fish toppings is vegetarian.

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

    Claude --> MCP
    MCP --> OntologyService
    OntologyService --> GraphStore
    OntologyService --> StateDb
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
| `onto_reason` | Run RDFS/OWL-RL inference (materialize triples) |
| `onto_extend` | Full pipeline: ingest → validate → reason |

## Benchmarks

### Pizza Ontology — Manchester University Tutorial

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

### Pizza Extension — CSV Data to Inferred Knowledge

**What this tests:** Can the extension pipeline correctly ingest tabular data into an ontology and use reasoning to infer facts that aren't in the original data?

**Setup:** The AI-generated Pizza ontology (1,168 triples, 95 classes) defines a class hierarchy where toppings are categorised — `PepperoniSausageTopping` is a `MeatTopping`, `MozzarellaTopping` is a `CheeseTopping`, `AnchovyTopping` is a `FishTopping`, etc. A pizza is vegetarian if it has no meat or fish toppings.

**Input data:** A 13-row CSV of restaurant menu items ([`benchmark/data/pizza-menu.csv`](benchmark/data/pizza-menu.csv)):

```csv
name,base,topping1,topping2,topping3,topping4,price,vegetarian
Margherita,ThinAndCrispy,Mozzarella,Tomato,,,8.99,true
American,DeepPan,Mozzarella,Tomato,Pepperoni,Sausage,12.99,false
Fiorentina,ThinAndCrispy,Mozzarella,Tomato,Spinach,Olive,11.49,true
Napoletana,ThinAndCrispy,Mozzarella,Tomato,Anchovy,Caper,11.99,false
PolloAdArrabbiata,ThinAndCrispy,Mozzarella,Tomato,Chicken,Chili,13.99,false
...
```

The `vegetarian` column is the **ground truth** — we strip it from the mapping and see if the reasoner can infer it from the ontology's topping hierarchy.

**What the pipeline does:**

| Step | Tool | What happens |
| ---- | ---- | ------------ |
| 1 | `onto_load` | Load Pizza ontology (TBox: 1,168 triples) |
| 2 | `onto_map` | Inspect CSV headers against ontology, generate mapping config |
| 3 | `onto_ingest` | Parse 13 rows, apply mapping, load 101 ABox triples |
| 4 | `onto_shacl` | Validate: every pizza has toppings, a base, a label → 0 violations |
| 5 | `onto_reason` | Run RDFS inference: 605 new triples in 4 iterations |
| 6 | `onto_query` | SPARQL: "which pizzas have only non-meat, non-fish toppings?" |

**Results — vegetarian classification accuracy:**

| Pizza | Ground Truth | Inferred | Match |
| ----- | ------------ | -------- | ----- |
| Margherita | Vegetarian | Vegetarian | YES |
| American | Non-veg | Non-veg | YES |
| AmericanHot | Non-veg | Non-veg | YES |
| Fiorentina | Vegetarian | Vegetarian | YES |
| Napoletana | Non-veg | Non-veg | YES |
| Parmense | Non-veg | Non-veg | YES |
| LaReine | Non-veg | Non-veg | YES |
| Capricciosa | Non-veg | Non-veg | YES |
| Mushroom | Vegetarian | Vegetarian | YES |
| FourSeasons | Non-veg | Non-veg | YES |
| Soho | Vegetarian | Vegetarian | YES |
| SloppyGiuseppe | Vegetarian | Vegetarian | YES |
| PolloAdArrabbiata | Non-veg | Non-veg | YES |

**Accuracy: 13/13 (100%)**

The reasoner correctly classifies all 13 pizzas by traversing the ontology's `rdfs:subClassOf` hierarchy — Pepperoni → MeatTopping, Anchovy → FishTopping, Mushroom → VegetableTopping — without any hardcoded rules about what "vegetarian" means.

| Metric | Value |
| ------ | ----- |
| TBox (ontology) | 1,168 triples |
| ABox (data) | 101 triples |
| Inferred (reasoning) | 605 triples |
| SHACL violations | 0 |
| Classification accuracy | **100%** |

**Files:**

- Menu data: [`benchmark/data/pizza-menu.csv`](benchmark/data/pizza-menu.csv)
- Mapping config: [`benchmark/data/pizza-mapping.json`](benchmark/data/pizza-mapping.json)
- SHACL shapes: [`benchmark/data/pizza-shapes.ttl`](benchmark/data/pizza-shapes.ttl)
- Comparison script: [`benchmark/pizza_extend_compare.py`](benchmark/pizza_extend_compare.py)

### IES4 Building Domain — BORO/4D Methodology

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
python3 pizza_extend_compare.py   # Extension: 13/13 vegetarian classification
python3 compare.py                # IES4: 86/86 compliance checks passed
```

## Stack

- **Rust** (edition 2024) — single binary, no JVM
- **Oxigraph 0.4** — pure Rust RDF/SPARQL engine
- **rmcp** — MCP protocol implementation
- **SQLite** — ontology version storage

## License

MIT
