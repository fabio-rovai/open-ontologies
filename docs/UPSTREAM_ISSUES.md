# Upstream issues

Reproducible defects found in dependencies while working on this repository.
Nothing here has been filed. Each entry is a minimal case, staged for a human to
send.

---

## 1. Twelve XSD integer-derived datatypes, and `xsd:dateTimeStamp`, are not preserved

**Two independent defects, in two crates.** They were first reported here as one,
blamed on storage. That was wrong, and the correction matters for anyone acting
on it: fixing the storage encoder alone leaves `DATATYPE()` still returning the
wrong IRI, because the expression evaluator collapses the same set again.

**Repository:** `oxigraph/oxigraph` · **Version:** 0.5.9 · **Severity: high.**
The store returns a literal with a different datatype IRI from the one it was
given. RDF 1.1 makes the datatype IRI part of a literal's identity, so this is a
change to the data, not a representation detail.

### Reproduction

```turtle
@prefix ex:  <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:s ex:p "0"^^xsd:nonNegativeInteger .
```

```sparql
SELECT (DATATYPE(?v) AS ?dt) WHERE { ?s ?p ?v }
```

Observed: `xsd:integer`. Expected: `xsd:nonNegativeInteger`, which is what
rdflib returns for the same input.

### Scope, measured

Affected, all collapsing to `xsd:integer`:

`byte`, `short`, `int`, `long`, `unsignedByte`, `unsignedShort`, `unsignedInt`,
`unsignedLong`, `positiveInteger`, `negativeInteger`, `nonPositiveInteger`,
`nonNegativeInteger`

`xsd:dateTimeStamp` collapses into `xsd:dateTime` the same way.

Not affected: `integer`, `decimal`, `float`, `double`, `boolean`, `string`,
`token`, `normalizedString`, `Name`, `NCName`, `language`, `anyURI`, `date`,
`dateTime`, `gYear`, `duration`, `yearMonthDuration`, `dayTimeDuration`.

### 1a. oxigraph storage: the datatype is dropped on insert

`oxigraph` 0.5.9, `src/storage/numeric_encoder.rs`. Twelve datatype IRIs share one
match arm calling `parse_integer_str`, which yields `EncodedTerm::IntegerLiteral`:
a single variant carrying no datatype IRI, so read-back can only reconstruct
`xsd:integer`.

The parser is not at fault. Parsing the document above and inspecting the quad
before insertion gives the correct datatype every time; the loss happens at
storage encoding. Demonstrated by a load-then-serialise round trip, which does
not involve the query evaluator at all: `"0"^^xsd:nonNegativeInteger` is written
back out as bare `0`.

### 1b. spareval evaluator: `DATATYPE()` collapses the same set again

`spareval` 0.2.6, `src/dataset.rs` maps the same twelve IRIs to
`ExpressionTerm::IntegerLiteral`, and `src/expression.rs` answers `DATATYPE()` on
that variant with `xsd:integer`.

This is independent of storage. A literal typed in the query text, never stored,
loses its datatype just the same:

```sparql
SELECT (DATATYPE("0"^^xsd:nonNegativeInteger) AS ?dt) WHERE { }
```

Observed `xsd:integer`; rdflib answers `xsd:nonNegativeInteger`. In the same
query `DATATYPE("abc"^^xsd:token)` correctly answers `xsd:token`, so the
behaviour is specific to this set and not a general canonicalisation policy.

SPARQL 1.1 defines `DATATYPE` as returning the datatype IRI of the literal, so
this is a conformance defect and not only a fidelity one.

### Why this reads as a defect rather than a deliberate simplification

`xsd:yearMonthDuration` and `xsd:dayTimeDuration` are derived types too, and they
have their own encodings and survive intact. The treatment of derived types is
inconsistent, and the inconsistency is not documented.

### Consequences seen in practice

1. A SHACL `sh:datatype xsd:nonNegativeInteger` constraint cannot be decided
   against the store: a conforming literal and a widened one are the same term by
   the time the query runs. Answering anyway reported a violation against every
   value that satisfied the shape, nine of them in one repository, which is how
   this was found.
2. A load-then-serialise round trip rewrites the published file.
   `"0"^^xsd:nonNegativeInteger` comes back out as bare `0`.

### Suggested fix

Both layers need the datatype IRI carried alongside the numeric value.

Declining the native encoding and falling through to the generic typed-literal
path fixes fidelity but costs numeric semantics: these types are in the SPARQL
numeric hierarchy, so arithmetic and ordering over them must keep working. That
makes the honest fix a datatype slot on the numeric variants of both
`EncodedTerm` and `ExpressionTerm`, rather than a re-routing of the match arms.

Neither enum has such a slot today, so this is a coordinated change across the
two crates, and it wants their conformance suites run against it. It is written
up here rather than attempted blind.

### Our workaround

`datatype_is_indistinguishable_in_store` in `src/shacl.rs` records a
`sh:datatype` constraint naming one of these as unevaluated rather than answering
it wrongly. `tests/datatype_preservation_test.rs` pins the exact affected set and
fails in both directions, so the workaround is removed when this is fixed.
