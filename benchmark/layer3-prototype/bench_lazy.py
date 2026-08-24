"""Does lazy beat eager? Measure cache saturation over a realistic claim stream.

The eager matrix pays O(n^2) reasoner calls before answering a single claim.
The lazy matrix pays one call per DISTINCT pair actually asked about. The bet is
that a real claim stream touches a small, heavily-repeated working set, so the
lazy total collapses to a tiny fraction of the eager total.

Claim streams are Zipf-distributed over classes, because real domain traffic is:
a handful of classes dominate, most are rare. Uniform sampling is the pessimal
case for a cache, so we report both.
"""
from __future__ import annotations

import random
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lazy_matrix import LazyMatrix  # noqa: E402

import duckdb  # noqa: E402


def classes_of(ontology_csv: str) -> list[str]:
    con = duckdb.connect()
    con.execute(f"CREATE TABLE m AS SELECT * FROM read_csv_auto('{ontology_csv}')")
    return sorted({r[0] for r in con.execute("SELECT a FROM m").fetchall()})


def run(ontology: str, classes: list[str], n_claims: int, dist: str) -> dict:
    lm = LazyMatrix(ontology)
    rng = random.Random(20260727)

    # Zipf: rank-ordered popularity, the realistic case.
    if dist == "zipf":
        weights = [1.0 / (i + 1) for i in range(len(classes))]
    else:
        weights = [1.0] * len(classes)

    milestones = {}
    t_start = time.perf_counter()
    for i in range(n_claims):
        picked = rng.choices(classes, weights=weights, k=2)
        if picked[0] == picked[1]:
            continue
        lm.check_claim([("x", picked[0]), ("x", picked[1])])
        if (i + 1) in (10, 50, 100, 500, 1000, 2000, 5000):
            s = lm.stats
            milestones[i + 1] = s["hit_rate"]
    wall = time.perf_counter() - t_start

    s = lm.stats
    lm.close()
    n = len(classes)
    eager_pairs = n * (n - 1) // 2
    return {
        "dist": dist,
        "classes": n,
        "claims": n_claims,
        "distinct_pairs_needed": s["cached_pairs"],
        "eager_pairs": eager_pairs,
        "fraction_of_eager": s["cached_pairs"] / eager_pairs if eager_pairs else 0,
        "hit_rate": s["hit_rate"],
        "oracle_ms_mean": s["oracle_ms_mean"],
        "wall_s": round(wall, 2),
        "milestones": milestones,
    }


if __name__ == "__main__":
    ont = sys.argv[1]
    matrix_csv = sys.argv[2]
    n_claims = int(sys.argv[3]) if len(sys.argv) > 3 else 2000

    classes = classes_of(matrix_csv)
    print(f"classes with a contradiction surface: {len(classes)}")
    for dist in ("zipf", "uniform"):
        r = run(ont, classes, n_claims, dist)
        print(f"\n=== {r['dist'].upper()} over {r['claims']} claims ===")
        print(f"  distinct pairs actually needed : {r['distinct_pairs_needed']:,}")
        print(f"  pairs an eager matrix computes : {r['eager_pairs']:,}")
        print(f"  fraction of eager work done    : {r['fraction_of_eager']*100:.2f}%")
        print(f"  final cache hit rate           : {r['hit_rate']*100:.1f}%")
        print(f"  mean reasoner call             : {r['oracle_ms_mean']} ms")
        print(f"  wall clock                     : {r['wall_s']} s")
        print(f"  hit rate at N claims           : {r['milestones']}")
