# 0015 · A blank line is a line, or it is not, and the format never said which

- **Status**: OPEN. Reported, not closed. Nothing on either side has been changed to make
  the disagreement go away, and nothing should be until somebody decides what the format
  means · found by `rocq/`, a third independent formalisation, run beside `lean/` over
  1,593 rows by `tests/rocq_kernel_differential_test.rs` · a second finding in the same
  run, a resource defect in the NEW checker, IS closed, below
- **Written**: 2026-09-18
- **Related**: decision 0008 (a binding is data), which is the same shape of defect found
  the same way by the second kernel; decision 0003 (a rule is data)

## The problem

`rocq/` is a third formalisation of the Horn certificate checker, written from the
specifications, the decision records and the fixture bytes. Run beside the Lean checker
over a corpus of 1,593 rows built from `tests/fixtures/horn/` and systematic mutations of
it, the two agreed on 1,269 and disagreed on 324, always in the same direction, with one
cause and nothing unexplained.

**Lean skips an empty line. Rocq refuses one.**

It holds in all three input files. A rules table, an asserted graph or a certificate that
carries an empty line anywhere, at the top, in the middle, or as a second newline at the
end, is read by Lean as if the line were not there, and is refused by Rocq with exit 2.
Every one of the 324 divergent rows is that, and every row in the corpus that carries an
empty line at all is divergent: 108 in the rules table, 108 in the asserted graph, 108 in
the certificate.

Neither checker is unsound, and saying so is not a softening. An empty line is not a
step, not a triple and not a rule. Skipping it derives nothing; refusing it asserts
nothing. Both soundness theorems are untouched by this and would be untouched by either
answer. **The defect is in the FORMAT.** Nothing in `docs/lean-certificates.md` or in
decision 0003 says whether an empty line is a line, so two readings of the same bytes were
both available and a certificate's readability depends on which verified checker reads it.

## The sharp part

The obvious reaction is that tolerance is harmless and Rocq is being difficult. There is a
fact that makes that reaction worse than wrong, and it was found by the same corpus.

**Lean's tolerance is for the EMPTY string exactly.** A line holding one space is refused.
A line holding one tab is refused. A line holding one carriage return is refused. Both
checkers agree on all three.

So the leniency does not cover the case that motivates leniency. The reason a parser
tolerates blank lines is that files acquire them: an editor adds one at the end, a shell
redirection adds one, a copy and paste adds one. The single most likely way for this
repository's files to acquire a blank line is a checkout with CRLF endings, and in such a
file a blank line is `\r`, which Lean refuses. `.gitattributes` pins LF on the certificate
formats, which is the right fix and is also the reason the tolerance has never been needed:
it is a rule that fires only on the inputs it was not written for.

## What is NOT being decided here

Which side should move. That is a question about the format and it belongs to whoever owns
the format, not to whoever wrote the third checker. Both answers are defensible:

- **Refuse, as Rocq does.** A parser that repairs its input decides what the input meant.
  Refusal is the reading with no convention in it, and it is what both checkers already do
  for a line of one space.
- **Skip, as Lean does.** Then say so in `docs/lean-certificates.md`, and make it uniform:
  skip a line that is empty after stripping trailing whitespace, so that the CRLF case the
  rule exists for is actually covered.

What must not happen is the third formalisation being edited to agree with the first.
`rocq/` was written from the specifications for exactly one purpose, and an independent
reading edited to match the thing it is reading is worth nothing. The same sentence appears
in `isabelle/README.md` and it is the same sentence for the same reason.

## The other finding, which IS closed, and which was in the new checker

The same run found a defect in `rocq/` rather than in the format, and it is the more
serious of the two.

A certificate citing rule `99999999999999999999` made the Rocq checker die of a
`Stack_overflow`. Rocq's `nat` is unary, the parser built the number before it looked at
it, and a twenty digit index is not a number a machine can build that way. That is a denial
of service, and it is not the worst of it: **OCaml exits an uncaught exception with status
2, which is this tool's code for a parse error.** The crash reported a verdict. Nothing in
the output said the checker had failed; it said the file could not be read.

The only reason anybody looked is that Lean answered 1 where Rocq answered 2, and a
one-digit difference in an exit code is what a differential is for.

Both halves are closed.

1. **The cause.** Count fields are parsed into binary naturals and nothing recurses on them
   (`R-PARSE-8` in `rocq/theories/Parse.v`, `R-STEP-3` in `rocq/theories/Syntax.v`,
   `R-CHK-6` in `rocq/theories/Checker.v`). The index lookup walks the rule table and
   decrements the index in binary, so an out of range index costs the length of the table
   whatever its magnitude. The answer is now a rejection, in four milliseconds, and it
   agrees with Lean.
2. **The disguise.** `rocq/driver/main.ml` catches every exception and exits 70,
   `EX_SOFTWARE`, which nothing else in the tool uses. A crash can no longer be mistaken
   for a verdict or for a parse error, and
   `tests/rocq_kernel_differential_test.rs` treats any exit code that is not 0, 1 or 2 as a
   failure of the whole comparison rather than as a row.

`a_crash_is_not_a_verdict` is the regression, and it pins the answer and the time it takes.

## The numbers

Over 1,593 rows: 27 base triples, being three rule tables crossed with nine shipped
(asserted graph, certificate) pairs, each mutated by fourteen byte level edits applied to
each of the three files and sixteen structural edits applied to the certificate's first
step.

    both accept:            82
    both reject:            809
    both unparseable:       378
    divergent, explained:   324
        108  the rules table contains an empty line
        108  the asserted graph contains an empty line
        108  the certificate contains an empty line
    divergent, unexplained: 0
    checker crashes:        0

`cargo test --test rocq_kernel_differential_test -- --nocapture` prints these and is the
only place they should be read from. The figures above are a snapshot of one run and will
be stale the moment the corpus changes, which is what happened three times to the numbers
decision 0008 carried.

The floor matters more than any of those columns. Agreement is cheap when one side refuses
everything, so the test requires at least twenty rows ACCEPTED BY BOTH, and it also
requires the quarantined divergence to still reproduce: a run in which the empty-line rows
stopped diverging means a checker changed or the generator broke, and either way somebody
should look rather than enjoy a green tick.

## Since written

Nothing yet.
