#!/usr/bin/env python3
"""Fail only on advisories a change brings in.

Usage: audit-new-advisories.py BASE.json HEAD.json

Both files are `cargo audit --json` output, for the base branch's Cargo.lock and
for the change's. A vulnerability is the triple (advisory id, crate, version).
One present on both sides was not introduced by the change: it is printed as a
warning, and main's own audit reports it. One present only in the change fails.
Unreadable or malformed input fails too, so a broken audit cannot pass.
"""
import json
import sys


def vulnerabilities(path):
    with open(path) as f:
        report = json.load(f)
    return {
        (v["advisory"]["id"], v["package"]["name"], v["package"]["version"])
        for v in report["vulnerabilities"]["list"]
    }


base, head = vulnerabilities(sys.argv[1]), vulnerabilities(sys.argv[2])
for advisory, crate, version in sorted(head & base):
    print(f"::warning::{advisory} in {crate} {version} is also on the base branch, so it does not block this change")
introduced = sorted(head - base)
for advisory, crate, version in introduced:
    print(f"::error::{advisory} in {crate} {version} arrives with this change")
print(f"{len(head)} advisories on this change, {len(base)} on the base branch, {len(introduced)} introduced here")
sys.exit(1 if introduced else 0)
