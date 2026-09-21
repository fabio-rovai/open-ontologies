"""Try it here: one sheet in, one ontology out, on the pinned engine.

One Vercel Python function, no reimplementation. Every request writes the
sheet to a scratch directory and runs the SAME `open-ontologies` binary a
user would download, in `batch` mode, and returns what it printed. The page
shows the engine's words, including the sentence that says an induced
ontology is a hypothesis about the sheet and not a truth about the domain.

Routes (all under /api/, the rewrite passes the tail as ?p=):
  GET  samples            the bundled sample sheets
  POST induce             {name, csv, class?}           -> the engine's induce report
  POST check              {name, csv, ontology_ttl, shapes_ttl, mapping} -> the engine's SHACL report
  GET  engine             the pinned tag and binary hash
"""
from __future__ import annotations

import hashlib
import json
import os
import shutil
import stat
import subprocess
import tempfile
import time
from http.server import BaseHTTPRequestHandler
from pathlib import Path
from urllib.parse import parse_qs, urlparse

HERE = Path(__file__).resolve().parent
SAMPLES = HERE / "_samples"
MAX_BYTES = 1_000_000
TIMEOUT_S = 40


def engine() -> Path:
    """The binary, executable, in a writable place. The deployment bundle is
    read-only and may not keep the mode bit, so it is copied once per warm
    instance into the scratch directory."""
    override = os.environ.get("OO_BIN")
    if override:
        return Path(override)
    src = HERE / "_bin" / "open-ontologies"
    dst = Path(tempfile.gettempdir()) / "oo-engine" / "open-ontologies"
    if not dst.exists() or dst.stat().st_size != src.stat().st_size:
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, dst)
        dst.chmod(dst.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return dst


def checker() -> Path | None:
    """The Lean certificate checker, executable, or None when the release did
    not carry one. The page says which; it never implies a check that did not
    happen."""
    override = os.environ.get("OO_CERT")
    src = Path(override) if override else (HERE / "_bin" / "oo-cert")
    if not src.exists():
        return None
    dst = Path(tempfile.gettempdir()) / "oo-engine" / "oo-cert"
    if not dst.exists() or dst.stat().st_size != src.stat().st_size:
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, dst)
        dst.chmod(dst.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return dst


def run_checker(cert_dir: Path) -> dict:
    """Run the Lean checker over a certificate directory and return what it said.

    The exit code is part of the answer, not an error: exit 1 with a named rule
    is the whole point of the third panel.
    """
    c = checker()
    if c is None:
        return {"absent": True, "means": "this release carries no oo-cert, so nothing was checked here"}
    proc = subprocess.run(
        [str(c), str(cert_dir / "asserted.tsv"), str(cert_dir / "derivations.tsv")],
        capture_output=True, text=True, timeout=TIMEOUT_S,
    )
    line = next((l for l in proc.stdout.splitlines() if l.startswith("{")), "")
    try:
        out = json.loads(line) if line else {}
    except json.JSONDecodeError:
        out = {}
    out["exit"] = proc.returncode
    if not line:
        out["stderr_tail"] = proc.stderr[-300:]
    return out


def engine_info() -> dict:
    tag_file = HERE / "_bin" / "TAG"
    tag = tag_file.read_text().strip() if tag_file.exists() else os.environ.get("OO_ENGINE_TAG", "local build")
    try:
        h = hashlib.sha256(engine().read_bytes()).hexdigest()
    except Exception as e:  # noqa: BLE001
        h = f"unavailable: {e}"
    c = checker()
    ch = None
    if c is not None:
        try:
            ch = hashlib.sha256(c.read_bytes()).hexdigest()
        except Exception as e:  # noqa: BLE001
            ch = f"unavailable: {e}"
    return {
        "tag": tag,
        "sha256": h,
        "checker_sha256": ch,
        "checker": "oo-cert" if ch else None,
        "means": "every answer on this page is the output of these binaries; the page draws, it does not decide",
    }


def run_batch(workdir: Path, script: str) -> list[dict]:
    """Run one batch script and return the engine's JSON lines, in order."""
    (workdir / "script.txt").write_text(script)
    data_dir = workdir / "data"
    data_dir.mkdir(exist_ok=True)
    t0 = time.time()
    proc = subprocess.run(
        [str(engine()), "--data-dir", str(data_dir), "batch", str(workdir / "script.txt")],
        capture_output=True, text=True, timeout=TIMEOUT_S, cwd=str(workdir),
    )
    out = []
    for line in proc.stdout.splitlines():
        if line.startswith("{"):
            try:
                out.append(json.loads(line))
            except json.JSONDecodeError:
                out.append({"command": "?", "result": {"error": f"unparseable engine line: {line[:200]}"}})
    if not out:
        raise RuntimeError(f"the engine printed nothing (exit {proc.returncode}): {proc.stderr[-800:]}")
    out.append({"command": "_meta", "result": {"exit": proc.returncode, "seconds": round(time.time() - t0, 3), "stderr_tail": proc.stderr[-400:]}})
    return out


def safe_name(name: str) -> str:
    base = Path(name or "sheet.csv").name
    keep = "".join(c if c.isalnum() or c in "-_." else "_" for c in base)
    if not keep.lower().endswith((".csv", ".json", ".ndjson", ".xml", ".yaml", ".yml")):
        keep += ".csv"
    return keep or "sheet.csv"


def do_induce(body: dict) -> dict:
    csv = body.get("csv") or ""
    if not csv.strip():
        return {"error": "the sheet is empty"}
    if len(csv.encode()) > MAX_BYTES:
        return {"error": f"the sheet is larger than {MAX_BYTES // 1000} kB; this page is for playing, run the engine locally for the real thing"}
    name = safe_name(body.get("name", "sheet.csv"))
    with tempfile.TemporaryDirectory(prefix="oo-try-") as td:
        wd = Path(td)
        (wd / name).write_text(csv)
        out = wd / "induced"
        cls = body.get("class") or ""
        cls_arg = f" --class {''.join(c for c in cls if c.isalnum() or c in '-_')}" if cls.strip() else ""
        base = "http://example.org/try/"
        script = (
            f"induce {wd / name} --out {out} --base-iri {base}{cls_arg}\n"
            f"shacl {out / 'shapes.ttl'}\n"
            "reason owl-rl\n"
            "stats\n"
        )
        lines = run_batch(wd, script)
        by = {l.get("command"): l.get("result") for l in lines}
        ind = by.get("induce") or {}
        if "error" in ind:
            return {"error": ind["error"], "engine": lines}
        return {
            "induced": ind,
            "shacl": by.get("shacl"),
            "reason": by.get("reason"),
            "stats": by.get("stats"),
            "meta": by.get("_meta"),
            "mapping_json": (out / "mapping.json").read_text() if (out / "mapping.json").exists() else None,
        }


def do_check(body: dict) -> dict:
    csv = body.get("csv") or ""
    if len(csv.encode()) > MAX_BYTES:
        return {"error": "too large"}
    for k in ("ontology_ttl", "shapes_ttl", "mapping"):
        if not body.get(k):
            return {"error": f"missing {k}; induce first"}
    name = safe_name(body.get("name", "sheet.csv"))
    with tempfile.TemporaryDirectory(prefix="oo-try-") as td:
        wd = Path(td)
        (wd / name).write_text(csv)
        (wd / "ontology.ttl").write_text(body["ontology_ttl"])
        (wd / "shapes.ttl").write_text(body["shapes_ttl"])
        mapping = body["mapping"]
        (wd / "mapping.json").write_text(mapping if isinstance(mapping, str) else json.dumps(mapping))
        base = "http://example.org/try/"
        script = (
            f"load {wd / 'ontology.ttl'}\n"
            f"ingest {wd / name} --mapping {wd / 'mapping.json'} --base-iri {base}\n"
            f"shacl {wd / 'shapes.ttl'}\n"
        )
        lines = run_batch(wd, script)
        by = {l.get("command"): l.get("result") for l in lines}
        return {"ingest": by.get("ingest"), "shacl": by.get("shacl"), "meta": by.get("_meta")}


def do_plan(body: dict) -> dict:
    """The whole loop, on the sheet the visitor brought.

    An induced ontology is flat, so a reasoner derives nothing from it: measured,
    0 derivations under both rdfs and owl-rl, because the mapping already asserts
    the class of every row and `rdfs:domain` can then only re-derive it. That is
    a true fact about a flat sheet, and manufacturing consequences to hide it
    would make the demo a lie.

    So the visitor adds one line, which is the front page's own argument: plan a
    change to an ontology and see what follows. One `rdfs:subClassOf` over the
    induced class gives one derived type for each row, and the certificate over
    those derivations is the thing the checker then accepts, and refuses when a
    conclusion in it is forged.
    """
    csv = body.get("csv") or ""
    extra = (body.get("axiom") or "").strip()
    for k in ("ontology_ttl", "mapping"):
        if not body.get(k):
            return {"error": f"missing {k}; induce first"}
    if len(csv.encode()) > MAX_BYTES:
        return {"error": "too large"}
    if len(extra) > 2000:
        return {"error": "the added axioms are longer than 2000 characters"}
    name = safe_name(body.get("name", "sheet.csv"))
    with tempfile.TemporaryDirectory(prefix="oo-try-") as td:
        wd = Path(td)
        (wd / name).write_text(csv)
        (wd / "plan.ttl").write_text(body["ontology_ttl"] + "\n" + extra + "\n")
        mapping = body["mapping"]
        (wd / "mapping.json").write_text(mapping if isinstance(mapping, str) else json.dumps(mapping))
        cert = wd / "cert"
        base = "http://example.org/try/"
        script = (
            f"load {wd / 'plan.ttl'}\n"
            f"ingest {wd / name} --mapping {wd / 'mapping.json'} --base-iri {base}\n"
            f"reason --profile owl-rl --certificate {cert}\n"
        )
        lines = run_batch(wd, script)
        by = {l.get("command"): l.get("result") for l in lines}
        reason = by.get("reason") or {}
        if "error" in reason:
            return {"error": reason["error"], "engine": lines}
        c = (reason.get("certificate") or {})

        accepted = run_checker(cert)

        # Now forge one conclusion and hand the SAME checker the SAME premises.
        forged = None
        derivations = (cert / "derivations.tsv").read_text() if (cert / "derivations.tsv").exists() else ""
        rows = [r for r in derivations.splitlines() if r.strip()]
        if rows and not accepted.get("absent"):
            f = wd / "forged"
            f.mkdir(exist_ok=True)
            (f / "asserted.tsv").write_text((cert / "asserted.tsv").read_text())
            fields = rows[0].split("\t")
            original = "\t".join(fields[:4])
            if len(fields) >= 4:
                fields[3] = "<http://example.org/try/ont#NotDerivable>"
            rows_forged = ["\t".join(fields)] + rows[1:]
            (f / "derivations.tsv").write_text("\n".join(rows_forged) + "\n")
            forged = run_checker(f)
            forged["line_before"] = original
            forged["line_after"] = "\t".join(fields[:4])
        # The three edge kinds the front-page figure draws, for the visitor's
        # own sheet: what a person asserted, what the engine derived and the
        # checker accepted, and the one line that was forged and refused. The
        # page draws them; it does not decide which is which, and the kind of
        # every edge here comes from which file the engine wrote it to.
        graph = build_graph(cert, rows, forged)
        return {
            "reason": reason,
            "certificate": c,
            "accepted": accepted,
            "forged": forged,
            "derivations_sample": rows[:6],
            "graph": graph,
            "meta": by.get("_meta"),
        }


# A budget for each kind, not one pool. The first version used a single cap and
# the asserted edges ate all of it: 260 asserted, 0 certified, 0 rejected, on a
# sheet whose run produced 134 derivations and one forged line. The picture
# exists to show those three together, so the two that are scarce are drawn
# first and the plentiful one fills what is left.
BUDGET = {"rejected": 8, "certified": 130, "asserted": 170}


def _local(term: str) -> str:
    """The readable tail of an IRI, for a label."""
    t = term.strip().strip("<>")
    if t.startswith('"'):
        return t.split('"')[1][:24] if '"' in t[1:] else t[:24]
    for sep in ("#", "/"):
        if sep in t:
            t = t.rsplit(sep, 1)[-1] or t
    return t[:24]


def build_graph(cert: Path, derivation_rows: list, forged: dict | None) -> dict:
    """The asserted, certified and rejected edges, ready to draw.

    Literals are dropped: a value node per cell turns the picture into a
    hairball and says nothing about structure. What is left is the shape of the
    ontology and the data as the engine saw it.
    """
    edges: list = []
    seen: set = set()
    used = {k: 0 for k in BUDGET}
    total = {k: 0 for k in BUDGET}

    def add(s: str, p: str, o: str, kind: str) -> None:
        total[kind] += 1
        if used[kind] >= BUDGET[kind]:
            return
        if o.strip().startswith('"') or s.strip().startswith('"'):
            return
        key = (s, o, kind)
        if key in seen:
            return
        seen.add(key)
        used[kind] += 1
        edges.append({"s": _local(s), "o": _local(o), "p": _local(p), "kind": kind})

    # Scarce first. A forged line looks like the rest, which is what makes a
    # checker worth having, so it is never the edge that gets dropped.
    if forged and forged.get("line_after"):
        f = forged["line_after"].split("\t")
        if len(f) >= 4:
            add(f[1], f[2], f[3], "rejected")
    for line in derivation_rows:
        f = line.split("\t")
        if len(f) >= 4:
            add(f[1], f[2], f[3], "certified")
    asserted = (cert / "asserted.tsv").read_text() if (cert / "asserted.tsv").exists() else ""
    for line in asserted.splitlines():
        f = line.split("\t")
        if len(f) >= 3:
            add(f[0], f[1], f[2], "asserted")

    nodes = sorted({n for e in edges for n in (e["s"], e["o"])})
    return {
        "nodes": nodes,
        "edges": edges,
        "counts": {
            k: sum(1 for e in edges if e["kind"] == k)
            for k in ("asserted", "certified", "rejected")
        },
        "drawn": dict(used),
        "total": dict(total),
        "truncated": any(used[k] < total[k] for k in used),
        "means": "grey is what a person asserted, green is what the engine derived and the "
                 "checker accepted, red is the line that was forged and refused. Literals are "
                 "not drawn",
    }


def do_samples() -> dict:
    items = []
    for p in sorted(SAMPLES.glob("*.csv")):
        text = p.read_text()
        items.append({"name": p.name, "rows": max(0, text.count("\n") - 1), "csv": text})
    return {"samples": items}


class handler(BaseHTTPRequestHandler):  # noqa: N801  (Vercel's expected name)
    def _send(self, code: int, payload: dict) -> None:
        data = json.dumps(payload).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _route(self) -> str:
        u = urlparse(self.path)
        q = parse_qs(u.query)
        if "p" in q:
            return q["p"][0].strip("/")
        return u.path.replace("/api/", "", 1).strip("/")

    def do_GET(self):  # noqa: N802
        r = self._route()
        try:
            if r == "samples":
                return self._send(200, do_samples())
            if r == "engine":
                return self._send(200, engine_info())
            return self._send(404, {"error": f"no such route: {r}"})
        except Exception as e:  # noqa: BLE001
            return self._send(500, {"error": str(e)})

    def do_POST(self):  # noqa: N802
        r = self._route()
        try:
            n = int(self.headers.get("Content-Length") or 0)
            if n > MAX_BYTES * 4:
                return self._send(413, {"error": "request too large"})
            body = json.loads(self.rfile.read(n) or b"{}")
            if r == "induce":
                res = do_induce(body)
            elif r == "check":
                res = do_check(body)
            elif r == "plan":
                res = do_plan(body)
            else:
                return self._send(404, {"error": f"no such route: {r}"})
            return self._send(400 if "error" in res else 200, res)
        except subprocess.TimeoutExpired:
            return self._send(504, {"error": f"the engine did not finish in {TIMEOUT_S} s"})
        except Exception as e:  # noqa: BLE001
            return self._send(500, {"error": str(e)})

    def log_message(self, *_):  # quiet
        pass
