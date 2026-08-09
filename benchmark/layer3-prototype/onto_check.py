"""Per-claim consistency checking as non-recursive columnar SQL.

This is the Tardygrada Layer 3 hot path. A claim is 5-50 triples. Every check
below is a flat join against tables precomputed by onto_compile.py. No
recursion, no model construction, no backtracking, so no way to hang.

Each check returns the contradicting evidence, which is exactly what an
explainable verdict needs: the layer that failed AND the axioms that clash.
"""
from __future__ import annotations

import time

import duckdb

# ── the checks ──────────────────────────────────────────────────────────
# Each is one SQL statement over (claim_type, claim_rel) plus compiled tables.

Q_DISJOINT = """
SELECT ct1.subj AS subject, ct1.cls AS class_a, ct2.cls AS class_b,
       d.a AS via_a, d.b AS via_b
FROM claim_type ct1
JOIN claim_type ct2 ON ct1.subj = ct2.subj AND ct1.cls < ct2.cls
JOIN sub_closure s1 ON s1.sub = ct1.cls
JOIN sub_closure s2 ON s2.sub = ct2.cls
JOIN disjoint_closure d ON d.a = s1.sup AND d.b = s2.sup
LIMIT 25
"""

Q_DOMAIN = """
SELECT r.subj AS subject, r.prop, dm.cls AS required_class
FROM claim_rel r
JOIN domain dm ON dm.prop = r.prop
WHERE NOT EXISTS (
  SELECT 1 FROM claim_type ct
  JOIN sub_closure sc ON sc.sub = ct.cls
  WHERE ct.subj = r.subj AND sc.sup = dm.cls
)
LIMIT 25
"""

Q_RANGE = """
SELECT r.obj AS object, r.prop, rg.cls AS required_class
FROM claim_rel r
JOIN range rg ON rg.prop = r.prop
WHERE NOT EXISTS (
  SELECT 1 FROM claim_type ct
  JOIN sub_closure sc ON sc.sub = ct.cls
  WHERE ct.subj = r.obj AND sc.sup = rg.cls
)
LIMIT 25
"""

Q_FUNCTIONAL = """
SELECT r.subj AS subject, r.prop, COUNT(DISTINCT r.obj) AS distinct_values
FROM claim_rel r
JOIN functional f ON f.prop = r.prop
GROUP BY r.subj, r.prop
HAVING COUNT(DISTINCT r.obj) > 1
LIMIT 25
"""

# Closed-world. Open-world OWL cannot flag an invented term; this can.
Q_UNKNOWN_CLASS = """
SELECT DISTINCT ct.cls AS undeclared_class
FROM claim_type ct
LEFT JOIN declared_class dc ON dc.iri = ct.cls
WHERE dc.iri IS NULL
LIMIT 25
"""

Q_UNKNOWN_PROP = """
SELECT DISTINCT r.prop AS undeclared_property
FROM claim_rel r
LEFT JOIN declared_prop dp ON dp.iri = r.prop
WHERE dp.iri IS NULL
LIMIT 25
"""

CHECKS = [
    ("disjointness", Q_DISJOINT),
    ("domain", Q_DOMAIN),
    ("range", Q_RANGE),
    ("functionality", Q_FUNCTIONAL),
    ("unknown_class", Q_UNKNOWN_CLASS),
    ("unknown_property", Q_UNKNOWN_PROP),
]


def prepare(con: duckdb.DuckDBPyConnection) -> None:
    """Create the per-claim staging tables once; reuse across claims."""
    con.execute("CREATE OR REPLACE TABLE claim_type(subj VARCHAR, cls VARCHAR)")
    con.execute("CREATE OR REPLACE TABLE claim_rel(subj VARCHAR, prop VARCHAR, obj VARCHAR)")


def check_claim(con: duckdb.DuckDBPyConnection,
                types: list[tuple[str, str]],
                rels: list[tuple[str, str, str]]) -> dict:
    """Check one candidate claim. Returns a verdict with the failing evidence."""
    t0 = time.perf_counter()
    con.execute("DELETE FROM claim_type")
    con.execute("DELETE FROM claim_rel")
    if types:
        con.executemany("INSERT INTO claim_type VALUES (?,?)", types)
    if rels:
        con.executemany("INSERT INTO claim_rel VALUES (?,?,?)", rels)

    violations = {}
    for name, sql in CHECKS:
        rows = con.execute(sql).fetchall()
        if rows:
            violations[name] = rows
    elapsed_ms = (time.perf_counter() - t0) * 1000.0

    return {
        "consistent": not violations,
        "violations": violations,
        "elapsed_ms": round(elapsed_ms, 3),
    }
