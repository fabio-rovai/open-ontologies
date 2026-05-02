# ADR-0004: Three-tier nested compliance model

**Status:** Accepted
**Date:** 2026-04-30
**Decider:** Editorial team
**Supersedes:** N/A

## Context

A digitisation standard could specify:

(a) A single bar of compliance — institutions either meet the standard or don't
(b) Multiple independent profiles — institutions pick which profile applies
(c) Tiered compliance — institutions can claim incremental levels of compliance

Each has trade-offs:

- (a) is simplest but creates a high adoption barrier — institutions either invest heavily upfront or don't engage at all
- (b) is flexible but creates fragmentation — different institutions with different profiles can't easily compare
- (c) requires more design work but enables incremental adoption while preserving comparability

## Decision

NAPH uses **three nested compliance tiers**:

- **Baseline** — minimum for computation-readiness
- **Enhanced** — supports research workflows
- **Aspirational** — supports semantic discovery

Each tier is a strict superset of the previous. A record claiming Enhanced MUST also satisfy Baseline. A record claiming Aspirational MUST also satisfy Enhanced.

## Consequences

### Positive

- **Low adoption barrier** — Baseline is achievable for any institution within weeks; doesn't require waiting for full Aspirational implementation
- **Incremental upgrade path** — institutions plan Enhanced and Aspirational adoption when capacity allows; never have to redo Baseline work
- **Comparability across institutions** — every NAPH-compliant record's tier is explicit; queries can filter by tier
- **Clear progress signal** — institutions and funders can measure adoption progress (% of records at each tier)
- **Right-sized investment** — institutions invest at the tier they can sustain rather than over-commit
- **Encourages partial compliance** — even Baseline-only adoption produces real research value

### Negative

- **More design work** — three tiers is more to maintain than one
- **Risk of tier inflation** — institutions claiming a tier they don't actually achieve. Mitigated by SHACL validation requirement
- **Risk of tier inertia** — institutions getting stuck at Baseline forever. Mitigated by clear upgrade path documentation, partner clinic encouragement, funder incentives at higher tiers

### Neutral

- The nesting (Aspirational ⊇ Enhanced ⊇ Baseline) is a deliberate choice over independent levels. Independent levels would allow "Aspirational without Enhanced," which we judge would harm comparability and adoption discipline.

## Alternatives considered

### Alternative 1: Single bar of compliance

Rejected because:

- Sets the bar too high for many institutions to engage
- Either Aspirational-only (very few institutions adopt) or Baseline-only (loses Enhanced/Aspirational research value)
- Doesn't match the actual gradient of institutional capability

### Alternative 2: Five tiers (Bronze / Silver / Gold / Platinum / Diamond)

Considered but rejected:

- Marketing-style tiers create more cognitive overhead than useful distinction
- Three meaningful gates (computation-ready / research-ready / semantic) covers the actual adoption journey
- More tiers create more shape combinations to maintain

### Alternative 3: Two tiers (Compliant / Compliant+)

Rejected because:

- Loses the meaningful distinction between research workflows (Enhanced) and semantic discovery (Aspirational)
- The Enhanced→Aspirational delta requires substantively different capabilities (ML/NLP, entity linking) — collapsing them hides the capability gap

### Alternative 4: Multiple parallel profiles per resource type, single tier

Rejected because of [ADR-0001](0001-narrow-vertical.md) — NAPH covers only aerial photography, no need for resource-type profiles.

## Tier design rationale

### Baseline — what's the minimum?

The Baseline question is: "what's the least metadata that allows computational find / identify / locate / use?"

- **Identifier** — to find the record
- **Date** — to filter temporally
- **Geographic footprint** — to filter spatially
- **Rights** — to determine reuse permission
- **Sortie / Collection link** — to locate within the institutional structure

Without any of these, computational queries fail. With all of these, basic computational research is enabled.

### Enhanced — what unlocks research workflows?

The Enhanced question is: "what additional metadata makes computational research **reproducible**?"

- **Digital surrogate variants** — preservation master + access copy
- **Capture context** — how the photograph was taken
- **Provenance chain** — how the artefact got to where it is
- **Workflow documentation** — how the institution does its work

These don't enable new query types; they enable trust in query results. A reproducible research workflow needs to know the provenance, not just the data.

### Aspirational — what unlocks semantic discovery?

The Aspirational question is: "what additional metadata makes the collection **part of the wider knowledge graph**?"

- **Subject classification** — what each photograph depicts
- **Place authority links** — connection to GeoNames, Wikidata
- **Cross-collection links** — connection to other archives
- **AI-derived enrichment with provenance** — automated classification with human validation

These enable the federated queries and content-based retrieval that justify the heaviest investment.

## Validation

The tier model is validated by:

- The sample data: 10 records distributed 3-4-3 across tiers, all validating cleanly via tier-specific SHACL shapes
- Module specifications: each module has explicit Baseline / Enhanced / Aspirational requirements that compose cleanly
- Cost analysis: per-tier costs scale roughly proportional to per-tier research value, suggesting the tier structure is well-calibrated

## Tier modification policy

Adding requirements to higher tiers (Enhanced, Aspirational) is a MINOR change.
Adding requirements to Baseline is a MAJOR change because it raises the bar for existing compliant records.
Removing requirements from any tier is a MAJOR change.

These constraints are enforced via the [RFC process](../../05-governance/rfc-process.md).

## Revisiting

This ADR may be revisited if:

- Real-world adoption shows institutions getting stuck at one tier (suggests tier definition needs adjustment)
- Research demand creates demand for an additional tier (e.g. above Aspirational)

Realistic timeframe: review at v1.5 or v2.0 milestone.

## Cross-references

- [NAPH Standard §3 — The three tiers](../../01-standard/NAPH-STANDARD.md#3-the-three-tiers)
- [Module specifications](../../01-standard/modules/) — each defines tier-specific requirements
- [Cost & effort analysis](../../../docs/cost-effort-analysis.md) — per-tier cost analysis validating the structure
- [Validation checklists](../../04-adoption-guidance/validation-checklists.md)
