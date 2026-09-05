# Foundry Ontology to OWL 2, and the loss in both directions

A working crosswalk between the Palantir Foundry Ontology and the W3C standards
stack, built to measure what the crossing costs rather than to claim it is free.

## Why this exists

Palantir's white paper *Enabling Interoperability and Preventing Lock-In with
Foundry* states that Foundry offers "the ability to import/export in multiple
open formats (RDF XML/TTL, OWL, etc.) natively".

The product documentation describes something else. The Ontology Manager export
produces JSON, and Palantir's own note on it reads: "You should not depend on
the exported JSON schema as it may change over time." The type reference says
only that Foundry data types are "inspired by similar concepts in RDF, OWL and
XSD". No RDF or OWL export path appears anywhere in the documentation.

So the bridge is missing. This case study builds it, and then measures the gap
it cannot close.

## What is measured

Two directions, both counted rather than asserted.

**Foundry to OWL.** Of the 22 property types in Palantir's `ObjectPropertyType`
union, 11 have a direct XSD counterpart, 2 need OGC GeoSPARQL, 2 are structural
rather than scalar, and 7 have no counterpart in any published standard. Among
those 7 is `marking`, the Foundry security classification. Exported to OWL, a
marking becomes an ordinary literal, indistinguishable from data and no longer
enforceable.

**OWL to Foundry.** Five real ontologies were parsed, their asserted constructs
counted, and each construct checked against the fields that exist in Palantir's
ontology model. The UK Government Information Exchange Standard, at its current
version 5.0.3, loses its entire property hierarchy: 202 `rdfs:subPropertyOf`
assertions have no destination. The archived IES4 loses 200 the same way, so
this is a property of the standard's shape rather than of one release. The pizza
reference ontology, which uses OWL as a logic rather than as a vocabulary,
strands 1,241 assertions.

The pattern is consistent. Foundry holds a taxonomy well. It has no field for a
theory. The constructs with no destination are precisely the ones a reasoner
uses to detect that something is wrong.

Full figures, with the chart, are in
[`ontology/coverage-report.md`](ontology/coverage-report.md).

## Ground truth

Nothing here rests on a screenshot or a blog post. The Foundry side is taken
from Palantir's own Apache-2.0 licensed source:

- `palantir/foundry-platform-python`, the wire models for the v2 Ontologies API,
  which define what a Foundry Ontology is.
- `palantir/osdk-ts`, the fixtures Palantir uses to mock its own API, including
  an object type named `objectTypeWithAllPropertyTypes` that exercises the whole
  union.

The ontologies audited in the other direction are vendored the same way. The
Information Exchange Standard is taken from `IES-Org/ont-ies`, which is the
maintained home of the standard since `dstl/IES4` was archived on 4 March 2025.

Both are vendored under `vendor/` with URL, licence and SHA-256 recorded in
[`data/palantir-sources.json`](data/palantir-sources.json). The property type
list is parsed out of Palantir's source at build time, so a type added upstream
breaks the build instead of passing silently into the unmappable bucket.

## Running it

```bash
python3 vendor_sources.py       # fetch Palantir's sources, record checksums
python3 extract_type_system.py  # parse the property type union
python3 build_fixture.py        # assemble an ontology export from the fixtures
python3 foundry_owl.py          # cross to OWL 2 and SHACL, write the loss report
python3 owl_to_foundry.py       # audit the other direction on real ontologies
python3 generate_report.py      # render the report and its charts
python3 -m pytest test_crosswalk.py -q
```

Verification uses this repository's own engine:

```bash
open-ontologies validate ontology/foundry-crosswalk.ttl
open-ontologies defects  ontology/foundry-crosswalk.ttl
open-ontologies lint     ontology/foundry-crosswalk.ttl
```

The defects check earned its place. It caught a real fault in the first version
of the crosswalk, which declared each recovered inverse in one direction only.

## Scope and method

The fixture is Palantir's published test data, not a production ontology, so the
counts describe the type system faithfully and the size of a real deployment not
at all. The mapping choices are ours and are stated in `TYPE_MAP`, where each
entry carries the reason it was made. Six lint warnings remain on the output;
all six are missing descriptions inherited from Palantir's fixture, where 5 of 9
object types and 55 of 68 properties carry none. The crosswalk does not invent
documentation to make its output look better.

## Licence

Apache-2.0, matching the vendored Palantir sources.
