"""Is the 2-hop join SOUND across ontologies, not just on Pizza?

The tiered design depends on one property:

    if the join says "incompatible", it IS incompatible

A single counterexample kills the design, because tier 1 rejects claims without
consulting a reasoner. Completeness is not required (tier 2 handles the
residual), but soundness is non-negotiable.

Ground truth is the brute-force pairwise matrix computed by HermiT, so this is
restricted to ontologies small enough for n^2 to be feasible.
"""
from __future__ import annotations

import csv
import itertools
import json
import re
import subprocess
import sys
from pathlib import Path

REASONER_DIR = Path(__file__).resolve().parent.parent / "reasoner"
JAVA = "/opt/homebrew/opt/openjdk@17/bin/java"
CP = f".:{REASONER_DIR}/lib/*"


def run(cls: str, *args: str, timeout: int = 600) -> dict | None:
    try:
        p = subprocess.run([JAVA, "-cp", CP, cls, *args], cwd=REASONER_DIR,
                           capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return None
    for line in p.stdout.splitlines():
        if line.startswith("{"):
            return json.loads(line)
    return None


def join_incompatible(compiled: dict):
    supers: dict[str, set[str]] = {}
    for sub, sup in compiled["subsumptions"]:
        supers.setdefault(sub, set()).add(sup)
    disj = set()
    for a, b in compiled["disjoint"]:
        disj.add((a, b))
        disj.add((b, a))
    unsat = set(compiled["unsatisfiable"])

    def f(a: str, b: str) -> bool:
        if a in unsat or b in unsat:
            return True
        return any((x, y) in disj
                   for x in supers.get(a, {a})
                   for y in supers.get(b, {b}))

    return f, sorted(supers)


def check(path: Path, max_classes: int) -> dict:
    res = {"ontology": path.name}
    n = len(re.findall(r"Declaration\(Class\(<", path.read_text(errors="ignore")))
    res["declared_classes"] = n
    if n > max_classes:
        res["status"] = "skipped_too_large"
        return res

    compiled = run("CompileOntology", str(path), "/tmp/vjs_compiled.json")
    if compiled is None:
        res["status"] = "compile_failed"
        return res
    res["classify_ms"] = compiled["classify_ms"]
    res["compiled_rows"] = compiled["subsumptions"] + compiled["disjoint_axioms"]

    matrix = run("DisjointnessMatrix", str(path), "/tmp/vjs_matrix.csv")
    if matrix is None:
        res["status"] = "matrix_timeout"
        return res
    res["matrix_ms"] = matrix["compile_ms"]
    res["matrix_pairs"] = matrix["incompatible_pairs"]

    data = json.load(open("/tmp/vjs_compiled.json"))
    f, classes = join_incompatible(data)
    truth = set()
    with open("/tmp/vjs_matrix.csv") as fh:
        rd = csv.reader(fh)
        next(rd, None)
        for a, b in rd:
            truth.add((a, b))

    settled = unsound = residual = missed = 0
    examples = []
    for a, b in itertools.combinations(classes, 2):
        if f(a, b):
            settled += 1
            if (a, b) not in truth:
                unsound += 1
                if len(examples) < 3:
                    examples.append((a.rsplit("#", 1)[-1], b.rsplit("#", 1)[-1]))
        else:
            residual += 1
            if (a, b) in truth:
                missed += 1

    total = settled + residual
    res.update({
        "status": "ok",
        "pairs": total,
        "settled_by_join": settled,
        "settled_pct": round(100 * settled / total, 1) if total else 0.0,
        "UNSOUND": unsound,
        "unsound_examples": examples,
        "residual": residual,
        "missed_by_join": missed,
    })
    return res


def main() -> None:
    corpus = Path(sys.argv[1])
    max_classes = int(sys.argv[2]) if len(sys.argv) > 2 else 250

    targets = [p for p in sorted(corpus.glob("*.owl"))]
    rows, done = [], 0
    for p in targets:
        if done >= 12:
            break
        r = check(p, max_classes)
        if r["status"] == "skipped_too_large":
            continue
        done += 1
        rows.append(r)
        print(f"{r['ontology'][:26]:28s} {r['status']:16s} "
              f"cls={r.get('declared_classes','-'):>5} "
              f"settled={r.get('settled_pct','-')}% "
              f"UNSOUND={r.get('UNSOUND','-')} "
              f"missed={r.get('missed_by_join','-')}", flush=True)
        if r.get("UNSOUND"):
            print(f"    !! counterexamples: {r['unsound_examples']}")

    ok = [r for r in rows if r["status"] == "ok"]
    print("\n================ SOUNDNESS VERDICT ================")
    print(f"  ontologies checked        : {len(ok)}")
    if ok:
        tu = sum(r["UNSOUND"] for r in ok)
        tp = sum(r["pairs"] for r in ok)
        ts = sum(r["settled_by_join"] for r in ok)
        tm = sum(r["missed_by_join"] for r in ok)
        print(f"  pairs compared            : {tp:,}")
        print(f"  settled by join           : {ts:,} ({100*ts/tp:.1f}%)")
        print(f"  UNSOUND (join wrongly rejects) : {tu}")
        print(f"  missed by join (goes to tier 2): {tm:,}")
        print(f"  compile: join {sum(r['classify_ms'] for r in ok):,} ms"
              f"  vs matrix {sum(r['matrix_ms'] for r in ok):,} ms")
        print(f"\n  {'SOUND on this sample' if tu == 0 else 'UNSOUND - DESIGN INVALID'}")
    Path("/tmp/join_soundness.json").write_text(json.dumps(rows, indent=2))


if __name__ == "__main__":
    main()
