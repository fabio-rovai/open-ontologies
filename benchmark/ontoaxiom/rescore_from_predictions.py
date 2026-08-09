#!/usr/bin/env python3
"""
Recompute scores from the raw predictions already stored in a results file.

Scoring is a pure function of (predictions, ground truth, normalizer). When the
normalizer is fixed, the stored model outputs can be rescored offline rather than
re-querying the model, which keeps the comparison honest: the same generations
are scored under the old and new rules.

  python3 rescore_from_predictions.py data/results/oo_conditionD_qwen_results.json
  python3 rescore_from_predictions.py data/results/*.json --write
"""
import argparse
import json
import os

from run_bare_llm_ablation import AXIOM_TYPES, load_gt, score

FLIP_TYPES = {"domain", "range"}


def rescore(path, write=False):
    with open(path) as f:
        blob = json.load(f)

    preds = blob.get("predictions") or {}
    if not preds:
        print(f"{os.path.basename(path)}: no stored predictions, skipping")
        return

    old_scores = blob.get("scores", {})
    new_scores = {}
    for onto, result in preds.items():
        if not isinstance(result, dict) or "error" in result:
            continue
        for ax in AXIOM_TYPES:
            gt = load_gt(onto, ax)
            new_scores[f"{onto}_{ax}"] = score(result.get(ax, []), gt, try_flip=(ax in FLIP_TYPES))

    def mean_f1(scores):
        f1s = [s["f1"] for s in scores.values()]
        return sum(f1s) / len(f1s) if f1s else 0.0

    old_overall, new_overall = mean_f1(old_scores), mean_f1(new_scores)

    print(f"\n{'=' * 74}\n{os.path.basename(path)}   ({blob.get('method')}, {blob.get('model')})\n{'=' * 74}")
    print(f"{'ontology':<16}{'old F1':>10}{'new F1':>10}{'delta':>10}")
    print("-" * 74)
    ontos = sorted({k.rsplit("_", 1)[0] for k in new_scores})
    for o in ontos:
        old = [old_scores[f"{o}_{ax}"]["f1"] for ax in AXIOM_TYPES if f"{o}_{ax}" in old_scores]
        new = [new_scores[f"{o}_{ax}"]["f1"] for ax in AXIOM_TYPES if f"{o}_{ax}" in new_scores]
        o_m = sum(old) / len(old) if old else 0.0
        n_m = sum(new) / len(new) if new else 0.0
        flag = "   <- was a scoring artifact" if o_m == 0.0 and n_m > 0.0 else ""
        print(f"{o:<16}{o_m:>10.3f}{n_m:>10.3f}{n_m - o_m:>+10.3f}{flag}")
    print("-" * 74)
    print(f"{'OVERALL':<16}{old_overall:>10.3f}{new_overall:>10.3f}{new_overall - old_overall:>+10.3f}")

    if write:
        blob["scores"] = new_scores
        blob["overall_f1"] = new_overall
        blob["rescored"] = "prefix-aware normalize (local_name); predictions unchanged"
        with open(path, "w") as f:
            json.dump(blob, f, indent=2)
        print(f"\nwritten back to {path}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("paths", nargs="+")
    ap.add_argument("--write", action="store_true", help="overwrite scores in place")
    args = ap.parse_args()
    for p in args.paths:
        rescore(p, write=args.write)


if __name__ == "__main__":
    main()
