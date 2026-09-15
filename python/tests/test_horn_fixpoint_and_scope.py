"""Termination, determinism, and which graph the certificate says was asserted.

Three properties, none of which the Lean checker can catch for you.

TERMINATION. There is no iteration cap by default, and there does not need to be
one: `parse_rules` refuses a head variable that does not occur in the body, so no
rule can mint a term, the Herbrand base over the terms in the graph and the table
is finite, and `known` grows monotonically, so the rounds are bounded by the size
of that base. `fixpoint_reached: True` is therefore an earned fact and not a hope.
The Rust `run_full` engine, by contrast, stops at a 64-iteration cap and reports
neither `fixpoint_reached` nor `incomplete`, so a capped run there reports a
number that reads as the closure and is not; `run_horn` gets this right and is
what this engine was copied from.

DETERMINISM. Python randomises string hashing per process, so a candidate list
built by iterating a set gives different bytes on every run. `horn.tsv` is sorted
at the top of each round for exactly that reason, and the test below runs the same
graph through two interpreters with different `PYTHONHASHSEED` and demands
identical bytes.

SCOPE. This is the one that matters most. Until 15 September 2026 the Rust
`run_horn` flattened every named graph into `asserted.tsv`, so a store holding a
previous materialisation in the inferred graph turned derived triples into
ASSERTIONS. The checker could not detect it: the soundness theorem is conditional
on the assertions, and it is TOLD what they are, so the result was a green
absolute verdict about a graph nobody asserted. It now reads
`src/reason.rs:2745`, `graph.triples_outside(&[INFERRED_GRAPH])`, and the
inferred graph is out of scope.

The two engines still differ in HOW, and the difference is deliberate. Rust
EXCLUDES the inferred graph; this package REFUSES a run that would read it. Both
keep a derived triple out of the asserted set, and refusing is the stricter of
the two, so a graph this package accepts is one the Rust engine would also have
scoped correctly.
"""

import os
import subprocess
import sys

import pyoxigraph as ox
import pytest

from open_ontologies_lite.horn.reason import INFERRED_GRAPH, AssertedGraphError, run_horn

RDFS = "http://www.w3.org/2000/01/rdf-schema#"
TYPE = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"

CHAIN = f"""
@prefix ex: <http://ex.org/> .
@prefix rdfs: <{RDFS}> .
ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C .
ex:C rdfs:subClassOf ex:D . ex:D rdfs:subClassOf ex:E .
ex:a a ex:A .
"""


def _store(turtle: str = CHAIN) -> ox.Store:
    s = ox.Store()
    s.load(turtle.encode("utf-8"), format=ox.RdfFormat.TURTLE)
    return s


def test_a_run_with_no_cap_reaches_a_genuine_fixpoint(tmp_path):
    result = run_horn(_store(), certificate_dir=tmp_path)
    assert result.fixpoint_reached is True
    assert result.incomplete is None
    assert result.derived_triples > 0


def test_running_again_over_the_closure_derives_nothing(tmp_path):
    """The definition of a fixpoint, tested rather than asserted. The closure is
    fed back in as ASSERTIONS of a second store, which is the only honest way to
    ask the question given that nothing is materialised."""
    first = run_horn(_store(), certificate_dir=tmp_path / "one")
    closed = _store()
    ntriples = "".join(f"{s} {p} {o} .\n" for s, p, o in sorted(first.derived()))
    closed.load(ntriples.encode("utf-8"), format=ox.RdfFormat.N_TRIPLES)
    second = run_horn(closed, certificate_dir=tmp_path / "two")
    assert second.derived_triples == 0
    assert second.fixpoint_reached is True
    assert second.iterations == 1


def test_a_cap_that_bites_says_the_count_is_a_lower_bound(tmp_path):
    result = run_horn(_store(), certificate_dir=tmp_path, max_iterations=1)
    assert result.fixpoint_reached is False
    assert result.incomplete is not None
    assert "LOWER BOUND" in result.incomplete
    assert result.iterations == 1


def test_the_refusal_count_is_distinct_conclusions_not_attempts(tmp_path):
    """A refused conclusion never enters `known`, so every later round matches
    the same body and re-derives it. A counter would grow with the iteration
    count and report 4 where a reader expects 1."""
    turtle = CHAIN + '\nex:a <http://www.w3.org/2002/07/owl#sameAs> "a literal" .\n'
    result = run_horn(_store(turtle), certificate_dir=tmp_path)
    assert result.iterations >= 3, "the graph must run several rounds for this to bite"
    assert result.skipped_unserialisable == 1
    assert len(result.skipped_examples) == 1


def test_a_graph_that_only_produces_refusals_still_terminates(tmp_path):
    """The round list is what decides the fixpoint, not the refusal set. Testing
    the refusal set instead would spin for ever here."""
    s = ox.Store()
    s.load(
        b'<http://ex.org/a> <http://www.w3.org/2002/07/owl#sameAs> "lit" .',
        format=ox.RdfFormat.N_TRIPLES,
    )
    result = run_horn(s, certificate_dir=tmp_path)
    assert result.fixpoint_reached is True
    assert result.iterations == 1
    assert result.derived_triples == 0
    # The refusal is still counted and still reported, so a reader is never left
    # to conclude that the table simply did not fire.
    assert result.skipped_unserialisable == 1


def test_named_graphs_are_not_asserted_by_default(tmp_path):
    s = _store()
    s.add(
        ox.Quad(
            ox.NamedNode("http://ex.org/x"),
            ox.NamedNode(f"{RDFS}subClassOf"),
            ox.NamedNode("http://ex.org/y"),
            ox.NamedNode("http://ex.org/other"),
        )
    )
    result = run_horn(s, certificate_dir=tmp_path)
    assert result.asserted_graphs == ["default"]
    assert "http://ex.org/x" not in (tmp_path / "asserted.tsv").read_text("utf-8")


def test_asking_for_every_graph_reports_which_ones_were_asserted(tmp_path):
    s = _store()
    s.add(
        ox.Quad(
            ox.NamedNode("http://ex.org/x"),
            ox.NamedNode(f"{RDFS}subClassOf"),
            ox.NamedNode("http://ex.org/y"),
            ox.NamedNode("http://ex.org/other"),
        )
    )
    result = run_horn(s, certificate_dir=tmp_path, graphs="all")
    assert result.asserted_graphs == ["<http://ex.org/other>", "default"]
    assert "http://ex.org/x" in (tmp_path / "asserted.tsv").read_text("utf-8")


def test_a_previous_materialisation_can_never_be_asserted(tmp_path):
    """The defect this package exists not to have. A certificate whose asserted
    set contains a previous run's conclusions certifies entailment from a graph
    nobody asserted, and the checker is TOLD what the assertions are, so it
    cannot notice."""
    s = _store()
    s.add(
        ox.Quad(
            ox.NamedNode("http://ex.org/a"),
            ox.NamedNode("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            ox.NamedNode("http://ex.org/E"),
            ox.NamedNode(INFERRED_GRAPH),
        )
    )
    with pytest.raises(AssertedGraphError, match="DERIVED"):
        run_horn(s, certificate_dir=tmp_path, graphs="all")
    with pytest.raises(AssertedGraphError):
        run_horn(s, certificate_dir=tmp_path, graphs=[INFERRED_GRAPH])
    # and the default is unaffected, because it never looked at that graph
    assert run_horn(s, certificate_dir=tmp_path).asserted_graphs == ["default"]


def test_a_nonsense_graph_selector_is_refused(tmp_path):
    with pytest.raises(ValueError, match="neither 'default' nor 'all'"):
        run_horn(_store(), certificate_dir=tmp_path, graphs="everything")
    with pytest.raises(ValueError, match="selects nothing"):
        run_horn(_store(), certificate_dir=tmp_path, graphs=[])


EMIT = """
import sys, pathlib
import pyoxigraph as ox
from open_ontologies_lite.horn.reason import run_horn
s = ox.Store()
s.load(pathlib.Path(sys.argv[1]).read_bytes(), format=ox.RdfFormat.TURTLE)
run_horn(s, certificate_dir=sys.argv[2])
sys.stdout.buffer.write(pathlib.Path(sys.argv[2], 'horn.tsv').read_bytes())
"""


def test_the_bytes_do_not_depend_on_the_hash_seed(tmp_path):
    """Without `sorted(known)` at the top of each round the candidate order comes
    off a set, and Python randomises string hashing per process."""
    import open_ontologies_lite
    import pyoxigraph

    turtle = tmp_path / "g.ttl"
    turtle.write_bytes(CHAIN.encode("utf-8"))
    pythonpath = os.pathsep.join(
        [
            str(__import__("pathlib").Path(open_ontologies_lite.__file__).parents[1]),
            str(__import__("pathlib").Path(pyoxigraph.__file__).parents[1]),
        ]
    )
    outputs = []
    for seed in ("0", "1", "12345"):
        env = dict(os.environ, PYTHONHASHSEED=seed, PYTHONPATH=pythonpath)
        out = tmp_path / f"cert{seed}"
        proc = subprocess.run(
            [sys.executable, "-c", EMIT, str(turtle), str(out)],
            capture_output=True,
            env=env,
        )
        assert proc.returncode == 0, proc.stderr.decode()
        outputs.append(proc.stdout)
    assert outputs[0] == outputs[1] == outputs[2]
    assert outputs[0] != b""


def test_naming_a_graph_explicitly_asserts_that_graph_and_only_it(tmp_path):
    """The explicit-list path builds `NamedNode`s out of strings a caller wrote,
    which is the one place in this module where a user string becomes a term. It
    accepts a graph either bare or in its N-Triples spelling, because anyone
    reading `asserted_graphs` out of a previous report will paste the spelled
    form straight back in."""
    s = _store()
    other = ox.NamedNode("http://ex.org/other")
    s.add(
        ox.Quad(
            ox.NamedNode("http://ex.org/x"),
            ox.NamedNode(f"{RDFS}subClassOf"),
            ox.NamedNode("http://ex.org/y"),
            other,
        )
    )
    for n, spelling in enumerate(("http://ex.org/other", "<http://ex.org/other>")):
        out = tmp_path / f"run{n}"
        result = run_horn(s, certificate_dir=out, graphs=[spelling])
        assert result.asserted_graphs == ["<http://ex.org/other>"]
        assert result.asserted_triples == 1
        text = (out / "asserted.tsv").read_text("utf-8")
        assert "http://ex.org/x" in text
        assert "http://ex.org/a>" not in text, "the default graph leaked in"


def test_naming_the_default_graph_in_a_list_selects_the_default_graph(tmp_path):
    result = run_horn(_store(), certificate_dir=tmp_path, graphs=["default"])
    assert result.asserted_graphs == ["default"]
    assert result.asserted_triples == 5
