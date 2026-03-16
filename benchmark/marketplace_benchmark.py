#!/usr/bin/env python3
"""Benchmark all 29 marketplace ontologies through the MCP pipeline."""
import json, time, subprocess, sys, urllib.request, urllib.error, re

HOST = "127.0.0.1"
PORT = 9998
MCP_URL = f"http://{HOST}:{PORT}/mcp"

IDS = [
    "owl", "rdfs", "rdf", "bfo", "dolce", "schema-org", "foaf", "skos",
    "dc-elements", "dc-terms", "dcat", "void", "doap", "prov-o", "owl-time",
    "org", "ssn", "sosa", "geosparql", "locn", "shacl", "vcard", "odrl",
    "cc", "sioc", "adms", "goodrelations", "fibo", "qudt"
]

SESSION_ID = ""
CALL_ID = 0


def mcp_request(payload_dict):
    """Send an MCP request and parse SSE response, returning the JSON-RPC result."""
    global SESSION_ID
    payload = json.dumps(payload_dict).encode()
    req = urllib.request.Request(MCP_URL, data=payload)
    req.add_header("Content-Type", "application/json")
    req.add_header("Accept", "application/json, text/event-stream")
    if SESSION_ID:
        req.add_header("Mcp-Session-Id", SESSION_ID)

    try:
        resp = urllib.request.urlopen(req, timeout=120)
        if not SESSION_ID:
            SESSION_ID = resp.headers.get("Mcp-Session-Id", "")
        body = resp.read().decode("utf-8", errors="replace")
        # Parse SSE: find data: lines with JSON-RPC results
        for line in body.split("\n"):
            if line.startswith("data: "):
                data = line[6:].strip()
                if data:
                    try:
                        parsed = json.loads(data)
                        if "result" in parsed or "error" in parsed:
                            return parsed
                    except json.JSONDecodeError:
                        continue
        return {}
    except urllib.error.HTTPError as e:
        if e.code == 202:
            return {"ok": True}  # Notification accepted
        return {"error": f"HTTP {e.code}: {e.read().decode()[:200]}"}
    except Exception as e:
        return {"error": str(e)}


def mcp_init():
    """Initialize MCP session."""
    global SESSION_ID, CALL_ID
    CALL_ID = 1
    mcp_request({
        "jsonrpc": "2.0", "id": CALL_ID,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {"name": "benchmark", "version": "1.0.0"}
        }
    })
    # Send initialized notification
    mcp_request({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    })
    print(f"Session: {SESSION_ID}")


def tool_call(name, args=None):
    """Call an MCP tool and return the parsed result content."""
    global CALL_ID
    CALL_ID += 1
    resp = mcp_request({
        "jsonrpc": "2.0", "id": CALL_ID,
        "method": "tools/call",
        "params": {"name": name, "arguments": args or {}}
    })
    try:
        content = resp.get("result", {}).get("content", [])
        if isinstance(content, list) and len(content) > 0:
            text = content[0].get("text", "{}")
            return json.loads(text)
    except:
        pass
    return resp


def main():
    global SESSION_ID

    # Start server
    print(f"Starting server on port {PORT}...")
    server = subprocess.Popen(
        ["./target/release/open-ontologies", "serve-http", "--host", HOST, "--port", str(PORT)],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    time.sleep(3)

    try:
        mcp_init()
        results = []

        for onto_id in IDS:
            print(f"{onto_id:<20}", end="", flush=True)

            # Clear store
            tool_call("onto_clear")

            # Install (timed)
            t0 = time.time()
            install = tool_call("onto_marketplace", {"action": "install", "id": onto_id})
            fetch_ms = int((time.time() - t0) * 1000)

            if "error" in install:
                print(f"FAILED: {install['error']}")
                results.append({
                    "id": onto_id, "status": "failed", "error": str(install.get("error", "")),
                    "classes": 0, "properties": 0, "triples_before": 0,
                    "triples_after": 0, "inferred": 0, "fetch_ms": fetch_ms, "reason_ms": 0
                })
                continue

            # Resolve owl:imports if the ontology declares them
            tool_call("onto_import")

            # Stats before reasoning
            stats = tool_call("onto_stats")
            classes = stats.get("classes", 0)
            properties = stats.get("properties", stats.get("object_properties", 0) + stats.get("data_properties", 0))
            triples_before = stats.get("triples", 0)

            # Reason RDFS (timed)
            t0 = time.time()
            tool_call("onto_reason", {"profile": "rdfs"})
            rdfs_reason_ms = int((time.time() - t0) * 1000)

            # Stats after RDFS
            stats_rdfs = tool_call("onto_stats")
            triples_rdfs = stats_rdfs.get("triples", triples_before)
            inferred_rdfs = triples_rdfs - triples_before

            # Reason OWL-RL on top of RDFS (timed)
            t0 = time.time()
            tool_call("onto_reason", {"profile": "owl-rl"})
            owl_reason_ms = int((time.time() - t0) * 1000)

            # Stats after OWL-RL
            stats_owl = tool_call("onto_stats")
            triples_owl = stats_owl.get("triples", triples_rdfs)
            inferred_owl = triples_owl - triples_before

            print(f"classes={classes:<6} props={properties:<6} triples={triples_before} RDFS={triples_rdfs}(+{inferred_rdfs}) OWL-RL={triples_owl}(+{inferred_owl}) fetch={fetch_ms}ms rdfs={rdfs_reason_ms}ms owl={owl_reason_ms}ms")

            results.append({
                "id": onto_id, "status": "ok",
                "classes": classes, "properties": properties,
                "triples_before": triples_before,
                "triples_rdfs": triples_rdfs, "inferred_rdfs": inferred_rdfs,
                "triples_owl": triples_owl, "inferred_owl": inferred_owl,
                "fetch_ms": fetch_ms, "rdfs_reason_ms": rdfs_reason_ms, "owl_reason_ms": owl_reason_ms,
            })

        # Save results
        with open("benchmark/marketplace_results.json", "w") as f:
            json.dump(results, f, indent=2)

        # Print table
        ok = [r for r in results if r.get("status") == "ok"]
        failed = [r for r in results if r.get("status") != "ok"]

        print("\n")
        print(f"{'ID':<20} {'Classes':>8} {'Props':>8} {'Triples':>10} {'+ RDFS':>10} {'+ OWL-RL':>10} {'Fetch':>8} {'RDFS':>8} {'OWL-RL':>8}")
        print("-" * 110)
        for r in results:
            mark = " *" if r.get("status") != "ok" else ""
            rdfs_inf = f"+{r.get('inferred_rdfs', 0)}"
            owl_inf = f"+{r.get('inferred_owl', 0)}"
            print(f"{r['id']:<20} {r['classes']:>8} {r['properties']:>8} {r['triples_before']:>10} {rdfs_inf:>10} {owl_inf:>10} {str(r['fetch_ms'])+'ms':>8} {str(r.get('rdfs_reason_ms', 0))+'ms':>8} {str(r.get('owl_reason_ms', 0))+'ms':>8}{mark}")

        total_triples = sum(r["triples_before"] for r in ok)
        total_rdfs = sum(r.get("inferred_rdfs", 0) for r in ok)
        total_owl = sum(r.get("inferred_owl", 0) for r in ok)
        total_classes = sum(r["classes"] for r in ok)
        total_props = sum(r["properties"] for r in ok)
        print("-" * 110)
        print(f"{'TOTAL':<20} {total_classes:>8} {total_props:>8} {total_triples:>10} {'+'+str(total_rdfs):>10} {'+'+str(total_owl):>10}")
        rdfs_pct = int(total_rdfs / total_triples * 100) if total_triples else 0
        owl_pct = int(total_owl / total_triples * 100) if total_triples else 0
        print(f"\n{len(ok)}/{len(results)} ontologies loaded successfully")
        print(f"RDFS adds {rdfs_pct}% more triples, OWL-RL adds {owl_pct}% more triples on average.")
        if failed:
            print(f"Failed: {', '.join(r['id'] for r in failed)}")

    finally:
        server.terminate()
        server.wait()


if __name__ == "__main__":
    main()
