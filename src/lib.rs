pub mod align;
pub mod align_fuzzy;
pub mod batch;
pub mod connect;
pub mod daemon;
pub mod output;
// The pure core of the certificate boundary. PRIVATE on purpose: these are
// the definitions `reason` and `tableaux` call, not a public API, and
// `aeneas/oo-boundary` reaches the file itself with `#[path]` rather than
// through this crate. See `docs/aeneas-boundary.md`.
mod boundary_core;
pub mod borderline_loop;
pub mod buffer;
pub mod cache;
/// Compiled claim verification (Tardygrada Layer 3 hot path).
pub mod claimcheck;
pub mod civex;
#[cfg(feature = "causal-pywhy")]
pub mod civex_pywhy;
pub mod classify_el;
pub mod clinical;
pub mod coevolve;
pub mod communities;
pub mod config;
pub mod cq;
pub mod defects;
pub mod dl_refute;
pub mod dlp;
pub mod drift;
pub mod dynamics;
pub mod dynamics_bcplus;
pub mod eval_alignment;
pub mod eval_rag;
pub mod extract_scaffold;
pub mod flora_pipeline;
pub mod pack;
pub mod support;
pub mod temporal;
pub mod policy;
/// Certified verdicts that cannot be spelled without the evidence. Every
/// module that prints a verdict word takes its vocabulary from here.
pub mod verdict;
pub mod closure_diff;
/// Deductive conservativity of an EXTENSION, under the rule table the engine
/// evaluates and never under a stronger reading than that.
pub mod conservativity;
/// Syntactic locality modules (`⊥`, `⊤`, `⊥⊤*`): a subset of the axioms with a
/// coverage theorem, as opposed to a slice with a measured loss.
pub mod module_extract;
pub mod projection_check;
pub mod projection_entailment;
pub mod shape_combinatorics;
// (re-exports keep the alphabetical ordering of the surrounding modules manageable)
#[cfg(feature = "embeddings")]
pub mod embed;
#[cfg(feature = "embeddings")]
pub mod embed_fingerprint;
#[cfg(feature = "embeddings")]
pub mod embed_remote;
pub mod enforce;
pub mod feedback;
pub mod graph;
#[cfg(feature = "embeddings")]
pub mod hnsw_index;
pub mod ingest;
pub mod inputs;
pub mod kgcl;
pub mod language;
pub mod lineage;
pub mod mapping;
pub mod marketplace;
pub mod monitor;
pub mod ontology;
pub mod ossie;
pub mod plan;
pub mod plan_classical;
pub mod plan_pddl;
pub mod plan_validate;
#[cfg(feature = "plugins")]
pub mod plugins;
#[cfg(feature = "embeddings")]
pub mod poincare;
pub mod reason;
pub mod reason_incremental;
/// Provenance semirings over the derivation DAG: what algebraic expression
/// over the asserted triples a derived triple carries.
pub mod provenance;
/// Axiom pinpointing: the minimal sets of asserted triples responsible for a
/// conclusion, or for a clash. Built over `provenance`'s DAG index.
pub mod justify;
/// Standard rule syntaxes (SWRL, RIF Core) into the Horn rule table
/// `reason::run_horn` evaluates and `lean/`'s `oo-horn` checks.
pub mod rulesyntax;
pub mod registry;
pub mod repo;
pub mod runtime;
pub mod schema;
pub mod segment_retrieve;
pub mod server;
pub mod shacl;
#[cfg(unix)]
pub mod socket;
#[cfg(windows)]
#[path = "socket_windows.rs"]
pub mod socket;
pub mod sql_sync;
pub mod sqlsource;
pub mod vocab_check;
pub mod state;
#[cfg(feature = "embeddings")]
pub mod structembed;
pub mod tableaux;
pub mod toolfilter;
/// First-order export (TPTP FOF and ISO/IEC 24707 CLIF) over the translation
/// owl-lean's machine-checked adequacy theorem is about.
pub mod fol_model;
pub mod fol_solve;
pub mod tptp;
/// Reading a prover's TSTP derivation and re-checking what can be re-checked.
/// Decision 0005's addendum says exactly what this earns and what it does not.
pub mod tstp;
#[cfg(feature = "turbovec")]
pub mod turbo_index;
#[cfg(feature = "embeddings")]
pub mod vecstore;
pub mod webhook;
