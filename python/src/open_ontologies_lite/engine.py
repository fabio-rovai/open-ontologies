"""Lightweight ontology engine over a prebuilt Oxigraph store.

No compilation, no native build step: pyoxigraph ships the Oxigraph engine as a
prebuilt wheel, so everything here is pure Python glue around it. This module is
deliberately MCP-free so it can be unit-tested in isolation; server.py wires it
to the MCP transport.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import pyoxigraph as ox

# Map friendly format names to Oxigraph RdfFormat values.
_FORMATS = {
    "turtle": ox.RdfFormat.TURTLE,
    "ttl": ox.RdfFormat.TURTLE,
    "ntriples": ox.RdfFormat.N_TRIPLES,
    "nt": ox.RdfFormat.N_TRIPLES,
    "nquads": ox.RdfFormat.N_QUADS,
    "nq": ox.RdfFormat.N_QUADS,
    "trig": ox.RdfFormat.TRIG,
    "rdfxml": ox.RdfFormat.RDF_XML,
    "rdf": ox.RdfFormat.RDF_XML,
    "xml": ox.RdfFormat.RDF_XML,
    "n3": ox.RdfFormat.N3,
    "jsonld": ox.RdfFormat.JSON_LD,
}

OWL_CLASS = "http://www.w3.org/2002/07/owl#Class"
RDFS_CLASS = "http://www.w3.org/2000/01/rdf-schema#Class"
RDF_PROPERTY = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property"


def resolve_format(name: str) -> ox.RdfFormat:
    key = name.strip().lower().lstrip(".")
    if key not in _FORMATS:
        raise ValueError(
            f"unknown format {name!r}; supported: {', '.join(sorted(set(_FORMATS)))}"
        )
    return _FORMATS[key]


@dataclass
class ValidationResult:
    ok: bool
    triples: int
    error: str | None = None


class OntologyEngine:
    """A single in-memory ontology store with the core lifecycle operations."""

    def __init__(self) -> None:
        self.store = ox.Store()

    # ---- validation -------------------------------------------------------
    @staticmethod
    def validate(data: str, fmt: str = "turtle") -> ValidationResult:
        """Parse RDF without loading it; report syntax validity and triple count."""
        rdf_format = resolve_format(fmt)
        try:
            count = sum(1 for _ in ox.parse(data.encode("utf-8"), format=rdf_format))
        except (SyntaxError, ValueError) as exc:
            return ValidationResult(ok=False, triples=0, error=str(exc))
        return ValidationResult(ok=True, triples=count)

    # ---- load / clear -----------------------------------------------------
    def load(self, data: str, fmt: str = "turtle") -> int:
        """Load RDF text into the store; return total triple count afterwards."""
        self.store.load(data.encode("utf-8"), format=resolve_format(fmt))
        return len(self.store)

    def load_path(self, path: str, fmt: str | None = None) -> int:
        p = Path(path)
        rdf_format = resolve_format(fmt) if fmt else ox.RdfFormat.from_extension(p.suffix.lstrip("."))
        if rdf_format is None:
            raise ValueError(f"cannot infer format from {path!r}; pass fmt explicitly")
        self.store.load(path=str(p), format=rdf_format)
        return len(self.store)

    def load_rows(
        self,
        rows_or_df,
        base_iri: str = "http://example.org/data/",
        class_iri: str | None = None,
        id_column: str | None = None,
    ) -> int:
        """Load tabular rows (or any dataframe-like object — fenic, polars,
        pandas, pyarrow) into the store as RDF; return total triple count."""
        from .dataframe import rows_from_dataframe, rows_to_turtle

        rows = rows_from_dataframe(rows_or_df)
        ntriples = rows_to_turtle(rows, base_iri=base_iri, class_iri=class_iri, id_column=id_column)
        return self.load(ntriples, "ntriples")

    def clear(self) -> None:
        self.store.clear()

    # ---- stats ------------------------------------------------------------
    def stats(self) -> dict[str, int]:
        return {
            "triples": len(self.store),
            "classes": self._count(
                f"SELECT (COUNT(DISTINCT ?c) AS ?n) WHERE {{ ?c a ?t "
                f"VALUES ?t {{ <{OWL_CLASS}> <{RDFS_CLASS}> }} }}"
            ),
            "properties": self._count(
                f"SELECT (COUNT(DISTINCT ?p) AS ?n) WHERE {{ ?p a <{RDF_PROPERTY}> }}"
            ),
            "individuals": self._count(
                "SELECT (COUNT(DISTINCT ?i) AS ?n) WHERE { ?i a ?t "
                f"FILTER(?t != <{OWL_CLASS}> && ?t != <{RDFS_CLASS}> && ?t != <{RDF_PROPERTY}>) }}"
            ),
        }

    def _count(self, sparql: str) -> int:
        for sol in self.store.query(sparql):
            return int(sol["n"].value)
        return 0

    # ---- query ------------------------------------------------------------
    def query(self, sparql: str) -> dict:
        """Run SPARQL. Returns a typed dict for SELECT / ASK / CONSTRUCT/DESCRIBE."""
        result = self.store.query(sparql)
        if isinstance(result, ox.QueryBoolean):
            return {"type": "boolean", "boolean": bool(result)}
        if isinstance(result, ox.QuerySolutions):
            var_objs = list(result.variables)
            names = [str(v).lstrip("?") for v in var_objs]
            rows = []
            for sol in result:
                row = {}
                for name, var in zip(names, var_objs):
                    term = sol[var]
                    row[name] = term.value if term is not None else None
                rows.append(row)
            return {"type": "table", "variables": names, "rows": rows}
        # QueryTriples (CONSTRUCT / DESCRIBE)
        triples = [
            {"subject": t.subject.value, "predicate": t.predicate.value, "object": t.object.value}
            for t in result
        ]
        return {"type": "graph", "triples": triples}

    # ---- save / convert ---------------------------------------------------
    def save(self, path: str, fmt: str | None = None) -> str:
        p = Path(path)
        rdf_format = resolve_format(fmt) if fmt else ox.RdfFormat.from_extension(p.suffix.lstrip("."))
        if rdf_format is None:
            raise ValueError(f"cannot infer format from {path!r}; pass fmt explicitly")
        _dump(self.store, str(p), rdf_format)
        return str(p)

    def dump(self, fmt: str = "turtle") -> str:
        """Serialize the loaded store to a string (needed to hand it to pySHACL)."""
        out = _dump(self.store, None, resolve_format(fmt))
        return out.decode("utf-8") if isinstance(out, (bytes, bytearray)) else str(out)

    @staticmethod
    def convert(data: str, from_fmt: str, to_fmt: str) -> str:
        tmp = ox.Store()
        tmp.load(data.encode("utf-8"), format=resolve_format(from_fmt))
        out = _dump(tmp, None, resolve_format(to_fmt))
        return out.decode("utf-8") if isinstance(out, (bytes, bytearray)) else str(out)

    # ---- diff -------------------------------------------------------------
    @staticmethod
    def diff(data_a: str, data_b: str, fmt: str = "turtle") -> dict:
        """Triple-level diff between two ontologies (added/removed vs A→B)."""
        rdf_format = resolve_format(fmt)
        a = {_triple_key(t) for t in ox.parse(data_a.encode("utf-8"), format=rdf_format)}
        b = {_triple_key(t) for t in ox.parse(data_b.encode("utf-8"), format=rdf_format)}
        return {
            "added": sorted(b - a),
            "removed": sorted(a - b),
            "added_count": len(b - a),
            "removed_count": len(a - b),
            "unchanged_count": len(a & b),
        }

    # ---- lint -------------------------------------------------------------
    def lint(self) -> dict:
        """Cheap structural hygiene checks: missing labels, domains, ranges."""
        missing_labels = self._terms(
            f"SELECT DISTINCT ?c WHERE {{ ?c a ?t VALUES ?t {{ <{OWL_CLASS}> <{RDFS_CLASS}> }} "
            "FILTER NOT EXISTS {{ ?c <http://www.w3.org/2000/01/rdf-schema#label> ?l }} }"
        )
        missing_domain = self._terms(
            f"SELECT DISTINCT ?p WHERE {{ ?p a <{RDF_PROPERTY}> "
            "FILTER NOT EXISTS {{ ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?d }} }"
        )
        missing_range = self._terms(
            f"SELECT DISTINCT ?p WHERE {{ ?p a <{RDF_PROPERTY}> "
            "FILTER NOT EXISTS {{ ?p <http://www.w3.org/2000/01/rdf-schema#range> ?r }} }"
        )
        issues = (
            [{"rule": "missing-label", "term": t} for t in missing_labels]
            + [{"rule": "missing-domain", "term": t} for t in missing_domain]
            + [{"rule": "missing-range", "term": t} for t in missing_range]
        )
        return {"issue_count": len(issues), "issues": issues}

    def _terms(self, sparql: str) -> list[str]:
        result = self.store.query(sparql)
        var = result.variables[0]
        out = []
        for sol in result:
            val = sol[var]
            if val is not None:
                out.append(val.value)
        return out


def _triple_key(t) -> str:
    tr = getattr(t, "triple", t)
    return f"{tr.subject} {tr.predicate} {tr.object}"


def _dump(store: ox.Store, output, rdf_format: ox.RdfFormat):
    """Dump a store, selecting the default graph for triples-only formats.

    A Store holds quads, so dumping to a dataset-incapable format (N-Triples,
    Turtle, RDF/XML, N3) requires naming a single graph to flatten.
    """
    if not rdf_format.supports_datasets:
        return store.dump(output=output, format=rdf_format, from_graph=ox.DefaultGraph())
    return store.dump(output=output, format=rdf_format)
