#!/usr/bin/env bash
#
# Fail when an integration test file compiles to a target that collects no tests.
#
# `cargo test` prints "running 0 tests" followed by "test result: ok" for a file
# whose feature gate is not satisfied. That is indistinguishable from passing,
# which is the worst version of this failure: a green badge over a file that has
# never run.
#
# tests/schema_test.rs sat behind `#![cfg(feature = "postgres")]` while CI ran a
# bare `cargo test`, so its six tests were never collected once — including the
# one asserting against `SchemaIntrospector::sql_to_xsd`, which carried a real
# defect the whole time (IEEE 754 columns mapped to xsd:decimal, fixed in #80).
#
# A file whose gate is NOT in the enabled set is skipped: not being built is
# expected there, and flagging it would make the check permanently red on any
# partial-feature leg.
#
# The enabled set is derived from the same cargo flags the test step used, and
# expanded through `[features]` in Cargo.toml, so `--features sql` correctly
# counts as postgres + duckdb. Passing a hand-written list instead would go
# stale silently, in the direction that hides files rather than flagging them.
#
# Two gate forms are recognised, because both are in use here:
#   #![cfg(feature = "x")]              — inner attribute, gates the whole file
#   #[cfg(feature = "x")] mod tests {}  — outer attribute over the only test mod
# Anything more intricate (several distinct gates, tests outside the gated mod)
# is treated as ungated, i.e. expected to collect. When the shape is unclear the
# check errs toward checking: a spurious red is loud and takes a minute to fix,
# a spurious green is the bug this script exists to prevent.
#
# Platform gates (`#![cfg(unix)]` in tests/socket_test.rs) are out of scope: the
# features job runs on Linux only, where that file does collect.
#
# Usage: bash scripts/check-test-collection.sh <cargo-test-log> "<cargo flags>"
#   bash scripts/check-test-collection.sh cargo-test.log "--features postgres,sql"
#   bash scripts/check-test-collection.sh cargo-test.log "--all-features"
#
set -euo pipefail

log=${1:?usage: check-test-collection.sh <cargo-test-log> "<cargo flags>"}
flags=${2:-}

[ -f "$log" ] || { echo "::error::log file not found: $log"; exit 1; }
[ -f Cargo.toml ] || { echo "::error::Cargo.toml not found — run from the crate root"; exit 1; }

# --- what Cargo.toml declares -------------------------------------------------
# One "<name><TAB><comma-separated members>" line per [features] entry. Handles
# arrays spread over several lines, and strips comments.
feature_table=$(awk '
    /^[[:space:]]*\[/ { in_f = ($0 ~ /^[[:space:]]*\[features\][[:space:]]*$/); next }
    in_f { line = $0; sub(/#.*/, "", line); buf = buf " " line }
    END {
        while (match(buf, /[A-Za-z0-9_-]+[[:space:]]*=[[:space:]]*\[[^]]*\]/)) {
            entry = substr(buf, RSTART, RLENGTH)
            buf   = substr(buf, RSTART + RLENGTH)
            name  = substr(entry, 1, index(entry, "=") - 1)
            gsub(/[[:space:]]/, "", name)
            members = substr(entry, index(entry, "[") + 1)
            sub(/\][[:space:]]*$/, "", members)
            gsub(/[" \t]/, "", members)
            print name "\t" members
        }
    }
' Cargo.toml)

[ -n "$feature_table" ] || { echo "::error::no [features] section found in Cargo.toml"; exit 1; }

# --- what the cargo flags enable ----------------------------------------------
all_features=0
no_default=0
seeds=""

set -- $flags
while [ $# -gt 0 ]; do
    case "$1" in
        --all-features)        all_features=1 ;;
        --no-default-features) no_default=1 ;;
        --features|-F)         shift; seeds="$seeds $(printf '%s' "${1:-}" | tr ',' ' ')" ;;
        --features=*)          seeds="$seeds $(printf '%s' "${1#--features=}" | tr ',' ' ')" ;;
        -F*)                   seeds="$seeds $(printf '%s' "${1#-F}" | tr ',' ' ')" ;;
    esac
    shift
done

if [ "$all_features" -eq 1 ]; then
    seeds=$(printf '%s\n' "$feature_table" | cut -f1 | tr '\n' ' ')
elif [ "$no_default" -eq 0 ]; then
    seeds="$seeds default"
fi

# Transitive closure over [features]. `dep:foo` and `pkg/feat` entries name
# dependencies, not gates of ours, and are dropped.
queue=$seeds
enabled=""
while [ -n "${queue// /}" ]; do
    name=${queue%% *}
    if [ "$name" = "$queue" ]; then queue=""; else queue=${queue#* }; fi
    [ -n "$name" ] || continue
    case " $enabled " in *" $name "*) continue ;; esac
    enabled="$enabled $name"
    members=$(printf '%s\n' "$feature_table" | awk -F'\t' -v n="$name" '$1 == n { print $2 }')
    for m in $(printf '%s' "$members" | tr ',' ' '); do
        case "$m" in dep:*|*/*) continue ;; esac
        queue="$queue $m"
    done
done
enabled=" ${enabled# } "

# --- check every test file ----------------------------------------------------
missing=()
empty=()
checked=0
skipped=0

for f in tests/*.rs; do
    [ -e "$f" ] || continue

    # Gate for the whole file, or empty when the file is (or looks) ungated.
    gate=$(awk '
        /^#!\[cfg\(/ {
            line = $0
            while (match(line, /"[A-Za-z0-9_-]+"/)) {
                crate = crate " " substr(line, RSTART + 1, RLENGTH - 2)
                line  = substr(line, RSTART + RLENGTH)
            }
            next
        }
        /^#\[cfg\(feature[[:space:]]*=[[:space:]]*"[A-Za-z0-9_-]+"\)\][[:space:]]*$/ {
            match($0, /"[A-Za-z0-9_-]+"/)
            g = substr($0, RSTART + 1, RLENGTH - 2)
            outer++
            if (mod_gate == "")      mod_gate = g
            else if (mod_gate != g)  mixed = 1
            if (tests > 0)           late = 1
            next
        }
        /^[[:space:]]*#\[(tokio::)?test\]/ { tests++ }
        END {
            if (tests == 0)                                  { print "__no_tests__"; exit }
            if (crate != "")                                 { print substr(crate, 2); exit }
            if (outer == 1 && !mixed && !late)               { print mod_gate; exit }
            print ""
        }
    ' "$f")

    if [ "$gate" = "__no_tests__" ]; then
        skipped=$((skipped + 1))
        continue
    fi

    # Several gates on one file (`#![cfg(all(feature = "a", feature = "b"))]`)
    # all have to be enabled for its tests to exist.
    gate_off=0
    for g in $gate; do
        case "$enabled" in *" $g "*) ;; *) gate_off=1 ;; esac
    done
    if [ "$gate_off" -eq 1 ]; then
        skipped=$((skipped + 1))
        continue
    fi

    checked=$((checked + 1))

    # cargo prints:  Running tests/foo.rs (target/debug/deps/foo-<hash>)
    # then, after a blank line:  running N tests
    #
    # The "Running" banner is colourised, and CI sets CARGO_TERM_COLOR=always,
    # so the SGR reset sits between the word and the path. Strip CSI sequences
    # before matching — a plain substring search finds nothing otherwise, and
    # the failure is total rather than partial: every file looks unbuilt.
    count=$(awk -v target="$f" '
        BEGIN { esc = sprintf("%c", 27) }
        { gsub(esc "\\[[0-9;]*[a-zA-Z]", "") }
        index($0, "Running " target " ") { seen = 1; next }
        seen && $1 == "running" { print $2; exit }
    ' "$log")

    if [ -z "$count" ]; then
        missing+=("$f")
    elif [ "$count" -eq 0 ]; then
        empty+=("$f")
    fi
done

status=0

if [ ${#empty[@]} -gt 0 ]; then
    for f in "${empty[@]}"; do
        echo "::error file=$f::collected 0 tests while its feature gate is enabled"
    done
    status=1
fi

if [ ${#missing[@]} -gt 0 ]; then
    # Not one finding per file: when nothing at all was located, the log is not
    # what this script thinks it is, and printing 48 identical errors buries
    # that. (It happened: CARGO_TERM_COLOR=always put an SGR reset between
    # "Running" and the path, and every file looked unbuilt.)
    if [ ${#missing[@]} -eq "$checked" ] && [ "$checked" -gt 1 ]; then
        echo "::error::no test target was found for any of the $checked expected files"
        echo "The log does not look like \`cargo test\` output at all. Check that the"
        echo "test step really wrote it, and that the \"Running <path>\" banner still"
        echo "has the format this script parses."
    else
        for f in "${missing[@]}"; do
            echo "::error file=$f::no test target ran for this file"
        done
    fi
    status=1
fi

echo "enabled features:${enabled}"

if [ "$status" -eq 0 ]; then
    echo "test collection OK — $checked test file(s) expected, all collected at least one test ($skipped skipped as gated out)"
else
    echo
    echo "A test file that collects nothing reports \"test result: ok\" and is"
    echo "indistinguishable from a passing file. Either its gate is wrong, or the"
    echo "feature set this leg enables no longer matches the files it should build."
fi

exit "$status"
