"""A result the Lean checker never saw must carry no verdict, in its own words.

This is the C2/C3 gate and the one CI actually runs, because CI installs `[dev]`
and nothing else: no elan, no `lake`, no `oo-horn`. Every green tick on this
repository's Python job is therefore a statement about the UNCHECKED path, and
this file is what makes that statement worth anything.

The failure being guarded against is not a crash. It is a report that looks
successful: a derivation count, a certificate directory, and a verdict word
sitting beside them that no checker ever pronounced. That is assurance
laundering with the engine's own output as the source, and the only defence that
survives refactoring is a test asserting that the verdict word does not appear
ANYWHERE in the report when nothing was checked, not merely that some `checked`
flag is False.

The first test simulates the real shape of the absence: a package with no
repository above it, which is what `pip install open-ontologies-lite` gives, and
an empty PATH. It copies the installed package to a temporary directory for
exactly that reason, since the repo probe in `find_checker` would otherwise find
the `lake build` output sitting two levels above this checkout and the test would
prove nothing about a wheel.
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import open_ontologies_lite

SCRIPT = """
import json, sys, tempfile
import pyoxigraph as ox
from open_ontologies_lite import OntologyEngine

e = OntologyEngine()
e.load('<http://ex.org/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://ex.org/A> .\\n'
       '<http://ex.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://ex.org/B> .',
       'ntriples')
report = e.reason_horn(tempfile.mkdtemp(){extra})
print(json.dumps(report))
"""


def _run(script: str, env: dict) -> dict:
    proc = subprocess.run(
        [sys.executable, "-c", script], capture_output=True, text=True, env=env
    )
    assert proc.returncode == 0, proc.stderr
    return json.loads(proc.stdout)


def test_a_run_the_checker_never_saw_carries_no_verdict(tmp_path):
    """A wheel install: no repository above the package, nothing on PATH."""
    installed = Path(open_ontologies_lite.__file__).parent
    staged = tmp_path / "site" / "open_ontologies_lite"
    shutil.copytree(installed, staged, ignore=shutil.ignore_patterns("__pycache__"))
    assert not (tmp_path / "lean" / "lakefile.toml").exists()

    env = dict(os.environ)
    env["PATH"] = ""
    env.pop("OO_HORN", None)
    env["PYTHONPATH"] = str(tmp_path / "site") + os.pathsep + _pyoxigraph_dir()

    report = _run(SCRIPT.format(extra=""), env)
    check = report["check"]
    assert check["status"] == "unchecked_no_checker"
    assert check["checked"] is False
    assert check["verdict"] is None
    assert check["pronounced_by"] is None
    assert "oo-horn" in check["warning"]
    assert "lake build" in check["warning"]
    assert "OO_HORN" in check["how_to_get_it"]
    # The load-bearing one. It catches the relativised verdict word too, since
    # that word contains this one, and it catches a verdict smuggled into any
    # field of the report rather than only into `check["verdict"]`.
    assert "entailed" not in json.dumps(report)


def test_naming_a_checker_that_is_not_there_is_not_silently_replaced(tmp_path):
    """An explicit path does not fall through to a search. A caller who named a
    binary meant that binary, and a report about a different checker found on
    PATH would be a report about something the caller did not run."""
    env = dict(os.environ)
    env["PYTHONPATH"] = str(Path(open_ontologies_lite.__file__).parents[1])
    missing = tmp_path / "missing-oo-horn"
    report = _run(
        SCRIPT.format(extra=f", checker={str(missing)!r}"), env
    )
    check = report["check"]
    assert check["status"] == "unchecked_no_checker"
    assert check["checked"] is False
    assert check["verdict"] is None
    assert str(missing) in check["how_to_get_it"]
    assert "entailed" not in json.dumps(report)


def test_check_false_writes_a_certificate_and_claims_nothing():
    env = dict(os.environ)
    env["PYTHONPATH"] = str(Path(open_ontologies_lite.__file__).parents[1])
    report = _run(SCRIPT.format(extra=", check=False"), env)
    check = report["check"]
    assert check["status"] == "not_requested"
    assert check["checked"] is False
    assert check["verdict"] is None
    assert check["pronounced_by"] is None
    assert "NOT checked" in check["warning"]
    assert "entailed" not in json.dumps(report)
    # The certificate is still on disk and the report says how to check it later.
    assert "oo-horn check" in check["check_with"]


def test_the_engine_reports_the_certificate_without_a_checker_installed(tmp_path):
    """The library half must WORK with no binary, not merely refuse politely."""
    from open_ontologies_lite import OntologyEngine

    e = OntologyEngine()
    e.load(
        "<http://ex.org/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
        "<http://ex.org/A> .\n"
        "<http://ex.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> "
        "<http://ex.org/B> .",
        "ntriples",
    )
    report = e.reason_horn(str(tmp_path), checker="/nonexistent/oo-horn")
    assert report["derived_triples"] == 1
    assert (tmp_path / "horn.tsv").read_bytes().count(b"\n") == 1
    assert report["check"]["checked"] is False


def _pyoxigraph_dir() -> str:
    import pyoxigraph

    return str(Path(pyoxigraph.__file__).parents[1])
