use thiserror::Error;

/// Errors that can occur in the Jet runtime.
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Failed to allocate memory: {0}")]
    MemoryAllocation(String),

    #[error("Failed to create memory layout: {0}")]
    MemoryLayout(String),

    #[error("Invalid pointer dereference")]
    NullPointer,

    #[error("Invariant violation: {0}")]
    InvariantViolation(String),
}

pub type Result<T> = std::result::Result<T, RuntimeError>;
