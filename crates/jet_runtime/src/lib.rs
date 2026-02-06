pub mod binding;
pub mod builtins;
pub mod exec;
pub mod runtime_builder;
pub mod symbols;

#[cfg(test)]
mod layout_tests;

// System architecture; These are defined by the EVM
pub const WORD_SIZE_BYTES: u32 = 32;
pub const STACK_SIZE_WORDS: u32 = 1024;
pub const ADDRESS_SIZE_BYTES: usize = 2;
pub const BLOCK_HASH_HISTORY_SIZE: usize = 256;

// Runtime sizes; These are defined by the Jet runtime
pub const MEMORY_INITIAL_SIZE_WORDS: u32 = 1024;
pub const STORAGE_INITIAL_SIZE_WORDS: u32 = 1024;
pub const SUB_CALL_RETURN_MAX_SIZE_WORDS: u32 = 1024;

pub use jet_ir::*;
pub use runtime_builder::RuntimeBuilder;
