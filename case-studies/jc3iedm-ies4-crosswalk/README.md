# JC3IEDM ↔ IES4 Crosswalk (Sketch)

A **sketch** crosswalk between JC3IEDM (Joint Consultation, Command and Control Information Exchange Data Model, NATO STANAG 5525) and IES4 (Information Exchange Standard, UK MOD / DBT). Closes [#26](https://github.com/fabio-rovai/open-ontologies/issues/26).

## Status

**Sketch, not authoritative.** This is a first-mover public artifact that establishes a methodology and provides ~10 entity-level mappings for review. It is NOT a complete or NATO-endorsed crosswalk. The intent is:

1. Make the methodology explicit and critiqueable
2. Provide a runnable starting point that integrates with `onto_align` and `onto_shacl_check`
3. Demonstrate the IES4 4D model's expressive coverage for ER-shaped JC3IEDM entities
4. Trigger expert review

A production crosswalk would need:
- NATO/MIP authorship or sign-off
- Coverage of all 271 JC3IEDM entities (this sketch covers 10)
- Treatment of JC3IEDM attributes and relationships (not just entity types)
- Bidirectional transformation logic, not just SKOS mappings
- Validation against operational MIP data

## Why this exists

Per the May 2026 IES4 ecosystem research (recorded in [memory](../../../../.claude/projects/-Users-fabio/memory/) under `project_open_ontologies_v0_2.md`):

> No public JC3IEDM ↔ IES4 crosswalk exists as of May 2026. JC3IEDM is ER-shaped and operational-C2; IES4 is 4D-ontological and intelligence-shaped. A crosswalk is almost certainly being built privately inside the IES Working Group (FMN Spiral 3 land-C2 profile is the natural pull) but nothing public.

This case study fills the public gap with a documented sketch. It can be cited as Open Ontologies' contribution to UK/NATO defence-modelling discussions.

## Methodology

For each JC3IEDM entity:

1. **Identify the IES4 nearest neighbour.** Walk the IES4 4D taxonomy (Particular vs ClassOfEntity vs Event vs State) and pick the class whose extension covers the JC3IEDM entity's instances. Prefer specificity (a JC3IEDM PERSON maps to IES4 Person, not to the abstract Particular).
2. **Choose the SKOS relation:**
   - `skos:exactMatch`: when the IES4 class's extension matches the JC3IEDM entity's intended population exactly. Rare in this sketch; the 4D vs snapshot difference usually prevents true exactMatch.
   - `skos:closeMatch`: when the IES4 class covers the JC3IEDM entity's population but adds 4D structure (time-extents, role-states) that JC3IEDM treats as attributes. Used for most mappings here.
   - `skos:relatedMatch`: when the IES4 class is conceptually adjacent but not a substitution candidate. Used for cases where the semantic gap is too wide for a straight match.
3. **Document the gap.** For every mapping, the crosswalk TTL includes an `rdfs:comment` explaining what the IES4 class adds, removes, or reframes vs the JC3IEDM entity.
4. **Use a placeholder JC3IEDM IRI namespace.** STANAG 5525 doesn't issue canonical web IRIs; this sketch uses `http://example.org/jc3iedm/` as a placeholder. A real implementation would use NATO/MIP-issued IRIs.

## Coverage

10 JC3IEDM entities → IES4 classes:

| JC3IEDM entity | IES4 class | Relation | Note |
|---|---|---|---|
| PERSON | `ies:Person` | `skos:closeMatch` | IES4 adds 4D temporal extent + role/state pattern |
| ORGANIZATION | `ies:Organisation` | `skos:closeMatch` | Same |
| MATERIEL | `ies:Asset` | `skos:closeMatch` | IES4 tracks possession-state explicitly |
| FACILITY | `ies:BuiltStructure` | `skos:closeMatch` | IES4 treats location as separate 4D extent |
| LOCATION | `ies:Location` | `skos:exactMatch` | Both are spatial Particulars; closest match in the set |
| EVENT | `ies:Event` | `skos:closeMatch` | IES4 requires participants (per the `ies4` enforce rule); JC3IEDM EVENT is looser |
| ACTION | `ies:CommunicationEvent` ⊔ `ies:Process` | `skos:relatedMatch` | JC3IEDM ACTION subsumes both deliberate processes and communication acts; needs splitting |
| CAPABILITY | `ies:PowerOfDisposition` | `skos:closeMatch` | Both encode a ZUR (ability) over actions; IES4 attaches it to a state |
| REPORTING-DATA | `ies:Representation` | `skos:closeMatch` | IES4 frames reports as Representations of underlying particulars |
| AFFILIATION | `ies:MemberState` | `skos:closeMatch` | JC3IEDM treats as attribute; IES4 models as state-of-being-member |

## Files

- [`crosswalk.ttl`](crosswalk.ttl): the SKOS mappings, runnable through Open Ontologies' tooling
- [`verify.sh`](verify.sh): checks the IES4 IRIs in the crosswalk resolve against `IES-Org/ont-ies` (live) and `dstl/IES4` v4.3.1 (frozen MIT baseline, via `onto_marketplace install ies-4.3.1`)

## Usage

```bash
# Load IES4 (frozen v4.3.1 baseline) as the validation target
open-ontologies marketplace install ies-4.3.1

# Validate the crosswalk's IES4 references structurally
open-ontologies shacl-check case-studies/jc3iedm-ies4-crosswalk/crosswalk.ttl
# Note: this tool checks for missing target classes / paths / class constraints,
# not SKOS-mapping validity. It will catch typos in IES4 IRIs.

# Or run the convenience script
bash case-studies/jc3iedm-ies4-crosswalk/verify.sh
```

## How to contribute / criticise

PRs welcome. Useful contributions in priority order:

1. **Correct misalignments**: if you have JC3IEDM expertise and an IES4 class is wrong for a given entity, open a PR with the fix and a rationale.
2. **Extend coverage**: pick a JC3IEDM entity from the remaining 261 and add a mapping.
3. **Add bidirectional transformation logic**: beyond SKOS, the real work is mapping JC3IEDM attributes and relationships to IES4's 4D patterns.

## References

- STANAG 5525 (JC3IEDM specification, NATO restricted)
- [IES Information Exchange Standard](https://informationexchangestandard.org/) (canonical portal; custodied by UK DBT since March 2025)
- [`IES-Org/ont-ies`](https://github.com/IES-Org/ont-ies) (canonical IES repo)
- [`dstl/IES4`](https://github.com/dstl/IES4) (archived; tag `v4.3.1` is the last MIT-licensed snapshot, available in this repo's marketplace as `ies-4.3.1`)
- FOUST 7 paper, "Comparing IES and BORO" (CEUR Vol-4176, JOWO 2024)

## Licence

MIT, matching the rest of Open Ontologies.
