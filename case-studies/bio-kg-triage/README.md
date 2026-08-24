# bio-kg-triage

**An ontology-grounded biomedical knowledge graph, validated: 0 hallucinated terms across a
real gene-disease KG, with a provenance-carrying hypothesis triage.**

Ungrounded RAG over the literature lets a language model assert edges with nothing checking
that the node types and predicates are real. This case study does the opposite: every edge is
typed with the real [Biolink Model](https://github.com/biolink/biolink-model) vocabulary and
passes a closed-world vocabulary gate (the [`onto_vocab_check`](../../README.md) principle)
before it enters the graph. It reuses, unchanged, the correctness gate benchmarked on
schema.org and IES4 in [onto-correctness-bench](../onto-correctness-bench/README.md), now on
the biomedical vocabulary and on real data.

Built for Encode / ARIA Challengescape items on target discovery relying on manual literature
reasoning, the lack of literature-to-KG synthesis tools, and fragmented antimicrobial-resistance
evidence. It covers all three with three grounded layers.

## The result

Three layers, each built from a real public source, typed with a real ontology, and validated by
the closed-world gate (full table in [`results/SUMMARY.md`](results/SUMMARY.md)):

| Layer | Source | Ontology policed | Grounded triples | Closed-world result |
|---|---|---|--:|---|
| Structured (target-disease) | Open Targets | Biolink | 284 | **0** violations; twin caught |
| Literature | PubTator3 | Biolink | 169 | **0** violations; twin caught |
| AMR (resistance-to-drug) | CARD / ARO | ARO | 1,283 | **0** violations; twin caught |
| AMR pathogen linkage | CARD + NCBI taxonomy | ARO + Biolink + NCBITaxon | 20,692 | fabricated taxid caught; 17 retired ids flagged |

In the first three layers the grounded KG has **0 SHACL and 0 closed-world vocabulary violations**,
and an ungrounded twin, identical but for one fabricated term, still reports `conforms=true` under
SHACL while the closed-world gate rejects it. The same open-world hole, across the biomedical
vocabulary, a second ontology (ARO), and a third (NCBITaxon).

- **Structured** ([`src/pipeline.py`](src/pipeline.py)): 40 live gene-disease associations from the
  **Open Targets Platform**, typed with the **Biolink Model** (868 declared terms).
- **Literature** ([`src/pubtator.py`](src/pubtator.py)): 57 gene-disease relations machine-extracted
  by **PubTator3** from 40 PubMed abstracts across the eight target genes. The fragmented-literature-
  to-KG step, grounded.
- **AMR** ([`src/amr.py`](src/amr.py)): 800 of 5,053 real "confers resistance to" relationships from
  **CARD's Antibiotic Resistance Ontology** (8,564 declared terms); the gate polices the ARO namespace.
- **AMR pathogen linkage** ([`src/pathogen.py`](src/pathogen.py)): 6,404 gene-organism edges over 740
  organisms from **CARD's card.json**, policing **three namespaces at once**, ARO for the gene, Biolink
  for the types and `in_taxon` predicate, and **NCBITaxon against the current NCBI taxonomy**
  (2,871,791 taxids from `nodes.dmp`). A fabricated taxon id is caught; and, run against the current
  taxonomy, the gate flags **17 organism ids in CARD as no longer current**, all confirmed
  retired-and-merged in NCBI's `merged.dmp`. That is a real data-freshness signal that open-world
  SHACL, and a naive "is it a number" check, both miss.

## Triage

Because every edge is validated and Biolink-typed, the graph answers a ranked, provenance-carrying
query directly. Top gene-disease hypotheses by Open Targets association score
([`results/triage.md`](results/triage.md)):

| Rank | Gene | Disease | Open Targets score |
|--:|---|---|--:|
| 1 | BRAF | cardiofaciocutaneous syndrome | 0.877 |
| 2 | TP53 | Li-Fraumeni syndrome | 0.876 |
| 3 | PTEN | Cowden syndrome 1 | 0.874 |
| 4 | EGFR | non-small cell lung carcinoma | 0.853 |
| 5 | BRCA1 | breast cancer | 0.839 |

Every row is a validated triple with a source. The score is the Open Targets association score,
not a model we invented; the contribution here is the *grounding and validation*, so a triage
built on this graph cannot rank on a fabricated edge.

## Where it fits

The graph is the substrate; the model that drafts against it is our shipped
[biology-ontology language model](https://huggingface.co/fabsssss/qwen3-coder-30b-a3b-bio)
(Biolink + GO-CAM + OBO, term conformance 0 to 100%). The model proposes edges; this gate
guarantees the edges use only real terms before they are committed. Neither does the other's job.

## Reproduce

```bash
./run-demo.sh     # fetches Biolink, ARO, live Open Targets and PubTator3 data; builds + validates all three layers
```

Requires Python 3.10+ and network access (Open Targets, PubTator3, CARD and the NCBI taxonomy dump
are fetched live). The pathogen layer downloads the 75 MB NCBI taxdump once.

## Honest scope

See [`BUILD_REPORT.md`](BUILD_REPORT.md). Built and validated: the structured target-disease layer
(Open Targets), the literature front end (PubTator3), the AMR resistance-to-drug layer (CARD/ARO),
and the AMR pathogen linkage (CARD + NCBITaxon). Remaining honest limits: the resistance-to-drug
layer uses a deterministic 800-edge slice of ARO's 5,053 relationships (logged, not hidden);
PubTator3's associations are its own machine extraction with its confidence scores, not re-verified
by us; and the triage ranks on Open Targets' own score, presented with provenance, not a new scoring
method. The 17 retired taxon ids the pathogen gate flags are a feature, not an error: they are real
but deprecated NCBI ids that CARD still carries.

---

### Built by Tesseract Academy

We build the correctness and assurance layer for AI-generated knowledge graphs. If you are
extracting biomedical knowledge with LLMs and need every emitted gene, disease, or relation to be
a real, typed, checkable term, the gate above runs on your vocabulary today.

[gov.tesseract.academy](https://gov.tesseract.academy) · fabio@thetesseractacademy.com
Part of [Open Ontologies](../../README.md) · MIT · real data, real numbers.
