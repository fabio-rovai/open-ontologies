#!/usr/bin/env python3
"""ORE 2015 classification benchmark: Open Ontologies against HermiT.

Both reasoners read the SAME bytes. By default every ORE input (OWL Functional
Syntax) is converted once to RDF/XML with the OWL API (Convert.java) and the
converted file is given to both. With --native the original file is given to
both instead, for when the engine reads Functional Syntax itself.

The reference is HermiT's FULL entailed named-class hierarchy (all superclasses
plus equivalent classes, see JavaReasoner.java), compared against the engine's
classification[].superclasses plus its equivalences. The TOLD closure (the
asserted named hierarchy, computed by the OWL API without a reasoner) is
subtracted from the reference to give the INFERRED-only pairs. A file whose
inferred-only set is empty is flagged told_only and kept out of the headline,
because a structural walk of the asserted axioms scores 100% on it.

Gates. A run reports no timing when its load line lacks triples_loaded or has
it at 0, when its reason line carries an error or is missing, when the process
exits non-zero, or when the driver's wall-clock cap kills it. --selftest feeds
deliberately broken engine outputs through the same code and asserts that each
gate fires.

Python 3 standard library only.
"""
import argparse
import json
import os
import shutil
import signal
import statistics
import subprocess
import sys
import tempfile
import time

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DEFAULT_CORPUS = os.path.join(SCRIPT_DIR, "corpora", "ore2015", "pool_sample")
DEFAULT_LIB = os.path.join(SCRIPT_DIR, "lib")
DEFAULT_CLASSES = os.path.join(SCRIPT_DIR, "target", "java")
DEFAULT_WORK = os.path.join(SCRIPT_DIR, "corpora", "ore2015", "converted")
DEFAULT_OUT = os.path.join(SCRIPT_DIR, "results")
JAVA_SOURCES = ("JavaReasoner.java", "Convert.java")

OWL_THING = "http://www.w3.org/2002/07/owl#Thing"
OWL_NOTHING = "http://www.w3.org/2002/07/owl#Nothing"


# ---------------------------------------------------------------------------
# Normalisation shared with compare_results.py
# ---------------------------------------------------------------------------

def norm_iri(term):
    """Engine terms come as <iri> or _:bnode. Return a bare IRI or None."""
    if not isinstance(term, str):
        return None
    t = term.strip()
    if t.startswith("<") and t.endswith(">"):
        t = t[1:-1]
    if not t or t.startswith("_:"):
        return None
    return t


def named_pair(sub, sup):
    """A comparable (sub, sup) pair, or None if it falls outside the contract."""
    a, b = norm_iri(sub), norm_iri(sup)
    if a is None or b is None or a == b:
        return None
    if a in (OWL_THING, OWL_NOTHING) or b in (OWL_THING, OWL_NOTHING):
        return None
    return (a, b)


def java_pairs(strings):
    out = set()
    for s in strings:
        sub, _, sup = s.partition(" -> ")
        p = named_pair(sub, sup)
        if p:
            out.add(p)
    return out


def engine_view(result):
    """Reduce an engine `reason --profile owl-dl` result to comparable sets."""
    pairs, dropped = set(), 0
    for entry in result.get("classification") or []:
        for sup in entry.get("superclasses") or []:
            p = named_pair(entry.get("class"), sup)
            if p:
                pairs.add(p)
            else:
                dropped += 1
    for eq in result.get("equivalences") or []:
        for a, b in ((eq.get("class_a"), eq.get("class_b")),
                     (eq.get("class_b"), eq.get("class_a"))):
            p = named_pair(a, b)
            if p:
                pairs.add(p)
    unsat = {norm_iri(c) for c in result.get("unsatisfiable_classes") or []}
    unsat.discard(None)
    unsat -= {OWL_NOTHING}
    undetermined = [c for c in result.get("undetermined_classes") or []]
    return {
        "pairs": pairs,
        "pairs_dropped_out_of_contract": dropped,
        "unsat": unsat,
        "undetermined": undetermined,
        "budget_exhausted_in": list(result.get("budget_exhausted_in") or []),
        "complete": result.get("complete"),
        "consistent": result.get("consistent"),
        "named_classes": result.get("named_classes"),
        "reason_ms": (result.get("agents") or {}).get("total_time_ms"),
    }


def compare(engine, hermit):
    """Score one engine view against one HermiT result. Pure function."""
    h_unsat = set(hermit.get("unsatisfiable") or [])
    gold = {p for p in java_pairs(hermit.get("subsumptions") or []) if p[0] not in h_unsat}
    told_raw = {p for p in java_pairs(hermit.get("told_subsumptions") or []) if p[0] not in h_unsat}
    told = told_raw & gold
    inferred = gold - told
    e_pairs = {p for p in engine["pairs"] if p[0] not in h_unsat}
    on_unsat = len(engine["pairs"]) - len(e_pairs)
    found = e_pairs & gold
    wrong = e_pairs - gold
    return {
        "gold": len(gold),
        "told": len(told),
        "told_not_entailed": len(told_raw - gold),
        "inferred_only": len(inferred),
        "engine_pairs": len(e_pairs),
        "engine_pairs_on_hermit_unsat_classes": on_unsat,
        "found": len(found),
        "found_inferred": len(e_pairs & inferred),
        "recall_all": (len(found) / len(gold)) if gold else None,
        "recall_inferred": (len(e_pairs & inferred) / len(inferred)) if inferred else None,
        "told_only": not inferred,
        "wrong": sorted("{} -> {}".format(a, b) for a, b in wrong),
        "hermit_unsat": len(h_unsat),
        "engine_unsat": len(engine["unsat"]),
        "wrong_unsat": sorted(engine["unsat"] - h_unsat),
        "missed_unsat": sorted(h_unsat - engine["unsat"]),
    }


# ---------------------------------------------------------------------------
# Engine
# ---------------------------------------------------------------------------

def parse_engine_output(stdout):
    """Pick the load and reason lines out of `batch -` output."""
    load = reason = None
    for line in stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except ValueError:
            continue
        if obj.get("command") == "load" and load is None:
            load = obj.get("result") or {}
        elif obj.get("command") == "reason":
            reason = obj.get("result") or {}
    return load, reason


def gate_engine_run(load, reason, returncode, timed_out):
    """Return (status, detail). Only status 'ok' may carry a timing."""
    if timed_out:
        return "engine_timeout", "killed at the driver's wall-clock cap"
    if load is None:
        return "load_failed", "no load line in output"
    if load.get("error"):
        return "load_failed", "load error: {}".format(load["error"])[:300]
    triples = load.get("triples_loaded")
    if not isinstance(triples, int) or triples <= 0:
        return "load_failed", "triples_loaded is {!r}".format(triples)
    if reason is None:
        return "engine_no_reason", "no reason line in output (exit {})".format(returncode)
    if reason.get("error"):
        return "reason_error", "reason error: {}".format(reason["error"])[:300]
    if returncode != 0:
        return "engine_exit", "exit code {}".format(returncode)
    return "ok", ""


def run_engine(binary, path, cap_s):
    if any(c.isspace() for c in path):
        raise SystemExit("ERROR: batch input paths may not contain whitespace: " + path)
    script = "load {}\nreason --profile owl-dl\n".format(path)
    start = time.monotonic()
    proc = subprocess.Popen([binary, "batch", "-"], stdin=subprocess.PIPE,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            text=True, start_new_session=True)
    timed_out = False
    try:
        out, err = proc.communicate(script, timeout=cap_s)
    except subprocess.TimeoutExpired:
        timed_out = True
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        out, err = proc.communicate()
    wall_ms = int((time.monotonic() - start) * 1000)
    load, reason = parse_engine_output(out or "")
    status, detail = gate_engine_run(load, reason, proc.returncode, timed_out)
    run = {
        "status": status,
        "detail": detail,
        "triples_loaded": (load or {}).get("triples_loaded"),
        "wall_ms": wall_ms if status == "ok" else None,
        "stderr_tail": (err or "")[-300:] if status != "ok" else "",
    }
    if status == "ok":
        view = engine_view(reason)
        run["view"] = view
        run["reason_ms"] = view["reason_ms"]
    return run


def run_signature(run):
    if run["status"] != "ok":
        return ("failed", run["status"])
    v = run["view"]
    return (tuple(sorted(v["pairs"])), tuple(sorted(v["unsat"])), tuple(sorted(v["undetermined"])))


# ---------------------------------------------------------------------------
# Java side
# ---------------------------------------------------------------------------

def java_cp(classes_dir, lib_dir):
    return "{}{}{}".format(classes_dir, os.pathsep, os.path.join(lib_dir, "*"))


def ensure_java_built(java, classes_dir, lib_dir):
    javac = os.path.join(os.path.dirname(os.path.realpath(shutil.which(java) or java)), "javac")
    if not os.path.exists(javac):
        javac = shutil.which("javac") or "javac"
    os.makedirs(classes_dir, exist_ok=True)
    stale = []
    for src in JAVA_SOURCES:
        s = os.path.join(SCRIPT_DIR, src)
        c = os.path.join(classes_dir, src.replace(".java", ".class"))
        if not os.path.exists(c) or os.path.getmtime(c) < os.path.getmtime(s):
            stale.append(s)
    if stale:
        r = subprocess.run([javac, "-nowarn", "-cp", os.path.join(lib_dir, "*"), "-d", classes_dir] + stale,
                           capture_output=True, text=True)
        if r.returncode != 0:
            raise SystemExit("ERROR: javac failed:\n" + r.stderr[-2000:])


def convert(java, cp, src, dst, cap_s):
    if os.path.exists(dst) and os.path.getmtime(dst) >= os.path.getmtime(src) and os.path.getsize(dst) > 0:
        return None
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    tmp = dst + ".part"
    try:
        r = subprocess.run([java, "-cp", cp, "Convert", src, tmp], capture_output=True, text=True, timeout=cap_s)
    except subprocess.TimeoutExpired:
        return "conversion timed out"
    if r.returncode != 0 or not os.path.exists(tmp) or os.path.getsize(tmp) == 0:
        return "conversion failed: " + r.stderr.strip()[-300:]
    os.replace(tmp, dst)
    return None


def run_hermit(java, cp, path, cap_s, out_json):
    start = time.monotonic()
    try:
        r = subprocess.run([java, "-cp", cp, "JavaReasoner", "hermit", path, out_json],
                           capture_output=True, text=True, timeout=cap_s)
    except subprocess.TimeoutExpired:
        return None, "hermit_timeout", None
    wall_ms = int((time.monotonic() - start) * 1000)
    if r.returncode != 0 or not os.path.exists(out_json):
        lines = [l.strip() for l in r.stderr.splitlines() if l.strip() and not l.strip().startswith("at ")
                 and not l.startswith("WARNING")]
        cause = next((l for l in reversed(lines) if "Exception" in l or "Error" in l), lines[-1] if lines else "")
        return None, "hermit_error: " + cause[:300], wall_ms
    with open(out_json) as f:
        return json.load(f), None, wall_ms


# ---------------------------------------------------------------------------
# Per-file evaluation and reporting
# ---------------------------------------------------------------------------

def evaluate_file(name, runs, hermit, hermit_error=None, hermit_wall_ms=None):
    row = {"file": name, "hermit_error": hermit_error, "runs": []}
    for r in runs:
        row["runs"].append({k: v for k, v in r.items() if k != "view"})
    first = runs[0]
    row["triples_loaded"] = first.get("triples_loaded")
    sigs = {run_signature(r) for r in runs}
    row["repeats"] = len(runs)
    row["identical_across_repeats"] = len(sigs) == 1 and all(r["status"] == "ok" for r in runs)
    ok_runs = [r for r in runs if r["status"] == "ok"]
    bad_runs = [r for r in runs if r["status"] != "ok"]
    # One failed repeat withholds the timing for the whole file: a median over
    # the runs that happened to finish would hide the ones that did not.
    timed = ok_runs if not bad_runs else []
    row["engine_wall_ms"] = int(statistics.median(r["wall_ms"] for r in timed)) if timed else None
    reason_ms = [r["reason_ms"] for r in timed if isinstance(r.get("reason_ms"), (int, float))]
    row["engine_reason_ms"] = int(statistics.median(reason_ms)) if reason_ms else None
    if hermit is not None:
        for k in ("jvm_start_ms", "load_ms", "reasoner_create_ms", "classify_ms"):
            row["hermit_" + k] = hermit.get(k)
        row["hermit_process_wall_ms"] = hermit_wall_ms
        row["hermit_consistent"] = hermit.get("consistent")

    if bad_runs:
        row["status"] = bad_runs[0]["status"]
        row["detail"] = "{} of {} runs: {}".format(len(bad_runs), len(runs), bad_runs[0]["detail"])
        if hermit is not None and hermit.get("consistent", True):
            # Show what was at stake, but score nothing: no recall, no timing.
            empty = {"pairs": set(), "unsat": set()}
            stake = compare(empty, hermit)
            for k in ("gold", "told", "inferred_only"):
                row[k] = stake[k]
        return row
    if hermit is None:
        row["status"] = "hermit_failed"
        row["detail"] = hermit_error
        return row
    if not hermit.get("consistent", True):
        engine_says = first["view"]["consistent"]
        row["engine_consistent"] = engine_says
        row["wrong_consistency"] = engine_says is True
        row["status"] = "hermit_inconsistent"
        row["detail"] = ("HermiT finds the ontology inconsistent and the engine reports "
                         "consistent: {}".format(engine_says))
        return row
    v = first["view"]
    row.update(compare(v, hermit))
    row["undetermined"] = len(v["undetermined"])
    row["budget_exhausted_in"] = v["budget_exhausted_in"]
    row["engine_complete"] = v["complete"]
    row["engine_pairs_dropped_out_of_contract"] = v["pairs_dropped_out_of_contract"]
    if len(ok_runs) > 1:
        # Recall is reported from the first run and the other runs are listed,
        # but a wrong answer from ANY run is reported: a defect that shows up
        # one time in three is still a defect.
        per = [compare(r["view"], hermit) for r in ok_runs]
        row["per_run"] = [{"engine_pairs": c["engine_pairs"], "found": c["found"],
                           "found_inferred": c["found_inferred"], "wrong": len(c["wrong"]),
                           "wrong_unsat": len(c["wrong_unsat"])} for c in per]
        row["wrong"] = sorted(set().union(*(c["wrong"] for c in per)))
        row["wrong_unsat"] = sorted(set().union(*(c["wrong_unsat"] for c in per)))
    row["status"] = "told_only" if row["told_only"] else "ok"
    return row


def headline(rows):
    scored = [r for r in rows if r.get("status") == "ok"]
    told_only = [r["file"] for r in rows if r.get("status") == "told_only"]
    failed = [(r["file"], r.get("status")) for r in rows if r.get("status") not in ("ok", "told_only")]
    gold = sum(r["gold"] for r in scored)
    inferred = sum(r["inferred_only"] for r in scored)
    macro = [r["recall_inferred"] for r in scored if r.get("recall_inferred") is not None]
    out = {
        "files_total": len(rows),
        "files_scored": len(scored),
        "files_told_only_excluded": told_only,
        "files_failed": failed,
        "micro_recall_all": (sum(r["found"] for r in scored) / gold) if gold else None,
        "micro_recall_inferred": (sum(r["found_inferred"] for r in scored) / inferred) if inferred else None,
        "macro_recall_inferred": (statistics.mean(macro) if macro else None),
        "wrong_pairs_total": sum(len(r.get("wrong") or []) for r in rows),
        "wrong_unsat_total": sum(len(r.get("wrong_unsat") or []) for r in rows),
        "wrong_consistency_files": [r["file"] for r in rows if r.get("wrong_consistency")],
        "missed_unsat_total": sum(len(r.get("missed_unsat") or []) for r in rows),
        "files_not_identical_across_repeats": [r["file"] for r in rows
                                               if r.get("status") in ("ok", "told_only")
                                               and not r["identical_across_repeats"]],
    }
    return out


def md_cell(x):
    return " ".join(str(x).split()).replace("|", "\\|")


def pct(x):
    return "n/a" if x is None else "{:.1f}%".format(100.0 * x)


def cell(x):
    return "n/a" if x is None else str(x)


def markdown(meta, rows, head):
    L = []
    L.append("# ORE 2015 classification, track `{}`".format(meta["track"]))
    L.append("")
    L.append("Generated {} by `run_ore.py`. Engine `{}`. Input: {}. Engine wall-clock cap {} s, "
             "repeats {}.".format(meta["generated"], meta["engine"],
                                  "original OWL Functional Syntax (--native)" if meta["native"]
                                  else "RDF/XML converted once by the OWL API, same file to both",
                                  meta["engine_timeout_s"], meta["repeat"]))
    L.append("")
    L.append("Engine ms is the median wall clock of the whole `batch -` process (load plus reason). "
             "Reason ms is the engine's own `agents.total_time_ms`. HermiT create and classify are "
             "`createReasoner` and `precomputeInferences` only, excluding JVM start and parsing. "
             "Gold is HermiT's full entailed named hierarchy; told is the asserted closure; inferred is "
             "gold minus told. A dash in the timing columns means the run failed a gate and its time "
             "is withheld.")
    L.append("")
    L.append("| file | triples | engine ms | reason ms | HermiT create / classify ms | gold | told | inferred "
             "| engine pairs | recall all | recall inferred | wrong | unsat H/E | undetermined "
             "| budget exhausted in | identical | status |")
    L.append("|" + "---|" * 17)
    for r in rows:
        hc = "{} / {}".format(cell(r.get("hermit_reasoner_create_ms")), cell(r.get("hermit_classify_ms")))
        L.append("| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |".format(
            r["file"], cell(r.get("triples_loaded")),
            r["engine_wall_ms"] if r.get("engine_wall_ms") is not None else "-",
            r["engine_reason_ms"] if r.get("engine_reason_ms") is not None else "-",
            hc, cell(r.get("gold")), cell(r.get("told")), cell(r.get("inferred_only")),
            cell(r.get("engine_pairs")), pct(r.get("recall_all")), pct(r.get("recall_inferred")),
            len(r["wrong"]) + len(r["wrong_unsat"]) if "wrong" in r else "n/a",
            "{}/{}".format(r["hermit_unsat"], r["engine_unsat"]) if "hermit_unsat" in r else "n/a",
            cell(r.get("undetermined")),
            ",".join(r.get("budget_exhausted_in") or []) or "none" if "budget_exhausted_in" in r else "n/a",
            "yes" if r["identical_across_repeats"] else "NO",
            md_cell(r["status"] + (" ({})".format(r["detail"]) if r.get("detail") else ""))))
    L.append("")
    L.append("## Headline (files with status ok only; told_only files excluded)")
    L.append("")
    L.append("- files scored: {} of {}".format(head["files_scored"], head["files_total"]))
    L.append("- told_only, excluded: {}".format(", ".join(head["files_told_only_excluded"]) or "none"))
    L.append("- failed a gate: {}".format(", ".join("{} ({})".format(f, s) for f, s in head["files_failed"]) or "none"))
    L.append("- micro recall, inferred-only pairs: {}".format(pct(head["micro_recall_inferred"])))
    L.append("- micro recall, all pairs: {}".format(pct(head["micro_recall_all"])))
    L.append("- wrong subsumption pairs, all files: {}".format(head["wrong_pairs_total"]))
    L.append("- wrong unsatisfiability claims, all files: {}".format(head["wrong_unsat_total"]))
    L.append("- unsatisfiable classes HermiT finds and the engine does not, all files: {}".format(
        head["missed_unsat_total"]))
    L.append("- engine reports consistent where HermiT finds inconsistency: {}".format(
        ", ".join(head["wrong_consistency_files"]) or "none"))
    L.append("- not identical across repeats: {}".format(", ".join(head["files_not_identical_across_repeats"]) or "none"))
    wrong_rows = [r for r in rows if r.get("wrong") or r.get("wrong_unsat")]
    if wrong_rows:
        L.append("")
        L.append("## Wrong answers (engine claims, HermiT rejects)")
        for r in wrong_rows:
            L.append("")
            L.append("`{}`:".format(r["file"]))
            for p in r.get("wrong", [])[:25]:
                L.append("- {}".format(p))
            if len(r.get("wrong", [])) > 25:
                L.append("- ... {} more in the JSON".format(len(r["wrong"]) - 25))
            for c in r.get("wrong_unsat", []):
                L.append("- unsatisfiable claimed: {}".format(c))
    L.append("")
    return "\n".join(L)


# ---------------------------------------------------------------------------
# Self-test: the gates must be able to fail
# ---------------------------------------------------------------------------

def selftest():
    ns = "http://selftest.example/"
    A, B, C, D = (ns + x for x in "ABCD")
    hermit = {
        "consistent": True, "unsatisfiable": [],
        # Told: A -> B -> C. Inferred by reasoning: D -> C.
        "subsumptions": ["{} -> {}".format(A, B), "{} -> {}".format(A, C), "{} -> {}".format(B, C),
                         "{} -> {}".format(D, C)],
        "told_subsumptions": ["{} -> {}".format(A, B), "{} -> {}".format(A, C), "{} -> {}".format(B, C)],
        "reasoner_create_ms": 1, "classify_ms": 1,
    }
    hermit_told_only = dict(hermit, subsumptions=hermit["told_subsumptions"])

    def reason(classification, **extra):
        r = {"named_classes": 4, "classification": classification, "equivalences": [],
             "unsatisfiable_classes": [], "undetermined_classes": [], "budget_exhausted_in": [],
             "complete": True, "consistent": True, "agents": {"total_time_ms": 5}}
        r.update(extra)
        return r

    def cls(c, *sups):
        return {"class": "<{}>".format(c), "superclasses": ["<{}>".format(s) for s in sups]}

    correct = [cls(A, B, C), cls(B, C), cls(D, C)]
    structural = [cls(A, B, C), cls(B, C)]
    load_ok = {"command": "load", "result": {"ok": True, "triples_loaded": 12}, "seq": 0}

    def lines(*objs):
        return "\n".join(json.dumps(o) for o in objs) + "\n"

    def reason_line(r):
        return {"command": "reason", "result": r, "seq": 1}

    tmp = tempfile.mkdtemp(prefix="run_ore_selftest_")
    counter = os.path.join(tmp, "counter")

    def fake(name, body):
        p = os.path.join(tmp, name)
        with open(p, "w") as f:
            f.write("#!{}\nimport sys, json, time, os\nsys.stdin.read()\n{}\n".format(sys.executable, body))
        os.chmod(p, 0o755)
        return p

    def emit(text):
        return "sys.stdout.write({!r})".format(text)

    engines = {
        "correct": fake("correct", emit(lines(load_ok, reason_line(reason(correct))))),
        "structural": fake("structural", emit(lines(load_ok, reason_line(reason(structural))))),
        "empty_classification": fake("empty_cls", emit(lines(load_ok, reason_line(reason([]))))),
        "fabricated_pair": fake("fabricated", emit(lines(load_ok, reason_line(
            reason(correct + [cls(C, A)] + [{"class": "_:b0", "superclasses": ["<{}>".format(A)]}]))))),
        "wrong_unsat": fake("wrong_unsat", emit(lines(load_ok, reason_line(
            reason(correct, unsatisfiable_classes=["<{}>".format(D)]))))),
        "empty_load": fake("empty_load", emit(lines(
            {"command": "load", "result": {"ok": True, "triples_loaded": 0}, "seq": 0},
            reason_line(reason([]))))),
        "load_error": fake("load_error", emit(lines(
            {"command": "load", "result": {"error": "Unexpected text event"}, "seq": 0},
            reason_line(reason([]))))),
        "missing_triples": fake("missing_triples", emit(lines(
            {"command": "load", "result": {"ok": True}, "seq": 0}, reason_line(reason(correct))))),
        "reason_error": fake("reason_error", emit(lines(load_ok, reason_line({"error": "boom"})))),
        "crash": fake("crash", emit(lines(load_ok)) + "\nsys.exit(101)"),
        "timeout": fake("timeout", "time.sleep(30)\n" + emit(lines(load_ok, reason_line(reason(correct))))),
        "hang_on_second": fake("hang2", "p={!r}\nn=int(open(p).read()) if os.path.exists(p) else 0\n"
                                        "open(p,'w').write(str(n+1))\n"
                                        "time.sleep(30 if n == 1 else 0)\n"
                                        "sys.stdout.write({!r})".format(
                                            counter + "2", lines(load_ok, reason_line(reason(correct))))),
        "wrong_on_second": fake("wrong2", "p={!r}\nn=int(open(p).read()) if os.path.exists(p) else 0\n"
                                          "open(p,'w').write(str(n+1))\n"
                                          "sys.stdout.write({!r} if n == 1 else {!r})".format(
                                              counter + "3",
                                              lines(load_ok, reason_line(reason(correct + [cls(C, A)]))),
                                              lines(load_ok, reason_line(reason(correct))))),
        "flaky": fake("flaky", "p={!r}\nn=int(open(p).read()) if os.path.exists(p) else 0\n"
                               "open(p,'w').write(str(n+1))\n"
                               "sys.stdout.write({!r} if n % 2 == 0 else {!r})".format(
                                   counter, lines(load_ok, reason_line(reason(correct))),
                                   lines(load_ok, reason_line(reason(structural))))),
    }
    dummy = os.path.join(tmp, "input.owl")
    open(dummy, "w").close()

    def go(name, ref=hermit, repeat=1, cap=10):
        runs = [run_engine(engines[name], dummy, cap) for _ in range(repeat)]
        return evaluate_file(name, runs, ref)

    checks = []

    def check(label, cond, got):
        checks.append((label, bool(cond), got))

    r = go("correct", repeat=3)
    check("positive control: correct engine scores 100% inferred recall, 0 wrong, ok, identical",
          r["status"] == "ok" and r["recall_inferred"] == 1.0 and not r["wrong"] and r["identical_across_repeats"]
          and r["engine_wall_ms"] is not None,
          (r["status"], r["recall_inferred"], r["wrong"], r["identical_across_repeats"]))
    r = go("structural")
    check("structural walk: 100% on told, 0% on inferred-only",
          r["recall_inferred"] == 0.0 and r["found"] == r["told"] == 3,
          (r["recall_inferred"], r["found"], r["told"]))
    r = go("empty_classification")
    check("empty classification: reported as 0 pairs and 0% recall, not as missing",
          r["status"] == "ok" and r["engine_pairs"] == 0 and r["recall_all"] == 0.0 and r["recall_inferred"] == 0.0,
          (r["status"], r["engine_pairs"], r["recall_all"]))
    r = go("fabricated_pair")
    check("fabricated pair C -> A is listed as WRONG; blank-node pair is dropped, not scored",
          r["wrong"] == ["{} -> {}".format(C, A)] and r["engine_pairs_dropped_out_of_contract"] == 1,
          (r["wrong"], r["engine_pairs_dropped_out_of_contract"]))
    r = go("wrong_unsat")
    check("engine claims D unsatisfiable, HermiT says satisfiable: listed as wrong_unsat",
          r["wrong_unsat"] == [D], r["wrong_unsat"])
    for name, want in (("empty_load", "load_failed"), ("load_error", "load_failed"),
                       ("missing_triples", "load_failed"), ("reason_error", "reason_error"),
                       ("crash", "engine_no_reason")):
        r = go(name)
        check("{}: status {} and timing withheld".format(name, want),
              r["status"] == want and r["engine_wall_ms"] is None and "recall_all" not in r,
              (r["status"], r["engine_wall_ms"]))
    t0 = time.monotonic()
    r = go("timeout", cap=1)
    check("timeout: killed at the 1 s cap, status engine_timeout, timing withheld",
          r["status"] == "engine_timeout" and r["engine_wall_ms"] is None and time.monotonic() - t0 < 10,
          (r["status"], r["engine_wall_ms"]))
    r = go("hang_on_second", repeat=3, cap=1)
    check("one repeat of three hangs: file status engine_timeout, no timing from the other two",
          r["status"] == "engine_timeout" and r["engine_wall_ms"] is None and "recall_all" not in r,
          (r["status"], r["engine_wall_ms"], r.get("detail")))
    r = go("wrong_on_second", repeat=3)
    check("a fabricated pair in the second of three runs only is still listed as WRONG",
          r["wrong"] == ["{} -> {}".format(C, A)] and r["identical_across_repeats"] is False,
          (r["wrong"], r["identical_across_repeats"]))
    r = go("flaky", repeat=3)
    check("flaky engine: identical_across_repeats is False", r["identical_across_repeats"] is False,
          r["identical_across_repeats"])
    r = go("correct", ref=dict(hermit, consistent=False, subsumptions=[], told_subsumptions=[]))
    check("inconsistent reference: an engine reporting consistent is flagged wrong_consistency",
          r["status"] == "hermit_inconsistent" and r["wrong_consistency"] is True,
          (r["status"], r.get("wrong_consistency")))
    r_told = go("structural", ref=hermit_told_only)
    check("told_only file: flagged, scores 100% recall, excluded from headline",
          r_told["status"] == "told_only" and r_told["recall_all"] == 1.0, (r_told["status"], r_told["recall_all"]))
    r_ok = go("structural")
    h = headline([r_told, r_ok, go("empty_load")])
    check("headline: told_only excluded, failed run listed, recall from the scored file only",
          h["files_scored"] == 1 and h["files_told_only_excluded"] == ["structural"]
          and h["files_failed"] == [("empty_load", "load_failed")] and h["micro_recall_inferred"] == 0.0,
          h)

    shutil.rmtree(tmp, ignore_errors=True)
    failed = 0
    for label, ok, got in checks:
        print("{}  {}".format("PASS" if ok else "FAIL", label))
        if not ok:
            failed += 1
            print("      got: {}".format(got))
    print("{} of {} self-test checks passed".format(len(checks) - failed, len(checks)))
    return 1 if failed else 0


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def select_files(args):
    order_path = os.path.join(args.corpus, args.track, "classification", "fileorder.txt")
    with open(order_path) as f:
        order = [l.strip() for l in f if l.strip()]
    if args.files:
        names = list(args.files)
    elif args.file_list:
        with open(args.file_list) as f:
            names = [l.strip() for l in f if l.strip()]
    else:
        names = order
    unknown = [n for n in names if n not in order]
    if unknown:
        raise SystemExit("ERROR: not in {} fileorder.txt: {}".format(args.track, ", ".join(unknown)))
    files_dir = os.path.join(args.corpus, "files")
    if args.max_bytes:
        names = [n for n in names if os.path.getsize(os.path.join(files_dir, n)) <= args.max_bytes]
    if args.sort_by_size:
        names.sort(key=lambda n: os.path.getsize(os.path.join(files_dir, n)))
    if args.limit:
        names = names[:args.limit]
    return names


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--track", choices=("el", "dl", "pure_dl"))
    ap.add_argument("--files", nargs="+", help="file names from the track's fileorder.txt")
    ap.add_argument("--file-list", help="text file with one ORE file name per line")
    ap.add_argument("--limit", type=int, help="take the first N files after filtering")
    ap.add_argument("--max-bytes", type=int, help="skip inputs larger than this")
    ap.add_argument("--sort-by-size", action="store_true", help="smallest inputs first")
    ap.add_argument("--engine", default=os.environ.get("OO_BIN", "open-ontologies"))
    ap.add_argument("--java", default=os.environ.get("JAVA", "java"))
    ap.add_argument("--corpus", default=DEFAULT_CORPUS, help="ORE pool_sample directory")
    ap.add_argument("--lib", default=DEFAULT_LIB, help="directory of OWL API and HermiT jars")
    ap.add_argument("--classes-dir", default=DEFAULT_CLASSES)
    ap.add_argument("--work-dir", default=DEFAULT_WORK, help="cache for converted RDF/XML")
    ap.add_argument("--out-dir", default=DEFAULT_OUT)
    ap.add_argument("--engine-timeout", type=float, default=120.0, help="seconds per engine run")
    ap.add_argument("--hermit-timeout", type=float, default=300.0, help="seconds per HermiT run")
    ap.add_argument("--repeat", type=int, default=3)
    ap.add_argument("--native", action="store_true",
                    help="feed the original OWL Functional Syntax file to both reasoners")
    ap.add_argument("--tag", default="", help="label appended to output file names")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()

    if args.selftest:
        sys.exit(selftest())
    if not args.track:
        ap.error("--track is required unless --selftest")
    if args.repeat < 1:
        ap.error("--repeat must be at least 1")

    names = select_files(args)
    if not names:
        raise SystemExit("ERROR: no files selected")
    ensure_java_built(args.java, args.classes_dir, args.lib)
    cp = java_cp(args.classes_dir, args.lib)
    os.makedirs(args.out_dir, exist_ok=True)
    scratch = tempfile.mkdtemp(prefix="run_ore_")

    rows = []
    for i, name in enumerate(names, 1):
        src = os.path.abspath(os.path.join(args.corpus, "files", name))
        if args.native:
            path = src
        else:
            path = os.path.abspath(os.path.join(args.work_dir, name.rsplit(".", 1)[0] + ".rdf"))
            err = convert(args.java, cp, src, path, args.hermit_timeout)
            if err:
                rows.append({"file": name, "status": "conversion_failed", "detail": err,
                             "identical_across_repeats": False, "engine_wall_ms": None})
                print("[{}/{}] {}: {}".format(i, len(names), name, err), file=sys.stderr)
                continue
        hermit, herr, hwall = run_hermit(args.java, cp, path, args.hermit_timeout,
                                         os.path.join(scratch, name + ".hermit.json"))
        runs = [run_engine(args.engine, path, args.engine_timeout) for _ in range(args.repeat)]
        row = evaluate_file(name, runs, hermit, herr, hwall)
        row["input"] = path
        row["input_bytes"] = os.path.getsize(path)
        rows.append(row)
        print("[{}/{}] {}: {} engine={} ms gold={} inferred={} recall_inferred={} wrong={}".format(
            i, len(names), name, row["status"], cell(row.get("engine_wall_ms")), cell(row.get("gold")),
            cell(row.get("inferred_only")), pct(row.get("recall_inferred")),
            len(row.get("wrong") or [])), file=sys.stderr)

    shutil.rmtree(scratch, ignore_errors=True)
    head = headline(rows)
    meta = {
        "generated": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "track": args.track, "engine": os.path.abspath(args.engine) if os.path.exists(args.engine) else args.engine,
        "native": args.native, "repeat": args.repeat, "engine_timeout_s": args.engine_timeout,
        "hermit_timeout_s": args.hermit_timeout, "corpus": os.path.abspath(args.corpus),
        "pair_contract": "named classes only; all entailed superclasses plus equivalents; "
                         "owl:Thing and owl:Nothing excluded; HermiT-unsatisfiable subclasses excluded",
    }
    stem = "ore_{}_{}{}".format(args.track, time.strftime("%Y%m%d_%H%M%S"),
                                "_" + args.tag if args.tag else "")
    jpath = os.path.join(args.out_dir, stem + ".json")
    mpath = os.path.join(args.out_dir, stem + ".md")
    with open(jpath, "w") as f:
        json.dump({"meta": meta, "headline": head, "files": rows}, f, indent=1)
    md = markdown(meta, rows, head)
    with open(mpath, "w") as f:
        f.write(md)
    print(md)
    print("wrote {}\nwrote {}".format(jpath, mpath), file=sys.stderr)


if __name__ == "__main__":
    main()
