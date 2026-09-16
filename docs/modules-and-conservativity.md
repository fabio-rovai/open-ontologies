# A subset with a theorem, and a change with a consequence

Two questions the rest of this repository could measure but not answer.

1. *Give me the part of this ontology that matters for these terms, and do not lose anything.*
   `onto_segment_retrieve` gives a neighbourhood and `onto_closure_diff` tells you what the
   neighbourhood dropped, which is the right pair when you are auditing a retriever and the wrong
   pair when you want a subset you can rely on. `onto_module_extract` gives one.
2. *I am about to add these axioms. Does anything I already said change?* `onto_plan` reports added
   classes, removed classes, blast radius and a risk score, and every one of those is about SHAPE.
   `onto_conservative_check`, and the `conservativity` block of `onto_plan`, answers the semantic
   question.

The design and its limits are in
[decision 0011](decisions/0011-a-module-carries-a-theorem-and-a-slice-carries-a-measurement.md).

## Modules

### What the guarantee is

For a signature `Σ`, an axiom is `⊥`-local when replacing every class and property name outside `Σ`
by `⊥` turns it into a tautology, and `⊤`-local when the same replacement with `⊤` does. Take every
axiom that is NOT local with respect to `Σ ∪ sig(M)`, repeat until the set stops growing, and the
result `M` satisfies

> `M ⊨ α` if and only if `O ⊨ α`, for every axiom `α` over `Σ`.

That is the locality theorem of Cuenca Grau, Horrocks, Kazakov and Sattler, JAIR 31 (2008),
*Modular Reuse of Ontologies: Theory and Practice*. It holds for OWL 2 DL and therefore for every
profile inside it. It is a COVERAGE guarantee, not a MINIMALITY one: a locality module is the
smallest set the syntactic test can justify, not the smallest set that would do.

**The theorem is cited, not machine-checked.** Nothing under `lean/` is about syntactic locality, so
the report names a paper and never names a Lean theorem, and
`the_module_report_never_names_a_lean_theorem` asserts that over the serialised report rather than
leaving it to good intentions.

### Running it

Both features are MCP tools over the loaded store; there is no CLI subcommand for either yet, so
load the ontology with `onto_load` (or `onto_repo_load`) and then call the tool.

```json
{
  "tool": "onto_module_extract",
  "signature": [
    "https://raw.githubusercontent.com/owlcs/pizza-ontology/refs/heads/master/pizza.owl#Veneziana",
    "https://raw.githubusercontent.com/owlcs/pizza-ontology/refs/heads/master/pizza.owl#Food"
  ],
  "locality": "star",
  "verify_out_dir": "/tmp/pizza-module"
}
```

`locality` is `bottom` (`⊥`), `top` (`⊤`) or `star` (the iterated `⊥⊤*`, the default and never
larger than either).

### What it says, measured

From `tests/module_extract_test.rs::the_guarantee_holds_on_the_pizza_ontology`, over the two-term
signature above:

| | |
|---|---|
| ontology | 1,345 axioms, 2,332 triples |
| `⊥⊤*` module | **238 axioms, 510 triples** |
| axioms this file could not classify | **0** |
| conclusions over the signature the module does not reach | **0**, out of 2,583 examined |
| annotation triples the module drops by design | 60, counted separately |

The last two rows are the point. `verify_out_dir` runs `onto_closure_diff` with the module as the
projection, so the guarantee is EXERCISED against the engine's own closure rather than asserted; a
non-zero `lost_over_signature_total` would be a defect in the extractor, not a property of the file.
Annotation triples are partitioned into `lost_over_signature_annotation_only` rather than filtered
out, because a silent filter is how a real loss would eventually hide there.

The verification has a blind spot and it is COUNTED. The module is serialised with the store's own
blank node labels and the closure diff skolemises the source before comparing, so a conclusion
carrying a blank node or a Skolem IRI has no term whose name means the same thing on both sides and
the signature question cannot be asked of it. `not_decided_blank_node_bearing` is that number, and
it is non-zero on pizza, which is full of blank-node restrictions. Skolemising the store before
extracting the module empties the bucket; nothing does that yet. This is the same complexity
[decision 0007](decisions/0007-a-slice-preserves-a-conclusion-or-it-does-not.md) documents and
declines to close.

### Why not a neighbourhood

A module and a neighbourhood look identical from outside, so here is the case that separates them.

```turtle
ex:Cat    rdfs:subClassOf ex:Mammal .
ex:Mammal rdfs:subClassOf ex:Animal .
ex:Animal rdfs:subClassOf ex:LivingThing .
ex:Plant  rdfs:subClassOf ex:LivingThing .
ex:Fungus rdfs:subClassOf ex:LivingThing .
ex:tom    a               ex:Cat .
ex:tom    ex:eats         ex:fish .
```

Signature `{Cat, LivingThing}`. The goal is `Cat ⊑ LivingThing`, which the whole ontology derives.

| | triples | `Cat ⊑ LivingThing` |
|---|---|---|
| every triple MENTIONING a signature term | **5** | **lost** |
| the `⊥⊤*` module | **4** | **preserved** |

The naive slice is BIGGER and still wrong. It loses because `Mammal ⊑ Animal` mentions neither
signature term while the chain runs through it; the module keeps that link because once
`Cat ⊑ Mammal` has been taken, `Mammal` is in the working signature and the next link stops being
local. Both rows are
`tests/module_extract_test.rs::a_naive_signature_slice_drops_an_entailment_the_module_keeps`.

### What is classified, and what is included without being classified

An RDF graph is not a list of OWL axioms, so the axioms have to be recovered from triples. Every
recovery failure resolves the same way: an axiom that cannot be classified is NEVER local, so it is
INCLUDED. Any `M'` with `M ⊆ M' ⊆ O` still has the coverage property, so including too much costs
size and excluding too much costs the theorem.

**Classified**, each with a locality test written for that kind: `rdfs:subClassOf`,
`owl:equivalentClass`, `owl:disjointWith`, `owl:AllDisjointClasses`, `owl:disjointUnionOf`,
`rdfs:subPropertyOf`, `owl:propertyChainAxiom`, `owl:equivalentProperty`,
`owl:propertyDisjointWith`, `owl:AllDisjointProperties`, `rdfs:domain`, `rdfs:range`,
`owl:inverseOf`, the seven property characteristics, `owl:hasKey`, class assertions, property
assertions, `owl:NegativePropertyAssertion`, declarations and annotations. Class expressions are
read through `owl:intersectionOf`, `owl:unionOf`, `owl:complementOf`, `owl:oneOf`,
`owl:someValuesFrom`, `owl:allValuesFrom`, `owl:hasValue`, `owl:hasSelf` and the six cardinality
forms, with `owl:inverseOf` on property expressions.

**Included conservatively**, with no locality test: `owl:sameAs`, `owl:differentFrom` and
`owl:AllDifferent`, which mention no class and no property name, so no replacement can make them
tautologies; every unrecognised predicate in the RDF, RDFS, OWL or XSD namespaces; every
blank-node structure whose shape is not one of the above; every malformed `rdf:List`; every class
expression nested past the depth bound. The counts are in `included_conservatively` and the rows in
`unclassified_axioms`, so "the module is small" and "the module is small because half the file was
unreadable" cannot render the same.

An unrecognised predicate in a USER namespace is a different decision: `ex:a ex:p ex:b` is an
`ObjectPropertyAssertion` under OWL 2 semantics, so it is `⊤`-local and may go. If every unknown
predicate were kept, the module would be the ontology and the guarantee would be free.

### Three places the direct semantics and this engine disagree

Resolved towards the rule table in all three, because the module has to satisfy both readings.

- **`X rdf:type owl:Class`** is logically vacuous in OWL 2 and is a PREMISE of OWL 2 RL's `scm-cls`.
  Kept whenever the declared term is in the working signature.
- **`X rdf:type owl:Thing`** is a tautology in OWL 2 and the rule table does not REGENERATE it, so
  dropping it removes a member of the closure. Read as a declaration for the same reason. This one
  was not predicted: the pizza verification reported five entailment losses over the signature and
  they were `Germany rdf:type owl:Thing` and four like it.
- **A datatype is not an external class name.** Replacing `xsd:integer` by `⊥` makes
  `∃hasAge.xsd:integer` look `⊥`-equivalent and drops the axiom, and the module still parses, so
  nothing looks broken. Data ranges are neither `⊥`- nor `⊤`-equivalent under any signature.

## Conservativity

### The question

An ontology is EXTENDED. Does it now say anything different about the terms that were already
there? This is `onto_closure_diff` pointed the other way: source `base ∪ extension`, projection
`base`, and `closure(G) \ closure(P)` restricted to triples every name of which the base already
used. Nothing is recomputed, so the rule, the blocking premises and the verdict discipline all come
from the existing code.

```json
{
  "tool": "onto_conservative_check",
  "extension_ttl": "@prefix ex: <http://ex.org/> . ex:hasParent rdfs:domain ex:Person .",
  "mode": "delta",
  "out_dir": "/tmp/cx"
}
```

Or, where it belongs, inside a plan:

```json
{"tool": "onto_plan", "new_turtle": "...", "check_conservativity": true}
```

### The one-triple change every other number misses

Base: `ex:Person a owl:Class`, `ex:hasParent a owl:ObjectProperty`, `ex:Alice ex:hasParent ex:Bob`.
Extension: one triple, `ex:hasParent rdfs:domain ex:Person`.

| what `onto_plan` reports | |
|---|---|
| `added_classes` | 0 |
| `removed_classes` | 0 |
| `risk_score` | `low` |
| `conservativity.conservativity_verdict` | **`not_conservative_under_rule_table`** |
| the row | `ex:Alice rdf:type ex:Person`, rule `rdfs2`, premise the base lacked: the domain axiom |

That is `tests/conservativity_test.rs::a_plan_reports_conservativity_or_reports_that_it_did_not_look`.
Every shape number stays green while every existing subject of the property is retyped.

A non-conservative extension is a **finding, not an error**. Changing what the ontology says about
existing terms is often the intended change; the point is that it should be intended rather than
discovered later. Exit 1 means there is something to look at, exit 2 means the question could not be
answered.

### The verdict words

`conservativity_verdict` is one of:

| word | meaning | `conservative_under_rule_table` | exit |
|---|---|---|---|
| `conservative_under_rule_table` | this rule table derives nothing new over the old names | `true` | 0 |
| `not_conservative_under_rule_table` | it does, and the rows say what | `false` | 1 |
| `undecided_scan_truncated` | the difference was larger than `scan_rows` | `null` | 2 |
| `undecided_not_an_extension` | the proposal REMOVES asserted triples, so the question does not apply | `null` | 2 |
| `undecided_engine_soundness_violation` | the base derived something the extended graph did not, which is impossible unless the engine is wrong | `null` | 2 |

There is deliberately no field called `conservative`, and the boolean is null rather than false
whenever the answer is "we could not tell", because a reader takes `false` for a finding.

The last row gets its own word rather than being folded into `not_conservative_under_rule_table`.
The base is a subset of the extended graph by construction and the rule table is monotone, so a
conclusion the base reaches and the extension does not is a soundness bug in the engine. Reporting
it as a non-conservative extension would send it to the ontology's author instead of the engine's,
which is the same mistake decision 0007 refuses to make about a lossy retriever and a hallucinating
generator.

### What this is NOT

Conservativity here is **with respect to the Horn rule table named in `rule_table`**, which derives
only positive ground triples. It is not deductive conservativity in a description logic, which is
ExpTime-complete for `EL`, 2ExpTime-complete for `ALC` (Ghilardi, Lutz and Wolter, KR 2006) and
UNDECIDABLE for `ALCQIO` (Lutz, Walther and Wolter, IJCAI 2007); and it is not model conservativity,
which is undecidable already for `EL` (Lutz and Wolter, JSC 2010).

The asymmetry is the point, and it travels in the payload as `what_this_is_not` rather than only
here:

- a new consequence reported **is** a real change to what this engine derives over the old names;
- finding none establishes only that this rule table derives nothing new, and never that the
  extension is conservative in any logic.

### Delta or replacement, asked for rather than guessed

A delta that restates one triple of the base and a replacement that dropped everything else are the
same bytes, and the two answers are opposite. `onto_plan` fixes the mode to `replacement`, because
that is what it receives. `onto_conservative_check` defaults to `delta` and refuses an unknown word
by name.

### Blank nodes

The base is skolemised under its OWN prefix before the extension is merged in. Two graphs
skolemised separately under the same prefix collide: the map is `_:label ↦ prefix + label`, so
`_:b0` on both sides would become one IRI naming two different existentials. The discriminator ends
in `/`, which cannot occur in an N-Triples blank node label, so no default-prefix skolem IRI can
spell a base-prefix one. The result is that the monotonicity gate stays ARMED on ontologies full of
restrictions, which `blank_nodes_in_the_base_do_not_disarm_the_gate` pins.

## Known limits

- **Neither feature reads a supplied rule table.** The conservativity verdict would then have to be
  `conservative_under_supplied_rules`, which is decision 0003's word, and it is not written.
- **`owl:hasKey` is decided on the class only.** `HasKey(C, (p))` with an empty `p` is vacuously
  true and could be local; the more conservative test is used, so some `hasKey` axioms sit in
  modules that do not need them.
- **`≥n R.C` for `n ≥ 2` is never treated as `⊤`-equivalent**, because `≥2 ⊤.⊤` fails in a
  one-element domain. Conservative, and it costs a few axioms on ontologies that lean on qualified
  cardinalities.
- **The module is not minimal**, and `guarantee` says so in the payload rather than leaving
  "smallest" to be read as minimality.
- **`module_fraction` is not a score.** A module is not better for being smaller; it is correct or
  it is not, and its size is what the signature costs.
