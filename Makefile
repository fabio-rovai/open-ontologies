.PHONY: build test lint audit verify verify-dafny bench bench-pizza bench-ontoaxiom bench-mushroom bench-vision bench-reasoner bench-oaei docker clean demo demo-verify demo-verify-pipeline

# ─── Development ─────────────────────────────────────────────────────────────

build:
	cargo build --release

test:
	cargo test

lint:
	cargo clippy -- -D warnings

audit:
	cargo audit

# Bounded model checking of the pure functions on the certificate layer's
# trusted boundary (docs/trusted-computing-base.md). Needs
# `cargo install --locked kani-verifier && cargo kani setup`, which pulls its
# own toolchain, so this is NOT part of `check`: it is minutes per harness and
# a developer without Kani installed must still be able to run the gates.
# `cargo test` covers the same statements by sampling.
# Every harness in `src/reason.rs` is here, which was not true before 15
# September 2026: `parse_pat_and_render_are_inverse` did not terminate (no
# verdict at 14 minutes and 9.5GB, worse when the input was constrained) because
# `parse_pat` returns `anyhow::Result` and CBMC flattens the error-formatting
# machinery whether or not the refusal paths are reachable. Lifting the
# classification into the pure `pat_of` closed it: it is now
# `pat_of_and_render_are_inverse_*` and it verifies in about four seconds.
verify:
	cargo kani --harness asserted_line_round_trips_at_0
	cargo kani --harness asserted_line_round_trips_at_1
	cargo kani --harness asserted_line_round_trips_at_2
	cargo kani --harness asserted_line_round_trips_at_3
	cargo kani --harness asserted_line_round_trips_at_4
	cargo kani --harness triple_fields_append_exactly_three_at_0
	cargo kani --harness triple_fields_append_exactly_three_at_2
	cargo kani --harness triple_fields_append_exactly_three_at_3
	cargo kani --harness triple_fields_append_exactly_three_at_4
	cargo kani --harness term_guard_admits_no_separator_at_3
	cargo kani --harness term_guard_admits_no_separator_at_6
	cargo kani --harness term_guard_separates_the_three_spellings
	cargo kani --harness pat_of_and_render_are_inverse_at_2
	cargo kani --harness pat_of_and_render_are_inverse_at_3
	cargo kani --harness writable_triple_decides_both_positions

# Verify dafny/RuleTable.dfy, a Dafny model of the rule-table grammar in
# src/reason.rs, and then mutate it three times and require every mutation to be
# rejected. Needs `brew install dafny`; Dafny bundles its own Z3. About sixteen
# seconds in total.
#
# NOT part of `check`, and read the next sentence before quoting any output of
# it. The model is a REIMPLEMENTATION, so what verifies is the Dafny and not the
# Rust, which is the opposite of the Kani harnesses above and of Aeneas. It is
# kept because it proves TCB-20's whole-line round trip with no length bound,
# which neither of those can, and because writing it found that TCB-20 was
# documented without the hypothesis it needs. See
# docs/decisions/0014-a-verifier-that-cannot-read-the-code-verifies-a-rewrite.md.
verify-dafny:
	./dafny/run.sh

check: lint test audit

# ─── Benchmarks ──────────────────────────────────────────────────────────────

bench: bench-pizza bench-ontoaxiom bench-mushroom bench-reasoner bench-oaei
	@echo "All benchmarks complete."

bench-pizza:
	@echo "=== Pizza Ontology Benchmark ==="
	cd benchmark && python3 pizza_benchmark.py 2>/dev/null || echo "Requires Python 3 + rdflib"

bench-ontoaxiom:
	@echo "=== OntoAxiom Benchmark ==="
	cd benchmark/ontoaxiom && python3 run_benchmark.py 2>/dev/null || echo "Requires Python 3 + dependencies"

bench-mushroom:
	@echo "=== Mushroom Classification Benchmark ==="
	cd benchmark/mushroom && python3 mushroom_benchmark.py 2>/dev/null || echo "Requires Python 3 + dependencies"

bench-vision:
	@echo "=== Vision Pipeline Benchmark ==="
	cd benchmark/vision && python3 vision_benchmark.py 2>/dev/null || echo "Requires Python 3 + dependencies"

bench-reasoner:
	@echo "=== Reasoner Comparison (HermiT vs Open Ontologies) ==="
	cd benchmark/reasoner && python3 compare_reasoners.py 2>/dev/null || echo "Requires Python 3 + Java for HermiT"

bench-oaei:
	@echo "=== OAEI Alignment Benchmark ==="
	cd benchmark/oaei && python3 download_oaei.py && python3 run_oaei_benchmark.py 2>/dev/null || echo "Requires Python 3 + mcp SDK"

# ─── Docker ──────────────────────────────────────────────────────────────────

docker:
	docker build -t open-ontologies:latest .

docker-run:
	docker run -i open-ontologies:latest serve

# ─── Release ─────────────────────────────────────────────────────────────────

init:
	cargo run --release -- init

serve:
	cargo run --release -- serve

serve-http:
	cargo run --release -- serve-http

# ─── Demo pipeline ────────────────────────────────────────────────────────────
#
# The brief for this target assumed a `corpus_pipeline.py --corpus ... --out
# ...` interface that does not exist: corpus_pipeline.py takes no --out flag
# and depends on demo/bundle/dcat-us-vocab.ttl + dcat-us-full.ttl, neither of
# which is present for the real DCAT-US corpus. The script that actually
# derives an ontology from demo/corpus/dcat-us/ end to end -- and that the
# demo/README.md "Running it" section already documents -- is
# demo/ontology_from_docs.py. contradiction_scan.py similarly targets the
# other (absent) bundle; the contradiction findings here are pulled straight
# out of the store ontology_from_docs.py writes, via build_precomputed.py.
#
# compare.json is not regenerated by this target. It is produced once by
# demo/build_compare.py (which calls the model for a grounded answer and a
# baseline answer per question) followed by a human read of both answers to
# write an honest `divergence` field -- exactly the judgement Task 8 decision
# 4 says a script must not make unsupervised. It is committed like the rest
# of demo/precomputed/ and is a precondition for `make demo`, the same way
# demo/corpus/dcat-us/ is: an input this target consumes, not one it derives.
demo:
	python3 demo/ontology_from_docs.py --corpus demo/corpus/dcat-us --workers 2
	python3 demo/corpus_text.py
	python3 demo/build_precomputed.py --out demo/precomputed
	python3 demo/build_validation.py --out demo/precomputed
	python3 demo/bundle_fixtures.py --in demo/precomputed --out demo/precomputed/bundle.json
	cd demo/precomputed && shasum -a 256 *.json | sort > MANIFEST.sha256

# demo-verify is the command a sceptic runs: it must work on a fresh clone,
# with no model server, no engine process, and no prior `make demo` run.
# demo/verify.py needs demo/derived/_ontology.ttl (written by `make demo`,
# which calls a local model) and a running engine over MCP, neither of which
# a fresh clone has, so it is NOT part of this target -- see
# demo-verify-pipeline below. Everything demo-verify does runs the
# conformance script and its tests first (dcat_conformance.py has no network
# access and no model call of its own), so the published numbers in
# demo/precomputed/findings.json are re-derived, not merely checked as bytes
# that have not moved; the checksum check after it additionally guards every
# other precomputed artifact against silent drift.
demo-verify:
	python3 demo/dcat_conformance.py
	python3 -m pytest demo/tests/test_dcat_conformance.py -q
	cd demo/precomputed && shasum -a 256 -c MANIFEST.sha256

# demo-verify-pipeline re-checks the document-to-ontology pipeline itself
# (parse, stats, lint, enforce, vocab, reason) against whatever is currently
# under demo/derived/. Unlike demo-verify, this needs state that only `make
# demo` produces (demo/derived/_ontology.ttl) and a running engine reachable
# over MCP -- ontology_from_docs.py's call_model() also expects a local model
# server at ONTO_LLM_BASE_URL (default http://localhost:8081/v1). It is a
# separate target precisely so a fresh clone can still run demo-verify.
demo-verify-pipeline:
	python3 demo/verify.py

# ─── Cleanup ─────────────────────────────────────────────────────────────────

clean:
	cargo clean
