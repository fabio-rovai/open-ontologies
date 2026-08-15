import pytest

from open_ontologies_lite import OntologyEngine

pytest.importorskip("pyshacl", reason="needs the [shacl] extra")

from open_ontologies_lite.shacl import shacl_validate  # noqa: E402

SHAPES = """
@prefix sh:   <http://www.w3.org/ns/shacl#> .
@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
@prefix ex:   <http://example.org/> .

ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:datatype xsd:string ;
                sh:message "A Person needs exactly one string name." ] .
"""

CONFORMING = """
@prefix ex: <http://example.org/> .
ex:ada a ex:Person ; ex:name "Ada" .
"""

VIOLATING = """
@prefix ex: <http://example.org/> .
ex:ada a ex:Person .
"""


def test_conforming_data_reports_no_violations():
    report = shacl_validate(CONFORMING, SHAPES)
    assert report["conforms"] is True
    assert report["count"] == 0
    assert report["violations"] == []


def test_violation_is_reported_with_focus_node_and_message():
    report = shacl_validate(VIOLATING, SHAPES)
    assert report["conforms"] is False
    assert report["count"] == 1
    v = report["violations"][0]
    assert v["focus_node"] == "http://example.org/ada"
    assert v["path"] == "http://example.org/name"
    assert "name" in v["message"]
    assert v["severity"] == "Violation"
    assert v["constraint"] == "MinCountConstraintComponent"
    assert report["by_severity"] == {"Violation": 1}


def test_validates_the_loaded_store_via_dump():
    """The path the MCP tool actually takes: store -> dump -> pySHACL."""
    eng = OntologyEngine()
    eng.load(VIOLATING)
    report = shacl_validate(eng.dump(), SHAPES)
    assert report["conforms"] is False
    assert report["violations"][0]["focus_node"] == "http://example.org/ada"


def test_dump_roundtrips_through_the_store():
    eng = OntologyEngine()
    eng.load(CONFORMING)
    dumped = eng.dump()
    again = OntologyEngine()
    assert again.load(dumped) == 2
