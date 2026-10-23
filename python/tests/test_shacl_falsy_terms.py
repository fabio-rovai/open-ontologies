"""Report RDF literals by presence rather than Python truthiness."""
import pytest
pytest.importorskip("pyshacl")
from open_ontologies_lite.shacl import shacl_validate

PREFIX = "@prefix ex:<http://example.org/>. @prefix sh:<http://www.w3.org/ns/shacl#>. @prefix xsd:<http://www.w3.org/2001/XMLSchema#>. "

@pytest.mark.parametrize("value, expected", [("0", "0"), ("false", "false"), ("1", "1"), ("true", "true")])
def test_violation_retains_literal_value(value, expected):
    report = shacl_validate(PREFIX + f"ex:n ex:p {value} .", PREFIX + "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:property [sh:path ex:p; sh:datatype xsd:string].")
    assert report["conforms"] is False
    assert len(report["violations"]) == 1
    violation = report["violations"][0]
    assert violation["value"] == expected
    assert violation["focus_node"] == "http://example.org/n"
    assert violation["path"] == "http://example.org/p"

@pytest.mark.parametrize("value, expected", [("0", "0"), ("false", "false"), ("1", "1"), ("true", "true")])
def test_literal_focus_node_retains_lexical_value(value, expected):
    report = shacl_validate("", PREFIX + f"ex:S a sh:NodeShape; sh:targetNode {value}; sh:datatype xsd:string.")
    assert report["conforms"] is False
    assert len(report["violations"]) == 1
    assert report["violations"][0]["focus_node"] == expected
    assert report["violations"][0]["value"] == expected

def test_absent_violation_value_stays_empty():
    report = shacl_validate("", PREFIX + "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:property [sh:path ex:p; sh:minCount 1].")
    assert report["conforms"] is False
    assert report["violations"][0]["value"] == ""

def test_valid_string_produces_no_violation():
    report = shacl_validate(PREFIX + 'ex:n ex:p "hello".', PREFIX + "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:property [sh:path ex:p; sh:datatype xsd:string].")
    assert report["conforms"] is True and report["violations"] == []
