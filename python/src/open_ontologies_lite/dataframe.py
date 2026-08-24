"""Dataframe bridge: turn tabular rows into RDF and load them.

Duck-typed against the export methods of the common dataframe stacks, so none
of them is a dependency:

- fenic ``DataFrame`` and pyarrow ``Table`` — ``to_pylist()``
- polars ``DataFrame`` — ``to_dicts()``
- pandas ``DataFrame`` — ``to_dict("records")``
- any iterable of dicts — used as-is

The primary consumer is fenic (typedef-ai): its semantic operators do the LLM
extraction, this bridge does none of it. Rows come in, deterministic typed
triples come out, and the existing engine surface (SHACL, lint, SPARQL) governs
the result. See ``python/examples/fenic_pipeline.py`` for the end-to-end shape.
"""

from __future__ import annotations

import re
from typing import Any, Iterable

XSD = "http://www.w3.org/2001/XMLSchema#"
RDF_TYPE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"

_LOCAL_SAFE = re.compile(r"[^A-Za-z0-9_.-]+")


def rows_from_dataframe(obj: Any) -> list[dict[str, Any]]:
    """Extract rows as a list of dicts from any dataframe-like object."""
    if hasattr(obj, "to_pylist"):
        return list(obj.to_pylist())
    if hasattr(obj, "to_dicts"):
        return list(obj.to_dicts())
    if hasattr(obj, "to_dict"):
        return list(obj.to_dict("records"))
    if isinstance(obj, Iterable):
        rows = list(obj)
        if all(isinstance(r, dict) for r in rows):
            return rows
    raise TypeError(
        f"cannot extract rows from {type(obj).__name__}: expected an object "
        "with to_pylist()/to_dicts()/to_dict('records') or an iterable of dicts"
    )


def _local_name(value: Any) -> str:
    return _LOCAL_SAFE.sub("_", str(value))


def _literal(value: Any) -> str | None:
    if value is None:
        return None
    if isinstance(value, bool):
        return f'"{str(value).lower()}"^^<{XSD}boolean>'
    if isinstance(value, int):
        return f'"{value}"^^<{XSD}integer>'
    if isinstance(value, float):
        return f'"{value!r}"^^<{XSD}double>'
    escaped = str(value).replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
    return f'"{escaped}"'


def rows_to_turtle(
    rows: list[dict[str, Any]],
    base_iri: str = "http://example.org/data/",
    class_iri: str | None = None,
    id_column: str | None = None,
) -> str:
    """Serialize rows to Turtle, one subject per row.

    Subjects are ``<base_iri><id>`` where ``id`` comes from ``id_column`` when
    given (rows missing it fall back to their index), else the row index.
    Predicates are ``<base_iri><column>``. Values map to typed literals
    (bool → xsd:boolean, int → xsd:integer, float → xsd:double, anything else
    → plain string literal); ``None`` values are skipped. Deterministic:
    same rows in, same document out.
    """
    lines: list[str] = []
    for i, row in enumerate(rows):
        key = row.get(id_column, i) if id_column else i
        subject = f"<{base_iri}{_local_name(key)}>"
        if class_iri:
            lines.append(f"{subject} <{RDF_TYPE}> <{class_iri}> .")
        for column, value in row.items():
            lit = _literal(value)
            if lit is not None:
                lines.append(f"{subject} <{base_iri}{_local_name(column)}> {lit} .")
    return "\n".join(lines) + ("\n" if lines else "")
