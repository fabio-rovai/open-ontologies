#!/usr/bin/env python3
"""Differential oracle: Z3 against cvc5, over the SAME SMT-LIB this engine emits.

TWO ORACLES ARE STILL TWO ORACLES. This is the whole design and it is not a
hedge. Decision 0006 draws the line down the middle of the SAT/SMT family: a
MODEL is a finite object `lean/Fol/` can check, so a `sat` answer whose model
the verified checker accepts is upgraded to a certificate, while an `unsat`
answer is an ORACLE OPINION for ever, because checking a CDCL(T) refutation
needs a mechanised calculus this repository does not have. Adding a second
solver does not move that line one inch. Two solvers agreeing on `unsat` is two
opinions and not a proof, and nothing in this file prints the word "proved".

What a second solver DOES buy is the thing a single solver cannot give at any
price: the chance to notice that the first one is wrong. That is the same
argument `tools/shacl_differential.py` makes for pyshacl and
`tools/fol_differential.py` makes for E, and it is why this tool reports
disagreement rather than adjudicating it.

What it does:

  1. Reasons over the ontology with `--certificate`, in its own store.
  2. Exports one SMT-LIB problem per certificate triple from a SEPARATE,
     UNREASONED store, so a conjecture is not entailed by the theory already
     asserting it. That trap is described at length in `tools/fol_differential.py`
     and is inherited here deliberately rather than re-derived.
  3. Runs both solvers over each emitted file and compares the status lines.

Every formula is built by the Rust emitter in `src/tptp.rs`, ONCE, under the
correspondence tests. This script constructs no SMT-LIB of its own, because a
file it built would not be the file the Z3 pipeline is tested on, and a
differential over a second emitter measures the second emitter.

The verdicts, in order of severity:

  CONTRADICTION - one solver said `sat` and the other said `unsat`, on the same
                  bytes. One of the two is WRONG, or the file means something
                  different to each, and any of these is a stop-the-line event
                  that exits non-zero. This is the result the tool exists for.
  ERROR         - a solver could not read the file. A defect in the emitter
                  until shown otherwise, because both solvers read SMT-LIB 2
                  and the file is meant to be in it.
  ONE_SIDED     - one gave a verdict and the other said `unknown` or ran out of
                  time. NOT a disagreement about the problem and NOT agreement
                  either: it is a difference in completeness, counted under its
                  own name so it can never be read as corroboration.
  BOTH_UNKNOWN  - neither answered. No second opinion was obtained at all.
  AGREE         - the same answer from both.

`--cvc5-default` also runs cvc5 with no options and reports how often the
tuning changed its answer, because a comparison that quietly configures one
side is a comparison of one configuration and should say so.

Usage:
    python3 tools/smt_differential.py [ONTOLOGY.ttl ...] [--json out.json]
                                      [--domains 1,2,unbounded] [--limit N]
                                      [--max-goals N] [--timeout S]
                                      [--profile owl-rl-ext] [--cvc5-default]

With no ontology arguments the corpus is what `git ls-files` reports for RDF,
under a size cap, which is the same rule `tests/projection_monotonicity_corpus_test.rs`
uses and for the same reason: a filesystem walk also sees whatever a developer
generated locally, and the run stops being comparable between machines.

Requires both solvers. If either is missing the run SKIPS LOUDLY with the
install line and exits 0, the way the Lean-dependent tests skip on a missing
`lake`; set SMT_DIFF_REQUIRE_SOLVERS=1 to turn that skip into a failure, as
OO_REQUIRE_FIXTURES=1 does in the Rust suite.
"""
import argparse
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

BIN = os.environ.get(
    "OPEN_ONTOLOGIES_BIN",
    str(pathlib.Path(__file__).resolve().parent.parent / "target" / "release" / "open-ontologies"),
)

# The pinned cvc5. A differential whose second opinion drifts with whatever the
# machine happens to have installed is not reproducible, and the number this
# tool prints would then describe a solver nobody can name. CI installs this
# exact release by checksum; `--version` is compared against it and a mismatch
# is REPORTED rather than silently tolerated, because the disagreement counts
# below are a property of a version pair.
CVC5_PINNED = "1.3.4"

# A file larger than this is reported as not run rather than run. Both solvers
# parse the whole problem, and the export of a multi-megabyte harvest turns a
# differential into an overnight job for no extra signal.
MAX_BYTES = int(os.environ.get("SMT_DIFF_MAX_BYTES", 4 * 1024 * 1024))

SOLVERS = {
    "z3": {
        "install": "brew install z3   (or apt-get install z3, or "
                   "https://github.com/Z3Prover/z3/releases)",
    },
    "cvc5": {
        "install": f"download cvc5 {CVC5_PINNED} from "
                   f"https://github.com/cvc5/cvc5/releases/tag/cvc5-{CVC5_PINNED} "
                   f"(cvc5-macOS-arm64-static.zip or cvc5-Linux-x86_64-static.zip) and put "
                   f"the binary on PATH",
    },
}

# A bare status on its own line, exactly as `fol_solve::read_sat` reads it.
# Line-oriented and anchored, because a substring match finds the `unsat`
# inside `unsatisfiable` in a header comment, and because both solvers print
# other things first: Z3 on older Ubuntu answers `unsupported` to one preamble
# command before solving correctly, and cvc5 prints `; cardinality of U is 1`
# inside its model block.
STATUS = re.compile(r"^(sat|unsat|unknown)$", re.MULTILINE)

# Z3 PRINTS THE BARE WORD `timeout`, NOT `unknown`, WHEN `-T:` FIRES.
#
# Measured on Z3 4.16.0 over a 168 KB export of benchmark/epc/iris-building.ttl
# at finite(2): `z3 -T:5` printed `timeout` and exited 0. A first version of
# this tool matched only the three SMT-LIB statuses and filed that under ERROR,
# "the solver could not read the file", which is a different and much more
# alarming claim: it accuses the emitter of writing something that is not
# SMT-LIB. On the same file cvc5 answered `sat` in under five seconds, so the
# row that should have read "cvc5 decided a problem Z3 could not" read
# "both solvers are broken" instead. A misclassified resource limit does not
# just lose signal, it invents a defect.
# cvc5 names its own limit in a sentence rather than a keyword, measured on
# 1.3.4: `cvc5 interrupted by timeout.` with no status line at all. Both
# spellings land in `unknown`, so recognising this one changes no count; what it
# changes is whether the report says "the solver ran out of time" or "the solver
# printed nothing", and only the first of those is true.
RESOURCE_OUT = re.compile(
    r"^(timeout|unsupported|interrupted)$"
    r"|(interrupted by timeout|resource limit|out of (?:memory|resources))",
    re.MULTILINE,
)

# Both solvers report a file they cannot read as `(error "…")`, measured on
# Z3 4.16.0 and cvc5 1.3.4 over a deliberately truncated declaration. This is
# what an ERROR row is allowed to rest on: a solver SAYING it could not read
# the file, rather than this tool inferring it from silence.
PARSE_ERROR = re.compile(r'^\(error\s', re.MULTILINE)

SKIP_DIRS = ("node_modules", "target", ".lake", ".git", ".venv", "venv",
             "site-packages", "__pycache__")

# OWL-RL derivation chains over a real ontology are deeper than Python's
# default frame limit, and a RecursionError that turned into a clean verdict is
# the failure mode this whole family of tools exists to avoid.
sys.setrecursionlimit(50000)


def which(name):
    return shutil.which(name)


def solver_version(name):
    """The solver's own version string, or None. Reported, never trusted."""
    try:
        out = subprocess.run([name, "--version"], capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        return None
    first = (out.stdout + out.stderr).strip().splitlines()
    return first[0].strip() if first else None


def skip_loudly(missing):
    """A solver is missing. Say so in a way nothing can mistake for a pass."""
    lines = [
        "",
        "=" * 72,
        "SKIPPED_FIXTURE: this differential needs two SMT solvers and found one.",
        "",
        f"Missing: {', '.join(missing)}",
        "",
        "A single solver is not a second opinion. Nothing below is agreement",
        "between Z3 and cvc5; it is the absence of a comparison.",
        "",
        "Install:",
    ]
    for name in missing:
        lines.append(f"  {name:<6} {SOLVERS[name]['install']}")
    lines += [
        "",
        "Set SMT_DIFF_REQUIRE_SOLVERS=1 to make a missing solver a failure",
        "instead of a skip, as OO_REQUIRE_FIXTURES=1 does in the Rust suite.",
        "=" * 72,
        "",
    ]
    print("\n".join(lines), flush=True)
    if os.environ.get("SMT_DIFF_REQUIRE_SOLVERS") == "1":
        print("SMT_DIFF_REQUIRE_SOLVERS=1, so this is a failure, not a skip.", flush=True)
        return 2
    return 0


def batch(store, script, timeout):
    """One batch run. The store is in-memory per process, so load and use must
    share an invocation, and each ontology needs its OWN store directory: a
    reused one accumulates the triples of every file loaded before it and the
    export then describes a union nobody chose."""
    proc = subprocess.run(
        [BIN, "--no-connect", "--data-dir", str(store), "batch", "-"],
        input=script, capture_output=True, text=True, timeout=timeout,
        env=dict(os.environ, OPEN_ONTOLOGIES_STORAGE_MODE="persistent"),
    )
    results = {}
    for line in proc.stdout.splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if "command" in row:
            results[row["command"]] = row.get("result", {})
    return results, proc


def emit_problems(ontology, workdir, profile, domains, timeout, max_goals):
    """Every SMT-LIB file the engine writes for one ontology, at every encoding.

    Returns (files, note). `files` is a list of (path, encoding, kind, label).

    `max_goals` caps the GOAL LIST and not the file list, which is not the same
    thing and the difference bit once already. Capping the files truncates the
    flat list of (encoding × goal) pairs, and since the encodings are emitted
    one after another a cap below the goal count keeps only the first encoding:
    a pilot run reported 24 problems over two ontologies and every one of them
    was `finite(1)`, so the unbounded encoding had been silently dropped from a
    differential that claimed to cover it. Capping the goals keeps every
    encoding over the same questions.

    The reasoning run and the exporting run use DIFFERENT stores, which is the
    whole point and not tidiness. `reason` materialises its inferences into the
    default graph, so an export from the same store would carry the derived
    triples as asserted axioms and every goal would be unsatisfiable by the
    trivial argument that the theory already states its negation's complement.
    That is a differential that cannot fail, and it was the first shape of
    `tools/fol_differential.py`.
    """
    cert = workdir / "cert"
    reasoned, proc_a = batch(
        workdir / "store_reason",
        f"load {ontology}\nreason --profile {profile} --certificate {cert}\n",
        timeout,
    )
    if "error" in reasoned.get("reason", {}):
        return [], f"reasoning failed: {json.dumps(reasoned.get('reason'))[:300]}"

    # asserted.tsv is `s TAB p TAB o` in N-Triples term spelling, so joining the
    # columns with spaces and appending a full stop IS N-Triples. Loading the
    # certificate's own asserted graph rather than the source file again also
    # keeps blank node labels stable, which matters because a derived triple can
    # name an anonymous class expression and a second parse would relabel it.
    asserted_nt = workdir / "asserted.nt"
    asserted_path = cert / "asserted.tsv"
    if not asserted_path.exists():
        return [], "the reasoner wrote no asserted.tsv"
    with open(asserted_path) as src, open(asserted_nt, "w") as dst:
        for line in src:
            cols = line.rstrip("\n").split("\t")
            if len(cols) == 3:
                dst.write(" ".join(cols) + " .\n")

    # Every triple anywhere in the certificate, asked as a goal in its own
    # right. The exporter's own `not_asked` list is then the exact set the
    # fragment cannot express, decided by the exporter rather than guessed here.
    goals = workdir / "goals.tsv"
    seen, count, capped = set(), 0, 0
    with open(goals, "w") as dst:
        for name in ("derivations.tsv", "asserted.tsv"):
            path = cert / name
            if not path.exists():
                continue
            with open(path) as src:
                for line in src:
                    cols = line.rstrip("\n").split("\t")
                    if name == "derivations.tsv":
                        cols = cols[1:]
                    for i in range(0, max(len(cols) - 2, 0), 3):
                        t = tuple(cols[i:i + 3])
                        if len(t) == 3 and t not in seen:
                            seen.add(t)
                            count += 1
                            if max_goals and count > max_goals:
                                capped += 1
                                continue
                            dst.write("\t".join(t) + "\n")
    if count == 0:
        return [], "the certificate named no triples, so there was nothing to ask"

    files, asserted_seen = [], None
    for enc in domains:
        out = workdir / f"export_{enc}"
        flag = "" if enc == "unbounded" else f" --smt-domain {enc}"
        exported, proc_b = batch(
            workdir / f"store_export_{enc}",
            f"load {asserted_nt}\n"
            f"fol --out {out} --format smtlib{flag} --goals {goals} --goals-skip-columns 0\n",
            timeout,
        )
        rep = exported.get("fol", {})
        if "error" in rep or not rep:
            return [], (f"the engine did not export at {enc}: "
                        f"{json.dumps(rep)[:300]}{(proc_a.stderr + proc_b.stderr)[:300]}")
        # The export must be over the ASSERTED graph. `asserted` in the
        # certificate is what the reasoner started from and `initial_triples` is
        # what the exporter saw; if they differ the export is over a
        # materialised store and every AGREE below would be vacuous.
        asserted = reasoned.get("reason", {}).get("certificate", {}).get("asserted")
        saw = rep.get("initial_triples")
        if asserted is not None and saw is not None and asserted != saw:
            return [], (f"the export saw {saw} triples and the reasoner started from "
                        f"{asserted}, so the exporter is looking at a materialised store. "
                        "Refusing to report a differential that cannot fail")
        asserted_seen = saw
        main = out / "ontology.smt2"
        if main.exists():
            files.append((main, enc, "ontology", ontology.name))
        for g in sorted((out / "goals").glob("goal_*.smt2")):
            files.append((g, enc, "goal", g.stem))

    # The cap is REPORTED rather than applied in silence. A run that asked
    # fewer questions than the certificate contained and printed a clean sweep
    # would be describing a subset nobody chose.
    note = f"{count - capped} of {count} goal triple(s), {asserted_seen} asserted triple(s)"
    if capped:
        note += f", {capped} over the --max-goals cap and not asked"
    return files, note


def ask(solver, path, timeout, tuned):
    """Run one solver on one file. Returns (status, detail, argv).

    `status` is one of sat / unsat / unknown / error, the last being "the
    solver could not read the file", which is never folded into `unknown`: one
    is a statement about the problem and the other is a statement about the
    pipeline.
    """
    if solver == "z3":
        argv = ["z3", f"-T:{timeout}", str(path)]
    else:
        argv = ["cvc5", f"--tlimit={timeout * 1000}"]
        if tuned:
            # --finite-model-find, and ONLY on the bounded encoding.
            #
            # Measured on cvc5 1.3.4: with default options cvc5 answers
            # `unknown` on the engine's quantified problems, including ones
            # whose carrier is a one-element datatype, because its default
            # quantifier strategy is E-matching and E-matching is incomplete.
            # With --finite-model-find it answers `sat` and prints a structure.
            #
            # On the FINITE encoding this flag adds no assumption whatsoever:
            # `(declare-datatypes ((U 0)) ((e0) … ))` already makes the carrier
            # finite and of known size, so "look for a finite model" is the
            # question the file asks. On the UNBOUNDED encoding it would be a
            # different question, and a run that searched for finite models and
            # reported the result under an unbounded file's name would be the
            # `no_model_up_to_size_k` mistake of decision 0006 item 4 wearing a
            # solver flag instead of a cardinality constraint. So the flag is
            # keyed on what the file declares and not on the caller's wishes.
            try:
                head = path.read_text()[:4096]
            except OSError:
                head = ""
            if "declare-datatypes" in head:
                argv.append("--finite-model-find")
        argv.append(str(path))
    try:
        proc = subprocess.run(argv, capture_output=True, text=True, timeout=timeout + 20)
    except subprocess.TimeoutExpired:
        return "unknown", f"{solver} exceeded {timeout + 20}s wall clock", argv
    except OSError as exc:
        return "error", f"{type(exc).__name__}: {exc}", argv
    text = proc.stdout + proc.stderr
    found = STATUS.findall(text)
    if found:
        # The FIRST standalone status is the verdict, which is what
        # `fol_solve::read_sat` takes. cvc5 prints its model block AFTER the
        # status and that block can itself contain the token, and Z3 prints
        # `unsupported` before the status on some builds without that being a
        # status at all.
        return found[0], "", argv
    if PARSE_ERROR.search(text):
        first = PARSE_ERROR.search(text)
        line = text[first.start():].splitlines()[0]
        return "error", f"{solver} refused the file: {line[:300]}", argv
    limit = RESOURCE_OUT.search(text)
    if limit:
        named = next(g for g in limit.groups() if g)
        return "unknown", f"{solver} stopped on a resource limit: {named}", argv
    # Neither a status, nor a refusal, nor a limit it named. This is unknown
    # and NOT an error: inferring "the file is unreadable" from a solver that
    # said nothing would put the blame on the emitter without evidence.
    tail = " / ".join(text.strip().splitlines()[-3:])[:300]
    return "unknown", f"{solver} printed no status at all; it said: {tail}", argv


def classify(z, c):
    """Return (verdict, detail). Order matters: a contradiction is checked first."""
    if z == "error" or c == "error":
        who = " and ".join(n for n, s in (("z3", z), ("cvc5", c)) if s == "error")
        return "ERROR", f"{who} could not read the file"
    decisive = {"sat", "unsat"}
    if z in decisive and c in decisive:
        if z != c:
            return "CONTRADICTION", f"z3 said {z} and cvc5 said {c}"
        return "AGREE", f"both said {z}"
    if z == "unknown" and c == "unknown":
        return "BOTH_UNKNOWN", "neither solver answered"
    said, quiet = (("z3", z), ("cvc5", c)) if z in decisive else (("cvc5", c), ("z3", z))
    return "ONE_SIDED", f"{said[0]} said {said[1]}, {quiet[0]} said unknown"


def corpus_from_git(repo):
    out = subprocess.run(
        ["git", "ls-files", "-z", "*.ttl", "*.owl", "*.rdf", "*.nt"],
        cwd=repo, capture_output=True, text=True,
    )
    if out.returncode != 0:
        raise SystemExit("git ls-files failed; pass ontology paths explicitly")
    files = []
    for p in out.stdout.split("\0"):
        if not p or any(seg in SKIP_DIRS for seg in p.split("/")):
            continue
        files.append(repo / p)
    return sorted(files)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ontologies", nargs="*")
    ap.add_argument("--json")
    ap.add_argument("--domains", default="1,2,unbounded",
                    help="comma-separated: integers for the finite datatype encoding, "
                         "`unbounded` for declare-sort")
    ap.add_argument("--limit", type=int, help="at most this many ontologies")
    ap.add_argument("--max-goals", type=int, default=12,
                    help="at most this many goal triples per ontology, asked at EVERY "
                         "encoding; the rest are reported as not asked")
    ap.add_argument("--timeout", type=int, default=10, help="seconds per solver invocation")
    ap.add_argument("--profile", default="owl-rl-ext")
    ap.add_argument("--cvc5-default", action="store_true",
                    help="also run cvc5 with no options and report how often the tuning "
                         "changed its answer")
    args = ap.parse_args()

    repo = pathlib.Path(__file__).resolve().parent.parent
    missing = [n for n in SOLVERS if not which(n)]
    if missing:
        return skip_loudly(missing)
    if not pathlib.Path(BIN).exists():
        raise SystemExit(f"no engine binary at {BIN}; `cargo build --release`, or set "
                         "OPEN_ONTOLOGIES_BIN")

    versions = {n: solver_version(n) for n in SOLVERS}
    pin_note = None
    if versions["cvc5"] and CVC5_PINNED not in versions["cvc5"]:
        pin_note = (f"cvc5 on PATH is {versions['cvc5']!r} and this tool is pinned to "
                    f"{CVC5_PINNED}. The counts below are a property of a version pair, so "
                    "they are not comparable with a run of the pinned pair.")
        print(f"VERSION_DRIFT  {pin_note}", flush=True)

    domains = [d.strip() for d in args.domains.split(",") if d.strip()]
    for d in domains:
        if d != "unbounded" and not d.isdigit():
            raise SystemExit(f"--domains takes integers and `unbounded`, not {d!r}")

    onts = [pathlib.Path(p) for p in args.ontologies] or corpus_from_git(repo)
    onts = [p for p in onts if p.exists()]
    if args.limit:
        onts = onts[: args.limit]

    rows, not_run, tuning = [], [], []
    for ont in onts:
        size = ont.stat().st_size
        if size > MAX_BYTES:
            reason = f"{size / 1e6:.1f} MB over the {MAX_BYTES / 1e6:.0f} MB cap"
            not_run.append({"name": str(ont), "reason": reason})
            print(f"{'NOT_RUN':<14} {ont}\n               {reason}", flush=True)
            continue
        with tempfile.TemporaryDirectory() as d:
            work = pathlib.Path(d)
            try:
                files, note = emit_problems(ont, work, args.profile, domains,
                                            max(args.timeout * 6, 120), args.max_goals)
            except subprocess.TimeoutExpired:
                files, note = [], "the engine timed out exporting"
            if not files:
                not_run.append({"name": str(ont), "reason": note})
                print(f"{'NOT_RUN':<14} {ont}\n               {note}", flush=True)
                continue
            print(f"{'EXPORTED':<14} {ont}  {len(files)} problem(s), {note}", flush=True)
            for path, enc, kind, label in files:
                z, zd, zargv = ask("z3", path, args.timeout, True)
                c, cd, cargv = ask("cvc5", path, args.timeout, True)
                verdict, detail = classify(z, c)
                row = {
                    "ontology": str(ont.relative_to(repo) if ont.is_absolute()
                                    and repo in ont.parents else ont),
                    "encoding": enc, "kind": kind, "label": label,
                    "z3": z, "cvc5": c, "verdict": verdict, "detail": detail,
                    "z3_detail": zd, "cvc5_detail": cd,
                    "cvc5_argv": " ".join(cargv[:-1]),
                }
                if args.cvc5_default:
                    c0, _, _ = ask("cvc5", path, args.timeout, False)
                    row["cvc5_default"] = c0
                    if c0 != c:
                        tuning.append({"file": str(path), "encoding": enc,
                                       "tuned": c, "default": c0})
                rows.append(row)
                if verdict != "AGREE":
                    why = "; ".join(d for d in (zd, cd) if d)
                    print(f"{verdict:<14} {enc} {kind} {label}\n               {detail}"
                          + (f"\n               {why}" if why else ""), flush=True)

    counts = {}
    for r in rows:
        counts[r["verdict"]] = counts.get(r["verdict"], 0) + 1

    print("\n" + "=" * 72)
    print(f"{len(rows)} problem(s) over {len(onts) - len(not_run)} ontology(ies), "
          f"{len(not_run)} not run")
    print(f"z3:   {versions['z3']}")
    print(f"cvc5: {versions['cvc5']}  (pinned {CVC5_PINNED})")
    for k in ("CONTRADICTION", "ERROR", "ONE_SIDED", "BOTH_UNKNOWN", "AGREE"):
        print(f"  {k:<14} {counts.get(k, 0)}")
    for r in rows:
        if r["verdict"] == "CONTRADICTION":
            print(f"  contradiction: {r['ontology']} {r['encoding']} {r['label']}: "
                  f"{r['detail']}")
    if args.cvc5_default:
        print(f"  cvc5 tuning changed the answer on {len(tuning)} of {len(rows)} problem(s)")
    for s in not_run:
        print(f"  not run: {s['name']} - {s['reason']}")

    # THE ANTI-VACUITY GUARD, and it is not belt and braces.
    #
    # A run in which no problem got a decisive answer from BOTH solvers has
    # compared nothing, and every counter above would read as a clean sweep. A
    # differential that cannot fail is worse than no differential: decision
    # 0005 item 7 records this repository shipping one, and the shape it took
    # was exactly a green run over a corpus that had quietly become trivial.
    decisive = sum(1 for r in rows
                   if r["z3"] in ("sat", "unsat") and r["cvc5"] in ("sat", "unsat"))
    if rows and decisive == 0:
        print("\nNO_SIGNAL: not one problem got a decisive answer from both solvers, so "
              "nothing was compared. This is not agreement.", flush=True)
    if not rows:
        print("\nNO_SIGNAL: no problem was emitted at all.", flush=True)

    if args.json:
        json.dump({
            "rows": rows, "not_run": not_run, "counts": counts,
            "versions": versions, "cvc5_pinned": CVC5_PINNED, "version_drift": pin_note,
            "domains": domains, "timeout_secs": args.timeout,
            "problems": len(rows), "decisive_both": decisive,
            "cvc5_tuning_changed": tuning if args.cvc5_default else None,
        }, open(args.json, "w"), indent=1)

    # A contradiction is the one result that must fail a pipeline. A run that
    # compared nothing must fail it too, for the reason above.
    if counts.get("CONTRADICTION"):
        return 1
    if not rows or decisive == 0:
        return 3
    return 0


if __name__ == "__main__":
    sys.exit(main())
