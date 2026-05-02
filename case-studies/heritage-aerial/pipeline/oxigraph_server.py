#!/usr/bin/env python3
"""
NAPH Oxigraph Server — persistent triple store for development and demo.

Runs Oxigraph (the Rust triple store backing Open Ontologies) as a persistent
HTTP server with NAPH ontology + sample data + (optionally) scraped data
preloaded. Provides:

- A SPARQL endpoint at http://localhost:7878/query
- A SPARQL update endpoint at http://localhost:7878/update
- Web UI at http://localhost:7878/

Use cases:
- Local development without re-loading on every query
- Demo SPARQL endpoint for federation testing
- Backing store for the compliance dashboard

The Open Ontologies CLI uses an in-memory store that resets between commands.
This script provides persistence for longer-running development workflows.

Usage:
    python3 pipeline/oxigraph_server.py start         # start server, load data
    python3 pipeline/oxigraph_server.py stop          # stop server
    python3 pipeline/oxigraph_server.py status        # check if running
    python3 pipeline/oxigraph_server.py reload-data   # reload all NAPH data
    python3 pipeline/oxigraph_server.py query "SELECT ..."   # one-off query
"""

import argparse
import json
import os
import signal
import subprocess
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA_DIR = ROOT / ".oxigraph"
PID_FILE = DATA_DIR / "server.pid"
PORT = int(os.environ.get("NAPH_OXIGRAPH_PORT", "7878"))
ENDPOINT = f"http://localhost:{PORT}"

ONTOLOGY = ROOT / "ontology" / "naph-core.ttl"
SHAPES = ROOT / "ontology" / "naph-shapes.ttl"
DATA = ROOT / "data" / "sample-photographs.ttl"
PIPELINE_OUTPUT = ROOT / "pipeline" / "generated-from-csv.ttl"


def is_running() -> bool:
    if not PID_FILE.exists():
        return False
    try:
        pid = int(PID_FILE.read_text().strip())
        os.kill(pid, 0)
        return True
    except (OSError, ValueError):
        return False


def start_server() -> int:
    """Start Open Ontologies HTTP server (which wraps Oxigraph) as background process."""
    DATA_DIR.mkdir(parents=True, exist_ok=True)

    if is_running():
        pid = int(PID_FILE.read_text().strip())
        print(f"Server already running (pid {pid}) at {ENDPOINT}", file=sys.stderr)
        return 0

    cmd = ["open-ontologies", "serve-http", "--host", "127.0.0.1", "--port", str(PORT)]
    proc = subprocess.Popen(
        cmd,
        stdout=open(DATA_DIR / "server.log", "w"),
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    PID_FILE.write_text(str(proc.pid))

    # Wait for server health check via /api/stats
    for _ in range(30):
        time.sleep(0.5)
        try:
            with urllib.request.urlopen(f"{ENDPOINT}/api/stats", timeout=1) as resp:
                if resp.status == 200:
                    print(f"Server started on {ENDPOINT} (pid {proc.pid})", file=sys.stderr)
                    return proc.pid
        except Exception:
            continue

    print(f"Server failed to start within 15s. Check {DATA_DIR / 'server.log'}", file=sys.stderr)
    return -1


def stop_server() -> None:
    if not PID_FILE.exists():
        print("Oxigraph not running (no PID file).", file=sys.stderr)
        return
    pid = int(PID_FILE.read_text().strip())
    try:
        os.kill(pid, signal.SIGTERM)
        print(f"Stopped Oxigraph (pid {pid})", file=sys.stderr)
    except OSError as e:
        print(f"Could not stop pid {pid}: {e}", file=sys.stderr)
    PID_FILE.unlink(missing_ok=True)


def status() -> None:
    if is_running():
        pid = int(PID_FILE.read_text().strip())
        print(f"Oxigraph running on {ENDPOINT} (pid {pid})")
    else:
        print("Oxigraph not running")


def reload_data() -> None:
    """Load ontology + shapes + sample data + (optionally) generated data via REST API."""
    if not is_running():
        print("Server not running. Start it first.", file=sys.stderr)
        sys.exit(1)

    files_to_load = [str(ONTOLOGY), str(DATA)]
    if PIPELINE_OUTPUT.exists():
        files_to_load.append(str(PIPELINE_OUTPUT))

    # POST /api/load for each file
    for path in files_to_load:
        body = json.dumps({"path": path}).encode()
        req = urllib.request.Request(
            f"{ENDPOINT}/api/load",
            data=body,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                result = json.load(resp)
                print(f"loaded {Path(path).name}: {result.get('triples_loaded', '?')} triples", file=sys.stderr)
        except urllib.error.HTTPError as e:
            print(f"failed to load {path}: HTTP {e.code} {e.read().decode()}", file=sys.stderr)
            sys.exit(1)

    # Final stats
    with urllib.request.urlopen(f"{ENDPOINT}/api/stats", timeout=5) as resp:
        stats = json.load(resp)
        print(f"total triples: {stats.get('triples', '?')}", file=sys.stderr)
        print(f"total individuals: {stats.get('individuals', '?')}", file=sys.stderr)


def run_query(sparql: str) -> dict:
    """Run a SPARQL query against the running server's REST API."""
    if not is_running():
        print("Server not running. Start it first.", file=sys.stderr)
        sys.exit(1)

    body = json.dumps({"query": sparql}).encode()
    req = urllib.request.Request(
        f"{ENDPOINT}/api/query",
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.load(resp)
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", errors="ignore")
        print(f"Query failed (HTTP {e.code}): {body}", file=sys.stderr)
        sys.exit(1)


def main():
    parser = argparse.ArgumentParser(description="NAPH persistent Oxigraph server controller.")
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("start", help="Start the server (idempotent)")
    sub.add_parser("stop", help="Stop the server")
    sub.add_parser("status", help="Check if server is running")
    sub.add_parser("reload-data", help="Clear and reload NAPH ontology + data")
    q = sub.add_parser("query", help="Run a one-off SPARQL query")
    q.add_argument("sparql")
    args = parser.parse_args()

    if args.cmd == "start":
        start_server()
    elif args.cmd == "stop":
        stop_server()
    elif args.cmd == "status":
        status()
    elif args.cmd == "reload-data":
        reload_data()
    elif args.cmd == "query":
        result = run_query(args.sparql)
        print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
