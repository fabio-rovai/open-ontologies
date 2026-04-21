#!/usr/bin/env python3
"""
OAEI Anatomy alignment signal ablation.
Tests 5 weight configurations by patching align.rs, rebuilding, and re-running.

Configurations:
1. Full (all 6 signals) - current defaults [0.25, 0.20, 0.15, 0.15, 0.15, 0.10]
2. Label only - [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]
3. No structural (label only, structural zeroed) - [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]
   (same as label only when no embeddings)
4. Structural only (no label) - [0.0, 0.25, 0.25, 0.20, 0.20, 0.10]
5. Equal weights - [0.167, 0.167, 0.167, 0.167, 0.167, 0.167]
"""

import subprocess, json, xml.etree.ElementTree as ET, time, re, sys, os

os.chdir('/Users/fabio/projects/open-ontologies')

# Parse reference
ref_file = 'benchmark/oaei/data/anatomy/reference.rdf'
tree = ET.parse(ref_file)
root = tree.getroot()
reference = set()
for cell in root.iter():
    if 'Cell' in cell.tag:
        e1 = e2 = None
        for child in cell:
            if 'entity1' in child.tag: e1 = child
            elif 'entity2' in child.tag: e2 = child
        if e1 is not None and e2 is not None:
            src = e1.get('{http://www.w3.org/1999/02/22-rdf-syntax-ns#}resource', '')
            tgt = e2.get('{http://www.w3.org/1999/02/22-rdf-syntax-ns#}resource', '')
            if src and tgt: reference.add((src, tgt))
print(f'Reference: {len(reference)} mappings')

ALIGN_RS = 'src/align.rs'
MOUSE = 'benchmark/oaei/data/anatomy/mouse.owl'
HUMAN = 'benchmark/oaei/data/anatomy/human.owl'
BINARY = './target/release/open-ontologies'
THRESHOLD = '0.80'

# Read original source
with open(ALIGN_RS, 'r') as f:
    original_src = f.read()

CONFIGS = {
    'full':           [0.25, 0.20, 0.15, 0.15, 0.15, 0.10],
    'label_only':     [1.00, 0.00, 0.00, 0.00, 0.00, 0.00],
    'no_label':       [0.00, 0.25, 0.25, 0.20, 0.20, 0.10],
    'equal':          [0.167, 0.167, 0.167, 0.167, 0.167, 0.167],
    'label_parent':   [0.40, 0.00, 0.40, 0.00, 0.00, 0.20],
}

results = {}

for name, weights in CONFIGS.items():
    print(f'\n{"="*60}')
    print(f'CONFIG: {name} = {weights}')
    print(f'{"="*60}')

    # Patch the weights in source
    weight_str = f'[{", ".join(f"{w:.3}" for w in weights)}]'
    patched = re.sub(
        r'(#\[cfg\(not\(feature = "embeddings"\)\)\]\s*const DEFAULT_WEIGHTS: \[f64; 6\] = )\[[^\]]+\]',
        f'\\g<1>{weight_str}',
        original_src
    )

    with open(ALIGN_RS, 'w') as f:
        f.write(patched)

    # Rebuild
    print('Building...')
    build = subprocess.run(
        ['cargo', 'build', '--release'],
        capture_output=True, text=True, timeout=300,
        env={**os.environ, 'CARGO_TARGET_DIR': 'target'}
    )
    if build.returncode != 0:
        print(f'BUILD FAILED: {build.stderr[-300:]}')
        continue

    # Run alignment
    print('Running alignment...')
    t0 = time.time()
    result = subprocess.run(
        [BINARY, 'align', MOUSE, HUMAN, '--min-confidence', THRESHOLD, '--dry-run'],
        capture_output=True, text=True, timeout=600
    )
    elapsed = time.time() - t0

    if result.returncode != 0:
        print(f'ALIGN FAILED: {result.stderr[:300]}')
        continue

    data = json.loads(result.stdout)
    candidates = data.get('candidates', [])
    predicted = set((c['source_iri'], c['target_iri']) for c in candidates)

    tp = len(predicted & reference)
    fp = len(predicted - reference)
    fn = len(reference - predicted)
    p = tp / (tp + fp) if (tp + fp) > 0 else 0
    r = tp / (tp + fn) if (tp + fn) > 0 else 0
    f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0

    results[name] = {
        'weights': weights,
        'candidates': len(candidates),
        'tp': tp, 'fp': fp, 'fn': fn,
        'precision': round(p, 4),
        'recall': round(r, 4),
        'f1': round(f1, 4),
        'time_s': round(elapsed, 1)
    }

    print(f'Candidates: {len(candidates)} | TP={tp} FP={fp} FN={fn}')
    print(f'P={p:.3f} R={r:.3f} F1={f1:.3f} ({elapsed:.0f}s)')

# Restore original source
with open(ALIGN_RS, 'w') as f:
    f.write(original_src)
print('\nRestored original align.rs')

# Save results
output_path = 'benchmark/oaei/results/ablation_signals.json'
with open(output_path, 'w') as f:
    json.dump(results, f, indent=2)
print(f'\nResults saved to {output_path}')

# Summary table
print(f'\n{"="*60}')
print(f'SUMMARY (min_confidence={THRESHOLD})')
print(f'{"="*60}')
print(f'{"Config":<15} {"Cands":>6} {"P":>7} {"R":>7} {"F1":>7}')
print('-' * 45)
for name, r in results.items():
    print(f'{name:<15} {r["candidates"]:>6} {r["precision"]:>7.3f} {r["recall"]:>7.3f} {r["f1"]:>7.3f}')
