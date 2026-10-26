"""`tools/shacl_differential.py` cannot go green on nothing, and its planted
controls bite (#320).

Before #320 the script exited 0 whenever it found no FALSE_CLEAN, which it also
did when it found no cases, when pyshacl was not installed (every pair came back
ERROR from the import inside the run), and when every pair was capped out. The
CI job "SHACL differential vs pyshacl" was therefore green on a run that
compared nothing. Each test below fails against that version of the script.

The engine here is a stand-in: a script that answers `load` and `shacl` the way
the release binary does, by running pyshacl and printing the result in the
engine's JSON shape. A faithful stand-in is the right fixture for testing the
HARNESS, because every disagreement it shows is then the harness's own doing,
and it keeps this file from needing a release build. The real binary is what
the CI job drives.
"""

import importlib.util
import json
import os
import pathlib
import subprocess
import sys
import textwrap

import pytest

pytest.importorskip("pyshacl")

REPO = pathlib.Path(__file__).resolve().parents[2]
TOOL = REPO / "tools" / "shacl_differential.py"

FAKE_ENGINE = textwrap.dedent(
    '''\
    import json, pathlib, sys
    from pyshacl import validate
    from rdflib import BNode, Graph, Namespace, URIRef

    SH = Namespace("http://www.w3.org/ns/shacl#")
    args = sys.argv[1:]
    store = pathlib.Path(args[args.index("--data-dir") + 1])
    cmd, target = args[args.index("--data-dir") + 2], args[args.index("--data-dir") + 3]
    if cmd == "load":
        with open(store / "loaded.txt", "a") as fh:
            fh.write(target + "\\n")
        sys.exit(0)
    data = Graph()
    for f in (store / "loaded.txt").read_text().split():
        data.parse(f)
    shapes = Graph().parse(target)
    conforms, results, _ = validate(data, shacl_graph=shapes)

    def term(t):
        if t is None:
            return None
        if isinstance(t, BNode):
            return "_:" + str(t)
        return str(t) if isinstance(t, URIRef) else t.n3()

    violations = []
    for r in results.subjects(SH.resultSeverity, None):
        shape = results.value(r, SH.sourceShape)
        parent = next(iter(shapes.subjects(SH.property, shape)), None)
        violations.append({
            "focus_node": term(results.value(r, SH.focusNode)),
            "result_path": term(results.value(r, SH.resultPath)),
            "severity": str(results.value(r, SH.resultSeverity)).rsplit("#", 1)[-1],
            "source_shape": term(shape),
            "node_shape": term(parent) if parent is not None else term(shape),
            "source_constraint_component": term(results.value(r, SH.sourceConstraintComponent)),
        })
    print(json.dumps({"conforms": conforms, "violations": violations}))
    '''
)

DATA = """\
@prefix ex: <http://example.org/t#> .
ex:a a ex:Thing .
"""
SHAPES = """\
@prefix ex: <http://example.org/t#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
ex:S a sh:NodeShape ; sh:targetClass ex:Thing ; sh:property [ sh:path ex:p ; sh:minCount 1 ] .
"""


@pytest.fixture
def engine(tmp_path):
    exe = tmp_path / "fake-engine"
    exe.write_text(f"#!{sys.executable}\n" + FAKE_ENGINE)
    exe.chmod(0o755)
    return exe


@pytest.fixture
def corpus(tmp_path):
    root = tmp_path / "corpus"
    (root / "one" / "shacl").mkdir(parents=True)
    (root / "one" / "data").mkdir()
    (root / "one" / "shacl" / "s.ttl").write_text(SHAPES)
    (root / "one" / "data" / "d.ttl").write_text(DATA)
    return root


def run(args, engine, extra_env=None):
    env = dict(os.environ, OPEN_ONTOLOGIES_BIN=str(engine), **(extra_env or {}))
    return subprocess.run([sys.executable, str(TOOL), *args], capture_output=True, text=True, env=env)


def load_tool(monkeypatch, engine):
    monkeypatch.setenv("OPEN_ONTOLOGIES_BIN", str(engine))
    spec = importlib.util.spec_from_file_location("shacl_differential_under_test", TOOL)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def test_a_faithful_engine_is_green_and_the_receipt_is_written(engine, corpus, tmp_path):
    out = tmp_path / "r.json"
    p = run(["--discover", str(corpus), "--json", str(out)], engine)
    assert p.returncode == 0, p.stdout + p.stderr
    r = json.loads(out.read_text())
    assert r["status"] == "GREEN"
    assert r["counts"] == {"AGREE": 1}
    import pyshacl

    assert r["receipt"]["pyshacl"] == pyshacl.__version__
    assert r["receipt"]["engine"]["sha256"]
    assert sorted(r["receipt"]["inputs"]) == sorted(
        str(p) for p in [corpus / "one" / "shacl" / "s.ttl", corpus / "one" / "data" / "d.ttl"]
    )
    assert all(len(h) == 64 for h in r["receipt"]["inputs"].values())
    assert [c["ok"] for c in r["controls"]] == [True] * 5, r["controls"]


def test_empty_discovery_is_not_measured(engine, tmp_path):
    empty = tmp_path / "empty"
    empty.mkdir()
    p = run(["--discover", str(empty)], engine)
    assert p.returncode == 2, p.stdout + p.stderr
    assert "NOT MEASURED" in p.stdout


def test_missing_pyshacl_is_not_measured(engine, corpus, tmp_path):
    # A package named pyshacl that refuses to import, first on the path: the
    # interpreter sees pyshacl as absent, exactly as on a runner without it.
    stub = tmp_path / "stub" / "pyshacl"
    stub.mkdir(parents=True)
    (stub / "__init__.py").write_text("raise ImportError('pyshacl withheld by the test')\n")
    p = run(["--discover", str(corpus)], engine, {"PYTHONPATH": str(stub.parent)})
    assert p.returncode == 2, p.stdout + p.stderr
    assert "NOT MEASURED" in p.stdout and "pyshacl" in p.stdout


def test_a_pin_mismatch_is_not_measured(engine, corpus):
    p = run(["--discover", str(corpus), "--pyshacl-version", "0.0.0"], engine)
    assert p.returncode == 2, p.stdout + p.stderr


def test_fewer_run_than_discovered_is_not_measured(engine, corpus):
    p = run(["--discover", str(corpus)], engine, {"SHACL_DIFF_MAX_BYTES": "1"})
    assert p.returncode == 2, p.stdout + p.stderr
    assert "were not run" in p.stdout


def test_a_comparator_blind_to_results_fails_on_its_controls(engine, corpus, monkeypatch, capsys):
    """A classifier that compares the verdict and nothing else would call every
    corpus pair here AGREE. The planted controls are what catch it."""
    mod = load_tool(monkeypatch, engine)
    real = mod.classify

    def verdict_only(ours, theirs):
        verdict, detail = real(ours, theirs)
        return ("AGREE", detail) if verdict == "PARTIAL" else (verdict, detail)

    monkeypatch.setattr(mod, "classify", verdict_only)
    assert mod.main(["--discover", str(corpus)]) == 1
    out = capsys.readouterr().out
    for name in ("one-result-dropped", "severity-changed", "source-shape-changed", "component-changed"):
        assert f"control `{name}` came out AGREE" in out, out


def test_a_comparator_that_never_agrees_fails_on_its_controls(engine, corpus, monkeypatch, capsys):
    mod = load_tool(monkeypatch, engine)
    monkeypatch.setattr(mod, "classify", lambda ours, theirs: ("PARTIAL", "always"))
    assert mod.main(["--discover", str(corpus)]) == 1
    assert "control `must-agree` came out PARTIAL" in capsys.readouterr().out


def test_w3c_arbitration_names_the_wrong_side(engine, monkeypatch):
    """`arbitrate` is what turns a disagreement on a W3C test into a verdict
    about one engine. Driven directly, on hand-built reports, so each branch is
    shown to be reachable and to say what it claims."""
    mod = load_tool(monkeypatch, engine)
    right = {"conforms": False, "keys": {("f", None, "sev", "shape", "comp")}}
    wrong = {"conforms": True, "keys": set()}

    # Both engines agree and the Working Group does not: the blind spot of a
    # two-engine differential, so it blocks.
    assert mod.arbitrate("AGREE", "", wrong, wrong, right, None)[0] == "AGREE_AGAINST_W3C"
    assert "AGREE_AGAINST_W3C" in mod.BLOCKING
    # We match the expected report and pyshacl does not.
    assert mod.arbitrate("FALSE_ALARM", "", right, wrong, right, None)[0] == "ORACLE_WRONG"
    # pyshacl is right and we are clean where we should not be: blocking unless
    # the conformance ratchet already holds this test as failing.
    assert mod.arbitrate("FALSE_CLEAN", "", wrong, right, right, "PASS")[0] == "FALSE_CLEAN"
    assert mod.arbitrate("FALSE_CLEAN", "", wrong, right, right, None)[0] == "FALSE_CLEAN"
    assert mod.arbitrate("FALSE_CLEAN", "", wrong, right, right, "FAIL")[0] == "KNOWN_FALSE_CLEAN"
    assert "KNOWN_FALSE_CLEAN" not in mod.BLOCKING


def test_horn_differential_is_not_measured_when_every_case_is_capped(tmp_path, monkeypatch, capsys):
    """The same hole in tools/horn_differential.py: its zero-cases check ran
    before the size cap, so a corpus capped out entirely compared nothing and
    exited 0. The preflight is stubbed because the hole is in what happens
    after it."""
    (tmp_path / "a.ttl").write_text(DATA)
    spec = importlib.util.spec_from_file_location(
        "horn_differential_under_test", REPO / "tools" / "horn_differential.py"
    )
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    monkeypatch.setattr(mod, "preflight", lambda: [])
    argv = ["horn_differential.py", "--root", str(tmp_path), "--max-bytes", "0", "--no-coverage"]
    monkeypatch.setattr(sys, "argv", argv)
    assert mod.main() == 2
    assert "compared nothing" in capsys.readouterr().out
