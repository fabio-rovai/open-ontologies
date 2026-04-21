#!/usr/bin/env python3
"""Score Condition D extracted axioms against ground truth.
Run AFTER the extraction agents have finished."""
import json, os, glob

os.chdir('/Users/fabio/projects/open-ontologies')
GT_DIR = 'benchmark/ontoaxiom/data/ontoaxiom'
EX_DIR = 'benchmark/ontoaxiom/results/condition_d'

AXIOM_TYPES = ['subclassof', 'disjoint', 'domain', 'range', 'subproperty']
# Map from ground truth dir names to possible keys in extracted JSON
GT_DIRS = {
    'subclassof': ['subclassof', 'subClassOf'],
    'disjoint': ['disjoint', 'disjointWith'],
    'domain': ['domain'],
    'range': ['range'],
    'subproperty': ['subproperty', 'subPropertyOf'],
}

ONTOLOGIES = ['pizza', 'foaf', 'gufo', 'time', 'saref', 'nordstream', 'goodrelations', 'era', 'music']

def normalize_pair(pair):
    """Normalize a pair for comparison: lowercase, strip whitespace."""
    return tuple(s.strip().lower() for s in pair)

def load_gt(ontology, axiom_type):
    """Load ground truth for an ontology and axiom type."""
    fname = f'{GT_DIR}/{axiom_type}/{ontology}_{axiom_type}.json'
    if not os.path.exists(fname):
        return set()
    with open(fname) as f:
        data = json.load(f)
    return set(normalize_pair(p) for p in data)

def load_extracted(ontology, axiom_type, gt_set=None):
    """Load extracted axioms, trying multiple key names.
    If gt_set is provided, try both orderings and pick the one with more matches."""
    fname = f'{EX_DIR}/{ontology}_extracted.json'
    if not os.path.exists(fname):
        return None
    with open(fname) as f:
        data = json.load(f)
    for key in GT_DIRS[axiom_type]:
        if key in data:
            pairs = data[key]
            normal = set(normalize_pair(p) for p in pairs)
            # Try reversed order too (GT sometimes has [class, prop] vs [prop, class])
            reversed_pairs = set(normalize_pair([p[1], p[0]]) for p in pairs if len(p) == 2)
            if gt_set is not None and len(reversed_pairs & gt_set) > len(normal & gt_set):
                return reversed_pairs
            return normal
    return set()

# Score each ontology
all_results = {}
grand_tp = grand_fp = grand_fn = 0

print(f'{"Ontology":>15} {"Type":>12} {"TP":>4} {"FP":>4} {"FN":>4} {"P":>6} {"R":>6} {"F1":>6}')
print('-' * 65)

for onto in ONTOLOGIES:
    extracted_file = f'{EX_DIR}/{onto}_extracted.json'
    if not os.path.exists(extracted_file):
        print(f'{onto:>15} {"MISSING":>12}')
        continue

    onto_tp = onto_fp = onto_fn = 0
    onto_results = {}

    for atype in AXIOM_TYPES:
        gt = load_gt(onto, atype)
        ex = load_extracted(onto, atype, gt_set=gt)
        if ex is None:
            continue

        tp = len(gt & ex)
        fp = len(ex - gt)
        fn = len(gt - ex)

        onto_tp += tp
        onto_fp += fp
        onto_fn += fn

        p = tp / (tp + fp) if (tp + fp) > 0 else (1.0 if len(gt) == 0 else 0.0)
        r = tp / (tp + fn) if (tp + fn) > 0 else (1.0 if len(gt) == 0 else 0.0)
        f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0.0

        if len(gt) > 0 or len(ex) > 0:
            print(f'{onto:>15} {atype:>12} {tp:>4} {fp:>4} {fn:>4} {p:>6.3f} {r:>6.3f} {f1:>6.3f}')

        onto_results[atype] = {'tp': tp, 'fp': fp, 'fn': fn, 'p': round(p, 4), 'r': round(r, 4), 'f1': round(f1, 4)}

    # Ontology-level
    p = onto_tp / (onto_tp + onto_fp) if (onto_tp + onto_fp) > 0 else 0
    r = onto_tp / (onto_tp + onto_fn) if (onto_tp + onto_fn) > 0 else 0
    f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0
    print(f'{onto:>15} {"OVERALL":>12} {onto_tp:>4} {onto_fp:>4} {onto_fn:>4} {p:>6.3f} {r:>6.3f} {f1:>6.3f}')
    print()

    grand_tp += onto_tp
    grand_fp += onto_fp
    grand_fn += onto_fn

    all_results[onto] = {'per_type': onto_results, 'overall': {'tp': onto_tp, 'fp': onto_fp, 'fn': onto_fn, 'p': round(p, 4), 'r': round(r, 4), 'f1': round(f1, 4)}}

# Grand total
p = grand_tp / (grand_tp + grand_fp) if (grand_tp + grand_fp) > 0 else 0
r = grand_tp / (grand_tp + grand_fn) if (grand_tp + grand_fn) > 0 else 0
f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0

print('=' * 65)
print(f'{"GRAND TOTAL":>15} {"":>12} {grand_tp:>4} {grand_fp:>4} {grand_fn:>4} {p:>6.3f} {r:>6.3f} {f1:>6.3f}')
print()
print(f'Condition B (bare LLM, name lists): F1 = 0.431')
print(f'Condition C (MCP tools, OWL files): F1 = 0.717')
print(f'Condition D (raw OWL file, no tools): F1 = {f1:.3f}')

all_results['_grand_total'] = {'tp': grand_tp, 'fp': grand_fp, 'fn': grand_fn, 'p': round(p, 4), 'r': round(r, 4), 'f1': round(f1, 4)}

with open(f'{EX_DIR}/condition_d_scores.json', 'w') as f:
    json.dump(all_results, f, indent=2)
print(f'\nSaved to {EX_DIR}/condition_d_scores.json')
