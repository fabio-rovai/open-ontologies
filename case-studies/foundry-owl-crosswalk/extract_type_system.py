"""Derive the Foundry object property type system from Palantir's own SDK source.

The list of property types a Foundry Ontology can hold is not typed by hand
anywhere in this case study. It is parsed out of the `ObjectPropertyType` union
in Palantir's published models, so it stays correct when Palantir changes it and
fails loudly when the parse stops matching.
"""
import json
import pathlib
import re

HERE = pathlib.Path(__file__).parent
MODELS = HERE / "vendor" / "models.py"
OUT = HERE / "data" / "foundry-type-system.json"

UNION_RE = re.compile(
    r"ObjectPropertyType: typing_extensions\.TypeAlias = .*?typing\.Union\[(.*?)\]",
    re.DOTALL,
)


def discriminator_for(class_name: str, source: str) -> str:
    """Find the JSON `type` discriminator Palantir assigns to a model class."""
    pattern = re.compile(
        r"class " + re.escape(class_name) + r"\(core\.ModelBase\):(.*?)(?=\nclass |\Z)",
        re.DOTALL,
    )
    match = pattern.search(source)
    if not match:
        return ""
    literal = re.search(r'type: typing\.Literal\["([^"]+)"\]', match.group(1))
    return literal.group(1) if literal else ""


def main() -> None:
    source = MODELS.read_text()
    union = UNION_RE.search(source)
    if union is None:
        raise SystemExit("ObjectPropertyType union not found; Palantir changed the SDK layout")

    members = [
        member.strip().strip('"').split(".")[-1]
        for member in union.group(1).split(",")
        if member.strip()
    ]

    core_models = HERE / "vendor" / "core_models.py"
    core_source = core_models.read_text() if core_models.exists() else ""

    types = []
    for member in members:
        discriminator = discriminator_for(member, source) or discriminator_for(
            member, core_source
        )
        types.append({"model": member, "discriminator": discriminator})

    OUT.write_text(
        json.dumps({"count": len(types), "types": types}, indent=2) + "\n"
    )
    print(f"{len(types)} property types parsed from ObjectPropertyType union")
    missing = [t["model"] for t in types if not t["discriminator"]]
    if missing:
        print(f"discriminator not resolved locally for {len(missing)}: {', '.join(missing)}")


if __name__ == "__main__":
    main()
