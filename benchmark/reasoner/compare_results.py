#!/usr/bin/env python3
"""Compare the engine's owl-dl classification with HermiT's, like with like.

Usage: compare_results.py <hermit.json> <oo_result.json> [--pellet <pellet.json>]

<hermit.json> is JavaReasoner output: the FULL entailed named hierarchy (all
superclasses plus equivalent classes) and the told closure. <oo_result.json> is
the engine's `reason --profile owl-dl` result. The engine side is read from
classification[].superclasses plus equivalences, never from a `subsumptions`
key, which the engine does not emit. An earlier version read that key from the
engine result, got an empty set, and compared HermiT's direct superclasses
against nothing.

The pair contract and the scoring are shared with run_ore.py, so the Pizza
figure and the ORE figures mean the same thing.
"""
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from run_ore import compare, engine_view, java_pairs, pct  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("hermit")
    ap.add_argument("oo")
    ap.add_argument("--pellet")
    args = ap.parse_args()

    hermit = json.load(open(args.hermit))
    oo = json.load(open(args.oo))
    if "subsumptions" not in hermit or "told_subsumptions" not in hermit:
        sys.exit("ERROR: {} is not JavaReasoner output with the full hierarchy; "
                 "rebuild JavaReasoner.java".format(args.hermit))
    if "classification" not in oo:
        sys.exit("ERROR: {} has no classification; it is not an owl-dl reason result".format(args.oo))

    view = engine_view(oo)
    r = compare(view, hermit)

    print("HermiT: load {} ms, create {} ms, classify {} ms (JVM start {} ms, excluded)".format(
        hermit.get("load_ms"), hermit.get("reasoner_create_ms"), hermit.get("classify_ms"),
        hermit.get("jvm_start_ms")))
    print("Engine: reason {} ms (agents.total_time_ms), complete={}, undetermined={}, "
          "budget_exhausted_in={}".format(view["reason_ms"], view["complete"],
                                          len(view["undetermined"]), view["budget_exhausted_in"] or "none"))
    print()
    print("Gold (HermiT, all superclasses + equivalents): {}".format(r["gold"]))
    print("  told closure (asserted hierarchy):            {}".format(r["told"]))
    print("  inferred only (gold minus told):              {}".format(r["inferred_only"]))
    if r["told_not_entailed"]:
        print("  WARNING: {} told pairs HermiT does not entail; the told closure is wrong".format(
            r["told_not_entailed"]))
    print("Engine pairs:                                   {}".format(r["engine_pairs"]))
    print("  found in gold:                                {}  recall {}".format(r["found"], pct(r["recall_all"])))
    print("  found among inferred-only:                    {}  recall {}".format(
        r["found_inferred"], pct(r["recall_inferred"])))
    print("  WRONG (engine claims, HermiT rejects):        {}".format(len(r["wrong"])))
    for p in r["wrong"][:20]:
        print("    WRONG: {}".format(p))
    print("Unsatisfiable: HermiT {}, engine {}; engine-only (wrong) {}, missed {}".format(
        r["hermit_unsat"], r["engine_unsat"], len(r["wrong_unsat"]), len(r["missed_unsat"])))
    for c in r["wrong_unsat"][:20]:
        print("    WRONG UNSAT: {}".format(c))
    if r["told_only"]:
        print("NOTE: no inferred-only pairs; a structural walk scores 100% here, so this file proves nothing about reasoning")

    if args.pellet:
        pellet = json.load(open(args.pellet))
        h, p = java_pairs(hermit["subsumptions"]), java_pairs(pellet.get("subsumptions") or [])
        diff = h ^ p
        print()
        print("HermiT vs Pellet (baseline): {}".format(
            "EXACT MATCH" if not diff else "{} differences".format(len(diff))))

    print()
    if r["wrong"] or r["wrong_unsat"]:
        print("RESULT: {} wrong answers vs HermiT".format(len(r["wrong"]) + len(r["wrong_unsat"])))
    else:
        print("RESULT: no wrong answers; {} of {} entailed pairs found ({} of {} inferred-only)".format(
            r["found"], r["gold"], r["found_inferred"], r["inferred_only"]))


if __name__ == "__main__":
    main()
