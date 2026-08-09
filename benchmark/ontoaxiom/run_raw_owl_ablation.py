#!/usr/bin/env python3
"""
OntoAxiom Benchmark: Condition D (raw OWL file -> LLM, no tools) — CROSS-MODEL ABLATION

The paper's "surprising result" is that an LLM handed the *raw OWL file*
(F1=0.323) does WORSE than the same LLM given only class/property name lists
(F1=0.431). Reviewers noted this could be a Claude-specific contamination
artifact. This script runs the *same* raw-file condition on a second model
(local Qwen3-Coder-30B). If the "raw file hurts" effect reproduces on Qwen, it
is a model-general phenomenon, not Claude memorization.

Uses the EXACT scorer from run_bare_llm_ablation.py, so Qwen condition D is
directly comparable to Qwen condition A produced by that script (single, shared
normalization — the repo's original condition_d scorer used a looser normalize,
which is not comparable to the bare-LLM numbers).

Backends: --backend qwen (default, free) | --backend claude
Large ontologies that exceed the model context are skipped and recorded as such.
"""
import argparse
import json
import os
import sys
import urllib.request

from run_bare_llm_ablation import (
    AXIOM_TYPES, MAX_TOKENS, ONTOLOGY_NAMES, load_gt, score, extract_json,
)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ONT_DIR = os.path.join(SCRIPT_DIR, "data", "ontoaxiom", "ontologies")

# Roughly cap by file size (bytes) to stay within a local model's context.
DEFAULT_MAX_BYTES = 160_000

PROMPT_TEMPLATE = """You are being tested on axiom identification. Below is the FULL source of an ontology in Turtle (RDF/OWL). Read it and extract the declared axiom pairs.

ONTOLOGY: {ontology_name}

--- BEGIN ONTOLOGY (Turtle) ---
{ttl}
--- END ONTOLOGY ---

Return ONLY a single JSON object with these keys (use the local names of classes/properties, not full IRIs):
- "subclassof": [[sub, super], ...]
- "disjoint": [[class1, class2], ...]
- "domain": [[property, domain_class], ...]
- "range": [[property, range_class], ...]
- "subproperty": [[sub_prop, super_prop], ...]

Output ONLY the JSON object, nothing else."""


def call_qwen(host, model, ontology, ttl, max_tokens=MAX_TOKENS, temp=0.2):
    prompt = PROMPT_TEMPLATE.format(
        ontology_name=ONTOLOGY_NAMES.get(ontology, ontology), ttl=ttl)
    body = json.dumps({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens, "temperature": temp,
    }).encode()
    req = urllib.request.Request(
        host.rstrip("/") + "/v1/chat/completions",
        data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=2400) as resp:
        data = json.load(resp)
    choice = data["choices"][0]
    result = extract_json(choice["message"]["content"])
    result["_finish_reason"] = choice.get("finish_reason")
    return result


def call_claude(model, ontology, ttl, max_tokens=MAX_TOKENS):
    import anthropic
    client = anthropic.Anthropic()
    prompt = PROMPT_TEMPLATE.format(
        ontology_name=ONTOLOGY_NAMES.get(ontology, ontology), ttl=ttl)
    r = client.messages.create(model=model, max_tokens=max_tokens,
                               messages=[{"role": "user", "content": prompt}])
    result = extract_json(r.content[0].text)
    result["_finish_reason"] = r.stop_reason
    return result


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--backend", choices=["qwen", "claude"], default="qwen")
    ap.add_argument("--host", default=os.environ.get("QWEN_HOST", "http://localhost:8080"))
    ap.add_argument("--model", default=None)
    ap.add_argument("--max-bytes", type=int, default=DEFAULT_MAX_BYTES)
    ap.add_argument("--max-tokens", type=int, default=MAX_TOKENS)
    ap.add_argument("--only", default=None, help="comma-separated ontology subset")
    args = ap.parse_args()

    if args.backend == "qwen":
        model = args.model or "mlx-community/Qwen3-Coder-30B-A3B-Instruct-8bit"
        caller = lambda o, ttl: call_qwen(args.host, model, o, ttl, max_tokens=args.max_tokens)
    else:
        model = args.model or "claude-opus-4-8"
        if not os.environ.get("ANTHROPIC_API_KEY"):
            print("ERROR: Set ANTHROPIC_API_KEY for --backend claude"); sys.exit(1)
        caller = lambda o, ttl: call_claude(model, o, ttl, max_tokens=args.max_tokens)

    ontologies = ["pizza", "foaf", "gufo", "nordstream", "era",
                  "goodrelations", "music", "saref", "time"]
    if args.only:
        ontologies = [o.strip() for o in args.only.split(",")]

    flip_types = {"domain", "range"}
    all_scores, all_results, skipped, truncated = {}, {}, [], []

    print("=" * 80)
    print(f"CONDITION D (raw OWL -> LLM, no tools)  backend={args.backend}  model={model}")
    print(f"max_tokens={args.max_tokens}  max_bytes={args.max_bytes}")
    print("=" * 80)

    for onto in ontologies:
        path = os.path.join(ONT_DIR, f"{onto}.ttl")
        if not os.path.exists(path):
            print(f"\n  {onto}: skipped (no .ttl)"); continue
        size = os.path.getsize(path)
        if size > args.max_bytes:
            print(f"\n  {onto}: SKIPPED ({size} bytes > {args.max_bytes} cap — exceeds model context)")
            skipped.append(onto); continue
        ttl = open(path).read()
        print(f"\n--- {onto.upper()} ({size} bytes) ---")
        try:
            result = caller(onto, ttl)
        except Exception as e:
            print(f"  ERROR: {e}")
            all_results[onto] = {"error": str(e)}; continue
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
            print(f"  {ax:<15} P={s['precision']:.3f}  R={s['recall']:.3f}  F1={s['f1']:.3f}  (tp={s['tp']}/{s['gt_size']})")

    print(f"\n{'=' * 80}\nAGGREGATE\n{'=' * 80}")
    all_f1 = [s["f1"] for s in all_scores.values()]
    overall = sum(all_f1) / len(all_f1) if all_f1 else 0
    scored = sorted({k.rsplit('_', 1)[0] for k in all_scores})
    failed = sorted(o for o, r in all_results.items() if isinstance(r, dict) and "error" in r)
    print(f"  OVERALL avg F1 (condition D, {args.backend}) = {overall:.3f}   over {len(scored)} ontologies")
    print(f"  Claude condition A (name lists)  = 0.431")
    print(f"  Claude condition D (raw OWL)     = 0.323   <- the 'surprising' number")
    if skipped:
        print(f"  Skipped (too large for context): {', '.join(skipped)}")
    if truncated:
        print(f"  TRUNCATED (salvaged prefix, recall is a lower bound): {', '.join(truncated)}")
    if failed:
        print(f"  FAILED (excluded from the average): {', '.join(failed)}")
    print("\n  NOTE: this average covers a different ontology set than condition A.")
    print("  Compare A vs D only on their common subset — see compare_conditions.py")

    tag = args.backend
    out_path = os.path.join(SCRIPT_DIR, "data", "results", f"oo_conditionD_{tag}_results.json")
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w") as f:
        json.dump({"method": "raw_owl_no_tools", "backend": args.backend, "model": model,
                   "input": "full Turtle source of the ontology",
                   "max_tokens": args.max_tokens, "max_bytes": args.max_bytes,
                   "scores": all_scores, "overall_f1": overall,
                   "ontologies_scored": scored,
                   "skipped_too_large": skipped, "truncated": truncated, "failed": failed,
                   "predictions": {k: (v if isinstance(v, dict) else {}) for k, v in all_results.items()},
                   }, f, indent=2)
    print(f"\nResults saved to {out_path}")


if __name__ == "__main__":
    main()
