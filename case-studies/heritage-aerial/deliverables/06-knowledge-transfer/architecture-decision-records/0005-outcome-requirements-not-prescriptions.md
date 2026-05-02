# ADR-0005: Outcome requirements over prescriptive workflows

**Status:** Accepted
**Date:** 2026-04-30
**Decider:** Editorial team

## Context

The standard could specify:

(a) Specific tools, vendors, and workflows that institutions MUST use (prescriptive)
(b) Outcomes that records MUST demonstrate, regardless of how achieved (outcome-based)

Most existing digitisation guidance is prescriptive — "scan at 600 DPI using a Phase One scanner with X colour profile." This works in some contexts but:

- Ages poorly as technology evolves
- Forces institutions to replace working equipment
- Creates vendor-lock-in concerns
- Doesn't accommodate variation in source materials

## Decision

NAPH specifies **outcome requirements**, not workflows. For each module, the spec says what the resulting record must demonstrate, not how to produce it.

Example, [Module A](../../01-standard/modules/A-capture-imaging.md) says:

> A Baseline-compliant digital surrogate MUST be a single uncompressed or losslessly-compressed image file (TIFF, JP2 lossless, PNG)

NOT:

> Use Phase One IQ4 scanners with Capture One processing at 1200 DPI and AdobeRGB colour profile

The institution chooses how to meet the outcome. Multiple paths to the same outcome are acceptable.

## Consequences

### Positive

- **Long-term durability** — outcomes don't depend on specific tooling that will become obsolete
- **No vendor lock-in** — institutions choose hardware, software, contractors freely
- **Accommodates source variation** — a panchromatic 35mm aerial negative requires different handling from a glass plate or born-digital file; outcome specs accommodate both
- **Reduces resistance to adoption** — institutions don't have to throw away existing equipment or retrain on mandated tools
- **Easier sector-wide adoption** — institutions with diverse capability levels can all meet the same outcomes via different paths

### Negative

- **Less prescriptive guidance** — institutions need their own competence to translate outcomes to actual workflows. Some institutions want more direction
- **Requires more validation discipline** — without a prescribed workflow, the only check is whether the outcome is met. This puts pressure on the validation toolkit
- **Harder to write** — outcome specifications are harder to draft than workflow specifications. Requires careful thought about what's essential vs. accidental

### Neutral

- This decision doesn't preclude **publishing reference workflows** as informative documents alongside the spec. It just means those workflows are not normative.

## Alternatives considered

### Alternative 1: Prescriptive workflows

Rejected because:

- Standards that prescribe technology age out within 5-10 years (think: "scan to JPEG at 96 DPI" guidance from the early 2000s)
- The standard would become obsolete or perpetually-revised
- Vendor-locked institutions resist adoption
- Source-material variation isn't accommodated

### Alternative 2: Hybrid — outcomes for some modules, prescriptive for others

Considered for Module A (capture) where one might argue "TIFF 1200 DPI" is universal enough to prescribe. Rejected because:

- Even Module A varies — multispectral satellite imagery needs different handling from monochrome aerial film
- Hybrid models invite scope creep — "if Module A is prescriptive, why not Module D?"
- Easier to be consistent: outcomes throughout

### Alternative 3: Mandate adherence to existing workflow standards (e.g. FADGI)

Rejected because:

- FADGI is itself outcome-based for the most part
- Requiring NAPH-compliance + FADGI-compliance doubles the validation burden
- NAPH outcomes can be aligned with FADGI guidance informally without normative requirement

## Implementation

For each MUST/SHOULD requirement in the modules, the language is:

- "MUST be a [class of outputs]" — outcome
- "MUST satisfy [property]" — outcome
- "MUST resolve to [class of representations]" — outcome
- NOT: "MUST use [specific tool]"
- NOT: "MUST produce by [specific workflow]"

The [adoption guidance](../../04-adoption-guidance/how-to-use-this-standard.md) provides reference workflows for institutions that want them, but these are non-normative.

## Risks and mitigations

### Risk: institutions interpret outcomes too liberally

Mitigation: SHACL validation provides automated checking of objectively-verifiable outcomes. Sample-based human review handles the rest. Failed validation is the visible signal that an outcome isn't being met.

### Risk: institutions need more guidance than outcomes provide

Mitigation: provide reference workflows in [adoption guidance](../../04-adoption-guidance/) as informative supplements. The partner clinic ([playbook](../../05-governance/partner-clinic-playbook.md)) explicitly addresses this — institutions can ask "what would this look like for our setup?"

### Risk: outcome specs are too vague to validate

Mitigation: every MUST in the modules is paired with a SHACL shape or other verification mechanism. If we can't validate it, we don't put it in the spec.

## Validation

This decision is validated by:

- Module specs: every requirement has a corresponding validation mechanism (SHACL shape, syntactic check, or sampling-based review)
- The case study: applied to NCAP-style data, the outcome requirements produced compliant records via reasonable workflows
- Adversarial CSV testing: outcome-based date/coordinate/rights validation catches the actual problems institutions have, regardless of how the source data was produced

## Cross-references

- [Module specifications](../../01-standard/modules/) — every module follows this pattern
- [Adoption guidance](../../04-adoption-guidance/how-to-use-this-standard.md) — reference workflows
- [Validation checklists](../../04-adoption-guidance/validation-checklists.md)
- [FADGI guidelines](https://www.digitizationguidelines.gov/) — example of an outcome-aligned external standard
