# Does the slice still support the answer?

`graph_projection_lossy_check` reports how much of a seed's neighbourhood survived a retrieval.
That number is a proxy. It is neither necessary nor sufficient for the property that matters, and it
moves the wrong way: it rises as the projection grows, so a retriever tuned on it learns to fetch
more rather than to fetch the right thing.

The property is **entailment preservation**. For a source `G`, a retrieved slice `P` and a claim `q`
an answer rests on, the question is whether `P` entails `q` exactly when `G` does. That is decidable
under the engine's rule tables, it is checkable, and every preserved claim carries a certificate the
Lean checker accepts.

This page is the how-to. The design and its limits are in
[decision 0007](decisions/0007-a-slice-preserves-a-conclusion-or-it-does-not.md).

Everything here MEASURES what a slice lost, which is the right thing to do to a retriever and the
wrong thing to want from a subset. If what you need is a part of the ontology that cannot lose an
entailment over a set of terms, extract a locality module instead:
[docs/modules-and-conservativity.md](modules-and-conservativity.md).

## The number that cannot see the damage

Measured on `benchmark/reference/pizza-reference.owl`, which this repository ships. Seeds are the
twenty-three classes under `NamedPizza`; the claim is `Veneziana rdfs:subClassOf Food`, which the
source derives through `NamedPizza rdfs:subClassOf Pizza` and `Pizza rdfs:subClassOf Food`.

| projection | `aggregate_coverage_ratio` | `ok` | the claim |
|---|---|---|---|
| the whole ontology minus that one `subClassOf` triple | **1.00000** | **true** | **lost** |
| three triples: `Veneziana ⊑ NamedPizza ⊑ Pizza ⊑ Food` | **0.00128** | false | **preserved, checked** |

Both rows are tests, in `tests/lean_projection_entailment_test.rs`. The first is why the coverage
ratio cannot be read as assurance; the second is why deleting it and reporting the size of the
difference set instead would not help either.

## Ask it of the claims an answer rests on

```bash
# The store is in-memory per process, so `batch` is the way to load a source
# and then ask about a slice of it in one run, exactly as for
# `reason --certificate`.
open-ontologies --no-connect --data-dir /tmp/store batch - <<'EOF'
load benchmark/reference/pizza-reference.owl
preserve --projection /tmp/slice.ttl --goals /tmp/goals.ttl --out /tmp/preserve \
         --profile owl-rl-ext \
         --seed https://raw.githubusercontent.com/owlcs/pizza-ontology/refs/heads/master/pizza.owl#Veneziana
EOF
```

`--goals` is a Turtle document in which every triple is a claim. It is parsed by the SAME parser the
store uses and round-tripped through a store, which is the only thing that makes a caller's spelling
of a term comparable with the interner's: `"01"^^xsd:integer` is stored as `"1"^^xsd:integer`, and a
goal that skipped the store would match nothing and be reported lost. A TSV works too, with
`--goals-skip-columns 1` so `derivations.tsv` from `reason --certificate` pipes straight in.

Over MCP, `graph_projection_entailment_check` takes `goals_ttl` and either `projected_ttl` or
`projection_graph`.

Exit codes: 0 every goal preserved and nothing refused, 1 something was lost, ungrounded or refused,
2 a stop-the-line disagreement.

### Where the goals come from

Four sources, ranked by how much trust each needs. The tool never extracts claims itself: a server
that decomposed an answer into triples would be embedding a language model in the server, which this
project has ruled out.

1. **The BGP of the query the answer was rendered from, with the answer's bindings substituted.** No
   trust in a generator at all, fully mechanical. `projection_entailment::goals_from_bindings`.
2. **The generator's own declared support set.** At query time the system knows what it is about to
   assert, so it lists the triples each sentence rests on before emitting. This is the intended
   caller.
3. **A prior full-graph certificate.** `derivations.tsv` over `G`, filtered to the conclusions that
   mention the seeds, turns the tool into a regression suite for the retriever at index-build time.
4. **Competency questions**, which an ontology project already has and which are goals by
   construction.

## Ask it of everything at once

```bash
open-ontologies --no-connect --data-dir /tmp/store batch - <<'EOF'
load benchmark/reference/pizza-reference.owl
closure-diff --projection /tmp/slice.ttl --out /tmp/diff --profile owl-rl-ext
EOF
```

No goals. Both graphs are reasoned to a fixpoint under the same table and `closure(G) \ closure(P)`
is reported. This is the offline form, for auditing a retrieval **strategy** rather than one answer,
and `SourceClosure::build` is paid once so a run comparing fifty projections pays for `closure(G)`
once.

The headline is `lost_in_projection_vocabulary`, not `lost_total`. `|closure(G) \ closure(P)|` grows
with `|G|` and is nearly all of it for any real slice, so minimising it means retrieving more: the
same perverse gradient wearing a better name. A lost conclusion is dangerous exactly when every one
of its terms occurs in `P`, because then an answer grounded in `P` is a claim over those terms. That
number has its own gaming direction, since shrinking `terms(P)` shrinks it, and the report says so in
the payload.

Each lost inference names the premises `closure(P)` does not hold. That is the triple the retriever
dropped, and the conclusion went with it.

## The verdicts

Three kinds of word, never collapsed.

| verdict | produced by | theorem | means |
|---|---|---|---|
| `preserved_checked` | the goal is in `P`'s `derivations.tsv`, the sub-certificate was extracted, and `oo-cert` exited 0 over `P`'s own `asserted.tsv` | `OOCert.certificate_sound` | `P` entails the goal, true in every model of `P` under `lean/OOCert/Semantics.lean`. Machine-checked |
| `preserved_under_supplied_rules_checked` | the same, over a SUPPLIED Horn table, pronounced by `oo-horn check` | `OOCert.horn_certificate_sound` | true in every model of `P` that **also satisfies** the supplied rules. The rules are assumed and never checked. Carries the table's sha256. Never shortened |
| `preserved_asserted` | the goal is in `P`'s `asserted.tsv` by exact string | none, and none is required | the goal is literally in the slice. Set membership over the canonical spelling. Not a theorem and it does not pretend to be |
| `preserved_unchecked` | the goal is derived and no checker ran | none | the ENGINE derived it. Its opinion about its own output, which is what the certificate layer exists to stop being trusted. The report carries the install line and the exact `lake exe oo-cert` command |
| `lost_under_profile_unchecked` | in `G`'s certificate and in neither part of `P`'s | none | not derivable from `P` under this profile. The engine implements 29 of OWL 2 RL's 78 rules, so this is bounded by the rule table and is not "`P` does not entail it" |
| `ungrounded_in_source` | in NEITHER certificate | none | the source does not derive it either, so the projection did not lose it. The fix is in the generator and not the retriever. Open-world: this is not "false" |
| `projection_only` | in `P`'s certificate and not in `G`'s | none, and it is a defect report | see the differential below |
| `certificate_rejected` | the checker REJECTED the sub-certificate this code assembled | none | a defect HERE or in the emitter. It downgrades to nothing, exits 2, and never becomes `preserved_unchecked` |
| `refused` | not a positive ground triple | none | counted in the headline so a refusal cannot shrink the denominator unnoticed |

For the closure diff, the warrant on a lost row is one of `checked`, `asserted_in_source` (a lookup,
so the checked count cannot rise without the checker running) and `engine_opinion`.

`coverage_ratio`, `aggregate_coverage_ratio`, `total_dropped_predicates` and
`total_dropped_objects` are not verdicts. They live under `coverage_proxy`, carry
`is_a_warrant: false`, and carry the label in the payload.

## The free differential

OWL RL is monotone and `P` is a subset of `G`, so `closure(P) ⊆ closure(G)`. A conclusion of `P` that
is not a conclusion of `G` is a **soundness bug in the engine**, not a retrieval finding, and it
exits 2 with `severity: "STOP_THE_LINE"`.

It is only allowed to speak when its antecedent holds:

| subset verified | both at a fixpoint | `P`-only conclusions | outcome |
|---|---|---|---|
| yes | yes | none | `armed`, nothing to report |
| yes | yes | some | `STOP_THE_LINE`, `monotonicity_violated`, exit 2 |
| no | either | any | `disarmed`, `projection_is_not_a_subset`. A `P`-only entailment from a non-subset is ordinary |
| yes | no | any | `disarmed`, `fixpoint_not_reached`. A truncated closure is a lower bound and cannot refute anything |
| blank nodes unmatched | either | any | `disarmed`, `blank_nodes_unmatched` |

A second stop-the-line, on the same block shape, is `certificate_rejected`, when `oo-cert` exits 1 on
a sub-certificate this code assembled. That means the extractor or the emitter is wrong. It
downgrades to nothing.

Subsethood is **computed** every run, never inferred from provenance: a retriever that normalises
IRIs, re-prefixes, inlines an imported vocabulary or adds a tidy `rdf:type owl:Class` declaration
produces a non-subset that looks like a faithful slice.

### What it found

Run over the shipped corpus, each ontology skolemised, sliced by `onto_segment_retrieve` from its
twelve busiest subjects at two hops, both closures certified and diffed
(`tests/projection_monotonicity_corpus_test.rs`):

```
swept                     275
gate ARMED                275
gate DISARMED               0
source certificate CHECKED 275
VIOLATIONS (stop-the-line)  0
incompleteness warnings     0
conclusions lost, total          117105
  of them in the slice's own vocabulary 899
excluded                    9      (four over the 1 MB sweep cap, five unparseable; each named)
seconds                    41.6
```

Nothing was found. That is worth more than it looks because the gate was ARMED on all 275 rather
than disarmed: subsethood held, both runs reached a fixpoint, and every source closure carried a
certificate `oo-cert` accepted. A sweep in which the gate is disarmed everywhere establishes
nothing, so the test fails if the gate was armed on fewer than half the corpus.

## Watching the gates fail

A gate that cannot fail is decoration, and a gate whose failure nobody has read is close to it.
`tests/gate_demonstration_test.rs` feeds each gate the input built to trip it, prints what the tool
actually said, and asserts the same thing:

```bash
cargo test --test gate_demonstration_test -- --nocapture
```

Thirteen gates, each with its broken input named on the line above the tool's answer. Nothing in it
is a mock.

## Blank nodes

`owl:Restriction` superclasses, `owl:intersectionOf` list cells and `owl:Axiom` reification are all
blank-node bearing, so a TBox slice of any real ontology hits this immediately. Oxigraph mints fresh
labels on every parse, so a slice serialised and re-read differs textually from the source in every
blank-node-bearing triple: the subset check reports spurious extras and the differential fires
against a correct engine.

**Skolemise the source before the retriever sees it.** `projection_entailment::skolemise` replaces
every blank node with an IRI under `https://open-ontologies.org/.well-known/genid/`, which RDF 1.1
section 3.5 reserves for exactly this. `closure-diff` does it by default. Measured on
`pizza-reference.owl`: 400 blank nodes skolemised, a 783-triple retrieved slice, zero triples not in
the source, the gate armed.

**Do not reach for `GraphStore::canonicalize_blank_nodes`.** RDFC-1.0 labels are a function of the
WHOLE graph, so the same blank node canonicalises differently in a slice than in the source, exactly
because the slice has fewer triples around it. Measured on `pizza-reference.owl` against a genuine
subgraph: canonicalising both sides separately and diffing reports 630 triples of the subgraph as
absent from the source, out of 2,276, where comparing the store's own labels reports 0. It is correct
for "are these two graphs isomorphic", which is not the question, and it looks principled, which is
worse. `canonicalising_both_sides_separately_is_worse_than_doing_nothing` pins it.

Deciding whether one graph is a subgraph of another up to blank-node renaming is simple entailment:
NP-complete in general, polynomial only when the target is ground. The `not_compared` bucket is a
consequence of that complexity rather than an unfinished feature, and it is never subtracted from a
denominator.

## What is refused, and why

- **A blank node in a goal** is an existential, not a claim. This layer asks whether a ground triple
  is entailed and cannot ask "there exists something such that".
- **A closed-world or negative claim**: "no supplier is sanctioned", "exactly one registered
  address". Not a triple. Reading "absent from the closure" as "the claim holds" would convert an
  open-world absence into a closed-world fact with a certificate stapled to it, which is the
  laundering this tool exists to attack, produced by the anti-laundering tool. The refusal message
  points at `onto_shacl`.
- **An empty goal set** is an error, not a green report. Every aggregate over zero goals is trivially
  perfect, and "all my goals were refused" is the realistic way to arrive at zero.

## Known limits

- `OOCert`'s entailment is not datatype-aware. `"1"^^xsd:int` and `"1"^^xsd:integer` are two distinct
  goals, and a `preserved_checked` on one says nothing about the other.
- A `lost_*` verdict is an engine opinion about a negative and carries no certificate in this layer,
  ever.
- The rule table is this engine's, not W3C's: 29 of OWL 2 RL's 78 rules.
- The coverage proxy is subject-only, so an inbound-edge loss is invisible to it, and its SPARQL
  carries `LIMIT 1000`, so a seed with more than a thousand outbound triples has its ratio computed
  against a truncated source. Seeds in that position are listed in `truncated_seeds`.
- A store that has been reasoned into makes `asserted.tsv` a lie, because the engine's own output
  appears as an axiom and `ungrounded_in_source` becomes unreachable.
  `source_hygiene.source_side_unreliable` says so; reload the source from its files.
