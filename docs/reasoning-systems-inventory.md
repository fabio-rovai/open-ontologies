# Reasoning systems: what this project uses, what it refuses, and why

Every external reasoning system that has been considered for this project, with its real status. The
point of the document is that a name appearing in a design discussion is not a capability, and a
reader deserves to know which is which without reading the commit log.

The organising principle is decision 0006. A model is a finite object and a verified checker can
validate one, so a satisfiability answer can be turned into a certificate. A refutation is a proof in
a calculus nobody has mechanised in core Lean here, so an unsatisfiability answer is testimony. That
single asymmetry decides how each system below is used, and it explains rankings that otherwise look
perverse.

## Status at a glance

| System | Kind | Status here |
| --- | --- | --- |
| Lean 4 | Proof assistant | The kernel. Every checker in this repository. |
| E 3.2.5 | First-order prover | Used as a differential oracle. Never as an authority. |
| Vampire 5.1.0 | First-order prover | Installed, same role as E. |
| Z3 4.16.0 | SMT solver | Model-certificate layer, under construction. |
| Mace4 | Finite model finder | Same. Its output is checkable; Prover9's is not. |
| Prover9 | First-order prover | Declined. Unmaintained since 2011, refutations uncheckable. |
| cvc5 | SMT solver | Not installed. Same role as Z3 when it is. |
| Isabelle/HOL | Proof assistant | Built, as an independent second kernel. It disagreed with the Lean. Below. |
| Dedukti | Logical framework | Declined. Reasons below. |
| Duper, lean-smt | Lean automation | Declined. Both require Mathlib. |
| Aeneas with Charon | Rust to Lean | Declined. Subset does not contain this codebase. |
| Verus, Creusot, Prusti | Rust verification | Declined. Each needs the code rewritten in its subset. |
| Kani | Rust bounded model checker | Being applied to the trusted boundary only. |
| TPTP and TSTP | Interchange | Implemented. The execution format. |
| CLIF, ISO/IEC 24707 | Interchange | Implemented. The conformance format. |
| SMT-LIB 2 | Interchange | Under construction. |
| RIF Core, SWRL | Rule languages | Front ends under construction. |
| CertifyingDatalog | Prior art | Not a dependency. The Horn layer is our analogue. |

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
proof logs Z3 can emit would need the same mechanised calculus that the first-order case lacks.

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
reaching depth 19 and fan-out 12, of which exercise the ordering discipline rather than 123.
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

## Dedukti, and why not

Dedukti is a logical framework designed so that proofs from different systems can be expressed in one
language and rechecked. The motivation is real and it is close to this project's own.

It is declined because it solves a problem we do not have. Dedukti pays when you hold large proof
libraries in several systems and want them to talk. We hold small certificates in one format, checked
by one small kernel, and adding a framework layer would enlarge the trusted base to buy portability
nobody has asked for. If a second consumer of our certificates ever appears, this is the first thing
to revisit.

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

## Interchange formats

TPTP is the execution format, because it is the only one an installed solver will read. CLIF is the
conformance format, because ISO/IEC 21838-1 requires a top-level ontology to carry an axiomatisation in
a language conforming to ISO/IEC 24707 and the Basic Formal Ontology discharges that in CLIF. SMT-LIB
is the model-finding format. All three are printers over one representation, and the correspondence
between that representation and the Lean translation is pinned by hand-computed tests and is not itself
proved. That last sentence is the honest trust boundary of the export layer and should never be dropped
when the layer is described.
