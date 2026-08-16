# Empirical Findings - How Close NCAP Already Is to Computation-Readiness

*A good-faith interoperability demonstration in support of NCAP's own
computation-ready goals. Measured against a small sample of 300 catalogue records
read from NCAP's public Air Photo Finder search interface on 2 July 2026. Metadata
only; no image binaries; read-only public endpoints; rate-limited. Rights in the
images and catalogue remain with NCAP / Historic Environment Scotland. See the
[good-faith note](#provenance-and-good-faith-note) and
[`pipeline/scrapers/ncap_airphotofinder.py`](../pipeline/scrapers/ncap_airphotofinder.py).*

The headline of this document is a compliment to NCAP, not a critique: **the hard
part is already done.** This supersedes the speculative parts of
[`gap-analysis.md`](gap-analysis.md), which reasoned from the *visible website* and
was too pessimistic. Reading the actual catalogue payload shows NCAP is much closer
to Baseline computation-readiness than a browser suggests: the computational
substrate is already in the data; it simply is not yet surfaced in a
standards-aligned form. NAPH is the thin, automatable layer that closes that last gap.

## Headline

> Every catalogue record already carries a machine-readable footprint polygon, an
> ISO-8601 date with an explicit precision flag, and a stable identifier. The
> residual work to reach NAPH **Baseline** is a coordinate reprojection, a rights-URI
> mapping, and URI minting, all automatable, and all demonstrated here on 292 real
> frames in a single script. This is a "you are 90% of the way there" finding.

## What we measured (n = 300)

| Field | Present | Notes |
|---|---:|---|
| Machine-readable footprint (`image_coordinates`, WKT `POLYGON`) | **300 / 300 (100%)** | In **EPSG:3857** (Web Mercator), not WGS84 (the one catch). |
| ISO-8601 capture date (`date`) | **300 / 300 (100%)** | Already `YYYY-MM-DD`. Not free-text. |
| Explicit date precision (`date_precision`) | **300 / 300 (100%)** | `day` 225, `year` 75; the API *self-declares* granularity. |
| Stable unique identifier (`UNI`) | **300 / 300 (100%)** | e.g. `000-000-357-415`. Not yet a resolvable URI. |
| Archival reference (`ISAD(G)`) | **258 / 300 (86%)** | e.g. `GB 551 NCAP/20-1-2-2`. |
| Image perspective (`image_type`) | **300 / 300 (100%)** | `vertical` 245, `oblique` 55. |
| Sortie + frame | **300 / 300 (100%)** | e.g. `PEGASUS/RN/H/0007` frame `0001`. |
| Collection context | **300 / 300 (100%)** | See distribution below. |
| Machine-readable **rights** | **0 / 300 (0%)** | **The genuine Baseline gap.** No rights field in the payload. |
| Subject / depicts, place authorities, cross-links | **0 / 300** | Expected: these are Enhanced / Aspirational concerns. |

Collection spread in the sample: Directorate of Overseas Surveys (88),
Defence Geographic Centre (75), Allied Central Interpretation Unit (75),
US National Archives and Records Administration (55), RCAHMW (7). The frames span
**1924–1956** and range from Hong Kong (a 1924 HMS *Pegasus* naval sortie) to the
Caribbean: genuine global reach, in one un-curated pull.

## Correcting the gap analysis

The original [`gap-analysis.md`](gap-analysis.md) marked two fields with a red ❌ or
amber ⚠️ that the API data shows are, in fact, green:

- **"Geographic footprint … not exposed as machine-readable WKT/GeoJSON"**: *incorrect.*
  It is exposed, as a WKT `POLYGON`, for 100% of records. The only issue is the CRS:
  it is delivered in EPSG:3857 rather than a geographic CRS. Reprojection to WGS84 is
  a closed-form transform (implemented in the harvester in ~6 lines) and is lossless
  for footprint purposes.
- **"Date of capture … often parseable but not guaranteed ISO 8601"**: *pessimistic.*
  Every record in the sample is already ISO-8601, and the API additionally publishes a
  `date_precision` enum that maps one-to-one onto the NAPH date-precision policy
  (ADR-0009): `day → xsd:date`, `month → xsd:gYearMonth`, `year → xsd:gYear`.

So the **true** Baseline gap for NCAP is narrow and precise:

1. **Reproject** `image_coordinates` EPSG:3857 → WGS84 and publish as GeoSPARQL `geo:asWKT`.
2. **Mint** a resolvable URI per frame (the `UNI` is a perfect stable key to hang it on).
3. **Attach a machine-readable rights statement.** This is the single field genuinely
   *absent* from the payload, and the only one requiring a policy decision rather than a
   transformation. Because NCAP's business model is licensing, rights are exactly the
   field worth getting right first.

Everything else Baseline needs is already in the data.

## The standard improved on contact with real data

Running the real sample through the NAPH SHACL shapes surfaced a genuine defect in
the *standard*, not the data. 75 of the 300 frames carry **year-precision** dates
(`xsd:gYear`). The Baseline shape had hard-coded `sh:datatype xsd:date`, so it
rejected them, even though the standard's own date-precision policy (ADR-0009) and
its CSV ingest pipeline both allow day/month/year precision. The synthetic sample had
never exercised a year-only date, so the contradiction had gone unnoticed.

The shape was corrected to admit all three precisions via `sh:or`
(see [`ontology/naph-shapes.ttl`](../ontology/naph-shapes.ttl)). After the fix, all
292 distinct real frames validate with **0 violations**. This is the case for testing
standards against real holdings rather than tidy exemplars: the real data found the bug.

## Reproduce it

```bash
# Harvest a fresh stratified sample (respectful, rate-limited, metadata only)
python3 pipeline/scrapers/ncap_airphotofinder.py --limit 300 \
    --raw-out pipeline/real-ncap-raw.json > data/real-ncap-sample.ttl

# Validate real data against the standard
open-ontologies validate data/real-ncap-sample.ttl
printf 'clear\nload ontology/naph-core.ttl\nload data/real-ncap-sample.ttl\nshacl ontology/naph-shapes.ttl\n' \
    | open-ontologies batch          # -> conforms: true, violations: 0

# Build shareable artefacts (IIIF collection + GeoJSON) through the triple store
python3 pipeline/build-real-artefacts.py
```

## Provenance and good-faith note

This harvest touches only public, read-only search endpoints: the same ones the Air
Photo Finder website calls to render its own map. It fetches metadata only: no image
binaries, no ordering/basket/account endpoints. It is rate-limited and identifies
itself in the User-Agent. It is a good-faith interoperability demonstration in support
of NCAP's own computation-readiness goals, not a bulk-extraction exercise. Rights in
the underlying images and catalogue remain with NCAP / Historic Environment Scotland;
please respect NCAP's terms of website use and licensing.
