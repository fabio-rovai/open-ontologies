import pytest
from open_ontologies_lite import OntologyEngine
from open_ontologies_lite.horn.reason import AssertedGraphError, INFERRED_GRAPH

@pytest.mark.parametrize('with_default',[False,True])
def test_reported_blank_graph_names_can_be_selected(tmp_path,with_default):
    engine=OntologyEngine()
    engine.load('<http://x/s> <http://x/p> <http://x/o> _:g .','nquads')
    if with_default:engine.load('<http://x/default> <http://x/p> <http://x/o> .')
    report=engine.reason_horn(certificate_dir=tmp_path/'all',graphs='all',check=False)
    assert any(label.startswith('_:') for label in report['asserted_graphs'])
    selected=engine.reason_horn(certificate_dir=tmp_path/'selected',graphs=report['asserted_graphs'],check=False)
    assert selected['asserted_graphs']==report['asserted_graphs']
    assert selected['asserted_triples']==report['asserted_triples']==1+int(with_default)
    assert set((tmp_path/'selected'/'asserted.tsv').read_text().splitlines())==set((tmp_path/'all'/'asserted.tsv').read_text().splitlines())
    assert selected['check']['status']=='not_requested'

@pytest.mark.parametrize('graphs',[['http://x/g'],['<http://x/g>'],['default']])
def test_existing_graph_name_forms_control(tmp_path,graphs):
    engine=OntologyEngine()
    engine.load('<http://x/s> <http://x/p> <http://x/o> <http://x/g> .','nquads')
    engine.load('<http://x/default> <http://x/p> <http://x/o> .')
    report=engine.reason_horn(certificate_dir=tmp_path/'control',graphs=graphs,check=False)
    assert report['asserted_triples']==1
    assert report['asserted_graphs']==(['default'] if graphs==['default'] else ['<http://x/g>'])

@pytest.mark.parametrize('graphs',[[INFERRED_GRAPH],['<'+INFERRED_GRAPH+'>']])
def test_inferred_graph_refusal_control(tmp_path,graphs):
    engine=OntologyEngine()
    with pytest.raises(AssertedGraphError):engine.reason_horn(certificate_dir=tmp_path/'refused',graphs=graphs,check=False)
