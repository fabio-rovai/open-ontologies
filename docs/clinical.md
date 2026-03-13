# Clinical Crosswalks

For healthcare ontologies, three tools bridge clinical coding systems:

- `onto_crosswalk` — Look up mappings between ICD-10 (diagnoses), SNOMED CT (clinical terms), and MeSH (medical literature) from a Parquet-backed crosswalk file
- `onto_enrich` — Insert `skos:exactMatch` triples linking ontology classes to clinical codes
- `onto_validate_clinical` — Check that class labels align with standard clinical terminology

## Setup

Build the crosswalk data file:

```bash
python3 scripts/build_crosswalks.py
```

This creates `data/crosswalks.parquet` from public terminology sources.

## Usage

```bash
# Look up a code
open-ontologies crosswalk I10 --system ICD10

# Enrich a class with a mapping
open-ontologies enrich http://example.org/Hypertension I10 --system ICD10

# Validate all labels against clinical terms
open-ontologies validate-clinical
```
