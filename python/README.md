# Open Ontologies Lite (Python bridge)

A lightweight, pip-installable Python bridge to the same [Oxigraph](https://github.com/oxigraph/oxigraph) RDF/OWL engine that powers [Open Ontologies](https://github.com/fabio-rovai/open-ontologies). **No Rust toolchain, no compilation, no multi-gigabyte build directory**. `pyoxigraph` ships the engine as a prebuilt wheel, so everything here is pure-Python glue installed from PyPI.

It exposes the core ontology lifecycle as both a Python library and an MCP server.

## Why this exists

The full Rust engine compiles a large dependency tree from source (5+ GB of build artifacts, heavy SSD churn). This bridge is the opposite trade: install in seconds, run anywhere Python runs, keep the Oxigraph SPARQL engine underneath. It covers the core surface (validate, load, query, diff, lint, convert, stats, save), not the full 100-tool engine.

## Install

```bash
pip install open-ontologies-lite        # one universal wheel, no compiler
```

## Use as a Python library

```python
from open_ontologies_lite import OntologyEngine

engine = OntologyEngine()
engine.load(open("ontology.ttl").read())          # load Turtle
print(engine.stats())                              # {'triples':..,'classes':..,..}

rows = engine.query(
    "SELECT ?c WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> }"
)
print([r["c"] for r in rows["rows"]])

print(engine.lint())                               # missing labels/domains/ranges
print(OntologyEngine.convert(ttl, "turtle", "ntriples"))
```

See [examples/python_usage.py](examples/python_usage.py) for a runnable end-to-end script.

### Version governance with KGCL

```python
from open_ontologies_lite import kgcl_diff

cs = kgcl_diff(open("v1.ttl").read(), open("v2.ttl").read())
print(cs.counts())     # {'node_creation': 1, 'node_rename': 1, ...}
print(cs.to_kgcl())    # KGCL change records, one per line
```

`kgcl_diff` classifies the change between two ontology versions into KGCL records
(node created/deleted, renamed, annotation changed, edge created/deleted). Pure
structural comparison, no model. Also exposed as the `onto_kgcl_diff` MCP tool.

### Dataframe ingestion (fenic, polars, pandas, pyarrow)

```python
engine.load_rows(df, base_iri="http://x.org/", class_iri="http://x.org/Thing", id_column="id")
```

`load_rows` duck-types against the common export methods (`to_pylist()`
(fenic DataFrame, pyarrow Table), `to_dicts()` (polars), `to_dict("records")`
(pandas)), or takes a plain list of dicts. Values become typed literals
(int/float/bool → XSD), `None` is skipped, and the output is deterministic.
The primary consumer is [fenic](https://github.com/typedef-ai/fenic): its
semantic operators do the LLM extraction, this bridge just loads and lets
SHACL/lint/SPARQL govern the result. See
[examples/fenic_pipeline.py](examples/fenic_pipeline.py) for the end-to-end
shape, and `docs/data-pipeline.md` for ingesting a fenic DuckDB catalog with
the full Rust engine.

### Alignment candidate generation with HNSW (optional `[align]` extra)

```bash
pip install "open-ontologies-lite[align]"
```

```python
from open_ontologies_lite import AlignmentIndex

idx = AlignmentIndex(dim=384)
idx.add("flw:PC-BAK", vec_bakery)       # vectors come from YOUR embedder
idx.add("FOODON:00001626", vec_foodon)
idx.build()
idx.query(vec_query, k=5)               # -> [Candidate(id, score), ...]
```

MCP-native by design: the package owns the HNSW index, **you supply the vectors**.
Lite never calls an embedding model; bring vectors from your orchestrator and let it
adjudicate the candidates.

## Use as an MCP server

```bash
open-ontologies-lite          # stdio MCP server
# or: python -m open_ontologies_lite
```

Register it with any MCP client (e.g. Claude):

```json
{
  "mcpServers": {
    "open-ontologies-lite": { "command": "open-ontologies-lite" }
  }
}
```

## Tools

| Tool | Purpose |
| --- | --- |
| `onto_validate` | Parse RDF/OWL and report syntax validity + triple count (no load) |
| `onto_load` / `onto_load_file` | Load RDF text or a file into the in-memory store |
| `onto_clear` | Reset the store |
| `onto_stats` | Triple / class / property / individual counts |
| `onto_query` | SPARQL SELECT / ASK / CONSTRUCT / DESCRIBE |
| `onto_save` | Serialize the store to a file |
| `onto_convert` | Convert between Turtle / N-Triples / N-Quads / TriG / RDF-XML / N3 / JSON-LD |
| `onto_diff` | Triple-level diff between two ontologies |
| `onto_kgcl_diff` | KGCL change records between two versions (governance / change logs) |
| `onto_lint` | Missing labels, domains, ranges |
| `onto_shacl` | SHACL conformance: violations with focus node, path, value, severity and constraint, plus `focus_nodes` and `unmatched_shapes` (needs the `[shacl]` extra) |
| `onto_vocab_check` | Closed-world check: which terms in the data are not declared in the loaded ontology |
| `onto_reason` | Forward-chain a Horn rule table over the store and write a certificate the Lean checker verifies |

### Certified reasoning

The reasoner here is pure Python over pyoxigraph and adds no dependency at all.
It is also **untrusted**, in exactly the way the Rust engine is untrusted: what
carries the warrant is not the engine but the certificate, and the certificate is
checked by a small verified checker written in core Lean 4, no Mathlib, whose
soundness is a machine-checked theorem with the axiom footprint pinned to
`[propext, Classical.choice, Quot.sound]`.

```python
from open_ontologies_lite import OntologyEngine

engine = OntologyEngine()
engine.load(open("ontology.ttl").read())
report = engine.reason_horn("out/cert")

report["derived_triples"]      # 859
report["check"]["verdict"]     # 'entailed'
report["check"]["theorem"]     # 'OOCert.entails_of_builtin_horn'
```

Three files land in `out/cert`: the rule table that ran, every triple the run
started from, and one line per derived triple carrying the rule index, the
binding and the premises. `oo-horn check` reads those three and pronounces.

**Two verdicts, and they never share a word.** A certificate over the built-in
table earns `entailed`: every conclusion is true in every model of the asserted
graph. A certificate over any other table, the built-ins plus one extra rule
included, earns `entailed_under_supplied_rules`: every conclusion is true in
every model that *also satisfies your rules*, which are assumed and never
checked. A rule reading "every supplier is compliant" produces certificates that
check green for ever. Only the Lean checker decides which of the two a run
earned; nothing in this package contains either word as an executable string,
and a test scans the abstract syntax tree to keep it that way.

**The checker is optional and external.** Build it with `cd lean && lake build`
in a checkout of the [Rust repository](https://github.com/fabio-rovai/open-ontologies),
or set `OO_HORN`. Without it the package still reasons and still writes the
certificate, and the report says so in its own words:

```python
report["check"]["status"]    # 'unchecked_no_checker'
report["check"]["checked"]   # False
report["check"]["verdict"]   # None
report["check"]["warning"]   # 'the certificate was written and NOT checked: ...
                             #  Nothing here has been proved; these are the
                             #  claims of an untrusted engine'
```

Nothing is materialised into the store. A conclusion drawn under a supplied rule
table holds only in models that satisfy that table, so writing it back in beside
the assertions is how one run's output becomes the next run's axiom. Only the
default graph is asserted, for the same reason.

Running this engine and the Rust one over a corpus and checking both certificates
gives a three-way differential. `tests/test_horn_differential.py` and
`tests/test_horn_three_way_differential.py` do exactly that, and report
disagreement rather than adjudicating it.

**How much the agreement is worth.** This engine and the Rust one are NOT
independent implementations. They run the same semi-naive forward-chaining
algorithm over the same rule table, this one was written with `src/reason.rs`
open, and its comments cite that file by line. Agreement between them is strong
evidence against a TRANSCRIPTION slip — an index off by one, a guard dropped, a
join in the wrong order — and close to no evidence against a SHARED MISREADING
of a W3C rule, which both would implement and agree on for ever. The independent
leg is the Lean checker, written from the W3C rules with a machine-checked
soundness theorem. `tools/horn_differential.py` prints that caveat next to its
agreement count on every run. Never quote the sweep as "two independent
reasoners agree".

### Closed-world checking

RDF is open-world, so a predicate nobody declared is unknown rather than wrong.
An extractor that invents `ex:hasProteinName` because it sounded plausible
produces RDF that parses, loads and satisfies SHACL without a murmur. Closing
that world is the only way to tell an invented term from a real one:

```python
from open_ontologies_lite import vocab_check

report = vocab_check(ontology_ttl, generated_data_ttl)
report["undeclared_terms"]   # ['http://example.org/onto#hasProteinName']
```

Instance IRIs are never policed, because individuals belong to the data rather
than the vocabulary, and the standard vocabularies are never policed either.
With no ontology loaded the check reports that nothing was checked and returns
`conforms: False`, never `True`: a green light from an empty vocabulary is the
failure this exists to prevent.

### Validation that never passes vacuously

A shapes graph whose targets match nothing validates every constraint against
the empty set and reports `conforms: True`, byte-identical to a run where every
constraint was checked and passed. `shacl_validate` reports how many focus nodes
were actually selected and names the shapes that selected none:

```python
report["focus_nodes"]        # 0
report["unmatched_shapes"]   # [{'shape': '...PersonShape', 'target_class': '...Person'}]
report["conforms"]           # None, because nothing was examined
```

## Relationship to the Rust engine

This is the **Python layer** of the project. For the full engine (three-layer Dynamics/Causal/Planner architecture, HNSW semantic search, SHIQ tableaux reasoning, PDDL planning, governance, 120 tools), use the [Rust build](https://github.com/fabio-rovai/open-ontologies). HNSW semantic search there is a compile-time feature, so build with `--features embeddings`; the published binaries do not carry it. Same Oxigraph core; pick the weight class you need.

## License

MIT
