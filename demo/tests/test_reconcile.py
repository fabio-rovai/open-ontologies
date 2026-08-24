from rdflib import Graph, Namespace, RDFS, OWL, RDF

from demo.ontology_from_docs import reconcile

EX = Namespace("https://example.org/t#")


def _graph(pairs):
    g = Graph()
    for sub, parent in pairs:
        g.add((sub, RDF.type, OWL.Class))
        g.add((sub, RDFS.subClassOf, parent))
    return g


def test_attribute_class_is_removed_when_a_partition_exists():
    g = _graph([(EX.PublishedDataset, EX.Dataset), (EX.DraftDataset, EX.Dataset)])
    g.add((EX.DatasetType, RDF.type, OWL.Class))
    out = reconcile(g)
    assert (EX.DatasetType, RDF.type, OWL.Class) not in out
    assert (EX.PublishedDataset, RDFS.subClassOf, EX.Dataset) in out


def test_status_is_spared_because_states_are_attributes():
    g = _graph([(EX.ActiveThing, EX.Thing), (EX.RetiredThing, EX.Thing)])
    g.add((EX.ThingStatus, RDF.type, OWL.Class))
    out = reconcile(g)
    assert (EX.ThingStatus, RDF.type, OWL.Class) in out
