"""Tests for the Foundry to OWL crosswalk.

Every number quoted in the case study is produced by this file or by the
report the crosswalk writes. Nothing is typed twice.
"""
from __future__ import annotations

import json
import pathlib
import subprocess
import sys

import pytest
from rdflib import RDF, RDFS, XSD, Graph, Literal, OWL, URIRef

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from foundry_owl import (  # noqa: E402
    DIRECT,
    EX,
    FCX,
    NONE,
    SH,
    STANDARD,
    STRUCTURAL,
    TYPE_MAP,
    Crosswalk,
    crosswalk_file,
)

HERE = pathlib.Path(__file__).parent
FIXTURE = HERE / "data" / "foundry-ontology.json"
TYPE_SYSTEM = HERE / "data" / "foundry-type-system.json"


@pytest.fixture(scope="module")
def crosswalked():
    return crosswalk_file(FIXTURE)


@pytest.fixture(scope="module")
def graph(crosswalked):
    return crosswalked[0]


@pytest.fixture(scope="module")
def report(crosswalked):
    return crosswalked[1]


# -- the gate on Palantir's own type system --------------------------------

def test_type_map_covers_palantir_type_system_exactly():
    """If Palantir adds or removes a property type, this fails rather than
    silently dropping the new type into the unmappable bucket."""
    declared = {
        entry["discriminator"]
        for entry in json.loads(TYPE_SYSTEM.read_text())["types"]
    }
    assert declared == set(TYPE_MAP), (
        f"in Palantir but not mapped: {sorted(declared - set(TYPE_MAP))}; "
        f"mapped but not in Palantir: {sorted(set(TYPE_MAP) - declared)}"
    )


def test_type_system_size_is_twenty_two():
    assert json.loads(TYPE_SYSTEM.read_text())["count"] == 22


def test_fidelity_split_is_eleven_two_two_seven():
    """The headline claim about the type system, asserted against the map."""
    counts: dict[str, int] = {}
    for mapping in TYPE_MAP.values():
        counts[mapping.fidelity] = counts.get(mapping.fidelity, 0) + 1
    assert counts == {DIRECT: 11, STANDARD: 2, STRUCTURAL: 2, NONE: 7}


def test_every_direct_type_targets_xsd():
    for name, mapping in TYPE_MAP.items():
        if mapping.fidelity == DIRECT:
            assert mapping.target is not None, name
            assert str(mapping.target).startswith(str(XSD)), name


def test_no_none_fidelity_type_has_a_target():
    for name, mapping in TYPE_MAP.items():
        if mapping.fidelity == NONE:
            assert mapping.target is None, name


# -- the output graph ------------------------------------------------------

def test_graph_parses_and_is_non_trivial(graph):
    assert len(graph) > 500


def test_every_object_type_became_a_class(graph, report):
    classes = set(graph.subjects(RDF.type, OWL.Class))
    metadata = json.loads(FIXTURE.read_text())
    for api_name in metadata["objectTypes"]:
        assert EX[api_name] in classes, api_name
    assert report.object_types == len(metadata["objectTypes"])


def test_primary_key_became_an_owl_key(graph, report):
    metadata = json.loads(FIXTURE.read_text())
    assert report.keys == len(metadata["objectTypes"])
    for api_name in metadata["objectTypes"]:
        assert (EX[api_name], OWL.hasKey, None) in graph, api_name


def test_cardinality_one_link_is_functional_and_capped(graph):
    """Employee.lead has cardinality ONE in Palantir's fixture."""
    lead = EX["link.Employee.lead"]
    assert (lead, RDF.type, OWL.FunctionalProperty) in graph
    shapes = [
        shape
        for shape in graph.subjects(SH.path, lead)
    ]
    assert shapes, "no property shape for the lead link"
    assert any((shape, SH.maxCount, Literal(1)) in graph for shape in shapes)


def test_cardinality_many_link_is_not_capped(graph):
    peeps = EX["link.Employee.peeps"]
    assert (peeps, RDF.type, OWL.FunctionalProperty) not in graph
    for shape in graph.subjects(SH.path, peeps):
        assert (shape, SH.maxCount, Literal(1)) not in graph


def test_link_sides_sharing_a_rid_became_inverses(graph, report):
    assert report.inverse_pairs == 2
    assert (EX["link.Office.occupants"], OWL.inverseOf, EX["link.Employee.officeLink"]) in graph


def test_interface_extension_became_subclass(graph):
    metadata = json.loads(FIXTURE.read_text())
    extending = [
        (name, interface["extendsInterfaces"])
        for name, interface in metadata["interfaceTypes"].items()
        if interface.get("extendsInterfaces")
    ]
    for name, parents in extending:
        for parent in parents:
            assert (EX[name], RDFS.subClassOf, EX[parent]) in graph


def test_array_property_has_no_max_count(graph):
    array_property = EX["objectTypeWithAllPropertyTypes.stringArray"]
    assert (array_property, RDFS.range, XSD.string) in graph
    for shape in graph.subjects(SH.path, array_property):
        assert (shape, SH.maxCount, Literal(1)) not in graph


def test_struct_property_became_an_object_property_with_a_class(graph):
    struct_property = EX["Employee.employeeProfile"]
    assert (struct_property, RDF.type, OWL.ObjectProperty) in graph
    struct_class = EX["Employee.employeeProfile.Struct"]
    assert (struct_property, RDFS.range, struct_class) in graph
    assert (struct_class, RDF.type, OWL.Class) in graph
    assert (EX["Employee.employeeProfile.yearsExperience"], RDFS.range, XSD.int) in graph


# -- the loss, which is the point ------------------------------------------

def test_unmappable_properties_are_marked_not_silently_typed(graph, report):
    """A property the crosswalk cannot carry must never acquire a range."""
    unmappable = set(graph.subjects(FCX.unmappable, Literal(True)))
    assert unmappable, "expected some properties to be unmappable"
    for subject in unmappable:
        assert (subject, RDFS.range, None) not in graph, subject
    assert len(unmappable) == len(report.lossy_properties)


def test_marking_property_is_reported_as_a_security_loss(report):
    """Palantir's exhaustive fixture does not include a marking property, so
    the mapping is asserted directly: a security marking must never be given a
    datatype range, because that would present a classification as data."""
    assert TYPE_MAP["marking"].fidelity == NONE
    assert TYPE_MAP["marking"].target is None
    assert "access control" in TYPE_MAP["marking"].note


def test_a_marking_property_survives_as_a_flagged_loss():
    """Inject a marking property and confirm it lands in the loss report."""
    metadata = json.loads(FIXTURE.read_text())
    employee = metadata["objectTypes"]["Employee"]["objectType"]
    employee["properties"]["clearance"] = {
        "dataType": {"type": "marking"},
        "rid": "ri.property.marking",
        "typeClasses": [],
    }
    _, injected = Crosswalk(metadata).run()
    flagged = [
        entry for entry in injected.lossyProperties_or_empty()
        if entry["property"] == "clearance"
    ] if hasattr(injected, "lossyProperties_or_empty") else [
        entry for entry in injected.lossy_properties if entry["property"] == "clearance"
    ]
    assert len(flagged) == 1
    assert flagged[0]["foundryType"] == "marking"


def test_behavioural_constructs_are_recorded_as_absent(report):
    constructs = {entry["construct"] for entry in report.dropped_constructs}
    assert {"actionType", "queryType", "valueType"} <= constructs


def test_report_totals_match_the_fixture(report):
    metadata = json.loads(FIXTURE.read_text())
    properties = sum(
        len(entry["objectType"]["properties"]) for entry in metadata["objectTypes"].values()
    )
    assert report.properties == properties
    assert sum(report.by_fidelity.values()) == properties


# -- proof that the gate bites ---------------------------------------------

def test_tampering_with_the_type_map_fails_the_gate(tmp_path):
    """Remove one type from the map and the coverage gate must fail. This is
    the evidence that the gate is load-bearing rather than decorative."""
    source = (HERE / "foundry_owl.py").read_text()
    tampered = source.replace(
        '    "vector": TypeMapping(', '    "_removed_vector": TypeMapping(', 1
    )
    assert tampered != source
    module_path = tmp_path / "foundry_owl.py"
    module_path.write_text(tampered)
    (tmp_path / "data").mkdir()
    (tmp_path / "data" / "foundry-type-system.json").write_text(TYPE_SYSTEM.read_text())
    probe = tmp_path / "probe.py"
    probe.write_text(
        "import json, sys, pathlib\n"
        "sys.path.insert(0, str(pathlib.Path(__file__).parent))\n"
        "from foundry_owl import TYPE_MAP\n"
        "declared = {e['discriminator'] for e in json.loads("
        "(pathlib.Path(__file__).parent / 'data' / 'foundry-type-system.json').read_text())['types']}\n"
        "sys.exit(0 if declared == set(TYPE_MAP) else 1)\n"
    )
    result = subprocess.run([sys.executable, str(probe)], capture_output=True)
    assert result.returncode == 1, "the coverage gate passed on a tampered type map"
