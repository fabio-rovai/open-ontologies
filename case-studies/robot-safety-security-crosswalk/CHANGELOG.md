# Changelog

All notable changes to the Robot safety and security crosswalk (RSSC) are
documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-08-08

Initial release. A cited, reified crosswalk from the cybersecurity clauses of
ISO 10218-1:2025 and ISO 10218-2:2025 to the IEC 62443 series, through
IEC TS 63074:2023, organised by a three-band autonomy gradient.

### Added

- RSSC ontology (`ontology/rssc.ttl`): {{ONTOLOGY_CLASS_COUNT}} classes,
  {{ONTOLOGY_PROPERTY_COUNT}} properties, {{ONTOLOGY_CONCEPT_COUNT}} SKOS
  concepts across 8 controlled schemes, {{ONTOLOGY_TRIPLES}} triples
- Crosswalk mappings (`crosswalk/*.ttl`):
  {{CROSSWALK_ASSERTIONS}} reified crosswalk assertions,
  {{SAFETY_IMPACT_ASSERTIONS}} safety-impact assertions,
  {{SECURITY_LEVEL_CLAIMS}} security-level claims and
  {{CONTROL_GAPS}} control gaps, every one carrying a citation, an evidence
  type and a confidence grade
- SHACL shapes (`shapes/rssc-shapes.ttl`): the falsifiability contract, at
  `sh:Violation` severity throughout, including the rule that no assertion
  resting on `ev:AnalyticalInference` may be graded `conf:high`
- Cited standards surface: {{STANDARDS}} standards, {{CLAUSE_REFERENCES}}
  clause references, {{SECURITY_REQUIREMENTS}} security requirements and
  {{SOURCES}} cited sources, each with the public URL it was read from and a
  retrieval date
- Shared graph loader (`pipeline/rssc_graph.py`) discovering every `.ttl` under
  `ontology/`, `crosswalk/`, `data/` and the case-study root by glob
- Validator (`pipeline/validate.py`): an independent contract check followed by
  pySHACL, exiting non-zero on any violation or contract failure
- Coverage report (`pipeline/coverage.py`) writing `pipeline/coverage-report.md`
  and `pipeline/coverage-metrics.json`, listing every unmapped clause in full
  rather than sampling
- `BUILD_REPORT.md`: the honesty log recording what was fetched, what was
  computed, which clause identifiers were verified against which public source,
  and what could not be obtained

### Findings recorded

- The safety-to-security chain is informative at every documented hop. Neither
  IEC TS 63074 nor any part of IEC 62443 appears in the normative references of
  ISO 10218-1:2025 or ISO 10218-2:2025; the sole normative reference of
  IEC TS 63074:2023 is IEC 62061:2021. Recorded as
  {{EV_INFORMATIVE_REFERENCE}} assertions typed `ev:InformativeReference` and
  {{EV_NORMATIVE_REFERENCE}} typed `ev:NormativeReference`
- Four cybersecurity clause locations in the ISO 10218:2025 pair, two of them
  (1: 7.5.11 and 2: 7.5.23) in the instruction handbook, which is the mechanism
  by which a machine user inherits a security obligation
- IEC TS 63074:2023 sub-clauses 5.2.2 to 5.2.8 reproduce the seven IEC 62443
  foundational requirements in order, and its Table 1 is the safety-to-security
  bridge in the published structure
- Coverage collapses as autonomy rises: {{ASSERTIONS_BY_BAND_A}} mappings for
  Band A, {{ASSERTIONS_BY_BAND_B}} for Band B, {{ASSERTIONS_BY_BAND_C}} for
  Band C, with the gaps distinguished as excluded by the standard's own scope
  clause, not addressed at all, or reached only through an informative route
- The claim that safety-related robot components map to security level SL2 could
  not be traced to a primary source and is recorded once, as a
  `rssc:SecurityLevelClaim` carrying `ev:SecondaryLiterature` and `conf:low`

### Known limitations

- ISO and IEC standards are paywalled. No normative text was read or reproduced.
  Every clause identifier and title in this release comes from a freely published
  preview, contents page or catalogue entry, which is primary evidence for where
  a requirement sits and no evidence at all for what it says
- The edition of IEC 63074 cited in the ISO 10218:2025 bibliographies is
  unresolved: the Technical Report of 2019 and the Technical Specification of
  2023 are not equivalent in standing, and the bibliography lies outside the free
  preview
- Whether IEC 62443 is named in the body of clauses 5.1.16 and 5.2.16 or only in
  the bibliography is unverified
- Numeric thresholds for the ISO 10218-1:2025 robot classes are not recorded.
  Sources disagree on the values and on whether the labels use Roman or Arabic
  numerals, so no threshold and no class individual was minted
- The literal requirement identifier format of IEC 62443-2-4:2023 Annex A,
  Table A.1 is not asserted; the preview stops before the annex
- SL0 is carried at moderate confidence only. Several authoritative descriptions
  of the IEC 62443 series enumerate SL1 to SL4 alone, so SL0 is best read as the
  absence of a requirement rather than a specified level
- No property is provided for mapping individual requirements to security levels.
  The tables that do this (IEC 62443-3-3:2013 Annex B Table B.1 and
  IEC 62443-4-2:2019 Annex B Table B.1) are informative and paywalled, and
  providing the property would invite reproducing them
- Band C rows are thin because no machinery type-C standard addresses
  learning-enabled systems, not because the mapping work was cut short. The empty
  cells are the measurement
- Publication dates are recorded at month precision (2025-02) because the ISO
  cover pages state a month. Secondary sources give three different days and none
  could be confirmed
- Official ISO and IEC titles separate title elements with an em dash. House
  style forbids that character, so titles are recorded in the BSI and EN
  full-stop rendering. The separator is the only alteration to any official title

### Independence

Independent, self-initiated open research. Not endorsed by, affiliated with or
approved by ISO, IEC, CEN, CENELEC, BSI, ISA, the ISA Security Compliance
Institute, IEEE, MITRE, KAN or any other body named within it. Released under
CC BY 4.0 (vocabulary, mappings, documentation) and MIT (pipeline scripts).
