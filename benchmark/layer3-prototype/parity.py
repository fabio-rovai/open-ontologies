"""Parity test: compiled DuckDB path vs HermiT full ABox consistency.

The Kalgera discipline. The fast path is only usable if it is pinned against
the audited baseline, otherwise it is an optimisation with no correctness story.

Generates random candidate claims from the ontology's own vocabulary, asks both
engines, and reports agreement. The asymmetry that matters:

  FALSE NEGATIVE  compiled says consistent, HermiT says inconsistent
                  -> a contradiction slipped through. This is the dangerous one.
  FALSE POSITIVE  compiled says inconsistent, HermiT says consistent
                  -> we rejected a valid claim. Annoying, not dangerous.

Usage: python parity.py <ontology.owl> [n_claims]
"""
from __future__ import annotations

import json
import random
import subprocess
import sys
import time
from pathlib import Path

import duckdb

REASONER_DIR = Path(__file__).resolve().parent.parent / "reasoner"
JAVA = "/opt/homebrew/opt/openjdk@17/bin/java"
CP = f".:{REASONER_DIR}/lib/*"


def compile_matrix(ont: str, out_csv: str) -> dict:
    r = subprocess.run(
        [JAVA, "-cp", CP, "DisjointnessMatrix", ont, out_csv],
        cwd=REASONER_DIR, capture_output=True, text=True,
    )
    for line in r.stdout.splitlines():
        if line.startswith("{"):
            return json.loads(line)
    raise RuntimeError(f"compile failed: {r.stdout[-400:]} {r.stderr[-400:]}")


def load_db(csv_path: str, ont: str) -> tuple[duckdb.DuckDBPyConnection, list[str]]:
    con = duckdb.connect()
    con.execute(
        f"CREATE TABLE incompatible AS SELECT * FROM read_csv_auto('{csv_path}')"
    )
    con.execute("CREATE INDEX ix ON incompatible(a)")
    con.execute("CREATE TABLE claim_type(subj VARCHAR, cls VARCHAR)")
    classes = sorted({r[0] for r in con.execute("SELECT a FROM incompatible").fetchall()})
    return con, classes


CHECK = """
SELECT 1 FROM claim_type c1
JOIN claim_type c2 ON c1.subj = c2.subj AND c1.cls < c2.cls
JOIN incompatible i ON i.a = c1.cls AND i.b = c2.cls
LIMIT 1
"""


def compiled_verdict(con, types) -> bool:
    """True = consistent."""
    con.execute("DELETE FROM claim_type")
    con.executemany("INSERT INTO claim_type VALUES (?,?)", types)
    return con.execute(CHECK).fetchone() is None


def main() -> None:
    ont = sys.argv[1]
    n = int(sys.argv[2]) if len(sys.argv) > 2 else 300
    csv_path = "/tmp/parity_matrix.csv"

    print(f"=== compiling {Path(ont).name} ===")
    stats = compile_matrix(ont, csv_path)
    print(f"  {stats}")

    con, classes = load_db(csv_path, ont)
    print(f"  classes in matrix: {len(classes)}")
    if len(classes) < 2:
        # No incompatible pairs at all: the ontology declares no disjointness
        # (explicit or inferred), so there is nothing for a contradiction check
        # to find. Worth reporting rather than crashing — a meaningful fraction
        # of real ontologies are in this state, and for them this layer adds
        # no value and the vocabulary checks are the only useful guard.
        print("\n  NO CONTRADICTION SURFACE: ontology has no unsatisfiable "
              "class pairs.\n  Compiled contradiction checking is a no-op here; "
              "only the closed-world\n  vocabulary checks would apply.")
        return

    # Generate claims: random pairs of classes asserted of one individual.
    rng = random.Random(20260727)
    claims = []
    for i in range(n):
        k = rng.choice([2, 2, 2, 3])
        picked = rng.sample(classes, min(k, len(classes)))
        claims.append({"id": f"c{i}", "types": [["x", c] for c in picked], "rels": []})

    # Compiled path
    t0 = time.perf_counter()
    compiled = {c["id"]: compiled_verdict(con, [(t[0], t[1]) for t in c["types"]])
                for c in claims}
    compiled_s = time.perf_counter() - t0

    # HermiT baseline
    payload = "\n".join(json.dumps(c) for c in claims)
    t0 = time.perf_counter()
    proc = subprocess.run(
        [JAVA, "-cp", CP, "ClaimConsistency", ont],
        cwd=REASONER_DIR, input=payload, capture_output=True, text=True,
    )
    hermit_s = time.perf_counter() - t0
    baseline = {}
    for line in proc.stdout.splitlines():
        if line.startswith("{"):
            d = json.loads(line)
            baseline[d["id"]] = d["consistent"]

    if not baseline:
        print("BASELINE FAILED:", proc.stderr[-500:])
        return

    agree = fn = fp = 0
    fn_examples = []
    for cid, comp in compiled.items():
        base = baseline.get(cid)
        if base is None:
            continue
        if comp == base:
            agree += 1
        elif comp and not base:
            fn += 1
            if len(fn_examples) < 3:
                types = next(c["types"] for c in claims if c["id"] == cid)
                fn_examples.append([t[1].rsplit("#", 1)[-1] for t in types])
        else:
            fp += 1

    total = len(baseline)
    print(f"\n=== PARITY over {total} claims ===")
    print(f"  agreement       {agree}/{total}  ({100.0*agree/total:.1f}%)")
    print(f"  FALSE NEGATIVES {fn}   (contradiction missed - DANGEROUS)")
    print(f"  false positives {fp}   (valid claim rejected)")
    for ex in fn_examples:
        print(f"    missed: {' + '.join(ex)}")
    print(f"\n=== SPEED over {total} claims ===")
    print(f"  compiled+DuckDB {compiled_s*1000:9.1f} ms total "
          f"({compiled_s/total*1000:.3f} ms/claim)")
    print(f"  HermiT baseline {hermit_s*1000:9.1f} ms total "
          f"({hermit_s/total*1000:.3f} ms/claim)")
    print(f"  speedup         {hermit_s/compiled_s:.0f}x")


if __name__ == "__main__":
    main()
