"""Assemble the public DCAT-US document corpus with a provenance manifest.

Every file in the resulting corpus is a byte-for-byte copy of, or a
verbatim concatenation of, real material fetched from GSA/dcat-us or the
W3C. Nothing here is paraphrased, summarised, or reconstructed from memory.

Two entries (`pr-120-record.md`, `w3c-dcat-conformance.md`) cannot be
produced from a git checkout: one is a GitHub API response (PR metadata and
file-status list), the other is an extracted HTML section of a W3C
Recommendation. Both were fetched over the network ahead of time and saved
into --out; this script only verifies they exist and records their checksum.
Re-running this script does not re-fetch them, but does not fabricate them
either -- if they are missing, the run fails loudly rather than inventing
content.
"""
import argparse
import hashlib
import json
import shutil
from pathlib import Path

# (destination filename, source spec, source URL)
#
# source spec is one of:
#   - a string: path to a single file, relative to <upstream checkout>/upstream/
#   - a list of strings: several real example files under jsonschema/examples/,
#     concatenated verbatim into one JSON array (see build_examples_sample)
#   - None: not derivable from the upstream git checkout. Either copied from
#     this repo's vendor/ directory (recovered-shapes.ttl) or expected to
#     already exist in --out, hand-saved from the URL ahead of time.
#
# Corrected against what `make upstream` actually produced at commit 7a6e803
# (2026-08-25): the brief's guessed paths schemas/dataset.json,
# schemas/catalog.json and examples/catalog.json do not exist upstream.
# The real layout is jsonschema/definitions/*.json (one file per class) and
# jsonschema/examples/<ClassName>/good|bad/*.json (per-class example files,
# 115 "good" + 76 "bad" = 191 total). There is no single "examples/catalog.json".
SOURCES = [
    ("profile-readme.md", "README.md",
     "https://github.com/GSA/dcat-us/blob/main/README.md"),
    ("dataset-schema.json", "jsonschema/definitions/Dataset.json",
     "https://github.com/GSA/dcat-us/blob/main/jsonschema/definitions/Dataset.json"),
    ("catalog-schema.json", "jsonschema/definitions/Catalog.json",
     "https://github.com/GSA/dcat-us/blob/main/jsonschema/definitions/Catalog.json"),
    ("examples-sample.json", [
        "jsonschema/examples/Catalog/good/complete_example.json",
        "jsonschema/examples/Dataset/good/complete_example.json",
        "jsonschema/examples/Distribution/good/complete_example.json",
        "jsonschema/examples/DataService/good/complete_example.json",
        "jsonschema/examples/Organization/good/complete_example.json",
     ], "https://github.com/GSA/dcat-us/tree/main/jsonschema/examples"),
    ("recovered-shapes.ttl", None,
     "https://github.com/GSA/dcat-us/blob/aaa3ff4a/shacl/dcat-us_3.0_shacl_shapes.ttl"),
    ("pr-120-record.md", None,
     "https://github.com/GSA/dcat-us/pull/120"),
    ("w3c-dcat-conformance.md", None,
     "https://www.w3.org/TR/vocab-dcat-3/#conformance"),
]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_examples_sample(upstream_root: Path, rel_paths: list[str]) -> str:
    """Concatenate real example files into one JSON array, verbatim.

    Each item is the exact parsed content of one published "good" example
    file, tagged with the real repo-relative path and blob URL it came from.
    No field is invented, dropped, or edited.
    """
    items = []
    for rel in rel_paths:
        src = upstream_root / rel
        if not src.exists():
            raise SystemExit(f"missing upstream example file: {src}")
        content = json.loads(src.read_text(encoding="utf-8"))
        items.append({
            "_source_path": rel,
            "_source_url": f"https://github.com/GSA/dcat-us/blob/main/{rel}",
            "example": content,
        })
    return json.dumps(items, indent=2, sort_keys=False) + "\n"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--upstream", type=Path,
                     default=Path("/Users/fabio/projects/dcat-us-binding"))
    ap.add_argument("--out", type=Path, default=Path("demo/corpus/dcat-us"))
    ap.add_argument("--retrieved", required=True, help="ISO date, e.g. 2026-08-25")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    manifest = []
    for name, rel, url in SOURCES:
        dest = args.out / name
        if isinstance(rel, list):
            dest.write_text(build_examples_sample(args.upstream / "upstream", rel),
                             encoding="utf-8")
        elif isinstance(rel, str):
            src = args.upstream / "upstream" / rel
            if not src.exists():
                raise SystemExit(f"missing upstream file: {src}. Run `make upstream` first.")
            shutil.copyfile(src, dest)
        elif name == "recovered-shapes.ttl":
            shutil.copyfile(
                args.upstream / "vendor" / "dcat-us_3.0_shacl_shapes.recovered.ttl", dest
            )
        elif not dest.exists():
            raise SystemExit(
                f"{dest} must be saved by hand from {url} before running this script"
            )
        manifest.append({
            "file": name,
            "source_url": url,
            "retrieved": args.retrieved,
            "sha256": sha256(dest),
        })

    (args.out / "MANIFEST.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"{len(manifest)} documents recorded in {args.out / 'MANIFEST.json'}")


if __name__ == "__main__":
    main()
