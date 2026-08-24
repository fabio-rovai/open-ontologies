#!/usr/bin/env bash
# extract-changelog.sh <version>
#
# Prints the CHANGELOG.md section body for the given version to stdout,
# excluding the heading line itself. Handles the mixed heading formats
# used in this repo:
#
#   ## 1.1.1 <em dash> 2026-08-03   (bare version, em dash separator)
#   ## [0.1.13] - 2026-05-01   (bracketed version, ASCII hyphen)
#   ## [Unreleased]
#
# The section ends at the next line starting with "## " or at EOF.
# Leading and trailing blank lines are stripped. A leading "v" on the
# argument is accepted ("v1.1.0" == "1.1.0"). Exits non-zero with a
# message on stderr when the version is not found.

set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: $0 <version>" >&2
  exit 2
fi

version="${1#v}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
changelog="${script_dir}/../CHANGELOG.md"

if [ ! -f "$changelog" ]; then
  echo "error: CHANGELOG.md not found at $changelog" >&2
  exit 1
fi

awk -v version="$version" '
  /^## / {
    if (printing) exit 0
    heading = $0
    sub(/^## /, "", heading)
    gsub(/[\[\]]/, "", heading)
    split(heading, parts, /[ \t]+/)
    if (parts[1] == version) { found = 1; printing = 1 }
    next
  }
  printing { lines[++n] = $0 }
  END {
    if (!found) exit 1
    first = 1
    while (first <= n && lines[first] ~ /^[[:space:]]*$/) first++
    last = n
    while (last >= first && lines[last] ~ /^[[:space:]]*$/) last--
    for (i = first; i <= last; i++) print lines[i]
  }
' "$changelog" || {
  echo "error: version '$1' not found in CHANGELOG.md" >&2
  exit 1
}
