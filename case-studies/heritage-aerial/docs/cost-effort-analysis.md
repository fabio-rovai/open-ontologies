# Cost & Effort Analysis — what NAPH adoption actually costs

Cost, skills, and organisational implications of NAPH adoption — modelled to inform investment decisions for institutions and funders considering aerial photography heritage research infrastructure. This document uses a notional 100,000-record collection comparable to a single NCAP sub-collection.

These figures are **modelled estimates** for illustrative purposes. They demonstrate what a cost study output would look like, structured how it should be structured. Real numbers should come from partner testing as institutions adopt the standard — but the framework for calculating them already exists.

## The cost structure of standard adoption

There are three categories of cost that institutions actually incur:

1. **One-time structural** — mapping legacy fields, choosing identifier conventions, selecting rights authorities. Done once, applies forever.
2. **Per-record processing** — applying the standard to existing records. Scales linearly with collection size, but most steps are automatable.
3. **Ongoing maintenance** — keeping pace with standard revisions, ingesting new acquisitions in compliant form, cross-collection link maintenance.

NAPH was designed to minimise category 1 and category 3 specifically by:

- Synthesising existing standards (PROV-O, GeoSPARQL, SKOS, DCAT, FOAF) so institutions don't make new mapping decisions for things the sector already agrees on.
- Specifying outcome requirements rather than prescriptive workflows, so institutions can reuse infrastructure they already have.
- Defining tier compliance as nested, so progress is incremental and never requires rebuild.

## Modelled costs per tier — for a 100,000-record collection

### Baseline tier — getting to computation-ready

| Task | Cost type | Notional FTE-days | Automation potential |
|---|---|---|---|
| Sortie/frame identifier scheme adoption | One-time | 2 | N/A (policy decision) |
| Legacy date format → ISO 8601 conversion | Per-record | 0.0001/record (10 days for 100k) | High — pattern matching, low residual manual review |
| Rights statement text → URI mapping | One-time | 1 | Full once mapping table built |
| Geographic footprint exposure (existing internal data → public WKT) | One-time + per-collection | 5 | Medium — depends on legacy format |
| BagIt / RO-Crate packaging implementation | One-time | 8 | Full once tooling configured |
| QA sampling + validation report generation | Per-record | 0.00005/record (5 days for 100k) | Full via SHACL automation |
| **Baseline total** |  | **~30 FTE-days** | mostly automatable |

At a notional senior-developer day rate of £700, **Baseline lift for 100k records ≈ £21,000**. Most of that is one-time structural; the per-record cost is dominated by automated transformation with sampling-based QA.

### Enhanced tier — supports research workflows

| Task | Cost type | Notional FTE-days | Automation potential |
|---|---|---|---|
| Internal digitisation metadata exposure | One-time | 6 | High once data plumbing in place |
| Capture context cross-reference (frame → flight log) | Per-record | 0.0005/record (50 days for 100k) | Medium — AI-assistable for legible logs |
| Provenance chain documentation per sortie | Per-sortie | 0.5/sortie (assume 200 sorties = 100 days) | Low — domain expertise required |
| Multiple surrogate format generation | Per-record | 0.00002/record (2 days for 100k) | Full automation |
| **Enhanced delta total** |  | **~158 FTE-days** | partially automatable |

**Enhanced tier delta ≈ £110,000** for 100k records over Baseline. The cost driver is provenance documentation, which is domain-expert work that doesn't scale automatically.

### Aspirational tier — supports semantic discovery

| Task | Cost type | Notional FTE-days | Automation potential |
|---|---|---|---|
| Subject classification (vision-language model + sample validation) | Per-record + sampling | 0.0002/record (20 days for 100k) + 30 days human validation | High for first-pass, medium for QC |
| Place authority linkage (GeoNames/Wikidata SPARQL federation) | Per-record | 0.0001/record (10 days for 100k) | Full automation, residual conflict resolution |
| Cross-collection record matching | Per-record | 0.0003/record (30 days for 100k) | High via embedding similarity |
| Event linkage (Wikidata events) | Per-record | 0.00005/record (5 days for 100k) | Full once authority lists curated |
| QA + drift monitoring infrastructure | One-time + ongoing | 12 + ongoing | Medium |
| **Aspirational delta total** |  | **~107 FTE-days** | mostly automatable with human gates |

**Aspirational tier delta ≈ £75,000** for 100k records over Enhanced. Most cost is in QA and validation of automated outputs rather than original cataloguing work.

## Cumulative cost projection

| Tier | Cumulative FTE-days | Cumulative cost (notional £700/day) | Cost per 1,000 records |
|---|---|---|---|
| Baseline | 30 | £21,000 | £210 |
| Enhanced | 188 | £132,000 | £1,320 |
| Aspirational | 295 | £206,000 | £2,060 |

For context: a single TaNC Discovery Project budget of £3.6M would fund 100% Aspirational compliance for **~1.7M records**, with budget remaining for ongoing maintenance and infrastructure development.

## What this tells funders

1. **Baseline is cheap and overwhelmingly automatable.** Any institution can reach Baseline compliance for any collection within budget. There is no good reason for a digitised collection in 2026 to remain non-computable.
2. **Enhanced has a real domain-expert cost but is one-off.** Once the provenance work is done, it doesn't need redoing. The work scales sub-linearly with collection size because much of it is per-sortie or per-acquisition-event rather than per-record.
3. **Aspirational is mostly automation + QA.** With vision-language models and entity-linking tooling, the per-record cost of semantic enrichment has fallen by an order of magnitude over the past three years. The standard should specify outcome requirements ("must be linked to authority X") rather than methods, allowing institutions to use the most cost-effective tooling available at the time.
4. **The cost gap between tiers reflects real research value.** Enhanced unlocks computational research workflows. Aspirational unlocks semantic discovery and cross-collection queries. The cost ratios mirror the value ratios.

## Skills required

| Tier | Skills needed | Existing in most GLAM institutions? |
|---|---|---|
| Baseline | Data engineering, basic Python/SPARQL, project management | Partially (often outsourced) |
| Enhanced | Above + domain archivist for provenance, metadata schema design | Mostly yes |
| Aspirational | Above + ML/NLP engineering, entity-linking, knowledge graph engineering | Rarely in-house |

The Aspirational skill gap is the strongest argument for shared sector infrastructure rather than per-institution Aspirational tier. A central N-RICH service running entity linking and cross-collection matching as a service would dramatically reduce per-institution cost.

## Adoption pathway recommendation

Based on this cost structure, the rational adoption pathway is:

1. **Year 1**: Baseline for the entire collection. Cost: per-record × collection-size. This unblocks computational research immediately.
2. **Year 2-3**: Enhanced for high-research-value subsets first (declassified WW2 reconnaissance, etc.) — the records most queried by external researchers.
3. **Year 3+**: Aspirational selectively, prioritising collections with active research demand and institutional partnerships.

This sequence delivers maximum research value per pound spent, and aligns with how funding bodies typically structure capacity-building grants.

---

**Note on figures.** All numbers above are modelled estimates based on assumptions about workflow efficiency, automation tooling, and institutional capacity. Real numbers should come from partner testing as institutions adopt the standard. This document is a worked example of what a cost study output should look like — its framework, structure, and decision-relevance — not an empirical dataset.
