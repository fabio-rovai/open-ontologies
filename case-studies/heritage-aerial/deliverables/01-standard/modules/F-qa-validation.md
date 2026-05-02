# Module F — QA & Validation

**Status:** Normative · v1.0
**Applies to:** All NAPH-compliant collections, all tiers (with tier-specific intensity)
**Defines:** the quality-assurance and validation workflow for assessing and asserting NAPH compliance

## F.1 Purpose

A standard without enforcement is aspirational language. Module F specifies how compliance is **assessed** and **asserted** so:

- Institutions can self-certify their compliance level with confidence
- Aggregators (Europeana, DPLA, N-RICH) can verify compliance independently
- Researchers can filter by verified compliance level
- Tier upgrades have a documented, reproducible pathway

Module F is the operational counterpart to Modules A-E. They specify what; Module F specifies how to verify.

## F.2 Outcome requirements

### F.2.1 Baseline (F-baseline)

A Baseline-compliant collection MUST:

- **F.B.1** Pass automated SHACL validation against the published `naph-shapes.ttl` for the Baseline tier
- **F.B.2** Publish a validation report (HTML or JSON) at a stable URL, dated and versioned
- **F.B.3** Re-validate at least quarterly OR upon any change to the collection

A Baseline-compliant collection SHOULD:

- **F.B.4** Run automated link-resolution checks for `naph:hasIdentifier` URIs
- **F.B.5** Sample-check 1% of records for human review at least annually

### F.2.2 Enhanced (F-enhanced)

An Enhanced-compliant collection MUST additionally:

- **F.E.1** Produce a structured conformance report (`naph:ConformanceReport`) recording each SHACL shape's outcome
- **F.E.2** Document any deviations from full compliance with rationale (`naph:nonConformanceNote`)
- **F.E.3** Run validation in CI/CD if the collection is updated programmatically

An Enhanced-compliant collection SHOULD additionally:

- **F.E.4** Sample 5% of records for human review per audit cycle
- **F.E.5** Track validation outcomes over time as a metric (drift detection)

### F.2.3 Aspirational (F-aspirational)

An Aspirational-compliant collection MUST additionally:

- **F.A.1** Run computational reuse tests as part of validation — actually execute typical research queries against the collection and assert they return correct results
- **F.A.2** Maintain a test fixture set of 50+ records with known expected outcomes for regression testing
- **F.A.3** Validate cross-collection links (broken `naph:linkedRecord` URIs MUST be flagged for repair)

## F.3 The validation toolkit

The reference validation toolkit is provided in [`pipeline/generate-report.py`](../../../pipeline/generate-report.py). It produces:

1. **SHACL conformance report** — pass/fail per shape, per record, with violation messages
2. **Tier distribution** — count of records at each tier
3. **Competency-question results** — does each documented research question return expected results?
4. **Drift report** — what changed since the last validation run?

The toolkit is open-source (MIT) and can be extended with collection-specific checks.

## F.4 SHACL validation

The canonical shapes are in [`ontology/naph-shapes.ttl`](../../../ontology/naph-shapes.ttl). They are organised into:

| Shape | Targets | Tier |
|---|---|---|
| `naph:BaselineShape` | every `naph:AerialPhotograph` (or profile equivalent) | Baseline |
| `naph:EnhancedShape` | records claiming Enhanced tier | Enhanced |
| `naph:AspirationalShape` | records claiming Aspirational tier | Aspirational |
| `naph:DigitalSurrogateShape` | every `naph:DigitalSurrogate` | All |
| `naph:PlaceShape` | every `naph:Place` | Aspirational |
| `naph:RightsStatementShape` | every `naph:RightsStatement` | All |

Run as:

```bash
open-ontologies shacl ontology/naph-shapes.ttl --data <your-collection>.ttl --pretty
```

## F.5 Sampling-based human review

Automated validation catches structural problems (missing fields, wrong types). It does NOT catch:

- Wrong but well-formed data ("Berlin" tagged with Tokyo coordinates)
- Plausible but incorrect inferences (Spitfire mistagged as Mosquito)
- Subtle semantic errors (Wikidata QID for Sound used for Hiroshima)

Human review remains essential. Module F specifies sample sizes:

| Tier | Sample size per audit | Frequency |
|---|---|---|
| Baseline | 1% of records | Annual |
| Enhanced | 5% of records | Quarterly |
| Aspirational | 10% of records, stratified by subject and tier | Quarterly |

Sampling MUST be:

- **Random** (not just first N records)
- **Stratified** for Aspirational (Aerial subject, Place authority link, Cross-collection link)
- **Documented** — sample IDs, reviewer, outcome recorded as `naph:qcReview` events (Module E)

## F.6 Computational reuse tests (Aspirational)

For Aspirational tier, the collection MUST pass a standard set of competency-question queries. The reference set is in [`docs/competency-queries.batch.txt`](../../../docs/competency-queries.batch.txt).

Each test:

1. Has a documented expected outcome (number of results, identity of results, or both)
2. MUST execute against the live collection
3. Result MUST match expected outcome within a tolerance (typically exact match)

A test failure does not automatically fail Aspirational compliance, but it MUST be documented and root-caused. Common reasons:

- A new record violates an unstated assumption
- A SPARQL endpoint configuration changed
- An external authority (Wikidata) deprecated a QID
- A collection update has not yet been validated

## F.7 Drift detection

Track these metrics over time:

| Metric | What it indicates |
|---|---|
| SHACL violations count | Quality drift — is compliance degrading? |
| Tier distribution | Are records being upgraded or down-graded? |
| Average record age (since last validation) | Is the collection stale? |
| External link health (Wikidata, GeoNames) | Are upstream authorities moving? |
| Competency-question result counts | Are research workflows still working? |

Drift detection runs as a scheduled task (cron, GitHub Actions) and produces a time-series record. The reference implementation is provided as part of the validation toolkit.

## F.8 Conformance report format

A conformance report is a JSON-LD or RDF document recording the outcome of a validation run.

```turtle
@prefix naph: <https://w3id.org/naph/ontology#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix prov: <http://www.w3.org/ns/prov#> .

ex:report-2024-04-30 a naph:ConformanceReport, prov:Activity ;
    rdfs:label "NAPH conformance report — 2024-04-30" ;
    prov:atTime "2024-04-30T20:00:00Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:NCAP-validation-tooling ;
    naph:specVersion "1.0" ;
    naph:assessedCollection ex:NCAPCollection ;
    naph:tierBaselineCount 42 ;
    naph:tierEnhancedCount 18 ;
    naph:tierAspirationalCount 5 ;
    sh:conforms true ;
    naph:competencyQuestionsPassed 7 ;
    naph:competencyQuestionsTotal 7 ;
    naph:reportURL <https://example.org/ncap/reports/2024-04-30.html> .
```

## F.9 Continuous integration

For institutions publishing programmatically, a CI/CD pipeline SHOULD:

```yaml
# Example GitHub Actions workflow
name: NAPH Validation
on:
  push:
    paths: ['data/**']
  schedule:
    - cron: '0 6 * * 1'  # Weekly Monday 06:00 UTC

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Open Ontologies
        run: |
          curl -L https://github.com/fabio-rovai/open-ontologies/releases/latest/download/open-ontologies-x86_64-unknown-linux-gnu -o oo
          chmod +x oo && sudo mv oo /usr/local/bin/open-ontologies
      - name: Validate
        run: |
          open-ontologies validate ontology/naph-core.ttl
          open-ontologies batch < pipeline/validation.batch.txt
      - name: Generate report
        run: python3 pipeline/generate-report.py > reports/validation-$(date -u +%Y-%m-%d).html
      - name: Commit report
        uses: stefanzweifel/git-auto-commit-action@v5
        with:
          commit_message: "Validation report for $(date -u +%Y-%m-%d)"
```

## F.10 Validation outcomes

| Outcome | Meaning | Action |
|---|---|---|
| `pass` | All shapes for the claimed tier conform | Publish report |
| `pass-with-warnings` | Conformant but recommended (`SHOULD`) items missing | Publish; flag warnings for next cycle |
| `partial` | Some records non-conformant; collection at lower tier | Either downgrade tier claim OR fix non-conformant records |
| `fail` | Critical structural problems | Block publication; investigate |

## F.11 Cross-references

- [Module A — Capture & Imaging](A-capture-imaging.md) (capture-quality QA)
- [Module B — Metadata & Data Structures](B-metadata-data-structures.md) (descriptive validation)
- [Module C — Rights, Licensing & Ethics](C-rights-licensing-ethics.md) (rights review)
- [Module D — Packaging & Publication](D-packaging-publication.md) (manifest integrity)
- [Module E — Paradata & Workflow](E-paradata-workflow.md) (provenance validation)
- [Validation checklist](../../04-adoption-guidance/validation-checklists.md)
- [`pipeline/generate-report.py`](../../../pipeline/generate-report.py) — reference implementation
