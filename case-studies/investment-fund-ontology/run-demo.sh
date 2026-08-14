#!/usr/bin/env bash
# Investment Fund Ontology (IFO) case study.
#
# Part 1 (always runs, self-contained): loads the vendored IFO core ontology,
# SKOS identifier-scheme registry, and the Vanguard example subgraph; runs
# registry/provenance queries and the layer-1/2 SHACL shapes.
#
# Part 2 (runs when the full graph is present): loads the 1.29M-triple US fund
# universe built by github.com/fabio-rovai/investment-fund-ontology and executes
# the four layer-3 governance rules as plain SPARQL, printing each count next to
# the number the IFO reference pipeline (set-based Python) published for it.
#
#   IFO_GRAPH=/path/to/fund_graph.ttl ./run-demo.sh
#
# Usage: ./run-demo.sh
# Produces a markdown report on stdout.

set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
IFO_GRAPH="${IFO_GRAPH:-$HOME/projects/investment-fund-ontology/data/build/fund_graph.ttl}"

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
# IFO on Open Ontologies — the layer-3 rules as plain SPARQL

Ontology under test: the Investment Fund Ontology
(github.com/fabio-rovai/investment-fund-ontology), vendored at v0.2.0.

## Part 1 — self-contained: core + registry + Vanguard subgraph

EOF_HEADER

echo '### Load and stats'
echo
echo '```json'
"$BIN" batch - <<EOF 2>&1 | filter_results
load $DIR/ifo-core.ttl
load $DIR/identifier-schemes.ttl
load $DIR/vanguard-index-funds.ttl
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
load $DIR/ifo-core.ttl
load $DIR/identifier-schemes.ttl
load $DIR/vanguard-index-funds.ttl
query "$query"
EOF
  echo '```'
}

# Scope as data: the registry declares each scheme entity-, issue-, or
# venue-scoped. This is the load-bearing design idea of IFO.
run_part1 "Identifier schemes by declared scope (SKOS registry)" \
'PREFIX ifo: <https://gov.tesseract.academy/def/fund#> PREFIX ifosch: <https://gov.tesseract.academy/def/fund/scheme#> PREFIX skos: <http://www.w3.org/2004/02/skos/core#> SELECT ?scope (COUNT(?scheme) AS ?schemes) (GROUP_CONCAT(?label; separator=\", \") AS ?members) WHERE { ?scheme skos:inScheme ifosch:schemes ; ifo:identifierScope ?scope ; skos:prefLabel ?label } GROUP BY ?scope'

# The VOO finding in miniature: every listing carries a ticker in public SEC
# data, but not one share class in the subgraph has an ISIN assertion from any
# open source. In the full graph this is IFO finding #4: only 12.3% of ETFs had
# any ISIN in GLEIF's open mapping, VOO among the missing.
run_part1 "Tickers quoted vs ISIN assertions present (the enclosure, in one query)" \
'PREFIX ifo: <https://gov.tesseract.academy/def/fund#> PREFIX ifosch: <https://gov.tesseract.academy/def/fund/scheme#> SELECT (COUNT(DISTINCT ?ticker) AS ?tickers_quoted) (COUNT(DISTINCT ?iid) AS ?isin_assertions) WHERE { ?tid ifo:identifierScheme ifosch:TickerSymbol ; ifo:identifierValue ?ticker ; ifo:identifies ?listing . ?listing ifo:listingOf ?class . OPTIONAL { ?iid ifo:identifierScheme ifosch:ISIN ; ifo:identifies ?class } }'

run_part1 "The Vanguard 500 ETF share class (VOO): ticker asserted, ISIN absent" \
'PREFIX ifo: <https://gov.tesseract.academy/def/fund#> PREFIX ifosch: <https://gov.tesseract.academy/def/fund/scheme#> SELECT ?class (COUNT(DISTINCT ?otherid) AS ?identifier_assertions) (COUNT(DISTINCT ?isin) AS ?isin_assertions) WHERE { ?tid ifo:identifierValue \"VOO\" ; ifo:identifies ?listing . ?listing ifo:listingOf ?class . OPTIONAL { ?otherid ifo:identifies ?class } OPTIONAL { ?isin ifo:identifierScheme ifosch:ISIN ; ifo:identifies ?class } } GROUP BY ?class'

# Identifier assertions per source system: cross-system disagreement as a
# query, not an audit project.
run_part1 "Identifier assertions per source system" \
'PREFIX ifo: <https://gov.tesseract.academy/def/fund#> SELECT ?source (COUNT(?id) AS ?assertions) WHERE { ?id a ifo:Identifier ; ifo:sourceSystem ?source } GROUP BY ?source ORDER BY DESC(?assertions)'

echo
echo '### Layer-1/2 SHACL over the subgraph'
echo
echo 'Includes the `sh:inversePath` registrant shape and severity grading.'
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
            key = (v['severity'], v['message'][:80])
            by[key] = by.get(key, 0) + 1
        print(json.dumps({'conforms': res.get('conforms'),
                          'total': res.get('violation_count'),
                          'by_message': [{'severity': k[0], 'message': k[1], 'count': c}
                                         for k, c in sorted(by.items(), key=lambda x: -x[1])]},
                         indent=2))
"
load $DIR/ifo-core.ttl
load $DIR/identifier-schemes.ttl
load $DIR/vanguard-index-funds.ttl
shacl $DIR/ifo-shapes.ttl
EOF
echo '```'

if [ ! -f "$IFO_GRAPH" ]; then
  cat <<'EOF_NOGRAPH'

## Part 2 — full US fund universe: SKIPPED

`fund_graph.ttl` not found. Clone github.com/fabio-rovai/investment-fund-ontology,
run its pipeline (`scripts/fetch_data.sh` then `pipeline/build_graph.py`), and
point IFO_GRAPH at data/build/fund_graph.ttl to replicate the layer-3 rules at
1.29M triples.
EOF_NOGRAPH
  exit 0
fi

cat <<EOF_P2

## Part 2 — full US fund universe ($IFO_GRAPH)

The four layer-3 rules from \`ifo-rules.ttl\`, executed as the plain SPARQL
inside their \`sh:select\` bodies. Expected counts are the ones the IFO
reference pipeline published in its governance report (14 Aug 2026 build):
R1 = 73 findings, R2 = 0, R3 = 214 funds, R5 = 2,148 funds. A rebuilt graph
from fresher source snapshots will differ.

\`\`\`json
EOF_P2

START=$(date +%s)
"$BIN" batch - <<EOF 2>&1 | filter_results
load $IFO_GRAPH
query "PREFIX ifo: <https://gov.tesseract.academy/def/fund#> PREFIX ifosch: <https://gov.tesseract.academy/def/fund/scheme#> SELECT (COUNT(*) AS ?r1_findings) (COUNT(DISTINCT ?this) AS ?r1_funds) WHERE { ?this ifo:hasFeature ifosch:ExchangeTradedFund . ?this ifo:hasShareClass ?value . FILTER NOT EXISTS { ?value ifo:hasListing ?l . } }"
query "PREFIX ifo: <https://gov.tesseract.academy/def/fund#> SELECT (COUNT(*) AS ?r2_conflicts) WHERE { ?this a ifo:Fund . ?this ifo:identifiedBy ?id1, ?id2 . ?id1 a ifo:LEI ; ifo:identifierValue ?value . ?id2 a ifo:LEI ; ifo:identifierValue ?v2 . FILTER (STR(?value) < STR(?v2)) }"
query "PREFIX ifo: <https://gov.tesseract.academy/def/fund#> SELECT (COUNT(DISTINCT ?this) AS ?r3_funds) WHERE { ?this ifo:fundOf ?reg . ?this ifo:identifiedBy ?fid . ?reg ifo:identifiedBy ?rid . ?fid a ifo:LEI ; ifo:identifierValue ?value . ?rid a ifo:LEI ; ifo:identifierValue ?value . }"
query "PREFIX ifo: <https://gov.tesseract.academy/def/fund#> SELECT (COUNT(DISTINCT ?this) AS ?r5_funds) WHERE { ?this ifo:reportedShareClassCount ?value . { SELECT ?this (COUNT(?c) AS ?observed) WHERE { ?this ifo:hasShareClass ?c . } GROUP BY ?this } FILTER (?observed > ?value) }"
EOF
END=$(date +%s)
echo '```'
echo
echo "Wall clock for load + all four rules, single process: $((END - START))s."
