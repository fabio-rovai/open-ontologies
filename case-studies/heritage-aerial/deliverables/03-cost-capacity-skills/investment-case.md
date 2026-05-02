# Investment Case — NAPH Adoption for Aerial Photography Heritage

The case for investment in NAPH adoption from three perspectives: institutional, sector-wide, and funder.

This document is **paired** with [`cost-effort-analysis.md`](../../docs/cost-effort-analysis.md) (modelled costs) and [`skills-map.md`](skills-map.md) (capability requirements).

## 1. The market

The UK aerial photography heritage vertical has scale that makes investment defensible:

| Holding | Records | Note |
|---|---|---|
| **NCAP** (HES) | ~30,000,000 | Largest single aerial photography archive in Europe; held by Historic Environment Scotland |
| **Imperial War Museums** | ~6,000,000 | Photographs and reconnaissance material |
| **National Archives (UK)** | ~3,000,000 | Crown Copyright reconnaissance, declassified post-1972 |
| **NARA (US, partnered with NCAP)** | ~6,000,000 | US-held aerial imagery digitised on behalf of NCAP |
| **Royal Air Force Museum** | ~1,500,000 | Specific RAF archives |
| **Local authority and academic archives** | ~5,000,000 (cumulative) | Varies enormously by institution |

**Total UK aerial photography heritage: ~50 million records.** Of this, perhaps 10% is currently catalogued in computation-ready form. NAPH adoption targets the gap.

Internationally, comparable national-level holdings exist for France (IGN), Germany (Bundesarchiv), Italy (IGM), Australia (NLA), Canada (LAC), and similar institutions. NAPH is designed for international adoption beyond the UK.

## 2. Researcher demand

Aerial photography heritage supports research across multiple disciplines:

| Field | Use case |
|---|---|
| Climate science | Coastal erosion, glacier retreat, vegetation change over decades |
| Conflict studies | Bombing damage assessment, troop movement analysis, infrastructure targeting patterns |
| Archaeology | Cropmark surveys, site discovery, landscape phase analysis |
| Urban history | Rebuilding patterns, neighbourhood change, infrastructure development |
| Environmental policy | Land-use change, deforestation, pollution events |
| Genealogy and local history | Family-property research, village reconstruction |

Each field produces published research that cites specific records. Currently, citation is manual (image numbers in footnotes) and irreproducible. NAPH-compliant collections enable persistent, citable, queryable records — making aerial-photography research first-class in the digital scholarly record.

## 3. The institutional case

### 3.1 Direct returns from Baseline adoption

For an institution adopting NAPH Baseline:

- **External research enquiries** — researchers can find collections via federated search rather than knowing which institution holds what
- **Aggregator participation** — Europeana, DPLA, national portals harvest with no extra effort
- **Funder reporting** — open-data, FAIR alignment, capability building all evidenced via SHACL conformance
- **Internal reuse** — institutional staff can build new applications on the data without rebuilding

### 3.2 Strategic returns from Enhanced adoption

- **Reproducible computational research** — papers cite specific records; results replicable
- **Cross-institutional federated discovery** — partnerships with NARA, IGN, Bundesarchiv become technically simple
- **Specialist tooling** — stereo-pair detection, change-detection, GSD-aware retrieval all become standardisable
- **National-programme participation** — N-RICH, RICHeS, future heritage research infrastructure investment relies on standards-compliant collections

### 3.3 Aspirational returns

- **Knowledge-graph queries** — "show me all aerial coverage of [location] linked to [historic event]" works as a single query
- **Vision-language model use** — content-based retrieval, automatic subject classification at sample-validated accuracy
- **AI-driven curation** — research-priority subsets surfaced automatically rather than manually

### 3.4 Concrete cost example

For an institution with 100,000 records (representative mid-sized aerial photography archive):

| Tier | One-time cost | Per-year ongoing |
|---|---|---|
| Baseline | £21,000 | £2,000 |
| Enhanced (delta) | £110,000 | £8,000 |
| Aspirational (delta) | £75,000 | £6,000 |

Total to reach Aspirational from nothing: ~£206,000 capital + ~£16,000/year ongoing.

By comparison, the current annual cost of *not* having computation-ready data — measured in lost research enquiries, manual customer-service overhead, missed funding compliance, missed aggregator inclusion — is plausibly £30,000-£100,000/year for an institution of this size, but is rarely tracked.

## 4. The sector case

If the UK aerial photography vertical adopts NAPH Baseline within 5 years:

- **50,000,000 records** become computation-ready
- **Cross-institutional research** becomes technically straightforward
- **Federated discovery** with NARA, IGN, Bundesarchiv etc. becomes standardisable
- **Sector-shared infrastructure** (entity linking, classification services) becomes investable because the standardisation pre-condition is met

This positions UK aerial photography as the most computationally accessible national archive globally. That position has soft-power, research-impact, and economic value (heritage tourism, education licensing, derivative works).

### 4.1 Shared sector infrastructure

NAPH adoption unlocks investments that don't make sense for individual institutions but do at sector level:

| Service | One-time cost | Per-year operating | Beneficiaries |
|---|---|---|---|
| Subject-classification service (vision-LM) | £400k | £200k | All Aspirational adopters |
| Entity-linking service (GeoNames/Wikidata) | £200k | £100k | All Aspirational adopters |
| Shared SPARQL hosting | £150k | £80k | All Enhanced+ adopters without in-house capacity |
| Stereo-pair and change-detection tooling | £600k | £150k | Research-active institutions |
| OCR / mission-log parsing service | £300k | £120k | All institutions with paper sortie logs |

Total sector-shared investment: ~£1.65M one-time + ~£650k/year. This is at the same scale as a single Discovery Project under TaNC — but unlocks capability for the entire vertical, not one academic project.

## 5. The funder case

For UKRI / AHRC / Research England:

### 5.1 Capability building

NAPH adoption builds durable capability:

- Linked-data engineering skills — currently a sector-wide gap
- Domain-specific (aerial-photography) computational research methods
- Cross-institutional working practices via shared standards

These skills are reusable across heritage research infrastructure, not just N-RICH.

### 5.2 Open data return

Public investment in heritage collections produces public-good open data. NAPH compliance makes this open-data investment legible — funders can demand and verify FAIR / CARE alignment via SHACL conformance reports rather than aspirational language.

### 5.3 International leverage

The UK has the largest aerial photography heritage holdings of any single country (NCAP alone is the largest in Europe). A UK-led NAPH standard, if adopted internationally, positions the UK as the convening authority for aerial-heritage standards globally — analogous to the British Library's role in bibliographic standards (MARC, RDA).

### 5.4 Research impact

NAPH-enabled research generates measurable impact:

- More citable papers using aerial photography heritage data
- Cross-disciplinary collaborations (climate × heritage, conflict × heritage)
- Public-engagement applications (interactive maps, change-detection visualisations)

These are direct REF-eligible impact metrics for HEI partners.

## 6. The risk profile

### 6.1 Adoption risk

Lower than average for sector standards:

- **Voluntary, incremental adoption** — Baseline tier is low-cost, no rebuild required
- **Built on existing standards** (PROV, SKOS, DCAT, GeoSPARQL, IIIF) — no novel technical risk
- **Governance designed for sustainability** — see [Governance Proposal](../05-governance/governance-proposal.md)

### 6.2 Maintenance risk

Mitigated by:

- Open-source release with permanent w3id.org URI
- Community governance with documented RFC process
- Multiple-institution stewardship (no single-point-of-failure dependency)
- Maintenance runbook designed for HES handover

### 6.3 Obsolescence risk

NAPH is built on stable W3C/OGC/IIIF standards. The risk of those parent standards becoming obsolete within a 10-year horizon is very low. NAPH itself is small enough (~30 classes, ~30 properties) to be replaced if necessary without catastrophic re-modelling cost.

## 7. Recommended investment sequence

For a funder considering NAPH-enabled heritage research infrastructure:

### Year 1 (Pilot)

- Complete v1.0 standard development (NAPH)
- Apply with 3-5 partner institutions
- Generate first round of cost evidence
- Establish governance via MOU with HES

### Years 2-3 (Capability building)

- Fund 5-10 partner institutions to reach Baseline tier
- Establish 1-2 shared sector services (priority: subject classification, entity linking)
- Run 3-5 research projects using NAPH-compliant data to demonstrate value

### Years 3-5 (Sector adoption)

- Open competitive funding for institutional NAPH adoption
- Expand shared sector services
- Fund cross-institutional research programmes

### Year 5+ (Maintenance + extension)

- HES (or successor) operates the standard under sustainable governance
- Investment shifts from adoption to use
- Adjacent vertical standards (terrestrial photogrammetry, LiDAR) may be added if domain demand justifies

Total programme investment over 5 years: ~£10M-£20M depending on shared-service ambition. Compare to TaNC's £18.9M programme spend that produced predominantly reports and workshops; NAPH-style funding produces a vertical of computation-ready collections.

## 8. The downside case

If NAPH does not get adopted:

- The UK aerial photography vertical remains in its current state — digitised but not computable
- Cross-institutional research remains manually federated, expensive, slow
- The opportunity for international standards leadership passes to another country (likely France IGN or Germany Bundesarchiv)
- Research investment in aerial photography heritage continues to produce one-off applications rather than reusable infrastructure

## 9. The recommendation

Fund NAPH adoption as a **vertical capability-building programme** running parallel to broader N-RICH infrastructure investment. The vertical focus makes investment legible, measurable, and successful at scale that broader GLAM-wide programmes have historically struggled to achieve.

The aerial photography vertical is the highest-leverage starting point because:

1. It has a willing primary host (HES via NCAP)
2. It has clear research demand (climate, archaeology, conflict studies)
3. It has manageable scale (50M records is large but bounded)
4. It has a domain expert pipeline (RAF Museum, IWM, RCAHMS, academic departments)
5. The case study evidence already exists (this work)

## Cross-references

- [Cost & effort analysis](../../docs/cost-effort-analysis.md)
- [Skills map](skills-map.md)
- [Governance proposal](../05-governance/governance-proposal.md)
- [Adoption guidance](../04-adoption-guidance/how-to-use-this-standard.md)
