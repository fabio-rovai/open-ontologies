# Partner Clinic — Scoping Document Template

This template is filled at the end of [Step 1 of the partner clinic](../05-governance/partner-clinic-playbook.md#3-step-1--readiness--scoping). Copy and complete with the institution.

---

## Institution

- **Name:** [Full institutional name]
- **Trading as:** [if applicable]
- **Primary contact:** [name, role, email, phone]
- **Sponsor:** [senior contact authorising engagement]
- **Other participants:** [collections manager, digital officer, archivist, ML/data engineer, etc.]

## Engagement context

- **Date of scoping call:** YYYY-MM-DD
- **Editorial team participants:** [Editorial lead, Technical Lead]
- **Engagement period:** [start date — end date, typically 4-6 weeks]

## Collection scope

- **Total holding:** [N records]
- **Subset selected for clinic:** [N records, ~M% of holding]
- **Subset rationale:** [why this subset — e.g. "high-research-value subset of declassified WW2 reconnaissance" or "representative sample across collection types"]
- **Subset characteristics:** [date range, primary collection origin (RAF/NARA/DOS/etc.), capture orientations, special features]

## Target tier

- **Aim:** [Baseline / Enhanced / Aspirational]
- **Realistic outcome by end of clinic:** [may be partial — e.g. "Baseline for sample subset; Enhanced specification for next steps"]
- **Tier progression beyond clinic:** [intended pathway — e.g. "Baseline rolled out to full collection by Q4; Enhanced for declassified subset by 12 months out"]

## Current state

### Identifier scheme

- **Existing scheme:** [description of current internal codes, e.g. "Collection / Sortie / Frame composite, e.g. RAF/106G/UK/1655/4023"]
- **Resolvability:** [yes / partial / no — and how]
- **Persistence guarantee:** [yes / unclear / no]
- **Action under NAPH:** [adopt as-is via URI wrapper / mint new scheme / hybrid]

### Date format

- **Current format(s):** [list — e.g. "DD/MM/YYYY for post-1980 records, free-text 'March 1944' for pre-WW2 catalogue"]
- **Convention assumed:** [DD/MM/YYYY / MM/DD/YYYY / mixed]
- **Action under NAPH:** [normalise to ISO 8601 / partial date support needed]

### Rights vocabulary

- **Existing strings used:** [list — e.g. "Crown Copyright", "Public Domain (US)", "RCAHMS Copyright", "Other"]
- **Mapping to rightsstatements.org:** [describe mapping per category]
- **Action under NAPH:** [adopt mapping table / case-by-case review for ambiguous cases]

### Geographic representation

- **Format:** [point lat/lon / footprint polygon / sortie-level only / other]
- **CRS:** [WGS84 / OSGB36 / other / unknown]
- **Action under NAPH:** [convert to WGS84 polygon / derive footprint from altitude+focal-length / hybrid]

### Digitisation provenance

- **Recorded:** [yes / partial / no]
- **Format:** [internal database fields / paper records / not centrally tracked]
- **Action under NAPH for Enhanced:** [extract from internal systems / catalogue retrospectively / mark as best-effort for older records]

### Existing aggregator participation

- **Europeana:** [yes / no / planned]
- **National portals:** [yes / no — which]
- **Action:** [maintain compatibility / improve via NAPH]

## Known constraints

| Constraint | Description | Impact | Mitigation |
|---|---|---|---|
| Legal | [e.g. donor agreement restrictions on specific subsets] | [...] | [...] |
| Technical | [e.g. legacy database tooling, no API access] | [...] | [...] |
| Capacity | [e.g. only 0.2 FTE digital officer time] | [...] | [...] |
| Data quality | [e.g. ~5% of records have unresolvable date fields] | [...] | [...] |

## Success criteria

What does "successful clinic" look like for this institution?

- [Criterion 1 — e.g. "Sample of 1,000 records validates clean against NAPH Baseline shapes"]
- [Criterion 2 — e.g. "Cost evidence collected for full-collection rollout estimate"]
- [Criterion 3 — e.g. "Clinic case study published with attribution"]
- [Criterion 4 — e.g. "At least 3 standard refinements proposed via RFC"]

## Risks

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| [Risk 1 — e.g. "Internal data export blocked by IT change-control"] | [Med] | [High] | [...] |
| [Risk 2 — e.g. "Rights vocabulary mapping disputed by legal team"] | [Low] | [Med] | [...] |

## Engagement plan

| Date | Activity | Editorial team | Institutional team | Output |
|---|---|---|---|---|
| YYYY-MM-DD | Pre-application data preparation | [TL email follow-up] | [Digital officer extracts sample] | Sample CSV |
| YYYY-MM-DD | Step 2 — Application Session | [TL onsite/remote] | [Digital officer + ML engineer] | Generated TTL + mapping config |
| YYYY-MM-DD | Step 3 — Computational Readiness Check | [TL onsite/remote] | [Digital officer] | Readiness check log |
| YYYY-MM-DD | Step 4 — Reflection & Refinement | [PM + TL] | [Sponsor + collections manager + digital officer] | Case study + refinement ledger |

## Communication

- **Default channel:** [email / Slack / Teams]
- **Document repository:** [shared Drive / GitHub repo / institutional intranet]
- **Cadence:** [weekly check-in / ad-hoc / fortnightly stand-up]

## Sign-off

- **Editorial lead:** [Name, signature, date]
- **Technical lead:** [Name, signature, date]
- **Institutional sponsor:** [Name, signature, date]

---

## Cross-references

- [Partner clinic playbook](../05-governance/partner-clinic-playbook.md)
- [How to use this standard](../04-adoption-guidance/how-to-use-this-standard.md)
- [Validation checklists](../04-adoption-guidance/validation-checklists.md)
