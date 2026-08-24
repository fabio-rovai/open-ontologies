"""
Onto-Correctness Checker — a live demo of the open-world hole.

Paste (or use the preloaded) RDF, pick the vocabulary, press Run. You see two
verdicts side by side:
  * SHACL (open-world): validates the shapes it was given, and stays silent about
    any term it has no shape for. A fabricated term slips through as conforms=true.
  * Closed-world vocabulary gate (the onto_vocab_check principle): flags every
    predicate and rdf:type class whose IRI belongs to the ontology namespace but
    is not declared in the ontology.

Vocabulary term-sets (declared class/property IRIs) are precomputed from the real
public ontologies, so the Space starts instantly with no large downloads.
Full benchmark and reproducible code: https://github.com/fabio-rovai/open-ontologies
"""
import json, os
import gradio as gr
import rdflib
from rdflib import Graph, RDF, URIRef, Literal
from rdflib.namespace import SH
from pyshacl import validate

HERE = os.path.dirname(os.path.abspath(__file__))

VOCAB = {}
for key, label in [("schemaorg", "schema.org"), ("ies4", "IES4"), ("obo", "OBO (PATO+RO)")]:
    with open(os.path.join(HERE, "vocab", f"{key}.json")) as f:
        d = json.load(f)
    VOCAB[label] = {"policed": d["policed"], "declared": set(d["declared"])}

EXAMPLES = {
    "schema.org": """@prefix schema: <https://schema.org/> .
@prefix ex: <https://example.org/> .

# A valid-looking Offer with two terms that DO NOT EXIST in schema.org:
#   schema:MerchandiseOffer  (fabricated class)
#   schema:priceBracket      (fabricated predicate)
ex:offer1 a schema:Offer , schema:MerchandiseOffer ;
    schema:priceCurrency "GBP" ;
    schema:price "49.00" ;
    schema:priceBracket "mid" .
""",
    "IES4": """@prefix ies: <http://ies.data.gov.uk/ontology/ies4#> .
@prefix ex: <https://example.org/> .

# ies:hasParticipant does not exist; the real term is ies:isParticipantIn.
ex:event1 a ies:Event ;
    ies:isParticipationOf ex:person1 ;
    ies:hasParticipant ex:person1 .
""",
    "OBO (PATO+RO)": """@prefix obo: <http://purl.obolibrary.org/obo/> .
@prefix ex: <https://example.org/> .

# obo:PATO_9999999 is a well-formed but undeclared PATO id (does not exist).
ex:x1 a obo:PATO_0000462 ;
    obo:RO_0000052 ex:thing1 ;
    a obo:PATO_9999999 .
""",
}

def in_policed(iri, policed):
    return any(str(iri).startswith(p) for p in policed)

def closed_world_flags(g, policed, declared):
    flagged = set()
    for s, p, o in g:
        if in_policed(p, policed) and str(p) not in declared:
            flagged.add(str(p))
        if p == RDF.type and isinstance(o, URIRef) and in_policed(o, policed) and str(o) not in declared:
            flagged.add(str(o))
    return sorted(flagged)

def build_shapes(g, policed, declared):
    """A realistic, non-closed shapes graph: for each typed subject, require the
    REAL (declared) properties it actually uses. Clean and hallucinated graphs both
    satisfy it; the fabricated terms are unconstrained extras SHACL never inspects."""
    shapes = Graph()
    n = 0
    for subj in set(g.subjects(RDF.type, None)):
        for cls in g.objects(subj, RDF.type):
            if not (isinstance(cls, URIRef) and in_policed(cls, policed) and str(cls) in declared):
                continue
            shape = URIRef(f"https://example.org/shape/{n}"); n += 1
            shapes.add((shape, RDF.type, SH.NodeShape))
            shapes.add((shape, SH.targetClass, cls))
            for p in set(g.predicates(subj, None)):
                if in_policed(p, policed) and str(p) in declared:
                    b = URIRef(f"https://example.org/shape/{n}/p"); n += 1
                    shapes.add((shape, SH.property, b))
                    shapes.add((b, SH.path, p))
                    shapes.add((b, SH.minCount, Literal(1)))
    return shapes

def run(turtle, vocab_label):
    v = VOCAB[vocab_label]
    try:
        g = Graph().parse(data=turtle, format="turtle")
    except Exception as e:
        return f"### Parse error\n\n```\n{e}\n```", ""
    shapes = build_shapes(g, v["policed"], v["declared"])
    try:
        conforms, _, _ = validate(g, shacl_graph=shapes, inference="none", abort_on_first=False)
    except Exception as e:
        conforms = True
    flags = closed_world_flags(g, v["policed"], v["declared"])

    shacl_md = (
        "## SHACL (open-world)\n\n"
        f"**conforms = {str(conforms).lower()}**\n\n"
        + ("SHACL raised no violation. It only checks the terms a shape targets, so any "
           "fabricated term is invisible to it.\n" if conforms else
           "SHACL found a violation on a term it *does* have a shape for.\n")
    )
    if flags:
        short = "\n".join(f"- `{f.rsplit('/', 1)[-1].rsplit('#', 1)[-1]}`  ({f})" for f in flags)
        cw_md = (
            "## Closed-world vocabulary gate\n\n"
            f"**REJECTED — {len(flags)} fabricated term(s) not declared in {vocab_label}:**\n\n{short}\n"
        )
    else:
        cw_md = ("## Closed-world vocabulary gate\n\n"
                 "**PASSED — every term in this graph is declared in the ontology.**\n")
    return shacl_md, cw_md

INTRO = """# The open-world hole, live

`SHACL` is the default RDF validator, and it is **open-world**: it validates the terms a
shape targets and stays silent about everything else. So when a language model emits a
*plausible-but-nonexistent* ontology term, SHACL reports `conforms = true` and the fake
data flows on, looking exactly like clean data.

A **closed-world vocabulary gate** asks one extra question of every triple: *is this term
actually declared in the ontology?* Pick a vocabulary, keep the preloaded example (or paste
your own RDF), and press **Run** to see the two verdicts side by side.

Measured across three real vocabularies, open-world SHACL passed **300 / 300** graphs
carrying a fabricated term; the closed-world gate caught **300 / 300** with **0** false
positives. [Full benchmark and code →](https://github.com/fabio-rovai/open-ontologies/tree/main/case-studies/onto-correctness-bench)
"""

with gr.Blocks(title="Onto-Correctness Checker") as demo:
    gr.Markdown(INTRO)
    with gr.Row():
        vocab = gr.Dropdown(list(VOCAB.keys()), value="schema.org", label="Vocabulary")
    turtle = gr.Code(value=EXAMPLES["schema.org"], language="python", label="RDF (Turtle)")
    run_btn = gr.Button("Run", variant="primary")
    with gr.Row():
        shacl_out = gr.Markdown()
        cw_out = gr.Markdown()

    def load_example(v):
        return EXAMPLES.get(v, "")
    vocab.change(load_example, inputs=vocab, outputs=turtle)
    run_btn.click(run, inputs=[turtle, vocab], outputs=[shacl_out, cw_out])
    demo.load(run, inputs=[turtle, vocab], outputs=[shacl_out, cw_out])

if __name__ == "__main__":
    demo.launch()
