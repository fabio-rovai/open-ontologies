"""SHACL validation over the loaded graph.

Conformance is the question SHACL answers: is this data shaped the way the
model says it must be. The package owns the validation call and the report
parsing; deciding what to do about a violation is the orchestrator's job, as
everywhere else here.

Requires the optional extra:  pip install "open-ontologies-lite[shacl]"

Reference: pySHACL, https://github.com/RDFLib/pySHACL
"""

from __future__ import annotations

SH = "http://www.w3.org/ns/shacl#"


def _require_pyshacl():
    try:
        import pyshacl  # noqa: F401

        return pyshacl
    except ImportError as exc:  # pragma: no cover - exercised by the extra being absent
        raise ImportError(
            "SHACL validation needs the optional extra: "
            'pip install "open-ontologies-lite[shacl]"'
        ) from exc


def _local(term: str) -> str:
    """Strip a SHACL namespace prefix so severities read as Violation/Warning/Info."""
    return term[len(SH) :] if term.startswith(SH) else term


def shacl_validate(
    data: str,
    shapes: str,
    *,
    data_format: str = "turtle",
    shapes_format: str = "turtle",
    inference: str | None = None,
) -> dict:
    """Validate `data` against `shapes` and return a structured report.

    `inference` is passed through to pySHACL ("rdfs", "owlrl", "both" or None).
    Inference is off by default: materialising entailments changes what counts
    as a violation, and that should be an explicit choice rather than a silent
    default.
    """
    pyshacl = _require_pyshacl()
    import rdflib

    conforms, results_graph, results_text = pyshacl.validate(
        data,
        shacl_graph=shapes,
        data_graph_format=data_format,
        shacl_graph_format=shapes_format,
        inference=inference,
        advanced=True,
    )

    sh = rdflib.Namespace(SH)
    violations: list[dict] = []
    for result in results_graph.subjects(rdflib.RDF.type, sh.ValidationResult):
        get = lambda p: results_graph.value(result, p)  # noqa: E731
        severity = get(sh.resultSeverity)
        violations.append(
            {
                "focus_node": str(get(sh.focusNode) or ""),
                "path": str(get(sh.resultPath) or ""),
                "value": str(get(sh.value) or ""),
                "message": str(get(sh.resultMessage) or ""),
                "severity": _local(str(severity)) if severity else "Violation",
                "source_shape": str(get(sh.sourceShape) or ""),
                "constraint": _local(str(get(sh.sourceConstraintComponent) or "")),
            }
        )

    violations.sort(key=lambda v: (v["focus_node"], v["path"], v["constraint"]))
    by_severity: dict[str, int] = {}
    for v in violations:
        by_severity[v["severity"]] = by_severity.get(v["severity"], 0) + 1

    return {
        "conforms": bool(conforms),
        "count": len(violations),
        "by_severity": by_severity,
        "violations": violations,
        "text": results_text,
    }
