# Open Ontologies v2 — The Ontology Runtime

## Vision

Every AI system that works with structured knowledge hits the same wall: ontologies are fragile, drift silently, break downstream, and nobody knows what changed or why. The tools are 20 years old, Java-based, and designed for humans clicking through Protege.

Open Ontologies v2 is the execution layer for ontology-driven AI. Everything that touches structured knowledge — KG builders, research agents, clinical systems, data pipelines — runs through this.

## Scope

Six new feature areas, all implemented as additional MCP tools in the existing server. No architecture change — same binary, same Oxigraph backend, same MCP protocol.

### What we're building

| Feature | Tools | Priority |
|---------|-------|----------|
| Plan/Apply/Migrate | `onto_plan`, `onto_apply`, `onto_migrate`, `onto_lock` | P0 |
| Drift Detection | `onto_drift` | P0 |
| Active Monitor | `onto_monitor`, `onto_monitor_status` | P0 |
| Design Pattern Enforcement | `onto_enforce` | P1 |
| Clinical Crosswalks | `onto_crosswalk`, `onto_enrich`, `onto_validate_clinical` | P1 |
| Lightweight Lineage | `onto_lineage` | P2 |

### What we're NOT building

- Ontology DSL (Claude generates Turtle/OWL natively — a DSL adds compilation for no gain)
- State manager with immutable graph (existing `onto_version`/`onto_rollback` is sufficient)
- Federated ontology discovery (OLS/BioPortal API wrappers — fragile, scope creep)
- Full DAG lineage with triple-level provenance (resource-heavy, low impact)

---

## Feature 1: Plan/Apply/Migrate

### Problem

Changes go straight into the store. Load, overwrite, hope for the best. No preview, no impact analysis, no controlled rollout. When a class rename breaks 800 triples and 2 SHACL shapes, you find out after the fact.

### Design

#### `onto_plan(new_turtle: String) -> JSON`

Compares desired ontology (new Turtle/OWL) against current store state. Produces a change plan with full blast radius analysis.

Output:

```json
{
  "plan": {
    "changes": [
      {
        "action": "rename_property",
        "from": "ex:authoredBy",
        "to": "ex:writtenBy",
        "blast_radius": {
          "triples": {"direct": 847, "inferred": 1203, "total": 2050},
          "shacl": {"broken_shapes": ["AuthorShape"], "new_violations": 23},
          "individuals": {"type_changes": 23, "lost_assertions": 847},
          "reasoning": {"subsumptions_lost": 4, "new_unsatisfiable": 0, "inferred_triples_invalidated": 1203}
        }
      }
    ],
    "risk_score": "high",
    "recommendation": "rename breaks downstream — consider owl:equivalentProperty bridge instead"
  }
}
```

Implementation:
- Load new Turtle into a temporary in-memory Oxigraph store
- SPARQL diff: classes/properties in current but not in new (removed), vice versa (added)
- For each removal, count triples referencing that IRI
- Run SHACL against the hypothetical new state to detect new violations
- Run reasoner on hypothetical state to detect new unsatisfiable classes
- Score risk: low (additions only), medium (modifications), high (removals with dependents)

#### `onto_apply(mode: String) -> JSON`

Executes the most recent plan.

Modes:

| Mode | Behavior |
|------|----------|
| `safe` | Snapshot first. Apply changes. Run monitor. Auto-rollback if unsatisfiable classes appear or SHACL violations increase. |
| `force` | Snapshot first. Apply changes. No guardrails. |
| `migrate` | Snapshot first. Apply changes. Auto-generate equivalence bridges for all renames/removals. |

#### `onto_migrate` -> JSON`

Generates backwards-compatibility triples for schema changes:

```turtle
ex:authoredBy owl:equivalentProperty ex:writtenBy .
ex:authoredBy owl:deprecated true .
ex:authoredBy rdfs:comment "Replaced by ex:writtenBy in v3. Migration applied 2026-03-11." .
ex:Author owl:equivalentClass ex:Researcher .
ex:Author owl:deprecated true .
```

Old IRIs forward to new ones. Downstream systems that still reference old terms don't break.

#### `onto_lock(iris: Vec<String>) -> JSON`

Freezes specific IRIs. Any `onto_plan` that would modify a locked IRI is rejected. Critical for production ontologies where downstream systems depend on stable IRIs.

Locks stored in SQLite. Checked during plan generation.

### Module

`plan.rs` — ~600 lines. Temporary Oxigraph store for hypothetical state comparison.

---

## Feature 2: Drift Detection

### Problem

Between iterations, the LLM silently renames classes/properties. `authoredBy` becomes `writtenBy` becomes `author`. Each version passes validation. The ontology drifts.

### Design

#### `onto_drift(version_a: String, version_b: String) -> JSON`

Compares two named versions (from existing `onto_version` snapshots).

Detection method — for every class/property in version A, check if it exists in version B. If missing, find rename candidates by scoring four signals:

1. **Same domain + range** (for properties) — strongest signal
2. **Same superclass position** (for classes) — hierarchy preservation
3. **Label similarity** — Jaro-Winkler on `rdfs:label`
4. **Same individual membership** — instances typed with old class now typed with new

Output:

```json
{
  "stable": ["Paper", "Person"],
  "likely_renames": [
    {
      "from": "authoredBy",
      "to": "writtenBy",
      "confidence": 0.87,
      "signals": {"domain_range": true, "label_similarity": 0.72, "hierarchy": false, "individuals": true}
    }
  ],
  "added": ["Researcher"],
  "removed": ["Author"],
  "drift_velocity": 0.15,
  "vocabulary_stability": 0.85
}
```

#### Self-calibrating confidence

The confidence model improves with use via a feedback loop.

When `onto_drift` reports a likely rename, the LLM or human confirms or rejects. Decisions stored in SQLite:

```sql
CREATE TABLE drift_feedback (
    id TEXT PRIMARY KEY,
    from_iri TEXT,
    to_iri TEXT,
    predicted TEXT,        -- 'rename' | 'different_concept'
    confidence REAL,
    actual TEXT,           -- 'rename' | 'different_concept' (user feedback)
    signal_domain_range INTEGER,
    signal_label_sim REAL,
    signal_hierarchy INTEGER,
    signal_individuals INTEGER,
    timestamp TEXT
);
```

After 20+ feedback entries, fit a logistic regression on the four signal features to re-weight confidence scoring. Pure Rust — no ML framework. Just:

```
confidence = sigmoid(w1 * domain_range + w2 * label_sim + w3 * hierarchy + w4 * individuals + bias)
```

Weights recalculated on each `onto_drift` call if new feedback exists. Falls back to equal weights with 0.5 threshold until enough data accumulates.

#### `onto_lock` (shared with plan.rs)

Locked IRIs are also checked during drift — if a locked IRI was renamed, drift severity escalates to critical.

### Module

`drift.rs` — ~400 lines. Jaro-Winkler implemented in pure Rust (small function). Logistic regression is ~30 lines.

---

## Feature 3: Active Monitor

### Problem

Ontology corruption is discovered too late — after queries return wrong results or downstream systems break. There's no continuous health check.

### Design

#### `onto_monitor(watchers: JSON) -> JSON`

Configure watchers. Stored in SQLite. Persist across sessions.

```json
{
  "watchers": [
    {"id": "unsatisfiable", "check": "unsatisfiable_classes", "threshold": 0, "severity": "critical", "action": "auto_rollback"},
    {"id": "shacl", "check": "shacl_violation_count", "threshold": 0, "severity": "error", "action": "block_next_apply"},
    {"id": "drift", "check": "vocabulary_change_rate", "threshold": 0.15, "severity": "warning", "action": "notify"},
    {"id": "orphans", "check": "classes_without_subclasses_or_instances", "threshold": 10, "severity": "warning", "action": "notify"},
    {"id": "enforce", "check": "enforce_rule_violations", "threshold": 0, "severity": "error", "action": "block_next_apply"},
    {"id": "budget", "check": "total_triples", "threshold": 100000, "severity": "warning", "action": "notify"}
  ]
}
```

Custom watchers via SPARQL:

```json
{
  "id": "patients_without_id",
  "check": "sparql",
  "query": "SELECT (COUNT(?p) AS ?c) WHERE { ?p a fhir:Patient . FILTER NOT EXISTS { ?p fhir:identifier ?id } }",
  "threshold": 0,
  "severity": "error",
  "action": "block_next_apply",
  "message": "Patients without identifiers"
}
```

#### Event-driven execution

Every tool that mutates the store (`onto_load`, `onto_apply`, `onto_reason`, `onto_ingest`) triggers the monitor automatically after execution. Not polling.

#### Actions

| Action | Behavior |
|--------|----------|
| `notify` | Warning included in tool response JSON |
| `block_next_apply` | Sets flag — `onto_apply` refuses until violations resolved |
| `auto_rollback` | Immediately restore pre-mutation snapshot |
| `log` | Record to lineage log silently |

#### Monitor output in every mutating tool response

```json
{
  "result": "loaded 847 triples",
  "monitor": {
    "status": "blocked",
    "alerts": [
      {"watcher": "shacl", "severity": "error", "value": 3, "threshold": 0, "action": "block_next_apply", "detail": "PersonShape.birthDate (minCount)"}
    ],
    "passed": ["unsatisfiable", "drift", "orphans", "budget"]
  }
}
```

#### `onto_monitor_status` -> JSON`

Check current state of all watchers without mutating anything. Returns same format as the monitor section above.

### Module

`monitor.rs` — ~350 lines. Runs after every mutation via a `run_watchers()` call at the end of each mutating tool handler.

---

## Feature 4: Design Pattern Enforcement

### Problem

OWL reasoners catch logic errors (unsatisfiable classes, contradictions). They don't catch bad modelling: flat taxonomies, missing BORO state classes, incomplete value partitions, orphan classes. LLMs produce these constantly.

### Design

#### `onto_enforce(rule_pack: String) -> JSON`

Run a named rule pack against the current store.

#### Built-in rule packs

**`generic`** (always available):

| Rule | SPARQL check | Severity |
|------|-------------|----------|
| Cyclic hierarchy | Class is its own ancestor via rdfs:subClassOf | error |
| Flat taxonomy | Class with >50 direct subclasses | warning |
| Missing domain/range | ObjectProperty without rdfs:domain or rdfs:range | warning |
| Orphan class | Class with no subclasses, no instances, no restrictions referencing it | warning |
| Duplicate labels | Two different IRIs with same rdfs:label | warning |
| Naming convention | Classes not PascalCase or properties not camelCase | warning |
| Missing label | owl:Class without rdfs:label | error |
| Missing comment | owl:Class without rdfs:comment | warning |

**`boro`** (4D perdurantist modelling):

| Rule | Check | Severity |
|------|-------|----------|
| Missing State class | Entity class without corresponding State subclass | error |
| BoundingState required | State class without isStartOf/isEndOf relationships | error |
| ClassOf hierarchy | Top-level domain classes must have ClassOf metaclasses | warning |
| Entity-State domain/range | Properties linking Entity↔State must have correct types | error |
| Temporal extent | Every temporal entity must have begin/end BoundingStates | warning |

**`value_partition`**:

| Rule | Check | Severity |
|------|-------|----------|
| Exhaustive partition | Partition values must cover the partitioned class (covering axiom) | error |
| Pairwise disjoint | All values in a partition must be owl:disjointWith each other | error |
| Complete assignment | Every instance of partitioned class must have exactly one value | warning |
| Orphan values | Partition value defined but never used in any restriction | warning |

**Custom rules** — user-defined SPARQL ASK:

```json
{
  "id": "every_drug_has_indication",
  "query": "ASK { ?d a :Drug . FILTER NOT EXISTS { ?d :hasIndication ?i } }",
  "severity": "error",
  "message": "Drug without indication"
}
```

Custom rules stored in config file or SQLite. Can be loaded via `onto_enforce` with a JSON rule set.

#### Output

```json
{
  "rule_pack": "boro",
  "violations": [
    {"rule": "missing_state_class", "entity": "ex:Building", "severity": "error", "message": "Building has no corresponding BuildingState class"},
    {"rule": "bounding_state", "entity": "ex:OccupancyState", "severity": "error", "message": "OccupancyState missing isStartOf/isEndOf"}
  ],
  "passed": 8,
  "failed": 2,
  "compliance": 0.80
}
```

### Module

`enforce.rs` — ~500 lines. Each rule is a SPARQL ASK/SELECT query executed against the store. Rule packs are just collections of queries.

---

## Feature 5: Clinical Crosswalks

### Problem

LLMs hallucinate medical terms. "HyperTensionSyndrome" instead of "Hypertension". No grounding against standard medical vocabularies. Existing solutions require UMLS API keys or 30GB downloads.

### Design

#### Data source

Pre-built Parquet file shipped with the project, assembled from free open crosswalk files:

- WHO ICD-10 mapping tables (~2MB CSV)
- SNOMED-CT to ICD-10 official WHO map (~5MB)
- MeSH descriptors subset (~3MB)
- BioPortal LOOM cross-ontology alignments (~3MB)

Build script (`scripts/build_crosswalks.py`) downloads these publicly available files, normalizes into a single Parquet:

```
| source_code | source_system | target_code | target_system | relation   | source_label       | target_label           |
|-------------|---------------|-------------|---------------|------------|--------------------|------------------------|
| 38341003    | SNOMED        | I10         | ICD10         | exactMatch | Hypertension       | Essential hypertension |
| D006973     | MeSH          | I10         | ICD10         | closeMatch | Hypertension       | Essential hypertension |
```

Ships as `data/crosswalks.parquet` (~2-5MB). No API keys, no licenses, no network at runtime.

#### `onto_crosswalk(code: String, source_system: String, target_system: String) -> JSON`

Query the local Parquet for concept mappings.

```json
{
  "source": {"code": "38341003", "system": "SNOMED", "label": "Hypertension"},
  "mappings": [
    {"code": "I10", "system": "ICD10", "label": "Essential hypertension", "relation": "exactMatch"},
    {"code": "I15", "system": "ICD10", "label": "Secondary hypertension", "relation": "narrowMatch"}
  ]
}
```

#### `onto_enrich(class_iri: String, code: String, system: String) -> JSON`

Link an ontology class to a standard medical concept:

```turtle
ex:Hypertension skos:exactMatch <http://purl.bioontology.org/ontology/SNOMEDCT/38341003> .
ex:Hypertension skos:notation "I10"^^icd10:code .
```

Grounds ontology classes in biomedical standards.

#### `onto_validate_clinical` -> JSON`

Check that clinical terms in the ontology actually exist in the crosswalk file. Catches hallucinated medical terms.

```json
{
  "validated": ["ex:Hypertension", "ex:Diabetes"],
  "unmatched": [
    {"class": "ex:HyperTensionSyndrome", "suggestion": "Hypertension (SNOMED:38341003)", "similarity": 0.82}
  ]
}
```

Uses the same Jaro-Winkler similarity from drift.rs for fuzzy matching against the Parquet labels.

### Module

`clinical.rs` — ~300 lines. Parquet reading via existing `arrow`/`parquet` crates.

---

## Feature 6: Lightweight Lineage

### Problem

When something goes wrong, nobody knows which operation caused it. Was it a reasoning step? A bad import? A drift from 5 iterations ago?

### Design

Minimal append-only log. Extension of the monitor, not a separate system. AI-readable compressed format.

#### Auto-recorded

Every tool call appends to the session log automatically. No separate tracking tool.

#### Format — compressed single-line events

```
G:abc123:1:1710168600:generate:0→847
V:abc123:2:1710168605:validate:ok
L:abc123:3:1710168606:load:847
R:abc123:4:1710168610:reason:owl-dl:847→1203
E:abc123:5:1710168612:enforce:2violations
P:abc123:6:1710168615:plan:3changes:risk=medium
A:abc123:7:1710168620:apply:safe:1203→1250
M:abc123:8:1710168622:monitor:blocked:shacl=3
```

Format: `type:session:seq:timestamp:operation:details`

Types: G=generate, V=validate, L=load, R=reason, E=enforce, P=plan, A=apply, M=monitor, D=drift, I=ingest, Q=query, X=crosswalk

Stored in SQLite. One row per event. Indexed by session_id.

#### `onto_lineage(session_id: String, format: String) -> String`

- `format="compact"` (default) — returns the compressed log lines
- `format="mermaid"` — optional Mermaid rendering for human inspection

No per-triple tracking. No DAG inference. Just an ordered log of what happened. Cheap to store, cheap to query, easy for the LLM to parse.

### Module

`lineage.rs` — ~150 lines. Append-only SQLite inserts. Format is just string concatenation.

---

## New module summary

| Module | File | Tools | Estimated lines |
|--------|------|-------|-----------------|
| Plan/Apply/Migrate | `src/plan.rs` | `onto_plan`, `onto_apply`, `onto_migrate`, `onto_lock` | ~600 |
| Drift Detection | `src/drift.rs` | `onto_drift` | ~400 |
| Active Monitor | `src/monitor.rs` | `onto_monitor`, `onto_monitor_status` | ~350 |
| Design Pattern Enforcement | `src/enforce.rs` | `onto_enforce` | ~500 |
| Clinical Crosswalks | `src/clinical.rs` | `onto_crosswalk`, `onto_enrich`, `onto_validate_clinical` | ~300 |
| Lightweight Lineage | `src/lineage.rs` | `onto_lineage` | ~150 |
| Server handlers | `src/server.rs` | 14 new tool handlers | ~400 |

**Total: ~2,700 lines of new Rust.**

## Dependencies

New crate additions to Cargo.toml:
- None required. All features use existing dependencies (oxigraph, rusqlite, serde_json, parquet, arrow, reqwest for build script only).

## Data files

- `data/crosswalks.parquet` — ~2-5MB, shipped with project
- `scripts/build_crosswalks.py` — one-time build script for the Parquet file

## Implementation order

1. Monitor (foundation — other features plug into it)
2. Lineage (cheap, monitor depends on logging)
3. Plan/Apply/Migrate (core Terraform loop)
4. Drift Detection (feeds into plan)
5. Enforce (plugs into monitor as a watcher)
6. Clinical Crosswalks (independent, can be parallel)
