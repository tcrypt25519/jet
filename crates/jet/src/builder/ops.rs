use std::num::NonZeroU32;

use inkwell::{
    basic_block::BasicBlock,
    builder::BuilderError,
    values::{AsValueRef, CallSiteValue, IntValue, PointerValue},
};

use jet_runtime::exec::ReturnCode;

use crate::{
    builder::{Error, contract::BuildCtx, stack::StackBackend},
    instructions::Instruction,
};

pub(crate) fn build_return<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    return_value: ReturnCode,
) -> Result<(), Error> {
    bctx.stack.materialize_for_return(bctx)?;
    let return_value = bctx.env.types().i8.const_int(return_value as u64, false);
    bctx.builder.build_return(Some(&return_value))?;
    Ok(())
}

pub(crate) fn push<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    bytes: [u8; 32],
) -> Result<(), Error> {
    let mut limbs = [0u64; 4];
    for (i, chunk) in bytes.chunks_exact(8).enumerate() {
        let chunk: [u8; 8] = chunk
            .try_into()
            .map_err(|_| Error::invariant_violation("invalid PUSH limb width"))?;
        limbs[i] = u64::from_le_bytes(chunk);
    }
    let value = bctx.env.types().i256.const_int_arbitrary_precision(&limbs);
    let known_u64 = limbs[1..].iter().all(|limb| *limb == 0).then_some(limbs[0]);
    bctx.stack.push_word_with_known_u64(bctx, value, known_u64)
}

pub(crate) fn dup<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    index: u8,
) -> Result<(), Error> {
    bctx.stack.dup(bctx, index)
}

pub(crate) fn swap<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    index: u8,
) -> Result<(), Error> {
    bctx.stack.swap(bctx, index)
}

pub(crate) fn stop<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    build_return(bctx, ReturnCode::Stop)
}

pub(crate) fn add<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = bctx.builder.build_int_add(a, b, "add_result")?;
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn mul<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = bctx.builder.build_int_mul(a, b, "mul_result")?;
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn sub<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = bctx.builder.build_int_sub(a, b, "sub_result")?;
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn div<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = build_zero_guarded_value(bctx, b, "div", |bctx| {
        bctx.builder.build_int_unsigned_div(a, b, "div_result")
    })?;
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn sdiv<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = build_signed_div_or_rem_value(bctx, a, b, "sdiv", true)?;
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn _mod<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = build_zero_guarded_value(bctx, b, "mod", |bctx| {
        bctx.builder.build_int_unsigned_rem(a, b, "mod_result")
    })?;
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn smod<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = build_signed_div_or_rem_value(bctx, a, b, "smod", false)?;
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn addmod<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let (a, b, c) = bctx.stack.pop_3(bctx)?;
    let result = build_wide_mod_op(bctx, a, b, c, "addmod", |lhs, rhs| {
        bctx.builder.build_int_add(lhs, rhs, "addmod_wide_sum")
    })?;
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn mulmod<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let (a, b, c) = bctx.stack.pop_3(bctx)?;
    let result = build_wide_mod_op(bctx, a, b, c, "mulmod", |lhs, rhs| {
        bctx.builder.build_int_mul(lhs, rhs, "mulmod_wide_product")
    })?;
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn exp<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (base, exponent) = bctx.stack.pop_2(bctx)?;
    let result = build_exp_value(bctx, base, exponent)?;
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn signextend<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let (b, x) = bctx.stack.pop_2(bctx)?;
    let t = bctx.env.types();
    let const_31 = t.i256.const_int(31, false);
    let const_8 = t.i256.const_int(8, false);
    let const_248 = t.i256.const_int(248, false);
    let in_range = bctx.builder.build_int_compare(
        inkwell::IntPredicate::ULE,
        b,
        const_31,
        "signextend_in_range",
    )?;
    let b_safe = bctx
        .builder
        .build_select(in_range, b, const_31, "signextend_b_safe")?
        .into_int_value();
    let b8 = bctx
        .builder
        .build_int_mul(b_safe, const_8, "signextend_b8")?;
    let shift = bctx
        .builder
        .build_int_sub(const_248, b8, "signextend_shift")?;
    let shl = bctx.builder.build_left_shift(x, shift, "signextend_shl")?;
    let extended = bctx
        .builder
        .build_right_shift(shl, shift, true, "signextend_sar")?;
    let result = bctx
        .builder
        .build_select(in_range, extended, x, "signextend_result")?
        .into_int_value();
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn lt<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    compare_2(bctx, inkwell::IntPredicate::ULT, "lt_result")
}

pub(crate) fn gt<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    compare_2(bctx, inkwell::IntPredicate::UGT, "gt_result")
}

pub(crate) fn slt<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    compare_2(bctx, inkwell::IntPredicate::SLT, "slt_result")
}

pub(crate) fn sgt<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    compare_2(bctx, inkwell::IntPredicate::SGT, "sgt_result")
}

pub(crate) fn eq<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    compare_2(bctx, inkwell::IntPredicate::EQ, "eq_result")
}

pub(crate) fn iszero<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let a = bctx.stack.pop_word(bctx)?;
    let result = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        a,
        bctx.env.types().i256.const_zero(),
        "iszero_result",
    )?;
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn and<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    bitwise_2(bctx, "and_result", |bctx, a, b| {
        bctx.builder.build_and(a, b, "and_result")
    })
}

pub(crate) fn or<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    bitwise_2(bctx, "or_result", |bctx, a, b| {
        bctx.builder.build_or(a, b, "or_result")
    })
}

pub(crate) fn xor<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    bitwise_2(bctx, "xor_result", |bctx, a, b| {
        bctx.builder.build_xor(a, b, "xor_result")
    })
}

pub(crate) fn not<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let a = bctx.stack.pop_word(bctx)?;
    let result = bctx.builder.build_not(a, "not_result")?;
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn byte<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (idx, word) = bctx.stack.pop_2(bctx)?;
    let t = bctx.env.types();
    let zero = t.i256.const_zero();
    let const_32 = t.i256.const_int(32, false);
    let is_oob =
        bctx.builder
            .build_int_compare(inkwell::IntPredicate::UGE, idx, const_32, "byte_oob")?;
    let idx_i32 = bctx
        .builder
        .build_int_truncate(idx, t.i32, "byte_idx_i32")?;
    let const_31 = t.i32.const_int(31, false);
    let idx_reversed = bctx
        .builder
        .build_int_sub(const_31, idx_i32, "byte_idx_reversed")?;
    let shift =
        bctx.builder
            .build_int_mul(idx_reversed, t.i32.const_int(8, false), "byte_shift")?;
    let shift = bctx
        .builder
        .build_int_z_extend(shift, t.i256, "byte_shift_i256")?;
    let shifted = bctx
        .builder
        .build_right_shift(word, shift, false, "byte_shifted")?;
    let byte = bctx
        .builder
        .build_int_truncate(shifted, t.i8, "byte_value")?;
    let byte = bctx
        .builder
        .build_int_z_extend(byte, t.i256, "byte_value_i256")?;
    let result = bctx
        .builder
        .build_select(is_oob, zero, byte, "byte_result")?
        .into_int_value();
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn shl<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    shift_left_or_right(bctx, true, false, "shl")
}

pub(crate) fn shr<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    shift_left_or_right(bctx, false, false, "shr")
}

pub(crate) fn sar<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (shift, value) = bctx.stack.pop_2(bctx)?;
    let t = bctx.env.types();
    let max_shift = t.i256.const_int(255, false);
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
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn keccak256<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let (offset, size) = bctx.stack.pop_2(bctx)?;
    let offset_i32 = truncate_to_i32(bctx, offset, "keccak_offset")?;
    let size_i32 = truncate_to_i32(bctx, size, "keccak_size")?;
    call_mem_expand_checked(bctx, offset_i32, size_i32, "keccak256")?;
    let result_ptr = bctx
        .builder
        .build_alloca(bctx.env.types().i256, "keccak_result")?;
    let ret = bctx.builder.build_call(
        bctx.env.symbols().keccak256(),
        &[
            bctx.registers.exec_ctx.into(),
            offset_i32.into(),
            size_i32.into(),
            result_ptr.into(),
        ],
        "keccak256",
    )?;
    branch_on_i8_success(bctx, ret, "keccak256")?;
    let result = bctx
        .builder
        .build_load(bctx.env.types().i256, result_ptr, "keccak_result")?
        .into_int_value();
    bctx.stack.push_word(bctx, result)
}

pub(crate) fn returndatasize<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let sub_call_ctx_ptr = bctx.builder.build_load(
        bctx.env.types().ptr,
        bctx.registers.sub_call,
        "sub_call_ctx_ptr",
    )?;
    let sub_call_ctx_ptr = unsafe { PointerValue::new(sub_call_ctx_ptr.as_value_ref()) };
    let return_length_ptr = bctx.builder.build_struct_gep(
        bctx.env.types().exec_ctx,
        sub_call_ctx_ptr,
        3,
        "return_length_ptr",
    )?;
    let return_length = bctx
        .builder
        .build_load(bctx.env.types().i32, return_length_ptr, "return_length")?
        .into_int_value();
    bctx.stack.push_word(bctx, return_length)
}

pub(crate) fn returndatacopy<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let (dest_off, src_off, len) = bctx.stack.pop_3(bctx)?;
    let sub_call_ctx_ptr = bctx.builder.build_load(
        bctx.env.types().ptr,
        bctx.registers.sub_call,
        "sub_call_ctx_ptr",
    )?;
    let sub_call_ctx_ptr = unsafe { PointerValue::new(sub_call_ctx_ptr.as_value_ref()) };
    let dest_off = truncate_to_i32(bctx, dest_off, "retcopy_dest")?;
    let src_off = truncate_to_i32(bctx, src_off, "retcopy_src")?;
    let len = truncate_to_i32(bctx, len, "retcopy_len")?;
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

pub(crate) fn blockhash<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    block_info_hash(bctx)
}

pub(crate) fn pop<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let _ = bctx.stack.pop_word(bctx)?;
    Ok(())
}

pub(crate) fn mload<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let loc = bctx.stack.pop_word(bctx)?;
    let loc_i32 = truncate_to_i32(bctx, loc, "mload_loc")?;
    let size = bctx.env.types().i32.const_int(32, false);
    call_mem_expand_checked(bctx, loc_i32, size, "mload")?;
    let value = build_mem_load_value(bctx, loc_i32)?;
    bctx.stack.push_word(bctx, value)
}

pub(crate) fn mstore<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let (loc, val) = bctx.stack.pop_2(bctx)?;
    let loc_i32 = truncate_to_i32(bctx, loc, "mstore_loc")?;
    let size = bctx.env.types().i32.const_int(32, false);
    call_mem_expand_checked(bctx, loc_i32, size, "mstore")?;
    build_mem_store_value(bctx, loc_i32, val)
}

pub(crate) fn mstore8<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let (loc, val) = bctx.stack.pop_2(bctx)?;
    let loc_i32 = truncate_to_i32(bctx, loc, "mstore8_loc")?;
    let size = bctx.env.types().i32.const_int(1, false);
    call_mem_expand_checked(bctx, loc_i32, size, "mstore8")?;
    build_mem_store_byte_value(bctx, loc_i32, val)
}

pub(crate) fn jump<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    jump_block: BasicBlock<'ctx>,
) -> Result<(), Error> {
    let pc = bctx.stack.pop_word(bctx)?;
    let pc = truncate_to_i32(bctx, pc, "jump_pc")?;
    bctx.builder.build_store(bctx.registers.jump_ptr, pc)?;
    bctx.builder.build_unconditional_branch(jump_block)?;
    Ok(())
}

pub(crate) fn jumpi<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    jump_block: BasicBlock<'ctx>,
    jump_else_block: BasicBlock<'ctx>,
) -> Result<(), Error> {
    let (pc, cond) = bctx.stack.pop_2(bctx)?;
    let pc = truncate_to_i32(bctx, pc, "jumpi_pc")?;
    bctx.builder.build_store(bctx.registers.jump_ptr, pc)?;
    let cmp = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        cond,
        bctx.env.types().i256.const_zero(),
        "jumpi_cmp",
    )?;
    bctx.builder
        .build_conditional_branch(cmp, jump_else_block, jump_block)?;
    Ok(())
}

pub(crate) fn pc<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    pc: usize,
) -> Result<(), Error> {
    let pc_value = bctx.env.types().i256.const_int(pc as u64, false);
    bctx.stack
        .push_word_with_known_u64(bctx, pc_value, Some(pc as u64))
}

pub(crate) fn call<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (_gas, to, _value, _in_off, _in_len, out_off, out_len) = bctx.stack.pop_7(bctx)?;
    let jit_engine = bctx.env.symbols().jit_engine();
    let jit_engine_ptr = jit_engine.as_pointer_value();
    let (addr_lo, addr_mid, addr_hi) = build_address_limbs(bctx, to)?;
    let out_off = truncate_to_i32(bctx, out_off, "call.out_off")?;
    let out_len = truncate_to_i32(bctx, out_len, "call.out_len")?;
    let make_contract_call = bctx.builder.build_call(
        bctx.env.symbols().contract_call_values(),
        &[
            bctx.registers.exec_ctx.into(),
            jit_engine_ptr.into(),
            addr_lo.into(),
            addr_mid.into(),
            addr_hi.into(),
            out_off.into(),
            out_len.into(),
        ],
        "contract_call",
    )?;
    let ret = unsafe { IntValue::new(make_contract_call.as_value_ref()) };
    bctx.stack.push_word(bctx, ret)
}

pub(crate) fn _return<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let (offset, size) = bctx.stack.pop_2(bctx)?;
    let offset = truncate_to_i32(bctx, offset, "return_offset")?;
    let size = truncate_to_i32(bctx, size, "return_length")?;
    bctx.builder
        .build_store(bctx.registers.return_offset, offset)?;
    bctx.builder
        .build_store(bctx.registers.return_length, size)?;
    build_return(bctx, ReturnCode::ExplicitReturn)
}

pub(crate) fn revert<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    build_return(bctx, ReturnCode::Revert)
}

pub(crate) fn invalid<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    build_return(bctx, ReturnCode::Invalid)
}

pub(crate) fn selfdestruct<'ctx, S: StackBackend<'ctx>>(
    _bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    Err(Error::UnimplementedInstruction(Instruction::SELFDESTRUCT))
}

fn compare_2<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    predicate: inkwell::IntPredicate,
    name: &str,
) -> Result<(), Error> {
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = bctx.builder.build_int_compare(predicate, a, b, name)?;
    bctx.stack.push_word(bctx, result)
}

fn bitwise_2<'ctx, S, F>(bctx: &BuildCtx<'ctx, '_, S>, _name: &str, op: F) -> Result<(), Error>
where
    S: StackBackend<'ctx>,
    F: FnOnce(
        &BuildCtx<'ctx, '_, S>,
        IntValue<'ctx>,
        IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, BuilderError>,
{
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = op(bctx, a, b)?;
    bctx.stack.push_word(bctx, result)
}

fn normalize_to_i256<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    value: IntValue<'ctx>,
) -> Result<IntValue<'ctx>, Error> {
    let bit_width = value.get_type().get_bit_width();
    match bit_width {
        1 | 8 | 32 => {
            Ok(bctx
                .builder
                .build_int_z_extend(value, bctx.env.types().i256, "int_to_word")?)
        }
        256 => Ok(value),
        _ => Err(Error::InvalidBitWidth(bit_width)),
    }
}

fn block_info_hash<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let hash_ptr = bctx.builder.build_struct_gep(
        bctx.env.types().block_info,
        bctx.registers.block_info,
        7,
        "block_info_hash_ptr",
    )?;
    let hash = bctx
        .builder
        .build_load(bctx.env.types().i256, hash_ptr, "block_info_hash")?
        .into_int_value();
    bctx.stack.push_word(bctx, hash)
}

fn build_zero_guarded_value<'ctx, S, F>(
    bctx: &BuildCtx<'ctx, '_, S>,
    divisor: IntValue<'ctx>,
    name: &str,
    nonzero_fn: F,
) -> Result<IntValue<'ctx>, Error>
where
    S: StackBackend<'ctx>,
    F: FnOnce(&BuildCtx<'ctx, '_, S>) -> Result<IntValue<'ctx>, BuilderError>,
{
    let zero = bctx.env.types().i256.const_zero();
    let is_zero = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        divisor,
        zero,
        &format!("{name}_by_zero"),
    )?;
    let zero_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_zero"));
    let nonzero_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_nonzero"));
    let cont = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_cont"));
    bctx.builder
        .build_conditional_branch(is_zero, zero_block, nonzero_block)?;

    bctx.builder.position_at_end(zero_block);
    bctx.builder.build_unconditional_branch(cont)?;

    bctx.builder.position_at_end(nonzero_block);
    let result = nonzero_fn(bctx)?;
    bctx.builder.build_unconditional_branch(cont)?;

    bctx.builder.position_at_end(cont);
    let phi = bctx
        .builder
        .build_phi(bctx.env.types().i256, &format!("{name}_phi"))?;
    phi.add_incoming(&[(&zero, zero_block), (&result, nonzero_block)]);
    Ok(phi.as_basic_value().into_int_value())
}

fn build_signed_div_or_rem_value<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    a: IntValue<'ctx>,
    b: IntValue<'ctx>,
    name: &str,
    is_div: bool,
) -> Result<IntValue<'ctx>, Error> {
    let t = bctx.env.types();
    let zero = t.i256.const_zero();
    let b_is_zero = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        b,
        zero,
        &format!("{name}_b_is_zero"),
    )?;
    let zero_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_zero"));
    let nonzero_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_nonzero"));
    let inner_cont = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_inner_cont"));
    let outer_cont = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_cont"));
    bctx.builder
        .build_conditional_branch(b_is_zero, zero_block, nonzero_block)?;

    bctx.builder.position_at_end(zero_block);
    bctx.builder.build_unconditional_branch(outer_cont)?;

    bctx.builder.position_at_end(nonzero_block);
    let min_int = t
        .i256
        .const_int_arbitrary_precision(&[0, 0, 0, 0x8000_0000_0000_0000]);
    let neg_one = t.i256.const_all_ones();
    let a_is_min = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        a,
        min_int,
        &format!("{name}_a_is_min"),
    )?;
    let b_is_neg_one = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        b,
        neg_one,
        &format!("{name}_b_is_neg_one"),
    )?;
    let is_overflow =
        bctx.builder
            .build_and(a_is_min, b_is_neg_one, &format!("{name}_is_overflow"))?;
    let overflow_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_overflow"));
    let normal_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_normal"));
    bctx.builder
        .build_conditional_branch(is_overflow, overflow_block, normal_block)?;

    bctx.builder.position_at_end(overflow_block);
    bctx.builder.build_unconditional_branch(inner_cont)?;

    bctx.builder.position_at_end(normal_block);
    let normal_result = if is_div {
        bctx.builder
            .build_int_signed_div(a, b, &format!("{name}_result"))?
    } else {
        bctx.builder
            .build_int_signed_rem(a, b, &format!("{name}_result"))?
    };
    bctx.builder.build_unconditional_branch(inner_cont)?;

    bctx.builder.position_at_end(inner_cont);
    let overflow_result = if is_div { min_int } else { zero };
    let inner_phi = bctx
        .builder
        .build_phi(bctx.env.types().i256, &format!("{name}_inner_phi"))?;
    inner_phi.add_incoming(&[
        (&overflow_result, overflow_block),
        (&normal_result, normal_block),
    ]);
    let inner_result = inner_phi.as_basic_value().into_int_value();
    bctx.builder.build_unconditional_branch(outer_cont)?;

    bctx.builder.position_at_end(outer_cont);
    let outer_phi = bctx
        .builder
        .build_phi(bctx.env.types().i256, &format!("{name}_result_phi"))?;
    outer_phi.add_incoming(&[(&zero, zero_block), (&inner_result, inner_cont)]);
    Ok(outer_phi.as_basic_value().into_int_value())
}

fn build_wide_mod_op<'ctx, S, F>(
    bctx: &BuildCtx<'ctx, '_, S>,
    a: IntValue<'ctx>,
    b: IntValue<'ctx>,
    modulus: IntValue<'ctx>,
    name: &str,
    op: F,
) -> Result<IntValue<'ctx>, Error>
where
    S: StackBackend<'ctx>,
    F: FnOnce(IntValue<'ctx>, IntValue<'ctx>) -> Result<IntValue<'ctx>, BuilderError>,
{
    let t = bctx.env.types();
    let zero = t.i256.const_zero();
    let modulus_is_zero = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        modulus,
        zero,
        &format!("{name}_modulus_is_zero"),
    )?;
    let zero_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_zero"));
    let nonzero_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_nonzero"));
    let cont = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_cont"));
    bctx.builder
        .build_conditional_branch(modulus_is_zero, zero_block, nonzero_block)?;

    bctx.builder.position_at_end(zero_block);
    bctx.builder.build_unconditional_branch(cont)?;

    bctx.builder.position_at_end(nonzero_block);
    let i512 = bctx
        .env
        .context()
        .custom_width_int_type(NonZeroU32::new(512).expect("non-zero bit width"))
        .expect("i512 should be valid");
    let a = bctx
        .builder
        .build_int_z_extend(a, i512, &format!("{name}_a512"))?;
    let b = bctx
        .builder
        .build_int_z_extend(b, i512, &format!("{name}_b512"))?;
    let modulus = bctx
        .builder
        .build_int_z_extend(modulus, i512, &format!("{name}_modulus512"))?;
    let wide_result = op(a, b)?;
    let reduced =
        bctx.builder
            .build_int_unsigned_rem(wide_result, modulus, &format!("{name}_reduced"))?;
    let reduced = bctx
        .builder
        .build_int_truncate(reduced, t.i256, &format!("{name}_result"))?;
    bctx.builder.build_unconditional_branch(cont)?;

    bctx.builder.position_at_end(cont);
    let phi = bctx
        .builder
        .build_phi(t.i256, &format!("{name}_result_phi"))?;
    phi.add_incoming(&[(&zero, zero_block), (&reduced, nonzero_block)]);
    Ok(phi.as_basic_value().into_int_value())
}

fn build_exp_value<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    base: IntValue<'ctx>,
    exponent: IntValue<'ctx>,
) -> Result<IntValue<'ctx>, Error> {
    let t = bctx.env.types();
    let initial_block = bctx
        .builder
        .get_insert_block()
        .ok_or_else(|| Error::invariant_violation("builder has no insertion block"))?;
    let loop_block = bctx.env.context().append_basic_block(bctx.func, "exp_loop");
    let body_block = bctx.env.context().append_basic_block(bctx.func, "exp_body");
    let cont_block = bctx.env.context().append_basic_block(bctx.func, "exp_cont");

    bctx.builder.build_unconditional_branch(loop_block)?;
    bctx.builder.position_at_end(loop_block);

    let result_phi = bctx.builder.build_phi(t.i256, "exp_result_phi")?;
    let base_phi = bctx.builder.build_phi(t.i256, "exp_base_phi")?;
    let exponent_phi = bctx.builder.build_phi(t.i256, "exp_exponent_phi")?;
    result_phi.add_incoming(&[(&t.i256.const_int(1, false), initial_block)]);
    base_phi.add_incoming(&[(&base, initial_block)]);
    exponent_phi.add_incoming(&[(&exponent, initial_block)]);

    let result = result_phi.as_basic_value().into_int_value();
    let base = base_phi.as_basic_value().into_int_value();
    let exponent = exponent_phi.as_basic_value().into_int_value();
    let is_done = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        exponent,
        t.i256.const_zero(),
        "exp_done",
    )?;
    bctx.builder
        .build_conditional_branch(is_done, cont_block, body_block)?;

    bctx.builder.position_at_end(body_block);
    let is_odd = bctx.builder.build_int_compare(
        inkwell::IntPredicate::NE,
        bctx.builder
            .build_and(exponent, t.i256.const_int(1, false), "exp_odd_bit")?,
        t.i256.const_zero(),
        "exp_is_odd",
    )?;
    let multiplied = bctx
        .builder
        .build_int_mul(result, base, "exp_result_multiplied")?;
    let next_result = bctx
        .builder
        .build_select(is_odd, multiplied, result, "exp_next_result")?
        .into_int_value();
    let next_base = bctx.builder.build_int_mul(base, base, "exp_next_base")?;
    let next_exponent = bctx.builder.build_right_shift(
        exponent,
        t.i256.const_int(1, false),
        false,
        "exp_next_exponent",
    )?;
    bctx.builder.build_unconditional_branch(loop_block)?;
    let body_block = bctx
        .builder
        .get_insert_block()
        .ok_or_else(|| Error::invariant_violation("builder has no insertion block"))?;

    result_phi.add_incoming(&[(&next_result, body_block)]);
    base_phi.add_incoming(&[(&next_base, body_block)]);
    exponent_phi.add_incoming(&[(&next_exponent, body_block)]);

    bctx.builder.position_at_end(cont_block);
    Ok(result)
}

fn build_mem_load_value<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    loc: IntValue<'ctx>,
) -> Result<IntValue<'ctx>, Error> {
    let mem_ptr = load_memory_ptr(bctx)?;
    let loc = memory_gep_index(bctx, loc, "mem.load.index")?;
    let byte_ptr = unsafe {
        bctx.builder
            .build_gep(bctx.env.types().i8, mem_ptr, &[loc], "mem.byte.ptr")
    }?;
    Ok(bctx
        .builder
        .build_load(bctx.env.types().i256, byte_ptr, "mem.value")?
        .into_int_value())
}

fn build_mem_store_value<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    loc: IntValue<'ctx>,
    value: IntValue<'ctx>,
) -> Result<(), Error> {
    let mem_ptr = load_memory_ptr(bctx)?;
    let loc = memory_gep_index(bctx, loc, "mem.store.index")?;
    let byte_ptr = unsafe {
        bctx.builder
            .build_gep(bctx.env.types().i8, mem_ptr, &[loc], "mem.byte.ptr")
    }?;
    bctx.builder
        .build_store(byte_ptr, normalize_to_i256(bctx, value)?)?;
    Ok(())
}

fn build_mem_store_byte_value<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    loc: IntValue<'ctx>,
    value: IntValue<'ctx>,
) -> Result<(), Error> {
    let mem_ptr = load_memory_ptr(bctx)?;
    let loc = memory_gep_index(bctx, loc, "mem.store8.index")?;
    let byte_ptr = unsafe {
        bctx.builder
            .build_gep(bctx.env.types().i8, mem_ptr, &[loc], "mem.byte.ptr")
    }?;
    let value = if value.get_type().get_bit_width() == 8 {
        value
    } else {
        bctx.builder
            .build_int_truncate(value, bctx.env.types().i8, "mem.store.byte")?
    };
    bctx.builder.build_store(byte_ptr, value)?;
    Ok(())
}

fn memory_gep_index<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    value: IntValue<'ctx>,
    name: &str,
) -> Result<IntValue<'ctx>, Error> {
    match value.get_type().get_bit_width() {
        64 => Ok(value),
        width if width < 64 => {
            Ok(bctx
                .builder
                .build_int_z_extend(value, bctx.env.types().i64, name)?)
        }
        _ => Ok(bctx
            .builder
            .build_int_truncate(value, bctx.env.types().i64, name)?),
    }
}

fn load_memory_ptr<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<PointerValue<'ctx>, Error> {
    let mem_ptr_addr = bctx.builder.build_struct_gep(
        bctx.env.types().exec_ctx,
        bctx.registers.exec_ctx,
        6,
        "mem.ptr.addr",
    )?;
    Ok(bctx
        .builder
        .build_load(bctx.env.types().ptr, mem_ptr_addr, "mem.ptr")?
        .into_pointer_value())
}

fn build_address_limbs<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    value: IntValue<'ctx>,
) -> Result<(IntValue<'ctx>, IntValue<'ctx>, IntValue<'ctx>), Error> {
    let value = normalize_to_i256(bctx, value)?;
    let shift_64 = bctx.env.types().i256.const_int(64, false);
    let shift_128 = bctx.env.types().i256.const_int(128, false);
    let addr_lo = bctx
        .builder
        .build_int_truncate(value, bctx.env.types().i64, "call.addr.lo")?;
    let addr_mid_src =
        bctx.builder
            .build_right_shift(value, shift_64, false, "call.addr.mid.src")?;
    let addr_mid =
        bctx.builder
            .build_int_truncate(addr_mid_src, bctx.env.types().i64, "call.addr.mid")?;
    let addr_hi_src =
        bctx.builder
            .build_right_shift(value, shift_128, false, "call.addr.hi.src")?;
    let addr_hi =
        bctx.builder
            .build_int_truncate(addr_hi_src, bctx.env.types().i32, "call.addr.hi")?;
    Ok((addr_lo, addr_mid, addr_hi))
}

fn truncate_to_i32<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    value: IntValue<'ctx>,
    name: &str,
) -> Result<IntValue<'ctx>, Error> {
    if value.get_type().get_bit_width() == 32 {
        return Ok(value);
    }
    Ok(bctx
        .builder
        .build_int_truncate(value, bctx.env.types().i32, name)?)
}

fn shift_left_or_right<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    left: bool,
    signed: bool,
    name: &str,
) -> Result<(), Error> {
    let (shift, value) = bctx.stack.pop_2(bctx)?;
    let t = bctx.env.types();
    let zero = t.i256.const_zero();
    let max_shift = t.i256.const_int(255, false);
    let is_large = bctx.builder.build_int_compare(
        inkwell::IntPredicate::UGT,
        shift,
        max_shift,
        &format!("{name}_is_large"),
    )?;
    let safe_shift = bctx
        .builder
        .build_select(is_large, zero, shift, &format!("{name}_safe_shift"))?
        .into_int_value();
    let shifted = if left {
        bctx.builder
            .build_left_shift(value, safe_shift, &format!("{name}_shifted"))?
    } else {
        bctx.builder
            .build_right_shift(value, safe_shift, signed, &format!("{name}_shifted"))?
    };
    let result = bctx
        .builder
        .build_select(is_large, zero, shifted, &format!("{name}_result"))?
        .into_int_value();
    bctx.stack.push_word(bctx, result)
}

fn branch_on_i8_success<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    ret: CallSiteValue<'ctx>,
    name: &str,
) -> Result<(), Error> {
    let ret_i8 = unsafe { IntValue::new(ret.as_value_ref()) };
    let zero = bctx.env.types().i8.const_int(0, false);
    let is_ok = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        ret_i8,
        zero,
        &format!("{name}_ok"),
    )?;
    let ok_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_success"));
    let err_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_error"));
    bctx.builder
        .build_conditional_branch(is_ok, ok_block, err_block)?;
    bctx.builder.position_at_end(err_block);
    build_return(bctx, ReturnCode::Invalid)?;
    bctx.builder.position_at_end(ok_block);
    Ok(())
}

fn call_mem_expand_checked<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    loc_i32: IntValue<'ctx>,
    size: IntValue<'ctx>,
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
