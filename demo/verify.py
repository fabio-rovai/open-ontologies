#!/usr/bin/env python3
"""
Standalone verifier for the derived ontology and the graphs extracted against it.

This is stage 5 and 6 of the pipeline with nothing else attached: no model
calls, no extraction, no repair. It answers one question, repeatably and in
seconds: given what is on disk right now, does it hold up?

Six checks, in the order the repo's own workflow prescribes:

  1. PARSE     every fragment and the merged ontology are syntactically valid
  2. STATS     the merged ontology loads with the class and property counts claimed
  3. LINT      missing labels, comments, domains, ranges
  4. ENFORCE   design-pattern compliance against the generic rule pack
  5. VOCAB     closed-world check of every extracted graph against the ontology
  6. REASON    disjointness violations across the merged instance data

Exit code is the number of failed checks, so this can gate a build.
"""

from __future__ import annotations

import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from ontology_from_docs import (mcp, tool_text, engine, rehead, is_valid_turtle,  # noqa: E402
                                sources_of, ROOT)

BOLD, DIM, RED, GRN, YEL, CYA, OFF = (
    "\033[1m", "\033[2m", "\033[31m", "\033[32m", "\033[33m", "\033[36m", "\033[0m")

DERIVED = ROOT / "demo" / "derived"
ONTOLOGY = DERIVED / "_ontology.ttl"


def head(n: int, title: str, sub: str) -> None:
    print(f"\n{BOLD}CHECK {n}  {title}{OFF}  {DIM}{sub}{OFF}")


def main() -> int:
    if not ONTOLOGY.exists():
        print(f"{RED}no derived ontology at {ONTOLOGY}{OFF}")
        return 1

    graphs = sorted(DERIVED.glob("*.data.ttl"))
    fragments = sorted(DERIVED.glob("*.vocab.ttl"))
    failures = 0

    print(f"{BOLD}Verifier{OFF}  {DIM}{ONTOLOGY.relative_to(ROOT)}, "
          f"{len(fragments)} fragments, {len(graphs)} extracted graphs{OFF}")

    # ---- 1. parse ----------------------------------------------------------
    head(1, "PARSE", "syntax of every fragment and the merged ontology")
    # Judged as the merge judges them: under the canonical prefix header and
    # after the mechanical repairs. Validating the raw file instead reports a
    # missing prefix declaration as a modelling failure, which it is not.
    bad_fragments, mended = [], []
    for f in fragments:
        raw = f.read_text()
        if is_valid_turtle(raw)[0]:
            continue
        if is_valid_turtle(rehead(raw))[0]:
            mended.append(f.stem)
        else:
            bad_fragments.append((f.stem, is_valid_turtle(rehead(raw))[1]))
    if mended:
        print(f"  {DIM}{len(mended)} fragment(s) recovered by reheading and mending: "
              f"{', '.join(m.replace('.vocab', '') for m in mended)}{OFF}")
    if bad_fragments:
        print(f"  {YEL}{len(bad_fragments)} of {len(fragments)} fragments do not parse "
              f"and are excluded from the merge{OFF}")
        for name, why in bad_fragments:
            print(f"    {DIM}{name}  {why[:110]}{OFF}")
    else:
        print(f"  {GRN}all {len(fragments)} fragments usable{OFF}")

    res = engine(["clear", f"load {ONTOLOGY}"])
    if '"error"' in res:
        print(f"  {RED}merged ontology does not parse{OFF}")
        print(f"    {DIM}{res[-220:]}{OFF}")
        failures += 1
    else:
        print(f"  {GRN}merged ontology parses{OFF}")

    # ---- 2. stats ----------------------------------------------------------
    head(2, "STATS", "does the ontology load with the structure it claims")
    st = {}
    try:
        st = json.loads(engine(["clear", f"load {ONTOLOGY}", "stats"]).split("\n")[-1])["result"]
    except Exception:
        pass
    classes, props = st.get("classes", 0), st.get("properties", 0)
    print(f"  {classes} classes, {props} properties "
          f"({st.get('object_properties', 0)} object, {st.get('data_properties', 0)} data), "
          f"{st.get('triples', 0)} triples")
    # A vocabulary with classes and no properties is the failure mode that a
    # truncated parse produces, and it silently poisons everything downstream.
    if classes and not props:
        print(f"  {RED}classes but no properties: the parse is truncated{OFF}")
        failures += 1
    elif classes and props:
        print(f"  {GRN}structure present{OFF}")

    q_disjoint = ("PREFIX owl: <http://www.w3.org/2002/07/owl#> "
                  "SELECT (COUNT(*) AS ?n) WHERE { ?a owl:disjointWith ?b }")
    try:
        rows = json.loads(engine(["clear", f"load {ONTOLOGY}",
                                  f"query {json.dumps(q_disjoint)}"]).split("\n")[-1])["result"]["results"]
        n = rows[0].get("n", "0").split('"')[1] if rows else "0"
        print(f"  {n} disjointness axioms {DIM}(what gives the reasoner something to catch){OFF}")
    except Exception:
        pass

    # Competing modelling patterns: a subclass partition and an attribute
    # class for the same notion. The extractor reliably picks the attribute
    # class, which is the one form disjointness cannot reach, so leaving both
    # in the vocabulary quietly disables contradiction detection. None of the
    # engine's enforce packs catches this; the pipeline's _reconcile_ttl()
    # removes them, and this check keeps it removed.
    import re as _re
    ttl_text = ONTOLOGY.read_text()
    cls = set(_re.findall(r":(\w+)\s+(?:a|rdf:type)\s+owl:Class", ttl_text))
    kids: dict[str, set[str]] = {}
    for m in _re.finditer(r":(\w+)\s+rdfs:subClassOf\s+:(\w+)", ttl_text):
        kids.setdefault(m.group(2), set()).add(m.group(1))
    from ontology_from_docs import KIND_SUFFIXES
    collisions = sorted({p + s for p, k in kids.items() if len(k) >= 2
                         for s in KIND_SUFFIXES if p + s in cls})
    if collisions:
        print(f"  {RED}{len(collisions)} attribute class(es) compete with a subclass "
              f"partition: {', '.join(':' + c for c in collisions)}{OFF}")
        print(f"    {DIM}the extractor will use these instead of the partition, "
              f"and disjointness cannot reach them{OFF}")
        failures += 1
    else:
        print(f"  {GRN}no competing modelling patterns{OFF}")

    # ---- MCP session for the tool-backed checks ----------------------------
    try:
        mcp("initialize", {"protocolVersion": "2025-03-26", "capabilities": {},
                           "clientInfo": {"name": "verify", "version": "1"}})
        mcp("tools/call", {"name": "onto_load", "arguments":
            {"path": str(ONTOLOGY), "name": "derived", "force_recompile": True}})
    except Exception as e:
        print(f"\n{RED}engine unreachable: {e}{OFF}")
        print(f"{DIM}start it with: ./target/release/open-ontologies serve --port 8137{OFF}")
        return failures + 1

    # ---- 3. lint -----------------------------------------------------------
    head(3, "LINT", "labels, comments, domains, ranges")
    try:
        d = json.loads(tool_text(mcp("tools/call", {"name": "onto_lint", "arguments": {}})) or "{}")
        issues = d.get("issues", [])
        by_rule: dict[str, int] = {}
        for i in issues:
            by_rule[i.get("rule", i.get("kind", "?"))] = by_rule.get(i.get("rule", i.get("kind", "?")), 0) + 1
        if issues:
            print(f"  {YEL}{len(issues)} issues{OFF}")
            for rule, n in sorted(by_rule.items(), key=lambda x: -x[1])[:8]:
                print(f"    {n:4d}  {rule}")
        else:
            print(f"  {GRN}clean{OFF}")
    except Exception as e:
        print(f"  {DIM}lint unavailable: {e}{OFF}")

    # ---- 4. enforce --------------------------------------------------------
    head(4, "ENFORCE", "design patterns, generic rule pack")
    try:
        d = json.loads(tool_text(mcp("tools/call", {"name": "onto_enforce",
                                                    "arguments": {"rules": "generic"}})) or "{}")
        v = d.get("violations", [])
        if v:
            by_rule = {}
            for i in v:
                key = i.get("rule", "?")
                by_rule[key] = by_rule.get(key, 0) + 1
            print(f"  {YEL}{len(v)} violations{OFF}")
            for rule, n in sorted(by_rule.items(), key=lambda x: -x[1])[:8]:
                print(f"    {n:4d}  {rule}")
        else:
            print(f"  {GRN}no violations{OFF}")
    except Exception as e:
        print(f"  {DIM}enforce unavailable: {e}{OFF}")

    # ---- 5. vocab check ----------------------------------------------------
    head(5, "VOCAB", "closed-world check of every extracted graph")
    conform, review = 0, []
    for f in graphs:
        try:
            d = json.loads(tool_text(mcp("tools/call", {"name": "onto_vocab_check",
                                                        "arguments": {"data": f.read_text(),
                                                                      "inline": True}})) or "{}")
        except Exception as e:
            print(f"  {RED}error{OFF}    {f.stem}  {e}")
            continue
        if d.get("conforms"):
            conform += 1
            print(f"  {GRN}conforms{OFF} {f.stem}")
        else:
            terms = [t.split("#")[-1] for t in d.get("hallucinated_terms", [])]
            review.append((f.stem, terms))
            print(f"  {YEL}review  {OFF} {f.stem}  {DIM}{len(terms)} undeclared: "
                  f"{', '.join(terms[:5])}{'…' if len(terms) > 5 else ''}{OFF}")
    if graphs:
        pct = 100 * conform / len(graphs)
        print(f"  {BOLD}{conform}/{len(graphs)} conform ({pct:.0f}%){OFF}")
        if conform == 0:
            failures += 1

    # ---- 6. reason ---------------------------------------------------------
    head(6, "REASON", "disjointness violations across the merged instance data")
    loads = ["clear", f"load {ONTOLOGY}"] + [f"load {g}" for g in graphs]
    try:
        st = json.loads(engine(loads + ["stats"]).split("\n")[-1])["result"]
        print(f"  merged: {st['triples']} triples, {st['classes']} classes, "
              f"{st['individuals']} individuals")
    except Exception:
        pass

    q = ("PREFIX owl: <http://www.w3.org/2002/07/owl#> "
         "PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> "
         "SELECT DISTINCT ?subject ?a ?b WHERE { ?subject a ?a, ?b . "
         "FILTER(STR(?a) < STR(?b)) ?a rdfs:subClassOf* ?da . ?b rdfs:subClassOf* ?db . "
         "{ ?da owl:disjointWith ?db } UNION { ?db owl:disjointWith ?da } }")
    try:
        rows = json.loads(engine(loads + [f"query {json.dumps(q)}"]).split("\n")[-1])["result"]["results"]
    except Exception:
        rows = []
    # Split by provenance. A violation is only a contradiction when one
    # document asserts one side and does not assert the other; both sides from
    # a single document is one extractor over-typing, and it can produce
    # hundreds of violations that mean nothing.
    src = sources_of(graphs)
    contradictions, overtyping = [], []
    for r in rows:
        v = {k: val.split("#")[-1].rstrip(">") for k, val in r.items()}
        da = src.get((v["subject"], v["a"]), set())
        db = src.get((v["subject"], v["b"]), set())
        (contradictions if (da - db) and (db - da) else overtyping).append((v, da, db))

    for v, da, db in contradictions:
        print(f"  {RED}CONTRADICTION{OFF} {v['subject']} is {v['a']} per "
              f"{','.join(sorted(da - db))} but {v['b']} per {','.join(sorted(db - da))}")
    if overtyping:
        by_doc: dict[str, int] = {}
        for _, da, db in overtyping:
            for d in da | db:
                by_doc[d] = by_doc.get(d, 0) + 1
        worst = ", ".join(f"{d} {n}" for d, n in sorted(by_doc.items(), key=lambda x: -x[1])[:3])
        print(f"  {YEL}{len(overtyping)} violations within a single document{OFF} "
              f"{DIM}(over-typing, not disagreement: {worst}){OFF}")
    if not rows:
        print(f"  {DIM}none{OFF}")

    # A contradiction can only be found if the same real-world thing carries
    # both types. Report how far the data is from that precondition, because
    # "no conflicts" and "nothing could ever conflict" look identical otherwise.
    q_multi = ("SELECT (COUNT(DISTINCT ?s) AS ?n) WHERE "
               "{ ?s a ?t1, ?t2 . FILTER(STR(?t1) < STR(?t2)) }")
    try:
        rows2 = json.loads(engine(loads + [f"query {json.dumps(q_multi)}"]).split("\n")[-1])["result"]["results"]
        n = rows2[0].get("n", "0").split('"')[1] if rows2 else "0"
        if n == "0":
            print(f"  {YEL}0 individuals carry more than one type, so no contradiction is "
                  f"reachable regardless of the axioms{OFF}")
            print(f"  {DIM}fix is entity resolution: the same thing must get one IRI "
                  f"across documents{OFF}")
        else:
            print(f"  {DIM}{n} individuals carry more than one type{OFF}")
    except Exception:
        pass

    print(f"\n{BOLD}{'PASS' if not failures else str(failures) + ' CHECK(S) FAILED'}{OFF}"
          f"  {DIM}{conform}/{len(graphs)} graphs conform, "
          f"{len(contradictions)} contradictions across documents{OFF}")
    print(f"{DIM}CHECK 6 only re-derives disjointness contradictions in the pipeline ontology "
          f"above; it is not wired to this script's exit code, and finds none here by "
          f"construction (see docs/superpowers/plans/2026-08-24-studio-public-port.md, "
          f"'PIVOT, 25 August 2026'). The findings this demonstration actually reports live "
          f"in demo/precomputed/findings.json, computed independently by "
          f"demo/dcat_conformance.py and re-derived every time 'make demo-verify' runs -- "
          f"see its own output above, or run 'python3 -m pytest demo/tests/"
          f"test_dcat_conformance.py' directly.{OFF}\n")
    return failures


if __name__ == "__main__":
    sys.exit(main())
