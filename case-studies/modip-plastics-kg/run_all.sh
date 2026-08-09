#!/usr/bin/env bash
# Reproduce the whole artifact from the committed raw records.
set -euo pipefail
cd "$(dirname "$0")"
python3 src/profile_data.py
python3 src/build_taxonomies.py
python3 src/build_graph.py
python3 src/validate.py
echo "OK — see build/ and ontology/"
