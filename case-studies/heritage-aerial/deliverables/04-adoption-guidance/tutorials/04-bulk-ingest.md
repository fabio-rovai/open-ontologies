# Tutorial 4 — Bulk Ingest from CSV

Manually crafting NAPH records as in Tutorials 1-3 is fine for understanding the format, but no-one wants to handcraft 100,000 records. This tutorial shows how to use the bulk ingest pipeline to convert legacy CSV exports into NAPH-compliant Turtle in one command.

**Estimated time:** 30-45 minutes
**Prerequisites:** Tutorial 1; Python 3.10+ installed

## The reference pipeline

The pipeline at [`pipeline/ingest.py`](../../../pipeline/ingest.py) handles common transformation needs:

- Date format normalisation (8 common formats → ISO 8601)
- Rights text → canonical URI mapping
- Coordinate point → polygon footprint construction
- Altitude unit conversion (feet → metres)
- Identifier minting from sortie+frame
- Coordinate range validation (rejects lat>90, lon>180)

It produces NAPH Baseline-compliant Turtle for any record where the source CSV has the expected fields.

## Step 1 — Prepare your CSV

The default ingest pipeline expects a CSV with these columns:

| Column | Required | Type | Description |
|---|---|---|---|
| `sortie_ref` | yes | string | Sortie reference (will become identifier component) |
| `frame_no` | yes | integer | Frame number within sortie |
| `date_text` | yes | string | Capture date in any common format |
| `squadron` | no | string | Squadron designation |
| `aircraft` | no | string | Aircraft type |
| `location` | yes | string | Place name (used in label) |
| `lat` | yes | decimal | Latitude (decimal degrees, WGS84) |
| `lon` | yes | decimal | Longitude (decimal degrees, WGS84) |
| `altitude_ft` | no | integer | Altitude in feet |
| `camera` | no | string | Camera type |
| `rights_text` | yes | string | Rights statement text (must match mapping) |
| `scan_date` | no | string | Digitisation date in any common format |
| `scan_dpi` | no | integer | Scan resolution in DPI |
| `scan_format` | no | string | Scan format ("TIFF", "JPEG", "JP2") |

Example CSV row:

```csv
RAF/106G/UK/1655,4023,28 March 1944,No 540 Sqn,Mosquito PR.IX,"Berlin, Germany",52.52,13.40,30000,F.52,Crown Copyright Expired,12/04/2018,1200,TIFF
```

## Step 2 — Run the ingest pipeline

```bash
python3 pipeline/ingest.py my-collection.csv > my-collection.ttl
```

The script:

1. Parses each row
2. Normalises dates to ISO 8601
3. Maps rights text to canonical URIs
4. Constructs WGS84 polygon footprint around the lat/lon point
5. Converts altitude from feet to metres
6. Mints stable URIs from sortie+frame
7. Outputs valid NAPH Turtle to stdout

You'll see a summary on stderr:

```
# Ingest complete: 100 records transformed.
```

Or, if there were errors:

```
# 5 errors during ingest:
#   RAF-106G-UK-1655-4023: Could not parse date: 'c. 1944'
#   RAF-test-UK-2-2: Could not parse date: '1944-13-45'
#   RAF-test-UK-5-5: latitude 91.0 out of valid range (-90, 90)
```

The script silently skips erroneous rows; the output Turtle contains only successfully-transformed records.

## Step 3 — Validate the output

```bash
open-ontologies validate my-collection.ttl
```

Should show:

```json
{"ok":true,"triples":N}
```

## Step 4 — Run SHACL validation

```bash
echo "clear
load ontology/naph-core.ttl
load my-collection.ttl
shacl ontology/naph-shapes.ttl" | open-ontologies batch
```

Expected: `"conforms": true, "violation_count": 0`.

## Step 5 — Self-assess

```bash
python3 pipeline/self-assessment.py my-collection.ttl
```

This produces a full self-assessment report including:

- Total records ingested
- Tier distribution (all should be Baseline from this pipeline)
- SHACL conformance
- Graph statistics

## Step 6 — Spot-check a few records

```bash
echo "clear
load ontology/naph-core.ttl
load my-collection.ttl
query \"PREFIX naph: <https://w3id.org/naph/ontology#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?photo ?label WHERE {
    ?photo a naph:AerialPhotograph ;
           rdfs:label ?label .
}
LIMIT 5\"" | open-ontologies batch
```

Verify the records look right — do labels make sense, are dates parsed correctly, are footprints in the right hemisphere?

## Customising the pipeline

The default `ingest.py` handles common cases. For your specific data:

### Adding a new date format

Edit `DATE_PATTERNS` in `pipeline/ingest.py`:

```python
DATE_PATTERNS = [
    "%d %B %Y",        # 28 March 1944
    "%d-%b-%Y",        # 15-Jun-1947
    # ... existing patterns ...
    "%Y%m%d",          # add: 19440328 (compact ISO)
]
```

### Adding a new rights mapping

Edit `RIGHTS_MAPPING`:

```python
RIGHTS_MAPPING = {
    # ... existing mappings ...
    "Your Custom Rights Text": (
        "http://your-custom-rights-uri",
        "Your Custom Rights Label",
        "YourCustomSlug",
    ),
}
```

### Mapping different CSV column names

If your CSV has different column names, you have two options:

1. Pre-process the CSV to rename columns to match the pipeline's expected names
2. Modify `emit_record()` to read the alternative column names

Option 1 is usually less work.

## Handling errors

Common ingest errors and remediation:

### "Could not parse date"

The date isn't in any recognised format. Options:

- Add the format to `DATE_PATTERNS`
- Pre-process the CSV to normalise dates
- Reject the record (don't include in NAPH publication until date is established)

### "Unmapped rights text"

The rights string isn't in `RIGHTS_MAPPING`. Options:

- Add the mapping to `RIGHTS_MAPPING`
- Pre-process the CSV to use a known rights text

### "Latitude/longitude out of valid range"

The coordinate is wrong (e.g. `91.0`, `200.0`). Options:

- Find and fix the source data error (typo, transposed values)
- Investigate whether the institution's coordinate convention is non-standard

### "Sortie reference contains invalid characters"

The sortie reference has characters that can't be slugified. Investigate — usually a data-entry artefact.

## Production considerations

For production use:

1. **Run in CI/CD** — every CSV update triggers re-ingest + re-validation
2. **Track ingest provenance** — record the pipeline version, ingest date, source CSV path in the output Turtle's metadata
3. **Validate before publishing** — never publish without successful SHACL validation
4. **Diff before deploying** — compare new vs old Turtle to catch regression
5. **Backup the source CSV** — the Turtle is derivative; the CSV is your source of truth

See the GitHub Actions workflow at [`.github/workflows/validate.yml`](../../../.github/workflows/validate.yml) for a CI-driven validation pattern.

## Next steps

- **Tutorial 5** — set up CI/CD validation
- **Tutorial 6** — add Enhanced/Aspirational metadata at scale
- **Tutorial 7** — publish to a SPARQL endpoint

## Cross-references

- [`pipeline/ingest.py`](../../../pipeline/ingest.py) — reference implementation
- [`pipeline/legacy-ncap-style.csv`](../../../pipeline/legacy-ncap-style.csv) — sample CSV
- [Module B — Metadata](../../01-standard/modules/B-metadata-data-structures.md)
- [Date Normalisation Decision Tree](../decision-trees/date-normalisation.md)
