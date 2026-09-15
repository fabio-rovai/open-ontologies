# 0008 · A binding is data, and evidence admits one reading

- **Status**: implemented · `lean/OOCert/Horn.lean`, where `bindingWellFormed` is a conjunct of
  `checkHornStep`, and `wellFormed_determines_instantiation` is the theorem that says what it
  buys · `horn_certificate_sound`, `horn_certificate_sound_fo` and `SatRuleFO.to_SatRule` are
  unchanged in statement and in axiom footprint, and the new material adds no axiom to either:
  it is free of `Classical.choice`, which `horn_certificate_sound` uses · nothing under
  `isabelle/` except its README is touched, and `OO_Check.thy` in particular is UNTOUCHED,
  because it was written independently and that independence is the whole value · the refusal
  can fail in Lean
  (`rejects_duplicate_binding_key`, `rejects_binding_that_omits_a_rule_variable` in
  `HornWitness.lean`) and on the real bytes
  (`tests/cross_kernel_differential_test.rs::resolved_d1_*`, `::resolved_d2*`) · the producer
  structurally cannot emit either shape, argued from `run_horn`'s construction and checked against
  the emitted bytes by
  `tests/reason_horn_emit_test.rs::no_emitted_binding_is_one_the_format_now_refuses` and
  `::a_head_variable_the_body_never_binds_never_reaches_the_evaluator`
- **Written**: 2026-09-14
- **Related**: decision 0003 (a rule is data, and an assumption is not a fact), whose format this
  amends; decision 0002 (an inference carries a certificate)
- **Run in CI since 15 September 2026, and not before.** This bullet used to say the opposite, and
  it was right: the differential was a LOCAL gate, because no workflow installed the second kernel,
  so the file skipped in the one job that ran it and was invoked by no job that could have made it
  strict. It needed Poly/ML rather than Isabelle2025-2, which is the thing that was wrong with the
  old reasoning: `isabelle/driver/oo_horn_generated.ML` is committed, so a 39 MB component installs
  it. The `lean` job now runs the file under `OO_REQUIRE_FIXTURES=1`, and
  `tests/ci_gate_coverage_test.rs` fails if any test file that can skip ever again runs nowhere.
  `lean_horn_certificate_test` and `reason_horn_emit_test` carry the Lean-side and producer-side
  halves and always did.

## The problem

The repository carries two verified checkers for one Horn certificate format: the Lean in
`lean/OOCert/Horn.lean` and an independent Isabelle/HOL formalisation in `isabelle/`, written from
the W3C sources with the Lean deliberately unread. Run over 1,718 certificates they agreed on 1,671
and disagreed on 47, always in the same direction, with one root cause:

**Isabelle validates the binding list as a data structure and Lean did not.** Isabelle's
`check_step` requires `distinct (map fst b)` and `binding_covers b r` before it instantiates
anything. Lean's `substOf` was `fun v => (List.lookup v l).getD v`, which turns any binding list
into a total function with a silent default, and it was applied without the list being inspected
at all.

Neither checker was unsound, and saying so is not a softening. Lean's soundness theorem quantifies
over whatever total substitution it built, so every acceptance held in its own theorem; Isabelle's
extra rejections were false alarms, which is the harmless direction. **The defect was in the
FORMAT.** Nothing in `docs/lean-certificates.md` or decision 0003 said what a repeated binding key
meant or what an incomplete binding meant, so two readings of the same bytes were both defensible
and a certificate's validity depended on which verified checker read it.

Two files make it concrete.

`isabelle/fixtures-added/bad_dup_key.tsv` binds `x` twice, first to `<http://ex.org/a>`, which makes
the step check, and then to `<http://ex.org/zzz>`, which does not. Lean returned exit 0 and the
ABSOLUTE verdict `entailed`. Isabelle returned `binding_dup_key`. Which one was right turned on
`List.lookup` being first-wins: a tie-break inside a standard-library function standing in for a
rule about a file format, deciding whether a certificate is valid.

`isabelle/fixtures-differential/unsafehead_cert.tsv` cites the rule `?s <p> ?o -> ?s <q> ?z` over
one ordinary triple of IRIs, binds `s` and `o`, and omits `z`. Lean accepted a conclusion whose
object was the bare term `z`, which is not an IRI, not a blank node, not a literal and not
writable RDF, minted out of a variable's NAME in the rule file. Nobody wrote that term. The fuzzer
then found the same hole through a one-character typo (`?o` became a bare `?`, read by both parsers
as a variable whose name is the empty string), which is what settles the objection that the first
case was contrived.

## The decision

**A binding list is DATA before it is a substitution, and a malformed binding is refused rather than
repaired.** `checkHornStep` now requires, of the binding and the rule it cites:

1. **Distinct keys.** A repeated key is refused.
2. **Coverage.** Every variable the cited rule mentions, in the body AND in the head, has a binding.

Both refusals are strictness with no soundness content, and that is the point. What they buy is a
sentence a soundness theorem cannot say: the certificate means ONE thing.
`wellFormed_determines_instantiation` states it and the kernel checks it.

The statement is worth reading carefully, because it is EXISTENCE AND UNIQUENESS and the two halves
are what the two refusals separately buy. Read a binding list as what it looks like: a set of
demands, "`v` is `t`", one per written pair. The membership reading, `(v, t) ∈ l → σ v = t`, is
deliberately not a statement about `List.lookup`, so it speaks about a checker carrying a partial
map, a checker carrying a total function with a default, and a checker nobody has written yet. Then
once the binding is well formed for the cited rule:

- **something meets those demands**, and `substOf` is it. This is what DISTINCT KEYS buy, and it is
  `substOf_extends_of_keysDistinct`.
- **everything that meets them instantiates the rule identically.** This is what COVERAGE buys, and
  it is `instantiation_unique_of_covers`.

Neither conjunct of `bindingWellFormed` is decorative, and `HornWitness.lean` refutes each half with
its own hypothesis dropped: `no_substitution_extends_a_duplicate_key` and
`two_extensions_of_an_uncovered_binding_disagree`.

### Why refusing, rather than writing the permissive reading into the format

Both questions had a permissive answer available, and both were rejected, though not on the same
ground. The repeated key turns out to be the easier of the two.

On the **repeated key**, the alternative was to make first-wins normative. One sentence would have
made the format determinate, so this is a real option and not a straw man. It is the wrong one, and
the reason is sharper than a preference between library conventions.

**A repeated key with two different values makes the certificate's own demands unsatisfiable.** The
pair `("x", a)` says `x` is `a` and the pair `("x", b)` says `x` is `b`, and no total substitution
does both. That is `no_substitution_extends_a_duplicate_key`, and it holds for every checker at
once rather than for this one. So first-wins and last-wins are not two readings of such a
certificate. There is no reading of it. They are two conventions for ignoring half of what it says,
and a normative first-wins rule would be a rule about which half to discard. A format should not
have one.

That argument covers a repeated key whose two values DIFFER, which is the case the fixture and the
corpus are made of, and it does not cover a key repeated with the SAME value. `keysDistinct` refuses
that too, and it is worth being honest that the refusal is a different kind there: the demands are
satisfiable, there is exactly one reading, and refusing is tidiness rather than determinacy. It is
still right, for two reasons that are not the one above. The decisive one is that Isabelle's
`distinct (map fst b)` refuses it, so a Lean that admitted it would REINTRODUCE the divergence this
decision exists to close, and the Isabelle is not available to be edited. The other is that
`distinct` is one primitive every language has, while "distinct keys, or repeated keys whose values
agree" is a rule an implementer has to get right on purpose, for a shape no producer needs to emit.

The library split is then the second-order point rather than the argument, and it is why the damage
would be silent. Of the four association-list primitives an implementer is most likely to reach
for, two are first-wins (`List.lookup` in Lean, `map_of` in Isabelle, whose own source comment in
`OO_Check.thy` names this) and two are last-wins (`dict(pairs)` in Python, `HashMap::from_iter` in
Rust, both of which insert in order and let the later value win). A normative first-wins rule would
be broken by half of that short list, and broken by ACCEPTING a certificate that means something
else rather than by failing. `distinct` has no such split: every language has it, and getting it
wrong is loud. Against that, a duplicate key carries no information a producer needs, since there
is no certificate writable with one that is not writable without one, so refusing costs exactly
nothing.

On the **incomplete binding**, the argument is harder, because on the question of ENTAILMENT the
permissive reading was right and Isabelle was the incomplete one. `EntailsR` quantifies over every
total substitution, so a step whose body instantiates into known triples under SOME total
substitution really does entail its conclusion. The refusal is therefore a deliberate choice of
well-formedness over completeness, and it is the right one for two reasons. First, the permissive
reading does not merely admit more certificates: it FABRICATES a term out of a variable's name and
puts it in the conclusion, so what the certificate says depends on a rule file's choice of
identifier. Second, and this is what makes the trade free, it loses no certificate anybody meant. A
step whose binding omits a variable can always be rewritten with that variable bound to the term the
conclusion already shows, and the rewritten step is accepted by both kernels.
`the_repair_is_accepted` in `HornWitness.lean` and
`the_refusal_costs_no_certificate_anybody_meant` in the differential are that claim, run.

### What is NOT refused

**A binding for a variable the cited rule never mentions is accepted.** It is never consulted, so it
cannot make a step mean two things, and there is no second reading to remove. Refusing it would be a
tidiness rule dressed as a determinacy one. Both kernels already agreed here
(`probe_extrabind`, exit 0 on both) and they still do.

### Can this engine emit what is now refused?

No, and structurally rather than by care, which matters because a producer that could write
certificates its own checker rejects would make the refusal a bug report against ourselves.
`run_horn` in `src/reason.rs` builds a step's binding as `rule.vars.iter().enumerate().map(...)`,
one pair per entry, so the binding is exactly `rule.vars` and the two questions become questions
about that one list.

- **No repeated key.** `rule.vars` is accumulated with `!vars.iter().any(|x| x == v)` guarding every
  push, so it is deduplicated before a single binding is written and a repeat has no way in.
- **No omitted variable.** The same loop runs over `r.atoms()`, which is
  `body.iter().chain(once(&head))`. So `rule.vars` is collected from body AND head, which is exactly
  the set `RulePattern.varList` gives the checker, and coverage holds by construction.
- **And the `expect` between them cannot fire.** `env[i].expect(..)` would panic on a rule variable
  the body never bound. `parse_rules` refuses any table whose head carries a variable that "does not occur
  in the body" before it reaches the evaluator, so that input never arrives.
  `a_head_variable_the_body_never_binds_never_reaches_the_evaluator` runs that refusal through both
  doors, `parse_rules` directly and `Reasoner::run_horn`.

That is the argument. `no_emitted_binding_is_one_the_format_now_refuses` is the check against the
bytes: it re-parses every table the engine wrote, re-derives the wanted variable set the way
`bindsCover` reads it, and asserts distinctness and coverage on every emitted step across three rule
tables and two graphs, with a floor so that a corpus that collapsed to nothing would fail rather
than pass. A structural argument nobody checks against the bytes is how a structural argument goes
stale.

## What this does not claim

**This decision does not make a conclusion writable RDF.** Binding `z` to the bare term `z` is now
required and still accepted, so a certificate can still conclude a triple no serialiser can write.
What changed is that the term must be WRITTEN by the certificate's author rather than minted by the
checker. Whether the format should additionally require every term to be well-formed N-Triples is a
separate question, it is not answered here, and `the_refusal_costs_no_certificate_anybody_meant`
exists partly so that the boundary is visible rather than assumed away.

It also does not add a theorem to `horn_certificate_sound`. That statement is unchanged, its proof
below the first line is unchanged, and the new conjunct is destructured and discarded. Adding a
check makes `checkHornStep = true` a STRONGER hypothesis, so the theorem cannot have been
weakened to accommodate it. The accounting runs the other way, and it is the accounting that
matters: a checker that refuses more is trivially easier to prove sound, so the evidence that this
one still accepts anything lives in `HornWitness.lean` and in the differential's floor of 20
mutually accepted certificates, not in the soundness theorem.

## What moved, and what deliberately did not

The **Lean moved**. The Isabelle was written from the specifications without reading it, and editing
it to agree would spend the only thing the second kernel is for. Where the two now differ is a new
finding and belongs in a report, not in a patch to whichever side is easier to change.

A property proved inside one formalisation is not a property of the format, and this is the case
that showed it. Isabelle's `coverage_implied` proves that removing its coverage check cannot change
its accept/reject bit. That is true of Isabelle, whose instantiation is partial, so a missing
binding kills the step anyway. It does not transfer: Lean's instantiation is total, so the same
check is load-bearing there and in the opposite direction.

## The numbers

Before, over 1,718 certificates (61 base, 1,291 mutated, 366 fuzzed, fuzz seed `0xd1ffed0001`):

    both accept:      349
    both reject:      857
    both exit 2:      465
    known divergence: 47
    unexplained:      0

After:

    both accept:      349
    both reject:      904
    both exit 2:      465
    divergent:        0
    malformed binding, refused by both (decision 0008): 286

The 47 moved into rejected-by-both and nowhere else: 857 plus 47 is 904, and the accepted and
unparseable counts did not move. The 286 is larger than 47 because most rows carrying a malformed
binding were already rejected by both kernels for some other reason; the gate runs on every row
rather than only on the ones that disagree, because a check that ran only on divergences could be
satisfied by silence. It splits 89 D1 and 197 D2.

### Re-measured on the deep corpus, 15 September 2026

The pair above is the SHALLOW corpus, which is what existed on the day this decision was written.
The corpus was deepened hours later, on a branch cut before this decision landed, and the two
pieces of work were merged separately: the pair that nobody had run was the deep corpus under the
fixed checker, and that is the pair the README stated a result for. Run in CI, which now installs
the second kernel:

Before, over 2,075 certificates (70 base, 1,585 mutated, 420 fuzzed, same seed):

    both accept:      402
    both reject:      1,080
    both exit 2:      539
    known divergence: 54

After:

    both accept:      402
    both reject:      1,134
    both exit 2:      539
    divergent:        0
    malformed binding, refused by both (decision 0008): 332

The shape repeats exactly: 1,080 plus 54 is 1,134, and the accepted and unparseable counts did not
move. Depth changed the size of every column and changed nothing about the decision.

One detail worth stating rather than rounding off: **all 47 corpus divergences were D1.** D2's
three certificates are committed probes with their own tests and are not members of the generated
corpus, so the corpus never counted them, and the before-run's classification prints only D1 rows.
That does not make D2 the weaker half of this decision. D2b is the case where the silent default
put a term nobody wrote into a conclusion, and D2c was found BY the fuzzer inside this same corpus
machinery before it was committed as a probe.

## Since written

Nothing yet.
