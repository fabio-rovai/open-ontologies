#!/usr/bin/env bash
# Reproduce certicoupla: fetch real OQMD materials, train + freeze a regressor,
# compare independent conformal vs coupled and Gaussian-copula certificates.
set -euo pipefail
cd "$(dirname "$0")"
if [ ! -d .venv ]; then python3 -m venv .venv; ./.venv/bin/pip -q install -r requirements.txt; fi
[ -f data/oqmd.json ] || ./.venv/bin/python src/fetch.py
./.venv/bin/python src/certicoupla.py
echo; echo "See results/SUMMARY.md and results/results.json"
