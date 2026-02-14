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

    let zero = bctx.env.types().i256.const_zero();
    // EVM spec: division by zero returns 0.
    // Must branch — LLVM udiv with b=0 is poison even inside a select.
    let b_is_zero =
        bctx.builder
            .build_int_compare(inkwell::IntPredicate::EQ, b, zero, "div_by_zero")?;
    build_zero_guard(bctx, b_is_zero, "div", |cont| {
        let result = bctx.builder.build_int_unsigned_div(a, b, "div_result")?;
        stack_push_int(bctx, result)?;
        bctx.builder.build_unconditional_branch(cont)?;
        Ok(())
    })
}

pub(crate) fn sdiv(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;

    let t = bctx.env.types();
    let zero = t.i256.const_zero();

    // Guard 1: b == 0 → result = 0
    let b_is_zero =
        bctx.builder
            .build_int_compare(inkwell::IntPredicate::EQ, b, zero, "sdiv_b_is_zero")?;
    build_zero_guard(bctx, b_is_zero, "sdiv", |cont| {
        // Guard 2: MIN_INT256 / -1 overflows in LLVM sdiv (poison).
        // EVM spec: the result wraps back to MIN_INT256.
        let min_int = t
            .i256
            .const_int_arbitrary_precision(&[0, 0, 0, 0x8000_0000_0000_0000]);
        let neg_one = t.i256.const_all_ones();
        let a_is_min = bctx.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            a,
            min_int,
            "sdiv_a_is_min",
        )?;
        let b_is_neg_one = bctx.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            b,
            neg_one,
            "sdiv_b_is_neg_one",
        )?;
        let is_overflow = bctx
            .builder
            .build_and(a_is_min, b_is_neg_one, "sdiv_is_overflow")?;

        let overflow_block = bctx
            .env
            .context()
            .append_basic_block(bctx.func, "sdiv_overflow");
        let normal_block = bctx
            .env
            .context()
            .append_basic_block(bctx.func, "sdiv_normal");
        bctx.builder
            .build_conditional_branch(is_overflow, overflow_block, normal_block)?;

        bctx.builder.position_at_end(overflow_block);
        stack_push_int(bctx, min_int)?;
        bctx.builder.build_unconditional_branch(cont)?;

        bctx.builder.position_at_end(normal_block);
        let result = bctx.builder.build_int_signed_div(a, b, "sdiv_result")?;
        stack_push_int(bctx, result)?;
        bctx.builder.build_unconditional_branch(cont)?;
        Ok(())
    })
}

pub(crate) fn _mod(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;

    let zero = bctx.env.types().i256.const_zero();
    // EVM spec: modulo by zero returns 0.
    // Must branch — LLVM urem with b=0 is poison even inside a select.
    let b_is_zero =
        bctx.builder
            .build_int_compare(inkwell::IntPredicate::EQ, b, zero, "mod_by_zero")?;
    build_zero_guard(bctx, b_is_zero, "mod", |cont| {
        let result = bctx.builder.build_int_unsigned_rem(a, b, "mod_result")?;
        stack_push_int(bctx, result)?;
        bctx.builder.build_unconditional_branch(cont)?;
        Ok(())
    })
}

pub(crate) fn smod(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;

    let t = bctx.env.types();
    let zero = t.i256.const_zero();

    // Guard 1: b == 0 → result = 0
    let b_is_zero =
        bctx.builder
            .build_int_compare(inkwell::IntPredicate::EQ, b, zero, "smod_b_is_zero")?;
    build_zero_guard(bctx, b_is_zero, "smod", |cont| {
        // Guard 2: MIN_INT256 % -1 — LLVM srem overflows (poison).
        // Mathematically the remainder is 0 (MIN_INT is exactly divisible by -1).
        let min_int = t
            .i256
            .const_int_arbitrary_precision(&[0, 0, 0, 0x8000_0000_0000_0000]);
        let neg_one = t.i256.const_all_ones();
        let a_is_min = bctx.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            a,
            min_int,
            "smod_a_is_min",
        )?;
        let b_is_neg_one = bctx.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            b,
            neg_one,
            "smod_b_is_neg_one",
        )?;
        let is_overflow = bctx
            .builder
            .build_and(a_is_min, b_is_neg_one, "smod_is_overflow")?;

        let overflow_block = bctx
            .env
            .context()
            .append_basic_block(bctx.func, "smod_overflow");
        let normal_block = bctx
            .env
            .context()
            .append_basic_block(bctx.func, "smod_normal");
        bctx.builder
            .build_conditional_branch(is_overflow, overflow_block, normal_block)?;

        bctx.builder.position_at_end(overflow_block);
        stack_push_int(bctx, zero)?;
        bctx.builder.build_unconditional_branch(cont)?;

        bctx.builder.position_at_end(normal_block);
        let result = bctx.builder.build_int_signed_rem(a, b, "smod_result")?;
        stack_push_int(bctx, result)?;
        bctx.builder.build_unconditional_branch(cont)?;
        Ok(())
    })
}

pub(crate) fn addmod(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b, c) = stack_pop_3(bctx)?;

    // Allocate result buffer on stack
    let result_alloca = bctx.builder.build_alloca(bctx.env.types().word_bytes, "addmod_result")?;

    // Call 512-bit addmod builtin for EVM-compliant overflow-safe arithmetic
    bctx.builder.build_call(
        bctx.env.symbols().addmod(),
        &[result_alloca.into(), a.into(), b.into(), c.into()],
        "addmod_call",
    )?;

    stack_push_ptr(bctx, result_alloca)?;
    Ok(())
}

pub(crate) fn mulmod(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b, c) = stack_pop_3(bctx)?;

    // Allocate result buffer on stack
    let result_alloca = bctx.builder.build_alloca(bctx.env.types().word_bytes, "mulmod_result")?;

    // Call 512-bit mulmod builtin for EVM-compliant overflow-safe arithmetic
    bctx.builder.build_call(
        bctx.env.symbols().mulmod(),
        &[result_alloca.into(), a.into(), b.into(), c.into()],
        "mulmod_call",
    )?;

    stack_push_ptr(bctx, result_alloca)?;
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

    // Load index as i256 so the out-of-range check sees the full EVM value.
    let idx = load_i256(bctx, idx)?;

    let t = bctx.env.types();
    let zero = t.i256.const_zero();
    let const_32 = t.i256.const_int(32, false);

    // EVM spec: if i >= 32 the result is 0.
    // Must branch — the GEP below is in-bounds only when idx < 32.
    let is_oob =
        bctx.builder
            .build_int_compare(inkwell::IntPredicate::UGE, idx, const_32, "byte_oob")?;

    let oob_block = bctx.env.context().append_basic_block(bctx.func, "byte_oob");
    let inbounds_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "byte_inbounds");
    let cont = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "byte_cont");
    bctx.builder
        .build_conditional_branch(is_oob, oob_block, inbounds_block)?;

    bctx.builder.position_at_end(oob_block);
    stack_push_int(bctx, zero)?;
    bctx.builder.build_unconditional_branch(cont)?;

    bctx.builder.position_at_end(inbounds_block);
    // Safe to truncate: idx is verified < 32, fits in i32.
    let idx_i32 = bctx
        .builder
        .build_int_truncate(idx, t.i32, "byte_idx_i32")?;
    let const_31 = t.i32.const_int(31, false);
    let idx_reversed = bctx
        .builder
        .build_int_sub(const_31, idx_i32, "byte_idx_reversed")?;
    let typ = t.word_bytes;
    let path = [idx_reversed];
    let byte_ptr = unsafe {
        bctx.builder
            .build_in_bounds_gep(typ, word, &path, "byte_ptr")
    }?;
    let byte = load_i8(bctx, byte_ptr)?;
    stack_push_int(bctx, byte)?;
    bctx.builder.build_unconditional_branch(cont)?;

    bctx.builder.position_at_end(cont);
    Ok(())
}

pub(crate) fn shl(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (shift, value) = stack_pop_2(bctx)?;
    let shift = load_i256(bctx, shift)?;
    let value = load_i256(bctx, value)?;

    let t = bctx.env.types();
    let zero = t.i256.const_zero();
    let max_shift = t.i256.const_int(255, false);

    // EVM spec: shift >= 256 → result = 0.
    // LLVM shl with shift >= 256 is poison, so clamp to 0 first, then select.
    let is_large = bctx.builder.build_int_compare(
        inkwell::IntPredicate::UGT,
        shift,
        max_shift,
        "shl_is_large",
    )?;
    let safe_shift = bctx
        .builder
        .build_select(is_large, zero, shift, "shl_safe_shift")?
        .into_int_value();
    let shifted = bctx
        .builder
        .build_left_shift(value, safe_shift, "shl_shifted")?;
    let result = bctx
        .builder
        .build_select(is_large, zero, shifted, "shl_result")?
        .into_int_value();
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn shr(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (shift, value) = stack_pop_2(bctx)?;
    let shift = load_i256(bctx, shift)?;
    let value = load_i256(bctx, value)?;

    let t = bctx.env.types();
    let zero = t.i256.const_zero();
    let max_shift = t.i256.const_int(255, false);

    // EVM spec: shift >= 256 → result = 0.
    // LLVM lshr with shift >= 256 is poison, so clamp to 0 first, then select.
    let is_large = bctx.builder.build_int_compare(
        inkwell::IntPredicate::UGT,
        shift,
        max_shift,
        "shr_is_large",
    )?;
    let safe_shift = bctx
        .builder
        .build_select(is_large, zero, shift, "shr_safe_shift")?
        .into_int_value();
    let shifted = bctx
        .builder
        .build_right_shift(value, safe_shift, false, "shr_shifted")?;
    let result = bctx
        .builder
        .build_select(is_large, zero, shifted, "shr_result")?
        .into_int_value();
    stack_push_int(bctx, result)?;
    Ok(())
}

pub(crate) fn sar(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (shift, value) = stack_pop_2(bctx)?;
    let shift = load_i256(bctx, shift)?;
    let value = load_i256(bctx, value)?;

    let t = bctx.env.types();
    let max_shift = t.i256.const_int(255, false);

    // EVM spec: shift >= 256 → fill with sign bit (0 or 0xFFFF…FF).
    // Clamp to 255: ashr(value, 255) already produces the correct sign-fill result,
    // so no select is needed — the clamp alone is sufficient.
    let is_large = bctx.builder.build_int_compare(
        inkwell::IntPredicate::UGT,
        shift,
        max_shift,
        "sar_is_large",
    )?;
    let safe_shift = bctx
        .builder
        .build_select(is_large, max_shift, shift, "sar_safe_shift")?
        .into_int_value();
    let result = bctx
        .builder
        .build_right_shift(value, safe_shift, true, "sar_result")?;
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

    // Expand memory if needed (MLOAD reads 32 bytes).
    // This must happen before jet.mem.load so that memory_ptr in the context
    // is up-to-date; jet_mem_expand may reallocate the buffer and update the
    // pointer, and jet.mem.load re-reads it from the context after the call.
    let loc_i32 = load_i32(bctx, loc)?;
    let size = bctx.env.types().i32.const_int(32, false);
    call_mem_expand_checked(bctx, loc_i32, size, "mload")?;

    let mem_value = bctx.builder.build_call(
        bctx.env.symbols().mem_load(),
        &[bctx.registers.exec_ctx.into(), loc.into()],
        "mload",
    )?;

    // mem_load returns the i256 value directly (not a pointer), preventing
    // UAF when memory is reallocated.
    let value = unsafe { IntValue::new(mem_value.as_value_ref()) };
    call_stack_push_i256(bctx, value)?;

    Ok(())
}

pub(crate) fn mstore(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (loc, val) = stack_pop_2(bctx)?;

    let loc_i32 = load_i32(bctx, loc)?;
    let size = bctx.env.types().i32.const_int(32, false);
    call_mem_expand_checked(bctx, loc_i32, size, "mstore")?;

    bctx.builder.build_call(
        bctx.env.symbols().mem_store(),
        &[bctx.registers.exec_ctx.into(), loc.into(), val.into()],
        "mstore",
    )?;
    Ok(())
}

pub(crate) fn mstore8(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (loc, val) = stack_pop_2(bctx)?;

    let loc_i32 = load_i32(bctx, loc)?;
    let size = bctx.env.types().i32.const_int(1, false);
    call_mem_expand_checked(bctx, loc_i32, size, "mstore8")?;

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
    let ptr = call_return_to_ptr(ret);
    
    // Check if stack underflow occurred (null pointer returned)
    let is_null = bctx.builder.build_is_null(ptr, "is_stack_underflow")?;
    
    // Create basic blocks for handling null/non-null cases
    let underflow_block = bctx.env.context().append_basic_block(bctx.func, "stack_underflow");
    let valid_block = bctx.env.context().append_basic_block(bctx.func, "stack_valid");
    
    bctx.builder.build_conditional_branch(is_null, underflow_block, valid_block)?;
    
    // In underflow block, return with StackUnderflow error
    bctx.builder.position_at_end(underflow_block);
    build_return(bctx, ReturnCode::StackUnderflow)?;
    
    // Continue in valid block
    bctx.builder.position_at_end(valid_block);
    
    Ok(ptr)
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

/// Emit jet.mem.expand and check its return value.
///
/// On success (return == 0) the builder is positioned at a new continuation block
/// and this function returns `Ok(())`.  On failure the emitted IR returns
/// `ReturnCode::Invalid` from the contract function directly, so the caller never
/// sees a non-zero result.
fn call_mem_expand_checked(
    bctx: &BuildCtx<'_, '_>,
    loc_i32: IntValue<'_>,
    size: IntValue<'_>,
    label: &str,
) -> Result<(), Error> {
    let ret = bctx.builder.build_call(
        bctx.env.symbols().mem_expand(),
        &[bctx.registers.exec_ctx.into(), loc_i32.into(), size.into()],
        &format!("{label}_expand"),
    )?;

    let ret_i8 = unsafe { IntValue::new(ret.as_value_ref()) };
    let zero = bctx.env.types().i8.const_int(0, false);
    let is_ok = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        ret_i8,
        zero,
        &format!("{label}_expand_ok"),
    )?;

    let ok_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{label}_mem_ok"));
    let err_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{label}_mem_err"));
    bctx.builder
        .build_conditional_branch(is_ok, ok_block, err_block)?;

    bctx.builder.position_at_end(err_block);
    build_return(bctx, ReturnCode::Invalid)?;

    bctx.builder.position_at_end(ok_block);
    Ok(())
}

/// Emit a guarded branch for operations whose denominator/modulus must not be zero.
///
/// Branches on `is_zero_cond`:
/// - **zero path**: pushes `0i256` onto the EVM stack and jumps to the continuation block.
/// - **nonzero path**: calls `nonzero_fn(cont_block)`, which is responsible for computing
///   the result, pushing it, and branching to `cont_block`.
///
/// After this call the builder is positioned at `cont_block`.
fn build_zero_guard<'ctx, F>(
    bctx: &BuildCtx<'ctx, '_>,
    is_zero_cond: IntValue<'ctx>,
    op_name: &str,
    nonzero_fn: F,
) -> Result<(), Error>
where
    F: FnOnce(BasicBlock<'ctx>) -> Result<(), Error>,
{
    let zero = bctx.env.types().i256.const_zero();

    let zero_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{op_name}_zero"));
    let nonzero_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{op_name}_nonzero"));
    let cont = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{op_name}_cont"));

    bctx.builder
        .build_conditional_branch(is_zero_cond, zero_block, nonzero_block)?;

    bctx.builder.position_at_end(zero_block);
    stack_push_int(bctx, zero)?;
    bctx.builder.build_unconditional_branch(cont)?;

    bctx.builder.position_at_end(nonzero_block);
    nonzero_fn(cont)?;

    bctx.builder.position_at_end(cont);
    Ok(())
}
