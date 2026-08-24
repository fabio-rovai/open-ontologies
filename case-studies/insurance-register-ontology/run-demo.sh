#!/usr/bin/env bash
# Insurance Register Ontology (IRO) case study.
#
# Part 1 (always runs, self-contained): loads the vendored IRO core ontology,
# the SKOS operation-mode and identifier-scheme registries, and a SYNTHETIC
# example subgraph (no EIOPA or GLEIF data is redistributed here); runs the
# registry and provenance queries, executes all six layer-3 rules as plain
# SPARQL against engineered defects with known counts, and validates the
# layer-1/2 shapes.
#
# Part 2 (runs when the full graph is present): loads the 276,683-triple EEA
# register graph built by github.com/fabio-rovai/insurance-register-ontology
# and executes the six layer-3 governance rules as plain SPARQL, printing each
# count next to the number the IRO reference pipeline (set-based Python)
# published for it.
#
#   IRO_GRAPH=/path/to/iro_graph.ttl ./run-demo.sh
#
# Usage: ./run-demo.sh
# Produces a markdown report on stdout.

set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
IRO_GRAPH="${IRO_GRAPH:-$HOME/projects/insurance-register-ontology/data/build/iro_graph.ttl}"

cargo build --release --manifest-path "$ROOT/Cargo.toml" --bin open-ontologies --quiet
BIN="$(cargo metadata --manifest-path "$ROOT/Cargo.toml" --format-version 1 | python3 -c 'import sys,json; print(json.load(sys.stdin)["target_directory"])')/release/open-ontologies"

filter_results() {
  python3 -c "
import json, sys
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        r = json.loads(line)
    except ValueError:
        continue
    if r.get('command') in ('query', 'stats', 'shacl'):
        print(json.dumps(r['result'], indent=2)[:1600])
"
}

cat <<'EOF_HEADER'
# IRO on Open Ontologies: the six layer-3 rules as plain SPARQL

Ontology under test: the Insurance Register Ontology
(github.com/fabio-rovai/insurance-register-ontology), vendored at its
14 Aug 2026 state. Part 1 runs on a clearly labelled SYNTHETIC subgraph;
no EIOPA register data is redistributed by this case study.

## Part 1: self-contained: core + registries + synthetic subgraph

EOF_HEADER

echo '### Load and stats'
echo
echo '```json'
"$BIN" batch - <<EOF 2>&1 | filter_results
load $DIR/iro-core.ttl
load $DIR/operation-modes.ttl
load $DIR/iro-example-synthetic.ttl
stats
EOF
echo '```'

run_part1() {
  local label="$1"
  local query="$2"
  echo
  echo "### $label"
  echo
  echo '```json'
  "$BIN" batch - <<EOF 2>&1 | filter_results
load $DIR/iro-core.ttl
load $DIR/operation-modes.ttl
load $DIR/iro-example-synthetic.ttl
query "$query"
EOF
  echo '```'
}

# Scope as data: the registry declares the LEI entity-scoped and the NCA
# code authority-scoped. A branch legitimately carrying its head office's
# LEI under a host NCA's code is modelled, not special-cased.
run_part1 "Identifier schemes by declared scope (SKOS registry)" \
'PREFIX iro: <https://gov.tesseract.academy/def/insurance#> PREFIX irosch: <https://gov.tesseract.academy/def/insurance/scheme#> PREFIX skos: <http://www.w3.org/2004/02/skos/core#> SELECT ?scope (GROUP_CONCAT(?label; separator=\", \") AS ?schemes) WHERE { ?scheme skos:inScheme irosch:identifierSchemes ; iro:identifierScope ?scope ; skos:prefLabel ?label } GROUP BY ?scope'

# The five operation modes, grounded in Directive 2009/138/EC, readable as
# data with the register's own notation next to each concept.
run_part1 "Operation modes (SKOS registry, notations from the register vocabulary)" \
'PREFIX irosch: <https://gov.tesseract.academy/def/insurance/scheme#> PREFIX skos: <http://www.w3.org/2004/02/skos/core#> SELECT ?label ?notation WHERE { ?mode skos:inScheme irosch:operationModes ; skos:prefLabel ?label ; skos:notation ?notation } ORDER BY ?label'

# All six layer-3 rules over the synthetic subgraph. The subgraph is
# engineered so the expected counts are 1 / 2 / 1 / 1 / 1 / 1.
echo
echo '### The six layer-3 rules over the synthetic subgraph (expected 1 / 2 / 1 / 1 / 1 / 1)'
echo
echo '```json'
"$BIN" batch - <<EOF 2>&1 | filter_results
load $DIR/iro-core.ttl
load $DIR/operation-modes.ttl
load $DIR/iro-example-synthetic.ttl
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> SELECT (COUNT(DISTINCT ?this) AS ?r1_active_no_lei) WHERE { ?this a iro:InsuranceUndertaking . ?reg iro:registrationOf ?this . FILTER NOT EXISTS { ?reg iro:registrationEnd ?end . } FILTER NOT EXISTS { ?this iro:identifiedBy ?id . ?id a iro:LEI . } }"
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> SELECT (COUNT(DISTINCT ?this) AS ?r2_impossible_leis) WHERE { ?this a iro:LEI ; iro:identifierValue ?value ; iro:checksumValid false . }"
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> SELECT (COUNT(DISTINCT ?this) AS ?r3_lapsed_lei) WHERE { ?this a iro:InsuranceUndertaking . ?reg iro:registrationOf ?this . FILTER NOT EXISTS { ?reg iro:registrationEnd ?end . } ?this iro:identifiedBy ?id . ?id a iro:LEI ; iro:identifierValue ?value ; iro:gleifRegistrationStatus \"LAPSED\" . }"
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> SELECT (COUNT(DISTINCT ?this) AS ?r4_entity_inactive) WHERE { ?this a iro:InsuranceUndertaking . ?reg iro:registrationOf ?this . FILTER NOT EXISTS { ?reg iro:registrationEnd ?end . } ?this iro:identifiedBy ?id . ?id a iro:LEI ; iro:identifierValue ?value ; iro:gleifEntityStatus \"INACTIVE\" . }"
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> SELECT (COUNT(DISTINCT ?this) AS ?r5_shared_leis) WHERE { ?this a iro:LEI ; iro:identifies ?u1, ?u2 . FILTER (STR(?u1) < STR(?u2)) ?this iro:identifierValue ?value . }"
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> SELECT (COUNT(DISTINCT ?this) AS ?r6_zombie_passports) WHERE { ?this a iro:CrossBorderOperation ; iro:operationOf ?u . FILTER NOT EXISTS { ?this iro:operationEnd ?oe . } ?reg iro:registrationOf ?u ; iro:registrationEnd ?re . }"
EOF
echo '```'

# Provenance: identifier assertions per source system, disagreement as a
# query, not an audit project.
run_part1 "Identifier assertions per source system" \
'PREFIX iro: <https://gov.tesseract.academy/def/insurance#> PREFIX skos: <http://www.w3.org/2004/02/skos/core#> SELECT ?sourceLabel (COUNT(?id) AS ?assertions) WHERE { ?id a iro:Identifier ; iro:sourceSystem ?source . ?source skos:prefLabel ?sourceLabel } GROUP BY ?sourceLabel ORDER BY DESC(?assertions)'

echo
echo '### Layer-1/2 SHACL over the synthetic subgraph'
echo
echo 'Exercises `sh:pattern` (the 19-character truncated LEI), `sh:hasValue`'
echo '(the checksum policy), `sh:minCount` (the provenance-free identifier),'
echo 'and the `sh:inversePath` registration shape with `sh:severity` grading.'
echo
echo '```json'
"$BIN" batch - <<EOF 2>&1 | python3 -c "
import json, sys
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        r = json.loads(line)
    except ValueError:
        continue
    if r.get('command') == 'shacl':
        res = r['result']
        by = {}
        for v in res.get('violations', []):
            key = (v['severity'], v['constraint'], v['message'][:70])
            by[key] = by.get(key, 0) + 1
        print(json.dumps({'conforms': res.get('conforms'),
                          'total': res.get('violation_count'),
                          'by_message': [{'severity': k[0], 'constraint': k[1], 'message': k[2], 'count': c}
                                         for k, c in sorted(by.items())]},
                         indent=2))
"
load $DIR/iro-core.ttl
load $DIR/operation-modes.ttl
load $DIR/iro-example-synthetic.ttl
shacl $DIR/iro-shapes.ttl
EOF
echo '```'

if [ ! -f "$IRO_GRAPH" ]; then
  cat <<'EOF_NOGRAPH'

## Part 2: full EEA register graph: SKIPPED

`iro_graph.ttl` not found. Clone github.com/fabio-rovai/insurance-register-ontology,
run its pipeline (fetch_eiopa.py, harvest_gleif.py, build_graph.py), and point
IRO_GRAPH at data/build/iro_graph.ttl to replicate the layer-3 rules at
276,683 triples. The EIOPA export is fetched by you, from EIOPA; it is not
distributed with this repository.
EOF_NOGRAPH
  exit 0
fi

cat <<EOF_P2

## Part 2: full EEA register graph ($IRO_GRAPH)

The six layer-3 rules from \`iro-rules.ttl\`, executed as the plain SPARQL
inside their \`sh:select\` bodies. Reference counts are the ones the IRO
pipeline (set-based Python over the source CSVs) published in its governance
report for the 14 Aug 2026 build: R1 = 643, R2 = 4, R3 = 118, R4 = 42,
R5 = 227 register keys (20 at graph level, after entity resolution),
R6 = 283 (291 at graph level; the LEI join attributes 8 more open operations
to their ended home registration than the CSV key join can see). A rebuilt
graph from fresher EIOPA/GLEIF snapshots will differ.

\`\`\`json
EOF_P2

START=$(date +%s)
"$BIN" batch - <<EOF 2>&1 | filter_results
load $IRO_GRAPH
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> SELECT (COUNT(DISTINCT ?this) AS ?r1_active_no_lei) WHERE { ?this a iro:InsuranceUndertaking . ?reg iro:registrationOf ?this . FILTER NOT EXISTS { ?reg iro:registrationEnd ?end . } FILTER NOT EXISTS { ?this iro:identifiedBy ?id . ?id a iro:LEI . } }"
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> SELECT (COUNT(DISTINCT ?this) AS ?r2_impossible_leis) WHERE { ?this a iro:LEI ; iro:identifierValue ?value ; iro:checksumValid false . }"
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> SELECT (COUNT(DISTINCT ?this) AS ?r3_lapsed_lei) WHERE { ?this a iro:InsuranceUndertaking . ?reg iro:registrationOf ?this . FILTER NOT EXISTS { ?reg iro:registrationEnd ?end . } ?this iro:identifiedBy ?id . ?id a iro:LEI ; iro:identifierValue ?value ; iro:gleifRegistrationStatus \"LAPSED\" . }"
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> SELECT (COUNT(DISTINCT ?this) AS ?r4_entity_inactive) WHERE { ?this a iro:InsuranceUndertaking . ?reg iro:registrationOf ?this . FILTER NOT EXISTS { ?reg iro:registrationEnd ?end . } ?this iro:identifiedBy ?id . ?id a iro:LEI ; iro:identifierValue ?value ; iro:gleifEntityStatus \"INACTIVE\" . }"
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> SELECT (COUNT(DISTINCT ?this) AS ?r5_shared_leis) WHERE { ?this a iro:LEI ; iro:identifies ?u1, ?u2 . FILTER (STR(?u1) < STR(?u2)) ?this iro:identifierValue ?value . }"
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> SELECT (COUNT(DISTINCT ?this) AS ?r6_zombie_passports) WHERE { ?this a iro:CrossBorderOperation ; iro:operationOf ?u . FILTER NOT EXISTS { ?this iro:operationEnd ?oe . } ?reg iro:registrationOf ?u ; iro:registrationEnd ?re . }"
query "PREFIX iro: <https://gov.tesseract.academy/def/insurance#> PREFIX irosch: <https://gov.tesseract.academy/def/insurance/scheme#> SELECT ?home (COUNT(?op) AS ?open_fps) WHERE { ?op a iro:CrossBorderOperation ; iro:operationMode irosch:EEAFreedomOfServices ; iro:operationOf ?u . FILTER NOT EXISTS { ?op iro:operationEnd ?e . } ?u iro:homeCountry ?home . } GROUP BY ?home ORDER BY DESC(?open_fps) LIMIT 6"
EOF
END=$(date +%s)
echo '```'
echo
echo "Wall clock for load + all six rules + the passporting query, single process: $((END - START))s."
