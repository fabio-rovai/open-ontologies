#!/usr/bin/env bash
#
# Fail when a release tag does not carry the version it claims.
#
# The tag and the `version` field in Cargo.toml are set by separate commits, so
# a tag can land before the bump. When it does, the release builds and publishes
# cleanly while the binary reports the previous version. `status` is the only
# version the engine reports about itself, so anything recording which engine
# produced an artifact records the wrong one, and a consumer pinning a release
# and verifying by the reported version cannot tell a correct build from a
# failed pin.
#
# It is a silent failure and it flatters the release: everything is green, the
# artifacts exist, and only the number is wrong.
#
# Three tags already carry it. v1.2.1 and v1.4.0 ship the previous version, and
# so does v1.6.0, the current release, which reports 1.5.0.
#
# Usage:
#   scripts/check-release-version.sh v1.6.0   # check one tag against Cargo.toml
#   scripts/check-release-version.sh --all    # audit every v* tag in the repo
#
# In CI the release workflow calls the single-tag form before anything is built,
# so a mismatched tag fails before an artifact carrying the wrong number exists.

set -euo pipefail

cargo_version() {
    # The [package] version, read without assuming a TOML parser is installed.
    awk '
        /^\[package\]/        { in_package = 1; next }
        /^\[/                 { in_package = 0 }
        in_package && /^version[[:space:]]*=/ {
            gsub(/^version[[:space:]]*=[[:space:]]*"/, "")
            gsub(/".*$/, "")
            print
            exit
        }
    ' "${1:-Cargo.toml}"
}

check_one() {
    local tag="$1" declared="$2" expected="${1#v}"
    if [ "$declared" != "$expected" ]; then
        printf '%s: Cargo.toml says %s\n' "$tag" "${declared:-<unset>}"
        return 1
    fi
    return 0
}

if [ "${1:-}" = "--all" ]; then
    failed=0
    while read -r tag; do
        declared="$(git show "${tag}:Cargo.toml" 2>/dev/null | cargo_version /dev/stdin || true)"
        check_one "$tag" "$declared" || failed=$((failed + 1))
    done < <(git tag --list 'v*' --sort=v:refname)
    if [ "$failed" -gt 0 ]; then
        printf '\n%d tag(s) do not carry the version they claim.\n' "$failed"
        exit 1
    fi
    echo "every v* tag carries the version it claims"
    exit 0
fi

tag="${1:-}"
if [ -z "$tag" ]; then
    echo "usage: $0 <tag>|--all" >&2
    exit 2
fi

declared="$(cargo_version Cargo.toml)"
if check_one "$tag" "$declared"; then
    printf '%s matches Cargo.toml version %s\n' "$tag" "$declared"
    exit 0
fi
cat >&2 <<MSG

The tag being released does not match the version the binary will report.
Bump Cargo.toml to ${tag#v} and move the tag onto that commit, or tag the
commit that already carries it.
MSG
exit 1
