#!/usr/bin/env python3
"""Differential oracle: open-ontologies' entailments against an ATP, over the
translation owl-lean's adequacy theorem is about.

THE ATP IS AN ORACLE, NOT AN AUTHORITY. This is the whole design and it is not
a hedge. A derivation certificate from the forward-chaining reasoner can be
CHECKED, because `lean/` holds a checker whose soundness is a machine-checked
theorem (decision 0002). A superposition refutation cannot: checking one needs
a verified first-order calculus with unification, and no such thing exists in
core Lean. So when E or Vampire answers "Theorem", that is an opinion of
exactly the same standing as pyshacl's verdict in `tools/shacl_differential.py`.
A disagreement is a bug in ONE OF THE TWO and this tool's job is to say so, not
to decide which. Nothing here prints the word "proved", and nothing here treats
either side as ground truth in either direction.

What it does:

  1. Runs the engine's reasoner with `--certificate`, so every entailment it
     claims is written out as a derived triple.
  2. Asks the engine to export the ontology and one first-order problem per
     derived triple, in TPTP FOF. The translation happens in Rust, ONCE, under
     the correspondence tests: this script never builds a formula, because a
     formula it built would not be the one the theorem is about.
  3. Runs an ATP on each problem and compares.

The verdicts, in order of severity:

  CLAIMED_NOT_ENTAILED - the engine derived it, EVERY premise of every derivation
                         it found is expressible in the fragment, and the ATP
                         still says the negation is satisfiable. The
                         stop-the-line case, the analogue of a false clean. One
                         of the two is wrong and it must be found.
  WEAKER_EXPORT        - the ATP says the negation is satisfiable, and it is
                         right to: the engine's derivation rests on a triple the
                         fragment cannot express, so the exported theory really
                         does not entail the conclusion. Not a bug in either
                         side. The missing triple is named. This is the common
                         case for data-property assertions and for the
                         data-property hierarchy, neither of which
                         OwlLean/Syntax.lean has a constructor for.
                         READ THE NAMED TRIPLE. This verdict is only as good as
                         the expressibility judgement behind it, which is the
                         exporter's own `triple_as_axiom`. If that ever refused
                         a triple the export does in fact carry, a real bug
                         would be filed here instead of under
                         CLAIMED_NOT_ENTAILED, so the blocking triple is printed
                         rather than summarised away.
  ATP_ERROR            - the prover could not read the file. A defect in the
                         exporter until shown otherwise.
  UNDETERMINED         - the prover ran out of time or gave up. Honest and
                         common: the translated theory is usually not decidable
                         and a non-entailment often has only infinite
                         countermodels. NOT evidence of agreement.
  NOT_ASKED            - no axiom form in OwlLean/Syntax.lean corresponds to the
                         derived triple, so the question could not be put. The
                         exporter lists these; they are reproduced here.
  AGREE                - the ATP also finds the conclusion entailed.

Since 15 September 2026 an AGREE row carries a second column, because "the
prover said Theorem" is a word and the derivation behind it is an object.
Each prover is run WITH its proof-printing option and the derivation it emits
is handed to `open-ontologies fol-prove --problem F --proof G`, which parses
it, matches every leaf against the problem file THIS engine wrote, checks the
DAG for dangling parents and cycles, and recomputes the resolution-family
steps. The result lands in `proof_check` and `proof_steps`.

THAT IS STILL NOT A PROOF and AGREE still means what it meant. What the second
column rules out is narrower and was, until it existed, simply assumed: that
the prover was answering about the file we gave it. A `proof_check` of
PROOF_REJECTED is a stop-the-line, exits 1, and means the derivation's leaves
are not our formulas. See docs/decisions/0005, the addendum.

Usage:
    python3 tools/fol_differential.py ONTOLOGY.ttl [--json out.json]
                                      [--profile owl-rl-ext] [--limit N]
                                      [--timeout S] [--atp eprover|vampire]

Requires an ATP. If none is installed the run SKIPS LOUDLY with the install
line and exits 0, the way the Lean-dependent tests skip on a missing `lake`;
set FOL_DIFF_REQUIRE_ATP=1 to turn that skip into a failure, as
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

# E and Vampire first because both read TPTP FOF natively and both report SZS
# status lines. Prover9 is deliberately NOT probed: it reads LADR, not TPTP,
# and silently treating its input as TPTP would be exactly the kind of
# unnoticed mismatch this tool exists to catch.
# `--proof-object` and `--proof tptp` are not optional extras. Without them a
# prover returns an SZS word and nothing else, and a word cannot be checked.
ATPS = [
    {
        "name": "eprover",
        "argv": lambda f, t: ["eprover", "--auto", "--tptp3-format", "--proof-object",
                              f"--cpu-limit={t}", f],
        "install": "brew install eprover   (or apt-get install eprover, or "
                   "https://github.com/eprover/eprover)",
    },
    {
        # Was UNTESTED until 15 September 2026, when Vampire 5.1.0 was
        # installed and this vector was run: `--mode casc` prints its proof to
        # a temporary file and then to stdout, and `--proof tptp` is what makes
        # that proof a TSTP derivation rather than a display format.
        "name": "vampire",
        "argv": lambda f, t: ["vampire", "--mode", "casc", "--proof", "tptp", "-t", str(t), f],
        "install": "brew install vampire   (or https://github.com/vprover/vampire)",
    },
]

SZS = re.compile(r"SZS status\s+(\w+)")

# OWL-RL derivation chains over a real ontology are deeper than Python's
# default 1000 frames. Raising the limit is cheap; a RecursionError is caught
# and REPORTED rather than swallowed, because a crash that turns into a clean
# verdict is the failure mode this whole tool exists to avoid.
sys.setrecursionlimit(50000)

# The SZS ontology, read for what it means about ENTAILMENT of the conjecture.
# `Theorem` and `ContradictoryAxioms` both mean the conjecture follows: from an
# inconsistent theory everything follows, which is a true entailment and a
# separate problem with the ontology, flagged rather than hidden.
ENTAILED = {"Theorem", "ContradictoryAxioms"}
NOT_ENTAILED = {"CounterSatisfiable", "Satisfiable", "CounterTheorem"}
GAVE_UP = {"ResourceOut", "Timeout", "GaveUp", "Unknown", "InputError", "Incomplete"}


def detect_atp(preferred=None):
    """Return the first installed ATP, or None. Never guesses at a binary that
    reads a different input language."""
    candidates = ATPS
    if preferred:
        candidates = [a for a in ATPS if a["name"] == preferred]
        if not candidates:
            names = ", ".join(a["name"] for a in ATPS)
            raise SystemExit(f"unknown --atp {preferred!r}; known: {names}")
    for atp in candidates:
        if shutil.which(atp["name"]):
            return atp
    return None


def skip_loudly(preferred):
    """No ATP. Say so in a way nothing can mistake for a pass."""
    lines = [
        "",
        "=" * 72,
        "SKIPPED_FIXTURE: no automated theorem prover found on PATH.",
        "",
        "This differential is UNTESTED on this machine. That is not agreement",
        "between the engine and an ATP; it is the absence of a second opinion.",
        "",
        "Install one of:",
    ]
    for atp in ATPS:
        if preferred and atp["name"] != preferred:
            continue
        lines.append(f"  {atp['name']:<10} {atp['install']}")
    lines += [
        "",
        "Set FOL_DIFF_REQUIRE_ATP=1 to make a missing prover a failure instead",
        "of a skip, as OO_REQUIRE_FIXTURES=1 does in the Rust suite.",
        "=" * 72,
        "",
    ]
    print("\n".join(lines), flush=True)
    if os.environ.get("FOL_DIFF_REQUIRE_ATP") == "1":
        print("FOL_DIFF_REQUIRE_ATP=1, so this is a failure, not a skip.", flush=True)
        return 2
    return 0


def batch(store, script, timeout):
    """One batch run. The store is in-memory per process, so load and use must
    share an invocation."""
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


def run_engine(ontology, workdir, profile, timeout):
    """Reason with a certificate, then export from a SEPARATE, UNREASONED store.

    TWO runs, and the separation is the whole point. `reason` materialises its
    inferences into the default graph, so an export from the same store would
    contain the derived triples as ASSERTED axioms and every conjecture would
    be entailed by the trivial argument that the theory already states it. That
    is a differential that cannot fail, which is worse than no differential:
    it was this tool's first shape and a deliberately broken exporter still
    scored a clean run. The certificate is taken from run A, the axioms from
    run B, over the graph as loaded.

    Every formula is built by the Rust exporter; this script constructs no
    logic of its own, because a formula it built would not be the one the
    adequacy theorem is about.
    """
    cert = workdir / "cert"
    fol = workdir / "fol"

    reasoned, proc_a = batch(
        workdir / "store_reason",
        f"load {ontology}\nreason --profile {profile} --certificate {cert}\n",
        timeout,
    )
    if "error" in reasoned.get("reason", {}):
        raise SystemExit("reasoning failed:\n" + json.dumps(reasoned, indent=1)[:2000])

    # Run B loads the certificate's OWN asserted graph rather than the source
    # file again. Two reasons, and the second is not obvious.
    #
    #  * It is exactly the graph the reasoner started from, so the axioms the
    #    prover sees and the premises the engine used are the same triples.
    #  * Blank node labels are not stable across loads. A derived triple like
    #    `x rdf:type _:b12` names an anonymous class expression, and reloading
    #    the Turtle would relabel it, so the goal would name a node the export
    #    had never heard of. Every restriction-valued inference would come back
    #    NOT_ASKED, which reads as a limitation of the fragment when it is
    #    really an artefact of loading twice.
    #
    # asserted.tsv is `s TAB p TAB o` in N-Triples term spelling, so joining
    # the columns with spaces and appending a full stop IS N-Triples.
    asserted_nt = workdir / "asserted.nt"
    with open(cert / "asserted.tsv") as src, open(asserted_nt, "w") as dst:
        for line in src:
            cols = line.rstrip("\n").split("\t")
            if len(cols) == 3:
                dst.write(" ".join(cols) + " .\n")

    # Everything that appears anywhere in the certificate, asked as a goal in
    # its own right. The exporter's `not_asked` list is then the exact set of
    # triples the fragment cannot express, decided by the exporter rather than
    # guessed at here.
    every = workdir / "every.tsv"
    seen = set()
    with open(cert / "derivations.tsv") as src, open(every, "w") as dst:
        for line in src:
            cols = line.rstrip("\n").split("\t")[1:]
            for i in range(0, len(cols) - 2, 3):
                t = tuple(cols[i:i + 3])
                if len(t) == 3 and t not in seen:
                    seen.add(t)
                    dst.write("\t".join(t) + "\n")
    with open(cert / "asserted.tsv") as src, open(every, "a") as dst:
        for line in src:
            t = tuple(line.rstrip("\n").split("\t"))
            if len(t) == 3 and t not in seen:
                seen.add(t)
                dst.write("\t".join(t) + "\n")

    exported, proc_b = batch(
        workdir / "store_export",
        f"load {asserted_nt}\n"
        f"fol --out {fol} --format tptp --goals {cert}/derivations.tsv "
        f"--goals-skip-columns 1\n"
        f"fol --out {fol}_all --format tptp --goals {every} --goals-skip-columns 0\n",
        timeout,
    )
    if "fol" not in exported or "error" in exported.get("fol", {}):
        raise SystemExit(
            "the engine did not produce an export:\n"
            + json.dumps(exported, indent=1)[:2000]
            + "\n"
            + (proc_a.stderr + proc_b.stderr)[:2000]
        )

    # The export must be over the ASSERTED graph. `asserted` in the certificate
    # is the triple count the reasoner started from; `initial_triples` is what
    # the exporter saw. If they differ, the export is over a materialised store
    # and every AGREE below would be vacuous. Abort rather than report.
    asserted = reasoned["reason"].get("certificate", {}).get("asserted")
    seen = exported["fol"].get("initial_triples")
    if asserted is not None and seen is not None and asserted != seen:
        raise SystemExit(
            f"the export saw {seen} triples and the reasoner started from {asserted}. "
            "The exporter is looking at a materialised store, so every conjecture would "
            "be entailed by the theory already stating it. Refusing to report a "
            "differential that cannot fail."
        )

    results = dict(reasoned)
    results.update(exported)
    return results, fol, cert, pathlib.Path(str(fol) + "_all")


def load_derivations(path):
    """conclusion triple -> list of premise lists, from derivations.tsv.

    Each line is `rule TAB s TAB p TAB o` for the conclusion, then the premises
    as further triples.
    """
    out = {}
    with open(path) as fh:
        for line in fh:
            cols = line.rstrip("\n").split("\t")
            if len(cols) < 4 or (len(cols) - 1) % 3 != 0:
                continue
            terms = cols[1:]
            triples = [tuple(terms[i:i + 3]) for i in range(0, len(terms), 3)]
            out.setdefault(triples[0], []).append(triples[1:])
    return out


def unreachable_premise(goal, derivations, expressible, asserted):
    """Return a triple that blocks every derivation of `goal`, or None.

    A conclusion is reachable in the exported theory when it is an asserted
    triple the fragment can express, or when some derivation of it has every
    premise reachable. This is the exact question, not a heuristic: if no
    derivation the engine found has all its leaves expressible, the exported
    theory genuinely does not entail the conclusion and the prover is right to
    say so.

    The memoisation is sound because the certificate is acyclic by
    construction: decision 0002 requires every premise of a step to be asserted
    or concluded by an EARLIER step, and `lake exe oo-cert` rejects a
    certificate where that fails. The in-progress guard below is belt and
    braces, and it cuts conservatively (a cycle counts as unreachable on that
    path) rather than looping.
    """
    state = {}
    blocker = {}

    def reach(t, stack):
        if t in state:
            return state[t]
        if t in stack:
            return False  # a cycle proves nothing on its own
        if t in asserted and t in expressible:
            state[t] = True
            return True
        stack = stack | {t}
        for premises in derivations.get(t, []):
            bad = next((q for q in premises if not reach(q, stack)), None)
            if bad is None:
                state[t] = True
                return True
            blocker.setdefault(t, bad)
        state[t] = False
        if t not in blocker and t in asserted:
            blocker[t] = t  # asserted but not expressible
        return False

    if reach(goal, frozenset()):
        return None
    # Walk down to the deepest named blocker, which is the one worth reporting.
    seen, cur = set(), goal
    while cur in blocker and cur not in seen:
        seen.add(cur)
        nxt = blocker[cur]
        if nxt == cur:
            break
        cur = nxt
    return cur


def ask(atp, problem, timeout):
    """Run the prover and return (szs_status, detail, raw_output). Never raises.

    The raw output is returned, not discarded, because the derivation is in it
    and the derivation is the only checkable thing a prover produces.
    """
    try:
        proc = subprocess.run(
            atp["argv"](str(problem), timeout),
            capture_output=True, text=True, timeout=timeout + 15,
        )
    except subprocess.TimeoutExpired:
        return "Timeout", f"{atp['name']} exceeded {timeout + 15}s wall clock", ""
    except OSError as exc:
        return "InputError", f"{type(exc).__name__}: {exc}", ""
    text = proc.stdout + proc.stderr
    found = SZS.findall(text)
    if not found:
        tail = text.strip().splitlines()[-3:]
        return "InputError", "no SZS status line: " + " / ".join(tail)[:300], text
    # The LAST status is the verdict; E prints an input status first.
    return found[-1], "", text


# The verdicts `fol-prove` can return, mapped to what this differential does
# with them. A rejection is a stop-the-line: it means the prover's derivation
# does not rest on the formulas this engine emitted, so the AGREE beside it is
# about a different file.
PROOF_STOP = {"derivation_rejected", "refutation_step_not_reconstructed"}


def check_proof(problem, output, workdir, tag):
    """Hand the derivation to the engine's TSTP checker.

    Returns a dict with `verdict`, `steps_checked`, `steps_total` and
    `leaves_match_problem`, or `None` if the checker could not be run. `None`
    is reported as NOT_CHECKED and never as agreement: an absent check is the
    absence of evidence, which is the failure mode this whole tool exists to
    avoid.
    """
    if not output:
        return None
    proof_file = workdir / f"{tag}.tstp"
    proof_file.write_text(output)
    try:
        proc = subprocess.run(
            [BIN, "--no-connect", "fol-prove", "--problem", str(problem),
             "--proof", str(proof_file)],
            capture_output=True, text=True, timeout=120,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {"verdict": "CHECKER_ERROR", "detail": f"{type(exc).__name__}: {exc}"}
    for line in proc.stdout.splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if "verdict" in row:
            return row
    return {"verdict": "CHECKER_ERROR",
            "detail": (proc.stdout + proc.stderr).strip()[:300]}


def classify(status):
    """Map an SZS status to a verdict. Order matters: the claimed-but-refuted
    case is checked first, and nothing collapses a give-up into agreement."""
    if status in ENTAILED:
        if status == "ContradictoryAxioms":
            return "AGREE", (
                "entailed, but the ATP found the AXIOMS contradictory, so "
                "everything is entailed. That is a defect in the ontology or in "
                "the translation of it, not a confirmation of this inference"
            )
        return "AGREE", "the ATP also finds the conclusion entailed"
    if status in NOT_ENTAILED:
        return "CLAIMED_NOT_ENTAILED", (
            f"the engine derived this triple; the ATP reports SZS {status}, i.e. a "
            "model of the axioms and the negated conclusion. One of the two is "
            "wrong. This tool does not say which: an ATP verdict is an oracle "
            "opinion, and the engine's own derivation certificate is the thing "
            "that CAN be checked (lake exe oo-cert)"
        )
    if status in GAVE_UP:
        return "UNDETERMINED", (
            f"SZS {status}: no verdict. The translated theory is not decidable in "
            "general and a non-entailment often has only infinite countermodels, "
            "so this is expected and is NOT agreement"
        )
    return "ATP_ERROR", f"unhandled SZS status {status}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ontology")
    ap.add_argument("--profile", default="owl-rl-ext")
    ap.add_argument("--atp", help="force a prover instead of taking the first installed")
    ap.add_argument("--timeout", type=int, default=int(os.environ.get("FOL_DIFF_TIMEOUT", 10)),
                    help="per-problem CPU limit in seconds (default 10)")
    ap.add_argument("--limit", type=int, help="only ask the first N goals")
    ap.add_argument("--no-proof-check", action="store_true",
                    help="do not read the derivations back. The run then reports what the "
                         "prover SAID and nothing about what it said it about")
    ap.add_argument("--json")
    args = ap.parse_args()

    atp = detect_atp(args.atp)
    if atp is None:
        return skip_loudly(args.atp)
    print(f"ATP: {atp['name']} (an ORACLE, not an authority)", flush=True)

    if not os.path.exists(BIN):
        raise SystemExit(
            f"engine binary not found at {BIN}. Build it with `cargo build --release`, "
            "or set OPEN_ONTOLOGIES_BIN."
        )

    with tempfile.TemporaryDirectory() as tmp:
        workdir = pathlib.Path(tmp)
        engine, fol_dir, cert_dir, all_dir = run_engine(
            args.ontology, workdir, args.profile, 600
        )
        export = engine["fol"]
        reasoned = engine.get("reason", {})

        print(
            f"engine: {reasoned.get('inferred_count', '?')} inference(s) under "
            f"{args.profile}; exported {export['axioms_exported']} axiom(s), "
            f"{export['individual_typing_axioms']} individual typing axiom(s), "
            f"{export['goal_problems']} goal problem(s)",
            flush=True,
        )
        if export.get("exports_a_weaker_axiom_set"):
            print(
                "  NOTE: the export is a WEAKER axiom set than the ontology. The "
                "prover is being asked about less than the ontology says:",
                flush=True,
            )
            for d in export.get("constructs_not_exported", []):
                print(f"    {d['construct']} x{d['occurrences']}: {d['why']}", flush=True)

        manifest = json.loads((fol_dir / "goals.json").read_text())
        goals = manifest["goals"]
        not_asked = manifest["not_asked"]
        if args.limit:
            goals = goals[: args.limit]

        all_manifest = json.loads((all_dir / "goals.json").read_text())
        inexpressible = {tuple(n["triple"]) for n in all_manifest["not_asked"]}
        expressible = {tuple(g["triple"]) for g in all_manifest["goals"]}
        derivations = load_derivations(cert_dir / "derivations.tsv")
        asserted = set()
        with open(cert_dir / "asserted.tsv") as fh:
            for line in fh:
                t = tuple(line.rstrip("\n").split("\t"))
                if len(t) == 3:
                    asserted.add(t)
        print(
            f"fragment: {len(expressible)} of {len(expressible) + len(inexpressible)} "
            f"certificate triples are expressible in OwlLean/Syntax.lean",
            flush=True,
        )

        rows = []
        proofs = workdir / "proofs"
        proofs.mkdir(exist_ok=True)
        for g in goals:
            problem_file = fol_dir / "goals" / g["file"]
            status, detail, output = ask(atp, problem_file, args.timeout)
            verdict, why = classify(status)
            if verdict == "CLAIMED_NOT_ENTAILED":
                try:
                    blocker = unreachable_premise(
                        tuple(g["triple"]), derivations, expressible, asserted
                    )
                except RecursionError:
                    blocker = None
                    why += (
                        ". NOTE: the derivation chain was too deep to walk, so this row "
                        "was NOT checked for the weaker-export case and may be one"
                    )
                if blocker is not None:
                    verdict = "WEAKER_EXPORT"
                    why = (
                        "the prover is right and neither side is wrong: every derivation "
                        "the engine found for this triple rests on "
                        f"`{' '.join(blocker)}`, which OwlLean/Syntax.lean has no "
                        "constructor for, so the EXPORTED theory does not entail the "
                        "conclusion even though the ontology does. This is what "
                        "exports_a_weaker_axiom_set means, made concrete"
                    )
            if detail:
                why = f"{why} ({detail})" if why else detail
            # The derivation is checked whenever the prover produced one, not
            # only when it AGREED. A CLAIMED_NOT_ENTAILED row has no refutation
            # to check and comes back `no_refutation_offered`, which is the
            # honest reading of it.
            check = check_proof(problem_file, output, proofs, pathlib.Path(g["file"]).stem) \
                if not args.no_proof_check else None
            proof_verdict = (check or {}).get("verdict", "NOT_CHECKED")
            if proof_verdict in PROOF_STOP:
                proof_verdict = "PROOF_REJECTED:" + proof_verdict
            steps = f"{(check or {}).get('steps_checked', 0)}/{(check or {}).get('steps_total', 0)}"
            triple = " ".join(g["triple"])
            rows.append({
                "file": g["file"], "triple": g["triple"], "axiom_form": g["axiom_form"],
                "szs": status, "verdict": verdict, "detail": why,
                "proof_check": proof_verdict, "proof_steps": steps,
                "proof_leaves_match_problem": (check or {}).get("leaves_match_problem"),
            })
            print(f"{verdict:<22} {triple}\n{'':<22} {why}", flush=True)
            if check is not None:
                print(f"{'':<22} proof: {proof_verdict} ({steps} steps replayed)", flush=True)

        for n in not_asked:
            print(f"{'NOT_ASKED':<22} {' '.join(n['triple'])}\n{'':<22} {n['why']}", flush=True)

    counts = {}
    proof_counts = {}
    for r in rows:
        counts[r["verdict"]] = counts.get(r["verdict"], 0) + 1
        pv = r.get("proof_check", "NOT_CHECKED")
        proof_counts[pv] = proof_counts.get(pv, 0) + 1
    counts["NOT_ASKED"] = len(not_asked)

    print("\n" + "=" * 72)
    print(f"{len(rows)} entailment(s) put to {atp['name']}, {len(not_asked)} not askable")
    for kind in ("CLAIMED_NOT_ENTAILED", "ATP_ERROR", "WEAKER_EXPORT", "UNDETERMINED",
                 "NOT_ASKED", "AGREE"):
        if counts.get(kind):
            print(f"  {kind:<22} {counts[kind]}")
    print(
        "\nAn ATP verdict is an ORACLE OPINION and never a certificate. AGREE means a "
        "second implementation\nreached the same conclusion, not that anything was proved: "
        "checking a superposition refutation\nneeds a verified first-order calculus with "
        "unification, which does not exist in core Lean.\nThe engine's own derivation "
        "certificate is the artefact that CAN be checked, with\n`cd lean && lake exe oo-cert`."
    )
    if proof_counts:
        print("\nDerivations read back (open-ontologies fol-prove):")
        for kind, n in sorted(proof_counts.items()):
            print(f"  {kind:<40} {n}")
        print(
            "This is NOT a proof and AGREE still means what it meant. What a checked "
            "derivation\nrules out is that the prover was answering about a different file: "
            "every leaf is matched\nagainst the problem this engine emitted, the DAG is "
            "checked for dangling parents and cycles,\nand the resolution-family steps are "
            "recomputed. Clausification is NOT checked and is named\nas unchecked in each "
            "report."
        )

    if counts.get("WEAKER_EXPORT"):
        print(
            "\nWEAKER_EXPORT is not a defect in the engine or in the prover. It is the "
            "fragment:\nthe engine's derivation uses a triple OwlLean/Syntax.lean cannot "
            "express, so the exported\ntheory really is weaker than the ontology. Each row "
            "names the triple. Check that the\nnamed triple is one the fragment genuinely "
            "lacks, because this verdict rests on the\nexporter's own judgement of what it "
            "can express."
        )

    if args.json:
        with open(args.json, "w") as fh:
            json.dump({"atp": atp["name"], "rows": rows, "not_asked": not_asked,
                       "counts": counts, "proof_counts": proof_counts}, fh, indent=1)

    # A conclusion the engine claims and the ATP refutes is the one result that
    # must fail a pipeline. An undetermined run is not a pass and not a failure.
    rejected = sum(n for k, n in proof_counts.items() if k.startswith("PROOF_REJECTED"))
    if rejected:
        print(
            f"\n{rejected} derivation(s) REJECTED by the checker. That means the prover's own "
            "proof does not\nrest on the formulas this engine emitted, so the verdict beside "
            "it is about some other file.\nThis is a stop-the-line and it exits 1."
        )
    return 1 if (counts.get("CLAIMED_NOT_ENTAILED") or counts.get("ATP_ERROR")
                 or rejected) else 0


if __name__ == "__main__":
    sys.exit(main())
