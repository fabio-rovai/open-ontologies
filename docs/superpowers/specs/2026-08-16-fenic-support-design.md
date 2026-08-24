# Fenic Support — Design

**Date:** 2026-08-16
**Status:** Implemented in the same autonomous session; awaiting Fabio's review
**Scope:** Make [fenic](https://github.com/typedef-ai/fenic) (typedef-ai's semantic
DataFrame framework, PySpark-inspired, LLM operators over DuckDB/Polars/Arrow) a
supported data source for the Open Ontologies pipeline.

## Verified facts (fenic 0.12.0, probed live)

- fenic's local catalog is a **plain DuckDB file** named `<app_name>.duckdb` in
  the session's working directory. `df.write.save_as_table("t")` lands the table
  at `typedef_default.t`. Internal schemas: `__fenic_system` (tool/schema
  metadata), `fenic_system` (query metrics).
- DataFrame exports: `to_polars()`, `to_pandas()`, `to_arrow()`, `to_pydict()`,
  `to_pylist()`; `df.write.parquet/csv` for files.
- Python ≥3.10; DuckDB 1.1.3–1.4.x writes the catalog. Our bundled `duckdb`
  crate (1.4-line) reads those files.
- `sql-ingest "duckdb://…/app.duckdb" "SELECT … FROM typedef_default.t"`
  **already works** (verified: 2 rows → 7 triples).
- `import-schema` against a fenic catalog returns **0 tables**: the DuckDB
  introspector hardcodes `table_schema = 'main'`.

## Design

Three lanes, smallest change that makes each true:

### 1. Rust engine — fix `introspect_duckdb` to scan user schemas

`SchemaIntrospector::introspect_duckdb` iterates `(table_schema, table_name)`
pairs across all schemas except system ones (`information_schema`,
`pg_catalog`), engine-internal ones (any schema starting with `__`), and
fenic's telemetry schema (`fenic_system`). Column/PK/FK queries are
parameterized by schema instead of hardcoding `'main'`. `TableInfo.name` stays
the bare table name (feeds `table_to_class`); if the same bare name appears in
two schemas, later occurrences are disambiguated as `<schema>_<table>`.
No signature changes; Postgres path untouched.

### 2. Python lite — duck-typed dataframe bridge

New `open_ontologies_lite/dataframe.py`:

- `rows_from_dataframe(obj)` — accepts fenic DataFrame / pyarrow Table
  (`to_pylist`), polars (`to_dicts`), pandas (`to_dict("records")`), or an
  iterable of dicts. No hard dependency on any of them.
- `rows_to_turtle(rows, base_iri, class_iri, id_column=None)` — deterministic
  row→RDF: int→`xsd:integer`, float→`xsd:double`, bool→`xsd:boolean`,
  str→plain literal, `None` skipped. Subject IRIs from `id_column` or row index.
- `OntologyEngine.load_rows(rows_or_df, ...)` — convenience: convert + load,
  returns triple count.

MCP-native: fenic does the LLM work (semantic.extract etc.); lite only
validates, loads, lints, SHACL-checks. No model calls in the bridge.

### 3. Docs + example

- `docs/data-pipeline.md`: "Fenic (semantic DataFrames)" section — the
  verified `sql-ingest` / `import-schema` commands against `<app_name>.duckdb`,
  the `typedef_default` qualification, the file-lock caveat (close the fenic
  session first), the Parquet handoff alternative, and the MCP composition note
  (fenic serves tables as MCP tools; connect both servers to one agent).
- `python/examples/fenic_pipeline.py`: runnable end-to-end — fenic session →
  DataFrame → `save_as_table` + `rows_from_dataframe` → lite load → SHACL
  validate → lint. Exits with a clear message if fenic is not installed; no
  API keys required (semantic operators shown as comments).

## Testing

- Rust: extend `tests/duckdb_test.rs` with a fenic-shaped catalog fixture
  (tables in `typedef_default`, decoy tables in `__fenic_system` +
  `fenic_system`) asserting user tables are found and internals excluded.
- Python: `python/tests/test_dataframe.py` — protocol fakes for each export
  method, turtle determinism, typed literals, `load_rows` round-trip via
  SPARQL. Fenic itself optional (`pytest.importorskip`).

## Out of scope

- No fenic dependency anywhere (not even optional extra).
- No RDF writer inside fenic (their side, not ours).
- No changes to `onto_sql_ingest` / mapping layer — already sufficient.
