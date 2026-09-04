# 0001 · An inference is not an assertion

- **Status**: implemented for the OWL-RL family · `InferenceTarget::Inferred` writes to
  `https://open-ontologies.org/graph/inferred`, `serialize` withholds that graph from the triple
  formats, `onto_reason` takes `inference_graph` · **opt-in, default unchanged** · the `owl-dl`
  tableaux path refuses the target rather than pretending to honour it · not done: `onto_pack`,
  `onto_shacl` and `onto_query` have not been told which dataset they want
- **Written**: 2026-09-04
- **Related**: flaw hunt 30 Aug 2026 defect D2; the temporal named-graph work of issue #95, whose
  serialiser fix this reuses and narrows

## The problem

`reason` merged materialised triples into the default graph with no marker. After a run,
`SELECT DISTINCT ?g WHERE { GRAPH ?g { ?s ?p ?o } }` returned empty and an inferred triple was
byte-identical in form to one a person had written. `save` then wrote them out: a source file of
8 triples came back as 9, newly asserting `<http://ex.org/ghost> a <http://ex.org/Person>`, which
nobody ever wrote and which is only true under a range entailment over a dangling reference.

The laundering was therefore not transient. It could be, and would be, published. For a tool whose
subject is register integrity that is the wrong defect to be carrying.

## Decisions

1. **A separate graph, not a marker triple.** Both make the distinction recordable; they fail in
   opposite directions. A marker obliges every consumer to filter, and the consumer that forgets
   publishes an inference as an assertion. A separate graph means the consumer that forgets sees
   fewer triples and never wrong ones. Choose the defence whose failure mode is silence.
2. **The triple formats withhold the inference graph; the dataset formats keep it.** This is the
   part that actually closes the defect, and naming the graph alone would not have. `serialize`
   deliberately flattens named graphs into the default graph for Turtle, RDF/XML and N-Triples,
   because that is the only thing a triple format can do (issue #95). Flattening the inference
   graph is exactly the laundering, so it is dropped instead. TriG, N-Quads and JSON-LD can carry
   the graph name, so they carry it and lose nothing.
3. **Opt-in, and the default is untouched.** Existing callers read inferences out of the default
   graph, `onto_query` among them. Flipping the default is a behaviour change for every downstream
   pipeline and is a separate decision, taken deliberately, not a side effect of fixing the
   provenance gap.
4. **`owl-dl` refuses the target rather than silently ignoring it.** The tableaux path materialises
   through its own code and has not been taught the graph. Accepting the flag and merging into the
   default graph anyway would leave a caller believing the inferences had been kept apart. An
   error naming the two profiles that do support it is the honest answer, and matches the
   discipline the SHACL layer already keeps for unimplemented constraint components.

## Dead ends

- **A marker triple on each inferred statement.** Rejected on decision 1, and also because
  a marker on a triple has nowhere to live in RDF without reification, which changes the shape of
  the data to record a fact about it.
- **Refusing to `save` at all after a reason run.** Considered because it is the loudest possible
  signal. Rejected: it punishes the legitimate case, which is saving what you asserted, and the
  narrower rule already makes the wrong outcome impossible.

## Open questions

- **`onto_shacl` and `onto_query` still read the default graph.** With the flag on, a shape now
  validates the asserted graph alone. That is arguably the more useful default, since validating
  the closure is what turns a genuine failure into a pass (defect D3), but it is a choice nobody
  has made explicitly and it should be made rather than inherited.
- **`onto_pack` writes sorted N-Triples.** A pack built after a reasoning run therefore now
  excludes the inferences. That is the safe direction and probably the right one, since a pack is
  a promotable artefact, but the manifest should say so rather than leaving it to be discovered.
- **When the default flips**, `all_triples()` reads the whole store including named graphs, so a
  second reasoning run consumes the first run's output as premises. OWL-RL is monotone, so the
  closure is unchanged and `inferred_count` correctly reports zero new triples, but the reported
  `initial_triples` then includes inferences and no longer means "what was asserted".
