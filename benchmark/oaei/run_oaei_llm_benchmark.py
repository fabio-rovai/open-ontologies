#!/usr/bin/env python3
"""
OAEI Alignment Benchmark — Hybrid Neuro-Symbolic Mode.

Pipeline:
1. Run onto_align at LOW threshold (0.7) to get candidate pairs
2. High-confidence pairs (>= 0.95) are accepted directly
3. Uncertain pairs (0.7-0.95) are sent to Claude for adjudication
4. Score against OAEI reference

This mirrors what OLaLa does (LLM-based matching) but combines it with
our structural signals for a hybrid approach.

Requirements: pip install mcp anthropic
"""
import asyncio
import json
import os
import sys
import time
from xml.etree import ElementTree as ET

try:
    from mcp import ClientSession, StdioServerParameters
    from mcp.client.stdio import stdio_client
except ImportError:
    print("ERROR: MCP SDK required. Install with: pip install mcp")
    sys.exit(1)

try:
    import anthropic
except ImportError:
    print("ERROR: anthropic SDK required. Install with: pip install anthropic")
    sys.exit(1)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(SCRIPT_DIR, "data")
RESULTS_DIR = os.path.join(SCRIPT_DIR, "results")
OO_BIN = os.path.expanduser("~/.cargo/shared-target/release/open-ontologies")
if not os.path.exists(OO_BIN):
    import shutil
    OO_BIN = shutil.which("open-ontologies") or OO_BIN

# Thresholds
HIGH_CONFIDENCE = 0.95   # Accept directly without LLM
LOW_CONFIDENCE = 0.70    # Below this, reject without LLM
BATCH_SIZE = 50          # Pairs per LLM call


def parse_oaei_reference(rdf_path: str) -> set:
    ALIGN = "{http://knowledgeweb.semanticweb.org/heterogeneity/alignment}"
    RDF = "{http://www.w3.org/1999/02/22-rdf-syntax-ns#}"
    tree = ET.parse(rdf_path)
    mappings = set()
    for cell in tree.getroot().iter(f"{ALIGN}Cell"):
        e1 = cell.find(f"{ALIGN}entity1")
        e2 = cell.find(f"{ALIGN}entity2")
        if e1 is not None and e2 is not None:
            uri1 = e1.get(f"{RDF}resource", "")
            uri2 = e2.get(f"{RDF}resource", "")
            if uri1 and uri2:
                mappings.add((uri1, uri2, "="))
    return mappings


def normalize_uri(uri: str) -> str:
    if "#" in uri:
        return uri.split("#")[-1].lower()
    return uri.rstrip("/").split("/")[-1].lower()


def score(predicted: set, reference: set) -> dict:
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
        "tp": tp, "fp": fp, "fn": fn,
        "reference_size": len(ref_normalized),
        "predicted_size": len(predicted),
    }


def build_llm_prompt(pairs: list) -> str:
    """Build a prompt for Claude to adjudicate alignment candidates."""
    lines = []
    for i, p in enumerate(pairs):
        src_labels = p.get("source_labels", p["source_iri"].split("#")[-1])
        tgt_labels = p.get("target_labels", p["target_iri"].split("#")[-1])
        src_parents = p.get("source_parents", "")
        tgt_parents = p.get("target_parents", "")
        conf = p.get("confidence", 0)
        lines.append(
            f"{i+1}. \"{src_labels}\" (parents: {src_parents}) "
            f"<-> \"{tgt_labels}\" (parents: {tgt_parents}) "
            f"[conf={conf:.2f}]"
        )

    pairs_text = "\n".join(lines)
    return f"""You are an expert in biomedical ontology alignment. Given pairs of classes from mouse anatomy and human anatomy ontologies, determine which pairs refer to the SAME anatomical concept.

For each pair, respond with ONLY the pair number if they match. If they don't match, skip it.

Pairs:
{pairs_text}

Respond with ONLY a JSON array of matching pair numbers, e.g. [1, 3, 5].
If none match, respond with [].
"""


async def llm_adjudicate(client: anthropic.Anthropic, pairs: list) -> list:
    """Send uncertain pairs to Claude for adjudication. Returns indices of accepted pairs."""
    accepted = []

    # Process in batches
    for batch_start in range(0, len(pairs), BATCH_SIZE):
        batch = pairs[batch_start:batch_start + BATCH_SIZE]
        prompt = build_llm_prompt(batch)

        try:
            response = client.messages.create(
                model="claude-sonnet-4-20250514",
                max_tokens=1024,
                messages=[{"role": "user", "content": prompt}],
            )
            text = response.content[0].text.strip()

            # Parse JSON array from response
            # Handle cases where Claude wraps in markdown
            if "```" in text:
                text = text.split("```")[1]
                if text.startswith("json"):
                    text = text[4:]
            text = text.strip()

            indices = json.loads(text)
            if isinstance(indices, list):
                for idx in indices:
                    if isinstance(idx, int) and 1 <= idx <= len(batch):
                        accepted.append(batch_start + idx - 1)
        except Exception as e:
            print(f"    [LLM ERROR] {e}")

    return accepted


async def call_tool(session, name: str, args: dict) -> str:
    result = await session.call_tool(name, args)
    if result.content:
        return result.content[0].text
    return ""


async def main():
    os.makedirs(RESULTS_DIR, exist_ok=True)

    if not os.path.exists(OO_BIN):
        print(f"ERROR: Binary not found at {OO_BIN}")
        sys.exit(1)

    anatomy_dir = os.path.join(DATA_DIR, "anatomy")
    mouse_path = os.path.join(anatomy_dir, "mouse.owl")
    human_path = os.path.join(anatomy_dir, "human.owl")
    ref_path = os.path.join(anatomy_dir, "reference.rdf")

    if not all(os.path.exists(p) for p in [mouse_path, human_path, ref_path]):
        print("ERROR: Anatomy data not found. Run download_oaei.py first.")
        sys.exit(1)

    # Load reference
    reference = parse_oaei_reference(ref_path)
    print(f"Reference: {len(reference)} mappings")

    # Initialize Anthropic client
    client = anthropic.Anthropic()
    print("Claude API initialized.")

    # Step 1: Run onto_align at low threshold
    print("\n=== Step 1: Structural alignment (onto_align, threshold=0.70) ===")
    server_params = StdioServerParameters(command=OO_BIN, args=["serve"])

    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            tools = await session.list_tools()
            print(f"MCP server connected. {len(tools.tools)} tools.")

            await call_tool(session, "onto_clear", {})
            await call_tool(session, "onto_load", {"path": mouse_path})

            start = time.time()
            align_result = await call_tool(session, "onto_align", {
                "source": mouse_path,
                "target": human_path,
                "min_confidence": LOW_CONFIDENCE,
                "dry_run": True,
            })
            align_time = time.time() - start
            print(f"Structural alignment completed in {align_time:.1f}s")

            data = json.loads(align_result)
            all_candidates = data.get("candidates", [])
            print(f"Total candidates: {len(all_candidates)}")

            # Also get labels and parents for LLM context
            # Query mouse labels
            mouse_labels = {}
            r = await call_tool(session, "onto_query", {
                "query": """
                    SELECT ?c ?label WHERE {
                        ?c a <http://www.w3.org/2002/07/owl#Class> ;
                           <http://www.w3.org/2000/01/rdf-schema#label> ?label .
                    }
                """
            })
            for row in json.loads(r).get("results", []):
                iri = row.get("class", "").strip("<>")
                label = row.get("label", "").strip('"')
                if iri:
                    mouse_labels[iri] = label

            # Load human and get labels
            await call_tool(session, "onto_clear", {})
            await call_tool(session, "onto_load", {"path": human_path})
            human_labels = {}
            r = await call_tool(session, "onto_query", {
                "query": """
                    SELECT ?c ?label WHERE {
                        ?c a <http://www.w3.org/2002/07/owl#Class> ;
                           <http://www.w3.org/2000/01/rdf-schema#label> ?label .
                    }
                """
            })
            for row in json.loads(r).get("results", []):
                iri = row.get("class", "").strip("<>")
                label = row.get("label", "").strip('"')
                if iri:
                    human_labels[iri] = label

    # Step 2: Split into high-confidence (accept) and uncertain (LLM)
    print(f"\n=== Step 2: Split candidates ===")
    accepted_pairs = set()
    uncertain = []

    for c in all_candidates:
        conf = c.get("confidence", 0)
        src = c["source_iri"]
        tgt = c["target_iri"]

        if conf >= HIGH_CONFIDENCE:
            accepted_pairs.add((normalize_uri(src), normalize_uri(tgt)))
        elif conf >= LOW_CONFIDENCE:
            # Enrich with labels for LLM
            c["source_labels"] = mouse_labels.get(src, src.split("#")[-1])
            c["target_labels"] = human_labels.get(tgt, tgt.split("#")[-1])
            uncertain.append(c)

    print(f"High-confidence (auto-accept): {len(accepted_pairs)}")
    print(f"Uncertain (send to LLM): {len(uncertain)}")

    # Step 3: LLM adjudication
    print(f"\n=== Step 3: LLM adjudication ({len(uncertain)} pairs) ===")
    llm_start = time.time()
    llm_accepted_indices = await llm_adjudicate(client, uncertain)
    llm_time = time.time() - llm_start
    print(f"LLM adjudication completed in {llm_time:.1f}s")
    print(f"LLM accepted: {len(llm_accepted_indices)} of {len(uncertain)}")

    for idx in llm_accepted_indices:
        c = uncertain[idx]
        accepted_pairs.add((normalize_uri(c["source_iri"]), normalize_uri(c["target_iri"])))

    # Step 4: Score
    print(f"\n=== Step 4: Final scoring ===")
    print(f"Total predicted: {len(accepted_pairs)}")

    scores = score(accepted_pairs, reference)
    scores["align_time_s"] = round(align_time, 2)
    scores["llm_time_s"] = round(llm_time, 2)
    scores["llm_calls"] = (len(uncertain) + BATCH_SIZE - 1) // BATCH_SIZE
    scores["auto_accepted"] = len(accepted_pairs) - len(llm_accepted_indices)
    scores["llm_accepted"] = len(llm_accepted_indices)

    print(f"Precision: {scores['precision']:.3f}")
    print(f"Recall:    {scores['recall']:.3f}")
    print(f"F1:        {scores['f1']:.3f}")
    print(f"TP: {scores['tp']}, FP: {scores['fp']}, FN: {scores['fn']}")

    # Comparison
    print(f"\n=== Comparison ===")
    print(f"{'System':<25} {'P':>8} {'R':>8} {'F1':>8}")
    print("-" * 50)
    print(f"{'AML':<25} {'0.950':>8} {'0.922':>8} {'0.936':>8}")
    print(f"{'BERTMap':<25} {'0.932':>8} {'0.916':>8} {'0.924':>8}")
    print(f"{'LogMap':<25} {'0.921':>8} {'0.903':>8} {'0.912':>8}")
    print(f"{'OLaLa (GPT-4)':<25} {'0.870':>8} {'0.910':>8} {'0.890':>8}")
    print(f"{'OO (structural only)':<25} {'0.897':>8} {'0.708':>8} {'0.791':>8}")
    print(f"{'OO + Claude (hybrid)':<25} {scores['precision']:>8.3f} {scores['recall']:>8.3f} {scores['f1']:>8.3f}")

    # Save
    results_path = os.path.join(RESULTS_DIR, "oaei_llm_results.json")
    with open(results_path, "w") as f:
        json.dump(scores, f, indent=2)
    print(f"\nResults saved to {results_path}")


if __name__ == "__main__":
    asyncio.run(main())
