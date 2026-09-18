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
| [0009](0009-a-translation-between-logics-carries-its-satisfaction-condition.md) | A translation between logics carries its satisfaction condition | Moving a sentence between logics and assuming the truth came with it |
| [0010](0010-the-input-is-a-value-and-not-a-store.md) | The input is a value and not a store | Certifying a run against a store that can change under it |
| [0011](0011-a-module-carries-a-theorem-and-a-slice-carries-a-measurement.md) | A module carries a theorem, a slice carries a measurement | Reading a measured slice as though it carried the theorem a module does |
| [0012](0012-concurrency-lives-below-the-certificate.md) | Concurrency lives below the certificate, and a proof cannot follow it there | Adopting a program logic for a hazard that lives in a dependency, and calling the result assurance |
| [0014](0014-a-verifier-that-cannot-read-the-code-verifies-a-rewrite.md) | A verifier that cannot read the code verifies a rewrite | Counting a green verifier as evidence about code it never read |
| [0015](0015-a-blank-line-is-a-line-or-it-is-not.md) | A blank line is a line, or it is not, and the format never said which | The same failure again, found by a third kernel and still OPEN: two checkers reading one file differently because nobody wrote down what an empty line is |
| [0016](0016-a-conclusion-names-the-axioms-responsible-for-it.md) | A conclusion names the axioms responsible for it, and minimality is re-run | Presenting a justification as minimal when nothing re-ran it |

There is no 0004. The decision now numbered 0006 was drafted as 0004 on a branch that never merged,
so the number never reached `main`. It is left as a hole rather than reused, because a reused number
makes an old citation silently come to mean something else.

Every record in this directory has a row above, and `tests/decision_numbers_test.rs` fails if that
stops being true, if two records take one number, or if a row points at a file that does not exist.

There used to be two records numbered 0009, written in parallel on branches that could not see each
other. Git does not catch that: the filenames differ, so both merge cleanly and the index gets a
plausible row from each. The later of the two, on justification, is now 0016. It moved rather than
the earlier one because a citation should keep meaning what it meant, and the earlier record had
been on `main` longer. The same collision then happened three more times in one day between
parallel branches, which is why the check exists rather than a note asking people to be careful.

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

**A third kernel, and a second format question nobody had written down.** `rocq/` is an independent
formalisation in Rocq 9.2, written from the specifications with `lean/OOCert/` and `isabelle/` unread.
Run beside the Lean checker over 1,593 rows it agreed on 1,269 and disagreed on 324, one cause,
nothing unexplained: Lean skips an empty line in all three input files and Rocq refuses one. Neither
is unsound, because an empty line is not a step, not a triple and not a rule. It is the same category
of defect as decision 0008's and it is
[decision 0015](0015-a-blank-line-is-a-line-or-it-is-not.md), which is OPEN rather than closed,
because which side should move is a question about the format and not about whoever wrote the third
checker.

The same run found something sharper in the new checker than in the format. A certificate citing rule
`99999999999999999999` made it die of a stack overflow, and OCaml exits an uncaught exception with
status 2, which is that tool's code for a parse error, so **the crash was reporting a verdict**. The
only reason anybody looked is that one checker answered 1 where the other answered 2. That is the
whole argument for differentials in one sentence, and it is worth more than the divergence it was
looking for.

That agreement was, until 15 September 2026, checked by nothing. No workflow installed the second
kernel, so the test requiring zero divergence skipped, and a skipped test reports `ok`. CI runs both
kernels over the whole corpus on every pull request now, and [docs/ci-gates.md](../ci-gates.md) is
the table of which other gates do and do not fire.

Every one of those had passed every test that existed before.

A single formalisation, however careful, cannot see a defect in the format it formalises. That is the
argument for the second kernel, and it is the only argument for it that survived contact with the
work: two proof assistants agreeing does not mean what a casual reader assumes, and the value came
from independence of the specification reading rather than from quantity.
