#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OO="${OO_BIN:-open-ontologies}"

echo "=== Mushroom Classification: Manual Expert Labels vs OWL Reasoning ==="
echo ""

# 1. Prepare data (add headers, expand codes)
echo "Step 1: Preparing dataset..."
python3 "$SCRIPT_DIR/prepare_data.py"
echo ""

# 2. Load ontology
echo "Step 2: Loading mushroom ontology..."
$OO load "$SCRIPT_DIR/mushroom-ontology.ttl" --pretty
echo ""

# 3. Ingest data
echo "Step 3: Ingesting 8,124 mushroom specimens..."
$OO ingest "$SCRIPT_DIR/mushrooms.csv" --mapping "$SCRIPT_DIR/mushroom-mapping.json" --pretty
echo ""

# 4. Reason
echo "Step 4: Running OWL reasoning..."
$OO reason --profile owl-rl --pretty
echo ""

# 5. Stats
echo "Step 5: Store stats..."
$OO stats --pretty
echo ""

# 6. Compare classifications
echo "Step 6: Comparing OWL classification vs expert labels..."
python3 "$SCRIPT_DIR/compare_classification.py"
echo ""

echo "Done."
