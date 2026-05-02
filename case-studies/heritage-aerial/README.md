# NAPH — Computation-Ready Digitisation Standard for Aerial Photography Heritage

A focused, vertical, openly-licensed digitisation standard for **aerial photography heritage collections**. Provides ontology, validation shapes, transformation pipeline, IIIF bridge, governance model, and full deliverables — the complete A-to-Z system designed for institutions like NCAP, IWM, NARA-partnered archives, and equivalent national aerial photography holdings worldwide.

Aligned with the goals of the **Towards a National Collection** programme (AHRC / UKRI) and the **N-RICH Prototype**. See [CHANGELOG](CHANGELOG.md).

## Why narrow

Generic GLAM-wide digitisation standards exist. Aerial photography heritage has distinctive characteristics — stereo pairs, ground sample distance, declassification provenance, sortie metadata — that benefit from a focused, deep treatment rather than a generic framework. NAPH is deliberately one vertical, done well. See [ADR-0001](deliverables/06-knowledge-transfer/architecture-decision-records/0001-narrow-vertical.md).

## Why this exists

The aerial photography heritage sector — NCAP, IWM, NARA, RAF Museum, RCAHMS, equivalent national institutions — holds collectively ~50 million records that are **digitised but not computable**. They sit on the web for human browsing but cannot be queried, aggregated, or analysed at scale by modern research tools.

NAPH is a focused vertical standard that closes that gap for aerial photography specifically, rather than chasing a generic GLAM-wide framework that has to compromise on every domain. Aerial photography has rich spatial, temporal, and event-linked context that becomes far more powerful when modelled as linked data — and that depth deserves a dedicated treatment.

This case study shows what a **3-tier digitisation standard** looks like when applied to a real-shaped heritage dataset — and what the gap is between current archive-cataloguing practice (ISAD-G, free-text fields, image-on-the-web) and computation-readiness.

## The three tiers

|Tier|Scope|Required fields|
|---|---|---|
| **Baseline** | Minimum viable computation-readiness | Stable identifier, ISO 8601 date, geographic footprint (WGS84 polygon), machine-readable rights statement, sortie reference, collection link |
| **Enhanced** | Supports research workflows | Baseline + digitisation provenance (date, resolution, format, operator), capture context (altitude, camera, squadron, aircraft), full provenance chain, multiple surrogate formats |
| **Aspirational** | Supports semantic discovery | Enhanced + subject classification, place authority links (GeoNames / Wikidata), cross-collection linked records, event linkage |

Each tier is **incrementally adoptable** — a collection at Baseline does not need to rebuild to reach Enhanced.

## The full deliverable set

The [`deliverables/`](deliverables/) directory contains the complete A-to-Z system covering the standard, governance, adoption guidance, and operational artefacts:

| Output | Location |
|---|---|
| **1. Standard v1.0** — formal specification | [`deliverables/01-standard/NAPH-STANDARD.md`](deliverables/01-standard/NAPH-STANDARD.md) |
| 6 module specifications (A: Capture, B: Metadata, C: Rights, D: Packaging, E: Paradata, F: QA) | [`deliverables/01-standard/modules/`](deliverables/01-standard/modules/) |
| Aerial Photography Profile (the single normative profile in v1.0) | [`deliverables/01-standard/profiles/aerial-photography.md`](deliverables/01-standard/profiles/aerial-photography.md) |
| 5 sub-profiles (reconnaissance, satellite, UAV, photogrammetric, aerial archaeology) | [`deliverables/01-standard/profiles/aerial-subprofiles/`](deliverables/01-standard/profiles/aerial-subprofiles/) |
| **2. Testing evidence** — partner clinic playbook | [`deliverables/05-governance/partner-clinic-playbook.md`](deliverables/05-governance/partner-clinic-playbook.md) |
| **3. Cost / capacity / skills analysis** | [`deliverables/03-cost-capacity-skills/`](deliverables/03-cost-capacity-skills/) |
| Investment case + skills map | [`investment-case.md`](deliverables/03-cost-capacity-skills/investment-case.md), [`skills-map.md`](deliverables/03-cost-capacity-skills/skills-map.md) |
| **4. Adoption guidance** | [`deliverables/04-adoption-guidance/`](deliverables/04-adoption-guidance/) |
| How to use this standard, validation checklists, FAQ | [`how-to-use-this-standard.md`](deliverables/04-adoption-guidance/how-to-use-this-standard.md), [`validation-checklists.md`](deliverables/04-adoption-guidance/validation-checklists.md), [`faq.md`](deliverables/04-adoption-guidance/faq.md) |
| Decision trees (rights, identifier, dates) | [`deliverables/04-adoption-guidance/decision-trees/`](deliverables/04-adoption-guidance/decision-trees/) |
| 4 step-by-step tutorials | [`deliverables/04-adoption-guidance/tutorials/`](deliverables/04-adoption-guidance/tutorials/) |
| Tier transition guides (B→E, E→A) | [`deliverables/04-adoption-guidance/transition-guides/`](deliverables/04-adoption-guidance/transition-guides/) |
| 40+ ready-to-use SPARQL queries | [`deliverables/04-adoption-guidance/sparql-library/`](deliverables/04-adoption-guidance/sparql-library/) |
| **5. Governance proposal** | [`deliverables/05-governance/governance-proposal.md`](deliverables/05-governance/governance-proposal.md) |
| RFC process | [`deliverables/05-governance/rfc-process.md`](deliverables/05-governance/rfc-process.md) |
| **6. Knowledge transfer** | [`deliverables/06-knowledge-transfer/`](deliverables/06-knowledge-transfer/) |
| Maintenance runbook for HES | [`deliverables/06-knowledge-transfer/maintenance-runbook.md`](deliverables/06-knowledge-transfer/maintenance-runbook.md) |
| 9 architecture decision records (ADRs) | [`deliverables/06-knowledge-transfer/architecture-decision-records/`](deliverables/06-knowledge-transfer/architecture-decision-records/) |
| Cross-collection federation playbook | [`deliverables/06-knowledge-transfer/federation-playbook/`](deliverables/06-knowledge-transfer/federation-playbook/) |
| Vision-language classification pipeline spec | [`deliverables/06-knowledge-transfer/vlm-pipeline-spec.md`](deliverables/06-knowledge-transfer/vlm-pipeline-spec.md) |
| **7. Templates** (rights, identifier policy, scoping, council charter) | [`deliverables/07-templates/`](deliverables/07-templates/) |
| **8. Compliance registry** (institutional declarations) | [`registry/`](registry/) |

## Implementation artefacts

The reference implementation that backs the standard:

```text
case-studies/heritage-aerial/
├── README.md                          (this file)
├── CHANGELOG.md                       (versioned release history)
├── LICENSE                            (CC BY 4.0 + CC0 + MIT)
├── CONTRIBUTING.md
├── ontology/
│   ├── naph-core.ttl                  (the ontology — 30 classes, 29 properties)
│   └── naph-shapes.ttl                (SHACL shapes — tiered + DigitalSurrogate + Place)
├── data/
│   └── sample-photographs.ttl         (10 illustrative records across all 3 tiers)
├── pipeline/
│   ├── legacy-ncap-style.csv          (current-state metadata: messy dates, free-text rights)
│   ├── ingest.py                      (CSV → NAPH TTL transformation pipeline)
│   ├── generated-from-csv.ttl         (output of ingest.py — all records lifted to Baseline)
│   ├── self-assessment.py             (self-service compliance check CLI)
│   ├── generate-report.py             (runs full validation + emits HTML report)
│   ├── iiif-bridge.py                 (NAPH → IIIF Presentation 3.0 manifest generator)
│   ├── footprint-from-flight.py       (vertical FOV polygon derivation from altitude+focal-length)
│   └── stereo-pair-detector.py        (overlap-based stereo pair detection)
├── registry/                          (compliance declaration registry format)
├── .github/workflows/
│   ├── validate.yml                   (CI: ontology + SHACL + ingest + IIIF on every push)
│   └── release.yml                    (CD: tagged releases produce versioned artefacts)
├── reports/
│   ├── validation-report.html         (live SHACL + competency-question results)
│   └── iiif-collection-manifest.json  (10 photos as IIIF Collection)
├── demo/
│   └── index.html                     (interactive map of 10 records, tier-coloured)
└── docs/
    ├── gap-analysis.md                (current NCAP vs computation-readiness)
    ├── cost-effort-analysis.md        (modelled per-tier costs for 100k records)
    ├── reasoning-inference.md         (RDFS inference + ecosystem integration)
    ├── competency-questions.md        (8 research questions the standard enables)
    └── competency-queries.batch.txt   (runnable verification suite)
```

## See it running

```bash
# Serve the case study locally
cd case-studies/heritage-aerial
python3 -m http.server 8765

# Open the demo
# http://localhost:8765/demo/index.html
```

The demo loads the IIIF collection manifest and renders the 10 records on an interactive map — tier-coloured markers, WGS84 footprint polygons, click-through metadata panel. The user-facing artefact that makes the standard concrete.

## The sample dataset

Ten illustrative photograph records distributed across the three tiers:

|#|Tier|Sortie|Subject|Date|
|---|---|---|---|---|
| 1 | Baseline | RAF/106G/UK/1655 | Berlin reconnaissance | 1944-03-28 |
| 2 | Baseline | CPE/SCOT/UK/216 | Edinburgh city centre | 1947-06-15 |
| 3 | Baseline | USN/VD-1/PAC | Saipan coastline | 1944-08-12 |
| 4 | Enhanced | RAF/541/HAM | Hamburg post-firestorm | 1943-07-30 |
| 5 | Enhanced | RAF/58/ABDN | Aberdeen harbour | 1948-09-22 |
| 6 | Enhanced | USAF/91SRS/KOR | Pyongyang industrial district | 1951-04-18 |
| 7 | Enhanced | DOS/KEN | Mount Kenya foothills | 1954-11-08 |
| 8 | Aspirational | RAF/540/PEEN | Peenemünde V-2 facility | 1943-06-23 |
| 9 | Aspirational | RAF/541/EDI | Edinburgh Castle | 1946-08-04 |
| 10 | Aspirational | USAAF/3PRS/HIR | Hiroshima post-detonation | 1945-08-11 |

**Note on the dataset:** these records are *modeled on the structure* of NCAP holdings — they use plausible sortie identifier formats, real squadron/aircraft pairings, real geographic coordinates, and rights statements aligned with rightsstatements.org. They are illustrative of what NCAP records *could look like* under the proposed standard, not assertions that these specific frames exist in the NCAP catalogue with these exact identifiers.

## What this demonstrates

**1. The cost of going from Baseline to Enhanced is small.** Most of the data already exists in NCAP — sortie metadata, squadron, aircraft, capture conditions are routinely recorded. The gap is structuring it (ISO dates, machine-readable fields, explicit provenance chains) rather than acquiring new information.

**2. The cost of going from Enhanced to Aspirational is significant — but partly automatable.** Subject classification, place authority linking, cross-collection record matching can be partially performed by vision-language models and entity-linking tools. The standard should specify *outcome requirements* (what needs to be linked, to which authorities) rather than prescribing manual workflows.

**3. The same standard works across collection sub-types.** RAF reconnaissance, US-transferred imagery, post-war urban surveys, and overseas surveys all model cleanly into the same ontology. The differences are in which optional fields apply, not in the core structure.

**4. Computation-ready is achievable with no new research.** The ontology is built from existing standards (PROV-O, GeoSPARQL, SKOS, Dublin Core, DCAT, FOAF) — synthesis, not invention.

## How to use it

### With Open Ontologies CLI

```bash
# Validate
open-ontologies validate ontology/naph-core.ttl

# Load and query
open-ontologies serve &
curl -X POST http://localhost:8080/api/load -d '{"path":"case-studies/heritage-aerial/ontology/naph-core.ttl"}'
curl -X POST http://localhost:8080/api/load -d '{"path":"case-studies/heritage-aerial/data/sample-photographs.ttl"}'

# Run SHACL validation against the tiered shapes
open-ontologies shacl --shapes ontology/naph-shapes.ttl --data data/sample-photographs.ttl
```

### With Open Ontologies Studio

Open the Studio app, load `naph-core.ttl` followed by `sample-photographs.ttl`. The 3D graph view will render the class hierarchy and individual records. Use the agent chat to run validation, reasoning, or natural-language queries.

## Example queries (computational research that current NCAP metadata can't answer cleanly)

```sparql
# All photographs covering Edinburgh between 1945 and 1950
PREFIX naph: <https://w3id.org/naph/ontology#>
PREFIX geo: <http://www.opengis.net/ont/geosparql#>
SELECT ?photo ?date WHERE {
  ?photo a naph:AerialPhotograph ;
         naph:capturedOn ?date ;
         naph:coversArea/naph:asWKT ?wkt .
  FILTER (?date >= "1945-01-01"^^xsd:date && ?date <= "1950-12-31"^^xsd:date)
  FILTER (geof:sfIntersects(?wkt, "POLYGON((-3.21 55.94, -3.16 55.94, -3.16 55.97, -3.21 55.97, -3.21 55.94))"^^geo:wktLiteral))
}

# All photographs depicting historic events with Wikidata links
SELECT ?photo ?event ?wikidata WHERE {
  ?photo a naph:AerialPhotograph ;
         naph:depicts ?event .
  ?event a naph:HistoricEvent ;
         skos:exactMatch ?wikidata .
}

# Compliance-tier distribution across the collection
SELECT ?tier (COUNT(?photo) AS ?count) WHERE {
  ?photo naph:compliesWithTier ?tier .
} GROUP BY ?tier
```

## End-to-end demonstration: legacy CSV → computation-ready

The most important piece of this case study is showing what the lift actually costs.

`pipeline/legacy-ncap-style.csv` contains 10 records in the form a real heritage cataloguer might receive: free-text dates in 8 different formats, free-text rights statements, point coordinates, altitudes in feet, mixed punctuation. This is what "digitised but not computable" looks like.

```bash
# One command: messy CSV → valid NAPH Turtle at Baseline tier
python3 pipeline/ingest.py pipeline/legacy-ncap-style.csv > pipeline/generated-from-csv.ttl

# Verify the output validates against the standard
open-ontologies validate pipeline/generated-from-csv.ttl
# → {"ok":true,"triples":263}

# Verify it conforms to the SHACL shapes
echo "clear
load ontology/naph-core.ttl
load pipeline/generated-from-csv.ttl
shacl ontology/naph-shapes.ttl" | open-ontologies batch
# → conforms: true, violations: 0
```

The pipeline performs the exact transformations the standard requires:

|Transformation|Input|Output|
|---|---|---|
| Date normalisation | `28 March 1944`, `15-Jun-1947`, `30/07/1943`, `23-Jun-43` | `1944-03-28`, `1947-06-15`, `1943-07-30`, `1943-06-23` |
| Rights mapping | `Crown Copyright Expired` | `https://rightsstatements.org/page/NoC-OKLR/1.0/` |
| Geometry construction | `lat=55.949, lon=-3.195` | WGS84 polygon footprint |
| Unit conversion | `30000` (feet) | `9144.0` (metres) |
| Identifier minting | `RAF/106G/UK/1655` + frame `4023` | `https://w3id.org/naph/photo/RAF-106G-UK-1655-4023` |

This is what an institution actually pays for when adopting the standard — and `ingest.py` is 200 lines.

## Generating validation reports

```bash
python3 pipeline/generate-report.py > reports/validation-report.html
```

Runs the full validation suite (load → SHACL → all competency queries) and emits a styled HTML report with live results. Designed as the artefact institutions publish alongside their data to demonstrate compliance — the dashboard view of NAPH adoption.

## IIIF interoperability

```bash
# One photo
python3 pipeline/iiif-bridge.py "https://w3id.org/naph/example/photo-009" > manifest.json

# Whole collection
python3 pipeline/iiif-bridge.py --all > reports/iiif-collection-manifest.json
```

NAPH-compliant records emit valid **IIIF Presentation API 3.0** Manifests, which means any IIIF viewer (Mirador, Universe Viewer) and any IIIF-aware computational tool can consume the collection without bespoke integration. NAPH metadata flows into IIIF `metadata` pairs; rights statements populate the IIIF `rights` field; the RDF representation is linked via `seeAlso`.

This is interoperability without lock-in: institutions adopt NAPH and gain the IIIF ecosystem for free.

## Roadmap

This is **v0.2** — working ontology, validated SHACL shapes, end-to-end ingest pipeline, IIIF bridge, validation reports.

- [ ] **v0.3** — expand sample dataset to 50+ records with real Air Photo Finder browse data, add CSV-W mapping spec
- [ ] **v0.4** — automated subject classification pipeline (vision-language models, drafts requiring human validation)
- [ ] **v0.5** — full case study writeup with cost/effort breakdown per tier and partner adoption playbook

## Why publish this as open source

This case study is published openly because:

- **Heritage collections need this work** — the problem is real and the gap is sector-wide.
- **Standards adopt faster when they have working reference implementations alongside specification documents.**
- **Open Ontologies is a general-purpose tool** — heritage / GLAM is one application domain among many, and a real-world case study makes the tool more useful to other domains too.

## Licence

- Ontology and shapes: [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)
- Sample data: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) (illustrative records, no original data claims)
- This documentation: [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)

## Acknowledgements

Built with [Open Ontologies](https://github.com/fabio-rovai/open-ontologies). Aligned with [PROV-O](https://www.w3.org/TR/prov-o/), [GeoSPARQL](https://www.ogc.org/standards/geosparql/), [SKOS](https://www.w3.org/2004/02/skos/), [Dublin Core](https://www.dublincore.org/), [FOAF](http://xmlns.com/foaf/0.1/), and [DCAT](https://www.w3.org/TR/vocab-dcat-3/). Tiered design informed by FAIR and CARE principles.

---

**Author:** Fabio Rovai · Kampakis and Co Ltd, trading as The Tesseract Academy · `fabio@thetesseractacademy.com`
