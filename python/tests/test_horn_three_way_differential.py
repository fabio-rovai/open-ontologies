"""The three-way differential in the suite, and PROVED ABLE TO FAIL.

`tools/horn_differential.py` runs the Rust engine, this Python engine and the Lean
checker over every RDF document the repository tracks and requires the two engines to
derive the same set of triples while the checker accepts both certificates. This file
keeps a bounded slice of that run inside the test suite, so the tool cannot rot
unnoticed between full runs, and it does the thing a differential is worthless without:
it breaks one engine on purpose and shows each gate firing.

HOW INDEPENDENT THE TWO ENGINES ARE, WHICH IS LESS THAN THE WORD SUGGESTS. They are not
independent implementations. Both run the same semi-naive forward-chaining algorithm
over the same rule table, and this engine's comments cite `src/reason.rs` by file and
line. Agreement is therefore STRONG evidence against a transcription slip in one of the
two and CLOSE TO NO evidence against a shared misreading of a W3C rule, which both would
implement and agree on for ever. The independent leg is the Lean checker. Nothing in
this file may be quoted as "two independent reasoners agree"; the tool prints that
sentence next to its agreement count on every run.

WHY THE GATE TESTS MATTER MORE THAN THE AGREEMENT TESTS. A comparison that cannot fail
is worse than no comparison, because it reports agreement for ever. Three deliberate
defects are injected below, each of which a careless differential would pass:

  * a derivation silently dropped, leaving a certificate the Lean checker STILL ACCEPTS,
    because every step that remains is sound. Only the set comparison sees the loss.
  * a conclusion forged, which the Lean checker rejects. The tool must call that
    CHECKER_REJECTED and not DISAGREE: one broken emitter is not two engines disagreeing.
  * an asserted graph the other engine never saw, which has to be caught BEFORE the
    conclusions are compared, because a derived-set comparison over two different graphs
    means nothing.

WHAT THE SLICE IS AND IS NOT. The cases below were chosen for RULE COVERAGE and not for
size, from the per-rule histogram of a full run on 14 September 2026. Between them they
fire every one of the 20 rules the whole corpus fires, over 17,001 asserted triples and
6,828 derived ones, and they exercise the refusal path (a rule head that instantiates to
a triple no serialiser can write) on 204 conclusions. They are a canary. THE EVIDENCE IS
THE FULL RUN, which this file does not reproduce because it takes twelve minutes:

    python3 tools/horn_differential.py --bundles --json out.json

Measured there on 15 September 2026: 291 single documents carrying 203,825 asserted
triples and deriving 19,624, plus 24 bundles of those same documents deriving 59,115
(the two derived counts overlap and must not be added, because a bundle re-derives most
of what its members derive alone), plus the 7 coverage fixtures, which are test data and
are counted apart from all of it. Identical sets on both engines in every one of the 322
cases, every certificate accepted, no disagreement. Fifteen cases did not run and each is
listed with its reason: four bundles over the 8.4 MB cap, nine documents that do not
parse or parse to nothing, and the two OAEI anatomy OWL files, which time out at 900s.
Set HORN_DIFF_FULL=1 to run every tracked document here too.

THE SEVEN RULES THE CORPUS CANNOT EXERCISE. `prp-inv2`, `eq-sym`, `cls-avf`, `cls-hv1`,
`cls-hv2`, `scm-svf2` and `scm-avf2` fire in no document this repository tracks, so the
corpus run compared the two engines on them zero times and said nothing whatever about
them. `tests/fixtures/horn-coverage/` now holds one deliberately constructed graph per
silent rule, each the smallest thing that makes exactly that rule fire, and
`test_the_seven_rules_the_corpus_cannot_reach_are_covered_by_fixtures` runs all seven
here. The two coverage figures are reported apart and must never be added: 20 of 27 on
the repository's own RDF, 27 of 27 once graphs written for the purpose are included. A
rule covered only by its own fixture has been compared on one graph made to fire it and
on no real ontology, which is a great deal better than zero and is not the same claim.
"""

import importlib.util
import os
import re
import tempfile
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
TOOL = REPO / "tools" / "horn_differential.py"

# Each entry is here because of what it makes fire. The comment is the reason, and it is
# the only thing that stops this list drifting into a pile of files nobody can justify.
CASES = {
    # prp-trp and prp-symp fire nowhere else in the repository.
    "zero emission aviation, ontology and graph": [
        "case-studies/zero-emission-aviation/ontology/zef.ttl",
        "case-studies/zero-emission-aviation/graph.ttl",
        "case-studies/zero-emission-aviation/shapes/zef-shapes.ttl",
    ],
    # prp-inv1 and cls-svf1.
    "nordstream": ["benchmark/ontoaxiom/data/ontoaxiom/ontologies/nordstream.ttl"],
    # scm-eqp1, both halves.
    "music": ["benchmark/ontoaxiom/data/ontoaxiom/ontologies/music.ttl"],
    # rdfs3, which is where a head instantiates to a literal subject and gets refused.
    "era": ["benchmark/ontoaxiom/data/ontoaxiom/ontologies/era.ttl"],
    # rdfs2 plus refusals.
    "time": ["benchmark/ontoaxiom/data/ontoaxiom/ontologies/time.ttl"],
    # 167 refused conclusions, the largest refusal count reachable in a second.
    "heritage aerial, ontology and data": [
        "case-studies/heritage-aerial/ontology/naph-core.ttl",
        "case-studies/heritage-aerial/ontology/naph-ric-o-crosswalk.ttl",
        "case-studies/heritage-aerial/data/real-napl-sample.ttl",
        "case-studies/heritage-aerial/data/sample-photographs.ttl",
    ],
    # The only place a TBox meets an ABox that instantiates it: rdfs7, rdfs9 and the
    # schema rules at volume. 4,103 derived triples from 1,654 asserted.
    "ies4 tbox and abox": [
        "benchmark/reference/ies-core.ttl",
        "benchmark/reference/ies-top.ttl",
        "benchmark/data/hospital.ttl",
        "benchmark/data/movement.ttl",
        "benchmark/data/event-participation.ttl",
        "benchmark/data/identifiers.ttl",
        "benchmark/data/when-and-where.ttl",
        "benchmark/data/characteristics-and-measures.ttl",
        "benchmark/data/relationships.ttl",
        "benchmark/data/events.ttl",
        "benchmark/data/event-linkages.ttl",
        "benchmark/data/period-of-time.ttl",
        "benchmark/data/assessment.ttl",
        "benchmark/data/communication.ttl",
        "benchmark/data/sometimes.ttl",
        "benchmark/data/types.ttl",
        "benchmark/data/valid-PersonState.ttl",
        "benchmark/data/invalid-PersonState.ttl",
    ],
    # RDF/XML rather than Turtle, and the only case in the repository that fires
    # scm-avf1. The slowest of these by a factor of six, and kept for that one rule.
    "pizza in rdf/xml": ["benchmark/reference/pizza-reference.owl"],
}

# The cheapest case that still derives something, used for the gate tests.
GATE_CASE = "nordstream"


def tool():
    if not TOOL.is_file():
        pytest.skip(f"{TOOL} is absent, so this is not a repository checkout")
    spec = importlib.util.spec_from_file_location("horn_differential", TOOL)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    missing = module.preflight()
    if missing:
        pytest.skip(
            "a three-way differential needs all three implementations and this machine has "
            "fewer. Missing: " + "; ".join(missing)
        )
    return module


def case_files(name):
    files = [REPO / rel for rel in CASES[name]]
    absent = [str(f) for f in files if not f.is_file()]
    if absent:
        pytest.skip("corpus document absent: " + ", ".join(absent))
    return [str(f) for f in files]


def test_relative_iris_resolve_from_an_escaped_file_uri(tmp_path):
    import pyoxigraph as ox

    folder = tmp_path / "folder with # and %"
    folder.mkdir()
    source = folder / "source.ttl"
    source.write_text("<> <http://example.org/linksTo> <target.ttl> .", encoding="utf-8")
    store = ox.Store()
    store.load(
        path=str(source), format=ox.RdfFormat.TURTLE, base_iri=tool().base_iri_for(source)
    )
    (quad,) = store
    assert quad.subject.value == source.as_uri()
    assert quad.object.value == (folder / "target.ttl").as_uri()


def run(module, name, patch=None):
    """One case through all three implementations, optionally with the Python engine
    sabotaged. The patch is applied to the module attribute rather than to the import in
    `run_case`, which binds at call time, so the tool itself is untouched."""
    import open_ontologies_lite.horn.reason as reason_module

    real = reason_module.run_horn
    if patch is not None:
        reason_module.run_horn = patch(real)
    try:
        with tempfile.TemporaryDirectory(prefix="horndiff-test-") as d:
            return module.run_case({"name": name, "files": case_files(name)}, Path(d), 600)
    finally:
        reason_module.run_horn = real


@pytest.fixture(scope="module")
def slice_rows():
    """Every case, run ONCE. The slice costs fifteen seconds and two tests read it, so
    running it per test would double the price of the file for no extra evidence."""
    module = tool()
    return {name: run(module, name) for name in sorted(CASES)}


@pytest.mark.parametrize("name", sorted(CASES))
def test_the_two_engines_derive_the_same_triples(name, slice_rows, capsys):
    row = slice_rows[name]
    with capsys.disabled():
        fired = sum(1 for v in (row.get("by_rule") or {}).values() if v)
        print(
            f"\n  {name}: {row['verdict']}, {row.get('asserted')} asserted, "
            f"{row.get('derived_python')} derived, {row.get('refused_python')} refused, "
            f"{fired} rules fired"
        )
    assert row["verdict"] == "AGREE", (
        f"{name}: {row['verdict']}\n{row['detail']}\n"
        "A difference here is a defect in one of the three implementations. Do not adjust "
        "either engine to make it disappear: go back to the W3C rule the row names and work "
        "out which side is wrong."
    )


def test_the_slice_is_not_vacuous(slice_rows):
    """A differential over graphs that derive nothing passes silently and proves nothing.
    These are the numbers the slice was measured at, as a floor and not a target."""
    rows = list(slice_rows.values())
    assert all(r["verdict"] == "AGREE" for r in rows), [r["detail"] for r in rows]
    assert sum(r["asserted"] for r in rows) >= 15000
    assert sum(r["derived_python"] for r in rows) >= 6000
    assert sum(r["refused_python"] for r in rows) >= 150, (
        "no rule head instantiated to an unwritable triple anywhere in the slice, so the "
        "refusal path, where the two engines' guards are spelled differently, is untested"
    )
    fired = set()
    for r in rows:
        fired |= {k for k, v in r["by_rule"].items() if v}
    # 20 and not 27: seven rules fire in no document this repository ships, and the
    # slice is made of documents. The other seven are covered by
    # `test_the_seven_rules_the_corpus_cannot_reach_are_covered_by_fixtures`, from
    # graphs written for the purpose, and the two counts are never added together.
    assert len(fired) >= 20, (
        f"only {len(fired)} of the 20 corpus-reachable rules fired: {sorted(fired)}"
    )


def test_a_silently_dropped_derivation_is_caught():
    """The certificate stays VALID: every step that remains is sound and `oo-horn` accepts
    it. Only the comparison against the other engine can see that one is gone."""
    module = tool()

    def patch(real):
        def dropping(store, **kwargs):
            result = real(store, **kwargs)
            horn = Path(kwargs["certificate_dir"]) / "horn.tsv"
            lines = [line for line in horn.read_text("utf-8").split("\n") if line]
            # Remove only the final step; preserve LF bytes on Windows too.
            horn.write_text("\n".join(lines[:-1]) + "\n", encoding="utf-8", newline="\n")
            return result

        return dropping

    row = run(module, GATE_CASE, patch)
    assert row["verdict"] == "DISAGREE", row
    assert row["only_rust"] and not row["only_python"], row
    assert "only rust" in row["detail"], row


def test_a_forged_conclusion_is_a_rejected_certificate_and_not_a_disagreement():
    """Severity order matters. Two certificates that differ, one of which the checker
    rejects, is one broken emitter and must not be filed as two working engines
    disagreeing."""
    module = tool()

    def patch(real):
        def forging(store, **kwargs):
            result = real(store, **kwargs)
            horn = Path(kwargs["certificate_dir"]) / "horn.tsv"
            lines = [line for line in horn.read_text("utf-8").split("\n") if line]
            fields = lines[0].split("\t")
            at = 2 + 2 * int(fields[1])
            fields[at + 2] = "<http://example.org/FORGED>"
            lines[0] = "\t".join(fields)
            horn.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
            return result

        return forging

    row = run(module, GATE_CASE, patch)
    assert row["verdict"] == "CHECKER_REJECTED", row
    assert "python certificate" in row["detail"], row


def test_a_different_asserted_graph_stops_before_the_conclusions_are_compared():
    module = tool()

    def patch(real):
        def ghosting(store, **kwargs):
            result = real(store, **kwargs)
            asserted = Path(kwargs["certificate_dir"]) / "asserted.tsv"
            asserted.write_text(
                asserted.read_text("utf-8")
                + "<http://example.org/ghost>\t<http://example.org/p>\t<http://example.org/o>\n",
                encoding="utf-8",
                newline="\n",
            )
            return result

        return ghosting

    row = run(module, GATE_CASE, patch)
    assert row["verdict"] == "INPUT_DIFFERS", row
    assert "ghost" in row["detail"], row


def test_the_corpus_the_tool_would_run_is_not_empty():
    """`discover` shells out to `git ls-files`. If that ever returns nothing the full run
    reports a clean sweep over zero documents, which is the worst outcome available."""
    module = tool()
    files = module.discover(REPO)
    assert len(files) > 100, f"only {len(files)} RDF documents discovered under {REPO}"


def test_the_coverage_fixtures_are_not_part_of_the_corpus():
    """The corpus figure must stay a number about real ontologies.

    `tests/fixtures/horn-coverage/` holds graphs written to make one named rule fire.
    Counting them as corpus documents would inflate the coverage figure with data
    constructed to hit it, which is measuring the ruler, so `discover` excludes the
    directory and this is the test that says so.
    """
    module = tool()
    files = module.discover(REPO)
    leaked = [f for f in files if module.is_coverage_fixture(f)]
    assert not leaked, f"coverage fixtures reached the corpus: {leaked}"
    assert module.coverage_cases(), (
        "no coverage fixtures were found at all. The seven rules the corpus cannot reach "
        "are then uncovered again and the headline is back to 20 of 27"
    )


# The rules no document in this repository fires. Measured from the per-rule histogram of
# the full run on 14 September 2026, and the reason `tests/fixtures/horn-coverage/` exists.
CORPUS_CANNOT_REACH = (
    "prp-inv2", "eq-sym", "cls-avf", "cls-hv1", "cls-hv2", "scm-svf2", "scm-avf2",
)


def test_the_seven_rules_the_corpus_cannot_reach_are_covered_by_fixtures(capsys):
    """Each fixture fires the rule its filename names, and only that rule.

    This is the whole of what moves the headline from 20 of 27 to 27 of 27, so it is
    checked rather than asserted: every fixture goes through both engines and the Lean
    checker like any other case, and the per-rule credit is read out of the run.

    A fixture that fires NOTHING would be a finding about the engine rather than about
    the fixture, and it fails here loudly instead of quietly leaving the rule uncovered.
    """
    module = tool()
    cases = {c["name"]: c for c in module.coverage_cases()}
    assert len(cases) == len(CORPUS_CANNOT_REACH), (
        f"{len(cases)} fixtures for {len(CORPUS_CANNOT_REACH)} uncovered rules: {sorted(cases)}"
    )

    covered, problems = {}, []
    for name, case in sorted(cases.items()):
        with tempfile.TemporaryDirectory(prefix="horndiff-cov-") as d:
            row = module.run_case(case, Path(d), 600)
        want = case["rule"]
        fired = {k.split("#")[0]: v for k, v in (row.get("by_rule") or {}).items() if v}
        with capsys.disabled():
            print(f"\n  {name}: {row['verdict']}, {row.get('asserted')} asserted, "
                  f"{row.get('derived_python')} derived, fired {fired}")
        if row["verdict"] != "AGREE":
            problems.append(f"{name}: {row['verdict']} {row['detail']}")
        elif want not in fired:
            problems.append(
                f"{name}: fired {sorted(fired) or 'nothing'} and not {want}. If no graph can "
                f"make {want} fire, that is a finding about the engine and belongs in the report"
            )
        elif len(fired) > 1:
            problems.append(
                f"{name}: fired {sorted(fired)}; a coverage fixture is meant to be the SMALLEST "
                f"graph that makes {want} fire, so anything else firing means it is not minimal"
            )
        else:
            covered[want] = fired[want]

    assert not problems, "\n".join(problems)
    assert sorted(covered) == sorted(CORPUS_CANNOT_REACH), (
        f"covered {sorted(covered)}, expected {sorted(CORPUS_CANNOT_REACH)}"
    )


# Skipped because it is SLOW, not because anything is missing, which is the one
# kind of skip `OO_REQUIRE_FIXTURES=1` must leave alone. See python/tests/conftest.py.
@pytest.mark.oo_opt_in
@pytest.mark.skipif(
    os.environ.get("HORN_DIFF_FULL") != "1",
    reason="the full corpus takes twelve minutes. HORN_DIFF_FULL=1 to run it here, or run "
    "tools/horn_differential.py --bundles",
)
def test_every_tracked_document_agrees():
    module = tool()
    blocking = []
    for path in module.discover(REPO):
        if path.stat().st_size > module.MAX_BYTES:
            continue
        with tempfile.TemporaryDirectory(prefix="horndiff-test-") as d:
            row = module.run_case(
                {"name": str(path.relative_to(REPO)), "files": [str(path)]}, Path(d), 600
            )
        if row["verdict"] in module.BLOCKING:
            blocking.append(row)
    assert not blocking, "\n".join(f"{r['verdict']} {r['name']}: {r['detail']}" for r in blocking)


# The form every citation of the Rust engine takes in this package: a backticked
# `src/reason.rs:N`, then a comma, then the backticked token that line is supposed to
# contain. Anything else is not matched and fails the test below rather than being
# skipped, because a citation this cannot read is a citation nobody is checking.
RUST_CITATION = re.compile(r"`src/reason\.rs:(\d+)`,\s*\n?\s*`([^`]+)`")


def test_the_python_engine_cites_the_rust_by_a_line_that_still_says_what_it_claims():
    """The evidence for the independence caveat, checked rather than asserted.

    The caveat at the top of this file says the two engines are not independent, and the
    reason given is that this package's comments cite `src/reason.rs` BY LINE. That is
    the load-bearing fact, and it was measured once and then went stale: three citations
    pointed roughly 700 lines short of their target, because `src/reason.rs` grew
    underneath them. A stale line number is worse than none, since a reader who follows
    it lands on unrelated code and concludes the claim was invented.

    So every citation is resolved here: the line must exist and must contain the token
    the citation names. A citation this regex cannot read is a FAILURE, not a skip, or
    the next one written in a new shape would be silently unguarded.
    """
    root = REPO / "python"
    sources = sorted(p for p in root.rglob("*.py") if "__pycache__" not in p.parts)
    assert sources, f"no Python sources found under {root}"
    rust = (REPO / "src" / "reason.rs").read_text().splitlines()

    cited, bad = 0, []
    for path in sources:
        text = path.read_text()
        rel = path.relative_to(REPO)
        for raw in re.findall(r"`src/reason\.rs:\d+`[^`]*`[^`]*`", text):
            m = RUST_CITATION.search(raw)
            if not m:
                bad.append(f"{rel}: {raw!r} does not name a token after the line number")
                continue
            cited += 1
            line, token = int(m.group(1)), m.group(2)
            if not 1 <= line <= len(rust):
                bad.append(f"{rel}: src/reason.rs:{line} is past the end ({len(rust)} lines)")
            elif token.rstrip("()") not in rust[line - 1]:
                where = [i + 1 for i, l in enumerate(rust) if token.rstrip("()") in l]
                bad.append(
                    f"{rel}: src/reason.rs:{line} does not contain {token!r}. "
                    f"It is at {where or 'no line at all'}. The file moved under the comment"
                )
    assert not bad, "\n".join(bad)
    assert cited >= 3, (
        f"only {cited} citations of src/reason.rs found in {root}. The independence caveat "
        "rests on this package citing the Rust by line; if that stopped being true the "
        "caveat's stated reason needs rewriting, not this floor lowering"
    )
