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


def engine_info() -> dict:
    tag_file = HERE / "_bin" / "TAG"
    tag = tag_file.read_text().strip() if tag_file.exists() else os.environ.get("OO_ENGINE_TAG", "local build")
    try:
        h = hashlib.sha256(engine().read_bytes()).hexdigest()
    except Exception as e:  # noqa: BLE001
        h = f"unavailable: {e}"
    return {"tag": tag, "sha256": h, "means": "every answer on this page is the output of this binary in batch mode; the page draws, it does not decide"}


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
            else:
                return self._send(404, {"error": f"no such route: {r}"})
            return self._send(400 if "error" in res else 200, res)
        except subprocess.TimeoutExpired:
            return self._send(504, {"error": f"the engine did not finish in {TIMEOUT_S} s"})
        except Exception as e:  # noqa: BLE001
            return self._send(500, {"error": str(e)})

    def log_message(self, *_):  # quiet
        pass
