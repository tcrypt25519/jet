//! Runtime support library for the Jet EVM JIT compiler.
//!
//! `jet_runtime` provides the host-side types and extern functions that
//! compiled contract code calls into at run time.  It is linked into every
//! LLVM module produced by the `jet` compiler.
//!
//! # Crate layout
//!
//! - [`address`] — The 20-byte EVM [`Address`] newtype.
//! - [`builtins`] — `extern "C"` functions called from JIT-compiled code.
//! - [`error`] — [`RuntimeError`] and the [`Result`] alias.
//! - [`exec`] — Execution context, block info, return codes and type aliases.
//! - [`runtime_builder`] — Builds the LLVM module that declares all builtins.
//! - [`symbols`] — String constants for every symbol exported to the JIT.

/// 20-byte EVM address newtype.
pub mod address;
mod binding;
/// `extern "C"` builtin functions invoked by JIT-compiled contract code.
pub mod builtins;
/// Runtime error type and [`Result`] alias.
pub mod error;
/// Execution context, block info, type aliases, and return codes.
pub mod exec;
/// Builds the LLVM module that declares all runtime builtins.
pub mod runtime_builder;
/// String constants for every symbol exported to the JIT execution engine.
pub mod symbols;

#[cfg(test)]
mod layout_tests;

pub use address::Address;
pub use error::{Result, RuntimeError};
pub use jet_ir::*;
pub use runtime_builder::RuntimeBuilder;
