#!/usr/bin/env python3
"""Full MCP pipeline benchmark: uses the actual Open Ontologies MCP server.

Starts `open-ontologies serve`, connects via the official MCP Python SDK,
and runs the complete tool chain: onto_clear → onto_validate → onto_load →
onto_stats → onto_lint → onto_query.

This is the same MCP protocol Claude uses when calling onto_* tools.

Requirements: pip install mcp
"""
import asyncio
import json
import os
import glob

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATASET_DIR = os.path.join(SCRIPT_DIR, "dataset")
OO_BIN = os.path.abspath(os.path.join(SCRIPT_DIR, "..", "..", "target", "release", "open-ontologies"))
GT_PATH = os.path.join(DATASET_DIR, "ground_truth.json")


def parse_tool_result(result):
    """Extract the parsed content from an MCP tool result."""
    for item in result.content:
        if item.type == "text":
            try:
                return json.loads(item.text)
            except (json.JSONDecodeError, ValueError):
                return item.text
    return str(result)


async def run_pipeline():
    print("=" * 80)
    print("FULL MCP PIPELINE: open-ontologies serve → MCP protocol (official SDK)")
    print("=" * 80)

    server_params = StdioServerParameters(
        command=OO_BIN,
        args=["serve"],
    )

    print("\nStarting Open Ontologies MCP server...")
    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            print("  Server connected via MCP protocol (JSON-RPC 2.0 over stdio)")

            # List available tools
            tools = await session.list_tools()
            tool_names = [t.name for t in tools.tools]
            print(f"  Available tools: {len(tool_names)}")

            ttl_files = sorted(glob.glob(os.path.join(DATASET_DIR, "img_*.ttl")))

            # Step 1: onto_clear — start fresh
            print("\n--- Step 1: onto_clear ---")
            result = await session.call_tool("onto_clear", {})
            print(f"  {parse_tool_result(result)}")

            # Step 2: onto_validate each TTL (using file paths)
            print("\n--- Step 2: onto_validate (each image) ---")
            valid_files = []
            total_triples = 0
            for ttl in ttl_files:
                ttl_abs = os.path.abspath(ttl)
                result = await session.call_tool("onto_validate", {"input": ttl_abs})
                data = parse_tool_result(result)
                if isinstance(data, dict):
                    ok = data.get("valid", data.get("ok", False))
                    triples = data.get("triple_count", data.get("triples", 0))
                else:
                    ok = False
                    triples = 0
                status = "VALID" if ok else "FAILED"
                print(f"  {os.path.basename(ttl)}: {status} — {triples} triples")
                if ok:
                    valid_files.append(ttl)
                    total_triples += triples
                else:
                    print(f"    Error: {data}")
            print(f"  Total: {total_triples} triples, {len(valid_files)}/{len(ttl_files)} valid")

            # Step 3: onto_load each validated TTL into Oxigraph
            print("\n--- Step 3: onto_load (into Oxigraph triple store) ---")
            for ttl in valid_files:
                ttl_abs = os.path.abspath(ttl)
                result = await session.call_tool("onto_load", {"path": ttl_abs})
                data = parse_tool_result(result)
                print(f"  {os.path.basename(ttl)}: {data}")

            # Step 4: onto_stats
            print("\n--- Step 4: onto_stats ---")
            result = await session.call_tool("onto_stats", {})
            stats = parse_tool_result(result)
            print(f"  {json.dumps(stats, indent=2) if isinstance(stats, dict) else stats}")

            # Step 5: onto_lint (check each TTL for quality issues)
            print("\n--- Step 5: onto_lint ---")
            total_issues = 0
            for ttl in valid_files:
                ttl_abs = os.path.abspath(ttl)
                result = await session.call_tool("onto_lint", {"input": ttl_abs})
                lint = parse_tool_result(result)
                if isinstance(lint, dict):
                    issues = lint.get("issues", [])
                    total_issues += len(issues)
                    if issues:
                        print(f"  {os.path.basename(ttl)}: {len(issues)} issues")
                        for issue in issues[:3]:
                            print(f"    - {issue}")
                    else:
                        print(f"  {os.path.basename(ttl)}: clean")
                else:
                    print(f"  {os.path.basename(ttl)}: {lint}")
            print(f"  Total lint issues: {total_issues}")

            # Step 6: onto_query — SPARQL queries across the combined graph
            print("\n--- Step 6: onto_query (SPARQL) ---")

            queries = [
                ("Images in graph",
                 "SELECT (COUNT(DISTINCT ?img) AS ?count) WHERE { ?img a <http://schema.org/ImageObject> }"),

                ("High-confidence objects (top 15)", """
                    PREFIX ex: <http://example.org/image/>
                    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
                    SELECT ?label ?conf WHERE {
                        ?obj ex:confidence ?conf .
                        ?obj rdfs:label ?label .
                    } ORDER BY DESC(?conf) LIMIT 15
                """),

                ("Animals found", """
                    PREFIX schema: <http://schema.org/>
                    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
                    SELECT DISTINCT ?label ?cat WHERE {
                        ?obj schema:category ?cat .
                        ?obj rdfs:label ?label .
                        FILTER(CONTAINS(LCASE(STR(?cat)), "animal"))
                    }
                """),

                ("Images with vehicles", """
                    PREFIX ex: <http://example.org/image/>
                    PREFIX schema: <http://schema.org/>
                    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
                    SELECT DISTINCT ?img ?label WHERE {
                        ?img a schema:ImageObject .
                        ?img (ex:hasObject|ex:containsObject|schema:hasPart|schema:about) ?obj .
                        ?obj schema:category ?cat .
                        ?obj rdfs:label ?label .
                        FILTER(CONTAINS(LCASE(STR(?cat)), "vehicle"))
                    }
                """),

                ("Total skos:altLabel synonyms",
                 "PREFIX skos: <http://www.w3.org/2004/02/skos/core#> SELECT (COUNT(?alt) AS ?total) WHERE { ?obj skos:altLabel ?alt }"),

                ("Total triples in Oxigraph store",
                 "SELECT (COUNT(*) AS ?total) WHERE { ?s ?p ?o }"),
            ]

            for label, sparql in queries:
                result = await session.call_tool("onto_query", {"query": sparql})
                data = parse_tool_result(result)
                if isinstance(data, (dict, list)):
                    print(f"\n  {label}: {json.dumps(data, indent=2)}")
                else:
                    print(f"\n  {label}: {data}")

            print(f"\n{'=' * 80}")
            print("Pipeline complete — all steps used the real MCP server (open-ontologies serve)")
            print(f"Tools called: onto_clear → onto_validate (×{len(ttl_files)}) → onto_load (×{len(valid_files)}) → onto_stats → onto_lint (×{len(valid_files)}) → onto_query (×{len(queries)})")
            print("=" * 80)

            # Save results
            results = {
                "pipeline": "MCP server (open-ontologies serve) via official MCP Python SDK",
                "protocol": "JSON-RPC 2.0 over stdio",
                "tools_used": ["onto_clear", "onto_validate", "onto_load", "onto_stats", "onto_lint", "onto_query"],
                "tools_available": len(tool_names),
                "images_processed": len(ttl_files),
                "images_valid": len(valid_files),
                "total_triples_validated": total_triples,
                "total_lint_issues": total_issues,
                "stats": stats if isinstance(stats, dict) else {},
            }
            out_path = os.path.join(DATASET_DIR, "mcp_pipeline_results.json")
            with open(out_path, "w") as f:
                json.dump(results, f, indent=2)
            print(f"\nResults saved to {out_path}")


def main():
    asyncio.run(run_pipeline())


if __name__ == "__main__":
    main()
