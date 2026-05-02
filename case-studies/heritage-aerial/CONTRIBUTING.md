# Contributing to NAPH

NAPH is a case study under the [Open Ontologies](https://github.com/fabio-rovai/open-ontologies) project, demonstrating tiered digitisation standards for aerial photography heritage collections.

## How to contribute

The most valuable contributions right now are:

### Real-world record curation

The sample dataset is currently 10 illustrative records modeled on publicly known NCAP collection structure. We'd welcome contributions of:

- Real records from publicly accessible aerial photography catalogues (NCAP Air Photo Finder, NARA, USGS Earth Explorer, Bing Maps Imagery archives)
- Mapped to the NAPH ontology with appropriate rights attribution
- Submitted as additions to `data/sample-photographs.ttl`

Please **do not** submit:

- Records under restricted rights without clear permission
- Synthetic data presented as real
- Records from non-aerial-photography collections (this is a single-domain case study)

### Ontology refinements

If you spot:

- Missing properties common to aerial photography that aren't modelled
- Mappings to additional standards (CIDOC-CRM, IIIF technical terms, EXIF for born-digital)
- Edge cases the SHACL shapes don't catch

…please open an issue with a worked example showing what's missing and why.

### Pipeline improvements

The CSV ingest pipeline (`pipeline/ingest.py`) handles the date formats and field types we've seen in the sample. Real heritage CSVs will have more variation. Pull requests welcome for:

- Additional date format handling
- Robust geometry construction (e.g. proper field-of-view from altitude + camera spec)
- Better identifier-minting strategies

### Cross-collection alignments

NAPH should align cleanly with adjacent standards. We'd value contributions of:

- SKOS exact-match crosswalks to CIDOC-CRM, EDM (Europeana Data Model), Schema.org
- Worked examples showing federation queries across NAPH and other linked-data heritage sources

## Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/your-change`)
3. Make your change, including:
   - Validation: `open-ontologies validate ontology/naph-core.ttl`
   - SHACL conformance: see commands in README
   - Competency-question regression: `open-ontologies batch docs/competency-queries.batch.txt`
4. Add or update relevant docs
5. Open a pull request with a clear description of the change and its rationale

## Style notes

- TTL files use 4-space indentation, one statement per logical group, comments explain non-obvious modelling choices
- Markdown follows the existing case-study tone — direct, evidence-led, no marketing fluff
- Python scripts are self-contained, type-hinted where it adds clarity, no external dependencies beyond the standard library

## Questions

Open an issue, or contact `fabio@thetesseractacademy.com`.
