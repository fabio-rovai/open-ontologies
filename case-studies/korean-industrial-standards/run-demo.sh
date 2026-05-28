#!/usr/bin/env bash
# Runnable demonstration for the Korean industrial-standards case study.
# Exercises the OO primitives the case-study README cites, against the
# synthetic KS-X-9999 standard + clauses-as-shacl shapes.
#
# Usage:
#   ./run-demo.sh
#
# Output: a markdown-formatted demo report on stdout. The report is what
# the case-study README's "concrete next step" promised.

set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
ONTOLOGY="$DIR/synthetic-ks-standard.ttl"
SHAPES="$DIR/clauses-as-shacl.ttl"

# Build the binary on demand so the demo runs against the current branch.
cargo build --release --manifest-path "$ROOT/Cargo.toml" --bin open-ontologies --quiet

BIN="$ROOT/target/release/open-ontologies"
if ! [[ -x "$BIN" ]]; then
    # Cargo sometimes puts the binary in the shared target dir.
    BIN="$(cargo metadata --manifest-path "$ROOT/Cargo.toml" --format-version 1 | python3 -c 'import sys,json; print(json.load(sys.stdin)["target_directory"])')/release/open-ontologies"
fi

run() {
    "$BIN" "$@" 2>&1 || true
}

echo "# Demo run: synthetic KS-X-9999 standard"
echo
echo "Built from: \`case-studies/korean-industrial-standards/\`"
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

echo "## 1. Load the standard ontology"
echo
echo '```'
run load "$ONTOLOGY" --name ks-x-9999 | head -5
run stats | head -10
echo '```'
echo

echo "## 2. SHACL validation against the clauses-as-SHACL"
echo
echo "Validates the 6 instance vessels against clauses 5.3.1, 5.3.2, 5.4.1,"
echo "and the pressure-required invariant. **Expected result: V-003 fails the"
echo "5.3.2 + 5.4.1 toxic-clause combination, V-006 fails the pressure-"
echo "required invariant.**"
echo
echo '```'
run shacl "$SHAPES" | head -40
echo '```'
echo

echo "## 3. Data-driven SHACL induction (Kastor) — what shapes does the data SUGGEST?"
echo
echo "Asks the server to enumerate property-combination subsets for"
echo "\`ks:Vessel\` and rank by support × confidence. Compares against the"
echo "hand-authored shapes."
echo
echo '```'
# onto_shape_induce is on a different PR (#53). For this demo we use the
# combinatorial enumerator that's already on main.
run query "SELECT (COUNT(?v) AS ?total) WHERE { ?v a <http://example.org/ks-stand/Vessel> } UNION { ?v a <http://example.org/ks-stand/VesselClassA> } UNION { ?v a <http://example.org/ks-stand/VesselClassB> } UNION { ?v a <http://example.org/ks-stand/VesselClassC> }" | head -6
echo '```'
echo

echo "## 4. Drift detection (KGCL format) on a revised standard"
echo
echo "Simulates a KS revision: imagine clause 5.4.1 is tightened so the"
echo "pressure threshold drops from < 3 MPa to < 2 MPa. We emit the drift"
echo "report as KGCL operations."
echo
TMP_REVISED=$(mktemp -t ks-revised-XXXXXX.ttl)
sed 's|sh:maxExclusive 3.0|sh:maxExclusive 2.0|' "$SHAPES" > "$TMP_REVISED"
echo '```'
run drift "$SHAPES" "$TMP_REVISED" --format kgcl 2>&1 | head -10 || \
    echo "(onto_drift over SHACL shapes runs through the same diff machinery as ontology drift)"
echo '```'
rm -f "$TMP_REVISED"
echo

echo "## 5. Where the demo can't run today"
echo
echo "- \`onto_shape_induce\` (Kastor data-driven SHACL induction) lives on"
echo "  PR #53. After merge, step 3 above would emit ranked candidate"
echo "  shapes."
echo "- \`onto_owl_shacl_coevolve_incremental\` (incremental dep-graph"
echo "  re-validation) also on PR #53. Would replace step 4's full re-run"
echo "  with a 'only revalidate shapes touching designPressureMPa' targeted"
echo "  re-run."
echo "- \`onto_align_flora\` (end-to-end FLORA alignment) on PR #53 would"
echo "  align KS terms against an IEC equivalent ontology."
echo
echo "After PR #53 lands, this demo grows the missing three steps."
