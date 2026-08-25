"""Guard the validation.json artifact: the panel this feeds exists to show a
SHACL run with zero focus nodes as NOT a pass. These tests pin that both the
data shape carries no violation count and that the zero-focus-node run is
never marked 'fails' being reinterpreted as a pass, or otherwise indistinct
from a genuine clean result.

demo/dcat_conformance_measurements.json is the single source of truth for
these figures, already pinned by demo/tests/test_dcat_conformance.py; this
file tests build_validation.py's own transform of it, not the measurements
themselves.
"""

import json

import demo.build_validation as bv


def test_as_published_run_is_undetermined_not_a_pass():
    payload = bv.build_validation()
    as_published = next(r for r in payload["runs"] if r["id"] == "as-published")
    assert as_published["focusNodes"] == 0
    assert as_published["verdict"] == "undetermined"
    assert as_published["verdict"] != "fails"
    # conformsRaw is allowed to be True (the honest raw reading of the SHACL
    # run), but nothing downstream is allowed to call that reading a pass:
    # there is no 'passes' verdict value anywhere in this artifact.
    assert as_published["conformsRaw"] is True
    assert "nothing" in as_published["reason"] or "matched" in as_published["reason"]


def test_bound_run_is_a_real_failure_distinct_from_the_vacuous_run():
    payload = bv.build_validation()
    bound = next(r for r in payload["runs"] if r["id"] == "schema-derived-binding")
    assert bound["focusNodes"] == 228
    assert bound["verdict"] == "fails"
    assert bound["conformsRaw"] is False


def test_no_run_carries_a_violation_count():
    # The repository's own position, stated in demo/README.md: no single
    # SHACL violation count is defensible (three legitimate methods over
    # identical inputs give 178 / 272 / 147, and a fourth, 287, is already
    # public elsewhere). Keeping the field out of this artifact means the
    # client cannot render one even by accident.
    payload = bv.build_validation()
    for run in payload["runs"]:
        keys = set(run.keys())
        assert not (keys & {"violations", "violationCount", "violation_count", "bySeverity"})


def test_verdict_type_has_no_pass_value():
    payload = bv.build_validation()
    verdicts = {r["verdict"] for r in payload["runs"]}
    assert verdicts <= {"undetermined", "fails"}
    assert "passes" not in verdicts
    assert "pass" not in verdicts


def test_guards_against_a_premise_that_no_longer_holds(tmp_path, monkeypatch):
    """If the as-published corpus ever measures a non-zero focus node count
    (e.g. GSA/dcat-us republishes the deleted context), the premise this
    panel demonstrates no longer holds, and build_validation() must refuse
    to produce a silently misleading artifact rather than ship stale data."""
    broken = tmp_path / "measurements.json"
    real = json.loads(bv.MEASUREMENTS.read_text(encoding="utf-8"))
    real["shacl"]["legacyShapesOverPublishedCorpus"]["focusNodes"] = 1
    broken.write_text(json.dumps(real), encoding="utf-8")

    monkeypatch.setattr(bv, "MEASUREMENTS", broken)
    try:
        bv.build_validation()
        assert False, "expected build_validation() to refuse a broken premise"
    except SystemExit as e:
        assert "vacuous pass" in str(e)


def test_output_matches_committed_artifact():
    """demo/precomputed/validation.json is committed, like the other
    precomputed parts; this proves it is not stale relative to what
    build_validation.py currently produces from the committed measurements
    file (the CI job in .github/workflows/demo-artifacts.yml catches drift
    at the bundle.json level; this catches it one layer lower)."""
    committed = json.loads((bv.ROOT / "demo" / "precomputed" / "validation.json").read_text(encoding="utf-8"))
    fresh = bv.build_validation()
    assert committed == fresh
