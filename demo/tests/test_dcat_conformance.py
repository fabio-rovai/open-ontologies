"""Pin the headline figures demo/dcat_conformance.py measures.

These numbers are recomputed here, from the vendored jsonschema/ tree under
demo/corpus/dcat-us/, not quoted from anywhere else. If GSA/dcat-us changes
its published examples or schema, this test fails rather than letting the
findings in demo/precomputed/findings.json silently drift away from what the
corpus actually measures.
"""

from demo.dcat_conformance import (
    CORPUS,
    DEFINITIONS,
    EXAMPLES,
    SHAPES_PATH,
    build_context,
    load_classes,
    load_shapes_namespaces,
    measure,
    merge_corpus,
    run_shacl,
    scan_bare_values,
)


def _good_files():
    return sorted(EXAMPLES.glob("*/good/*.json"))


def test_corpus_is_vendored():
    assert DEFINITIONS.is_dir()
    assert EXAMPLES.is_dir()
    assert SHAPES_PATH.is_file()
    assert len(list(DEFINITIONS.glob("*.json"))) == 26
    assert len(_good_files()) == 115
    assert len(list(EXAMPLES.glob("*/bad/*.json"))) == 76


def test_two_classes_have_zero_bound_properties():
    # skos:Concept and adms:Identifier are the schema's two most frequently
    # instantiated types in the example corpus; the schema names properties
    # for both but assigns none of them an RDF term.
    classes = load_classes(DEFINITIONS)
    assert classes["Concept"].properties == {}
    assert classes["Identifier"].properties == {}


def test_as_published_examples_carry_no_dcat_vocabulary():
    files = _good_files()
    result = measure(files, None)
    assert result["files"] == 115
    assert result["triples"] == 76
    assert result["distinctPredicates"] == 1
    assert result["dcatPredicateTriples"] == 0
    assert result["zeroTripleFiles"] == 38
    assert len(result["errors"]) == 1


def test_binding_the_schema_implies_recovers_dcat_vocabulary():
    classes = load_classes(DEFINITIONS)
    namespaces = load_shapes_namespaces(SHAPES_PATH)
    files = _good_files()
    lenient_terms = scan_bare_values(files, classes)
    assert len(lenient_terms) == 22
    context = build_context(classes, namespaces, lenient_terms=lenient_terms)
    result = measure(files, context)
    assert result["triples"] == 1510
    assert result["distinctPredicates"] == 123
    assert result["dcatPredicateTriples"] == 228
    assert result["zeroTripleFiles"] == 10
    assert len(result["errors"]) == 0


def test_deleted_shapes_select_nothing_as_published_but_fire_once_bound():
    classes = load_classes(DEFINITIONS)
    namespaces = load_shapes_namespaces(SHAPES_PATH)
    files = _good_files()
    lenient_terms = scan_bare_values(files, classes)
    context = build_context(classes, namespaces, lenient_terms=lenient_terms)

    published = run_shacl(SHAPES_PATH, merge_corpus(files, None))
    assert published["conforms"] is True
    assert published["violations"] == 0
    assert published["focusNodes"] == 0

    bound = run_shacl(SHAPES_PATH, merge_corpus(files, context))
    assert bound["conforms"] is False
    assert bound["focusNodes"] == 228
    assert bound["violations"] == 272
