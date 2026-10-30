"""`OO_REQUIRE_FIXTURES=1` turns a skip into a failure here too.

The Rust suite has had this since `tests/common/mod.rs` was written: a test that
skips for want of a toolchain prints a `SKIPPED_FIXTURE:` marker, and any job
that is supposed to PROVIDE that toolchain sets `OO_REQUIRE_FIXTURES=1`, which
makes the skip fatal. The `lean` job runs nineteen legs that way, and
`w3c-shacl` one more. The Python suite had no equivalent of any kind: no
`conftest.py` existed, no test read the variable, and every `pytest.skip` and
`pytest.importorskip` in `python/tests/` was unconditional and unpromotable.

That mattered most where it hurt most. `test_horn_three_way_differential.py` is
the three-way differential — the Rust engine, this Python engine and the Lean
checker over the same documents — and the CI `python` job installed the `dev`
extra and nothing else: no `cargo build`, so no Rust engine; no elan, so no
`oo-horn`. The file's own preflight found neither and skipped every test in it.
A green tick on that job said nothing whatsoever about the differential, which
is the precise shape of failure this repository exists to attack: a gate that
passes by not running. Setting `OO_REQUIRE_FIXTURES=1` on the job would not have
helped, because nothing on this side read it.

The rule is deliberately GLOBAL rather than per-call-site. In Rust a skip has to
be written through `common::skip_unless` to be catchable, so a new one added by
hand is invisible to the convention; here every skip is caught, whatever raised
it — `pytest.skip`, a `skipif` marker, a module-level `pytest.importorskip` —
and a new one cannot be added without either providing the dependency or
declaring the skip deliberate.

DELIBERATE SKIPS. A test skipped because it is SLOW rather than because
something is missing is a different thing, and marking it `@pytest.mark.oo_opt_in`
exempts it. There is exactly one: the full-corpus differential, which takes
twelve minutes. The marker is greppable, which a reason string is not.
"""

import os

import pytest

# Same spelling and same semantics as tests/common/mod.rs:22 — the string must
# be exactly "1". Anything else, including "true" and "yes", is not set.
STRICT = os.environ.get("OO_REQUIRE_FIXTURES") == "1"

# Same marker the Rust helper prints, so one grep counts skips across both
# suites and the `build` job's existing counter needs no second pattern.
MARKER = "SKIPPED_FIXTURE"

OPT_IN = "oo_opt_in"


def pytest_configure(config):
    config.addinivalue_line(
        "markers",
        f"{OPT_IN}: this test is skipped by choice (cost, not a missing "
        f"dependency), so OO_REQUIRE_FIXTURES=1 leaves it skipped.",
    )


def _reason(report):
    """The skip reason, however pytest happens to have packed it."""
    longrepr = getattr(report, "longrepr", None)
    if isinstance(longrepr, tuple) and len(longrepr) == 3:
        return str(longrepr[2]).removeprefix("Skipped: ")
    return str(longrepr) if longrepr else "no reason given"


def _fail(report, nodeid, reason):
    print(f"{MARKER}: {nodeid}: {reason}", flush=True)
    if not STRICT:
        return
    report.outcome = "failed"
    report.longrepr = (
        f"{nodeid} skipped: {reason}\n"
        f"OO_REQUIRE_FIXTURES=1, so this is a failure, not a skip. This job is "
        f"supposed to provide what the test needs; provide it, or stop claiming "
        f"the test ran."
    )


@pytest.hookimpl(wrapper=True, trylast=True)
def pytest_runtest_makereport(item, call):
    report = yield
    if report.skipped and not hasattr(report, "wasxfail"):
        if item.get_closest_marker(OPT_IN) is None:
            _fail(report, item.nodeid, _reason(report))
    return report


_COLLECTION_SKIPS = []


def pytest_collectreport(report):
    """A module-level `importorskip` skips at COLLECTION, not at run time.

    `pytest_runtest_makereport` never sees those, so three of this suite's files
    — the two pyshacl ones and the hnswlib one — would have stayed exempt from a
    rule written only there.

    This hook can SEE such a skip and cannot FAIL it. Until this was changed it
    set `report.outcome = "failed"` here, through `_fail`, and that never
    reached the exit status. MEASURED on pytest 9.0.3 and 9.1.1, with one
    passing test and one module that skips at collection, under
    OO_REQUIRE_FIXTURES=1: `1 passed, 1 skipped`, exit 0. Marked `tryfirst`, the
    summary said `1 error` and the exit was still 0. Run with `-rs` or `-ra`,
    the terminal reporter crashed on the rewritten report. So the skip is
    recorded here, and `pytest_sessionfinish` fails the run.
    `test_conftest_collection_skip.py` holds this.
    """
    if report.skipped:
        reason = _reason(report)
        print(f"{MARKER}: {report.nodeid}: {reason}", flush=True)
        _COLLECTION_SKIPS.append(f"{report.nodeid}: {reason}")


@pytest.hookimpl(trylast=True)
def pytest_sessionfinish(session, exitstatus):
    """Where a skip at collection becomes a failure. See `pytest_collectreport`."""
    if not (STRICT and _COLLECTION_SKIPS):
        return
    print(
        f"\n{MARKER}: OO_REQUIRE_FIXTURES=1, so this is a failure, not a skip. "
        f"{len(_COLLECTION_SKIPS)} module(s) skipped at collection and none of "
        f"their tests ran:",
        flush=True,
    )
    for line in _COLLECTION_SKIPS:
        print(f"  {line}", flush=True)
    # A status that already says something went wrong is left as it is.
    if session.exitstatus == pytest.ExitCode.OK:
        session.exitstatus = pytest.ExitCode.TESTS_FAILED
