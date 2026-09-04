#!/usr/bin/env python3
"""Differential oracle: open-ontologies SHACL against pyshacl, over real shapes.

A validator is trusted on evidence, not on its own test suite. This runs both
engines over the same (data, shapes) pairs and compares three things in order of
severity:

  1. FALSE CLEAN   - we say conforms, pyshacl says not. The one failure mode the
                     validator must not have. Any of these is a stop-the-line bug.
  2. FALSE ALARM   - we say not conforms, pyshacl says conforms. Noise that makes
                     a gate untrustworthy in the other direction.
  3. UNDETERMINED  - we return null because something was skipped. Honest, but a
                     gap: the shapes graph got no verdict. These are the work list.

Agreement on the verdict is not enough, so where both engines give a verdict the
violation sets are compared as (focus_node, path) pairs.

Usage:
    python3 tools/shacl_differential.py CORPUS.json [--json out.json]

CORPUS.json is a list of {"name", "data": [ttl...], "shapes": ttl} objects, or
pass --discover ROOT to build one from a tree laid out as */shacl/*.ttl or
*/shapes/*.ttl with data in */graph/*.ttl, */data/*.ttl or */examples/*.ttl.

Nothing is capped silently: every pair excluded for size or timeout is listed in
the report with the reason.
"""
import argparse
import json
import os
import pathlib
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
            return json.loads(r.stdout)
        except json.JSONDecodeError:
            return {"error": f"unparseable report: {(r.stdout + r.stderr).strip()[:300]}"}


def run_pyshacl(data_files, shapes_file):
    from pyshacl import validate
    from rdflib import Graph

    data = Graph()
    for f in data_files:
        data.parse(str(f))
    shapes = Graph().parse(str(shapes_file))
    conforms, results_graph, _ = validate(data, shacl_graph=shapes)
    violations = set()
    from rdflib.namespace import Namespace
    SH = Namespace("http://www.w3.org/ns/shacl#")
    # Every severity, not just sh:Violation. Collecting only Violations here
    # compared pyshacl's violations against our results at ALL severities, and
    # reported the difference as our defect: four sh:Warning results in the
    # enterprise-knowledge vertical showed up as four extra violations we had
    # invented. An oracle that manufactures findings is worse than no oracle.
    for result in set(results_graph.subjects(SH.resultSeverity, None)):
        focus = results_graph.value(result, SH.focusNode)
        path = results_graph.value(result, SH.resultPath)
        violations.add((str(focus), str(path) if path else None))
    return {"conforms": conforms, "violations": violations}


def ours_violation_set(report):
    out = set()
    for v in report.get("violations", []) or []:
        out.add((str(v.get("focus_node")), v.get("path")))
    return out


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
        return "FALSE_CLEAN", f"pyshacl found {len(theirs['violations'])} violation(s), we found none"
    if our_conforms is False and their_conforms is True:
        return "FALSE_ALARM", f"we reported {len(ours_violation_set(ours))} violation(s), pyshacl found none"

    ours_v, theirs_v = ours_violation_set(ours), theirs["violations"]
    missed, extra = theirs_v - ours_v, ours_v - theirs_v
    if missed or extra:
        return "PARTIAL", f"{len(missed)} missed, {len(extra)} extra (verdict agrees)"
    return "AGREE", f"{len(ours_v)} violation(s), identical sets"


def discover(root):
    cases, root = [], pathlib.Path(root)
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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("corpus", nargs="?")
    ap.add_argument("--discover")
    ap.add_argument("--json")
    ap.add_argument("--limit", type=int)
    args = ap.parse_args()

    cases = discover(args.discover) if args.discover else json.load(open(args.corpus))
    if args.limit:
        cases = cases[: args.limit]

    rows, skipped_pairs = [], []
    for case in cases:
        data = [f for f in case["data"] if os.path.exists(f)]
        size = sum(os.path.getsize(f) for f in data) + os.path.getsize(case["shapes"])
        if size > MAX_BYTES:
            reason = f"{size/1e6:.0f} MB over the {MAX_BYTES/1e6:.0f} MB cap"
            skipped_pairs.append({"name": case["name"], "reason": reason})
            print(f"{'NOT_RUN':<13} {case['name']}\n              {reason}", flush=True)
            continue
        try:
            ours = run_open_ontologies(data, case["shapes"])
            theirs = run_pyshacl(data, case["shapes"])
            verdict, detail = classify(ours, theirs)
        except subprocess.TimeoutExpired:
            reason = f"timed out after {TIMEOUT}s"
            skipped_pairs.append({"name": case["name"], "reason": reason})
            print(f"{'NOT_RUN':<13} {case['name']}\n              {reason}", flush=True)
            continue
        except Exception as exc:  # a parse failure in either engine is a result, not a crash
            verdict, detail = "ERROR", f"{type(exc).__name__}: {exc}"[:300]
        rows.append({"name": case["name"], "verdict": verdict, "detail": detail})
        print(f"{verdict:<13} {case['name']}\n              {detail}", flush=True)

    counts = {}
    for r in rows:
        counts[r["verdict"]] = counts.get(r["verdict"], 0) + 1
    print("\n" + "=" * 70)
    print(f"{len(rows)} pair(s) run, {len(skipped_pairs)} not run")
    for k in ("FALSE_CLEAN", "FALSE_ALARM", "PARTIAL", "UNDETERMINED", "AGREE", "ERROR"):
        if k in counts:
            print(f"  {k:<13} {counts[k]}")
    for s in skipped_pairs:
        print(f"  not run: {s['name']} - {s['reason']}")

    if args.json:
        json.dump({"rows": rows, "not_run": skipped_pairs, "counts": counts}, open(args.json, "w"), indent=1)

    # A false clean is the one result that must fail a pipeline.
    return 1 if counts.get("FALSE_CLEAN") else 0


if __name__ == "__main__":
    sys.exit(main())
