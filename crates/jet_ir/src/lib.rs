/// Shared LLVM IR types and function registry for Jet EVM JIT compiler.
/// This crate contains the unified type system used by both the runtime builder
/// and contract builder to ensure consistent memory layouts.

pub mod types;
pub mod constants;

pub use types::Types;
pub use constants::*;
