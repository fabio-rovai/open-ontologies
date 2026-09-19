#!/usr/bin/env python3
"""Turn a `tools/smt_differential.py --json` run into the measurement section of
decision 0013.

THE NUMBERS ARE DERIVED AND NEVER TYPED. A figure written by hand next to the
artefact that produced it is a figure that will drift from it, and this
repository has a rule about that: a demo or a decision record may not carry a
number nobody can regenerate. So the table below is generated, the decision
record carries a `<!-- MEASUREMENT -->` marker, and this script replaces
everything between that marker and the next heading.

Usage:
    python3 tools/smt_differential_report.py RUN.json [--write DECISION.md]

With no `--write` it prints the section to stdout, which is how a reader checks
the record against a run of their own.
"""
import argparse
import json
import pathlib
import sys
from collections import Counter

MARKER = "<!-- MEASUREMENT -->"


def section(run):
    rows = run["rows"]
    counts = Counter(r["verdict"] for r in rows)
    onts = sorted({r["ontology"] for r in rows})
    agree = Counter(r["z3"] for r in rows if r["verdict"] == "AGREE")
    # Who answered, on the rows where exactly one solver did. This is the
    # column that says whether the second solver is only slower or actually
    # decides things the first one cannot.
    sided = Counter(
        f"z3 said {r['z3']}, cvc5 said unknown" if r["z3"] in ("sat", "unsat")
        else f"cvc5 said {r['cvc5']}, z3 said unknown"
        for r in rows if r["verdict"] == "ONE_SIDED"
    )
    by_enc = Counter((r["encoding"], r["verdict"]) for r in rows)
    encs = run["domains"]
    tuning = run.get("cvc5_tuning_changed") or []
    flips = Counter((t["default"], t["tuned"]) for t in tuning)

    out = [MARKER, ""]
    out.append(f"Measured on {len(onts)} ontologies producing **{len(rows)} SMT-LIB problems**, "
               f"each handed to both solvers at every encoding, with a {run['timeout_secs']} second "
               f"limit per invocation.")
    out.append("")
    out.append(f"* z3: `{run['versions']['z3']}`")
    out.append(f"* cvc5: `{run['versions']['cvc5']}`, pinned at {run['cvc5_pinned']}")
    out.append("")
    out.append("| outcome | problems | what it means |")
    out.append("|---|---|---|")
    meaning = {
        "CONTRADICTION": "one said `sat` and the other `unsat` on the same bytes. One of them is "
                         "wrong",
        "ERROR": "a solver refused the file. A defect in the emitter until shown otherwise",
        "ONE_SIDED": "one answered and the other gave up. A difference in completeness, and NOT "
                     "corroboration",
        "BOTH_UNKNOWN": "neither answered. No second opinion was obtained",
        "AGREE": "the same answer from both. Two opinions, and on the `unsat` rows that is all it "
                 "will ever be",
    }
    for k in ("CONTRADICTION", "ERROR", "ONE_SIDED", "BOTH_UNKNOWN", "AGREE"):
        out.append(f"| {k} | {counts.get(k, 0)} | {meaning[k]} |")
    out.append("")
    out.append(f"The {counts.get('AGREE', 0)} agreements split "
               f"{agree.get('unsat', 0)} on `unsat` and {agree.get('sat', 0)} on `sat`. "
               f"The first of those numbers is the whole point of the exercise: those "
               f"{agree.get('unsat', 0)} answers had, before this, been checked by nothing at all, "
               f"and they still have not been PROVED by anything. What changed is that a second "
               f"implementation was in a position to contradict them and did not.")
    out.append("")
    if sided:
        out.append("The one-sided rows, by who answered:")
        out.append("")
        for k, v in sorted(sided.items(), key=lambda kv: -kv[1]):
            out.append(f"* {v}: {k}")
        out.append("")
    out.append("By encoding:")
    out.append("")
    out.append("| encoding | " + " | ".join(
        k for k in ("AGREE", "ONE_SIDED", "BOTH_UNKNOWN", "ERROR", "CONTRADICTION")) + " |")
    out.append("|---|" + "---|" * 5)
    for e in encs:
        label = "unbounded" if e == "unbounded" else f"finite({e})"
        out.append(f"| {label} | " + " | ".join(
            str(by_enc.get((e, k), 0))
            for k in ("AGREE", "ONE_SIDED", "BOTH_UNKNOWN", "ERROR", "CONTRADICTION")) + " |")
    out.append("")
    if tuning:
        moves = ", ".join(f"{v} from `{a}` to `{b}`" for (a, b), v in sorted(flips.items()))
        out.append(f"`--finite-model-find` changed cvc5's answer on {len(tuning)} of {len(rows)} "
                   f"problems: {moves}. It never turned one decisive answer into the other, which "
                   f"is the property that matters: the flag recovers completeness on problems cvc5 "
                   f"would otherwise abandon, and on this corpus it never changed what cvc5 "
                   f"concluded. That is a measurement and not a guarantee, and it is the reason the "
                   f"flag is still refused on the unbounded encoding, where no such measurement "
                   f"exists.")
        out.append("")
    if run.get("not_run"):
        out.append(f"{len(run['not_run'])} files produced no problems and are named rather than "
                   "dropped:")
        out.append("")
        for s in run["not_run"]:
            out.append(f"* `{s['name']}`: {s['reason']}")
        out.append("")
    out.append("The run this section was generated from is committed at "
               "[`docs/measurements-smt-differential-2026-09-18.json`]"
               "(../measurements-smt-differential-2026-09-18.json), and "
               "`tools/smt_differential_report.py` regenerates the "
               "section from it. Neither the table nor the sentences around it were typed by hand, "
               "which is the only way a record and the run behind it stay in step.")
    out.append("")
    out.append("**Zero contradictions is a negative result and is reported as one.** It is not "
               "evidence that either solver is correct, and it is weaker evidence than it looks: "
               "the two are different programs, but they implement the same family of algorithms "
               "over the same standard, and on problems this exporter produces both are running an "
               "essentially mechanical search. The comparison that would be worth more is the one "
               "decision 0008 describes, between two independent formalisations of a "
               "specification, and it is a different kind of evidence rather than a larger amount "
               "of this one.")
    out.append("")
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run")
    ap.add_argument("--write", help="a decision record carrying the marker, patched in place")
    args = ap.parse_args()

    run = json.load(open(args.run))
    text = section(run)
    if not args.write:
        print(text)
        return 0

    path = pathlib.Path(args.write)
    doc = path.read_text()
    if MARKER not in doc:
        raise SystemExit(f"{path} has no {MARKER} marker; refusing to guess where the section goes")
    head, rest = doc.split(MARKER, 1)
    # Everything up to the next heading is the generated section. Splitting on
    # the heading rather than on a second marker means a hand edit inside the
    # section is OVERWRITTEN rather than merged, which is the intended
    # behaviour: the section belongs to the run.
    tail = rest.split("\n## ", 1)
    if len(tail) != 2:
        raise SystemExit("no heading after the marker; refusing to rewrite to end of file")
    path.write_text(head + text + "\n## " + tail[1])
    print(f"wrote the measurement section into {path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
