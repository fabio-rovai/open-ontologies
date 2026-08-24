"""Measure the DuckDB Layer 3 hot path against the 500ms budget."""
from __future__ import annotations

import statistics
import time

from onto_check import check_claim, prepare
from onto_compile import compile_ontology

P = "http://www.co-ode.org/ontologies/pizza/pizza.owl#"

# Realistic candidate claims, the kind Layer 1 would emit from LLM prose.
CLAIMS = {
    "valid: a pizza with a topping": (
        [("ex:p1", P + "American"), ("ex:t1", P + "MozzarellaTopping")],
        [("ex:p1", P + "hasTopping", "ex:t1")],
    ),
    "CONTRADICTION: one thing is both a pizza and a topping": (
        [("ex:x", P + "American"), ("ex:x", P + "AnchoviesTopping")],
        [],
    ),
    "CONTRADICTION: two named pizzas at once": (
        [("ex:x", P + "American"), ("ex:x", P + "AmericanHot")],
        [],
    ),
    "CONTRADICTION: functional property with two values": (
        [("ex:p2", P + "American")],
        [("ex:p2", P + "hasBase", "ex:b1"), ("ex:p2", P + "hasBase", "ex:b2")],
    ),
    "CONTRADICTION: domain violation (topping hasTopping)": (
        [("ex:t9", P + "MozzarellaTopping")],
        [("ex:t9", P + "hasTopping", "ex:t8")],
    ),
    "HALLUCINATION: invented class": (
        [("ex:z", P + "QuantumPineappleTopping")],
        [],
    ),
    "HALLUCINATION: invented property": (
        [("ex:p3", P + "American")],
        [("ex:p3", P + "hasVibes", "ex:v1")],
    ),
}


def main() -> None:
    con, stats = compile_ontology("/tmp/pizza_real.owl")
    print("=== OFFLINE COMPILE (once per ontology) ===")
    for k, v in stats.items():
        print(f"  {k:20s} {v}")

    prepare(con)

    print("\n=== PER-CLAIM VERDICTS ===")
    for name, (types, rels) in CLAIMS.items():
        r = check_claim(con, types, rels)
        verdict = "CONSISTENT" if r["consistent"] else "REJECTED"
        fired = ",".join(r["violations"]) or "-"
        print(f"  {verdict:11s} {r['elapsed_ms']:7.3f} ms  [{fired:38s}] {name}")

    # Latency distribution over a realistic mixed stream.
    print("\n=== LATENCY OVER 1000 CLAIMS (mixed) ===")
    keys = list(CLAIMS)
    lat = []
    for i in range(1000):
        types, rels = CLAIMS[keys[i % len(keys)]]
        lat.append(check_claim(con, types, rels)["elapsed_ms"])
    lat.sort()
    print(f"  mean   {statistics.mean(lat):.3f} ms")
    print(f"  median {lat[len(lat)//2]:.3f} ms")
    print(f"  p95    {lat[int(len(lat)*0.95)]:.3f} ms")
    print(f"  p99    {lat[int(len(lat)*0.99)]:.3f} ms")
    print(f"  max    {lat[-1]:.3f} ms")
    print(f"\n  500ms budget headroom at p95: {500/lat[int(len(lat)*0.95)]:.0f}x")


if __name__ == "__main__":
    main()
