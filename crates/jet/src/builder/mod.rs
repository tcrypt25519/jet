use inkwell::{builder::BuilderError, support::LLVMString};
use thiserror::Error;

use crate::instructions::Instruction;

/// Compiles a single EVM contract ROM into an LLVM function.
pub mod contract;
/// Build-time configuration and LLVM environment setup.
pub mod env;
/// Manages multiple contract compilations within a shared LLVM module.
pub mod manager;
pub(crate) mod ops;
pub(crate) mod stack;
pub(crate) mod symbolic;

/// An unrecognised opcode byte encountered while translating EVM bytecode.
#[derive(Error, Debug)]
#[error("invalid opcode 0x{opcode:02x} at pc {pc}")]
pub struct InvalidOpcode {
    /// The program counter at which the invalid opcode was found.
    pub pc: usize,
    /// The raw opcode byte that could not be decoded.
    pub opcode: u8,
}

/// Errors that can occur while compiling EVM bytecode to LLVM IR.
#[derive(Error, Debug)]
pub enum Error {
    /// An inkwell/LLVM builder operation failed.
    #[error(transparent)]
    Builder(#[from] BuilderError),
    /// A raw LLVM error string was returned by the LLVM C API.
    #[error(transparent)]
    LLVM(#[from] LLVMString),

    /// The bytecode contained an opcode byte that is not a valid EVM instruction.
    #[error(transparent)]
    InvalidOpcode(#[from] InvalidOpcode),

    /// LLVM IR verification failed after contract compilation.
    #[error("verify error")]
    Verify,

    /// The instruction is valid EVM but has not been implemented in Jet yet.
    #[error("instruction is unimplemented: {}", .0)]
    UnimplementedInstruction(Instruction),

    /// The instruction appeared in a position where it is not expected
    /// (e.g. `JUMPDEST` outside a jump target).
    #[error("instruction is unexpected: {}", .0)]
    UnexpectedInstruction(Instruction),

    /// The opcode byte was not recognised as any EVM instruction.
    #[error("instruction is unknown: {}", .0)]
    UnknownInstruction(u8),

    /// An internal compiler invariant was violated.
    #[error("invariant violation: {}", .0)]
    InvariantViolation(String),

    /// An LLVM integer type was requested with an unsupported bit-width.
    #[error("invalid bit-width: {}", .0)]
    InvalidBitWidth(u32),

    /// A runtime error propagated from [`jet_runtime`].
    #[error(transparent)]
    Runtime(#[from] jet_runtime::RuntimeError),
}

impl Error {
    /// Constructs an [`Error::InvariantViolation`] with the given message.
    pub fn invariant_violation<T: Into<String>>(msg: T) -> Self {
        Error::InvariantViolation(msg.into())
    }
}
