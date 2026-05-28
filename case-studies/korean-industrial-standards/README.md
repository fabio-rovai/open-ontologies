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

## Empirical run — actually doing it

This directory ships a runnable demo: a synthetic KS-X-9999 pressure-vessel standard (`synthetic-ks-standard.ttl`), the clauses encoded as SHACL (`clauses-as-shacl.ttl`), and a script (`run-demo.sh`).

The synthetic standard contains **six vessels** designed to exercise the failure modes the Hanyang paper documents:

| ID | Class | Temp | Pressure | Interval | Expected verdict |
|---|---|---|---|---|---|
| V-001 | A | 150°C | 4.0 MPa | 12mo | Compliant |
| V-002 | A | 250°C | **2.5 MPa** | 12mo | Compliant (exception 5.4.1 applies: hot but low-pressure) |
| V-003 | A | 230°C | **5.0 MPa** | 12mo | **Violates 5.3.2 + 5.4.1 (hot + high-pressure must inspect ≤6mo)** |
| V-004 | B | 80°C | 3.5 MPa | 24mo | Compliant |
| V-005 | C | 60°C | 1.2 MPa | 36mo | Compliant |
| V-006 | A | 180°C | **missing** | 12mo | **Violates pressure-required invariant** |

### What actually happened when we ran it

**Without OWL-RL ramification, SHACL silently passes everything.** V-006 is typed as `ks:VesselClassA`, not `ks:Vessel`, so the targetClass-based pressure-required shape never fires. This is *exactly* the failure the K-CAP 2025 SHACL co-evolution work flags.

```bash
$ open-ontologies batch <<EOF
load synthetic-ks-standard.ttl
shacl simple-shapes.ttl
EOF
# conforms: true, violation_count: 0  ← WRONG
```

**With OWL-RL materialisation first (`onto_reason`), V-006 is correctly flagged:**

```bash
$ open-ontologies batch <<EOF
load synthetic-ks-standard.ttl
reason
shacl simple-shapes.ttl
EOF
# conforms: false, violation_count: 1
# {focus_node: "vessel_06", path: "designPressureMPa", constraint: "minCount", ...}
```

**The toxic-clause interaction (5.3.2 + 5.4.1) was easier to express as SPARQL than as SHACL:** Oxigraph's SHACL engine doesn't (yet) handle `sh:sparql` constraints or deeply-nested `sh:or` with multiple `sh:property` children. The SPARQL formulation works first time:

```sparql
SELECT ?vessel ?temp ?pressure ?interval WHERE {
  ?vessel ks:operatingTemperatureC ?temp ;
          ks:designPressureMPa ?pressure ;
          ks:inspectionInterval ?interval .
  FILTER(?temp > 200 && ?pressure >= 3.0 && ?interval > 6)
}
# → 1 result: vessel_03 (temp=230, pressure=5.0, interval=12)
```

V-003 is correctly identified as the only toxic-clause violator. V-002, which satisfies the exception, is not flagged.

### What this run proves

1. **The Hanyang failure mode is real and reproducible.** Without reasoning, SHACL misses inherited-type violations. With OO's `reason` + `shacl` chain, it doesn't.
2. **OO does not currently express conditional toxic-clause patterns through SHACL alone.** SPARQL is the actual expression layer; SHACL handles the simpler constraints. The case-study README must reflect that, not promise more.
3. **Both V-003 (toxic interaction) and V-006 (missing data) are caught by the OO pipeline,** but through different primitives: SPARQL for V-003, OWL-RL + SHACL for V-006. The Hanyang pipeline catches both via its propositional decomposer, expressed inside the bespoke LLM extraction layer.

## What if Hanyang tried to bridge the gap?

The honest question. If Park et al. saw this comparison and decided to add OO's advantages to their stack, what would they need to build, and could their architecture absorb it?

| OO advantage | What Hanyang would need to add | Can their stack absorb it? |
|---|---|---|
| **MCP-native conversation** | Wrap their pipeline behind an MCP server; expose extraction + validation as tools | **Yes, mechanically.** The architectural friction is that their LLM-extraction logic is currently the "intelligence" of the system; pulling it out and trusting Claude over MCP to do that work is a strategy change, not just a refactor. |
| **OWL-RL ramification before SHACL** | Bolt a reasoner (HermiT, ELK, openllet) ahead of their KG-RAG step | **Yes, easily.** This is a 200-line change. It also removes one of OO's clearest wins. |
| **Incremental SHACL co-evolution** | Build the shape-→-OWL dependency graph + change-detection routing | **Hard.** Their pipeline today doesn't even have an OWL closure step. The infrastructure has to ship reasoner + shape parser + delta tracking. ~2-3 weeks. |
| **FLORA fuzzy alignment** | Adopt the FLORA paper (ISWC 2025 Best, open-source) | **Yes — same as us.** FLORA is public; anyone can port it. The only real lead OO has here is timing + the MCP exposure pattern. |
| **BC+ Dynamics + Causal what-if** | Build an action-schema language + reasoner-over-actions + identifiability check | **Very hard.** This is genuinely the three-layer architecture OO is built around. Bolting it onto a KG-RAG extraction pipeline as an afterthought would be awkward; cleaner to write from scratch in their stack. ~2-3 months. |

### Recompute after the bridge attempt

If Hanyang absorbs the *easy* changes (reasoner before SHACL, port FLORA), what's left?

| Dimension | OO advantage before | After bridge |
|---|---|---|
| OWL-RL ramification | Real win | **Neutralised** (200-line addition for them) |
| FLORA alignment | Real win | **Neutralised** (port the same paper) |
| MCP-native conversation | Real win | **Strategic question** — adoption depends on whether Korean enterprise wants MCP, not on architecture |
| Incremental SHACL co-evolution | Real win | **Stays a win for 2-3 weeks of their team's time** |
| BC+ Dynamics + Causal | Real win | **Stays a win for 2-3 months** — and probably never gets built because the use case isn't yet a buyer demand |
| Document-aware ingestion | **OO weakness** | **Stays an OO weakness.** They have it; we don't. |
| Korean-language tuning | **OO weakness** | **Stays an OO weakness.** Domain-specific, doesn't transfer cheaply. |
| Standards-RAG eval framework | **OO weakness** | **Stays an OO weakness** until OO ships one. |
| Toxic-clause detector packaged | **OO weakness** | **Closes** if we ship a `kt:ToxicClause` SHACL pattern library — ~1 week of work. |
| Korean industrial credibility | **OO weakness** | **Stays an OO weakness** structurally — academic + vendor credentialing takes years. |

**Bottom line after recompute:** OO retains a *structural* advantage on the Dynamics + Causal layers (because they're large architectural commitments) and a *timing* advantage on incremental SHACL co-evolution (small enough that Hanyang would close it, but slowly). Every other axis either neutralises or stays an OO weakness.

The asymmetry that matters: **OO can close the toxic-clause-detector packaged gap in ~1 week. The Korean industrial credibility gap doesn't close on a code-shipping timeline at all.** That second one is the binding constraint on adoption, not the architecture.

### What this implies for the project

- Pour code into the Dynamics + Causal layers (the structural moats) rather than the SHACL co-evolution layer (Hanyang would close that within a sprint if it mattered to them).
- Ship the document-aware ingestion gap first if any actual buyer surfaces. Without it, Hanyang's full pipeline is one deployment artefact and OO is a library that needs glue.
- The Korean industrial credibility gap is closed by a co-authored paper or a deployed pilot, not by more modules.

The earlier framing — "what OO would change if they were Hanyang" — was self-flattering. The recompute clarifies that the actual contest is fought on adoption, deployment, and credibility, not on Mamdani inference rules.

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
