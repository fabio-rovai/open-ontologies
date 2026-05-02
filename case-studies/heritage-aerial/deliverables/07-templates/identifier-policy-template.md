# [Institution] — Identifier Policy

**Status:** Adopted YYYY-MM-DD
**Owner:** [Name, Role]
**Review cycle:** every 3 years
**Standard alignment:** NAPH v1.0, [Module B](https://w3id.org/naph/ontology) §B.3

This document is the institutional identifier policy for [Institution]'s aerial photography heritage holdings, asserted as part of NAPH compliance. Copy this template, adapt the bracketed sections, and publish at a stable URL.

## 1. Scope

This policy applies to all records published as NAPH-compliant under the [Institution] namespace.

It does NOT apply to:

- Internal cataloguing identifiers (those continue under existing institutional practice)
- Vendor-system identifiers (e.g. KE EMu accession numbers)
- Identifiers asserted by partner institutions

## 2. Identifier scheme

[Institution] mints identifiers under the namespace:

```
https://w3id.org/[institution-slug]/
```

OR

```
https://[institutional-permanent-domain]/
```

[Choose one and document the rationale here. Most institutions should use w3id.org.]

### 2.1 Pattern

The canonical pattern for individual records is:

```
https://[namespace]/photo/{collection-code}-{sortie-slug}-{frame-number}
```

Examples:

```
https://w3id.org/[institution-slug]/photo/RAF-106G-UK-1655-4023
https://w3id.org/[institution-slug]/photo/USAF-91SRS-KOR-1951-04-0892
https://w3id.org/[institution-slug]/photo/HEXAGON-1117-1-DA-0037
```

### 2.2 Slug formation rules

When converting a sortie reference to a URI-safe slug:

- Replace `/` with `-`
- Remove all non-alphanumeric, non-hyphen characters
- Preserve case as in the original sortie reference
- Do not collapse repeated hyphens

Example:

```
Original: RAF/106G/UK/1655 frame 4023
Slug:     RAF-106G-UK-1655-4023
URI:      https://w3id.org/[ns]/photo/RAF-106G-UK-1655-4023
```

### 2.3 Reserved characters

The following characters MUST NOT appear in slugs:

- spaces (replace with `-`)
- `/`, `\`, `?`, `#`, `&`, `=`, `+`, `%` (URI-reserved characters)
- diacritics (transliterate to ASCII; e.g. `Peenemünde` → `Peenemuende`)

## 3. Persistence guarantee

[Institution] guarantees that:

- Once minted, an identifier WILL NEVER be reassigned to a different record
- An identifier WILL ALWAYS resolve, returning either:
  - The current record's representation, OR
  - A `301 Moved Permanently` redirect to the current canonical URI, OR
  - A `410 Gone` with metadata explaining withdrawal (rare; only for legal compulsion)

Withdrawal is documented in the record's provenance with `owl:deprecated true` and a `dcterms:isReplacedBy` link to any successor.

## 4. Resolution

When a NAPH identifier is dereferenced via HTTP(S):

- With `Accept: text/html` (browser): returns HTML representation
- With `Accept: text/turtle` or `Accept: application/ld+json`: returns RDF representation
- With no `Accept`: returns HTML by default
- With `Accept: application/json`: returns JSON-LD

Implementation: HTTP content negotiation handled by [Institution]'s web server / middleware.

## 5. Authority for assignment

Identifiers are minted by:

[Specify role(s)] — typically the digital officer, repository manager, or a designated cataloguer.

The minting process MUST:

- Check uniqueness against the existing identifier register before assignment
- Record the assignment in the institutional minting log
- Use only the canonical pattern from §2.1

## 6. Versioning and deprecation

If a record is split, merged, or withdrawn:

- Original identifiers are NEVER reassigned
- New identifiers are minted for resulting records
- Old identifiers retain `owl:deprecated true` and link to successor(s) via `dcterms:isReplacedBy`

Example:

```turtle
old:photo-001 owl:deprecated true ;
    rdfs:comment "Split into two records on 2025-01-15 after re-cataloguing" ;
    dcterms:isReplacedBy old:photo-001a, old:photo-001b .
```

## 7. Migration policy

If [Institution] migrates from this scheme to another:

- All existing identifiers MUST continue to resolve indefinitely
- HTTP redirects from old to new scheme MUST be in place
- Migration plan MUST be published at least 12 months before any deprecation

## 8. Compatibility with existing institutional identifiers

[Institution] also maintains existing internal identifiers (e.g. KE EMu accession numbers).

The mapping between internal and NAPH identifiers is published at:

```
https://[namespace]/identifier-mapping.csv
```

This mapping MUST be updated whenever identifiers are minted or revised.

## 9. Federated identification

NAPH identifiers in the [Institution] namespace are intended for:

- Citation in research publications
- Cross-collection linking from peer institutions
- Aggregator harvesting (Europeana, DPLA, national portals)
- Long-term archival reference

Researchers and institutions are encouraged to use these identifiers as canonical citations.

## 10. Contact

Questions, errata, or migration requests:

[Institution contact details]

## 11. References

- [NAPH Standard v1.0](https://w3id.org/naph/ontology) — Module B §B.3 Identifier requirements
- [w3id.org service](https://w3id.org/) — permanent URI service
- [Identifier policy decision tree](https://w3id.org/naph/ontology/decision-trees/identifier-policy)

## 12. Document history

- YYYY-MM-DD — initial policy adopted
