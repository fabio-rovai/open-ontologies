"""Tests for the dataframe bridge (fenic / polars / pandas / pyarrow duck-typing)."""

import pytest

from open_ontologies_lite import OntologyEngine, rows_from_dataframe, rows_to_turtle

ROWS = [
    {"id": 1, "name": "Bakery", "parent": None},
    {"id": 2, "name": "Dairy", "parent": "Food"},
]


class FakeFenicDataFrame:
    """fenic DataFrame / pyarrow Table protocol: to_pylist()."""

    def to_pylist(self):
        return list(ROWS)


class FakePolarsDataFrame:
    def to_dicts(self):
        return list(ROWS)


class FakePandasDataFrame:
    def to_dict(self, orient):
        assert orient == "records"
        return list(ROWS)


@pytest.mark.parametrize(
    "df", [FakeFenicDataFrame(), FakePolarsDataFrame(), FakePandasDataFrame(), ROWS]
)
def test_rows_from_dataframe_protocols(df):
    assert rows_from_dataframe(df) == ROWS


def test_rows_from_dataframe_rejects_unknown():
    with pytest.raises(TypeError):
        rows_from_dataframe(object())


def test_rows_to_turtle_typed_literals_and_none_skipped():
    ttl = rows_to_turtle(ROWS, base_iri="http://x.org/", id_column="id")
    assert '<http://x.org/1> <http://x.org/name> "Bakery" .' in ttl
    assert '"1"^^<http://www.w3.org/2001/XMLSchema#integer>' in ttl
    assert "parent> ." not in ttl  # None dropped, no dangling triple
    assert ttl == rows_to_turtle(ROWS, base_iri="http://x.org/", id_column="id")  # deterministic


def test_rows_to_turtle_class_and_index_subjects():
    ttl = rows_to_turtle(
        [{"v": True, "w": 1.5}],
        base_iri="http://x.org/",
        class_iri="http://x.org/Thing",
    )
    assert "<http://x.org/0> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.org/Thing> ." in ttl
    assert '"true"^^<http://www.w3.org/2001/XMLSchema#boolean>' in ttl
    assert '"1.5"^^<http://www.w3.org/2001/XMLSchema#double>' in ttl


def test_rows_to_turtle_escapes_strings_and_sanitizes_ids():
    ttl = rows_to_turtle(
        [{"id": "a b/c", "note": 'say "hi"\nbye'}],
        base_iri="http://x.org/",
        id_column="id",
    )
    assert "<http://x.org/a_b_c>" in ttl
    assert '"say \\"hi\\"\\nbye"' in ttl


def test_engine_load_rows_roundtrip():
    engine = OntologyEngine()
    n = engine.load_rows(
        FakeFenicDataFrame(),
        base_iri="http://x.org/",
        class_iri="http://x.org/Category",
        id_column="id",
    )
    assert n == engine.stats()["triples"]
    res = engine.query(
        "SELECT ?name WHERE { ?s a <http://x.org/Category> ; <http://x.org/name> ?name } ORDER BY ?name"
    )
    assert [r["name"] for r in res["rows"]] == ["Bakery", "Dairy"]


def test_real_fenic_dataframe_if_installed():
    fc = pytest.importorskip("fenic")
    session = fc.Session.get_or_create(fc.SessionConfig(app_name="oo_lite_test"))
    df = session.create_dataframe({"id": [1, 2], "name": ["Bakery", "Dairy"]})
    engine = OntologyEngine()
    n = engine.load_rows(df, base_iri="http://x.org/", id_column="id")
    assert n == 4
    session.stop()
