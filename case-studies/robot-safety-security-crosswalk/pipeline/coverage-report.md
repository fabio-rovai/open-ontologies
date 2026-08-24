# RSSC coverage report

Counted output of `pipeline/coverage.py`. Every number below is produced
by that script from the committed Turtle; none is hand-entered. Re-run the
script and this file is rewritten.

- generated: 2026-08-08 20:31 UTC
- case study root: `/Users/fabio/projects/open-ontologies/case-studies/robot-safety-security-crosswalk`
- graph: 2167 triples from 2 file(s)

| File | Triples | sha256 (first 16) |
|---|---:|---|
| `ontology/rssc.ttl` | 675 | `f4856da49a1dd473` |
| `crosswalk/iso10218-iec62443.ttl` | 1492 | `638f35b752554ba6` |

Data locations that contributed nothing, listed so that a thin graph
cannot be mistaken for a complete one:

- `data/` does not exist
- `(case study root)` holds no `.ttl` files

## 1. What was counted, and against what denominator

The denominator for clause coverage is the set of `rssc:ClauseReference`
nodes **declared in this graph** and bound with `rssc:inStandard` to an
ISO 10218:2025 part. It is not the clause tree of the published standard.
ISO 10218-1:2025 and ISO 10218-2:2025 are paywalled: their contents pages
are public, their clause bodies are not. Dividing by a clause count taken
from a document nobody in this project has read in full would be a
fabricated denominator, so no percentage against the whole standard is
printed anywhere in this report.

What is printed instead: of the clauses this project could cite and did
declare, how many carry at least one mapping. That is a smaller claim and
a checkable one.

## 2. Inventory

| Node kind | Count |
|---|---:|
| `rssc:CrosswalkAssertion` | **32** |
| `rssc:SafetyImpactAssertion` | **6** |
| `rssc:SecurityLevelClaim` | **3** |
| `rssc:ControlGap` | **7** |
| all `rssc:EvidencedAssertion` | **48** |
| `rssc:Standard` | **8** |
| `rssc:ClauseReference` | **38** |
| `rssc:SecurityRequirement` | **20** |
| `rssc:FoundationalRequirement` | **7** |
| `rssc:SafetyFunction` | **6** |
| `rssc:AutonomyBand` | **3** |
| `rssc:Source` | **10** |

## 3. ISO 10218:2025 clause coverage

| Standard | Clauses declared | With >=1 mapping | With no mapping | Reached by any assertion |
|---|---:|---:|---:|---:|
| ISO 10218-1:2025 | **13** | **9** | **4** | **12** |
| ISO 10218-2:2025 | **8** | **6** | **2** | **8** |
| **total** | **21** | **15** | **6** | **20** |

Mapping rate over declared clauses: **0.7143** (15 of 21).
Reached rate, counting mappings, gaps and safety impacts: **0.9524** (20 of 21).

### 3.1 Every declared ISO 10218:2025 clause

The full list. Not sampled, not truncated, not sorted to flatter.

| Standard | Clause | Title | Status | Mappings | Gaps | Safety impacts | Bands |
|---|---|---|---|---:|---:|---:|---|
| ISO 10218-1:2025 | `1` | Scope | normative | 0 | 1 | 0 | B |
| ISO 10218-1:2025 | `5.1.16` | Cybersecurity | normative | 2 | 2 | 0 | A, B, C |
| ISO 10218-1:2025 | `5.2.8` | Means of controlling the robot | normative | 0 | 1 | 0 | C |
| ISO 10218-1:2025 | `5.3.1` | General | normative | 1 | 0 | 0 | A, B, C |
| ISO 10218-1:2025 | `5.3.5` | Parameterization of safety functions | normative | 1 | 1 | 1 | A, B, C |
| ISO 10218-1:2025 | `5.3.6` | Communications | normative | 1 | 0 | 0 | A, B |
| ISO 10218-1:2025 | `5.4.2` | Emergency stop | normative | 1 | 0 | 1 | A, B |
| ISO 10218-1:2025 | `5.5.1` | Single-point-of-control | normative | 1 | 0 | 1 | A, B |
| ISO 10218-1:2025 | `5.5.3` | Speed limit(s) monitoring | normative | 1 | 0 | 1 | A, B |
| ISO 10218-1:2025 | `5.5.6` | Stopping time limiting | normative | 0 | 0 | 1 | A, B |
| ISO 10218-1:2025 | `5.10.3` | Speed and separation monitoring (SSM) | normative | 1 | 0 | 1 | A, B |
| ISO 10218-1:2025 | `7.5.11` | Cybersecurity | normative | 1 | 0 | 0 | A |
| ISO 10218-1:2025 | `Annex A` | List of significant hazards | informative | 0 | 0 | 0 | none |
| ISO 10218-2:2025 | `1` | Scope | normative | 0 | 1 | 0 | B |
| ISO 10218-2:2025 | `5.2.16` | Cybersecurity | normative | 3 | 2 | 0 | A, B, C |
| ISO 10218-2:2025 | `5.3.4` | Operational modes with multi-robot applications or robot cells | normative | 0 | 1 | 0 | C |
| ISO 10218-2:2025 | `5.3.5` | Local control, remote control and single-point-of-control | normative | 1 | 0 | 0 | A, B |
| ISO 10218-2:2025 | `5.5.1` | General | normative | 1 | 0 | 0 | A, B, C |
| ISO 10218-2:2025 | `5.7.6` | Local and remote control | normative | 1 | 0 | 0 | A, B |
| ISO 10218-2:2025 | `7.5.11` | Remote interventions | normative | 1 | 1 | 0 | A, B, C |
| ISO 10218-2:2025 | `7.5.23` | Cybersecurity | normative | 1 | 0 | 0 | A |

### 3.2 Declared ISO 10218:2025 clauses with NO mapping

6 of 21 declared clauses have no
`rssc:CrosswalkAssertion`. All of them are listed here.

| Standard | Clause | Title | Reached by a gap or safety impact? |
|---|---|---|---|
| ISO 10218-1:2025 | `1` | Scope | 1 gap(s) |
| ISO 10218-1:2025 | `5.2.8` | Means of controlling the robot | 1 gap(s) |
| ISO 10218-1:2025 | `5.5.6` | Stopping time limiting | 1 safety impact(s) |
| ISO 10218-1:2025 | `Annex A` | List of significant hazards | **no** |
| ISO 10218-2:2025 | `1` | Scope | 1 gap(s) |
| ISO 10218-2:2025 | `5.3.4` | Operational modes with multi-robot applications or robot cells | 1 gap(s) |

## 4. Autonomy band distribution

| Band | rssc:CrosswalkAssertion | rssc:SafetyImpactAssertion | rssc:SecurityLevelClaim | rssc:ControlGap | Total |
|---|---:|---:|---:|---:|---:|
| **A** | 30 | 6 | 3 | 2 | **41** |
| **B** | 27 | 6 | 1 | 4 | **38** |
| **C** | 15 | 1 | 1 | 5 | **22** |
| _(no band declared)_ | 0 | 0 | 0 | 0 | **0** |

Every declared band carries at least one assertion.

### 4.1 ISO 10218:2025 clauses reached, per band

| Band | Declared ISO 10218 clauses reached | Clause identifiers |
|---|---:|---|
| **A** | 13 | `5.1.16`, `5.2.16`, `5.3.1`, `5.3.5`, `5.3.6`, `5.4.2`, `5.5.1`, `5.5.3`, `5.5.6`, `5.7.6`, `5.10.3`, `7.5.11`, `7.5.23` |
| **B** | 13 | `1`, `5.1.16`, `5.2.16`, `5.3.1`, `5.3.5`, `5.3.6`, `5.4.2`, `5.5.1`, `5.5.3`, `5.5.6`, `5.7.6`, `5.10.3`, `7.5.11` |
| **C** | 8 | `5.1.16`, `5.2.8`, `5.2.16`, `5.3.1`, `5.3.4`, `5.3.5`, `5.5.1`, `7.5.11` |

## 5. Security side: what the crosswalk reaches

### 5.1 Foundational requirements

| FR | Label | Assertions pointing at it |
|---|---|---:|
| `FR1` | FR 1. Identification and authentication control | 5 |
| `FR2` | FR 2. Use control | 7 |
| `FR3` | FR 3. System integrity | 12 |
| `FR4` | FR 4. Data confidentiality | 1 |
| `FR5` | FR 5. Restricted data flow | 3 |
| `FR6` | FR 6. Timely response to events | 1 |
| `FR7` | FR 7. Resource availability | 3 |

Every foundational requirement is reached by at least one assertion.

### 5.2 Security requirements

20 declared, of which **20** are pointed at by at least one
assertion. Every declared requirement is listed.

| Requirement | Title | Assertions pointing at it |
|---|---|---:|
| `CR 2.13` | Use of physical diagnostic and test interfaces | 1 |
| `CR 3.14` | Integrity of the boot process | 1 |
| `CR 3.4` | Software and information integrity | 3 |
| `SD-4` | Secure design best practices | 1 |
| `SG-3` | Security hardening guidelines | 1 |
| `SR 1.1` | Human user identification and authentication | 1 |
| `SR 1.13` | Access via untrusted networks | 2 |
| `SR 1.6` | Wireless access management | 1 |
| `SR 2.1` | Authorization enforcement | 2 |
| `SR 2.2` | Wireless use control | 1 |
| `SR 2.3` | Use control for portable and mobile devices | 1 |
| `SR 2.6` | Remote session termination | 1 |
| `SR 3.1` | Communication integrity | 4 |
| `SR 3.5` | Input validation | 3 |
| `SR 3.6` | Deterministic output | 2 |
| `SR 5.1` | Network segmentation | 2 |
| `SR 7.1` | Denial of service protection | 2 |
| `SUM-1` | Security update qualification | 1 |
| `ZCR 3.3` | Separate safety related assets | 1 |
| `ZCR 5.6` | Determine SL-T | 2 |

## 6. Evidence type and confidence

| Evidence type | Assertions |
|---|---:|
| `analytical-inference` | 36 |
| `published-preview` | 9 |
| `secondary-literature` | 2 |
| `published-summary` | 1 |

| Confidence | Assertions |
|---|---:|
| `moderate` | 36 |
| `high` | 9 |
| `low` | 3 |

## 7. What is NOT covered

This section exists so that the reader does not have to reconstruct the
negative space from the tables above. Every item is a count taken from the
same graph.

1. **ISO 10218:2025 clauses declared with no mapping: 6** of 21. Listed in full at 3.2.
2. **ISO 10218:2025 clauses declared that no assertion of any kind reaches: 1.**
   `Annex A` (ISO 10218-1:2025)
3. **Autonomy bands with zero assertions: 0** of 3 declared.
4. **Foundational requirements no assertion reaches: 0** of 7.
5. **Declared security requirements no assertion reaches: 0** of 20.
6. **Assertions with no autonomy band declared: 0.** Such an assertion is a claim about the corpus that is not tied to a
   point on the gradient, so it is excluded from the per-band arithmetic at 4.
7. **Assertions carrying no `rssc:citation`: 0.** This number must be zero; `validate.py` fails the build if it is not.
8. **Assertions with no `prov:wasDerivedFrom` link to a `rssc:Source`: 0.**
9. **Declared sources nothing is derived from: 0.**
10. **Standards declared with no clause reference attached: 3.**
   IEC 62443-3-2:2020, IEC 62443-4-1:2018, IEC TS 62443-1-1:2009
11. **Clause references bound to no standard: 0.**

## 8. Limits of this count

1. Coverage here means a mapping exists and carries evidence. It does not
   mean the mapping is correct. Correctness is contestable per assertion,
   which is why each one is reified and carries its own citation.
2. The clause denominator is what this project declared, not what ISO
   published. See section 1.
3. No normative text from any ISO or IEC standard was read or reproduced
   to produce these counts. The graph records clause identifiers, clause
   titles, part numbers and dates, which are published free in the front
   matter and contents pages, and nothing else.
4. A clause reached only by a `rssc:ControlGap` is reached by a negative
   finding: the crosswalk says that clause does not address the exposure.
   It is counted separately from a mapping throughout, and the two are
   never added together into a single coverage figure.

