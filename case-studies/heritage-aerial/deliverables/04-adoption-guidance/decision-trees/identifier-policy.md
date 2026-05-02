# Identifier Policy Decision Tree

A practical decision tree for choosing an identifier scheme for a NAPH-compliant collection. The choice you make here is **load-bearing**: identifiers persist for decades and changing them after publication is expensive.

## The three requirements

Whatever scheme you choose, identifiers MUST satisfy:

1. **Persistent** — once assigned, never reassigned, never silently changed
2. **Resolvable** — when dereferenced via HTTP(S), returns a representation
3. **Globally unique** — no collision with any other heritage organisation's identifiers

## Q1: Do you have an existing institutional URL prefix that you can guarantee will outlive any specific website redesign?

→ Yes: **go to Q2** (use it)
→ No: **go to Q3** (use a permanent URL service)

---

## Q2: Have you already minted identifiers in a structured pattern?

### Q2.1: Yes, and they're resolvable URIs

Use them. Wrap them in your existing URL prefix.

```turtle
ex:photo-X naph:hasIdentifier "https://yourinst.org/collection/abc/123" .
```

### Q2.2: Yes, but they're internal codes (not URIs)

Mint URIs by prefixing your existing codes with your URL namespace:

```
Existing: RAF/106G/UK/1655/4023
NAPH URI: https://yourinst.org/heritage/RAF-106G-UK-1655-4023
```

Document the mapping in your institutional identifier policy and never deviate.

### Q2.3: No, you're starting fresh

**go to Q4**

---

## Q3: Use a permanent URL service

The institution lacks a stable URL prefix. Use a permanent URL service that decouples identifier persistence from your website's stability.

| Service | Best for |
|---|---|
| [w3id.org](https://w3id.org/) | Free, GitHub-managed, recommended for ontologies and small/medium collections |
| [purl.org](https://purl.org/) | Free, OCLC-managed, established |
| [DOI](https://www.doi.org/) | Per-item identifiers, paid, citation-ready |
| [Handle](https://www.handle.net/) | Institutional Handle prefix, sustainable |

For most UK heritage institutions: **w3id.org** is the right answer. It's free, has a community-maintained PR-based update process, and gives you `https://w3id.org/<your-name>/...` permanently.

**To set up w3id.org:**

1. Fork [github.com/perma-id/w3id.org](https://github.com/perma-id/w3id.org)
2. Add a directory for your namespace (e.g. `ncap/`)
3. Add a `.htaccess` file with redirection rules to your current website
4. Open a PR

This gives you a permanent URI that you control via PRs even if your website moves.

---

## Q4: Designing a fresh identifier scheme

### Q4.1: What's the most granular thing a researcher will query?

→ Individual photograph / record: identifier per record
→ Sortie / acquisition group: identifier per group, sub-identifier per record
→ Both: hierarchical identifiers

### Q4.2: Pick a pattern

Recommended patterns by collection type:

#### Photographic, sortie-based (NCAP-style)

```
{namespace}/photo/{collection-code}-{sortie-ref}-{frame-no}

Example:
https://w3id.org/naph/photo/RAF-106G-UK-1655-4023
```

#### Photographic, accession-based

```
{namespace}/photo/{accession-year}/{accession-no}/{frame-no}

Example:
https://w3id.org/example/photo/1998/12345/0042
```

#### Manuscripts, archival hierarchy

```
{namespace}/{fonds}/{series}/{file}/{item}

Example:
https://w3id.org/example/papers-J-Smith/correspondence/1923/letter-014
```

#### Thematic collection

```
{namespace}/{collection-code}/{accession}/{item-id}

Example:
https://w3id.org/example/civil-rights-1968/2010-acquisitions/photo-march-001
```

### Q4.3: Pattern rules

- **Use only ASCII alphanumerics, hyphens, and forward slashes.** No spaces, no diacritics, no encoding-dependent characters.
- **Use stable codes.** A collection name might change ("Industrial Heritage Collection" → "Industrial Archive"). The identifier MUST NOT.
- **Avoid encoding mutable facts.** Don't put the date of the most recent edit in the identifier. Don't include the cataloguer's name.
- **Do encode immutable structural information.** Sortie reference, frame number, archival fonds — these are stable.

### Q4.4: Test your scheme before committing

1. Mint 10 identifiers across your collection's variety
2. Check: does each identifier resolve uniquely? (no collisions)
3. Check: would these identifiers still make sense 30 years from now?
4. Check: can you bulk-generate them programmatically from your existing catalogue?

If yes to all four → commit to the scheme and document it in your institutional identifier policy.

---

## Q5: Migration from existing identifiers

If you already have identifiers and need to migrate to a NAPH-compliant scheme:

### Q5.1: Your existing identifiers are URIs

Keep them. Just ensure they resolve to RDF (per [Module D](../../01-standard/modules/D-packaging-publication.md)).

### Q5.2: Your existing identifiers are internal codes

Migration plan:

1. Map each existing code to a NAPH-compliant URI
2. Publish the mapping table (`old-code,new-uri.csv`) at a stable URL
3. Set up HTTP redirects from any old URLs to new URIs
4. Update internal systems progressively
5. Deprecate the old code (continue serving it but flag as legacy)

This is a one-time migration cost. After it, the new URIs are durable.

### Q5.3: Your existing identifiers are reused (collision)

E.g. two records have the same code because they came from different acquisitions. This is a data-quality issue that must be resolved before NAPH adoption.

Options:

- Add a disambiguator (acquisition year prefix)
- Mint genuinely unique URIs and link both legacy codes to the appropriate URIs
- Manually de-duplicate

---

## Q6: Special cases

### Q6.1: Records that may be later split or merged

If a record might later be split (e.g. an archival item is found to comprise multiple distinct items) or merged (e.g. duplicate accessions reconciled):

- Mint NEW URIs for the resulting records
- Mark the old URI as `owl:deprecated` and link to the successor(s)
- Never reuse the old URI for a different record

```turtle
ex:photo-001-deprecated owl:deprecated true ;
    rdfs:comment "Split into photo-001a and photo-001b on 2025-01-15" ;
    dcterms:isReplacedBy ex:photo-001a, ex:photo-001b .
```

### Q6.2: Records with restricted access

Even if the record's content is restricted (e.g. sensitive material, in copyright), the **identifier itself** should remain resolvable, returning at least metadata and a rights/access notice.

```turtle
ex:photo-restricted naph:hasIdentifier "https://yourinst.org/photo/X" ;
    naph:hasRightsStatement ex:in-copyright ;
    naph:accessRestrictions "Onsite only — no digital surrogate available" .
```

### Q6.3: Born-digital with vendor-assigned identifiers

For born-digital material with identifiers assigned by a vendor system:

- Wrap the vendor identifier in your namespace
- Don't expose vendor URLs as canonical identifiers (the vendor may go out of business)

---

## After choosing a scheme

Document it. The [identifier policy template](../../07-templates/identifier-policy-template.md) is a 1-page document covering:

- The chosen scheme and pattern
- Authority for assignment (who mints)
- Persistence guarantee
- Resolution mechanism
- Versioning and deprecation policy

Publish this document at a stable URL. Reference it in your collection's manifest.

## Cross-references

- [Module B — Metadata & Data Structures](../../01-standard/modules/B-metadata-data-structures.md) §B.3
- [Module D — Packaging & Publication](../../01-standard/modules/D-packaging-publication.md)
- [w3id.org service](https://w3id.org/)
- [Identifier policy template](../../07-templates/identifier-policy-template.md)
