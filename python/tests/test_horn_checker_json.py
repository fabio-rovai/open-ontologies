"""Checker transport fixtures exercise subprocess output, not Lean proofs."""

import json
import sys

import pytest

from open_ontologies_lite.horn.certify import check_horn


def _checker_response(tmp_path, monkeypatch, output, code=0):
    # The configured Python executable runs our task-owned `check` script.
    # This uses the real subprocess transport without requiring a Lean binary.
    (tmp_path / "check").write_text(
        "import sys\nsys.stdout.write(" + repr(output) + ")\nraise SystemExit(" + str(code) + ")\n",
        encoding="utf-8",
    )
    monkeypatch.chdir(tmp_path)
    return check_horn(tmp_path, checker=sys.executable)


@pytest.mark.parametrize("payload", [[], None, True, 7, "unexpected"], ids=["array", "null", "boolean", "number", "string"])
@pytest.mark.parametrize("code", [0, 1], ids=["exit-zero", "exit-one"])
def test_non_object_checker_json_is_unchecked(tmp_path, monkeypatch, payload, code):
    report = _checker_response(tmp_path, monkeypatch, json.dumps(payload), code)
    assert report["status"] == "unchecked_error"
    assert report["checked"] is False
    assert report["verdict"] is None
    assert report["pronounced_by"] is None
    assert "JSON output is not an object" in report["error"]


@pytest.mark.parametrize(
    "output,code,status,checked",
    [
        ("not JSON", 0, "unchecked_error", False),
        ('{"ok": true}', 0, "unchecked_error", False),
        ('{"ok": false}', 1, "rejected", True),
        ('{"ok": false}', 3, "unchecked_error", False),
        ("", 2, "unreadable", False),
    ],
    ids=["malformed-json", "missing-fields", "rejection", "unexpected-exit", "unreadable"],
)
def test_existing_checker_outcomes_are_preserved(tmp_path, monkeypatch, output, code, status, checked):
    report = _checker_response(tmp_path, monkeypatch, output, code)
    assert report["status"] == status
    assert report["checked"] is checked
    assert report["verdict"] is None
