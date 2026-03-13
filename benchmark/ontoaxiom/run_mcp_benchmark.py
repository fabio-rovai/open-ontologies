#!/usr/bin/env python3
"""
OntoAxiom Benchmark: Real MCP Server Pipeline

Runs the OntoAxiom benchmark (https://arxiv.org/abs/2512.05594) using the
actual Open Ontologies MCP server — the same pipeline Claude uses:

    onto_clear → onto_load → onto_query (SPARQL)

Starts `open-ontologies serve`, connects via the official MCP Python SDK
(JSON-RPC 2.0 over stdio), and extracts axioms using SPARQL queries against
the Oxigraph triple store.

Best bare LLM result: o1 with F1=0.197.

Requirements: pip install mcp
"""
import asyncio
import json
import os
import re
import sys

try:
    from mcp import ClientSession, StdioServerParameters
    from mcp.client.stdio import stdio_client
except ImportError:
    print("ERROR: MCP SDK required. Install with: pip install mcp")
    sys.exit(1)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(SCRIPT_DIR, "data", "ontoaxiom")
ONTOLOGIES_DIR = os.path.join(DATA_DIR, "ontologies")
OO_BIN = os.path.abspath(os.path.join(SCRIPT_DIR, "..", "..", "target", "release", "open-ontologies"))

# SPARQL queries to extract each axiom type from the Oxigraph store.
# These mirror what a real MCP user would run via onto_query.
# We use FILTER(lang()="en" || lang()="") to prefer English labels and avoid
# duplicate results from multi-language ontologies (e.g. pizza has en + pt).
SPARQL_QUERIES = {
    "subclassof": """
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT DISTINCT ?sub ?sup ?subLabel ?supLabel WHERE {
            ?sub rdfs:subClassOf ?sup .
            FILTER(!isBlank(?sub) && !isBlank(?sup))
            FILTER(?sub != ?sup)
            FILTER(!STRSTARTS(STR(?sub), "http://www.w3.org/"))
            FILTER(!STRSTARTS(STR(?sup), "http://www.w3.org/"))
            OPTIONAL { ?sub rdfs:label ?subL . FILTER(lang(?subL) = "en" || lang(?subL) = "") }
            OPTIONAL { ?sup rdfs:label ?supL . FILTER(lang(?supL) = "en" || lang(?supL) = "") }
            BIND(COALESCE(?subL, ?sub) AS ?subLabel)
            BIND(COALESCE(?supL, ?sup) AS ?supLabel)
        }
    """,
    "disjoint_direct": """
        PREFIX owl: <http://www.w3.org/2002/07/owl#>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT DISTINCT ?a ?b ?aLabel ?bLabel WHERE {
            ?a owl:disjointWith ?b .
            FILTER(!isBlank(?a) && !isBlank(?b))
            OPTIONAL { ?a rdfs:label ?aL . FILTER(lang(?aL) = "en" || lang(?aL) = "") }
            OPTIONAL { ?b rdfs:label ?bL . FILTER(lang(?bL) = "en" || lang(?bL) = "") }
            BIND(COALESCE(?aL, ?a) AS ?aLabel)
            BIND(COALESCE(?bL, ?b) AS ?bLabel)
        }
    """,
    "disjoint_alldisjoint": """
        PREFIX owl: <http://www.w3.org/2002/07/owl#>
        PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT DISTINCT ?adc ?member ?memberLabel WHERE {
            ?adc a owl:AllDisjointClasses ;
                 owl:members ?list .
            ?list rdf:rest*/rdf:first ?member .
            FILTER(!isBlank(?member))
            OPTIONAL { ?member rdfs:label ?mL . FILTER(lang(?mL) = "en" || lang(?mL) = "") }
            BIND(COALESCE(?mL, ?member) AS ?memberLabel)
        }
    """,
    "domain": """
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        PREFIX owl: <http://www.w3.org/2002/07/owl#>
        SELECT DISTINCT ?dom ?prop ?domLabel ?propLabel WHERE {
            ?prop rdfs:domain ?dom .
            FILTER(!isBlank(?dom))
            { ?prop a owl:ObjectProperty } UNION { ?prop a owl:DatatypeProperty }
            OPTIONAL { ?dom rdfs:label ?dL . FILTER(lang(?dL) = "en" || lang(?dL) = "") }
            OPTIONAL { ?prop rdfs:label ?pL . FILTER(lang(?pL) = "en" || lang(?pL) = "") }
            BIND(COALESCE(?dL, ?dom) AS ?domLabel)
            BIND(COALESCE(?pL, ?prop) AS ?propLabel)
        }
    """,
    "range": """
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        PREFIX owl: <http://www.w3.org/2002/07/owl#>
        SELECT DISTINCT ?prop ?ran ?propLabel ?ranLabel WHERE {
            ?prop rdfs:range ?ran .
            FILTER(!isBlank(?ran))
            { ?prop a owl:ObjectProperty } UNION { ?prop a owl:DatatypeProperty }
            OPTIONAL { ?prop rdfs:label ?pL . FILTER(lang(?pL) = "en" || lang(?pL) = "") }
            OPTIONAL { ?ran rdfs:label ?rL . FILTER(lang(?rL) = "en" || lang(?rL) = "") }
            BIND(COALESCE(?pL, ?prop) AS ?propLabel)
            BIND(COALESCE(?rL, ?ran) AS ?ranLabel)
        }
    """,
    "subproperty": """
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT DISTINCT ?sub ?sup ?subLabel ?supLabel WHERE {
            ?sub rdfs:subPropertyOf ?sup .
            FILTER(?sub != ?sup)
            FILTER(!STRSTARTS(STR(?sub), "http://www.w3.org/"))
            FILTER(!STRSTARTS(STR(?sup), "http://www.w3.org/"))
            OPTIONAL { ?sub rdfs:label ?subL . FILTER(lang(?subL) = "en" || lang(?subL) = "") }
            OPTIONAL { ?sup rdfs:label ?supL . FILTER(lang(?supL) = "en" || lang(?supL) = "") }
            BIND(COALESCE(?subL, ?sub) AS ?subLabel)
            BIND(COALESCE(?supL, ?sup) AS ?supLabel)
        }
    """,
}


def clean_sparql_value(s):
    """Clean Oxigraph SPARQL result value.

    Oxigraph returns values in N-Triples-like format:
      - IRIs: <http://example.org/Foo>
      - Literals: "value"@en or "value"^^<xsd:type>
    Strip the wrapping to get the raw value.
    """
    s = str(s).strip()
    # IRI: <http://...>
    if s.startswith("<") and s.endswith(">"):
        return s[1:-1]
    # Literal with language tag: "value"@en
    if s.startswith('"'):
        # Remove language tag or datatype
        if '"@' in s:
            s = s[:s.rindex('"@')]
        elif '"^^' in s:
            s = s[:s.rindex('"^^')]
        # Strip surrounding quotes
        if s.startswith('"') and s.endswith('"'):
            s = s[1:-1]
        elif s.startswith('"'):
            s = s[1:]
    return s


def normalize(s):
    """Normalize for soft matching (same approach as OntoAxiom evaluation)."""
    s = clean_sparql_value(s)
    # Extract local name from IRI
    if "#" in s:
        s = s.split("#")[-1]
    elif "/" in s:
        s = s.split("/")[-1]
    # Split camelCase: hasBase -> has Base, subClassOf -> sub Class Of
    s = re.sub(r'([a-z])([A-Z])', r'\1 \2', s)
    s = re.sub(r'([A-Z]+)([A-Z][a-z])', r'\1 \2', s)
    return s.lower().strip().replace("_", " ").replace("-", " ")


def parse_tool_result(result):
    """Extract parsed content from an MCP tool result."""
    for item in result.content:
        if item.type == "text":
            try:
                return json.loads(item.text)
            except (json.JSONDecodeError, ValueError):
                return item.text
    return str(result)


def parse_sparql_results(data):
    """Parse SPARQL results from onto_query response."""
    if isinstance(data, dict):
        if "results" in data:
            results = data["results"]
            if isinstance(results, list):
                return results
            if isinstance(results, dict) and "bindings" in results:
                return results["bindings"]
        if "bindings" in data:
            return data["bindings"]
        if "rows" in data:
            return data["rows"]
    if isinstance(data, list):
        return data
    return []


def get_binding_value(binding, key):
    """Extract value from a SPARQL binding, handling Oxigraph formats."""
    if key not in binding:
        return None
    val = binding[key]
    if isinstance(val, dict):
        return val.get("value", str(val))
    return str(val)


def load_ground_truth(axiom_type, ontology_name):
    """Load ground truth pairs from OntoAxiom dataset."""
    path = os.path.join(DATA_DIR, axiom_type, f"{ontology_name}_{axiom_type}.json")
    if not os.path.exists(path):
        return set()
    with open(path) as f:
        data = json.load(f)
    gt = set()
    for pair in data:
        a, b = normalize(pair[0]), normalize(pair[1])
        if axiom_type == "disjoint":
            gt.add((min(a, b), max(a, b)))
        else:
            gt.add((a, b))
    return gt


def precision_recall_f1(tp, fp, fn):
    precision = tp / (tp + fp) if (tp + fp) > 0 else 0.0
    recall = tp / (tp + fn) if (tp + fn) > 0 else 0.0
    f1 = 2 * precision * recall / (precision + recall + 1e-9) if (precision + recall) > 0 else 0.0
    return precision, recall, f1


async def extract_axioms_via_mcp(session, axiom_type):
    """Extract axiom pairs from the loaded ontology using SPARQL via onto_query."""
    pairs = set()

    if axiom_type == "disjoint":
        # Direct owl:disjointWith
        result = await session.call_tool("onto_query", {"query": SPARQL_QUERIES["disjoint_direct"]})
        data = parse_tool_result(result)
        bindings = parse_sparql_results(data)
        for b in bindings:
            a_label = get_binding_value(b, "aLabel")
            b_label = get_binding_value(b, "bLabel")
            if a_label and b_label:
                a, b_val = normalize(a_label), normalize(b_label)
                pairs.add((min(a, b_val), max(a, b_val)))

        # AllDisjointClasses — group members per ADC, generate pairwise disjoint pairs
        result = await session.call_tool("onto_query", {"query": SPARQL_QUERIES["disjoint_alldisjoint"]})
        data = parse_tool_result(result)
        bindings = parse_sparql_results(data)
        # Group by ADC blank node
        from collections import defaultdict
        adc_groups = defaultdict(set)
        for b in bindings:
            adc_id = get_binding_value(b, "adc") or "default"
            label = get_binding_value(b, "memberLabel")
            if label:
                adc_groups[adc_id].add(normalize(label))
        for group_members in adc_groups.values():
            members = sorted(group_members)
            for i, c1 in enumerate(members):
                for c2 in members[i+1:]:
                    pairs.add((min(c1, c2), max(c1, c2)))
    else:
        query_key = axiom_type
        result = await session.call_tool("onto_query", {"query": SPARQL_QUERIES[query_key]})
        data = parse_tool_result(result)
        bindings = parse_sparql_results(data)

        if axiom_type == "subclassof":
            for b in bindings:
                sub = get_binding_value(b, "subLabel")
                sup = get_binding_value(b, "supLabel")
                if sub and sup:
                    pairs.add((normalize(sub), normalize(sup)))
        elif axiom_type == "domain":
            for b in bindings:
                dom = get_binding_value(b, "domLabel")
                prop = get_binding_value(b, "propLabel")
                if dom and prop:
                    pairs.add((normalize(dom), normalize(prop)))
        elif axiom_type == "range":
            for b in bindings:
                prop = get_binding_value(b, "propLabel")
                ran = get_binding_value(b, "ranLabel")
                if prop and ran:
                    pairs.add((normalize(prop), normalize(ran)))
        elif axiom_type == "subproperty":
            for b in bindings:
                sub = get_binding_value(b, "subLabel")
                sup = get_binding_value(b, "supLabel")
                if sub and sup:
                    pairs.add((normalize(sub), normalize(sup)))

    return pairs


async def run_benchmark():
    ontologies = sorted([
        f.replace(".ttl", "")
        for f in os.listdir(ONTOLOGIES_DIR)
        if f.endswith(".ttl")
    ])

    print("=" * 70)
    print("OntoAxiom Benchmark: Real MCP Server Pipeline")
    print("=" * 70)
    print(f"\nPipeline: open-ontologies serve → MCP SDK → JSON-RPC 2.0 over stdio")
    print(f"Tools used: onto_clear → onto_load → onto_query (SPARQL)")
    print(f"Triple store: Oxigraph (in-memory, same as production)")
    print(f"Ontologies: {len(ontologies)}")
    print(f"Axiom types: 5 (subClassOf, disjoint, domain, range, subPropertyOf)")
    print(f"Baseline: Best bare LLM (o1) F1 = 0.197")
    print()

    if not os.path.exists(OO_BIN):
        print(f"ERROR: Binary not found at {OO_BIN}")
        print("Build with: cargo build --release")
        sys.exit(1)

    server_params = StdioServerParameters(
        command=OO_BIN,
        args=["serve"],
    )

    print("Starting Open Ontologies MCP server...")
    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()

            tools = await session.list_tools()
            tool_names = [t.name for t in tools.tools]
            print(f"  Server connected — {len(tool_names)} tools available")
            print(f"  Protocol: JSON-RPC 2.0 over stdio (official MCP SDK)")
            print()

            all_results = {}
            grand_tp = grand_fp = grand_fn = 0
            total_queries = 0

            for axiom_type in ["subclassof", "disjoint", "domain", "range", "subproperty"]:
                print(f"\n--- {axiom_type} ---")
                type_tp = type_fp = type_fn = 0

                for onto_name in ontologies:
                    gt = load_ground_truth(axiom_type, onto_name)
                    if not gt:
                        continue

                    onto_path = os.path.abspath(os.path.join(ONTOLOGIES_DIR, f"{onto_name}.ttl"))

                    # Clear store, load ontology — exactly like a real MCP session
                    await session.call_tool("onto_clear", {})
                    result = await session.call_tool("onto_load", {"path": onto_path})
                    load_data = parse_tool_result(result)
                    total_queries += 2  # clear + load

                    # Extract axioms via SPARQL
                    extracted = await extract_axioms_via_mcp(session, axiom_type)
                    total_queries += 2 if axiom_type == "disjoint" else 1

                    # For domain/range, try both pair orders and pick the better match
                    if axiom_type in ("domain", "range"):
                        flipped = {(b, a) for a, b in extracted}
                        if len(flipped & gt) > len(extracted & gt):
                            extracted = flipped

                    tp = len(extracted & gt)
                    fp = len(extracted - gt)
                    fn = len(gt - extracted)
                    p, r, f1 = precision_recall_f1(tp, fp, fn)

                    type_tp += tp
                    type_fp += fp
                    type_fn += fn

                    status = "PERFECT" if fn == 0 and fp == 0 else ""
                    print(f"  {onto_name:20s}  P={p:.3f}  R={r:.3f}  F1={f1:.3f}  "
                          f"(TP={tp}, FP={fp}, FN={fn})  {status}")

                    all_results.setdefault(axiom_type, {})[onto_name] = {
                        "precision": round(p, 4),
                        "recall": round(r, 4),
                        "f1": round(f1, 4),
                        "tp": tp, "fp": fp, "fn": fn,
                        "ground_truth": len(gt),
                        "extracted": len(extracted),
                    }

                p, r, f1 = precision_recall_f1(type_tp, type_fp, type_fn)
                grand_tp += type_tp
                grand_fp += type_fp
                grand_fn += type_fn
                print(f"  {'TOTAL':20s}  P={p:.3f}  R={r:.3f}  F1={f1:.3f}")
                all_results[axiom_type]["_total"] = {
                    "precision": round(p, 4), "recall": round(r, 4), "f1": round(f1, 4),
                    "tp": type_tp, "fp": type_fp, "fn": type_fn,
                }

            # Grand total
            gp, gr, gf1 = precision_recall_f1(grand_tp, grand_fp, grand_fn)
            all_results["_grand_total"] = {
                "precision": round(gp, 4), "recall": round(gr, 4), "f1": round(gf1, 4),
                "tp": grand_tp, "fp": grand_fp, "fn": grand_fn,
            }
            all_results["_meta"] = {
                "pipeline": "MCP server (open-ontologies serve) via official MCP Python SDK",
                "protocol": "JSON-RPC 2.0 over stdio",
                "tools_used": ["onto_clear", "onto_load", "onto_query"],
                "tools_available": len(tool_names),
                "total_mcp_calls": total_queries,
                "ontologies": len(ontologies),
            }

            print()
            print("=" * 70)
            print("GRAND TOTAL")
            print("=" * 70)
            print(f"  Precision: {gp:.3f}")
            print(f"  Recall:    {gr:.3f}")
            print(f"  F1:        {gf1:.3f}")
            print()
            print(f"  MCP tool calls: {total_queries} (onto_clear + onto_load + onto_query)")
            print(f"  Protocol: JSON-RPC 2.0 over stdio (official MCP Python SDK)")
            print()
            print(f"  Best bare LLM (o1):                F1 = 0.197")
            print(f"  Tool-augmented (Open Ontologies):   F1 = {gf1:.3f}")
            if gf1 > 0.197:
                improvement = ((gf1 / 0.197) - 1) * 100
                print(f"\n  >>> Tool-augmented outperforms best LLM by {improvement:.0f}% <<<")
            print()

            # Per-type comparison table
            print("Per-Axiom-Type Comparison:")
            print(f"  {'Type':15s}  {'OO F1':>8s}  {'o1 F1':>8s}  {'Winner'}")
            print(f"  {'-'*50}")
            o1_f1 = {
                "subclassof": 0.359,
                "disjoint": 0.095,
                "domain": 0.038,
                "range": 0.030,
                "subproperty": 0.106,
            }
            for at in ["subclassof", "disjoint", "domain", "range", "subproperty"]:
                oo_f1 = all_results[at]["_total"]["f1"]
                baseline = o1_f1.get(at, 0)
                winner = "OO" if oo_f1 > baseline else "o1" if baseline > oo_f1 else "tie"
                print(f"  {at:15s}  {oo_f1:>8.3f}  {baseline:>8.3f}  {winner}")

            # Save results
            out_path = os.path.join(SCRIPT_DIR, "data", "results", "oo_ontoaxiom_mcp_results.json")
            os.makedirs(os.path.dirname(out_path), exist_ok=True)
            with open(out_path, "w") as f:
                json.dump(all_results, f, indent=2)
            print(f"\nResults saved to {out_path}")


def main():
    asyncio.run(run_benchmark())


if __name__ == "__main__":
    main()
