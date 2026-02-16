//! EVM and Jet runtime constants shared between the compiler and runtime.

/// Size of a single EVM stack word in bytes.
///
/// Every value on the EVM stack is a 256-bit (32-byte) big-endian integer.
pub const WORD_SIZE_BYTES: u32 = 32;

/// Maximum number of words on the EVM stack.
///
/// The EVM specification defines a stack depth of 1024 words. Pushing beyond
/// this limit is a stack-overflow fault.
pub const STACK_SIZE_WORDS: u32 = 1024;

/// Size of an EVM address in bytes.
///
/// EVM addresses are 160-bit (20-byte) values derived from the last 20 bytes
/// of a Keccak-256 hash of the public key.
pub const ADDRESS_SIZE_BYTES: usize = 20;

/// Number of historical block hashes accessible via the `BLOCKHASH` opcode.
///
/// EVM contracts can query the hash of up to the 256 most recent blocks.
pub const BLOCK_HASH_HISTORY_SIZE: usize = 256;

/// Number of 32-byte words in the initial EVM memory allocation for a contract.
///
/// Memory is expanded lazily; this controls the size of the initial heap buffer
/// allocated before contract execution begins.
pub const MEMORY_INITIAL_SIZE_WORDS: u32 = 1024;

/// Number of 32-byte words in the initial storage allocation.
pub const STORAGE_INITIAL_SIZE_WORDS: u32 = 1024;

/// Maximum size of return data from a sub-call, measured in 32-byte words.
pub const SUB_CALL_RETURN_MAX_SIZE_WORDS: u32 = 1024;
