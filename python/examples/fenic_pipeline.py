"""fenic → Open Ontologies Lite: semantic DataFrames, governed.

fenic (https://github.com/typedef-ai/fenic) is typedef-ai's PySpark-inspired
DataFrame framework: its ``semantic.*`` operators (extract, classify, map,
join) turn unstructured text into typed rows with an LLM. This example shows
the handoff — fenic produces the rows, Open Ontologies turns them into RDF and
governs them (SHACL validation + lint), with no model calls on this side.

Run:  pip install open-ontologies-lite fenic && python fenic_pipeline.py

The same rows can instead flow through the full Rust engine: a fenic session's
local catalog is a plain DuckDB file (``<app_name>.duckdb``, tables under the
``typedef_default`` schema), so after ``df.write.save_as_table(...)``:

    open-ontologies sql-ingest "duckdb:///path/to/oo_fenic_demo.duckdb" \
        "SELECT id, name, parent FROM typedef_default.product_category"
    open-ontologies import-schema "duckdb:///path/to/oo_fenic_demo.duckdb"

See docs/data-pipeline.md ("Fenic" section) for that route.
"""

import sys

from open_ontologies_lite import OntologyEngine

try:
    import fenic as fc
except ImportError:
    sys.exit("fenic is not installed — pip install fenic")

BASE = "http://example.org/shop/"

# 1. A fenic pipeline. In real use the rows would come out of semantic
#    operators over unstructured text, e.g.:
#        df.select(fc.semantic.extract(fc.col("description"), ProductSchema))
#    Those need provider API keys; the DataFrame mechanics are identical, so
#    this example sticks to structured input.
session = fc.Session.get_or_create(fc.SessionConfig(app_name="oo_fenic_demo"))
df = session.create_dataframe(
    {
        "id": [1, 2, 3],
        "name": ["Bakery", "Dairy", "Sourdough Loaf"],
        "parent": [None, None, "Bakery"],
        "price": [None, None, 4.20],
    }
)
df = df.filter(fc.col("name").is_not_null())
df.write.save_as_table("product_category", mode="overwrite")  # → typedef_default.product_category

# 2. Hand the rows to Open Ontologies Lite. load_rows() accepts the fenic
#    DataFrame directly (duck-typed via to_pylist()).
engine = OntologyEngine()
triples = engine.load_rows(df, base_iri=BASE, class_iri=f"{BASE}Category", id_column="id")
print(f"loaded {triples} triples")

# 3. Govern: SHACL — every Category must have exactly one name.
shapes = f"""
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
<{BASE}CategoryShape> a sh:NodeShape ;
    sh:targetClass <{BASE}Category> ;
    sh:property [ sh:path <{BASE}name> ; sh:minCount 1 ; sh:maxCount 1 ] .
"""
try:
    from open_ontologies_lite.shacl import shacl_validate

    report = shacl_validate(engine.dump("turtle"), shapes)
    print("SHACL conforms:", report["conforms"])
except ImportError:
    print("pyshacl not installed — skipping SHACL step (pip install pyshacl)")

# 4. Ask questions back.
rows = engine.query(
    f"SELECT ?name WHERE {{ ?s <{BASE}parent> \"Bakery\" ; <{BASE}name> ?name }}"
)["rows"]
print("children of Bakery:", [r["name"] for r in rows])

session.stop()
