"""Should the compiled lookups live in Oxigraph (already a dependency) or DuckDB?

Fair test: identical compiled data, identical question, two substrates.

  Oxigraph  the incompatible-pair table as RDF triples, queried with SPARQL ASK
  DuckDB    the same table as a relation, queried with SQL

This matters because Oxigraph is ALREADY a dependency of open-ontologies.
If it is fast enough, DuckDB is an avoidable dependency, and "one store" is a
better engineering story than two.
"""
from __future__ import annotations

import statistics
import time

import duckdb
import pyoxigraph as ox

INCOMPAT = "http://tardygrada.example/incompatibleWith"


def load_pairs(csv_path: str) -> list[tuple[str, str]]:
    con = duckdb.connect()
    rows = con.execute(
        f"SELECT a, b FROM read_csv_auto('{csv_path}')"
    ).fetchall()
    con.close()
    return rows


def bench_duckdb(pairs, probes, reps):
    con = duckdb.connect()
    con.execute("CREATE TABLE incompatible(a VARCHAR, b VARCHAR)")
    con.executemany("INSERT INTO incompatible VALUES (?,?)", pairs)
    con.execute("CREATE INDEX ix ON incompatible(a)")

    lat = []
    for i in range(reps):
        a, b = probes[i % len(probes)]
        t0 = time.perf_counter()
        con.execute(
            "SELECT 1 FROM incompatible WHERE a=? AND b=? LIMIT 1", [a, b]
        ).fetchone()
        lat.append((time.perf_counter() - t0) * 1000)
    con.close()
    return lat


def bench_oxigraph(pairs, probes, reps):
    store = ox.Store()
    pred = ox.NamedNode(INCOMPAT)
    quads = [
        ox.Quad(ox.NamedNode(a), pred, ox.NamedNode(b))
        for a, b in pairs
        if a.startswith("http") and b.startswith("http")
    ]
    store.extend(quads)

    lat = []
    for i in range(reps):
        a, b = probes[i % len(probes)]
        q = f"ASK {{ <{a}> <{INCOMPAT}> <{b}> }}"
        t0 = time.perf_counter()
        store.query(q)
        lat.append((time.perf_counter() - t0) * 1000)
    return lat, len(quads)


def summarise(name, lat):
    lat = sorted(lat)
    return (f"  {name:10s} median {lat[len(lat)//2]:7.4f} ms | "
            f"p95 {lat[int(len(lat)*0.95)]:7.4f} ms | "
            f"mean {statistics.mean(lat):7.4f} ms")


if __name__ == "__main__":
    import sys

    csv_path = sys.argv[1] if len(sys.argv) > 1 else "/tmp/pizza_disjoint.csv"
    reps = int(sys.argv[2]) if len(sys.argv) > 2 else 3000

    pairs = load_pairs(csv_path)
    print(f"compiled pairs: {len(pairs):,}")

    # Probe with a mix of hits and misses, which is what a real stream does.
    hits = pairs[:50]
    misses = [(a, a) for a, _ in pairs[:50]]
    probes = [p for pair in zip(hits, misses) for p in pair]

    d = bench_duckdb(pairs, probes, reps)
    o, nquads = bench_oxigraph(pairs, probes, reps)
    print(f"oxigraph quads: {nquads:,}\n")
    print(summarise("DuckDB", d))
    print(summarise("Oxigraph", o))
    dm = statistics.mean(d)
    om = statistics.mean(o)
    faster = "Oxigraph" if om < dm else "DuckDB"
    print(f"\n  {faster} is {max(dm, om)/min(dm, om):.1f}x faster on mean latency")
