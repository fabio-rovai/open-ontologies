# Using Open Ontologies beside a knowledge graph platform

Several excellent open projects build knowledge graphs from documents:
[Microsoft GraphRAG](https://github.com/microsoft/graphrag) (MIT),
[Semantica](https://github.com/semantica-agi/semantica) (MIT), and
[TrustGraph](https://github.com/trustgraph-ai/trustgraph) (Apache-2.0).

Open Ontologies is not a competitor to any of them. It is a different layer,
and the combination is stronger than either part.

## The division of labour

Those platforms **generate**: they ingest documents, call a model to extract
entities and relationships, and assemble a graph. They are pipelines, written
in Python, with orchestration, storage, and retrieval included.

Open Ontologies **verifies and governs**: it is a single Rust binary with no
model client inside it, exposing ~95 MCP tools over a formal store. Its job
starts where extraction ends.

```
document corpus
      |
      v
  [ platform ]      GraphRAG / Semantica / TrustGraph
  extract, index    a model proposes entities and relations
      |
      v
  [ open-ontologies ]
  onto_validate     is it even syntactically RDF
  onto_load         all or nothing: no silently partial graphs
  onto_vocab_check  CLOSED-WORLD: which terms were invented
  onto_shacl        cardinality and datatype constraints
  onto_enforce      design patterns, including competing modelling patterns
  onto_reason       materialise, then find contradictions with provenance
  onto_plan/apply   governed change with risk scoring and locked IRIs
      |
      v
  a graph you can defend
```

The asymmetry worth naming: a generation pipeline cannot check its own
output. An extractor that invents `:hasProteinName` because it sounded
plausible produces RDF that parses, loads, and passes open-world SHACL
without complaint, because in the open world an undeclared term is merely
unknown, not wrong. Closed-world checking is the missing half, and it is
what `onto_vocab_check` does.

## Interop is free: everyone speaks the standards

No adapters, no plugins, no coupling. All four projects read and write W3C
standards, so the handoff is a file or a SPARQL endpoint:

| Concern | Shared ground |
|---|---|
| Serialisation | Turtle, N-Triples, RDF/XML, JSON-LD, TriG |
| Schema | OWL, RDFS, SKOS |
| Constraints | SHACL |
| Provenance | PROV-O (`prov:wasDerivedFrom`) |
| Transport | SPARQL 1.1, and MCP for tool access |

```bash
# whatever produced graph.ttl, verify it before trusting it
open-ontologies validate graph.ttl
open-ontologies load graph.ttl
open-ontologies enforce generic
```

Or, over MCP, let an agent do the same with `onto_load`, `onto_vocab_check`,
`onto_enforce` and `onto_reason` as tools in its session.

## What each pairing gives you

**With Microsoft GraphRAG.** GraphRAG's community detection and community
reports answer global questions ("what are the themes across this corpus")
that entity traversal cannot. Open Ontologies adds a schema those entities
must conform to, and a check that the extraction did not invent any of it.
`onto_communities` computes the same hierarchical community structure
deterministically inside the engine, and returns skeletons for the connected
model to summarise, so the expensive part stays under your control.

**With Semantica.** Semantica already embeds Oxigraph and emits PROV-O, so
the graphs move between the two with no conversion at all. Its pipeline is
close in shape to what verification wants: extract, detect conflicts,
deduplicate, record provenance. Add closed-world checking after extraction
and the plan/enforce/apply lifecycle around changes.

**With Semantica, worked through.** The section below is that pairing run for
real, on Semantica 0.6.5 and 0.6.6, with the numbers it produced.

**With TrustGraph.** Knowledge cores are versioned, promotable knowledge
artifacts. `onto_pack` produces the same kind of artifact from a verified
graph (ontology, instances, provenance, embedding fingerprint, manifest,
checksum), so what you promote between environments is a graph that has
already passed its checks, with the evidence bundled alongside it.

## A worked round trip: Semantica 0.6.5 and 0.6.6

The argument above is that a generation pipeline cannot check its own output.
Here is what that looked like when the check was actually run. The harness,
the raw output files, and the issues that came out of it are at
[semantica-contrib](https://github.com/fabio-rovai/semantica-contrib).

**A strict second reader disagrees with a lenient one, and the disagreement
is the finding.** `GraphBuilder` defaults an entity's id to its surface text
and its type to the NER label, so the canonical path produces this Turtle:

```turtle
<Acme Corp> a <ORG> ;
    semantica:text "Acme Corp" ;
    semantica:confidence 0.91 .
```

rdflib parses it. It resolves `<Acme Corp>` against the current working
directory, gives you `file:///home/you/project/Acme%20Corp`, and warns. Load
the same file through `open-ontologies validate` and Oxigraph refuses it:
`Invalid IRI code point ' '`. The lenient reader hands you a graph whose
identifiers depend on which directory you were standing in. The strict reader
tells you there is no graph. Both behaviours are defensible; only one of them
tells you something is wrong.

**Silent partial reads are the same problem one level up.** Semantica's
JSON-LD export put a top-level `@id` beside a top-level `@graph`, which makes
every member of that graph a quad named by the `@id` rather than a triple in
the default graph. On 0.6.6, a two-entity, one-relationship export parsed as
**2 triples** through `rdflib.Graph.parse()` and **21 quads** through
`Dataset()`. No error either way. The 19 missing statements were the whole
payload. `onto_load` is all-or-nothing for this reason: a partially loaded
graph that reports success is worse than a refusal, because everything
downstream then computes over a fraction of the data and looks fine doing it.

**Open-world SHACL passes what closed-world checking catches.** The generated
shapes minted their targets under `/shapes/` while the generated data used
`/ns#`. No `sh:targetClass` ever matched a node, so pySHACL returned
`conforms=True` on data that violated every constraint in the file. A test
asserting "validation passes" would have gone on passing forever.
`onto_vocab_check` asks the different question that catches this class of
defect: not whether the data satisfies the shapes, but which terms in the
graph were never declared anywhere. A vocabulary that nothing matches shows up
immediately as terms with no home.

**What the round trip produced.** Seventeen issues, of which the four fixed so
far are upstream in Semantica: deterministic entity IRIs replacing a per-process
`hash()`, a declared vocabulary at
`semantica/ontology/vocabulary/semantica-ns.ttl` where the namespace had
returned 404, JSON-LD `@id`s minted the same way the RDF serializers mint
them, and timezone-aware timestamps with `sem:exportedAt` tightened to
`xsd:dateTimeStamp`. That last one is the clearest single argument for this
layer. Timestamps were written naive, in two idioms that mean different
things, and a `FILTER(?t < "...Z"^^xsd:dateTime)` over them in Oxigraph drops
every affected row through XSD 1.1's indeterminate comparison. The query
returns an answer. The answer is missing the data it asked about.

## The MCP-native convention, and why it matters here

Open Ontologies deliberately contains no model client. Where a task needs
judgement (is this alignment candidate a true duplicate; what should this
community be called), the engine returns the structured evidence and the
*connected orchestrator* decides, feeding verdicts back through feedback
tools that retrain the scorers.

This is why the pairing composes rather than conflicts. The platform brings
its own models and its own pipeline; the engine never competes for that
role. It supplies the primitives a model cannot compute for itself: formal
semantics, sound inference, closed-world checks, deterministic community
structure, and an audit trail.
