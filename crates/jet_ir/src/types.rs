/// Unified LLVM type system.
/// Defines all LLVM types used by both runtime and contract builders
/// to ensure consistent memory layouts.

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

    // Memory fields
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
    /// Create a new type registry from an LLVM context.
    /// Uses pointer-based memory representation as per ADR-002.
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

        // Memory fields
        let mem_ptr = ptr;
        let mem_len = i32;
        let mem_cap = i32;

        // Registers
        let stack_ptr = i32;
        let jump_ptr = i32;
        let return_offset = i32;
        let return_length = i32;

        // Execution context structure
        // Field order:
        // 0: stack_ptr (i32)
        // 1: jump_ptr (i32)
        // 2: return_offset (i32)
        // 3: return_length (i32)
        // 4: sub_call (ptr)
        // 5: stack ([1024 x i256])
        // 6: memory_ptr (ptr)
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
                mem_ptr.into(),
                mem_len.into(),
                mem_cap.into(),
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
