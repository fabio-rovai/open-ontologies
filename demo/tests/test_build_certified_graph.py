"""The committed replay graph says what its legend says (#425).

demo/build_certified_graph.py writes demo/precomputed/graph.json only after
oo-cert, oo-horn and the Isabelle checker have judged the run, and it needs
all three built, so CI's demo job cannot rerun it. What it CAN check, on a
fresh clone with nothing built, is that the committed file is the file that
script writes: every warrant on an edge is backed by the evidence block beside
it, the counts agree with the edges, the forged claim is refused and is not
also drawn as asserted or certified, and the source is the ies-core.ttl in the
repository, byte for byte.
"""

import collections
import hashlib
import json
from pathlib import Path

import demo.build_certified_graph as bcg

ROOT = Path(__file__).resolve().parents[2]
GRAPH = json.loads((ROOT / "demo" / "precomputed" / "graph.json").read_text(encoding="utf-8"))
EVIDENCE = GRAPH["evidence"]


def test_the_source_is_the_ies_core_file_in_this_repository():
    src = GRAPH["source"]
    assert src["file"] == "ies-core.ttl" and src["path"] == "benchmark/reference/ies-core.ttl"
    assert hashlib.sha256((ROOT / src["path"]).read_bytes()).hexdigest() == src["sha256"]
    assert src["triples"] == EVIDENCE["builtin_certificate"]["asserted"] == 1083


def test_every_checker_accepted_the_certificates():
    b = EVIDENCE["builtin_certificate"]
    assert b["oo_cert"] == {"exit": 0, "ok": True, "theorem": "OOCert.certificate_sound"}
    h = EVIDENCE["horn_certificate"]
    assert h["oo_horn"]["exit"] == 0 and h["oo_horn"]["verdict"] == "entailed"
    assert h["isabelle"]["exit"] == 0 and h["isabelle"]["verdict"] == "entailed"


def test_every_checker_refused_the_forged_line_and_oo_cert_named_its_rule():
    f = EVIDENCE["forged"]
    assert f["oo_cert"]["exit"] == 1 and f["oo_cert"]["ok"] is False
    assert f["oo_cert"]["rule"] in EVIDENCE["builtin_certificate"]["by_rule"]
    assert f["oo_cert"]["first_rejected"] == EVIDENCE["builtin_certificate"]["derivations"]
    assert f["oo_horn"] == {"exit": 1, "ok": False}
    assert f["isabelle"]["exit"] == 1 and f["isabelle"]["ok"] is False


def test_the_edges_are_the_ones_the_evidence_counts():
    by = collections.Counter(e["warrant"] for e in GRAPH["edges"])
    drawn = EVIDENCE["drawn"]
    # Three more than the ontology's own: the verification layer's edges
    # (file -> certificate asserted, certificate -> Lean and -> Isabelle certified).
    assert by["asserted"] == drawn["asserted"] + 1
    assert by["certified"] == drawn["certified"] + 2
    assert by["rejected"] == drawn["rejected"] == 1


def test_the_forged_edge_is_drawn_once_and_only_as_rejected():
    s, _, o = EVIDENCE["forged"]["claim"]
    warrants = [e["warrant"] for e in GRAPH["edges"] if (e["source"], e["target"]) == (s, o)]
    assert warrants == ["rejected"]


def test_every_edge_ends_on_a_drawn_node():
    nodes = {c["iri"] for c in GRAPH["classes"]}
    assert len(nodes) == len(GRAPH["classes"])
    for e in GRAPH["edges"]:
        assert e["source"] in nodes and e["target"] in nodes, e


def test_the_verification_layer_carries_the_names_graph3d_pins():
    names = {c.get("label") for c in GRAPH["classes"]}
    assert {"ies-core.ttl", "certificate", "Lean 4 · oo-cert", "Isabelle/HOL"} <= names


def test_a_forged_horn_step_changes_only_the_conclusion_object():
    step = ["5", "2", "?a", "<a>", "?c", "<c>", "<a>", "<sc>", "<c>", "<a>", "<sc>", "<b>"]
    forged = bcg.forge_horn(step, "<x>")
    assert bcg.horn_conclusion(forged) == ("<a>", "<sc>", "<x>")
    assert forged[:6] == step[:6] and forged[9:] == step[9:]


def test_the_forgery_targets_a_class_the_subject_does_not_reach():
    certified = [("<a>", "<c>")]
    asserted = {("<a>", "<b>"), ("<b>", "<c>")}
    (edge, target) = bcg.choose_forgery(certified, asserted, ["<a>", "<b>", "<c>", "<d>"])
    assert edge == ("<a>", "<c>") and target == "<d>"
