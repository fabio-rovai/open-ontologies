#!/usr/bin/env python3
"""Compare reasoner classification results for correctness."""
import json
import sys


def load_subsumptions(path):
    with open(path) as f:
        data = json.load(f)
    return set(data.get("subsumptions", [])), data.get("time_ms", 0), data.get("reasoner", "unknown")


def main():
    if len(sys.argv) < 4:
        print("Usage: compare_results.py <hermit.json> <pellet.json> <oo.json>")
        sys.exit(1)

    hermit_subs, hermit_ms, _ = load_subsumptions(sys.argv[1])
    pellet_subs, pellet_ms, _ = load_subsumptions(sys.argv[2])
    oo_subs, oo_ms, _ = load_subsumptions(sys.argv[3])

    # Compare OO against HermiT (reference)
    oo_vs_hermit_missing = hermit_subs - oo_subs
    oo_vs_hermit_extra = oo_subs - hermit_subs

    # Compare OO against Pellet
    oo_vs_pellet_missing = pellet_subs - oo_subs
    oo_vs_pellet_extra = oo_subs - pellet_subs

    # Compare HermiT vs Pellet (baseline)
    hermit_vs_pellet_diff = hermit_subs.symmetric_difference(pellet_subs)

    print(f"{'Reasoner':<25} {'Time (ms)':<12} {'Subsumptions':<15}")
    print("-" * 52)
    print(f"{'HermiT':<25} {hermit_ms:<12} {len(hermit_subs):<15}")
    print(f"{'Pellet':<25} {pellet_ms:<12} {len(pellet_subs):<15}")
    print(f"{'Open Ontologies':<25} {oo_ms:<12} {len(oo_subs):<15}")
    print()

    if hermit_vs_pellet_diff:
        print(f"HermiT vs Pellet: {len(hermit_vs_pellet_diff)} differences (baseline)")
    else:
        print("HermiT vs Pellet: EXACT MATCH (baseline)")

    print()

    if not oo_vs_hermit_missing and not oo_vs_hermit_extra:
        print("OO vs HermiT: EXACT MATCH")
    else:
        print(f"OO vs HermiT: {len(oo_vs_hermit_missing)} missing, {len(oo_vs_hermit_extra)} extra")
        for s in sorted(oo_vs_hermit_missing)[:10]:
            print(f"  MISSING: {s}")
        for s in sorted(oo_vs_hermit_extra)[:10]:
            print(f"  EXTRA:   {s}")

    if not oo_vs_pellet_missing and not oo_vs_pellet_extra:
        print("OO vs Pellet: EXACT MATCH")
    else:
        print(f"OO vs Pellet: {len(oo_vs_pellet_missing)} missing, {len(oo_vs_pellet_extra)} extra")

    # Summary
    print()
    total_issues = len(oo_vs_hermit_missing) + len(oo_vs_hermit_extra)
    if total_issues == 0:
        print("RESULT: Open Ontologies produces identical classification to HermiT")
    else:
        print(f"RESULT: {total_issues} differences found vs HermiT reference")


if __name__ == "__main__":
    main()
