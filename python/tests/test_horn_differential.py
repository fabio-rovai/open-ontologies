"""Three implementations over one corpus, and what a difference means.

This is the reason a second reasoner is worth writing at all. The Rust engine,
this Python engine and the Lean checker are three separate implementations, so
running all three over the same graphs finds defects that none of them finds
alone. What this file does NOT do is adjudicate: it is modelled on
`tools/fol_differential.py`, whose first paragraph is "THE ATP IS AN ORACLE, NOT
AN AUTHORITY", and the same discipline applies here. A disagreement is a defect in
ONE OF THE THREE and the job is to report it, not to decide which.

SEPARATE IS NOT INDEPENDENT, AND THIS FILE ONCE SAID IT WAS. The two ENGINES are
not independent implementations: they run the same semi-naive forward-chaining
algorithm over the same rule table, this one was written with `src/reason.rs`
open, and the comments below cite that file by line. Their agreement is strong
evidence against a TRANSCRIPTION slip in one of the two and close to no evidence
against a SHARED MISREADING of a W3C rule, which both would implement and agree
on for ever. The independent leg is the LEAN CHECKER, written from the W3C rules
with a machine-checked soundness theorem. Nothing here may be quoted as "two
independent reasoners agree"; `tools/horn_differential.py` prints that sentence
next to its agreement count on every run.

GATED on both binaries and skips loudly wherever it cannot run. CI has neither.

WHAT IS COMPARED, AND WHAT IS NOT.

The assertion is SET EQUALITY OF DERIVED TRIPLES against the Rust `run_horn`, over
the same `builtin_rules.tsv`. Byte identity of the two `horn.tsv` files is NOT a
goal and must not be asserted: the Rust emitter sorts interned `u32` triples
(`src/reason.rs:2823`, `all.sort_unstable()` over `Vec<(u32,u32,u32)>`), so its
line order is first-appearance order in the store, not lexicographic order.
Reproducing it from Python would mean reimplementing the interner and betting on
identical store-iteration order across two bindings, to buy nothing the set
comparison does not already give.

The target is `run_horn` and not `run_full`, and conflating the two will burn a
day. `run_full` is the hardcoded-arm engine and it carries guards the rule table
does not: `rdfs11` is guarded `a != b && b != c && a != c` and `rdfs3` is guarded
on a non-literal object. `run_horn` is generic over the table and carries none of
them, which is why exact agreement is the right expectation here. A comparison
against `run_full` is a CHARACTERISATION run, not an assertion, and anyone who
sees a gap there and "fixes" Python to match will break the table equality and
lose the absolute verdict.

Measured on 14 September 2026 over `demo/derived/_store.ttl`, the largest graph
in the corpus that derives anything: `run_full` under `--profile owl-rl` derives
859 and both table-driven engines derive 859, in three iterations each. So the
predicted divergence does not materialise on this corpus. That is a fact about
the corpus and not about the guards: `rdfs11`'s `a != b && b != c && a != c` and
`rdfs3`'s literal-object filter only bite where those patterns occur, and here
they do not. Do not read the agreement as evidence that the guards are absent,
and do not read a future divergence as a defect.

Blank-node-bearing triples are compared separately and by shape, because blank
node labels are scoped to a parse and two parses of one file may label them
differently. A label difference is not a disagreement about what follows.
"""

import json
import os
import shutil
import subprocess
from pathlib import Path

import pyoxigraph as ox
import pytest

from open_ontologies_lite.horn.certify import CheckerUnavailable, check_horn, find_checker
from open_ontologies_lite.horn.reason import run_horn
from open_ontologies_lite.horn.rules import builtin_rules_bytes

REPO = Path(__file__).resolve().parents[2]


def _oo_binary() -> Path:
    for candidate in (
        os.environ.get("OO_BIN"),
        REPO / "target" / "release" / "open-ontologies",
        REPO / "target" / "debug" / "open-ontologies",
        shutil.which("open-ontologies"),
    ):
        if candidate and Path(candidate).is_file():
            return Path(candidate)
    pytest.skip(
        "the Rust engine is not built. `cargo build --release --bin open-ontologies`, "
        "or set OO_BIN"
    )


def _checker() -> Path:
    try:
        return find_checker("oo-horn")
    except CheckerUnavailable as exc:
        pytest.skip(str(exc))


def _corpus() -> list[Path]:
    demo = REPO / "demo"
    if not demo.is_dir():
        pytest.skip(f"not in a repository checkout: {demo} is absent")
    return sorted(
        p
        for pattern in ("*.ttl", "*.owl", "*.rdf")
        for p in demo.rglob(pattern)
    )


CORPUS = (
    sorted(
        p
        for pattern in ("*.ttl", "*.owl", "*.rdf")
        for p in (REPO / "demo").rglob(pattern)
    )
    if (REPO / "demo").is_dir()
    else []
)


def _conclusions(horn_tsv: Path, rules_tsv: Path) -> set[tuple[str, str, str]]:
    """Read the conclusion of every step out of a `horn.tsv`, whoever wrote it.

    The conclusion sits after the binding pairs, whose count is the second field,
    so this is `parseHornSteps`' own arithmetic and it works on both engines'
    output without either engine's help.
    """
    out = set()
    for line in horn_tsv.read_text("utf-8").split("\n"):
        if not line:
            continue
        fields = line.split("\t")
        at = 2 + 2 * int(fields[1])
        out.add(tuple(fields[at : at + 3]))
    return out


def _ground(triples):
    return {t for t in triples if not any(x.startswith("_:") for x in t)}


def _shapes(triples):
    """Blank labels normalised away, so two parses can still be compared."""
    return sorted(
        tuple("_:_" if x.startswith("_:") else x for x in t)
        for t in triples
        if any(x.startswith("_:") for x in t)
    )


def _rust_run(binary: Path, source: Path, out: Path, rules: Path, data_dir: Path) -> dict:
    batch = (
        f"load {source}\n"
        f"reason --rules {rules} --certificate {out}\n"
    )
    proc = subprocess.run(
        [
            str(binary),
            "batch",
            "--no-connect",
            "--json",
            "--data-dir",
            str(data_dir),
            "-",
        ],
        input=batch,
        capture_output=True,
        text=True,
        timeout=300,
    )
    assert proc.returncode == 0, proc.stderr
    lines = [json.loads(line) for line in proc.stdout.splitlines() if line.strip()]
    results = {entry["command"]: entry["result"] for entry in lines}
    assert results["load"].get("ok") is True, results["load"]
    assert "error" not in results["reason"], results["reason"]
    return results["reason"]


@pytest.mark.parametrize("source", CORPUS, ids=lambda p: p.name)
def test_the_two_engines_derive_the_same_triples(source, tmp_path, capsys):
    binary = _oo_binary()
    _checker()
    rules = tmp_path / "builtin_rules.tsv"
    rules.write_bytes(builtin_rules_bytes())

    store = ox.Store()
    fmt = ox.RdfFormat.from_extension(source.suffix.lstrip("."))
    try:
        store.load(path=str(source), format=fmt)
    except (SyntaxError, ValueError) as exc:
        pytest.skip(f"{source.name} does not parse: {exc}")

    py_dir = tmp_path / "python"
    py = run_horn(store, certificate_dir=py_dir)
    rust_dir = tmp_path / "rust"
    rust = _rust_run(binary, source, rust_dir, rules, tmp_path / "data")

    py_concl = _conclusions(py_dir / "horn.tsv", py_dir / "rules.tsv")
    rust_concl = _conclusions(rust_dir / "horn.tsv", rust_dir / "rules.tsv")

    # Both certificates must stand on their own before their contents are
    # compared. A disagreement between two certificates one of which the checker
    # rejects is not a differential result, it is one broken emitter.
    py_check = check_horn(py_dir)
    rust_check = check_horn(rust_dir)
    assert py_check["status"] == "accepted", py_check
    assert rust_check["status"] == "accepted", rust_check
    assert py_check["verdict"] == rust_check["verdict"]
    assert py_check["digests_agree"] is True
    assert rust_check["digests_agree"] is True

    with capsys.disabled():
        print(
            f"\n  {source.relative_to(REPO)}: asserted {py.asserted_triples}, "
            f"python derived {len(py_concl)}, rust derived {len(rust_concl)}, "
            f"python refused {py.skipped_unserialisable}, "
            f"rust refused {rust.get('skipped_unserialisable', 0)}, "
            f"iterations {py.iterations}/{rust['iterations']}"
        )

    only_py = _ground(py_concl) - _ground(rust_concl)
    only_rust = _ground(rust_concl) - _ground(py_concl)
    assert not only_py and not only_rust, (
        f"the two engines disagree on {source.name}. A difference here is a defect in "
        f"one of the three implementations, and this test does not adjudicate which.\n"
        f"  only Python derived: {sorted(only_py)[:5]}\n"
        f"  only Rust derived:   {sorted(only_rust)[:5]}"
    )
    assert _shapes(py_concl) == _shapes(rust_concl), (
        "the two engines derive the same ground triples but a different number or shape "
        "of blank-node conclusions"
    )
    assert py.fixpoint_reached is True
    assert rust["fixpoint_reached"] is True
    assert py.skipped_unserialisable == rust.get("skipped_unserialisable", 0)


def test_the_corpus_is_not_empty():
    """A parametrised test over an empty list passes silently, which would make
    every claim above vacuous."""
    assert _corpus(), "no .ttl/.owl/.rdf under demo/ — the differential ran over nothing"
