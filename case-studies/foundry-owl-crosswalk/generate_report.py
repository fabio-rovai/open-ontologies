"""Render the coverage report and its chart from the measured data.

No number in the report or the chart is typed here. Everything is read from
the JSON the crosswalk and the audit produce, so a changed measurement changes
the prose and the picture together.
"""
from __future__ import annotations

import json
import pathlib

HERE = pathlib.Path(__file__).parent
TYPES = json.loads((HERE / "data" / "foundry-type-system.json").read_text())
REPORT = json.loads((HERE / "data" / "crosswalk-report.json").read_text())
AUDIT = json.loads((HERE / "data" / "owl-to-foundry-audit.json").read_text())
SOURCES = json.loads((HERE / "data" / "palantir-sources.json").read_text())

import foundry_owl  # noqa: E402

INK = "#1a2b32"
MUTED = "#5c6670"
TEAL = "#00897b"
AMBER = "#b45309"
GRAY = "#8c959a"
RULE = "#d7dee0"

FIDELITY_COLOUR = {
    "direct": TEAL,
    "standard": "#4db6ac",
    "structural": GRAY,
    "none": AMBER,
}
FIDELITY_LABEL = {
    "direct": "Direct XSD counterpart",
    "standard": "Carried by OGC GeoSPARQL",
    "structural": "Structural, expressed as a shape",
    "none": "No counterpart in any standard",
}
VERDICT_COLOUR = {"carried": TEAL, "partial": GRAY, "none": AMBER}
VERDICT_LABEL = {
    "carried": "A Foundry field holds it",
    "partial": "A Foundry field holds a weaker version",
    "none": "No Foundry field can hold it",
}


def type_system_counts() -> dict[str, int]:
    counts: dict[str, int] = {}
    for mapping in foundry_owl.TYPE_MAP.values():
        counts[mapping.fidelity] = counts.get(mapping.fidelity, 0) + 1
    return counts


def bars(rows, width=470, label_width=250, row_height=30) -> str:
    """A horizontal bar chart as inline SVG. Rows are (label, value, colour)."""
    largest = max((value for _, value, _ in rows), default=1) or 1
    bar_width = width - label_width - 60
    height = row_height * len(rows) + 10
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" '
        f'width="{width}" height="{height}" role="img">'
    ]
    for index, (label, value, colour) in enumerate(rows):
        y = index * row_height + 8
        length = max(2, int(bar_width * value / largest))
        parts.append(
            f'<text x="0" y="{y + 13}" font-family="system-ui,sans-serif" '
            f'font-size="12" fill="{INK}">{label}</text>'
        )
        parts.append(
            f'<rect x="{label_width}" y="{y + 2}" width="{length}" height="15" '
            f'rx="2" fill="{colour}"/>'
        )
        parts.append(
            f'<text x="{label_width + length + 7}" y="{y + 14}" '
            f'font-family="system-ui,sans-serif" font-size="12" font-weight="600" '
            f'fill="{MUTED}">{value}</text>'
        )
    parts.append("</svg>")
    return "".join(parts)


def main() -> None:
    counts = type_system_counts()
    order = ["direct", "standard", "structural", "none"]

    type_chart = bars(
        [(FIDELITY_LABEL[key], counts.get(key, 0), FIDELITY_COLOUR[key]) for key in order]
    )
    (HERE / "ontology" / "type-system-fidelity.svg").write_text(type_chart + "\n")

    instance_chart = bars(
        [
            (FIDELITY_LABEL[key], REPORT["propertiesByFidelity"].get(key, 0), FIDELITY_COLOUR[key])
            for key in order
        ]
    )
    (HERE / "ontology" / "property-fidelity.svg").write_text(instance_chart + "\n")

    audit_rows = []
    for entry in AUDIT["audits"]:
        by_verdict = entry["assertionsByVerdict"]
        for verdict in ("carried", "partial", "none"):
            audit_rows.append(
                (
                    f"{entry['ontology']} — {VERDICT_LABEL[verdict]}",
                    by_verdict.get(verdict, 0),
                    VERDICT_COLOUR[verdict],
                )
            )
    (HERE / "ontology" / "axiom-survival.svg").write_text(
        bars(audit_rows, width=560, label_width=330, row_height=26) + "\n"
    )

    total_types = TYPES["count"]
    lines = [
        "# Coverage report",
        "",
        "Every figure on this page is written by `generate_report.py` from the",
        "JSON that `foundry_owl.py` and `owl_to_foundry.py` produce. None is typed.",
        "",
        "## Sources",
        "",
        "| File | Repository | Licence | Bytes | SHA-256 (first 16) |",
        "| --- | --- | --- | ---: | --- |",
    ]
    for source in SOURCES["sources"]:
        lines.append(
            f"| `{source['file']}` | {source['repository']} | {source['licence']} | "
            f"{source['bytes']} | `{source['sha256'][:16]}` |"
        )

    lines += [
        "",
        "## The Foundry property type system",
        "",
        f"Palantir's `ObjectPropertyType` union declares {total_types} property types.",
        "Parsed from their own SDK, not transcribed.",
        "",
        "| Fidelity | Types | Meaning |",
        "| --- | ---: | --- |",
    ]
    for key in order:
        lines.append(f"| {key} | {counts.get(key, 0)} | {FIDELITY_LABEL[key]} |")

    lines += [
        "",
        "![Foundry property types by crossing fidelity](type-system-fidelity.svg)",
        "",
        "### Types with no counterpart in any standard",
        "",
        "| Foundry type | Why it does not cross |",
        "| --- | --- |",
    ]
    for name, mapping in foundry_owl.TYPE_MAP.items():
        if mapping.fidelity == "none":
            lines.append(f"| `{name}` | {mapping.note} |")

    lines += [
        "",
        "## The crossing, measured on Palantir's own fixture",
        "",
        "| Measure | Value |",
        "| --- | ---: |",
        f"| Object types | {REPORT['objectTypes']} |",
        f"| Properties | {REPORT['properties']} |",
        f"| Link sides | {REPORT['linkSides']} |",
        f"| Interfaces | {REPORT['interfaces']} |",
        f"| Shared property types | {REPORT['sharedPropertyTypes']} |",
        f"| Inverse pairs recovered | {REPORT['inversePairs']} |",
        f"| owl:hasKey axioms written | {REPORT['keyAxioms']} |",
        f"| Properties the crosswalk cannot carry | {len(REPORT['lossyProperties'])} |",
        "",
        "![Properties by crossing fidelity](property-fidelity.svg)",
        "",
        "### Documentation present in the source",
        "",
        "The crosswalk never invents a definition. What the source omits stays omitted,",
        "and the linter then reports it.",
        "",
        "| Measure | Value |",
        "| --- | ---: |",
        f"| Object types with no description | {REPORT['objectTypesWithoutDescription']} of {REPORT['objectTypes']} |",
        f"| Properties with no description | {REPORT['propertiesWithoutDescription']} of {REPORT['properties']} |",
        "",
        "## The other direction",
        "",
        "Each ontology below was parsed, its asserted constructs counted, and each",
        "construct checked against the fields that exist in Palantir's ontology model.",
        "",
        "| Ontology | Triples | Constructs used | Carried | Partial | No destination |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for entry in AUDIT["audits"]:
        by_verdict = entry["assertionsByVerdict"]
        lines.append(
            f"| `{entry['ontology']}` | {entry['triples']} | {entry['constructsUsed']} | "
            f"{by_verdict.get('carried', 0)} | {by_verdict.get('partial', 0)} | "
            f"{by_verdict.get('none', 0)} |"
        )

    lines += [
        "",
        "![Axiom survival by ontology](axiom-survival.svg)",
        "",
        "### Where each ontology loses its axioms",
        "",
    ]
    for entry in AUDIT["audits"]:
        stranded = [row for row in entry["rows"] if row["verdict"] == "none"]
        if not stranded:
            continue
        lines.append(f"**{entry['ontology']}**")
        lines.append("")
        lines.append("| Construct | Assertions | Why it has no destination |")
        lines.append("| --- | ---: | --- |")
        for row in stranded:
            lines.append(f"| `{row['construct']}` | {row['assertions']} | {row['note']} |")
        lines.append("")

    (HERE / "ontology" / "coverage-report.md").write_text("\n".join(lines) + "\n")
    print(f"coverage report written, {len(lines)} lines")


if __name__ == "__main__":
    main()
