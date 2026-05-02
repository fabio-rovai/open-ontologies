# Governance Proposal

How NAPH is maintained, evolved, and stewarded after NAPH. Designed to be **lightweight but credible** — small enough that HES can operate it without specialist resourcing, robust enough that adopting institutions trust their investment is durable.

## 1. Principles

NAPH governance follows these principles, in priority order:

1. **Lightweight** — not creating committee overhead disproportionate to a single-vertical standard
2. **Transparent** — every decision is visible in a public repository with documented rationale
3. **Multi-stakeholder** — adopting institutions have a voice; users (researchers) have a voice; the host institution maintains operational decision rights
4. **Sustainable** — designed to outlive the initial delivery contract; HES-operable
5. **Open** — all governance artefacts (RFCs, decisions, change log) under the same open licence as the standard itself

## 2. Roles and decision rights

### 2.1 Steward (operational owner)

**Recommended:** Historic Environment Scotland (HES), via NCAP / N-RICH responsibility.

**Responsibilities:**

- Operate the canonical Git repository
- Triage incoming RFCs and issues
- Convene the Standards Council (annual)
- Publish releases and changelogs
- Maintain the public website and SPARQL endpoint
- Liaise with adjacent standards bodies (W3C, OGC, IIIF Consortium, AHRC)

**Decision rights:** the Steward has final say on operational matters (release timing, repository policy, issue triage) and on editorial changes (clarifications, typos). For substantive changes, the Steward acts with Standards Council advice ([§3](#3-the-rfc-process)).

### 2.2 Standards Council (advisory)

**Composition:**

- One representative from the Steward (chair)
- One representative each from up to 5 adopting institutions
- Two external representatives — one researcher, one technical (linked-data / heritage informatics)
- Editor (typically a contracted role; the editor can be the Steward representative or external)

**Term:** institutional representatives serve 2-year terms, renewable once. External representatives serve 3-year terms.

**Responsibilities:**

- Annual review of the standard's direction
- Advice on substantive RFCs (recommend accept / reject / defer)
- Annual public report on adoption metrics
- Adjudication of disputes (rare, but possible — e.g. a contested rights modelling decision)

**Decision rights:** advisory only. The Steward retains final say but is expected to follow Council advice in most cases. Council disagreement with the Steward is recorded publicly.

### 2.3 Editor (operational role)

**Responsibilities:**

- Drafting changes to the spec
- Maintaining ontology, shapes, and associated tooling
- Triaging editorial vs. substantive decisions
- Publishing releases

**Decision rights:** editorial decisions only. May be the Steward representative or contracted externally; the role is operational, not strategic.

### 2.4 Contributors (open)

Anyone can contribute via the [RFC process](rfc-process.md). Contributors include:

- Institutional staff at adopting institutions
- Researchers using NAPH-compliant data
- Standards-adjacent practitioners (W3C, OGC, IIIF Consortium working groups)
- Open-source community members

Contributions follow the [Code of Conduct](#7-code-of-conduct).

## 3. The RFC process

Substantive changes follow the RFC (Request for Comments) process, documented in detail at [rfc-process.md](rfc-process.md). Summary:

1. Anyone opens an RFC as a pull request to the `rfcs/` directory
2. The Editor triages — is it substantive or editorial?
3. Substantive RFCs go through:
   - Public comment period (30 days minimum)
   - Standards Council review and recommendation
   - Steward decision
4. Accepted RFCs are merged with a designated version number
5. Editorial changes go through standard PR review without RFC

## 4. Release cadence

### 4.1 Versioning

NAPH follows [semantic versioning 2.0.0](https://semver.org/):

- **MAJOR** — backwards-incompatible changes. Reserved for v2.0+
- **MINOR** — backwards-compatible additions
- **PATCH** — clarifications, errata, doc fixes

A v1.x record is guaranteed to validate against any future v1.y. v2.0 is a deliberate breaking-change point requiring migration tooling.

### 4.2 Release cadence

- **PATCH releases:** as needed, typically 4-6 per year
- **MINOR releases:** roughly annual (one per Standards Council cycle)
- **MAJOR releases:** rare. Anticipated frequency: every 5-10 years

Each release includes:

- Tagged Git commit
- Updated `naph-core.ttl` and `naph-shapes.ttl`
- Release notes documenting all changes since the previous release
- Migration guide if any deprecations or behaviour changes
- Updated documentation
- Re-published validation report against the canonical sample data

### 4.3 Long-term support

Each MINOR version is supported for at least 2 years from release. Critical PATCH fixes will be backported to supported MINOR versions where reasonable.

The current v1.x line is supported until at least 2028.

## 5. Deprecation policy

A property, class, or shape slated for removal in a future MAJOR version MUST:

1. Be marked `owl:deprecated true` in the ontology
2. Have a `dcterms:isReplacedBy` link to the successor (if any)
3. Remain present and functional for at least one MINOR release cycle (~1 year)
4. Be documented in release notes with migration guidance

Removed elements MUST NOT be reintroduced with different semantics.

## 6. Conflict of interest policy

Standards Council members and the Editor MUST disclose:

- Commercial interests in NAPH adoption (consultancy, vendor relationships)
- Institutional affiliations that may benefit from specific decisions
- Authorship of competing or overlapping standards

Conflicted members MUST recuse from votes where the conflict applies. Disclosures are recorded in a public register.

## 7. Code of conduct

NAPH governance adopts the [Contributor Covenant v2.1](https://www.contributor-covenant.org/version/2/1/code_of_conduct/) for all interactions in the project's communication channels (issue tracker, RFCs, mailing list, in-person meetings).

Violations are reported to the Steward, who is responsible for response and resolution. Severe or repeated violations may result in temporary or permanent removal from the project's communication channels.

## 8. Adoption metrics and transparency

The Steward publishes an **annual adoption report** including:

- Number of institutions claiming each tier of compliance
- Number of records published as NAPH-compliant
- Geographic and temporal coverage of NAPH-compliant collections
- RFC throughput (opened, merged, declined)
- Issue resolution times
- External standards engagement (W3C, OGC, IIIF input or output)

The report is published publicly under CC BY 4.0.

## 9. Funding and sustainability

### 9.1 Pilot phase

The initial delivery contract funds:

- Initial standard development (this work)
- Initial documentation, ontology, shapes, pipeline
- Initial 3 partner applications

### 9.2 Year 1 post-v1.0 (handover)

Anticipated funding sources:

- HES operational budget for Steward role (typically 0.2-0.4 FTE)
- One-off transition funding for editorial work (~£15k-£30k)
- AHRC strategic funding for Standards Council convening

### 9.3 Years 2+

Sustainable funding model options:

- HES operational budget continues to fund Steward role
- AHRC / Research England fund occasional editorial work
- Adopting institutions contribute in-kind via Standards Council representation
- Research grants requiring NAPH compliance may include line-item funding for standard contribution

The standard itself does NOT charge for use, certification, or compliance assessment. Free use is a precondition for sector adoption.

### 9.4 Worst-case sustainability

If post-v1.0 funding fails:

- The repository remains accessible (open-source, multiple mirrors)
- Existing NAPH-compliant collections continue to validate against v1.x indefinitely
- Forks may emerge under new stewardship if the original Steward becomes unable to operate

This is a real but low-probability scenario; the standard is small enough that even unmaintained, it remains usable.

## 10. Adjacent standards relationships

NAPH actively engages with:

- **W3C** (DCAT, PROV, SKOS) — via public mailing lists; participation in working groups when scope permits
- **OGC** (GeoSPARQL) — particularly on geographic-footprint best practices
- **IIIF Consortium** — Presentation API and Image API alignment
- **rightsstatements.org** — coordinated rights vocabulary
- **DPLA / Europeana** — aggregator harvesting compatibility

NAPH does not seek to be a member organisation of these bodies but does monitor their work and contribute where relevant.

## 11. Annual review

Each year, the Standards Council reviews:

- Whether the standard remains fit for purpose
- Adoption metrics and trends
- Emerging research needs
- Emerging adjacent standards
- Recommendations for the next MINOR release

The annual review is published as part of the adoption report.

## 12. Dispute resolution

For technical disputes (e.g. modelling decisions):

1. Discussed in the relevant RFC
2. Escalated to the Standards Council for advisory recommendation
3. Final decision by the Steward, recorded with rationale

For governance disputes (e.g. Code of Conduct violation, Steward inaction):

1. Reported to the Steward; if Steward is the subject, reported to the Standards Council Chair
2. Escalated to the Standards Council
3. In extreme cases, the Standards Council may publicly recommend a fork or change of Steward

## 13. Change to this governance proposal

This proposal itself follows the RFC process. Changes require Standards Council recommendation and Steward acceptance.

## Cross-references

- [RFC process](rfc-process.md)
- [Maintenance runbook](../06-knowledge-transfer/maintenance-runbook.md)
- [Standards Council charter template](../07-templates/standards-council-charter-template.md)
- [Code of Conduct](https://www.contributor-covenant.org/version/2/1/code_of_conduct/)
