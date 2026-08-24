"""Vacuous conformance: shapes that selected no focus nodes.

pySHACL answers `conforms: True` for a shapes graph whose targets match nothing,
with no indication that nothing was examined. That report is byte-identical to
one where every constraint ran and passed, so a pipeline gating on `conforms`
publishes unvalidated data on a green light.

The Rust engine reports `focus_nodes` and `unmatched_shapes` and withholds the
verdict when nothing matched. These tests hold the Python package to the same
contract, so the two surfaces cannot disagree about what a validation run means.
"""

import pytest

pytest.importorskip("pyshacl", reason="needs the [shacl] extra")

from open_ontologies_lite.shacl import shacl_validate  # noqa: E402

# Shapes in one namespace, data in another: the mismatch that makes generated
# shapes target nothing while every report still reads clean.
SHAPES_OTHER_NAMESPACE = """
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/shapes/> .
ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [ sh:path ex:name ; sh:minCount 1 ] .
"""

SHAPES_SAME_NAMESPACE = """
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [ sh:path ex:name ; sh:minCount 1 ] .
"""

DATA = """
@prefix ex: <http://example.org/> .
ex:ada a ex:Person ; ex:name "Ada" .
"""

DATA_MISSING_NAME = """
@prefix ex: <http://example.org/> .
ex:ada a ex:Person .
"""


def test_shapes_that_select_nothing_are_named():
    report = shacl_validate(DATA, SHAPES_OTHER_NAMESPACE)

    assert report["focus_nodes"] == 0
    assert [u["target_class"] for u in report["unmatched_shapes"]] == [
        "http://example.org/shapes/Person"
    ]


def test_a_run_that_selected_nothing_withholds_the_verdict():
    report = shacl_validate(DATA_MISSING_NAME, SHAPES_OTHER_NAMESPACE)

    assert report["conforms"] is None, "nothing was checked, so there is no verdict"
    assert "focus node" in report["warning"]


def test_an_ordinary_passing_run_still_conforms():
    report = shacl_validate(DATA, SHAPES_SAME_NAMESPACE)

    assert report["conforms"] is True
    assert report["focus_nodes"] == 1
    assert report["unmatched_shapes"] == []


def test_a_failing_run_is_unaffected():
    report = shacl_validate(DATA_MISSING_NAME, SHAPES_SAME_NAMESPACE)

    assert report["conforms"] is False
    assert report["focus_nodes"] == 1
    assert report["count"] == 1


def test_subclass_instances_count_as_focus_nodes():
    """SHACL selects SHACL-instances, so a subclass instance is a focus node."""
    data = """
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix ex: <http://example.org/> .
    ex:Employee rdfs:subClassOf ex:Person .
    ex:ada a ex:Employee ; ex:name "Ada" .
    """
    report = shacl_validate(data, SHAPES_SAME_NAMESPACE)

    assert report["focus_nodes"] == 1
    assert report["unmatched_shapes"] == []


def test_target_predicates_other_than_class_are_counted():
    shapes = """
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:AuthoredShape a sh:NodeShape ;
      sh:targetSubjectsOf ex:wrote ;
      sh:property [ sh:path ex:name ; sh:minCount 1 ] .
    """
    data = """
    @prefix ex: <http://example.org/> .
    ex:ada ex:wrote ex:notes ; ex:name "Ada" .
    """
    report = shacl_validate(data, shapes)

    assert report["focus_nodes"] == 1
    assert report["unmatched_shapes"] == []
    assert report["conforms"] is True


def test_a_shape_with_no_declared_target_is_not_called_unmatched():
    """Property shapes and advanced targets are not judged, only reported honestly."""
    shapes = """
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:Floating a sh:NodeShape ;
      sh:property [ sh:path ex:name ; sh:minCount 1 ] .
    ex:PersonShape a sh:NodeShape ;
      sh:targetClass ex:Person ;
      sh:property [ sh:path ex:name ; sh:minCount 1 ] .
    """
    report = shacl_validate(DATA, shapes)

    assert report["unmatched_shapes"] == []
    assert report["focus_nodes"] == 1
