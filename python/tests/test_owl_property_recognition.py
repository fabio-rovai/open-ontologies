"""Explicit OWL properties participate in the lightweight schema tools."""
import pytest
from open_ontologies_lite import OntologyEngine

PREFIX = "@prefix ex:<http://example.org/>. @prefix owl:<http://www.w3.org/2002/07/owl#>. @prefix rdf:<http://www.w3.org/1999/02/22-rdf-syntax-ns#>. @prefix rdfs:<http://www.w3.org/2000/01/rdf-schema#>. "

@pytest.mark.parametrize("kind", ["owl:ObjectProperty", "owl:DatatypeProperty", "owl:AnnotationProperty", "rdf:Property"])
def test_explicit_properties_are_schema_not_individuals(kind):
    engine = OntologyEngine()
    engine.load(PREFIX + f"ex:p a {kind}.")
    assert engine.stats() == {"triples": 1, "classes": 0, "properties": 1, "individuals": 0}
    assert {(i["rule"], i["term"]) for i in engine.lint()["issues"]} == {
        ("missing-domain", "http://example.org/p"), ("missing-range", "http://example.org/p")}

@pytest.mark.parametrize("kind", ["owl:ObjectProperty", "owl:DatatypeProperty", "owl:AnnotationProperty", "rdf:Property"])
def test_complete_property_has_no_hygiene_issues(kind):
    engine = OntologyEngine()
    engine.load(PREFIX + f"ex:p a {kind}; rdfs:domain ex:C; rdfs:range ex:D.")
    assert engine.stats()["properties"] == 1
    assert engine.stats()["individuals"] == 0
    assert engine.lint()["issues"] == []

def test_multiple_property_declarations_are_counted_once():
    engine = OntologyEngine()
    engine.load(PREFIX + "ex:p a rdf:Property, owl:ObjectProperty. ex:c a owl:Class. ex:i a ex:c, owl:NamedIndividual.")
    assert engine.stats() == {"triples": 5, "classes": 1, "properties": 1, "individuals": 1}
    assert len(engine.lint()["issues"]) == 3

def test_schema_punning_preserves_explicit_instance_type():
    engine = OntologyEngine()
    engine.load(PREFIX + "ex:p a owl:ObjectProperty, ex:CustomClass.")
    assert engine.stats()["properties"] == 1
    assert engine.stats()["individuals"] == 1
