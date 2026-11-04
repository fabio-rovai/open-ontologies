#!/usr/bin/env bash
#
# Fail when the workflows or the Dockerfile name more than one Rust version.
#
# CI, the release, the benchmark, the audit and the Docker image used to compile
# with whatever Rust was newest on the day: `dtolnay/rust-toolchain@stable` and
# `FROM rust:1-slim-bookworm`. On 1 Nov 2026 that was 1.99.1, and Rust 1.100.0
# is due on 12 Nov 2026 by the six-week schedule. A new stable brings new clippy
# lints, which `-D warnings` turns into errors in a required check, and it can
# stop a dependency compiling at all: ci.yml's Audit step installs cargo-audit
# with --locked because tinyvec 1.13.0 stopped compiling on a new stable. Either
# way a required check goes red with no commit here. The compiler now changes
# when a commit changes it, and this script is what makes a partial bump fail.
#
# What it requires:
#   - Every `dtolnay/rust-toolchain` step is pinned to a 40-character commit SHA,
#     and to the same one. A branch name runs whatever the branch holds that day.
#   - Every such step passes `toolchain:` as a full x.y.z version, and every one
#     names the same version. Two parts, such as 1.99, follow patch releases.
#   - Every job that runs cargo has such a step. Without one it compiles with
#     the Rust the runner image carries, which changes when the image does.
#   - Each `FROM rust:` line in the Dockerfile names that version too.
#   - Two places differ on purpose, and each is held to its own rule instead:
#     the `msrv` job in ci.yml must check exactly the rust-version Cargo.toml
#     declares, and be named after it, so neither can move without the other;
#     and canary.yml takes
#     stable and beta from its matrix, because its job is to show whether the
#     next version is safe. canary.yml is never a required check.
#
# Finding no pinned step at all is a failure too, so a renamed action or a moved
# directory cannot turn this into a check that passes by reading nothing.
#
# Usage: bash scripts/check-toolchain-pins.sh   (from the repository root)

set -euo pipefail

[ -f Cargo.toml ] || { echo "::error::Cargo.toml not found; run from the repository root"; exit 1; }
[ -f Dockerfile ] || { echo "::error::Dockerfile not found; run from the repository root"; exit 1; }

shopt -s nullglob
files=(.github/workflows/*.yml .github/workflows/*.yaml .github/actions/*/action.yml .github/actions/*/action.yaml)
[ ${#files[@]} -gt 0 ] || { echo "::error::no workflow files under .github/workflows/"; exit 1; }

# Three kinds of record, one per line, separated by tabs:
#   STEP <file> <line> <job> <ref> <toolchain>   a dtolnay/rust-toolchain step
#   BARE <file> <line> <job>                     a job that runs cargo without one
#   NAME <file> <line> <job> <name>              a job's display name
# A step runs from its `- ` line to the next line indented no deeper, so `uses:`
# and `with:` may come in either order. A `uses:` line is matched with its
# quotes stripped, so double, single or no quotes read the same. A job runs
# cargo when a line has `cargo` before a space, a tab or the line's end, as a
# word or as the last segment of a path such as ~/.cargo/bin/cargo; the
# directory in .cargo/ and the word cargo-audit do not count. Jobs are the keys
# one level under `jobs:`; a composite action has none. POSIX awk only, because
# CI has mawk and macOS has BSD awk.
records=$(awk -v sq="'" '
    BEGIN { OFS = "\t" }
    function depth(s) { match(s, /^ */); return RLENGTH }
    function bare(s) { sub(/[ \t]+#.*$/, "", s); gsub(/"/, "", s); gsub(sq, "", s); return s }
    function end_step() {
        if (in_step && uses != "") {
            print "STEP", file, uses_line, step_job, uses, (tc == "" ? "-" : tc)
            job_has_toolchain = 1
        }
        in_step = 0; uses = ""; tc = ""
    }
    function end_job() {
        end_step()
        if (job != "" && cargo_line > 0 && !job_has_toolchain) print "BARE", job_file, cargo_line, job
        job = ""; cargo_line = 0; job_has_toolchain = 0
    }
    FNR == 1 { end_job(); in_jobs = 0; job_depth = -1 }
    {
        line = $0
        sub(/\r$/, "", line)
        if (line ~ /^[ \t]*$/ || line ~ /^[ \t]*#/) next
        d = depth(line)
        if (in_step && d <= step_depth) end_step()
        if (d == 0) { end_job(); in_jobs = (line ~ /^jobs:[ \t]*(#.*)?$/); next }
        if (in_jobs && (job_depth < 0 || d == job_depth) && line ~ /^ *[A-Za-z0-9_-]+:[ \t]*(#.*)?$/) {
            end_job()
            job_depth = d; job_file = FILENAME; key_depth = -1
            job = line; sub(/^ */, "", job); sub(/:.*/, "", job)
            next
        }
        if (job != "" && d > job_depth) {
            if (key_depth < 0) key_depth = d
            if (d == key_depth && line ~ /^ *name:/) {
                n = line; sub(/^ *name:[ \t]*/, "", n)
                print "NAME", FILENAME, FNR, job, bare(n)
            }
        }
        if (job != "" && cargo_line == 0 && (" " line " ") ~ /[^A-Za-z0-9_.-]cargo[ \t]/) cargo_line = FNR
        if (!in_step && line ~ /^ *- /) {
            in_step = 1; step_depth = d; file = FILENAME; step_job = (job == "" ? "-" : job)
        }
        if (!in_step) next
        unquoted = line; gsub(/"/, "", unquoted); gsub(sq, "", unquoted)
        if (match(unquoted, /uses:[ \t]*dtolnay\/rust-toolchain@[^ \t#]*/)) {
            uses = substr(unquoted, RSTART, RLENGTH); sub(/.*@/, "", uses)
            if (uses == "") uses = "-"
            uses_line = FNR
        }
        if (line ~ /^[ \t-]*toolchain:/) {
            tc = line
            sub(/^[ \t-]*toolchain:[ \t]*/, "", tc)
            tc = bare(tc); gsub(/[ \t]/, "", tc)
        }
    }
    END { end_job() }
' "${files[@]}")

# The rust-version Cargo.toml declares under [package], or nothing.
msrv=$(awk '
    /^\[package\]/ { in_package = 1; next }
    /^\[/          { in_package = 0 }
    in_package && /^rust-version[ \t]*=/ {
        sub(/^rust-version[ \t]*=[ \t]*"/, ""); sub(/".*$/, ""); print; exit
    }
' Cargo.toml)
msrv_toolchain=""
if [ -n "$msrv" ]; then
    case "$msrv" in
        *.*.*) msrv_toolchain=$msrv ;;
        *.*)   msrv_toolchain=$msrv.0 ;;
        *)     echo "::error file=Cargo.toml::rust-version \"$msrv\" is not major.minor or major.minor.patch"; exit 1 ;;
    esac
fi

failed=0
fail() {
    # $1 file, $2 line or empty, $3 message, as a GitHub annotation.
    if [ -n "$2" ]; then echo "::error file=$1,line=$2::$3"; else echo "::error file=$1::$3"; fi
    failed=1
}
is_version() { [[ $1 =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; }
count_lines() { awk 'NF { n++ } END { print n + 0 }'; }

steps=0
refs=""
pinned=""
msrv_seen=0
while IFS=$'\t' read -r kind file line job ref toolchain; do
    [ -n "$kind" ] || continue
    if [ "$job" = "-" ]; then where=$file; else where="$job in $file"; fi
    if [ "$kind" = BARE ]; then
        fail "$file" "$line" "$where runs cargo with no dtolnay/rust-toolchain step, so it compiles with whatever Rust the runner image carries"
        continue
    fi
    if [ "$kind" = NAME ]; then
        # Only the msrv job's name carries a version, the one in Cargo.toml.
        if [ "$job" = msrv ] && [ "$file" = .github/workflows/ci.yml ] && [ -n "$msrv" ] \
            && [ "$ref" != "msrv ($msrv)" ]; then
            fail "$file" "$line" "$where is named '$ref', and Cargo.toml declares rust-version $msrv; name it 'msrv ($msrv)'"
        fi
        continue
    fi
    steps=$((steps + 1))
    if [[ $ref =~ ^[0-9a-f]{40}$ ]]; then
        refs="$refs$ref"$'\n'
    else
        fail "$file" "$line" "$where: dtolnay/rust-toolchain@$ref is not a commit SHA; pin the action by SHA and pass the version as toolchain:"
    fi
    if [ "$toolchain" = "-" ]; then
        fail "$file" "$line" "$where: no toolchain: input, so nothing here says which Rust this step installs"
        continue
    fi
    case "$file" in
        */canary.yml|*/canary.yaml) continue ;;  # floats on purpose; see the header
    esac
    if [ "$job" = msrv ] && [ "$file" = .github/workflows/ci.yml ]; then
        msrv_seen=1
        if [ -z "$msrv_toolchain" ]; then
            fail "$file" "$line" "$where checks Rust $toolchain, and Cargo.toml declares no rust-version"
        elif [ "$toolchain" != "$msrv_toolchain" ]; then
            fail "$file" "$line" "$where checks Rust $toolchain, and Cargo.toml declares rust-version $msrv ($msrv_toolchain)"
        fi
        continue
    fi
    if ! is_version "$toolchain"; then
        fail "$file" "$line" "$where: toolchain $toolchain is not a full x.y.z version"
        continue
    fi
    pinned="$pinned$toolchain $file:$line ($job)"$'\n'
done <<< "$records"

if [ "$steps" -eq 0 ]; then
    fail .github/workflows "" "no dtolnay/rust-toolchain step in any workflow; this check would pass by reading nothing"
fi
if [ -n "$msrv_toolchain" ] && [ "$msrv_seen" -eq 0 ]; then
    fail Cargo.toml "" "rust-version $msrv is declared and no msrv job in ci.yml checks it"
fi

distinct_refs=$(printf '%s' "$refs" | sort -u)
if [ "$(printf '%s' "$distinct_refs" | count_lines)" -gt 1 ]; then
    fail .github/workflows "" "dtolnay/rust-toolchain is pinned to more than one SHA: $(printf '%s' "$distinct_refs" | tr '\n' ' ')"
fi

versions=$(printf '%s' "$pinned" | awk 'NF { print $1 }' | sort -u)
version=""
case "$(printf '%s' "$versions" | count_lines)" in
    0) fail .github/workflows "" "no workflow step names a pinned Rust version" ;;
    1) version=$versions ;;
    *)
        fail .github/workflows "" "the workflows name more than one Rust version: $(printf '%s' "$versions" | tr '\n' ' ')"
        printf '%s' "$pinned" | sed 's/^/  /'
        ;;
esac

# Every FROM line on the official rust image, with or without a registry
# prefix, a --platform flag or a digest.
docker_from=$(awk '
    toupper($1) == "FROM" {
        for (i = 2; i <= NF; i++) if ($i !~ /^--/) break
        image = $i; sub(/[:@].*$/, "", image)
        if (image ~ /^(docker\.io\/)?(library\/)?rust$/) print NR, $i
    }
' Dockerfile)
if [ -z "$docker_from" ]; then
    fail Dockerfile "" "no FROM rust: line; if the builder image changed, teach this script where its version is"
fi
while read -r line image; do
    [ -n "$line" ] || continue
    tag=""
    case "$image" in *:*) tag=${image#*:} ;; esac
    tag=${tag%%@*}
    tag_version=${tag%%-*}
    if ! is_version "$tag_version"; then
        fail Dockerfile "$line" "FROM $image does not name a full x.y.z Rust version"
    elif [ -n "$version" ] && [ "$tag_version" != "$version" ]; then
        fail Dockerfile "$line" "FROM $image is Rust $tag_version, and the workflows pin $version"
    fi
done <<< "$docker_from"

if [ "$failed" -ne 0 ]; then
    echo
    echo "Bump every place together: each toolchain: input under .github/, the"
    echo "Dockerfile's FROM rust: tag, and, for the msrv job, rust-version in Cargo.toml."
    exit 1
fi

echo "Rust $version in $(printf '%s' "$pinned" | count_lines) workflow steps and in the Dockerfile, through dtolnay/rust-toolchain@$distinct_refs."
if [ -n "$msrv_toolchain" ]; then
    echo "The msrv job checks Rust $msrv_toolchain, the rust-version in Cargo.toml."
fi
