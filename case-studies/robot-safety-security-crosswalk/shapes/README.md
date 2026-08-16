# RSSC SHACL shapes

The shapes in `rssc-shapes.ttl` are the falsifiability contract of this case
study. They are not a linting pass. They are the mechanism by which an uncited
mapping, an over-claimed confidence grade or a malformed clause number fails
the build instead of shipping.

Part of [Open Ontologies](../../../README.md). Released under CC BY 4.0.

## How to run them

```bash
python3 -c '
import pyshacl
from rdflib import Graph
d = Graph(); d.parse("ontology/rssc.ttl"); d.parse("crosswalk/iso10218-iec62443.ttl")
s = Graph(); s.parse("shapes/rssc-shapes.ttl")
c, _, t = pyshacl.validate(d, shacl_graph=s, inference="rdfs",
                           abort_on_first=False, allow_warnings=True)
print(c); print(t if not c else "")'
```

`allow_warnings=True` is required. Two constraints in Section 8 sit at
`sh:Warning`, and pySHACL reports `conforms=False` for a warning unless that
flag is set. Every constraint that carries the falsifiability contract is at
the SHACL default, `sh:Violation`, and is unaffected by the flag.

`inference="rdfs"` and `inference="none"` both work. Each assertion shape
lists `rssc:EvidencedAssertion` and all four of its subclasses as
`sh:targetClass`, so the shapes behave identically either way.

## What each shape protects against

Seventeen named node shapes. Sixteen at `sh:Violation`, one at `sh:Warning`.

| Shape | Protects against | Severity |
|---|---|---|
| `EvidencedAssertionShape` | An assertion with no citation, no basis, no evidence type, no confidence grade, no `dcterms:source` IRI and no `prov:wasDerivedFrom` link to a retrievable source. Also rejects the evidence type and confidence given as bare literals rather than concept IRIs. | Violation |
| `AnalyticalInferenceCeilingShape` | Our own reasoning presented as high confidence. | Violation |
| `SecondaryLiteratureCeilingShape` | Trade press and consultancy explainers presented as high confidence. | Violation |
| `CrosswalkAssertionShape` | A mapping with no subject clause, no security-side anchor, two subjects, or no declared SKOS relation. Enforces the permitted target classes that the ontology deliberately leaves range-free. | Violation |
| `ExactMatchDisciplineShape` | An identity claim across the safety and security boundary asserted at less than high confidence. | Violation |
| `SafetyImpactAssertionShape` | The central claim of the case study made without naming the degraded safety function, or with a consequence too vague to argue with. | Violation |
| `SecurityLevelClaimShape` | A bare security level attached to a product. The level type (target, capability, achieved) is mandatory and has no default. | Violation |
| `ControlGapShape` | A gap with no autonomy band, no cited clause, no gap kind, or a candidate control that is not of a permitted kind. | Violation |
| `ControlGapCandidateControlShape` | A gap whose candidate control is empty without the emptiness being declared. See the note below: silence stays possible, silent silence does not. | Violation |
| `SilenceGapCeilingShape` | An argument from silence about a paywalled document graded high confidence. | Violation |
| `StandardShape` | An undated designation, a missing document type, an unstated paywall, and a month-only publication date silently upgraded to a fabricated day. | Violation |
| `ClauseReferenceShape` | A clause identifier that is not in either of the two real ISO and IEC forms, an unattributed clause, and an unstated normative status. | Violation |
| `SecurityRequirementShape` | A requirement identifier that is not in one of the four real IEC 62443 grammars, and the `SR 3.1` versus `SR-1` collision between IEC 62443-3-3 and IEC 62443-4-1. | Violation |
| `SourceShape` | A source that cannot be retraced: no URL, or no retrieval date. | Violation |
| `SafetyFunctionShape` | A safety function with no clause behind it, which is an opinion rather than a requirement. | Violation |
| `RoboticSystemShape` | A system placed on an autonomy band that does not exist. | Violation |
| `MintedTermDocumentationShape` | A minted class or property with no label or comment. | Warning |

## Prose versus codes

Human-readable fields (`rssc:citation`, `rssc:basis`, `rssc:safetyConsequence`,
`rssc:exposure`, `rssc:clauseTitle`, `rssc:requirementTitle`) accept either a
plain `xsd:string` or an `@en`-tagged `rdf:langString`. Prose is written for a
reader and may legitimately be translated.

Everything that functions as a code stays strictly `xsd:string`: clause and
requirement identifiers, designations, editions, normative status, standard
type and identifier scheme. A language tag on `5.1.16` or on `informative`
would be meaningless, and it would silently defeat the `sh:in` matching that
enforces the controlled sets.

## Where the identifier patterns come from

Neither pattern was invented. Both were derived from identifiers read in the
published free previews of the actual documents, then tested against the real
identifiers on one side and against plausible fabrications on the other.

**Clause identifiers.** Two forms exist and only two: a dotted decimal clause
number of up to five levels, which may begin with zero (IEC 62443-3-3:2013
clause 0.3 is real), and an annex reference of a single capital letter with up
to three numeric sub-levels. Accepted, and confirmed real: `5.1.16`, `7.5.11`,
`5.2.16`, `7.5.23`, `6.3.7`, `5.2.2`, `5.11.2`, `0.3`, `11`, `4`, `Annex C`,
`Annex L`, `Annex C.3`. Rejected: `clause 5.1.16`, `5.1.16.`, `Annex 4`,
`annex C`, `Table B.1`, `Figure A.1`, `5.1.16 Cybersecurity`. Table and figure
numbers are not clause identifiers and must not be recorded as such.

**Requirement identifiers.** Four grammars, and only four: the dotted
requirement with an optional enhancement (`SR 3.1`, `CR 2.13`, `CR 1.1 RE 1`,
`NDR 1.6 RE 1`, with the component-type variants EDR, HDR, NDR and SAR); the
zone and conduit requirement (`ZCR 1`, `ZCR 3.3`, `ZCR 5.13`); the common
component security constraint (`CCSC 1` to `CCSC 4`); and the hyphenated
practice requirement of IEC 62443-4-1:2018 (`SM-13`, `SR-5`, `SVV-5`, `SUM-1`,
`SG-7`, `DM-6`, `SD-4`, `SI-2`). The leading digit of a dotted requirement is
the foundational requirement number, so it is constrained to 1 to 7 and
`SR 8.1` is rejected. Also rejected: `SR3.1`, `CR 1.1RE1`, `sr 3.1`, `FR 3`,
`SL 2`, `CCSC 9`, `SUM-0`, and `SP.03.01`, which is the commonly reported but
unconfirmed format for IEC 62443-2-4 Annex A requirement IDs. The preview of
that document does not reach Annex A, so the format was not confirmed and is
therefore not accepted.

## What the patterns cannot do, stated plainly

A pattern catches a malformed identifier. It cannot catch a well-formed
identifier that is simply wrong. One published commentary cites the ISO
10218-2:2025 cybersecurity clause as `5.2.26`; the correct identifier, read in
the official ISO preview contents, is `5.2.16`. Both match the pattern.
Nothing in this file will tell you which one is right.

That is why the pattern is the weakest of the seventeen shapes and the citation
requirement is the strongest. The defence against a wrong clause number is not
a regular expression, it is `rssc:citation` plus `prov:wasDerivedFrom` plus a
retrieval date, which together let a reader open the same preview and check.

## The one principled exception, and why it is not a loophole

A gap with no named candidate control is normally a complaint rather than a
finding, so `ControlGapCandidateControlShape` requires one. But the ontology is
explicit that where nothing in the public corpus would address the exposure,
`rssc:wouldBeAddressedBy` should be left unset, because an unfilled gap is a
stronger result than a forced one.

Both rules are right. A flat `sh:minCount 1` would have resolved the conflict
by forcing a fabricated answer into the two most valuable findings in the
artefact: the Band C goal-channel gap, where the IEC 62443 series was searched
and nothing governs a semantic instruction channel, and the informative-chain
gap, where nothing in the public corpus makes the ISO 10218 to IEC 62443
chain normative. That would be the shapes causing precisely the failure they
exist to prevent.

So the constraint is a disjunction. A control gap must either name at least one
candidate control, or name none **and** declare in `rssc:basis`, in as many
words, that the property was deliberately left unset, which obliges the author
to record what corpus was searched and found empty. Silence stays possible;
silent silence does not. An author who simply forgot to fill the property still
fails the build, because forgetting does not write the sentence.

## A scoping choice, recorded rather than hidden

The confidence ceilings are scoped to assertions, not to everything.
`AnalyticalInferenceCeilingShape` targets `rssc:EvidencedAssertion` and its
four subclasses, not every subject of `rssc:evidenceType`. The three autonomy
band concepts in `ontology/rssc.ttl` carry `ev:AnalyticalInference` together
with `conf:high`, which under a wider target would fail the build. That
combination is defensible on a scheme member, where it records that the band
boundaries are ours and that we are certain they are ours, and it is not
defensible on a claim about what a document says. The narrower scope is the
honest reading of the rule. It is flagged here so that a reader who disagrees
can widen the target in one line and see what breaks.

## Verification

The shapes are themselves tested, because a constraint that never fires is
worth nothing and looks identical to one that works.

- Against the real artefact (`ontology/rssc.ttl` plus
  `crosswalk/iso10218-iec62443.ttl`, 2,167 triples): conforms, with zero
  violations and zero warnings.
- Against 44 adversarial cases, each mutating exactly one thing in an otherwise
  valid assertion: every case behaved as specified. Each rule was shown to
  reject the thing it claims to reject (a missing citation, a bare-literal
  confidence, analytical inference graded high, an exact match below high
  confidence, a gap with no band, an undeclared empty candidate control, a
  fabricated `Clause 5.1.16`, an invented `SR 9.99`, a free-text publication
  date, a source with no retrieval date), and every legitimate form was shown
  to pass (`Annex C`, `0.3`, `CR 1.1 RE 1`, `ZCR 3.3`, `SUM-1`, a month-only
  `xsd:gYearMonth` date, and a declared empty candidate control).

The warning shape was checked in both directions: a class with no comment
reports under `allow_warnings=False` and does not fail the build under
`allow_warnings=True`.

## Why a failing artefact is more useful than a passing one

A crosswalk that passes because it asserts nothing checkable is worth less
than one that fails loudly at the point of over-claim.

The failure mode this whole case study exists to avoid is the consultancy
explainer that says ISO 10218:2025 mandates IEC 62443 compliance. That claim
is unfalsifiable as usually written, because it names no clause, no evidence
type and no source. Made inside this vocabulary it becomes falsifiable, and it
then turns out to be false: the normative reference lists of both parts of
ISO 10218:2025 were read in the official previews and contain neither IEC TS
63074 nor any part of IEC 62443, so the link exists only through a NOTE or a
bibliography entry. A mapping asserting it with `ev:NormativeReference` should
fail review. A mapping asserting it with `ev:InformativeReference` and
`conf:moderate` is a finding, and a checkable one.

The same logic runs through every shape here. `SecurityLevelClaimShape` exists
because the widely repeated claim that robot safety components map to SL2
could not be traced to a primary source, and because the claim omits its own
level type, without which it cannot be evaluated at all. `SilenceGapCeilingShape`
exists because an argument from silence about a document you have not bought
is weak evidence, and grading it high would be dishonest about the evidence
base. The ceilings do not make the artefact more cautious. They make its
confidence grades mean something, so that the assertions carrying `conf:high`
can be trusted and the ones carrying `conf:low` can be argued with.

A build that fails tells you exactly which assertion over-reached, names it,
and refuses to ship it. A build that passes tells you that every mapping in
the graph can be checked by a reader with two free PDF previews and no budget.
Neither outcome is a formality. That is the point.

## Licence

CC BY 4.0. No normative text from any ISO or IEC standard is reproduced in
these shapes. Clause identifiers, requirement identifiers, clause titles, part
numbers, editions and publication dates are cited as facts read from published
free previews and catalogue entries. ISO and IEC standards remain the copyright
of ISO and IEC and must be purchased from the issuing body.

Independent, self-initiated open research. Not endorsed by, affiliated with or
approved by ISO, IEC, CEN, CENELEC, BSI, ISA, the ISA Security Compliance
Institute, IEEE, MITRE or any other body named within it.

---

### Built by Tesseract Academy

We build open, checkable reference artefacts in domains where the public
commentary is confident and the primary sources are paywalled.

[gov.tesseract.academy](https://gov.tesseract.academy) · fabio@thetesseractacademy.com
Part of [Open Ontologies](../../../README.md) · CC BY 4.0 · verified, reproducible.
