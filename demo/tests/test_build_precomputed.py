"""Guard against demo/build_precomputed.py silently emptying findings.json.

The demonstration's five conformance findings come from
demo/dcat_conformance.py and are curated by hand into
demo/precomputed/findings.json (see demo/README.md, "The conformance
finding"). An earlier version of build_precomputed.py's main() called
build_findings() -- the old disjointness/provenance scan, which returns zero
findings on this corpus -- and wrote its result over findings.json
unconditionally. Because `make demo` also regenerates bundle.json and
MANIFEST.sha256 from whatever is in demo/precomputed/, running the documented
command would replace the real findings with an empty list and re-sign the
emptied result, with CI still green.

This test proves main() cannot do that: given a findings.json already
populated with real conformance findings, running main() end to end must
leave it byte-for-byte unchanged.
"""

import json
import sys

import demo.build_precomputed as bp


def test_main_never_touches_a_populated_findings_file(tmp_path, monkeypatch):
    populated = json.dumps([
        {"id": "conformance-1-readme-vs-examples", "subject": "dcat-us:profile",
         "kind": "conformance", "claims": [{"document": "profile-readme.md",
                                            "predicate": "claims", "object": "..."}]},
    ])
    findings_path = tmp_path / "findings.json"
    findings_path.write_text(populated)

    # main() only needs STORE and BIN to exist as paths (its precondition
    # check); it does not need a real engine binary or derived store for this
    # test, because the SPARQL calls that would use them are stubbed out.
    monkeypatch.setattr(bp, "STORE", tmp_path)
    monkeypatch.setattr(bp, "BIN", tmp_path)
    monkeypatch.setattr(bp, "sparql", lambda *_a, **_k: [])
    monkeypatch.setattr(sys, "argv", ["build_precomputed.py", "--out", str(tmp_path)])

    bp.main()

    assert findings_path.read_text() == populated, (
        "build_precomputed.py must never write findings.json -- conformance "
        "findings are hand-curated from demo/dcat_conformance.py's measurements"
    )


def test_build_findings_is_not_called_from_main(tmp_path, monkeypatch):
    """A second, more direct guard: build_findings() must not be invoked at all."""
    calls = []
    monkeypatch.setattr(bp, "build_findings", lambda: calls.append(1) or [])
    monkeypatch.setattr(bp, "STORE", tmp_path)
    monkeypatch.setattr(bp, "BIN", tmp_path)
    monkeypatch.setattr(bp, "sparql", lambda *_a, **_k: [])
    monkeypatch.setattr(sys, "argv", ["build_precomputed.py", "--out", str(tmp_path)])

    bp.main()

    assert calls == [], "build_findings() must not be called from main()"
