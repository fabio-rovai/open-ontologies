"""Closed-world vocabulary checking.

In the open world an undeclared IRI is unknown, not wrong, so an extractor that
invents `ex:hasProteinName` produces RDF that parses, loads and passes SHACL
without complaint. Closed-world checking is the only thing that separates a real
term from a plausible-looking one, and it is the check that has no equivalent
elsewhere in the Python RDF stack.

Mirrors the semantics of the Rust engine's `vocab_check`: police the namespaces
the ontology itself declares terms in, never instance IRIs, never the standard
vocabularies, and refuse to return a pass when there is no vocabulary to check
against.
"""

import pytest

pytest.importorskip("pyoxigraph")

from open_ontologies_lite.vocab_check import vocab_check  # noqa: E402

ONTOLOGY = """
@prefix owl:  <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex:   <http://example.org/onto#> .

ex:Person a owl:Class .
ex:Protein a owl:Class .
ex:hasName a owl:DatatypeProperty ; rdfs:domain ex:Person .
"""

CLEAN_DATA = """
@prefix ex: <http://example.org/onto#> .
<http://example.org/instances/ada> a ex:Person ; ex:hasName "Ada" .
"""

INVENTED_TERM = """
@prefix ex: <http://example.org/onto#> .
<http://example.org/instances/ada> a ex:Person ; ex:hasProteinName "Ada" .
"""


def test_clean_data_conforms():
    report = vocab_check(ONTOLOGY, CLEAN_DATA)

    assert report["conforms"] is True
    assert report["undeclared_terms"] == []
    assert report["ontology_terms"] == 3


def test_an_invented_predicate_is_flagged():
    report = vocab_check(ONTOLOGY, INVENTED_TERM)

    assert report["conforms"] is False
    assert report["undeclared_terms"] == ["http://example.org/onto#hasProteinName"]


def test_instance_iris_are_not_policed():
    """Individuals live in the data, not the vocabulary; flagging them is noise."""
    data = """
    @prefix ex: <http://example.org/onto#> .
    <http://example.org/onto#ada> a ex:Person ; ex:hasName "Ada" .
    """
    report = vocab_check(ONTOLOGY, data)

    assert report["conforms"] is True, report["undeclared_terms"]


def test_terms_outside_the_policed_namespaces_are_left_alone():
    data = """
    @prefix ex:   <http://example.org/onto#> .
    @prefix dct:  <http://purl.org/dc/terms/> .
    <http://example.org/instances/ada> a ex:Person ; dct:title "Ada" .
    """
    report = vocab_check(ONTOLOGY, data)

    assert report["conforms"] is True
    assert "http://purl.org/dc/terms/" not in report["checked_namespaces"]


def test_standard_vocabularies_are_never_policed():
    data = """
    @prefix ex:   <http://example.org/onto#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    <http://example.org/instances/ada> a ex:Person ; rdfs:label "Ada" .
    """
    report = vocab_check(ONTOLOGY, data)

    assert report["conforms"] is True


def test_no_vocabulary_never_returns_a_pass():
    """The footgun this check exists to kill: a green light from an empty ontology."""
    report = vocab_check("", CLEAN_DATA)

    assert report["conforms"] is False
    assert "nothing was checked" in report["warning"]
    assert report["ontology_terms"] == 0


def test_extra_namespaces_can_be_policed_explicitly():
    report = vocab_check(
        "", INVENTED_TERM, extra_namespaces=["http://example.org/onto#"]
    )

    assert report["conforms"] is False
    assert "http://example.org/onto#hasProteinName" in report["undeclared_terms"]


def test_a_term_carrying_only_a_domain_axiom_counts_as_declared():
    ontology = """
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix ex:   <http://example.org/onto#> .
    ex:hasName rdfs:domain ex:Person .
    """
    data = """
    @prefix ex: <http://example.org/onto#> .
    <http://example.org/instances/ada> ex:hasName "Ada" .
    """
    report = vocab_check(ontology, data)

    assert report["conforms"] is True
