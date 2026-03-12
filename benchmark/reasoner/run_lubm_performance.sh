#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OO_BIN="${OO_BIN:-open-ontologies}"
RESULTS_DIR="$SCRIPT_DIR/results"
SIZES="1000 5000 10000 50000"

mkdir -p "$RESULTS_DIR"

echo "=== LUBM Performance Benchmark ==="
echo ""

# 1. Generate ontologies at each scale
echo "Generating LUBM ontologies..."
python3 "$SCRIPT_DIR/generate_lubm.py" $SIZES
echo ""

# 2. Run benchmarks
RESULTS_FILE="$RESULTS_DIR/lubm_results.json"
echo "[" > "$RESULTS_FILE"
FIRST=true

for SIZE in $SIZES; do
    OWL_FILE="$RESULTS_DIR/lubm_${SIZE}.owl"
    if [ ! -f "$OWL_FILE" ]; then
        echo "SKIP: $OWL_FILE not found"
        continue
    fi

    echo "--- Size: $SIZE axioms ---"

    # Open Ontologies
    START_MS=$(python3 -c "import time; print(int(time.time()*1000))")
    $OO_BIN load "$OWL_FILE" > /dev/null 2>&1
    OO_OUT=$($OO_BIN reason --profile owl-dl 2>/dev/null || echo '{"error":"failed"}')
    END_MS=$(python3 -c "import time; print(int(time.time()*1000))")
    OO_MS=$((END_MS - START_MS))
    echo "  Open Ontologies: ${OO_MS}ms"

    # Java reasoners (if available)
    HERMIT_MS="null"
    PELLET_MS="null"
    if ls "$SCRIPT_DIR/lib/"*.jar 1>/dev/null 2>&1; then
        START_MS=$(python3 -c "import time; print(int(time.time()*1000))")
        java -cp "$SCRIPT_DIR:$SCRIPT_DIR/lib/*" JavaReasoner hermit "$OWL_FILE" "$RESULTS_DIR/hermit_lubm_${SIZE}.json" 2>/dev/null || true
        END_MS=$(python3 -c "import time; print(int(time.time()*1000))")
        HERMIT_MS=$((END_MS - START_MS))
        echo "  HermiT: ${HERMIT_MS}ms"

        START_MS=$(python3 -c "import time; print(int(time.time()*1000))")
        java -cp "$SCRIPT_DIR:$SCRIPT_DIR/lib/*" JavaReasoner pellet "$OWL_FILE" "$RESULTS_DIR/pellet_lubm_${SIZE}.json" 2>/dev/null || true
        END_MS=$(python3 -c "import time; print(int(time.time()*1000))")
        PELLET_MS=$((END_MS - START_MS))
        echo "  Pellet: ${PELLET_MS}ms"
    fi

    if [ "$FIRST" = true ]; then
        FIRST=false
    else
        echo "," >> "$RESULTS_FILE"
    fi

    cat >> "$RESULTS_FILE" <<ENTRY
  {"size": $SIZE, "oo_ms": $OO_MS, "hermit_ms": $HERMIT_MS, "pellet_ms": $PELLET_MS}
ENTRY

    echo ""
done

echo "]" >> "$RESULTS_FILE"

echo "Results saved to $RESULTS_FILE"
echo ""

# 3. Plot if matplotlib available
if python3 -c "import matplotlib" 2>/dev/null; then
    echo "Generating chart..."
    python3 "$SCRIPT_DIR/plot_results.py"
else
    echo "SKIP: matplotlib not installed — run 'pip install matplotlib' to generate charts"
fi
