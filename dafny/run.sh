#!/usr/bin/env bash
#
# Verify dafny/RuleTable.dfy, and then prove that doing so can fail.
#
# OPTIONAL AND SEPARATE, the way the Kani harnesses and aeneas/run.sh are.
# Nothing in `make check`, `cargo build`, `cargo test` or `lean/`'s `lake build`
# calls this script, and nothing it produces is on any of those paths.
# `docs/ci-gates.md` records that, next to Kani and Aeneas, which are under the
# same arrangement and for the same reason.
#
# Needs `brew install dafny` (4.11.0 here) or the equivalent. Dafny bundles its
# own Z3.
#
# WHAT THIS IS AND IS NOT, because the distinction is the whole of decision 0013:
# `RuleTable.dfy` is a REIMPLEMENTATION of the rule-table grammar in
# `src/reason.rs`. Verifying it proves things about the Dafny. It proves nothing
# about the Rust, and no output of this script should ever be quoted as though it
# did. What it is good for is stated in
# `docs/decisions/0013-a-verifier-that-cannot-read-the-code-verifies-a-rewrite.md`.
#
# The second half is the part worth reading. A verification script that only ever
# reports success is indistinguishable from one that verifies nothing, and this
# repository has shipped that shape of defect before: a skipped test reports `ok`.
# So this script MUTATES the specification three times and requires each mutation
# to be REJECTED. If a mutation verifies, the proof was not resting on the thing
# it claims to rest on, and the script fails.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
src="$here/RuleTable.dfy"
interner="$here/Interner.dfy"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

if ! command -v dafny >/dev/null 2>&1; then
  echo "dafny not on PATH. brew install dafny, or see docs/ci-gates.md." >&2
  exit 2
fi

echo "== dafny $(dafny --version)"
echo

echo "== 1. the specifications verify"
dafny verify "$src"
dafny verify "$interner"
echo

# Each mutation is a `sed`-able edit that removes something the proof depends on.
# The third column is what it is meant to break, so a reader can tell a real
# rejection from an incidental one.
mutate_in () {
  local target="$1" name="$2" from="$3" to="$4" breaks="$5"
  python3 - "$target" "$work/mut.dfy" "$from" "$to" <<'PY'
import sys
src, dst, old, new = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
s = open(src).read()
if old not in s:
    sys.exit("mutation pattern no longer present, this script is stale: " + old)
open(dst, "w").write(s.replace(old, new, 1))
PY
  if dafny verify "$work/mut.dfy" >"$work/out" 2>&1; then
    echo "FAIL: mutation '$name' VERIFIED. It was supposed to break $breaks."
    echo "      Either the proof does not depend on that, or the property is vacuous."
    cat "$work/out"
    exit 1
  fi
  echo "  ok: '$name' rejected (breaks $breaks)"
  grep -E "finished with" "$work/out" | sed 's/^/      /'
}

echo "== 2. the proof can fail"

# The load-bearing one. TCB-20 is stated unconditionally on
# docs/trusted-computing-base.md and it is FALSE unconditionally: a rule name
# carrying a tab renders to a line that reads back as a different rule. Removing
# the hypothesis that says otherwise must break the round-trip theorem.
mutate_in "$src" "drop the tab-freeness of a rule name" \
  '    && NoByte(r.name, TAB)
' '' \
  "RuleStrRoundTrips and the counterexample lemmas"

# The classification. `pat_of` must not treat a bare '?' as a constant, or a
# variable and a constant stop being distinguishable by their first byte.
mutate_in "$src" "let pat_of read a bare '?' as a constant" \
  'if |f| == 1 then None else Some(Var(f[1..]))' \
  'if |f| == 1 then Some(Const(f)) else Some(Var(f[1..]))' \
  "PatOfClassifies and VariablesAndConstantsDoNotCollide"

# The empty-name refusal, which is what makes a parsed rule writable.
mutate_in "$src" "drop the empty-name refusal from ParseRuleLine" \
  'else if |fields[0]| == 0 then None' \
  'else if false then None' \
  "ParsedRulesAreWritable"


# ---------------------------------------------------------------------------
# Interner.dfy: TCB-15, TCB-16 and TCB-17, which Kani cannot reach at all.
#
# A harness over the real `Interner` returns 5108 of 5109 checks undetermined,
# because `std`'s HashMap seeds RandomState through `CCRandomGenerateBytes`, a
# foreign C function Kani does not model. Fixing the hasher removes that call
# and the harness still fails, so the obstacle is HashMap rather than the seed
# and no bound helps. These mutations are what stop this file being a
# restatement of its own conclusion.

echo
echo "== 3. the interner proof can fail"

# Reuse an id instead of appending. This is the collision that would let a rule
# fire on a premise nobody asserted, so injectivity must break.
mutate_in "$interner" "let intern reuse identifier zero" \
  'i.toId[s := |i.toStr|], i.toStr + [s]), |i.toStr|)' \
  'i.toId[s := 0], i.toStr + [s]), 0)' \
  "Injective and RoundTrip"

# Drop the clause tying the map back to the sequence. Without it an id can
# resolve to a string nobody interned under it.
mutate_in "$interner" "drop the map-to-sequence agreement from Wf" \
  'i.toStr[i.toId[s]] == s)' \
  'true)' \
  "RoundTrip"

# Let a later intern rewrite the front of the sequence. Stability must break.
mutate_in "$interner" "let a later intern rewrite the sequence" \
  'i.toStr + [s]), |i.toStr|)' \
  '[s] + i.toStr), |i.toStr|)' \
  "RoundTripSurvives and StableAcrossManyInterns"

echo
echo "== done. Both specifications verify and all six mutations were rejected."
