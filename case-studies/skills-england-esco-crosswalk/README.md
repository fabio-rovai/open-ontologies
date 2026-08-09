# Skills England ↔ ESCO occupation crosswalk

A SKOS crosswalk from the **Skills England occupational standards** (the 1,269
occupations behind England's apprenticeship and technical-education system, as
modelled in [SEOM](../skills-england-occupational-maps)) to the EU's **ESCO**
occupation classification, with conservative, labelled confidence.

Part of [Open Ontologies](../../README.md). SEOM names OGL v3.0; ESCO © European
Union, CC BY 4.0; this crosswalk CC BY 4.0.

## Why

ESCO is the reference vocabulary for skills and occupations across Europe and is
increasingly used for labour-market analytics and cross-border comparison.
England's occupational standards are a separate, nationally-specific taxonomy.
Anyone who wants to read English apprenticeship data against ESCO-tagged
evidence needs a crosswalk, and needs to know how far to trust each link.

## What it finds

| Band | Definition | Occupations |
|---|---|---|
| exactMatch | label identity (sim ≥ 0.95) | **114** |
| closeMatch | strong lexical match (≥ 0.82) | **281** |
| relatedMatch | partial lexical match (≥ 0.62) | **270** |
| unmatched | no lexical match ≥ 0.62 | **604** |

**52.4%** of English occupational standards find a lexical match to an ESCO
occupation; only **31%** match strongly (exact or close). The large unmatched
tail is the headline, not a failure: it quantifies where England's occupational
language diverges from ESCO's, which is exactly the surface a semantic crosswalk
(or a national extension to ESCO) has to cover.

## Honest about method

This is a **lexical** crosswalk: normalised-label similarity (difflib sequence
ratio combined with token overlap and containment) against each ESCO candidate's
title and English alternative labels. It is a candidate-generation and triage
asset, not an authoritative equivalence. Every link carries its similarity score
and `matchMethod` so a downstream user can set their own threshold, and the
unmatched set is published in full. A semantic method (embeddings + adjudication)
would recover more of the related/unmatched tail; that is stated as future work,
not quietly assumed away.

## Files

- `data/esco_candidates.jsonl` — cached ESCO API candidates per occupation
- `data/crosswalk.json` — full banded crosswalk incl. unmatched
- `crosswalk.ttl` — SKOS mappings (skos:exactMatch / closeMatch / relatedMatch)
- `pipeline/fetch_esco.py`, `pipeline/build_crosswalk.py` — reproducible build
- `metrics.json` — band counts and match rate

## Reproduce

```bash
python3 pipeline/fetch_esco.py       # caches ESCO candidates (public API)
python3 pipeline/build_crosswalk.py  # builds banded SKOS crosswalk
```

Independent, self-initiated open research built from public data; endorsed by no
named body.
