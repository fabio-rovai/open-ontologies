"""Crosswalk a Foundry Ontology export into OWL 2 and SHACL, and report the loss.

Palantir's marketing states that Foundry supports "the ability to import/export
in multiple open formats (RDF XML/TTL, OWL, etc.) natively". The product
documentation describes a proprietary JSON export and warns that its schema may
change. This module builds the bridge that is missing and, more usefully,
measures precisely what does not survive the crossing.

Fidelity levels assigned to each Foundry property type:

  direct      an XSD datatype exists with the same value space
  standard    a published non-XSD standard carries it (OGC GeoSPARQL)
  structural  not a datatype at all; expressed as a shape, not a range
  none        no counterpart in any W3C or OGC standard
"""
from __future__ import annotations

import json
import pathlib
from dataclasses import dataclass, field

from rdflib import RDF, RDFS, XSD, BNode, Graph, Literal, Namespace, OWL, URIRef
from rdflib.collection import Collection

HERE = pathlib.Path(__file__).parent

EX = Namespace("https://ontology.tesseract.academy/foundry-crosswalk/")
FCX = Namespace("https://ontology.tesseract.academy/foundry-crosswalk/meta#")
SH = Namespace("http://www.w3.org/ns/shacl#")
SKOS = Namespace("http://www.w3.org/2004/02/skos/core#")
GEO = Namespace("http://www.opengis.net/ont/geosparql#")

DIRECT = "direct"
STANDARD = "standard"
STRUCTURAL = "structural"
NONE = "none"


@dataclass(frozen=True)
class TypeMapping:
    """How one Foundry property type crosses into the standards world."""

    fidelity: str
    target: URIRef | None
    note: str


# Keyed by the JSON discriminator Palantir assigns in its own SDK. The key set
# is checked against the derived type system in the tests, so a Foundry type
# added upstream fails the build rather than passing silently.
TYPE_MAP: dict[str, TypeMapping] = {
    "string": TypeMapping(DIRECT, XSD.string, "Unicode string."),
    "boolean": TypeMapping(DIRECT, XSD.boolean, "Two-valued boolean."),
    "byte": TypeMapping(DIRECT, XSD.byte, "8-bit signed integer."),
    "short": TypeMapping(DIRECT, XSD.short, "16-bit signed integer."),
    "integer": TypeMapping(DIRECT, XSD.int, "32-bit signed integer."),
    "long": TypeMapping(DIRECT, XSD.long, "64-bit signed integer."),
    "float": TypeMapping(DIRECT, XSD.float, "IEEE 754 single precision."),
    "double": TypeMapping(DIRECT, XSD.double, "IEEE 754 double precision."),
    "decimal": TypeMapping(DIRECT, XSD.decimal, "Arbitrary precision decimal."),
    "date": TypeMapping(DIRECT, XSD.date, "ISO 8601 local date."),
    "timestamp": TypeMapping(DIRECT, XSD.dateTime, "ISO 8601 instant."),
    "geopoint": TypeMapping(
        STANDARD,
        GEO.wktLiteral,
        "Carried by OGC GeoSPARQL, not by XSD. A consumer that reads only XSD "
        "sees an opaque literal.",
    ),
    "geoshape": TypeMapping(
        STANDARD,
        GEO.wktLiteral,
        "Carried by OGC GeoSPARQL, not by XSD.",
    ),
    "array": TypeMapping(
        STRUCTURAL,
        None,
        "Cardinality, not a datatype. Expressed by omitting sh:maxCount and "
        "ranging on the element type.",
    ),
    "struct": TypeMapping(
        STRUCTURAL,
        None,
        "Expressed as a generated class with its own node shape, so a Foundry "
        "struct becomes an object property in OWL.",
    ),
    "marking": TypeMapping(
        NONE,
        None,
        "A Foundry security marking. No W3C standard carries access control. "
        "Exported as an opaque literal, the classification is no longer "
        "enforceable and no longer distinguishable from ordinary data.",
    ),
    "cipherText": TypeMapping(
        NONE,
        None,
        "Ciphertext bound to a Foundry cipher channel. The channel reference is "
        "meaningless outside the platform.",
    ),
    "attachment": TypeMapping(
        NONE,
        None,
        "A resource identifier pointing into Foundry blob storage. The value "
        "does not resolve outside the platform.",
    ),
    "mediaReference": TypeMapping(
        NONE,
        None,
        "A reference into a Foundry media set. Does not resolve outside the "
        "platform.",
    ),
    "timeseries": TypeMapping(
        NONE,
        None,
        "A handle onto a Foundry time series, not a value. The series itself is "
        "not in the export.",
    ),
    "geotimeSeriesReference": TypeMapping(
        NONE,
        None,
        "A handle onto a Geotime integration. The track is not in the export.",
    ),
    "vector": TypeMapping(
        NONE,
        None,
        "An embedding. The export declares its dimension and sometimes the "
        "producing model, but no W3C datatype carries a vector, so both "
        "become annotations rather than a range.",
    ),
}


@dataclass
class CrosswalkReport:
    """Everything the crossing could not carry, counted rather than asserted."""

    object_types: int = 0
    properties: int = 0
    link_sides: int = 0
    interfaces: int = 0
    shared_property_types: int = 0
    inverse_pairs: int = 0
    keys: int = 0
    object_types_without_description: int = 0
    properties_without_description: int = 0
    by_fidelity: dict[str, int] = field(default_factory=dict)
    lossy_properties: list[dict] = field(default_factory=list)
    dropped_constructs: list[dict] = field(default_factory=list)

    def as_dict(self) -> dict:
        return {
            "objectTypes": self.object_types,
            "properties": self.properties,
            "linkSides": self.link_sides,
            "interfaces": self.interfaces,
            "sharedPropertyTypes": self.shared_property_types,
            "inversePairs": self.inverse_pairs,
            "keyAxioms": self.keys,
            "objectTypesWithoutDescription": self.object_types_without_description,
            "propertiesWithoutDescription": self.properties_without_description,
            "propertiesByFidelity": self.by_fidelity,
            "lossyProperties": self.lossy_properties,
            "droppedConstructs": self.dropped_constructs,
        }


def _safe(name: str) -> str:
    return "".join(character if character.isalnum() else "_" for character in name)


class Crosswalk:
    def __init__(self, metadata: dict):
        self.metadata = metadata
        self.graph = Graph()
        self.report = CrosswalkReport()
        for prefix, namespace in (
            ("ex", EX), ("fcx", FCX), ("sh", SH), ("skos", SKOS),
            ("geo", GEO), ("owl", OWL), ("xsd", XSD),
        ):
            self.graph.bind(prefix, namespace)

    # -- naming -------------------------------------------------------------
    def class_iri(self, api_name: str) -> URIRef:
        return EX[_safe(api_name)]

    def property_iri(self, object_type: str, property_name: str) -> URIRef:
        return EX[f"{_safe(object_type)}.{_safe(property_name)}"]

    def link_iri(self, object_type: str, link_name: str) -> URIRef:
        return EX[f"link.{_safe(object_type)}.{_safe(link_name)}"]

    def shape_iri(self, api_name: str) -> URIRef:
        return EX[f"{_safe(api_name)}Shape"]

    # -- emit ---------------------------------------------------------------
    def run(self) -> tuple[Graph, CrosswalkReport]:
        self._emit_meta()
        self._emit_interfaces()
        self._emit_shared_property_types()
        self._emit_object_types()
        self._emit_links()
        self._note_absent_constructs()
        return self.graph, self.report

    def _emit_meta(self) -> None:
        ontology = self.metadata.get("ontology", {})
        node = EX[""]
        self.graph.add((node, RDF.type, OWL.Ontology))
        if ontology.get("displayName"):
            self.graph.add((node, RDFS.label, Literal(ontology["displayName"])))
        if ontology.get("description"):
            self.graph.add((node, RDFS.comment, Literal(ontology["description"])))
        for term, comment in (
            (FCX.foundryRid, "The Foundry resource identifier the term came from."),
            (FCX.foundryType, "The Foundry property type discriminator."),
            (FCX.foundryElementType, "The element type of a Foundry array property."),
            (FCX.foundryDetail, "Foundry type detail with no standards counterpart."),
            (FCX.fidelity, "How faithfully the Foundry type crossed into standards."),
            (FCX.unmappable, "Marks a term whose Foundry meaning has no standards counterpart."),
            (FCX.titleProperty, "The Foundry display title property."),
        ):
            self.graph.add((term, RDF.type, OWL.AnnotationProperty))
            self.graph.add((term, RDFS.comment, Literal(comment)))

    def _emit_interfaces(self) -> None:
        for api_name, interface in self.metadata.get("interfaceTypes", {}).items():
            iri = self.class_iri(api_name)
            self.graph.add((iri, RDF.type, OWL.Class))
            self.graph.add((iri, RDFS.label, Literal(interface.get("displayName", api_name))))
            if interface.get("description"):
                self.graph.add((iri, RDFS.comment, Literal(interface["description"])))
            if interface.get("rid"):
                self.graph.add((iri, FCX.foundryRid, Literal(interface["rid"])))
            for parent in interface.get("extendsInterfaces", []):
                self.graph.add((iri, RDFS.subClassOf, self.class_iri(parent)))
            self.report.interfaces += 1

    def _emit_shared_property_types(self) -> None:
        for api_name, spt in self.metadata.get("sharedPropertyTypes", {}).items():
            iri = EX[f"spt.{_safe(api_name)}"]
            mapping = TYPE_MAP.get(spt.get("dataType", {}).get("type", ""))
            kind = OWL.DatatypeProperty
            if mapping and mapping.fidelity == STRUCTURAL:
                kind = OWL.ObjectProperty
            self.graph.add((iri, RDF.type, kind))
            self.graph.add((iri, RDFS.label, Literal(spt.get("displayName", api_name))))
            if spt.get("description"):
                self.graph.add((iri, RDFS.comment, Literal(spt["description"])))
            if mapping and mapping.target is not None:
                self.graph.add((iri, RDFS.range, mapping.target))
            self.report.shared_property_types += 1

    def _emit_object_types(self) -> None:
        for api_name, entry in self.metadata.get("objectTypes", {}).items():
            object_type = entry["objectType"]
            iri = self.class_iri(api_name)
            shape = self.shape_iri(api_name)
            self.graph.add((iri, RDF.type, OWL.Class))
            self.graph.add((iri, RDFS.label, Literal(object_type.get("displayName", api_name))))
            if object_type.get("description"):
                self.graph.add((iri, RDFS.comment, Literal(object_type["description"])))
            if object_type.get("pluralDisplayName"):
                self.graph.add((iri, SKOS.altLabel, Literal(object_type["pluralDisplayName"])))
            for alias in object_type.get("aliases") or []:
                self.graph.add((iri, SKOS.altLabel, Literal(alias)))
            if object_type.get("rid"):
                self.graph.add((iri, FCX.foundryRid, Literal(object_type["rid"])))
            if object_type.get("titleProperty"):
                self.graph.add((iri, FCX.titleProperty, Literal(object_type["titleProperty"])))
            for interface in entry.get("implementsInterfaces", []):
                self.graph.add((iri, RDFS.subClassOf, self.class_iri(interface)))

            self.graph.add((shape, RDF.type, SH.NodeShape))
            self.graph.add((shape, SH.targetClass, iri))

            primary_key = object_type.get("primaryKey")
            for property_name, prop in object_type.get("properties", {}).items():
                self._emit_property(api_name, iri, shape, property_name, prop, primary_key)
                self.report.properties += 1

            if primary_key and primary_key in object_type.get("properties", {}):
                key_list = BNode()
                Collection(self.graph, key_list, [self.property_iri(api_name, primary_key)])
                self.graph.add((iri, OWL.hasKey, key_list))
                self.report.keys += 1

            if not object_type.get("description"):
                self.report.object_types_without_description += 1
            self.report.object_types += 1

    def _emit_property(
        self,
        object_type_name: str,
        class_iri: URIRef,
        shape: URIRef,
        property_name: str,
        prop: dict,
        primary_key: str | None,
    ) -> None:
        data_type = prop.get("dataType", {})
        discriminator = data_type.get("type", "")
        is_array = discriminator == "array"

        # An array is cardinality, not a type. The element carries the meaning.
        element = (data_type.get("subType") or {}) if is_array else data_type
        element_type = element.get("type", "")
        mapping = TYPE_MAP.get(element_type)
        declared = TYPE_MAP.get(discriminator)

        iri = self.property_iri(object_type_name, property_name)
        is_object_valued = element_type == "struct"

        self.graph.add(
            (iri, RDF.type, OWL.ObjectProperty if is_object_valued else OWL.DatatypeProperty)
        )
        self.graph.add((iri, RDFS.label, Literal(prop.get("displayName") or property_name)))
        if prop.get("description"):
            self.graph.add((iri, RDFS.comment, Literal(prop["description"])))
        self.graph.add((iri, RDFS.domain, class_iri))
        self.graph.add((iri, FCX.foundryType, Literal(discriminator)))
        if not prop.get("description"):
            self.report.properties_without_description += 1
        if is_array and element_type:
            self.graph.add((iri, FCX.foundryElementType, Literal(element_type)))

        fidelity = mapping.fidelity if mapping else NONE
        self.graph.add((iri, FCX.fidelity, Literal(fidelity)))
        self.report.by_fidelity[fidelity] = self.report.by_fidelity.get(fidelity, 0) + 1

        property_shape = BNode()
        self.graph.add((shape, SH.property, property_shape))
        self.graph.add((property_shape, SH.path, iri))
        self.graph.add((property_shape, SH.name, Literal(property_name)))

        if is_object_valued:
            struct_class = self._emit_struct_class(object_type_name, property_name, element)
            self.graph.add((iri, RDFS.range, struct_class))
            self.graph.add((property_shape, SH["class"], struct_class))
        elif mapping is not None and mapping.target is not None:
            self.graph.add((iri, RDFS.range, mapping.target))
            self.graph.add((property_shape, SH.datatype, mapping.target))
        else:
            self.graph.add((iri, FCX.unmappable, Literal(True)))
            note = (mapping or declared).note if (mapping or declared) else (
                f"Unknown Foundry property type '{element_type or discriminator}'."
            )
            self.graph.add((iri, RDFS.comment, Literal(f"Not carried by the crosswalk. {note}")))
            self.report.lossy_properties.append(
                {
                    "objectType": object_type_name,
                    "property": property_name,
                    "foundryType": discriminator,
                    "elementType": element_type if is_array else None,
                    "reason": note,
                }
            )
            # Keep whatever the export declared, as annotation rather than range.
            for key in ("dimension", "itemType", "embeddingModel"):
                if key in element:
                    value = element[key]
                    self.graph.add(
                        (
                            iri,
                            FCX.foundryDetail,
                            Literal(f"{key}={json.dumps(value, sort_keys=True)}"),
                        )
                    )

        if not is_array:
            self.graph.add((property_shape, SH.maxCount, Literal(1)))
        if property_name == primary_key:
            self.graph.add((property_shape, SH.minCount, Literal(1)))

    def _emit_struct_class(
        self, object_type_name: str, property_name: str, element: dict
    ) -> URIRef:
        struct_class = EX[f"{_safe(object_type_name)}.{_safe(property_name)}.Struct"]
        self.graph.add((struct_class, RDF.type, OWL.Class))
        self.graph.add((struct_class, RDFS.label, Literal(f"{property_name} struct")))
        self.graph.add(
            (
                struct_class,
                RDFS.comment,
                Literal(
                    "Generated by the crosswalk. A Foundry struct is an inline "
                    f"record on {object_type_name}.{property_name}; OWL has no "
                    "inline record, so it becomes a class with its own shape."
                ),
            )
        )
        struct_shape = EX[f"{_safe(object_type_name)}.{_safe(property_name)}.StructShape"]
        self.graph.add((struct_shape, RDF.type, SH.NodeShape))
        self.graph.add((struct_shape, SH.targetClass, struct_class))
        for field_definition in element.get("structFieldTypes") or []:
            field_name = field_definition.get("apiName")
            if not field_name:
                continue
            field_type = (field_definition.get("dataType") or {}).get("type", "")
            field_mapping = TYPE_MAP.get(field_type)
            field_iri = EX[
                f"{_safe(object_type_name)}.{_safe(property_name)}.{_safe(field_name)}"
            ]
            self.graph.add((field_iri, RDF.type, OWL.DatatypeProperty))
            self.graph.add((field_iri, RDFS.label, Literal(field_name)))
            self.graph.add((field_iri, RDFS.domain, struct_class))
            self.graph.add((field_iri, FCX.foundryType, Literal(field_type)))
            field_shape = BNode()
            self.graph.add((struct_shape, SH.property, field_shape))
            self.graph.add((field_shape, SH.path, field_iri))
            self.graph.add((field_shape, SH.maxCount, Literal(1)))
            if field_mapping is not None and field_mapping.target is not None:
                self.graph.add((field_iri, RDFS.range, field_mapping.target))
                self.graph.add((field_shape, SH.datatype, field_mapping.target))
        return struct_class

    def _emit_links(self) -> None:
        seen_rids: dict[str, URIRef] = {}
        for api_name, entry in self.metadata.get("objectTypes", {}).items():
            source = self.class_iri(api_name)
            shape = self.shape_iri(api_name)
            for side in entry.get("linkTypes", []):
                link = self.link_iri(api_name, side["apiName"])
                target = self.class_iri(side["objectTypeApiName"])
                self.graph.add((link, RDF.type, OWL.ObjectProperty))
                self.graph.add((link, RDFS.label, Literal(side.get("displayName", side["apiName"]))))
                self.graph.add((link, RDFS.domain, source))
                self.graph.add((link, RDFS.range, target))
                self.graph.add((link, FCX.foundryRid, Literal(side["linkTypeRid"])))

                property_shape = BNode()
                self.graph.add((shape, SH.property, property_shape))
                self.graph.add((property_shape, SH.path, link))
                self.graph.add((property_shape, SH.name, Literal(side["apiName"])))
                self.graph.add((property_shape, SH["class"], target))
                if side["cardinality"] == "ONE":
                    self.graph.add((property_shape, SH.maxCount, Literal(1)))
                    self.graph.add((link, RDF.type, OWL.FunctionalProperty))

                rid = side["linkTypeRid"]
                if rid in seen_rids and seen_rids[rid] != link:
                    # Declare both directions. A reader who consults only the
                    # second property should still learn about the first.
                    self.graph.add((link, OWL.inverseOf, seen_rids[rid]))
                    self.graph.add((seen_rids[rid], OWL.inverseOf, link))
                    self.report.inverse_pairs += 1
                else:
                    seen_rids[rid] = link
                self.report.link_sides += 1

    def _note_absent_constructs(self) -> None:
        """Record Foundry constructs the export carries that OWL cannot express."""
        action_types = self.metadata.get("actionTypes", {})
        query_types = self.metadata.get("queryTypes", {})
        value_types = self.metadata.get("valueTypes", {})
        for label, count, note in (
            (
                "actionType",
                len(action_types),
                "A Foundry action is a permissioned state change with parameters, "
                "validation and side effects. OWL describes what holds, not what "
                "may be done, so no axiom carries it.",
            ),
            (
                "queryType",
                len(query_types),
                "A Foundry query is a named function. SPARQL can express a query "
                "but the ontology has no place to declare one.",
            ),
            (
                "valueType",
                len(value_types),
                "A Foundry value type is a reusable semantic constraint. The "
                "nearest OWL construct is a datatype restriction, which cannot "
                "carry the Foundry formatting and validation metadata.",
            ),
        ):
            self.report.dropped_constructs.append(
                {"construct": label, "countInExport": count, "reason": note}
            )


def crosswalk_file(path: pathlib.Path) -> tuple[Graph, CrosswalkReport]:
    metadata = json.loads(path.read_text())
    return Crosswalk(metadata).run()


def main() -> None:
    graph, report = crosswalk_file(HERE / "data" / "foundry-ontology.json")
    (HERE / "ontology").mkdir(exist_ok=True)
    graph.serialize(destination=HERE / "ontology" / "foundry-crosswalk.ttl", format="turtle")
    (HERE / "data" / "crosswalk-report.json").write_text(
        json.dumps(report.as_dict(), indent=2) + "\n"
    )
    print(json.dumps(report.as_dict(), indent=2)[:1200])
    print(f"\ntriples: {len(graph)}")


if __name__ == "__main__":
    main()
