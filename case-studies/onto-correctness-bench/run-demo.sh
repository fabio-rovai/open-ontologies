#!/usr/bin/env bash
# Reproduce the onto-correctness-bench result end to end.
# Downloads the three real vocabularies (if absent), then runs the deterministic
# benchmark and regenerates results/results.json + results/SUMMARY.md.
set -euo pipefail
cd "$(dirname "$0")"

if [ ! -d .venv ]; then
  python3 -m venv .venv
  ./.venv/bin/pip -q install -r requirements.txt
fi

mkdir -p data
[ -f data/schemaorg.ttl ] || curl -sL https://schema.org/version/latest/schemaorg-current-https.ttl -o data/schemaorg.ttl
[ -f data/pato.owl ]      || curl -sL http://purl.obolibrary.org/obo/pato.owl -o data/pato.owl
[ -f data/ro.owl ]        || curl -sL http://purl.obolibrary.org/obo/ro.owl   -o data/ro.owl
# IES4 is bundled in the parent repo at ../../benchmark/reference/ies4.ttl

./.venv/bin/python src/bench.py 2>/dev/null | grep -v 'does not look like'
echo
echo "See results/SUMMARY.md and results/results.json"
