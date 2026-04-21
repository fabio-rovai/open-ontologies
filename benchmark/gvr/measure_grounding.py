#!/usr/bin/env python3
"""Measure grounding degree of generated ontology against reference."""
import subprocess, json, sys, os

os.chdir('/Users/fabio/projects/open-ontologies')
BINARY = './target/release/open-ontologies'

def get_triples(ontology_path, reason_profile=None):
    """Load ontology, optionally reason, return set of (s,p,o) triples."""
    cmds = [f'load {ontology_path}']
    if reason_profile:
        cmds.append(f'reason --profile {reason_profile}')
    cmds.append('query "SELECT ?s ?p ?o WHERE { ?s ?p ?o }"')

    input_text = '\n'.join(cmds) + '\n'
    result = subprocess.run(
        [BINARY, 'batch'],
        input=input_text, capture_output=True, text=True, timeout=120
    )

    triples = set()
    for line in result.stdout.strip().split('\n'):
        if not line.strip(): continue
        try:
            d = json.loads(line)
            if d.get('command') == 'query':
                for r in d['result'].get('results', []):
                    s = r.get('s', '')
                    p = r.get('p', '')
                    o = r.get('o', '')
                    if s and p and o:
                        triples.add((s, p, o))
        except json.JSONDecodeError:
            continue
    return triples

def get_stats(ontology_path, reason_profile=None):
    """Load ontology, optionally reason, return stats."""
    cmds = [f'load {ontology_path}']
    if reason_profile:
        cmds.append(f'reason --profile {reason_profile}')
    cmds.append('stats')
    cmds.append('lint')

    input_text = '\n'.join(cmds) + '\n'
    result = subprocess.run(
        [BINARY, 'batch'],
        input=input_text, capture_output=True, text=True, timeout=120
    )

    stats = {}
    lint_issues = 0
    for line in result.stdout.strip().split('\n'):
        if not line.strip(): continue
        try:
            d = json.loads(line)
            if d.get('command') == 'stats':
                stats = d.get('result', {})
            elif d.get('command') == 'lint':
                r = d.get('result', {})
                lint_issues = r.get('total_issues', len(r.get('issues', [])))
        except json.JSONDecodeError:
            continue
    return stats, lint_issues

STRUCTURAL_PREDICATES = {
    '<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>',
    '<http://www.w3.org/2000/01/rdf-schema#subClassOf>',
    '<http://www.w3.org/2000/01/rdf-schema#subPropertyOf>',
    '<http://www.w3.org/2000/01/rdf-schema#domain>',
    '<http://www.w3.org/2000/01/rdf-schema#range>',
    '<http://www.w3.org/2002/07/owl#equivalentClass>',
    '<http://www.w3.org/2002/07/owl#equivalentProperty>',
    '<http://www.w3.org/2002/07/owl#disjointWith>',
    '<http://www.w3.org/2002/07/owl#inverseOf>',
    '<http://www.w3.org/2002/07/owl#onProperty>',
    '<http://www.w3.org/2002/07/owl#someValuesFrom>',
    '<http://www.w3.org/2002/07/owl#allValuesFrom>',
    '<http://www.w3.org/2002/07/owl#imports>',
}

def filter_structural(triples):
    """Keep only structural triples (subClassOf, type, domain, range, etc.).
    Removes annotation triples (labels, comments) that cause noise."""
    return {(s, p, o) for s, p, o in triples if p in STRUCTURAL_PREDICATES}

def measure_grounding(gen_path, ref_path, reason_profile=None, structural_only=False):
    """Measure grounding degree."""
    ref_triples = get_triples(ref_path, reason_profile)
    gen_triples = get_triples(gen_path)

    if structural_only:
        ref_triples = filter_structural(ref_triples)
        gen_triples = filter_structural(gen_triples)

    grounded = gen_triples & ref_triples
    ungrounded = gen_triples - ref_triples

    total = len(gen_triples)
    grounded_count = len(grounded)
    degree = grounded_count / total if total > 0 else 0
    coverage = grounded_count / len(ref_triples) if ref_triples else 0

    return {
        'gen_triples': total,
        'ref_triples': len(ref_triples),
        'grounded': grounded_count,
        'ungrounded': len(ungrounded),
        'grounding_degree': round(degree, 4),
        'coverage': round(coverage, 4),
        'mode': 'structural' if structural_only else 'all'
    }

if __name__ == '__main__':
    ref = 'benchmark/ontoaxiom/data/ontoaxiom/ontologies/pizza.ttl'
    gen = sys.argv[1] if len(sys.argv) > 1 else 'benchmark/gvr/iteration1.ttl'
    profile = sys.argv[2] if len(sys.argv) > 2 else None

    print(f'Generated: {gen}')
    print(f'Reference: {ref}')
    print(f'Reasoning: {profile or "none"}')
    print()

    gen_stats, gen_lint = get_stats(gen)
    print(f'Generated stats: {json.dumps(gen_stats)}')
    print(f'Lint issues: {gen_lint}')

    # All triples
    g_all = measure_grounding(gen, ref, profile, structural_only=False)
    print(f'\nAll triples:')
    for k, v in g_all.items():
        print(f'  {k}: {v}')

    # Structural only
    g_struct = measure_grounding(gen, ref, profile, structural_only=True)
    print(f'\nStructural only:')
    for k, v in g_struct.items():
        print(f'  {k}: {v}')
