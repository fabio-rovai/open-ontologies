# RSSC coverage report

Counted output of `pipeline/coverage.py`. Every number below is produced
by that script from the committed Turtle; none is hand-entered. Re-run the
script and this file is rewritten.

- generated: 2026-08-08 09:35 UTC
- case study root: `/Users/fabio/projects/open-ontologies/case-studies/robot-safety-security-crosswalk/pipeline/selftest/valid`
- graph: 725 triples from 2 file(s)

| File | Triples | sha256 (first 16) |
|---|---:|---|
| `ontology/rssc.ttl` | 675 | `f4856da49a1dd473` |
| `crosswalk/fixture-valid.ttl` | 50 | `7f638a45eacaace1` |

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
| `rssc:CrosswalkAssertion` | **1** |
| `rssc:SafetyImpactAssertion` | **0** |
| `rssc:SecurityLevelClaim` | **0** |
| `rssc:ControlGap` | **1** |
| all `rssc:EvidencedAssertion` | **2** |
| `rssc:Standard` | **1** |
| `rssc:ClauseReference` | **2** |
| `rssc:SecurityRequirement` | **0** |
| `rssc:FoundationalRequirement` | **7** |
| `rssc:SafetyFunction` | **0** |
| `rssc:AutonomyBand` | **3** |
| `rssc:Source` | **1** |

## 3. ISO 10218:2025 clause coverage

| Standard | Clauses declared | With >=1 mapping | With no mapping | Reached by any assertion |
|---|---:|---:|---:|---:|
| ISO 10218-1:2025 | **2** | **1** | **1** | **1** |
| **total** | **2** | **1** | **1** | **1** |

Mapping rate over declared clauses: **0.5** (1 of 2).
Reached rate, counting mappings, gaps and safety impacts: **0.5** (1 of 2).

### 3.1 Every declared ISO 10218:2025 clause

The full list. Not sampled, not truncated, not sorted to flatter.

| Standard | Clause | Title | Status | Mappings | Gaps | Safety impacts | Bands |
|---|---|---|---|---:|---:|---:|---|
| ISO 10218-1:2025 | `5.1.16` | Cybersecurity | normative | 1 | 1 | 0 | A, C |
| ISO 10218-1:2025 | `7.5.11` | Cybersecurity | normative | 0 | 0 | 0 | none |

### 3.2 Declared ISO 10218:2025 clauses with NO mapping

1 of 2 declared clauses carry no
`rssc:CrosswalkAssertion`. All of them are listed here.

| Standard | Clause | Title | Reached by a gap or safety impact? |
|---|---|---|---|
| ISO 10218-1:2025 | `7.5.11` | Cybersecurity | **no** |

## 4. Autonomy band distribution

| Band | rssc:CrosswalkAssertion | rssc:SafetyImpactAssertion | rssc:SecurityLevelClaim | rssc:ControlGap | Total |
|---|---:|---:|---:|---:|---:|
| **A** | 1 | 0 | 0 | 0 | **1** |
| **B** | 0 | 0 | 0 | 0 | **0** |
| **C** | 0 | 0 | 0 | 1 | **1** |
| _(no band declared)_ | 0 | 0 | 0 | 0 | **0** |

**Bands with zero assertions: B.**
An empty band is a result of this crosswalk, not a hole in it. Where
no standard in the corpus reaches a band, forcing a clause onto that
row would manufacture coverage that does not exist.

### 4.1 ISO 10218:2025 clauses reached, per band

| Band | Declared ISO 10218 clauses reached | Clause identifiers |
|---|---:|---|
| **A** | 1 | `5.1.16` |
| **B** | 0 | none |
| **C** | 1 | `5.1.16` |

## 5. Security side: what the crosswalk reaches

### 5.1 Foundational requirements

| FR | Label | Assertions pointing at it |
|---|---|---:|
| `FR1` | FR 1. Identification and authentication control | 0 |
| `FR2` | FR 2. Use control | 0 |
| `FR3` | FR 3. System integrity | 2 |
| `FR4` | FR 4. Data confidentiality | 0 |
| `FR5` | FR 5. Restricted data flow | 0 |
| `FR6` | FR 6. Timely response to events | 0 |
| `FR7` | FR 7. Resource availability | 0 |

**Foundational requirements no assertion reaches: `FR1`, `FR2`, `FR4`, `FR5`, `FR6`, `FR7`.**

### 5.2 Security requirements

No `rssc:SecurityRequirement` instances are declared in the graph.

## 6. Evidence type and confidence

| Evidence type | Assertions |
|---|---:|
| `analytical-inference` | 1 |
| `published-preview` | 1 |

| Confidence | Assertions |
|---|---:|
| `low` | 1 |
| `moderate` | 1 |

## 7. What is NOT covered

This section exists so that the reader does not have to reconstruct the
negative space from the tables above. Every item is a count taken from the
same graph.

1. **ISO 10218:2025 clauses declared with no mapping: 1** of 2. Listed in full at 3.2.
2. **ISO 10218:2025 clauses declared that no assertion of any kind reaches: 1.**
   `7.5.11` (ISO 10218-1:2025)
3. **Autonomy bands with zero assertions: 1** of 3 declared.
4. **Foundational requirements no assertion reaches: 6** of 7.
5. **Declared security requirements no assertion reaches: 0** of 0.
6. **Assertions with no autonomy band declared: 0.** These are claims about the corpus that are not tied to a point on the
   gradient, so they are excluded from the per-band arithmetic at 4.
7. **Assertions carrying no `rssc:citation`: 0.** This number must be zero; `validate.py` fails the build if it is not.
8. **Assertions with no `prov:wasDerivedFrom` link to a `rssc:Source`: 0.**
9. **Declared sources nothing is derived from: 0.**
10. **Standards declared with no clause reference attached: 0.**
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

