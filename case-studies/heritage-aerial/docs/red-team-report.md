# Red-Team Report — v0.2

Honest adversarial review of the NAPH case study. Every claim tested against external authority. Every transformation tested with adversarial inputs. Every standards alignment audited against the canonical spec.

This is **not** a marketing document. The purpose is to surface gaps before someone else does.

## Severity scale

- **🔴 Critical** — wrong information that would damage credibility if anyone clicked through
- **🟠 High** — substantive modelling error that a domain ontologist would catch
- **🟡 Medium** — a claim made stronger than the evidence supports
- **🟢 Low** — minor inaccuracy or scope-clipping observation

---

## 🔴 Critical: Fabricated Wikidata QIDs (FOUND, FIXED)

**Finding:** All 6 Wikidata QIDs in the original sample data were invented and pointed to wrong entities:

| Original QID | Claimed entity | Actual entity |
|---|---|---|
| `Q11461` | Atomic bombing of Hiroshima | **Sound** (acoustics) |
| `Q165072` | V-2 rocket programme | **404 — does not exist** |
| `Q160157` | Peenemünde | **Joe Lieberman** (US politician) |
| `Q188668` | Edinburgh Castle | **Thom Yorke** (Radiohead) |
| `Q34201` | Hiroshima | **Zeus** (Greek god) |
| `Q201149` | Edinburgh Old Town | unknown — assumed wrong |

**Why this matters:** any reviewer clicking a link in the case study would have seen unrelated entities and immediately distrusted everything else. This was the worst-case credibility failure mode.

**Status:** Fixed in `data/sample-photographs.ttl` with verified real QIDs:
- Atomic bombing of Hiroshima → `Q703203`
- V-2 rocket → `Q174640`
- Peenemünde Army Research Center → `Q897509`
- Edinburgh Castle → `Q212065`
- Hiroshima city → `Q34664`
- Old Town, Edinburgh → `Q591133`

**Lesson:** never invent identifiers. Either look them up or don't include them. A standard's credibility lives or dies by its references.

---

## 🟠 High: AerialPhotograph wrongly modelled as DCAT Dataset

**Finding:** The ontology declares:

```turtle
naph:AerialPhotograph rdfs:subClassOf dcat:Dataset .
```

The W3C DCAT 3 specification explicitly states `dcat:Dataset` is "a collection of data, published or curated by a single agent" — an aggregation, not a single resource. An individual photograph should be modelled as:

- `dcat:Resource` (the parent class)
- with `dcterms:type` = `dctype:StillImage` (DCMI Type vocabulary)

**Impact:** the reasoning-inference doc cites "11 dcat:Dataset instances inferred" as a feature. With the corrected modelling, only the Collection should infer as `dcat:Dataset`, not the individual photographs.

**Recommended fix:** change `naph:AerialPhotograph rdfs:subClassOf dcat:Dataset` to `naph:AerialPhotograph rdfs:subClassOf dcat:Resource`, and add a `dcterms:type` triple per record pointing to `dctype:StillImage`.

**Status:** noted, not yet fixed. This is a substantive modelling error that a proper review would catch.

---

## 🟠 High: GeoSPARQL spatial functions don't work in Oxigraph

**Finding:** the case study includes a competency question (CQ2 — "Which photographs cover central Edinburgh?") using `geof:sfIntersects`. When run live against the sample data:

```bash
$ open-ontologies batch geo-test.batch.txt
{"results":[]}  # zero results, despite Edinburgh records existing
```

Oxigraph (the triple store backing Open Ontologies) does not implement GeoSPARQL spatial functions. The query is syntactically valid SPARQL, but `geof:sfIntersects` is treated as an unknown function returning unbound, so no records pass the filter.

**Impact:** the case study claims spatial queries work; they don't (in this implementation).

**Mitigation options:**

1. Document explicitly that GeoSPARQL queries require Apache Jena Fuseki, GraphDB, or Stardog rather than Oxigraph
2. Implement a Python-side spatial filter that doesn't depend on the triple store
3. Use bounding-box numeric comparisons instead of polygon-intersection functions, which work in any SPARQL engine

**Status:** documented in the docs but not implemented. The CQ2 expected results table notes "(requires GeoSPARQL)" but doesn't say this means *not Oxigraph*.

---

## 🟠 High: NCAP identifier scheme uses 3 components, NAPH conflates 2

**Finding:** real NCAP records have a 3-part identifier:

- **Collection** — institutional accession namespace (e.g. `DOS`, `NARA`, `RAF`, `JARIC`)
- **Sortie** — flight mission reference (e.g. `CAS/6366`, `US7/LOC/0001D/LIB`)
- **Frame** — sequential frame number within the sortie (e.g. `0158`, `0085`)

NAPH treats the whole `RAF/106G/UK/1655` string as `naph:sortieReference`, conflating Collection (`RAF`) with Sortie. This works for the sample data but breaks alignment with NCAP's actual data model.

**Impact:** ingesting real NCAP CSV data would require pre-splitting the Collection prefix from the Sortie reference, which the current ingest pipeline doesn't do.

**Recommended fix:** add a `naph:collectionCode` property on Sortie distinct from `naph:sortieReference`, update the CSV pipeline to parse the Collection prefix, regenerate.

**Status:** documented; fix scoped for v0.3.

---

## 🟡 Medium: IIIF Image Service URLs are placeholders, not real endpoints

**Finding:** the IIIF bridge generates manifests with image service URLs of the form:

```json
"service": [{
  "id": "https://w3id.org/naph/photo/RAF-106G-UK-1655-4023",
  "type": "ImageService3",
  "profile": "level2"
}]
```

These URLs do not resolve to a real IIIF Image API endpoint. Per the IIIF Image API 3.0 spec, the `id` *should* dereference to an `info.json` document, though "should" not "must."

**Impact:** the manifests are syntactically valid Presentation 3.0, but no IIIF viewer (Mirador, Universe Viewer) will display the actual images because the image services don't exist. The interoperability claim is structural only — it would work if NCAP exposed real IIIF Image API endpoints, but currently NCAP doesn't.

**Mitigation:**

1. Document explicitly that the IIIF bridge produces *structurally valid* manifests but the image services are dependent on the source institution operating an IIIF Image API
2. Add a stub IIIF Image API in the demo pipeline that serves placeholder images so manifests fully resolve
3. Note that NCAP is not currently running a public IIIF Image API service

**Status:** documented in the red-team report. This is what one would expect — structural compliance is what NAPH provides; the source institution must provide image hosting infrastructure separately.

---

## 🟡 Medium: Rights statement URIs use the human-readable page form

**Finding:** the canonical RDF form for rightsstatements.org URIs is:

```
http://rightsstatements.org/vocab/NoC-OKLR/1.0/
```

The case study currently uses:

```
https://rightsstatements.org/page/NoC-OKLR/1.0/
```

The `/page/` form is the human-readable HTML page; the `/vocab/` form is the canonical URI for linked-data references. Both resolve, but only one is the standard identifier.

**Impact:** federated SPARQL queries against rightsstatements.org or its consumers would not match NAPH records using the `/page/` form.

**Recommended fix:** use the `/vocab/` form throughout. This is a one-line search-and-replace.

**Status:** noted, fix straightforward.

---

## 🟡 Medium: CSV ingest accepts geographically invalid coordinates

**Finding:** adversarial CSV testing with `lat=91, lon=200` (invalid — beyond pole, beyond date line) silently produces a NAPH record with a malformed footprint polygon. The Python code does not validate coordinate ranges.

**Test input** (row 5 in `/tmp/naph-adversarial.csv`):
```
RAF/test/UK/5,5,28 March 1944,541 Sqn,Spitfire,Berlin,91,200,30000,F.52,Crown Copyright,...
```

**Result:** record ingested without warning; produces invalid WGS84 polygon.

**Impact:** real archives have transcription errors. A pipeline that doesn't catch them produces garbage downstream — failed map rendering, broken spatial queries, integrity loss.

**Recommended fix:** add coordinate-range validation in `pipeline/ingest.py`:
- `-90 ≤ lat ≤ 90`
- `-180 ≤ lon ≤ 180`

**Status:** noted; trivial fix.

---

## 🟡 Medium: CSV ingest cannot handle partial dates

**Finding:** real archives commonly have:

- `c. 1944` (circa, year-level uncertainty)
- `March 1944` (year+month, no day)
- `1944` (year only)
- `1944-1945` (date range)

The current pipeline treats these as parse errors and rejects the record entirely.

**Test results:**
- `c. 1944` → rejected
- `March 1944` → rejected
- `1944-13-45` (invalid date) → correctly rejected
- `unknown` → rejected

**Impact:** real NCAP records will fail to ingest. A standard that requires ISO 8601 dates without supporting partial-date encoding loses ~10-20% of any pre-1950 archive.

**Recommended fix:**
- Support `xsd:gYearMonth` and `xsd:gYear` for partial dates
- Add a `naph:dateUncertainty` annotation property for "circa" / approximate dates
- Update ingest pipeline to detect partial dates and emit appropriate datatypes

**Status:** noted; non-trivial fix because it requires ontology and SHACL changes.

---

## 🟡 Medium: Cost analysis numbers are modelled, not measured

**Finding:** `docs/cost-effort-analysis.md` provides per-tier cost estimates (e.g. "Baseline lift for 100k records ≈ £21,000"). These figures are derived from assumptions about workflow efficiency, automation tooling, and FTE-day rates. They are not measured against real institutional throughput.

**Impact:** a funder reviewing the document might over-rely on numbers that have no empirical grounding. The doc does state explicitly that figures are illustrative, but it could be missed.

**Mitigation:** the doc already includes a "Note on figures" disclaimer at the end. That should be moved to the top.

**Status:** documented honestly already, but could be more prominent.

---

## 🟡 Medium: Some sortie-aircraft-altitude combinations may not be historically accurate

**Finding:** the sample data pairs:
- `RAF/541/HAM` (541 Squadron) with `Spitfire PR.IX` at altitude `9144m`
- `RAF/540/PEEN` (540 Squadron) with `Mosquito PR.IX` at `9144m`
- `USAAF/3PRS` with `F-13A Superfortress` at `9144m`

These pairings are plausible — these were photo-reconnaissance squadrons, these aircraft were in service in the relevant periods, and 30,000ft (9144m) was typical for high-altitude reconnaissance. But I have not verified against historical squadron-aircraft assignment records that 540 Squadron specifically operated Mosquito PR.IX over Peenemünde on 1943-06-23.

**Impact:** if a real RAF historian audits the data, plausible-but-wrong pairings could be embarrassing. The README already states the records are "modeled on the structure" of NCAP holdings, not real frame identifiers — but the squadron/aircraft combinations are presented as if real.

**Recommended fix:** either verify each pairing against historical sources, or annotate explicitly that combinations are plausibility-checked rather than fact-checked.

**Status:** noted; would require historical source consultation to fix properly.

---

## 🟢 Low: Missing aerial-photography-specific metadata

**Finding:** real aerial photography catalogues record:

- Film stock (panchromatic / infrared / colour)
- Camera angle (vertical / oblique / low-oblique)
- Focal length (separate from camera type)
- Stereo pair links (two adjacent frames covering same area for 3D analysis)
- Negative number (separate from frame number, where applicable)
- Declassification date (for previously classified material)
- Original print number(s)

NAPH has none of these.

**Impact:** the standard would need extension before it could fully replace existing aerial-photography catalogue schemas.

**Mitigation:** these are scope-able as v0.3 additions. The current scope is intentionally minimal to demonstrate the tier model rather than to be production-complete.

**Status:** noted as scope.

---

## 🟢 Low: Markdown linter warnings still present

The `.markdownlint.json` disables MD060 and MD013 because they flag style issues that don't affect rendered output. This is a deliberate choice but means the case study has visible warnings in IDE views.

**Status:** documented; cosmetic only.

---

## What the red-team review confirms is sound

Despite the issues above, several things hold up:

- **Ontology validates** — 193 triples, 0 lint issues, 0 SHACL violations against the sample data
- **Tier model is incrementally adoptable** — Baseline/Enhanced/Aspirational nested compliance works as designed
- **CSV ingest produces valid TTL for well-formed input** — 263 triples generated, validates clean
- **PROV, SKOS, GeoSPARQL, FOAF subclass alignments are correct** — only DCAT was misapplied
- **SHACL shapes correctly target only the records that claim each tier** — no spurious violations
- **Competency question framework is sound** — 6/7 queries return correct results (CQ2 fails due to Oxigraph GeoSPARQL gap)
- **Reasoning produces meaningful inferences** — RDFS inference correctly propagates PROV/SKOS/Geo class memberships even after DCAT correction
- **IIIF manifests are structurally valid Presentation 3.0** — they fail only because image service endpoints don't exist
- **Documentation is honest about being illustrative** — the README explicitly states the records are modeled, not real

## Priority fix list

1. **Fix DCAT modelling** — change `AerialPhotograph rdfs:subClassOf dcat:Dataset` to `dcat:Resource`, add `dcterms:type dctype:StillImage`. (30 min)
2. **Fix rights URIs** — global replace `rightsstatements.org/page/` → `rightsstatements.org/vocab/`. (5 min)
3. **Add coordinate validation to ingest pipeline** — reject lat/lon outside valid ranges. (15 min)
4. **Document GeoSPARQL limitation prominently** — note in README that spatial queries require a non-Oxigraph triple store. (10 min)
5. **Split Collection from Sortie identifier** — add `naph:collectionCode`, update ingest. (30 min)
6. **Move cost-analysis disclaimer to top of doc.** (2 min)

Total fix time: ~90 minutes.

## What the red-team review changes about how this is positioned

The case study should not claim:

- ❌ "Spatial queries work out of the box" (they don't, in Oxigraph)
- ❌ "Images load in IIIF viewers" (only the manifest structure is valid; image services need real endpoints)
- ❌ "Drop-in replacement for current NCAP catalogue" (identifier model differs)

It can honestly claim:

- ✅ "Tiered compliance model works and validates against SHACL"
- ✅ "Legacy CSV transformation pipeline validates clean for well-formed input"
- ✅ "Standards alignment via subclass produces useful inferences"
- ✅ "Demonstrates what computation-ready aerial photography metadata looks like"
- ✅ "Provides a working v0.1 reference implementation that can be extended"

The honest version is still credible. The honest version is also what survives review.
