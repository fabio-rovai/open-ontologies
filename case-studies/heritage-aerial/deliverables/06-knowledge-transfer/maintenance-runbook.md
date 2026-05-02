# Maintenance Runbook — for HES (or successor Steward)

Operational handover document. Designed so HES can take over NAPH stewardship at end-of-Pilot without bringing the Editorial team back for routine work.

This runbook assumes the reader is the **operational Steward** — the person at HES (or successor institution) responsible for keeping NAPH alive day-to-day.

## 1. What you've inherited

```
Repository: github.com/<org>/naph (or equivalent under HES namespace)
├── ontology/
│   ├── naph-core.ttl
│   └── naph-shapes.ttl
├── data/
│   └── canonical-sample/        # 10-record reference dataset
├── pipeline/
│   ├── ingest.py
│   ├── iiif-bridge.py
│   ├── generate-report.py
│   └── self-assessment.py
├── deliverables/                # Spec, modules, profile, governance
├── rfcs/                        # RFC archive
├── reports/                     # Validation reports per release
├── .github/workflows/           # CI/CD
└── CHANGELOG.md
```

Public services:

- Permanent ontology URI: `https://w3id.org/naph/ontology` → resolves to canonical TTL
- Project website: `https://naph.standards.scot` (or chosen URL)
- Issue tracker: GitHub issues
- Mailing list / forum: tbd

## 2. Routine operations

### 2.1 Daily

Nothing required. The standard is static between releases.

### 2.2 Weekly

- Triage incoming GitHub issues (15-30 min)
- Respond to user questions on the discussion forum / mailing list
- Watch CI/CD failures and address them

### 2.3 Monthly

- Review pending pull requests
- Triage RFCs in their early stages
- Convene Standards Council meeting (if scheduled — typically every 1-2 months during active periods)

### 2.4 Quarterly

- Re-validate the canonical sample data against current spec
- Generate quarterly metrics for the adoption report
- Review any external standards updates (W3C, OGC, IIIF) for NAPH impact

### 2.5 Annually

- Convene Standards Council annual review
- Publish annual adoption report
- Plan next MINOR release roadmap
- Renew Standards Council appointments (rotation cycle)

## 3. Issue triage

When an issue arrives:

1. **Categorise** — bug / question / RFC candidate / out-of-scope
2. **Label** — `bug`, `question`, `rfc-needed`, `documentation`, `out-of-scope`
3. **Assign priority** — critical / high / medium / low
4. **Respond within 5 working days** — at minimum acknowledging receipt and rough timeline

### 3.1 Critical issues

Examples: ontology contains a circular reference; SHACL shapes contain a syntax error preventing validation; published URI doesn't resolve.

Response: fix and release a PATCH within 5 working days. Optionally use the expedited RFC process ([§9.1 of RFC process](../05-governance/rfc-process.md)).

### 3.2 RFC-needed issues

The reporter has identified something substantive that should change. Direct them to the [RFC process](../05-governance/rfc-process.md). Offer to help draft the RFC if they're new to the format.

### 3.3 Out-of-scope issues

Examples: requests for features in non-aerial verticals; requests for vendor-specific tooling; requests that contradict adopted RFCs.

Politely close with rationale. Reference [ADR-0001](architecture-decision-records/0001-narrow-vertical.md) if the request is to expand vertical scope.

## 4. Releasing

### 4.1 PATCH release (e.g. v1.0.1)

Use case: typo fix, errata, doc update, no semantic change.

Process:

1. Make changes via PR
2. Update `CHANGELOG.md`
3. Run validation suite (CI does this)
4. Tag the release (`git tag v1.0.1`)
5. Publish release notes via GitHub releases
6. Update the website's "current version" pointer

Time required: ~30 min once you've done it once.

### 4.2 MINOR release (e.g. v1.1.0)

Use case: accepted RFCs, additive changes, no breaking changes.

Process:

1. Confirm all RFCs targeting this version are merged
2. Update spec documents to reflect changes
3. Update ontology and shapes
4. Re-run full validation suite + canonical sample regression
5. Update worked examples
6. Update `CHANGELOG.md` with release notes
7. Run pre-release peer review (Editor + at least one Council member)
8. Tag the release
9. Publish announcements to mailing list, forum, project website
10. Update aggregator notifications (DPLA, Europeana if applicable)

Time required: 4-12 hours depending on scope.

### 4.3 MAJOR release (e.g. v2.0.0)

Use case: breaking changes. **Rare.**

Process:

1. RFCs accepted for v2.0 must be reviewed in aggregate
2. Migration guide MUST be written
3. Migration tooling MUST be available before release
4. v1.x line continues to be supported for at least 1 year post-v2.0 release
5. Public consultation of at least 90 days
6. Standards Council formal endorsement
7. Steward formal release decision

Time required: weeks to months. Engage external expertise if uncertain.

## 5. Validation pipeline operations

The validation pipeline runs:

- On every commit to `main` (via GitHub Actions)
- On every PR
- Quarterly via scheduled cron (regression check against canonical sample)

### 5.1 If CI fails

The pipeline runs in this order:

1. `open-ontologies validate ontology/naph-core.ttl` — basic Turtle syntax
2. `open-ontologies validate ontology/naph-shapes.ttl` — shapes syntax
3. `open-ontologies validate data/canonical-sample/sample-photographs.ttl` — sample data syntax
4. `open-ontologies batch < pipeline/full-validation.batch.txt` — load + SHACL
5. `python3 pipeline/self-assessment.py` — check all assertions
6. Generate validation report

Common failures and their remediation:

| Failure | Likely cause | Action |
|---|---|---|
| `validate` step fails | Turtle syntax error | Find the offending line; usually a missing `.` or `;` |
| SHACL violations on canonical sample | Sample data doesn't match new shape requirement | Either update sample to new shape OR back-out the breaking shape change |
| Self-assessment script error | Code regression | Roll back recent changes to `self-assessment.py` |
| Network failure during external link check | Wikidata / GeoNames temporarily unavailable | Retry; if persistent, mark check as warning not error |

### 5.2 Manual validation

To run validation locally without CI:

```bash
cd /path/to/naph-repo
open-ontologies batch < pipeline/full-validation.batch.txt
python3 pipeline/self-assessment.py data/canonical-sample/sample-photographs.ttl
python3 pipeline/generate-report.py > reports/manual-$(date -u +%Y-%m-%d).html
```

## 6. Permanent URI maintenance

The canonical ontology URI is `https://w3id.org/naph/ontology`. This is operated via the [w3id.org](https://w3id.org/) PR-based redirect service.

To update the redirect:

1. Fork [github.com/perma-id/w3id.org](https://github.com/perma-id/w3id.org)
2. Edit `naph/.htaccess` to point to the new canonical TTL location
3. Open a PR
4. PR review by w3id.org maintainers (~1-7 days)

The redirect MUST always point to the latest stable v1.x ontology TTL. Do NOT redirect to a pre-release or unstable version.

## 7. Working with external standards

### 7.1 Watching for changes

Monitor:

- W3C announcements for DCAT, PROV, SKOS updates
- OGC announcements for GeoSPARQL updates
- IIIF Consortium announcements
- rightsstatements.org announcements

When a parent standard changes:

1. Assess impact on NAPH (does anything we use change semantics?)
2. If breaking change in parent standard: open RFC, plan migration
3. If non-breaking: monitor; update documentation if relevant

### 7.2 Contributing back

Where the NAPH team identifies issues with parent standards (e.g. ambiguity, missing features), report upstream:

- W3C: through the relevant working group's tracker
- OGC: through their contribution process
- IIIF: through GitHub issues

Cite NAPH as the use case where helpful. This positions NAPH as a productive participant in the standards ecosystem.

## 8. Standards Council operations

### 8.1 Meeting cadence

Recommended:

- Monthly during active RFC periods
- Quarterly during steady state
- Annual review meeting always held

### 8.2 Meeting structure

Default agenda:

1. Open issues review (15 min)
2. RFCs in review (variable)
3. Adoption metrics update (5 min)
4. AOB

Meetings are open to public observation. Minutes are published within 7 days.

### 8.3 Recruitment

Standards Council appointments rotate every 2-3 years. Recruit:

- From institutions actively using NAPH
- Researcher voices via academic contacts
- Technical voices from linked-data community

Aim for diversity in institution size, geographic distribution, and career stage.

## 9. Adoption metrics

Quarterly, collect:

- Number of institutions claiming each tier of compliance (self-reported via a registry submission)
- Number of records published as NAPH-compliant (estimated from registry)
- Geographic coverage of NAPH-compliant collections
- RFC throughput (opened, merged, declined)
- GitHub stars, forks, issues, traffic
- Mailing list / forum engagement
- External mentions (Twitter, blog posts, papers)

Annually, compile into the [adoption report](../05-governance/governance-proposal.md#8-adoption-metrics-and-transparency).

## 10. When to escalate

Escalate to the Standards Council:

- Disputes over modelling decisions you can't resolve as Editor
- RFC decisions where Council recommendation is ambiguous
- Resource shortfalls threatening operational continuity
- External standards changes that may require breaking changes

Escalate to HES senior management:

- Funding model failures
- Personnel resourcing gaps
- Partnership disputes
- Strategic decisions about the standard's future

## 11. Tooling you need

### 11.1 Required

- Open Ontologies CLI (or compatible Turtle/SHACL/SPARQL tooling)
- Python 3.10+ for pipeline scripts
- Git + GitHub access
- A text editor with Turtle syntax support (VS Code with extensions, or specialised RDF editor)

### 11.2 Recommended

- Apache Jena Fuseki or equivalent for SPARQL endpoint testing
- A IIIF viewer (Mirador or Universal Viewer) for manifest validation
- ProtégÃ© for occasional ontology visualisation (not required for daily work)

## 12. Worst-case scenarios

### 12.1 Steward unable to operate

If HES becomes unable to operate as Steward:

1. Standards Council convenes emergency meeting
2. Council recommends a successor Steward (most likely candidates: TaNC successor body, AHRC, or a sector consortium)
3. Repository, permanent URIs, and operational tooling transferred
4. Adopting institutions notified

This is a scenario the standard is designed to survive, not to be optimal in.

### 12.2 Critical security or correctness issue

If a critical issue is identified (e.g. a SHACL shape allows obviously incorrect data):

1. Fix is developed within 24-48 hours
2. Expedited PATCH release within 5 working days
3. Adopters notified via mailing list and project website
4. Post-mortem published within 14 days

### 12.3 Adversarial attempt to corrupt the standard

If an adversarial actor attempts to subvert the standard via PRs / RFCs:

1. Editor declines the contribution with rationale
2. Standards Council backs the Editor's decision
3. If contribution is malicious (e.g. attempting trademark capture, vendor lock-in), report to GitHub abuse and any relevant authorities

## 13. Editorial team contact

For the first 12 months post-v1.0, the Editorial team commits to:

- Respond to operational questions within 5 working days
- Provide ad-hoc support for tooling issues
- Co-author at least one case study with an early adopter
- Hand-off any institutional knowledge gaps as they're identified

Contact: `fabio@thetesseractacademy.com`

## 14. Cross-references

- [Governance proposal](../05-governance/governance-proposal.md)
- [RFC process](../05-governance/rfc-process.md)
- [Architecture Decision Records](architecture-decision-records/)
- [Validation toolkit](../../pipeline/generate-report.py)
