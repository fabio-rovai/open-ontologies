# Case study: Korean industrial-standards knowledge graphs

**Reference paper:** Park, Jeon, Lee, Hong & Kim. *Ontology-Based KG Framework for Industrial Standard Documents via Hierarchical and Propositional Structuring.* Hanyang University + The Miraclesoft, arXiv [2512.08398](https://arxiv.org/abs/2512.08398), Dec 2025.

This is a paper-grounded comparison of how a Korean industry/academia team is solving knowledge-graph construction for KS / KEPCO / KOSHA standards today, and where Open Ontologies would change the architecture. It is not a sales pitch. The "where OO is weaker" section is intentional.

---

## The problem

Korean industrial standards (KS family, IEC translations used across the Hyundai / POSCO / KEPCO supply chains, KOSHA workplace standards) are structurally hostile to vanilla KG-RAG. Clauses combine:

- **Scope of application** (this clause applies to product class X under condition Y).
- **Exceptions** (does not apply when Z).
- **Conditional rules** (if temperature > 100°C, use formula A; else use formula B).
- **Numerical formulae and thresholds** embedded inline.
- **Tables** that act as decision matrices.

Park et al. document that vanilla KG-RAG fails QA over these documents, while their hierarchical + propositional pipeline lifts F1 by **64%** and surfaces "toxic clauses" (constraints that interact badly with other clauses). The concrete pain point: a Korean manufacturer reading a KS standard needs to know *which sub-clauses apply to their product type, under which exceptions, with which numerical thresholds, and how those interact across the standard.*

## How Park et al. solve it today

1. LLM-based triple extraction from structured documents.
2. Hierarchical semantic structuring of the document tree (chapters → clauses → sub-clauses).
3. **Propositional decomposition**: each conditional or numerical rule becomes an atomic proposition with explicit antecedent and consequent.
4. Integration into an ontology layer used as RAG context.
5. Result: **+64% F1** on standards QA vs. conventional KG-RAG; toxic-clause detection as a packaged feature.

The intelligence (clause decomposition, condition extraction) lives inside a custom LLM extraction pipeline. The ontology layer is largely fixed once authored. There is no formal reasoner-in-the-loop and no incremental co-evolution when standards are revised — which they are, frequently (annual cycle for many KS standards, faster for safety standards).

## Where Open Ontologies would change the architecture

Specific changes, not generic claims:

### 1. Replace the bespoke LLM extraction pipeline with MCP-native scaffolding

The OO server exposes primitive tools (`onto_extract_scaffold`, `onto_extract_validate`, `onto_shacl_check`, `onto_drift` with KGCL output). Claude over MCP plays the role the custom Hanyang LLM pipeline plays today, but **the intelligence stays in the model** and **the verification stays in the server**. The Hanyang team's three separate codebases (extraction pipeline + ontology authoring + drift handling) become a single conversation against one MCP server.

### 2. Data-driven SHACL induction for clause constraints

Hanyang's "toxic clause" detection is currently heuristic. OO's [`onto_shape_combinatorics`](../../src/shape_combinatorics.rs) primitive enumerates property-combination subsets per class and (in the Kastor-grade implementation on PR #53) ranks them by `support × confidence` against the loaded data. The induced SHACL shapes are valid Turtle that parses back through `onto_load`. A real reasoner-in-the-loop, not just prompt context.

### 3. Incremental SHACL co-evolution

When KS standards are revised, OO's [`onto_owl_shacl_coevolve_incremental`](../../src/coevolve.rs) (PR #53) reports *which downstream shapes are invalidated by which changed IRIs*, via the shape→OWL-dependency map. Skipped shapes don't get revalidated. The Hanyang pipeline today must re-extract the whole document on every revision.

### 4. FLORA-style fuzzy adjudication for cross-standard alignment

When a Korean manufacturer maps a KS standard to its IEC equivalent (Hyundai, POSCO and KEPCO suppliers do this every day for export certification), HNSW does candidate generation and the 10-rule Mamdani inference engine in [`onto_align_fuzzy`](../../src/align_fuzzy.rs) adjudicates — interpretable, no training data, every accept decision carries a rule trace. The Hanyang paper does not address cross-standard alignment at all; that gap is the highest-leverage adjacent application.

### 5. Dynamics + Causal layers for "what if this clause is violated"

OO's BC+ action schemas + CIVeX backdoor identification let a compliance engineer ask: "If clause 5.3.2 is breached, which downstream certifications fail, and through which causal path?" The Hanyang pipeline can answer *what the clause says*, not *what happens if you break it*. This is the architectural advantage of having a Dynamics layer separate from the static-document KG.

### Why MCP-native is the right fit

Industrial standards work is intrinsically conversational. A compliance engineer iterates with the model: refining the ontology, checking constraints, re-validating after revisions. Embedding an LLM client inside the server (as the Hanyang pipeline does) forces a fixed prompt strategy. The MCP-native split lets the engineer's Claude session do the reasoning while the server provides validation primitives that are guaranteed to be sound (Oxigraph 0.5.8 `SparqlEvaluator`, OWL-RL ramification, SHACL co-evolution).

## Where OO is weaker than the Hanyang approach

Honest list. Build before pitching.

1. **No document-aware ingestion.** The Hanyang pipeline reads tables and conditional clauses *in situ* from PDF/HWP/DOCX. OO assumes the KG already exists; it does not parse standards documents. A real deployment needs a separate doc-to-RDF front end (Unstructured, Docling, GROBID) feeding OO. That's an integration cost the Hanyang stack avoids by owning both ends.

2. **No Korean-language tuning.** The Hanyang authors have, implicitly, a model and prompt set tuned on Korean technical prose (KS standards are in Korean with English transliterations of terms). OO inherits whatever the connected LLM brings; clinical crosswalks ship for ICD-10 / SNOMED / MeSH (English-default) but no comparable Korean industrial-standards lexicons. Out-of-the-box Korean term normalisation is weaker.

3. **No QA evaluation framework specific to standards-RAG.** The Hanyang paper ships an evaluation showing 64% F1 lift on QA over standards. OO has 200+ lib tests but no equivalent end-task benchmark for industrial-standards QA. A buyer would have to construct one.

4. **No "toxic clause" detector as a packaged primitive.** Their pipeline ships this as a working feature. OO can *express* the same check via SHACL + reasoning, but the user has to author the shapes themselves. The Hanyang stack hands it to them pre-built.

5. **Korean industrial credibility gap.** Hanyang has a Korean university + Korean software vendor co-author line, a Korean dataset, and a paper. OO has @ziozzang's PR-11 contribution from Hyundai and that's the entire Korean industrial footprint. Closing that gap means building this case study into a co-authored paper with a Korean partner, not winning on architecture alone.

## What concrete next step the comparison suggests

Build a small, runnable demonstration in this case-study directory:

- Pick one publicly available Korean industrial standard (or a synthetic stand-in to avoid licensing).
- Express its clauses as KGCL `create` operations against an `onto_action_register`'d schema.
- Author SHACL shapes for two or three "toxic clause" patterns.
- Use `onto_shape_induce` to suggest additional shapes from the encoded data.
- Use `onto_owl_shacl_coevolve_incremental` to demonstrate that revising one clause re-validates only the shapes touched.
- Use `onto_align_flora` to align the standard's terms against a published IEC equivalent.

That demonstration is the realistic ISWC-style submission angle — In-Use track, not Best Paper track. The Hanyang paper as the prior art reference.

## Sources

- [Hanyang + Miraclesoft, *Ontology-Based KG Framework for Industrial Standard Documents* (arXiv 2512.08398)](https://arxiv.org/abs/2512.08398) — the paper this case study is built around.
- [Ontology-guided multi-level KG for blast furnace ironmaking (Advanced Engineering Informatics, Nov 2024)](https://dl.acm.org/doi/abs/10.1016/j.aei.2024.102927) — POSCO-adjacent precedent on Korean industrial KG work.
- [FLORA: Unsupervised KG Alignment by Fuzzy Logic (ISWC 2025 Best Paper)](https://arxiv.org/abs/2510.20467) — the alignment primitive OO ports.
- [KROMA: Ontology Matching with Knowledge Retrieval and LLMs (ISWC 2025)](https://link.springer.com/chapter/10.1007/978-3-032-09527-5_34) — RAG-pre-filter alignment pattern.
- [GenOM: Ontology Matching with Description Generation and LLMs (OAEI Bio-ML)](https://arxiv.org/abs/2508.10703) — LLM-as-author pattern OO's `onto_extract_scaffold` mirrors.
- [Agent-OM (PVLDB 2025)](https://arxiv.org/abs/2312.00326) — agent-in-the-loop OM, contrast with MCP-native split.

## Appendix: ISWC 2024–2025 paper survey informing this case study

| # | Paper | Venue | Pattern | One concrete limitation |
|---|---|---|---|---|
| 1 | FLORA: Unsupervised KG Alignment by Fuzzy Logic | ISWC 2025 main, Best Paper | Standalone classical algorithm, no LLM | Degrades on noisy/sparse textual labels where embeddings dominate |
| 2 | KROMA: Ontology Matching with Knowledge Retrieval + LLMs | ISWC 2025 main | LLM-in-the-loop, RAG pre-filter, bisimilarity pruning | Bisimilarity pruning brittle on cyclic/richly-axiomatised TBoxes |
| 3 | Ontology-Enhanced KG Completion using LLMs (Manchester) | ISWC 2025 main | LLM-as-oracle with schema-conditioned prompts | Uses ontology as static prompt augmentation only; no reasoner round-trip |
| 4 | LLMs4OL 2025 Overview (2nd LLMs for Ontology Learning Challenge) | ISWC challenge | Hybrid pipelines (commercial LLM + domain embed + fine-tune) | Leaderboard format encourages overfitting; no transfer evaluation |
| 5 | LM-KBC 2025 Challenge | ISWC workshop | LLM standalone | Single-shared-LLM track; 5-submission sample size |
| 6 | From Matching to Retrieval (Hu & Ichise, OM 2025) | ISWC workshop | LLM-in-the-loop at retrieval, classical match scoring after | Zero-shot prompting only; no per-domain reliability calibration |
| 7 | OAEI-LLM / OAEI-LLM-T benchmark | ISWC 2024 HGAIS | Evaluation-oriented (LLM is system under test) | Hallucination labels conflate hallucination with legitimate ambiguity |
| 8 | Agent-OM (Monash / ANU) | PVLDB 2025, OM workshop | Full LLM-agent-in-the-loop with tool use | Cost scales with ontology size; no incremental update story |
| 9 | GenOM (Manchester) | OAEI Bio-ML / WWW Springer | LLM standalone for description-generation, classical retrieval after | Synthetic descriptions can drift from intended ontology semantics |
| 10 | NeOn-GPT (ESWC 2024 satellite) | ESWC 2024 | LLM-as-author, human-in-the-loop afterwards | Gold standard is a textbook ontology; untested on messy real domains |
| 11 | IBM Skills and Expertise KG | ISWC 2024 In-Use | Traditional ontology engineering, KG-first, LLM-light | Closed enterprise data, limited reproducibility outside IBM |
| 12 | Hanyang + Miraclesoft industrial-standards framework | arXiv 2512.08398 (post-ISWC 2025) | LLM-in-the-loop for extraction; ontology authored upstream | Rule-based, document-format-specific decomposer; no formal compliance reasoner |
