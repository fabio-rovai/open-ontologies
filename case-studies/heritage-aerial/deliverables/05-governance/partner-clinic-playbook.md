# Partner Clinic Playbook

Operational guide for engaging GLAM institutions in NAPH adoption. The 4-step partner clinic is the recommended engagement model for institutions applying NAPH to a representative subset of their collection — generating evidence, refining the standard, and building institutional capability.

## 1. Purpose of the clinic

A partner clinic is a structured engagement with a GLAM institution to:

- Apply the standard to their actual collection (or a representative subset)
- Generate evidence of cost, capability gaps, workflow implications
- Refine the standard based on what doesn't work in practice
- Build institutional capability to continue NAPH adoption beyond the engagement

The clinic is **collaborative, not extractive**. The standard is refined as much as it is applied. Outputs are shared between institution and Editorial team.

## 2. The 4-step cycle (per institution)

```
Step 1 — Readiness & Scoping (2-3 hrs)
   ↓
Step 2 — Application Session (half-day, 4 hrs)
   ↓
Step 3 — Computational Readiness Check (2-3 hrs)
   ↓
Step 4 — Reflection & Refinement (1-2 hrs)
   ↓
Output: Application case study + spec refinements + cost evidence
```

Total time commitment per institution: **~12 hours** of active engagement, spread over 4-6 weeks.

## 3. Step 1 — Readiness & Scoping

**Format:** 2-3 hour video call with institutional team
**Participants:** Editorial team (Programme Manager + Technical Lead) + institutional team (collections manager + digital officer + project sponsor)
**Output:** scoping document with agreed test subset, current-state assessment, and engagement plan

### 3.1 Pre-call preparation

The Editorial team sends, 1 week before:

- Brief readiness questionnaire (15 questions, ~30 min to complete)
- Link to the [Standard v1.0](../01-standard/NAPH-STANDARD.md) (executive summary only)
- Sample data for context (the aerial photography case study)

The institution prepares:

- Sample of 100 records from their collection — CSV, XML, or database export
- Existing rights vocabulary (text strings used)
- Existing identifier scheme documentation (if any)
- Names and roles of internal staff who will be involved

### 3.2 Call agenda

| Time | Activity |
|---|---|
| 0:00 - 0:15 | Introductions, programme context, what we mean by "clinic" |
| 0:15 - 0:45 | Walkthrough of NAPH Standard at the institution's collection level |
| 0:45 - 1:30 | Joint review of institution's sample data — identify Baseline gaps |
| 1:30 - 2:00 | Scope agreement — which records get applied, which tier, what timeline |
| 2:00 - 2:30 | Engagement plan — who does what, when, how feedback flows |

### 3.3 Scoping document template

The scoping document records:

- **Institution:** name, primary contact, sponsor
- **Collection scope:** which sub-collection, how many records, why this subset
- **Target tier:** Baseline / Enhanced / Aspirational (clinic typically targets Baseline first)
- **Current state:** identifier scheme, rights vocabulary, date format, geographic representation
- **Known constraints:** legal restrictions, technical blockers, internal capacity
- **Success criteria:** what does "successful application" look like for this institution
- **Risks:** what could prevent success, and mitigations

Template: [`07-templates/clinic-scoping-template.md`](../07-templates/clinic-scoping-template.md)

## 4. Step 2 — Application Session

**Format:** half-day workshop (4 hours), can be in-person or video
**Participants:** Technical lead + institutional digital officer + (optional) institutional ML/data engineer
**Output:** sample subset transformed to NAPH-compliant form; mapping documentation

### 4.1 Pre-session preparation

Editorial team prepares:

- Pre-configured ingest pipeline targeted at the institution's sample format
- Mapping configuration draft (rights text → URI, date format conventions)
- Preliminary identifier minting policy proposal

Institution prepares:

- Slightly larger sample (1,000-5,000 records)
- Permission to test transformations without affecting their primary system
- Working environment (their laptop, repository, sandbox)

### 4.2 Session agenda

| Time | Activity |
|---|---|
| 0:00 - 0:30 | Quick recap; agree session outputs |
| 0:30 - 1:30 | Run ingest pipeline on sample; review output Turtle |
| 1:30 - 2:30 | Identify what worked, what didn't — refine mapping config together |
| 2:30 - 3:00 | Run SHACL validation; review violations |
| 3:00 - 3:30 | Generate validation report; review competency-question results |
| 3:30 - 4:00 | Discuss next steps — what's needed to scale to full collection |

### 4.3 Common findings during application

From the case study and likely from real institutions:

- Date formats more variable than expected — additional patterns needed
- Rights vocabulary has institution-specific entries that don't map cleanly to rightsstatements.org
- Identifier scheme has historical inconsistencies (different prefixes used over time)
- Geographic data may be only at sortie level, not per-frame
- Internal data has fields that NAPH doesn't represent — opportunity for profile extension

These findings feed back into the standard via [RFCs](rfc-process.md) where general; or via institution-specific extensions where bespoke.

## 5. Step 3 — Computational Readiness Check

**Format:** 2-3 hour focused work session
**Participants:** Technical lead + institutional digital officer
**Output:** evidence that the transformed data actually supports research workflows

### 5.1 The point of this step

Module F.A.1 says: "computational reuse tests... actually execute typical research queries against the collection and assert they return correct results."

Step 3 is where this happens during the clinic. We don't just check the schema — we actually try to use the data.

### 5.2 Test workflow examples

For an aerial photography collection, the readiness check runs:

1. **Spatial query:** "find all records covering [a specific geographic polygon]"
2. **Temporal range:** "find records captured between [date X] and [date Y]"
3. **Cross-attribute filter:** "find records with [specific aircraft type] at [specific altitude range]"
4. **Provenance audit:** "find all records originating from [specific transfer event]"
5. **Open-rights subset:** "find all records with rights cleared for open-access publication"
6. **Cross-collection link health:** "for records claiming Wikidata links, check resolution"

For each test, the result is recorded:

- Did the query execute? (binary)
- Did it return the expected records? (validation)
- How long did it take? (performance baseline)

### 5.3 Findings from real readiness checks

Findings often include:

- A query that should return N records but returns N-3 — investigation shows 3 records have inconsistent metadata
- A spatial query that fails because the SPARQL endpoint doesn't support GeoSPARQL — implementation gap
- Acceptable performance for 100,000 records but degradation predicted at 1,000,000 — capacity planning needed
- External authority links that have moved or been deprecated — link health monitoring required

These become input to either institutional remediation or standard refinement (RFCs).

## 6. Step 4 — Reflection & Refinement

**Format:** 1-2 hour video call
**Participants:** Editorial team (full) + institutional team (full)
**Output:** clinic case study; standard refinements; cost evidence

### 6.1 Call agenda

| Time | Activity |
|---|---|
| 0:00 - 0:20 | Recap of the cycle — what was done, what was learned |
| 0:20 - 0:50 | Standard refinements — what should change in the spec based on this engagement |
| 0:50 - 1:20 | Cost evidence — actual hours, actual blockers, actual learning curve |
| 1:20 - 1:50 | Future plans — what does the institution do next, what support would help |
| 1:50 - 2:00 | Wrap, agreed next-step actions, agreed publication of case study |

### 6.2 Clinic case study

Each partner engagement produces a published case study:

- Anonymous or named depending on institutional preference
- Documents what was done, what worked, what didn't
- Includes cost data (where institution agrees to share)
- Co-authored by Editorial team and institutional team

These case studies are durable evidence outputs of NAPH.

### 6.3 Standard refinement ledger

The clinic cycle produces an explicit list of "what would have made this easier."

Each item is triaged:

- **Spec change needed** → RFC opened
- **Tooling change needed** → ingest pipeline / validation toolkit issue
- **Documentation change needed** → editorial PR
- **Out of scope** → recorded for future consideration

The refinement ledger is a public artefact of each clinic.

## 7. Across-clinic synthesis

After 3-5 clinic cycles, the Editorial team produces a **synthesis report**:

- Patterns observed across institutions
- Common refinement themes (clusters of similar findings)
- Costs distributions (faster than expected? slower?)
- Capability gaps consistently observed
- Recommendations for v1.1 / v1.2 spec changes

The synthesis report is the major output of NAPH's testing-evidence stream.

## 8. Selection of partners

**Target:** 3-5 institutions for NAPH's funded application phase.

**Selection criteria:**

- Mix of scales: at least one large (>1M records), one medium, one small
- Mix of collection origins: at least one from RAF/Crown sources, one from NARA/US sources, one from a third source
- Geographic diversity: not all London-based
- Capacity diversity: at least one institution with low technical capacity (so we evidence what's needed for sector-wide adoption, not just well-resourced sites)
- Willingness: institutions that can commit ~12 hours over 4-6 weeks

**Recommended candidate pool:**

- NCAP / HES (primary host candidate; ~30M records)
- Imperial War Museums
- The National Archives
- Royal Air Force Museum
- A regional or local-authority aerial archive
- A research institution with aerial photography holdings (e.g. RCAHMS, archaeology departments)

## 9. Resourcing per clinic

Estimated Editorial team time per partner:

- Step 1 (Readiness): 4 hours per side (incl. prep)
- Step 2 (Application): 6 hours per side
- Step 3 (Readiness check): 4 hours per side
- Step 4 (Reflection): 3 hours per side
- Case study writing: 8 hours per Editorial team

**Total per partner: ~25 hours of Editorial team time** + ~12 hours of institutional team time

For 5 partners: ~125 hours of Editorial team effort + 60 hours institutional engagement.

This is roughly £15,000-£20,000 at typical Pilot-team rates — a significant but tractable commitment within the contract budget.

## 10. Outputs

For each clinic:

- Scoping document (institutional)
- Application output (sample TTL)
- Validation report (HTML)
- Computational readiness check log (JSON)
- Clinic case study (Markdown, published)
- Refinement ledger entries (issues / RFCs)

Across NAPH:

- Synthesis report
- Cost-evidence dataset (anonymised cost numbers)
- Standard v1.x update incorporating clinic-driven refinements

## 11. After NAPH

Post-Pilot, partner institutions are encouraged to:

- Continue tier progression at their own pace
- Maintain Standards Council representation (if invited)
- Contribute case studies as they reach new milestones
- Refer their peers to the standard

The Editorial team commits to remain a contactable resource for ~12 months post-contract for partners' follow-up questions.

## Cross-references

- [Governance proposal](governance-proposal.md)
- [RFC process](rfc-process.md)
- [Adoption guidance](../04-adoption-guidance/how-to-use-this-standard.md)
- [Clinic scoping template](../07-templates/clinic-scoping-template.md)
- [Validation toolkit](../../pipeline/generate-report.py)
