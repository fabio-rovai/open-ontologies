//! The extraction target for Charon and Aeneas.
//!
//! This crate has ONE line of its own. `src/boundary_core.rs` at the repository
//! root is reached with `#[path]`, so the Lean model that comes out of
//! `aeneas/run.sh` is a model of the bytes the shipped engine compiles, not of
//! a transcription of them. Copying the file here would make the whole exercise
//! worthless the first time the two drifted, and drift is what happens.
//!
//! `aeneas/lean/OOBoundary/Generated.lean` is the output. See
//! `docs/aeneas-boundary.md`.
#[path = "../../../src/boundary_core.rs"]
pub mod boundary_core;
