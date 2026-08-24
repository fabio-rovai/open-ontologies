#!/usr/bin/env bash
# Reproduce verifiabench against any OpenAI-compatible endpoint (default: local MLX on :8080).
# Set VB_API to point elsewhere. Pass model ids as arguments.
set -euo pipefail
cd "$(dirname "$0")"
if [ ! -d .venv ]; then python3 -m venv .venv; ./.venv/bin/pip -q install -r requirements.txt; fi
./.venv/bin/python src/verifiabench.py "$@"
echo; echo "See results/results.json and results/SUMMARY.md"
