"""Public consumer regression for RDF literal datatype identity."""
import json

import pytest
from open_ontologies_lite import kgcl_diff

PREFIX = '@prefix ex: <http://example.org/> . @prefix xsd: <http://www.w3.org/2001/XMLSchema#> . '

@pytest.mark.parametrize("old_object,new_object", [
    ('"7"', '"7"^^xsd:integer'),
    ('"7"^^xsd:integer', '"7"^^xsd:decimal'),
    ('"7"^^xsd:integer', '"7"^^xsd:double'),
], ids=["string-to-integer", "integer-to-decimal", "integer-to-double"])
def test_kgcl_diff_detects_literal_datatype_change(old_object, new_object):
    old = PREFIX + f'ex:Thing ex:value {old_object} .'
    new = PREFIX + f'ex:Thing ex:value {new_object} .'
    result = kgcl_diff(old, new)
    assert result.counts() == {"edge_creation": 1, "edge_deletion": 1}
    assert {change["subject"] for change in result.changes} == {"http://example.org/Thing"}
    expected = {}
    for kind, value in [("edge_deletion", old_object), ("edge_creation", new_object)]:
        if "^^xsd:" in value:
            lexical, datatype = value.split("^^xsd:")
            value = lexical + "^^<http://www.w3.org/2001/XMLSchema#" + datatype + ">"
        expected[kind] = value
    assert {change["type"]: change["object"] for change in result.changes} == expected
    assert kgcl_diff(old, old).changes == []
    assert kgcl_diff(new, new).changes == []

@pytest.mark.parametrize("object_text", ['"7"', '"7"^^xsd:integer', '"7"^^xsd:decimal', '"7"^^xsd:double', '"seven"@en'], ids=["string", "integer", "decimal", "double", "language"])
def test_kgcl_unchanged_literal_is_empty(object_text):
    document = PREFIX + f'ex:Thing ex:value {object_text} .'
    assert kgcl_diff(document, document).changes == []

@pytest.mark.parametrize("label", ['"seven"', '"seven"@en', '"7"^^xsd:integer'], ids=["plain", "language", "typed"])
def test_created_node_label_retains_lexical_value(label):
    document = PREFIX + f'ex:Thing <http://www.w3.org/2000/01/rdf-schema#label> {label} .'
    result = kgcl_diff("", document)
    assert result.changes == [{"type": "node_creation", "node": "http://example.org/Thing", "label": "7" if "integer" in label else "seven"}]


def test_rename_and_annotation_fields_remain_lexical():
    old = PREFIX + 'ex:Thing <http://www.w3.org/2000/01/rdf-schema#label> "old"@en ; <http://www.w3.org/2000/01/rdf-schema#comment> "before" .'
    new = PREFIX + 'ex:Thing <http://www.w3.org/2000/01/rdf-schema#label> "new"@en ; <http://www.w3.org/2000/01/rdf-schema#comment> "after" .'
    changes = {change["type"]: change for change in kgcl_diff(old, new).changes}
    assert changes["node_rename"]["old"] == "old"
    assert changes["node_rename"]["new"] == "new"
    assert changes["node_annotation_change"]["old"] == "before"
    assert changes["node_annotation_change"]["new"] == "after"

@pytest.mark.parametrize("predicate,change_type", [
    ("label", "node_rename"), ("comment", "node_annotation_change"),
])
@pytest.mark.parametrize("kind", ["named", "blank"])
def test_nonliteral_label_and_annotation_changes_keep_term_spelling(predicate, change_type, kind):
    old_object, new_object = ("<http://example.org/old>", "<http://example.org/new>") if kind == "named" else ("_:old", "_:new")
    pred = "http://www.w3.org/2000/01/rdf-schema#" + predicate
    old = PREFIX + f'ex:Thing <{pred}> {old_object} .'
    new = PREFIX + f'ex:Thing <{pred}> {new_object} .'
    changes = kgcl_diff(old, new).changes
    assert len(changes) == 1 and changes[0]["type"] == change_type
    if kind == "named":
        assert changes[0]["old"] == "http://example.org/old"
        assert changes[0]["new"] == "http://example.org/new"
    else:
        assert changes[0]["old"].startswith("_:")
        assert changes[0]["new"].startswith("_:")


@pytest.mark.parametrize("object_text", ["<http://example.org/label>", "_:label"], ids=["named", "blank"])
def test_created_node_nonliteral_labels_stay_absent(object_text):
    document = PREFIX + f'ex:Thing <http://www.w3.org/2000/01/rdf-schema#label> {object_text} .'
    assert kgcl_diff("", document).changes == [{"type": "node_creation", "node": "http://example.org/Thing", "label": None}]


def test_escaped_label_lexical_value_is_not_parsed_from_display_text():
    before = 'say "old" and \\path'
    after = 'say "new" and \\path'
    pred = "http://www.w3.org/2000/01/rdf-schema#label"
    old = PREFIX + f'ex:Thing <{pred}> {json.dumps(before)} .'
    new = PREFIX + f'ex:Thing <{pred}> {json.dumps(after)} .'
    assert kgcl_diff("", old).changes[0]["label"] == before
    assert kgcl_diff(old, new).changes == [{"type": "node_rename", "node": "http://example.org/Thing", "old": before, "new": after}]
