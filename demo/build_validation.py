#!/usr/bin/env python3
"""
Build validation.json, the sixth artifact demo/bundle_fixtures.py merges
into bundle.json (PARTS gains "validation" alongside corpus, graph,
findings, chat, compare).

This is the surface the pivot recorded in .superpowers/sdd/progress.md could
not put in the web replay, because showing it required the live engine: a
SHACL validation that reports conforms=true over zero focus nodes, meaning
nothing was actually checked. src/shacl.rs (around lines 552-567) refuses to
call that a pass -- it reports conforms: null with a warning that the shapes
selected nothing to check -- and this script carries the same two runs a
viewer would need to see that refusal for themselves, read straight out of
the measurements demo/dcat_conformance.py already produced and
demo/tests/test_dcat_conformance.py already pins (demo/dcat_conformance_
measurements.json):

  - "as published": the corpus exactly as GSA/dcat-us publishes it, checked
    against the profile's own recovered SHACL shapes (recovered-shapes.ttl,
    unchanged). shacl.legacyShapesOverPublishedCorpus: conforms=true,
    0 focus nodes, 0 of 34 target classes matched.
  - "schema-derived binding applied": the same corpus, the same unchanged
    shapes file, with the schema-derived RDF binding demo/dcat_conformance.py
    builds from the JSON Schema's own _oldDocs residue (the "observed"
    variant, which additionally relaxes terms the corpus itself publishes as
    prose rather than IRIs).
    shacl.legacyShapesOverObservedBoundCorpus: conforms=false,
    228 focus nodes, 24 of 34 target classes matched.

Deliberately excludes every violation count. Three legitimate measurement
methods over identical inputs give 178 / 272 / 147 violations, and a fourth
figure (287) is already public in case-studies/dcat-us-binding/README.md;
demo/README.md's own stated position is that none of the four is defensible
as "the" figure. Leaving violation counts out of this artifact means the
client cannot render one even by accident -- see
studio/src/lib/validation-source.ts and studio/src/components/
ValidationPanel.tsx, neither of which is given a number to show.

Usage:
    python3 demo/build_validation.py --out demo/precomputed
"""
import argparse
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
MEASUREMENTS = ROOT / "demo" / "dcat_conformance_measurements.json"
SHAPES_FILE = "recovered-shapes.ttl"


def build_validation() -> dict:
    m = json.loads(MEASUREMENTS.read_text(encoding="utf-8"))
    shacl = m["shacl"]
    published = shacl["legacyShapesOverPublishedCorpus"]
    bound = shacl["legacyShapesOverObservedBoundCorpus"]

    # The whole point of this panel depends on this premise. Fail loudly at
    # build time rather than silently shipping a validation.json that no
    # longer demonstrates a vacuous pass.
    if published["focusNodes"] != 0 or published["conforms"] is not True:
        raise SystemExit(
            "legacyShapesOverPublishedCorpus no longer measures a vacuous pass "
            f"(focusNodes={published['focusNodes']}, conforms={published['conforms']}); "
            "the premise this panel demonstrates no longer holds against "
            "demo/dcat_conformance_measurements.json. Re-check before regenerating."
        )
    if bound["focusNodes"] == 0 or bound["conforms"] is not False:
        raise SystemExit(
            "legacyShapesOverObservedBoundCorpus no longer measures a real failure "
            f"(focusNodes={bound['focusNodes']}, conforms={bound['conforms']}); "
            "re-check before regenerating."
        )

    return {
        "shapesFile": SHAPES_FILE,
        "commit": m["commit"],
        "measured": m["measured"],
        "runs": [
            {
                "id": "as-published",
                "label": "Corpus as published",
                "corpusDescription": (
                    "115 GSA/dcat-us good examples, exactly as published, checked "
                    "against the profile's own recovered SHACL shapes"
                ),
                "dataTriples": published["dataTriples"],
                "focusNodes": published["focusNodes"],
                "matchedClassCount": published["matchedClassCount"],
                "targetClassCount": published["targetClassCount"],
                "conformsRaw": published["conforms"],
                "verdict": "undetermined",
                "reason": (
                    "0 of {target} target classes in the shapes file matched anything "
                    "in the data, so nothing was checked. Reporting conforms=true here "
                    "would be the same claim the engine's own SHACL tool refuses to make "
                    "(src/shacl.rs): a validator that selects nothing to check has not "
                    "determined conformance."
                ).format(target=published["targetClassCount"]),
            },
            {
                "id": "schema-derived-binding",
                "label": "Corpus with schema-derived binding applied",
                "corpusDescription": (
                    "the same 115 examples, the same unchanged shapes file, with the "
                    "schema-derived RDF binding applied (demo/dcat_conformance.py)"
                ),
                "dataTriples": bound["dataTriples"],
                "focusNodes": bound["focusNodes"],
                "matchedClassCount": bound["matchedClassCount"],
                "targetClassCount": bound["targetClassCount"],
                "conformsRaw": bound["conforms"],
                "verdict": "fails",
                "reason": None,
            },
        ],
    }


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True, type=pathlib.Path)
    args = ap.parse_args()
    payload = build_validation()
    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / "validation.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(
        "validation.json: as-published focusNodes="
        f"{payload['runs'][0]['focusNodes']}, bound focusNodes={payload['runs'][1]['focusNodes']}"
    )


if __name__ == "__main__":
    main()
