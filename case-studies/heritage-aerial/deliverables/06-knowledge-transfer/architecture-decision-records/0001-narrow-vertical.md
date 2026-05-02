# ADR-0001: Narrow vertical scope — aerial photography heritage only

**Status:** Accepted
**Date:** 2026-04-30
**Decider:** Editorial team (initial), expected Standards Council ratification at v1.x
**Supersedes:** N/A
**Superseded by:** N/A

## Context

The Towards a National Collection programme identifies three priority collection types in the heritage research infrastructure space: photographic, manuscripts/archives, and integrated thematic. Initial drafts of NAPH attempted to cover all three with a generic core ontology and three profiles.

During development, two converging signals indicated this generic approach was problematic:

1. **Each collection type has substantively different domain characteristics.** Aerial photography has stereo pairs, sortie metadata, ground sample distance. Manuscripts have hierarchy (fonds/series/file/item), HTR provenance, multilingual transcription. Integrated thematic collections cross resource types and require relationship modelling. A single ontology covering all three would be either too generic to be useful or so large it would be impractical to maintain.

2. **Adoption is more likely if the standard is narrow and deep than broad and shallow.** A generic GLAM standard is one of many; a focused aerial-photography standard is the only one of its kind and has clear primary stakeholders (NCAP, IWM, NARA-partnered collections).

## Decision

NAPH v1.0 covers only **aerial photography heritage**. The single normative profile is the [Aerial Photography Profile](../01-standard/profiles/aerial-photography.md).

Manuscripts/archives and integrated thematic collections are explicitly out of scope. Other organisations may develop adjacent standards for those domains; NAPH does not aim to be the umbrella standard for all heritage.

## Consequences

### Positive

- **Clearer adoption story** — a UK aerial archive can read the standard and immediately recognise it as describing their world. Generic GLAM standards leave readers wondering "but how does this apply to me?"
- **Deeper domain treatment** — stereo pair modelling, GSD derivation, declassification provenance, sortie metadata can all be normatively defined without inflating the standard's scope
- **Higher likelihood of host institution adoption** — HES via NCAP has existential interest in aerial-specific computational capability; less interest in being the host for a generic GLAM standard
- **Cleaner standards collaboration** — adjacent standards bodies (W3C, OGC, IIIF) prefer specific use cases to generic claims
- **Smaller surface area to maintain** — fewer classes, fewer shapes, fewer profile-specific complications
- **Easier to test** — partner clinics can be aerial-photography-only, with consistent expectations

### Negative

- **Smaller market** — NAPH only addresses one vertical; institutions with manuscripts or thematic collections are not direct beneficiaries
- **Apparent narrowness** — funders looking for sector-wide impact may prefer broader standards
- **Narrower scope than the broader programme** — TaNC mentions three collection types; NAPH covers one. Adjacent verticals are scoped for separate standards under shared governance.

### Neutral

- **Potential for adjacent standards** — manuscripts and thematic collections may eventually need their own focused standards; NAPH as a model could be replicated
- **Potential merge** — if multiple vertical standards emerge, a future v2 or v3 of NAPH (or a successor) could federate them under common governance

## Alternatives considered

### Alternative 1: Generic GLAM-wide standard with three profiles

The original approach. Rejected because:

- Each profile would need substantial additional modelling that the generic core can't anticipate
- Maintenance burden compounds across profiles
- Adoption is harder when the standard is one of many
- A focused initial release timeline is insufficient to deliver three profiles to production quality

### Alternative 2: Two profiles — photographic (broad) and everything-else

Rejected because:

- "Photographic" still spans aerial, studio, documentary, fine-art with very different conventions
- A photograph in an aerial reconnaissance archive is a fundamentally different artefact (computationally) from a fine-art photograph in a museum collection — same media, different research use cases
- The "everything-else" profile would still suffer from the genericness problem

### Alternative 3: Aerial photography only (chosen)

Selected because:

- Highest research-value vertical: climate, archaeology, conflict studies, urban history all depend on aerial photography
- Largest holdings concentration: NCAP alone has 30M records
- Clear primary host: HES via NCAP
- Deep domain characteristics that benefit from focused treatment
- Manageable scope for a focused initial release

### Alternative 4: Aerial reconnaissance only (sub-vertical)

Considered but rejected:

- Excludes satellite imagery and UAV imagery — both are growing parts of the aerial heritage corpus
- Would force institutions with mixed aerial-source collections (e.g. modern UAV survey alongside historic reconnaissance) to use multiple standards
- The unifying technical characteristics (orientation, GSD, footprint geometry) apply across reconnaissance, satellite, and UAV

The chosen scope — all aerial-platform photographic heritage — is the right granularity.

## Validation

This decision is validated by:

- The case study evidence: aerial-specific features (stereo pairs, GSD, sortie metadata) are demonstrably useful and missed by generic GLAM standards
- Partner institution interest: HES, IWM, RAF Museum, NARA-partner archives all benefit from a focused standard
- Adjacent standards: rightsstatements.org, IIIF, GeoSPARQL are all narrow-purpose standards that succeeded by being focused

## Revisiting

This ADR may be revisited if:

- Strong demand emerges for adjacent verticals AND there is funding to develop them
- A successor body chooses to merge multiple vertical standards
- Aerial-photography adoption proves insufficient to justify the standard's existence (very unlikely given holdings volumes)

Reasonable timeframe for revisitation: post-2030 once v1.x has matured.

## Cross-references

- [NAPH Standard v1.0](../../01-standard/NAPH-STANDARD.md)
- [Aerial Photography Profile](../../01-standard/profiles/aerial-photography.md)
- [Investment case](../../03-cost-capacity-skills/investment-case.md)
- [Towards a National Collection / N-RICH Prototype](https://www.nationalcollection.org.uk/n-rich-prototype)
