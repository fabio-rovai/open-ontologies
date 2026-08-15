"""MCP server exposing the lightweight ontology engine as `onto_*` tools.

This is a thin transport layer. All real work lives in engine.py, which is pure
Python over the prebuilt pyoxigraph (Oxigraph) wheel. Run with:

    python -m open_ontologies_lite        # stdio MCP server
"""

from __future__ import annotations

from mcp.server.fastmcp import FastMCP

from . import __version__
from .engine import OntologyEngine

mcp = FastMCP(
    "open-ontologies-lite",
    instructions=(
        "Lightweight RDF/OWL ontology engine. Validate, load, query (SPARQL), "
        "diff, lint, convert, and persist ontologies in an in-memory Oxigraph store."
    ),
)

_engine = OntologyEngine()


@mcp.tool()
def onto_validate(data: str, format: str = "turtle") -> dict:
    """Parse RDF/OWL text and report syntax validity and triple count (does not load it)."""
    r = OntologyEngine.validate(data, format)
    return {"ok": r.ok, "triples": r.triples, "error": r.error}


@mcp.tool()
def onto_load(data: str, format: str = "turtle") -> dict:
    """Load RDF/OWL text into the in-memory store. Returns the total triple count."""
    return {"triples": _engine.load(data, format)}


@mcp.tool()
def onto_load_file(path: str, format: str | None = None) -> dict:
    """Load an RDF/OWL file from disk. Format is inferred from the extension if omitted."""
    return {"triples": _engine.load_path(path, format)}


@mcp.tool()
def onto_clear() -> dict:
    """Clear the in-memory store."""
    _engine.clear()
    return {"cleared": True}


@mcp.tool()
def onto_stats() -> dict:
    """Return triple, class, property, and individual counts for the loaded ontology."""
    return _engine.stats()


@mcp.tool()
def onto_query(sparql: str) -> dict:
    """Run a SPARQL query (SELECT / ASK / CONSTRUCT / DESCRIBE) against the store."""
    return _engine.query(sparql)


@mcp.tool()
def onto_save(path: str, format: str | None = None) -> dict:
    """Serialize the store to a file. Format is inferred from the extension if omitted."""
    return {"path": _engine.save(path, format)}


@mcp.tool()
def onto_convert(data: str, from_format: str, to_format: str) -> dict:
    """Convert RDF text between formats (turtle, ntriples, nquads, trig, rdfxml, n3, jsonld)."""
    return {"output": OntologyEngine.convert(data, from_format, to_format)}


@mcp.tool()
def onto_diff(data_a: str, data_b: str, format: str = "turtle") -> dict:
    """Triple-level diff between two ontologies (added/removed going A to B)."""
    return OntologyEngine.diff(data_a, data_b, format)


@mcp.tool()
def onto_lint() -> dict:
    """Structural hygiene checks on the loaded ontology: missing labels, domains, ranges."""
    return _engine.lint()


@mcp.tool()
def onto_kgcl_diff(data_a: str, data_b: str, format: str = "turtle") -> dict:
    """Classify the change from version A to B as KGCL change records.

    Returns structured changes (node_creation/deletion, node_rename,
    node_annotation_change, edge_creation/deletion), their counts, and a KGCL
    text rendering. Use for ontology version governance and change logs.
    """
    from .kgcl import kgcl_diff

    cs = kgcl_diff(data_a, data_b, format)
    return {"changes": cs.changes, "counts": cs.counts(), "kgcl": cs.to_kgcl()}


@mcp.tool()
def onto_shacl(
    shapes: str, shapes_format: str = "turtle", inference: str | None = None
) -> dict:
    """Validate the loaded graph against SHACL shapes.

    Returns conformance plus each violation's focus node, path, offending value,
    message, severity and the constraint component that produced it. `inference`
    ("rdfs", "owlrl", "both") is off by default, because materialising
    entailments changes what counts as a violation.

    Needs the optional extra: pip install "open-ontologies-lite[shacl]"
    """
    from .shacl import shacl_validate

    return shacl_validate(
        _engine.dump(), shapes, shapes_format=shapes_format, inference=inference
    )


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(
        prog="open-ontologies-lite",
        description="Lightweight RDF/OWL ontology MCP server (stdio transport).",
    )
    parser.add_argument(
        "--version", action="version", version=f"open-ontologies-lite {__version__}"
    )
    # No positional args: parsing only intercepts --help/--version (which exit
    # cleanly) instead of falling straight through into a blocking server start.
    parser.parse_args()
    mcp.run(transport="stdio")


if __name__ == "__main__":
    main()
