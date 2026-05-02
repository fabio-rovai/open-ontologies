# Date Normalisation Decision Tree

A practical decision tree for converting messy date data into NAPH-compliant ISO 8601 / XSD-typed dates.

## The basic rule

Free-text dates are **not permitted** in NAPH-compliant records. Every date MUST be expressed as one of:

- `xsd:date` — full date (`1944-03-28`)
- `xsd:gYearMonth` — year and month (`1944-03`)
- `xsd:gYear` — year only (`1944`)
- A `dcterms:PeriodOfTime` for a range

If the source data cannot be resolved to at least year precision, the record cannot use `naph:capturedOn` and must be flagged for review.

---

## Q1: What does the source date look like?

→ ISO 8601 already (`1944-03-28`, `1944-03`, `1944`): **commit as-is**, just add the XSD type
→ Common date format (e.g. `28 March 1944`, `28/03/1944`): **go to Q2**
→ Range or interval (`1943-1944`, `Spring 1944`): **go to Q3**
→ Relative or approximate (`c. 1944`, `circa 1944`, `early 1940s`): **go to Q4**
→ Pure free-text (`unknown`, `n.d.`, `before WW2`): **go to Q5**

---

## Q2: Common date format normalisation

For mechanical date format conversion, use the [`pipeline/ingest.py`](../../../pipeline/ingest.py) `normalise_date` function or equivalent. It handles:

- `28 March 1944` → `1944-03-28`
- `28/03/1944` → `1944-03-28` (UK convention)
- `03/28/1944` → `1944-03-28` (US convention — must be detected from context)
- `15-Jun-1947` → `1947-06-15`
- `15-Jun-47` → `1947-06-15` (two-digit years assumed 19xx for archival)

### Q2.1: UK or US date convention?

For UK heritage data: assume DD/MM/YYYY unless the institution's convention says otherwise.

For US-sourced data: assume MM/DD/YYYY.

For mixed sources: **always** require an explicit convention declaration in the source CSV header (e.g. `date_format=DD/MM/YYYY`).

### Q2.2: Two-digit years

`28/03/44` could mean 1944 OR 2044. For pre-1950 archives, the rule is:

- 00-29 → 19xx (i.e. `28` → `1928`)
- 30-99 → 19xx (i.e. `47` → `1947`)
- 30-29 → 20xx (i.e. `28` → `2028`) **only if** the collection contains records that postdate 2000

Modern collections should require four-digit years.

---

## Q3: Range or interval

### Q3.1: A specific range with known start and end

```turtle
ex:photo-X dcterms:temporal [
    a dcterms:PeriodOfTime ;
    dcat:startDate "1943-09-01"^^xsd:date ;
    dcat:endDate "1944-03-31"^^xsd:date
] ;
naph:capturedOn "1944"^^xsd:gYear .
```

Use `dcterms:temporal` for the full range AND `naph:capturedOn` for the most-likely date (year-precision).

### Q3.2: Season

`Spring 1944` → use `xsd:gYearMonth` for the central month, with annotation:

- Northern Hemisphere Spring → use `1944-04` and annotate `naph:dateUncertaintyNote "Spring 1944 — exact date unknown"`
- Use the conventional centre month for each season

### Q3.3: Decade

`Early 1940s` → use `xsd:gYear` `1942` (or pick a representative year), annotate.

`Mid-1940s` → use `1945`.

`Late 1940s` → use `1948`.

Always include an uncertainty annotation.

---

## Q4: Approximate / circa

`c. 1944` → use `xsd:gYear` `1944`, annotate:

```turtle
ex:photo-X naph:capturedOn "1944"^^xsd:gYear ;
    naph:dateUncertainty "approximate" ;
    naph:dateUncertaintyNote "c. 1944 — based on archival arrangement context" .
```

### Q4.1: How accurate is "circa"?

The convention varies:

- "c." in archival context → typically ±5 years
- "early/mid/late" → typically resolve to a third of the decade

If the institution's documentation defines precision more narrowly, use that.

### Q4.2: Why the year-precision date even when uncertain?

Because researchers will query date ranges. A photograph dated "c. 1944" should match a query for `1940s photographs`. Without `naph:capturedOn` of any precision, the record is unfindable temporally.

---

## Q5: Free-text or unknown dates

### Q5.1: "Unknown", "n.d." (no date), "undated"

The record cannot use `naph:capturedOn`. Three options:

1. **Exclude from NAPH publication** — return to cataloguing for date research
2. **Publish at a fallback tier** with an annotation:

```turtle
ex:photo-X naph:dateUnknown true ;
    naph:dateUncertaintyNote "Date unknown — cataloguer notes give no temporal context" .
```

Note: a record without `naph:capturedOn` does NOT meet Baseline tier requirements. It must be excluded from any tier compliance claim, OR it must use a coarse `naph:capturedOn` based on collection-level context.

### Q5.2: Approximate context ("WW2-era", "Victorian", "post-war")

Resolve to the conventional year range for the period:

- "WW2" / "Wartime" → `xsd:gYear` `1942` with range annotation `1939-1945`
- "Victorian" → `xsd:gYear` `1875` with range annotation `1837-1901`
- "Post-war" (UK) → `xsd:gYear` `1948` with range annotation `1945-1955`

Always annotate the source phrase.

### Q5.3: "Before X" / "After X"

```turtle
ex:photo-X naph:dateUncertaintyNote "Before 1945 — verso annotation 'wartime issue'" ;
    naph:capturedOn "1944"^^xsd:gYear ;
    naph:dateUncertainty "before-bound" .
```

The `naph:capturedOn` value is the institution's best estimate; `naph:dateUncertainty` records the type of uncertainty.

---

## Q6: Date range expressed as start + end

For a record covering a sortie or campaign that spans multiple dates:

```turtle
ex:sortie-X dcterms:temporal [
    a dcterms:PeriodOfTime ;
    dcat:startDate "1943-09-01"^^xsd:date ;
    dcat:endDate "1944-03-31"^^xsd:date
] .
```

Each individual record (frame) within the sortie has its own specific `naph:capturedOn`.

---

## Q7: Edge cases

### Q7.1: BCE / pre-1583 dates

NAPH allows them but the XSD types support both:

- `xsd:date` `-0500-04-15` (proleptic Gregorian)
- For pre-Gregorian dates, also annotate the calendar system

For most heritage collections, this is rare. When it applies, follow [W3C XSD date documentation](https://www.w3.org/TR/xmlschema11-2/#date).

### Q7.2: Multiple plausible dates

If a record could be 1944-03-28 OR 1944-04-15:

```turtle
ex:photo-X naph:capturedOn "1944-03"^^xsd:gYearMonth ;  # use shared precision
    naph:dateUncertaintyNote "Possibly 28 March or 15 April 1944 — sortie log incomplete" .
```

Use the most precise date that both candidates share.

### Q7.3: Different time zones

Most heritage records have date-only precision; time zones don't matter. For records with sub-day precision (modern born-digital):

- Always use UTC (`xsd:dateTime` ending in `Z`)
- Annotate the original local time zone if relevant

---

## Implementation checklist

For your collection:

- [ ] Audit all existing date formats — which ones appear?
- [ ] Identify any UK/US convention ambiguity
- [ ] Decide policy on partial / approximate dates
- [ ] Configure ingest pipeline `normalise_date` patterns
- [ ] Run normalisation on a 100-record sample, manually verify
- [ ] Roll out to full collection
- [ ] Document any records that couldn't be normalised — these need cataloguing review

## Cross-references

- [Module B.4 — Date handling](../../01-standard/modules/B-metadata-data-structures.md#b4-date-handling)
- [Ingest pipeline](../../../pipeline/ingest.py) — reference implementation
- [W3C XSD date types](https://www.w3.org/TR/xmlschema11-2/)
- [DCMI Period of Time](https://www.dublincore.org/specifications/dublin-core/dcmi-terms/#http://purl.org/dc/terms/PeriodOfTime)
