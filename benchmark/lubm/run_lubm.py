#!/usr/bin/env python3
"""LUBM harness: load, reason, and run the 14 standard queries, with timings.

LUBM (Guo, Pan and Heflin) is the benchmark RDF stores publish against, so it
is the honest way to find out where this engine actually stands rather than
asserting a position. The point is not to win: it is to replace opinion with
a table, including the rows where we lose.

What is measured
  - load time and the resulting triple count, cold, from the generated files
  - OWL-RL materialisation time and the inferred triple count
  - each of the 14 LUBM queries, best of N runs, with result counts

Queries 4, 6, 8, 10, 11, 12, 13 need inference to return complete answers
(subclass and subproperty entailment, transitive subOrganizationOf,
inverse properties). Running the suite before and after materialisation
therefore also shows which answers the reasoner is responsible for, which is
more informative than a single number.

Usage:
    python3 run_lubm.py --data data1 --binary ../../target/release/open-ontologies
    python3 run_lubm.py --data data10 --reason --runs 3 --out results-10.json
"""

from __future__ import annotations

import argparse
import glob
import json
import pathlib
import subprocess
import time

UB = "http://swat.cse.lehigh.edu/onto/univ-bench.owl#"
RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
RDFS = "http://www.w3.org/2000/01/rdf-schema#"

# The 14 standard LUBM queries. Kept in SPARQL rather than the original
# KIF/description-logic notation, which is how every published comparison
# states them.
QUERIES: list[tuple[str, str, bool]] = [
    ("Q1", f"""SELECT ?x WHERE {{
        ?x <{RDF}type> <{UB}GraduateStudent> .
        ?x <{UB}takesCourse> <http://www.Department0.University0.edu/GraduateCourse0> }}""", False),
    ("Q2", f"""SELECT ?x ?y ?z WHERE {{
        ?x <{RDF}type> <{UB}GraduateStudent> . ?y <{RDF}type> <{UB}University> .
        ?z <{RDF}type> <{UB}Department> . ?x <{UB}memberOf> ?z .
        ?z <{UB}subOrganizationOf> ?y . ?x <{UB}undergraduateDegreeFrom> ?y }}""", True),
    ("Q3", f"""SELECT ?x WHERE {{
        ?x <{RDF}type> <{UB}Publication> .
        ?x <{UB}publicationAuthor> <http://www.Department0.University0.edu/AssistantProfessor0> }}""", False),
    ("Q4", f"""SELECT ?x ?y1 ?y2 ?y3 WHERE {{
        ?x <{RDF}type> <{UB}Professor> .
        ?x <{UB}worksFor> <http://www.Department0.University0.edu> .
        ?x <{UB}name> ?y1 . ?x <{UB}emailAddress> ?y2 . ?x <{UB}telephone> ?y3 }}""", True),
    ("Q5", f"""SELECT ?x WHERE {{
        ?x <{RDF}type> <{UB}Person> .
        ?x <{UB}memberOf> <http://www.Department0.University0.edu> }}""", True),
    ("Q6", f"""SELECT ?x WHERE {{ ?x <{RDF}type> <{UB}Student> }}""", True),
    ("Q7", f"""SELECT ?x ?y WHERE {{
        ?x <{RDF}type> <{UB}Student> . ?y <{RDF}type> <{UB}Course> .
        ?x <{UB}takesCourse> ?y .
        <http://www.Department0.University0.edu/AssociateProfessor0> <{UB}teacherOf> ?y }}""", True),
    ("Q8", f"""SELECT ?x ?y ?z WHERE {{
        ?x <{RDF}type> <{UB}Student> . ?y <{RDF}type> <{UB}Department> .
        ?x <{UB}memberOf> ?y .
        ?y <{UB}subOrganizationOf> <http://www.University0.edu> .
        ?x <{UB}emailAddress> ?z }}""", True),
    ("Q9", f"""SELECT ?x ?y ?z WHERE {{
        ?x <{RDF}type> <{UB}Student> . ?y <{RDF}type> <{UB}Faculty> .
        ?z <{RDF}type> <{UB}Course> . ?x <{UB}advisor> ?y .
        ?y <{UB}teacherOf> ?z . ?x <{UB}takesCourse> ?z }}""", True),
    ("Q10", f"""SELECT ?x WHERE {{
        ?x <{RDF}type> <{UB}Student> .
        ?x <{UB}takesCourse> <http://www.Department0.University0.edu/GraduateCourse0> }}""", True),
    ("Q11", f"""SELECT ?x WHERE {{
        ?x <{RDF}type> <{UB}ResearchGroup> .
        ?x <{UB}subOrganizationOf> <http://www.University0.edu> }}""", True),
    ("Q12", f"""SELECT ?x ?y WHERE {{
        ?x <{RDF}type> <{UB}Chair> . ?y <{RDF}type> <{UB}Department> .
        ?x <{UB}worksFor> ?y .
        ?y <{UB}subOrganizationOf> <http://www.University0.edu> }}""", True),
    ("Q13", f"""SELECT ?x WHERE {{
        ?x <{RDF}type> <{UB}Person> .
        <http://www.University0.edu> <{UB}hasAlumnus> ?x }}""", True),
    ("Q14", f"""SELECT ?x WHERE {{ ?x <{RDF}type> <{UB}UndergraduateStudent> }}""", False),
]


def batch(binary: str, commands: list[str]) -> list[dict]:
    """Run a batch and return the parsed JSON line per command."""
    proc = subprocess.run(
        [binary, "batch", "-"],
        input="\n".join(commands) + "\n",
        capture_output=True,
        text=True,
    )
    out = []
    for line in proc.stdout.splitlines():
        line = line.strip()
        if line.startswith("{"):
            try:
                out.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", default="data1", help="directory of generated .owl files")
    ap.add_argument("--binary", default="../../target/release/open-ontologies")
    ap.add_argument("--ontology", default="univ-bench.owl")
    ap.add_argument("--reason", action="store_true", help="materialise before querying")
    ap.add_argument("--profile", default="owl-rl-ext", help="rdfs | owl-rl | owl-rl-ext")
    ap.add_argument("--runs", type=int, default=3, help="runs per query, best taken")
    ap.add_argument("--out", default=None, help="write results as JSON")
    args = ap.parse_args()

    files = sorted(glob.glob(str(pathlib.Path(args.data) / "*.owl")))
    if not files:
        print(f"no .owl files in {args.data}: generate with the UBA generator first")
        return 1

    print(f"LUBM harness: {len(files)} files from {args.data}")

    # Load, cold. The ontology first so its schema is present for reasoning.
    loads = ["clear", f"load {args.ontology}"] + [f"load {f}" for f in files]
    t0 = time.time()
    results = batch(args.binary, loads + ["stats"])
    load_seconds = time.time() - t0
    stats = next((r["result"] for r in results if r.get("command") == "stats"), {})
    triples = stats.get("triples", 0)
    print(f"  load: {load_seconds:.1f}s for {triples} triples "
          f"({triples / load_seconds:,.0f} triples/s)" if load_seconds else "")

    reason_seconds = None
    inferred = None
    if args.reason:
        t0 = time.time()
        r = batch(args.binary, loads + [f"reason {args.profile}", "stats"])
        reason_seconds = time.time() - t0 - load_seconds
        after = next((x["result"] for x in r if x.get("command") == "stats"), {})
        inferred = after.get("triples", 0) - triples
        print(f"  reason ({args.profile}): {reason_seconds:.1f}s, +{inferred} inferred triples")
        loads = loads + [f"reason {args.profile}"]

    rows = []
    for name, query, needs_inference in QUERIES:
        flat = " ".join(query.split())
        best = None
        count = 0
        for _ in range(args.runs):
            t0 = time.time()
            r = batch(args.binary, loads + [f"query {json.dumps(flat)}"])
            elapsed = (time.time() - t0 - load_seconds) * 1000
            res = next((x["result"] for x in r if x.get("command") == "query"), {})
            count = len(res.get("results", []))
            best = elapsed if best is None else min(best, elapsed)
        rows.append({
            "query": name,
            "results": count,
            "ms": round(max(best, 0.0), 1),
            "needs_inference": needs_inference,
        })
        flag = " (needs inference)" if needs_inference else ""
        print(f"  {name:>4}: {count:>7} results  {max(best, 0.0):>8.1f} ms{flag}")

    summary = {
        "dataset": args.data,
        "files": len(files),
        "triples": triples,
        "load_seconds": round(load_seconds, 2),
        "reasoned": bool(args.reason),
        "profile": args.profile if args.reason else None,
        "reason_seconds": round(reason_seconds, 2) if reason_seconds else None,
        "inferred_triples": inferred,
        "runs_per_query": args.runs,
        "queries": rows,
    }
    if args.out:
        pathlib.Path(args.out).write_text(json.dumps(summary, indent=2))
        print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
