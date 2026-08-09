"""Compile an OWL ontology into flat DuckDB tables — the Kalgera pattern.

The ontology stays the SPECIFICATION. Execution is native columnar SQL.

Key move: all the recursion (subsumption closure, disjointness propagation)
happens ONCE, offline, here. The runtime hot path is then non-recursive
analytic SQL over flat tables, which is what columnar engines are good at.
This sidesteps the usual objection that recursive CTEs in a general SQL engine
are slower than a purpose-built Datalog engine: at query time there is no
recursion left to do.

Emitted tables:
  sub_closure(sub, sup)        transitive reflexive subclass closure
  disjoint_closure(a, b)       disjointness propagated DOWN both hierarchies
  domain(prop, cls)            rdfs:domain
  range(prop, cls)             rdfs:range
  functional(prop)             owl:FunctionalProperty
  declared_class(iri)          closed-world vocabulary
  declared_prop(iri)
"""
from __future__ import annotations

import sys
import time

import duckdb
from rdflib import OWL, RDF, RDFS, Graph, URIRef


def compile_ontology(path: str, db_path: str = ":memory:") -> tuple[duckdb.DuckDBPyConnection, dict]:
    t0 = time.perf_counter()
    g = Graph()
    g.parse(path)
    parse_s = time.perf_counter() - t0

    t0 = time.perf_counter()

    # ── direct subclass edges between NAMED classes ─────────────────────
    named = {s for s in g.subjects(RDF.type, OWL.Class) if isinstance(s, URIRef)}
    direct: dict[URIRef, set[URIRef]] = {c: set() for c in named}
    for s, o in g.subject_objects(RDFS.subClassOf):
        if isinstance(s, URIRef) and isinstance(o, URIRef):
            direct.setdefault(s, set()).add(o)
            named.add(s)
            named.add(o)
    # equivalentClass gives subsumption in both directions
    for s, o in g.subject_objects(OWL.equivalentClass):
        if isinstance(s, URIRef) and isinstance(o, URIRef):
            direct.setdefault(s, set()).add(o)
            direct.setdefault(o, set()).add(s)
            named.update({s, o})

    # ── transitive reflexive closure, computed ONCE ─────────────────────
    closure: dict[URIRef, set[URIRef]] = {}
    for c in named:
        seen = {c}
        stack = [c]
        while stack:
            cur = stack.pop()
            for sup in direct.get(cur, ()):
                if sup not in seen:
                    seen.add(sup)
                    stack.append(sup)
        closure[c] = seen

    # ── disjointness, propagated DOWN both sides ────────────────────────
    # If A disjoint B, then every subclass of A is disjoint from every
    # subclass of B. Doing this once turns a reasoning step into a lookup.
    sub_of: dict[URIRef, set[URIRef]] = {c: set() for c in named}
    for c, sups in closure.items():
        for s in sups:
            sub_of.setdefault(s, set()).add(c)

    disj: set[tuple[str, str]] = set()
    for a, b in g.subject_objects(OWL.disjointWith):
        if not (isinstance(a, URIRef) and isinstance(b, URIRef)):
            continue
        for x in sub_of.get(a, {a}) | {a}:
            for y in sub_of.get(b, {b}) | {b}:
                disj.add((str(x), str(y)))
                disj.add((str(y), str(x)))
    # owl:AllDisjointClasses
    for adc in g.subjects(RDF.type, OWL.AllDisjointClasses):
        members = []
        for lst in g.objects(adc, OWL.members):
            members = [m for m in g.items(lst) if isinstance(m, URIRef)]
        for i, a in enumerate(members):
            for b in members[i + 1:]:
                for x in sub_of.get(a, {a}) | {a}:
                    for y in sub_of.get(b, {b}) | {b}:
                        disj.add((str(x), str(y)))
                        disj.add((str(y), str(x)))

    domains = [(str(p), str(c)) for p, c in g.subject_objects(RDFS.domain)
               if isinstance(p, URIRef) and isinstance(c, URIRef)]
    ranges = [(str(p), str(c)) for p, c in g.subject_objects(RDFS.range)
              if isinstance(p, URIRef) and isinstance(c, URIRef)]
    funcs = [(str(p),) for p in g.subjects(RDF.type, OWL.FunctionalProperty)
             if isinstance(p, URIRef)]
    props = {str(p) for p in g.subjects(RDF.type, OWL.ObjectProperty) if isinstance(p, URIRef)}
    props |= {str(p) for p in g.subjects(RDF.type, OWL.DatatypeProperty) if isinstance(p, URIRef)}

    compile_s = time.perf_counter() - t0

    # ── load into DuckDB ────────────────────────────────────────────────
    t0 = time.perf_counter()
    con = duckdb.connect(db_path)
    con.execute("CREATE TABLE sub_closure(sub VARCHAR, sup VARCHAR)")
    con.executemany("INSERT INTO sub_closure VALUES (?,?)",
                    [(str(c), str(s)) for c, sups in closure.items() for s in sups])
    con.execute("CREATE TABLE disjoint_closure(a VARCHAR, b VARCHAR)")
    if disj:
        con.executemany("INSERT INTO disjoint_closure VALUES (?,?)", sorted(disj))
    con.execute("CREATE TABLE domain(prop VARCHAR, cls VARCHAR)")
    if domains:
        con.executemany("INSERT INTO domain VALUES (?,?)", domains)
    con.execute("CREATE TABLE range(prop VARCHAR, cls VARCHAR)")
    if ranges:
        con.executemany("INSERT INTO range VALUES (?,?)", ranges)
    con.execute("CREATE TABLE functional(prop VARCHAR)")
    if funcs:
        con.executemany("INSERT INTO functional VALUES (?)", funcs)
    con.execute("CREATE TABLE declared_class(iri VARCHAR)")
    con.executemany("INSERT INTO declared_class VALUES (?)", [(str(c),) for c in named])
    con.execute("CREATE TABLE declared_prop(iri VARCHAR)")
    con.executemany("INSERT INTO declared_prop VALUES (?)", [(p,) for p in sorted(props)])

    # Indexes: this is what makes the per-claim path a lookup, not a scan.
    con.execute("CREATE INDEX i_sub ON sub_closure(sub)")
    con.execute("CREATE INDEX i_disj ON disjoint_closure(a)")
    con.execute("CREATE INDEX i_dom ON domain(prop)")
    con.execute("CREATE INDEX i_rng ON range(prop)")
    load_s = time.perf_counter() - t0

    stats = {
        "parse_s": round(parse_s, 3),
        "compile_s": round(compile_s, 3),
        "load_s": round(load_s, 3),
        "named_classes": len(named),
        "sub_closure_rows": sum(len(v) for v in closure.values()),
        "disjoint_rows": len(disj),
        "domain_rows": len(domains),
        "range_rows": len(ranges),
        "functional_props": len(funcs),
    }
    return con, stats


if __name__ == "__main__":
    con, stats = compile_ontology(sys.argv[1])
    for k, v in stats.items():
        print(f"{k:20s} {v}")
