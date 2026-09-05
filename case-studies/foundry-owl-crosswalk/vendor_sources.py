"""Vendor Palantir's own published ontology definitions, with provenance.

Ground truth for this case study is not a blog post or a marketing PDF. It is
Palantir's own Apache-2.0 licensed SDK source: the platform SDK models that
define what a Foundry Ontology is over the wire, and the OSDK test fixtures
that Palantir uses to mock its own API.
"""
import hashlib
import json
import pathlib
import urllib.request

HERE = pathlib.Path(__file__).parent
VENDOR = HERE / "vendor"

SOURCES = {
    "models.py": (
        "https://raw.githubusercontent.com/palantir/foundry-platform-python/"
        "develop/foundry_sdk/v2/ontologies/models.py",
        "palantir/foundry-platform-python",
        "Apache-2.0",
        "Wire model definitions for the v2 Ontologies API.",
    ),
    "core_models.py": (
        "https://raw.githubusercontent.com/palantir/foundry-platform-python/"
        "develop/foundry_sdk/v2/core/models.py",
        "palantir/foundry-platform-python",
        "Apache-2.0",
        "Core wire models: the scalar property data types.",
    ),
    "ies-common.ttl": (
        "https://raw.githubusercontent.com/IES-Org/ont-ies/main/docs/specification/ies-common.ttl",
        "IES-Org/ont-ies",
        "OGL / see repository",
        "The UK Government Information Exchange Standard, version 5.0.3. The "
        "earlier dstl/IES4 repository was archived on 4 March 2025.",
    ),
    "objectTypeV2.ts": (
        "https://raw.githubusercontent.com/palantir/osdk-ts/"
        "main/packages/shared.test/src/stubs/objectTypeV2.ts",
        "palantir/osdk-ts",
        "Apache-2.0",
        "Object type fixtures used by Palantir to mock the Foundry API.",
    ),
    "linkTypes.ts": (
        "https://raw.githubusercontent.com/palantir/osdk-ts/"
        "main/packages/shared.test/src/stubs/linkTypes.ts",
        "palantir/osdk-ts",
        "Apache-2.0",
        "Link type fixtures used by Palantir to mock the Foundry API.",
    ),
    "spts.ts": (
        "https://raw.githubusercontent.com/palantir/osdk-ts/"
        "main/packages/shared.test/src/stubs/spts.ts",
        "palantir/osdk-ts",
        "Apache-2.0",
        "Shared property type fixture.",
    ),
    "interfaceTypes.ts": (
        "https://raw.githubusercontent.com/palantir/osdk-ts/"
        "main/packages/shared.test/src/stubs/interfaceTypes.ts",
        "palantir/osdk-ts",
        "Apache-2.0",
        "Interface type fixtures.",
    ),
}


def main() -> None:
    VENDOR.mkdir(exist_ok=True)
    manifest = []
    for name, (url, repo, licence, note) in SOURCES.items():
        target = VENDOR / name
        if not target.exists():
            with urllib.request.urlopen(url, timeout=60) as response:
                target.write_bytes(response.read())
        payload = target.read_bytes()
        manifest.append(
            {
                "file": name,
                "url": url,
                "repository": repo,
                "licence": licence,
                "note": note,
                "bytes": len(payload),
                "sha256": hashlib.sha256(payload).hexdigest(),
            }
        )
    (HERE / "data" / "palantir-sources.json").write_text(
        json.dumps({"sources": manifest}, indent=2) + "\n"
    )
    for entry in manifest:
        print(f"{entry['file']:20s} {entry['bytes']:>8d} bytes  {entry['sha256'][:16]}")


if __name__ == "__main__":
    main()
