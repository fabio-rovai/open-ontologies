"""Lazy compatibility matrix: compute pairs on demand, cache forever.

Removes the O(n^2) offline compile. Instead of precomputing every pair, we
consult a DuckDB cache and only ask a warm reasoner about pairs we have never
seen. Claim streams touch a small working set of classes, so the cache
saturates quickly and steady-state cost collapses to a lookup.

Cache is persistent, so the cost is paid once across the lifetime of a
deployment rather than once per process.
"""
from __future__ import annotations

import json
import subprocess
from pathlib import Path

import duckdb

REASONER_DIR = Path(__file__).resolve().parent.parent / "reasoner"
JAVA = "/opt/homebrew/opt/openjdk@17/bin/java"
CP = f".:{REASONER_DIR}/lib/*"


class LazyMatrix:
    """On-demand class-compatibility oracle with a persistent DuckDB cache."""

    def __init__(self, ontology: str, cache_path: str = ":memory:"):
        self.ontology = ontology
        self.con = duckdb.connect(cache_path)
        self.con.execute(
            "CREATE TABLE IF NOT EXISTS pair_cache("
            "  a VARCHAR, b VARCHAR, compatible BOOLEAN, PRIMARY KEY (a, b))"
        )
        self.proc = None
        self.warmup_ms = None
        self.oracle_calls = 0
        self.cache_hits = 0
        self.oracle_ms_total = 0.0

    # ── oracle lifecycle ────────────────────────────────────────────────
    def _ensure_oracle(self) -> None:
        """Start the reasoner only when a cache miss actually needs it."""
        if self.proc is not None:
            return
        self.proc = subprocess.Popen(
            [JAVA, "-cp", CP, "PairOracle", self.ontology],
            cwd=REASONER_DIR, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True, bufsize=1,
        )
        while True:
            line = self.proc.stdout.readline()
            if not line:
                raise RuntimeError("oracle failed to start")
            if line.startswith("{"):
                self.warmup_ms = json.loads(line)["warmup_ms"]
                return

    def close(self) -> None:
        if self.proc is not None:
            self.proc.stdin.close()
            self.proc.wait(timeout=30)
            self.proc = None

    # ── the lookup ──────────────────────────────────────────────────────
    def compatible(self, a: str, b: str) -> bool:
        """True if A ⊓ B is satisfiable. Cached after the first ask."""
        if a > b:
            a, b = b, a  # canonical order halves the cache

        hit = self.con.execute(
            "SELECT compatible FROM pair_cache WHERE a=? AND b=?", [a, b]
        ).fetchone()
        if hit is not None:
            self.cache_hits += 1
            return hit[0]

        self._ensure_oracle()
        self.proc.stdin.write(f"{a}\t{b}\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        d = json.loads(line)
        self.oracle_calls += 1
        self.oracle_ms_total += d["ms"]

        self.con.execute(
            "INSERT OR REPLACE INTO pair_cache VALUES (?,?,?)", [a, b, d["compatible"]]
        )
        return d["compatible"]

    def check_claim(self, types: list[tuple[str, str]]) -> tuple[bool, list]:
        """Check one claim. Returns (consistent, offending_pairs)."""
        by_subject: dict[str, list[str]] = {}
        for subj, cls in types:
            by_subject.setdefault(subj, []).append(cls)

        offending = []
        for subj, classes in by_subject.items():
            for i, a in enumerate(classes):
                for b in classes[i + 1:]:
                    if not self.compatible(a, b):
                        offending.append((subj, a, b))
        return (not offending), offending

    @property
    def stats(self) -> dict:
        total = self.cache_hits + self.oracle_calls
        return {
            "warmup_ms": self.warmup_ms,
            "cache_hits": self.cache_hits,
            "oracle_calls": self.oracle_calls,
            "hit_rate": round(self.cache_hits / total, 4) if total else 0.0,
            "oracle_ms_total": round(self.oracle_ms_total, 1),
            "oracle_ms_mean": round(self.oracle_ms_total / self.oracle_calls, 3)
            if self.oracle_calls else 0.0,
            "cached_pairs": self.con.execute(
                "SELECT count(*) FROM pair_cache").fetchone()[0],
        }
