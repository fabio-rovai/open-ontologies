#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BENCHMARK_DIR="$SCRIPT_DIR/.."
PIZZA_OWL="$BENCHMARK_DIR/reference/pizza-reference.owl"
OO_BIN="${OO_BIN:-open-ontologies}"
RESULTS_DIR="$SCRIPT_DIR/results"

mkdir -p "$RESULTS_DIR"

echo "=== Pizza Ontology Correctness Benchmark ==="
echo ""

if [ ! -f "$PIZZA_OWL" ]; then
    echo "ERROR: Pizza reference ontology not found at $PIZZA_OWL"
    exit 1
fi

# 1. Open Ontologies
echo "Running Open Ontologies (owl-dl)..."
$OO_BIN load "$PIZZA_OWL"
$OO_BIN reason --profile owl-dl > "$RESULTS_DIR/oo_result.json"
echo "  Done."

# 2. HermiT
if ls "$SCRIPT_DIR/lib/"*.jar 1>/dev/null 2>&1; then
    echo "Running HermiT..."
    java -cp "$SCRIPT_DIR:$SCRIPT_DIR/lib/*" JavaReasoner hermit "$PIZZA_OWL" "$RESULTS_DIR/hermit_result.json"

    # 3. Pellet
    echo "Running Pellet..."
    java -cp "$SCRIPT_DIR:$SCRIPT_DIR/lib/*" JavaReasoner pellet "$PIZZA_OWL" "$RESULTS_DIR/pellet_result.json"

    # 4. Compare
    echo ""
    echo "Comparing results..."
    python3 "$SCRIPT_DIR/compare_results.py" \
        "$RESULTS_DIR/hermit_result.json" \
        "$RESULTS_DIR/pellet_result.json" \
        "$RESULTS_DIR/oo_result.json"
else
    echo ""
    echo "SKIPPING Java reasoners (no jars in $SCRIPT_DIR/lib/)"
    echo "See README.md for setup instructions."
    echo ""
    echo "Open Ontologies result saved to $RESULTS_DIR/oo_result.json"
fi
