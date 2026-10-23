"""Declarative targets identify shapes even without a NodeShape type."""
import pytest
pytest.importorskip("pyshacl")
from open_ontologies_lite.shacl import shacl_validate

PREFIX = "@prefix ex:<http://example.org/>. @prefix sh:<http://www.w3.org/ns/shacl#>. "
KINDS = ["implicit-node", "explicit-property", "implicit-property", "explicit-node"]

def shape(kind, target):
    typing = {"implicit-node": "", "explicit-property": "a sh:PropertyShape; ", "implicit-property": "", "explicit-node": "a sh:NodeShape; "}[kind]
    constraint = "sh:path ex:required; sh:minCount 1" if "property" in kind else "sh:nodeKind sh:IRI"
    return PREFIX + f"ex:S {typing}{target}; {constraint}."

@pytest.mark.parametrize("kind", KINDS)
@pytest.mark.parametrize("target", ["class", "subjects", "objects"])
@pytest.mark.parametrize("matched", [True, False], ids=["matched", "empty"])
def test_graph_targets_report_checked_nodes(kind, target, matched):
    predicate = {"class": "sh:targetClass ex:Person", "subjects": "sh:targetSubjectsOf ex:p", "objects": "sh:targetObjectsOf ex:p"}[target]
    data = {"class": "ex:n a ex:Person.", "subjects": "ex:n ex:p ex:o.", "objects": "ex:o ex:p ex:n."}[target] if matched else ""
    report = shacl_validate(PREFIX + data, shape(kind, predicate))
    assert report["focus_nodes"] == int(matched)
    if matched:
        assert report["unmatched_shapes"] == []
        assert report["conforms"] is (False if "property" in kind else True)
    else:
        assert report["conforms"] is None
        assert report["unmatched_shapes"] == [{"shape": "http://example.org/S", "target_class": "http://example.org/Person" if target == "class" else ""}]

@pytest.mark.parametrize("kind", KINDS)
def test_node_target_counts_even_without_data_node(kind):
    report = shacl_validate("", shape(kind, "sh:targetNode ex:n"))
    assert report["focus_nodes"] == 1 and report["unmatched_shapes"] == []
    assert report["conforms"] is (False if "property" in kind else True)

def test_multiple_declarative_targets_count_each_shape_node_once():
    report = shacl_validate(PREFIX + "ex:n a ex:Person.", shape("explicit-node", "sh:targetClass ex:Person; sh:targetNode ex:n"))
    assert report["focus_nodes"] == 1 and report["unmatched_shapes"] == []

def test_untargeted_shape_does_not_become_unmatched():
    report = shacl_validate("", PREFIX + "ex:S a sh:NodeShape; sh:nodeKind sh:IRI.")
    assert report["focus_nodes"] == 0 and report["unmatched_shapes"] == [] and report["conforms"] is True


def test_sparql_only_target_keeps_unknown_coverage_policy():
    shapes = PREFIX + 'ex:S a sh:NodeShape; sh:target [a sh:SPARQLTarget; sh:select "SELECT ?this WHERE { ?this a <http://example.org/Person> }"]; sh:nodeKind sh:IRI.'
    report = shacl_validate(PREFIX + "ex:n a ex:Person.", shapes)
    assert report["conforms"] is True
    assert report["focus_nodes"] == 0 and report["unmatched_shapes"] == []


@pytest.mark.parametrize("inference", ["rdfs", "owlrl", "both"])
def test_coverage_uses_the_graph_that_received_inference(inference):
    from open_ontologies_lite import OntologyEngine
    data = PREFIX + "@prefix rdfs:<http://www.w3.org/2000/01/rdf-schema#>. ex:p rdfs:domain ex:Person. ex:n ex:p ex:o."
    engine = OntologyEngine()
    engine.load(data)
    before = engine.dump("ntriples")
    report = shacl_validate(engine.dump(), shape("explicit-property", "sh:targetClass ex:Person"), inference=inference)
    assert report["conforms"] is False
    assert report["focus_nodes"] == 1 and report["unmatched_shapes"] == []
    assert report["count"] == 1
    assert engine.dump("ntriples") == before

def test_disabled_inference_keeps_unmatched_target():
    data = PREFIX + "@prefix rdfs:<http://www.w3.org/2000/01/rdf-schema#>. ex:p rdfs:domain ex:Person. ex:n ex:p ex:o."
    report = shacl_validate(data, shape("explicit-node", "sh:targetClass ex:Person"))
    assert report["conforms"] is None and report["focus_nodes"] == 0
    assert report["count"] == 0 and len(report["unmatched_shapes"]) == 1

def test_explicit_membership_with_inference_retains_coverage():
    report = shacl_validate(PREFIX + "ex:n a ex:Person.", shape("explicit-node", "sh:targetClass ex:Person"), inference="rdfs")
    assert report["conforms"] is True and report["focus_nodes"] == 1
    assert report["unmatched_shapes"] == []


def test_advanced_rules_use_owned_validation_graph():
    from open_ontologies_lite import OntologyEngine
    engine = OntologyEngine()
    engine.load(PREFIX + "ex:n ex:p ex:o.")
    before = engine.dump("ntriples")
    shapes = PREFIX + "@prefix rdf:<http://www.w3.org/1999/02/22-rdf-syntax-ns#>. ex:Generate a sh:NodeShape; sh:targetNode ex:n; sh:rule [a sh:TripleRule; sh:subject sh:this; sh:predicate rdf:type; sh:object ex:Derived]. ex:Check a sh:NodeShape; sh:targetClass ex:Derived; sh:property [sh:path ex:required; sh:minCount 1]."
    report = shacl_validate(engine.dump(), shapes)
    assert engine.dump("ntriples") == before
    assert report["conforms"] is False and report["count"] == 1
    assert report["focus_nodes"] == 2 and report["unmatched_shapes"] == []
