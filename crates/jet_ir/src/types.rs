use inkwell::{
    AddressSpace,
    context::Context,
    types::{ArrayType, FunctionType, IntType, PointerType, StructType},
};
/// Unified LLVM type system.
/// Defines all LLVM types used by both runtime and contract builders
/// to ensure consistent memory layouts.
use std::num::NonZeroU32;

use crate::constants::*;

const PACK_STRUCTS: bool = true;

/// Unified type registry for LLVM IR generation.
/// This struct holds all LLVM types needed for generating both runtime
/// and contract code, ensuring consistent memory layouts.
pub struct Types<'ctx> {
    // Primitives
    /// 8-bit integer type (`i8`).
    pub i8: IntType<'ctx>,
    /// 32-bit integer type (`i32`).
    pub i32: IntType<'ctx>,
    /// 64-bit integer type (`i64`).
    pub i64: IntType<'ctx>,
    /// 160-bit integer type used for EVM addresses (`i160`).
    pub i160: IntType<'ctx>,
    /// 256-bit integer type used for EVM stack words (`i256`).
    pub i256: IntType<'ctx>,
    /// Opaque pointer type (`ptr`).
    pub ptr: PointerType<'ctx>,
    /// A 32-byte array type (`[32 x i8]`) representing one EVM word in memory.
    pub word_bytes: ArrayType<'ctx>,

    // Architecture
    /// The EVM operand stack: an array of [`STACK_SIZE_WORDS`] 256-bit integers.
    pub stack: ArrayType<'ctx>,

    // Memory fields
    /// Pointer type used for the EVM memory buffer.
    pub mem_ptr: PointerType<'ctx>,
    /// `i32` type used for the accessible EVM memory length.
    pub mem_len: IntType<'ctx>,
    /// `i32` type used for the EVM memory buffer capacity.
    pub mem_cap: IntType<'ctx>,

    // Runtime registers
    /// `i32` type used for the stack depth register.
    pub stack_ptr: IntType<'ctx>,
    /// `i32` type used for the jump-target register.
    pub jump_ptr: IntType<'ctx>,
    /// `i32` type used for the return-data byte offset register.
    pub return_offset: IntType<'ctx>,
    /// `i32` type used for the return-data byte length register.
    pub return_length: IntType<'ctx>,

    // Complex types
    /// The LLVM struct type for [`jet_runtime::exec::Context`].
    pub exec_ctx: StructType<'ctx>,
    /// The LLVM struct type for [`jet_runtime::exec::BlockInfo`].
    pub block_info: StructType<'ctx>,
    /// The LLVM function type for a compiled contract: `fn(*const Context, *const BlockInfo) -> i8`.
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
        let i160 = context
            .custom_width_int_type(NonZeroU32::new(160).expect("160 is non-zero"))
            .expect("i160 is a valid LLVM integer width");
        let i256 = context
            .custom_width_int_type(NonZeroU32::new(256).expect("256 is non-zero"))
            .expect("i256 is a valid LLVM integer width");
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

        let hash_history = word_bytes.array_type(
            BLOCK_HASH_HISTORY_SIZE
                .try_into()
                .expect("block hash history size fits in u32"),
        );
        let address_bytes = i8.array_type(20);

        // Block information structure
        let block_info = context.struct_type(
            &[
                i64.into(),           // number
                i64.into(),           // difficulty
                i64.into(),           // gas_limit
                i64.into(),           // timestamp
                i64.into(),           // base_fee
                i64.into(),           // blob_base_fee
                i64.into(),           // chain_id
                word_bytes.into(),    // hash
                hash_history.into(),  // hash_history
                address_bytes.into(), // coinbase
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
