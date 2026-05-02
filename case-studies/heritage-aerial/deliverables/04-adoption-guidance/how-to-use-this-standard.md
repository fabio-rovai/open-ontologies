# How to Use This Standard

A pragmatic guide for collections managers, digital officers, project managers, and funders. This is **not** a technical reference — for that, read the [Standard](../01-standard/NAPH-STANDARD.md) and [module specifications](../01-standard/modules/). This is the on-ramp.

## Who this guide is for

| Role | What you need from this guide |
|---|---|
| **Collections manager** | Understand what NAPH compliance means in practice for your daily work, what changes in cataloguing, what doesn't |
| **Digital officer** | The technical decisions you'll make: identifier scheme, rights URIs, packaging format, validation tooling |
| **Project manager** | Realistic scope, sequencing, who needs what training, how to phase adoption |
| **Funder** | What capability building NAPH adoption builds, what's the realistic timeline, how progress is measurable |

You do not need to read everything. Start with §1 to confirm relevance, then jump to your role.

## 1. Is NAPH right for you?

NAPH applies to a heritage collection if **all** of the following are true:

- ✅ The collection has been or will be digitised (scanned, photographed, captured)
- ✅ You expect computational research use (search, aggregation, AI-driven analysis) — not just human browsing
- ✅ Your records will be discoverable beyond your institution's primary catalogue
- ✅ You have at least one person with technical capacity for ontology / RDF / SHACL work, or budget to engage one

If your collection is **only** for in-house human browsing and you don't expect external research use, NAPH is overhead with no payoff. Stop here.

If your collection meets the above, continue.

## 2. The 90-second tour

NAPH has three nested compliance tiers:

- **Baseline** — minimum metadata structure for computational find-identify-locate-use
- **Enhanced** — adds digitisation provenance, capture context, full provenance chain
- **Aspirational** — adds semantic enrichment, place/event linking, cross-collection links

Pick a tier based on your goals (§3). Apply the [six modules](../01-standard/modules/) for that tier (§4). Validate via the [validation toolkit](validation-checklists.md). Publish (§5). Maintain (§6).

## 3. Picking your starting tier

| If your goal is… | Start at tier… | Time to first compliant record |
|---|---|---|
| Be discoverable to external researchers | Baseline | 2-4 weeks |
| Support reproducible computational research | Enhanced | 2-4 months |
| Enable cross-collection knowledge graphs | Aspirational | 6-12 months |

You don't need to start at the highest tier. **Most collections should start at Baseline.** It's incrementally upgradeable to higher tiers without rework.

### What changes at each tier

#### Baseline — what you change

You add structure to existing data:

- ISO 8601 dates (replace free-text)
- Stable URIs as identifiers
- Machine-readable rights statements (rightsstatements.org URIs)
- WGS84 polygon footprints (often derivable from existing point data)
- A manifest listing all records

You don't change cataloguing practice substantively. Most fields stay the same; structure becomes standard.

#### Enhanced — what you add

On top of Baseline:

- Multiple surrogate variants (preservation master + access copy)
- Documented digitisation events with operator, equipment, date
- Per-record provenance chains
- Workflow documents referenced by URL

This is where the work increases. Existing internal data is sufficient; the work is exposing and structuring it.

#### Aspirational — what you build

On top of Enhanced:

- Subject classification (where applicable)
- Place authority links (GeoNames, Wikidata)
- Cross-collection record matching
- Computational reuse testing as part of validation

This is genuinely new work, but most of it is automatable.

## 4. The six-module checklist

For each tier you target, ensure each module's requirements are met:

| Module | Topic | Baseline ask |
|---|---|---|
| [A](../01-standard/modules/A-capture-imaging.md) | Capture & Imaging | Lossless format, ≥300 DPI, capture date documented |
| [B](../01-standard/modules/B-metadata-data-structures.md) | Metadata | Stable identifier, ISO date, WGS84 polygon, sortie/accession link |
| [C](../01-standard/modules/C-rights-licensing-ethics.md) | Rights | Machine-readable rights statement URI per record |
| [D](../01-standard/modules/D-packaging-publication.md) | Packaging | Manifest of all records, content-negotiable URIs, bulk-download mechanism |
| [E](../01-standard/modules/E-paradata-workflow.md) | Paradata | (Enhanced+) provenance chain documented |
| [F](../01-standard/modules/F-qa-validation.md) | QA & Validation | SHACL validation passes; sample human review |

For each module, follow the linked specification.

## 5. Adoption sequence — typical Baseline rollout

A realistic 12-week rollout for a 100,000-record collection:

| Week | Phase | Activity | Deliverable |
|---|---|---|---|
| 1 | Scoping | Catalogue audit — what fields exist, what's missing, what format | Audit report |
| 2 | Identifier policy | Decide identifier scheme (existing scheme + URI prefix, or new) | [Identifier policy doc](decision-trees/identifier-policy.md) |
| 3 | Rights mapping | Map current rights vocabulary to rightsstatements.org URIs | [Rights decision tree](decision-trees/rights-decision-tree.md) |
| 4-5 | Date normalisation | Build/run date-normalisation pipeline | Normalised dates in source system |
| 6-7 | Geometry | Add WGS84 footprint structure | Geographic coverage exposed |
| 8 | Manifest | Build BagIt or DCAT manifest | Manifest at stable URL |
| 9 | Validation tooling | Set up SHACL validation in CI | Validation report at stable URL |
| 10 | Pilot | Validate first 1,000 records | Issues backlog for cleanup |
| 11 | Scale | Apply to full collection | Full collection validated |
| 12 | Publication | Publish manifest, validation report | Public Baseline conformance |

This isn't a definitive schedule — small institutions may need 6 weeks, large institutions with messy legacy data may need 24. The sequence is the durable insight.

## 6. After Baseline — incremental upgrade

Once Baseline is achieved, plan Enhanced/Aspirational by **research-value** not by record count.

- Pick a high-research-value subset (e.g. declassified WW2 reconnaissance for an aerial collection)
- Apply Enhanced tier requirements to just that subset
- Validate, publish, gather researcher feedback
- Iterate

This avoids the "all 100,000 records, fully upgraded by end of year" trap that rarely succeeds. Research value is unevenly distributed; investment should match.

## 7. Common questions

### Q: Does this replace my existing catalogue system?

**No.** NAPH is a *publication layer* alongside your existing system. You keep your catalogue (KE EMu, AdLib, ArchivesSpace, custom). You publish NAPH-compliant RDF derived from it.

### Q: Do I need to learn RDF / SPARQL?

For Baseline adoption, mostly no. The transformation happens in tooling (the [ingest pipeline](../../pipeline/ingest.py) is one example). Your role is to specify the mappings.

For Enhanced/Aspirational, someone on the team should understand RDF basics. The [training materials](../06-knowledge-transfer/training-materials.md) cover this.

### Q: What if my catalogue doesn't have field X that NAPH requires?

Three options, in order of preference:

1. Extract X from existing data via transformation (e.g. extract date from a free-text title field)
2. Add X to your catalogue as part of NAPH adoption work
3. Adopt at a lower tier that doesn't require X

What you cannot do is omit a `MUST` field at the tier you claim. SHACL validation will fail.

### Q: How do I handle records I'm uncertain about?

For Baseline, every record published MUST have ISO date, footprint, rights, identifier, sortie/accession link, collection link. If you don't have those for a record, exclude it from the published set or hold it in a "draft" status until it has them.

### Q: My institution is small. Can we do this with limited capacity?

Yes. Baseline is achievable with a single capable person and 4-12 weeks. The work is structured (transformation), not creative.

For Enhanced/Aspirational, you may benefit from a small consultancy engagement (5-30 days of expertise) rather than building in-house.

### Q: What's the ROI?

For most collections:

- **Discoverability** — external researchers can find your collection via federated search
- **Funding** — many funders increasingly require open-data compliance; NAPH demonstrates it
- **Aggregator participation** — Europeana, DPLA, national portals harvest NAPH-compliant data with no extra effort
- **Internal reuse** — your own staff can build new applications on the data without rebuilding

A typical Baseline lift pays back in <2 years through a combination of these.

## 8. What to do now

If this guide convinces you NAPH is relevant:

1. Read the [Standard v1.0](../01-standard/NAPH-STANDARD.md) (~30 minutes)
2. Read the relevant [profile](../01-standard/profiles/) — Photographic, Manuscripts & Archives, or Integrated Thematic
3. Run a small pilot: 100 records, 1 week, see how the data flows
4. Estimate full-scale based on pilot — then decide tier and timeline

If you're stuck, reach out via the project's [issue tracker](https://github.com/fabio-rovai/open-ontologies/issues) or the maintainers contact in [governance](../05-governance/governance-proposal.md).

## Cross-references

- [Standard v1.0](../01-standard/NAPH-STANDARD.md)
- [Validation checklists](validation-checklists.md) — what to check at each tier
- [Worked examples](worked-examples.md) — tier-by-tier complete examples
- [Decision trees](decision-trees/) — rights, identifiers, dates, tier progression
- [Skills map](../03-cost-capacity-skills/skills-map.md) — what skills the team needs
- [Cost & effort analysis](../../docs/cost-effort-analysis.md) — what it actually costs
- [Partner clinic playbook](../05-governance/partner-clinic-playbook.md) — if you want hands-on engagement
