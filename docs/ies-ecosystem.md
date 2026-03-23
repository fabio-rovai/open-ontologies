# IES Ecosystem Demo

The [Information Exchange Standard (IES)](https://github.com/IES-Org) is a layered ontology framework used by the UK National Digital Twin Programme (NDTP). Open Ontologies supports the full IES stack through its marketplace.

## The IES Layers

IES is split into three tiers, each building on the last:

```
ies-top     (ToLO)    ~22 classes    BORO foundations, 4D extensionalism
    |
ies-core              ~131 classes   Persons, states, events, identifiers
    |
ies-common            ~513 classes   Full ontology — everything above + domain patterns
```

## Quick Start: Load the Full Stack

```text
# Load all three layers individually
onto_marketplace install ies-top
onto_stats
onto_clear

onto_marketplace install ies-core
onto_stats
onto_clear

# Or load the full IES Common (includes everything)
onto_marketplace install ies
onto_stats
```

## Full Pipeline Demo

### Step 1: Load and Validate

```text
onto_marketplace install ies
onto_stats
onto_lint
```

Expected output:
- 513 classes, 206 object properties, 4,040 triples
- 0 lint issues (IES v5 is well-formed)

### Step 2: Reason

```text
onto_reason --profile rdfs
onto_stats
```

RDFS materialises **+3,094 inferred triples** (77% growth) — transitive subclass chains across 241 State subclasses, 117 ClassOfEntity subclasses, and 102 Event subclasses.

### Step 3: Explore with SPARQL

See [ies-examples.md](ies-examples.md) for 5 ready-to-use queries. Quick taste:

```text
# Count State subclasses (the 4D temporal pattern)
onto_query "SELECT (COUNT(DISTINCT ?c) AS ?count) WHERE {
  ?c rdfs:subClassOf* <http://informationexchangestandard.org/ont/ies/common/State> .
  FILTER(?c != <http://informationexchangestandard.org/ont/ies/common/State>)
}"
# Result: 241
```

### Step 4: Load Example Data

IES has ~42 example Turtle files across its repos. Pull them directly:

```text
# Event participation patterns
onto_pull https://raw.githubusercontent.com/IES-Org/ont-ies/main/docs/examples/sample-data/event-participation.ttl

# Hospital scenario
onto_pull https://raw.githubusercontent.com/IES-Org/ont-ies/main/docs/examples/sample-data/hospital.ttl

# Movement tracking
onto_pull https://raw.githubusercontent.com/IES-Org/ont-ies/main/docs/examples/sample-data/movement.ttl

# GeoSPARQL integration
onto_pull https://raw.githubusercontent.com/IES-Org/ont-ies/main/docs/examples/sample-data/geosparql.ttl
```

Additional examples from telicent-oss:

```text
# Ship movement (4D spatiotemporal)
onto_pull https://raw.githubusercontent.com/telicent-oss/ies-examples/main/additional_examples/ship_movement.ttl

# Aircraft observation
onto_pull https://raw.githubusercontent.com/telicent-oss/ies-examples/main/additional_examples/observing_moving_aircraft.ttl

# Countries reference data
onto_pull https://raw.githubusercontent.com/telicent-oss/ies-examples/main/regions_of_the_world/countries.ttl
```

### Step 5: SHACL Validation

IES provides SHACL shapes for data validation:

```text
# Validate against IES Common shapes
onto_pull https://raw.githubusercontent.com/IES-Org/ont-ies/main/docs/specification/ies-common.shacl
onto_shacl
```

Core-level shapes for specific patterns:

```text
# Person state validation
onto_pull https://raw.githubusercontent.com/IES-Org/ies-core/main/spec/validation_artefacts/PersonState.shacl.ttl

# Birth/death state validation
onto_pull https://raw.githubusercontent.com/IES-Org/ies-core/main/spec/validation_artefacts/BirthStateShape.shacl.ttl
onto_pull https://raw.githubusercontent.com/IES-Org/ies-core/main/spec/validation_artefacts/DeathStateShape.shacl.ttl
```

### Step 6: Alignment

Map IES to other domain ontologies:

```text
# Save IES, load target, align
onto_save /tmp/ies.ttl
onto_clear
onto_marketplace install schema-org
onto_save /tmp/schema-org.ttl
onto_clear
onto_align --source /tmp/ies.ttl --target /tmp/schema-org.ttl
```

See [ies-alignment.md](ies-alignment.md) for the full IES:Building alignment walkthrough.

## IES Ecosystem Map

| Resource | Repo | What's There |
| --- | --- | --- |
| **IES Top (ToLO)** | `IES-Org/ies-top` | Foundational ontology + 2 SHACL shapes |
| **IES Core** | `IES-Org/ies-core` | Core patterns + 6 SHACL shapes + 7 test data files |
| **IES Common** | `IES-Org/ont-ies` | Full ontology (4 formats) + SHACL shapes + 15 examples + user guides + 70 diagrams |
| **IES Examples** | `telicent-oss/ies-examples` | 28 Turtle example files + regions reference data |
| **IES Tool** | `telicent-oss/ies-tool` | Python library + bundled v4.3 ontology + SHACL shapes |
| **RDF Transform** | `telicent-oss/telicent-rdf-transform` | IES4 to IES-Next migration mappings |
| **NDTP AI Extension** | `National-Digital-Twin/ndtp-ai-ontology-extension` | LLM-driven ontology extension tooling |
| **IES4 (archived)** | `dstl/IES4` | Legacy v4 ontology + examples + full spec PDF |

## Benchmark Results

| Layer | Classes | Properties | Triples | + RDFS | Fetch | RDFS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| IES Top (ToLO) | ~22 | TBD | TBD | TBD | TBD | TBD |
| IES Core | ~131 | TBD | TBD | TBD | TBD | TBD |
| **IES Common** | **513** | **206** | **4,040** | **+3,094** | **911ms** | **63ms** |

IES Common is the second-largest ontology in the marketplace by class count (after Schema.org's 1,009). RDFS reasoning adds 77% more triples — the richest inference gain of any non-general ontology, driven by the deep 4D State/Event/ClassOfEntity hierarchies.
