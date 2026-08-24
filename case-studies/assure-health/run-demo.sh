#!/usr/bin/env bash
# Reproduce the assurance report card: fetch the UCI Diabetes readmission data,
# train a frozen model, run per-subgroup membership inference + equity under a DP sweep.
set -euo pipefail
cd "$(dirname "$0")"
if [ ! -d .venv ]; then python3 -m venv .venv; ./.venv/bin/pip -q install -r requirements.txt; fi
[ -f data/diabetic_data.csv ] || ./.venv/bin/python -c "from ucimlrepo import fetch_ucirepo; import pandas as pd; d=fetch_ucirepo(id=296); pd.concat([d.data.features,d.data.targets],axis=1).to_csv('data/diabetic_data.csv',index=False)"
./.venv/bin/python src/assure.py
echo; echo "See results/report_card.json and results/SUMMARY.md"
