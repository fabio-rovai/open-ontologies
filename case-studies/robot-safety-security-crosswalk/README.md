# Robot safety ↔ security crosswalk (RSSC)

A provenance-first, machine-readable crosswalk binding the safety clauses of
**ISO 10218-1:2025** and **ISO 10218-2:2025** to the industrial security
requirements of the **IEC 62443** series, through the machinery bridge document
**IEC TS 63074:2023**, organised by a three-band autonomy gradient, in which
every mapping is reified as its own node and carries a citation, an evidence
type and a confidence grade or it does not enter the graph.

Part of [Open Ontologies](../../README.md). Released under CC BY 4.0. No ISO or
IEC normative text is reproduced anywhere in this case study.

## Why this exists

A cyber compromise of a robot is a safety event. If an attacker can raise a
speed limit, suppress a protective stop, delay a stopping function past its
stopping time, or feed spoofed perception data into a separation-distance
calculation, the consequence is a person injured, not a record disclosed. The
machinery safety canon and the industrial security canon were written by
different committees, in different vocabularies, for different audiences, and
until very recently they did not connect.

They now connect. ISO 10218-1:2025 (third edition) and ISO 10218-2:2025 (second
edition), both cover-dated 2025-02, are the first industrial robot safety
standards to carry cybersecurity requirements. Each foreword lists "adding
requirements for cybersecurity" among the main changes from the 2011 editions,
and each part places one cybersecurity clause in the design requirements and a
second in the instruction handbook:

| Part | Clause | Title | Character |
|---|---|---|---|
| ISO 10218-1:2025 | 5.1.16 | Cybersecurity | requirement, under 5.1 Robot design |
| ISO 10218-1:2025 | 7.5.11 | Cybersecurity | information for use, instruction handbook |
| ISO 10218-2:2025 | 5.2.16 | Cybersecurity | requirement, under 5.2 Design |
| ISO 10218-2:2025 | 7.5.23 | Cybersecurity | information for use, instruction handbook |

All four clause identifiers and titles were read in the official ISO free
preview contents pages, which are published without charge. The second pair
matters as much as the first and is almost entirely absent from public
commentary: the standards do not only require the manufacturer and the
integrator to assess cyber risk, they require security information to be handed
down the supply chain in the instruction handbook. That is the mechanism by
which a factory, a warehouse or a hospital inherits a security obligation it
never negotiated.

What is missing is the mapping itself. Every consultancy explainer, vendor page
and trade article asserts a chain from ISO 10218 to IEC TS 63074 to IEC 62443,
usually in a diagram, usually without saying which links are requirements and
which are bibliography. None of them publishes the mapping as data you can
query, count and contradict. This case study does, one cited assertion at a
time, and the counts below are produced by the pipeline rather than claimed.

## What this is not

Read this before using anything below.

- **It is not a substitute for the standards.** You cannot conform to ISO 10218
  or IEC 62443 by reading this repository. Buy the standards from the issuing
  bodies. This graph tells you which clause to open, not what it says.
- **It reproduces no normative text.** ISO and IEC standards are paywalled. What
  is recorded here is the public surface those bodies and their distributors
  publish free: designations, part numbers, editions, cover dates, clause
  identifiers, clause titles, annex letters, annex normative status, scopes and
  normative-reference lists. Where a definition appears in this graph it is our
  own paraphrase and is labelled as one. This constraint is treated as a design
  feature: an artefact built from public surfaces alone is reproducible by any
  reader without buying anything, and the `rssc:paywalled` flag makes the limits
  of the evidence base queryable rather than merely admitted.
- **It is not endorsed by anybody.** Independent, self-initiated open research.
  Not endorsed by, affiliated with or approved by ISO, IEC, CEN, CENELEC, BSI,
  ISA, the ISA Security Compliance Institute, IEEE, MITRE, KAN or any other body
  named within it.
- **It is not a conformity assessment, a certification, or advice.** No claim
  here establishes that any product, cell or installation meets any requirement.
- **Every mapping is either a cited standards-body statement or our own
  analytical inference, and the data says which, on every single assertion.**
  The autonomy gradient is ours. Every judgement that a standard's scope does or
  does not reach a class of system is ours. Those carry
  `rssc:evidenceType ev:AnalyticalInference` and can never be marked high
  confidence. A reader who rejects an inference can discard it without
  discarding the cited facts underneath it, because the two are different nodes.

One further honesty note, stated up front because it contradicts the popular
account of these standards. The widely repeated claim that ISO 10218:2025
"mandates IEC 62443 compliance" is false as stated. The complete normative
reference lists of both parts were read in the official previews. Neither
IEC TS 63074, nor any part of IEC 62443, nor ISO/TR 22100-4 appears in either
list. The linkage is real, and it is informative. What is mandated is a
cybersecurity threat assessment; what follows from it is guidance.

## What it finds

Every number in this section is a key in
[`pipeline/coverage-metrics.json`](pipeline/coverage-metrics.json), written by
[`pipeline/coverage.py`](pipeline/coverage.py). The conformance gate is
[`pipeline/validate.py`](pipeline/validate.py), which exits non-zero on any
violation. Nothing here is typed in by hand.

| | | `coverage-metrics.json` key |
|---|---:|---|
| Standards recorded | **{{STANDARDS}}** | `standards` |
| Clause references | **{{CLAUSE_REFERENCES}}** | `clause_references` |
| Security requirements | **{{SECURITY_REQUIREMENTS}}** | `security_requirements` |
| Evidenced assertions, total | **{{EVIDENCED_ASSERTIONS}}** | `evidenced_assertions` |
| of which crosswalk mappings | **{{CROSSWALK_ASSERTIONS}}** | `crosswalk_assertions` |
| of which safety-impact claims | **{{SAFETY_IMPACT_ASSERTIONS}}** | `safety_impact_assertions` |
| of which security-level claims | **{{SECURITY_LEVEL_CLAIMS}}** | `security_level_claims` |
| of which control gaps | **{{CONTROL_GAPS}}** | `control_gaps` |
| Distinct cited sources | **{{SOURCES}}** | `sources` |
| Assertions lacking a citation | **{{ASSERTIONS_WITHOUT_CITATION}}** | `assertions_without_citation` |
| Assertions lacking a source link | **{{ASSERTIONS_WITHOUT_SOURCE_LINK}}** | `assertions_without_source_link` |
| Triples in the loaded graph | **{{GRAPH_TRIPLES}}** | `graph_triples` |

### Finding 1: the chain is informative at every hop, and that is countable

From `evidence_types` and `confidence_levels` in `coverage-metrics.json`:

| Evidence type | Assertions | Confidence | Assertions |
|---|---:|---|---:|
| Normative reference | **{{EV_NORMATIVE_REFERENCE}}** | High | **{{CONF_HIGH}}** |
| Informative reference | **{{EV_INFORMATIVE_REFERENCE}}** | Moderate | **{{CONF_MODERATE}}** |
| Published preview | **{{EV_PUBLISHED_PREVIEW}}** | Low | **{{CONF_LOW}}** |
| Published summary | **{{EV_PUBLISHED_SUMMARY}}** | | |
| Secondary literature | **{{EV_SECONDARY_LITERATURE}}** | | |
| Analytical inference | **{{EV_ANALYTICAL_INFERENCE}}** | | |

Two facts drive that distribution, and both are checkable by any reader at zero
cost from the free previews:

1. **ISO 10218-1:2025 and ISO 10218-2:2025 do not normatively reference IEC TS
   63074 or IEC 62443.** Part 1's normative reference list runs from
   ISO 3864-1:2011 to IEC 62745:2017 and includes ISO 13849-1:2023 and
   IEC 62061:2021; Part 2's ends at IEC 62061:2021. Neither list contains any
   security document. Any reference to the security canon therefore sits in a
   NOTE or in the Bibliography, both of which are informative.
2. **IEC TS 63074:2023 does not normatively reference IEC 62443 either.** Its
   clause 2 contains exactly one entry, IEC 62061:2021. The IEC 62443 parts
   appear in the introduction, in `[SOURCE: ...]` attributions for defined terms,
   and in the bibliography.

So the bridge is conceptual and terminological rather than a normative
dependency chain. That is not a debunking; the bridge is real and strong. IEC TS
63074:2023 sub-clauses 5.2.2 to 5.2.8 are the seven IEC 62443 foundational
requirements in order (identification and authentication, use control, system
integrity, data confidentiality, restricted data flow, timely response to
events, resource availability), and its Table 1 is titled "Overview of
foundational requirements and possible influence(s) on an SCS". A machinery
committee, IEC TC 44, adopted the structure of a security series owned by a
different committee, IEC TC 65, and re-expressed it against the safety-related
control system. That is the crosswalk, and it is a documented fact rather than
an inference. The distinction this artefact insists on is between that fact and
the marketing claim built on top of it.

### Finding 2: coverage collapses as autonomy rises, and the empty cells are the result

| Band | Assertions | ISO 10218:2025 clauses reached |
|---|---:|---:|
| A. Teleoperated and pre-programmed | **{{ASSERTIONS_BY_BAND_A}}** | **{{ISO10218_CLAUSES_BY_BAND_A}}** |
| B. Conditionally autonomous | **{{ASSERTIONS_BY_BAND_B}}** | **{{ISO10218_CLAUSES_BY_BAND_B}}** |
| C. Highly autonomous and learning-enabled | **{{ASSERTIONS_BY_BAND_C}}** | **{{ISO10218_CLAUSES_BY_BAND_C}}** |

From `assertions_by_band` and `iso10218_clauses_by_band`. Of the
**{{ISO10218_CLAUSES_DECLARED}}** ISO 10218:2025 clauses this project could cite
from the public contents pages, **{{ISO10218_CLAUSES_MAPPED}}** carry at least
one mapping and **{{ISO10218_CLAUSES_UNMAPPED}}** carry none. The unmapped
clauses are listed in full in
[`pipeline/coverage-report.md`](pipeline/coverage-report.md), never sampled and
never truncated.

Note the denominator. It is the set of clauses **declared in this graph**, not
the full clause tree of either part. Those documents are paywalled: their
contents pages are public, their clause bodies are not. A percentage against the
whole standard would rest on a fabricated denominator, so none is printed.

The thin rows are the headline, not a failure of effort. ISO 10218:2025
explicitly excludes, in the scope clause of both parts, underwater, law
enforcement, military and defence, airborne and space, medical, healthcare,
prosthetic, service and consumer applications, and anything lifting or
transporting people. Part 1 additionally excludes robots fixed to or forming
part of driverless industrial trucks or mobile platforms, and Part 2 excludes
the same where integrated with them. The standard therefore governs Band A and
the caged, fixed-installation end of Band B, and stops. Autonomous mobile
robots, agricultural platforms and uncrewed aircraft flown beyond visual line of
sight fall to other documents entirely. Learning-enabled Band C systems are
addressed by no machinery type-C standard at all.

We do not fill those cells by forcing ISO 10218 clauses onto rows they do not
cover. Where a band implies an exposure that no cited clause reaches, the graph
records a `rssc:ControlGap` with one of three kinds, and the difference between
them carries the argument:

| Gap kind | Meaning | Evidential strength |
|---|---|---|
| `excluded-by-scope` | the standard's own scope clause puts it outside coverage | strongest: a committee decision, recorded in a clause published free |
| `not-addressed` | the published structure is silent, and the scope neither includes nor excludes it | weaker: an absence, and an argument from silence about a paywalled document, so it carries lower confidence |
| `informative-only` | reached, but only through a NOTE, a bibliography entry or an informative annex | coverage a reader may mistake for an obligation, and the kind public commentary most often misreports |

The **{{CONTROL_GAPS}}** control gaps are broken down by kind, band and the
clause each one points at in
[`pipeline/coverage-report.md`](pipeline/coverage-report.md).

### Finding 3: the SL2 claim does not survive checking

The most repeated single assertion about these standards is that safety-related
robot components map to IEC 62443 security level SL2. We could not trace it to
any primary source. It is not in the ISO 10218-1:2025 preview, and an
independent clause-by-clause revision guide for Part 2 states that the standard
names no security levels at all. The traceable origins are an expert commentary
and an unattributed news aggregator.

The nearest defensible statement is narrower and belongs to a named expert
rather than to the standard: writing in KANBrief 2/2023 about the final drafts
submitted in March 2022, Otto Görnemann of SICK AG states that security level 2
is "generally assumed adequate" for parts of the control system that may affect
safety, and security level 1 for other parts. Note three things that downstream
repetition drops: the wording is an assumption rather than a mandate, the
statement is two-tier and the SL1 half is always omitted, and it describes a
draft roughly two years before the February 2025 publication. A separate and
better-sourced claim, that SL2 is a sensible minimum capability level for
industrial control system components generally, comes from the ISA Security
Compliance Institute and is not about robots.

In this graph that claim exists only as a `rssc:SecurityLevelClaim` carrying
`ev:SecondaryLiterature` and `conf:low`, with the source named. It is recorded
so that it can be contradicted, not so that it can be cited.

A related discipline the vocabulary enforces structurally: a claim that a robot
"is SL2" is unusable unless it says which kind of level is meant. A target level
(SL-T) is a requirement set by risk assessment, a capability level (SL-C) is a
property of a product as supplied, and an achieved level (SL-A) is a property of
an installation as it actually stands. A controller with a capability level of 2
dropped into a flat, unsegmented network attains far less than it could. Every
security-level claim in this graph must carry its type, and there is no default.

## The design commitment: no unsourced mapping

Every mapping is **reified** as its own `rssc:EvidencedAssertion` node carrying a
citation, an evidence type from a controlled scheme, a confidence grade, a
plain-language basis saying what the evidence is taken to show, and a
`prov:wasDerivedFrom` link to a `rssc:Source` with a resolvable URL and a
retrieval date. This is not a convention, it is enforced. Every rule below runs
at `sh:Violation` severity with no downgrade, so any one of them failing fails
the build. The only two constraints in
[`shapes/rssc-shapes.ttl`](shapes/rssc-shapes.ttl) that carry `sh:Warning` are
cosmetic (a minted class or property missing its label or comment) and they are
the only ones. The build fails if any of the following does not hold:

1. Every `rssc:EvidencedAssertion` carries at least one `rssc:citation`.
2. Every `rssc:EvidencedAssertion` carries exactly one `rssc:evidenceType`, drawn
   from the six concepts of the evidence-type scheme.
3. Every `rssc:EvidencedAssertion` carries exactly one `rssc:confidence`, drawn
   from `conf:high`, `conf:moderate` or `conf:low`.
4. **No assertion whose evidence type is `ev:AnalyticalInference` may carry
   `conf:high`.** Our own reasoning is never high-confidence evidence. This is
   the rule that stops the artefact laundering opinion into fact, and it is the
   one to break first if you want to falsify the work. Three further ceilings
   work the same way and for the same reason:
   `ev:SecondaryLiterature` may not carry `conf:high` either, because commentary
   is frequently wrong at clause level; a `rssc:ControlGap` of kind
   `gap:not-addressed` may not carry `conf:high`, because it rests on an
   argument from silence about a document whose body we cannot read; and
   `skos:exactMatch` may only be asserted at `conf:high`, because an identity
   claimed on weak evidence is a guess wearing a stronger relation.
5. Every `rssc:Source` carries exactly one `rssc:sourceUrl` and one
   `rssc:retrievedOn` date.
6. Every `rssc:CrosswalkAssertion` carries exactly one `rssc:subjectClause`, at
   least one `rssc:targetRequirement`, and a `rssc:mappingRelation` drawn from
   the five SKOS mapping relations.
7. Every `rssc:SecurityLevelClaim` carries both a `rssc:claimedLevel` and a
   `rssc:securityLevelType`. There is no default type.
8. Every `rssc:ControlGap` carries a `rssc:gapKind` and points at the specific
   clause it says does not address the exposure, not at a whole standard.
9. Every `rssc:ClauseReference` and `rssc:SecurityRequirement` carries exactly
   one identifier literal and exactly one `rssc:inStandard`. An identifier with
   no document behind it is unattributable and therefore unusable.
10. Every `rssc:SafetyFunction` carries a `rssc:definedByClause`. A safety
    function with no clause behind it is an opinion, not a requirement.

Match discipline for the mapping relation, on the model used elsewhere in this
repository: `skos:exactMatch` only where two terms denote the same thing with no
scope difference, which across a safety and security boundary is almost never;
`skos:closeMatch` for a strong but not identical correspondence;
`skos:broadMatch` or `skos:narrowMatch` where one side is strictly wider;
`skos:relatedMatch` where the connection is real but the scopes cross rather
than nest. When in doubt the relation is weakened and the reason is written into
`rssc:basis`.

The corresponding negative commitment is recorded in the ontology itself, in a
closing block listing what was removed rather than guessed. Nothing was
constructed by inference from a numbering pattern. Where two sources disagreed
about a clause identifier, neither was recorded as fact.

## The autonomy gradient

The organising variable is not sector and not robot morphology. It is how much
of the safety envelope is computed at run time rather than fixed at design time,
because that is what determines whether a machinery type-C standard can reach
the system at all, and it is also what determines which attack surfaces exist.

| Band | Definition | Typical members | Where the safety envelope lives | Reached by ISO 10218:2025 |
|---|---|---|---|---|
| **A** | Teleoperated and pre-programmed | fixed industrial cells, teleoperated inspection manipulators | fixed at design and commissioning | yes, this is the band it was written for |
| **B** | Conditionally autonomous | warehouse and hospital AMRs, agricultural platforms, drones beyond visual line of sight | run-time perception | only the caged, fixed-installation end; mobile platforms are excluded by scope |
| **C** | Highly autonomous and learning-enabled | foundation-model-driven manipulation, humanoids, swarms | learned policy, updated after deployment | no |

The gradient is an analytical construct of this project. It is deliberately
coarse and it is not any standards body's classification. It is explicitly not
the SAE J3016 driving-automation levels, and no mapping to them is asserted
anywhere. It is also not the ISO 10218-1:2025 robot classes (Class I and Class
II), which grade the robot's own force and motion capability for functional
safety purposes, not its autonomy. The three band concepts carry
`ev:AnalyticalInference` and say so in their own definitions.

Why it is the right variable, band by band:

- **Band A.** Sensing is not the safety envelope; a guard and a fixed limit are.
  The exposure is the configuration channel: whoever holds the teach pendant, the
  engineering laptop or the safety-configuration tool can change a safety limit.
  That is an identification, authentication and use-control problem, and it maps
  cleanly onto the first two foundational requirements.
- **Band B.** Sensing *is* the safety envelope. Any security property protecting
  the integrity or availability of perception data becomes a safety property.
  The relevant hooks are documented and specific: IEC TS 63074:2023 clause 6.3.7
  Remote access, 6.3.5 Portable devices, 6.3.4 Network architecture and 6.3.6
  Wireless communication. Meanwhile the type-C robot standard has excluded the
  mobile platform by scope, so the safety case and the security case are being
  made in two documents that do not cite each other.
- **Band C.** The trained model, its weights, its update channel and its prompt
  or goal channel are all attack surfaces whose compromise changes behaviour
  without any component failing. Neither the motion nor, in the general case, the
  set of reachable states is enumerable before operation, which is what a type-C
  standard's verification and validation clauses assume. A working-group expert
  account records that the ISO 10218 drafts deliberately do not support
  requirements concerning the application of self-developing AI in safety
  functions. The Band C rows are therefore thin by construction, and that is the
  measurement.

## Files

The graph is committed as Turtle rather than generated from a private source, so
what you read is what is validated. There is no build step to trust.

```text
case-studies/robot-safety-security-crosswalk/
├── README.md                          (this file)
├── BUILD_REPORT.md                    (the honesty log)
├── CHANGELOG.md
├── CONTRIBUTING.md
├── LICENSE                            (CC BY 4.0 + MIT + ISO/IEC rights)
├── ontology/
│   └── rssc.ttl                       (the vocabulary)
├── crosswalk/
│   └── iso10218-iec62443.ttl          (the reified, cited mappings)
├── shapes/
│   ├── rssc-shapes.ttl                (the falsifiability contract)
│   └── README.md                      (what each shape refuses, and why)
├── pipeline/
│   ├── rssc_graph.py                  (shared loader)
│   ├── validate.py                    (the gate)
│   ├── coverage.py                    (the counter)
│   ├── coverage-report.md             (generated, committed)
│   ├── coverage-metrics.json          (generated, committed)
│   ├── requirements.txt
│   └── selftest/                      (fixtures that must pass and must fail)
└── demo/
    └── index.html                     (offline browser, no network at all)
```

- `ontology/rssc.ttl`: the RSSC vocabulary. Classes, properties and the eight
  controlled SKOS schemes (autonomy bands, supply chain roles, security levels,
  security level types, foundational requirements, evidence types, confidence
  levels, gap kinds). Reuses SKOS, PROV-O, Dublin Core, W3C ORG and FOAF rather
  than minting equivalents.
- `crosswalk/iso10218-iec62443.ttl`: the mapping triples, and the record of
  account. Every reified assertion with its citation, evidence type, confidence,
  basis, band and role, plus the `rssc:Source` nodes they derive from.
- `shapes/rssc-shapes.ttl`: the SHACL shapes that enforce the falsifiability
  contract listed above. `shapes/README.md` explains each shape in prose.
- `pipeline/rssc_graph.py`: the shared loader. Finds every `.ttl` under
  `ontology/`, `crosswalk/`, `data/` and the case-study root by glob, so both
  scripts see exactly the same graph, and reports the locations that were empty
  rather than assuming they do not exist.
- `pipeline/validate.py`: SHACL validation plus an independent contract check
  that runs before SHACL. Exits 0 if everything passed, 1 on violations or
  contract failures, 2 if there was nothing to validate.
- `pipeline/coverage.py`: writes `pipeline/coverage-report.md` and
  `pipeline/coverage-metrics.json`. Every count in this README comes from the
  latter.
- `pipeline/selftest/`: a fixture that must validate and a fixture that must
  fail, so that a shapes file which quietly stopped enforcing the contract is
  itself caught.
- `pipeline/requirements.txt`: `rdflib>=7.0`, `pyshacl>=0.26`.
- `demo/index.html`: a single self-contained page for reading the crosswalk by
  autonomy band. No web fonts, no CDN, no remote images, no network calls of any
  kind. Open it from the filesystem.
- `BUILD_REPORT.md`: the honesty log. What was fetched, what was computed, which
  clause identifiers were verified against which public source, and what could
  not be obtained.

## Reproduce

The only prerequisite is Python 3.9 or later. Nothing here needs the
`open-ontologies` binary, a triple store, or a network connection, and there is
no `make` target.

```bash
cd case-studies/robot-safety-security-crosswalk
python3 -m venv .venv
./.venv/bin/pip install -r pipeline/requirements.txt
```

Run the validator. This is the gate: it loads every Turtle file in the case
study, applies the contract check, then runs pySHACL against
`shapes/rssc-shapes.ttl`, printing each violation with its focus node, path,
value, source shape and message.

```bash
./.venv/bin/python pipeline/validate.py
echo $?          # 0 = passed, 1 = violations or contract failures, 2 = nothing to validate
```

A non-zero exit is the falsifiability contract firing, not a fault in your
environment. The contract check deliberately duplicates part of the shapes: a
shapes file that quietly dropped the citation rule would otherwise let an
uncited mapping through, and an uncited mapping is the one thing this artefact
promises cannot enter the graph. Both must pass.

Run the coverage report. It counts what is covered, and lists in full what is
not: every declared ISO 10218:2025 clause with no mapping, every autonomy band
that comes out empty, and every foundational requirement nothing reaches.

```bash
./.venv/bin/python pipeline/coverage.py
```

That writes two files and prints the summary:

```bash
less pipeline/coverage-report.md                   # the readable report
python3 -m json.tool pipeline/coverage-metrics.json # the machine-readable counts
```

Both scripts take no arguments and read fixed paths, as the house pipelines do.
To run pySHACL directly against the shapes, for example while editing them:

```bash
./.venv/bin/python -m pyshacl -s shapes/rssc-shapes.ttl -f human \
    ontology/rssc.ttl crosswalk/*.ttl
```

Check the shapes themselves. `pipeline/selftest/` holds one fixture that must
validate and one that must not, so that a shapes file which has quietly stopped
enforcing the contract is caught rather than trusted. Point the loader at a
fixture with `RSSC_ROOT`, which is the only reason that variable exists:

```bash
RSSC_ROOT=pipeline/selftest/valid   ./.venv/bin/python pipeline/validate.py  # expect 0
RSSC_ROOT=pipeline/selftest/invalid ./.venv/bin/python pipeline/validate.py  # expect 1
```

Read the crosswalk without a triple store. `demo/index.html` is a single
self-contained page listing the exposures by autonomy band with the safety
clause and the security requirement on each row, and the evidence type and
confidence beside them. It makes no network calls at all, so open it straight
from the filesystem:

```bash
open demo/index.html          # macOS; use xdg-open on Linux
```

Verified on this build with Python 3.13.14, rdflib 7.6.0 and pyshacl 0.40.1, and
on Python 3.9.6 with rdflib 7.6.0 and pyshacl 0.31.0. The scripts avoid PEP 604
unions and `match` statements so that 3.9 stays supported.

## Honest scope

This is a crosswalk built entirely from the public surfaces of paywalled
documents. That constraint is stated in the ontology, flagged per source with
`rssc:paywalled`, and it bounds what the artefact can claim. Specifically:

- **We know where clauses are, not what they say.** A clause identifier and title
  read from a contents page is primary evidence for the existence and placement
  of a requirement. It is not evidence for the requirement's content. Where this
  graph characterises what a clause requires, the evidence type is
  `ev:PublishedSummary` or weaker and the assertion says so.
- **Some things could not be confirmed and were left out rather than guessed.**
  Which edition of IEC 63074 the ISO 10218 bibliography actually cites, the
  Technical Report of 2019 or the Technical Specification of 2023, is unresolved,
  and the two are not equivalent in standing. Whether IEC 62443 is named in the
  body of the cybersecurity clauses or only in the bibliography is unverified.
  The numeric thresholds for the ISO 10218-1:2025 robot classes are not recorded,
  because sources disagree both on the values and on whether the labels use Roman
  or Arabic numerals. The literal requirement identifier format of
  IEC 62443-2-4:2023 Annex A is not asserted, because the preview stops before
  the annex.
- **SL0 is a degenerate case, not a specified level.** Several authoritative
  descriptions of the IEC 62443 series enumerate only SL1 to SL4. SL0 is included
  in the scheme at moderate confidence so that "no level required" can be said
  explicitly rather than by omission.
- **The requirement-to-security-level mapping tables are deliberately absent.**
  IEC 62443-3-3:2013 Annex B Table B.1 and IEC 62443-4-2:2019 Annex B Table B.1
  are paywalled, and both are informative rather than normative, which is itself
  worth knowing. The vocabulary provides no property for that mapping, because
  providing one would invite reproducing the tables.
- **The Band C rows are thin because the standards are thin, not because the
  work stopped.** We resisted the temptation to map foundation-model exposures
  onto ISO 10218 clauses that plainly do not cover them. If you want the Band C
  rows filled, the honest route is a new standard, not a longer crosswalk.
- **Coverage is counted per band, never asserted overall.** A single headline
  percentage across all three bands would be meaningless, because the denominator
  differs by band and one of the bands is out of scope by the standard's own
  decision.
- **Public commentary on these standards is unreliable at clause level, and we
  are not exempt.** One widely read source numbers the Part 2 cybersecurity
  clause 5.2.26; the official contents page says 5.2.16. Publication dates of
  31 January, 5 February and 18 February 2025 all circulate; the ISO cover pages
  say 2025-02, so this graph records a month and not a day. If we have made the
  same class of error somewhere, the citation attached to the assertion is what
  lets you find it.
- **Official ISO and IEC titles use an em dash to separate title elements.**
  House style forbids that character, so titles appear here in the BSI and EN
  full-stop rendering ("Robotics. Safety requirements. Part 1: Industrial
  robots"). The separator is the only alteration made to any official title.

### Contributing a correction

Corrections are the point of publishing this. The most valuable contribution is
a falsification: a clause identifier we got wrong, an evidence type we graded too
generously, or a mapping that does not hold.

Open an issue or a pull request against the Turtle under `crosswalk/`, which is
the record of account. A correction needs three things and will be rejected
without them: the assertion IRI or clause identifier you are contesting, the
public source you read (a URL that resolves, naming the page or contents entry
you read it on), and what the evidence actually supports. If the fix is "this
should be `conf:low`, not `conf:moderate`", say so; downgrades are as welcome as
new mappings and are merged faster. Then run `pipeline/validate.py` and
`pipeline/coverage.py`, and update any count quoted above from the regenerated
`coverage-metrics.json`. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the full
workflow and for the list of things not to submit.

If you hold the standards and can confirm or refute one of the unresolved items
above, please do not quote the normative text into a public issue. Tell us which
way it goes and cite the clause; we will record the outcome with an appropriate
evidence type and credit the correction in `CHANGELOG.md`.

## Licence and citation

Released under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). See
[`LICENSE`](LICENSE) for the per-layer terms, including the third-party rights
statement covering ISO and IEC material.

ISO and IEC standards remain the copyright of ISO and IEC and must be purchased
from the issuing body. Standard designations, part numbers, editions,
publication dates, clause identifiers and clause titles are cited here as facts,
read from publicly published front matter, contents pages, scopes,
normative-reference lists and catalogue entries.

```text
Rovai, F. (2026). Robot safety and security crosswalk (RSSC), version 0.1.0.
Kampakis and Co Ltd, trading as The Tesseract Academy. CC BY 4.0.
https://github.com/fabio-rovai/open-ontologies/tree/main/case-studies/robot-safety-security-crosswalk
```

---

### Built by Tesseract Academy

We build the assurance layer for systems that have to be right rather than
merely plausible: ontologies, crosswalks and benchmarks in which every claim
carries its own evidence and the failures are counted instead of smoothed over.
If you are integrating robots and need the safety case and the security case to
be the same case, we can build the mapping for your installation and leave you
the graph.

[gov.tesseract.academy](https://gov.tesseract.academy) · fabio@thetesseractacademy.com
Part of [Open Ontologies](../../README.md) · CC BY 4.0 · cited, counted, falsifiable.
