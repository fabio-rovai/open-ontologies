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


def _focus_nodes(data_graph, shapes_graph) -> tuple[int, list[dict]]:
    """Count the nodes each shape actually selected.

    A shapes graph whose targets match nothing validates every constraint against
    the empty set and reports conformance, which is indistinguishable from a run
    that examined the data and found it sound. Counting the selected nodes is
    what separates "checked and clean" from "checked nothing".

    Only the four declarative target predicates are counted. A shape with no
    declared target, or one using SPARQL-based `sh:target`, is not judged here:
    reporting it as unmatched would be a guess.
    """
    import rdflib

    sh = rdflib.Namespace(SH)
    rdfs = rdflib.RDFS

    total = 0
    unmatched: list[dict] = []

    for shape in set(shapes_graph.subjects(rdflib.RDF.type, sh.NodeShape)):
        selected: set = set()
        declared = False

        for target_class in shapes_graph.objects(shape, sh.targetClass):
            declared = True
            # SHACL selects SHACL-instances, so instances of subclasses count.
            selected.update(data_graph.subjects(rdflib.RDF.type, target_class))
            for sub in data_graph.transitive_subjects(rdfs.subClassOf, target_class):
                selected.update(data_graph.subjects(rdflib.RDF.type, sub))

        for node in shapes_graph.objects(shape, sh.targetNode):
            declared = True
            selected.add(node)

        for prop in shapes_graph.objects(shape, sh.targetSubjectsOf):
            declared = True
            selected.update(data_graph.subjects(prop, None))

        for prop in shapes_graph.objects(shape, sh.targetObjectsOf):
            declared = True
            selected.update(data_graph.objects(None, prop))

        if not declared:
            continue

        total += len(selected)
        if not selected:
            unmatched.append(
                {
                    "shape": str(shape),
                    "target_class": next(
                        (str(t) for t in shapes_graph.objects(shape, sh.targetClass)),
                        "",
                    ),
                }
            )

    unmatched.sort(key=lambda u: (u["target_class"], u["shape"]))
    return total, unmatched


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

    data_graph = rdflib.Graph().parse(data=data, format=data_format)
    shapes_graph = rdflib.Graph().parse(data=shapes, format=shapes_format)

    conforms, results_graph, results_text = pyshacl.validate(
        data_graph,
        shacl_graph=shapes_graph,
        inference=inference,
        advanced=True,
    )

    focus_nodes, unmatched_shapes = _focus_nodes(data_graph, shapes_graph)

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

    report = {
        "conforms": bool(conforms),
        "count": len(violations),
        "by_severity": by_severity,
        "violations": violations,
        "focus_nodes": focus_nodes,
        "unmatched_shapes": unmatched_shapes,
        "text": results_text,
    }

    # Every shape that declared a target selected nothing, so no constraint was
    # applied to anything. Reporting conformance here would be the same lie as
    # reporting it for a constraint that never ran.
    if focus_nodes == 0 and unmatched_shapes:
        report["conforms"] = None
        report["warning"] = (
            f"no focus node was selected: all {len(unmatched_shapes)} targeted shape(s) "
            "match nothing in the data, so conformance is undetermined. "
            "See unmatched_shapes."
        )

    return report
