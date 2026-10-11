"""The executable we locate must be the executable we launch."""
import sys
import pytest
from open_ontologies_lite.horn import certify
from open_ontologies_lite.horn.certify import check_horn, find_checker

@pytest.mark.parametrize("source", ["explicit", "environment", "path", "absolute"])
def test_located_checker_executes_from_selected_location(tmp_path, monkeypatch, source):
    monkeypatch.chdir(tmp_path)
    # PATH is lower priority than a built repository checker. Isolate discovery
    # so this transport fixture does not accidentally invoke the real Lean binary.
    monkeypatch.setattr(certify, "_repo_candidate", lambda name: None)
    binary = tmp_path / "oo-horn"
    try:
        binary.symlink_to(sys.executable)
    except (OSError, NotImplementedError) as exc:
        pytest.skip(f"executable symlink fixture unavailable: {exc}")
    (tmp_path / "check").write_text('import json,sys; print(json.dumps({"ok": False})); sys.exit(1)', encoding="utf-8")
    # Python runs the owned script named "check" with the checker's actual argv.
    # This exercises transport, not the Lean certification algorithm.
    explicit = None
    monkeypatch.delenv("OO_HORN", raising=False)
    monkeypatch.setenv("PATH", str(tmp_path / "unrelated-bin"))
    if source == "explicit":
        explicit = "./oo-horn"
    elif source == "environment":
        monkeypatch.setenv("OO_HORN", "./oo-horn")
    elif source == "path":
        monkeypatch.setenv("PATH", ".")
    else:
        explicit = str(binary)
    located = find_checker(explicit=explicit)
    assert located.is_file()
    report = check_horn("certificate", checker=explicit)
    assert report["status"] == "rejected", report
    assert report["checked"] is True and report["verdict"] is None
