# Open Ontologies

## Ontology Engineering Workflow

When building or modifying ontologies, follow this workflow. Claude decides which tools to call and in what order based on results — this is not a fixed pipeline.

### Generate

1. Understand the domain requirements (natural language, competency questions, methodology constraints)
2. Generate Turtle/OWL directly — Claude knows OWL, RDF, BORO, 4D modeling natively

### Validate and Load

3. Call `onto_validate` on the generated Turtle — if it fails, fix the syntax errors and re-validate
4. Call `onto_load` to load into the Oxigraph triple store
5. Call `onto_stats` to verify class count, property count, triple count match expectations

### Reason

6. Call `onto_defects` FIRST. A reasoner amplifies whatever the declarations say, so a self-contradicting ontology makes every inference suspect. Fix what it reports before materializing anything
7. Call `onto_dlp_boundary` SECOND, and read it before trusting anything `onto_reason` returns. `onto_defects` asks whether the declarations contradict each other; this asks a different question the rest of the pipeline never answers: **which of your axioms the rule table can actually see**. A certificate from `onto_reason` is a sound proof about the axioms the rules read and says nothing at all about the ones no rule fires on, so a green certificate over an ontology whose TBox is half invisible is true and misleading at once. The report keeps three failures apart and you act on each differently, because they are fixed in three different places. An axiom `outside_the_fragment` (a disjunction in the consequent, an existential in the head, a cardinality restriction, a negation in the antecedent) is not Horn, and only a rewrite of the ontology helps. An axiom `inside_the_fragment_but_a_rule_is_not_implemented` (`owl:hasKey`, `owl:propertyChainAxiom`, `owl:FunctionalProperty`) is Horn, has an OWL 2 RL rule, and is invisible only because this engine does not run it, so nothing about the ontology is wrong. And an axiom `partially_inside_the_fragment` is half seen, and the question is which half you needed. `not_fully_seen_by_the_rule_table` carries all three counts and never adds them
8. Call `onto_reason` with profile `rdfs` or `owl-rl` to materialize inferred triples (transitive subclass chains, domain/range propagation, equivalentClass expansion). The profile decides how much of the rule table runs: `rdfs` evaluates 6 of the 29 rules, `owl-rl` 17 and `owl-rl-ext` all 29, and `onto_dlp_boundary` reports which
9. Call `onto_stats` again to verify inferred triple counts are reasonable

### Verify

10. Call `onto_lint` to check for missing labels, comments, domains, ranges — fix any issues found
11. Call `onto_enforce` with rule pack `generic` to check design pattern compliance — fix any violations
12. Call `onto_query` with SPARQL to verify structure:
    - Are all expected classes present?
    - Do subclass hierarchies match the spec?
    - Can competency questions be answered?
13. If a reference ontology exists, call `onto_diff` to compare

### Iterate

14. If any step above reveals problems, fix the Turtle and restart from step 3
15. This loop continues until validation passes, stats match, lint is clean, enforce has no violations, and SPARQL queries return expected results

### Persist

16. Call `onto_save` to write the final ontology to a .ttl file
17. Call `onto_version` to save a named snapshot for rollback — always version after save

### Key Principle

Claude dynamically decides the next tool call based on what the previous tool returned. If `onto_validate` fails, Claude fixes and retries. If `onto_stats` shows wrong counts, Claude regenerates. If `onto_lint` finds missing labels, Claude adds them. The MCP tools are individual operations — Claude is the orchestrator.

## Tool Reference

| Tool | When to use |
| ---- | ----------- |
| `onto_status` | To check if the server is running and healthy |
| `onto_validate` | After generating or modifying Turtle — always validate first |
| `onto_load` | After validation passes — loads into triple store for querying |
| `onto_stats` | After loading — sanity check on class/property/triple counts |
| `onto_lint` | After loading — catches missing labels, domains, ranges |
| `onto_query` | To verify structure, answer competency questions, explore the ontology |
| `onto_diff` | To compare against a reference or previous version |
| `onto_save` | To persist the ontology to a file |
| `onto_convert` | To convert between formats (Turtle, N-Triples, RDF/XML, N-Quads, TriG) |
| `onto_clear` | To reset the store before loading a different ontology |
| `onto_marketplace` | To browse and install ontologies: 33 curated W3C/ISO/industry standards plus community packs from the open registry (`community/registry.json`, fetched at runtime; `community=false` for curated/offline only). Curated IDs always win over community IDs |
| `onto_plugin_list` | To discover installed WASM plugins (community `.wasm` tools in `~/.open-ontologies/plugins` or `./plugins`) and the tools each declares. Requires a build with `--features plugins` |
| `onto_plugin_call` | To invoke a plugin tool. Plugins are sandboxed pure JSON→JSON transforms with no store access — pass `sparql` to run a SELECT and inject its rows into the plugin input as `bindings`. Use for domain-specific lint/validation logic contributed by the community |
| `onto_pull` | To fetch an ontology from a remote URL or SPARQL endpoint |
| `onto_push` | To push an ontology to a SPARQL endpoint |
| `onto_import` | To resolve and load owl:imports chains |
| `onto_version` | To save a named snapshot before making changes |
| `onto_history` | To list saved version snapshots |
| `onto_rollback` | To restore a previous version if something goes wrong |
| `onto_ingest` | To parse structured data (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet) into RDF and load into the store |
| `onto_sql_ingest` | To run a SQL `SELECT` against PostgreSQL or DuckDB and ingest the result rows into RDF (uses the same mapping format as `onto_ingest`). DuckDB acts as a federation backbone via its `httpfs`/`parquet`/`csv`/`postgres_scanner`/`iceberg` extensions. Connection strings: `postgres://…`, `duckdb:///path.duckdb`, `:memory:`, or `*.duckdb` file path. |
| `onto_map` | To generate a mapping config from data schema + loaded ontology for review |
| `onto_ossie_import` | To compile an Apache Ossie (incubating, formerly Open Semantic Interchange) ontology document into OWL 2 DL + SHACL and optionally load it, making a vendor semantic model reasonable/validatable for the first time. Reports the four constructs OWL 2 DL cannot express (`OneToOne` onto a `ValueType`, `ManyToOne` on arity>=3, `derived_by`, non-scalar `requires`) instead of dropping them |
| `onto_shacl` | To validate loaded data against SHACL shapes (cardinality, datatypes, classes) |
| `onto_vocab_check` | To closed-world-check generated DATA: flags any predicate/class used that is not declared in the loaded ontology (hallucinated terms). Catches what open-world SHACL silently passes. Run on LLM-generated Turtle before `onto_load` |
| `onto_defects` | BEFORE trusting any fact-level result, and on every marketplace pack before adopting it. Checks the ONTOLOGY against itself with no data present: `transitive_and_functional`, `symmetric_and_asymmetric`, `subclass_cycle`, `sub_property_cycle`, `disjoint_with_ancestor`, `inherited_disjoint`, `self_inverse`, `inverse_not_mutual`. A different question from `onto_dl_check`: a transitive functional property is satisfiable and is still a trap, because the pair manufactures contradictions the moment instances arrive. Each kind is listed at most 50 times, with the full total under `truncated` |
| `onto_dlp_boundary` | BEFORE trusting any `onto_reason` result, in the same register as `onto_defects`. Asks which of YOUR axioms the rule engine can actually SEE. A certificate from `onto_reason` is a sound proof about the axioms the rules read and says nothing about the ones no rule fires on, so a user can reason, get a green machine-checked certificate, and never learn that a third of the TBox was invisible. TWO DIMENSIONS THAT ARE NEVER MERGED, because one is a rewrite of the ontology and the other is a patch to this engine. (1) THE FRAGMENT: is the axiom Horn at all? The line is OWL 2 RL's class grammar (OWL 2 Profiles 4.3), and `outside` there would hold of a perfect OWL 2 RL engine: disjunction in the consequent, existential in the head, cardinality restriction, negation in the antecedent, each listed per axiom with the reason. (2) THIS ENGINE: does a rule fire on it? `owl:hasKey` and `owl:propertyChainAxiom` are Horn, OWL 2 RL has `prp-key` and `prp-spo2`, and this engine runs neither, so they are INSIDE the fragment and still invisible. An axiom that splits soundly is `partially_inside`: `A subClassOf (B and Out)` keeps its `A subClassOf B` half, and an `owl:equivalentClass` with an existential on one side keeps the direction `cls-svf1` evaluates. A conjunction in the ANTECEDENT does not split, because dropping a conjunct from a rule body makes it fire more often. The rule-table figures (78 OWL 2 RL rules, 29 evaluated, 10 more detected only as a clash, 7 that conclude false and are not looked for) are DERIVED from `src/reason.rs`, not typed. Every triple lands in an axiom bucket or a counted `not_classified` one. NOT a consistency check |
| `onto_reason` | To run RDFS or OWL-RL inference, materializing inferred triples. Pass `inference_graph: true` to keep them in `https://open-ontologies.org/graph/inferred` instead of the default graph, so nothing downstream can read an inference as an assertion and `onto_save` to Turtle/RDF-XML cannot publish one. Default false (unchanged behaviour); not available for `owl-dl`. Pass `certificate_dir` to also write a derivation certificate (`asserted.tsv` + `derivations.tsv`) that the proved-sound Lean checker in `lean/` verifies, plus `refutation.tsv` when the run finds a contradiction `oo-refute` can judge. Ten of the seventeen OWL 2 RL rules that conclude `false` are detected and only `cax-dw` is certifiable, so `inconsistency.verdict` is `clash_found_by_this_engine` and NEVER the checker's `unsatisfiable_under_disjointness`; no clash found is not consistency; see docs/lean-certificates.md |
| `onto_reason_incremental` | After adding facts to an already-materialised graph. Derives the consequences of the ADDED triples only (semi-naive evaluation, joining the delta against the closure), so the cost tracks what changed rather than the size of the store: effectively instant against a 1.9M-triple store where full re-materialisation takes seconds. Refuses schema axioms (subClassOf, domain, range, inverseOf, equivalentClass) with an explanation, because those change what the whole store entails: run `onto_reason` for them |
| `onto_rules_import` | To read rules you wrote in a STANDARD rule syntax into the Horn rule table `onto_reason` evaluates with `rules_file`. `from: "swrl"` reads SWRL rules encoded in RDF (`swrl:Imp` with `swrl:body`/`swrl:head` as `rdf:List` atom lists) out of the loaded graph or out of `file`; `from: "rif"` reads RIF Core in its XML syntax (the presentation syntax is refused, not half-read). Only a FRAGMENT of each language is a Horn table over triple patterns: SWRL built-in atoms, same-individual and different-individual atoms, data ranges and anonymous class expressions are refused, as are RIF equality, External, Expr, `rif:local` constants, list terms, Or/Neg/Naf, an existential conclusion and an Atom of arity 0 or 3+. Every refusal is NAMED AND COUNTED and by default fails the whole import with no table written, because a rule set that quietly lost half its rules still reaches a fixpoint and still produces a certificate that checks green. `allow_partial: true` imports the rest and flags `certifies_a_weaker_rule_set`. A table from this tool can only ever earn `entailed_under_supplied_rules` |
| `onto_extend` | To run the full pipeline: ingest → SHACL validate → reason in one call |
| `onto_import_schema` | To import a PostgreSQL or DuckDB database schema as an OWL ontology (requires `postgres` and/or `duckdb` features). Auto-dispatches on connection-string scheme. |
| `onto_plan` | Before applying changes — shows added/removed classes, blast radius, risk score. All of that is about SHAPE; pass `check_conservativity: true` for the only part that is about MEANING, namely whether the change alters any consequence over the names the store already uses. A new `rdfs:domain` reclassifies every existing individual of that property while adding no class and removing nothing, so every other number in the plan stays green. Opt-in because it reasons both graphs to a fixpoint; when it is off the plan says so rather than staying silent |
| `onto_apply` | After plan + enforce — applies changes in `safe` or `migrate` mode |
| `onto_module_extract` | To take a subset of an ontology that CANNOT lose an entailment over a signature, as opposed to a slice whose loss then has to be measured. Syntactic locality: `bottom` (⊥), `top` (⊤) or `star` (the iterated ⊥⊤*, default and smallest). The coverage theorem is Cuenca Grau, Horrocks, Kazakov and Sattler (JAIR 31, 2008) and is CITED, NOT machine-checked (nothing under `lean/` is about locality), so pass `verify_out_dir` to have the consequence measured instead: it reasons the ontology and the module to a fixpoint and reports every conclusion over the signature the module does not reach, which must be none. The guarantee covers subclass, equivalence, disjointness, disjoint union, subproperty, property chains, domain, range, inverses, the seven property characteristics, `owl:hasKey`, class/property/negative-property assertions, declarations and annotations, over the full OWL 2 class-expression grammar. `owl:sameAs`, `owl:differentFrom`, `owl:AllDifferent`, any unrecognised RDF/RDFS/OWL predicate, any unrecognised blank-node structure and any malformed `rdf:List` are included CONSERVATIVELY with no locality test, and are counted and named in the report so a small module and an unreadable one cannot look the same |
| `onto_conservative_check` | To ask whether adding axioms changes what the ontology ALREADY said, over the names it already used. This is the semantic safety net for the plan/apply lifecycle. Reports every new consequence over the old signature with the rule and the premises the base lacked; a non-conservative extension is a FINDING, not an error. Read the verdict from `conservativity_verdict` (`conservative_under_rule_table`, `not_conservative_under_rule_table`, `undecided_scan_truncated`, `undecided_not_an_extension`) and note there is deliberately no field called `conservative`: what is computed is conservativity with respect to the Horn rule table the engine evaluates, which is NOT deductive conservativity in a description logic (ExpTime-complete for EL, 2ExpTime-complete for ALC, undecidable for ALCQIO) and NOT model conservativity (undecidable already for EL). A finding is real; finding nothing proves only that this rule table derives nothing new. `mode` is `delta` or `replacement` and is never guessed |
| `onto_lock` | To protect production IRIs from removal |
| `onto_drift` | To compare two versions — rename detection, drift velocity, self-calibrating confidence |
| `onto_enforce` | After loading — design pattern checks: `generic`, `boro`, `value_partition`, `hierarchy`, or custom rules |
| `onto_monitor` | After apply — run SPARQL watchers with threshold alerts. Watchers with `webhook_url` POST alerts to external systems (Slack, PagerDuty, etc.) |
| `onto_monitor_clear` | To clear blocked state after resolving monitor alerts |
| `onto_crosswalk` | To look up clinical terminology mappings (ICD-10 ↔ SNOMED ↔ MeSH) |
| `onto_enrich` | To add skos:exactMatch triples linking classes to clinical codes |
| `onto_validate_clinical` | To check class labels against clinical crosswalk terminology |
| `onto_align` | To detect alignment candidates (equivalentClass, exactMatch, subClassOf) between two ontologies using 7 weighted signals (6 structural + embedding similarity when embeddings are loaded). Labels are matched with their parsed BCP-47 language tag; with the multilingual embedder loaded, cross-lingual pairs that share no surface tokens (e.g. `Dog`↔`Chien`) are admitted via the embedding signal. Restrict the languages consulted with `[language] preferred = [...]` / `OPEN_ONTOLOGIES_LANGUAGES` (empty = all) |
| `onto_align_feedback` | To accept/reject alignment candidates for self-calibrating confidence weights |
| `onto_communities` | To cluster the entity graph and get a SKELETON per community (size, top members by degree, internal relations, bridges to other communities). Deterministic modularity optimisation, no model involved. Write one report per skeleton, then answer corpus-wide questions ("what are the themes", "summarise this corpus") from the reports instead of traversing from an anchor entity, which such questions do not have. The GraphRAG global-search pattern with the expensive half left to the orchestrator |
| `onto_support_check` | To check whether claims are backed by the sources they cite. Returns which claims cite NO source (computed) and, for the rest, verification TASKS: the claim as a sentence, the source, what to decide. Complements `onto_vocab_check`: conformance asks whether a claim is expressible, support asks whether it is true to its source, and a claim can fail either independently |
| `onto_support_verdict` | To record supported / refuted / unrelated for a claim after reading its source. Verdicts persist and are skipped on the next check |
| `onto_support_report` | To summarise provenance quality: unsourced-claim rate over the graph, support rate across the claims judged so far |
| `onto_pack` | To write the loaded graph and its verification evidence to a portable versioned artifact: sorted N-Triples, a manifest (name, version, counts, timestamp, tool version, sha256) and the lint/enforce results recorded at pack time. What you promote between environments is then a graph that has already passed its checks, with the evidence attached |
| `onto_unpack` | To load a pack, refusing it if the checksum does not match. `verify_only` inspects the manifest and evidence without loading |
| `onto_temporal_snapshot` | To see which named graphs are in scope at a point in time. Two independent clocks: `valid_at` asks what was TRUE then, `as_of` asks what was KNOWN then. Both intervals are half-open: `temporal:recordedUntil` closes the recorded one, so `as_of` answers what was believed then rather than everything recorded by then. Graphs without validity metadata are timeless and always in scope, so the vocabulary is additive to an existing store. Bounds on all four axes are read as instants on the UTC timeline from `xsd:date`, `xsd:dateTime`, `xsd:gYearMonth` or `xsd:gYear`: a less precise bound names the FIRST instant of its period, and a value with no timezone offset is UTC. A bound matching none of those, or two different instants on one axis, makes the graph `invalid`: reported with a reason, never in scope and never timeless |
| `onto_temporal_query` | To run a SPARQL graph pattern against only the graphs in temporal scope: what the graph said at a given valid time, as known at a given recorded time |
| `onto_temporal_conflicts` | To separate genuine contradictions from pairs that never met and from asserted corrections. A pair where one graph `temporal:supersedes` the other, directly or through a chain, lands in `corrections` whatever its periods, lineage that is asserted and never inferred; otherwise a disjointness violation only counts when the two assertions claim OVERLAPPING validity, and everything else lands in `non_overlapping`, which proves only that the two share no instant, judged on their bounds read as instants on the UTC timeline so a timezone offset is honoured: not that one replaced the other, and it also holds pairs separated by a gap, which is missing coverage rather than history. A pair whose graph has temporal metadata that could not be read goes to `undecided`, in neither bucket, because an unreadable period is not an open one. `superseded` is the same set under its old name, deprecated and dropped at 2.0 |
| `onto_lineage` | To view the session's lineage trail (plan → enforce → apply → monitor → drift) |
| `onto_lint_feedback` | To accept/dismiss a lint issue — teaches lint to suppress repeatedly dismissed warnings |
| `onto_enforce_feedback` | To accept/dismiss an enforce violation — teaches enforce to suppress repeatedly dismissed violations |
| `onto_dl_explain` | To explain why a class is unsatisfiable using DL tableaux reasoning — returns clash trace |
| `onto_dl_check` | To check if one class is subsumed by another using DL tableaux reasoning |
| `onto_fol_export` | To hand the ontology to the automated-theorem-proving ecosystem. Writes TPTP FOF (what E and Vampire read), ISO/IEC 24707 CLIF (the standards-track interchange syntax, and what ISO/IEC 21838-2 publishes BFO in), restricted both to the first-order-equivalent fragment of Common Logic and to the subdialect ISO/IEC 24707 A.4.2 calls exactly semantically conformant, each checked by a test rather than asserted, and ISO/IEC 24707 CGIF, the second of Common Logic's three dialects, in CORE CGIF and in the compact sub-dialect clause 7.1.1 names (no sequence markers, which is what clause 6.5 says takes Common Logic past first order; also no `#?` type label and no actor). CGIF conformance is pinned by a lexer, parser and checker written from Annex B's EBNF IN THE TEST, not by an independent parser, because no installable CGIF parser exists; the CLIF claims rest on two external parsers and the CGIF claims do not, and the report names XCL as the dialect still not emitted. Both are serialisers over ONE translation: the one `OwlLean.adequacy` in the sibling owl-lean project is machine-checked about, including the background axioms and the individual typing axioms that theorem requires. The correspondence between this emitter and that Lean is PINNED BY TESTS AND NOT PROVED, and the emitted file says so. Constructs outside the fragment are named with counts and reasons in `constructs_not_exported`, never dropped silently. CLIF sentences are emitted BARE with standalone `(cl:comment '...')` labels, because the two CLIF parsers that exist both return an EMPTY theory from the wrapped `(cl:comment '...' SENTENCE)` form that ISO/IEC 21838-2's own BFO files use; `clif_comments: "wrapped"` restores that form and loses the content. `smtlib` (SMT-LIB 2, what Z3 reads) and `ladr` (what Mace4 reads, every symbol MANGLED with the table in `symbols.tsv`, because LADR reads a name beginning with u, v, w, x, y or z as a VARIABLE) write the same translation for MODEL FINDERS, and assert the NEGATED goal because a countermodel to `G |= phi` is a model of `G + {not phi}`. Every run also writes `problem.tsv` with its digest, the format the verified checker reads. Pass `goals_file` to also write one problem per conjecture. A PROVER'S VERDICT ON THE OUTPUT IS AN ORACLE OPINION AND NEVER A CERTIFICATE: use tools/fol_differential.py, which reports disagreement between this engine and an ATP and does not adjudicate it. See docs/first-order-export.md and decision 0005 |
| `onto_fol_model` | To find a FINITE MODEL of the loaded ontology and CHECK it, so the answer names what it rests on. The SAT/SMT family, and deliberately not symmetric: a refutation cannot be replayed in core Lean, so a solver's `unsat` is an ORACLE OPINION for ever, while a MODEL is a finite object and `Fol.satisfiable_of_check` in `lean/Fol/` turns an accepted structure into satisfiability of exactly the formulas it was checked against. With a goal, `Fol.not_entails_of_check` gives a machine-checked NON-ENTAILMENT, the one sentence no prover can produce. FIVE FIELDS THAT ARE NEVER COLLAPSED: `solver_verdict`, `encoding`, `checker_exit`, `verdict`, `owl_reading`. Five verdict words: `model_checked` is the ONLY certified one and requires `checker_exit: 0`; `no_model_up_to_size_k` is an exhausted BOUNDED search and IS NOT unsatisfiability; `unsatisfiable_oracle` may come only from a run with no cardinality constraint. A solver answering `sat` whose model the checker REJECTS is a STOP_THE_LINE disagreement, never `rejected` and never `model_checked`. Needs z3 or mace4 and `oo-folmodel` (`cd lean && lake build`); the absence of either is reported in `skipped`. See docs/decisions/0006 |
| `onto_embed` | After loading an ontology — generates text + Poincaré structural embeddings for all classes. The default local model is **multilingual** (`paraphrase-multilingual-MiniLM-L12-v2`), so labels in different natural languages embed into a shared space. Honours `[embeddings] provider = "local" \| "openai"` in `config.toml`; OpenAI-compatible gateways (Azure, Ollama, vLLM, LocalAI, …) are supported via `OPEN_ONTOLOGIES_EMBEDDINGS_*` env vars |
| `onto_search` | To find classes by natural language description — requires onto_embed first |
| `onto_similarity` | To compute embedding similarity between two specific IRIs |
| `onto_unload` | To unload the active ontology from memory. Optional `name` targets a specific cached entry; `delete_cache=true` also removes the on-disk N-Triples cache file |
| `onto_recompile` | To re-parse the source file and rebuild the cache. Optional `name` rebuilds a non-active cached entry without disturbing the in-memory store (safe background refresh) |
| `onto_cache_status` | To inspect the compile cache: active slot, all cached entries, and effective `[cache]` config (TTL, auto_refresh, dir) |
| `onto_cache_list` | To list cached ontologies with metadata (`is_active`, `in_memory`, mtime, size) — lighter than `onto_cache_status` |
| `onto_cache_remove` | To remove a cached ontology by `name`. Pass `delete_file=false` to keep the on-disk N-Triples |
| `onto_repo_list` | To enumerate RDF/OWL files in directories configured under `[general] ontology_dirs`. Use in containerized deployments to discover ontologies without hardcoding paths |
| `onto_repo_load` | To load an ontology from a configured `ontology_dirs` repo by bare name, relative path, or absolute path. Reuses the same compile-cache / TTL-eviction path as `onto_load` |

## Ontology Lifecycle

When evolving an ontology in production, follow this Terraform-style cycle. Claude decides which steps to include based on the change.

### Plan

1. Call `onto_plan` with the proposed Turtle — returns added/removed classes/properties, blast radius, risk score
2. If any IRIs are locked (`onto_lock`), locked violations will appear in the plan — resolve before proceeding
3. Review the risk score: `low` (additions only), `medium` (modifications), `high` (removals with dependents)

### Enforce

4. Call `onto_enforce` with a rule pack (`generic`, `boro`, `value_partition`, `hierarchy`, `ies4`) — checks design pattern compliance. The `ies4` pack catches 4D-modelling violations specific to the UK Information Exchange Standard (particular/ClassOfEntity overlap as error; State without `isStateOf` and Event without participant pattern as warnings).
5. Fix any violations before applying

### Apply

6. Call `onto_apply` with mode `safe` (clear + reload) or `migrate` (add owl:equivalentClass/Property bridges)
7. Lineage is recorded automatically

### Monitor

8. Call `onto_monitor` to run SPARQL watchers — alerts trigger notify, block, or auto-rollback actions
9. If blocked, resolve the issue and call `onto_monitor_clear`

### Drift

1. Call `onto_drift` to compare versions — drift velocity, rename detection with self-calibrating confidence
2. Feed back rename accuracy to improve future confidence scores

## Data Extension Workflow

When applying an ontology to external data:

### Inspect and Map

1. Call `onto_map` with the data file — it returns field names, ontology classes/properties, and a suggested mapping
2. Review the mapping — adjust predicates, set the class, mark lookup fields
3. Optionally save the mapping to a file for reuse

### Ingest

4. Call `onto_ingest` with the data file and mapping — it generates RDF triples and loads them into the store
5. Call `onto_stats` to verify triple counts match expectations

### Validate

6. Call `onto_shacl` with SHACL shapes to validate the data against constraints
7. Fix any violations (adjust mapping or data), re-ingest if needed

### Reason

8. Call `onto_reason` with profile `rdfs` or `owl-rl` to infer new triples
9. Call `onto_query` to verify inferred knowledge is correct

### Or use the convenience pipeline

10. Call `onto_extend` to run ingest → SHACL → reason in one call

## Semantic Search & Embedding Workflow

When exploring or aligning ontologies using semantic embeddings:

### Setup

1. Confirm the server was built with `--features embeddings`, then run `open-ontologies init` to download the model. On a default build, every tool in this section returns `Compiled without embeddings feature`.
2. Call `onto_load` to load the ontology
3. Call `onto_embed` to generate text + structural embeddings for all classes

### Search

4. Call `onto_search` with a natural language query — returns most similar classes
5. Use `mode: "text"` for label/definition similarity, `mode: "structure"` for hierarchy position, `mode: "product"` for combined

### Compare

6. Call `onto_similarity` with two IRIs to see cosine + Poincaré distance between them

### Alignment Enhancement

7. When running `onto_align`, embedding similarity is automatically used as signal #7 if embeddings are loaded
8. This catches semantically equivalent classes that have different labels (e.g., Vehicle ↔ Automobile)

### Cross-Lingual Alignment

The default `onto_embed` model is multilingual, so labels in different natural
languages embed into a shared vector space. `onto_align` parses the BCP-47
language tag off each label and, when label similarity is near zero (as it is
across languages — `Dog` vs `Chien` share no tokens), lets a strong embedding
match bypass the label pre-filter so the pair is still scored. Such pairs
typically surface as `borderline` candidates with their language-tagged labels
in the `context` block, for review via `onto_align_feedback`.

Control which languages are consulted with the `[language]` config section:

```toml
[language]
# Empty (default) = keep ALL languages — multilingual matching.
# Restrict to a set to pin matching to specific languages (untagged labels are
# always kept). Override with OPEN_ONTOLOGIES_LANGUAGES=en,fr
preferred = []
```

### Borderline-Candidate Review (LLM-as-Oracle Pattern)

`onto_align` partitions candidates into three buckets by confidence:

- `auto_applied`: confidence ≥ `high_threshold` (default 0.85) — applied as triples
- `borderline`: confidence in [`low_threshold`, `high_threshold`) (default low 0.4) — surfaced with rich `context` (source/target labels, source/target parents) for review
- below `low_threshold`: dropped

When borderline pairs are present, the tool returns a `summary_for_review` string instructing you (the connected LLM) to:

1. **Inspect each borderline pair** — read its `context` block (labels, parent classes) and the per-signal breakdown in `signals`. The structural context tells you whether the pair represents a true match obscured by label difference, a partial overlap that warrants `skos:closeMatch`, or a false positive that needs rejection.
2. **Decide accept / reject** based on the structural and lexical evidence in the conversation, plus any external knowledge you have about the domain.
3. **Call `onto_align_feedback`** for each verdict — this writes to the SQLite feedback table and the self-calibrating-weights model learns from it. Future `onto_align` runs will weight the seven signals better.

This is the MCP-native form of the LogMap-LLM "LLM-as-oracle" pattern (Jiménez-Ruiz et al., EACL 2026, top-2 OAEI 2025 Bio-ML). The server provides the scorer + borderline surface; you do the judging in-conversation; the feedback loop closes via existing tools.

## Architecture Convention: MCP-Native Tool Design

When a tool needs LLM-style judgment (NL generation, semantic matching, accept/reject decisions), the server must NOT embed its own LLM client. The connected orchestrator (you, over MCP) is already a capable LLM. The server's role is to provide:

- **Validation primitives** that the orchestrator can't compute structurally itself (e.g., `onto_shacl_check` verifies proposed SHACL references real classes/properties in the loaded ontology)
- **Scaffolding outputs** that give the orchestrator the schema context it needs to author against (e.g., `onto_stats`, `onto_query` for SPARQL, `borderline` candidates with parent IRIs)
- **Feedback channels** so the orchestrator's verdicts can train the server's self-calibrating models (e.g., `onto_align_feedback`, `onto_lint_feedback`, `onto_enforce_feedback`)

Concrete shape: if a tool description starts with "the server will call an LLM to ..." — flip the design. Have the server return what needs judging; do the judging in the conversation; pipe verdicts back through a feedback tool.

This pattern is the project convention going forward. See `onto_align` borderline buckets (commit a7d3990) and `onto_shacl_check` (commit 9867133) for canonical examples.

## Enforcer Rules (Optional)

If [OpenCheir](https://github.com/fabio-rovai/opencheir) is also connected as an MCP server, its enforcer rules provide workflow safety:

- **onto_validate_after_save** — warns if you save 3+ times without validating
- **onto_version_before_push** — warns if you push without saving a version snapshot first

To enable automatic governance (no Claude orchestration needed), start with the governance webhook:

```bash
# Start OpenCheir first (it listens on port 9900 by default)
opencheir serve &

# Then start Open Ontologies pointing at OpenCheir's enforcer endpoint
GOVERNANCE_WEBHOOK=http://localhost:9900/api/enforcer/event open-ontologies serve
```

Every lineage event (plan, apply, save, push, etc.) is automatically POSTed to OpenCheir's enforcer, which evaluates rules and logs verdicts.

These rules are optional — Open Ontologies works perfectly without OpenCheir.

## Benchmarks

This repo contains reference ontologies and comparison scripts in `benchmark/`. Use them as starting points or to verify the AI-native approach against traditional methods.
