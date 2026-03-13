#!/usr/bin/env python3
"""
OntoAxiom Benchmark: Bare LLM (no tools)

Replicates the OntoAxiom paper's methodology using Claude Opus:
gives the LLM only class/property name lists and asks it to predict
axiom pairs. No ontology files, no MCP tools, no SPARQL.

This is an apples-to-apples comparison against the paper's LLM results.

Requirements: pip install anthropic
Environment: ANTHROPIC_API_KEY must be set
"""
import json
import os
import re
import sys

try:
    import anthropic
except ImportError:
    print("ERROR: Anthropic SDK required. Install with: pip install anthropic")
    sys.exit(1)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(SCRIPT_DIR, "data", "ontoaxiom")

ONTOLOGIES = ["pizza", "foaf", "gufo", "nordstream", "era", "goodrelations", "music", "saref", "time"]
AXIOM_TYPES = ["subclassof", "disjoint", "domain", "range", "subproperty"]

ONTOLOGY_NAMES = {
    "pizza": "Pizza Ontology",
    "foaf": "FOAF (Friend of a Friend) Ontology",
    "gufo": "gUFO (gentle Unified Foundational Ontology)",
    "nordstream": "NordStream Ontology (about the Nord Stream pipeline events)",
    "era": "ERA (European Union Agency for Railways) Ontology",
    "goodrelations": "GoodRelations (e-commerce) Ontology",
    "music": "Music Ontology",
    "saref": "SAREF (Smart Appliances REFerence) Ontology",
    "time": "OWL-Time Ontology",
}

PROMPT_TEMPLATE = """You are being tested on axiom identification from an ontology. You will be given ONLY class names and property names. You must identify axiom pairs based on your knowledge alone — no tools, no files, no lookups.

ONTOLOGY: {ontology_name}

CLASSES: {classes}

PROPERTIES: {properties}

For each axiom type below, return ALL pairs you can identify. Output ONLY valid JSON, no explanations.

Format your response as a single JSON object with these keys:
- "subclassof": [[sub, super], ...] — subclass relationships
- "disjoint": [[class1, class2], ...] — pairs of disjoint classes
- "domain": [[property, domain_class], ...] — property domain declarations
- "range": [[property, range_class], ...] — property range declarations
- "subproperty": [[sub_prop, super_prop], ...] — subproperty relationships

Be exhaustive. List EVERY pair you believe exists. Output ONLY the JSON object, nothing else."""


def normalize(s):
    """Normalize: split camelCase, lowercase, normalize separators."""
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


def call_claude(client, ontology, classes, properties):
    """Call Claude Opus with name lists only, return predicted axioms."""
    prompt = PROMPT_TEMPLATE.format(
        ontology_name=ONTOLOGY_NAMES.get(ontology, ontology),
        classes=json.dumps(classes),
        properties=json.dumps(properties),
    )
    response = client.messages.create(
        model="claude-opus-4-6",
        max_tokens=8192,
        messages=[{"role": "user", "content": prompt}],
    )
    text = response.content[0].text
    # Extract JSON from response (may be wrapped in ```json blocks)
    text = re.sub(r'^```json\s*', '', text.strip())
    text = re.sub(r'\s*```$', '', text.strip())
    return json.loads(text)


def main():
    if not os.environ.get("ANTHROPIC_API_KEY"):
        print("ERROR: Set ANTHROPIC_API_KEY environment variable")
        sys.exit(1)

    client = anthropic.Anthropic()
    flip_types = {"domain", "range"}
    all_scores = {}
    all_results = {}

    print("=" * 80)
    print("BARE CLAUDE OPUS — OntoAxiom Benchmark")
    print("Same input as paper: class/property name lists only, no tools")
    print("=" * 80)

    for onto in ONTOLOGIES:
        classes, props = load_names(onto)
        if not classes:
            print(f"\n  {onto}: skipped (no class data)")
            continue

        print(f"\n--- {onto.upper()} ({len(classes)} classes, {len(props)} properties) ---")
        try:
            result = call_claude(client, onto, classes, props)
        except Exception as e:
            print(f"  ERROR: {e}")
            continue

        all_results[onto] = result
        key_map = {"subclassof": "subclassof", "disjoint": "disjoint",
                    "domain": "domain", "range": "range", "subproperty": "subproperty"}

        for ax in AXIOM_TYPES:
            gt = load_gt(onto, ax)
            pred = result.get(ax, [])
            s = score(pred, gt, try_flip=(ax in flip_types))
            all_scores[f"{onto}_{ax}"] = s
            marker = " *" if s["f1"] == 1.0 else ""
            print(f"  {ax:<15} P={s['precision']:.3f}  R={s['recall']:.3f}  F1={s['f1']:.3f}  (tp={s['tp']}/{s['gt_size']}){marker}")

    # Aggregates
    print(f"\n{'=' * 80}")
    print("AGGREGATE BY AXIOM TYPE")
    print(f"{'=' * 80}")
    for ax in AXIOM_TYPES:
        f1s = [all_scores[k]["f1"] for k in all_scores if k.endswith(f"_{ax}")]
        if f1s:
            avg = sum(f1s) / len(f1s)
            print(f"  {ax:<15} avg F1 = {avg:.3f}")

    all_f1 = [s["f1"] for s in all_scores.values()]
    overall = sum(all_f1) / len(all_f1) if all_f1 else 0
    print(f"\n  OVERALL avg F1 = {overall:.3f}")
    print(f"  o1 (paper's best)  = 0.197")

    # Save results
    out_path = os.path.join(SCRIPT_DIR, "data", "results", "oo_bare_opus_results.json")
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w") as f:
        json.dump({
            "method": "bare_llm",
            "model": "claude-opus-4-6",
            "input": "class/property name lists only (same as OntoAxiom paper)",
            "tools_used": 0,
            "scores": all_scores,
            "overall_f1": overall,
            "predictions": {k: {ax: v.get(ax, []) for ax in AXIOM_TYPES}
                           for k, v in all_results.items()},
        }, f, indent=2)
    print(f"\nResults saved to {out_path}")


if __name__ == "__main__":
    main()
