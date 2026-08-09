"""Parity with STRUCTURAL claim generation.

The previous sweep sampled random class pairs. Only 11.5% of the resulting
claims exercised a contradiction, so 88% of the "100% agreement" result was two
engines agreeing that an unremarkable claim was fine. That is weak evidence, and
it also mislabelled two ontologies as having no contradiction surface when they
declare 5 and 63 disjointness axioms respectively.

This generator derives claims from the compiled structure instead, in three
deliberately adversarial classes:

  CONTRADICTORY   built from a disjoint pair, then pushed DOWN the hierarchy to
                  subclasses so the contradiction is only reachable by
                  inference. Both engines must reject.
  PLAUSIBLE       two classes sharing a superclass but not known disjoint.
                  These are where the join's incompleteness would show up.
  BENIGN          a class with one of its own superclasses. Must never be
                  rejected; a rejection here is an unsoundness bug.

The BENIGN class is the important one: it is a targeted soundness probe, not
padding.
"""
from __future__ import annotations

import json
import random
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REASONER_DIR = Path(__file__).resolve().parent.parent / "reasoner"
JAVA = "/opt/homebrew/opt/openjdk@17/bin/java"
CP = f".:{REASONER_DIR}/lib/*"


def compile_ontology(path: str, out: str) -> dict | None:
    try:
        p = subprocess.run([JAVA, "-cp", CP, "CompileOntology", path, out],
                           cwd=REASONER_DIR, capture_output=True, text=True, timeout=900)
    except subprocess.TimeoutExpired:
        return None
    for line in p.stdout.splitlines():
        if line.startswith("{"):
            return json.loads(line)
    return None


def build(compiled: dict):
    supers = defaultdict(set)
    subs = defaultdict(set)
    for a, b in compiled["subsumptions"]:
        supers[a].add(b)
        subs[b].add(a)
    disj = set()
    for a, b in compiled["disjoint"]:
        disj.add((a, b))
        disj.add((b, a))
    return supers, subs, disj


def generate(compiled: dict, n: int, rng: random.Random):
    supers, subs, disj = build(compiled)
    classes = sorted(supers)
    claims = []

    disj_pairs = sorted({tuple(sorted(p)) for p in disj})

    # CONTRADICTORY: descend from a disjoint pair to its subclasses, so the
    # clash is only visible through the inferred hierarchy.
    for a, b in disj_pairs:
        da = sorted(subs.get(a, set()) | {a})
        db = sorted(subs.get(b, set()) | {b})
        if not da or not db:
            continue
        claims.append(("contradictory", rng.choice(da), rng.choice(db)))
        if len(claims) >= n // 2:
            break

    # PLAUSIBLE: siblings under a shared superclass, not known disjoint.
    by_super = defaultdict(list)
    for c in classes:
        for s in supers[c]:
            if s != c:
                by_super[s].append(c)
    sibling_groups = [g for g in by_super.values() if len(g) >= 2]
    target = n // 4
    tries = 0
    added = 0
    while added < target and sibling_groups and tries < target * 40:
        tries += 1
        g = rng.choice(sibling_groups)
        x, y = rng.sample(g, 2)
        if (x, y) not in disj:
            claims.append(("plausible", x, y))
            added += 1

    # BENIGN: a class plus one of its own superclasses. Must never be rejected.
    added = 0
    tries = 0
    while added < n - len(claims) and tries < n * 40:
        tries += 1
        c = rng.choice(classes)
        sup = [s for s in supers[c] if s != c]
        if sup:
            claims.append(("benign", c, rng.choice(sup)))
            added += 1

    return claims


def baseline(ont: str, claims, timeout: int) -> dict:
    payload = "\n".join(
        json.dumps({"id": f"c{i}", "types": [["x", a], ["x", b]], "rels": []})
        for i, (_, a, b) in enumerate(claims)
    )
    try:
        p = subprocess.run([JAVA, "-cp", CP, "ClaimConsistency", ont],
                           cwd=REASONER_DIR, input=payload, capture_output=True,
                           text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return {}
    out = {}
    for line in p.stdout.splitlines():
        if line.startswith("{"):
            d = json.loads(line)
            out[d["id"]] = d["consistent"]
    return out


def join_fn(compiled: dict):
    supers, _, disj = build(compiled)
    unsat = set(compiled["unsatisfiable"])

    def f(a, b):
        if a in unsat or b in unsat:
            return True
        return any((x, y) in disj for x in supers.get(a, {a}) for y in supers.get(b, {b}))

    return f


def main() -> None:
    ont = sys.argv[1]
    n = int(sys.argv[2]) if len(sys.argv) > 2 else 120

    compiled_meta = compile_ontology(ont, "/tmp/sp_compiled.json")
    if compiled_meta is None:
        print("compile failed/timeout")
        return
    compiled = json.load(open("/tmp/sp_compiled.json"))
    print(f"{Path(ont).name}: {compiled_meta}")

    rng = random.Random(20260727)
    claims = generate(compiled, n, rng)
    if not claims:
        print("  no claims generated (no hierarchy/disjointness)")
        return
    kinds = defaultdict(int)
    for k, _, _ in claims:
        kinds[k] += 1
    print(f"  generated {len(claims)} claims: {dict(kinds)}")

    f = join_fn(compiled)
    base = baseline(ont, claims, 300)
    if not base:
        print("  baseline timeout")
        return

    stats = defaultdict(lambda: [0, 0, 0, 0])  # compared, agree, FN, FP
    unsound_examples = []
    for i, (kind, a, b) in enumerate(claims):
        hb = base.get(f"c{i}")
        if hb is None:
            continue
        join_rejects = f(a, b)
        truth_rejects = not hb
        s = stats[kind]
        s[0] += 1
        if join_rejects == truth_rejects:
            s[1] += 1
        elif truth_rejects and not join_rejects:
            s[2] += 1  # missed: goes to tier 2
        else:
            s[3] += 1  # UNSOUND
            if len(unsound_examples) < 3:
                unsound_examples.append((kind, a.rsplit("#", 1)[-1], b.rsplit("#", 1)[-1]))

    print(f"\n  {'kind':14s} {'n':>5s} {'agree':>6s} {'tier2':>6s} {'UNSOUND':>8s}")
    tot = [0, 0, 0, 0]
    for k in ("contradictory", "plausible", "benign"):
        c, a, fn, fp = stats[k]
        if c:
            print(f"  {k:14s} {c:5d} {a:6d} {fn:6d} {fp:8d}")
        for j in range(4):
            tot[j] += stats[k][j]
    print(f"  {'TOTAL':14s} {tot[0]:5d} {tot[1]:6d} {tot[2]:6d} {tot[3]:8d}")
    if tot[0]:
        rate = 100 * (tot[0] - tot[2] - tot[3]) / tot[0]
        print(f"\n  exercised a real contradiction: "
              f"{sum(1 for i,(k,a,b) in enumerate(claims) if base.get(f'c{i}') is False)}"
              f"/{tot[0]}")
        print(f"  join agreement: {rate:.1f}%   UNSOUND: {tot[3]}")
    for e in unsound_examples:
        print(f"    !! UNSOUND {e}")


if __name__ == "__main__":
    main()
