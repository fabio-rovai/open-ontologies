# UK Zero-Emission Flight Ecosystem — an open, computation-ready reference graph

An open, SHACL-validated, **provenance-first** knowledge graph of the UK hydrogen and electric
aviation ecosystem: organisations, airports, programmes, projects, funders, bodies, alliances,
standards and technologies; the hydrogen production-to-use chain; and dated, sourced
technology-readiness and quantity claims. Built by [The Tesseract Academy](https://gov.tesseract.academy)
as an open reference asset for computation-ready modelling of a fragmented innovation ecosystem.

> **Independent open reference dataset.** Compiled from public sources for demonstration of
> computation-ready ecosystem modelling. Not affiliated with, endorsed by, or a deliverable for any
> organisation named within it. Maturity bands and any TRL values are indicative, each carrying
> provenance, and are not an authoritative assessment. Licence: CC BY 4.0.

## Why this exists, and why it is first-of-kind

The UK zero-emission-flight sector has strong policy signals and funding under the Jet Zero Strategy,
ATI FlyZero and the DfT-funded, CPC-led Zero Emission Flight Infrastructure (ZEFI) programme, but the
data on who is building what, who funds whom, and which technologies gate which pathway sits scattered
across press releases, programme pages and reports. A structured literature review conducted for this
work found **no published ontology or knowledge graph dedicated to hydrogen or zero-emission aviation**,
and **no formal ontology model of Technology Readiness Level** at all. The credible build path is
therefore composition and reuse: adopt the two-axis structure of the HOLY hydrogen-market ontology,
the provenance discipline of PECO, and the standard W3C building blocks, and apply them to this domain
for the first time.

## What it contains (measured, reproducible)

| Metric | Value |
| --- | --- |
| Entities | 45 |
| Relationships | 56 |
| RDF triples | 482 |
| SHACL violations | **0** |
| Planted dangling edge caught by validation | **yes** |
| Provenanced quantities (each with unit + source) | 8 |
| Reified TRL assessments (dated + sourced) | 1 |
| Citable sources | 14 |
| Hydrogen chain stages modelled | 5 |
| Competency questions answered by SPARQL | 6 |

Entities by type: 15 organisations, 6 projects, 3 programmes, 3 funders, 3 bodies, 4 airports,
3 standards, 1 alliance, 7 technologies.

## Design and methodology

The ontology is engineered, not hand-waved. It follows the **Linked Open Terms (LOT)** framework
(Poveda-Villalón et al., 2022) — requirements, implementation, publication — with these disciplines:

- **Competency-question driven** (Grüninger & Fox, 1995). The scope is fixed by six competency
  questions (`queries/competency.rq`), and those same questions are re-run as SPARQL **acceptance
  tests** by the build pipeline, so the ontology is proven to answer them, not just to type-check.
  This is the requirements-and-QA loop of SAMOD (Peroni, 2017) in one artefact.
- **Reuse before invention** (LOT term-reuse; NeOn Scenario 3). Provenance reuses **PROV-O**; controlled
  and ordinal scales (maturity bands, the nine-level TRL scale) reuse **SKOS**; quantities align to
  **QUDT**; source metadata reuses **Dublin Core Terms**. Only genuinely domain-specific terms are minted
  in the `zef:` namespace.
- **Provenance-first, after PECO** (Markovic et al., 2023). Maturity and every quantity are *reified*:
  a `zef:TRLAssessment` carries a value, an assessment date and `prov:wasDerivedFrom` a `zef:Source`;
  a `zef:Quantity` carries a numeric value, a unit and a source. No maturity and no number is ever a
  bare, unsourced literal.
- **Two-axis structure, after HOLY** (Ascencion Arevalo & Neunsinger, 2023). One axis is the ecosystem
  actors (organisations, funders, programmes, projects); the other is the technology value chain
  (`zef:ChainStage` from production through liquefaction, storage and refuelling to propulsion, linked
  by `zef:feedsInto`). Technologies `zef:realisesStage` a stage.
- **Continuant/occurrent separation, after OEO** (Booshehri et al., 2021). Technologies and organisations
  are continuants; chain stages are the process/flow (occurrent) layer, kept as a distinct module.

## The validation discipline (the point for a coordination tool)

The SHACL shapes (`shapes/zef-shapes.ttl`) enforce, and the pipeline proves:

1. **Every entity is labelled and typed.**
2. **Maturity is controlled and sourced.** A technology must carry either a maturity band from the
   controlled SKOS vocabulary, or a reified TRL assessment (integer 1-9) that is dated and derived from
   a declared source.
3. **Every quantity carries a unit and a source.**
4. **Referential integrity on every relationship.** The object of every ecosystem link must be a
   declared entity of the correct type. This is the graph-level equivalent of "only validated, provided
   relationships are shown, and none are inferred". A negative test injects a dangling edge and confirms
   the validator rejects it.

## Competency questions (answered by the build, see `competency-results.md`)

1. Which technologies gate a pathway because they are below TRL 6?
2. For each hydrogen chain stage, which organisations develop technologies that realise it?
3. Which provenanced quantities (funding, capacity, benefit) does the graph hold, and from which source?
4. Which funders fund which projects and programmes?
5. Which technologies are governed by a named safety or fuelling standard?
6. Which airports are demonstration sites, for which projects and technologies?

CQ3, for example, returns eight provenanced figures including the January 2021 £84.6m green-aviation
package, GKN H2GEAR (£54.4m total / £27.2m ATI grant), ZeroAvia HyFlyer II (£24.6m / £12.3m ATI),
GKN H2FlyGHT (£44m, 2 MW), and the Hydrogen in Aviation alliance's projected £34bn/year UK benefit by
2050 — each traceable to a primary source.

## Related work (verified, 2020-2026 unless noted as foundational)

**Hydrogen and energy ontologies / KGs**
- Ascencion Arevalo K M, Neunsinger C. *HOLY: An Ontology Covering the Hydrogen Market.* ISWC 2023,
  LNCS 14266. https://doi.org/10.1007/978-3-031-47243-5_1 — the two-axis actors x value-chain design.
- Booshehri M et al. *Introducing the Open Energy Ontology.* Energy and AI 5, 2021.
  https://doi.org/10.1016/j.egyai.2021.100074 — BFO-based modular energy ontology; continuant/occurrent.
- Santos G et al. *Intelligent Energy Systems Ontology (IESO).* Energy and AI, 2023.
  https://doi.org/10.1016/j.ecmx.2023.100495 — integrate-don't-reinvent + SHACL + unit conversion.
- Haghgoo M et al. *SARGON – Smart energy domain ontology.* IET Smart Cities 2(4), 2020.
  https://doi.org/10.1049/iet-smc.2020.0049 — SAREF-extension pattern (lightweight, no upper ontology).

**Emissions provenance and quantities**
- Markovic M, Garijo D, Germano S, Naja I. *PECO: The Provenance of Emission Calculations Ontology*, 2023.
  https://w3id.org/peco — PROV-O + QUDT + SOSA composition for auditable figures.

**Aviation / aerospace knowledge engineering**
- Wittenborg T et al. *Knowledge-Based Aerospace Engineering — A Systematic Literature Review.* arXiv
  2505.10142, 2025. https://arxiv.org/abs/2505.10142 — process/software/data backbone; names the
  sustainable-aviation gap.
- Kabashkin I. *Ontology-Driven Digital Twin Framework for Aviation Maintenance and Operations.*
  Mathematics 13(17):2817, 2025. https://doi.org/10.3390/math13172817 — modular multi-ontology design.
- Georgiou J et al. *The ICARUS Ontology: A General Aviation Ontology.* WIMS 2020.
  https://doi.org/10.1145/3405962.3405983 — reuse + generic-metadata/domain-layer separation.

**Funding and technology-maturity KGs**
- Chialva D, Mugabushaka A-M. *DINGO: an ontology for projects and grants linked data.* SKG/TPDL 2020.
  https://arxiv.org/abs/2006.13438 · https://w3id.org/dingo — the funds/grant/policy schema.
- EURIO / CORDIS Knowledge Graph, Publications Office of the EU.
  https://op.europa.eu/en/web/eu-vocabularies/eurio — EU-funding interoperability target.
- Trappey A J C, Lin G-B, Hung L-P. *Intelligent Text Mining for Ontological KG Refinement and Patent
  Portfolio Analysis — Net-Zero Data Center.* Information 15(7):374, 2024.
  https://doi.org/10.3390/info15070374 — couples an ontology/KG to technology-maturity analytics.

**Reusable W3C / standard building blocks**
- Haller A et al. *Semantic Sensor Network Ontology (SOSA/SSN).* W3C Recommendation, 2017.
  https://www.w3.org/TR/vocab-ssn/
- Lebo T et al. *PROV-O: The PROV Ontology.* W3C Recommendation, 2013. https://www.w3.org/TR/prov-o/
- Miles A, Bechhofer S. *SKOS Reference.* W3C Recommendation, 2009. https://www.w3.org/TR/skos-reference/
- QUDT — Quantities, Units, Dimensions and Types. https://qudt.org/

**Ontology-engineering methodology**
- Poveda-Villalón M et al. *LOT: An industrial oriented ontology engineering framework.* EAAI 111, 2022.
  https://doi.org/10.1016/j.engappai.2022.104755
- Grüninger M, Fox M S. *Methodology for the Design and Evaluation of Ontologies.* IJCAI-95 workshop
  (foundational). — competency questions.
- Poveda-Villalón M et al. *OOPS! (OntOlogy Pitfall Scanner!).* IJSWIS 10(2), 2014.
  https://doi.org/10.4018/ijswis.2014040102
- Wilkinson M D et al. *The FAIR Guiding Principles.* Scientific Data 3:160018, 2016.
  https://doi.org/10.1038/sdata.2016.18
- Peroni S. *SAMOD: A Simplified Agile Methodology for Ontology Development.* OWLED-ORE 2016.
  https://doi.org/10.1007/978-3-319-54627-8_5

## Reproduce

```bash
pip install rdflib pyshacl networkx matplotlib
python3 pipeline/build_and_validate.py
```

This builds the graph from `data/ecosystem.json`, validates it against the shapes at zero violations,
runs the negative referential-integrity test, executes the six competency questions, and writes
`graph.ttl`, `demo/graph.json`, `competency-results.md`, `metrics.json` and `assets/ecosystem-graph.png`.
Open `demo/index.html` for the interactive network view.

## Files

| Path | What |
| --- | --- |
| `data/ecosystem.json` | The curated, sourced dataset (entities, chain, sources, TRL, quantities, relations). |
| `ontology/zef.ttl` | The ZEF vocabulary: classes, properties, and the SKOS maturity/TRL scales. |
| `shapes/zef-shapes.ttl` | SHACL shapes (labels, controlled+sourced maturity, unit+source quantities, referential integrity). |
| `queries/competency.rq` | Six competency questions as runnable SPARQL. |
| `pipeline/build_and_validate.py` | Build, validate, negative test, run competency questions, export. |
| `competency-results.md` | Auto-generated answers to the competency questions. |
| `graph.ttl` / `metrics.json` | The generated RDF graph and machine-readable metrics. |
| `demo/index.html` | Self-contained interactive network view. |
| `assets/ecosystem-graph.png` | Static network render. |

## FAIR self-assessment

Findable (persistent GitHub URIs, typed entities), Accessible (open repo, plain RDF/Turtle),
Interoperable (reuses PROV-O, SKOS, QUDT, Dublin Core; aligns to HOLY/DINGO patterns), Reusable
(CC BY 4.0 licence, documented provenance on every claim, reproducible pipeline). The ontology is
authored to clear the OOPS! critical/important pitfalls (explicit domains/ranges, labels and comments
on every term, no cycles in the SKOS hierarchy).
