#!/bin/bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BENCHMARK_DIR="$SCRIPT_DIR/.."
PIZZA_OWL="${PIZZA_OWL:-$BENCHMARK_DIR/reference/pizza-reference.owl}"
OO_BIN="${OO_BIN:-open-ontologies}"
JAVA="${JAVA:-java}"
LIB_DIR="${LIB_DIR:-$SCRIPT_DIR/lib}"
CLASSES_DIR="$SCRIPT_DIR/target/java"
RESULTS_DIR="${RESULTS_DIR:-$SCRIPT_DIR/results}"

mkdir -p "$RESULTS_DIR"

echo "=== Pizza Ontology Correctness Benchmark ==="
echo ""

if [ ! -f "$PIZZA_OWL" ]; then
    echo "ERROR: Pizza reference ontology not found at $PIZZA_OWL"
    exit 1
fi

# 1. Open Ontologies
#
# `load` and `reason` must run in ONE process. The store is in memory and dies
# with the process, so invoking them separately reasoned over an empty graph.
# `batch -` keeps one process alive across both commands.
echo "Running Open Ontologies (owl-dl)..."
# The engine exits non-zero when any batch command fails. Capture the status
# instead of letting `set -e` end the script silently, so the gate below can
# say which command failed.
set +e
printf '%s\n' "load $PIZZA_OWL" "reason --profile owl-dl" \
    | $OO_BIN batch - > "$RESULTS_DIR/oo_batch.json"
OO_STATUS=$?
set -e

# Gate the run before anything downstream reads it. The load line must report
# triples_loaded > 0 and carry no error, and the reason line must carry no
# error and report a non-zero class count under `named_classes`, a key the
# owl-dl result actually has. (An earlier version read `classes` or
# `class_count`, neither of which exists, so it aborted on every real run.)
python3 - "$RESULTS_DIR/oo_batch.json" "$RESULTS_DIR/oo_result.json" "$OO_STATUS" <<'PY'
import json, sys
load = reason = None
for line in open(sys.argv[1]):
    if not line.strip():
        continue
    obj = json.loads(line)
    if obj.get("command") == "load":
        load = obj.get("result") or {}
    elif obj.get("command") == "reason":
        reason = obj.get("result") or {}
if load is None:
    sys.exit("ERROR: batch produced no load line")
if load.get("error"):
    sys.exit("ERROR: load failed: {}".format(load["error"]))
triples = load.get("triples_loaded")
if not isinstance(triples, int) or triples <= 0:
    sys.exit("ERROR: load reported triples_loaded={!r}. The reasoner would run "
             "over an empty graph, so no figure is written.".format(triples))
if reason is None:
    sys.exit("ERROR: batch produced no reason line")
if reason.get("error"):
    sys.exit("ERROR: reason failed: {}".format(reason["error"]))
classes = reason.get("named_classes")
if classes is None:
    classes = ((reason.get("agents") or {}).get("satisfiability_agent") or {}).get("classes_checked")
if not isinstance(classes, int) or classes <= 0:
    sys.exit("ERROR: reason result reports {!r} named classes over {} loaded "
             "triples; refusing to write a result.".format(classes, triples))
if sys.argv[3] != "0":
    sys.exit("ERROR: the engine exited with status {}".format(sys.argv[3]))
reason["triples_loaded"] = triples
json.dump(reason, open(sys.argv[2], "w"), indent=1)
print("  {} triples loaded, {} named classes reasoned, complete={}.".format(
    triples, classes, reason.get("complete")))
PY
echo "  Done."

# 2. Java reasoners
if ! ls "$LIB_DIR/"*.jar 1>/dev/null 2>&1; then
    echo ""
    echo "SKIPPING Java reasoners (no jars in $LIB_DIR)"
    echo "See README.md for setup instructions."
    echo ""
    echo "Open Ontologies result saved to $RESULTS_DIR/oo_result.json"
    exit 0
fi

mkdir -p "$CLASSES_DIR"
JAVAC="$(dirname "$(command -v "$JAVA")")/javac"
[ -x "$JAVAC" ] || JAVAC=javac
"$JAVAC" -nowarn -cp "$LIB_DIR/*" -d "$CLASSES_DIR" "$SCRIPT_DIR/JavaReasoner.java"
CP="$CLASSES_DIR:$LIB_DIR/*"

echo "Running HermiT..."
"$JAVA" -cp "$CP" JavaReasoner hermit "$PIZZA_OWL" "$RESULTS_DIR/hermit_result.json"

PELLET_ARG=()
if ls "$LIB_DIR"/pellet-*.jar "$LIB_DIR"/openllet-*.jar 1>/dev/null 2>&1; then
    echo "Running Pellet..."
    "$JAVA" -cp "$CP" JavaReasoner pellet "$PIZZA_OWL" "$RESULTS_DIR/pellet_result.json"
    PELLET_ARG=(--pellet "$RESULTS_DIR/pellet_result.json")
else
    echo "Skipping Pellet (no pellet-*.jar or openllet-*.jar in $LIB_DIR)."
fi

# 3. Compare. Both sides are the FULL entailed named hierarchy: JavaReasoner
# emits all superclasses plus equivalents, and compare_results.py reads the
# engine's classification[].superclasses plus its equivalences.
echo ""
echo "Comparing results..."
python3 "$SCRIPT_DIR/compare_results.py" \
    "$RESULTS_DIR/hermit_result.json" \
    "$RESULTS_DIR/oo_result.json" \
    ${PELLET_ARG[@]+"${PELLET_ARG[@]}"}
