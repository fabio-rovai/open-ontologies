#!/usr/bin/env python3
"""Warm-store query latency and concurrent throughput over HTTP.

The batch harness reloads the dataset for every query because the CLI is
stateless, so its per-query numbers are ~200 ms of setup wrapped around the
query. That is useless for comparison against a server product, which is
always measured warm. This script loads once, then measures.

Two things, both the way a store vendor would report them:

  - LATENCY: each of the 14 LUBM queries, warmed then timed over N runs,
    reported as median and p95 rather than a mean, because the tail is what
    a caller actually feels;
  - THROUGHPUT: the same queries issued by C concurrent clients for a fixed
    duration, reported as queries per second.

Usage:
    python3 query_latency.py --data data1 --profile owl-rl-ext
    python3 query_latency.py --data data10 --runs 50 --clients 8
"""

from __future__ import annotations

import argparse
import glob
import json
import pathlib
import statistics
import subprocess
import sys
import threading
import time
import urllib.request

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from run_lubm import QUERIES  # noqa: E402


class Engine:
    """One MCP session against a running server."""

    def __init__(self, url: str):
        self.url = url
        self.session: str | None = None
        self._id = 0
        self.initialize()

    def rpc(self, method: str, params: dict) -> dict:
        self._id += 1
        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream",
        }
        if self.session:
            headers["Mcp-Session-Id"] = self.session
        request = urllib.request.Request(
            self.url,
            headers=headers,
            data=json.dumps(
                {"jsonrpc": "2.0", "id": self._id, "method": method, "params": params}
            ).encode(),
        )
        with urllib.request.urlopen(request, timeout=600) as response:
            self.session = response.headers.get("Mcp-Session-Id") or self.session
            body = response.read().decode().strip()
        if body.startswith("{"):
            return json.loads(body)
        for line in body.split("\n"):
            if line.startswith("data:"):
                try:
                    return json.loads(line[5:].strip())
                except json.JSONDecodeError:
                    pass
        return {}

    def initialize(self) -> None:
        self.rpc(
            "initialize",
            {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "lubm-latency", "version": "1"},
            },
        )

    def tool(self, name: str, args: dict) -> str:
        result = self.rpc("tools/call", {"name": name, "arguments": args})
        content = result.get("result", {}).get("content", [{}])
        return content[0].get("text", "") if content else ""

    def query(self, sparql: str) -> int:
        raw = self.tool("onto_query", {"query": sparql})
        try:
            return len(json.loads(raw).get("results", []))
        except json.JSONDecodeError:
            return -1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", default="data1")
    ap.add_argument("--binary", default="../../target/release/open-ontologies")
    ap.add_argument("--ontology", default="univ-bench.owl")
    ap.add_argument("--port", type=int, default=8399)
    ap.add_argument("--profile", default="owl-rl-ext")
    ap.add_argument("--runs", type=int, default=25, help="timed runs per query")
    ap.add_argument("--warmup", type=int, default=3)
    ap.add_argument("--clients", type=int, default=8, help="concurrent clients")
    ap.add_argument("--seconds", type=float, default=5.0, help="throughput duration")
    ap.add_argument("--out", default=None)
    args = ap.parse_args()

    files = sorted(glob.glob(str(pathlib.Path(args.data) / "*.owl")))
    if not files:
        print(f"no .owl files in {args.data}")
        return 1

    server = subprocess.Popen(
        [args.binary, "serve-http", "--port", str(args.port)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    url = f"http://127.0.0.1:{args.port}/mcp"
    try:
        for _ in range(40):
            time.sleep(0.5)
            try:
                Engine(url)
                break
            except Exception:
                continue
        else:
            print("server did not start")
            return 1

        # onto_load REPLACES the active store rather than appending, so the
        # dataset is merged into one file first. Measuring against a store
        # holding only the last file read is the sort of mistake that makes a
        # benchmark look fast and mean nothing.
        merged = pathlib.Path(args.data).with_suffix(".merged.ttl")
        if not merged.exists():
            print(f"merging {len(files)} files into {merged}")
            cmds = ["clear", f"load {args.ontology}"] + [f"load {f}" for f in files]
            # `save` writes Turtle whatever format is asked for, so the
            # extension must agree or format detection rejects the file.
            cmds.append(f"save {merged}")
            subprocess.run([args.binary, "batch", "-"], input="\n".join(cmds) + "\n",
                           capture_output=True, text=True)

        loader = Engine(url)
        print(f"loading the warm store from {merged}")
        t0 = time.time()
        loader.tool("onto_load", {"path": str(merged), "name": "lubm", "force_recompile": True})
        load_seconds = time.time() - t0
        stats = json.loads(loader.tool("onto_stats", {}) or "{}")
        print(f"  loaded {stats.get('triples', 0):,} triples in {load_seconds:.1f}s")

        t0 = time.time()
        loader.tool("onto_reason", {"profile": args.profile})
        reason_seconds = time.time() - t0
        after = json.loads(loader.tool("onto_stats", {}) or "{}")
        print(f"  {args.profile}: {reason_seconds:.1f}s, "
              f"{after.get('triples', 0):,} triples total")

        # ── latency ────────────────────────────────────────────────────────
        print(f"\nlatency, warm store, {args.runs} runs per query")
        rows = []
        for name, query, _ in QUERIES:
            flat = " ".join(query.split())
            for _ in range(args.warmup):
                loader.query(flat)
            samples = []
            count = 0
            for _ in range(args.runs):
                t0 = time.perf_counter()
                count = loader.query(flat)
                samples.append((time.perf_counter() - t0) * 1000)
            samples.sort()
            median = statistics.median(samples)
            p95 = samples[min(len(samples) - 1, int(len(samples) * 0.95))]
            rows.append({
                "query": name, "results": count,
                "median_ms": round(median, 2), "p95_ms": round(p95, 2),
            })
            print(f"  {name:>4}: {count:>7} results   median {median:7.2f} ms   p95 {p95:7.2f} ms")

        # ── throughput ─────────────────────────────────────────────────────
        print(f"\nthroughput, {args.clients} concurrent clients, {args.seconds}s")
        flats = [" ".join(q.split()) for _, q, _ in QUERIES]
        counts = [0] * args.clients
        stop = time.time() + args.seconds

        def worker(slot: int) -> None:
            try:
                engine = Engine(url)
            except Exception:
                return
            i = slot
            while time.time() < stop:
                engine.query(flats[i % len(flats)])
                counts[slot] += 1
                i += 1

        threads = [threading.Thread(target=worker, args=(i,)) for i in range(args.clients)]
        t0 = time.time()
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        elapsed = time.time() - t0
        total = sum(counts)
        qps = total / elapsed
        print(f"  {total} queries in {elapsed:.1f}s = {qps:,.0f} queries/s")

        summary = {
            "dataset": args.data,
            "triples": stats.get("triples", 0),
            "triples_after_reasoning": after.get("triples", 0),
            "load_seconds": round(load_seconds, 2),
            "reason_seconds": round(reason_seconds, 2),
            "profile": args.profile,
            "runs_per_query": args.runs,
            "latency": rows,
            "throughput": {
                "clients": args.clients,
                "seconds": round(elapsed, 2),
                "queries": total,
                "qps": round(qps, 1),
            },
        }
        if args.out:
            pathlib.Path(args.out).write_text(json.dumps(summary, indent=2))
            print(f"\nwrote {args.out}")
        return 0
    finally:
        server.terminate()
        server.wait(timeout=10)


if __name__ == "__main__":
    raise SystemExit(main())
