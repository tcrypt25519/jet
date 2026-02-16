//! Shared LLVM IR constants and types for the Jet compiler.
//!
//! This crate is a thin shared library used by both the `jet` compiler crate
//! and the `jet_runtime` crate to ensure that LLVM type definitions and
//! numeric constants are kept in sync.  Consumers import from the crate root;
//! the internal modules are an implementation detail.

mod constants;
mod types;

pub use constants::*;
pub use types::Types;
