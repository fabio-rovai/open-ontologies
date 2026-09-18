# Derivation certificates and the Lean checker

Every inference the forward-chaining reasoner makes can be written out with the rule that
produced it and the premises the rule read, and a checker in `lean/` verifies that record against a
formal semantics. The checker's soundness is a machine-checked theorem, so a certificate it accepts
contains only triples entailed by the asserted graph, whatever the Rust engine did to find them.

This page is the how-to. The design and its limits are in
[decision 0002](decisions/0002-an-inference-carries-a-certificate.md).

## Produce a certificate

```bash
# CLI: any of rdfs, owl-rl, owl-rl-ext. The tableaux path (owl-dl) refuses the flag.
open-ontologies load ontology.ttl
open-ontologies reason --profile owl-rl-ext --certificate /tmp/cert

# Batch, when the store is in-memory per process:
printf 'load ontology.ttl\nreason --profile owl-rl-ext --certificate /tmp/cert\n' \
  | open-ontologies --no-connect --data-dir /tmp/store batch -
```

Over MCP, `onto_reason` takes `certificate_dir`. The response gains a `certificate` object:

```json
{
  "inferred_count": 268,
  "certificate": {
    "dir": "/tmp/cert",
    "format": "oo-cert/1",
    "asserted": 1128,
    "derivations": 268,
    "by_rule": {"rdfs2": 41, "rdfs9": 190, "scm-eqc1": 3, "...": "..."},
    "check_with": "cd lean && lake exe oo-cert <dir>/asserted.tsv <dir>/derivations.tsv"
  }
}
```

`derivations` always equals `inferred_count`: one line per inferred triple, recorded the first time
it is derived.

## Check it

The checker needs a Lean 4 toolchain. [elan](https://github.com/leanprover/elan) installs the
version `lean/lean-toolchain` pins; nothing else is downloaded, the project has no dependencies.

```bash
cd lean
lake build                      # builds the checker AND checks the proofs
lake exe oo-cert /tmp/cert/asserted.tsv /tmp/cert/derivations.tsv
```

Exit codes: `0` every step checks; `1` a step was rejected, and the JSON on stdout names the first
one with its rule, conclusion and premises; `2` a file could not be read or parsed.

```json
{"ok":true,"asserted":1128,"derivations":268,"theorem":"OOCert.certificate_sound"}
```

## The files

Tab-separated. Terms are in N-Triples spelling, exactly as the engine's interner holds
them, so a term is spelled identically wherever it appears and tabs and newlines cannot occur
inside one.

- `asserted.tsv`: one triple per line, `s TAB p TAB o`. Every triple the run started from.
- `derivations.tsv`: one step per line, `rule TAB s TAB p TAB o` for the conclusion, then the
  premises as further triples, in the order documented per rule in `lean/OOCert/Rules.lean`.
- `refutation.tsv`, only when the run found a contradiction it can certify: the line `oo-refute/1`,
  then the derivation steps that reached the clash in the same spelling as `derivations.tsv`, then
  one `refute TAB rule` line with the clash rule's premises. See the refutation section below.

## What the file is NOT enough for

`derivations.tsv` is a derivation DAG and it is tempting to read explanations out of it. Do not.
It records ONE step per inferred triple, the first the fixpoint reached, which is exactly what a
checker that re-derives needs and exactly wrong for the question "which of my axioms did this". A
triple derived two independent ways has two justifications and the file shows one, so an
explanation built from it would be specific, confident and false.

`onto_justify` and `onto_provenance` therefore ask the reasoner for every applicable ground rule
instance instead, and never open the file. See [docs/explanation.md](explanation.md) and
[decision 0009](decisions/0009-a-conclusion-names-the-axioms-responsible-for-it.md). What the
certificate IS enough for, those tools use: with `certificate_dir`, each justification gets its own
certified run over its own triples, so `oo-cert` can verify that the conclusion really does follow
from that subset. Minimality is not in the theorem and is re-run instead.

## What is proved

`OOCert.certificate_sound` in `lean/OOCert/Soundness.lean`:

> if `checkCert G steps = true` then for every step, `G ⊨ step.conclusion`

where `G ⊨ t` is truth in every model of `G` under the semantics in `lean/OOCert/Semantics.lean`:
the RDF-based reading of the twenty-nine rules' vocabulary, with each semantic condition the *if*
direction of the W3C condition or a consequence of it, never more.

Weaker conditions admit more interpretations, so the result carries outward, and that transfer is
now a theorem rather than a claim. `lean/OOCert/W3C.lean` states the OWL 2 RDF-Based Semantics
conditions at full strength, one field per table cell quoted verbatim from the raw HTML of the
specification, and `OOCert.W3CEntails.of_entails` proves that everything `Entails` gives is true in
every `W3CModel`, which is an interpretation meeting those fields.
`OOCert.certificate_w3c_sound` is `certificate_sound` restated over that class, with the same
certificates and the same checker. **It is not the sentence "true in every conforming
interpretation", the file says so in its own docstring, and no document here may say otherwise.**
That sentence is `OOCert.certificate_conforming_sound`, a layer further out, and the section after
next is about it.

Fourteen arms of the soundness proof used to be *posited*: the theorem that `scm-dom1` is sound read
a field off `Conditions` which said that `scm-dom1` holds. All fourteen are now derived, and each
derivation is a proof term that depends on no axiom at all. The cell that makes them derivable is
Table 5.8's connective, which carries `rowspan="4"` in the source and so states an `iff` for
`rdfs:subClassOf`, `rdfs:subPropertyOf`, `rdfs:domain` and `rdfs:range`. RDFS alone gives those rows
only the `if-then` direction, and the twelve `scm-*` arms among the fourteen do not follow from it.

That step used to be an argument rather than a theorem. It is now a theorem, in
`lean/OOCert/Conforming.lean`, and this section records both the result and what it does not cover.

An OWL 2 RDF-Based interpretation is a Lean structure there, `OOCert.Interpretation`: the parts of
RBS Table 5.1, with `IR` as the carrier type so that Table 5.1's `IP ⊆ IR`, `IC ⊆ IR`,
`ICEXT(x) ⊆ IR` and `IEXT(x) ⊆ IR × IR` hold by typing, plus `IS`, `IL` and the blank-node
assignment of RDF 1.1 Semantics section 5.1. `ICEXT` and `IC` are definitions, per section 9's
"ICEXT(y) is defined to be …" and "IC is defined to be ICEXT(I(rdfs:Class))". Truth is section 5's
clause with its `I(p) is in IP` conjunct, and the bridge to `Interp` is chosen so that
`Interp.sat` and `Interpretation.Sat` are the same proposition: `OOCert.Interpretation.sat_eq` is
`Iff.rfl`.

`OOCert.Conforming.toW3C` then proves the reading that used to be prose, and
`OOCert.Conforming.toW3CModel` carries it to `W3CModel`, which includes proving that every
`Chain G` list is a semantic sequence in the specification's sense whenever `G` is satisfied
(`OOCert.Conforming.seq_of_chain`). `OOCert.certificate_conforming_sound` is the end of it: a
checked certificate's conclusions are true, in section 5's sense, in every such interpretation of
the asserted graph.

The five `IP` memberships that bridge used to assume, namely `I(rdf:type)`, `I(rdfs:subClassOf)`,
`I(rdfs:subPropertyOf)`, `I(rdfs:domain)` and `I(rdfs:range)`, are now discharged. Each is a field
of `OOCert.Conforming` asserting one axiomatic triple whose OWN PREDICATE is the term in question,
so section 5's truth clause hands the membership back as that triple's first conjunct, with no table
consulted:

| fact | axiomatic triple | table |
|---|---|---|
| `I(rdf:type) ∈ IP` | `rdf:type rdf:type rdf:Property .` | RDF 1.1 §8, RDF axioms |
| `I(rdfs:subClassOf) ∈ IP` | `rdfs:Datatype rdfs:subClassOf rdfs:Class .` | RDF 1.1 §9, RDFS axiomatic triples |
| `I(rdfs:subPropertyOf) ∈ IP` | `rdfs:isDefinedBy rdfs:subPropertyOf rdfs:seeAlso .` | same |
| `I(rdfs:domain) ∈ IP` | `rdf:type rdfs:domain rdfs:Resource .` | same |
| `I(rdfs:range) ∈ IP` | `rdf:type rdfs:range rdfs:Class .` | same |

Taking axiomatic triples at all is licensed by a three-link chain, quoted in the file: RBS
Definition 4.2 makes an OWL 2 RDF-Based interpretation a D-interpretation; RBS section 4.2 says a
D-interpretation "has to meet … those for RDF interpretations and RDFS interpretations"; RDF 1.1
section 9 says an RDFS interpretation satisfies "all the triples in the subsequent table of RDFS
axiomatic triples". Only five are taken rather than the whole tables, because importing more would
shrink the model class for nothing.

**The last row of that table is a correction.** `W3C.lean` sourced `I(rdfs:range) ∈ IP` to
`rdfs:range rdfs:domain rdf:Property .` with the note "whose truth puts its own predicate in `IP`".
The predicate of that triple is `rdfs:domain`, so its truth gives `I(rdfs:domain) ∈ IP` and not
`I(rdfs:range) ∈ IP`; the fact is still available from it, through Table 5.8's `rdfs:domain` row,
but that step was not in the entry. The triple above needs no such step.

The second kernel makes three of the same assumptions explicitly, as `c_type_IP`, `c_sco_IP` and
`c_spo_IP` in `isabelle/OO_Semantics.thy`, and needs no counterpart to the other two. An earlier
version of this documentation and of `W3C.lean` claimed the Lean needed none of them and was
therefore ahead of the Isabelle on this point; the opposite was true, and dropping the `IP` conjunct
from `Interp.sat` moved the obligation into the bridge rather than removing it. The Lean is now
ahead on this point for a different reason: it discharges all five where the Isabelle assumes three.

**What is left, and it is a containment rather than a reading.** `OOCert.Conforming` carries a
SUBSET of the Recommendation's conditions, which is the safe direction. A subset admits more
interpretations, so every conforming interpretation is one of these and a sentence true throughout
is true of every one of them. That it is a subset is checked cell by cell by a reader, not by Lean,
and the Recommendation has fifteen condition tables where this carries rows from six.

Three further items are named at their fields rather than folded into the word "conforming".
Blank nodes are rigid, which makes the entailment relation strictly smaller and the theorem
strictly stronger, the same decision and the same argument as `OO_Semantics.thy`'s DECISION M4.
`IEXT` is total on the carrier and every condition relativises to `IP` exactly where the
specification's quantifier does, as in DECISION M6. And `IL` is total, following RBS section 4.2's
wording rather than RDF 1.1 section 5's "partial mapping"; **this one does not run in the safe
direction**, because it excludes RDF 1.1 interpretations in which a literal fails to denote.
Closing it needs a term-occurrence lemma about `checkStep`, saying that every term of a checked
conclusion occurs in the asserted graph, so that in any model of the graph every such term denotes.
Nobody has written it.

One reading of a cell is load-bearing and is flagged at the field: RBS Table 5.4 at `n = 0`, where
`ICEXT(c1) ∩ … ∩ ICEXT(cn)` is read as `IR`. `Rules.lean`'s `takeChain` accepts an empty chain, so
`cls-int1` fires on `c owl:intersectionOf rdf:nil` and concludes `x rdf:type c` for every `x`, and
`Model.int` quantifies over the whole domain, so the engine and the existing Lean already depend on
that reading.

`lean/OOCert/ConformingWitness.lean` exhibits an interpretation satisfying every stated condition,
plus every RDF and RDFS axiomatic triple in both tables and three identity rows of RBS Table 5.2
that `W3CWitness.lean`'s model records itself as violating. It also states, as checked theorems,
which conditions it leaves untested (all eleven OWL-vocabulary extensions are empty there) and which
Table 5.2 row it breaks (`owl:Nothing | = ∅`).

`W3CModel` is also deliberately weaker than conformance, because `W3C` omits every table row no rule
consumes, so its class is larger than the bridge image. Both gaps run in the safe direction for
soundness, where a larger class makes the conclusion stronger, and neither runs in the safe
direction for a non-entailment result.

**The claim that the result carries over to the OWL 2 Direct Semantics read through triples is
withdrawn.** Nobody verified it, and it is false as written for those twelve arms, which need a
backward direction the Direct Semantics has no `rdfs:subClassOf` triples to carry. Do not reinstate
it without a formalisation of that translation.

The axioms the theorem depends on are pinned in the source by `#guard_msgs`:
`propext`, `Classical.choice`, `Quot.sound`. A `sorry`, or a `native_decide`, fails `lake build`.

## Why the theorem is not vacuous

A soundness theorem about an unsatisfiable semantics proves nothing: if no interpretation met the
conditions, every triple would be entailed and the checker could accept anything. `lean/OOCert/Witness.lean`
closes that by construction, and its own axiom lists are pinned the same way.

| theorem | says |
|---|---|
| `saturated_is_a_model` | every graph has a model, so the conditions are satisfiable and no graph is inconsistent here |
| `not_everything_is_entailed` | some triple is not entailed, so `Entails` is not the trivial relation |
| `the_old_svf_derivation_is_not_entailed` | `C ⊑ ∃p.D` with `x p y` and `y ∈ D` does **not** entail `x ∈ C` |
| `the_sound_half_survives` | the same premises **do** entail `x ∈ ∃p.D`, so the fix did not overshoot |

The third is the one worth reading. It is a machine-checked refutation of the derivation this engine
used to make: a model of the premises in which `x` is not a `C`. So the removed rule was unsound in
fact, not merely unjustified by the rule set the checker implements. The witness is the Herbrand
interpretation of the premises plus the single consequence the semantics does force.

### Which model class a negative result is about

Entailment transfers outward and non-entailment does not. A triple true in every model of the
weaker `Conditions` is true in every `W3CModel`, which is what `W3CEntails.of_entails` proves; a
triple *false* in some model of `Conditions` need not be false in any `W3CModel`, because that model
need not be one.

**So a `¬ Entails`, a `¬ Unsat` or an exhibited `Model I G` is a statement about the Lean's own
model class unless something restates it over `W3CModel`.** What that does *not* mean is that such a
restatement is hard. `IP` is a free parameter of `W3C`, not a field of `Interp`, and `sp_bwd`,
`dom_bwd` and `rng_bwd` are the only fields that take an `IP` membership as a hypothesis, so
`IP := fun _ => False` makes all three vacuous. `sc_bwd` is guarded by `IC` instead, and over a
Herbrand interpretation `IC` is the set of things the graph types as `rdfs:Class`, so a witness
graph with no such typing makes that field vacuous too. Nine statements here are a `¬ Entails` or a
`¬ Unsat`. ALL NINE are now over `W3CModel`: four with their existing witness interpretations
unchanged, four on a finite structure built for them, and one by unfolding.

| result | over `W3CModel` | how |
|---|---|---|
| `not_everything_is_entailed` | yes | `not_everything_is_w3c_entailed`, the empty graph's Herbrand interpretation |
| `the_natural_avf2_direction_is_not_entailed` | yes | `the_natural_avf2_direction_is_not_w3c_entailed`, a thirty-five-element structure built for it |
| `an_unlisted_individual_is_not_entailed` | yes | `an_unlisted_individual_is_not_w3c_entailed`, `ooWitness` unchanged |
| `membership_in_one_member_does_not_give_the_intersection` | yes | `membership_in_one_member_is_not_w3c_enough`, `intWitness` unchanged |
| `mix_not_absolutely_entailed` | yes | `mix_not_absolutely_w3c_entailed` in `Mixed.lean`, `mixH` unchanged |
| `the_old_svf_derivation_is_not_entailed` | yes | `the_old_svf_derivation_is_not_w3c_entailed`, an eighteen-element structure built for it |
| `feed_is_not_refuted` | yes | `feed_is_not_w3c_refuted`, a thirteen-element structure that also satisfies `RefuteConditions`; the verdict is `¬ W3CUnsat`, which is STRONGER than `¬ Unsat` |
| `and_the_old_verdict_does_not_notice` | yes | `the_old_verdict_does_not_notice_over_w3c`, the same carrier with one row changed |
| `not_unsat_of_joint_model` | yes | `not_w3cUnsat_of_joint_w3c_model`, by unfolding |

`Mixed.lean`'s `mix_relative_is_not_everything` and `HornWitness.lean`'s
`demo_not_everything_entailed` are `¬ EntailsR`, about the models of a graph that also satisfy a
supplied rule table. There is no `W3CEntailsR`, none is invented, and nothing is claimed about those
two beyond `Conditions`.

`mix_not_absolutely_entailed` is the one with a verdict hanging off it: it is the machine-checked
basis for a run reporting `entailed_under_supplied_rules` rather than `entailed`, and it is now the
strong version of that sentence rather than the one about this layer's own conditions.

The last four rows used to read "no", with the field that fails pinned as a `decide`-checked
theorem: `svfWitness` carries `R owl:onProperty p` and types `R` as nothing, so RBS Table 5.3's
`owl:onProperty` row fails on its first conjunct; `feedClosure` and `grazeClosure` carry
`Lion rdfs:subClassOf Carnivore` and type nothing as an `rdfs:Class`, so Table 5.8 row 1 forward
fails on the `IC` conjunct it concludes. Those theorems were true and they were about the HERBRAND
WITNESSES rather than about the results. A witness that is not a `W3CModel` is a reason to build one,
and `svfI` and `refI` are the structures that were built.

**An earlier version of this section said the opposite, and the reason it gave was inverted.** It
read: "a Herbrand witness carrying no `rdf:type` triple has `IC` empty, so Table 5.8's backward
direction forces `rdfs:subClassOf` and `rdfs:subPropertyOf` triples that the witness does not
contain", and concluded that "every conforming countermodel has to be a hand-built finite
structure". An empty `IC` is exactly what makes `sc_bwd` *vacuous*; `sp_bwd`, `dom_bwd` and
`rng_bwd` are guarded by a parameter the refuter chooses; and four of the results above transfer
with nothing hand-built.

One obstruction from that paragraph survives, and it says something narrower than it was used for.
`RefuteWitness.lean`'s `Der`, the repository's only general "every graph has a model" machine,
cannot be extended to carry Table 5.8's backward halves: the constructor would put `Der` to the left
of an arrow and Lean rejects the strictly negative occurrence. That is mathematical and not an
artefact of Lean, because `sc_bwd` is antitone in `ICEXT(a)` and there is no least fixed point by
monotonicity. What follows is that there is no general *closure operator* taking any graph to a
`W3CModel`. What does not follow is that no particular Herbrand interpretation is one.

The live witness, in `lean/OOCert/W3CWitness.lean`:

| theorem | says |
|---|---|
| `live_is_a_w3c_model` | a thirty-five-element interpretation meets the quoted cells of Tables 5.2, 5.3, 5.6, 5.8, 5.9, 5.12 and 5.13, so `W3C` is satisfiable |
| `live_is_live` | and it is not degenerate: `IC` is not the carrier, one class extension is everything, another is a proper subset witnessed on both sides, the filler extension is non-empty and proper, and `IEXT(p1)` is non-empty and a proper subset of `IEXT(p2)` |
| `live_exercises_every_arm` | all fourteen derivations fire at concrete instances of it, which is stronger than any field being non-empty |
| `live_fires_every_field` | a satisfied antecedent for each of the twenty-one fields of `W3C`, guards and quantified clause together for the four backward ones, so no field holds for want of anything to check |
| `sameAs_has_no_off_diagonal_instance` | the one residual limit, and it is about every interpretation rather than this one: RBS Table 5.9 row 1 is an `iff` whose right side is an equation, so `same_fwd` can be exercised and can never be exercised at `a ≠ b` |
| `sameAs_diagonal_needs_an_off_diagonal_subproperty` | and what that cell costs a model builder: with `owl:sameAs` carrying the diagonal, `IEXT(rdfs:subPropertyOf)` cannot be the diagonal too, so a countermodel needs two distinct properties one below the other |
| `the_natural_avf2_direction_is_not_w3c_entailed` | the reversed `scm-avf2` conclusion fails in that interpretation |
| `not_everything_is_w3c_entailed` | `W3CEntails` is not the trivial relation either |

The first version of that model was vacuous where it mattered most and is recorded in the file as
such. `IEXT(p1)` and `ICEXT(Y)` were both empty, nine of the twenty-one fields of `W3C` held because
nothing was in the extension their antecedent reads, six of the fourteen arms rested entirely on
those nine, and the refutation's own premise `p1 rdfs:subPropertyOf p2` held because `sp_bwd` had
nothing to check. Worse, the two emptinesses were conjuncts of `live_is_live`, the theorem whose job
is to catch exactly that. The rebuilt model gives `p1` a pair, `Y` a member, and each of the six
dead arms a configuration to fire on. It left five fields still vacuous, and a second rebuild closed
those too: `owl:sameAs`, `owl:inverseOf`, `owl:hasValue`, `owl:SymmetricProperty` and
`owl:TransitiveProperty` had been falling through the denotation table to the junk element, and they
now denote elements with rows of their own. The note that came with the five said `owl:sameAs`
"cannot be exercised by any model at all", which is false: RBS section 4.2 makes IR nonempty and
Table 5.9 row 1 makes the extension the whole diagonal on it, so a model that leaves it empty is
failing to model that cell rather than obeying it.

`the_natural_avf2_direction_is_not_w3c_entailed` is still not a proof that the triple fails to be
OWL 2 RDF-Based entailed, and nobody should write that sentence. `W3CModel`'s class is larger than
the conforming interpretations, because `W3C` omits every table row no rule consumes: the model
violates Table 5.2's `owl:Thing | = IR` and `rdf:Property | = IP` rows, and the axiomatic triple
tables are absent entirely. It is closer to the specification than anything else here and it does
not arrive.

## What is not proved

- Completeness. The checker rejects anything it cannot re-derive by pattern, including valid
  inferences in an order it does not expect. A rejection is a false alarm at worst, never a false
  pass.
- The parser and the file format (`lean/OOCert/Parse.lean`, `lean/Main.lean`). A parse error
  rejects.
- Anything outside the forward-chaining family: the SHACL validator (the pyshacl differential in
  `tools/shacl_differential.py` is its gate), the SHIQ tableaux reasoner, the RDF parsers.
- Datatype semantics. Two spellings of one literal value are two terms. No rule compares literals
  by value, so nothing is lost, but do not read `Entails` as datatype-aware.

## What it caught on day one

An adversarial audit of the whole layer on 13 September 2026 found nine more defects, seven of them
predating the certificate work: a false clean from unscoped prefix declarations, an ignored
`sh:deactivated`, a truncation that silently disabled any constraint mentioning a typed literal, a
blank-node shape acting as a wildcard, a reasoner that was not a fixpoint, four rules that could
emit an unserialisable triple, and a CI job that could not pass. All are fixed and pinned; the
CHANGELOG lists them.

`cls-svf1` in the `owl-rl-ext` profile derived `x rdf:type C` from `C rdfs:subClassOf ∃p.D`,
`x p y` and `y rdf:type D`. That is the converse of the axiom. It also treated `x p D`, with `D` the
filler class IRI itself, as a witness. Neither has a sound rule, so neither could be given one in
the checker, and both are gone (`tests/reason_rl_ext_soundness_test.rs`). The old derivation is
kept in `tests/lean_certificate_test.rs` as a forged certificate the checker must reject.

## In CI

The `lean` job builds `lean/` (which is the proof check), then runs
`tests/lean_certificate_test.rs` with `OO_REQUIRE_FIXTURES=1`. Every RDF file the repository
tracks, enumerated from `git ls-files`, is loaded, reasoned under `owl-rl-ext` with a certificate,
and the certificate checked. Files over 4 MB and files that do not parse are listed with the reason,
never dropped silently. The same test appends three forgeries and requires each to be rejected.

The corpus was 122 files and 37,133 derivations when that was written. Counted on 14 September 2026
under the test's own `SKIP_DIRS` filter it is **290 tracked RDF files, 21,256,445 bytes (20.3 MiB)**, one of which
(`case-studies/skills-england-occupational-maps/ontology/occupational-map.ttl`, 5.70 MB) is over the
4 MB cap and is therefore already excluded and named. The derivation count is not restated here,
because the only honest source for it is the run's own output.

The corpus used to be five hand-named directories, three of which hold no RDF, so 46% of the
repository's RDF was never walked and was excluded without being named. Widening it is what
surfaced the literal-subject defect in `prp-symp`, `prp-inv1`, `prp-inv2` and `eq-sym`, which lived
in `benchmark/`.

## Refutations: certifying that a graph has NO model

A derivation certificate says a triple follows. A *refutation* says the graph contradicts itself, and
it needs a second format because there is no triple to conclude. `lean/OOCert/Refute.lean` holds it,
`oo-refute` checks it, and `OOCert.refutation_sound` is proved about it.

The reasoner now writes one. `reason --certificate DIR` looks for a contradiction in the closure it
reached and, when it finds one the checker can judge, writes `DIR/refutation.tsv` beside the other
two files.

```bash
open-ontologies reason --profile owl-rl --certificate /tmp/cert
cd lean
lake exe oo-refute check /tmp/cert/asserted.tsv /tmp/cert/refutation.tsv
lake exe oo-refute guard /tmp/cert/asserted.tsv /tmp/cert/derivations.tsv /tmp/cert/refutation.tsv
```

`check` exits 0 when the refutation is valid, which is a good exit code reporting bad news. `guard`
runs both checkers and REFUSES the derivation certificate when the refutation succeeds, because over
a refuted graph every triple is entailed under the disjointness reading and an ordinary certificate
carries no information (`OOCert.a_certificate_adds_nothing_when_the_graph_is_refuted`). `oo-cert`
will still accept that certificate and will still be telling the truth: its verdict quantifies over
a model class that ignores disjointness, and that class is never empty.

The response gains an `inconsistency` object:

```json
{
  "inconsistency": {
    "found": true,
    "verdict": "clash_found_by_this_engine",
    "checked_by_lean": false,
    "by_rule": {"cax-dw": 1},
    "refutation": {
      "written": true,
      "rule": "cax-dw",
      "prefix": 2,
      "verdict": "refutation_written_not_yet_checked",
      "check_with": "cd lean && lake exe oo-refute check /tmp/cert/asserted.tsv /tmp/cert/refutation.tsv"
    }
  }
}
```

**Read the two verdicts apart.** `clash_found_by_this_engine` is this engine saying so, with nothing
behind it. `unsatisfiable_under_disjointness` is what `oo-refute` prints when it has ACCEPTED a
refutation, and the engine never states it.
`tests/lean_refutation_producer_test.rs::the_engine_never_states_the_checkers_verdict` fails if any
string in the response is ever that verdict, on the pattern the Horn layer's laundering guard set.

### Which rules conclude `false`, and which of them can be certified

Seventeen rules of the OWL 2 RL profile conclude `false` rather than a triple. None of them is in the
forward-chaining table, and none could be: a certificate step's conclusion is a triple. They are
looked for once, after the fixpoint.

| rule | detected | certifiable |
|---|---|---|
| `cax-dw` (`c1 owl:disjointWith c2`, `x a c1`, `x a c2`) | yes | **yes** |
| `cls-com` (`owl:complementOf`) | yes | no |
| `cls-nothing2` (`x a owl:Nothing`) | yes | no |
| `cls-maxc1` (`owl:maxCardinality 0`) | yes | no |
| `eq-diff1` (`owl:sameAs` and `owl:differentFrom`) | yes | no |
| `prp-irp` (`owl:IrreflexiveProperty`) | yes | no |
| `prp-asyp` (`owl:AsymmetricProperty`) | yes | no |
| `prp-pdw` (`owl:propertyDisjointWith`) | yes | no |
| `prp-npa1`, `prp-npa2` (negative property assertions) | yes | no |
| `cax-adc`, `prp-adp`, `eq-diff2`, `eq-diff3` | no, they read an RDF list | no |
| `cls-maxqc1`, `cls-maxqc2` | no, qualified cardinality is not implemented | no |
| `dt-not-type` | no, an ill-typed literal needs a datatype value space | no |

Only `cax-dw` is certifiable, and the reason belongs to the checker rather than to the engine.
`OOCert.RefuteConditions` carries exactly one semantic field, for `cax-dw`, and `oo-refute` refuses a
refutation naming any other clash rule with exit 2 rather than guessing at it. So the producer writes
a refutation for that rule alone. For the other nine it detects, the response says a clash was found
and `"refutation": {"written": false}` with the reason. Every rule the engine does not look for is
listed in `rules_not_detected` with its own reason, so a run with no clash is never a consistency
result: most of the table was not tried.

`lean/OOCert/Refute.lean` says sixteen rules conclude `false`. Counted against the W3C tables it is
seventeen; the difference is `dt-not-type`, which that file excludes elsewhere on the stated ground
that the Lean semantics has no datatype value space. Nothing computes with the number.

### The limit, which is real

`cax-dw` needs an INDIVIDUAL in two disjoint classes. A TBox that is unsatisfiable with no individual
asserted cannot be seen this way at all. `:Lion rdfs:subClassOf :Carnivore, :Herbivore` with those
two disjoint makes `:Lion` unsatisfiable, and until something is asserted to BE a `:Lion` the
rule-based route has nothing to fire on. That is a boundary of forward chaining, not a defect in the
producer.

The SHIQ tableau (`--profile owl-dl`, `src/tableaux.rs`) does see it, and reports
`unsatisfiable_classes`. **Its answer carries no certificate**, which its own report already states
(`"not_covered": "unsatisfiability and inconsistency carry no certificate"`). No refutation is
emitted from it, and the obstruction is precise rather than a matter of effort:

- `oo-refute/1` can express exactly one contradiction, `cax-dw`, whose three premises must each be
  asserted or concluded by a step of the 29-rule forward-chaining prefix. A tableau clash reached
  through `∃`/`∀` expansion, node merging or a cardinality bound is not three triples of that shape
  and there is nothing in the format to write it as.
- Where a tableau clash IS a `cax-dw` instance over triples the forward-chaining rules can reach, the
  rule-based producer has already found it, so the tableau adds nothing that can be certified.
- Feeding the tableau's inferred subsumptions back into the store and re-reasoning would let more
  clashes surface, but those subsumptions would then appear in `asserted.tsv` as axioms with nothing
  marking them derived. That is the assumption-laundering failure this layer exists to prevent, so it
  is not done.
- Adding a clash rule to the format means a new field in `OOCert.RefuteConditions`, a new arm in
  `OOCert.checkRefuteStep` and a new soundness case. That is a change to `lean/`, not to the producer.

`tests/lean_refutation_producer_test.rs::the_tbox_only_case_is_invisible_here_and_the_tableau_sees_it`
computes this boundary on one ontology rather than asserting it here.

`onto_defects` asks a different question again, and the three do not substitute for one another.
It checks the ontology against ITSELF with no data present, so `disjoint_with_ancestor` and
`inherited_disjoint` catch the schema that will contradict itself the moment an instance arrives.
This layer needs the instance to be there. The tableau needs neither and certifies nothing.

## User-written rules

The twenty built-in rules were twenty arms in the checker, which does not extend to rules you write.
`lean/OOCert/Horn.lean` makes a rule into data: a body and a head over triple patterns with
variables. A certificate step cites a rule by index into a table you supply and gives a binding, and
one theorem, `OOCert.horn_certificate_sound`, covers every rule table at once.

```bash
cd lean
lake build
lake exe oo-horn rules                                    # the built-in table, as data
lake exe oo-horn check RULES.tsv ASSERTED.tsv HORN.tsv    # check a certificate
```

**The verdict tells you what it is relative to, and you must read it.**

| table | verdict | theorem | means |
|---|---|---|---|
| exactly the built-ins | `entailed` | `OOCert.entails_of_builtin_horn` | true in every model of the asserted graph |
| anything else | `entailed_under_supplied_rules` | `OOCert.horn_certificate_sound` | true in every model that **also satisfies your rules** |

The second is weaker and the difference is not academic. A rule reading "every supplier is
compliant" makes certificates that check green for ever, because the certificate certifies the
inference and never the premises. The report carries a digest of the table that was in force so two
runs can be compared, and `tests/lean_horn_certificate_test.rs` fails if a user-rule run ever
reports the absolute verdict.

Twenty-seven of the engine's rules are Horn rules and appear in the built-in table, the count
`the_built_in_rules_are_emitted_as_data` pins. Four are not: `cls-int1`, `cls-int2`, `cls-uni` and
`cls-oo` each read an RDF list off the graph, so the LIST is a premise and the premise count is data
rather than fixed by the rule. They remain hardcoded arms. A user rule language stops at the same
boundary.

### What a binding must look like

A step's binding is a list of `var TAB term` pairs and **it is data before it is a substitution**.
Two shapes are refused, on both checkers:

| shape | example | why |
|---|---|---|
| a key bound twice | `x <http://ex.org/a> x <http://ex.org/zzz>` | the two pairs demand two different values for one variable, so NO substitution satisfies both and there is nothing for a checker to read: first-wins (Lean's `List.lookup`, Isabelle's `map_of`) and last-wins (Python's `dict()`, Rust's `HashMap::from_iter`) are two conventions for discarding half the certificate, not two readings of it |
| a variable of the cited rule with no binding | binding `s`, `o` for the rule `?s <p> ?o -> ?s <q> ?z` | the checker would otherwise have to invent a value, and the only one available is the variable's NAME, so the conclusion would carry a term nobody wrote |

A binding for a variable the cited rule never mentions **is accepted**: it is never consulted, so it
cannot make a certificate mean two things.

Both refusals are strictness and neither carries soundness content. What they buy is that a
certificate has ONE meaning, which `OOCert.wellFormed_determines_instantiation` states and the Lean
kernel checks. Read the binding as a set of demands, "this variable is that term", one per written
pair. Distinct keys make those demands SATISFIABLE and coverage makes every substitution satisfying
them instantiate the cited rule the SAME WAY, so one refusal buys existence and the other buys
uniqueness. The defect they close was found by running this
checker and the independent Isabelle/HOL one in `isabelle/` over 1,718 certificates: 47 rows
disagreed, all from this one cause. See
[decision 0008](decisions/0008-a-binding-is-data-and-evidence-admits-one-reading.md).

This does **not** make a conclusion writable RDF. Binding `z` to the bare term `z` is required and
still accepted, so a certificate can still conclude a triple no serialiser can write; what changed is
that the term must be written by the certificate's author rather than minted by the checker.
`reason --rules` cannot emit either refused shape, which
`tests/reason_horn_emit_test.rs::no_emitted_binding_is_one_the_format_now_refuses` checks against the
bytes rather than asserting.

`reason --rules TABLE --certificate DIR` evaluates a supplied table to a fixpoint over the loaded
graph and writes the three files `oo-horn check` reads. Nothing is materialised: a conclusion drawn
under a table nobody has checked holds only in models that satisfy that table, and writing it into
the store beside the assertions would lose exactly that distinction. See
[decision 0003](decisions/0003-a-rule-is-data-and-an-assumption-is-not-a-fact.md).

## Rules you wrote in SWRL or RIF Core

`rules.tsv` is an internal encoding, not a language anybody writes in. `rules-import` reads two
standard rule syntaxes into it.

```bash
open-ontologies load ontology.ttl
open-ontologies rules-import --from swrl --out rules.tsv              # SWRL, out of the loaded graph
open-ontologies rules-import --from swrl --file rules.owl --out rules.tsv
open-ontologies rules-import --from rif  --file rules.xml --out rules.tsv
open-ontologies reason --rules rules.tsv --certificate cert/
cd lean && lake exe oo-horn check ../cert/rules.tsv ../cert/asserted.tsv ../cert/horn.tsv
```

**Only part of each language is a Horn table over triple patterns, and the exact fragment is in
[docs/rule-syntax-front-ends.md](rule-syntax-front-ends.md) and in every response.** A rule outside
it is named, counted and refused; by default one refusal fails the whole import and writes nothing,
because a table that quietly lost a rule still reaches a fixpoint and still produces a certificate
that checks green, which is a sound proof about a rule set nobody wrote. `--allow-partial` imports
the rest and flags the result `certifies_a_weaker_rule_set`.

Every imported rule is a rule you wrote, so it lands on the second row of the table above without
exception: imported rules are named `swrl/…` and `rif/…`, no built-in rule is, and `oo-horn` awards
the absolute verdict only to a table that renders identically to the built-in one.
The Rust reasoner evaluates a supplied rule table through `reason --rules RULES.tsv --certificate
DIR`, which writes `rules.tsv`, `asserted.tsv` and `horn.tsv` and states no verdict of its own.
Nothing is materialised into the store: a conclusion drawn under a table you wrote holds only in
models that satisfy that table, and merging it in beside the assertions would lose exactly the
distinction this is about. See
[decision 0003](decisions/0003-a-rule-is-data-and-an-assumption-is-not-a-fact.md).

## What the checker assumes about the engine

The theorem is conditional. It says that IF the asserted graph is `G` and IF these steps check,
THEN the conclusions hold in every model of `G`. Everything to the left of that is the Rust writing
down the truth, and the checker cannot see any of it: it reads files, not the store and not the
run. `docs/trusted-computing-base.md` enumerates that boundary as thirty checkable properties
and says for each what checks it. Read it before relying on a certificate, because a certificate is
a claim about a file and the file's relationship to the store is the part nobody proved.
## First-order model certificates: `oo-folmodel`

A solver's `sat` answer is worth nothing on its own and everything once the model comes with it.
`lean/Fol/` holds a checker for a finite first-order structure, and `Fol.satisfiable_of_check` says
that a structure it accepts really does satisfy every formula of the problem, so the problem is
satisfiable. `Fol.not_entails_of_check` is the sharper form: a checked model of `¬φ` together with
`Γ` is a machine-checked proof that `Γ` does **not** entail `φ`.

```bash
cd lean
lake build                                                  # builds the checker AND checks the proofs
lake exe oo-folmodel PROBLEM.tsv MODEL.tsv
```

```json
{"verdict":"model_checked","formulas":5,"domain":2,"closed":true,
 "goal_negated_present":true,"source":"z3","cardinality_search":"1,2",
 "problem_digest":"4403d8aaa0c422f7","theorem":"Fol.satisfiable_of_check",
 "non_entailment_theorem":"Fol.not_entails_of_check"}
```

The two file formats, with a worked example whose digest is pinned by the build, are documented in
`lean/Fol/Syntax.lean`. Runnable copies of every case below are in `tests/fixtures/folmodel/`.

**Exit codes, and the fixture that demonstrates each.**

| exit | verdict | reason | fixture |
|---|---|---|---|
| 0 | `model_checked` | | `problem.tsv` + `model.tsv` |
| 1 | `rejected` | the checker rejected the structure | `model_no_edge.tsv` |
| 1 | `rejected` | `problem_digest_mismatch` | `model_wrong_digest.tsv`, `problem_other.tsv` |
| 1 | `rejected` | `undeclared_symbol` | `model_undeclared.tsv` |
| 2 | `unreadable` | the carrier is empty | `model_empty_domain.tsv` |
| 2 | `unreadable` | a row contradicts a declared arity | `model_bad_arity.tsv` |
| 2 | `unreadable` | an index is outside the carrier | `model_index_out_of_range.tsv` |
| 2 | `unreadable` | a declared symbol has no row | `model_missing_row.tsv` |
| 2 | `unreadable` | the problem carries no formulas | `problem_empty.tsv` |

**The verdict tells you what it is relative to, and you must read it.** These four are the SOLVER
level, and a driver that collapses any two of them is reporting a falsehood from correct solver
output.

| solver said | encoding | verdict | theorem | means |
|---|---|---|---|---|
| `sat`, with a model the checker accepted | either | `model_checked` | `Fol.satisfiable_of_check` | the problem has a model, exhibited |
| `sat`, no checkable model | either | `satisfiable_oracle` | none | trust the solver |
| `unsat` | `finite(k)` | `no_model_up_to_size_k` | none | **not** unsatisfiability |
| `unsat` | `unbounded` | `unsatisfiable_oracle` | none | trust the solver, and never more |

`no_model_up_to_size_k` is the one people get wrong. `∀x∃y (r(x,y) ∧ x≠y)` is `unsat` at carrier 1
and `sat` at carrier 2, so a bounded `unsat` is frequently not evidence of anything. A solver that
answers `sat` whose model the checker then REJECTS is `satisfiable_oracle` with `checker_exit: 1`,
never `rejected` as though the ontology were at fault.

**Two gates that are not soundness gates, and are labelled so.**

- `undeclared_symbol` is an ATTRIBUTION gate. `Fol.FinModel`'s three fields are total, so an
  undeclared symbol is still interpreted and the theorem holds without the gate. What it buys is
  that the structure certified is the structure the solver described rather than that structure
  completed with defaults. The JSON carries `"gate":"attribution"` so a report cannot quietly
  promote it.
- `problem_digest_mismatch` binds `model.tsv` to `problem.tsv`. The digest is FNV-1a 64 over the
  canonical re-serialisation of the parsed formula list, specified in `lean/Fol/Parse.lean` so a
  Rust writer can reproduce it. It identifies; it does not commit.

**Cost, measured on this machine** (Apple silicon, compiled `oo-folmodel`, 2000 formulas at
quantifier depth 3, so `carrier^3` evaluation points per formula):

| carrier | evaluation points | user time |
|---|---|---|
| 8 | 1.0M | 0.14 s |
| 16 | 8.2M | 0.92 s |
| 24 | 27.6M | 2.93 s |
| 32 | 65.5M | 6.80 s |

About 9M points per second, cubic in the carrier at this depth. Solvers return carriers in the
single digits on real ontologies, so this is a cap to state rather than a problem to solve:
`--max-domain` defaults to 16 and the flag goes higher with the cost documented.

### Driving all of it in one command: `fol-model`

`oo-folmodel` checks a model somebody already has. `open-ontologies fol-model` produces one: it
exports the ontology, runs Z3 or Mace4, reads the structure back, runs the checker, and reports the
verdict with the five fields above.

```bash
cd lean && lake build && cd ..                     # the checker, and its proofs
printf 'load case-studies/blast-furnace-ironmaking/blast-furnace-ontology.ttl\n\
fol-model --out /tmp/solve --solver z3 --max-domain 8\n' \
  | open-ontologies --no-connect --data-dir /tmp/store batch -
```

```json
{"verdict":"model_checked","solver_verdict":"sat","encoding":"finite(1)","checker_exit":0,
 "owl_reading":null,"theorem":"Fol.satisfiable_of_check","formulas":45,
 "problem_digest":"27751aa61e9de6fd","cardinality_search":[1],
 "bounded_search":"a model was reported at carrier 1","dropped_symbols":[],"seconds":0.084}
```

Every intermediate file lands in `--out`, so a run is reproducible by hand from what it leaves
behind: `problem.tsv` and its digest, `problem_k1.smt2` (or `problem.in` plus `symbols.tsv` for
Mace4), the solver's raw output, `model.tsv`, and `checker.json`.

Pass `--goals` a TSV of triples — `derivations.tsv` from `reason --certificate` is the intended
input, with `--goals-skip-columns 1` — and each becomes its own run asking whether the ontology
FAILS to entail it. A `model_checked` there carries `owl_reading:
"not_entailed_under_unproved_translation"`, which is `Fol.not_entails_of_check` plus two things
this layer does not prove: `OwlLean.adequacy` lives in a sibling project, and the Rust-to-Lean
translation correspondence is pinned by tests and not proved. Never shorten it to "not entailed".

**A rejected model stops the line.** If a solver answers `sat` and the checker rejects the
structure, the verdict stays `satisfiable_oracle` — it is not a fact about the ontology — but the
report carries a `disagreement` block with `severity: STOP_THE_LINE`, the run summary counts it,
and the command exits non-zero. That is the treatment `tools/shacl_differential.py` gives a
FALSE_CLEAN, for the same reason: a disagreement between a checker and the thing it is checking is
the one result that must fail a pipeline.

**The two finders differ in what they can be asked, and the verdict rules read the difference.**

| | Z3 | Mace4 |
|---|---|---|
| format | SMT-LIB 2 (`fol --format smtlib`) | LADR (`fol --format ladr`) |
| smallest carrier | 1 | 2 — `mace4 -n 1` is a FATAL error, measured |
| unbounded question | yes, so `unsatisfiable_oracle` is reachable | no, it is a finite model finder |
| symbols | quoted with vertical bars | MANGLED to `p0`/`r0`/`c0`, table in `symbols.tsv` |
| reduct | auxiliary functions | Skolem functions and constants |

Mangling is not a style choice. LADR reads a name whose first letter is in `{u,v,w,x,y,z}` as a
VARIABLE, so an IRI beginning with one of those becomes universally quantified and Mace4 silently
searches a different problem. Measured on LADR 2009-11A: `p0(w0). -p0(k0).` is echoed in Mace4's
own `CLAUSES FOR SEARCH` block as `p0(x).` and `-p0(k0).`, and the run reports `exit (exhausted)`
with no error at all.

## What is proved, and what is not, on this layer

Every result in `lean/Fol/` has axiom footprint `[propext, Quot.sound]` or smaller, pinned by
`#guard_msgs` in the source. `Classical.choice` appears nowhere in the layer, which is tighter than
the triple the older layers carry.

| theorem | says |
|---|---|
| `Fol.eval_iff` | the Boolean evaluator and the `Prop` satisfaction relation agree, at every formula and environment |
| `Fol.satisfiable_of_check` | an accepted structure is an existence proof: the problem is satisfiable |
| `Fol.check_complete` | the gate can fail: a rejection is about the structure, not the checker's patience |
| `Fol.check_complete_closed` | for a problem of sentences, a rejected structure is not a model under **any** assignment |
| `Fol.not_entails_of_check` | a checked model of `¬φ :: Γ` proves `Γ` does not entail `φ` |

`lean/Fol/Witness.lean` closes the vacuity question by construction: `employment_is_satisfiable`
accepts a real structure, `the_model_does_not_satisfy_everything` shows that same structure refuses
a formula, `not_everything_is_satisfiable` shows `Satisfiable` is not the trivial predicate, and
five named forgeries are each rejected on their own through `check_complete`, so it is the
checker's own rejection that is certified rather than an independent argument.

Not proved, and named next to the verdict rather than in a footnote:

- **Nothing about unsatisfiability, in any direction, ever.**
- **The absence of a finite model implies nothing.** SHIQ lacks the finite model property, so a
  satisfiable ontology can have only infinite models and will never receive a certificate.
- **The OWL-level reading** rides on `OwlLean.adequacy` in the sibling project *and* on the
  Rust-to-Lean translation correspondence, which decision 0005 states is pinned by tests and not
  proved. It carries its own word, `not_entailed_under_unproved_translation`, which must never be
  shortened to "not entailed".
- **The parser and the file formats** (`lean/Fol/Parse.lean`, `lean/FolMain.lean`). A parse error
  is exit 2 and never a verdict in either direction.

Nothing in `src/` writes either file yet, so today this layer checks certificates rather than
producing them. See
[decision 0006](decisions/0006-a-model-is-a-certificate-and-a-refutation-is-not.md).
## Does a retrieved slice still support the answer?

A certificate says the engine's inferences over ONE graph are entailed by it. A GraphRAG pipeline
asks a different question: an answer was grounded in a retrieved slice, so does that slice still
entail the claims the answer rests on?

`preserve` and `graph_projection_entailment_check` ask it per claim, and every preserved-and-derived
claim gets a SUB-CERTIFICATE extracted from the projection's own `derivations.tsv` and checked by
`oo-cert` against the projection's own `asserted.tsv`. No new Lean was written:
`OOCert.certificate_sound` covers it unchanged, and the sub-certificate's line order is valid
because `run_full` computes a round's conclusions from the closure as it stood at the start of the
round, so every premise was asserted or concluded strictly earlier.

The verdict table follows the one above.

| verdict | theorem | means |
|---|---|---|
| `preserved_checked` | `OOCert.certificate_sound` | the projection entails the claim, machine-checked |
| `preserved_under_supplied_rules_checked` | `OOCert.horn_certificate_sound` | true in every model of the projection that **also satisfies** your rules. Carries the table's digest. Never shortened |
| `preserved_asserted` | none | the claim is literally in the slice. A lookup, not a theorem |
| `preserved_unchecked` | none | the ENGINE derived it and no checker looked |
| `lost_under_profile_unchecked` | none | not derivable from the slice under this profile, bounded by a rule table covering 29 of the profile's 78 rules |
| `ungrounded_in_source` | none | NEITHER graph derives it, so the generator invented it and a better retriever will not help |

Every run also checks monotonicity: OWL RL is monotone and a projection is a subset, so anything the
projection entails and the source does not is a soundness bug in this engine, reported at
`STOP_THE_LINE` rather than as a retrieval result. It is disarmed, loudly and with its own reason,
when the projection is not a subset, when blank nodes could not be matched, or when either run
stopped at the iteration cap instead of a fixpoint.

Run over the shipped corpus, 275 ontologies swept with the gate armed on all 275 and every source
certificate accepted by `oo-cert`, it found nothing.

See [docs/projection-entailment.md](projection-entailment.md) and
[decision 0007](decisions/0007-a-slice-preserves-a-conclusion-or-it-does-not.md).

## Known limitations

Stated rather than discovered later.

- **A conclusion no RDF serialiser can write is refused, not derived.** A literal in subject
  position or a non-IRI in predicate position produces a triple the store cannot hold. Such a
  conclusion is not materialised, not certified, and not used as a premise, so a run that hits one
  derives LESS than its rule set licenses. The count and up to three examples come back as
  `skipped_unserialisable` and `skipped_examples`. This is reachable from ordinary OWL: `:p
  rdfs:subPropertyOf [ owl:inverseOf :q ]` is what the OWL 2 mapping to RDF produces for
  `SubObjectPropertyOf(:p ObjectInverseOf(:q))`, and it makes `rdfs7` conclude a triple with a
  blank node in predicate position.
- **The rule table is this engine's, not W3C's.** `by_rule` describes what this engine did. It is
  not an OWL 2 RL conformance claim, and the engine does not implement every rule in the profile.
- **The Rust and Python reasoners are NOT independent, so their agreement is worth less than it
  reads.** `tools/horn_differential.py` runs `src/reason.rs`, the pure-Python engine in
  `python/src/open_ontologies_lite/horn/` and `oo-horn` over every RDF document the repository
  tracks, and reports a clean sweep. The two ENGINES run the same semi-naive forward-chaining
  algorithm over the same rule table; the Python was written with the Rust open and its comments
  cite `src/reason.rs` by file and line. That makes their agreement STRONG evidence against a
  transcription slip in one of the two — an index off by one, a guard dropped, a join in the wrong
  order — and CLOSE TO NO evidence against a shared misreading of a W3C rule, which both would
  implement and agree on for ever. The independent leg is the Lean checker, written from the W3C
  rules with a machine-checked soundness theorem. The tool prints this caveat next to its agreement
  count on every run. Never quote the sweep as "two independent reasoners agree".
- **Seven of the twenty-seven rules fire nowhere in the repository's own RDF.** Measured on 14
  September 2026: the corpus exercises 20 of 27, and `prp-inv2`, `eq-sym`, `cls-avf`, `cls-hv1`,
  `cls-hv2`, `scm-svf2` and `scm-avf2` never fire in it, so the differential compared the two
  engines on them zero times. `tests/fixtures/horn-coverage/` holds one deliberately built graph
  per silent rule, each the smallest thing that makes exactly that rule fire, and the differential
  runs them as a separate labelled set. The two figures are reported apart and must not be added:
  20 of 27 on real ontologies, 27 of 27 once fixtures written for the purpose are included. A rule
  covered only by its own fixture has been compared on one graph made to fire it and on no real
  data. Re-measured on 15 September 2026 over a corpus that had meanwhile grown to 291 documents
  and 203,825 asserted triples: the same 20, the same seven silent, and all seven fixtures firing
  exactly their own rule once. No rule in the table is unreachable, so the seven were a gap in the
  corpus and not a defect in the engine.
- **`asserted.tsv` is the store, not your file.** Quads are flattened, so a triple present in two
  named graphs appears on two lines. Literals are in the store's post-parse canonical spelling, so
  `"01"^^xsd:integer` is written `"1"^^xsd:integer`. The guarantee is relative to that file.
- **Reasoning twice into one store.** The run reaches a fixpoint, so a second run adds nothing. If
  you materialise into the DEFAULT graph of a store that already held inferences, they appear in
  `asserted.tsv` as assumptions with nothing marking them derived: the merge is what the caller
  asked for and it is lossy. `inference_graph: true` (decision 0001) now fixes it rather than
  mitigating it. The certified paths read `GraphStore::triples_outside(&[INFERRED_GRAPH])`, so a
  later run does not read an earlier run's conclusions back as axioms, and the JSON reports
  `graphs_read` and `graphs_excluded` so the certificate says which graphs it is about. Until 15
  September 2026 `all_triples` read every named graph and the separation protected `save` and not
  the certificate; TCB-8 in `docs/trusted-computing-base.md` has the history.
- **No clash found is not consistency.** Seven of the seventeen clash rules are not looked for at
  all and eight of the ten that are cannot be certified, so a clean `reason` run means "none of the
  ten rules tried fired", never "this ontology is consistent". A rejected refutation means the same:
  `oo-refute`'s own report says so in every verdict it prints.
- **One clash is reported per refutation, and only the first certifiable one is written.** A graph
  with twenty disjointness clashes gets one `refutation.tsv`. `clash_count` and `by_rule` carry the
  rest; ten are listed in full under `clashes`. Refuting a graph once is enough to invalidate every
  derivation over it, so a second refutation would add nothing.
- **The refutation's derivation prefix is not byte-stable across runs.** Which of several valid
  derivations of a premise the fixpoint recorded first depends on the same hash iteration order that
  already decides the line order of `derivations.tsv`. The contradiction itself IS stable: clashes
  are sorted on their N-Triples spelling before one is chosen.
- **A SHACL report double-counts a node selected by two target declarations of one shape.** Both
  `focus_nodes` and `violation_count` are affected. Collapsing identical results is not the fix:
  SHACL emits one result per SPARQL solution, pyshacl does too, and deduplicating breaks an exact
  agreement with it. The fix is to union a shape's focus nodes across its declarations, which is a
  change to the evaluation loop and is not done.
