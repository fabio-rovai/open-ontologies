# RFC Process

How substantive changes to NAPH are proposed, debated, decided, and merged. Inspired by the Rust, IETF, and W3C processes but scaled appropriately for a single-vertical standard.

## 1. When to write an RFC

An RFC is required for changes that:

- Add, modify, or remove classes or properties in `naph-core.ttl`
- Add, modify, or remove SHACL shapes in `naph-shapes.ttl`
- Change tier definitions or compliance requirements
- Change the standard's scope (e.g. adding adjacent verticals)
- Change governance, including this RFC process itself
- Make claims about external standards or authorities (e.g. "NAPH aligns with X" requires X to actually align)

An RFC is **not** required for:

- Typo corrections, formatting, examples
- Adding worked examples without changing semantics
- Updating cross-references between docs
- Bug fixes in pipeline scripts that don't change behaviour
- Operational documentation updates

If unsure, open an issue first — the Editor will indicate whether an RFC is needed.

## 2. RFC lifecycle

```
draft → public comment → council review → decision → merged
                                             ↓
                                          declined
                                             ↓
                                         postponed
```

### 2.1 Draft

Author opens a pull request to the `rfcs/` directory containing:

- A new file `rfcs/NNNN-short-title.md` (NNNN = next available number, padded)
- Following the [RFC template](#5-rfc-template)
- Marked as "Draft" in title

The Editor triages within 5 working days:

- Confirms RFC is needed (vs. editorial)
- Assigns the RFC number formally
- Reviews the structure, asks for clarifications
- Marks the RFC as "Public Comment" when ready

### 2.2 Public comment

Minimum 30 days; longer for complex RFCs.

During public comment:

- Anyone can comment on the PR
- The author MUST respond to substantive comments
- Comments and responses are public and form part of the decision record
- Author MAY revise the proposal; revisions extend the comment period by 14 days

### 2.3 Council review

After public comment closes, the Standards Council reviews the RFC:

- Scheduled for the next monthly Council meeting
- Council members read the RFC and comments before the meeting
- Council recommends: accept / reject / defer / accept with revisions
- Recommendation is published as a comment on the PR

### 2.4 Decision

The Steward makes the final decision:

- Within 14 days of Council recommendation
- Decisions follow Council recommendation in most cases
- Decisions against Council recommendation MUST include written rationale
- Decisions are published as a comment on the PR

### 2.5 Merge

Accepted RFCs are merged with:

- The version number they target (e.g. v1.2)
- A link to the implementing PRs
- Updated changelog entry

Declined RFCs are closed with the decision rationale visible.

Postponed RFCs are kept open with a "Postponed" label and may be revisited.

## 3. Decision criteria

The Standards Council and Steward consider:

| Criterion | Weight | Question |
|---|---|---|
| Necessity | High | Is the change actually needed? Is the existing spec genuinely insufficient? |
| Backwards compatibility | High | Does the change preserve validity of existing NAPH-compliant records? |
| Standards alignment | Medium-high | Does the change move toward or away from W3C/OGC/IIIF standards? |
| Implementation cost | Medium | What does the change cost adopting institutions? |
| Maintenance cost | Medium | What does the change cost the Steward and tooling maintainers? |
| Domain authenticity | High | For aerial-specific changes — does it match real archival practice? |
| Author commitment | Low-medium | Is the author committed to implementing the change? |

A change that scores high on Necessity, Backwards Compatibility, and Standards Alignment is normally accepted. A change that scores poorly on Backwards Compatibility requires strong Necessity.

## 4. Backwards compatibility

For v1.x, all RFCs MUST be backwards-compatible:

- New properties may be added; existing properties cannot be removed or have semantics changed
- New classes may be added; existing classes cannot be removed
- New SHACL shapes may be added; existing shapes cannot be made stricter
- New tier requirements MAY be added at higher tiers but MUST NOT raise the bar at any existing tier

Breaking changes are reserved for v2.0+ and require a separate migration plan, deprecation period, and tooling.

## 5. RFC template

```markdown
# RFC NNNN: [Short descriptive title]

- **Author:** [name + affiliation]
- **Status:** Draft / Public Comment / Council Review / Accepted / Declined / Postponed
- **Targets version:** v1.x
- **Created:** YYYY-MM-DD
- **Discussion:** [link to PR]

## Summary

One paragraph plain-English description of the proposed change.

## Motivation

What problem does this solve? Why is the existing spec insufficient?
Provide concrete evidence: example records that can't be modelled, queries that can't be expressed, institutional adoption blockers.

## Detailed design

Specific proposed changes:

- New / modified / removed classes
- New / modified / removed properties
- New / modified / removed shapes
- Documentation changes

Include exact Turtle syntax for ontology changes.

## Alternatives considered

What other approaches were considered? Why is this the recommended one?

## Backwards compatibility

Does this change preserve validity of existing v1.x records?
If not, why is the change unavoidable? What's the migration path?

## Implementation

What needs to change:

- [ ] Update `naph-core.ttl`
- [ ] Update `naph-shapes.ttl`
- [ ] Update relevant module spec
- [ ] Update aerial-photography profile
- [ ] Update validation pipeline
- [ ] Update worked examples
- [ ] Update changelog

Estimate: [hours/days/weeks of editor and tooling-maintainer work]

## Risks

- Domain authenticity risk: [...]
- Adoption risk: [...]
- Standards-alignment risk: [...]

## Adoption indicators

How will we know if this RFC, once merged, has achieved its goal? What metric improves?

## Cross-references

- [Relevant module specs]
- [Existing related RFCs]
- [External standards being aligned with]
```

## 6. Editor responsibilities

The Editor:

- Triages incoming RFCs within 5 working days
- Provides editorial feedback on draft RFCs
- Schedules Council review
- Maintains the canonical RFC index
- Publishes decisions
- Implements accepted RFCs in the relevant artefacts (or oversees implementation)

The Editor may **not** make substantive decisions about whether to accept or reject an RFC; that's the Steward's role with Council advice.

## 7. Standards Council responsibilities

For each RFC in Council Review:

- Read the RFC and public comments
- Discuss in the monthly meeting
- Provide written recommendation: accept / reject / defer / accept-with-revisions
- Disclose any conflicts of interest

Council meetings are open to public observation; minutes are published.

## 8. Worked example — a hypothetical RFC walkthrough

To illustrate, here's how an RFC for adding a `naph:declassificationEvent` property would typically flow:

### Day 1 — Draft

Author opens PR with `rfcs/0007-declassification-event.md`. Editor triages, confirms it's substantive, assigns RFC #7, asks author to clarify how this relates to existing `prov:Activity` declassification modelling.

### Day 1-7 — Pre-comment iteration

Author revises to clarify `naph:declassificationEvent` is a typed shortcut for the common case while remaining `prov:Activity`-compatible. Editor marks "Public Comment".

### Day 7-37 — Public comment

Comments from:

- IWM cataloguer: supports, gives example records this would simplify
- Linked-data specialist: questions whether this duplicates `prov:Activity` modelling
- NCAP archivist: supports, notes operational benefit

Author responds, revises proposal slightly. Comment period extended 14 days due to revision (Day 51).

### Day 51 — Council review

Council meets, reviews. Recommendation: **accept**, with note that the property should be a sub-class of `prov:Activity` (not replace it).

### Day 65 — Decision

Steward accepts with the Council-recommended revision. Author merges PR.

### Subsequent

The next MINOR release (e.g. v1.3) includes `naph:declassificationEvent` as a new typed Activity. Migration guide notes the new shortcut for institutions modelling declassification events.

Total elapsed time: ~9 weeks from draft to merge.

## 9. Special-case processes

### 9.1 Security or critical bug

If an RFC fixes a critical issue (e.g. ontology contains a circular subclass reference), the public comment period may be reduced to 7 days at the Editor's discretion.

### 9.2 External-standards alignment

If a parent standard (e.g. W3C DCAT) makes a change that requires NAPH alignment, the RFC may be expedited and Council recommends acceptance based on parent-standard requirements.

### 9.3 Domain expert challenge

If domain experts (typically aerial archivists) raise a substantive challenge to an existing modelling decision, an RFC may be opened to revise the decision. These are treated with high weight on the Domain Authenticity criterion.

## 10. Anti-patterns

The RFC process is intended to be lightweight. Avoid:

- **Bureaucratic creep** — RFCs that add procedural overhead without substance
- **Premature abstraction** — RFCs that add capability for hypothetical future use
- **Standards collision** — RFCs that override or contradict W3C/OGC/IIIF standards (these will normally be rejected)
- **Vendor lock-in** — RFCs that mandate specific vendor tooling

## 11. Annual RFC review

Each year, the Standards Council reviews:

- All RFCs accepted in the past year — implemented as expected? Adopted?
- All RFCs declined — has new evidence emerged that should re-open them?
- All RFCs postponed — should they be revisited or closed?

This review is published as part of the [annual adoption report](governance-proposal.md#8-adoption-metrics-and-transparency).

## Cross-references

- [Governance proposal](governance-proposal.md)
- [Maintenance runbook](../06-knowledge-transfer/maintenance-runbook.md)
- [Standards Council charter](../07-templates/standards-council-charter-template.md)
