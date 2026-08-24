# Gap Analysis - Current NCAP Metadata vs Computation-Readiness

This document examines the gap between what NCAP currently exposes through Air Photo Finder and what the **NAPH Baseline tier** requires.

> **Update (verified against the live API).** The table below was originally
> reasoned from the *visible website*. We have since harvested a real 300-record
> sample from NCAP's public `/api/v1` and measured what the payload actually
> contains. The result is materially more optimistic than the amber/red marks
> below suggested: machine-readable footprints and ISO-8601 dates are present for
> 100% of records. The corrected, evidence-based picture is in
> [`empirical-api-findings.md`](empirical-api-findings.md). The rows below are
> annotated **[measured: …]** where the live data overturned the original guess.

## What NCAP currently has

Based on the publicly visible Air Photo Finder interface, NCAP's published cataloguing approach (ISAD(G) archival standard), **and a measured sample of the public API** (see [empirical findings](empirical-api-findings.md)):

| Field | Current state | Computation-ready? |
|-------|--------------|--------------------|
| Sortie reference | Recorded | ✅ Structured |
| Frame number | Recorded | ✅ Structured |
| Date of capture | ISO-8601 in the API, with a `date_precision` flag | ✅ **[measured: 100% ISO 8601; day/year precision self-declared]** |
| Geographic footprint | WKT `POLYGON` in the API (EPSG:3857) | ✅ **[measured: present for 100%; needs reprojection to WGS84, not new data]** |
| Squadron / aircraft | Sometimes recorded | ⚠️ Free text, no controlled vocabulary |
| Rights statement | Implicit (licensing-based business model) | ❌ **[measured: absent from payload, the one genuine Baseline gap]** |
| Persistent identifier | `UNI` + ISAD(G) reference | ⚠️ **[measured: stable UNI on 100%, ISAD(G) on 86%; needs minting into a resolvable URI]** |
| Provenance chain | Documented in finding aids | ❌ Not linked to individual records |
| Digitisation metadata | Internal | ❌ Not exposed publicly |
| Subject / depicts | None | ❌ Not present |
| Cross-collection linking | None | ❌ Not present |

## The Baseline gap

To get from current public metadata to **NAPH Baseline tier**, the work required is:

1. **Date normalisation**: convert any free-text dates to ISO 8601, flag uncertain or partial dates explicitly (e.g. `"1944-03"` for known month, with a confidence annotation rather than `"March 1944"` as free text).
2. **Geographic footprint exposure**: the data already exists for cataloguing the map overlay. Exposing it as WKT polygons or GeoJSON requires a presentation-layer change, not new content.
3. **Rights statement linkage**: replace implicit "Crown Copyright" with explicit URI references to `rightsstatements.org` or the UK Government Licensing Framework. This is a one-time mapping per rights category.
4. **Persistent identifier policy**: adopt a w3id.org or equivalent stable identifier scheme, mapping current sortie+frame composites into URI form.
5. **Structured packaging**: bundle records with a manifest (BagIt, RO-Crate) for bulk research access.

**Estimated cost for Baseline retrofit:** moderate. Most of the data exists; the work is structural and policy-based, not new digitisation.

## The Enhanced gap

To go from Baseline to **Enhanced tier**:

1. **Digitisation metadata exposure**: already recorded internally (resolution, format, scan date, operator). Needs to be published as part of the public record, not just held in internal systems.
2. **Capture context structuring**: flight altitude, camera type, lens specification often exist in flight logs but aren't connected to individual frames. Linking requires cross-referencing the frame catalogue against scanned flight logs (potentially AI-assistable for legible logs).
3. **Provenance chain documentation**: existing finding aids describe transfer history at collection level. Enhanced tier requires connecting this to individual frames, which can be done at the sortie level for most material.

**Estimated cost for Enhanced retrofit:** higher. Requires exposing internal data and structuring known but unlinked metadata.

## The Aspirational gap

To go from Enhanced to **Aspirational tier**:

1. **Subject classification**: what each photograph depicts (places, structures, events). At 30 million images, manual tagging is impossible. The viable approach:
   - Use existing geographic footprints to auto-suggest place candidates (every photo over Edinburgh implicitly depicts named monuments within its footprint).
   - Use vision-language models on a sample to extract dominant features (urban / rural, specific landmarks, terrain type), treated as drafts requiring human validation at quality gates.
   - Prioritise high-research-value subsets first.
2. **Place authority linking**: once subject classification is in place, link to GeoNames / Wikidata / Pleiades / OS Open Names. Largely automatable for unambiguous cases.
3. **Cross-collection record matching**: connect photographs to monument records (Canmore, Historic England), other archive items (IWM, NARA, RAF Museum), and event records (Wikidata historic events).
4. **Computational vision features**: extracted feature vectors enable image similarity search, change detection over time at fixed locations, and content-based retrieval.

**Estimated cost for Aspirational retrofit:** high per-record but scaleable through automation. Critical to specify outcome requirements rather than prescriptive workflows.

## Why this matters for NAPH

The NCAP collection illustrates the central insight of the Towards a National Collection / N-RICH Prototype:

> Most UK heritage data is **digitised but not computable**.

A photograph that is scanned, catalogued in ISAD-G, and presented on an interactive map is "digitised" by the metrics that have driven heritage funding for 20 years. But it is not **computable**: a researcher cannot bulk-query "all photographs over urban centres in 1944-1946" without manual browsing, cannot run image-similarity searches, cannot cross-reference to other collections programmatically.

The Pilot's standard must close this gap not by demanding institutions start over, but by structuring what they already have so that the next 20 years of research can actually use it.

## What this case study contributes

- A concrete example of what the standard could look like in TTL/SHACL form
- A demonstration that the three tiers are achievable incrementally
- An ontology that other collection types can adapt (the same structure works for satellite imagery, historic mapping, photographic archives more broadly)
- A reference implementation that uses existing W3C and OGC standards rather than inventing new ones
