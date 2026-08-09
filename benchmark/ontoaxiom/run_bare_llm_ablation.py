#!/usr/bin/env python3
"""
OntoAxiom Benchmark: Bare LLM (no tools) — CROSS-MODEL ABLATION

Same task and scoring as run_bare_llm_benchmark.py (class/property name lists
only, no ontology files, no MCP tools), but runnable against a *second* model —
a local Qwen3-Coder-30B served over an OpenAI-compatible endpoint — as well as
Claude. This exists to answer a reviewer question: is the bare-LLM baseline (and
the gain the MCP tools add on top of it) a property of the *tooling*, or just of
one vendor's model? Run this for the bare baseline on Qwen; run run_mcp_benchmark.py
for the tool-augmented condition.

Backends:
  --backend qwen    local MLX/OpenAI-compatible server (default; free, no tokens)
  --backend claude  Anthropic API (needs ANTHROPIC_API_KEY + `pip install anthropic`)

Examples:
  python3 run_bare_llm_ablation.py --backend qwen
  python3 run_bare_llm_ablation.py --backend qwen \
      --model mlx-community/Qwen3-Coder-30B-A3B-Instruct-8bit
"""
import argparse
import json
import os
import re
import sys
import urllib.request

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(SCRIPT_DIR, "data", "ontoaxiom")

ONTOLOGIES = ["pizza", "foaf", "gufo", "nordstream", "era", "goodrelations", "music", "saref", "time"]
AXIOM_TYPES = ["subclassof", "disjoint", "domain", "range", "subproperty"]

# 8192 truncated era/music mid-JSON. Their exhaustive answers run past 30k chars.
MAX_TOKENS = 32768

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


def local_name(s):
    """Reduce an IRI, QName or bare name to its local part.

    Condition A hands the model bare names so it echoes bare names, but in
    condition D the model reads real Turtle and answers in QNames (foaf:Person,
    mo:Arranger, :DateTimeDescription). Ground truth is stored bare, so without
    this the two never intersect and the ontology scores a spurious 0.0.
    """
    s = s.strip()
    if s.startswith('<') and s.endswith('>'):
        s = s[1:-1]
    if '#' in s:
        s = s.rsplit('#', 1)[1]
    elif s.startswith(('http://', 'https://')):
        s = s.rstrip('/').rsplit('/', 1)[1]
    elif ':' in s:
        s = s.rsplit(':', 1)[1]
    return s


def normalize(s):
    s = local_name(s)
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
    pred = set()
    for p in predicted_pairs:
        if isinstance(p, (list, tuple)) and len(p) == 2:
            pred.add(normalize_pair(p))
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


def salvage_pairs(text):
    """Recover complete [a, b] pairs from a JSON object truncated mid-generation.

    A completion cut short by max_tokens still has a valid prefix. Pairs that were
    fully emitted are real predictions and get scored; the unemitted tail counts
    against recall, which is the honest outcome.
    """
    out = {}
    for ax in AXIOM_TYPES:
        m = re.search(r'"%s"\s*:\s*\[' % re.escape(ax), text)
        if not m:
            continue
        pairs, i, depth, buf = [], m.end(), 0, None
        while i < len(text):
            c = text[i]
            if c == '[':
                if depth == 0:
                    buf = i
                depth += 1
            elif c == ']':
                if depth == 0:
                    break
                depth -= 1
                if depth == 0 and buf is not None:
                    try:
                        p = json.loads(text[buf:i + 1])
                        if isinstance(p, list) and len(p) == 2 and all(isinstance(x, str) for x in p):
                            pairs.append(p)
                    except ValueError:
                        pass
                    buf = None
            i += 1
        out[ax] = pairs
    if not any(out.values()):
        raise ValueError("no salvageable pairs in truncated output")
    return out


def extract_json(text, allow_salvage=True):
    """Pull the first {...} JSON object out of a possibly chatty completion."""
    text = text.strip()
    text = re.sub(r'^```(?:json)?\s*', '', text)
    text = re.sub(r'\s*```$', '', text)
    start = text.find('{')
    end = text.rfind('}')
    candidate = text[start:end + 1] if (start != -1 and end != -1 and end > start) else text
    try:
        return json.loads(candidate)
    except ValueError:
        if not allow_salvage:
            raise
        salvaged = salvage_pairs(text[start:] if start != -1 else text)
        salvaged["_salvaged"] = True
        return salvaged


def call_qwen(host, model, ontology, classes, properties, max_tokens=MAX_TOKENS, temp=0.2):
    prompt = PROMPT_TEMPLATE.format(
        ontology_name=ONTOLOGY_NAMES.get(ontology, ontology),
        classes=json.dumps(classes),
        properties=json.dumps(properties),
    )
    body = json.dumps({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
        "temperature": temp,
    }).encode()
    req = urllib.request.Request(
        host.rstrip("/") + "/v1/chat/completions",
        data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=1800) as resp:
        data = json.load(resp)
    choice = data["choices"][0]
    result = extract_json(choice["message"]["content"])
    result["_finish_reason"] = choice.get("finish_reason")
    return result


def call_claude(model, ontology, classes, properties, max_tokens=MAX_TOKENS):
    import anthropic
    client = anthropic.Anthropic()
    prompt = PROMPT_TEMPLATE.format(
        ontology_name=ONTOLOGY_NAMES.get(ontology, ontology),
        classes=json.dumps(classes),
        properties=json.dumps(properties),
    )
    response = client.messages.create(
        model=model, max_tokens=max_tokens,
        messages=[{"role": "user", "content": prompt}],
    )
    result = extract_json(response.content[0].text)
    result["_finish_reason"] = response.stop_reason
    return result


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--backend", choices=["qwen", "claude"], default="qwen")
    ap.add_argument("--host", default=os.environ.get("QWEN_HOST", "http://localhost:8080"))
    ap.add_argument("--model", default=None)
    ap.add_argument("--max-tokens", type=int, default=MAX_TOKENS)
    ap.add_argument("--only", default=None, help="comma-separated ontology subset")
    ap.add_argument("--merge", action="store_true",
                    help="fold results into the existing results file instead of replacing it")
    args = ap.parse_args()

    if args.backend == "qwen":
        model = args.model or "mlx-community/Qwen3-Coder-30B-A3B-Instruct-8bit"
        caller = lambda o, c, p: call_qwen(args.host, model, o, c, p, max_tokens=args.max_tokens)
    else:
        model = args.model or "claude-opus-4-8"
        if not os.environ.get("ANTHROPIC_API_KEY"):
            print("ERROR: Set ANTHROPIC_API_KEY for --backend claude")
            sys.exit(1)
        caller = lambda o, c, p: call_claude(model, o, c, p, max_tokens=args.max_tokens)

    ontologies = [o.strip() for o in args.only.split(",")] if args.only else ONTOLOGIES

    flip_types = {"domain", "range"}
    all_scores, all_results = {}, {}
    truncated = []

    print("=" * 80)
    print(f"BARE LLM — OntoAxiom Ablation   backend={args.backend}  model={model}")
    print(f"Same input as paper: class/property name lists only, no tools   max_tokens={args.max_tokens}")
    print("=" * 80)

    for onto in ontologies:
        classes, props = load_names(onto)
        if not classes:
            print(f"\n  {onto}: skipped (no class data)")
            continue
        print(f"\n--- {onto.upper()} ({len(classes)} classes, {len(props)} properties) ---")
        try:
            result = caller(onto, classes, props)
        except Exception as e:
            print(f"  ERROR: {e}")
            all_results[onto] = {"error": str(e)}
            continue
        all_results[onto] = result
        if result.get("_finish_reason") == "length" or result.get("_salvaged"):
            truncated.append(onto)
            print(f"  !! TRUNCATED at max_tokens={args.max_tokens}"
                  f"{' — scored from salvaged prefix' if result.get('_salvaged') else ''}")
        for ax in AXIOM_TYPES:
            gt = load_gt(onto, ax)
            pred = result.get(ax, []) if isinstance(result, dict) else []
            s = score(pred, gt, try_flip=(ax in flip_types))
            all_scores[f"{onto}_{ax}"] = s
            marker = " *" if s["f1"] == 1.0 else ""
            print(f"  {ax:<15} P={s['precision']:.3f}  R={s['recall']:.3f}  F1={s['f1']:.3f}  (tp={s['tp']}/{s['gt_size']}){marker}")

    tag = args.backend + ("" if args.model is None else "_custom")
    out_path = os.path.join(SCRIPT_DIR, "data", "results", f"oo_bare_{tag}_results.json")
    os.makedirs(os.path.dirname(out_path), exist_ok=True)

    if args.merge and os.path.exists(out_path):
        with open(out_path) as f:
            prior = json.load(f)
        merged_scores = dict(prior.get("scores", {}))
        merged_scores.update(all_scores)
        merged_preds = dict(prior.get("predictions", {}))
        merged_preds.update(all_results)
        rerun = sorted(all_results)
        print(f"\n  merged {len(rerun)} rerun ontologies ({', '.join(rerun)}) into {len(merged_preds)} total")
        all_scores, all_results = merged_scores, merged_preds

    print(f"\n{'=' * 80}\nAGGREGATE BY AXIOM TYPE\n{'=' * 80}")
    for ax in AXIOM_TYPES:
        f1s = [all_scores[k]["f1"] for k in all_scores if k.endswith(f"_{ax}")]
        if f1s:
            print(f"  {ax:<15} avg F1 = {sum(f1s) / len(f1s):.3f}  (n={len(f1s)})")

    all_f1 = [s["f1"] for s in all_scores.values()]
    overall = sum(all_f1) / len(all_f1) if all_f1 else 0
    scored = sorted({k.rsplit('_', 1)[0] for k in all_scores})
    failed = sorted(o for o, r in all_results.items() if isinstance(r, dict) and "error" in r)
    print(f"\n  OVERALL avg F1 = {overall:.3f}   over {len(scored)}/{len(ONTOLOGIES)} ontologies")
    print(f"  o1 (paper's best)          = 0.197")
    print(f"  bare Claude Opus (reported) = 0.431")
    if truncated:
        print(f"  TRUNCATED (scored from salvaged prefix, recall is a lower bound): {', '.join(truncated)}")
    if failed:
        print(f"  FAILED (excluded from the average): {', '.join(failed)}")

    with open(out_path, "w") as f:
        json.dump({
            "method": "bare_llm",
            "backend": args.backend,
            "model": model,
            "input": "class/property name lists only (same as OntoAxiom paper)",
            "tools_used": 0,
            "max_tokens": args.max_tokens,
            "scores": all_scores,
            "overall_f1": overall,
            "ontologies_scored": scored,
            "truncated": truncated,
            "failed": failed,
            "predictions": {k: (v if isinstance(v, dict) else {}) for k, v in all_results.items()},
        }, f, indent=2)
    print(f"\nResults saved to {out_path}")


if __name__ == "__main__":
    main()
