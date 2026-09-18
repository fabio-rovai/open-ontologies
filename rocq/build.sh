#!/usr/bin/env bash
#
# Build the Rocq development, check the axiom footprint, and link the extracted
# checker into a native binary.
#
#   rocq/build.sh                            build and gate
#   rocq/build.sh --prove-the-gate-can-fail  break the audit on a scratch copy
#                                            and require the build to fail
#
# The second form exists because a gate nobody has watched fail is decoration.
# It copies the tree, replaces one Qed with Admitted, runs the same build, and
# fails if that build SUCCEEDS.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

ROCQ="${ROCQ:-rocq}"
# ocamlfind is what a developer machine usually has; the Rocq container image has
# ocamlopt for certain and ocamlfind probably. Neither is needed for the PROOFS, only
# for linking the extracted checker, so fall back rather than fail.
if [ -z "${OCAMLOPT:-}" ]; then
  if command -v ocamlfind >/dev/null 2>&1; then
    OCAMLOPT="ocamlfind ocamlopt"
  else
    OCAMLOPT="ocamlopt"
  fi
fi

# The theorems whose axiom footprint has to come back empty. Every one of them
# is quoted somewhere outside this directory, in rocq/README.md or in the PR
# that added it, so every one of them is a claim that can go stale.
AUDITED=(
  run_is_sound
  horn_certificate_sound
  entails_of_builtin_horn
  horn_certificate_sound_without_wellformedness
  wellformed_determines_instantiation
  no_substitution_extends_a_duplicate_key
  two_extensions_of_an_uncovered_binding_disagree
  builtin_sound
  the_conditions_are_satisfiable
  not_everything_is_absolutely_entailed
  the_relative_verdict_is_strictly_weaker
  the_built_in_table_is_the_committed_fixture
  good_is_accepted_with_the_absolute_verdict
  deep_mutual_support_is_rejected
  deep_mutual_seeded_is_accepted
)

build() {
  local root="$1"
  cd "$root"

  # The fixture bytes become Rocq strings here rather than being committed, so
  # the kernel always evaluates what is in the tree. See gen_fixture_data.sh.
  sh gen_fixture_data.sh

  "$ROCQ" makefile -f _CoqProject -o Makefile.rocq >/dev/null
  make -f Makefile.rocq -j "$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 2)"

  # Re-run the audit file on its own so its output is captured rather than
  # buried in a parallel make log.
  mkdir -p build
  "$ROCQ" c -q -Q theories OOCertRocq -w -notation-overridden,-deprecated \
      theories/Audit.v > build/audit.txt 2>&1 || {
    echo "the audit file failed to compile" >&2
    cat build/audit.txt >&2
    exit 1
  }
}

audit_gate() {
  local root="$1"
  cd "$root"
  local report="$root/build/audit.txt"
  local closed
  closed="$(grep -c 'Closed under the global context' "$report" || true)"
  local want="${#AUDITED[@]}"

  echo "--- axiom footprint (${report#"$here"/})"
  cat "$report"
  echo "---"

  if [ "$closed" -ne "$want" ]; then
    echo "AXIOM GATE FAILED: expected $want theorems closed under the global context, saw $closed." >&2
    echo "A theorem that depends on an axiom, or that is Admitted, prints a list of" >&2
    echo "constants instead of that phrase. The report above says which." >&2
    return 1
  fi

  # The report has to actually mention every audited name, or the count above
  # could be satisfied by printing one theorem fifteen times.
  local missing=()
  for n in "${AUDITED[@]}"; do
    if ! grep -q "Print Assumptions $n\." theories/Audit.v; then missing+=("$n"); fi
  done
  if [ ${#missing[@]} -ne 0 ]; then
    echo "AXIOM GATE FAILED: theories/Audit.v does not print assumptions for: ${missing[*]}" >&2
    return 1
  fi

  # Nothing in the theories may be admitted or postulated. Print Assumptions
  # catches a dependency; this catches a file that compiles with an unused one.
  if grep -nE '^\s*(Admitted|Axiom|Parameter|Conjecture|Hypothesis)\b' theories/*.v; then
    echo "AXIOM GATE FAILED: the lines above postulate rather than prove." >&2
    return 1
  fi
  echo "axiom gate: $closed of $want theorems closed under the global context, no postulates."
}

link() {
  local root="$1"
  cd "$root/driver"
  # shellcheck disable=SC2086
  $OCAMLOPT rocq_horn_core.mli rocq_horn_core.ml main.ml -o ../build/oo-horn-rocq
  rm -f ./*.cmi ./*.cmx ./*.o
  echo "linked $root/build/oo-horn-rocq"
}

if [ "${1:-}" = "--prove-the-gate-can-fail" ]; then
  scratch="$(mktemp -d)"
  trap 'rm -rf "$scratch"' EXIT
  cp -R "$here" "$scratch/rocq"
  mkdir -p "$scratch/tests/fixtures"
  cp -R "$here/../tests/fixtures/horn" "$scratch/tests/fixtures/horn"
  rm -rf "$scratch/rocq/build" "$scratch/rocq/theories"/*.vo* "$scratch/rocq/theories"/*.glob

  # One Qed becomes Admitted. The theorem still exists and every file still compiles;
  # only the audit changes. The line is FOUND rather than written down, so a rename or
  # a reflow cannot quietly turn this into a no-op that reports success.
  target="$scratch/rocq/theories/Sound.v"
  line="$(grep -n '^Theorem horn_certificate_sound :' "$target" | head -1 | cut -d: -f1)"
  if [ -z "$line" ]; then
    echo "cannot find horn_certificate_sound in Sound.v; fix build.sh" >&2
    exit 1
  fi
  qed="$(awk -v start="$line" 'NR>=start && $0=="Qed." {print NR; exit}' "$target")"
  if [ -z "$qed" ]; then
    echo "cannot find the Qed of horn_certificate_sound; fix build.sh" >&2
    exit 1
  fi
  awk -v n="$qed" 'NR==n {print "Admitted."; next} {print}' "$target" > "$target.tmp"
  mv "$target.tmp" "$target"
  echo "mutated Sound.v line $qed: Qed became Admitted"

  mkdir -p "$scratch/rocq/build"
  echo "=== building a copy with one Qed replaced by Admitted ==="
  if build "$scratch/rocq" && audit_gate "$scratch/rocq"; then
    echo "THE GATE CANNOT FAIL. An admitted proof passed the audit." >&2
    exit 1
  fi
  echo
  echo "the gate failed on an admitted proof, which is what it is for."
  exit 0
fi

mkdir -p "$here/build"
build "$here"
audit_gate "$here"
link "$here"
