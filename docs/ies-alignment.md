# IES:Building Alignment Example

This guide demonstrates aligning the [IES Building Extension](../benchmark/generated/ies-building-extension.ttl) with another domain ontology using `onto_align`. This directly mirrors NDTP's active work transforming energy performance data into RDF aligned with IES:Building.

## Background

NDTP is building data pipelines that transform energy performance data (EPCs, heating systems, insulation records) into RDF aligned with IES. The IES Building Extension models:

- **Building** → subclass of `ies:Facility` and `ies:Asset`
- **Dwelling** → subclass of `ies:PartOfFacility` (a self-contained residential unit)
- **EnergyPerformanceCertificate** → subclass of `ies:Document`
- **HeatingSystem** / **InsulationElement** → subclasses of `ies:Asset`
- **RetrofitIntervention** / **EnergyPerformanceReview** → subclasses of `ies:Event`

All following the 4D extensionalist (BORO) pattern with temporal states and bounding states.

## Step 1: Load Both Ontologies

Load the IES Building Extension as the source and Schema.org as the target for alignment:

```text
# Load IES Building Extension
onto_validate benchmark/generated/ies-building-extension.ttl
onto_load benchmark/generated/ies-building-extension.ttl

# Save current state, then load target for alignment
onto_save /tmp/ies-building.ttl
onto_clear

# Load Schema.org as alignment target
onto_marketplace install schema-org
onto_save /tmp/schema-org.ttl
onto_clear
```

## Step 2: Run Alignment

```text
onto_align --source /tmp/ies-building.ttl --target /tmp/schema-org.ttl
```

The alignment engine uses 6 structural signals (+ embedding similarity if embeddings are loaded):

1. **Label match** — Jaro-Winkler similarity on rdfs:label
2. **Local name match** — IRI fragment comparison
3. **Comment overlap** — Token-level Jaccard on rdfs:comment
4. **Parent match** — Shared superclass names
5. **Property overlap** — Common property names in domain/range
6. **Sibling overlap** — Shared sibling class names
7. **Embedding similarity** — Cosine + Poincaré distance (if `onto_embed` was run)

### Expected Alignment Candidates

| IES Building Class | Schema.org Class | Signal |
| --- | --- | --- |
| `bldg:Building` | `schema:Accommodation` / `schema:House` | Label + comment overlap |
| `bldg:PostalCode` | `schema:postalCode` | Label match |
| `bldg:HeatingSystem` | `schema:EngineSpecification` | Property overlap (fuel, type) |
| `bldg:EnergyPerformanceCertificate` | `schema:EnergyConsumptionDetails` | Comment overlap |

## Step 3: Review and Accept Candidates

```text
# Accept a good match
onto_align_feedback --accept "bldg:Building = schema:Accommodation"

# Reject a false positive
onto_align_feedback --reject "bldg:HeatingSystem = schema:EngineSpecification"
```

Accepted/rejected feedback trains the self-calibrating confidence weights for future alignments.

## Step 4: Generate Alignment Triples

After reviewing candidates, use SPARQL CONSTRUCT to produce bridging triples:

```sparql
PREFIX bldg: <http://example.org/ontology/ies-building#>
PREFIX schema: <https://schema.org/>
PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX skos: <http://www.w3.org/2004/02/skos/core#>

CONSTRUCT {
  bldg:Building skos:closeMatch schema:Accommodation .
  bldg:Dwelling skos:closeMatch schema:House .
  bldg:PostalCode skos:relatedMatch schema:postalCode .
  bldg:EnergyPerformanceCertificate skos:relatedMatch schema:EnergyConsumptionDetails .
}
WHERE { }
```

Use `skos:closeMatch` (not `owl:equivalentClass`) because these are cross-domain approximations, not logical equivalences. The IES 4D model has temporal states that Schema.org lacks.

## NDTP Pipeline Pattern

In a real NDTP pipeline, the alignment flows like this:

```
EPC CSV data
  → onto_ingest (CSV → RDF using IES Building mapping)
  → onto_shacl (validate against IES Building SHACL shapes)
  → onto_reason (materialise inferred triples via OWL-RL)
  → onto_align (map to Schema.org / BRICK / other targets)
  → onto_push (publish to SPARQL endpoint)
```

This mirrors NDTP's actual architecture: ingest raw energy performance data, validate against IES, reason to fill gaps, then align to downstream consumer schemas.

## Aligning with BRICK Schema

For building management systems, [BRICK](https://brickschema.org/) is a more natural alignment target than Schema.org:

```text
# Pull BRICK ontology
onto_pull https://brickschema.org/schema/Brick.ttl
onto_save /tmp/brick.ttl
onto_clear

# Run alignment
onto_align --source /tmp/ies-building.ttl --target /tmp/brick.ttl
```

Expected high-confidence matches:

| IES Building | BRICK | Confidence |
| --- | --- | --- |
| `bldg:Building` | `brick:Building` | Very high (label + structure) |
| `bldg:HeatingSystem` | `brick:Heating_System` | High (label + parent overlap) |
| `bldg:WallInsulation` | `brick:Insulation` | Medium (partial label match) |

This demonstrates that Open Ontologies can serve as the alignment layer in NDTP's data integration architecture — mapping between IES (the canonical model) and domain-specific schemas used by different systems.
