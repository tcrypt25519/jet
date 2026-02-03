/// Unified LLVM type system for Jet EVM JIT compiler.
/// This module defines all LLVM types used by both runtime and contract builders
/// to ensure consistent memory layouts across the system.

use inkwell::{
    AddressSpace,
    context::Context,
    types::{ArrayType, FunctionType, IntType, PointerType, StructType},
};

use crate::constants::*;

const PACK_STRUCTS: bool = true;

/// Unified type registry for LLVM IR generation.
/// This struct holds all LLVM types needed for generating both runtime
/// and contract code, ensuring consistent memory layouts.
pub struct Types<'ctx> {
    // Primitives
    pub i8: IntType<'ctx>,
    pub i32: IntType<'ctx>,
    pub i64: IntType<'ctx>,
    pub i160: IntType<'ctx>,
    pub i256: IntType<'ctx>,
    pub ptr: PointerType<'ctx>,
    pub word_bytes: ArrayType<'ctx>,

    // Architecture
    pub stack: ArrayType<'ctx>,

    // Memory fields (now individual fields instead of struct)
    pub mem_ptr: PointerType<'ctx>,
    pub mem_len: IntType<'ctx>,
    pub mem_cap: IntType<'ctx>,

    // Runtime registers
    pub stack_ptr: IntType<'ctx>,
    pub jump_ptr: IntType<'ctx>,
    pub return_offset: IntType<'ctx>,
    pub return_length: IntType<'ctx>,

    // Complex types
    pub exec_ctx: StructType<'ctx>,
    pub block_info: StructType<'ctx>,
    pub contract_fn: FunctionType<'ctx>,
}

impl<'ctx> Types<'ctx> {
    /// Create a new unified type registry from an LLVM context.
    /// This uses pointer-based memory representation as per layout-mismatch-analysis.md.
    pub fn new(context: &'ctx Context) -> Self {
        // Primitives
        let i8 = context.i8_type();
        let i32 = context.i32_type();
        let i64 = context.i64_type();
        let i160 = context.custom_width_int_type(160);
        let i256 = context.custom_width_int_type(256);
        let ptr = context.ptr_type(AddressSpace::default());
        let word_bytes = i8.array_type(32);

        // Architecture
        let stack = i256.array_type(STACK_SIZE_WORDS);

        // Memory fields - using pointer-based representation
        // as recommended in docs/layout-mismatch-analysis.md
        let mem_ptr = ptr;
        let mem_len = i32;
        let mem_cap = i32;

        // Registers
        let stack_ptr = i32;
        let jump_ptr = i32;
        let return_offset = i32;
        let return_length = i32;

        // Execution context structure with unified memory layout
        // Field order:
        // 0: stack_ptr (i32)
        // 1: jump_ptr (i32)
        // 2: return_offset (i32)
        // 3: return_length (i32)
        // 4: sub_call (ptr)
        // 5: stack ([1024 x i256])
        // 6: memory_ptr (ptr) - changed from struct to individual fields
        // 7: memory_len (i32)
        // 8: memory_cap (i32)
        let exec_ctx = context.struct_type(
            &[
                stack_ptr.into(),
                jump_ptr.into(),
                return_offset.into(),
                return_length.into(),
                ptr.into(),
                stack.into(),
                mem_ptr.into(),    // Changed: now individual ptr field
                mem_len.into(),    // Changed: now at struct level
                mem_cap.into(),    // Changed: now at struct level
            ],
            PACK_STRUCTS,
        );

        // Block information structure
        let block_info = context.struct_type(
            &[
                i64.into(),   // timestamp
                i64.into(),   // number
                i64.into(),   // gaslimit
                i64.into(),   // chainid
                i64.into(),   // selfbalance
                i64.into(),   // basefee
                i64.into(),   // prevrandao
                i256.into(),  // difficulty
                i160.into(),  // coinbase
            ],
            PACK_STRUCTS,
        );

        // Contract function signature: func(ctx: &exec_ctx, block_info: &BlockInfo) -> i8
        let contract_fn = i8.fn_type(&[ptr.into(), ptr.into()], false);

        Self {
            i8,
            i32,
            i64,
            i160,
            i256,
            ptr,
            word_bytes,

            stack,

            mem_ptr,
            mem_len,
            mem_cap,

            stack_ptr,
            jump_ptr,
            return_offset,
            return_length,

            exec_ctx,
            block_info,
            contract_fn,
        }
    }
}
