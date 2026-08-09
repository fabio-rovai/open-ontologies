#!/usr/bin/env python3
"""
Score every OntoAxiom condition under ONE evaluator, and report macro and micro.

The benchmark scripts in this directory grew independently and disagreed on three
axes, each of which silently moved the headline numbers:

  1. Normalizer.  Condition A splits camelCase; the original condition-D scorer
     only lowercased, so it could not match the QNames and rdfs:label text that a
     model naturally emits when it is reading real Turtle.
  2. Averaging.   Condition A reports a MACRO mean over per-(ontology, axiom) F1
     cells. Condition D and the MCP benchmark report a MICRO F1 over pooled
     TP/FP/FN, which is dominated by a few huge axiom sets (Pizza `disjoint`
     alone carries 785 ground-truth pairs).
  3. Pair flip.   Condition A tries the reversed orientation only for domain and
     range; the original condition-D scorer tried it for every axiom type.

Mixing these makes cross-condition comparison meaningless. This script fixes the
normalizer (shared with run_bare_llm_ablation), fixes the flip policy to
domain/range only, and prints BOTH averages side by side so a reader can see
which conclusions depend on the choice.

  python3 score_all_conditions.py
"""
import glob
import json
import os

from run_bare_llm_ablation import AXIOM_TYPES, ONTOLOGIES, load_gt, normalize_pair

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
RESULTS = os.path.join(SCRIPT_DIR, "data", "results")
COND_D = os.path.join(SCRIPT_DIR, "results", "condition_d")
FLIP_TYPES = {"domain", "range"}

# Claude's condition-D dumps are inconsistent: most use OWL camelCase keys, but
# pizza_extracted.json uses lowercase. Try both spellings or pizza silently scores 0.
D_KEYS = {"subclassof": ["subclassof", "subClassOf"], "disjoint": ["disjoint", "disjointWith"],
          "domain": ["domain"], "range": ["range"], "subproperty": ["subproperty", "subPropertyOf"]}


def cell(pred_pairs, gt, try_flip):
    pred = {normalize_pair(p) for p in pred_pairs if isinstance(p, (list, tuple)) and len(p) == 2}
    if try_flip:
        flipped = {(b, a) for a, b in pred}
        if len(flipped & gt) > len(pred & gt):
            pred = flipped
    tp, fp, fn = len(pred & gt), len(pred - gt), len(gt - pred)
    return tp, fp, fn


def f1_of(tp, fp, fn):
    p = tp / (tp + fp) if tp + fp else 0.0
    r = tp / (tp + fn) if tp + fn else 0.0
    return 2 * p * r / (p + r) if p + r else 0.0


def aggregate(cells, keys=None):
    """cells: {cell_key: (tp, fp, fn)}. Returns (macro, micro, n_cells)."""
    vals = [cells[k] for k in (keys if keys is not None else cells)] if cells else []
    if not vals:
        return 0.0, 0.0, 0
    macro = sum(f1_of(*c) for c in vals) / len(vals)
    tp = sum(c[0] for c in vals)
    fp = sum(c[1] for c in vals)
    fn = sum(c[2] for c in vals)
    return macro, f1_of(tp, fp, fn), len(vals)


def pick(result, ax, keymap):
    """Read an axiom type's pairs, tolerating alternative key spellings."""
    for key in (keymap[ax] if keymap else [ax]):
        if key in result:
            return result[key]
    return []


def from_predictions(preds, keymap=None):
    """Score a {ontology: {axiom_type: [[a,b],...]}} prediction dump, keyed by cell."""
    cells = {}
    for onto, result in preds.items():
        if not isinstance(result, dict) or "error" in result:
            continue
        for ax in AXIOM_TYPES:
            gt = load_gt(onto, ax)
            if not gt:                       # no ground truth -> not a scorable cell
                continue
            cells[f"{onto}_{ax}"] = cell(pick(result, ax, keymap), gt, ax in FLIP_TYPES)
    return cells


def from_scored_cells(scores):
    """Reuse tp/fp/fn already stored by a prior run (valid when preds were bare names)."""
    return {k: (s["tp"], s["fp"], s["fn"]) for k, s in scores.items() if s.get("gt_size", 1) > 0}


def load(path):
    return json.load(open(path)) if os.path.exists(path) else None


def main():
    conds = {}

    # Claude condition A — predictions not stored, but per-cell tp/fp/fn are.
    # Its inputs are bare name lists, so the normalizer fix provably cannot move it.
    a_claude = load(os.path.join(RESULTS, "oo_bare_opus_subagent_results.json"))
    if a_claude:
        conds[("Claude Opus", "A: name lists")] = from_scored_cells(a_claude["scores"])

    # Claude condition D — raw Turtle, predictions stored per ontology.
    d_preds = {}
    for f in glob.glob(os.path.join(COND_D, "*_extracted.json")):
        d_preds[os.path.basename(f).replace("_extracted.json", "")] = json.load(open(f))
    if d_preds:
        conds[("Claude Opus", "D: raw OWL")] = from_predictions(d_preds, D_KEYS)

    for cond, path in [("A: name lists", "oo_bare_qwen_results.json"),
                       ("D: raw OWL", "oo_conditionD_qwen_results.json")]:
        blob = load(os.path.join(RESULTS, path))
        if blob and blob.get("predictions"):
            conds[("Qwen3-Coder-30B", cond)] = from_predictions(blob["predictions"])

    # MCP extraction — stored as {axiom_type: {ontology: {tp,fp,fn}}}.
    mcp = load(os.path.join(RESULTS, "oo_ontoaxiom_mcp_results.json"))
    if mcp:
        cells = {}
        for ax in AXIOM_TYPES:
            for onto, s in (mcp.get(ax) or {}).items():
                if onto.startswith("_") or not isinstance(s, dict) or "tp" not in s:
                    continue
                cells[f"{onto}_{ax}"] = (s["tp"], s["fp"], s["fn"])
        if cells:
            conds[("MCP + SPARQL", "full OWL")] = cells

    print("=" * 82)
    print("OntoAxiom — all conditions under ONE evaluator")
    print("shared normalizer (camelCase + prefix strip); flip on domain/range only")
    print("=" * 82)
    print(f"\n{'model':<18}{'condition':<18}{'macro F1':>11}{'micro F1':>11}{'cells':>8}")
    print("-" * 82)
    for (model, cond), cells in conds.items():
        macro, micro, n = aggregate(cells)
        print(f"{model:<18}{cond:<18}{macro:>11.3f}{micro:>11.3f}{n:>8}")
    print("-" * 82)
    print("\n  macro = mean of per-(ontology, axiom) F1 — every axiom type counts equally")
    print("  micro = F1 over pooled TP/FP/FN — dominated by large axiom sets")

    print(f"\n{'=' * 82}\nA vs D, restricted to the cells BOTH conditions scored\n{'=' * 82}")
    for model in ["Claude Opus", "Qwen3-Coder-30B"]:
        a, d = conds.get((model, "A: name lists")), conds.get((model, "D: raw OWL"))
        if not a or not d:
            continue
        common = sorted(set(a) & set(d))
        am, ami, _ = aggregate(a, common)
        dm, dmi, _ = aggregate(d, common)
        wins = sum(1 for k in common if f1_of(*d[k]) > f1_of(*a[k]))
        print(f"\n  {model}  ({len(common)} common cells; A has {len(a)}, D has {len(d)})")
        print(f"    macro   A={am:.3f}  D={dm:.3f}   delta {dm - am:+.3f}")
        print(f"    micro   A={ami:.3f}  D={dmi:.3f}   delta {dmi - ami:+.3f}")
        print(f"    raw OWL beats name lists on {wins}/{len(common)} cells")
        print(f"    verdict: raw OWL {'HELPS' if dm > am else 'HURTS'} (macro and micro agree: "
              f"{'yes' if (dm > am) == (dmi > ami) else 'NO'})")

    print("\n  The paper reports condition D = 0.323: the MICRO figure under the old")
    print("  lowercase-only normalizer. Condition A's 0.431 is a MACRO figure. They were")
    print("  never comparable on either axis.")


if __name__ == "__main__":
    main()
