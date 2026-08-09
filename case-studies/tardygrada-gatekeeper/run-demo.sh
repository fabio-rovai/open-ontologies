#!/usr/bin/env bash
# Reproduce the exhaustive verification across all three scenarios.
# Pure Python standard library, no dependencies.
set -euo pipefail
cd "$(dirname "$0")"
echo "== 1. Warehouse (bounded multi-agent) =="; python3 src/gatekeeper.py
echo; echo "== 2. Island Navigation (DeepMind AI Safety Gridworlds) =="; python3 src/gridworld_island.py
echo; echo "== 3. Commons Harvest (Melting Pot substrate) =="; python3 src/commons_harvest.py
echo; echo "Results in results/*.json"
