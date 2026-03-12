# Infrastructure Positioning — Design

**Date:** 2026-03-12
**Goal:** Transform Open Ontologies from a Claude Code-only MCP server into production infrastructure for AI-generated knowledge graphs. "Terraform for Knowledge Graphs."

## Workstreams

1. CLI — thin wrapper over existing library
2. SQL schema import — postgres → OWL
3. Benchmarks — correctness (Pizza), performance (LUBM), end-to-end (SQL→OWL)
4. CI pipeline — GitHub Actions
5. README rewrite — infrastructure pitch, demo, benchmarks

## 1. CLI Architecture

Extend the existing `clap` binary with one subcommand per MCP tool. The binary already has `init` and `serve`. Each new subcommand calls the same library functions the MCP tools use.

### Subcommands

```
# Core ontology
validate <file|->
load <file>
save <file> [--format turtle|ntriples|rdfxml]
clear
stats
query <sparql|->
diff <file1> <file2>
lint <file>
convert <file> --to <format>
status

# Remote
pull <url>
push <endpoint>
import-owl <url>

# Versioning
version <name>
history
rollback <name>

# Data pipeline
map <datafile>
ingest <datafile> [--mapping <json>]
shacl <shapesfile>
reason [--profile rdfs|owl-rl|owl-rl-ext|owl-dl]
extend <datafile> [--shapes <file>] [--profile <profile>]

# Lifecycle
plan <file>
apply <safe|migrate>
lock <iri> [--reason <text>]
drift <file1> <file2>
enforce <generic|boro|value_partition>
monitor
monitor-clear
lineage

# Clinical
crosswalk <code> --system <system>
enrich <class-iri> <code> --system <system>
validate-clinical

# Schema import (NEW)
import-schema <connection-string> [--base-iri <iri>]
```

### Design decisions

- Subcommands map 1:1 to MCP tools (kebab-case)
- `stdin` support via `-` for piping
- JSON output by default, `--pretty` flag for human-readable
- Stateless: initializes StateDb + GraphStore per invocation (no daemon)
- No new Rust modules — each subcommand calls existing library functions

## 2. SQL Schema Import

New subcommand: `open-ontologies import-schema postgres://user:pass@host/dbname`

### Database crate

Use `sqlx` 0.8 with `runtime-tokio` + `postgres` features. Supports multi-database (postgres, mysql, sqlite) behind feature flags for future expansion. Start with PostgreSQL only.

### Schema introspection queries

Tables:
```sql
SELECT table_name FROM information_schema.tables
WHERE table_schema = 'public' AND table_type = 'BASE TABLE';
```

Columns:
```sql
SELECT column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_schema = 'public' AND table_name = $1
ORDER BY ordinal_position;
```

Primary keys:
```sql
SELECT kcu.column_name
FROM information_schema.table_constraints tc
JOIN information_schema.key_column_usage kcu
  ON tc.constraint_name = kcu.constraint_name
WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_name = $1;
```

Foreign keys:
```sql
SELECT kcu.column_name AS child_column,
       ccu.table_name AS parent_table,
       ccu.column_name AS parent_column
FROM information_schema.table_constraints tc
JOIN information_schema.key_column_usage kcu
  ON tc.constraint_name = kcu.constraint_name
JOIN information_schema.constraint_column_usage ccu
  ON tc.constraint_name = ccu.constraint_name
WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_name = $1;
```

### SQL → OWL mapping

| SQL Concept | OWL Concept |
|-------------|-------------|
| Table | `owl:Class` |
| Column (non-FK) | `owl:DatatypeProperty` |
| Foreign key | `owl:ObjectProperty` |
| Primary key | `owl:FunctionalProperty` + identifier annotation |
| NOT NULL | `owl:minCardinality 1` |
| UNIQUE | `owl:maxCardinality 1` |
| SQL type | `xsd:` datatype range |

### SQL type → XSD mapping

| SQL | XSD |
|-----|-----|
| integer, bigint, smallint | `xsd:integer` |
| numeric, decimal, real, double | `xsd:decimal` |
| boolean | `xsd:boolean` |
| varchar, text, char | `xsd:string` |
| date | `xsd:date` |
| timestamp | `xsd:dateTime` |
| bytea, blob | `xsd:hexBinary` |

### Output

Generates Turtle, validates, loads into graph store, prints stats JSON. Base IRI defaults to `http://example.org/db/`, overridable with `--base-iri`.

## 3. Benchmarks

Three tiers, each proving a different claim.

### Tier 1: Correctness (Pizza)

- Load Manchester Pizza OWL into HermiT, Pellet, and Open Ontologies
- Run full classification (compute subsumption hierarchy)
- Compare inferred subsumptions — must match exactly
- Output: pass/fail table + differences

Implementation: Java wrapper using OWL API + HermiT/Pellet jars. Rust CLI for Open Ontologies. Python script diffs results.

### Tier 2: Performance (LUBM)

- LUBM generator produces ontologies at 1K, 5K, 10K, 50K axiom counts
- Run classification on all three reasoners
- Measure wall-clock time, peak memory
- Output: comparison table + chart

Implementation: Shell script orchestrates. Java for HermiT/Pellet. Rust CLI for Open Ontologies. Python plots chart (matplotlib).

### Tier 3: End-to-End (SQL→OWL)

- Docker compose: postgres with sample schema (users, orders, products)
- `import-schema → reason → query` pipeline, timed
- Not a reasoner comparison — demonstrates full workflow

Implementation: Docker compose + seed SQL + shell script.

### No Rust recompilation

Benchmark harness is shell + Java + Python only. Rust binary is built once upfront.

## 4. CI Pipeline

GitHub Actions, three workflows.

### ci.yml — on push/PR

```yaml
- cargo build
- cargo test
- cargo clippy -- -D warnings
- cargo audit
```

### benchmark.yml — manual trigger only

```yaml
- Build Open Ontologies
- Download HermiT + Pellet jars
- Set up Java
- Run Pizza correctness benchmark
- Run LUBM performance benchmark
- Upload results as artifacts
```

### release.yml — on tag push

```yaml
- Cross-compile (linux-x86_64, macos-aarch64, macos-x86_64)
- Create GitHub release with binaries
```

### README badges

- CI status
- Test count
- License
- Latest release

## 5. README Rewrite

### Structure

1. Title + one-liner ("Terraform for Knowledge Graphs")
2. Badges (CI, tests, license, release)
3. Keep "Why not just ask Claude directly?" section as-is
4. NEW: Infrastructure pitch
   - "AI can generate knowledge, but only consistent knowledge is usable."
   - "Open Ontologies is the safety and automation layer."
5. NEW: Demo — SQL→OWL 5-command walkthrough
6. How it works (keep existing)
7. Extending ontologies with data (keep existing)
8. Ontology Lifecycle (keep v2 section)
9. Architecture (keep updated diagram)
10. Tools table (keep, 35 tools)
11. NEW: CLI reference
12. NEW: Benchmarks (HermiT/Pellet comparison table + chart)
13. Keep existing detailed benchmark methodology below
14. OWL2-DL Reasoning (keep existing)
15. Replicate it yourself (update with CLI)
16. Stack
17. License

### Key messaging

- Position as infrastructure, not a library
- "Terraform / Kubernetes for Knowledge Graphs"
- "AI can generate knowledge, but only consistent knowledge is usable."
- "Open Ontologies is the safety and automation layer."
- Blog-style narrative stays in detailed benchmarks section, hard numbers go in new benchmarks section above it

## Execution Order

1. CLI subcommands (wrapping existing library)
2. SQL schema import (new `import-schema` subcommand)
3. Benchmarks (Pizza correctness, LUBM performance, SQL→OWL end-to-end)
4. CI pipeline (GitHub Actions)
5. README rewrite (infrastructure pitch, demo, benchmarks, CLI reference)
