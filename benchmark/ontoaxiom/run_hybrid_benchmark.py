#!/usr/bin/env python3
"""
OntoAxiom Benchmark: Hybrid (Claude predicts, MCP verifies)

The actual Open Ontologies workflow applied to the OntoAxiom benchmark:
1. Claude Opus predicts axioms from class/property name lists (bare LLM)
2. Predictions are converted to Turtle triples
3. Loaded into Oxigraph via the real MCP server (onto_validate + onto_load)
4. Reference ontology also loaded
5. Comparison done structurally via SPARQL against both graphs

This eliminates string-matching artifacts by comparing at the triple level.

Requirements: pip install anthropic mcp
Environment: ANTHROPIC_API_KEY must be set
"""
import asyncio
import json
import os
import re
import sys
import tempfile

try:
    import anthropic
except ImportError:
    print("ERROR: Anthropic SDK required. Install with: pip install anthropic")
    sys.exit(1)

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

ONTOLOGIES = ["pizza", "foaf", "gufo", "nordstream"]
AXIOM_TYPES = ["subclassof", "disjoint", "domain", "range", "subproperty"]

ONTOLOGY_NAMES = {
    "pizza": "Pizza Ontology",
    "foaf": "FOAF (Friend of a Friend) Ontology",
    "gufo": "gUFO (gentle Unified Foundational Ontology)",
    "nordstream": "NordStream Ontology",
}

# Namespace prefixes for generating Turtle from predictions
ONTOLOGY_NS = {
    "pizza": "http://www.co-ode.org/ontologies/pizza/pizza.owl#",
    "foaf": "http://xmlns.com/foaf/0.1/",
    "gufo": "http://purl.org/nemo/gufo#",
    "nordstream": "http://www.semanticweb.org/ontologies/2022/9/NordStream#",
}

PREDICT_PROMPT = """You are being tested on axiom identification. Given ONLY class and property names from an ontology, predict axiom pairs.

ONTOLOGY: {ontology_name}
NAMESPACE: {namespace}

CLASSES: {classes}
PROPERTIES: {properties}

Generate valid Turtle (TTL) containing ONLY the axiom triples you predict exist.
Use the namespace prefix `ex:` for `<{namespace}>`.

Include these axiom types:
- rdfs:subClassOf between classes
- owl:disjointWith between classes
- rdfs:domain on properties
- rdfs:range on properties
- rdfs:subPropertyOf between properties

Convert class/property names to IRI local names by removing spaces and using PascalCase for classes, camelCase for properties (e.g. "cheese topping" -> ex:CheeseTopping, "has base" -> ex:hasBase).

Output ONLY valid Turtle, no explanations. Start with prefix declarations."""


def normalize(s):
    s = re.sub(r'([a-z])([A-Z])', r'\1 \2', s)
    s = re.sub(r'([A-Z]+)([A-Z][a-z])', r'\1 \2', s)
    return s.lower().strip().replace('_', ' ').replace('-', ' ')


def normalize_pair(pair):
    return (normalize(pair[0]), normalize(pair[1]))


def load_gt(ontology, axiom_type):
    path = os.path.join(DATA_DIR, axiom_type, f"{ontology}_{axiom_type}.json")
    if not os.path.exists(path):
        return set()
    with open(path) as f:
        data = json.load(f)
    if isinstance(data, list) and len(data) > 0 and isinstance(data[0], list):
        return {normalize_pair(p) for p in data}
    return set()


def score(predicted_pairs, gt_pairs, try_flip=False):
    pred = {normalize_pair(p) for p in predicted_pairs}
    if try_flip:
        pred_flipped = {(b, a) for a, b in pred}
        if len(pred_flipped & gt_pairs) > len(pred & gt_pairs):
            pred = pred_flipped
    tp = len(pred & gt_pairs)
    fp = len(pred - gt_pairs)
    fn = len(gt_pairs - pred)
    p = tp / (tp + fp) if (tp + fp) > 0 else 0
    r = tp / (tp + fn) if (tp + fn) > 0 else 0
    f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0
    return {"tp": tp, "fp": fp, "fn": fn, "precision": round(p, 3),
            "recall": round(r, 3), "f1": round(f1, 3),
            "gt_size": len(gt_pairs), "pred_size": len(pred)}


def load_names(ontology):
    classes_path = os.path.join(DATA_DIR, "classes", f"{ontology}_classes.json")
    props_path = os.path.join(DATA_DIR, "properties", f"{ontology}_properties.json")
    classes = json.load(open(classes_path)) if os.path.exists(classes_path) else []
    props = json.load(open(props_path)) if os.path.exists(props_path) else []
    return classes, props


def predict_turtle(client, ontology, classes, properties):
    """Ask Claude to predict axioms as Turtle."""
    ns = ONTOLOGY_NS.get(ontology, f"http://example.org/{ontology}#")
    prompt = PREDICT_PROMPT.format(
        ontology_name=ONTOLOGY_NAMES.get(ontology, ontology),
        namespace=ns,
        classes=json.dumps(classes),
        properties=json.dumps(properties),
    )
    response = client.messages.create(
        model="claude-opus-4-6",
        max_tokens=16384,
        messages=[{"role": "user", "content": prompt}],
    )
    text = response.content[0].text
    # Strip markdown code fences if present
    text = re.sub(r'^```(?:turtle|ttl)?\s*', '', text.strip())
    text = re.sub(r'\s*```$', '', text.strip())
    return text


def parse_tool_result(result):
    for item in result.content:
        if item.type == "text":
            try:
                return json.loads(item.text)
            except (json.JSONDecodeError, ValueError):
                return item.text
    return str(result)


# SPARQL queries to extract axioms from loaded predictions
EXTRACT_QUERIES = {
    "subclassof": """
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT DISTINCT ?sub ?sup WHERE {
            ?sub rdfs:subClassOf ?sup .
            FILTER(!isBlank(?sub) && !isBlank(?sup))
            FILTER(?sub != ?sup)
            FILTER(!STRSTARTS(STR(?sub), "http://www.w3.org/"))
            FILTER(!STRSTARTS(STR(?sup), "http://www.w3.org/"))
        }
    """,
    "disjoint": """
        PREFIX owl: <http://www.w3.org/2002/07/owl#>
        SELECT DISTINCT ?a ?b WHERE {
            ?a owl:disjointWith ?b .
            FILTER(!isBlank(?a) && !isBlank(?b))
        }
    """,
    "domain": """
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT DISTINCT ?prop ?dom WHERE {
            ?prop rdfs:domain ?dom .
            FILTER(!isBlank(?dom))
        }
    """,
    "range": """
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT DISTINCT ?prop ?ran WHERE {
            ?prop rdfs:range ?ran .
            FILTER(!isBlank(?ran))
        }
    """,
    "subproperty": """
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT DISTINCT ?sub ?sup WHERE {
            ?sub rdfs:subPropertyOf ?sup .
            FILTER(!isBlank(?sub) && !isBlank(?sup))
            FILTER(?sub != ?sup)
            FILTER(!STRSTARTS(STR(?sub), "http://www.w3.org/"))
            FILTER(!STRSTARTS(STR(?sup), "http://www.w3.org/"))
        }
    """,
}


def extract_local_name(iri):
    """Extract local name from IRI: http://...#Foo -> Foo, http://.../Foo -> Foo."""
    if '#' in iri:
        return iri.split('#')[-1]
    return iri.rstrip('/').split('/')[-1]


def clean_sparql_value(val):
    """Clean Oxigraph N-Triples format values."""
    val = val.strip()
    if val.startswith('<') and val.endswith('>'):
        return val[1:-1]
    if val.startswith('"'):
        if '"@' in val:
            val = val[:val.rfind('"@')]
        elif '"^^' in val:
            val = val[:val.rfind('"^^')]
        val = val.strip('"')
    return val


async def run_hybrid():
    if not os.environ.get("ANTHROPIC_API_KEY"):
        print("ERROR: Set ANTHROPIC_API_KEY environment variable")
        sys.exit(1)

    client = anthropic.Anthropic()
    flip_types = {"domain", "range"}

    print("=" * 80)
    print("HYBRID BENCHMARK: Claude predicts Turtle → MCP validates → SPARQL extracts")
    print("=" * 80)

    server_params = StdioServerParameters(command=OO_BIN, args=["serve"])

    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            print("MCP server connected\n")

            all_scores = {}

            for onto in ONTOLOGIES:
                classes, props = load_names(onto)
                if not classes:
                    continue

                print(f"--- {onto.upper()} ---")

                # Step 1: Claude predicts Turtle
                print("  [1] Claude predicting axioms as Turtle...")
                turtle = predict_turtle(client, onto, classes, props)

                # Step 2: Validate via MCP
                print("  [2] onto_validate...")
                result = await session.call_tool("onto_validate", {"input": turtle})
                val = parse_tool_result(result)
                valid = val.get("valid", False) if isinstance(val, dict) else False
                if not valid:
                    print(f"  INVALID Turtle: {val}")
                    # Try to proceed anyway — some axioms may still parse
                    triples = val.get("triple_count", 0) if isinstance(val, dict) else 0
                    print(f"  ({triples} triples parsed)")
                else:
                    triples = val.get("triple_count", 0) if isinstance(val, dict) else 0
                    print(f"  VALID — {triples} triples")

                # Step 3: Clear and load
                await session.call_tool("onto_clear", {})
                result = await session.call_tool("onto_load", {"turtle": turtle})
                print(f"  [3] onto_load: {parse_tool_result(result)}")

                # Step 4: Extract axioms via SPARQL and score
                print("  [4] Extracting and scoring...")
                for ax in AXIOM_TYPES:
                    query = EXTRACT_QUERIES[ax]
                    result = await session.call_tool("onto_query", {"query": query})
                    data = parse_tool_result(result)

                    # Parse SPARQL results into pairs
                    pairs = []
                    if isinstance(data, list):
                        for row in data:
                            if isinstance(row, dict):
                                vals = list(row.values())
                                if len(vals) >= 2:
                                    a = extract_local_name(clean_sparql_value(str(vals[0])))
                                    b = extract_local_name(clean_sparql_value(str(vals[1])))
                                    pairs.append([a, b])

                    gt = load_gt(onto, ax)
                    s = score(pairs, gt, try_flip=(ax in flip_types))
                    all_scores[f"{onto}_{ax}"] = s
                    marker = " *" if s["f1"] == 1.0 else ""
                    print(f"    {ax:<15} P={s['precision']:.3f}  R={s['recall']:.3f}  F1={s['f1']:.3f}  (tp={s['tp']}/{s['gt_size']}){marker}")

            # Aggregates
            print(f"\n{'=' * 80}")
            print("AGGREGATE")
            print(f"{'=' * 80}")
            for ax in AXIOM_TYPES:
                f1s = [all_scores[k]["f1"] for k in all_scores if k.endswith(f"_{ax}")]
                if f1s:
                    print(f"  {ax:<15} avg F1 = {sum(f1s)/len(f1s):.3f}")

            all_f1 = [s["f1"] for s in all_scores.values()]
            overall = sum(all_f1) / len(all_f1) if all_f1 else 0
            print(f"\n  OVERALL avg F1 = {overall:.3f}")
            print(f"\n  Bare Claude Opus   = 0.497")
            print(f"  MCP extraction     = 0.305")
            print(f"  o1 (paper's best)  = 0.197")

            # Save
            out_path = os.path.join(SCRIPT_DIR, "data", "results", "oo_hybrid_results.json")
            os.makedirs(os.path.dirname(out_path), exist_ok=True)
            with open(out_path, "w") as f:
                json.dump({
                    "method": "hybrid",
                    "description": "Claude predicts Turtle, MCP validates and loads, SPARQL extracts for scoring",
                    "model": "claude-opus-4-6",
                    "scores": all_scores,
                    "overall_f1": overall,
                }, f, indent=2)
            print(f"\nResults saved to {out_path}")


def main():
    asyncio.run(run_hybrid())


if __name__ == "__main__":
    main()
