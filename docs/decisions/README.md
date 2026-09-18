# The rules, and what they have caught

One file per rule. Each names the failure it exists to prevent, because a rule whose cost is visible
and whose benefit is not gets dropped the first time it is inconvenient.

| | Rule | The failure it prevents |
| --- | --- | --- |
| [0001](0001-an-inference-is-not-an-assertion.md) | An inference is not an assertion | A materialised conclusion becoming the next run's premise, so the engine cites itself |
| [0002](0002-an-inference-carries-a-certificate.md) | An inference carries a certificate, and the certificate has a proof | Trusting the engine's report of its own work |
| [0003](0003-a-rule-is-data-and-an-assumption-is-not-a-fact.md) | A rule is data, and an assumption is not a fact | A user-supplied rule table earning the word reserved for the checked one |
| [0005](0005-a-prover-is-an-oracle-and-a-translation-is-a-theorem.md) | A prover is an oracle, and a translation is a theorem | Reading a theorem prover's confident answer as evidence |
| [0006](0006-a-model-is-a-certificate-and-a-refutation-is-not.md) | A model is a certificate, and a refutation is not | Treating an exhausted bounded search as unsatisfiability |
| [0007](0007-a-slice-preserves-a-conclusion-or-it-does-not.md) | A slice preserves a conclusion, or it does not | A retrieval slice quietly dropping the conclusion it was asked about |
| [0008](0008-a-binding-is-data-and-evidence-admits-one-reading.md) | A binding is data, and evidence admits one reading | A certificate format admitting two readings, so two checkers disagree |
| [0012](0012-concurrency-lives-below-the-certificate.md) | Concurrency lives below the certificate, and a proof cannot follow it there | Adopting a program logic for a hazard that lives in a dependency, and calling the result assurance |
| [0014](0014-a-verifier-that-cannot-read-the-code-verifies-a-rewrite.md) | A verifier that cannot read the code verifies a rewrite | Counting a green verifier as evidence about code it never read |

There is no 0004. The decision now numbered 0006 was drafted as 0004 on a branch that never merged,
so the number never reached `main`. It is left as a hole rather than reused, because a reused number
makes an old citation silently come to mean something else.

This table has rows for 0009 to 0011 missing rather than absent. Those records exist in this
directory, there are two numbered 0009, and nobody has written their rows. The gap is recorded here
so that a reader does not conclude from the jump that the numbers were skipped the way 0004 was.

## What this discipline has caught

In one week of running it against this engine.

**Five description-logic false cleans.** Each an inconsistent ontology reported consistent, with full
confidence.

**A rule that could conclude a triple no serialiser can write**, reachable from ordinary OWL, which
left the store non-deterministic: three runs of one input kept 40, 9 and 24 inferences.

**Two independently verified kernels disagreeing on the same certificates**, always in the safe
direction, tracing to a gap in the format that neither proof could see. It did not say what a
repeated binding key meant, so one kernel refused the shape and the other answered from whatever its
lookup happened to do. The sharper fact came out of the second implementation rather than the first:
a key bound twice to different values is satisfied by no substitution at all, so there were never two
readings, only two ways of discarding half the certificate. Closed by
[decision 0008](0008-a-binding-is-data-and-evidence-admits-one-reading.md), and the two kernels now
return the same answer on every row of a corpus of 2,075 certificates, 484 of which exercise the
ordering property both inductions rest on, against 123 before that corpus was deepened.

That agreement was, until 15 September 2026, checked by nothing. No workflow installed the second
kernel, so the test requiring zero divergence skipped, and a skipped test reports `ok`. CI runs both
kernels over the whole corpus on every pull request now, and [docs/ci-gates.md](../ci-gates.md) is
the table of which other gates do and do not fire.

Every one of those had passed every test that existed before.

A single formalisation, however careful, cannot see a defect in the format it formalises. That is the
argument for the second kernel, and it is the only argument for it that survived contact with the
work: two proof assistants agreeing does not mean what a casual reader assumes, and the value came
from independence of the specification reading rather than from quantity.
