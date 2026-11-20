#!/usr/bin/env python3
"""Build the replay's 3D graph (demo/precomputed/graph.json) from a certified run.

The Studio's 3D legend says what every edge on screen is: ASSERTED, read from
ies-core.ttl; CERTIFIED, derived and then proved under OOCert.certificate_sound;
REJECTED, forged, with the checker exiting 1 and naming the rule. Its footer
says Lean 4 decides and Isabelle/HOL checks the same bytes independently. Until
this script, the replay drew 28 DCAT-US classes with no warrant on any edge, so
none of that was true of the picture it was printed over (#425).

This script makes each line true by running it:

1. The engine loads ies-core.ttl and writes two certificates of one store:
   `reason --profile rdfs --certificate` (asserted.tsv + derivations.tsv, the
   format `oo-cert` reads) and `reason --rules <built-in table> --certificate`
   (rules.tsv + asserted.tsv + horn.tsv, the format `oo-horn` and the Isabelle
   checker read).
2. `oo-cert` must accept the first and name `OOCert.certificate_sound`.
   `oo-horn` must accept the second with the verdict `entailed`, which it
   prints only for the built-in table, and the Isabelle checker must accept
   the same three files with the same verdict.
3. One derived subClassOf line is copied with its conclusion changed to a
   class the closure does not reach, and appended to each certificate. All
   three checkers must refuse the forged copies, and `oo-cert` must exit 1 and
   name the rule of the forged line.
4. The graph is drawn from that run: the asserted linking edges of ies-core
   (subClassOf, rdf:type, rdfs:domain, rdfs:range, as docs/assets/
   knowledge-graph.py draws them), the derived subClassOf edges BOTH
   certificates conclude as CERTIFIED, and the forged edge as REJECTED, beside
   the four parts of the verification layer the run used.

Anything that does not come out as stated stops the script before graph.json is
written, so a regeneration cannot publish a warrant nobody checked. The
evidence block keeps only what a rerun reproduces: verdicts, counts, exit codes
and the digests of the certificate files, never a machine path or the digest of
a locally built checker.

Run (builds are the ones CI makes; paths can be overridden):
    cargo build --release
    (cd lean && lake build oo-cert oo-horn)
    isabelle/build_native.sh
    python3 demo/build_certified_graph.py --out demo/precomputed
then rebuild bundle.json and MANIFEST.sha256 as `make demo-graph` does.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ONTOLOGY = Path("benchmark/reference/ies-core.ttl")
BUILTIN_RULES = Path("tests/fixtures/horn/builtin_rules.tsv")

RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
RDFS = "http://www.w3.org/2000/01/rdf-schema#"
OWL = "http://www.w3.org/2002/07/owl#"
XSD = "http://www.w3.org/2001/XMLSchema#"
SUBCLASS = f"<{RDFS}subClassOf>"
TYPE = f"<{RDF}type>"
DOMAIN = f"<{RDFS}domain>"
RANGE = f"<{RDFS}range>"
ONTOLOGY_CLASS = f"<{OWL}Ontology>"
# The four predicates that make ies-core one graph (docs/assets/knowledge-graph.py
# measured it: subClassOf alone leaves six pieces).
LINKING = (SUBCLASS, TYPE, DOMAIN, RANGE)
PREFIXES = {RDF: "rdf", RDFS: "rdfs", OWL: "owl", XSD: "xsd"}

# The verification layer, by the names Graph3D pins (studio/src/lib/graph3d-model.ts).
TOOL_IRI = "urn:open-ontologies:replay:"
FILE_NODE = "ies-core.ttl"
CERT_NODE = "certificate"
LEAN_NODE = "Lean 4 · oo-cert"
ISABELLE_NODE = "Isabelle/HOL"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tsv(path: Path) -> list[list[str]]:
    return [line.rstrip("\n").split("\t") for line in path.read_text(encoding="utf-8").splitlines() if line]


def run(cmd: list[str], *, stdin: str | None = None, env: dict | None = None) -> tuple[int, str]:
    proc = subprocess.run(cmd, input=stdin, capture_output=True, text=True, env=env, cwd=ROOT)
    return proc.returncode, proc.stdout


def last_json(text: str) -> dict:
    for line in reversed(text.splitlines()):
        line = line.strip()
        if line.startswith("{"):
            return json.loads(line)
    raise SystemExit(f"no JSON report in checker output:\n{text}")


def require(cond: bool, what: str) -> None:
    if not cond:
        raise SystemExit(f"refusing to write graph.json: {what}")


def horn_conclusion(fields: list[str]) -> tuple[str, str, str]:
    """`ruleIndex TAB bindCount TAB (var TAB term)* TAB cs TAB cp TAB co TAB premises...`"""
    n = int(fields[1])
    s, p, o = fields[2 + 2 * n: 5 + 2 * n]
    return s, p, o


def forge_horn(fields: list[str], new_object: str) -> list[str]:
    n = int(fields[1])
    out = list(fields)
    out[4 + 2 * n] = new_object
    return out


def label(iri: str) -> str | None:
    """A prefixed name for the W3C vocabulary, so rdfs:Class and owl:Class are
    two nodes with two names and not two nodes both called `Class`."""
    for ns, prefix in PREFIXES.items():
        if iri.startswith(ns):
            return f"{prefix}:{iri[len(ns):]}"
    return None


def choose_forgery(certified: list[tuple[str, str]], asserted_sc: set, classes: list[str]):
    """The first certified edge (s, o), in sorted order, and the first class X
    that s does not reach: not s, not asserted, not derived. Deterministic, so
    a rerun forges the same line."""
    reach: dict[str, set[str]] = {}
    for s, o in list(asserted_sc) + certified:
        reach.setdefault(s, set()).add(o)
    for s, o in certified:
        for x in classes:
            if x != s and x not in reach.get(s, set()):
                return (s, o), x
    raise SystemExit("no class to forge a subClassOf edge towards")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--out", type=Path, default=ROOT / "demo" / "precomputed")
    ap.add_argument("--engine", type=Path,
                    default=Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "release" / "open-ontologies")
    ap.add_argument("--oo-cert", type=Path, default=ROOT / "lean" / ".lake" / "build" / "bin" / "oo-cert")
    ap.add_argument("--oo-horn", type=Path, default=ROOT / "lean" / ".lake" / "build" / "bin" / "oo-horn")
    ap.add_argument("--isabelle", type=Path, default=ROOT / "isabelle" / "build" / "oo-horn-isabelle")
    args = ap.parse_args()
    for tool in (args.engine, args.oo_cert, args.oo_horn, args.isabelle):
        require(tool.is_file() and os.access(tool, os.X_OK), f"{tool} is not an executable")

    work = Path(tempfile.mkdtemp(prefix="oo-replay-graph-"))
    try:
        env = dict(os.environ, HOME=str(work / "home"))
        b_dir, h_dir = work / "builtin", work / "horn"

        # 1. Two certificates of one file, each from a fresh store.
        for data, script in (
            ("store-builtin", f"load {ONTOLOGY}\nreason --profile rdfs --certificate {b_dir}\n"),
            ("store-horn", f"load {ONTOLOGY}\nreason --rules {BUILTIN_RULES} --certificate {h_dir}\n"),
        ):
            code, out = run([str(args.engine), "--data-dir", str(work / data), "batch", "-", "--no-connect"],
                            stdin=script, env=env)
            require(code == 0, f"the engine run failed ({code}):\n{out}")
        asserted = tsv(b_dir / "asserted.tsv")
        require((b_dir / "asserted.tsv").read_bytes() == (h_dir / "asserted.tsv").read_bytes(),
                "the two certificates do not start from the same asserted triples")
        derivations = tsv(b_dir / "derivations.tsv")
        horn = tsv(h_dir / "horn.tsv")

        # 2. Every checker accepts the honest certificates.
        code, out = run([str(args.oo_cert), str(b_dir / "asserted.tsv"), str(b_dir / "derivations.tsv")])
        cert = last_json(out)
        require(code == 0 and cert.get("ok") is True and cert.get("theorem") == "OOCert.certificate_sound",
                f"oo-cert did not accept the built-in certificate: {out}")
        horn_args = [str(h_dir / "rules.tsv"), str(h_dir / "asserted.tsv"), str(h_dir / "horn.tsv")]
        code, out = run([str(args.oo_horn), "check", *horn_args])
        lean_horn = last_json(out)
        require(code == 0 and lean_horn.get("verdict") == "entailed"
                and lean_horn.get("rules_digest") == lean_horn.get("builtin_rules_digest"),
                f"oo-horn did not return `entailed` for the built-in table: {out}")
        code, out = run([str(args.isabelle), "check", *horn_args])
        isa = last_json(out)
        require(code == 0 and isa.get("ok") is True and isa.get("verdict") == "entailed",
                f"the Isabelle checker did not accept the same bytes: {out}")

        # 3. The graph's edges, from the run.
        header = {r[0] for r in asserted if r[1] == TYPE and r[2] == ONTOLOGY_CLASS}
        header |= {r[2] for r in asserted if r[0] in header}
        a_edges = sorted({(r[0], r[2]) for r in asserted
                          if r[1] in LINKING and r[2].startswith("<")
                          and r[0] not in header and r[2] not in header})
        asserted_sc = {(r[0], r[2]) for r in asserted if r[1] == SUBCLASS}
        b_sc = {(r[1], r[3]) for r in derivations if r[2] == SUBCLASS}
        h_sc = {(s, o) for s, p, o in map(horn_conclusion, horn) if p == SUBCLASS}
        require(b_sc and b_sc == h_sc,
                f"the two certificates conclude different subClassOf edges "
                f"({len(b_sc - h_sc)} only in derivations.tsv, {len(h_sc - b_sc)} only in horn.tsv)")
        certified = sorted(b_sc - set(a_edges))

        classes_drawn = sorted({n for e in a_edges + certified for n in e})
        sc_terms = sorted({n for e in list(asserted_sc) + certified for n in e if n not in header})
        (fs, fo), forged_to = choose_forgery(certified, asserted_sc, sc_terms)

        # 4. Forge one line in each format and require every checker to refuse it.
        b_line = next(r for r in derivations if r[1] == fs and r[2] == SUBCLASS and r[3] == fo)
        forged_b = [b_line[0], fs, SUBCLASS, forged_to, *b_line[4:]]
        h_line = next(f for f in horn if horn_conclusion(f) == (fs, SUBCLASS, fo))
        forged_h = forge_horn(h_line, forged_to)
        fb, fh = work / "forged-builtin", work / "forged-horn"
        shutil.copytree(b_dir, fb)
        shutil.copytree(h_dir, fh)
        with open(fb / "derivations.tsv", "a", encoding="utf-8") as f:
            f.write("\t".join(forged_b) + "\n")
        with open(fh / "horn.tsv", "a", encoding="utf-8") as f:
            f.write("\t".join(forged_h) + "\n")

        code_c, out = run([str(args.oo_cert), str(fb / "asserted.tsv"), str(fb / "derivations.tsv")])
        refused = last_json(out)
        require(code_c == 1 and refused.get("ok") is False and refused.get("rule") == b_line[0]
                and refused.get("first_rejected") == len(derivations),
                f"oo-cert did not refuse the forged line and name its rule: {out}")
        forged_horn_args = [str(fh / "rules.tsv"), str(fh / "asserted.tsv"), str(fh / "horn.tsv")]
        code_h, out = run([str(args.oo_horn), "check", *forged_horn_args])
        require(code_h == 1 and last_json(out).get("ok") is False, f"oo-horn accepted the forged line: {out}")
        code_i, out = run([str(args.isabelle), "check", *forged_horn_args])
        isa_refused = last_json(out)
        require(code_i == 1 and isa_refused.get("ok") is False,
                f"the Isabelle checker accepted the forged line: {out}")

        # 5. The graph, and the evidence for every warrant on it.
        def node(iri_term: str) -> dict:
            iri = iri_term.strip("<>")
            name = label(iri)
            return {"iri": iri, "label": name} if name else {"iri": iri}

        tools = {name: f"{TOOL_IRI}{name}" for name in (FILE_NODE, CERT_NODE, LEAN_NODE, ISABELLE_NODE)}
        classes = [node(t) for t in classes_drawn] + [{"iri": iri, "label": name} for name, iri in tools.items()]
        edges = [{"source": s.strip("<>"), "target": o.strip("<>"), "warrant": "asserted"} for s, o in a_edges]
        edges += [{"source": s.strip("<>"), "target": o.strip("<>"), "warrant": "certified"} for s, o in certified]
        edges.append({"source": fs.strip("<>"), "target": forged_to.strip("<>"), "warrant": "rejected"})
        edges += [
            {"source": tools[FILE_NODE], "target": tools[CERT_NODE], "warrant": "asserted"},
            {"source": tools[CERT_NODE], "target": tools[LEAN_NODE], "warrant": "certified"},
            {"source": tools[CERT_NODE], "target": tools[ISABELLE_NODE], "warrant": "certified"},
        ]

        by_rule: dict[str, int] = {}
        for r in derivations:
            by_rule[r[0]] = by_rule.get(r[0], 0) + 1
        graph = {
            "classes": classes,
            "properties": [],
            "edges": edges,
            "source": {
                "file": ONTOLOGY.name,
                "path": str(ONTOLOGY),
                "sha256": sha256(ROOT / ONTOLOGY),
                "triples": len(asserted),
            },
            "evidence": {
                "generator": "demo/build_certified_graph.py",
                "drawn": {
                    "asserted": len(a_edges),
                    "certified": len(certified),
                    "rejected": 1,
                    "predicates": ["rdfs:subClassOf", "rdf:type", "rdfs:domain", "rdfs:range"],
                },
                "builtin_certificate": {
                    "command": "reason --profile rdfs --certificate",
                    "asserted": len(asserted),
                    "derivations": len(derivations),
                    "by_rule": dict(sorted(by_rule.items())),
                    "asserted_sha256": sha256(b_dir / "asserted.tsv"),
                    # Not the digest of derivations.tsv itself: the engine
                    # records a different premise for some rdfs11 steps, in a
                    # different order, on every run (#427). The conclusions
                    # are the same set every time, so this digests those.
                    "conclusions_sha256": hashlib.sha256("".join(
                        sorted("\t".join(r[:4]) + "\n" for r in derivations)).encode()).hexdigest(),
                    "oo_cert": {"exit": 0, "ok": True, "theorem": cert["theorem"]},
                },
                "horn_certificate": {
                    "command": f"reason --rules {BUILTIN_RULES} --certificate",
                    "rules": lean_horn["rules"],
                    "derivations": len(horn),
                    "rules_sha256": sha256(h_dir / "rules.tsv"),
                    "horn_sha256": sha256(h_dir / "horn.tsv"),
                    "oo_horn": {"exit": 0, "verdict": lean_horn["verdict"], "theorem": lean_horn["theorem"]},
                    "isabelle": {"exit": 0, "verdict": isa["verdict"], "theorem": isa["theorem"]},
                },
                "forged": {
                    "claim": [fs.strip("<>"), f"{RDFS}subClassOf", forged_to.strip("<>")],
                    "copied_from": [fs.strip("<>"), f"{RDFS}subClassOf", fo.strip("<>")],
                    "oo_cert": {"exit": code_c, "ok": False, "first_rejected": refused["first_rejected"],
                                "rule": refused["rule"]},
                    "oo_horn": {"exit": code_h, "ok": False},
                    "isabelle": {"exit": code_i, "ok": False, "strict": isa_refused.get("strict")},
                },
            },
        }
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / "graph.json").write_text(
            json.dumps(graph, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"graph.json: {len(classes)} nodes, {len(a_edges)} asserted, {len(certified)} certified, "
              f"1 rejected ({refused['rule']} refused at line {refused['first_rejected']}), "
              f"from {len(asserted)} triples of {ONTOLOGY.name}")
    finally:
        shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
