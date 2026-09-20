"""AlignmentIndex tests. Skipped unless the [align] extra (hnswlib) is installed.

Run: pip install -e ".[align,dev]" && pytest -q
"""
import subprocess
import sys

import pytest


def _first_import_that_dies() -> str | None:
    """Import numpy, then hnswlib, each in its own SUBPROCESS, and name the
    first one that kills the interpreter.

    Two, not one, because hnswlib imports numpy: a probe that imports only
    hnswlib blames hnswlib for a numpy crash, and the fix for each is
    different. numpy dispatches on the CPU at run time and should not do
    this; hnswlib compiles with -march=native and does it whenever the wheel
    was built on a newer CPU than the one running it.
    """
    for module in ("numpy", "hnswlib"):
        r = subprocess.run([sys.executable, "-c", f"import {module}"], capture_output=True)
        if r.returncode != 0:
            return f"{module} (exit {r.returncode}{', SIGILL' if r.returncode in (-4, 132) else ''})"
    return None


def _import_survives_in_a_subprocess() -> bool:
    """Import `hnswlib` in a SUBPROCESS before importing it here.

    A wheel compiled for a newer CPU than the machine provides raises SIGILL on
    import, and no `try`/`except` reaches that: the interpreter dies during
    COLLECTION and takes every other test in the run with it. The job then
    reports "Fatal Python error: Illegal instruction" and a core dump, with no
    indication of which import did it. That happened twice on 19 September 2026,
    on branches that touched no Python.

    CI builds hnswlib from source so this should not fire. It is here so that if
    it ever does, the run says which library and why instead of dying.
    """
    return subprocess.run(
        [sys.executable, "-c", "import hnswlib"],
        capture_output=True,
    ).returncode == 0


_DEAD = _first_import_that_dies()
if _DEAD is not None:
    pytest.fail(
        f"{_DEAD} crashes the interpreter on import: a wheel built for a CPU this "
        "machine does not have. For hnswlib, build it here without -march=native: "
        "HNSWLIB_NO_NATIVE=1 pip install --no-binary hnswlib --force-reinstall hnswlib",
        pytrace=False,
    )

hnswlib = pytest.importorskip("hnswlib")  # noqa: F841

from open_ontologies_lite import AlignmentIndex  # noqa: E402


def test_nearest_neighbour_recovers_match():
    # Three orthogonal-ish concept vectors; query close to the first.
    idx = AlignmentIndex(dim=3)
    idx.add("flw:PC-BAK", [1.0, 0.0, 0.0])
    idx.add("flw:PC-DRY", [0.0, 1.0, 0.0])
    idx.add("flw:PC-VEG", [0.0, 0.0, 1.0])
    idx.build()
    out = idx.query([0.9, 0.1, 0.0], k=2)
    assert out[0].id == "flw:PC-BAK"
    assert out[0].score > out[1].score  # best candidate ranks first
    assert 0.0 <= out[0].score <= 1.0


def test_dim_mismatch_raises():
    idx = AlignmentIndex(dim=4)
    with pytest.raises(ValueError):
        idx.add("x", [1.0, 2.0, 3.0])


def test_len_and_empty_query():
    idx = AlignmentIndex(dim=2)
    assert len(idx) == 0
    assert idx.query([1.0, 0.0]) == []
    idx.add_many([("a", [1.0, 0.0]), ("b", [0.0, 1.0])])
    assert len(idx) == 2
