#!/usr/bin/env python3
"""Serve the page locally against a local engine: OO_BIN=target/debug/open-ontologies python3 local.py"""
import os, sys
from http.server import HTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent / "api"))
from index import handler as api  # noqa: E402

PUBLIC = Path(__file__).parent / "public"

class H(api):
    def do_GET(self):
        if self.path.startswith("/api/"):
            return api.do_GET(self)
        p = PUBLIC / (self.path.split("?")[0].strip("/") or "index.html")
        if not p.exists():
            self.send_response(404); self.end_headers(); return
        data = p.read_bytes(); self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8" if p.suffix == ".html" else "application/octet-stream")
        self.send_header("Content-Length", str(len(data))); self.end_headers(); self.wfile.write(data)

port = int(os.environ.get("PORT", "8787"))
print(f"engine: {os.environ.get('OO_BIN', '(bundled)')}  ->  http://127.0.0.1:{port}/")
HTTPServer(("127.0.0.1", port), H).serve_forever()
