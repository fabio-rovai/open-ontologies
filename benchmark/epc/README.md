# EPC Ontology Benchmark

Benchmark comparing two IES Building ontologies on the same EPC data.

## Design

- **Data**: 200 synthetic UK domestic EPC records following real DESNZ distributions
- **Queries**: 15 analytical questions from published government/ONS EPC reports
- **Method**: Same CSV → each ontology's mapping → ingest → SPARQL → score

## Leakage Prevention

The queries are derived from published EPC policy analysis:
- DESNZ EPC Statistical Release Q2 2025
- ONS Energy Efficiency of Housing 2025
- English Housing Survey 2022-23
- MEES enforcement guidance
- Fuel Poverty Strategy Technical Annex

No query references ontology-specific class names. All queries use
generic RDF patterns (rdf:type, rdfs:label, property values).

## Scoring

Each query scores 0 or 1:
- **1**: Returns correct, complete results
- **0**: Fails (no mapping for required columns, missing classes, wrong results)

Maximum score: 15

## Column Coverage Results

| Metric | NDTP/IRIS | Open Ontologies |
| --- | ---: | ---: |
| EPC columns covered | 18/36 (50%) | **36/36 (100%)** |
| Validated triples | 1,349 | **3,068** |
