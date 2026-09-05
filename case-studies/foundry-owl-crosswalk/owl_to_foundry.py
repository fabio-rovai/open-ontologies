"""Audit the other direction: which OWL axioms have a destination in Foundry?

The Foundry to OWL direction loses behaviour and security. The OWL to Foundry
direction loses something else, and the loss is easy to miss because it is
invisible in the resulting object graph: the axioms that let a reasoner detect
a contradiction have nowhere to go.

This module takes a real ontology, counts the axioms it actually asserts, and
checks each construct against the destination fields that exist in Palantir's
own published ontology model. The verdict per construct is one of:

  carried     a Foundry field holds the same meaning
  partial     a Foundry field holds a weaker version of it
  none        no field in the Foundry ontology model can hold it
"""
from __future__ import annotations

import json
import pathlib
import sys

from rdflib import OWL, RDF, RDFS, Graph, URIRef

HERE = pathlib.Path(__file__).parent
SHACL = URIRef("http://www.w3.org/ns/shacl#")

CARRIED = "carried"
PARTIAL = "partial"
NONE = "none"

# Destination is quoted from the field names in
# palantir/foundry-platform-python foundry_sdk/v2/ontologies/models.py.
DESTINATIONS: dict[URIRef, tuple[str, str, str]] = {
    OWL.Class: (CARRIED, "ObjectTypeV2 / InterfaceType", "A class becomes an object type or an interface."),
    OWL.DatatypeProperty: (PARTIAL, "PropertyV2.dataType", "Carried only when the range maps to a Foundry property type."),
    OWL.ObjectProperty: (CARRIED, "LinkTypeSideV2", "A link between two object types."),
    OWL.AnnotationProperty: (PARTIAL, "PropertyV2.description / typeClasses", "Free text survives; a typed annotation does not."),
    RDFS.domain: (CARRIED, "ObjectTypeV2.properties", "A property belongs to the object type that declares it."),
    RDFS.range: (PARTIAL, "PropertyV2.dataType / LinkTypeSideV2.objectTypeApiName", "Carried for the types in the Foundry union only."),
    RDFS.label: (CARRIED, "displayName", ""),
    RDFS.comment: (CARRIED, "description", ""),
    OWL.inverseOf: (CARRIED, "LinkTypeSideV2.linkTypeRid", "Two link sides sharing a rid are inverse."),
    OWL.FunctionalProperty: (PARTIAL, "LinkTypeSideV2.cardinality = ONE", "Carried for links; a functional datatype property has no field."),
    OWL.hasKey: (PARTIAL, "ObjectTypeV2.primaryKey", "primaryKey is one property, so a composite key has no destination."),
    RDFS.subClassOf: (PARTIAL, "InterfaceType.extendsInterfaces / implementsInterfaces", "Interfaces may extend interfaces. One object type may not subclass another."),
    OWL.equivalentClass: (NONE, "", "No field asserts that two types have the same members."),
    OWL.disjointWith: (NONE, "", "No field asserts that two types cannot share a member."),
    OWL.AllDisjointClasses: (NONE, "", "No field asserts mutual disjointness."),
    OWL.Restriction: (NONE, "", "No field carries a class defined by a condition on a property."),
    OWL.someValuesFrom: (NONE, "", "Existential restriction has no Foundry field."),
    OWL.allValuesFrom: (NONE, "", "Universal restriction has no Foundry field."),
    OWL.hasValue: (NONE, "", "Value restriction has no Foundry field."),
    OWL.cardinality: (NONE, "", "Exact cardinality on a property has no Foundry field."),
    OWL.minCardinality: (NONE, "", "Minimum cardinality has no Foundry field."),
    OWL.maxCardinality: (NONE, "", "Maximum cardinality on a property has no Foundry field."),
    OWL.minQualifiedCardinality: (NONE, "", "Qualified cardinality has no Foundry field."),
    OWL.maxQualifiedCardinality: (NONE, "", "Qualified cardinality has no Foundry field."),
    OWL.qualifiedCardinality: (NONE, "", "Qualified cardinality has no Foundry field."),
    OWL.TransitiveProperty: (NONE, "", "No field declares a property transitive."),
    OWL.SymmetricProperty: (NONE, "", "No field declares a property symmetric."),
    OWL.AsymmetricProperty: (NONE, "", "No field declares a property asymmetric."),
    OWL.ReflexiveProperty: (NONE, "", "No field declares a property reflexive."),
    OWL.IrreflexiveProperty: (NONE, "", "No field declares a property irreflexive."),
    OWL.InverseFunctionalProperty: (NONE, "", "No field declares a property inverse functional."),
    OWL.propertyChainAxiom: (NONE, "", "No field composes properties."),
    RDFS.subPropertyOf: (NONE, "", "No field arranges properties in a hierarchy."),
    OWL.unionOf: (NONE, "", "No field defines a class as a union."),
    OWL.intersectionOf: (NONE, "", "No field defines a class as an intersection."),
    OWL.complementOf: (NONE, "", "No field defines a class as a complement."),
    OWL.oneOf: (NONE, "", "No field defines a class by enumeration."),
    OWL.disjointUnionOf: (NONE, "", "No field defines a disjoint union."),
    OWL.sameAs: (NONE, "", "No field asserts two individuals are the same."),
    OWL.differentFrom: (NONE, "", "No field asserts two individuals differ."),
    OWL.AllDifferent: (NONE, "", "No field asserts mutual difference."),
    OWL.NegativePropertyAssertion: (NONE, "", "No field asserts that a statement is false."),
    OWL.withRestrictions: (NONE, "", "No field carries a datatype restriction."),
    OWL.onDatatype: (NONE, "", "No field carries a derived datatype."),
}


def count_constructs(graph: Graph) -> dict[URIRef, int]:
    """Count how often each construct is actually asserted in the ontology."""
    counts: dict[URIRef, int] = {}
    for construct in DESTINATIONS:
        # A construct is used either as a predicate or as the object of rdf:type.
        as_predicate = sum(1 for _ in graph.triples((None, construct, None)))
        as_type = sum(1 for _ in graph.triples((None, RDF.type, construct)))
        total = as_predicate + as_type
        if total:
            counts[construct] = total
    return counts


def audit(graph: Graph, name: str) -> dict:
    counts = count_constructs(graph)
    rows = []
    for construct, count in sorted(counts.items(), key=lambda item: -item[1]):
        verdict, destination, note = DESTINATIONS[construct]
        rows.append(
            {
                "construct": graph.namespace_manager.normalizeUri(construct),
                "assertions": count,
                "verdict": verdict,
                "foundryDestination": destination,
                "note": note,
            }
        )
    totals: dict[str, int] = {}
    axioms: dict[str, int] = {}
    for row in rows:
        totals[row["verdict"]] = totals.get(row["verdict"], 0) + 1
        axioms[row["verdict"]] = axioms.get(row["verdict"], 0) + row["assertions"]
    return {
        "ontology": name,
        "triples": len(graph),
        "constructsUsed": len(rows),
        "constructsByVerdict": totals,
        "assertionsByVerdict": axioms,
        "rows": rows,
    }


def main() -> None:
    targets = sys.argv[1:] or [
        str(HERE.parents[1] / "benchmark" / "reference" / "ies4.ttl"),
        str(HERE.parents[1] / "benchmark" / "reference" / "pizza-reference.owl"),
    ]
    results = []
    for target in targets:
        path = pathlib.Path(target)
        if not path.exists():
            print(f"skipped, not found: {path}")
            continue
        graph = Graph()
        graph.parse(path)
        result = audit(graph, path.name)
        results.append(result)
        by_verdict = result["assertionsByVerdict"]
        print(
            f"{path.name}: {result['triples']} triples, "
            f"{result['constructsUsed']} constructs used, "
            f"axioms carried {by_verdict.get(CARRIED, 0)}, "
            f"partial {by_verdict.get(PARTIAL, 0)}, "
            f"no destination {by_verdict.get(NONE, 0)}"
        )
    (HERE / "data" / "owl-to-foundry-audit.json").write_text(
        json.dumps({"audits": results}, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
