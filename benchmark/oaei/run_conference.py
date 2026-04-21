#!/usr/bin/env python3
"""Run OAEI Conference track and compute P/R/F1."""
import subprocess, json, xml.etree.ElementTree as ET, os, glob

os.chdir('/Users/fabio/projects/open-ontologies')
BINARY = './target/release/open-ontologies'
CONF_DIR = 'benchmark/oaei/data/conference'

def parse_reference(ref_file):
    """Parse OAEI reference alignment RDF."""
    tree = ET.parse(ref_file)
    root = tree.getroot()
    mappings = set()
    for cell in root.iter():
        if 'Cell' in cell.tag or 'map' in cell.tag:
            e1 = e2 = None
            for child in cell:
                if 'entity1' in child.tag: e1 = child
                elif 'entity2' in child.tag: e2 = child
            if e1 is not None and e2 is not None:
                src = e1.get('{http://www.w3.org/1999/02/22-rdf-syntax-ns#}resource', '')
                tgt = e2.get('{http://www.w3.org/1999/02/22-rdf-syntax-ns#}resource', '')
                if src and tgt:
                    mappings.add((src, tgt))
    return mappings

# Process all pairs
total_tp = total_fp = total_fn = 0
pair_results = []

for ref_file in sorted(glob.glob(f'{CONF_DIR}/reference/*.rdf')):
    base = os.path.basename(ref_file).replace('.rdf', '')
    parts = base.split('-')
    if len(parts) != 2: continue
    onto1, onto2 = parts

    src = f'{CONF_DIR}/ontologies/{onto1}.owl'
    tgt = f'{CONF_DIR}/ontologies/{onto2}.owl'

    if not os.path.exists(src) or not os.path.exists(tgt): continue
    with open(src) as f:
        if f.read(15).startswith('<!DOCTYPE'): continue
    with open(tgt) as f:
        if f.read(15).startswith('<!DOCTYPE'): continue

    reference = parse_reference(ref_file)
    if not reference: continue

    result = subprocess.run(
        [BINARY, 'align', src, tgt, '--min-confidence', '0.80', '--dry-run'],
        capture_output=True, text=True, timeout=120
    )
    if result.returncode != 0: continue

    data = json.loads(result.stdout)
    candidates = data.get('candidates', [])
    predicted = set((c['source_iri'], c['target_iri']) for c in candidates)

    tp = len(predicted & reference)
    fp = len(predicted - reference)
    fn = len(reference - predicted)

    total_tp += tp
    total_fp += fp
    total_fn += fn

    p = tp / (tp + fp) if (tp + fp) > 0 else 0
    r = tp / (tp + fn) if (tp + fn) > 0 else 0
    f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0

    pair_results.append({
        'pair': base, 'tp': tp, 'fp': fp, 'fn': fn,
        'ref_size': len(reference), 'pred_size': len(candidates),
        'p': round(p, 3), 'r': round(r, 3), 'f1': round(f1, 3)
    })
    print(f'{base:>25}: {len(candidates):>3} cands, ref={len(reference):>3} | TP={tp} FP={fp} FN={fn} | P={p:.3f} R={r:.3f} F1={f1:.3f}')

# Micro-average
p = total_tp / (total_tp + total_fp) if (total_tp + total_fp) > 0 else 0
r = total_tp / (total_tp + total_fn) if (total_tp + total_fn) > 0 else 0
f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0

print(f'\n{"MICRO-AVERAGE":>25}: TP={total_tp} FP={total_fp} FN={total_fn} | P={p:.3f} R={r:.3f} F1={f1:.3f}')

# Save
results = {
    'pairs': pair_results,
    'micro_avg': {'tp': total_tp, 'fp': total_fp, 'fn': total_fn,
                  'precision': round(p, 4), 'recall': round(r, 4), 'f1': round(f1, 4)}
}
with open('benchmark/oaei/results/conference_results.json', 'w') as f:
    json.dump(results, f, indent=2)
print(f'\nSaved to benchmark/oaei/results/conference_results.json')
