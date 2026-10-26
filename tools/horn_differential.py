#!/usr/bin/env python3
"""Three-way differential: the Rust engine, the Python engine and the Lean checker,
over one Horn rule table and every ontology this repository ships.

WHY THIS EXISTS. `src/reason.rs` and `python/src/open_ontologies_lite/horn/` are two
implementations of the same specification, and `lean/` holds a checker whose soundness
is a machine-checked theorem. Running all three over one corpus finds defects none of
them finds alone. Agreement on a handful of hand-made files is weak evidence, so the
corpus is every RDF document tracked in the repository, and the report says how many
ontologies and how many derived triples the claim rests on.

HOW INDEPENDENT THE TWO ENGINES ARE, WHICH IS LESS THAN "INDEPENDENT". This is the
first thing to know about every number below and it is printed on every run, next to
the agreement count, so it cannot be quoted without it.

The Rust and Python reasoners are NOT independent implementations. They run the same
semi-naive forward-chaining algorithm over the same rule table, the Python was written
with the Rust open, and the Python's comments cite `src/reason.rs` by file and line.
What their agreement is therefore strong evidence about is a TRANSCRIPTION SLIP: an
index off by one, a guard dropped, a join in the wrong order, a rule arm that reads the
subject where it meant the object. Two people copying one design do not make the same
slip twice.

What it is close to NO evidence about is a SHARED MISREADING. If the design itself
misreads a W3C rule, both engines implement the misreading and agree perfectly for
ever. The clean sweep says nothing about that, and this tool must never be quoted as
"two independent reasoners agree".

The leg of this differential that IS independent is the LEAN CHECKER. It was written
from the W3C rules, its soundness theorem is machine-checked, and it is the only
component here whose acceptance means anything on its own. Two engines and a checker,
and the checker is the one carrying the warrant.

THIS TOOL DOES NOT ADJUDICATE. It is modelled on `tools/shacl_differential.py` and on
`tools/fol_differential.py`, whose first paragraph is "THE ATP IS AN ORACLE, NOT AN
AUTHORITY". A disagreement is a defect in ONE OF THE THREE and the job is to name it,
print the ontology and the triple, and stop the line. Nothing here edits either engine
to make a difference disappear, and no verdict word is decided in this file: both are
read out of the checker's JSON.

WHAT IS COMPARED.

  1. THE SAME GRAPH REACHES BOTH ENGINES, and this is the part that is easy to get
     wrong. Two parses of one Turtle document label blank nodes differently (measured:
     1267 of pizza.ttl's 1848 triples carry one, and not a single label survives), so a
     naive run compares two graphs that are isomorphic but not equal and reports a
     thousand phantom disagreements. The fix is a canonical intermediate: Python parses
     the source once, dumps the default graph to N-Triples, and BOTH engines read that
     file. Oxigraph preserves explicit `_:` labels from N-Triples, so the asserted sets
     then agree line for line, which every run asserts before comparing anything else.
     A difference there is INPUT_DIFFERS and blocks, because a derived-set comparison
     over two different graphs means nothing.

  2. SET EQUALITY OF DERIVED TRIPLES, blank nodes included, no shape fudging. Byte
     identity of the two `horn.tsv` files is NOT a goal and must not be asserted: the
     Rust emitter sorts interned `u32` triples (`src/reason.rs`, `all.sort_unstable()`),
     so its line order is first-appearance order in the store, not lexicographic.

  3. BOTH CERTIFICATES STAND ON THEIR OWN. Each goes to `oo-horn check` before their
     contents are compared. Two certificates that disagree, one of which the checker
     rejects, is not a differential result: it is one broken emitter.

  4. COVERAGE BY RULE. A rule that never fires anywhere in the corpus is a rule this run
     says nothing about. The per-rule histogram is printed for exactly that reason, and
     a reader who skips it will overstate what agreement was shown.

     Measured on 14 September 2026, the repository's own RDF fired 20 of the 27 rules
     and left seven silent: prp-inv2, eq-sym, cls-avf, cls-hv1, cls-hv2, scm-svf2 and
     scm-avf2. `tests/fixtures/horn-coverage/` now holds one deliberately constructed
     graph per silent rule, each the smallest thing that makes exactly that rule fire.
     They are run as a SEPARATE, LABELLED set and are excluded from `discover()`, so
     the corpus number stays a number about real ontologies. Two coverage figures are
     reported and never added together: how many rules fired on the repository's own
     RDF, and how many fired once the fixtures are included. A rule covered only by a
     fixture written to cover it has been compared on a graph made for the purpose and
     on nothing else, which is better than zero and is not the same thing. Re-measured
     on 15 September 2026 over a corpus that had grown to 291 documents: the same 20,
     the same seven, and every fixture firing exactly its own rule once. No rule in the
     table is unreachable.

     WHICH RULE SET THIS COUNTS. The 27 rows of `tests/fixtures/horn/builtin_rules.tsv`,
     which is what this tool passes to `reason --rules`. It is NOT the `owl-rl-ext`
     profile, and the two differ on real data: on `benchmark/reference/pizza-reference.owl`
     the table derives 387 `rdfs11`, 102 `scm-svf1` and 1 `scm-avf1` where the profile
     derives 357, 101 and 0, and the profile also fires `cls-int2` and `cls-oo`, which
     this table does not contain. `tests/reason_rl_coverage_test.rs` measures the profile
     and asserts `scm-avf1` fires nowhere; this tool credits it once. Both are true of
     their own rule set, and neither may be quoted as "the corpus" without saying which.

BUNDLES, AND WHY THEY ARE NOT OPTIONAL. Run one file at a time and most of the OWL
rules never fire, because the repository ships a TBox in one directory and the ABox
that instantiates it in another: `rdfs9` needs a type assertion and a subclass axiom in
the same graph, `rdfs2` needs a property with a domain and somebody using it. A
per-file run over 297 documents exercised 15 of 27 rules. `--bundles` unions the
documents of a directory, of an area, and of the two directories the repository pairs by
convention, so the ABox meets the TBox. Every bundle is named in the report with the
number of files in it.

The target is `run_horn` and not `run_full`. `run_full` is the hardcoded-arm engine and
carries guards the table does not (`rdfs11` is guarded `a != b && b != c && a != c`,
`rdfs3` on a non-literal object), so a comparison against it is characterisation, not an
assertion.

Verdicts, in order of severity:

  DISAGREE          the two engines were handed the same asserted graph and derived
                    different sets. The stop-the-line case. The ontology and the triples
                    are printed, never summarised away.
  CHECKER_REJECTED  `oo-horn` rejected a certificate. Worse than a disagreement: a
                    machine-checked theorem says a rejected step does not follow.
  VERDICT_DIFFERS   both accepted, and the checker pronounced differently on them, or a
                    rule-table digest does not match. The two runs were not over the
                    same table.
  INPUT_DIFFERS     the two asserted graphs are not equal, so nothing downstream is
                    comparable.
  INCOMPLETE        an engine stopped on an iteration cap. Its count is a lower bound
                    and not the closure, so equality would be meaningless.
  PARSE_DIFFERS     informational, and not a reasoning result: loading the ORIGINAL
                    source directly in Rust yields a different triple count from
                    Python's parse of it. The reasoning comparison is unaffected,
                    because both engines read the canonical N-Triples.
  REFUSAL_DIFFERS   informational: equal derived sets, different counts of conclusions
                    refused as unwritable.
  ATTRIBUTION_DIFFERS  informational: equal derived sets, different per-rule credit. A
                    triple two rules can derive is credited to whichever fired first, so
                    this is an ordering artefact and not a defect.
  NOT_RUN           skipped, with the reason. Never silent.
  AGREE             identical derived sets, both certificates accepted, same verdict.

Usage:
    python3 tools/horn_differential.py [--bundles] [--json out.json] [--limit N]
                                       [--jobs N] [--timeout S] [--max-bytes N]
                                       [--include SUBSTRING] [--exclude SUBSTRING]
                                       [--root DIR] [--no-coverage] [--verbose]

Requires the Rust binary and the Lean checker. If either is missing the run SKIPS
LOUDLY and exits 0, the way the Lean-dependent tests skip on a missing `lake`; set
HORN_DIFF_REQUIRE_TOOLS=1 to turn that skip into a failure, as OO_REQUIRE_FIXTURES=1
does in the Rust suite.
"""
from __future__ import annotations

import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor, as_completed

REPO = pathlib.Path(__file__).resolve().parent.parent

BIN = os.environ.get(
    "OPEN_ONTOLOGIES_BIN",
    str(REPO / "target" / "release" / "open-ontologies"),
)

# A case larger than this is reported as skipped rather than run. The Python engine is a
# readable reference implementation and not a fast one, and a multi-million-triple
# closure turns a diff run into an overnight job for no extra signal. Nothing is capped
# silently: every case excluded is listed in the report with the reason and the number.
MAX_BYTES = int(os.environ.get("HORN_DIFF_MAX_BYTES", 8 * 1024 * 1024))
TIMEOUT = int(os.environ.get("HORN_DIFF_TIMEOUT", 900))

EXTENSIONS = (".ttl", ".owl", ".rdf", ".nt", ".jsonld", ".nq", ".trig")

# The areas whose subdirectories are bundled. Anything else (a stray file at the repo
# root, say) is compared on its own and never bundled.
AREAS = ("benchmark", "case-studies", "demo", "tests")

# Directories the repository pairs by convention and a per-directory bundle would never
# put together: `benchmark/reference` holds the IES4 TBox and `benchmark/data` holds the
# instance data that uses it (`ies:` is `http://ies.data.gov.uk/ontology/ies4#` in
# both). Without this pairing `rdfs9`, `rdfs2` and `rdfs3` have nothing to fire on in
# the whole corpus.
CROSS_BUNDLES = {
    "benchmark/reference + benchmark/data": ("benchmark/reference", "benchmark/data"),
}

# Deliberate test data: one graph per rule the corpus never fires, each written to make
# exactly that rule fire. Kept OUT of the corpus by `discover`, reported separately, and
# never added to the corpus coverage figure. See the directory's README.md.
COVERAGE_DIR = REPO / "tests" / "fixtures" / "horn-coverage"
COVERAGE_PREFIX = "coverage "

# The caveat that travels with every agreement number this tool prints. It is here as a
# constant rather than in the docstring alone because a docstring is not printed, and the
# agent that built this tool asked that the sweep never be quoted without it.
INDEPENDENCE_CAVEAT = """\
  WHAT THE AGREEMENT ABOVE IS WORTH
    The Rust and Python reasoners are NOT independent implementations. They run the
    same semi-naive forward-chaining algorithm over the same rule table, and the
    Python's comments cite src/reason.rs by file and line. Their agreement is STRONG
    evidence against a transcription slip in one of the two, and it is close to NO
    evidence against a shared misreading of a W3C rule, because a misreading in the
    design is copied into both and they then agree for ever.
    Do not quote this run as "two independent reasoners agree".
    The independent leg is the LEAN CHECKER: written from the W3C rules, soundness
    machine-checked, and the only component here whose acceptance means anything on
    its own."""

BLOCKING = ("DISAGREE", "CHECKER_REJECTED", "VERDICT_DIFFERS", "INPUT_DIFFERS")
ORDER = (
    "DISAGREE",
    "CHECKER_REJECTED",
    "VERDICT_DIFFERS",
    "INPUT_DIFFERS",
    "INCOMPLETE",
    "PARSE_DIFFERS",
    "REFUSAL_DIFFERS",
    "ATTRIBUTION_DIFFERS",
    "ERROR",
    "AGREE",
)
COMPARED = ("AGREE", "PARSE_DIFFERS", "REFUSAL_DIFFERS", "ATTRIBUTION_DIFFERS", "DISAGREE")


# ---------------------------------------------------------------------------
# Format detection, mirrored from src/graph.rs::detect_format_sniffed.
#
# Not a convenience. If Python decides a `.owl` file is RDF/XML and the Rust side
# sniffs the body and decides it is Turtle, the two stacks read different documents and
# every difference downstream is an artefact of this function. It is transcribed from
# the Rust rather than approximated, and the PARSE_DIFFERS check is what would catch a
# transcription error: it loads the original through the Rust path and compares counts.
# ---------------------------------------------------------------------------
def detect_format(path: pathlib.Path, content: str):
    import pyoxigraph as ox

    name = path.name
    if name.endswith(".ttl") or name.endswith(".turtle"):
        ext = ox.RdfFormat.TURTLE
    elif name.endswith(".nt") or name.endswith(".ntriples"):
        ext = ox.RdfFormat.N_TRIPLES
    elif name.endswith(".rdf") or name.endswith(".xml") or name.endswith(".owl"):
        ext = ox.RdfFormat.RDF_XML
    elif name.endswith(".nq"):
        ext = ox.RdfFormat.N_QUADS
    elif name.endswith(".trig"):
        ext = ox.RdfFormat.TRIG
    elif name.endswith(".jsonld") or name.endswith(".json"):
        ext = ox.RdfFormat.JSON_LD
    else:
        ext = ox.RdfFormat.TURTLE

    head = ""
    for line in content.split("\n"):
        line = line.strip()
        if line and not line.startswith("#"):
            head = line
            break

    if head.startswith("<?xml") or head.startswith("<rdf:") or head.startswith("<RDF"):
        return ox.RdfFormat.RDF_XML
    directive = (
        head.startswith("@prefix")
        or head.startswith("@base")
        or head.upper().startswith("PREFIX ")
        or head.upper().startswith("BASE ")
    )
    if directive and ext == ox.RdfFormat.RDF_XML:
        return ox.RdfFormat.TURTLE
    opens_json = head.startswith("{") or head.startswith("[")
    keyword = '"@context"' in content or '"@id"' in content or '"@graph"' in content
    if opens_json and keyword:
        return ox.RdfFormat.JSON_LD
    return ext


def base_iri_for(path: pathlib.Path) -> str:
    """The base the Rust loader uses: `file://` plus the canonical path.

    `src/graph.rs::load_file` does this because a document's own location is its default
    base per RFC 3986, and without it every W3C SHACL test file, each of which names its
    own ontology with a relative IRI, fails to parse. Python must use the same string or
    the two stacks see different IRIs for the same subject.
    """
    return "file://" + str(path.resolve())


def batch(script: str, data_dir: pathlib.Path, timeout: int):
    proc = subprocess.run(
        [BIN, "batch", "--no-connect", "--json", "--data-dir", str(data_dir), "-"],
        input=script,
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    results = {}
    for line in proc.stdout.splitlines():
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if "command" in row:
            results[row["command"]] = row.get("result", {})
    return results, proc


def conclusions(horn_tsv: pathlib.Path) -> set:
    """The conclusion of every step in a `horn.tsv`, whoever wrote it.

    The conclusion sits after the binding pairs, whose count is the second field, so
    this is `parseHornSteps`' own arithmetic and it reads both engines' output without
    either engine's help.
    """
    out = set()
    for line in horn_tsv.read_text("utf-8").split("\n"):
        if not line:
            continue
        fields = line.split("\t")
        at = 2 + 2 * int(fields[1])
        out.add(tuple(fields[at : at + 3]))
    return out


def run_case(case: dict, workdir: pathlib.Path, timeout: int) -> dict:
    """One case, one or many documents, through all three implementations.

    Returns a row and never raises: an engine crash is a result to report, not a
    traceback to lose.
    """
    import pyoxigraph as ox

    from open_ontologies_lite.horn.certify import check_horn
    from open_ontologies_lite.horn.reason import run_horn
    from open_ontologies_lite.horn.rules import builtin_rules_bytes

    sources = [pathlib.Path(f) for f in case["files"]]
    row = {"name": case["name"], "files": len(sources), "verdict": "ERROR", "detail": ""}

    rules = workdir / "rules_in.tsv"
    rules.write_bytes(builtin_rules_bytes())

    # --- Python parses once, and that parse is the corpus item. -------------
    store = ox.Store()
    dropped = []
    t0 = time.time()
    for source in sources:
        try:
            content = source.read_text("utf-8", errors="replace")
            fmt = detect_format(source, content)
            store.load(path=str(source), format=fmt, base_iri=base_iri_for(source))
        except (SyntaxError, ValueError, OSError) as exc:
            dropped.append(f"{source.name}: {str(exc)[:120]}")
    parse_seconds = time.time() - t0
    if dropped:
        row["dropped"] = dropped
    if len(dropped) == len(sources):
        return {
            **row,
            "verdict": "NOT_RUN",
            "detail": "no document parsed. " + "; ".join(dropped),
        }

    # Only the default graph reaches either engine: `run_horn(graphs="default")` on the
    # Python side, and the N-Triples dump below on the Rust side. Counting `len(store)`
    # here instead would overstate the asserted graph for any quad-bearing source.
    asserted_python = sum(1 for _ in store.quads_for_pattern(None, None, None, ox.DefaultGraph()))
    if asserted_python == 0:
        return {**row, "verdict": "NOT_RUN", "detail": "parsed to zero default-graph triples"}

    # --- The canonical intermediate both engines read. ----------------------
    canon = workdir / "canon.nt"
    store.dump(output=str(canon), format=ox.RdfFormat.N_TRIPLES, from_graph=ox.DefaultGraph())

    # --- Rust, over the canonical graph. ------------------------------------
    rust_dir = workdir / "rust"
    t0 = time.time()
    try:
        results, proc = batch(
            f"load {canon}\nreason --rules {rules} --certificate {rust_dir}\n",
            workdir / "data",
            timeout,
        )
    except subprocess.TimeoutExpired:
        return {**row, "verdict": "NOT_RUN", "detail": f"the rust engine timed out after {timeout}s"}
    rust_seconds = time.time() - t0
    if proc.returncode != 0:
        return {**row, "detail": f"rust exit {proc.returncode}: {proc.stderr.strip()[:300]}"}
    if results.get("load", {}).get("ok") is not True:
        return {**row, "detail": f"rust load: {json.dumps(results.get('load'))[:300]}"}
    rust = results.get("reason", {})
    if "error" in rust:
        return {**row, "detail": f"rust reason: {str(rust['error'])[:300]}"}

    # --- Python, over the same store, so the labels are the ones in canon.nt. -
    py_dir = workdir / "python"
    t0 = time.time()
    try:
        py = run_horn(store, certificate_dir=py_dir)
    except Exception as exc:
        return {**row, "detail": f"python reason: {type(exc).__name__}: {str(exc)[:250]}"}
    python_seconds = time.time() - t0

    row.update(
        asserted=asserted_python,
        derived_python=py.derived_triples,
        derived_rust=rust.get("derived_triples"),
        refused_python=py.skipped_unserialisable,
        refused_rust=rust.get("skipped_unserialisable"),
        iterations_python=py.iterations,
        iterations_rust=rust.get("iterations"),
        by_rule={r["name"] + "#" + str(r["index"]): r["derivations"] for r in py.by_rule},
        seconds={
            "parse": round(parse_seconds, 2),
            "python": round(python_seconds, 2),
            "rust": round(rust_seconds, 2),
        },
    )

    # --- Preconditions, before any comparison of conclusions. ---------------
    py_asserted = set((py_dir / "asserted.tsv").read_text("utf-8").split("\n")) - {""}
    rust_asserted = set((rust_dir / "asserted.tsv").read_text("utf-8").split("\n")) - {""}
    if py_asserted != rust_asserted:
        return {
            **row,
            "verdict": "INPUT_DIFFERS",
            "detail": (
                f"the two engines were handed different graphs: "
                f"{len(py_asserted - rust_asserted)} line(s) only in Python's asserted.tsv, "
                f"{len(rust_asserted - py_asserted)} only in Rust's. Nothing downstream is "
                f"comparable.\n"
                + "".join(f"                only python: {t}\n" for t in sorted(py_asserted - rust_asserted)[:3])
                + "".join(f"                only rust:   {t}\n" for t in sorted(rust_asserted - py_asserted)[:3])
            ).rstrip(),
        }

    if (py_dir / "rules.tsv").read_bytes() != (rust_dir / "rules.tsv").read_bytes():
        return {
            **row,
            "verdict": "VERDICT_DIFFERS",
            "detail": "the two certificates cite different rule tables",
        }

    if py.fixpoint_reached is not True or rust.get("fixpoint_reached") is not True:
        return {
            **row,
            "verdict": "INCOMPLETE",
            "detail": (
                f"an iteration cap was reached, so a derived count is a lower bound and not the "
                f"closure (python fixpoint {py.fixpoint_reached} after {py.iterations}, rust "
                f"fixpoint {rust.get('fixpoint_reached')} after {rust.get('iterations')})"
            ),
        }

    # --- The checker judges both, before their contents are compared. -------
    py_check = check_horn(py_dir, timeout=float(timeout))
    rust_check = check_horn(rust_dir, timeout=float(timeout))
    row["check_python"] = py_check["status"]
    row["check_rust"] = rust_check["status"]
    row["verdict_python"] = py_check.get("verdict")
    row["verdict_rust"] = rust_check.get("verdict")

    for who, res in (("python", py_check), ("rust", rust_check)):
        if res["status"] == "rejected":
            return {
                **row,
                "verdict": "CHECKER_REJECTED",
                "detail": f"oo-horn rejected the {who} certificate: {res.get('means')}",
            }
        if res["status"] != "accepted":
            return {
                **row,
                "verdict": "ERROR",
                "detail": f"oo-horn on the {who} certificate: {res['status']} {res.get('error', '')}"[:300],
            }

    if py_check["verdict"] != rust_check["verdict"]:
        return {
            **row,
            "verdict": "VERDICT_DIFFERS",
            "detail": f"python earned {py_check['verdict']!r}, rust earned {rust_check['verdict']!r}",
        }
    if not (py_check.get("digests_agree") and rust_check.get("digests_agree")):
        return {
            **row,
            "verdict": "VERDICT_DIFFERS",
            "detail": "a rule-table digest does not match the built-in table",
        }

    # --- The comparison this tool exists for. -------------------------------
    py_concl = conclusions(py_dir / "horn.tsv")
    rust_concl = conclusions(rust_dir / "horn.tsv")
    only_py = py_concl - rust_concl
    only_rust = rust_concl - py_concl
    if only_py or only_rust:
        return {
            **row,
            "verdict": "DISAGREE",
            "detail": (
                f"{len(only_py)} triple(s) only Python derived, {len(only_rust)} only Rust. A "
                f"difference here is a defect in one of the three implementations and this tool "
                f"does not adjudicate which.\n"
                + "".join(f"                only python: {t}\n" for t in sorted(only_py)[:5])
                + "".join(f"                only rust:   {t}\n" for t in sorted(only_rust)[:5])
            ).rstrip(),
            "only_python": sorted(only_py)[:500],
            "only_rust": sorted(only_rust)[:500],
        }

    # --- Differences that are real but not defects, reported not buried. ----
    if py.skipped_unserialisable != rust.get("skipped_unserialisable"):
        return {
            **row,
            "verdict": "REFUSAL_DIFFERS",
            "detail": (
                f"identical derived sets, but python refused {py.skipped_unserialisable} "
                f"unwritable conclusion(s) and rust refused {rust.get('skipped_unserialisable')}"
            ),
        }
    rust_by_rule = {
        r["name"] + "#" + str(r["index"]): r["derivations"] for r in rust["certificate"]["by_rule"]
    }
    if row["by_rule"] != rust_by_rule:
        differing = sorted(k for k in row["by_rule"] if row["by_rule"][k] != rust_by_rule.get(k))
        return {
            **row,
            "verdict": "ATTRIBUTION_DIFFERS",
            "detail": (
                "identical derived sets, different per-rule credit on "
                + ", ".join(f"{k} {row['by_rule'][k]}/{rust_by_rule.get(k)}" for k in differing[:6])
                + ". A triple two rules can derive is credited to whichever fired first."
            ),
        }

    return {
        **row,
        "verdict": "AGREE",
        "detail": (
            f"{asserted_python} asserted, {len(py_concl)} derived, identical sets, "
            f"both certificates {py_check['verdict']}"
        ),
    }


def parse_check(sources, expected: int, workdir: pathlib.Path, timeout: int):
    """Does the Rust stack read the ORIGINAL documents the way Python did?

    Not a reasoning result and it never blocks. The canonical N-Triples intermediate
    deliberately takes parsing out of the reasoning comparison, which would otherwise
    hide a parser difference inside a derived-set difference. This puts it back, on its
    own, where it can be read for what it is.
    """
    script = "".join(f"load {s}\n" for s in sources)
    try:
        results, proc = batch(script, workdir / "parsedata", timeout)
    except subprocess.TimeoutExpired:
        return f"the rust load of the original timed out after {timeout}s"
    load = results.get("load", {})
    if proc.returncode != 0 or load.get("ok") is not True:
        return f"rust cannot load the original: {json.dumps(load)[:200] or proc.stderr[:200]}"
    # `batch` keys results by command name, so only a single-document case can be
    # checked this way; `worker` calls this for nothing else.
    got = load.get("triples_loaded")
    if got != expected:
        return f"rust read {got} triple(s) from the original, python read {expected}"
    return None


def worker(spec_path: str, timeout: int) -> dict:
    case = json.loads(pathlib.Path(spec_path).read_text("utf-8"))
    with tempfile.TemporaryDirectory(prefix="horndiff-") as d:
        workdir = pathlib.Path(d)
        row = run_case(case, workdir, timeout)
        if row["verdict"] == "AGREE" and row.get("asserted") and len(case["files"]) == 1:
            note = parse_check(case["files"], row["asserted"], workdir, timeout)
            if note:
                row["verdict"] = "PARSE_DIFFERS"
                row["detail"] = note + (
                    " (the reasoning comparison is unaffected: both engines read the "
                    "canonical N-Triples)"
                )
        return row


def is_coverage_fixture(path: pathlib.Path) -> bool:
    """Is this one of the graphs written to make a silent rule fire?

    The exclusion is by DIRECTORY and is applied in `discover`, so a fixture can never
    be counted as a corpus document by accident. Mixing the two would inflate the
    coverage figure with graphs written to hit it, which is measuring the ruler.
    """
    try:
        return COVERAGE_DIR.resolve() in path.resolve().parents
    except OSError:
        return False


def discover(root: pathlib.Path) -> list:
    """Every RDF document the repository tracks, EXCEPT the coverage fixtures.
    `git ls-files` rather than a walk, so build output and scratch files cannot inflate
    the corpus count."""
    try:
        out = subprocess.run(
            ["git", "-C", str(root), "ls-files", *(f"*{e}" for e in EXTENSIONS)],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.split("\n")
        files = [root / f for f in out if f.strip()]
        if files:
            return sorted(f for f in files if not is_coverage_fixture(f))
    except (subprocess.CalledProcessError, FileNotFoundError):
        pass
    return sorted(
        p for e in EXTENSIONS for p in root.rglob(f"*{e}") if not is_coverage_fixture(p)
    )


def coverage_cases() -> list:
    """One case per file in `tests/fixtures/horn-coverage/`.

    The target rule is read off the FILENAME, so the check below is against what the
    fixture was named for rather than against whatever it turned out to do. A fixture
    that fires nothing, or that fires a rule other than its name, is reported and fails
    the run: in the first case the rule cannot be made to fire and that is a finding
    about the engine, in the second the fixture is not the minimal graph it claims.
    """
    if not COVERAGE_DIR.is_dir():
        return []
    return [
        {"name": COVERAGE_PREFIX + p.name, "files": [str(p)], "rule": p.stem}
        for p in sorted(COVERAGE_DIR.iterdir())
        if p.suffix in EXTENSIONS
    ]


def audit_coverage(rows: list) -> tuple:
    """Did each coverage fixture fire the rule it is named for, and only that rule?

    Returns `(lines, failures)`: one report line per fixture, and the fixtures that did
    not do what their name says. `by_rule` keys are `name#index`, and two rows of the
    built-in table share the name `scm-eqc1`, so the comparison is on the NAME half.
    """
    lines, failures = [], []
    for row in sorted(rows, key=lambda r: r["name"]):
        want = row["name"][len(COVERAGE_PREFIX):].rsplit(".", 1)[0]
        fired = {k.split("#")[0]: v for k, v in (row.get("by_rule") or {}).items() if v}
        note = f"{row['verdict']}, derived {row.get('derived_python')}"
        if row["verdict"] not in COMPARED:
            state = "NOT COMPARED"
            failures.append((row["name"], f"{row['verdict']}: {row['detail']}"))
        elif want not in fired:
            state = "DID NOT FIRE"
            failures.append(
                (row["name"], f"fired {sorted(fired) or 'nothing'}, not {want}. Either the "
                 f"fixture does not instantiate the rule or the engine cannot reach it")
            )
        elif len(fired) > 1:
            state = "FIRED MORE THAN ITS RULE"
            failures.append(
                (row["name"], f"fired {sorted(fired)}; a coverage fixture is supposed to be "
                 f"the SMALLEST graph that makes {want} fire")
            )
        else:
            state = f"fires {want} x{fired[want]}"
        lines.append(f"    {row['name']:<28} {state:<26} ({note})")
    return lines, failures


def bundle_cases(files, root: pathlib.Path) -> list:
    """Directory bundles, area bundles, and the pairings the repository ships apart."""
    by_dir, by_area = {}, {}
    for f in files:
        parts = f.relative_to(root).parts
        if parts[0] not in AREAS or len(parts) < 2:
            continue
        by_dir.setdefault("/".join(parts[:2]), []).append(f)
        by_area.setdefault(parts[0], []).append(f)

    cases, seen = [], set()

    def add(name, group):
        key = tuple(sorted(str(p) for p in group))
        if len(group) < 2 or key in seen:
            return
        seen.add(key)
        cases.append(
            {"name": f"bundle {name} [{len(group)} files]", "files": [str(p) for p in group]}
        )

    for key, group in sorted(by_dir.items()):
        add(key, group)
    # An area whose files all live in one subdirectory would repeat that bundle under a
    # second name; `seen` drops it rather than double-counting its derivations.
    for key, group in sorted(by_area.items()):
        add(key + "/*", group)
    for name, dirs in CROSS_BUNDLES.items():
        add(name, [p for d in dirs for p in by_dir.get(d, [])])
    return cases


def skip_loudly(missing: list) -> int:
    lines = [
        "",
        "=" * 74,
        "SKIPPED_FIXTURE: the differential cannot run on this machine.",
        "",
        "This is NOT agreement between the three implementations. It is the absence of",
        "two of them.",
        "",
    ]
    lines += [f"  missing: {m}" for m in missing]
    lines += [
        "",
        "  cargo build --release --bin open-ontologies      (or set OPEN_ONTOLOGIES_BIN)",
        "  cd lean && lake build                            (or set OO_HORN)",
        "",
        "Set HORN_DIFF_REQUIRE_TOOLS=1 to make this a failure instead of a skip, as",
        "OO_REQUIRE_FIXTURES=1 does in the Rust suite.",
        "=" * 74,
        "",
    ]
    print("\n".join(lines), flush=True)
    if os.environ.get("HORN_DIFF_REQUIRE_TOOLS") == "1":
        print("HORN_DIFF_REQUIRE_TOOLS=1, so this is a failure, not a skip.", flush=True)
        return 2
    return 0


def preflight() -> list:
    missing = []
    if not pathlib.Path(BIN).is_file():
        found = shutil.which("open-ontologies")
        if found:
            globals()["BIN"] = found
        else:
            missing.append(f"the Rust engine, looked for {BIN}")
    try:
        import pyoxigraph  # noqa: F401

        from open_ontologies_lite.horn.certify import CheckerUnavailable, find_checker
    except ImportError as exc:
        missing.append(f"the Python package: {exc}")
        return missing
    try:
        find_checker("oo-horn")
    except CheckerUnavailable as exc:
        missing.append(f"the Lean checker: {str(exc)[:200]}")
    return missing


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=str(REPO))
    ap.add_argument("--json")
    ap.add_argument("--limit", type=int)
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 4) // 2))
    ap.add_argument("--timeout", type=int, default=TIMEOUT)
    ap.add_argument("--max-bytes", type=int, default=MAX_BYTES)
    ap.add_argument("--include", action="append", default=[])
    ap.add_argument("--exclude", action="append", default=[])
    ap.add_argument("--bundles", action="store_true")
    ap.add_argument("--no-singles", action="store_true")
    ap.add_argument(
        "--no-coverage",
        action="store_true",
        help="skip tests/fixtures/horn-coverage/, reproducing the corpus-only figure",
    )
    ap.add_argument("--verbose", action="store_true")
    ap.add_argument("--one", help=argparse.SUPPRESS)
    args = ap.parse_args()

    if args.one:
        print(json.dumps(worker(args.one, args.timeout)))
        return 0

    missing = preflight()
    if missing:
        return skip_loudly(missing)

    root = pathlib.Path(args.root).resolve()
    files = discover(root)
    if args.include:
        files = [c for c in files if any(s in str(c) for s in args.include)]
    if args.exclude:
        files = [c for c in files if not any(s in str(c) for s in args.exclude)]

    cases = [] if args.no_singles else [{"name": str(f.relative_to(root)), "files": [str(f)]} for f in files]
    if args.bundles:
        cases += bundle_cases(files, root)
    if args.limit:
        cases = cases[: args.limit]
    # After the limit, deliberately: a `--limit` that silently dropped the coverage
    # fixtures would print a coverage figure that is about a truncated corpus while
    # reading like the real one.
    if not args.no_coverage:
        cases += coverage_cases()
    if not cases:
        print("no RDF documents found: the differential ran over nothing", flush=True)
        return 2

    rows, not_run, to_run = [], [], []
    for c in cases:
        size = sum(pathlib.Path(f).stat().st_size for f in c["files"])
        if size > args.max_bytes:
            not_run.append(
                {"name": c["name"], "reason": f"{size/1e6:.1f} MB over the {args.max_bytes/1e6:.1f} MB cap"}
            )
            continue
        to_run.append(c)

    start = time.time()
    specdir = pathlib.Path(tempfile.mkdtemp(prefix="horndiff-specs-"))

    def one(index_case):
        i, case = index_case
        spec = specdir / f"case-{i}.json"
        spec.write_text(json.dumps(case), "utf-8")
        # A child process per case: a hard wall-clock timeout the parent can enforce,
        # and an engine that exhausts memory takes down its own process rather than the
        # run.
        try:
            proc = subprocess.run(
                [sys.executable, str(pathlib.Path(__file__).resolve()), "--one", str(spec),
                 "--timeout", str(args.timeout)],
                capture_output=True,
                text=True,
                timeout=args.timeout + 120,
            )
        except subprocess.TimeoutExpired:
            return {"name": case["name"], "verdict": "NOT_RUN",
                    "detail": f"timed out after {args.timeout}s"}
        if proc.returncode != 0 or not proc.stdout.strip():
            return {"name": case["name"], "verdict": "NOT_RUN",
                    "detail": f"worker exit {proc.returncode}: "
                              f"{(proc.stderr or proc.stdout).strip()[-300:]}"}
        return json.loads(proc.stdout.strip().split("\n")[-1])

    # `as_completed` and not `map`: results are printed as they land. With `map` a
    # single slow case blocks every line behind it, so a run that is killed part way
    # through prints nothing at all, which is how the first version of this tool lost
    # 290 finished comparisons to two unfinished ones.
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        futures = [pool.submit(one, ic) for ic in enumerate(to_run)]
        for future in as_completed(futures):
            row = future.result()
            if row["verdict"] == "NOT_RUN":
                not_run.append({"name": row["name"], "reason": row["detail"]})
                print(f"{'NOT_RUN':<20} {row['name']}\n                {row['detail']}", flush=True)
                continue
            rows.append(row)
            if row.get("dropped"):
                print(f"{'DROPPED_MEMBER':<20} {row['name']}", flush=True)
                for d in row["dropped"]:
                    print(f"                {d}", flush=True)
            if row["verdict"] != "AGREE" or args.verbose:
                print(f"{row['verdict']:<20} {row['name']}\n                {row['detail']}", flush=True)
            else:
                print(f"{row['verdict']:<20} {row['name']}  ({row['detail']})", flush=True)
    shutil.rmtree(specdir, ignore_errors=True)
    # The empty-corpus check above runs before the size cap and the workers, so
    # a corpus whose every case was capped, timed out or crashed reached the
    # exit at the bottom with zero rows and returned 0 (#320). Nothing compared
    # is not agreement.
    if not rows:
        print(f"all {len(cases)} case(s) were NOT_RUN: the differential compared nothing", flush=True)
        return 2

    counts = {}
    for r in rows:
        counts[r["verdict"]] = counts.get(r["verdict"], 0) + 1
    coverage_rows = [r for r in rows if r["name"].startswith(COVERAGE_PREFIX)]
    compared = [
        r
        for r in rows
        if r["verdict"] in COMPARED and not r["name"].startswith(COVERAGE_PREFIX)
    ]
    singles = [r for r in compared if not r["name"].startswith("bundle ")]
    bundles = [r for r in compared if r["name"].startswith("bundle ")]

    def total(group, field):
        return sum(r.get(field, 0) for r in group)

    def histogram(group):
        out = {}
        for r in group:
            for k, v in (r.get("by_rule") or {}).items():
                out[k] = out.get(k, 0) + v
        return out

    # Two histograms, never one. `by_rule` is the corpus of real ontologies; the
    # fixtures are graphs written to make a named rule fire and are counted apart.
    compared_fixtures = [r for r in coverage_rows if r["verdict"] in COMPARED]
    by_rule = histogram(compared)
    by_rule_fixture = histogram(compared_fixtures)

    print("\n" + "=" * 74)
    print("THREE-WAY DIFFERENTIAL: rust engine, python engine, lean checker")
    # Single documents and bundles are reported SEPARATELY and the two derived counts
    # must not be added: a bundle re-derives most of what its members derive alone, so
    # one total over both would count the same conclusion several times and read as
    # more evidence than there is.
    print(f"  single documents        {len(singles):>6}   asserted {total(singles, 'asserted'):>7}"
          f"   derived {total(singles, 'derived_python'):>7}"
          f"   refused {total(singles, 'refused_python'):>6}")
    print(f"  bundles of those        {len(bundles):>6}   asserted {total(bundles, 'asserted'):>7}"
          f"   derived {total(bundles, 'derived_python'):>7}"
          f"   refused {total(bundles, 'refused_python'):>6}")
    print("                                   (a bundle re-derives much of what its members "
          "derive alone:")
    print("                                    the two rows overlap and must not be summed)")
    print(f"  coverage fixtures       {len(coverage_rows):>6}   "
          f"(tests/fixtures/horn-coverage/, deliberate test data, NOT corpus)")
    print(f"  derived sets identical on both engines in every case: "
          f"{all(r.get('derived_python') == r.get('derived_rust') for r in compared + compared_fixtures)}")
    print(f"  cases not run           {len(not_run):>6}")
    print(f"  wall clock              {time.time() - start:.0f}s")
    print()
    print(INDEPENDENCE_CAVEAT)
    print("\n  verdicts")
    for k in ORDER:
        if k in counts:
            print(f"    {k:<22} {counts[k]}")

    print("\n  coverage by rule (python's credit; corpus | fixture, counted apart)")
    every = sorted(set(by_rule) | set(by_rule_fixture), key=lambda k: int(k.split("#")[1]))
    silent = [k for k in every if not by_rule.get(k)]
    fixture_only = [k for k in silent if by_rule_fixture.get(k)]
    still_silent = [k for k in silent if not by_rule_fixture.get(k)]
    for k in every:
        corpus, fixture = by_rule.get(k, 0), by_rule_fixture.get(k, 0)
        if corpus:
            note = ""
        elif fixture:
            note = "   FIXTURE ONLY"
        else:
            note = "   NEVER FIRED"
        print(f"    {k:<18} {corpus:>9} | {fixture:<4}{note}")
    print(f"\n    {len(every) - len(silent)} of {len(every)} rules fired on the "
          f"repository's own RDF.")
    if coverage_rows:
        print(f"    {len(every) - len(still_silent)} of {len(every)} once "
              f"tests/fixtures/horn-coverage/ is included.")
    else:
        print("    The coverage fixtures did not run (--no-coverage), so there is no second "
              "figure.")
    if fixture_only:
        print("    COVERED ONLY BY A FIXTURE WRITTEN FOR THE PURPOSE, so these rules have "
              "been compared\n    on a graph made to fire them and on no real ontology: "
              + ", ".join(fixture_only))
    if still_silent:
        print("    THIS RUN SAYS NOTHING WHATEVER ABOUT: " + ", ".join(still_silent))

    coverage_failures = []
    if coverage_rows:
        lines, coverage_failures = audit_coverage(coverage_rows)
        print("\n  coverage fixtures, each against the rule its filename names")
        for line in lines:
            print(line)

    if not_run:
        print("\n  not run")
        for s in not_run:
            print(f"    {s['name']}\n      {s['reason']}")

    if args.json:
        with open(args.json, "w") as fh:
            json.dump(
                {
                    "rows": rows,
                    "not_run": not_run,
                    "counts": counts,
                    "totals": {
                        "single_documents": len(singles),
                        "single_asserted": total(singles, "asserted"),
                        "single_derived": total(singles, "derived_python"),
                        "single_refused": total(singles, "refused_python"),
                        "bundles": len(bundles),
                        "bundle_derived": total(bundles, "derived_python"),
                        "bundle_refused": total(bundles, "refused_python"),
                    },
                    "by_rule": by_rule,
                    "by_rule_coverage_fixtures": by_rule_fixture,
                    "rules_fired_on_corpus": len(every) - len(silent),
                    "rules_fired_including_fixtures": len(every) - len(still_silent),
                    "rules_covered_only_by_a_fixture": fixture_only,
                    "rules_never_fired": still_silent,
                    "independence_caveat": INDEPENDENCE_CAVEAT,
                },
                fh,
                indent=1,
            )

    blocking = sum(counts.get(k, 0) for k in BLOCKING)
    if blocking:
        print(f"\n  {blocking} STOP-THE-LINE result(s). One of the three implementations is wrong.")
        return 1
    if coverage_failures:
        print(f"\n  {len(coverage_failures)} coverage fixture(s) did not do what their name says.")
        for name, why in coverage_failures:
            print(f"    {name}\n      {why}")
        print("    A fixture that cannot make its rule fire is a finding about the ENGINE and "
              "belongs\n    in the report, not in a quiet skip.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
