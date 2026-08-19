# Open Ontologies Lite (Python bridge)

A lightweight, pip-installable Python bridge to the same [Oxigraph](https://github.com/oxigraph/oxigraph) RDF/OWL engine that powers [Open Ontologies](https://github.com/fabio-rovai/open-ontologies). **No Rust toolchain, no compilation, no multi-gigabyte build directory** — `pyoxigraph` ships the engine as a prebuilt wheel, so everything here is pure-Python glue installed from PyPI.

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

`load_rows` duck-types against the common export methods — `to_pylist()`
(fenic DataFrame, pyarrow Table), `to_dicts()` (polars), `to_dict("records")`
(pandas) — or takes a plain list of dicts. Values become typed literals
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

This is the **Python layer** of the project. For the full engine (three-layer Dynamics/Causal/Planner architecture, HNSW semantic search, OWL2-DL tableaux reasoning, PDDL planning, governance, 100 tools), use the [Rust build](https://github.com/fabio-rovai/open-ontologies). Same Oxigraph core; pick the weight class you need.

## License

MIT
