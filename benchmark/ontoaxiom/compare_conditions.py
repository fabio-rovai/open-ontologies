#!/usr/bin/env python3
"""
Compare OntoAxiom condition A (class/property name lists) against condition D
(full raw Turtle source) for a given backend.

The paper's surprising claim is that handing the LLM the raw OWL file HURTS
(Claude: A=0.431 vs D=0.323). Reviewers suspected a Claude-specific memorization
artifact. This script checks whether the effect reproduces on a second model.

The two runs do not cover the same ontologies — condition D skips any file that
exceeds the context cap (era.ttl is 558 KB). Averaging over different sets and
comparing the results is a category error, so the headline number here is the
COMMON SUBSET average. Per-ontology deltas are printed so a single outlier can't
hide inside the mean.
"""
import argparse
import json
import os

from run_bare_llm_ablation import AXIOM_TYPES, ONTOLOGIES

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
RESULTS_DIR = os.path.join(SCRIPT_DIR, "data", "results")


def load(path):
    if not os.path.exists(path):
        raise SystemExit(f"missing results file: {path}\nRun the ablation first.")
    with open(path) as f:
        return json.load(f)


def onto_f1(scores, onto):
    """Mean F1 across axiom types for one ontology, or None if it wasn't scored."""
    f1s = [scores[f"{onto}_{ax}"]["f1"] for ax in AXIOM_TYPES if f"{onto}_{ax}" in scores]
    return sum(f1s) / len(f1s) if f1s else None


def mean(xs):
    return sum(xs) / len(xs) if xs else 0.0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--backend", default="qwen")
    args = ap.parse_args()

    a = load(os.path.join(RESULTS_DIR, f"oo_bare_{args.backend}_results.json"))
    d = load(os.path.join(RESULTS_DIR, f"oo_conditionD_{args.backend}_results.json"))

    a_scores, d_scores = a["scores"], d["scores"]
    common = [o for o in ONTOLOGIES
              if onto_f1(a_scores, o) is not None and onto_f1(d_scores, o) is not None]
    a_only = [o for o in ONTOLOGIES if onto_f1(a_scores, o) is not None and o not in common]
    d_only = [o for o in ONTOLOGIES if onto_f1(d_scores, o) is not None and o not in common]

    print("=" * 78)
    print(f"CONDITION A (name lists) vs CONDITION D (raw OWL)   backend={args.backend}")
    print(f"model: {a.get('model')}")
    print("=" * 78)

    print(f"\n{'ontology':<16}{'A (names)':>12}{'D (raw OWL)':>14}{'delta':>10}   effect")
    print("-" * 78)
    deltas = []
    for o in common:
        fa, fd = onto_f1(a_scores, o), onto_f1(d_scores, o)
        delta = fd - fa
        deltas.append(delta)
        effect = "raw OWL hurts" if delta < -0.01 else ("raw OWL helps" if delta > 0.01 else "no change")
        print(f"{o:<16}{fa:>12.3f}{fd:>14.3f}{delta:>+10.3f}   {effect}")

    a_common, d_common = mean([onto_f1(a_scores, o) for o in common]), mean([onto_f1(d_scores, o) for o in common])
    print("-" * 78)
    print(f"{'COMMON SUBSET':<16}{a_common:>12.3f}{d_common:>14.3f}{d_common - a_common:>+10.3f}   <- the comparable number")

    hurt = sum(1 for x in deltas if x < -0.01)
    print(f"\n  raw OWL hurt on {hurt}/{len(common)} ontologies")
    print(f"  reproduces the paper's direction: {'YES' if d_common < a_common else 'NO'}")
    print(f"  Claude reference:  A=0.431  D=0.323  (delta -0.108)")
    print(f"  {args.backend} measured: A={a_common:.3f}  D={d_common:.3f}  (delta {d_common - a_common:+.3f})")

    print("\n  Coverage and caveats")
    print(f"    common subset ({len(common)}): {', '.join(common) or 'none'}")
    if a_only:
        print(f"    A only ({len(a_only)}): {', '.join(a_only)} — excluded from the comparison")
    if d_only:
        print(f"    D only ({len(d_only)}): {', '.join(d_only)} — excluded from the comparison")
    for tag, res in (("A", a), ("D", d)):
        if res.get("truncated"):
            print(f"    {tag} truncated (recall is a lower bound): {', '.join(res['truncated'])}")
        if res.get("failed"):
            print(f"    {tag} failed: {', '.join(res['failed'])}")
        if res.get("skipped_too_large"):
            print(f"    {tag} skipped, too large for context: {', '.join(res['skipped_too_large'])}")

    if a.get("model") != d.get("model"):
        print(f"\n  WARNING: model mismatch — A={a.get('model')} vs D={d.get('model')}")


if __name__ == "__main__":
    main()
