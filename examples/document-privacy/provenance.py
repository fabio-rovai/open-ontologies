#!/usr/bin/env python3
"""Provenance triples for the Studio's 3D document map.

The 3D view projects documents as hubs and entities as the bridges between
them, which answers "how does information interrelate across this corpus"
at a glance: shared entities pull documents together, and an entity typed
into disjoint classes by different documents shows up as a red knot between
the documents that disagree.

That projection needs one thing the extraction usually has and the merged
store usually loses: which document asserted each subject. This helper takes
per-document graphs and emits it.

    from provenance import emit_provenance
    store_tail = emit_provenance({"SOP-201": ttl_1, "REF-601": ttl_2})
    # append store_tail to the merged store before loading
"""

import re

PROV = "<http://www.w3.org/ns/prov#wasDerivedFrom>"


def emit_provenance(doc_graphs: dict[str, str], ns_prefix: str = ":") -> str:
    """One wasDerivedFrom triple per subject per document, plus doc labels.

    `doc_graphs` maps a document id to the Turtle body extracted from it.
    Subjects are recognised as `:name` at the start of a line, which matches
    the shape most extraction pipelines emit.
    """
    lines: list[str] = []
    for doc_id, body in doc_graphs.items():
        doc_node = f"{ns_prefix}DOC_{re.sub(r'[^A-Za-z0-9_]', '_', doc_id)}"
        lines.append(f'{doc_node} <http://www.w3.org/2000/01/rdf-schema#label> "{doc_id}" .')
        for subject in sorted(set(re.findall(r"^:(\w+)", body, re.M))):
            lines.append(f"{ns_prefix}{subject} {PROV} {doc_node} .")
    return "\n".join(lines) + "\n"
