# Data Pipeline

Take any structured data — CSV, JSON, Parquet, XLSX, XML, YAML, **or rows
returned by a SQL query against PostgreSQL or DuckDB** — and terraform it into
a validated, reasoned knowledge graph.

```mermaid
flowchart LR
    File["File: CSV / JSON / XLSX / Parquet / ..."]
    SQL["SQL backbone: Postgres / DuckDB"]
    Map["onto_map — generate mapping"]
    Ingest["onto_ingest / onto_sql_ingest — parse to RDF"]
    Validate["onto_shacl — check constraints"]
    Reason["onto_reason — infer new facts"]
    Query["onto_query — ask questions"]

    File --> Map
    SQL  --> Map
    Map --> Ingest
    Ingest --> Validate
    Validate -->|violations| Map
    Validate -->|ok| Reason
    Reason --> Query
```

| Manual process                                   | Open Ontologies equivalent                          |
| ------------------------------------------------ | --------------------------------------------------- |
| Domain expert defines classes by hand            | `import-schema` or Claude generates OWL             |
| Analyst maps spreadsheet columns to ontology     | `onto_map` auto-generates mapping config            |
| Data engineer writes ETL to RDF                  | `onto_ingest` parses CSV/JSON/Parquet/XLSX → RDF    |
| Data engineer writes ETL from a database         | `onto_sql_ingest` runs a SQL query → RDF            |
| Ontologist validates data constraints            | `onto_shacl` checks cardinality, datatypes, classes |
| Reasoner classifies instances (Protege + HermiT) | `onto_reason` runs native OWL reasoning             |
| Quality reviewer checks consistency              | `onto_enforce` + `onto_lint` + `onto_monitor`       |

## Two ways to feed the pipeline

The pipeline accepts data from **two sources** that converge on the same
mapping/ingest/SHACL/reason loop:

1. **File-based** — a single file in any of the supported formats below.
   Use `onto_ingest` (MCP) or `ingest` (CLI).
2. **SQL-based** — a SQL `SELECT` against a relational backbone (PostgreSQL
   or DuckDB). Use `onto_sql_ingest` (MCP) or `sql-ingest` (CLI). Rows
   returned by the query become "virtual records" that flow through the
   exact same mapping config.

In both cases the *same* `MappingConfig` is used, so a mapping written for a
CSV can be re-used verbatim against the SQL row stream, and vice versa.

## Supported file formats

| Format  | Extension   |
| ------- | ----------- |
| CSV     | `.csv`      |
| JSON    | `.json`     |
| NDJSON  | `.ndjson`   |
| XML     | `.xml`      |
| YAML    | `.yaml`     |
| Excel   | `.xlsx`     |
| Parquet | `.parquet`  |

## Supported SQL backbones

`onto_sql_ingest` and `onto_import_schema` accept a connection string and
auto-dispatch on its scheme:

| Connection string                          | Driver       | Cargo feature              |
| ------------------------------------------ | ------------ | -------------------------- |
| `postgres://user:pass@host/db`             | PostgreSQL   | `--features postgres`      |
| `postgresql://user:pass@host/db`           | PostgreSQL   | `--features postgres`      |
| `duckdb:///absolute/path/to/file.duckdb`   | DuckDB       | `--features duckdb`        |
| `duckdb:./relative/path.ddb`               | DuckDB       | `--features duckdb`        |
| `:memory:`                                 | DuckDB       | `--features duckdb`        |
| `/abs/file.duckdb` *(bare path)*           | DuckDB       | `--features duckdb`        |

Use the umbrella `--features sql` to enable both at once. By default the
binary is built with **no** SQL features so the dependency footprint stays
small; consumers opt in only to what they need.

> **Why DuckDB?** DuckDB is *not* used as a SPARQL parser — that role is
> already filled by Oxigraph. DuckDB is wired in as a **data integration
> backbone**: a single embedded process that can `SELECT` over CSV, Parquet,
> JSON, S3/HTTPFS, PostgreSQL, SQLite, Iceberg, and Delta tables through its
> extension ecosystem. One SQL query becomes one RDF feed, and the rest of
> the pipeline (`onto_shacl`, `onto_reason`, `onto_query`) is unchanged.

## Mapping config

The mapping bridges tabular data and RDF. The exact same JSON config is
accepted by `onto_ingest` (file rows) and `onto_sql_ingest` (SQL rows).

```json
{
  "base_iri": "http://www.co-ode.org/ontologies/pizza/pizza.owl#",
  "id_field": "name",
  "class": "http://www.co-ode.org/ontologies/pizza/pizza.owl#NamedPizza",
  "mappings": [
    { "field": "base",     "predicate": "pizza:hasBase",    "lookup": true },
    { "field": "topping1", "predicate": "pizza:hasTopping", "lookup": true },
    { "field": "price",    "predicate": "pizza:hasPrice",   "datatype": "xsd:decimal" }
  ]
}
```

- **`lookup: true`** — IRI reference (links to another entity).
- **`datatype`** — typed literal (decimal, integer, date, …).
- **Neither** — plain string literal.
- **`id_field`** — column whose value is appended to `base_iri` to mint the
  subject IRI.

When a mapping is omitted, `onto_ingest` / `onto_sql_ingest` auto-generate one
from the column names so you can iterate fast and refine later.

## SQL type → XSD datatype contract (v1)

The `datatype` field documented just above is **declarative**: you write it, and
`onto_ingest` / `onto_sql_ingest` honour it. This section is about the other
mechanism — the **automatic** one. When `onto_import_schema` reads a live
Postgres or DuckDB schema, nothing declares the datatypes; `SchemaIntrospector::sql_to_xsd`
(`src/schema.rs`) infers them from the SQL type names. The two are easy to
conflate and behave differently: a declared `datatype` is never overridden by
this table, and this table never applies to a mapping you wrote by hand.

Treat what follows as a contract. It is versioned, and any change to a row gets
a CHANGELOG entry, because a change here silently alters the shape of every
ontology generated downstream.

### The table

| SQL type (case-insensitive, parameters stripped) | XSD datatype |
|---|---|
| `integer` `int` `int4` `bigint` `int8` `smallint` `int2` `tinyint` `int1` `hugeint` `serial` `bigserial` `smallserial` `ubigint` `uinteger` `usmallint` `utinyint` | `xsd:integer` |
| `numeric` `decimal` | `xsd:decimal` |
| `real` `float4` | `xsd:float` |
| `double precision` `double` `float8` `float` | `xsd:double` |
| `boolean` `bool` | `xsd:boolean` |
| `date` | `xsd:date` |
| `timestamp` `timestamptz` `timestamp with time zone` `timestamp without time zone` `datetime` | `xsd:dateTime` |
| `time` `time with time zone` `time without time zone` | `xsd:time` |
| `bytea` `blob` | `xsd:hexBinary` |
| `uuid` | `xsd:string` |
| **anything else** | `xsd:string` |

### The decisions behind it

Each of these is defensible on its own and invisible without a document, which
is the reason this page exists.

**Parameters are stripped before matching.** `DECIMAL(18,2)` and `DECIMAL(38,10)`
both become a bare `xsd:decimal`; `VARCHAR(255)` becomes `xsd:string`. Precision
and scale are not represented in the range. If you need them enforced, express
them with SHACL rather than expecting the range to carry them.

**IEEE 754 floats are not `xsd:decimal`.** The value space of `xsd:decimal` is
integers over powers of ten, so it cannot represent `NaN`, `INF` or `-INF`, and
asserting it would claim an exactness the column does not have. `real` and
`float4` are `xsd:float`; `double precision`, `double` and `float8` are
`xsd:double`.

**Bare `float` widens to `xsd:double`.** It is genuinely dialect-dependent —
Postgres reads it as `float8`, DuckDB as an alias of `REAL`. Widening is the
safe direction: every `xsd:float` value is exactly representable as an
`xsd:double`, so calling a DuckDB single a double overstates the range without
misrepresenting a value, whereas the reverse would declare a range narrower
than the data. `float(p)` normalises to this arm as well.

**Timezone information is not represented.** `timestamp with time zone` and
`timestamp without time zone` both map to `xsd:dateTime`. XSD does distinguish
them through the presence of an offset in the lexical form, but the range does
not record which flavour the column was.

**`uuid` is `xsd:string`, not `xsd:anyURI`.** A UUID is not a locator, and
`xsd:string` keeps round-tripping lossless.

**The catch-all is the one to watch.** Any type not listed above becomes
`xsd:string` silently — DuckDB `LIST`, `STRUCT` and `MAP`, Postgres `jsonb`,
arrays, `interval`, `inet`, enums, and domain types. A structured column gets
`rdfs:range xsd:string` with no warning.

The declarative `datatype` field above **cannot** correct this.
`SchemaIntrospector::generate_turtle` takes only the introspected tables and a
base IRI; it never reads a mapping, and `import-schema` accepts no mapping
argument. A mapping governs the literals ingest produces later, not the ranges
the schema import asserts — so setting one leaves the generated ontology
unchanged and can leave the two disagreeing. Two remedies that do work:

- **Cast before importing.** Import from a view that presents the column as a
  type in the table above. Effective for domain types, aliases and anything
  numeric-adjacent; it cannot rescue a `STRUCT`.
- **Amend the range after importing.** The generated ontology is loaded into
  the triple store, so a SPARQL `UPDATE` replacing the `rdfs:range` of the
  affected property is a supported edit. Pair it with SHACL when the real
  constraint is richer than a datatype.

### What else the schema import decides

The datatype is one of four things `onto_import_schema` derives, and the others
matter when reading the generated ontology:

- **Foreign key columns never reach this table.** They become an
  `owl:ObjectProperty` whose `rdfs:range` is the parent class, not a datatype
  property.
- **Primary key columns that are not also foreign keys** get
  `owl:FunctionalProperty` in addition to `owl:DatatypeProperty`. The foreign
  key branch is taken first, so a shared primary key — the 1:1 pattern where a
  table's PK is also an FK to its parent — yields an `owl:ObjectProperty` only,
  with no functional axiom.
- **`NOT NULL` columns** add an `owl:Restriction` subclass axiom with
  `owl:minCardinality 1`.
- **Property names** are `<table>_<column>`; class names are the table name
  split on `_` with each segment capitalised. That is PascalCase for
  snake_case identifiers, which is the assumed shape. It is only a split on
  underscores: a quoted identifier containing spaces or hyphens passes through
  unchanged, so `"order details"` yields the class `Order details` and the
  property `db:order details_qty` — neither of which is a valid Turtle
  prefixed name. Sanitise such identifiers, or import through a view that
  renames them.

### Versioning

**v1** — the table above, first documented against the tree that includes the
float-mapping correction (`real`/`float4` → `xsd:float`, `float8`/`double`/`float`
→ `xsd:double`, previously all `xsd:decimal`).

Any row that changes bumps this section and gets a CHANGELOG entry under
`### Changed`. A refactor of `sql_to_xsd` that alters output without one is a
breaking change to every downstream integrator at once, which is precisely what
this contract exists to prevent.

## SQL ingest in three modes

### 1. Pull from PostgreSQL

```bash
# Run a query over a live Postgres database
open-ontologies sql-ingest \
  postgres://demo:demo@localhost/shop \
  "SELECT id AS name, base, price FROM pizza WHERE active" \
  --mapping ./mapping.pizza.json \
  --base-iri http://example.org/data/
```

### 2. Pull from DuckDB (file-backed warehouse)

```bash
# Materialised analytical store — fast columnar reads, zero infra
open-ontologies sql-ingest \
  duckdb:///data/warehouse.duckdb \
  "SELECT customer_id AS name, region, lifetime_value FROM customer_summary" \
  --mapping ./mapping.customer.json
```

### 3. Federated SQL via DuckDB extensions (in-memory)

DuckDB's strength is *federation*: a single SQL query can read remote files,
object stores, and other databases. `onto_sql_ingest` simply runs the query —
the federation is handled inside DuckDB.

```bash
# Federate Parquet on S3, a CSV on HTTPS, and a Postgres scanner — all in one
# SQL query — then ingest the result.
open-ontologies sql-ingest :memory: "
INSTALL httpfs; LOAD httpfs;
INSTALL postgres_scanner; LOAD postgres_scanner;
ATTACH 'host=localhost dbname=shop user=demo' AS shop (TYPE postgres);

SELECT
    o.order_id              AS name,
    c.country_code          AS country,
    o.total                 AS price,
    p.category              AS category
FROM read_parquet('s3://datalake/orders/*.parquet') o
JOIN shop.customers c USING (customer_id)
JOIN read_csv_auto('https://example.com/products.csv') p USING (product_id)
WHERE o.created_at >= '2026-01-01'
" --mapping ./mapping.orders.json
```

The same query runs identically as MCP from Claude:

```jsonc
{
  "tool": "onto_sql_ingest",
  "arguments": {
    "connection": ":memory:",
    "sql": "INSTALL httpfs; LOAD httpfs; SELECT … FROM read_parquet('s3://…')",
    "mapping": "./mapping.orders.json",
    "base_iri": "http://example.org/data/"
  }
}
```

> 🛈 The `httpfs`, `postgres_scanner`, `iceberg`, `delta`, `aws`, `azure`, and
> `gcp` DuckDB extensions are loaded inside the SQL itself with `INSTALL …;
> LOAD …;` — they live entirely in DuckDB and require no special handling
> from Open Ontologies. Credentials are passed via DuckDB's standard
> `CREATE SECRET` / environment-variable mechanism, never written to RDF.

### 4. Pull from a fenic catalog (semantic DataFrames)

[fenic](https://github.com/typedef-ai/fenic) is typedef-ai's PySpark-inspired
DataFrame framework whose `semantic.*` operators (extract, classify, map, join)
turn unstructured text into typed rows with an LLM. Its local catalog is a
**plain DuckDB file**: a session with `app_name="my_app"` writes
`my_app.duckdb` into the working directory, and `df.write.save_as_table("t")`
lands the table at `typedef_default.t`. That makes every fenic pipeline output
directly ingestable — fenic does the LLM extraction, Open Ontologies does the
ontology governance.

```bash
# Import the fenic tables as an OWL ontology (fenic's internal
# __fenic_system / fenic_system schemas are excluded automatically)
open-ontologies import-schema duckdb:///path/to/my_app.duckdb

# Ingest rows from a fenic table — note the typedef_default schema qualifier
open-ontologies sql-ingest \
  duckdb:///path/to/my_app.duckdb \
  "SELECT id, name, parent FROM typedef_default.product_category"
```

Caveats and alternatives:

- **Close the fenic session first** (`session.stop()`): DuckDB files are
  single-writer, so ingest after the pipeline finishes — or have fenic
  `df.write.parquet("out.parquet")` and feed that to `onto_ingest` instead.
- **Python-side handoff:** `open-ontologies-lite` accepts a fenic DataFrame
  directly — `engine.load_rows(df, base_iri=…, class_iri=…, id_column=…)`
  (duck-typed via `to_pylist()`; polars, pandas, and pyarrow objects work
  too). See [python/examples/fenic_pipeline.py](../python/examples/fenic_pipeline.py).
- **MCP composition:** fenic can serve its tables as MCP tools
  (`fenic-serve`), and Open Ontologies is an MCP server — connect both to the
  same agent and it can query fenic tables and govern the resulting graph in
  one conversation.

## Schema → ontology in 3 commands

`onto_import_schema` introspects a database, generates OWL, and loads it into
the triple store. The same command works against PostgreSQL **and** DuckDB —
only the connection string changes.

The datatypes it infers are not ad hoc: see
[SQL type → XSD datatype contract](#sql-type--xsd-datatype-contract-v1) for the
full table, the catch-all behaviour, and what the import decides beyond the
datatype (foreign keys, primary keys, `NOT NULL`).

```bash
# Import a PostgreSQL schema as OWL (requires --features postgres)
open-ontologies import-schema postgres://demo:demo@localhost/shop

# Import a DuckDB schema as OWL (requires --features duckdb)
open-ontologies import-schema duckdb:///data/warehouse.duckdb

# In-memory DuckDB — useful for unit tests / one-off CSV → OWL conversion
open-ontologies import-schema :memory:
```

DuckDB's `CREATE TABLE … FROM read_csv(…)` lets you turn a directory of CSVs
into a relational schema and then into OWL with a single pipeline:

```bash
duckdb /tmp/staging.duckdb <<'SQL'
CREATE TABLE customers AS SELECT * FROM read_csv_auto('customers.csv');
CREATE TABLE orders    AS SELECT * FROM read_csv_auto('orders.csv');
ALTER TABLE orders ADD CONSTRAINT fk_cust FOREIGN KEY (customer_id) REFERENCES customers(id);
SQL

open-ontologies import-schema duckdb:///tmp/staging.duckdb
open-ontologies reason --profile owl-rl
open-ontologies query "SELECT ?c ?label WHERE { ?c a owl:Class ; rdfs:label ?label }"
```

## Putting it together: the end-to-end loop

```bash
# 1. Generate or import the ontology
open-ontologies import-schema duckdb:///data/warehouse.duckdb

# 2. Generate a starter mapping from sample data (file or SQL — both work)
open-ontologies map ./sample.csv --save mapping.json

# 3. Ingest from SQL using that mapping
open-ontologies sql-ingest duckdb:///data/warehouse.duckdb \
    "SELECT * FROM customer_summary" --mapping mapping.json

# 4. Validate, reason, query
open-ontologies shacl ./shapes.ttl
open-ontologies reason --profile rdfs
open-ontologies query "SELECT ?c WHERE { ?c a :Customer }"
```

Or, in MCP, use the convenience pipeline `onto_extend` for file-based data
(it composes ingest + SHACL + reason in one call). The SQL equivalent is to
call `onto_sql_ingest` followed by `onto_shacl` and `onto_reason`.

## Tool reference

| Tool                  | Purpose                                                                       |
| --------------------- | ----------------------------------------------------------------------------- |
| `onto_map`            | Inspect a data file, propose a mapping config                                 |
| `onto_ingest`         | Parse a file (CSV/JSON/NDJSON/XML/YAML/XLSX/Parquet) → RDF and load           |
| `onto_sql_ingest`     | Run SQL against Postgres or DuckDB → RDF and load (uses same mapping format)  |
| `onto_import_schema`  | Introspect Postgres or DuckDB → OWL classes/properties/cardinality            |
| `onto_shacl`          | Validate loaded data against SHACL shapes                                     |
| `onto_reason`         | Materialise inferred triples (rdfs / owl-rl)                                  |
| `onto_extend`         | File-based convenience: `onto_ingest` + `onto_shacl` + `onto_reason`          |

## Build matrix

| Build command                        | Postgres | DuckDB | Embeddings |
| ------------------------------------ | -------- | ------ | ---------- |
| `cargo build`                        | ✗        | ✗      | ✗          |
| `cargo build --features postgres`    | ✓        | ✗      | ✗          |
| `cargo build --features duckdb`      | ✗        | ✓      | ✗          |
| `cargo build --features sql`         | ✓        | ✓      | ✗          |
| `cargo build --features sql,embeddings` | ✓     | ✓      | ✓          |

The `duckdb` crate is vendored with the `bundled` feature flag — no system
DuckDB install is required, but the C++ source compile adds a few minutes to
a clean build. Subsequent incremental builds are fast.

## FAQ

**Why not let DuckDB run SPARQL too?** DuckDB does not natively understand
SPARQL or RDF, and Oxigraph already handles those. The split keeps each
engine doing what it is best at: DuckDB owns *tabular SQL over heterogeneous
sources*, Oxigraph owns *graph queries and reasoning*.

**Can I use DuckDB to read a remote Parquet file directly without saving it
locally?** Yes — `INSTALL httpfs; LOAD httpfs;` inside the SQL string passed
to `onto_sql_ingest`. The remote bytes never touch disk; rows flow into RDF
in one pass.

**Does `onto_sql_ingest` support transactions?** No — it is read-only. The
query may be a multi-statement script (e.g. `INSTALL httpfs; LOAD httpfs;
SELECT …;`) and only the *final* `SELECT`'s rows are ingested.

**How are NULLs handled?** They become empty strings in the row map. Use
`datatype` mappings or SHACL to enforce typing where it matters.

**Where do credentials go?** Connection strings are passed through verbatim
to the underlying driver. For DuckDB, prefer `CREATE SECRET` (DuckDB's
built-in secret manager) or environment variables (`AWS_…`, `AZURE_…`) over
inlining secrets into SQL. For Postgres, prefer `~/.pgpass` or the standard
`PGPASSWORD` env var.
