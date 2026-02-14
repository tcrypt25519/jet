use inkwell::{
    basic_block::BasicBlock,
    builder::BuilderError,
    types::IntType,
    values::{AsValueRef, CallSiteValue, IntValue, PointerValue},
};

use jet_runtime::exec::ReturnCode;

use crate::{
    builder::{Error, contract::BuildCtx},
    instructions::Instruction,
};

type StackPop1<'ctx> = PointerValue<'ctx>;
type StackPop2<'ctx> = (PointerValue<'ctx>, PointerValue<'ctx>);
type StackPop3<'ctx> = (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>);
type StackPop7<'ctx> = (
    PointerValue<'ctx>,
    PointerValue<'ctx>,
    PointerValue<'ctx>,
    PointerValue<'ctx>,
    PointerValue<'ctx>,
    PointerValue<'ctx>,
    PointerValue<'ctx>,
);

// OPCode implementations
//

pub(crate) fn build_return(bctx: &BuildCtx<'_, '_>, return_value: ReturnCode) -> Result<(), Error> {
    let return_value = bctx.env.types().i8.const_int(return_value as u64, false);
    bctx.builder.build_return(Some(&return_value))?;
    Ok(())
}

pub(crate) fn push(bctx: &BuildCtx<'_, '_>, bytes: [u8; 32]) -> Result<(), Error> {
    let t = bctx.env.types();

    let values = bytes
        .iter()
        .map(|byte| t.i8.const_int(*byte as u64, false))
        .collect::<Vec<_>>();

    let values = t.i8.const_array(&values);
    let values_ptr = bctx.builder.build_alloca(t.word_bytes, "push_bytes.ptr")?;
    bctx.builder.build_store(values_ptr, values)?;

    stack_push_ptr(bctx, values_ptr)?;

    Ok(())
}

pub(crate) fn dup(bctx: &BuildCtx<'_, '_>, index: u8) -> Result<(), Error> {
    let peeked_value_ptr = call_stack_peek(bctx, index)?;
    call_stack_push_ptr(bctx, peeked_value_ptr)?;
    Ok(())
}

pub(crate) fn swap(bctx: &BuildCtx<'_, '_>, index: u8) -> Result<(), Error> {
    call_stack_swap(bctx, index)?;
    Ok(())
}

pub(crate) fn stop(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    build_return(bctx, ReturnCode::Stop)
}

pub(crate) fn add(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx.builder.build_int_add(a, b, "add_result")?;
    call_stack_push_i256(bctx, result)?;
    Ok(())
}

pub(crate) fn mul(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx.builder.build_int_mul(a, b, "mul_result")?;
    call_stack_push_i256(bctx, result)?;
    Ok(())
}

pub(crate) fn sub(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx.builder.build_int_sub(a, b, "sub_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn div(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;

    // EVM spec: division by zero returns 0
    let zero = bctx.env.types().i256.const_zero();
    let b_is_zero =
        bctx.builder
            .build_int_compare(inkwell::IntPredicate::EQ, b, zero, "b_is_zero")?;

    let div_result = bctx.builder.build_int_unsigned_div(a, b, "div_result")?;
    let result = bctx
        .builder
        .build_select(b_is_zero, zero, div_result, "div_final")?;
    let result = result.into_int_value();

    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn sdiv(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx.builder.build_int_signed_div(a, b, "sdiv_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn _mod(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;

    // EVM spec: modulo by zero returns 0
    let zero = bctx.env.types().i256.const_zero();
    let b_is_zero =
        bctx.builder
            .build_int_compare(inkwell::IntPredicate::EQ, b, zero, "b_is_zero")?;

    let mod_result = bctx.builder.build_int_unsigned_rem(a, b, "mod_result")?;
    let result = bctx
        .builder
        .build_select(b_is_zero, zero, mod_result, "mod_final")?;
    let result = result.into_int_value();

    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn smod(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx.builder.build_int_signed_rem(a, b, "smod_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn addmod(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b, c) = stack_pop_3(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let c = load_i256(bctx, c)?;
    let result = bctx.builder.build_int_add(a, b, "addmod_add_result")?;
    let result = bctx
        .builder
        .build_int_unsigned_rem(result, c, "addmod_mod_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn mulmod(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b, c) = stack_pop_3(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let c = load_i256(bctx, c)?;
    let result = bctx.builder.build_int_mul(a, b, "mulmod_mul_result")?;
    let result = bctx
        .builder
        .build_int_unsigned_rem(result, c, "mulmod_mod_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn exp(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (base, exponent) = stack_pop_2(bctx)?;
    bctx.builder.build_call(
        bctx.env.symbols().exp(),
        &[base.into(), exponent.into()],
        "exp_result",
    )?;
    call_stack_push_ptr(bctx, base)?;
    Ok(())
}

pub(crate) fn signextend(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (b_ptr, x_ptr) = stack_pop_2(bctx)?;
    let b = load_i256(bctx, b_ptr)?;
    let x = load_i256(bctx, x_ptr)?;

    let t = bctx.env.types();
    let const_31 = t.i256.const_int(31, false);
    let const_8 = t.i256.const_int(8, false);
    let const_248 = t.i256.const_int(248, false);

    // in_range = b <= 31; if b >= 32 the result is x unchanged
    let in_range = bctx.builder.build_int_compare(
        inkwell::IntPredicate::ULE,
        b,
        const_31,
        "signextend_in_range",
    )?;

    // Clamp b to 31 to keep shift amount non-negative
    let b_safe = bctx
        .builder
        .build_select(in_range, b, const_31, "signextend_b_safe")?
        .into_int_value();

    // shift = 248 - 8 * b_safe  (moves sign bit at position 8*b+7 to bit 255)
    let b8 = bctx
        .builder
        .build_int_mul(b_safe, const_8, "signextend_b8")?;
    let shift = bctx
        .builder
        .build_int_sub(const_248, b8, "signextend_shift")?;

    // (x << shift) >>arithmetic shift  — sign-extends from the original sign bit
    let shl = bctx.builder.build_left_shift(x, shift, "signextend_shl")?;
    let extended = bctx
        .builder
        .build_right_shift(shl, shift, true, "signextend_sar")?;

    // If b > 31 return x unchanged, otherwise return the sign-extended value
    let result = bctx
        .builder
        .build_select(in_range, extended, x, "signextend_result")?
        .into_int_value();

    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn lt(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx
        .builder
        .build_int_compare(inkwell::IntPredicate::ULT, a, b, "lt_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn gt(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx
        .builder
        .build_int_compare(inkwell::IntPredicate::UGT, a, b, "gt_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn slt(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx
        .builder
        .build_int_compare(inkwell::IntPredicate::SLT, a, b, "slt_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn sgt(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx
        .builder
        .build_int_compare(inkwell::IntPredicate::SGT, a, b, "sgt_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn eq(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx
        .builder
        .build_int_compare(inkwell::IntPredicate::EQ, a, b, "eq_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn iszero(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let a = stack_pop_1(bctx)?;
    let a = load_i256(bctx, a)?;
    let result = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        a,
        bctx.env.types().i256.const_zero(),
        "iszero_result",
    )?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn and(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx.builder.build_and(a, b, "and_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn or(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx.builder.build_or(a, b, "or_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn xor(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;
    let result = bctx.builder.build_xor(a, b, "xor_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn not(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let a = stack_pop_1(bctx)?;
    let a = load_i256(bctx, a)?;
    let result = bctx.builder.build_not(a, "not_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn byte(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (idx, word) = stack_pop_2(bctx)?;

    // Load the index and sub from 31 to reverse endianess
    let idx = load_i32(bctx, idx)?;
    let const_31 = bctx.env.types().i32.const_int(31, false);
    let idx_i32 = bctx.builder.build_int_sub(const_31, idx, "byte_idx")?;

    // GEP into the word array and load the byte
    let typ = bctx.env.types().word_bytes;
    let path = [idx_i32];
    let byte_ptr = unsafe { bctx.builder.build_in_bounds_gep(typ, word, &path, "byte") }?;

    // Load byte and then push as an int instead of pushing as pointer directly, otherwise we'll
    // write 31 bytes of garbage instead of padding.
    let byte = load_i8(bctx, byte_ptr)?;
    stack_push_int(bctx, byte)?;

    Ok(())
}

pub(crate) fn shl(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (shift, value) = stack_pop_2(bctx)?;
    let shift = load_i256(bctx, shift)?;
    let value = load_i256(bctx, value)?;
    let result = bctx.builder.build_left_shift(value, shift, "shl_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn shr(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (shift, value) = stack_pop_2(bctx)?;
    let shift = load_i256(bctx, shift)?;
    let value = load_i256(bctx, value)?;
    let result = bctx
        .builder
        .build_right_shift(value, shift, false, "shr_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn sar(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (shift, value) = stack_pop_2(bctx)?;
    let shift = load_i256(bctx, shift)?;
    let value = load_i256(bctx, value)?;
    let result = bctx
        .builder
        .build_right_shift(value, shift, true, "sar_result")?;
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn keccak256(ctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let data_ptr = stack_pop_1(ctx)?;

    // TODO: Check return code
    ctx.builder.build_call(
        ctx.env.symbols().keccak256(),
        &[data_ptr.into()],
        "keccak256",
    )?;

    // TODO: We could instead simply increase the stack ptr
    call_stack_push_ptr(ctx, data_ptr)?;
    Ok(())
}

pub(crate) fn returndatasize(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    // Load sub call ctx
    let sub_call_ctx_ptr = bctx.builder.build_load(
        bctx.env.types().ptr,
        bctx.registers.sub_call,
        "sub_call_ctx_ptr",
    )?;

    let sub_call_ctx_ptr = unsafe { PointerValue::new(sub_call_ctx_ptr.as_value_ref()) };

    // GetElementPointer to the return length
    let return_length_ptr = bctx.builder.build_struct_gep(
        bctx.env.types().exec_ctx,
        sub_call_ctx_ptr,
        3,
        "return_length_ptr",
    )?;

    let return_length = load_i32(bctx, return_length_ptr)?;

    stack_push_int(bctx, return_length)?;
    Ok(())
}

pub(crate) fn returndatacopy(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (dest_off, src_off, len) = stack_pop_3(bctx)?;

    // Load sub call ctx
    let sub_call_ctx_ptr = bctx.builder.build_load(
        bctx.env.types().ptr,
        bctx.registers.sub_call,
        "sub_call_ctx_ptr",
    )?;

    let sub_call_ctx_ptr = unsafe { PointerValue::new(sub_call_ctx_ptr.as_value_ref()) };

    let dest_off = load_i32(bctx, dest_off)?;
    let src_off = load_i32(bctx, src_off)?;
    let len = load_i32(bctx, len)?;

    // Call the runtime function to copy the return data
    bctx.builder.build_call(
        bctx.env.symbols().contract_call_return_data_copy(),
        &[
            bctx.registers.exec_ctx.into(),
            sub_call_ctx_ptr.into(),
            dest_off.into(),
            src_off.into(),
            len.into(),
        ],
        "return_data_copy",
    )?;

    Ok(())
}

pub(crate) fn blockhash(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    block_info_hash(bctx)?;
    Ok(())
}

pub(crate) fn pop(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    // TODO: We could simply decrement stack ptr
    stack_pop_1(bctx)?;
    Ok(())
}

pub(crate) fn mload(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let loc = stack_pop_1(bctx)?;
    let mem_ptr = bctx.builder.build_call(
        bctx.env.symbols().mem_load(),
        &[bctx.registers.exec_ctx.into(), loc.into()],
        "mload",
    )?;

    let mem_ptr = unsafe { PointerValue::new(mem_ptr.as_value_ref()) };
    stack_push_ptr(bctx, mem_ptr)?;

    Ok(())
}

pub(crate) fn mstore(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (loc, val) = stack_pop_2(bctx)?;

    // Expand memory if needed (MSTORE writes 32 bytes)
    let loc_i32 = load_i32(bctx, loc)?;
    let size = bctx.env.types().i32.const_int(32, false);
    bctx.builder.build_call(
        bctx.env.symbols().mem_expand(),
        &[bctx.registers.exec_ctx.into(), loc_i32.into(), size.into()],
        "mstore_expand",
    )?;

    bctx.builder.build_call(
        bctx.env.symbols().mem_store(),
        &[bctx.registers.exec_ctx.into(), loc.into(), val.into()],
        "mstore",
    )?;
    Ok(())
}

pub(crate) fn mstore8(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (loc, val) = stack_pop_2(bctx)?;

    // Expand memory if needed (MSTORE8 writes 1 byte)
    let loc_i32 = load_i32(bctx, loc)?;
    let size = bctx.env.types().i32.const_int(1, false);
    bctx.builder.build_call(
        bctx.env.symbols().mem_expand(),
        &[bctx.registers.exec_ctx.into(), loc_i32.into(), size.into()],
        "mstore8_expand",
    )?;

    bctx.builder.build_call(
        bctx.env.symbols().mem_store_byte(),
        &[bctx.registers.exec_ctx.into(), loc.into(), val.into()],
        "mstore8",
    )?;
    Ok(())
}

pub(crate) fn jump(bctx: &BuildCtx<'_, '_>, jump_block: BasicBlock) -> Result<(), Error> {
    let pc = stack_pop_1(bctx)?;

    let pc_i32 = load_i32(bctx, pc)?;

    bctx.builder.build_store(bctx.registers.jump_ptr, pc_i32)?;
    bctx.builder.build_unconditional_branch(jump_block)?;
    Ok(())
}

pub(crate) fn jumpi(
    bctx: &BuildCtx<'_, '_>,

    jump_block: BasicBlock,
    jump_else_block: BasicBlock,
) -> Result<(), Error> {
    let (pc, cond) = stack_pop_2(bctx)?;

    let pc = load_i32(bctx, pc)?;
    let cond = load_i64(bctx, cond)?;

    bctx.builder.build_store(bctx.registers.jump_ptr, pc)?;
    let zero = bctx.env.types().i256.const_zero();
    let cmp = bctx
        .builder
        .build_int_compare(inkwell::IntPredicate::EQ, cond, zero, "jumpi_cmp")?;
    bctx.builder
        .build_conditional_branch(cmp, jump_else_block, jump_block)?;
    Ok(())
}

pub(crate) fn pc(bctx: &BuildCtx<'_, '_>, pc: usize) -> Result<(), Error> {
    let pc = bctx.env.types().i256.const_int(pc as u64, false);
    stack_push_int(bctx, pc)?;
    Ok(())
}

pub(crate) fn call(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (_gas, to, _value, _in_off, _in_len, out_off, out_len) = stack_pop_7(bctx)?;

    // Call the contract with the call context
    let contract_call_fn = bctx.env.symbols().contract_call();
    let jit_engine = bctx.env.symbols().jit_engine();
    let jit_engine_ptr = jit_engine.as_pointer_value();
    let make_contract_call = bctx.builder.build_call(
        contract_call_fn,
        &[
            bctx.registers.exec_ctx.into(),
            jit_engine_ptr.into(),
            to.into(),
            out_off.into(),
            out_len.into(),
        ],
        "contract_call",
    )?;

    let ret = unsafe { IntValue::new(make_contract_call.as_value_ref()) };

    stack_push_int(bctx, ret)?;

    Ok(())
}

pub(crate) fn _return(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (offset, size) = stack_pop_2(bctx)?;

    // TODO: Copy instead of load and re-store
    let offset = load_i32(bctx, offset)?;
    let size = load_i32(bctx, size)?;

    bctx.builder
        .build_store(bctx.registers.return_offset, offset)?;
    bctx.builder
        .build_store(bctx.registers.return_length, size)?;

    build_return(bctx, ReturnCode::ExplicitReturn)
}

pub(crate) fn revert(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    build_return(bctx, ReturnCode::Revert)
}

pub(crate) fn invalid(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    build_return(bctx, ReturnCode::Invalid)
}

pub(crate) fn selfdestruct(_bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    Err(Error::UnimplementedInstruction(Instruction::SELFDESTRUCT))
}

// Private helpers
//

fn block_info_hash(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let hash_ptr = bctx.builder.build_struct_gep(
        bctx.env.types().block_info,
        bctx.registers.block_info,
        7,
        "block_info_hash_ptr",
    )?;

    let hash = load_i256(bctx, hash_ptr)?;
    stack_push_int(bctx, hash)?;
    Ok(())
}

fn stack_push_int<'ctx>(bctx: &BuildCtx<'ctx, '_>, value: IntValue<'ctx>) -> Result<(), Error> {
    let bit_width = value.get_type().get_bit_width();
    let value_i256 = match bit_width {
        1 | 8 | 32 => {
            bctx.builder
                .build_int_z_extend(value, bctx.env.types().i256, "int_to_word")?
        }
        256 => value,
        _ => {
            return Err(Error::InvalidBitWidth(bit_width));
        }
    };

    call_stack_push_i256(bctx, value_i256)?;
    Ok(())
}

fn stack_push_ptr<'ctx>(bctx: &BuildCtx<'ctx, '_>, value: PointerValue<'ctx>) -> Result<(), Error> {
    call_stack_push_ptr(bctx, value)?;
    Ok(())
}

fn stack_pop_1<'ctx>(bctx: &BuildCtx<'ctx, '_>) -> Result<StackPop1<'ctx>, Error> {
    let a = call_stack_pop(bctx)?;
    Ok(a)
}

fn stack_pop_2<'ctx>(bctx: &BuildCtx<'ctx, '_>) -> Result<StackPop2<'ctx>, Error> {
    let a = call_stack_pop(bctx)?;
    let b = call_stack_pop(bctx)?;

    Ok((a, b))
}

fn stack_pop_3<'ctx>(bctx: &BuildCtx<'ctx, '_>) -> Result<StackPop3<'ctx>, Error> {
    let a = call_stack_pop(bctx)?;
    let b = call_stack_pop(bctx)?;
    let c = call_stack_pop(bctx)?;

    Ok((a, b, c))
}

fn stack_pop_7<'ctx>(bctx: &BuildCtx<'ctx, '_>) -> Result<StackPop7<'ctx>, Error> {
    let a = call_stack_pop(bctx)?;
    let b = call_stack_pop(bctx)?;
    let c = call_stack_pop(bctx)?;
    let d = call_stack_pop(bctx)?;
    let e = call_stack_pop(bctx)?;
    let f = call_stack_pop(bctx)?;
    let g = call_stack_pop(bctx)?;

    Ok((a, b, c, d, e, f, g))
}

fn call_stack_push_i256<'ctx>(
    bctx: &BuildCtx<'ctx, '_>,
    value: IntValue<'ctx>,
) -> Result<CallSiteValue<'ctx>, BuilderError> {
    bctx.builder.build_call(
        bctx.env.symbols().stack_push_word(),
        &[bctx.registers.exec_ctx.into(), value.into()],
        "stack_push_i256",
    )
}

fn call_stack_push_ptr<'ctx>(
    bctx: &BuildCtx<'ctx, '_>,
    ptr: PointerValue<'ctx>,
) -> Result<CallSiteValue<'ctx>, BuilderError> {
    bctx.builder.build_call(
        bctx.env.symbols().stack_push_ptr(),
        &[bctx.registers.exec_ctx.into(), ptr.into()],
        "stack_push_ptr",
    )
}

fn call_stack_pop<'ctx>(bctx: &BuildCtx<'ctx, '_>) -> Result<PointerValue<'ctx>, Error> {
    let ret = bctx.builder.build_call(
        bctx.env.symbols().stack_pop(),
        &[bctx.registers.exec_ctx.into()],
        "word_ptr",
    )?;
    Ok(call_return_to_ptr(ret))
}

fn call_stack_peek<'ctx>(
    bctx: &BuildCtx<'ctx, '_>,
    index: u8,
) -> Result<PointerValue<'ctx>, Error> {
    let index_value = bctx.env.types().i8.const_int(index as u64, false);
    let ret = bctx.builder.build_call(
        bctx.env.symbols().stack_peek(),
        &[bctx.registers.exec_ctx.into(), index_value.into()],
        "stack_peek_word_result",
    )?;
    Ok(call_return_to_ptr(ret))
}

fn call_stack_swap(bctx: &BuildCtx<'_, '_>, index: u8) -> Result<(), Error> {
    let index_value = bctx.env.types().i8.const_int(index as u64, false);

    bctx.builder.build_call(
        bctx.env.symbols().stack_swap(),
        &[bctx.registers.exec_ctx.into(), index_value.into()],
        "stack_swap_ret",
    )?;
    Ok(())
}

fn load_i8<'a>(bctx: &BuildCtx<'a, '_>, ptr: PointerValue<'a>) -> Result<IntValue<'a>, Error> {
    let int = load_int(bctx, ptr, bctx.env.types().i8)?;
    Ok(int)
}

fn load_i32<'a>(bctx: &BuildCtx<'a, '_>, ptr: PointerValue<'a>) -> Result<IntValue<'a>, Error> {
    let int = load_int(bctx, ptr, bctx.env.types().i32)?;
    Ok(int)
}

fn load_i64<'a>(bctx: &BuildCtx<'a, '_>, ptr: PointerValue<'a>) -> Result<IntValue<'a>, Error> {
    let int = load_int(bctx, ptr, bctx.env.types().i64)?;
    Ok(int)
}

fn load_i256<'a>(bctx: &BuildCtx<'a, '_>, ptr: PointerValue<'a>) -> Result<IntValue<'a>, Error> {
    let int = load_int(bctx, ptr, bctx.env.types().i256)?;
    Ok(int)
}

fn load_int<'a>(
    bctx: &BuildCtx,
    ptr: PointerValue<'a>,
    ty: IntType<'a>,
) -> Result<IntValue<'a>, Error> {
    let value = bctx.builder.build_load(ty, ptr, "load_int")?;
    let value_ref = value.as_value_ref();
    let value_int = unsafe { IntValue::new(value_ref) };
    Ok(value_int)
}

fn call_return_to_ptr(ret: CallSiteValue) -> PointerValue {
    let value_ref = ret.as_value_ref();
    unsafe { PointerValue::new(value_ref) }
}
