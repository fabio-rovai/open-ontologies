#!/usr/bin/env python3
"""Score Condition D (raw OWL file -> LLM, no tools) against ground truth.
Run AFTER the extraction agents have finished.

HISTORY — this script used to produce F1 = 0.323 for condition D, the number behind
the OntoAxiom paper's "raw OWL hurts" result. That number is an artifact of how this
script scored, not of how the model performed. Three bugs, all one-directional:

  1. Its normalizer only lowercased. Condition A's normalizer (run_bare_llm_benchmark.py)
     also splits camelCase. Condition D is the condition where the model reads real
     Turtle, so it answers in QNames (foaf:Person) and rdfs:label text ("personal
     mailbox" for mbox). A lowercase-only normalizer matches none of those against
     ground truth's bare, camelCase local names. The bias only ever penalizes D.
  2. It reported a MICRO F1 (pooled TP/FP/FN) while condition A reported a MACRO mean
     of per-cell F1. The 0.323 (micro) was compared against A's 0.431 (macro). Those
     were never the same statistic.
  3. It tried the reversed pair order on EVERY axiom type; condition A only flips
     domain and range.

All three are fixed here: the normalizer is imported from run_bare_llm_ablation and
shared with every other script in this directory, flipping is restricted to
domain/range, and BOTH averages are printed so no reader has to guess which one a
headline number refers to. Cells with no ground truth are skipped rather than scored
as zero.

Under the corrected evaluator, condition D scores far ABOVE condition A on both
Claude and Qwen: reading the ontology helps, it does not hurt. See
ONTOAXIOM_SHOWDOWN.md, and score_all_conditions.py for the cross-condition table.

Pass --legacy to reproduce the historical broken number for comparison.
"""
import argparse
import json
import os

from run_bare_llm_ablation import normalize_pair

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
GT_DIR = os.path.join(SCRIPT_DIR, 'data', 'ontoaxiom')
EX_DIR = os.path.join(SCRIPT_DIR, 'results', 'condition_d')

AXIOM_TYPES = ['subclassof', 'disjoint', 'domain', 'range', 'subproperty']
FLIP_TYPES = {'domain', 'range'}
GT_DIRS = {
    'subclassof': ['subclassof', 'subClassOf'],
    'disjoint': ['disjoint', 'disjointWith'],
    'domain': ['domain'],
    'range': ['range'],
    'subproperty': ['subproperty', 'subPropertyOf'],
}
ONTOLOGIES = ['pizza', 'foaf', 'gufo', 'time', 'saref', 'nordstream', 'goodrelations', 'era', 'music']


def legacy_pair(pair):
    """The original normalizer: lowercase and strip, nothing else."""
    return tuple(s.strip().lower() for s in pair)


def f1_of(tp, fp, fn):
    p = tp / (tp + fp) if (tp + fp) else 0.0
    r = tp / (tp + fn) if (tp + fn) else 0.0
    return 2 * p * r / (p + r) if (p + r) else 0.0


def load_gt(ontology, axiom_type, pairfn):
    fname = os.path.join(GT_DIR, axiom_type, f'{ontology}_{axiom_type}.json')
    if not os.path.exists(fname):
        return set()
    with open(fname) as f:
        data = json.load(f)
    return {pairfn(p) for p in data if isinstance(p, list) and len(p) == 2}


def load_extracted(ontology, axiom_type, gt, pairfn, allow_flip):
    fname = os.path.join(EX_DIR, f'{ontology}_extracted.json')
    if not os.path.exists(fname):
        return None
    with open(fname) as f:
        data = json.load(f)
    for key in GT_DIRS[axiom_type]:
        if key in data:
            pairs = [p for p in data[key] if isinstance(p, list) and len(p) == 2]
            normal = {pairfn(p) for p in pairs}
            if allow_flip:
                flipped = {pairfn([p[1], p[0]]) for p in pairs}
                if len(flipped & gt) > len(normal & gt):
                    return flipped
            return normal
    return set()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--legacy', action='store_true',
                    help="reproduce the historical broken scoring (lowercase-only "
                         "normalizer, flip every axiom type) that yielded F1 = 0.323")
    args = ap.parse_args()

    pairfn = legacy_pair if args.legacy else normalize_pair
    all_results = {}
    grand_tp = grand_fp = grand_fn = 0
    macro_cells = []

    mode = 'LEGACY (broken, for comparison only)' if args.legacy else 'CORRECTED (shared normalizer)'
    print(f'Condition D scoring — {mode}\n')
    print(f'{"Ontology":>15} {"Type":>12} {"TP":>4} {"FP":>4} {"FN":>4} {"P":>6} {"R":>6} {"F1":>6}')
    print('-' * 65)

    for onto in ONTOLOGIES:
        if not os.path.exists(os.path.join(EX_DIR, f'{onto}_extracted.json')):
            print(f'{onto:>15} {"MISSING":>12}')
            continue

        onto_tp = onto_fp = onto_fn = 0
        onto_results = {}

        for atype in AXIOM_TYPES:
            gt = load_gt(onto, atype, pairfn)
            if not gt:                       # no ground truth -> not a scorable cell
                continue
            allow_flip = True if args.legacy else (atype in FLIP_TYPES)
            ex = load_extracted(onto, atype, gt, pairfn, allow_flip)
            if ex is None:
                continue

            tp, fp, fn = len(gt & ex), len(ex - gt), len(gt - ex)
            onto_tp += tp
            onto_fp += fp
            onto_fn += fn
            grand_tp += tp
            grand_fp += fp
            grand_fn += fn

            f1 = f1_of(tp, fp, fn)
            macro_cells.append(f1)
            p = tp / (tp + fp) if (tp + fp) else 0.0
            r = tp / (tp + fn) if (tp + fn) else 0.0
            print(f'{onto:>15} {atype:>12} {tp:>4} {fp:>4} {fn:>4} {p:>6.3f} {r:>6.3f} {f1:>6.3f}')
            onto_results[atype] = {'tp': tp, 'fp': fp, 'fn': fn,
                                   'p': round(p, 4), 'r': round(r, 4), 'f1': round(f1, 4)}

        print(f'{onto:>15} {"OVERALL":>12} {onto_tp:>4} {onto_fp:>4} {onto_fn:>4} '
              f'{"":>6} {"":>6} {f1_of(onto_tp, onto_fp, onto_fn):>6.3f}\n')
        all_results[onto] = {'per_type': onto_results,
                             'overall': {'tp': onto_tp, 'fp': onto_fp, 'fn': onto_fn,
                                         'f1': round(f1_of(onto_tp, onto_fp, onto_fn), 4)}}

    micro = f1_of(grand_tp, grand_fp, grand_fn)
    macro = sum(macro_cells) / len(macro_cells) if macro_cells else 0.0

    print('=' * 65)
    print(f'{"GRAND TOTAL":>15} {"":>12} {grand_tp:>4} {grand_fp:>4} {grand_fn:>4}')
    print(f'\n  micro F1 (pooled TP/FP/FN)           = {micro:.3f}')
    print(f'  macro F1 (mean of {len(macro_cells):>2} scored cells)  = {macro:.3f}')
    print('\n  Condition A (bare LLM, name lists)   : macro 0.451  micro 0.397')
    print('  Condition C (MCP tools, OWL files)   : macro 0.713  micro 0.717')
    print(f'  Condition D (raw OWL file, no tools) : macro {macro:.3f}  micro {micro:.3f}')
    if args.legacy:
        print('\n  NOTE: --legacy reproduces the historical micro F1 = 0.323, an artifact of')
        print('  the lowercase-only normalizer. Re-run without --legacy for the real number.')
    else:
        print('\n  Raw OWL BEATS name lists on both averages. The paper\'s "raw OWL hurts"')
        print('  result does not survive a scorer shared with condition A.')

    all_results['_grand_total'] = {'tp': grand_tp, 'fp': grand_fp, 'fn': grand_fn,
                                   'micro_f1': round(micro, 4), 'macro_f1': round(macro, 4),
                                   'legacy_scoring': bool(args.legacy)}

    # --legacy writes to its own file. It used to share the canonical path, so a
    # single reproduction run silently replaced the corrected scores on disk with
    # the broken ones the corrected scores exist to refute.
    name = 'condition_d_scores_legacy.json' if args.legacy else 'condition_d_scores.json'
    out = os.path.join(EX_DIR, name)
    with open(out, 'w') as f:
        json.dump(all_results, f, indent=2)
    print(f'\nSaved to {out}')


if __name__ == '__main__':
    main()
