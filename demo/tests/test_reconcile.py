from demo.ontology_from_docs import _reconcile_ttl

PREFIXES = ("@prefix : <https://example.org/t#> .\n"
            "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n"
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n")


def _merged(tmp_path, body):
    path = tmp_path / "merged.ttl"
    path.write_text(PREFIXES + body)
    return path


def test_attribute_class_is_removed_when_a_partition_exists(tmp_path):
    path = _merged(tmp_path, """
:Dataset a owl:Class .
:PublishedDataset a owl:Class .
:PublishedDataset rdfs:subClassOf :Dataset .
:DraftDataset a owl:Class .
:DraftDataset rdfs:subClassOf :Dataset .
:DatasetType a owl:Class .
""")
    doomed = _reconcile_ttl(path)
    assert doomed == ["DatasetType"]
    out = path.read_text()
    assert ":DatasetType a owl:Class ." not in out
    assert ":PublishedDataset rdfs:subClassOf :Dataset ." in out


def test_status_is_spared_because_states_are_attributes(tmp_path):
    path = _merged(tmp_path, """
:Thing a owl:Class .
:ActiveThing a owl:Class .
:ActiveThing rdfs:subClassOf :Thing .
:RetiredThing a owl:Class .
:RetiredThing rdfs:subClassOf :Thing .
:ThingStatus a owl:Class .
""")
    doomed = _reconcile_ttl(path)
    assert doomed == []
    out = path.read_text()
    assert ":ThingStatus a owl:Class ." in out
