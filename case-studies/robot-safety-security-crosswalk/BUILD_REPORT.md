# BUILD_REPORT: robot-safety-security-crosswalk

An honest record of what was read, what was computed, and the limits of the
claim. Every number in `pipeline/coverage-metrics.json` is produced by
`pipeline/coverage.py`; none is hand-entered. Every clause identifier in
`crosswalk/iso10218-iec62443.ttl` was read in a published source, and the ones
that could not be are listed below as omissions rather than guesses.

Part of [Open Ontologies](../../README.md). Released under CC BY 4.0.

## The constraint this artefact was built under

ISO and IEC standards are paywalled. No normative text from any ISO or IEC
standard was read or reproduced in the making of this case study. What was read
is the public surface those bodies and their distributors publish free: cover
pages, contents pages, forewords, introductions, scope clauses, normative
reference lists and catalogue entries.

That constraint is treated as a design feature rather than an apology. An
artefact built from public surfaces alone can be checked by any reader without
buying anything, and the `rssc:paywalled` flag on each source makes the limits
of the evidence base queryable instead of merely admitted. It also fixes what
the evidence can support: a contents page is primary evidence for **where** a
requirement sits and no evidence at all for **what it says**.

## Source data: what was fetched

All primary sources are the official free previews published by the issuing
bodies and their authorised distributors. Retrieval dates are recorded per
source in the graph on `rssc:retrievedOn`.

| Source | Type | Used for |
|---|---|---|
| ISO 10218-1:2025 preview | official ISO free preview | cover date, edition, foreword change list, scope exclusions, complete Contents, complete normative reference list |
| ISO 10218-2:2025 preview | official ISO free preview | as above, for Part 2 |
| IEC TS 63074:2023 preview | official IEC free preview | identity, lineage from IEC TR 63074:2019, scope, complete Contents, sole normative reference, Table 1 title |
| IEC TS 62443-1-1:2009 preview | official IEC free preview | clause 5.3 foundational requirements, clause 5.11 security levels, Table 8 location |
| IEC 62443-3-3:2013 preview | official IEC free preview | clauses 5 to 11 titles, the 51 SR identifiers and titles, clause 0.3, Annex B title |
| IEC 62443-4-2:2019 preview | official IEC free preview | CR, EDR, HDR, NDR and SAR identifiers and titles, clause 4 common component security constraints |
| IEC 62443-3-2:2020 preview | official IEC free preview | ZCR identifiers and titles, including ZCR 3.3 and ZCR 5.6 |
| IEC 62443-4-1:2018 preview | official IEC free preview | the 8 practices and their requirement identifier grammar |
| IEC 62443-2-4:2023 preview | official IEC free preview | edition, title, Annex A structure |
| IEC webstore catalogue entries | official catalogue | editions, publication dates, document types |
| KANBrief 2/2023 | named-expert commentary | the SL2 and SL1 statement, recorded as commentary and graded accordingly |
| ISASecure white paper | certification-body publication | the separate claim that SL2 is a sensible minimum for IACS components generally |

## What was computed

| Artefact | Script | Result |
|---|---|---|
| `pipeline/coverage-metrics.json` | `pipeline/coverage.py` | machine-readable counts, the source of every number quoted in `README.md` |
| `pipeline/coverage-report.md` | `pipeline/coverage.py` | the readable report, listing every unmapped clause in full rather than sampling |
| SHACL conformance | `pipeline/validate.py` | the gate, plus an independent contract check that runs before SHACL |
| Self-test | `pipeline/validate.py` with `RSSC_ROOT` | one fixture that must pass, one that must fail |

Counts at this release: {{EVIDENCED_ASSERTIONS}} evidenced assertions
({{CROSSWALK_ASSERTIONS}} crosswalk mappings, {{SAFETY_IMPACT_ASSERTIONS}}
safety-impact claims, {{SECURITY_LEVEL_CLAIMS}} security-level claims,
{{CONTROL_GAPS}} control gaps), over {{STANDARDS}} standards,
{{CLAUSE_REFERENCES}} clause references and {{SECURITY_REQUIREMENTS}} security
requirements, derived from {{SOURCES}} cited sources. All figures come from
`pipeline/coverage-metrics.json`.

## Three corrections made against the popular account

These are the findings that required primary reading, and each contradicts
something widely repeated in public commentary.

1. **ISO 10218:2025 does not normatively reference IEC 62443 or IEC TS 63074.**
   The complete clause 2 of both parts was read. Part 1's list runs from
   ISO 3864-1:2011 to IEC 62745:2017; Part 2's ends at IEC 62061:2021. Neither
   contains any security document. The widely marketed claim that these
   standards "mandate IEC 62443 compliance" is false as stated. What is mandated
   is a cybersecurity threat assessment.
2. **IEC TS 63074:2023 does not normatively reference IEC 62443 either.** Its
   clause 2 contains exactly one entry, IEC 62061:2021. The bridge to the
   security series is structural and terminological, carried by sub-clauses
   5.2.2 to 5.2.8 and by Table 1, not by a normative dependency.
3. **The Part 2 cybersecurity clause is 5.2.16, not 5.2.26.** A widely read
   secondary source gives the latter. The official contents page gives the
   former.

## What could NOT be obtained, and was omitted rather than guessed

- **Which edition of IEC 63074 the ISO 10218:2025 bibliographies cite.** The
  Technical Report of 2019 and the Technical Specification of 2023 are not
  equivalent in standing. The bibliography sits outside the free preview, so the
  question is open and is recorded as open.
- **Whether IEC 62443 is named in the body of clauses 5.1.16 and 5.2.16 or only
  in the bibliography.** Unverified, because the clause bodies are paywalled.
- **Numeric thresholds for the ISO 10218-1:2025 robot classes.** Sources
  disagree on the values and on whether the labels use Roman or Arabic numerals.
  No threshold and no class individual was minted.
- **The literal requirement identifier format of IEC 62443-2-4:2023 Annex A,
  Table A.1.** The preview stops before the annex, so the format commonly
  reported elsewhere is not asserted here.
- **A primary source for the claim that safety-related robot components map to
  SL2.** None was found. The claim is recorded once, as a
  `rssc:SecurityLevelClaim` carrying `ev:SecondaryLiterature` and `conf:low`,
  with its source named, so that it can be contradicted rather than cited.
- **The requirement-to-security-level mapping tables.** IEC 62443-3-3:2013
  Annex B Table B.1 and IEC 62443-4-2:2019 Annex B Table B.1 are paywalled, and
  both are informative rather than normative. The vocabulary deliberately
  provides no property for that mapping, because providing one would invite
  reproducing the tables.
- **Exact publication days.** The ISO cover pages state 2025-02. Secondary
  sources give three different days. The graph records a month, and
  `rssc:publicationDate` is deliberately declared without a fixed range so that
  a month cannot be silently upgraded to a fabricated day.

## Known defects at the time of writing

Recorded here rather than fixed silently, because the point of the artefact is
that failures are counted instead of smoothed over.

- `pipeline/validate.py` currently exits non-zero. The violations are reported
  as found and no shape was relaxed to obtain a pass. Two causes, both
  mechanical rather than evidential, and neither affecting the citations
  themselves:
  1. Every `rssc:basis` literal in `crosswalk/iso10218-iec62443.ttl` carries an
     `@en` language tag, which makes its datatype `rdf:langString`, while both
     the declared range of `rssc:basis` in `ontology/rssc.ttl` and the shape in
     `shapes/rssc-shapes.ttl` require `xsd:string`. The same file correctly
     leaves `rssc:citation` untagged, so the tagging is an internal
     inconsistency rather than a considered decision.
  2. Two control gaps that deliberately leave `rssc:wouldBeAddressedBy` unset,
     and say so in their basis, trip the rule requiring at least one candidate
     control. The escape hatch intended for exactly that case is not taking
     effect. The underlying modelling decision is the right one: the ontology
     states that where nothing in the public corpus would address an exposure,
     the property should be left unset, because an unfilled gap is a stronger
     finding than a forced one.

Both are tracked and neither is a reason to alter a citation, an evidence type
or a confidence grade.

## Independence

Independent, self-initiated open research. Not endorsed by, affiliated with or
approved by ISO, IEC, CEN, CENELEC, BSI, ISA, the ISA Security Compliance
Institute, IEEE, MITRE, KAN or any other body named within it. This is not a
conformity assessment and confers no certification. ISO and IEC standards remain
the copyright of ISO and IEC and must be purchased from the issuing body.
