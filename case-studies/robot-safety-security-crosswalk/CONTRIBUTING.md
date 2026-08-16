# Contributing to RSSC

RSSC is a case study under the [Open Ontologies](https://github.com/fabio-rovai/open-ontologies)
project, crosswalking machinery and robot safety clauses to industrial cyber
security requirements across a three-band autonomy gradient.

The single most valuable contribution is a **falsification**. Every assertion in
this graph is reified precisely so that it can be contested one at a time. A
correction that downgrades one of our confidence grades is worth more to the
artefact than a dozen new mappings, and it will be merged faster.

## How to contribute

### Corrections to clause identifiers, titles and dates

Public commentary on ISO 10218:2025 and the IEC 62443 series is unreliable at
clause level, and we are not exempt from that. If you can show that an
identifier, a title, an edition, a normative status or a date in the crosswalk
Turtle is wrong, open an issue or a pull request with:

- the assertion IRI or the clause identifier you are contesting
- the public source you read, as a URL that resolves, naming the page, contents
  entry or catalogue field you read it on
- what the evidence actually supports

If you hold a paywalled standard and can settle one of the open items in
`CHANGELOG.md`, please **do not quote the normative text** into a public issue or
pull request. Tell us which way it goes, cite the clause, and we will record the
outcome with an appropriate evidence type and credit you in the changelog.

### Evidence-type and confidence downgrades

Report an assertion whose `rssc:evidenceType` is graded too generously. The
distinctions that matter most, in the order we get them wrong:

- `ev:NormativeReference` used where the link is actually a bibliography entry
  or a NOTE, which makes it `ev:InformativeReference`
- `ev:PublishedPreview` used to support a claim about what a clause *requires*,
  when a preview is evidence only for where a clause *sits*
- `ev:AnalyticalInference` presented as though a standards body had said it
- any assertion graded `conf:high` on a single secondary source

The shapes already refuse `ev:AnalyticalInference` combined with `conf:high`. If
you find a way to launder an inference into a high-confidence claim that the
shapes do not catch, that is a bug in the contract and we want the report.

### New mappings

New crosswalk assertions are welcome, subject to the same contract as the
existing ones. A mapping must carry a citation, an evidence type, a confidence
grade, a plain-language `rssc:basis` saying what the evidence is taken to show,
and a `prov:wasDerivedFrom` link to a source with a resolvable URL and a
retrieval date. The build will reject it otherwise, which is the intended
behaviour rather than an obstacle to work around.

Weaken the mapping relation when in doubt. `skos:exactMatch` across a safety and
security boundary is almost never right; say why you chose the relation you chose
in the basis.

### New bands, sectors and adjacent standards

The autonomy gradient stops at three bands on purpose, but the Band B and Band C
rows are thin because the machinery standards are thin, and the adjacent
documents that do cover them are the obvious next surface: ISO 3691-4 for
driverless industrial trucks, ISO 18497 for agricultural machinery, aviation
regulation for uncrewed aircraft beyond visual line of sight, and the emerging
AI-assurance material for learning-enabled behaviour. Worked additions that keep
the same evidence discipline are welcome.

### Competency questions

If there is a question a reader of these standards genuinely needs answered and
the graph cannot answer it, open an issue with the question in plain English and,
if you can, the SPARQL you tried. Questions that the graph *should* be able to
answer and cannot are design feedback, not user error.

## Please do not submit

- Normative text from any ISO or IEC standard, in any quantity, in any form
- A clause identifier, requirement identifier, edition or date you have not read
  in a published source. Constructing one by inference from a numbering pattern
  is exactly the failure this artefact exists to avoid
- A mapping without a citation and an evidence type. It will fail the build
- Assertions sourced from AI-generated news aggregators or content farms. One
  such source is the traceable origin of the SL2 claim this artefact declines to
  endorse
- Marketing claims about standards conformity, from any vendor, presented as
  facts about a standard
- Certification or conformity-assessment advice. This is a reference artefact,
  not a compliance service

## Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b fix/clause-identifier-xyz`)
3. Edit the Turtle under `crosswalk/`, which is the record of account. The
   vocabulary in `ontology/rssc.ttl` changes only when a mapping cannot be
   expressed with the existing terms, and that needs its own justification
4. Validate, then recount:

   ```bash
   python3 -m venv .venv
   ./.venv/bin/pip install -r pipeline/requirements.txt
   ./.venv/bin/python pipeline/validate.py    # must exit 0
   ./.venv/bin/python pipeline/coverage.py    # rewrites the coverage report
   ```

5. Check that the counts you changed moved as you expected in
   `pipeline/coverage-metrics.json`, and update any count quoted in `README.md`
   from that file. Every number in the README is a key in it; nothing is
   asserted by hand
6. Add a line to `CHANGELOG.md` under `## [Unreleased]`
7. Open a pull request describing the change and the evidence behind it

## Style notes

- TTL files use 4-space indentation, one statement per logical group, comments
  explain non-obvious modelling choices
- `rssc:evidenceType` and `rssc:confidence` take **concept IRIs**, not literals.
  `rssc:confidence conf:low` is correct; `rssc:confidence "low"` will fail SHACL.
  The flat string form is available as `skos:notation` on each concept
- Mint clause anchors under this project's own instance base
  (`https://tesseract.academy/id/rssc/`) and carry the real identifier as a
  literal. Never mint an `iso:` or `iec:` namespace: ISO and IEC issue no
  dereferenceable IRIs for their clauses, and an invented one implies an
  authority that does not exist
- An `rdfs:comment` on a mapping is a paragraph of justification, not a gloss.
  Say what would falsify it
- Markdown follows the existing case-study tone: direct, evidence-led, no
  marketing fluff. British English in prose, `licence` as a noun, `LICENSE` as a
  filename, `dcterms:license` as a property
- No em dashes anywhere. Use commas, colons or parentheses
- Python scripts are self-contained and type-hinted, with no dependencies beyond
  `rdflib` and `pyshacl`

## Questions

Open an issue, or contact `fabio@thetesseractacademy.com`.
