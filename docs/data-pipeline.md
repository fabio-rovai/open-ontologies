# Data Pipeline

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

## Supported Formats

| Format | Extension |
| ------ | --------- |
| CSV | `.csv` |
| JSON | `.json` |
| NDJSON | `.ndjson` |
| XML | `.xml` |
| YAML | `.yaml` |
| Excel | `.xlsx` |
| Parquet | `.parquet` |

## Mapping Config

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

## Demo: Database to Ontology in 3 Commands

```bash
# Import a PostgreSQL schema as OWL
open-ontologies import-schema postgres://demo:demo@localhost/shop

# Classify with native OWL2-DL reasoner
open-ontologies reason --profile owl-dl

# Query the result
open-ontologies query "SELECT ?c ?label WHERE { ?c a owl:Class . ?c rdfs:label ?label }"
```
