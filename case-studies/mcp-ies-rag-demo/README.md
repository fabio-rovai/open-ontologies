# MCP-over-IES-RAG Demo

Closes [#27](https://github.com/fabio-rovai/open-ontologies/issues/27).

A working demonstration of **ontology-grounded retrieval-augmented generation** using Open Ontologies as the substrate. The pattern: load an IES4-conformant knowledge graph + sample data; let the connected LLM (Claude over MCP) author SPARQL queries via `onto_query`; answer competency questions with structural certainty that flat document RAG cannot deliver on temporal / identity / participation questions.

## Why this case study exists

Per the May 2026 IES4 ecosystem research finding: nobody has publicly grafted LLM/RAG onto IES. Telicent's Smart-Cache-Graph stack is clearly designed as RAG substrate but isn't marketed that way. There's no published paper or product combining IES4 + LLMs in 2024-2026. This is the wide-open public niche this case study fills.

## What's in here

```
case-studies/mcp-ies-rag-demo/
├── README.md                          this file: the pattern, the why
├── data/
│   ├── ies-sample.ttl                 small IES4-conformant graph (~30 individuals,
│   │                                  ~80 triples): people / orgs / locations /
│   │                                  4D role-states / events with participants
│   └── competency-questions.md        6 natural-language questions, each with
│                                      the ontology-grounded answer and the
│                                      flat-RAG failure mode it exposes
└── transcripts/
    └── claude-walkthrough.md          sample Claude/MCP conversation answering
                                       all six questions
```

## The pattern in one paragraph

`onto_embed` over the loaded IES4 graph gives Claude semantic retrieval over particulars (named entities like Alice Patel, Acme Bristol). `onto_query` lets Claude run SPARQL when a question requires structural reasoning (temporal extents, set membership, participant edges). The two combine: Claude retrieves the relevant subgraph by semantic search, then queries it precisely to answer. The 4D modelling means that **states are first-class** ("Alice was the Bristol manager" is a state with a period, not an attribute of Alice), so questions like "who held that role in 2022?" reduce to a SPARQL FILTER over period strings rather than a guess.

## Why this beats flat document RAG

Flat document RAG vectorises prose chunks. Temporal facts are stored as English sentences that humans wrote ("Alice took over the Bristol depot in January 2020"). Retrieval works on semantic similarity to the question. For "who was the Bristol manager in 2022?":

- The chunk "Alice was promoted to London operations director in October 2023" and the chunk "Alice managed Bristol from 2020" both surface
- The LLM has to do its own temporal reasoning on the prose
- Errors compound when multiple Bristol-managers exist across time
- Confidence is low and uncalibrated

Ontology-grounded RAG over IES4:

- Alice's two role-states are separate first-class IES4 individuals
- Each has a `ies:inPeriod` with explicit start/end
- The SPARQL `FILTER` over period strings is mechanical, not interpretive
- The connected LLM still does the natural-language framing of the answer; it just doesn't have to GUESS the temporal facts

The case study's `data/competency-questions.md` documents 6 specific questions where this difference matters.

## Running it locally

```bash
# 1. Load the IES4 v4.3.1 baseline (provides ies:Person, ies:Event, etc.)
open-ontologies marketplace install ies-4.3.1

# 2. Load the demo dataset
open-ontologies load case-studies/mcp-ies-rag-demo/data/ies-sample.ttl

# 3. (Optional) generate embeddings if you want to use onto_search
open-ontologies init  # downloads the embedding model the first time
# in MCP mode: ask Claude to call onto_embed

# 4. Run the competency questions
# See data/competency-questions.md for the SPARQL bodies
```

In MCP mode (Claude Code connected to Open Ontologies as a server), the workflow is:

1. Claude reads the user's question
2. Calls `onto_query` with a SPARQL it composed from the IES4 patterns the loaded ontology defines
3. Optionally calls `onto_search` first if the question references entities by description rather than IRI
4. Forms the natural-language answer from the SPARQL results

The transcript at [`transcripts/claude-walkthrough.md`](transcripts/claude-walkthrough.md) shows this in action across all 6 questions.

## Scope and honesty

This is a **case study**, not a product. It:

- Uses a synthetic 30-individual dataset, not real operational data. Real deployments would load actual IES-conformant graphs (e.g. NDTP-style assets, MOD intelligence corpora).
- Uses a placeholder `demo:roleIn` helper property. A production IES4 implementation would use the canonical `ies:MemberState` + `ies:isMemberOf` pattern. The shortcut keeps the demo Turtle readable.
- Doesn't include a quantitative evaluation. A real comparison vs flat document RAG would need a benchmark corpus and human-evaluated accuracy on both temporal and atemporal questions; that's a follow-up paper, not a case study.
- Doesn't ship runtime code beyond what's already in Open Ontologies (load, query, embed, search). The case study is the dataset + the questions + the documented pattern.

## How to extend

PRs welcome. Useful directions:

1. **Add more competency questions** to `data/competency-questions.md`, particularly questions that combine multiple temporal extents (e.g. "who held overlapping roles in different orgs simultaneously?").
2. **Add a real dataset**: replace the synthetic data with an IES-conformant transformation of public NDTP data, the Wales valuation report dataset, or similar.
3. **Add a quantitative benchmark**: define a question set, run flat-RAG and onto-grounded-RAG on the same corpus, measure accuracy + latency.
4. **Extend the JC3IEDM crosswalk** (the sibling case study at [`../jc3iedm-ies4-crosswalk/`](../jc3iedm-ies4-crosswalk/)) so you can ingest JC3IEDM-shaped data and answer questions via the IES4 4D model.

## References

- [IES Information Exchange Standard](https://informationexchangestandard.org/)
- [`IES-Org/ont-ies`](https://github.com/IES-Org/ont-ies): canonical IES repo (custodied by UK DBT since March 2025)
- [Telicent Smart-Cache-Graph](https://github.com/orgs/telicent-oss/repositories): the RAG-substrate-shaped commercial stack from the IES Working Group's primary implementation partner
- FOUST 7 paper, "Comparing IES and BORO" (CEUR Vol-4176, JOWO 2024)

## Licence

MIT, matching the rest of Open Ontologies.
