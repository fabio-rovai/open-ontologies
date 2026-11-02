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
# so does v1.6.0, which reports 1.5.0.
#
# Two JSON manifests record the version too, and the single-tag form checks them
# against the tag as well. .mcp/server.json is the MCP registry entry, and two
# of its values are checked: its `version`, and the tag in its OCI `identifier`,
# which names the image a client installs (release.yml pushes it as X.Y.Z, with
# no v). .claude-plugin/plugin.json is the Claude Code plugin manifest, and
# Claude Code keeps an installed plugin on its cached copy until that `version`
# changes. Nothing checked either file, and both drifted: at every tag up to
# v2.1.0, server.json named an older release than its tag (0.1.9 and the :0.1.9
# image from v0.1.10 on), and plugin.json said 0.1.0.
#
# Usage:
#   scripts/check-release-version.sh v1.6.0   # one tag, against all three files
#   scripts/check-release-version.sh --all    # every v* tag, against Cargo.toml
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

manifest_versions() {
    # Every version a JSON manifest records, one `<field><TAB><value>` line
    # each: the top-level `version`, any package `version`, and the tag of any
    # OCI image. A field that is missing or not a string prints empty, so it
    # fails the comparison instead of being skipped. Python's json module reads
    # it, because a pattern over the text cannot tell which `version` it
    # matched, and -I keeps a json.py in the working directory from being
    # imported.
    python3 -I - "$1" <<'PY'
import json
import sys

path = sys.argv[1]
try:
    with open(path, encoding="utf-8") as f:
        manifest = json.load(f)
except (OSError, ValueError) as e:
    sys.exit(f"{path}: {e}")
if not isinstance(manifest, dict):
    sys.exit(f"{path}: not a JSON object")


def text(value):
    return value if isinstance(value, str) else ""


def show(field, value):
    # Escaped, so a tab or newline inside a value cannot split or pad the line.
    print(field, json.dumps(value)[1:-1], sep="\t")


show("version", text(manifest.get("version")))
for i, package in enumerate(manifest.get("packages") or []):
    if not isinstance(package, dict):
        sys.exit(f"{path}: packages[{i}] is not a JSON object")
    if "version" in package:
        show(f"packages[{i}] version", text(package["version"]))
    if package.get("registryType") == "oci":
        # registry/namespace/repository:tag, optionally @digest. A registry
        # port has a colon too, so only the last path segment is read.
        image = text(package.get("identifier")).split("@", 1)[0]
        tag = image.rsplit("/", 1)[-1].partition(":")[2]
        show(f"packages[{i}] image tag", tag)
PY
}

check_one() {
    local tag="$1" declared="$2" expected="${1#v}" source="${3:-Cargo.toml}"
    if [ "$declared" != "$expected" ]; then
        printf '%s: %s says %s\n' "$tag" "$source" "${declared:-<unset>}"
        return 1
    fi
    return 0
}

# The audit reads Cargo.toml only. Every tag up to v2.1.0 predates the manifest
# check, v0.1.0 predates server.json itself, and none of them would pass it, so
# reading the manifests here would flag every tag and hide the ones whose binary
# reports the wrong version, which is what the audit is for.
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

failed=0
declared="$(cargo_version Cargo.toml)"
if check_one "$tag" "$declared"; then
    printf '%s matches Cargo.toml version %s\n' "$tag" "$declared"
else
    failed=1
fi
for manifest in .mcp/server.json .claude-plugin/plugin.json; do
    # A manifest that cannot be read fails the tag; it is never skipped.
    if ! fields="$(manifest_versions "$manifest")"; then
        printf '%s: %s could not be read\n' "$tag" "$manifest"
        failed=1
        continue
    fi
    while IFS=$'\t' read -r field value; do
        if check_one "$tag" "$value" "$manifest $field"; then
            printf '%s matches %s %s %s\n' "$tag" "$manifest" "$field" "$value"
        else
            failed=1
        fi
    done <<<"$fields"
done
if [ "$failed" -eq 0 ]; then
    exit 0
fi
cat >&2 <<MSG

The tag being released does not match every version recorded for it. The
binary reports Cargo.toml's, the MCP registry publishes .mcp/server.json's with
the image it names, and Claude Code updates the plugin only when
.claude-plugin/plugin.json's changes. Set each one named above to ${tag#v} and
move the tag onto that commit, or tag the commit that already carries it.
MSG
exit 1
