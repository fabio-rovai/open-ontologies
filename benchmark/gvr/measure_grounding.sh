#!/bin/bash
# Measure grounding degree of a generated ontology against the reference Pizza
# Usage: ./measure_grounding.sh <generated.ttl> [reason_profile]
# reason_profile: none, rdfs, owl-rl (default: none)

set -e
BINARY=./target/release/open-ontologies
REF=benchmark/ontoaxiom/data/ontoaxiom/ontologies/pizza.ttl
GEN=$1
PROFILE=${2:-none}

if [ -z "$GEN" ]; then
    echo "Usage: $0 <generated.ttl> [none|rdfs|owl-rl]"
    exit 1
fi

# Step 1: Load reference, optionally reason, dump all triples
REF_TRIPLES=$(mktemp)
if [ "$PROFILE" = "none" ]; then
    printf "load %s\nquery SELECT ?s ?p ?o WHERE { ?s ?p ?o }\n" "$REF" | \
        $BINARY batch 2>/dev/null | python3 -c "
import sys, json
for line in sys.stdin:
    d = json.loads(line.strip())
    if d.get('command') == 'query':
        for r in d['result'].get('results', []):
            print(f\"{r.get('s','')}\t{r.get('p','')}\t{r.get('o','')}\")
" > "$REF_TRIPLES"
else
    printf "load %s\nreason --profile %s\nquery SELECT ?s ?p ?o WHERE { ?s ?p ?o }\n" "$REF" "$PROFILE" | \
        $BINARY batch 2>/dev/null | python3 -c "
import sys, json
for line in sys.stdin:
    d = json.loads(line.strip())
    if d.get('command') == 'query':
        for r in d['result'].get('results', []):
            print(f\"{r.get('s','')}\t{r.get('p','')}\t{r.get('o','')}\")
" > "$REF_TRIPLES"
fi

# Step 2: Load generated, dump all triples
GEN_TRIPLES=$(mktemp)
printf "load %s\nquery SELECT ?s ?p ?o WHERE { ?s ?p ?o }\n" "$GEN" | \
    $BINARY batch 2>/dev/null | python3 -c "
import sys, json
for line in sys.stdin:
    d = json.loads(line.strip())
    if d.get('command') == 'query':
        for r in d['result'].get('results', []):
            print(f\"{r.get('s','')}\t{r.get('p','')}\t{r.get('o','')}\")
" > "$GEN_TRIPLES"

# Step 3: Compute grounding
python3 -c "
ref = set()
with open('$REF_TRIPLES') as f:
    for line in f:
        ref.add(line.strip())

gen = set()
with open('$GEN_TRIPLES') as f:
    for line in f:
        gen.add(line.strip())

grounded = gen & ref
ungrounded = gen - ref

total = len(gen)
grounded_count = len(grounded)
degree = grounded_count / total if total > 0 else 0

print(f'Generated triples: {total}')
print(f'Grounded triples:  {grounded_count}')
print(f'Ungrounded:        {len(ungrounded)}')
print(f'Reference triples: {len(ref)}')
print(f'Grounding degree:  {degree:.4f}')
print(f'Coverage (ref):    {grounded_count / len(ref):.4f}' if len(ref) > 0 else '')
"

# Cleanup
rm -f "$REF_TRIPLES" "$GEN_TRIPLES"
