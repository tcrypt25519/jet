//! An LLVM-based JIT compiler for EVM smart contracts.
//!
//! `jet` translates EVM bytecode into native machine code using LLVM IR as an
//! intermediate representation. Contracts are compiled on demand and executed
//! via an LLVM JIT execution engine.
//!
//! # Crate layout
//!
//! - [`builder`] — Translates EVM bytecode into LLVM IR.
//! - [`engine`] — High-level entry point; builds and runs contracts end-to-end.
//! - [`instructions`] — EVM opcode definitions and bytecode iterator.

/// Translates EVM bytecode into LLVM IR.
pub mod builder;
/// High-level JIT engine that compiles and executes EVM contracts.
pub mod engine;
/// EVM instruction (opcode) definitions and a bytecode iterator.
pub mod instructions;
