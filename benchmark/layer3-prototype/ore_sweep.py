"""ORE 2015 sweep: parity and coverage of the lazy compiled path.

Answers the two questions that decide whether this architecture is claimable:

  1. COVERAGE  What fraction of real ontologies actually have a contradiction
               surface? Where there is none, contradiction checking is a no-op
               and only the closed-world vocabulary checks apply.
  2. PARITY    On ontologies that do, does the compiled path agree with HermiT
               doing full ABox consistency? False negatives are disqualifying.

Reports per-ontology and in aggregate. Ontologies that fail to load or time out
are counted, not silently dropped, because a sweep that hides its failures is
how the last benchmark went wrong.
"""
from __future__ import annotations

import json
import random
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lazy_matrix import LazyMatrix  # noqa: E402

REASONER_DIR = Path(__file__).resolve().parent.parent / "reasoner"
JAVA = "/opt/homebrew/opt/openjdk@17/bin/java"
CP = f".:{REASONER_DIR}/lib/*"

CLASS_RE = re.compile(r"Declaration\(Class\(<([^>]+)>\)\)")
RDFXML_CLASS_RE = re.compile(r'owl:Class rdf:about="([^"]+)"')


def classes_of(path: Path) -> list[str]:
    txt = path.read_text(errors="ignore")
    found = set(CLASS_RE.findall(txt)) or set(RDFXML_CLASS_RE.findall(txt))
    return sorted(c for c in found if "error#Error" not in c)


def hermit_baseline(ont: str, claims: list[dict], timeout: int) -> dict:
    payload = "\n".join(json.dumps(c) for c in claims)
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


def sweep_one(path: Path, n_claims: int, timeout: int,
              budget_s: float = 90.0) -> dict:
    res = {"ontology": path.name, "status": "ok"}
    classes = classes_of(path)
    res["classes"] = len(classes)
    if len(classes) < 2:
        res["status"] = "no_classes"
        return res

    rng = random.Random(20260727)
    weights = [1.0 / (i + 1) for i in range(len(classes))]

    lm = LazyMatrix(str(path))
    claims, compiled = [], {}
    try:
        t0 = time.perf_counter()
        for i in range(n_claims):
            # Large ontologies carry a real one-off reasoner warm-up (measured
            # ~15 s at 6,929 classes, plus ~15 s on the first pair). Budget it
            # and record the truncation rather than stalling the sweep.
            if time.perf_counter() - t0 > budget_s:
                res["status"] = "budget_exceeded"
                res["claims_done"] = len(compiled)
                break
            picked = rng.choices(classes, weights=weights, k=2)
            if picked[0] == picked[1]:
                continue
            cid = f"c{i}"
            ok, _ = lm.check_claim([("x", picked[0]), ("x", picked[1])])
            compiled[cid] = ok
            claims.append({"id": cid,
                           "types": [["x", picked[0]], ["x", picked[1]]],
                           "rels": []})
        res["compiled_s"] = round(time.perf_counter() - t0, 3)
        res.update({k: lm.stats[k] for k in ("cache_hits", "oracle_calls", "cached_pairs")})
    except Exception as e:
        res["status"] = f"compiled_error:{type(e).__name__}"
        try:
            lm.close()
        except Exception:
            pass
        return res
    finally:
        try:
            lm.close()
        except Exception:
            pass

    # Does this ontology have any contradiction surface at all?
    res["rejected_by_compiled"] = sum(1 for v in compiled.values() if not v)
    if res["rejected_by_compiled"] == 0:
        res["status"] = "no_contradiction_surface"

    if not claims:
        res["status"] = "no_claims_completed"
        return res

    t0 = time.perf_counter()
    base = hermit_baseline(str(path), claims, timeout)
    res["hermit_s"] = round(time.perf_counter() - t0, 3)
    if not base:
        res["status"] = "baseline_timeout_or_error"
        return res

    agree = fn = fp = 0
    for cid, c in compiled.items():
        b = base.get(cid)
        if b is None:
            continue
        if c == b:
            agree += 1
        elif c and not b:
            fn += 1
        else:
            fp += 1
    res.update({"compared": agree + fn + fp, "agree": agree,
                "false_negatives": fn, "false_positives": fp})
    return res


def main() -> None:
    corpus = Path(sys.argv[1])
    n_ont = int(sys.argv[2]) if len(sys.argv) > 2 else 25
    n_claims = int(sys.argv[3]) if len(sys.argv) > 3 else 60
    timeout = 120

    files = sorted(corpus.glob("*.owl"))
    rng = random.Random(20260727)
    sample = rng.sample(files, min(n_ont, len(files)))

    rows = []
    for i, f in enumerate(sample, 1):
        try:
            r = sweep_one(f, n_claims, timeout)
        except Exception as e:
            r = {"ontology": f.name, "status": f"fatal:{type(e).__name__}"}
        rows.append(r)
        print(f"[{i}/{len(sample)}] {r['ontology'][:28]:30s} "
              f"{r.get('status'):26s} cls={r.get('classes','-')} "
              f"agree={r.get('agree','-')} FN={r.get('false_negatives','-')}",
              flush=True)

    print("\n================ AGGREGATE ================")
    total = len(rows)
    by_status: dict[str, int] = {}
    for r in rows:
        by_status[r["status"]] = by_status.get(r["status"], 0) + 1
    for k, v in sorted(by_status.items(), key=lambda x: -x[1]):
        print(f"  {k:32s} {v:3d}/{total}")

    usable = [r for r in rows if r.get("compared")]
    if usable:
        c = sum(r["compared"] for r in usable)
        a = sum(r["agree"] for r in usable)
        fn = sum(r["false_negatives"] for r in usable)
        fp = sum(r["false_positives"] for r in usable)
        cs = sum(r["compiled_s"] for r in usable)
        hs = sum(r["hermit_s"] for r in usable)
        print(f"\n  ontologies with a baseline    : {len(usable)}")
        print(f"  claims compared               : {c}")
        print(f"  agreement                     : {a}/{c} ({100*a/c:.2f}%)")
        print(f"  FALSE NEGATIVES (dangerous)   : {fn}")
        print(f"  false positives               : {fp}")
        print(f"  compiled total                : {cs:.2f} s")
        print(f"  hermit total                  : {hs:.2f} s")
        print(f"  speedup                       : {hs/cs:.1f}x")
        surf = [r for r in usable if r.get("rejected_by_compiled", 0) > 0]
        print(f"  with a contradiction surface  : {len(surf)}/{len(usable)}"
              f" ({100*len(surf)/len(usable):.0f}%)")

    Path("/tmp/ore_sweep_results.json").write_text(json.dumps(rows, indent=2))
    print("\n  raw results -> /tmp/ore_sweep_results.json")


if __name__ == "__main__":
    main()
