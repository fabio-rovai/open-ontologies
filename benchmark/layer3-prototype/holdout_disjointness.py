"""Holdout evaluation of the assumed-disjointness WARN tier.

The dominant real-world gap: most large ontologies declare NO disjointness, so
the entailed contradiction tier is empty and verification value collapses onto
vocabulary checks. The WARN tier reconstructs a candidate surface:

    proposer  suggests class pairs that "should" be disjoint
    vetter    (VetDisjointness + HermiT) filters to admissible assumptions
    WARN tier surfaces conflicts with those assumptions, never rejects on them

This harness measures how good that reconstruction is, honestly:

  1. strip ALL disjointness from an ontology that has it
  2. propose pairs (built-in: sibling heuristic — classes sharing a direct
     parent; or --proposals file from an LLM, the MCP-native slot)
  3. vet against the STRIPPED ontology
  4. score the admissible set's two-hop closure against the ORIGINAL
     ontology's exhaustive incompatibility matrix:
       recall     fraction of truly-incompatible pairs the WARN tier covers
       precision  fraction of WARN-tier flags that are truly incompatible

Precision is the number that matters: a WARN tier that cries wolf is worse
than none.

Usage: holdout_disjointness.py <ontology.owl> <matrix.csv> [--proposals f.json]
"""
from __future__ import annotations

import csv
import itertools
import json
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REASONER_DIR = Path(__file__).resolve().parent.parent / "reasoner"
JAVA = "/opt/homebrew/opt/openjdk@17/bin/java"
CP = f".:{REASONER_DIR}/lib/*"


def run_tool(cls: str, *args: str, stdin: str | None = None, timeout: int = 900):
    p = subprocess.run([JAVA, "-cp", CP, cls, *args], cwd=REASONER_DIR,
                       input=stdin, capture_output=True, text=True, timeout=timeout)
    return [json.loads(l) for l in p.stdout.splitlines() if l.startswith("{")]


def direct_parents(supers: dict[str, set[str]]) -> dict[str, set[str]]:
    """Direct (minimal) parents from the reflexive-transitive closure."""
    out: dict[str, set[str]] = {}
    for c, sups in supers.items():
        strict = {s for s in sups if s != c}
        out[c] = {p for p in strict
                  if not any(p in supers.get(q, set()) and q != p for q in strict - {p})}
    return out


def sibling_proposals(supers: dict[str, set[str]]) -> list[tuple[str, str]]:
    """Classes sharing a direct parent, excluding comparable pairs.

    The 'strong disjointness assumption' from the ontology-learning
    literature: siblings are usually intended to be disjoint even when nobody
    wrote the axiom.
    """
    dp = direct_parents(supers)
    by_parent: dict[str, list[str]] = defaultdict(list)
    for c, parents in dp.items():
        for p in parents:
            by_parent[p].append(c)
    seen, out = set(), []
    for group in by_parent.values():
        for a, b in itertools.combinations(sorted(group), 2):
            if b in supers.get(a, set()) or a in supers.get(b, set()):
                continue  # comparable — vetting would reject anyway
            key = (a, b)
            if key not in seen:
                seen.add(key)
                out.append(key)
    return out


def main() -> None:
    ont = Path(sys.argv[1]).resolve()
    matrix = Path(sys.argv[2]).resolve()
    proposals_file = None
    if "--proposals" in sys.argv:
        proposals_file = sys.argv[sys.argv.index("--proposals") + 1]

    stripped = Path("/tmp") / (ont.stem + "_stripped.owl")
    removed = run_tool("StripDisjointness", str(ont), str(stripped))[0]["removed"]
    print(f"stripped {removed} disjointness axioms -> {stripped.name}")

    compiled = run_tool("CompileOntology", str(stripped), "/tmp/holdout_compiled.json")[0]
    print(f"stripped compile: {compiled}")
    data = json.load(open("/tmp/holdout_compiled.json"))
    supers: dict[str, set[str]] = {}
    for sub, sup in data["subsumptions"]:
        supers.setdefault(sub, set()).add(sup)

    if proposals_file:
        proposals = [tuple(p) for p in json.load(open(proposals_file))]
        source = f"LLM proposals ({proposals_file})"
    else:
        proposals = sibling_proposals(supers)
        source = "sibling heuristic"
    print(f"proposer [{source}]: {len(proposals)} candidate pairs")

    stdin = "\n".join(f"{a}\t{b}" for a, b in proposals)
    verdicts = run_tool("VetDisjointness", str(stripped), stdin=stdin)
    by_verdict: dict[str, int] = defaultdict(int)
    admissible = []
    for v in verdicts:
        by_verdict[v["verdict"]] += 1
        if v["verdict"] == "admissible":
            admissible.append((v["a"], v["b"]))
    print(f"vetting: {dict(by_verdict)}")

    # WARN-tier closure: two-hop over the stripped hierarchy + admissible set.
    assumed = set()
    for a, b in admissible:
        assumed.add((a, b))
        assumed.add((b, a))

    def warn(a: str, b: str) -> bool:
        return any((x, y) in assumed
                   for x in supers.get(a, {a}) for y in supers.get(b, {b}))

    truth = set()
    with open(matrix) as f:
        rd = csv.reader(f)
        next(rd, None)
        for a, b in rd:
            truth.add((a, b))

    classes = sorted(supers)
    tp = fp = fn = 0
    fp_examples = []
    for a, b in itertools.combinations(classes, 2):
        w = warn(a, b)
        t = (a, b) in truth
        if w and t:
            tp += 1
        elif w and not t:
            fp += 1
            if len(fp_examples) < 5:
                fp_examples.append((a.rsplit("#", 1)[-1], b.rsplit("#", 1)[-1]))
        elif t and not w:
            fn += 1

    print(f"\n=== WARN-tier reconstruction vs original ground truth ===")
    print(f"  truly incompatible pairs : {tp + fn:,}")
    print(f"  covered by WARN tier     : {tp:,}  (recall {100*tp/(tp+fn):.1f}%)"
          if tp + fn else "  (no ground truth)")
    if tp + fp:
        print(f"  WARN flags raised        : {tp + fp:,}  (precision {100*tp/(tp+fp):.1f}%)")
    print(f"  false warnings           : {fp:,}")
    for e in fp_examples:
        print(f"    false warn: {e[0]} + {e[1]}")


if __name__ == "__main__":
    main()
