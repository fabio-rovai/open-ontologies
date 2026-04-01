#!/usr/bin/env python3
"""
OAEI Alignment Benchmark for Open Ontologies.

Evaluates onto_align against OAEI standard tracks (Anatomy, Conference)
and compares with published results from LogMap, AML, BERTMap, OLaLa.

Pipeline per pair:
    onto_clear → onto_load(source) → onto_load(target) → onto_embed → onto_align → score

Requirements: pip install mcp lxml
"""
import asyncio
import json
import os
import re
import sys
import time
from collections import defaultdict
from xml.etree import ElementTree as ET

try:
    from mcp import ClientSession, StdioServerParameters
    from mcp.client.stdio import stdio_client
except ImportError:
    print("ERROR: MCP SDK required. Install with: pip install mcp")
    sys.exit(1)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(SCRIPT_DIR, "data")
RESULTS_DIR = os.path.join(SCRIPT_DIR, "results")
OO_BIN = os.path.abspath(
    os.path.join(SCRIPT_DIR, "..", "..", "target", "release", "open-ontologies")
)
# Fallback: check shared Cargo target or PATH
if not os.path.exists(OO_BIN):
    shared = os.path.expanduser("~/.cargo/shared-target/release/open-ontologies")
    if os.path.exists(shared):
        OO_BIN = shared
    else:
        import shutil
        found = shutil.which("open-ontologies")
        if found:
            OO_BIN = found

# Published OAEI 2023.5 results for comparison
PUBLISHED_RESULTS = {
    "anatomy": {
        "LogMap":  {"precision": 0.921, "recall": 0.903, "f1": 0.912},
        "AML":     {"precision": 0.950, "recall": 0.922, "f1": 0.936},
        "BERTMap": {"precision": 0.932, "recall": 0.916, "f1": 0.924},
        "OLaLa":   {"precision": 0.870, "recall": 0.910, "f1": 0.890},
    },
    "conference": {
        "LogMap":  {"precision": 0.720, "recall": 0.620, "f1": 0.670},
        "AML":     {"precision": 0.740, "recall": 0.630, "f1": 0.680},
        "BERTMap": {"precision": 0.700, "recall": 0.720, "f1": 0.710},
        "OLaLa":   {"precision": 0.690, "recall": 0.750, "f1": 0.720},
    },
}


def parse_oaei_reference(rdf_path: str) -> set:
    """
    Parse OAEI reference alignment (RDF/XML format).
    Returns set of (entity1_uri, entity2_uri, relation) tuples.
    """
    if not os.path.exists(rdf_path):
        return set()

    tree = ET.parse(rdf_path)
    root = tree.getroot()

    # OAEI uses this namespace (without trailing #)
    ALIGN = "{http://knowledgeweb.semanticweb.org/heterogeneity/alignment}"
    RDF = "{http://www.w3.org/1999/02/22-rdf-syntax-ns#}"

    mappings = set()

    for cell in root.iter(f"{ALIGN}Cell"):
        e1 = cell.find(f"{ALIGN}entity1")
        e2 = cell.find(f"{ALIGN}entity2")
        rel = cell.find(f"{ALIGN}relation")

        if e1 is not None and e2 is not None:
            uri1 = e1.get(f"{RDF}resource", "")
            uri2 = e2.get(f"{RDF}resource", "")
            relation = rel.text.strip() if rel is not None and rel.text else "="
            if uri1 and uri2:
                mappings.add((uri1, uri2, relation))

    return mappings


def normalize_uri(uri: str) -> str:
    """Extract local name from URI for comparison."""
    if "#" in uri:
        return uri.split("#")[-1].lower()
    return uri.rstrip("/").split("/")[-1].lower()


def parse_align_result(result_json: str) -> set:
    """
    Parse onto_align output into set of (source_local, target_local) pairs.
    onto_align returns JSON with candidates array.
    """
    try:
        data = json.loads(result_json)
    except json.JSONDecodeError:
        return set()

    pairs = set()

    # Handle various output formats from onto_align
    candidates = data if isinstance(data, list) else data.get("candidates", data.get("alignments", []))

    for candidate in candidates:
        if isinstance(candidate, dict):
            src = candidate.get("source", candidate.get("source_iri", ""))
            tgt = candidate.get("target", candidate.get("target_iri", ""))
            if src and tgt:
                pairs.add((normalize_uri(src), normalize_uri(tgt)))

    return pairs


def score(predicted: set, reference: set) -> dict:
    """Compute precision, recall, F1 from predicted and reference pair sets."""
    # Normalize reference pairs to local names for comparison
    ref_normalized = set()
    for uri1, uri2, rel in reference:
        ref_normalized.add((normalize_uri(uri1), normalize_uri(uri2)))

    tp = len(predicted & ref_normalized)
    fp = len(predicted - ref_normalized)
    fn = len(ref_normalized - predicted)

    precision = tp / (tp + fp) if (tp + fp) > 0 else 0.0
    recall = tp / (tp + fn) if (tp + fn) > 0 else 0.0
    f1 = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0.0

    return {
        "precision": round(precision, 4),
        "recall": round(recall, 4),
        "f1": round(f1, 4),
        "tp": tp,
        "fp": fp,
        "fn": fn,
        "reference_size": len(ref_normalized),
        "predicted_size": len(predicted),
    }


async def call_tool(session, name: str, args: dict) -> str:
    """Call an MCP tool and return the text result."""
    result = await session.call_tool(name, args)
    if result.content:
        return result.content[0].text
    return ""


async def run_anatomy_benchmark(session) -> dict:
    """Run OAEI Anatomy track: mouse.owl vs human.owl."""
    anatomy_dir = os.path.join(DATA_DIR, "anatomy")
    mouse_path = os.path.join(anatomy_dir, "mouse.owl")
    human_path = os.path.join(anatomy_dir, "human.owl")
    ref_path = os.path.join(anatomy_dir, "reference.rdf")

    if not all(os.path.exists(p) for p in [mouse_path, human_path, ref_path]):
        print("  [SKIP] Anatomy data not found. Run download_oaei.py first.")
        return {"error": "data not found"}

    print("  Loading reference alignment...")
    reference = parse_oaei_reference(ref_path)
    print(f"  Reference: {len(reference)} mappings")

    print("  Clearing store...")
    await call_tool(session, "onto_clear", {})

    print("  Loading mouse.owl (source) into store...")
    await call_tool(session, "onto_load", {"path": mouse_path})

    # Generate embeddings for better alignment quality
    print("  Generating embeddings (this may take a moment)...")
    try:
        await call_tool(session, "onto_embed", {})
    except Exception as e:
        print(f"  [WARN] Embedding generation failed: {e}")

    print("  Running onto_align (source=mouse, target=human)...")
    start = time.time()
    align_result = await call_tool(session, "onto_align", {
        "source": mouse_path,
        "target": human_path,
        "min_confidence": 0.85,
        "dry_run": True,
    })
    elapsed = time.time() - start
    print(f"  Alignment completed in {elapsed:.1f}s")

    predicted = parse_align_result(align_result)
    print(f"  Predicted: {len(predicted)} mappings")

    scores = score(predicted, reference)
    scores["time_s"] = round(elapsed, 2)

    print(f"  Precision: {scores['precision']:.3f}")
    print(f"  Recall:    {scores['recall']:.3f}")
    print(f"  F1:        {scores['f1']:.3f}")

    return scores


async def run_conference_benchmark(session) -> dict:
    """Run OAEI Conference track: 7 ontologies, 21 pairwise alignments."""
    conf_dir = os.path.join(DATA_DIR, "conference")
    onto_dir = os.path.join(conf_dir, "ontologies")
    ref_dir = os.path.join(conf_dir, "reference")

    if not os.path.exists(onto_dir):
        print("  [SKIP] Conference data not found. Run download_oaei.py first.")
        return {"error": "data not found"}

    ontologies = sorted([f for f in os.listdir(onto_dir) if f.endswith(".owl")])
    names = [o.replace(".owl", "") for o in ontologies]

    all_scores = []
    total_tp, total_fp, total_fn = 0, 0, 0

    for i, name1 in enumerate(names):
        for name2 in names[i + 1:]:
            ref_file = os.path.join(ref_dir, f"{name1}-{name2}.rdf")
            if not os.path.exists(ref_file):
                continue

            source_path = os.path.join(onto_dir, f"{name1}.owl")
            target_path = os.path.join(onto_dir, f"{name2}.owl")

            print(f"  Aligning {name1} <-> {name2}...")

            reference = parse_oaei_reference(ref_file)
            if not reference:
                print(f"    [SKIP] No reference mappings found")
                continue

            await call_tool(session, "onto_clear", {})
            await call_tool(session, "onto_load", {"path": source_path})

            try:
                align_result = await call_tool(session, "onto_align", {
                    "source": source_path,
                    "target": target_path,
                    "min_confidence": 0.85,
                })
                predicted = parse_align_result(align_result)
            except Exception as e:
                print(f"    [ERROR] {e}")
                predicted = set()

            pair_scores = score(predicted, reference)
            pair_scores["pair"] = f"{name1}-{name2}"
            all_scores.append(pair_scores)

            total_tp += pair_scores["tp"]
            total_fp += pair_scores["fp"]
            total_fn += pair_scores["fn"]

            print(f"    P={pair_scores['precision']:.3f} R={pair_scores['recall']:.3f} F1={pair_scores['f1']:.3f}")

    # Micro-averaged totals
    precision = total_tp / (total_tp + total_fp) if (total_tp + total_fp) > 0 else 0.0
    recall = total_tp / (total_tp + total_fn) if (total_tp + total_fn) > 0 else 0.0
    f1 = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0.0

    return {
        "pairs": all_scores,
        "micro_avg": {
            "precision": round(precision, 4),
            "recall": round(recall, 4),
            "f1": round(f1, 4),
            "tp": total_tp,
            "fp": total_fp,
            "fn": total_fn,
        },
    }


async def main():
    os.makedirs(RESULTS_DIR, exist_ok=True)

    if not os.path.exists(OO_BIN):
        print(f"ERROR: Binary not found at {OO_BIN}")
        print("Run: cargo build --release")
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
            print(f"Connected. {len(tools.tools)} tools available.\n")

            # Check if onto_align is available
            tool_names = [t.name for t in tools.tools]
            if "onto_align" not in tool_names:
                print("ERROR: onto_align tool not found. Ensure alignment feature is compiled.")
                sys.exit(1)

            results = {
                "_meta": {
                    "benchmark": "OAEI 2023",
                    "system": "Open Ontologies (onto_align)",
                    "tools_used": ["onto_clear", "onto_load", "onto_embed", "onto_align"],
                    "date": time.strftime("%Y-%m-%d"),
                },
            }

            # Anatomy track
            print("=" * 60)
            print("OAEI Anatomy Track (Mouse <-> Human)")
            print("=" * 60)
            results["anatomy"] = await run_anatomy_benchmark(session)

            # Conference track
            print("\n" + "=" * 60)
            print("OAEI Conference Track (7 ontologies, pairwise)")
            print("=" * 60)
            results["conference"] = await run_conference_benchmark(session)

            # Add published results for comparison
            results["published"] = PUBLISHED_RESULTS

            # Save results
            results_path = os.path.join(RESULTS_DIR, "oaei_results.json")
            with open(results_path, "w") as f:
                json.dump(results, f, indent=2)
            print(f"\nResults saved to {results_path}")

            # Print comparison table
            print("\n" + "=" * 60)
            print("COMPARISON WITH PUBLISHED RESULTS")
            print("=" * 60)

            for track in ["anatomy", "conference"]:
                print(f"\n{track.upper()} TRACK:")
                print(f"{'System':<20} {'Precision':>10} {'Recall':>10} {'F1':>10}")
                print("-" * 50)

                # Published results
                for system, scores in PUBLISHED_RESULTS.get(track, {}).items():
                    print(f"{system:<20} {scores['precision']:>10.3f} {scores['recall']:>10.3f} {scores['f1']:>10.3f}")

                # Our results
                our = results.get(track, {})
                if track == "conference" and "micro_avg" in our:
                    our = our["micro_avg"]
                if "f1" in our:
                    print(f"{'Open Ontologies':<20} {our['precision']:>10.3f} {our['recall']:>10.3f} {our['f1']:>10.3f}")


if __name__ == "__main__":
    asyncio.run(main())
