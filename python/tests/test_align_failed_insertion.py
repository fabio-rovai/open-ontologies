import pytest
pytest.importorskip("hnswlib")
from open_ontologies_lite.align import AlignmentIndex

def state(index):
    result = {"length": len(index), "native_count": index._index.get_current_count()}
    try:
        result["ids"] = sorted(candidate.id for candidate in index.query([1., 0.], k=10))
    except RuntimeError as exc:
        result["query_error"] = type(exc).__name__
    return result

@pytest.mark.parametrize("failure", ["capacity", "numeric"])
def test_failed_native_insertion_keeps_existing_index_usable(failure):
    index = AlignmentIndex(2, max_elements=1 if failure == "capacity" else 3)
    index.add("first", [1., 0.])
    with pytest.raises(RuntimeError if failure == "capacity" else ValueError):
        index.add("rejected", [0., 1.] if failure == "capacity" else ["not-a-number", 0.])
    assert state(index) == {"length": 1, "native_count": 1, "ids": ["first"]}

def test_valid_insertion_after_rejection_has_no_ghost_label():
    index = AlignmentIndex(2, max_elements=3)
    index.add("first", [1., 0.])
    with pytest.raises(ValueError):
        index.add("rejected", ["not-a-number", 0.])
    label = index.add("second", [0., 1.])
    assert {"label": label, **state(index)} == {"label": 1, "length": 2, "native_count": 2, "ids": ["first", "second"]}

def test_normal_add_and_query_control():
    index = AlignmentIndex(2, max_elements=2)
    assert index.add("first", [1., 0.]) == 0
    assert state(index) == {"length": 1, "native_count": 1, "ids": ["first"]}

def test_empty_index_control():
    index = AlignmentIndex(2)
    assert len(index) == 0 and index.query([1., 0.]) == []


def test_first_rejected_insertion_keeps_empty_index_and_build_state():
    index = AlignmentIndex(2, max_elements=2)
    with pytest.raises(ValueError):
        index.add("rejected", ["not-a-number", 0.])
    assert index._built is False
    assert state(index) == {"length": 0, "native_count": 0, "ids": []}
    assert index.add("first", [1., 0.]) == 0
    assert index._built is True
    assert state(index) == {"length": 1, "native_count": 1, "ids": ["first"]}
