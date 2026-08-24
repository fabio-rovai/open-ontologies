#!/usr/bin/env bash
# Reproduce bio-kg-triage end to end: fetch the Biolink vocabulary and live Open
# Targets associations, build the grounded + ungrounded KGs, validate both.
set -euo pipefail
cd "$(dirname "$0")"
if [ ! -d .venv ]; then python3 -m venv .venv; ./.venv/bin/pip -q install -r requirements.txt; fi
mkdir -p data
[ -f data/biolink-model.yaml ] || curl -sL https://raw.githubusercontent.com/biolink/biolink-model/master/src/biolink_model/schema/biolink_model.yaml -o data/biolink-model.yaml
# rebuild the declared-vocabulary set from the Biolink model
./.venv/bin/python - <<'PY'
import yaml, json
m=yaml.safe_load(open("data/biolink-model.yaml")); BASE="https://w3id.org/biolink/vocab/"
pas=lambda n:"".join(w[:1].upper()+w[1:] for w in n.split())
cl={BASE+pas(n) for n in (m.get("classes") or {})}; sl={BASE+n.replace(" ","_") for n in (m.get("slots") or {})}
json.dump({"policed":[BASE],"declared":sorted(cl|sl)}, open("data/biolink_vocab.json","w"))
print("Biolink declared terms:", len(cl|sl))
PY
./.venv/bin/python src/pipeline.py
echo; echo "See results/SUMMARY.md, results/triage.md, results/results.json"

# --- literature front end (PubTator3) and AMR layer (CARD/ARO) ---
[ -f data/aro.obo ] || curl -sL http://purl.obolibrary.org/obo/aro.obo -o data/aro.obo
./.venv/bin/python src/pubtator.py
./.venv/bin/python src/amr.py
echo; echo "See results/SUMMARY.md, results/results.json, results_literature.json, results_amr.json"

# --- AMR pathogen linkage (CARD card.json + NCBI taxonomy) ---
[ -f data/card/card.json ] || { curl -sL https://card.mcmaster.ca/latest/data -o data/card-data.tar.bz2; mkdir -p data/card; tar xjf data/card-data.tar.bz2 -C data/card; }
[ -f data/nodes.dmp ] || { curl -sL https://ftp.ncbi.nlm.nih.gov/pub/taxonomy/taxdump.tar.gz -o data/taxdump.tar.gz; tar xzf data/taxdump.tar.gz -C data nodes.dmp merged.dmp delnodes.dmp; }
./.venv/bin/python src/pathogen.py
