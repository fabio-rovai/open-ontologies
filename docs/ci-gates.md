# Which gates actually run in CI

A test that skips reports `ok`. A job full of them reports success. This
repository exists to attack exactly that shape of claim, and it had the shape in
its own suite: the CI `python` job installed one extra and nothing else, so the
three-way differential — the Rust engine, the Python engine and the Lean checker
over the same documents — ran its preflight, found neither of the two tools it
needs, skipped every test in the file, and exited 0.

This page is the answer to "does a green tick mean this ran?", so that nobody has
to work it out again from six workflow files and the thirty-four test files that
can skip — twenty-five Rust, nine Python. It is a statement about CI
configuration, which changes. Its Rust half is now checked by a test rather than
by hand (`tests/ci_gate_coverage_test.rs`), because the by-hand version went
stale exactly where it mattered; the Python half still carries a date at the
bottom, and the way to re-check it is there too.

## The convention

`tests/common/mod.rs` defines `skip_unless(available, what, how_to_get_it)`.
When the thing is missing it prints `SKIPPED_FIXTURE: …` and returns `true`, and
when `OO_REQUIRE_FIXTURES` is exactly `"1"` it panics instead. Any job that is
supposed to PROVIDE a toolchain sets that variable, and the suite can then no
longer quietly shrink inside it.

`python/tests/conftest.py` does the same for the Python suite, and does it
globally: every skip is caught, whatever raised it — `pytest.skip`, a `skipif`
marker, a module-level `pytest.importorskip` — because in Python a skip needs no
helper to be written and a per-call-site rule would miss the next one. A skip
that is deliberate rather than missing (there is one, the twelve-minute
full-corpus differential) carries `@pytest.mark.oo_opt_in` and is exempt. The
marker is greppable; a reason string is not.

That mechanism only bites on a test that RUNS, which leaves the hole it cannot
see: a file no job invokes at all. `tests/ci_gate_coverage_test.rs` closes it by
reconciling this page's table against `tests/` and `.github/workflows/` on every
run. See "Re-checking this page" at the bottom.

Two tool scripts mirror the variable under their own names because they are CLIs
rather than test suites, and say so where they define it:
`HORN_DIFF_REQUIRE_TOOLS=1` in `tools/horn_differential.py` and
`FOL_DIFF_REQUIRE_ATP=1` in `tools/fol_differential.py`. Both exit 2 rather than
panicking.

## The table

**strict** means the leg runs under `OO_REQUIRE_FIXTURES=1` in a job that
provides its toolchain, so a missing tool fails the job. **skips** means the
toolchain is not provided and the test does nothing, loudly. **not run** means no
job invokes the file at all.

### Rust — `tests/*.rs`

| test file | needs | provided by | in CI |
|---|---|---|---|
| `lean_certificate_test.rs` | lake | `lean` job | **strict** |
| `justify_test.rs` | `benchmark/reference/pizza-reference.owl`, which ships with the repository | `lean` job | **strict** |
| `provenance_test.rs` | the same shipped ontology | `lean` job | **strict** |
| `shacl_core_verified_test.rs` | lake + vendored W3C SHACL suite | `lean` job | **strict** |
| `dl_model_certificate_test.rs` | lake + `oo-dlmodel` | `lean` job | **strict** |
| `shacl_verified_tool_test.rs` | lake + `oo-shacl` | `lean` job | **strict** |
| `dl_refutation_certificate_test.rs` | lake + `oo-dlrefute` | `lean` job | **strict** |
| `lean_refutation_test.rs` | lake + `oo-refute` | `lean` job | **strict** |
| `lean_refutation_producer_test.rs` | lake + `oo-refute` | `lean` job | **strict** |
| `lean_mixed_certificate_test.rs` | lake | `lean` job | **strict** |
| `lean_horn_certificate_test.rs` | lake | `lean` job | **strict** |
| `readme_example_test.rs` | lake | `lean` job | **strict** |
| `demo_asset_claims_test.rs` | lake | `lean` job | **strict** |
| `lean_fol_model_test.rs` | lake + `oo-folmodel` + z3 | `lean` job | **strict** |
| `fol_solver_verdict_test.rs` | z3 + `oo-folmodel` + the debug CLI | `lean` job | **strict** |
| `smt_second_oracle_test.rs` | z3 **and** cvc5 1.3.4 + `oo-folmodel` + the release binary | `lean` job installs cvc5 by digest and builds the release binary | **strict** |
| `dl_consistency_differential_test.rs` | z3 + `oo-folmodel` | `lean` job | **strict** |
| `lean_projection_entailment_test.rs` | lake | `lean` job | **strict** |
| `closure_diff_test.rs` | lake | `lean` job | **strict** |
| `projection_monotonicity_corpus_test.rs` | lake | `lean` job | **strict** |
| `preserve_cli_test.rs` | lake | `lean` job | **strict** |
| `gate_demonstration_test.rs` | lake | `lean` job | **strict** |
| `w3c_shacl_conformance_test.rs` | vendored W3C SHACL suite (in tree) | `w3c-shacl` job | **strict** |
| `certificate_boundary_proptest.rs` | lake | `lean` job | **strict** |
| `premise_order_test.rs` | `lean/OOCert/Rules.lean` (in tree) | every job that runs `cargo test` | **runs everywhere**: it READS the Lean source rather than building it, so TCB-14 is gated without a toolchain |
| `reason_rl_coverage_test.rs` | lake | `lean` job | **strict** |
| `rule_syntax_frontend_test.rs` | lake | `lean` job | **strict** |
| `reason_horn_emit_test.rs` | lake + `tests/fixtures/horn/` (in tree) | `lean` job | **strict** |
| `cross_kernel_differential_test.rs` | lake **and** Poly/ML | `lean` job | **strict** |
| `rocq_kernel_differential_test.rs` | lake **and** Rocq 9.2, which CI gets from the `rocq/rocq-prover:9.2.0` image because no Ubuntu repository carries it | `lean` job | **strict** |
| `tstp_derivation_test.rs` | the recorded Vampire and E derivations in `tests/fixtures/tstp/` (in tree); ONE test also wants vampire on `PATH` | any job for the recorded part; no job installs a prover | **skips** — 34 of 35 need nothing, but the file cannot be made strict while the 35th wants a prover no job installs |
| `tstp_cli_test.rs` | the same fixtures + the debug CLI | any job | **strict** |
| `clinical_test.rs` | `data/crosswalks.parquet` | nothing — `data/` is gitignored | **skips** |
| `embed_test.rs` | ONNX model in `~/.open-ontologies/models/` | `features/depth` compiles it; no job runs `open-ontologies init` | **skips** |
| `embedding_e2e_test.rs` | same ONNX model + tokenizer | same | **skips** |
| `claimcheck_pizza_bench.rs` | `/tmp/pizza_compiled.json` from `CompileOntology.java` | only `benchmark.yml` has a JDK, it is `workflow_dispatch` and does not produce the file | **skips** |
| `reasoner_budget_corpus_bench.rs` | the tracked corpus (in tree) | any job — but the one test is `#[ignore]` | **not run**, by design: it is a wall-clock measurement, run by hand |

`fol_model_ingest_test.rs` runs in the `lean` job under the same variable and has
no skip path at all, which is why it is not in the table.

`module_extract_test.rs` and `conservativity_test.rs` are absent for a different
reason: they need nothing the tree does not already carry. Both run the reasoner
and the closure diff with no checker on the path, take `engine_opinion` for the
certificate verdict, and assert nothing that depends on Lean, so there is no skip
path to make strict. `module_extract_test.rs` reads
`benchmark/reference/pizza-reference.owl`, which is tracked.

### Python — `python/tests/*.py`

| test file | needs | provided by | in CI |
|---|---|---|---|
| `test_horn_three_way_differential.py` | Rust engine + `oo-horn` + corpus | `python` job builds both | **strict** |
| `test_horn_differential.py` | Rust engine + `oo-horn` + `demo/` corpus | `python` job | **strict** |
| `test_horn_checked.py` | `oo-horn` + `tests/fixtures/horn/` | `python` job | **strict** |
| `test_horn_rule_table.py` | `tests/fixtures/horn/` + git | `python` job | **strict** |
| `test_shacl.py` | `pyshacl` (the `shacl` extra) | `python` job installs it | **strict** |
| `test_shacl_vacuous.py` | `pyshacl` | `python` job | **strict** |
| `test_align.py` | `hnswlib` (the `align` extra) | `python` job installs it | **strict** |
| `test_vocab_check.py` | `pyoxigraph` (a base dependency) | always present | **strict** |
| `test_dataframe.py` | `fenic` | **nothing** — `fenic` is in no extra and no dependency list | **skips** |

The other seven files under `python/tests/` have no skip path.

## What is still open, and why

- **No job installs a first-order prover.** `tools/fol_differential.py` skips
  loudly without one and `FOL_DIFF_REQUIRE_ATP=1` turns that skip into a
  failure, but no workflow sets it because no workflow installs E or Vampire.
  The derivation checker added on 15 September 2026 is in the same position: its
  recorded fixtures run everywhere, and `a_live_vampire_run_agrees_with_the_recorded_one`
  skips. The recorded half is not a substitute for the live half, since it pins
  the checker against two real derivations and not against whatever the
  installed prover does today, so a `brew install vampire` step, or its apt
  equivalent, is the smallest thing that would close this.
- **`clinical_test.rs`, `embed_test.rs`, `embedding_e2e_test.rs`.** All three
  want an artefact the repository deliberately does not carry: a licensed
  crosswalk table and a model download. Closing them means a job that fetches
  those, which is a cost and a licence question rather than a CI oversight.
- **`claimcheck_pizza_bench.rs`.** Needs a Java compile step that exists only in
  the dispatch-only benchmark workflow. Its skip at least goes through
  `skip_unless` now; before, it printed `SKIP:` and was invisible both to the
  `build` job's skip counter and to `OO_REQUIRE_FIXTURES`.
- **`test_dataframe.py`.** `fenic` is an undeclared optional test dependency.
  Declaring it as an extra and installing it in the `python` job is the fix; it
  was not made here because the dependency has not been assessed.
- **Two skips inside now-strict Python files are conditions, not missing
  toolchains, and `OO_REQUIRE_FIXTURES=1` will fail the job on either.** Both are
  wanted. `test_horn_differential.py` skips a corpus file that pyoxigraph cannot
  parse; a file under `demo/` that does not parse is a defect and should stop the
  job rather than vanish into an `s`. `test_horn_rule_table.py` skips when `git`
  is absent or the tree is not a checkout, which on a GitHub runner after
  `actions/checkout` cannot happen. Measured on the current tree: the eight
  strict Python files are 67 passed, 1 skipped, and the one skip is the
  `oo_opt_in` twelve-minute differential.
- **The `features/breadth` job runs no tests** (`test: false`); it is
  `cargo check` plus clippy across all features. That is deliberate and is not a
  skip, but it does mean an all-features build is type-checked and never run.
- **Kani.** `make verify` runs fifteen bounded-model-checking harnesses. No job
  installs Kani, so none of them runs in CI. `docs/trusted-computing-base.md`
  says so where it reports their results. (The count was three until 15
  September 2026 and this line had not followed it.)
- **Aeneas.** `aeneas/run.sh` re-translates `src/boundary_core.rs` into
  `aeneas/lean/OOBoundary/Generated.lean` and fails if the model moved, and
  `cd aeneas/lean && lake build` checks the theorems about it. NEITHER runs in
  any job, and neither is reachable from `make check`, `cargo build`,
  `cargo test` or `lean/`'s `lake build`. The first needs a 130 MB release
  tarball and a pinned rustc nightly; the second needs Lean v4.31.0 and a 7.6 GB
  `.lake` including Mathlib. `docs/aeneas-boundary.md` reports what they produced
  when they were run here, which is the same arrangement Kani is under and for
  the same reason: a gate nobody can run locally is not a gate.
- **Dafny.** `dafny/run.sh` verifies `dafny/RuleTable.dfy` and `dafny/Interner.dfy` and then mutates them six times and
  requires every mutation to be rejected, so the script fails both when the specification breaks and
  when the proof turns out not to depend on what it claims to. No job runs it, and `make
  verify-dafny` is the only thing that does. It is in the same position as Kani and Aeneas and for
  the same reason. It also proves less than either of those: it is a REIMPLEMENTATION of the
  rule-table grammar rather than the shipped Rust, so nothing it reports is evidence about
  `src/reason.rs`. Decision 0014 says why it is kept anyway.
- **The Makefile and CI have drifted.** No workflow invokes `make`. `make check`
  is `lint test audit`, and its `test` is a bare `cargo test` with no
  `OO_REQUIRE_FIXTURES`, so a local `make check` is the permissive run whatever
  is installed.

## Re-checking this page

The Rust half of this page no longer needs re-checking by hand, because a page
that has to be re-checked by hand is a page that goes stale. This one did: the
row for `cross_kernel_differential_test.rs` said the second kernel ran nowhere
in CI while `README.md` reported the result of that comparison in its opening
pages, and nothing connected the two.

`tests/ci_gate_coverage_test.rs` is the mechanism. It reads `tests/*.rs` for a
skip path, reads `.github/workflows/` for every `cargo test --test NAME` run
under `OO_REQUIRE_FIXTURES=1`, and fails when a file that can skip is neither
strict anywhere nor on a written exception list carrying the reason. It then
checks the Rust table above against the same two sources, so a row saying
**strict** for a leg no workflow runs is a test failure rather than a sentence.
It runs in the `lean` job and needs no toolchain of its own.

What it deliberately does not do is force every file to be strict. A contributor
without Poly/ML, without a licensed crosswalk table and without a downloaded
embedding model must still be able to run `cargo test` and get a pass. The
property is that CI cannot skip in silence, not that a laptop cannot skip at all.

The Python half is still by hand:

```bash
# every Python file that can skip: 10 hits, of which conftest.py is the
# mechanism and not a test, so 9, and the table has 9 rows
grep -rln 'pytest.skip\|importorskip\|skipif' python/tests/*.py
# every leg CI makes strict
grep -rn 'OO_REQUIRE_FIXTURES=1' .github/workflows/
```

Last checked against `.github/workflows/` on 15 September 2026.
