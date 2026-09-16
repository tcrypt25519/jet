use std::num::NonZeroU32;

use inkwell::{
    basic_block::BasicBlock,
    builder::BuilderError,
    intrinsics::Intrinsic,
    values::{AsValueRef, CallSiteValue, IntValue, PhiValue, PointerValue},
};
use jet_runtime::{BLOCK_HASH_HISTORY_SIZE, exec::ReturnCode};

use crate::{
    builder::{Error, contract::BuildCtx, gas::DynamicGas, stack::StackBackend},
    instructions::Instruction,
};

/// A memory region that has already been expanded by `jet_mem_expand`.
///
/// Only [`expand_memory_region`] can construct this type, so any helper that
/// accepts a `MemoryRegion` cannot be reached without first expanding memory.
#[derive(Copy, Clone)]
struct MemoryRegion<'ctx> {
    offset: IntValue<'ctx>,
    size: IntValue<'ctx>,
}

impl<'ctx> MemoryRegion<'ctx> {
    fn offset(&self) -> IntValue<'ctx> {
        self.offset
    }

    fn size(&self) -> IntValue<'ctx> {
        self.size
    }
}

fn expand_memory_region<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    offset: IntValue<'ctx>,
    size: IntValue<'ctx>,
    label: &str,
) -> Result<MemoryRegion<'ctx>, Error> {
    call_mem_expand_checked(bctx, offset, size, label)?;
    Ok(MemoryRegion { offset, size })
}

pub(crate) fn charge_static_gas<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    cost: u64,
    pc: usize,
) -> Result<(), Error> {
    if cost == 0 {
        return Ok(());
    }

    charge_gas_value(bctx, bctx.env.types().i64.const_int(cost, false), pc)
}

fn charge_gas_value<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    cost_value: IntValue<'ctx>,
    pc: usize,
) -> Result<(), Error> {
    let available = bctx.gas_remaining()?;
    let can_pay = bctx.builder.build_int_compare(
        inkwell::IntPredicate::UGE,
        available,
        cost_value,
        "gas.can_pay",
    )?;
    let success_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "gas.success");
    let failure_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "gas.failure");
    bctx.builder
        .build_conditional_branch(can_pay, success_block, failure_block)?;

    bctx.builder.position_at_end(failure_block);
    bctx.stack.materialize_for_return(bctx)?;
    bctx.builder.build_call(
        bctx.env.symbols().gas_failure_static(),
        &[
            bctx.registers.exec_ctx.into(),
            bctx.env.types().i32.const_int(pc as u64, false).into(),
            available.into(),
            cost_value.into(),
        ],
        "gas.record_failure",
    )?;
    let return_value = bctx
        .env
        .types()
        .i8
        .const_int(ReturnCode::OutOfGas as u64, false);
    bctx.builder.build_return(Some(&return_value))?;

    bctx.builder.position_at_end(success_block);
    let remaining =
        bctx.builder
            .build_int_sub(available, cost_value, "gas.remaining.after_charge")?;
    bctx.set_gas_remaining(remaining);
    Ok(())
}

pub(crate) fn charge_dynamic_gas<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    kind: DynamicGas,
    static_cost: u64,
    pc: usize,
) -> Result<(), Error> {
    if kind.stack_inputs() == 0 {
        return charge_dynamic_gas_available(bctx, kind, static_cost, pc);
    }

    let before = bctx.gas_remaining()?;
    let has_inputs = bctx.stack.has_words(bctx, kind.stack_inputs())?;
    let charge_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "gas.dynamic.inputs_available");
    let skip_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "gas.dynamic.inputs_missing");
    let continuation = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "gas.dynamic.inputs_cont");
    bctx.builder
        .build_conditional_branch(has_inputs, charge_block, skip_block)?;

    bctx.builder.position_at_end(charge_block);
    charge_dynamic_gas_available(bctx, kind, static_cost, pc)?;
    let charged = bctx.gas_remaining()?;
    let charged_pred = bctx
        .builder
        .get_insert_block()
        .ok_or_else(|| Error::invariant_violation("missing dynamic gas predecessor"))?;
    bctx.builder.build_unconditional_branch(continuation)?;

    bctx.builder.position_at_end(skip_block);
    bctx.builder.build_unconditional_branch(continuation)?;

    bctx.builder.position_at_end(continuation);
    let gas_phi = bctx
        .builder
        .build_phi(bctx.env.types().i64, "gas.dynamic.remaining")?;
    gas_phi.add_incoming(&[(&charged, charged_pred), (&before, skip_block)]);
    bctx.set_gas_remaining(gas_phi.as_basic_value().into_int_value());
    Ok(())
}

fn charge_dynamic_gas_available<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    kind: DynamicGas,
    static_cost: u64,
    pc: usize,
) -> Result<(), Error> {
    let dynamic_cost = match kind {
        DynamicGas::Gas => bctx.env.types().i64.const_zero(),
        DynamicGas::Exp => exp_dynamic_gas(bctx)?,
        DynamicGas::Keccak256 => {
            let offset = bctx.stack.peek_word(bctx, 0)?;
            let size = bctx.stack.peek_word(bctx, 1)?;
            let expansion = memory_expansion_gas(bctx, &[(offset, size)])?;
            let words = word_count(bctx, gas_bounded_i32(bctx, size, "keccak.gas.size")?)?;
            let copy_cost = bctx.builder.build_int_mul(
                words,
                bctx.env.types().i64.const_int(6, false),
                "keccak.word.gas",
            )?;
            bctx.builder
                .build_int_add(expansion, copy_cost, "keccak.dynamic.gas")?
        }
        DynamicGas::Memory { fixed_size, .. } => memory_operation_dynamic_gas(bctx, fixed_size)?,
        DynamicGas::ReturnDataCopy => {
            let dest = bctx.stack.peek_word(bctx, 0)?;
            let size = bctx.stack.peek_word(bctx, 2)?;
            let expansion = memory_expansion_gas(bctx, &[(dest, size)])?;
            let words = word_count(bctx, gas_bounded_i32(bctx, size, "retcopy.gas.size")?)?;
            let copy_cost = bctx.builder.build_int_mul(
                words,
                bctx.env.types().i64.const_int(3, false),
                "retcopy.word.gas",
            )?;
            bctx.builder
                .build_int_add(expansion, copy_cost, "retcopy.dynamic.gas")?
        }
        DynamicGas::Call => {
            let in_offset = bctx.stack.peek_word(bctx, 3)?;
            let in_size = bctx.stack.peek_word(bctx, 4)?;
            let out_offset = bctx.stack.peek_word(bctx, 5)?;
            let out_size = bctx.stack.peek_word(bctx, 6)?;
            let expansion =
                memory_expansion_gas(bctx, &[(in_offset, in_size), (out_offset, out_size)])?;
            bctx.builder.build_int_add(
                expansion,
                bctx.env.types().i64.const_int(100, false),
                "call.dynamic.gas",
            )?
        }
    };
    let total = bctx.builder.build_int_add(
        dynamic_cost,
        bctx.env.types().i64.const_int(static_cost, false),
        "gas.total.dynamic",
    )?;
    charge_gas_value(bctx, total, pc)
}

fn memory_operation_dynamic_gas<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    fixed_size: Option<u32>,
) -> Result<IntValue<'ctx>, Error> {
    let offset = bctx.stack.peek_word(bctx, 0)?;
    let size = match fixed_size {
        Some(size) => bctx.env.types().i256.const_int(size as u64, false),
        None => bctx.stack.peek_word(bctx, 1)?,
    };
    memory_expansion_gas(bctx, &[(offset, size)])
}

fn gas_bounded_i32<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    value: IntValue<'ctx>,
    name: &str,
) -> Result<IntValue<'ctx>, Error> {
    let t = bctx.env.types();
    if value.get_type().get_bit_width() <= 32 {
        return Ok(if value.get_type().get_bit_width() == 32 {
            value
        } else {
            bctx.builder.build_int_z_extend(value, t.i32, name)?
        });
    }
    let max = value.get_type().const_int(u32::MAX as u64, false);
    let wide = bctx.builder.build_int_compare(
        inkwell::IntPredicate::UGT,
        value,
        max,
        &format!("{name}.wide"),
    )?;
    let truncated = bctx.builder.build_int_truncate(value, t.i32, name)?;
    Ok(bctx
        .builder
        .build_select(
            wide,
            t.i32.const_all_ones(),
            truncated,
            &format!("{name}.bounded"),
        )?
        .into_int_value())
}

fn word_count<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    size: IntValue<'ctx>,
) -> Result<IntValue<'ctx>, Error> {
    let size = bctx
        .builder
        .build_int_z_extend(size, bctx.env.types().i64, "gas.size.i64")?;
    let rounded = bctx.builder.build_int_add(
        size,
        bctx.env.types().i64.const_int(31, false),
        "gas.size.rounded",
    )?;
    Ok(bctx.builder.build_int_unsigned_div(
        rounded,
        bctx.env.types().i64.const_int(32, false),
        "gas.word.count",
    )?)
}

fn memory_expansion_gas<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    regions: &[(IntValue<'ctx>, IntValue<'ctx>)],
) -> Result<IntValue<'ctx>, Error> {
    let t = bctx.env.types();
    let memory_len_ptr = bctx.builder.build_struct_gep(
        t.exec_ctx,
        bctx.registers.exec_ctx,
        7,
        "gas.memory_len.ptr",
    )?;
    let memory_len = bctx
        .builder
        .build_load(t.i32, memory_len_ptr, "gas.memory_len")?
        .into_int_value();
    let old_words = bctx.builder.build_int_unsigned_div(
        bctx.builder
            .build_int_z_extend(memory_len, t.i64, "gas.old_size")?,
        t.i64.const_int(32, false),
        "gas.old_words",
    )?;
    let mut new_words = old_words;
    for (index, (offset, size)) in regions.iter().enumerate() {
        let offset = gas_bounded_i32(bctx, *offset, &format!("gas.memory.offset.{index}"))?;
        let size = gas_bounded_i32(bctx, *size, &format!("gas.memory.size.{index}"))?;
        let offset = bctx.builder.build_int_z_extend(
            offset,
            t.i64,
            &format!("gas.memory.offset64.{index}"),
        )?;
        let size64 =
            bctx.builder
                .build_int_z_extend(size, t.i64, &format!("gas.memory.size64.{index}"))?;
        let end = bctx
            .builder
            .build_int_add(offset, size64, &format!("gas.memory.end.{index}"))?;
        let words = bctx.builder.build_int_unsigned_div(
            bctx.builder.build_int_add(
                end,
                t.i64.const_int(31, false),
                &format!("gas.memory.rounded.{index}"),
            )?,
            t.i64.const_int(32, false),
            &format!("gas.memory.words.{index}"),
        )?;
        let nonzero_words = bctx
            .builder
            .build_select(
                bctx.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    size,
                    t.i32.const_zero(),
                    &format!("gas.memory.zero_size.{index}"),
                )?,
                old_words,
                words,
                &format!("gas.memory.nonzero_words.{index}"),
            )?
            .into_int_value();
        new_words = bctx
            .builder
            .build_select(
                bctx.builder.build_int_compare(
                    inkwell::IntPredicate::UGT,
                    nonzero_words,
                    new_words,
                    &format!("gas.memory.grows.{index}"),
                )?,
                nonzero_words,
                new_words,
                &format!("gas.memory.max_words.{index}"),
            )?
            .into_int_value();
    }
    let old_cost = memory_cost(bctx, old_words)?;
    let new_cost = memory_cost(bctx, new_words)?;
    Ok(bctx
        .builder
        .build_int_sub(new_cost, old_cost, "gas.memory.expansion")?)
}

fn memory_cost<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    words: IntValue<'ctx>,
) -> Result<IntValue<'ctx>, Error> {
    let linear = bctx.builder.build_int_mul(
        words,
        bctx.env.types().i64.const_int(3, false),
        "gas.memory.linear",
    )?;
    let square = bctx
        .builder
        .build_int_mul(words, words, "gas.memory.square")?;
    let quadratic = bctx.builder.build_int_unsigned_div(
        square,
        bctx.env.types().i64.const_int(512, false),
        "gas.memory.quadratic",
    )?;
    Ok(bctx
        .builder
        .build_int_add(linear, quadratic, "gas.memory.cost")?)
}

fn exp_dynamic_gas<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<IntValue<'ctx>, Error> {
    let exponent = bctx.stack.peek_word(bctx, 1)?;
    let ctlz = Intrinsic::find("llvm.ctlz")
        .ok_or_else(|| Error::invariant_violation("llvm.ctlz intrinsic is unavailable"))?;
    let declaration = ctlz
        .get_declaration(bctx.env.module(), &[bctx.env.types().i256.into()])
        .ok_or_else(|| Error::invariant_violation("llvm.ctlz.i256 declaration is unavailable"))?;
    let call = bctx.builder.build_call(
        declaration,
        &[
            exponent.into(),
            bctx.env.context().bool_type().const_zero().into(),
        ],
        "gas.exp.leading_zeros",
    )?;
    let leading_zeros = call.try_as_basic_value().unwrap_basic().into_int_value();
    let bits = bctx.builder.build_int_sub(
        bctx.env.types().i256.const_int(263, false),
        leading_zeros,
        "gas.exp.rounded_bits",
    )?;
    let bytes = bctx.builder.build_int_unsigned_div(
        bits,
        bctx.env.types().i256.const_int(8, false),
        "gas.exp.bytes",
    )?;
    let bytes =
        bctx.builder
            .build_int_truncate(bytes, bctx.env.types().i64, "gas.exp.bytes.i64")?;
    Ok(bctx.builder.build_int_mul(
        bytes,
        bctx.env.types().i64.const_int(50, false),
        "gas.exp.dynamic",
    )?)
}

pub(crate) fn build_return<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    return_value: ReturnCode,
) -> Result<(), Error> {
    bctx.stack.materialize_for_return(bctx)?;
    bctx.builder
        .build_store(bctx.registers.gas_remaining, bctx.gas_remaining()?)?;
    let return_value = bctx.env.types().i8.const_int(return_value as u64, false);
    bctx.builder.build_return(Some(&return_value))?;
    Ok(())
}

pub(crate) fn push<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    bytes: [u8; 32],
    known_u64: Option<u64>,
) -> Result<(), Error> {
    let mut limbs = [0u64; 4];
    for (i, chunk) in bytes.as_chunks::<8>().0.iter().enumerate() {
        limbs[i] = u64::from_le_bytes(*chunk);
    }
    debug_assert_eq!(
        known_u64,
        limbs[1..].iter().all(|limb| *limb == 0).then_some(limbs[0])
    );
    let value = bctx.env.types().i256.const_int_arbitrary_precision(&limbs);
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
    let region = expand_memory_region(bctx, offset_i32, size_i32, "keccak256")?;
    let result_ptr = bctx
        .builder
        .build_alloca(bctx.env.types().i256, "keccak_result")?;
    let ret = bctx.builder.build_call(
        bctx.env.symbols().keccak256(),
        &[
            bctx.registers.exec_ctx.into(),
            region.offset().into(),
            region.size().into(),
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
    let is_null = bctx
        .builder
        .build_is_null(sub_call_ctx_ptr, "sub_call_is_null")?;
    let zero = bctx.env.types().i32.const_int(0, false);
    let non_null_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "returndatasize_non_null");
    let cont_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "returndatasize_cont");
    let entry_block = bctx.builder.get_insert_block().unwrap();
    bctx.builder
        .build_conditional_branch(is_null, cont_block, non_null_block)?;
    bctx.builder.position_at_end(non_null_block);
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
    bctx.builder.build_unconditional_branch(cont_block)?;
    bctx.builder.position_at_end(cont_block);
    let result = bctx
        .builder
        .build_phi(bctx.env.types().i32, "returndatasize_result")?;
    result.add_incoming(&[(&zero, entry_block), (&return_length, non_null_block)]);
    bctx.stack
        .push_word(bctx, result.as_basic_value().into_int_value())
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
    let region = expand_memory_region(bctx, dest_off, len, "returndatacopy")?;
    let ret = bctx.builder.build_call(
        bctx.env.symbols().contract_call_return_data_copy(),
        &[
            bctx.registers.exec_ctx.into(),
            sub_call_ctx_ptr.into(),
            region.offset().into(),
            src_off.into(),
            region.size().into(),
        ],
        "return_data_copy",
    )?;
    branch_on_i8_success(bctx, ret, "returndatacopy")
}

pub(crate) fn blockhash<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let requested = bctx.stack.pop_word(bctx)?;
    let t = bctx.env.types();
    let number_ptr = bctx.builder.build_struct_gep(
        t.block_info,
        bctx.registers.block_info,
        0,
        "block_info_number_ptr",
    )?;
    let number = bctx
        .builder
        .build_load(t.i64, number_ptr, "block_info_number")?
        .into_int_value();
    let number = bctx
        .builder
        .build_int_z_extend(number, t.i256, "block_info_number_i256")?;
    let distance = bctx
        .builder
        .build_int_sub(number, requested, "blockhash_distance")?;
    let before_current = bctx.builder.build_int_compare(
        inkwell::IntPredicate::ULT,
        requested,
        number,
        "blockhash_before_current",
    )?;
    let within_history = bctx.builder.build_int_compare(
        inkwell::IntPredicate::ULE,
        distance,
        t.i256.const_int(BLOCK_HASH_HISTORY_SIZE as u64, false),
        "blockhash_within_history",
    )?;
    let valid = bctx
        .builder
        .build_and(before_current, within_history, "blockhash_valid")?;
    let valid_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "blockhash_valid");
    let invalid_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "blockhash_invalid");
    let cont_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "blockhash_cont");
    bctx.builder
        .build_conditional_branch(valid, valid_block, invalid_block)?;

    bctx.builder.position_at_end(valid_block);
    let history_ptr = bctx.builder.build_struct_gep(
        t.block_info,
        bctx.registers.block_info,
        8,
        "block_info_hash_history_ptr",
    )?;
    let history_index = bctx.builder.build_int_sub(
        distance,
        t.i256.const_int(1, false),
        "blockhash_history_index",
    )?;
    let history_index =
        bctx.builder
            .build_int_truncate(history_index, t.i32, "blockhash_history_index_i32")?;
    let history_type = t.word_bytes.array_type(BLOCK_HASH_HISTORY_SIZE as u32);
    let hash_ptr = unsafe {
        bctx.builder.build_gep(
            history_type,
            history_ptr,
            &[t.i32.const_zero(), history_index],
            "blockhash_ptr",
        )
    }?;
    let hash = bctx
        .builder
        .build_load(t.i256, hash_ptr, "blockhash")?
        .into_int_value();
    bctx.builder.build_unconditional_branch(cont_block)?;

    bctx.builder.position_at_end(invalid_block);
    bctx.builder.build_unconditional_branch(cont_block)?;

    bctx.builder.position_at_end(cont_block);
    let result = bctx.builder.build_phi(t.i256, "blockhash_result")?;
    let zero = t.i256.const_zero();
    result.add_incoming(&[(&hash, valid_block), (&zero, invalid_block)]);
    bctx.stack
        .push_word(bctx, result.as_basic_value().into_int_value())
}

pub(crate) fn coinbase<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    push_block_info_i160_field(bctx, 9, "block_info_coinbase")
}

pub(crate) fn timestamp<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    push_block_info_u64_field(bctx, 3, "block_info_timestamp")
}

pub(crate) fn number<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    push_block_info_u64_field(bctx, 0, "block_info_number")
}

pub(crate) fn difficulty<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    push_block_info_u64_field(bctx, 1, "block_info_difficulty")
}

pub(crate) fn gaslimit<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    push_block_info_u64_field(bctx, 2, "block_info_gaslimit")
}

pub(crate) fn chainid<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    push_block_info_u64_field(bctx, 6, "block_info_chainid")
}

pub(crate) fn basefee<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    push_block_info_u64_field(bctx, 4, "block_info_basefee")
}

pub(crate) fn blobbasefee<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    push_block_info_u64_field(bctx, 5, "block_info_blobbasefee")
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
    let region = expand_memory_region(bctx, loc_i32, size, "mload")?;
    let value = build_mem_load_value(bctx, &region)?;
    bctx.stack.push_word(bctx, value)
}

pub(crate) fn mstore<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let (loc, val) = bctx.stack.pop_2(bctx)?;
    let loc_i32 = truncate_to_i32(bctx, loc, "mstore_loc")?;
    let size = bctx.env.types().i32.const_int(32, false);
    let region = expand_memory_region(bctx, loc_i32, size, "mstore")?;
    build_mem_store_value(bctx, &region, val)
}

pub(crate) fn mstore8<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let (loc, val) = bctx.stack.pop_2(bctx)?;
    let loc_i32 = truncate_to_i32(bctx, loc, "mstore8_loc")?;
    let size = bctx.env.types().i32.const_int(1, false);
    let region = expand_memory_region(bctx, loc_i32, size, "mstore8")?;
    build_mem_store_byte_value(bctx, &region, val)
}

pub(crate) fn jump<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    jump_block: BasicBlock<'ctx>,
    jump_gas_phi: PhiValue<'ctx>,
) -> Result<(), Error> {
    let pc = bctx.stack.pop_word(bctx)?;
    let pc = truncate_jump_target(bctx, pc, "jump_pc")?;
    bctx.builder.build_store(bctx.registers.jump_ptr, pc)?;
    let pred = bctx
        .builder
        .get_insert_block()
        .ok_or_else(|| Error::invariant_violation("missing JUMP predecessor"))?;
    let gas = bctx.gas_remaining()?;
    jump_gas_phi.add_incoming(&[(&gas, pred)]);
    bctx.builder.build_unconditional_branch(jump_block)?;
    Ok(())
}

pub(crate) fn jumpi<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    jump_block: BasicBlock<'ctx>,
    jump_gas_phi: PhiValue<'ctx>,
    jump_else_block: BasicBlock<'ctx>,
    jump_else_gas_phi: PhiValue<'ctx>,
) -> Result<(), Error> {
    let (pc, cond) = bctx.stack.pop_2(bctx)?;
    let pc = truncate_jump_target(bctx, pc, "jumpi_pc")?;
    bctx.builder.build_store(bctx.registers.jump_ptr, pc)?;
    let cmp = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        cond,
        bctx.env.types().i256.const_zero(),
        "jumpi_cmp",
    )?;
    let pred = bctx
        .builder
        .get_insert_block()
        .ok_or_else(|| Error::invariant_violation("missing JUMPI predecessor"))?;
    let gas = bctx.gas_remaining()?;
    jump_gas_phi.add_incoming(&[(&gas, pred)]);
    jump_else_gas_phi.add_incoming(&[(&gas, pred)]);
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

pub(crate) fn address<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    push_call_info_i160_field(bctx, 2, "address")
}

pub(crate) fn origin<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    push_call_info_i160_field(bctx, 3, "origin")
}

pub(crate) fn caller<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    push_call_info_i160_field(bctx, 4, "caller")
}

pub(crate) fn callvalue<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let call_info = call_info_ptr(bctx)?;
    let value_ptr =
        bctx.builder
            .build_struct_gep(bctx.env.types().call_info, call_info, 5, "callvalue_ptr")?;
    let value = bctx
        .builder
        .build_load(bctx.env.types().i256, value_ptr, "callvalue")?
        .into_int_value();
    bctx.stack.push_word(bctx, value)
}

pub(crate) fn calldatasize<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let call_info = call_info_ptr(bctx)?;
    let len_ptr = bctx.builder.build_struct_gep(
        bctx.env.types().call_info,
        call_info,
        1,
        "calldata_len_ptr",
    )?;
    let len = bctx
        .builder
        .build_load(bctx.env.types().i32, len_ptr, "calldata_len")?
        .into_int_value();
    let len = bctx
        .builder
        .build_int_z_extend(len, bctx.env.types().i256, "calldata_len_i256")?;
    bctx.stack.push_word(bctx, len)
}

pub(crate) fn calldataload<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let offset = bctx.stack.pop_word(bctx)?;
    let offset_ptr = bctx
        .builder
        .build_alloca(bctx.env.types().i256, "calldata_offset")?;
    bctx.builder.build_store(offset_ptr, offset)?;
    let out_ptr = bctx
        .builder
        .build_alloca(bctx.env.types().i256, "calldata_out")?;
    bctx.builder.build_call(
        bctx.env.symbols().call_data_load(),
        &[
            bctx.registers.exec_ctx.into(),
            offset_ptr.into(),
            out_ptr.into(),
        ],
        "calldataload",
    )?;
    let value = bctx
        .builder
        .build_load(bctx.env.types().i256, out_ptr, "calldata_word")?
        .into_int_value();
    bctx.stack.push_word(bctx, value)
}

pub(crate) fn call<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (_gas, to, value, _in_off, _in_len, out_off, out_len) = bctx.stack.pop_7(bctx)?;
    let jit_engine = bctx.env.symbols().jit_engine();
    let jit_engine_ptr = jit_engine.as_pointer_value();
    let (addr_lo, addr_mid, addr_hi) = build_address_limbs(bctx, to)?;
    let value_ptr = bctx
        .builder
        .build_alloca(bctx.env.types().i256, "call.value")?;
    bctx.builder.build_store(value_ptr, value)?;
    let out_off = truncate_to_i32(bctx, out_off, "call.out_off")?;
    let out_len = truncate_to_i32(bctx, out_len, "call.out_len")?;
    let out_region = expand_memory_region(bctx, out_off, out_len, "call")?;
    let make_contract_call = bctx.builder.build_call(
        bctx.env.symbols().contract_call_values(),
        &[
            bctx.registers.exec_ctx.into(),
            bctx.registers.block_info.into(),
            jit_engine_ptr.into(),
            addr_lo.into(),
            addr_mid.into(),
            addr_hi.into(),
            value_ptr.into(),
            out_region.offset().into(),
            out_region.size().into(),
        ],
        "contract_call",
    )?;
    let ret = unsafe { IntValue::new(make_contract_call.as_value_ref()) };
    let success = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        ret,
        bctx.env.types().i8.const_zero(),
        "contract_call_success",
    )?;
    bctx.stack.push_word(bctx, success)
}

pub(crate) fn _return<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    set_return_range(bctx)?;
    build_return(bctx, ReturnCode::ExplicitReturn)
}

pub(crate) fn revert<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    set_return_range(bctx)?;
    build_return(bctx, ReturnCode::Revert)
}

fn set_return_range<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let (offset, size) = bctx.stack.pop_2(bctx)?;
    let offset = truncate_to_i32(bctx, offset, "return_offset")?;
    let size = truncate_to_i32(bctx, size, "return_length")?;
    let region = expand_memory_region(bctx, offset, size, "return")?;
    bctx.builder
        .build_store(bctx.registers.return_offset, region.offset())?;
    bctx.builder
        .build_store(bctx.registers.return_length, region.size())?;
    Ok(())
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

pub(crate) fn gas<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    bctx.stack.push_word(bctx, bctx.gas_remaining()?)
}

pub(crate) fn msize<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<(), Error> {
    let memory_len_ptr = bctx.builder.build_struct_gep(
        bctx.env.types().exec_ctx,
        bctx.registers.exec_ctx,
        7,
        "memory_len_ptr",
    )?;
    let memory_len = bctx
        .builder
        .build_load(bctx.env.types().i32, memory_len_ptr, "memory_len")?
        .into_int_value();
    bctx.stack.push_word(bctx, memory_len)
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

fn push_block_info_u64_field<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    field_index: u32,
    name: &str,
) -> Result<(), Error> {
    let field_ptr = bctx.builder.build_struct_gep(
        bctx.env.types().block_info,
        bctx.registers.block_info,
        field_index,
        &format!("{name}_ptr"),
    )?;
    let field = bctx
        .builder
        .build_load(bctx.env.types().i64, field_ptr, name)?
        .into_int_value();
    bctx.stack.push_word(bctx, field)
}

fn push_block_info_i160_field<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    field_index: u32,
    name: &str,
) -> Result<(), Error> {
    let field_ptr = bctx.builder.build_struct_gep(
        bctx.env.types().block_info,
        bctx.registers.block_info,
        field_index,
        &format!("{name}_ptr"),
    )?;
    let field = bctx
        .builder
        .build_load(bctx.env.types().i160, field_ptr, name)?
        .into_int_value();
    bctx.stack.push_word(bctx, field)
}

fn call_info_ptr<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<PointerValue<'ctx>, Error> {
    Ok(bctx
        .builder
        .build_load(bctx.env.types().ptr, bctx.registers.call_info, "call_info")?
        .into_pointer_value())
}

fn push_call_info_i160_field<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    field_index: u32,
    name: &str,
) -> Result<(), Error> {
    let call_info = call_info_ptr(bctx)?;
    let field_ptr = bctx.builder.build_struct_gep(
        bctx.env.types().call_info,
        call_info,
        field_index,
        &format!("{name}_ptr"),
    )?;
    let field = bctx
        .builder
        .build_load(bctx.env.types().i160, field_ptr, name)?
        .into_int_value();
    let field =
        bctx.builder
            .build_int_z_extend(field, bctx.env.types().i256, &format!("{name}_i256"))?;
    bctx.stack.push_word(bctx, field)
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
    region: &MemoryRegion<'ctx>,
) -> Result<IntValue<'ctx>, Error> {
    let mem_ptr = load_memory_ptr(bctx)?;
    let loc = memory_gep_index(bctx, region.offset(), "mem.load.index")?;
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
    region: &MemoryRegion<'ctx>,
    value: IntValue<'ctx>,
) -> Result<(), Error> {
    let mem_ptr = load_memory_ptr(bctx)?;
    let loc = memory_gep_index(bctx, region.offset(), "mem.store.index")?;
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
    region: &MemoryRegion<'ctx>,
    value: IntValue<'ctx>,
) -> Result<(), Error> {
    let mem_ptr = load_memory_ptr(bctx)?;
    let loc = memory_gep_index(bctx, region.offset(), "mem.store8.index")?;
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
    let bit_width = value.get_type().get_bit_width();
    if bit_width == 32 {
        return Ok(value);
    }
    if bit_width < 32 {
        return Ok(bctx
            .builder
            .build_int_z_extend(value, bctx.env.types().i32, name)?);
    }

    let is_too_wide = bctx.builder.build_int_compare(
        inkwell::IntPredicate::UGT,
        value,
        value.get_type().const_int(u32::MAX as u64, false),
        &format!("{name}_is_too_wide"),
    )?;
    let error_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_error"));
    let valid_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_valid"));
    bctx.builder
        .build_conditional_branch(is_too_wide, error_block, valid_block)?;
    bctx.builder.position_at_end(error_block);
    build_return(bctx, ReturnCode::Invalid)?;
    bctx.builder.position_at_end(valid_block);
    Ok(bctx
        .builder
        .build_int_truncate(value, bctx.env.types().i32, name)?)
}

/// Narrows a 256-bit jump target to the i32 dispatch width. Targets above
/// `u32::MAX` would otherwise wrap and could collide with a real jumpdest, so
/// they map to `u32::MAX`, which no jumpdest pc can occupy, and dispatch
/// falls through to the jump failure default.
pub(crate) fn truncate_jump_target<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    pc: IntValue<'ctx>,
    name: &str,
) -> Result<IntValue<'ctx>, Error> {
    let bit_width = pc.get_type().get_bit_width();
    if bit_width != 256 {
        return Err(Error::InvalidBitWidth(bit_width));
    }
    let t = bctx.env.types();
    let max = t.i256.const_int(u32::MAX as u64, false);
    let is_wide = bctx.builder.build_int_compare(
        inkwell::IntPredicate::UGT,
        pc,
        max,
        &format!("{name}_is_wide"),
    )?;
    let truncated = bctx.builder.build_int_truncate(pc, t.i32, name)?;
    let sentinel = t.i32.const_int(u32::MAX as u64, false);
    Ok(bctx
        .builder
        .build_select(is_wide, sentinel, truncated, &format!("{name}_bounded"))?
        .into_int_value())
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
