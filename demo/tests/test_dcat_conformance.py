"""Pin the headline figures demo/dcat_conformance.py measures.

These numbers are recomputed here, from the vendored jsonschema/ tree and
the vendored recovered-shapes.ttl / recovered-context.jsonld under
demo/corpus/dcat-us/, not quoted from anywhere else. If GSA/dcat-us changes
its published examples or schema, this test fails rather than letting the
findings in demo/precomputed/findings.json silently drift away from what the
corpus actually measures.

Nothing in this file asserts a single SHACL violation count as though it
were settled. Three independent, reproducible measurements against the same
unmodified recovered-shapes.ttl disagree with each other (schema-derived
declared binding, schema-derived observed binding, and the profile's own
real recovered context); test_shacl_violation_counts_disagree_across_methods
pins the disagreement itself, not a winner.
"""

from demo.dcat_conformance import (
    CORPUS,
    DEFINITIONS,
    EXAMPLES,
    REAL_CONTEXT_PATH,
    SHAPES_PATH,
    build_context,
    check_real_context_org_prefix,
    load_classes,
    load_real_context,
    load_shapes_namespaces,
    measure,
    merge_corpus,
    per_file_shacl_violations,
    per_file_triples,
    run_shacl,
    scan_bare_values,
    title_to_rdf_class,
)


def _good_files():
    return sorted(EXAMPLES.glob("*/good/*.json"))


def test_corpus_is_vendored():
    assert DEFINITIONS.is_dir()
    assert EXAMPLES.is_dir()
    assert SHAPES_PATH.is_file()
    assert REAL_CONTEXT_PATH.is_file()
    assert len(list(DEFINITIONS.glob("*.json"))) == 26
    assert len(_good_files()) == 115
    assert len(list(EXAMPLES.glob("*/bad/*.json"))) == 76


def test_two_classes_unbound_in_schema_residue_but_bound_in_real_context():
    # skos:Concept and adms:Identifier are the schema's two most frequently
    # instantiated types in the example corpus. The schema's own remaining
    # metadata (_oldDocs, what is left after pull request 120 deleted the
    # context and the shapes) assigns neither class's properties an RDF
    # term. That is true only of this residue, not of the profile's actual
    # publication history: see the second half of this test.
    classes = load_classes(DEFINITIONS)
    assert classes["Concept"].properties == {}
    assert classes["Identifier"].properties == {}

    # The profile's own deleted context -- recovered separately from pull
    # request 120's base commit, not reconstructed from the schema residue
    # above -- did bind every one of these properties before it was removed
    # by the same pull request that removed the shapes. This is the guard
    # against the original, corrected error: asserting that these classes
    # carry no RDF term anywhere, rather than only in what the schema alone
    # still states.
    real_context = load_real_context(REAL_CONTEXT_PATH)
    concept_terms = set(real_context["@context"]["skos:Concept"]["@context"].keys())
    identifier_terms = set(real_context["@context"]["adms:Identifier"]["@context"].keys())
    assert {"prefLabel", "altLabel", "definition", "inScheme", "notation"} <= concept_terms
    assert {"schemaAgency", "creator", "issued", "version", "notation"} <= identifier_terms


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


def test_real_context_verbatim_binds_almost_nothing_against_todays_examples():
    # The profile's own recovered context, injected exactly as retrieved,
    # with no rewriting. Its class bindings are keyed by full CURIE
    # ('dcat:Dataset', 'skos:Concept'); every good example's own '@type' is
    # the bare schema title ('Dataset', 'Concept'). JSON-LD type-scoped
    # contexts require an exact term match, so this is nearly indistinguishable
    # from publishing no context at all -- a real, measured fact about the
    # naming drift between the real context and today's examples, not a bug
    # in this script.
    real_context = load_real_context(REAL_CONTEXT_PATH)
    files = _good_files()
    result = measure(files, real_context)
    assert result["triples"] == 76
    assert result["distinctPredicates"] == 1
    assert result["dcatPredicateTriples"] == 0


def test_real_context_typed_recovers_dcat_vocabulary_the_reconstruction_also_finds():
    # The same unmodified real context, with each example's bare '@type'
    # mechanically rewritten to the CURIE the schema's own
    # _oldDocs.rdfClass already names for that title -- a reformatting of a
    # class name the schema itself states, not an invented binding.
    classes = load_classes(DEFINITIONS)
    real_context = load_real_context(REAL_CONTEXT_PATH)
    retype_map = title_to_rdf_class(classes)
    files = _good_files()
    result = measure(files, real_context, retype=retype_map)
    assert result["triples"] == 1069
    assert result["distinctPredicates"] == 114
    assert result["dcatPredicateTriples"] == 184
    assert len(result["errors"]) == 0


def test_shacl_violation_counts_disagree_across_methods():
    """Three reproducible measurements against the SAME unmodified
    recovered-shapes.ttl and the SAME 115 examples do not agree on a
    violation count. This test pins that disagreement, not a winner: it is
    the guard against a future change collapsing three real, differently
    computed numbers into one asserted "the" count.
    """
    classes = load_classes(DEFINITIONS)
    namespaces = load_shapes_namespaces(SHAPES_PATH)
    files = _good_files()

    published = run_shacl(SHAPES_PATH, merge_corpus(files, None))
    assert published["conforms"] is True
    assert published["violations"] == 0
    assert published["focusNodes"] == 0

    declared = build_context(classes, namespaces)
    lenient_terms = scan_bare_values(files, classes)
    observed = build_context(classes, namespaces, lenient_terms=lenient_terms)
    bound_declared = run_shacl(SHAPES_PATH, merge_corpus(files, declared))
    bound_observed = run_shacl(SHAPES_PATH, merge_corpus(files, observed))
    assert bound_declared["conforms"] is False
    assert bound_declared["focusNodes"] == 228
    assert bound_declared["violations"] == 178
    assert bound_observed["conforms"] is False
    assert bound_observed["focusNodes"] == 228
    assert bound_observed["violations"] == 272

    real_context = load_real_context(REAL_CONTEXT_PATH)
    retype_map = title_to_rdf_class(classes)
    bound_real_typed = run_shacl(SHAPES_PATH, merge_corpus(files, real_context, retype=retype_map))
    assert bound_real_typed["conforms"] is False
    assert bound_real_typed["focusNodes"] == 183
    assert bound_real_typed["violations"] == 147

    # The point of this test: three legitimate, reproducible methods, same
    # inputs, three different violation counts. None should be published as
    # THE count.
    counts = {bound_declared["violations"], bound_observed["violations"], bound_real_typed["violations"]}
    assert len(counts) == 3


def test_per_file_figures_quoted_in_findings_json_are_reproducible():
    """demo/precomputed/findings.json quotes per-file triple and violation
    counts for specific example files (findings 1, 3 and 5). Those numbers
    must be exactly what per_file_triples() / per_file_shacl_violations()
    compute, not hand-typed approximations: this test is the thing a
    sceptical reader runs to check them.
    """
    classes = load_classes(DEFINITIONS)
    namespaces = load_shapes_namespaces(SHAPES_PATH)
    files = _good_files()
    lenient_terms = scan_bare_values(files, classes)
    observed = build_context(classes, namespaces, lenient_terms=lenient_terms)
    real_context = load_real_context(REAL_CONTEXT_PATH)
    retype_map = title_to_rdf_class(classes)

    as_published = per_file_triples(files, None)
    # Finding 1: Catalog and Dataset each expand to 1 triple as published
    # (rdf:type of a manufactured IRI, no @context).
    assert as_published["Catalog/good/complete_example.json"] == 1
    assert as_published["Dataset/good/complete_example.json"] == 1

    observed_violations = per_file_shacl_violations(files, SHAPES_PATH, observed)
    real_typed_violations = per_file_shacl_violations(files, SHAPES_PATH, real_context, retype=retype_map)

    # Finding 5: firesViolations, observed binding vs. real context typed.
    assert observed_violations["Dataset/good/complete_example.json"]["violations"] == 35
    assert real_typed_violations["Dataset/good/complete_example.json"]["violations"] == 15
    assert observed_violations["Catalog/good/complete_example.json"]["violations"] == 19
    assert real_typed_violations["Catalog/good/complete_example.json"]["violations"] == 0
    assert observed_violations["Distribution/good/complete_example.json"]["violations"] == 14
    assert real_typed_violations["Distribution/good/complete_example.json"]["violations"] == 5

    # Finding 3: Concept/Identifier reachability under the real context.
    assert real_typed_violations["Concept/good/complete_example.json"]["violations"] == 0
    assert real_typed_violations["Concept/good/minimal_object.json"]["violations"] == 1
    assert real_typed_violations["Identifier/good/complete_example.json"]["violations"] == 1


def test_per_file_keys_are_unique_across_reused_filenames():
    # "minimal_example.json", "null_example.json" and "typical_example.json"
    # each recur under many classes' good/ directories. A per-file mapping
    # keyed by bare filename would silently collide; keying by
    # '<Class>/good/<file>' must not.
    files = _good_files()
    result = per_file_triples(files, None)
    assert len(result) == len(files) == 115


def test_recovered_files_disagree_on_org_prefix():
    # A defect internal to the two recovered files themselves, both deleted
    # by the same pull request as part of the same profile release: they
    # disagree about what 'org:' expands to.
    namespaces = load_shapes_namespaces(SHAPES_PATH)
    real_context = load_real_context(REAL_CONTEXT_PATH)
    check = check_real_context_org_prefix(real_context, namespaces)
    assert check["recoveredContextOrgPrefix"] == "http://www.w3c.org/ns/org#"
    assert check["recoveredShapesOrgPrefix"] == "http://www.w3.org/ns/org#"
    assert check["agree"] is False
