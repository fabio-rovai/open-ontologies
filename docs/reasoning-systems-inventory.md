# Reasoning systems: what this project uses, what it refuses, and why

Every external reasoning system that has been considered for this project, with its real status. The
point of the document is that a name appearing in a design discussion is not a capability, and a
reader deserves to know which is which without reading the commit log.

The organising principle is decision 0006. A model is a finite object and a verified checker can
validate one, so a satisfiability answer can be turned into a certificate. A refutation is a proof in
a calculus nobody has mechanised in core Lean here, so an unsatisfiability answer is testimony. That
single asymmetry decides how each system below is used, and it explains rankings that otherwise look
perverse.

Since 15 September 2026 the testimony comes with an exhibit. Both provers will print the derivation
they found, and `src/tstp.rs` reads it back: every leaf is matched against the problem this engine
emitted, the DAG is checked for dangling parents and cycles, and the resolution-family steps are
recomputed. That does not upgrade the testimony into a certificate and no word in the output says it
does, because the calculus is not mechanised and the replayer is unverified Rust. What it removes is
the part of the trust that was never about the prover's soundness at all: whether it was answering
about the file we gave it. The addendum to decision 0005 draws the new line.

## Status at a glance

| System | Kind | Status here |
| --- | --- | --- |
| Lean 4 | Proof assistant | The kernel. Every checker in this repository. |
| E 3.2.5 | First-order prover | Differential oracle. Never an authority. Its derivations are now read back and structurally checked. |
| Vampire 5.1.0 | First-order prover | Same role as E, and the one whose derivations replay furthest. |
| Z3 4.16.0 | SMT solver | Model-certificate layer, under construction. |
| Mace4 | Finite model finder | Same. Its output is checkable; Prover9's is not. |
| Prover9 | First-order prover | Declined. Unmaintained since 2011, refutations uncheckable. |
| cvc5 | SMT solver | Not installed. Same role as Z3 when it is. |
| Isabelle/HOL | Proof assistant | Built, as an independent second FORMALISATION. It disagreed with the Lean. Below. |
| Dedukti, Lambdapi | Logical framework | Declined twice, for portability and for re-checking. Reasons below. |
| lean4export, nanoda | Lean export and external checker | Investigated, RUN, not adopted. `docs/independent-rechecking.md`. |
| leanchecker | Ships in the Lean toolchain | Passes here. Not in CI yet. Not an external verifier. |
| Duper, lean-smt | Lean automation | Declined. Both require Mathlib. |
| Aeneas with Charon | Rust to Lean | Declined. Subset does not contain this codebase. |
| Verus, Creusot, Prusti | Rust verification | Declined. Each needs the code rewritten in its subset. |
| Iris, RefinedRust | Concurrent separation logic | Declined. The shared state is inside Oxigraph, and the certificate layer is pure. Below. |
| Dafny 4.11.0 | Verification-aware language | Declined as a dependency, RUN and kept as a specification. It verifies a rewrite, not this code. Below. |
| Kani | Rust bounded model checker | Being applied to the trusted boundary only. |
| loom | Rust interleaving explorer | The right tool for the two latent lock defects. Not yet wired in. |
| TPTP and TSTP | Interchange | Implemented, in both directions. TPTP out, TSTP back in and checked. |
| CLIF, ISO/IEC 24707 | Interchange | Implemented. The conformance format. |
| SMT-LIB 2 | Interchange | Under construction. |
| RIF Core, SWRL | Rule languages | Front ends under construction. |
| CertifyingDatalog | Prior art | Not a dependency. The Horn layer is our analogue. |
| Hets | Heterogeneous tool set | Declined as a dependency. Its institution and comorphism core is reimplemented small and machine-checked here. Below. |
| DOL | Specification language | Parsing deliberately deferred. Decision 0009 says why. |

Anything marked under construction is on an unmerged branch and is not a capability yet. This
document will be wrong the moment that changes, so treat the branch state as authoritative.

## The provers, and why the strongest one is still only an oracle

Vampire and E are the live first-order provers. Vampire has dominated the relevant competition
divisions for two decades and E is the more embeddable of the two. Both read TPTP, which is why TPTP
is our execution format regardless of what else we print.

Neither can give us a certificate. Replaying a superposition refutation needs a verified calculus with
unification, term orderings and redundancy criteria, and none exists in core Lean. So both are used the
way pyshacl is used for validation: as a differential oracle whose disagreement with our engine means
one of the two is wrong, and whose agreement means nothing has been proved. On its first real run
against one public vocabulary the comparison disagreed with the engine on fifty-eight of one hundred and
eighty-one claimed entailments, and every one was a real defect, including one in the exporter itself.
That is the value, and it does not require trusting the prover at all.

What both now also give us is a derivation to read. `fol-prove` and `onto_fol_prove` run the prover
with its proof-printing option, parse the TSTP it emits, and report a verdict on the derivation
separately from the verdict the prover reported on the problem. Measured over FOAF, one problem per
claimed entailment: Vampire refuted all one hundred and eighty-one and one thousand two hundred and
seventy-nine of its three thousand three hundred and fourteen steps were recomputed here, while E
refuted all one hundred and eighty-one and only one hundred and eighty-one of its four thousand six
hundred and forty-four steps were. Every one of the five hundred and thirty-four leaves matched the
problem on both sides, and nothing was rejected.

The gap between the two is structural rather than a matter of quality, and it is the most useful
thing this measurement produced. Vampire prints each inference as its own annotated formula with the
conclusion attached, so a step can be recomputed from its premises. E nests inference records inside
parent positions, and a nested record carries a rule and parents but no formula, so neither it nor
the step it feeds can be replayed at all. Anyone choosing a prover for a pipeline that wants to
inspect its own evidence should know that before choosing.

Prover9 is declined. Its author died in 2011, it has had no maintainer since, and its refutations are
no more checkable than Vampire's. There is no version of the argument where an unmaintained prover
beats a maintained one at the same job.

Mace4, from the same distribution, is a different matter entirely and is kept. It is a finite model
finder, its output is a finite structure, and a verified evaluator can check it. Half of a dead
toolchain is alive here because of what kind of evidence it produces, not how well maintained it is.

## SAT and SMT

Z3 is the fifth reasoning family and the last one to be built. The interesting part is not the SMT-LIB
printer, which is a third rendering of a representation we already have, but that Z3 returns models.
A model that our verified checker accepts converts a solver's opinion into a proof of satisfiability,
which is the only place in this architecture where an external tool's answer is upgraded rather than
merely corroborated.

Its unsat answers stay oracle answers. An unsat core is not a proof object we can replay, and the
proof logs Z3 can emit would need the same mechanised calculus that the first-order case lacks. The
structural treatment the first-order provers now get has no counterpart here yet: nothing reads Z3's
proof logs, and doing so is a separate piece of work from reading TSTP.

## Isabelle, and what the second kernel found

This section said "declined for now" until 14 September 2026, and it was stale for several hours
after the work had merged. That is recorded here rather than silently overwritten, because a
document that says a thing was not built when it was is the same defect as one that claims a
proof it does not have.

Isabelle/HOL is not used the way Sledgehammer uses it, reconstructing external provers' answers
through a kernel. It is used for something narrower and, for this project, more valuable: an
INDEPENDENT SECOND FORMALISATION of the Horn certificate checker, written from the W3C
specifications and the fixture data with the Lean deliberately unread, so that a definitional
mistake shared by nobody could be caught. A machine-checked proof rules out a bad argument and
does nothing about a bad definition, and a bad definition is invisible from inside a single
formalisation by construction. The formalisation lives in `isabelle/`, builds with no `sorry`
and no `oops`, and its checker is exported to executable code so it can be run over the same
bytes the Lean reads.

Then both verified checkers were run over 1,718 certificates. They disagreed on 47, all in the
same direction, with none unexplained, and the cause was neither proof. Isabelle validates the
binding list as a data structure; Lean turned any binding list into a total function with a
silent default. On a certificate that binds one variable twice, first to a value that makes the
step check and then to one that does not, Lean returned the absolute verdict and Isabelle
refused. The defect was in the FORMAT, which never said what a repeated key means or what an
incomplete binding means, and only a second implementation could have surfaced it.

That gap is now CLOSED. `docs/decisions/0008-a-binding-is-data-and-evidence-admits-one-reading.md`
refuses both shapes, the Lean moved and no Isabelle theory was touched, and the 47 moved into
rejected-by-both and nowhere else. The per-bucket counts that decision was measured against are
in the decision record, dated, and describe the corpus as it was that day.

Those 1,718 certificates were one step deep. Sixty of the 61 base certificates contained no step
citing an earlier step's conclusion, so on those the checkers' ordering logic had nothing to
decide, and strict prefix visibility — a step may cite only what came strictly before it and never
itself, which is the property both inductions rest on — was differentially exercised by a single
two-step fixture.
On 15 September 2026 the corpus gained generated chains and fans, six mutations a flat
certificate cannot express, and two hand-built adversarial certificates. It is now 2,075 rows
reaching depth 19 and fan-out 12, of which 484 exercise the ordering discipline rather than 123.
Depth found no new divergence, which is a result about the two formalisations and not a null
one — the property their inductions are built on was, until then, barely tested against data
that could violate it.

**Read the numbers in the two paragraphs above as history, because for a day they were read as a
description.** The 47-of-1,718 and the zero that replaced it were both measured on the shallow
corpus. A later count of 54 disagreements over the deep one was measured on a branch cut before
the binding fix, and is not repeated here for that reason. The two pieces of work were authored
eleven minutes apart and merged separately, so the combination nobody had run was the deep corpus
under the FIXED checker, which is exactly the combination the README asserted a result for. It
has now been run, on this tree and in CI, and the two kernels return the same answer on every
one of the 2,075 rows. The 54 moved into rejected-by-both and nowhere else, the accepted and
unparseable counts did not move at all, and the per-bucket figures are in
[decision 0008](decisions/0008-a-binding-is-data-and-evidence-admits-one-reading.md), dated, next
to the shallow pair they repeat the shape of.

The reason none of this was caught is worth more than the numbers. The test that requires zero
divergence needs Poly/ML as well as lake, no workflow installed Poly/ML, so it skipped in the one
job that ran it and was invoked by no job that could have made it strict. A skipped test reports
`ok`. [docs/ci-gates.md](ci-gates.md) now states which gates fire and is itself held to the
workflows by a test.

The adversarial review of that comparison established four things about the Lean layer that the
Lean's own build could never have shown, and they are the current work rather than a footnote:
eight of the twenty-seven rules were assumed as primitive semantic conditions rather than
derived, so the machine-checked content for those arms was close to nothing, and the Isabelle
derives them; the Lean's model class is larger than the specification's, so non-entailment does
not transfer outward to a conforming interpretation; the two checkers print the same verdict word
for theorems over model classes nobody has ordered; and both sides' non-vacuity witnesses were
vacuous exactly where their conditions are strongest. Each of those is a case of the layer
claiming more than it had earned, which by this project's standards is the same category as a
soundness bug.

Three of those four are now closed and the fourth is narrower than it was. The eight arms are
derived on both sides. Both non-vacuity witnesses are rebuilt and both carry a gate that fails if
they are hollowed out later: `live_exercises_every_arm` and `live_fires_every_field` on the Lean
side, `M4_exercises_every_derivation` and `M4_every_condition_has_a_live_antecedent` on the
Isabelle. Every non-entailment in the Lean is now stated over the class carrying the quoted table
cells as well, four of them on finite structures built for the purpose. What is NOT closed is the
ordering of the two kernels' model classes, which nobody has established in either direction, so
"both said entailed" still means less than it looks.

What remains declined is the wider use. Reconstructing Vampire or E refutations through
Isabelle's kernel would give this project kernel-checked refutations, which it lacks, at the cost
of a second proof assistant in the trust surface for every user rather than for a cross-check.
That trade is still not being made, and the reasoning has not changed.

One thing this bought and one thing it did not, because the two get confused. It bought independence
of the SPEC READING: two definitions written from one standard, disagreeing where the standard was
silent. It did NOT buy independence of the KERNEL. The Lean proofs are still checked by exactly one
program, and the Isabelle proofs by exactly one other, and neither re-checks the other's proofs
because they are proofs of different theorems about different definitions. Nothing anywhere
re-checks a Lean proof.
[docs/independent-rechecking.md](independent-rechecking.md) is the investigation of what that would
take, including an export of this repository's own Lean and a run through an independent Rust
checker.

## Dedukti, and why not

Dedukti is a logical framework designed so that proofs from different systems can be expressed in one
language and rechecked. The motivation is real and it is close to this project's own.

It is declined because it solves a problem we do not have. Dedukti pays when you hold large proof
libraries in several systems and want them to talk. We hold small certificates in one format, checked
by one small kernel, and adding a framework layer would enlarge the trusted base to buy portability
nobody has asked for. If a second consumer of our certificates ever appears, this is the first thing
to revisit.

That paragraph answers the PORTABILITY question and it still stands. It does not answer the
RE-CHECKING question, which is whether a Lean proof could be checked again in a different
foundation, and that was investigated separately on 15 September 2026 with a different and firmer
answer: no, not today, for reasons that are structural rather than a matter of polish. The only
Lean-to-Dedukti translator pins Lean v4.18.0-rc1 against the v4.33.1 pinned here, needs a personal
fork of Dedukti to terminate, and stubs out the constants it cannot handle, which converts the hard
cases into postulates and would hand back a green result with nothing behind it. The onward step
into Rocq or Agda does not exist: Lambdapi's exports run before elaboration and discard rewrite
rules, and a Dedukti encoding of Lean's type theory IS a rewrite system. Evidence, versions and the
authors' own statements are in
[docs/independent-rechecking.md](independent-rechecking.md), which also records what DOES work,
which is `lean4export` plus an independent checker written in Rust.

## Lean automation we do not use

Duper is a superposition prover written inside Lean that produces kernel-checked proof terms, and
lean-smt reconstructs cvc5 proofs the same way. Either would give us precisely the missing capability,
kernel-checked refutations, without leaving Lean.

Both depend on Mathlib. The axiom footprint of every result here is pinned to `propext`,
`Classical.choice` and `Quot.sound`, the build takes no external package, and the argument that anyone
can read the whole checker rests on that. Taking Mathlib to gain refutation checking is a real trade
and might one day be the right one. It is not being made silently.

## Putting the Rust in Lean

The demand that all the code be verified in Lean cannot be met literally. There are roughly fifty
thousand lines of Rust here. Aeneas with Charon translates Rust into Lean for a restricted subset that
this codebase is not inside, and the same is true of Verus, Creusot and Prusti, each of which verifies
Rust written in its own dialect against SMT.

The demand has a real core and that core is small. The Lean theorems are conditional: they say that IF
the asserted graph is what the certificate says and IF the derivation steps are the ones the engine
took, THEN the conclusions follow. Everything to the left of that is the trusted computing base, and
it is a serialiser, a parser and an interner rather than fifty thousand lines. That boundary is what
is being property-tested and, where it pays, model-checked with Kani. What remains trusted after that
work will be named explicitly rather than left for a reader to infer.

Dafny belongs in this section rather than beside Verus, and the difference is worth stating because
it makes the objection stronger and not weaker. Verus, Creusot and Prusti verify Rust written in a
dialect, so the artefact that ships is the artefact that was verified. Dafny does not verify Rust at
all. It is a separate language that compiles to one, so the only way a Dafny theorem becomes a
statement about this engine is if the engine runs Dafny's generated Rust.

It was installed and run rather than argued about. `dafny/RuleTable.dfy` models the rule-table
grammar of `src/reason.rs` over bytes and proves TCB-20's round trip with no bound on field length,
field count or body size: 57 obligations, 0 errors, seven seconds. That is a real capability gap
closed. Kani proves only the per-field half and only at field lengths of exactly two and three,
because a symbolic length exhausted CBMC, and Aeneas cannot reach the function at all because `Pat`
carries a `String`. Writing the property out also found that TCB-20 was stated on
`docs/trusted-computing-base.md` without the hypothesis it needs, and is false without it.

It is still declined, for a reason that was measured rather than predicted. Compiling the model
emits 872 lines of Rust against a `dafny_runtime` of 8,590 lines pulling `num`, `once_cell` and
`itertools`, and its entry point takes a `Sequence<u8>` and returns an `Rc<Option<Rc<Pat>>>`, so
every call from `src/reason.rs` would cross a hand-written, unverified marshalling layer sitting at
the precise boundary the work exists to shrink. Verifying about 127 lines by adding roughly 8,600
unverified ones to the trusted base is the wrong direction. The file is kept as a specification and
as the evidence, it is a gate nowhere, and
[decision 0014](decisions/0014-a-verifier-that-cannot-read-the-code-verifies-a-rewrite.md) carries
the numbers and the terms.

## Iris, and the concurrency that is not ours

Iris is the higher-order concurrent separation logic built in Rocq, and the obvious reason to want it
here is that the Rust side is plainly not pure: a store behind an `Arc`, fourteen process-global
atomics in `src/runtime.rs`, three rayon fan-outs, and an MCP server that spawns every request as its
own tokio task. That last fact is worth knowing on its own, since it means tool calls run in parallel
in stdio mode and not only over HTTP.

It is declined, and the argument is measured rather than asserted.
[docs/concurrency-inventory.md](concurrency-inventory.md) names every piece of shared mutable state in
the engine with a file and a line, and
[decision 0012](decisions/0012-concurrency-lives-below-the-certificate.md) is the ruling.

The short version has three parts. The one genuinely shared mutable object is
`oxigraph::store::Store`, which synchronises itself inside a dependency backed by RocksDB, so there is
no ordering property in our code to prove and no way for a Rocq proof to reach the one that matters.
The concurrency we do write has nothing to establish: all fourteen atomics are independent scalars
read relaxed, the rayon phases are read-only over frozen data with one monotone latch, and `Arc`,
`Mutex` and `RwLock` were already verified in Iris by RustBelt, so we consume that theorem by using
`std` rather than by re-proving it. And the subset objection that declined Aeneas, Verus, Creusot and
Prusti in the section above applies harder to RefinedRust, whose subset is narrower still.

The decisive evidence is empirical. The inventory went looking for a property worth proving and came
back with three defects instead, none of which is a race: an evictor wired to a registry nothing loads
into, so it can never fire; a missing critical section in `load_file`; and a missing read lease around
long-running tools. `loom` and a two-thread test find the second and third. The first needs no
concurrency tooling at all and is pinned by `tests/registry_evictor_wiring_test.rs`.

## Rule languages

The Horn certificate layer is generic over rule tables, with one soundness theorem covering every
table at once, which is what makes RIF Core, SWRL and Datalog a single problem rather than three. Until
the front ends land, that coverage is architectural rather than actual, because nothing produces a
rule table from a standard rule syntax.

The verdict rule from decision 0003 governs all of it and is the reason this is safe to build. A
certificate over the built-in table earns `entailed`. A certificate over a table a user supplied,
whatever syntax it arrived in, earns `entailed_under_supplied_rules`, because those rules are
assumptions the certificate carries and never facts it establishes.

CertifyingDatalog, presented at ITP 2025, is the closest published prior art and is not a dependency.
It certifies Datalog derivations in Lean; our Horn layer is the analogue reached independently, and the
comparison is worth making in any write-up rather than avoided.

## Hets, institutions, and the part that was worth taking

The Heterogeneous Tool Set is the engineering that goes with Goguen and Burstall's institutions. Its
value is breadth: dozens of logics, and translations between them, in one program. Its comorphisms
are Haskell type-class instances supplying a signature map, a sentence map and a model map, and the
satisfaction condition that makes those three a translation of logics is discharged in the
literature rather than by the program. Decision 0005 already said as much, "Hets proves its
OWL-to-CASL comorphism on paper", and nobody here has read the Hets source, so that is a statement
about its published design and not a code review.

It is declined as a dependency for the obvious reasons and one that matters more. The obvious ones:
it is a Haskell program, nothing in this CI builds Haskell, and running it would add a large
unverified tool to the trust surface. The one that matters: what it would add is exactly the thing
this project does not accept on testimony. An unchecked satisfaction condition is the same category
of claim as an unchecked refutation, and decision 0005 already rules on that category.

What was taken is the idea, which is public and is the only part that carries weight.
`lean/OOCert/Institution.lean` and `lean/OOCert/Comorphism.lean` state an institution and a
comorphism in core Lean with the satisfaction condition as a FIELD, so neither can be constructed
without a proof of it, and `lean/OOCert/InstitutionRdf.lean` and `lean/OOCert/InstitutionFol.lean`
discharge it for three institutions and two comorphisms built out of model classes this repository
already had. That is three logics against Hets's dozens, and it is three logics whose translations
are machine-checked rather than argued. The trade is breadth for evidence and it is the same trade
this project makes everywhere else.

Reflection is kept apart from preservation in the types, because the two are different theorems and
only the first is free. `lean/OOCert/InstitutionWitness.lean` exhibits a comorphism between two of
these institutions that preserves entailment and provably does not reflect it, so the separation is
measured here rather than warned about.

DOL, the language for saying "interpret this specification into that one along this comorphism", is
not parsed and will not be until there is more than one proved comorphism for it to distribute over.
Building the notation before the table it indexes would be a front end onto two rows.

## Interchange formats

TPTP is the execution format, because it is the only one an installed solver will read, and TSTP is
now read in the other direction: `src/tstp.rs` parses a prover's annotated-formula list, its
`inference(RULE, [status], [parents])` records and its `file(…)` and `introduced(…)` provenance, and
turns them into a DAG this engine can check against its own output. CLIF is the
conformance format, because ISO/IEC 21838-1 requires a top-level ontology to carry an axiomatisation in
a language conforming to ISO/IEC 24707 and the Basic Formal Ontology discharges that in CLIF. SMT-LIB
is the model-finding format. All three are printers over one representation, and the correspondence
between that representation and the Lean translation is pinned by hand-computed tests and is not itself
proved. That last sentence is the honest trust boundary of the export layer and should never be dropped
when the layer is described.
