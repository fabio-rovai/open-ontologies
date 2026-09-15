#!/bin/sh
# Link the verified checker into a native executable, so the differential harness can
# run hundreds of certificates without paying Poly/ML's compile time on every one.
#
#   isabelle/build_native.sh  ->  isabelle/build/oo-horn-isabelle
#
# This is the LINK STEP ONLY. Nothing is recompiled from the theories: the input is
# driver/oo_horn_generated.ML, which `isabelle/export.sh` writes out of the session, plus
# driver/oo_horn_cli.sml, which is untrusted by construction.
#
# Why not `polyc`. Poly/ML ships one, and it does not work here: its CFLAGS carry the
# absolute path of the machine Isabelle was BUILT on
# (/tmp/isabelle-makarius/build.../gmp/target/lib), and `-lgmp` then resolves against a
# directory that does not exist on this machine. The bundled runtime directory holds
# libgmp.10.dylib but no libgmp.dylib, so the linker cannot find it there either. The two
# steps below are what polyc does, with the gmp search path pointed somewhere real.
#
# LINUX. This script was macOS-only until 15 September 2026: it looked for `libgmp.dylib`
# and for an Isabelle app bundle under /Applications, so the whole Isabelle half of the
# cross-kernel differential could only run on a developer's Mac, and the README claim it
# backs was therefore gated by nothing in CI. The full Isabelle distribution is NOT needed
# to run it, because `driver/oo_horn_generated.ML` is committed. Only Poly/ML is, and Isabelle
# publishes that on its own as a ~39 MB component carrying every platform. So CI sets
# POLYDIR at .../polyml-5.9.2-2/x86_64-linux and this script links there. See
# .github/workflows/ci.yml, the "second kernel" steps of the `lean` job.
#
# Two things differ on Linux and both have bitten this repository before. GNU ld resolves
# libraries strictly left to right, so the object file comes first and `-lpolymain` before
# `-lpolyml`, which is also the order Poly/ML's own `polyc` uses. And the runtime needs
# `-lpthread -lm -ldl` there, which `polyml.pc` lists and macOS does not have as separate
# libraries at all: passing `-ldl` to Apple's linker fails, because libdl is inside
# libSystem. The link line is therefore built per platform rather than shared.
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=${1:-$HERE/build/oo-horn-isabelle}

# Where Poly/ML lives. POLYDIR wins; then Isabelle, because ISABELLE_HOME is the only
# answer that is right on a machine that is not this one; then the macOS app bundle; then
# a Poly/ML on PATH that was built from source, whose libraries sit beside its binary.
ARCH=$(uname -m); [ "$ARCH" = "aarch64" ] && ARCH=arm64
OS=$(uname -s | tr 'A-Z' 'a-z')
if [ -z "$POLYDIR" ] && command -v isabelle >/dev/null 2>&1; then
  IHOME=$(isabelle getenv -b ISABELLE_HOME 2>/dev/null || true)
  [ -n "$IHOME" ] && POLYDIR=$(ls -d "$IHOME"/contrib/polyml-*/"$ARCH-$OS" 2>/dev/null | head -1)
fi
POLYDIR=${POLYDIR:-$(ls -d /Applications/Isabelle*.app/contrib/polyml-*/"$ARCH-$OS" 2>/dev/null | head -1)}
if [ -z "$POLYDIR" ] && command -v poly >/dev/null 2>&1; then
  CAND=$(dirname "$(command -v poly)")
  [ -f "$CAND/libpolyml.a" ] && POLYDIR=$CAND
fi
[ -x "$POLYDIR/poly" ] || { echo "poly not found; set POLYDIR=/path/to/polyml/arch-dir" >&2; exit 3; }
# The archives are what this script LINKS, and a directory holding `poly` without them is
# the shape Ubuntu's `polyml` package has: it ships /usr/bin/poly and libpolymain.a and no
# libpolyml.a at all, so the failure would otherwise land in the linker with no
# explanation of what is missing.
for lib in libpolymain.a libpolyml.a; do
  [ -f "$POLYDIR/$lib" ] || {
    echo "$POLYDIR has poly but no $lib, so there is nothing to link against." >&2
    echo "Point POLYDIR at a full Poly/ML library directory: an Isabelle contrib/polyml-*" >&2
    echo "platform directory, or the target/ of a from-source build." >&2
    exit 3
  }
done

# Where libgmp actually lives on this machine. A system copy first, then the one Isabelle
# bundles, which carries only the versioned name and is therefore linked by full path.
if [ -f /opt/homebrew/lib/libgmp.dylib ]; then
  GMP="-L/opt/homebrew/lib -lgmp"
elif [ -f /usr/local/lib/libgmp.dylib ]; then
  GMP="-L/usr/local/lib -lgmp"
elif [ -f "$POLYDIR/libgmp.10.dylib" ]; then
  GMP="$POLYDIR/libgmp.10.dylib"
elif ls /usr/lib/*/libgmp.so >/dev/null 2>&1 || [ -f /usr/lib/libgmp.so ]; then
  GMP="-lgmp"
elif [ -f "$POLYDIR/libgmp.so.10" ]; then
  GMP="$POLYDIR/libgmp.so.10"
else
  echo "libgmp not found; install gmp (libgmp-dev on Debian) or set GMP=..." >&2; exit 3
fi

# The platform's own runtime libraries. `polyml.pc` says -lpthread -lgmp -lm -ldl -lstdc++.
case "$OS" in
  darwin) SYSLIBS="-lstdc++" ;;
  *)      SYSLIBS="-lpthread -lm -ldl -lstdc++" ;;
esac

mkdir -p "$(dirname "$OUT")"
OBJ=$(mktemp "${TMPDIR:-/tmp}/oo_horn_obj.XXXXXX").o

# 1. Compile the generated core and the CLI, and export `main` as an object file.
printf 'val () = use "%s";\nval () = use "%s";\nval () = PolyML.export("%s", main);\n' \
  "$HERE/driver/oo_horn_generated.ML" "$HERE/driver/oo_horn_cli.sml" "$OBJ" \
  | "$POLYDIR/poly" -q --error-exit

# 2. Link it against the Poly/ML runtime.
#
# The linker's stderr used to go to /dev/null, which hid the ONE message that ever
# matters here. It is captured instead and printed only when the link fails, so a
# successful build stays quiet and a broken one says why.
LOG=$(mktemp "${TMPDIR:-/tmp}/oo_horn_link.XXXXXX")
if ! g++ -std=gnu++11 -O2 "$OBJ" -o "$OUT" \
     -L"$POLYDIR" -Wl,-rpath,"$POLYDIR" -lpolymain -lpolyml $GMP $SYSLIBS 2>"$LOG"; then
  echo "link failed against POLYDIR=$POLYDIR" >&2
  cat "$LOG" >&2
  rm -f "$LOG" "$OBJ"
  exit 3
fi
rm -f "$LOG"

rm -f "$OBJ"
echo "wrote $OUT (POLYDIR=$POLYDIR)"
