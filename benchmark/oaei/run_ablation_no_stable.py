#!/usr/bin/env python3
"""
Ablation WITHOUT stable matching - tests whether signal weights matter
when the 1-to-1 constraint is removed.
"""

import subprocess, json, xml.etree.ElementTree as ET, time, re, os

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

with open(ALIGN_RS, 'r') as f:
    original_src = f.read()

# Disable stable matching by commenting it out
disabled_matching = original_src.replace(
    '// Stable matching: for each source class',
    '/* DISABLED FOR ABLATION // Stable matching: for each source class'
).replace(
    '}\n\n        // Auto-apply above threshold',
    '*/\n\n        // Auto-apply above threshold'
)

# Verify the patch worked
if 'DISABLED FOR ABLATION' not in disabled_matching:
    # Try alternative patch
    disabled_matching = re.sub(
        r'(        // Stable matching.*?)\n(        \{.*?        \})\n',
        r'        // ABLATION: stable matching disabled\n',
        original_src,
        flags=re.DOTALL
    )

CONFIGS = {
    'full_no_stable':    [0.25, 0.20, 0.15, 0.15, 0.15, 0.10],
    'label_no_stable':   [1.00, 0.00, 0.00, 0.00, 0.00, 0.00],
    'struct_no_stable':  [0.00, 0.25, 0.25, 0.20, 0.20, 0.10],
}

# First test: run WITH stable matching (current code) at different thresholds
print('\n=== WITH stable matching (varying threshold) ===')
for threshold in ['0.70', '0.75', '0.80', '0.85']:
    result = subprocess.run(
        [BINARY, 'align', MOUSE, HUMAN, '--min-confidence', threshold, '--dry-run'],
        capture_output=True, text=True, timeout=600
    )
    if result.returncode != 0: continue
    data = json.loads(result.stdout)
    candidates = data.get('candidates', [])
    predicted = set((c['source_iri'], c['target_iri']) for c in candidates)
    tp = len(predicted & reference)
    fp = len(predicted - reference)
    fn = len(reference - predicted)
    p = tp / (tp + fp) if (tp + fp) > 0 else 0
    r = tp / (tp + fn) if (tp + fn) > 0 else 0
    f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0
    print(f'  conf={threshold}: {len(candidates):>5} cands | P={p:.3f} R={r:.3f} F1={f1:.3f}')

# Now test WITHOUT stable matching
# We need to actually remove the stable matching block from the code
# Find the exact block to remove
stable_block_start = '        // Stable matching: for each source class, keep only the top-scoring target.'
stable_block_end = '        // Auto-apply above threshold'

if stable_block_start in original_src:
    idx_start = original_src.index(stable_block_start)
    idx_end = original_src.index(stable_block_end)
    no_stable_src = original_src[:idx_start] + '\n' + original_src[idx_end:]

    print('\n=== WITHOUT stable matching ===')

    for name, weights in CONFIGS.items():
        weight_str = f'[{", ".join(f"{w:.3}" for w in weights)}]'
        patched = re.sub(
            r'(#\[cfg\(not\(feature = "embeddings"\)\)\]\s*const DEFAULT_WEIGHTS: \[f64; 6\] = )\[[^\]]+\]',
            f'\\g<1>{weight_str}',
            no_stable_src
        )

        with open(ALIGN_RS, 'w') as f:
            f.write(patched)

        build = subprocess.run(
            ['cargo', 'build', '--release'],
            capture_output=True, text=True, timeout=300,
            env={**os.environ, 'CARGO_TARGET_DIR': 'target'}
        )
        if build.returncode != 0:
            print(f'{name}: BUILD FAILED')
            continue

        for threshold in ['0.80', '0.85']:
            result = subprocess.run(
                [BINARY, 'align', MOUSE, HUMAN, '--min-confidence', threshold, '--dry-run'],
                capture_output=True, text=True, timeout=600
            )
            if result.returncode != 0: continue
            data = json.loads(result.stdout)
            candidates = data.get('candidates', [])
            predicted = set((c['source_iri'], c['target_iri']) for c in candidates)
            tp = len(predicted & reference)
            fp = len(predicted - reference)
            fn = len(reference - predicted)
            p = tp / (tp + fp) if (tp + fp) > 0 else 0
            r = tp / (tp + fn) if (tp + fn) > 0 else 0
            f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0
            print(f'  {name} conf={threshold}: {len(candidates):>6} cands | P={p:.3f} R={r:.3f} F1={f1:.3f}')
else:
    print('ERROR: Could not find stable matching block')

# Restore
with open(ALIGN_RS, 'w') as f:
    f.write(original_src)

# Rebuild with original
subprocess.run(
    ['cargo', 'build', '--release'],
    capture_output=True, text=True, timeout=300,
    env={**os.environ, 'CARGO_TARGET_DIR': 'target'}
)
print('\nRestored original align.rs and rebuilt')
