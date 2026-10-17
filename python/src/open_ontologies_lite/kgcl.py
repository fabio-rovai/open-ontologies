"""KGCL (Knowledge Graph Change Language) diff.

A deterministic, model-free primitive: given two versions of an ontology, classify
the difference into KGCL change records (node created/deleted, node renamed,
annotation changed, edge created/deleted) and render them in KGCL text syntax.

This is scaffolding, not intelligence: it reports *what* changed between two graphs
so an orchestrator (or a human governance process) can decide what it means. No LLM,
no heuristics beyond structural triple comparison.

Reference: KGCL, https://github.com/INCATools/kgcl
"""
from __future__ import annotations

from dataclasses import dataclass, field

import pyoxigraph as ox

from .engine import resolve_format

RDF_TYPE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
LABEL_PREDICATES = {
    "http://www.w3.org/2000/01/rdf-schema#label",
    "http://www.w3.org/2004/02/skos/core#prefLabel",
}
ANNOTATION_PREDICATES = {
    "http://www.w3.org/2000/01/rdf-schema#comment",
    "http://www.w3.org/2004/02/skos/core#definition",
    "http://purl.org/dc/terms/description",
}


def _term(t) -> str:
    """Render a pyoxigraph term as a comparable/printable string."""
    if isinstance(t, ox.NamedNode):
        return t.value
    if isinstance(t, ox.BlankNode):
        return f"_:{t.value}"
    # Preserve a literal's datatype, language and escaped lexical form.
    return str(t)


def _parse(data: str, fmt: str):
    """Return (triples_set, by_subject dict) for a serialized graph."""
    triples = set()
    by_subj: dict[str, set] = {}
    for q in ox.parse(data.encode("utf-8"), format=resolve_format(fmt)):
        tr = getattr(q, "triple", q)
        s, p, o = _term(tr.subject), _term(tr.predicate), _term(tr.object)
        triples.add((s, p, o))
        # Compare RDF terms directly; labels still need their lexical value.
        by_subj.setdefault(s, set()).add((p, tr.object))
    return triples, by_subj


def _short(iri: str) -> str:
    for sep in ("#", "/"):
        if sep in iri:
            return iri.rsplit(sep, 1)[-1]
    return iri


@dataclass
class ChangeSet:
    """A classified set of KGCL changes between two ontology versions."""

    changes: list[dict] = field(default_factory=list)

    def counts(self) -> dict[str, int]:
        c: dict[str, int] = {}
        for ch in self.changes:
            c[ch["type"]] = c.get(ch["type"], 0) + 1
        return c

    def to_kgcl(self) -> str:
        """Render the change set in KGCL text syntax (one statement per line)."""
        lines = []
        for ch in self.changes:
            t = ch["type"]
            if t == "node_creation":
                lbl = f' "{ch["label"]}"' if ch.get("label") else ""
                lines.append(f'create node <{ch["node"]}>{lbl}')
            elif t == "node_deletion":
                lines.append(f'delete node <{ch["node"]}>')
            elif t == "node_rename":
                lines.append(f'rename <{ch["node"]}> from "{ch["old"]}" to "{ch["new"]}"')
            elif t == "node_annotation_change":
                lines.append(
                    f'change annotation of <{ch["node"]}> <{ch["predicate"]}> '
                    f'from "{ch["old"]}" to "{ch["new"]}"'
                )
            elif t == "edge_creation":
                lines.append(f'create edge <{ch["subject"]}> <{ch["predicate"]}> {ch["object"]}')
            elif t == "edge_deletion":
                lines.append(f'delete edge <{ch["subject"]}> <{ch["predicate"]}> {ch["object"]}')
        return "\n".join(lines)


def _label_of(triples: set, predicates: set) -> str | None:
    for p, o in triples:
        if p in predicates and isinstance(o, ox.Literal):
            return o.value
    return None


def kgcl_diff(old: str, new: str, fmt: str = "turtle") -> ChangeSet:
    """Classify the change from ``old`` to ``new`` into KGCL change records.

    Detected change types: node_creation, node_deletion, node_rename,
    node_annotation_change, edge_creation, edge_deletion.
    """
    old_t, old_s = _parse(old, fmt)
    new_t, new_s = _parse(new, fmt)
    cs = ChangeSet()

    subjects = set(old_s) | set(new_s)
    accounted: set[tuple] = set()  # (s,p,o) handled as node/rename/annotation, not raw edge

    for s in sorted(subjects):
        o_tr = old_s.get(s, set())
        n_tr = new_s.get(s, set())
        s_is_node = any(p == RDF_TYPE for p, _ in (n_tr or o_tr))

        if s not in old_s and s in new_s:
            cs.changes.append({"type": "node_creation", "node": s,
                               "label": _label_of(n_tr, LABEL_PREDICATES)})
            accounted |= {(s, p, _term(o)) for p, o in n_tr}
            continue
        if s in old_s and s not in new_s:
            cs.changes.append({"type": "node_deletion", "node": s,
                               "label": _label_of(o_tr, LABEL_PREDICATES)})
            accounted |= {(s, p, _term(o)) for p, o in o_tr}
            continue

        # Subject in both: look for label / annotation changes per predicate.
        for preds, ctype in ((LABEL_PREDICATES, "node_rename"),
                             (ANNOTATION_PREDICATES, "node_annotation_change")):
            for p in preds:
                ov = {o for q, o in o_tr if q == p}
                nv = {o for q, o in n_tr if q == p}
                if ov != nv and ov and nv:
                    old_v = next(iter(ov)); new_v = next(iter(nv))
                    rec = {"type": ctype, "node": s,
                           "old": old_v.value if isinstance(old_v, ox.Literal) else _term(old_v),
                           "new": new_v.value if isinstance(new_v, ox.Literal) else _term(new_v)}
                    if ctype == "node_annotation_change":
                        rec["predicate"] = p
                    cs.changes.append(rec)
                    accounted |= {(s, p, _term(old_v)), (s, p, _term(new_v))}

    # Remaining raw triple differences become edge creations / deletions.
    for (s, p, o) in sorted(new_t - old_t):
        if (s, p, o) not in accounted and p not in LABEL_PREDICATES | ANNOTATION_PREDICATES:
            cs.changes.append({"type": "edge_creation", "subject": s, "predicate": p, "object": o})
    for (s, p, o) in sorted(old_t - new_t):
        if (s, p, o) not in accounted and p not in LABEL_PREDICATES | ANNOTATION_PREDICATES:
            cs.changes.append({"type": "edge_deletion", "subject": s, "predicate": p, "object": o})

    return cs
