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

# The Java side, compiled once, the way run_pizza_correctness.sh does it.
JAVA_CP=""
if ls "$SCRIPT_DIR/lib/"*.jar 1>/dev/null 2>&1; then
    CLASSES_DIR="$SCRIPT_DIR/target/java"
    mkdir -p "$CLASSES_DIR"
    JAVAC="$(dirname "$(command -v java)")/javac"
    [ -x "$JAVAC" ] || JAVAC=javac
    "$JAVAC" -nowarn -cp "$SCRIPT_DIR/lib/*" -d "$CLASSES_DIR" "$SCRIPT_DIR/JavaReasoner.java"
    JAVA_CP="$CLASSES_DIR:$SCRIPT_DIR/lib/*"
fi

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
    #
    # MUST be a single process. Each CLI invocation calls setup(), which
    # builds a fresh empty in-memory GraphStore (src/main.rs), so `load` in
    # one process and `reason` in another would time reasoning over ZERO
    # triples — flat ~15ms of process start-up regardless of input size.
    # Batch mode shares one store across commands, which is the only valid
    # way to time this.
    #
    # Errors are NOT swallowed. A benchmark that silently reports a failed
    # run as a fast run is worse than no benchmark, so the harness asserts
    # triples > 0 before trusting any timing.
    #
    # The status is captured with `|| OO_STATUS=$?` rather than read from `$?`
    # on the next line. Under `set -e` a failing assignment ends the script
    # there, so the FAILED branch below was unreachable and a failed engine run
    # stopped the whole benchmark without a word.
    START_MS=$(python3 -c "import time; print(int(time.time()*1000))")
    OO_STATUS=0
    OO_OUT=$(printf 'load %s\nstats\nreason --profile owl-dl\n' "$OWL_FILE" \
             | $OO_BIN batch -) || OO_STATUS=$?
    END_MS=$(python3 -c "import time; print(int(time.time()*1000))")
    OO_MS=$((END_MS - START_MS))

    # Verify the store was actually populated before trusting the timing.
    OO_TRIPLES=$(printf '%s' "$OO_OUT" | python3 -c "
import json,sys
n = 0
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        rec = json.loads(line)
    except json.JSONDecodeError:
        continue
    if rec.get('command') == 'stats':
        n = rec.get('result', {}).get('triples', 0)
print(n)
")

    if [ "$OO_STATUS" -ne 0 ] || [ "${OO_TRIPLES:-0}" -eq 0 ]; then
        echo "  Open Ontologies: FAILED (exit $OO_STATUS, $OO_TRIPLES triples in store)"
        printf '%s\n' "$OO_OUT" | head -5
        OO_MS="null"
    else
        echo "  Open Ontologies: ${OO_MS}ms (${OO_TRIPLES} triples loaded)"
    fi

    # Java reasoners (if available)
    #
    # A run that fails records null, never its wall time. This used to put
    # $SCRIPT_DIR on the classpath, where JavaReasoner.class has never been,
    # and discard the error with `2>/dev/null || true`, so every row recorded
    # how long a JVM takes to report a missing class as the HermiT and Pellet
    # timings. The class is compiled where run_pizza_correctness.sh compiles
    # it, and stderr is left visible.
    HERMIT_MS="null"
    PELLET_MS="null"
    if [ -n "$JAVA_CP" ]; then
        START_MS=$(python3 -c "import time; print(int(time.time()*1000))")
        if java -cp "$JAVA_CP" JavaReasoner hermit "$OWL_FILE" "$RESULTS_DIR/hermit_lubm_${SIZE}.json"; then
            END_MS=$(python3 -c "import time; print(int(time.time()*1000))")
            HERMIT_MS=$((END_MS - START_MS))
            echo "  HermiT: ${HERMIT_MS}ms"
        else
            echo "  HermiT: FAILED (exit $?), recorded as null"
        fi

        START_MS=$(python3 -c "import time; print(int(time.time()*1000))")
        if java -cp "$JAVA_CP" JavaReasoner pellet "$OWL_FILE" "$RESULTS_DIR/pellet_lubm_${SIZE}.json"; then
            END_MS=$(python3 -c "import time; print(int(time.time()*1000))")
            PELLET_MS=$((END_MS - START_MS))
            echo "  Pellet: ${PELLET_MS}ms"
        else
            echo "  Pellet: FAILED (exit $?), recorded as null"
        fi
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
