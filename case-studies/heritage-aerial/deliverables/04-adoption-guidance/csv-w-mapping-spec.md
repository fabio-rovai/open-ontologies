# CSV-W Mapping Specification for NAPH

A specification for using [CSV on the Web (CSV-W)](https://www.w3.org/TR/tabular-data-primer/) metadata files alongside CSV exports of aerial photography heritage collections, so the CSV becomes self-describing for direct ingestion into NAPH-compliant Turtle.

## Why CSV-W

Many institutional cataloguing systems export CSV. The default NAPH ingest pipeline ([`pipeline/ingest.py`](../../pipeline/ingest.py)) expects a fixed column layout. Real-world institutions have different column names, orderings, and conventions.

CSV-W provides **a JSON-LD descriptor** that accompanies a CSV file and declares:

- Each column's data type and mapping to NAPH properties
- Per-column transformations (date format, value mappings)
- Identifier minting rules
- Required vs optional fields

With a CSV-W descriptor, an institution's existing CSV can be ingested into NAPH without writing custom code — the descriptor IS the configuration.

## Minimal NAPH CSV-W descriptor

For a CSV file `my-collection.csv` with columns `accession_no, sortie, frame, capture_date, lat, lon, rights`, an accompanying `my-collection.csv-metadata.json` would look like:

```json
{
  "@context": ["http://www.w3.org/ns/csvw", {"@language": "en"}],
  "url": "my-collection.csv",
  "dialect": {
    "delimiter": ",",
    "header": true,
    "encoding": "utf-8"
  },
  "tableSchema": {
    "columns": [
      {
        "name": "accession_no",
        "titles": "accession_no",
        "datatype": "string",
        "propertyUrl": "https://w3id.org/naph/ontology#hasIdentifier",
        "valueUrl": "https://example.org/photo/{accession_no}"
      },
      {
        "name": "sortie",
        "titles": "sortie",
        "datatype": "string",
        "propertyUrl": "https://w3id.org/naph/ontology#sortieReference"
      },
      {
        "name": "frame",
        "titles": "frame",
        "datatype": "integer",
        "propertyUrl": "https://w3id.org/naph/ontology#frameNumber"
      },
      {
        "name": "capture_date",
        "titles": "capture_date",
        "datatype": {
          "base": "date",
          "format": "yyyy-MM-dd"
        },
        "propertyUrl": "https://w3id.org/naph/ontology#capturedOn"
      },
      {
        "name": "lat",
        "titles": "lat",
        "datatype": "decimal"
      },
      {
        "name": "lon",
        "titles": "lon",
        "datatype": "decimal"
      },
      {
        "name": "rights",
        "titles": "rights",
        "datatype": "string",
        "propertyUrl": "https://w3id.org/naph/ontology#rightsLabel"
      }
    ],
    "primaryKey": "accession_no",
    "aboutUrl": "https://example.org/photo/{accession_no}"
  }
}
```

## NAPH-specific extensions

NAPH defines a small set of extension properties for CSV-W to express NAPH-specific transformations that core CSV-W doesn't cover:

### Date format normalisation

```json
{
  "name": "capture_date",
  "datatype": {
    "base": "string",
    "format": "naph:auto-date"
  }
}
```

The `naph:auto-date` format triggers the multi-pattern date normalisation logic in `pipeline/ingest.py`, including partial dates (`xsd:gYearMonth`, `xsd:gYear`) and circa annotations.

### Rights text → URI mapping

```json
{
  "name": "rights",
  "valueMapping": {
    "Crown Copyright Expired": "http://rightsstatements.org/vocab/NoC-OKLR/1.0/",
    "Crown Copyright": "https://www.nationalarchives.gov.uk/.../crown-copyright/",
    "Public Domain (US)": "http://rightsstatements.org/vocab/NoC-US/1.0/"
  },
  "propertyUrl": "https://w3id.org/naph/ontology#rightsURI"
}
```

The `valueMapping` is a custom NAPH extension. Standard CSV-W has `@id` references but no inline mapping table.

### Coordinate aggregation

To produce a `naph:GeographicFootprint` from separate lat/lon columns:

```json
{
  "virtualColumns": [
    {
      "name": "footprint",
      "valueUrl": "footprint-{accession_no}",
      "propertyUrl": "https://w3id.org/naph/ontology#coversArea",
      "valueExpression": "naph:fov-from-flight(lat, lon, altitude_m, focal_length_mm)"
    }
  ]
}
```

The `naph:fov-from-flight` virtual column expression invokes the field-of-view derivation logic from [`pipeline/footprint-from-flight.py`](../../pipeline/footprint-from-flight.py).

### Three-component identifier parsing

```json
{
  "virtualColumns": [
    {
      "name": "collection_code",
      "valueExpression": "naph:parse-collection-code(sortie_ref)"
    }
  ]
}
```

Splits a sortie reference like `RAF/106G/UK/1655` into `collection_code: "RAF"` and `sortie_local: "106G/UK/1655"`.

## Reference implementations

### CSV-W aware ingest (planned v0.4)

`pipeline/ingest-csvw.py` (planned) will:

1. Read the CSV-W descriptor
2. Apply the column mapping
3. Run NAPH-specific transformations (date, rights, FOV)
4. Output NAPH-compliant Turtle

For now, the existing `pipeline/ingest.py` works with a fixed column schema. CSV-W support is a v0.4 feature.

### Conversion from existing schemas

For institutions whose CSV doesn't yet have a CSV-W descriptor:

1. Run the CSV-W generator at https://csvw.io/converter (or use `csvw-tools` Python package)
2. Manually add NAPH `propertyUrl` fields to each column
3. Validate the descriptor
4. Use it for repeatable ingest

## Validation

Once a CSV has a CSV-W descriptor, the descriptor itself can be validated:

```bash
# Validate descriptor structure (W3C CSV-W spec compliance)
csvw-tools validate my-collection.csv-metadata.json

# Validate that descriptor produces valid NAPH (NAPH-specific)
python3 pipeline/ingest-csvw.py my-collection.csv | open-ontologies validate -
```

## Use cases

### Recurring institutional ingest

Store `my-collection.csv-metadata.json` in version control alongside the CSV export. Each export iteration uses the same descriptor, ensuring repeatability.

### Multi-institution interoperability

Two institutions with different CSV column conventions can both adopt NAPH by:

1. Each maintaining their own CSV-W descriptor
2. Both producing NAPH-compliant Turtle output
3. Federation queries work across both — the source CSV format is invisible at the RDF layer

### Aggregator-friendly publishing

Publishing the CSV-W descriptor alongside the CSV makes the dataset self-describing. Aggregators (Europeana, DPLA) can consume the CSV directly via the descriptor without bespoke transformations.

## Trade-offs

CSV-W is a **standardisation cost up front** that pays off in:

- Repeatability of ingest pipelines
- Interoperability across institutions
- Auditability of transformations
- Reduction of bespoke ETL code

For a one-off ingest, CSV-W adds overhead. For institutions doing repeated periodic exports (most NAPH adopters), CSV-W is a clear win.

## Cross-references

- [W3C CSV-W Primer](https://www.w3.org/TR/tabular-data-primer/)
- [W3C CSV-W Spec](https://www.w3.org/TR/tabular-metadata/)
- [`pipeline/ingest.py`](../../pipeline/ingest.py) — current fixed-schema ingest
- [Module B — Metadata](../01-standard/modules/B-metadata-data-structures.md)
- [Identifier Policy decision tree](decision-trees/identifier-policy.md)
