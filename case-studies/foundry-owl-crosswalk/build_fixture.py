"""Assemble a Foundry ontology export from Palantir's own published fixtures.

Palantir ships the fixtures it uses to mock its own API, including an object
type named `objectTypeWithAllPropertyTypes` that exercises every property type
in the union. Those fixtures are TypeScript. This module strips the type
annotations, evaluates the object literals with node, and assembles them into
the `OntologyFullMetadata` shape that the v2 API returns, so the crosswalk is
exercised against Palantir's data rather than data invented here.
"""
import json
import pathlib
import re
import subprocess
import tempfile

HERE = pathlib.Path(__file__).parent
VENDOR = HERE / "vendor"
OUT = HERE / "data" / "foundry-ontology.json"

ORDER = ["spts.ts", "objectTypeV2.ts", "linkTypes.ts", "interfaceTypes.ts"]

IMPORT_RE = re.compile(
    r'^import\s+(?:type\s+)?(?:\{[^}]*\}|\w+)\s+from\s+"[^"]+";', re.M | re.S
)
ANNOTATION_RE = re.compile(r"^(\s*(?:export\s+)?const\s+\w+)\s*:\s*[A-Za-z0-9_.]+(?:\[\])?\s*=", re.M)
SATISFIES_RE = re.compile(r"\s+satisfies\s+[A-Za-z0-9_.]+")
AS_CAST_RE = re.compile(r"\s+as\s+(?:const|[A-Za-z0-9_.]+(?:\[\])?)\b")


def strip_typescript(source: str) -> tuple[str, list[str]]:
    exported = re.findall(r"^export\s+const\s+(\w+)", source, re.M)
    source = IMPORT_RE.sub("", source)
    source = ANNOTATION_RE.sub(r"\1 =", source)
    source = SATISFIES_RE.sub("", source)
    source = AS_CAST_RE.sub("", source)
    source = re.sub(r"^export\s+const\s+", "const ", source, flags=re.M)
    return source, exported


def evaluate() -> dict:
    chunks, names = [], []
    for filename in ORDER:
        stripped, exported = strip_typescript((VENDOR / filename).read_text())
        chunks.append(f"// ---- {filename}\n{stripped}")
        names.extend(exported)
    unique = sorted(set(names))
    collector = ",\n".join(f'  "{name}": {name}' for name in unique)
    script = "\n".join(chunks) + f"\nconst __out = {{\n{collector}\n}};\nconsole.log(JSON.stringify(__out));\n"
    with tempfile.NamedTemporaryFile("w", suffix=".mjs", delete=False) as handle:
        handle.write(script)
        path = handle.name
    result = subprocess.run(["node", path], capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(f"node failed evaluating fixtures:\n{result.stderr[:2000]}")
    return json.loads(result.stdout)


def assemble(values: dict) -> dict:
    """Build the OntologyFullMetadata shape the v2 API returns."""
    object_types = {
        value["apiName"]: value
        for value in values.values()
        if isinstance(value, dict) and "primaryKey" in value and "properties" in value
    }
    link_sides = [
        value
        for value in values.values()
        if isinstance(value, dict) and "cardinality" in value and "linkTypeRid" in value
    ]
    interfaces = {
        value["apiName"]: value
        for value in values.values()
        if isinstance(value, dict) and "extendsInterfaces" in value
    }
    shared_properties = {
        value["apiName"]: value
        for value in values.values()
        if isinstance(value, dict)
        and "dataType" in value
        and "apiName" in value
        and "rid" in value
        and str(value.get("rid", "")).startswith("ri.sharedPropertyType")
    }

    # A link side names the object type it points AT. Group by rid so the two
    # sides of one link are matched, then attach each side to its source type.
    by_rid: dict[str, list[dict]] = {}
    for side in link_sides:
        by_rid.setdefault(side["linkTypeRid"], []).append(side)

    attached: dict[str, list[dict]] = {name: [] for name in object_types}
    for sides in by_rid.values():
        if len(sides) == 2:
            first, second = sides
            attached.setdefault(second["objectTypeApiName"], []).append(first)
            attached.setdefault(first["objectTypeApiName"], []).append(second)
        else:
            for side in sides:
                attached.setdefault(side["objectTypeApiName"], []).append(side)

    full = {}
    for api_name, object_type in object_types.items():
        full[api_name] = {
            "objectType": object_type,
            "linkTypes": attached.get(api_name, []),
            "implementsInterfaces": [],
            "implementsInterfaces2": {},
            "sharedPropertyTypeMapping": {},
        }

    return {
        "ontology": {
            "apiName": "palantir-osdk-test-fixtures",
            "displayName": "Palantir OSDK shared test fixtures",
            "description": (
                "Assembled from the fixtures Palantir publishes in "
                "palantir/osdk-ts to mock the Foundry Ontology API."
            ),
            "rid": "ri.ontology.main.ontology.fixture",
        },
        "objectTypes": full,
        "actionTypes": {},
        "actionTypesFullMetadata": {},
        "queryTypes": {},
        "interfaceTypes": interfaces,
        "sharedPropertyTypes": shared_properties,
        "valueTypes": {},
    }


def main() -> None:
    metadata = assemble(evaluate())
    OUT.write_text(json.dumps(metadata, indent=2) + "\n")
    properties = sum(
        len(entry["objectType"]["properties"]) for entry in metadata["objectTypes"].values()
    )
    links = sum(len(entry["linkTypes"]) for entry in metadata["objectTypes"].values())
    print(
        f"object types: {len(metadata['objectTypes'])}  "
        f"properties: {properties}  link sides: {links}  "
        f"interfaces: {len(metadata['interfaceTypes'])}  "
        f"shared property types: {len(metadata['sharedPropertyTypes'])}"
    )


if __name__ == "__main__":
    main()
