# Tool reference

Every tool this server registers, with one line each. Generated from the
`#[tool(...)]` attributes in `src/server.rs` and gated by
`tests/tool_reference_test.rs`, so a tool added without a line here fails the
build rather than going unlisted.

The front page does not list these. It shows one loop — `plan`, `apply`,
`drift`, `certify`, `rollback` — because that is what this is for, and a reader
who meets a catalogue first cannot tell what it is for. Everything below keeps
its code and its own documentation; it loses its place above the fold.


121 tools.

| Tool | What it does |
| --- | --- |
| `borderline_partition` | Generalised borderline-pair partitioning (#37, NORA NeurIPS 2025). |
| `borderline_record_verdict` | Persist an orchestrator's verdict on a borderline candidate (#37). |
| `eval_rag` | mmRAG benchmark scoring (#41, ISWC 2025). |
| `eval_rag_mmrag` | Parse a full mmRAG dataset JSON and score it in one call. |
| `graph_projection_entailment_check` | Does a retrieved slice still support the claims an answer rests on? Supply the claims as Turtle (goals_ttl); for each one this reports whether the projection entails it exactly when the source does, under one pinned rule profile, with a machine-checked Lean certificate (OOCert.certificate_sound) for each claim the… |
| `graph_projection_lossy_check` | Audit a projected Turtle slice against the loaded ontology's full neighbourhood of the seed IRIs. |
| `onto_action_applicable` | Evaluate a registered action's SPARQL preconditions against the loaded graph under the given parameter bindings. |
| `onto_action_apply` | Apply a registered action's effects with the given parameter bindings. |
| `onto_action_apply_concurrent` | Fire a tick of concurrent BC+ actions atomically. |
| `onto_action_list` | List the names of all action schemas registered in this server's Dynamics store. |
| `onto_action_register` | Persist a named action schema (Dynamics layer #43). |
| `onto_align` | Detect alignment candidates (owl:equivalentClass, skos:exactMatch, rdfs:subClassOf) between two ontologies using label similarity, property overlap, parent overlap, instance overlap, restriction patterns, and graph neighborhood. |
| `onto_align_feedback` | Accept or reject an alignment candidate to improve future confidence scoring. |
| `onto_align_flora` | End-to-end FLORA alignment (#38). |
| `onto_align_fuzzy` | FLORA-style fuzzy-logic alignment adjudication (#38, ISWC 2025 Best Paper). |
| `onto_apply` | Apply a plan produced by onto_plan, defaulting to the most recent one. |
| `onto_cache_list` | List all cached ontologies with metadata (name, source_path, triple_count, source_mtime, source_size, cache_path, compiled_at, last_access_at) and runtime flags (is_active, in_memory). |
| `onto_cache_remove` | Remove a cached ontology by name. |
| `onto_cache_status` | Inspect the compile cache: active ontology, all cached entries, and the cache configuration (TTL, auto_refresh, dir). |
| `onto_certify_action` | CIVeX-style causal certificate for a proposed state-changing ontology action. |
| `onto_classify_el` | Classify the loaded ontology in the OWL-EL fragment (#30). |
| `onto_clear` | Clear all triples from the in-memory ontology store and unload the active registry slot (cache file is preserved) |
| `onto_closure_diff` | Which CONCLUSIONS a projection preserves, with no goals supplied. |
| `onto_coevolve_dependency_graph` | Build the shape→OWL-dependency map for a SHACL document. |
| `onto_communities` | Detect communities in the loaded entity graph and return a SKELETON per community: size, top members by degree, internal relations, and the bridges connecting it to other communities. |
| `onto_conservative_check` | Does adding these axioms change anything the ontology ALREADY said? Reasons base and base+extension to a fixpoint under one rule table and reports closure(base ∪ extension) minus closure(base), restricted to triples every name of which the base already used. |
| `onto_convert` | Convert an RDF file between formats: turtle, ntriples, rdfxml, nquads, trig |
| `onto_cq_run` | Run a batch of competency questions (CQs) against the loaded ontology (#29). |
| `onto_cq_verdicts_list` | List all stored verdicts for a CQ id, most-recent first. |
| `onto_crosswalk` | Look up clinical crosswalk mappings for a code and system (ICD10, SNOMED, MeSH). |
| `onto_default_apply` | Apply every registered BC+ default-value law whose condition currently holds. |
| `onto_default_register` | Register a BC+ default-value law. |
| `onto_defects` | Check the ONTOLOGY itself, before any data is judged against it. |
| `onto_diff` | Compare two ontology files and show added/removed triples |
| `onto_dl_check` | Check if one class is subsumed by another using DL tableaux reasoning. |
| `onto_dl_explain` | Explain why a class is unsatisfiable using DL tableaux reasoning. |
| `onto_dlp_boundary` | Ask which of YOUR axioms the rule engine can actually SEE, before trusting a reasoning result. |
| `onto_drift` | Detect drift between two ontology versions. |
| `onto_embed` | Generate text + structural Poincaré embeddings for all classes in the loaded ontology. |
| `onto_enforce` | Enforce design patterns on the loaded ontology. |
| `onto_enforce_feedback` | Accept or dismiss an enforce violation to improve future enforce runs. |
| `onto_enrich` | Enrich an ontology class with a SKOS mapping triple from the clinical crosswalks. |
| `onto_eval_alignment` | OAEI-style P/R/F1 scoring (#31). |
| `onto_extend` | Convenience pipeline: ingest data → validate with SHACL → run OWL reasoning, all in one call. |
| `onto_extract_scaffold` | Build a schema-guided structured-extraction scaffold for a class (#28, OntoGPT SPIRES MCP-native). |
| `onto_extract_validate` | Validate an LLM-supplied extraction (JSON array of objects) against a scaffold previously emitted by `onto_extract_scaffold`. |
| `onto_fol_export` | Export the loaded ontology as first-order logic, so it can be handed to the automated-theorem-proving ecosystem. |
| `onto_fol_model` | Find a finite model of the loaded ontology and CHECK IT, so the answer names what it rests on. |
| `onto_fol_prove` | Run a first-order prover, READ the derivation it prints back, and RE-CHECK what can honestly be re-checked. |
| `onto_history` | List all saved ontology version snapshots |
| `onto_hnsw_build` | Build (or rebuild) the HNSW cosine index over the loaded text embeddings with explicit `ef_construction` and `ef_search` parameters. |
| `onto_import` | Resolve and load all owl:imports from the currently loaded ontology |
| `onto_import_schema` | Import a relational database schema as an OWL ontology. |
| `onto_induce` | ONE SHEET IN, ONE ONTOLOGY OUT: induce an OWL class, typed properties, a SHACL shape and a loading mapping from one data sheet, with the evidence for every line. |
| `onto_ingest` | Parse a structured data file (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet) into RDF triples and load into the ontology store. |
| `onto_invariant_check` | Evaluate every registered BC+ invariant against the current graph and return the names + descriptions of any that fail. |
| `onto_invariant_list` | List all registered BC+ static causal laws (invariants). |
| `onto_invariant_register` | Persist a BC+ static causal law (SPARQL ASK invariant). |
| `onto_invariant_remove` | Remove a registered BC+ invariant by name. |
| `onto_justify` | AXIOM PINPOINTING: which ASSERTED triples are responsible for a conclusion, or for a contradiction. |
| `onto_lineage` | Get the compact lineage log for the current or specified session. |
| `onto_lint` | Check an ontology for quality issues: missing labels, comments, domains, ranges |
| `onto_lint_feedback` | Accept or dismiss a lint issue to improve future lint runs. |
| `onto_load` | Load an RDF file or inline Turtle content into the in-memory ontology store. |
| `onto_lock` | Lock IRIs to prevent removal during plan/apply. |
| `onto_map` | Generate a mapping config by inspecting a data file's schema against the currently loaded ontology. |
| `onto_marketplace` | Browse and install ontologies from the curated catalogue of 33 W3C/ISO/industry standards plus the open community-pack registry (community/registry.json, fetched at runtime; override with OPEN_ONTOLOGIES_COMMUNITY_REGISTRY). |
| `onto_module_extract` | A MODULE over a signature, not a slice: the smallest subset of the axioms syntactic locality can justify, such that every entailment of the WHOLE ontology over those IRIs is still an entailment of the subset. |
| `onto_monitor` | Run active monitoring watchers. |
| `onto_monitor_clear` | Clear the monitor blocked flag, allowing apply operations to proceed. |
| `onto_ossie_import` | Compile an Apache Ossie (incubating, formerly Open Semantic Interchange) ontology document into OWL 2 DL + SHACL, and optionally load it into the active store so every other tool here works on it. |
| `onto_owl_shacl_coevolve_check` | Combined OWL+SHACL validation (#33, K-CAP 2025). |
| `onto_owl_shacl_coevolve_incremental` | Incremental coevolve check (#33 follow-on, K-CAP 2025). |
| `onto_pack` | Write the loaded graph and its verification evidence to a portable, versioned pack: sorted N-Triples plus a manifest (name, version, counts, timestamp, sha256, and the lint/enforce results recorded at pack time). |
| `onto_plan` | Terraform-style plan: diff current store against proposed Turtle. |
| `onto_plan_classical` | Invoke Fast Downward as a subprocess on a precompiled PDDL domain + problem (#50). |
| `onto_plan_compile_pddl` | Compile a PDDL domain from registered Dynamics action schemas (#43) plus a problem instance from the loaded graph and a goal Turtle slice (#45 Planner stub). |
| `onto_plan_validate` | Validate a candidate plan (sequence of registered-action steps) against the loaded graph WITHOUT mutating the real store. |
| `onto_plugin_call` | Invoke a tool on an installed WASM plugin. |
| `onto_plugin_list` | Discover installed WASM plugins and the tools they provide. |
| `onto_policy_check` | Evaluate a proposed action's target IRIs against every registered policy rule. |
| `onto_policy_list` | List all registered ARGOS policy rules. |
| `onto_policy_register` | Register an ARGOS-style policy rule (#40, ISWC 2025 WOP). |
| `onto_provenance` | PROVENANCE SEMIRINGS: the algebraic expression over the ASSERTED triples that a derived triple carries. |
| `onto_pull` | Fetch an ontology from a remote URL or SPARQL endpoint and load it into the store |
| `onto_push` | Push the current ontology store to a remote SPARQL endpoint |
| `onto_query` | Run a SPARQL query against the loaded ontology store. |
| `onto_reason` | Run inference over the loaded ontology. |
| `onto_reason_incremental` | Derive the consequences of newly added triples WITHOUT recomputing the whole closure. |
| `onto_recompile` | Force-recompile an ontology from its source file, ignoring the on-disk cache. |
| `onto_repo_list` | List RDF/OWL files in the configured ontology repository directories ([general] ontology_dirs). |
| `onto_repo_load` | Load an ontology from one of the configured repository directories ([general] ontology_dirs) into the active store. |
| `onto_rollback` | Restore the ontology store to a previously saved version |
| `onto_rules_import` | Read rules written in a STANDARD rule syntax into the Horn rule table `onto_reason` evaluates with `rules_file` and the proved-sound Lean checker verifies with `lake exe oo-horn check`. |
| `onto_save` | Save the current ontology store to a file |
| `onto_search` | Semantic search over the loaded ontology using natural language. |
| `onto_segment_retrieve` | Retrieve a TBox-slice neighbourhood of seed IRIs for grounding LLM reasoning (#34, SEMANTiCS 2025 GrOWL-RAG). |
| `onto_shacl` | Validate the loaded ontology data against SHACL shapes. |
| `onto_shacl_check` | Dry-run structural check on proposed SHACL shapes against the loaded ontology. |
| `onto_shape_combinatorics` | Enumerate the property-combination lattice for a class (#36, K-CAP 2025 Kastor). |
| `onto_shape_induce` | Kastor-style data-driven SHACL shape induction (#36, K-CAP 2025). |
| `onto_similarity` | Compute embedding similarity between two IRIs — returns cosine similarity (text), Poincaré distance (structural), and product score. |
| `onto_sql_ingest` | Run a SQL query against a relational backbone (PostgreSQL or DuckDB) and ingest the resulting rows into the triple store as RDF. |
| `onto_sql_sync_reset` | Clear the recorded CDC watermark for a sync_key. |
| `onto_sql_sync_state` | Read the recorded CDC watermark for a sync_key. |
| `onto_sql_sync_states_list` | List every recorded CDC sync state across all sync_keys. |
| `onto_stats` | Get statistics about the loaded ontology (triple count, classes, properties, individuals) |
| `onto_status` | Returns health status of the Open Ontologies server |
| `onto_support_check` | Check whether the graph's claims are backed by the sources they cite. |
| `onto_support_report` | Summarise provenance quality: the unsourced-claim rate over the whole graph, and the support rate across the claims judged so far. |
| `onto_support_verdict` | Record whether a cited source supports a claim: supported, refuted, or unrelated. |
| `onto_temporal_conflicts` | Disjointness violations that claim OVERLAPPING validity, separated from those that do not. |
| `onto_temporal_query` | Run a SPARQL graph pattern against only the graphs in temporal scope: what the graph said at a given valid time, as known at a given recorded time. |
| `onto_temporal_snapshot` | Which named graphs are in scope at a point in time, and which are excluded and why. |
| `onto_unload` | Unload an ontology from memory. |
| `onto_unpack` | Load a pack written by onto_pack, refusing it if the checksum does not match. |
| `onto_validate` | Validate RDF/OWL syntax. |
| `onto_validate_clinical` | Validate all class labels in the loaded ontology against clinical crosswalk data. |
| `onto_verify_cq` | Persist an LLM-supplied (or human-supplied) verdict on a CQ result (#39, ISWC 2025 Lippolis). |
| `onto_version` | Save a named snapshot of the current ontology store |
| `onto_vocab_check` | Closed-world vocabulary check on a Turtle DATA graph: verify that every predicate and every rdf:type class used in the data is actually DECLARED in the loaded ontology. |
