#!/usr/bin/env python3
"""Run the commerce-profile negative controls and assert expected outcomes.

A validator that never fails proves nothing. Exit code 0 means every control
behaved as expected; any deviation exits non-zero and names the control.
"""
import pathlib
import sys

from pyshacl import validate

HERE = pathlib.Path(__file__).resolve().parent
SHAPES = HERE.parent / "ontology" / "naph-commerce-shapes.ttl"

EXPECTED = {
    "c1_licensable_no_offer.ttl": False,   # must NOT conform
    "c2_offer_no_route.ttl": False,        # must NOT conform
    "c3_offer_with_route.ttl": True,       # must conform
}


def main() -> int:
    failures = []
    for name, want in EXPECTED.items():
        conforms, _, text = validate(
            data_graph=str(HERE / name),
            shacl_graph=str(SHAPES),
            advanced=True,
        )
        verdict = "conforms" if conforms else "fails"
        expected = "conforms" if want else "fails"
        marker = "OK " if conforms == want else "UNEXPECTED"
        print(f"{marker} {name}: {verdict} (expected {expected})")
        if conforms != want:
            failures.append(name)
            print(text)
    if failures:
        print(f"CONTROL MISMATCH: {failures}")
        return 1
    print("All controls behaved as expected.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
