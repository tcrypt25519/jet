pub mod constants;
/// Shared LLVM IR types and constants.
/// This crate contains the unified type system used by both the runtime builder
/// and contract builder to ensure consistent memory layouts.
pub mod types;

pub use constants::*;
pub use types::Types;
