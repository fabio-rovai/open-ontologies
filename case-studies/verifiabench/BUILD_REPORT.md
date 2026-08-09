# BUILD_REPORT: verifiabench

Honest record of the benchmark, the oracle and the limits. Numbers in `results/` are produced by
`src/verifiabench.py` at temperature 0 against a local MLX OpenAI-compatible endpoint.

## What was used (real)

- **Authority:** the Biolink Model (biolink/biolink-model), 868 declared class and slot IRIs, reused
  from the bio-kg-triage case study.
- **Tasks:** 30 well-established gene-disease facts (BRCA1/breast cancer, HTT/Huntington disease,
  CFTR/cystic fibrosis, ...). Each task prompts the model to write Biolink-typed RDF asserting the fact.
- **Models under test (9):** five local open checkpoints over a local MLX server on port 8080
  (Qwen3-Coder-30B-A3B 8bit, Qwen3.6-35B-A3B 8bit, Qwen2.5-3B, Llama-3.2-3B, gemma-2-2b,
  Qwen2.5-0.5B), and Claude Haiku, Sonnet and Opus via the `claude -p` CLI. The runner speaks the
  OpenAI chat-completions API (any endpoint works via `VB_API`) and dispatches ids of the form
  `claude:<alias>` to `claude -p --model <alias>`.
- **Reasoning models.** Qwen3.6-35B is a reasoning model: it returns its chain-of-thought in a
  `reasoning` field and the answer in `content`. We score the `content` (the answer) only, never the
  chain-of-thought, and raised the token budget (`VB_MAXTOK=6000`) so it can finish reasoning and
  emit the answer. Under the default 400-token budget it never reached `content` and scored a
  spurious 0.00; that run was discarded, not published. The final 35B number uses the larger budget.

## The oracle (deterministic, no LLM judge)

For each model output:
1. Strip any markdown code fence.
2. Parse the Turtle with rdflib. This resolves whatever prefix the model bound to the Biolink
   namespace (models used both `biolink:` and `bio:`), so the check is format-robust, not
   prefix-brittle. If it does not parse, fall back to prefix-resolved regex extraction.
3. Collect every predicate and every `rdf:type` object whose IRI is in the Biolink namespace.
4. **Term existence:** each such term is real iff it is in Biolink's declared set; otherwise fabricated.
5. **Structure:** does the graph have a node typed `biolink:Gene`, a node typed `biolink:Disease`,
   and at least one real Biolink association predicate.
6. **raw_ok** = produced any Biolink-namespace term (the fluency proxy). **verified_ok** = real Gene
   AND real Disease AND real association predicate AND zero fabricated terms.

Per-task outputs (including the raw model text, truncated) are saved under `results/` for inspection.

## The result, precisely

- Raw capability saturates: 7 of 9 models score 1.00 (every competent model produces structured
  Biolink RDF), so fluency is nearly uninformative.
- Verified capability separates fully: Qwen3-Coder-30B and Claude Opus 1.00 (0 fabricated terms);
  Claude Haiku 0.93; Qwen3.6-35B 0.77; Claude Sonnet 0.73; Qwen2.5-3B and Llama-3.2-3B 0.00 (30/30
  tasks contain a fabricated term, ~50% term-existence); gemma-2-2b and Qwen2.5-0.5B produce little
  or no valid Biolink.
- Two failure modes the oracle distinguishes: **hallucination** (Qwen2.5-3B, Llama-3.2-3B emit
  genuine non-terms such as `biolink:has_association`, `biolink:has_role`) and **structural
  incompleteness** (Claude Sonnet: 0 fabricated terms but a missing typed `biolink:Disease` on some
  tasks, so verified < 1 despite 1.00 term-existence).
- Manual spot-check confirms the oracle in both directions: Qwen3-Coder-30B and Claude Opus emit
  real terms in valid structure (verified 1.00), so the oracle is measuring a real capability, not
  failing everything.

## Limits of the claim (do not overstate)

1. **One task family, one authority.** Biolink gene-disease RDF authoring. The method is
   authority-agnostic (GO, ChEBI, Reactome, EDAM are drop-in), but this run does not yet cover them;
   a multi-domain suite is the next step.
2. **The facts are public, not a secret split.** 30 well-known gene-disease pairs. The honest use is
   relative model comparison and a versioned, inspectable oracle, not a hold-out leaderboard.
3. **Existence and coarse structure, not full semantics.** The oracle checks that terms exist and
   that a Gene, a Disease and a real association predicate are present. It does not check that the
   *specific* predicate is the ideal one for the relation; that is the certified-denotation direction.
4. **Small models are underserved by the strict "zero fabricated terms" rule.** A model that gets the
   structure right but emits one extra invented term scores verified=0. This is deliberate: in a
   scientific pipeline a single fabricated identifier is a real defect. It is stated, not hidden.
5. **Local quantised models.** 4-8bit MLX quantisations, not full-precision or frontier API models;
   the point is the evaluation method and the raw-vs-verified gap, which any endpoint can reproduce.

## Reproducibility

`./run-demo.sh <models...>` reruns against a live endpoint (temperature 0). Model outputs can vary
across server versions, but the raw-vs-verified separation is the stable, reproducible finding.
rdflib at build time; models as listed.

## Expansion: multi-domain, multi-hop (`src/verifiabench_multi.py`)

A second authority (Gene Ontology, `go-basic.obo`, 48,329 term ids) and three task families:
single-hop gene-disease (Biolink), GO annotation (a real GO term for a named biological process),
and cross-ontology multi-hop (gene, disease and process, Biolink + GO in one graph). The oracle is
closed-world across BOTH authorities: every `biolink:` term must be a declared Biolink term and every
GO id a real GO id; multi-hop additionally requires the full structure from both authorities, so it
cannot be gamed by getting one right and inventing the other.

Result (6 models): Claude Opus 1.00 on all three families; Sonnet 0.75; Qwen3-Coder-30B 0.48 (1.00
Biolink, 0.00 GO, 0.07 multi-hop); Haiku 0.43; the two small local models 0.00. The single-family
benchmark had the 30B tied with Opus at 1.00; the multi-domain version breaks that tie and shows the
30B's ontology competence is narrow (Biolink) rather than general (it hallucinates GO ids).

**Caveat on the Claude runs.** The `claude -p` CLI loads the user's global CLAUDE.md and memory and
boots the full agent harness per call (about 9 s of overhead, the reason the Claude models are the
slow part; the calls are run concurrently to compensate). So the Claude numbers are "Claude Code with
this machine's configuration answering", not a bare API model. The oracle scores term existence
regardless, but a clean comparison would use the raw Anthropic API; that is a configuration change,
not a change to the benchmark.
