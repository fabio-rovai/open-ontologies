# ADR-0009: Date precision policy — three XSD types

**Status:** Accepted
**Date:** 2026-04-30
**Decider:** Editorial team

## Context

Aerial photography heritage has variable date precision:

- **Modern digital captures**: full date with timestamp known
- **Most WW2 reconnaissance**: full date (sortie logs are typically per-day)
- **Some pre-1939 material**: full date sometimes, year-and-month sometimes
- **Some legacy archives**: only year-level precision known
- **Unknown / undated**: a small but real subset

A standard must accommodate this variation while maintaining computational queryability.

## Decision

NAPH supports three XSD date types for `naph:capturedOn`:

| Precision | XSD type | Example |
|---|---|---|
| Day | `xsd:date` | `"1944-03-28"^^xsd:date` |
| Month | `xsd:gYearMonth` | `"1944-03"^^xsd:gYearMonth` |
| Year | `xsd:gYear` | `"1944"^^xsd:gYear` |

Records with no resolvable date cannot use `naph:capturedOn` and are not Baseline-compliant. They MAY publish with a `naph:dateUnknown true` annotation, but cannot claim Baseline tier.

For approximate dates ("c. 1944"), use the appropriate precision type with an additional annotation:

```turtle
ex:photo-X naph:capturedOn "1944"^^xsd:gYear ;
    naph:dateUncertainty "approximate" ;
    naph:dateUncertaintyNote "c. 1944 — based on archival arrangement context" .
```

## Consequences

### Positive

- **Computational queryability preserved at all precision levels** — `xsd:gYearMonth` still allows year and month range queries
- **Aligns with W3C XSD recommendation** — these are standard XSD types, not NAPH-specific inventions
- **Query-friendly** — SPARQL engines understand these types and can range-query across them
- **Lossless precision** — no rounding "1944" to a fake "1944-01-01" with 100% loss of precision information

### Negative

- **More complex querying** — researchers must handle three datatypes when range-querying
- **Some engines mishandle gYearMonth/gYear** — older or simpler engines may not range-query correctly
- **Pipeline complexity** — the ingest pipeline must detect partial dates and emit the right datatype

### Neutral

- Tier compliance is uniform: any precision is acceptable at Baseline as long as the type is correct.

## Alternatives considered

### Alternative 1: Mandate full xsd:date only

Rejected because:

- Forces fake precision — "March 1944" → "1944-03-01" implies a day-level claim that's not in the source data
- Loses ~5-15% of records that genuinely don't have day-level precision

### Alternative 2: Allow free-text dates

Rejected because:

- Defeats the purpose of NAPH — computation-readiness requires structured fields
- Range queries don't work on free-text
- Aggregators can't process free-text dates

### Alternative 3: Use Edinburgh Time Format or similar academic ontology

Considered briefly. Some heritage standards use complex temporal ontologies (CIDOC-CRM E52 Time-Span, etc.).

Rejected because:

- These are over-engineered for aerial photography use cases
- Most queries are range-based, which XSD types handle natively
- Reduces interoperability with non-heritage tools

### Alternative 4: Two types only (xsd:date + xsd:gYear)

Considered. Reasonable — the gYearMonth type sees less use.

Rejected because:

- "March 1944" is a common precision in heritage data
- Forcing it to either day-level or year-level loses information
- Three types is not significantly more complex than two

## Implementation

The SHACL shape `naph:BaselineShape` validates:

```turtle
sh:property [
    sh:path naph:capturedOn ;
    sh:minCount 1 ;
    sh:maxCount 1 ;
    sh:or (
        [ sh:datatype xsd:date ]
        [ sh:datatype xsd:gYearMonth ]
        [ sh:datatype xsd:gYear ]
    ) ;
    sh:message "Capture date must be xsd:date, xsd:gYearMonth, or xsd:gYear — no free-text."
] ;
```

(Note: this requires SHACL `sh:or` support — not all SHACL engines implement this; Apache Jena does. Where `sh:or` is unavailable, the validation can be split into multiple shapes.)

## Querying across precisions

To query across all precisions in a date range:

```sparql
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>

SELECT ?photo
WHERE {
    ?photo naph:capturedOn ?date .
    FILTER (
        (DATATYPE(?date) = xsd:date && ?date >= "1939-09-01"^^xsd:date && ?date <= "1945-09-02"^^xsd:date) ||
        (DATATYPE(?date) = xsd:gYearMonth && ?date >= "1939-09"^^xsd:gYearMonth && ?date <= "1945-09"^^xsd:gYearMonth) ||
        (DATATYPE(?date) = xsd:gYear && ?date >= "1939"^^xsd:gYear && ?date <= "1945"^^xsd:gYear)
    )
}
```

Most SPARQL engines also allow cross-type comparison via STR() and SUBSTR() — see the [SPARQL library temporal queries](../../04-adoption-guidance/sparql-library/temporal.sparql) for examples.

## Validation

The decision is validated by:

- Sample data exercises the precision policy (most records have `xsd:date`, partial-date support is documented)
- The competency-question CQ1 returns correct results across mixed-precision data
- The XSD types are W3C-standard and supported by all major triple stores

## Cross-references

- [Module B §B.4 — Date handling](../../01-standard/modules/B-metadata-data-structures.md#b4-date-handling)
- [Date Normalisation Decision Tree](../../04-adoption-guidance/decision-trees/date-normalisation.md)
- [W3C XSD 1.1 — Date types](https://www.w3.org/TR/xmlschema11-2/#dateTime)
