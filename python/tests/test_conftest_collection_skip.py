"""`OO_REQUIRE_FIXTURES=1` fails a run in which a module skipped at COLLECTION.

A module-level skip happens before any test item exists, so only
`pytest_collectreport` sees it. `conftest.py` used to rewrite the report there,
and that did not reach the exit status: one passing test plus one such module
gave `1 passed, 1 skipped` and exit 0 under the variable. Four files in the
strict leg of the CI `python` job skip this way (`test_shacl.py`,
`test_shacl_vacuous.py`, `test_align.py`, `test_vocab_check.py`), so that leg
stayed green if their extras went missing.

Each test runs pytest in a subprocess, on a throwaway directory that holds a
COPY of this suite's own `conftest.py`. The hook under test is the shipped one,
not a restatement of it.
"""

import os
import shutil
import subprocess
import sys
from pathlib import Path

CONFTEST = Path(__file__).with_name("conftest.py")

PASSES = "def test_passes():\n    assert True\n"

SKIPS_AT_COLLECTION = (
    "import pytest\n"
    "pytest.importorskip('a_module_that_is_installed_nowhere')\n"
    "\n"
    "def test_never_runs():\n"
    "    assert True\n"
)


def _run(tmp_path, strict):
    shutil.copy(CONFTEST, tmp_path / "conftest.py")
    (tmp_path / "test_passes.py").write_text(PASSES)
    (tmp_path / "test_skips_at_collection.py").write_text(SKIPS_AT_COLLECTION)
    env = dict(os.environ)
    env.pop("OO_REQUIRE_FIXTURES", None)
    env.pop("PYTEST_ADDOPTS", None)
    if strict:
        env["OO_REQUIRE_FIXTURES"] = "1"
    return subprocess.run(
        [sys.executable, "-m", "pytest", "-q", "-p", "no:cacheprovider",
         "--rootdir", str(tmp_path), str(tmp_path)],
        cwd=tmp_path,
        env=env,
        capture_output=True,
        text=True,
    )


def test_a_skip_at_collection_fails_the_strict_run(tmp_path):
    r = _run(tmp_path, strict=True)
    assert r.returncode == 1, r.stdout + r.stderr
    assert "test_skips_at_collection.py" in r.stdout, r.stdout


def test_the_same_skip_passes_without_the_variable(tmp_path):
    """The control. A conftest that failed EVERY run would pass the test above."""
    r = _run(tmp_path, strict=False)
    assert r.returncode == 0, r.stdout + r.stderr
    assert "1 passed, 1 skipped" in r.stdout, r.stdout
