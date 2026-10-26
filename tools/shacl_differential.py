#!/usr/bin/env python3
"""Differential oracle: open-ontologies SHACL against pyshacl, over real shapes.

A validator is trusted on evidence, not on its own test suite. This runs both
engines over the same (data, shapes) pairs and compares them in order of
severity:

  1. FALSE CLEAN   - we say conforms, pyshacl says not. The one failure mode the
                     validator must not have. Any of these is a stop-the-line bug.
  2. FALSE ALARM   - we say not conforms, pyshacl says conforms. Noise that makes
                     a gate untrustworthy in the other direction.
  3. PARTIAL       - the verdicts agree and the result sets do not.
  4. UNDETERMINED  - we return null because something was skipped. Honest, but a
                     gap: the shapes graph got no verdict. These are the work list.

Agreement on the verdict is not enough, so where both engines give a verdict the
result sets are compared on (focus node, result path, severity, source shape,
source constraint component). A blank-node source shape is keyed by the IRI of
the node shape holding it under sh:property plus its sh:path, the same key
tests/w3c_shacl_conformance_test.rs uses, because a blank label is not portable
between two parses, let alone two engines. sh:value is NOT in the key: on the
default path the engine does not emit it yet (#318), so including it would turn
every case with a violation into PARTIAL and hide every other difference behind
that one.

Exit codes, after the CRUX harness rule (aprender, contracts/crux-shacl-jena-v1):

  0  GREEN         every planted control classified as planted, and no FALSE_CLEAN
  1  RED           a FALSE_CLEAN, an AGREE_AGAINST_W3C, or a misclassified control
  2  NOT MEASURED  no pyshacl, the wrong pyshacl, no engine, nothing discovered,
                   fewer pairs run than discovered, or no pair yielding a
                   comparison. Never green: before #320 every one of these exited
                   0, so a run that compared nothing looked like a run that found
                   nothing.

Planted controls run on EVERY invocation, before the corpus, so a run is never
trusted on a comparator nobody has seen disagree. One must-agree pair goes
through both real engines and must come out AGREE. Copies of pyshacl's report on
it are then mutated (one result dropped; one result's severity, source shape and
component changed in turn) and each copy must come out PARTIAL or FALSE_CLEAN.

W3C arbitration. With --w3c, the cases are the vendored W3C SHACL suite and each
carries the Working Group's expected report. When the two engines disagree the
expected report says which one is wrong: we = W3C != pyshacl is ORACLE_WRONG
(GREEN, recorded), we = pyshacl != W3C is AGREE_AGAINST_W3C (RED, because two
engines agreeing on a wrong answer is the case a two-engine differential is
blind to). A FALSE_CLEAN the W3C report confirms, on a test the conformance
ratchet already records as failing, is KNOWN_FALSE_CLEAN and does not block
(see `arbitrate`). Entries expecting sht:Failure carry no report and are listed
as EXCLUDED. The Working Group's own `sh:value` is dropped from the key for the
same reason as above.

Why the CI job passes --w3c as well as --discover case-studies: measured on 24
Oct 2026, discovery over case-studies/ finds 4 pairs and all 4 come back
UNDETERMINED (two skip sh:xone / sh:not / sh:or, two match no focus node), so
on its own the corpus compares nothing, and before #320 that run exited 0 and
was the whole job.

Every run writes a receipt into the JSON: the pyshacl and rdflib versions, the
engine binary's sha256, and the sha256 of every input file, so a drift in the
oracle and a change in the engine are told apart by a diff.

Usage:
    python3 tools/shacl_differential.py CORPUS.json [--json out.json]
    python3 tools/shacl_differential.py --discover case-studies [--json out.json]
    python3 tools/shacl_differential.py --w3c tests/w3c-shacl [--json out.json]
    python3 tools/shacl_differential.py --discover case-studies --w3c tests/w3c-shacl

The sources combine; the exit rules apply to the whole run.

CORPUS.json is a list of {"name", "data": [ttl...], "shapes": ttl} objects.
--discover ROOT builds one from a tree laid out as */shacl/*.ttl or
*/shapes/*.ttl with data in */graph/*.ttl, */data/*.ttl or */examples/*.ttl.
--pyshacl-version V makes any other installed version NOT MEASURED.

Nothing is capped silently: every pair excluded for size or timeout is listed in
the report with the reason, and makes the run NOT MEASURED.
"""
import argparse
import hashlib
import json
import os
import pathlib
import platform
import subprocess
import sys
import tempfile

BIN = os.environ.get(
    "OPEN_ONTOLOGIES_BIN",
    str(pathlib.Path(__file__).resolve().parent.parent / "target" / "release" / "open-ontologies"),
)
# A pair larger than this is reported as skipped rather than run. Both engines
# parse the whole graph, and a multi-hundred-megabyte harvest turns a diff run
# into an overnight job for no extra signal.
MAX_BYTES = int(os.environ.get("SHACL_DIFF_MAX_BYTES", 40 * 1024 * 1024))
TIMEOUT = int(os.environ.get("SHACL_DIFF_TIMEOUT", 300))

GREEN, RED, NOT_MEASURED = 0, 1, 2
SH = "http://www.w3.org/ns/shacl#"
RDF_NS = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
W3C_BASE = "https://w3c.github.io/data-shapes/data-shapes-test-suite/tests/"
# Verdicts that are a comparison. UNDETERMINED and ERROR are not, and a run made
# only of them measured nothing.
MEASURED = {"AGREE", "PARTIAL", "FALSE_CLEAN", "KNOWN_FALSE_CLEAN", "FALSE_ALARM", "ORACLE_WRONG",
            "AGREE_AGAINST_W3C"}
BLOCKING = ("FALSE_CLEAN", "AGREE_AGAINST_W3C")

CONTROL_DATA = """\
@prefix ex: <http://example.org/oo-control#> .
ex:alice a ex:Person ; ex:age "forty" .
ex:bob a ex:Person ; ex:name "Bob" ; ex:age 40 .
"""
# Two results, chosen to exercise every widened key field: a BLANK property
# shape (keyed by parent + path), an IRI property shape, and a non-default
# severity.
CONTROL_SHAPES = """\
@prefix ex: <http://example.org/oo-control#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [ sh:path ex:name ; sh:minCount 1 ] ;
  sh:property ex:AgeShape .
ex:AgeShape a sh:PropertyShape ;
  sh:path ex:age ;
  sh:datatype xsd:integer ;
  sh:severity sh:Warning .
"""


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def oracle_versions():
    """pyshacl and rdflib versions, or None when pyshacl cannot be imported."""
    try:
        import pyshacl
        import rdflib
    except ImportError:
        return None
    return {"pyshacl": pyshacl.__version__, "rdflib": rdflib.__version__}


# ── keys ─────────────────────────────────────────────────────────────────────


def shape_key(parent, path):
    return f"{parent} sh:property [sh:path {path}]"


def severity_iri(s):
    """The engine reports the local name of sh:severity; read a bare name as
    being in the SHACL namespace, as tests/w3c_shacl_conformance_test.rs does."""
    if not s:
        return None
    return s if ":" in s else SH + s


def our_term(s):
    """The engine prints IRIs bare and literals in N-Triples form; blank nodes
    are not portable, so they collapse."""
    if s is None or s == "":
        return None
    return "_:" if s.startswith("_:") else s


def ours_keys(report):
    out = set()
    for v in report.get("violations", []) or []:
        # Every violation passes through `attribute()`, which mirrors `path`
        # into `result_path`, and a sh:sparql result populates only
        # `result_path`, so reading it first is what makes SPARQL-based
        # results comparable at all.
        path = our_term(v.get("result_path") or v.get("path"))
        shape = v.get("source_shape")
        if shape and shape.startswith("_:"):
            parent = v.get("node_shape")
            shape = shape_key(parent, path) if parent and not parent.startswith("_:") and path else "_:"
        out.add((
            our_term(v.get("focus_node")),
            path,
            severity_iri(v.get("severity")),
            our_term(shape),
            our_term(v.get("source_constraint_component")),
        ))
    return out


def rdf_term(t):
    from rdflib import BNode, URIRef

    if t is None:
        return None
    if isinstance(t, BNode):
        return "_:"
    if isinstance(t, URIRef):
        return str(t)
    return t.n3()


def path_form(graph, t, depth=0):
    """Structural rendering of a SHACL path, the same canonical form the Rust
    harness uses: bare IRI, ^IRI, (a/b), (a|b), (p)* (p)+ (p)?. A well-formed
    path is an acyclic tree, so equal renderings mean isomorphic paths."""
    from rdflib import BNode, Namespace
    from rdflib.namespace import RDF

    sh = Namespace(SH)
    if not isinstance(t, BNode):
        return rdf_term(t)
    if depth > 32:
        return "(path too deep)"
    sub = lambda x: path_form(graph, x, depth + 1)  # noqa: E731
    inv = graph.value(t, sh.inversePath)
    if inv is not None:
        return "^" + sub(inv)
    alt = graph.value(t, sh.alternativePath)
    if alt is not None:
        return "(" + "|".join(sub(x) for x in graph.items(alt)) + ")"
    for pred, op in (("zeroOrMorePath", "*"), ("oneOrMorePath", "+"), ("zeroOrOnePath", "?")):
        x = graph.value(t, sh[pred])
        if x is not None:
            return f"({sub(x)}){op}"
    if graph.value(t, RDF.first) is not None:
        return "(" + "/".join(sub(x) for x in graph.items(t)) + ")"
    return "_:"


def rdf_keys(results_graph, results, shapes_graph):
    """Keys for rdflib sh:result nodes. `shapes_graph` resolves a blank source
    shape to its parent; pyshacl reports the shapes graph's own blank node."""
    from rdflib import BNode, Namespace, URIRef

    sh = Namespace(SH)
    out = set()
    for r in results:
        p = results_graph.value(r, sh.resultPath)
        path = None
        if p is not None:
            path = path_form(results_graph if (p, None, None) in results_graph else shapes_graph, p)
        shape = results_graph.value(r, sh.sourceShape)
        if isinstance(shape, BNode):
            parents = list(shapes_graph.subjects(sh.property, shape))
            spath = shapes_graph.value(shape, sh.path)
            if len(parents) == 1 and isinstance(parents[0], URIRef) and spath is not None:
                shape_s = shape_key(str(parents[0]), path_form(shapes_graph, spath))
            else:
                shape_s = "_:"
        else:
            shape_s = rdf_term(shape)
        out.add((
            rdf_term(results_graph.value(r, sh.focusNode)),
            path,
            rdf_term(results_graph.value(r, sh.resultSeverity)),
            shape_s,
            rdf_term(results_graph.value(r, sh.sourceConstraintComponent)),
        ))
    return out


# ── engines ──────────────────────────────────────────────────────────────────


def run_open_ontologies(data_files, shapes_file):
    with tempfile.TemporaryDirectory() as d:
        env = dict(os.environ, OPEN_ONTOLOGIES_STORAGE_MODE="persistent")
        for f in data_files:
            r = subprocess.run(
                [BIN, "--data-dir", d, "load", str(f)],
                capture_output=True, text=True, env=env, timeout=TIMEOUT,
            )
            if r.returncode != 0:
                return {"error": f"load failed: {r.stderr.strip()[:300]}"}
        r = subprocess.run(
            [BIN, "--data-dir", d, "shacl", str(shapes_file)],
            capture_output=True, text=True, env=env, timeout=TIMEOUT,
        )
        try:
            report = json.loads(r.stdout)
        except json.JSONDecodeError:
            return {"error": f"unparseable report: {(r.stdout + r.stderr).strip()[:300]}"}
        report["keys"] = ours_keys(report)
        return report


def run_pyshacl(data_files, shapes_file, originals=None):
    """`originals` maps a rewritten N-Triples file back to (Turtle source,
    base). pyshacl is handed the source: measured on the W3C suite, the
    N-Triples rewrite drops the prefix bindings pyshacl resolves `sh:prefixes`
    through, and `sparql/node/prefixes-001` became a pyshacl ERROR that the
    original file does not cause."""
    from pyshacl import validate
    from rdflib import Graph, Namespace

    sh = Namespace(SH)
    originals = originals or {}

    def parse(graph, f):
        src, base = originals.get(str(f), (str(f), None))
        return graph.parse(src, format="turtle", publicID=base) if base else graph.parse(src)

    data = Graph()
    for f in data_files:
        parse(data, f)
    shapes = parse(Graph(), shapes_file)
    conforms, results_graph, _ = validate(data, shacl_graph=shapes)
    # Every severity, not just sh:Violation. Collecting only Violations here
    # compared pyshacl's violations against our results at ALL severities, and
    # reported the difference as our defect: four sh:Warning results in the
    # enterprise-knowledge vertical showed up as four extra violations we had
    # invented. An oracle that manufactures findings is worse than no oracle.
    results = set(results_graph.subjects(sh.resultSeverity, None))
    return {"conforms": conforms, "keys": rdf_keys(results_graph, results, shapes)}


# ── classification ───────────────────────────────────────────────────────────


def classify(ours, theirs):
    """Return (verdict, detail). Order matters: false clean is checked first."""
    if "error" in ours:
        return "ERROR", ours["error"]
    our_conforms = ours.get("conforms")
    their_conforms = theirs["conforms"]

    if our_conforms is None:
        skipped = ours.get("skipped_constraints") or []
        names = sorted({s.get("constraint", "?") for s in skipped})
        if names:
            return "UNDETERMINED", "skipped: " + ", ".join(names)
        return "UNDETERMINED", ours.get("warning", "no verdict")

    if our_conforms is True and their_conforms is False:
        return "FALSE_CLEAN", f"pyshacl found {len(theirs['keys'])} result(s), we found none"
    if our_conforms is False and their_conforms is True:
        return "FALSE_ALARM", f"we reported {len(ours['keys'])} result(s), pyshacl found none"

    missed, extra = theirs["keys"] - ours["keys"], ours["keys"] - theirs["keys"]
    if missed or extra:
        return "PARTIAL", f"{len(missed)} missed, {len(extra)} extra (verdict agrees)"
    return "AGREE", f"{len(ours['keys'])} result(s), identical sets"


def same(a, b):
    return a["conforms"] == b["conforms"] and a["keys"] == b["keys"]


def arbitrate(verdict, detail, ours, theirs, expected, baseline):
    """Let the W3C expected report decide a disagreement, or flag an agreement
    it contradicts. Only called where the engine gave a verdict.

    A FALSE_CLEAN the W3C report confirms, on a test the conformance ratchet
    (tests/w3c_shacl_baseline.json) does not record as PASS, is KNOWN_FALSE_CLEAN:
    reported on every run, and not blocking here, because the `w3c-shacl` job
    already fails if that FAIL count rises or any recorded PASS regresses.
    Measured 24 Oct 2026: 12 of the 113 comparable W3C tests, every one a
    `conforms: true` where the suite expects false, and every one already FAIL
    in that baseline. On a test the baseline records as PASS it stays FALSE_CLEAN
    and blocks, since that is a regression the ratchet has not seen."""
    if verdict == "AGREE":
        if not same(theirs, expected):
            return "AGREE_AGAINST_W3C", "both engines agree and the W3C expected report does not"
        return verdict, detail
    if same(ours, expected):
        return "ORACLE_WRONG", f"W3C sides with us against pyshacl ({verdict}: {detail})"
    side = "W3C sides with pyshacl" if same(theirs, expected) else "W3C matches neither engine"
    if verdict == "FALSE_CLEAN" and baseline not in (None, "PASS"):
        return "KNOWN_FALSE_CLEAN", f"{detail}; {side}; held by the w3c-shacl ratchet as {baseline}"
    return verdict, f"{detail}; {side}"


# ── planted controls ─────────────────────────────────────────────────────────


def run_controls(workdir):
    """Plant the controls and return (rows, failures). A failure is a control
    whose classification is not the one it was planted to have."""
    data = pathlib.Path(workdir) / "control-data.ttl"
    shapes = pathlib.Path(workdir) / "control-shapes.ttl"
    data.write_text(CONTROL_DATA)
    shapes.write_text(CONTROL_SHAPES)
    ours = run_open_ontologies([data], shapes)
    theirs = run_pyshacl([data], shapes)

    rows, failures = [], []

    def plant(name, planted_theirs, allowed):
        verdict, detail = classify(ours, planted_theirs)
        ok = verdict in allowed
        rows.append({"control": name, "verdict": verdict, "expected": sorted(allowed),
                     "detail": detail, "ok": ok})
        if not ok:
            failures.append(f"control `{name}` came out {verdict}, planted to be "
                            f"{' or '.join(sorted(allowed))}: {detail}")

    plant("must-agree", theirs, {"AGREE"})
    keys = sorted(theirs["keys"], key=lambda k: tuple(x or "" for x in k))
    if len(keys) < 2:
        # Without two results the mutations below cannot be planted at all, and
        # a control that is not planted is not a control.
        failures.append(f"control pair yielded {len(keys)} pyshacl result(s), need 2 to plant mutations")
        return rows, failures
    victim = keys[0]
    plant("one-result-dropped", {**theirs, "keys": set(keys[1:])}, {"PARTIAL", "FALSE_CLEAN"})
    for field, i in (("severity", 2), ("source-shape", 3), ("component", 4)):
        changed = list(victim)
        changed[i] = f"{changed[i]}-planted"
        plant(f"{field}-changed", {**theirs, "keys": set(keys[1:]) | {tuple(changed)}}, {"PARTIAL"})
    return rows, failures


# ── discovery ────────────────────────────────────────────────────────────────


def discover(root):
    cases, root = [], pathlib.Path(root)
    if not root.is_dir():
        return cases
    for repo in sorted(p for p in root.iterdir() if p.is_dir()):
        shapes = sorted(list((repo / "shacl").glob("*.ttl")) + list((repo / "shapes").glob("*.ttl")))
        if not shapes:
            continue
        data = []
        for sub in ("graph", "data", "examples", "ontology"):
            data += sorted((repo / sub).glob("*.ttl"))
        if not data:
            continue
        for s in shapes:
            cases.append({"name": f"{repo.name}/{s.parent.name}/{s.name}",
                          "data": [str(p) for p in data], "shapes": str(s)})
    return cases


def discover_w3c(suite, workdir):
    """Walk the vendored W3C suite from its root manifest, the walk
    tests/w3c_shacl_conformance_test.rs does. Each case's graphs are re-written
    as N-Triples with the suite's published base, because every file uses
    relative IRIs and `open-ontologies shacl` takes no base IRI. Entries whose
    mf:result is sht:Failure carry no report to compare and are returned as
    excluded, by name."""
    from rdflib import Graph, Namespace
    from rdflib.namespace import RDF

    mf = Namespace("http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#")
    sht = Namespace("http://www.w3.org/ns/shacl-test#")
    sh = Namespace(SH)
    root = os.path.realpath(suite)
    if not os.path.isfile(os.path.join(root, "manifest.ttl")):
        return [], []

    def base_of(p):
        return W3C_BASE + os.path.relpath(p, root)

    def path_of(iri):
        return os.path.join(root, str(iri)[len(W3C_BASE):])

    def load(p):
        return Graph().parse(p, format="turtle", publicID=base_of(p))

    # The conformance ratchet's per-test outcomes. A W3C test it does not record
    # as PASS is an engine defect that ratchet already holds (its FAIL count may
    # not rise), so a FALSE_CLEAN there is KNOWN rather than new; see arbitrate.
    baseline_file = pathlib.Path(root).parent / "w3c_shacl_baseline.json"
    baseline = json.loads(baseline_file.read_text())["outcomes"] if baseline_file.is_file() else {}

    cases, excluded, seen = [], [], set()
    out = pathlib.Path(workdir) / "w3c"

    def absolute(p):
        dest = out / (os.path.relpath(p, root) + ".nt")
        if not dest.exists():
            dest.parent.mkdir(parents=True, exist_ok=True)
            load(p).serialize(destination=str(dest), format="nt")
        return str(dest)

    def walk(path):
        path = os.path.realpath(path)
        if path in seen:
            return
        seen.add(path)
        g = load(path)
        name = os.path.relpath(path, root)[: -len(".ttl")]
        for manifest in g.subjects(RDF.type, mf.Manifest):
            for entries in g.objects(manifest, mf.entries):
                for entry in g.items(entries):
                    if (entry, RDF.type, sht.Validate) not in g:
                        continue
                    result = g.value(entry, mf.result)
                    if result == sht.Failure:
                        excluded.append({"name": "w3c/" + name, "reason": "expects sht:Failure, not a report"})
                        continue
                    action = g.value(entry, mf.action)
                    data = path_of(g.value(action, sht.dataGraph))
                    shapes = path_of(g.value(action, sht.shapesGraph))
                    expected = {
                        "conforms": bool(g.value(result, sh.conforms).toPython()),
                        "keys": rdf_keys(g, set(g.objects(result, sh.result)), g),
                    }
                    nt_data, nt_shapes = absolute(data), absolute(shapes)
                    cases.append({"name": "w3c/" + name, "data": [nt_data], "shapes": nt_shapes,
                                  "originals": {nt_data: (data, base_of(data)),
                                                nt_shapes: (shapes, base_of(shapes))},
                                  "inputs": sorted({path, data, shapes}), "expected": expected,
                                  "baseline": baseline.get(name)})
        for inc in sorted(g.objects(None, mf.include)):
            walk(path_of(inc))

    walk(os.path.join(root, "manifest.ttl"))
    cases.sort(key=lambda c: c["name"])
    return cases, excluded


# ── main ─────────────────────────────────────────────────────────────────────


def not_measured(why, report, json_path):
    print(f"\nNOT MEASURED: {why}\nThis is not agreement between the engines; it is the absence "
          "of a comparison, and it exits 2.", flush=True)
    report.update({"status": "NOT_MEASURED", "exit": NOT_MEASURED, "why": why})
    if json_path:
        with open(json_path, "w") as fh:
            json.dump(report, fh, indent=1, default=sorted)
    return NOT_MEASURED


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("corpus", nargs="?")
    ap.add_argument("--discover")
    ap.add_argument("--w3c")
    ap.add_argument("--json")
    ap.add_argument("--limit", type=int)
    ap.add_argument("--pyshacl-version")
    args = ap.parse_args(argv)

    report = {"receipt": {"python": platform.python_version(), "engine": {"path": BIN}}}
    versions = oracle_versions()
    if versions is None:
        return not_measured("pyshacl is not importable; there is no oracle", report, args.json)
    report["receipt"].update(versions)
    print(f"oracle: pyshacl {versions['pyshacl']}, rdflib {versions['rdflib']}", flush=True)
    if args.pyshacl_version and versions["pyshacl"] != args.pyshacl_version:
        return not_measured(f"pyshacl {versions['pyshacl']} is installed and {args.pyshacl_version} "
                            "is pinned; a different oracle is a different measurement", report, args.json)
    if not os.path.isfile(BIN):
        return not_measured(f"no engine binary at {BIN}", report, args.json)
    report["receipt"]["engine"]["sha256"] = sha256(BIN)

    with tempfile.TemporaryDirectory() as workdir:
        if not (args.w3c or args.discover or args.corpus):
            ap.error("give a CORPUS.json, --discover ROOT and/or --w3c SUITE")
        cases, excluded = [], []
        if args.corpus:
            cases += json.load(open(args.corpus))
        if args.discover:
            cases += discover(args.discover)
        if args.w3c:
            w3c_cases, excluded = discover_w3c(args.w3c, workdir)
            cases += w3c_cases
        if args.limit:
            cases = cases[: args.limit]
        report["excluded"] = excluded
        for e in excluded:
            print(f"{'EXCLUDED':<17} {e['name']}\n                  {e['reason']}", flush=True)

        inputs = set()
        for case in cases:
            inputs.update(case.get("inputs") or (list(case["data"]) + [case["shapes"]]))
        report["receipt"]["inputs"] = {f: sha256(f) for f in sorted(inputs) if os.path.isfile(f)}
        report["discovered"] = len(cases)

        controls, control_failures = run_controls(workdir)
        report["controls"] = controls
        for c in controls:
            print(f"CONTROL {'ok ' if c['ok'] else 'BAD'} {c['control']:<22} {c['verdict']}", flush=True)

        if not cases:
            return not_measured("discovery found zero (data, shapes) pairs", report, args.json)

        rows, skipped_pairs = [], []
        for case in cases:
            data = [f for f in case["data"] if os.path.exists(f)]
            size = sum(os.path.getsize(f) for f in data) + os.path.getsize(case["shapes"])
            if size > MAX_BYTES:
                reason = f"{size/1e6:.0f} MB over the {MAX_BYTES/1e6:.0f} MB cap"
                skipped_pairs.append({"name": case["name"], "reason": reason})
                print(f"{'NOT_RUN':<17} {case['name']}\n                  {reason}", flush=True)
                continue
            try:
                ours = run_open_ontologies(data, case["shapes"])
                theirs = run_pyshacl(data, case["shapes"], case.get("originals"))
                verdict, detail = classify(ours, theirs)
                if "expected" in case and verdict not in ("ERROR", "UNDETERMINED"):
                    verdict, detail = arbitrate(verdict, detail, ours, theirs, case["expected"],
                                                case.get("baseline"))
            except subprocess.TimeoutExpired:
                reason = f"timed out after {TIMEOUT}s"
                skipped_pairs.append({"name": case["name"], "reason": reason})
                print(f"{'NOT_RUN':<17} {case['name']}\n                  {reason}", flush=True)
                continue
            except Exception as exc:  # a parse failure in either engine is a result, not a crash
                verdict, detail = "ERROR", f"{type(exc).__name__}: {exc}"[:300]
            rows.append({"name": case["name"], "verdict": verdict, "detail": detail})
            print(f"{verdict:<17} {case['name']}\n                  {detail}", flush=True)

    counts = {}
    for r in rows:
        counts[r["verdict"]] = counts.get(r["verdict"], 0) + 1
    measured = sum(n for k, n in counts.items() if k in MEASURED)
    report.update({"rows": rows, "not_run": skipped_pairs, "counts": counts, "measured": measured})
    print("\n" + "=" * 70)
    print(f"{len(cases)} pair(s) discovered, {len(rows)} run, {len(skipped_pairs)} not run, "
          f"{measured} compared")
    for k in ("FALSE_CLEAN", "AGREE_AGAINST_W3C", "KNOWN_FALSE_CLEAN", "FALSE_ALARM", "PARTIAL",
              "ORACLE_WRONG",
              "UNDETERMINED", "AGREE", "ERROR"):
        if k in counts:
            print(f"  {k:<17} {counts[k]}")
    for s in skipped_pairs:
        print(f"  not run: {s['name']} - {s['reason']}")

    if control_failures:
        code, status = RED, "RED"
        for f in control_failures:
            print(f"\nRED: {f}")
        print("A comparator that misclassifies a planted difference cannot be trusted on the "
              "corpus, whatever the corpus said.")
    elif any(counts.get(k) for k in BLOCKING):
        code, status = RED, "RED"
    elif skipped_pairs:
        return not_measured(f"{len(skipped_pairs)} of {len(cases)} discovered pair(s) were not run",
                            report, args.json)
    elif measured == 0:
        return not_measured(f"none of the {len(rows)} pair(s) run yielded a comparison "
                            "(every one ERROR or UNDETERMINED)", report, args.json)
    else:
        code, status = GREEN, "GREEN"

    report.update({"status": status, "exit": code})
    if args.json:
        with open(args.json, "w") as fh:
            json.dump(report, fh, indent=1, default=sorted)
    return code


if __name__ == "__main__":
    sys.exit(main())
