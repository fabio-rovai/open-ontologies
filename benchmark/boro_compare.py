#!/usr/bin/env python3
"""
BORO Comparison: Hand-crafted vs AI-generated building ontology.

Loads both BORO building ontologies and compares them on structural metrics,
showing how an AI-generated ontology achieves equivalent domain coverage
with fewer intermediate abstractions.

Requires: rdflib (pip install rdflib)
"""

import sys
import os

try:
    from rdflib import Graph, Namespace, RDF, RDFS, OWL, XSD
except ImportError:
    print("rdflib not found. Install it with: pip install rdflib")
    sys.exit(1)

# --- Namespaces ---
IES = Namespace("http://ies.data.gov.uk/ontology/ies4#")
BOROB = Namespace("http://example.org/ontology/boro-building-handcrafted#")
BOROA = Namespace("http://example.org/ontology/boro-building-ai#")


def load(path):
    """Load a Turtle file into an rdflib Graph."""
    g = Graph()
    g.parse(path, format="turtle")
    return g


def local_name(uri):
    """Extract the local name from a URI."""
    s = str(uri)
    return s.split("#")[-1] if "#" in s else s.split("/")[-1]


def classes_in_ns(g, ns):
    """Return a sorted list of local class names in the given namespace."""
    out = set()
    for s in g.subjects(RDF.type, RDFS.Class):
        if str(s).startswith(str(ns)):
            out.add(local_name(s))
    return sorted(out)


def properties_in_ns(g, ns):
    """Return sorted lists of (object_props, datatype_props) in the namespace."""
    obj = set()
    dat = set()
    for s in g.subjects(RDF.type, OWL.ObjectProperty):
        if str(s).startswith(str(ns)):
            obj.add(local_name(s))
    for s in g.subjects(RDF.type, OWL.DatatypeProperty):
        if str(s).startswith(str(ns)):
            dat.add(local_name(s))
    return sorted(obj), sorted(dat)


def individuals_in_ns(g, ns):
    """Return named individuals (not classes/properties/ontologies) in the namespace."""
    skip_types = {str(RDFS.Class), str(OWL.ObjectProperty),
                  str(OWL.DatatypeProperty), str(OWL.Ontology)}
    out = set()
    for s, _, o in g.triples((None, RDF.type, None)):
        if str(s).startswith(str(ns)) and str(o) not in skip_types:
            out.add(local_name(s))
    return sorted(out)


def entity_state_pairs(classes):
    """Find Entity+State pairs and unpaired entities."""
    entities = [c for c in classes if not c.endswith("State") and not c.startswith("ClassOf")]
    paired = []
    unpaired = []
    for e in entities:
        state = e + "State"
        if state in classes:
            paired.append(e)
        else:
            unpaired.append(e)
    return paired, unpaired


def classof_classes(classes):
    """Return ClassOf classes."""
    return [c for c in classes if c.startswith("ClassOf")]


def bounding_state_classes(classes):
    """Return BoundingState subclasses (names containing 'State' that are lifecycle markers)."""
    markers = []
    for c in classes:
        if c.endswith("State") and not c.startswith("ClassOf"):
            # Heuristic: lifecycle markers have specific names
            if any(kw in c for kw in ("Constructed", "Demolished", "Installed",
                                       "Removed", "Occupied", "Vacated")):
                markers.append(c)
    return markers


def intermediate_classes(classes):
    """Return classes that are pure intermediate abstractions (not Entity+State, not ClassOf, not BoundingState)."""
    entity_names = set()
    for c in classes:
        if not c.endswith("State") and not c.startswith("ClassOf"):
            entity_names.add(c)
    state_names = {c for c in classes if c.endswith("State") and not c.startswith("ClassOf")}
    classof_names = {c for c in classes if c.startswith("ClassOf")}

    all_structural = entity_names | state_names | classof_names
    return sorted(set(classes) - all_structural)


def count_triples_in_ns(g, ns):
    """Count triples where the subject is in the namespace."""
    count = 0
    for s, _, _ in g:
        if str(s).startswith(str(ns)):
            count += 1
    return count


def count_comments(g, ns):
    """Count rdfs:comment triples in the namespace."""
    count = 0
    for s in g.subjects(RDFS.comment, None):
        if str(s).startswith(str(ns)):
            count += 1
    return count


def count_labels(g, ns):
    """Count rdfs:label triples in the namespace."""
    count = 0
    for s in g.subjects(RDFS.label, None):
        if str(s).startswith(str(ns)):
            count += 1
    return count


def avg_comment_length(g, ns):
    """Average length of rdfs:comment values in the namespace."""
    lengths = []
    for s, _, o in g.triples((None, RDFS.comment, None)):
        if str(s).startswith(str(ns)):
            lengths.append(len(str(o)))
    return sum(lengths) / len(lengths) if lengths else 0


def format_table(headers, rows, align=None):
    """Format a markdown table."""
    if align is None:
        align = ["left"] * len(headers)

    col_widths = [len(h) for h in headers]
    for row in rows:
        for i, cell in enumerate(row):
            col_widths[i] = max(col_widths[i], len(str(cell)))

    def pad(val, width, a):
        s = str(val)
        if a == "right":
            return s.rjust(width)
        elif a == "center":
            return s.center(width)
        return s.ljust(width)

    sep = []
    for i, a in enumerate(align):
        if a == "right":
            sep.append("-" * (col_widths[i] - 1) + ":")
        elif a == "center":
            sep.append(":" + "-" * (col_widths[i] - 2) + ":")
        else:
            sep.append("-" * col_widths[i])

    lines = []
    lines.append("| " + " | ".join(pad(h, col_widths[i], align[i]) for i, h in enumerate(headers)) + " |")
    lines.append("| " + " | ".join(sep) + " |")
    for row in rows:
        lines.append("| " + " | ".join(pad(row[i], col_widths[i], align[i]) for i in range(len(headers))) + " |")
    return "\n".join(lines)


def run_comparison(handcrafted_path, ai_path):
    """Run the full comparison and return markdown output."""

    # Load
    g_hand = load(handcrafted_path)
    g_ai = load(ai_path)

    # --- Basic counts ---
    hand_classes = classes_in_ns(g_hand, BOROB)
    ai_classes = classes_in_ns(g_ai, BOROA)

    hand_obj, hand_dat = properties_in_ns(g_hand, BOROB)
    ai_obj, ai_dat = properties_in_ns(g_ai, BOROA)

    hand_indiv = individuals_in_ns(g_hand, BOROB)
    ai_indiv = individuals_in_ns(g_ai, BOROA)

    hand_triples = count_triples_in_ns(g_hand, BOROB)
    ai_triples = count_triples_in_ns(g_ai, BOROA)

    # --- Structural analysis ---
    hand_paired, hand_unpaired = entity_state_pairs(hand_classes)
    ai_paired, ai_unpaired = entity_state_pairs(ai_classes)

    hand_classof = classof_classes(hand_classes)
    ai_classof = classof_classes(ai_classes)

    hand_bounding = bounding_state_classes(hand_classes)
    ai_bounding = bounding_state_classes(ai_classes)

    hand_avg_comment = avg_comment_length(g_hand, BOROB)
    ai_avg_comment = avg_comment_length(g_ai, BOROA)

    # --- Build output ---
    out = []
    out.append("# BORO Ontology Comparison: Hand-crafted vs AI-generated\n")

    # Summary table
    out.append("## Structural Metrics\n")
    rows = [
        ("Total classes", str(len(hand_classes)), str(len(ai_classes)),
         f"{len(hand_classes) - len(ai_classes):+d}"),
        ("Entity+State pairs", str(len(hand_paired)), str(len(ai_paired)),
         f"{len(hand_paired) - len(ai_paired):+d}"),
        ("Unpaired entities (no State)", str(len(hand_unpaired)), str(len(ai_unpaired)),
         f"{len(hand_unpaired) - len(ai_unpaired):+d}"),
        ("ClassOf classes", str(len(hand_classof)), str(len(ai_classof)),
         f"{len(hand_classof) - len(ai_classof):+d}"),
        ("BoundingState subclasses", str(len(hand_bounding)), str(len(ai_bounding)),
         f"{len(hand_bounding) - len(ai_bounding):+d}"),
        ("Object properties", str(len(hand_obj)), str(len(ai_obj)),
         f"{len(hand_obj) - len(ai_obj):+d}"),
        ("Datatype properties", str(len(hand_dat)), str(len(ai_dat)),
         f"{len(hand_dat) - len(ai_dat):+d}"),
        ("Named individuals", str(len(hand_indiv)), str(len(ai_indiv)),
         f"{len(hand_indiv) - len(ai_indiv):+d}"),
        ("Triples (in namespace)", str(hand_triples), str(ai_triples),
         f"{hand_triples - ai_triples:+d}"),
        ("Avg comment length (chars)", str(int(hand_avg_comment)), str(int(ai_avg_comment)),
         f"{int(hand_avg_comment - ai_avg_comment):+d}"),
    ]
    out.append(format_table(
        ["Metric", "Hand-crafted", "AI-generated", "Delta"],
        rows,
        ["left", "right", "right", "right"]
    ))
    out.append("")

    # Classes only in hand-crafted
    ai_class_set = set(ai_classes)
    hand_only = [c for c in hand_classes if c not in ai_class_set]
    if hand_only:
        out.append("## Classes in Hand-crafted but NOT in AI-generated\n")
        out.append("These classes represent intermediate abstractions or speculative")
        out.append("hierarchies that the AI considered unnecessary:\n")
        for c in hand_only:
            out.append(f"- `{c}`")
        out.append("")

    # Classes only in AI
    hand_class_set = set(hand_classes)
    ai_only = [c for c in ai_classes if c not in hand_class_set]
    if ai_only:
        out.append("## Classes in AI-generated but NOT in Hand-crafted\n")
        for c in ai_only:
            out.append(f"- `{c}`")
        out.append("")

    # Properties only in hand-crafted
    ai_prop_set = set(ai_obj + ai_dat)
    hand_props = hand_obj + hand_dat
    hand_only_props = [p for p in hand_props if p not in ai_prop_set]
    if hand_only_props:
        out.append("## Properties in Hand-crafted but NOT in AI-generated\n")
        out.append("These properties serve the intermediate classes that the AI omitted:\n")
        for p in hand_only_props:
            out.append(f"- `{p}`")
        out.append("")

    # Entity+State pair comparison
    out.append("## Entity+State Pair Comparison\n")
    all_entities = sorted(set(hand_paired + hand_unpaired + ai_paired + ai_unpaired))
    pair_rows = []
    for e in all_entities:
        in_hand = "yes" if e in hand_paired else ("entity only" if e in hand_unpaired else "---")
        in_ai = "yes" if e in ai_paired else ("entity only" if e in ai_unpaired else "---")
        pair_rows.append((e, in_hand, in_ai))
    out.append(format_table(
        ["Entity", "Hand-crafted has State?", "AI has State?"],
        pair_rows,
        ["left", "center", "center"]
    ))
    out.append("")

    # ClassOf comparison
    out.append("## ClassOf Hierarchy Comparison\n")
    all_classof = sorted(set(hand_classof + ai_classof))
    if all_classof:
        classof_rows = []
        for c in all_classof:
            in_hand = "yes" if c in hand_classof else "---"
            in_ai = "yes" if c in ai_classof else "---"
            classof_rows.append((c, in_hand, in_ai))
        out.append(format_table(
            ["ClassOf Class", "Hand-crafted", "AI-generated"],
            classof_rows,
            ["left", "center", "center"]
        ))
    else:
        out.append("Neither ontology uses ClassOf classes (AI approach: use ies:similarEntity).")
    out.append("")

    # Reduction summary
    if len(hand_classes) > 0:
        reduction_pct = (1 - len(ai_classes) / len(hand_classes)) * 100
    else:
        reduction_pct = 0

    if hand_triples > 0:
        triple_reduction_pct = (1 - ai_triples / hand_triples) * 100
    else:
        triple_reduction_pct = 0

    out.append("## Summary\n")
    out.append(f"- **Class reduction**: {reduction_pct:.0f}% fewer classes ({len(hand_classes)} -> {len(ai_classes)})")
    out.append(f"- **Triple reduction**: {triple_reduction_pct:.0f}% fewer triples ({hand_triples} -> {ai_triples})")
    out.append(f"- **Comment verbosity**: hand-crafted averages {int(hand_avg_comment)} chars/comment vs AI's {int(ai_avg_comment)} chars/comment")
    out.append(f"- **ClassOf overhead**: hand-crafted has {len(hand_classof)} ClassOf classes; AI has {len(ai_classof)}")
    out.append(f"- **BoundingState overhead**: hand-crafted has {len(hand_bounding)} BoundingState subclasses; AI has {len(ai_bounding)}")
    out.append("")

    # Key differences narrative
    out.append("## Key Differences Explained\n")
    out.append("### 1. Intermediate Entity Classes")
    out.append("The hand-crafted ontology creates a `BuildingElement` superclass with")
    out.append("concrete subclasses (`Wall`, `Roof`, `Window`, `Door`), each with their")
    out.append("own `*State` class. The AI version omits these entirely -- individual")
    out.append("building components are modelled as part-of relationships from Building,")
    out.append("classified via `ies:similarEntity` when needed.\n")
    out.append("### 2. ClassOf Powertype Hierarchies")
    out.append("The hand-crafted ontology creates dedicated `ClassOfX` and `ClassOfXState`")
    out.append("for every entity type (Building, Room, Floor, Dwelling, BuildingElement).")
    out.append("The AI version uses the existing `ies:ClassOfEntity` directly for")
    out.append("classification instances, avoiding 10 extra classes.\n")
    out.append("### 3. BoundingState Proliferation")
    out.append("The hand-crafted version creates 6 BoundingState subclasses")
    out.append("(`ConstructedState`, `DemolishedState`, `OccupiedState`, `VacatedState`,")
    out.append("`InstalledState`, `RemovedState`). The AI version keeps only the 2")
    out.append("essential ones (`ConstructedState`, `DemolishedState`) and handles")
    out.append("occupancy via events.\n")
    out.append("### 4. Comment Verbosity")
    out.append("Hand-crafted comments explain BORO theory in each definition.")
    out.append("AI-generated comments state what the class *is* concisely.\n")
    out.append("### 5. Speculative Classification Classes")
    out.append("The hand-crafted version includes `OccupancyType`, `BuildingCondition`,")
    out.append("`ElementMaterial`, and `FloorLevelDesignation` as intermediate")
    out.append("classification hierarchies. The AI version omits these because they")
    out.append("can be added when actually needed, following YAGNI principles.")
    out.append("")

    out.append("## Why AI-native Ontology Engineering Matters\n")
    out.append("Traditional ontology engineering treats completeness as a virtue. Every")
    out.append("conceivable classification axis is modelled upfront, every entity gets")
    out.append("the full BORO treatment (Entity + State + ClassOf + ClassOfState), and")
    out.append("comments explain the methodology rather than the domain.\n")
    out.append("The result is an ontology that is *architecturally correct* but")
    out.append("operationally heavy. In this comparison:\n")
    out.append(f"- The hand-crafted version has **{len(hand_classes)} classes** to cover a domain")
    out.append(f"  that the AI covers with **{len(ai_classes)} classes** -- a {reduction_pct:.0f}% reduction.")
    out.append(f"- The hand-crafted version produces **{hand_triples} triples** vs the AI's **{ai_triples}**")
    out.append(f"  -- {triple_reduction_pct:.0f}% less data to store, query, and maintain.")
    out.append(f"- The AI's comments average **{int(ai_avg_comment)} characters** vs **{int(hand_avg_comment)}**")
    out.append("  -- they say what something *is* rather than why BORO requires it.\n")
    out.append("AI-native ontology engineering does not abandon rigour. Both ontologies")
    out.append("use the same IES4 upper ontology, the same 4D perdurantist patterns,")
    out.append("and the same Entity+State temporal modelling. The AI simply applies")
    out.append("these patterns where they deliver value, rather than everywhere they")
    out.append("*could* be applied.\n")
    out.append("The practical benefit: an AI-generated ontology is easier to understand,")
    out.append("faster to query, cheaper to maintain, and can be extended incrementally")
    out.append("when new requirements emerge -- rather than trying to anticipate all")
    out.append("possible future needs upfront.")

    return "\n".join(out)


def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))

    handcrafted_path = os.path.join(script_dir, "reference", "boro-building-handcrafted.ttl")
    ai_path = os.path.join(script_dir, "generated", "boro-building-ai.ttl")

    if len(sys.argv) >= 3:
        handcrafted_path = sys.argv[1]
        ai_path = sys.argv[2]

    if not os.path.exists(handcrafted_path):
        print(f"Error: hand-crafted ontology not found at {handcrafted_path}")
        sys.exit(1)
    if not os.path.exists(ai_path):
        print(f"Error: AI-generated ontology not found at {ai_path}")
        sys.exit(1)

    print(f"Loading hand-crafted: {handcrafted_path}")
    print(f"Loading AI-generated: {ai_path}")
    print()

    md = run_comparison(handcrafted_path, ai_path)
    print(md)

    # Also write to file
    out_path = os.path.join(script_dir, "BORO_COMPARISON.md")
    with open(out_path, "w") as f:
        f.write(md + "\n")
    print(f"\nResults written to {out_path}")


if __name__ == "__main__":
    main()
