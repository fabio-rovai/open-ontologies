import pytest
from open_ontologies_lite import OntologyEngine

DATA='<http://x/a> <http://x/b> <<( <http://x/s> <http://x/p> <http://x/o> )>> .'
QUERIES={
    'select':'SELECT ?o WHERE { <http://x/a> <http://x/b> ?o }',
    'construct':'CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }',
    'describe':'DESCRIBE <http://x/a>',
}
@pytest.mark.parametrize('kind',QUERIES)
def test_query_rdf_triple_term(kind):
    engine=OntologyEngine()
    assert engine.load(DATA)==1
    result=engine.query(QUERIES[kind])
    expected=str(next(iter(engine.store)).object)
    if kind=='select':
        assert result=={'type':'table','variables':['o'],'rows':[{'o':expected}]}
    else:
        assert result=={'type':'graph','triples':[{'subject':'http://x/a','predicate':'http://x/b','object':expected}]}

@pytest.mark.parametrize('kind',QUERIES)
def test_ordinary_literal_query_control(kind):
    engine=OntologyEngine()
    engine.load('<http://x/a> <http://x/b> "ordinary" .')
    result=engine.query(QUERIES[kind])
    if kind=='select':assert result=={'type':'table','variables':['o'],'rows':[{'o':'ordinary'}]}
    else:assert result=={'type':'graph','triples':[{'subject':'http://x/a','predicate':'http://x/b','object':'ordinary'}]}

def test_ask_control():
    engine=OntologyEngine();engine.load(DATA)
    assert engine.query('ASK { ?s ?p ?o }')=={'type':'boolean','boolean':True}

def test_unbound_select_control():
    engine=OntologyEngine();engine.load('<http://x/a> <http://x/b> "ordinary" .')
    assert engine.query('SELECT ?s ?unbound WHERE {?s ?p ?o}')=={'type':'table','variables':['s','unbound'],'rows':[{'s':'http://x/a','unbound':None}]}

@pytest.mark.parametrize('term,expected', [
    ('<http://x/o>', 'http://x/o'),
    ('"7"^^<http://www.w3.org/2001/XMLSchema#integer>', '7'),
    ('"bonjour"@fr', 'bonjour'),
])
def test_scalar_lexical_value_control(term,expected):
    engine=OntologyEngine()
    engine.load('<http://x/a> <http://x/b> '+term+' .')
    assert engine.query(QUERIES['select'])=={'type':'table','variables':['o'],'rows':[{'o':expected}]}
