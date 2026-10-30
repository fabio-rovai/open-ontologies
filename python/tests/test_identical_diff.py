import pytest
from open_ontologies_lite import OntologyEngine
from open_ontologies_lite.kgcl import kgcl_diff

@pytest.mark.parametrize('data,count', [
 ('@prefix ex:<http://x/>. ex:S ex:p [ex:q "value"].',2),
 ('@prefix ex:<http://x/>. ex:S ex:p [ex:q [ex:r "value"]].',3),
 ('@prefix ex:<http://x/>. ex:S ex:p ("a" "b").',5),
 ('@prefix ex:<http://x/>. ex:S ex:p _:b. _:b ex:q "value".',2),
 ('@prefix ex:<http://x/>. ex:S ex:p "value". ex:S ex:p "value".',1),
])
@pytest.mark.parametrize('api',['engine','kgcl'])
def test_identical_document_has_no_changes(data,count,api):
    if api == 'engine':
        result=OntologyEngine.diff(data,data)
        assert result == {'added':[], 'removed':[], 'added_count':0, 'removed_count':0, 'unchanged_count':count}
    else:
        assert kgcl_diff(data,data).changes == []

@pytest.mark.parametrize('diff',[OntologyEngine.diff,kgcl_diff])
def test_identical_invalid_document_still_rejected(diff):
    with pytest.raises(SyntaxError): diff('not turtle','not turtle')

def test_genuine_named_node_difference_control():
    old='<http://x/S> <http://x/p> <http://x/A>.'
    new='<http://x/S> <http://x/p> <http://x/B>.'
    result=OntologyEngine.diff(old,new)
    assert result['added_count']==result['removed_count']==1
    assert result['unchanged_count']==0
    assert kgcl_diff(old,new).counts() == {'edge_creation':1,'edge_deletion':1}
