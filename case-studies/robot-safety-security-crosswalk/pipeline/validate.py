#!/usr/bin/env python3
"""Validate the RSSC crosswalk graph against its SHACL shapes and exit non-zero on any violation.

Loads every .ttl in ontology/, crosswalk/, data/ and the case-study root as the
data graph, every .ttl in shapes/ as the shapes graph, runs pySHACL, and prints
a readable summary: the
number of evidenced assertions by class, the breakdown by evidence type and by
confidence, and every violation with its focus node, path, value, source shape
and message.

The script keeps one promise on top of whatever the shapes say. Before SHACL
runs it applies an independent contract check: every rssc:EvidencedAssertion
must carry rssc:citation, rssc:evidenceType and rssc:confidence, the latter two
as concept IRIs and not as bare literals. That check exists because a shapes
file that omits the contract would otherwise let an uncited mapping through
silently, and an uncited mapping is the one thing this artefact promises cannot
enter the graph. It duplicates the shapes deliberately; it does not replace
them, and both must pass.

Takes no arguments and reads fixed paths, as the house build pipelines do. Set
RSSC_ROOT only to run the self-test fixture in pipeline/selftest.

Exit codes: 0 all checks passed; 1 SHACL violations or contract failures;
2 nothing to validate (files or directories missing, or a parse error).
"""

import sys
from collections import Counter

import pyshacl
from rdflib import Literal, RDF, URIRef
from rdflib.namespace import SH

import rssc_graph as R

RULE = "=" * 78
THIN = "-" * 78


def banner(text):
    print()
    print(RULE)
    print(text)
    print(RULE)


def report_load(label, result):
    print(f"{label}:")
    if result.missing_dirs:
        for d in result.missing_dirs:
            print(f"  MISSING DIRECTORY  {d}")
    if result.empty_dirs:
        for d in result.empty_dirs:
            print(f"  NO .ttl FILES IN   {d}")
    for path, count, digest in result.files:
        print(f"  {path.relative_to(R.ROOT)}  {count} triples  sha256:{digest}")
    for path, err in result.errors:
        print(f"  PARSE FAILURE      {path}")
        print(f"                     {err}")
    print(f"  total: {len(result.files)} file(s), {result.triples} triples")


def collect_assertions(data):
    """Return {class: set(nodes)} for each assertion class, plus the union."""
    by_class = {}
    union = set()
    for cls in R.ASSERTION_CLASSES:
        members = R.instances_of(data, R.subclass_closure(data, cls))
        by_class[cls] = members
        union |= members
    # Anything typed directly as the abstract superclass, or as a subclass of
    # it that is not one of the four named kinds, must still be counted.
    all_assertion_classes = R.subclass_closure(data, R.EVIDENCED_ASSERTION)
    union |= R.instances_of(data, all_assertion_classes)
    return by_class, union


def print_distribution(title, counter, total, width=44):
    print(f"{title}:")
    if not counter:
        print("  (none)")
        return
    for key, count in sorted(counter.items(), key=lambda kv: (-kv[1], kv[0])):
        share = (100.0 * count / total) if total else 0.0
        bar = "#" * int(round(count * 30.0 / max(counter.values())))
        print(f"  {key:<{width}} {count:>5}  {share:5.1f}%  {bar}")


def contract_check(data, assertions):
    """Independent falsifiability check. Returns a list of failure strings."""
    failures = []
    for node in sorted(assertions, key=str):
        citations = list(data.objects(node, R.P_CITATION))
        evidence = list(data.objects(node, R.P_EVIDENCE_TYPE))
        confidence = list(data.objects(node, R.P_CONFIDENCE))
        label = R.short(node)
        if not citations:
            failures.append(f"{label}  has no rssc:citation")
        if not evidence:
            failures.append(f"{label}  has no rssc:evidenceType")
        if not confidence:
            failures.append(f"{label}  has no rssc:confidence")
        for value in evidence:
            if isinstance(value, Literal):
                failures.append(
                    f"{label}  rssc:evidenceType is a literal "
                    f'"{value}" and must be a concept IRI from scheme:evidence-type'
                )
        for value in confidence:
            if isinstance(value, Literal):
                failures.append(
                    f"{label}  rssc:confidence is a literal "
                    f'"{value}" and must be a concept IRI from scheme:confidence'
                )
    return failures


def undefined_terms(data):
    """rssc: terms used in the graph but never defined as subjects of a triple."""
    used = set()
    for s, p, o in data:
        for node in (s, p, o):
            if isinstance(node, URIRef) and str(node).startswith(R.RSSC):
                used.add(node)
    undefined = []
    for term in sorted(used, key=str):
        if (term, None, None) not in data:
            undefined.append(R.short(term))
    return undefined


def parse_violations(report_graph):
    """Extract sh:ValidationResult nodes into sortable dicts."""
    rows = []
    for result in report_graph.subjects(RDF.type, SH.ValidationResult):
        rows.append(
            {
                "severity": R.short(R.one(report_graph, result, SH.resultSeverity)),
                "focus": R.short(R.one(report_graph, result, SH.focusNode)),
                "path": R.short(R.one(report_graph, result, SH.resultPath)),
                "value": R.one(report_graph, result, SH.value),
                "shape": R.short(R.one(report_graph, result, SH.sourceShape)),
                "component": R.short(
                    R.one(report_graph, result, SH.sourceConstraintComponent)
                ),
                "message": " ".join(
                    str(m) for m in report_graph.objects(result, SH.resultMessage)
                ),
            }
        )
    rows.sort(key=lambda r: (r["severity"], r["focus"], r["path"], r["component"]))
    return rows


def main():
    banner("RSSC validate: SHACL and falsifiability contract")
    print(f"case study root: {R.ROOT}")

    data_load = R.load_data()
    shapes_load = R.load_shapes()

    print()
    report_load("Data graph (ontology/ + crosswalk/ + data/ + root)", data_load)
    print()
    report_load("Shapes graph (shapes/)", shapes_load)

    fatal = []
    if data_load.errors or shapes_load.errors:
        fatal.append("one or more Turtle files failed to parse")
    if not data_load.files:
        fatal.append("no Turtle files found in any data location, nothing to validate")
    if not shapes_load.files:
        fatal.append("no Turtle files found in shapes/, no shapes to validate against")
    if fatal:
        banner("RESULT: CANNOT VALIDATE")
        for line in fatal:
            print(f"  {line}")
        print()
        print("  Nothing was weakened and nothing was skipped. The pipeline")
        print("  reports the missing inputs rather than reporting a pass.")
        return 2

    data = data_load.graph
    shapes = shapes_load.graph

    # ---------------------------------------------------------------- inventory
    by_class, assertions = collect_assertions(data)

    banner("Assertion inventory")
    total = len(assertions)
    for cls in R.ASSERTION_CLASSES:
        print(f"  {R.short(cls):<32} {len(by_class[cls]):>5}")
    named = set()
    for cls in R.ASSERTION_CLASSES:
        named |= by_class[cls]
    other = assertions - named
    if other:
        print(f"  {'(other EvidencedAssertion)':<32} {len(other):>5}")
    print(f"  {'TOTAL evidenced assertions':<32} {total:>5}")

    print()
    for label, cls in (
        ("rssc:Standard", R.STANDARD),
        ("rssc:ClauseReference", R.CLAUSE_REFERENCE),
        ("rssc:SecurityRequirement", R.SECURITY_REQUIREMENT),
        ("rssc:FoundationalRequirement", R.FOUNDATIONAL_REQUIREMENT),
        ("rssc:SafetyFunction", R.SAFETY_FUNCTION),
        ("rssc:AutonomyBand", R.AUTONOMY_BAND),
        ("rssc:Source", R.SOURCE),
    ):
        count = len(R.instances_of(data, R.subclass_closure(data, cls)))
        print(f"  {label:<32} {count:>5}")

    # -------------------------------------------------- evidence and confidence
    banner("Evidence type and confidence")
    ev_counter = Counter()
    conf_counter = Counter()
    band_counter = Counter()
    for node in assertions:
        values = list(data.objects(node, R.P_EVIDENCE_TYPE))
        if not values:
            ev_counter["(MISSING)"] += 1
        for value in values:
            ev_counter[R.notation(data, value)] += 1
        values = list(data.objects(node, R.P_CONFIDENCE))
        if not values:
            conf_counter["(MISSING)"] += 1
        for value in values:
            conf_counter[R.notation(data, value)] += 1
        values = list(data.objects(node, R.P_APPLIES_TO_BAND))
        if not values:
            band_counter["(no band declared)"] += 1
        for value in values:
            band_counter[R.notation(data, value)] += 1

    print_distribution("By rssc:evidenceType", ev_counter, total)
    print()
    print_distribution("By rssc:confidence", conf_counter, total)
    print()
    print_distribution("By rssc:appliesToBand", band_counter, total)

    # ------------------------------------------------------- contract and terms
    banner("Independent falsifiability contract check")
    failures = contract_check(data, assertions)
    if failures:
        print(f"  {len(failures)} contract failure(s):")
        for line in failures:
            print(f"    {line}")
    elif total == 0:
        print("  NOT RUN. The graph contains no rssc:EvidencedAssertion, so the")
        print("  contract has nothing to hold. This is not a pass.")
    else:
        print(f"  PASS. All {total} evidenced assertion(s) carry a citation,")
        print("  an evidence type and a confidence level, all as concept IRIs.")

    undefined = undefined_terms(data)
    print()
    if undefined:
        print(f"  {len(undefined)} rssc: term(s) used but never defined:")
        for term in undefined:
            print(f"    {term}")
    else:
        print("  PASS. Every rssc: term used in the graph is defined in the graph.")

    # -------------------------------------------------------------------- SHACL
    banner("pySHACL validation")
    print(f"  pyshacl {pyshacl.__version__}, data {len(data)} triples, "
          f"shapes {len(shapes)} triples")
    conforms, report_graph, report_text = pyshacl.validate(
        data,
        shacl_graph=shapes,
        inference="rdfs",
        advanced=True,
        abort_on_first=False,
        meta_shacl=False,
    )
    violations = parse_violations(report_graph)
    severities = Counter(row["severity"] for row in violations)

    print(f"  conforms: {conforms}")
    if violations:
        breakdown = ", ".join(f"{k}={v}" for k, v in sorted(severities.items()))
        print(f"  results: {len(violations)}  ({breakdown})")
    else:
        print("  results: 0")

    if violations:
        print()
        for i, row in enumerate(violations, 1):
            print(THIN)
            print(f"  [{i}] {row['severity']}  {row['component']}")
            print(f"      focus node : {row['focus']}")
            print(f"      path       : {row['path']}")
            if row["value"] is not None:
                print(f"      value      : {R.short(row['value'])}")
            print(f"      source shape: {row['shape']}")
            print(f"      message    : {row['message']}")
        print(THIN)

    # ------------------------------------------------------------------- verdict
    hard_violations = [r for r in violations if r["severity"] != "sh:Warning"]
    bad = bool(failures) or bool(undefined) or bool(hard_violations) or not conforms

    # A conforming run over a graph with no assertions in it is not a pass. It
    # is a measurement of an empty store, which is the single easiest way to
    # ship an artefact that validates and proves nothing. The gate is here so
    # that state cannot be mistaken for a green build.
    vacuous = (total == 0) and not bad

    if bad:
        verdict, code = "FAIL", 1
    elif vacuous:
        verdict, code = "VACUOUS, NOT A PASS", 2
    else:
        verdict, code = "PASS", 0

    banner("RESULT: " + verdict)
    print(f"  evidenced assertions      : {total}")
    print(f"  contract failures         : {len(failures)}")
    print(f"  undefined rssc: terms     : {len(undefined)}")
    print(f"  SHACL conforms            : {conforms}")
    print(f"  SHACL results             : {len(violations)} "
          f"({len(hard_violations)} at severity above Warning)")
    print()
    if bad:
        print("  Reported as found. No shape was relaxed to obtain a pass.")
    if vacuous:
        print("  The shapes conform, but they conform over a graph containing no")
        print("  rssc:EvidencedAssertion. Every falsifiability constraint in")
        print("  shapes/ targets an assertion class, so with no assertions present")
        print("  each constraint is satisfied vacuously and nothing has been")
        print("  tested. Exit code 2, deliberately, so that a build script cannot")
        print("  read this as green. Load the crosswalk assertions and re-run.")
    return code


if __name__ == "__main__":
    sys.exit(main())
