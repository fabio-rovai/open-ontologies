#!/usr/bin/env bash
#
# Re-translate src/boundary_core.rs into aeneas/lean/OOBoundary/Generated.lean.
#
# OPTIONAL AND SEPARATE, the way the Kani harnesses are. Nothing in `make
# check`, `cargo build`, `cargo test` or `lean/`'s `lake build` calls this
# script, and nothing it produces is on any of those paths. It downloads a
# hundred and thirty megabytes of release tarball the first time it runs.
#
# What it does:
#   1. fetches the pinned Charon and Aeneas release for this platform;
#   2. runs Charon over aeneas/oo-boundary, which is src/boundary_core.rs
#      reached with #[path] and nothing else, producing LLBC;
#   3. runs Aeneas over the LLBC, producing the Lean model;
#   4. tells you whether the model changed.
#
# Step 4 is the one that matters in review. The proofs in
# aeneas/lean/OOBoundary/Proofs.lean are about the file this writes, so a
# change to src/boundary_core.rs that changes the model is a change that can
# break them, and the diff is how you find out.
#
# Building the proofs is a separate command and a separate toolchain:
#
#     cd aeneas/lean && lake exe cache get && lake build
#
# See docs/aeneas-boundary.md for what that pulls in and why it is not in
# lean/.
set -euo pipefail

# The pinned release. Aeneas publishes a nightly per day; the tag and the
# commit are both named so that the lakefile's `rev` and this script cannot
# drift apart silently.
AENEAS_TAG="${AENEAS_TAG:-nightly-2026.09.15-505b6ca}"
AENEAS_REV="505b6ca35217e7be5c96c3e2f8045edfbdf47291"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tools="${OO_AENEAS_HOME:-$here/.toolchain}"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) asset="aeneas-macos-aarch64.tar.gz" ;;
  Linux-x86_64) asset="aeneas-linux-x86_64.tar.gz" ;;
  Linux-aarch64) asset="aeneas-linux-aarch64.tar.gz" ;;
  *) echo "no Aeneas release for $(uname -s)-$(uname -m); build from source" >&2; exit 1 ;;
esac

if [ ! -x "$tools/aeneas/aeneas" ]; then
  echo "fetching Aeneas $AENEAS_TAG ($asset)"
  mkdir -p "$tools/aeneas"
  gh release download "$AENEAS_TAG" --repo AeneasVerif/aeneas \
    --pattern "$asset" --dir "$tools" --clobber
  tar xzf "$tools/$asset" -C "$tools/aeneas"
fi

# The Aeneas bundle ships the Charon binary it was built against, and the
# `rust-toolchain` that Charon links its `rustc_private` against. Using any
# other pair is how you get an LLBC that does not match the binary reading it.
shipped_channel="$(grep -E '^channel' "$tools/aeneas/rust-toolchain" | tr -d ' "')"
ours_channel="$(grep -E '^channel' "$here/oo-boundary/rust-toolchain" | tr -d ' "')"
if [ "$shipped_channel" != "$ours_channel" ]; then
  echo "TOOLCHAIN MISMATCH. Charon is a rustc driver and pins its nightly exactly." >&2
  echo "  release ships: $shipped_channel" >&2
  echo "  this repo has: $ours_channel" >&2
  echo "Update aeneas/oo-boundary/rust-toolchain, or pin an older AENEAS_TAG." >&2
  exit 1
fi

out="$repo/aeneas/lean/OOBoundary/Generated.lean"
previous="$(mktemp)"
[ -f "$out" ] && cp "$out" "$previous"

cd "$here/oo-boundary"
"$tools/aeneas/charon" cargo --preset aeneas
"$tools/aeneas/aeneas" -backend lean oo_boundary.llbc \
  -dest "$repo/aeneas/lean/OOBoundary" -namespace OOBoundary
mv "$repo/aeneas/lean/OOBoundary/OoBoundary.lean" "$out"

if [ -s "$previous" ] && ! diff -q "$previous" "$out" >/dev/null; then
  echo
  echo "THE MODEL CHANGED. The proofs in OOBoundary/Proofs.lean are about the"
  echo "old one. Diff:"
  diff -u "$previous" "$out" || true
  rm -f "$previous"
  exit 2
fi
rm -f "$previous"
echo "model unchanged: $out"
