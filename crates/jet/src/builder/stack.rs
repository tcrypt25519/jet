//! Stack backend trait and concrete implementations.
//!
//! [`StackBackend`] is the only interface between EVM opcode semantics and the
//! underlying stack representation.  There are two concrete implementations:
//!
//! * [`RuntimeStackBackend`] – emits LLVM `call` instructions to the `jet_runtime` C-ABI builtins, keeping the EVM stack in the `Context` struct at
//!   runtime.  This is a direct replacement for the `call_stack_*` helpers currently in `ops.rs`.
//!
//! * [`SymbolicStackBackend`] – tracks values as LLVM [`IntValue`]s at compile time, eliminating all runtime stack traffic for supported ops.
//!
//! [`BuildCtx`]: crate::builder::contract::BuildCtx

use std::cell::RefCell;

use inkwell::values::IntValue;
use jet_runtime::exec::ReturnCode;

use crate::builder::{Error, contract::BuildCtx, symbolic::SymbolicStack};

type SevenWords<'ctx> = (
    IntValue<'ctx>,
    IntValue<'ctx>,
    IntValue<'ctx>,
    IntValue<'ctx>,
    IntValue<'ctx>,
    IntValue<'ctx>,
    IntValue<'ctx>,
);

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Abstraction over EVM stack mechanics used during LLVM IR construction.
///
/// All methods speak exclusively in `IntValue<'ctx>` (256-bit EVM words).
/// Each implementation is responsible for any internal normalisation, loads,
/// stores, or compile-time state management needed to satisfy that contract.
///
/// # Index convention
/// `dup` and `swap` take a 1-based index matching the EVM opcode family:
/// `DUP1`/`SWAP1` pass `1`, `DUP16`/`SWAP16` pass `16`.
pub(crate) trait StackBackend<'ctx>: Sized {
    /// Push a 256-bit word onto the stack.
    ///
    /// Values narrower than 256 bits must be zero-extended by the
    /// implementation before being stored.
    fn push_word<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
        value: IntValue<'ctx>,
    ) -> Result<(), Error>;

    /// Push a word with optional compile-time known-u64 metadata.
    fn push_word_with_known_u64<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
        value: IntValue<'ctx>,
        _known_u64: Option<u64>,
    ) -> Result<(), Error> {
        self.push_word(bctx, value)
    }

    /// Pop and return the top 256-bit word.
    ///
    /// Returns an error (or emits a terminal IR path) on underflow.
    fn pop_word<'b>(&self, bctx: &BuildCtx<'ctx, 'b, Self>) -> Result<IntValue<'ctx>, Error>;

    /// Returns a word without changing the stack.
    fn peek_word<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
        depth_from_top: u8,
    ) -> Result<IntValue<'ctx>, Error>;

    /// Returns whether at least `count` words are available.
    fn has_words<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
        count: u32,
    ) -> Result<IntValue<'ctx>, Error>;

    /// Duplicate the word at depth `index` (1 = top) and push the copy.
    fn dup<'b>(&self, bctx: &BuildCtx<'ctx, 'b, Self>, index: u8) -> Result<(), Error>;

    /// Swap the top word with the word at depth `index + 1` (1 = second from top).
    fn swap<'b>(&self, bctx: &BuildCtx<'ctx, 'b, Self>, index: u8) -> Result<(), Error>;

    /// Export any backend-local final stack state into the runtime context.
    fn materialize_for_return<'b>(&self, _bctx: &BuildCtx<'ctx, 'b, Self>) -> Result<(), Error> {
        Ok(())
    }

    /// Pop two words. Returns `(top, second)` matching EVM stack order.
    fn pop_2<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
    ) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
        let a = self.pop_word(bctx)?;
        let b = self.pop_word(bctx)?;
        Ok((a, b))
    }

    /// Pop three words. Returns `(top, second, third)` matching EVM stack order.
    fn pop_3<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
    ) -> Result<(IntValue<'ctx>, IntValue<'ctx>, IntValue<'ctx>), Error> {
        let a = self.pop_word(bctx)?;
        let b = self.pop_word(bctx)?;
        let c = self.pop_word(bctx)?;
        Ok((a, b, c))
    }

    /// Pop seven words. Returns `(top, second, ..., seventh)` matching EVM stack order.
    fn pop_7<'b>(&self, bctx: &BuildCtx<'ctx, 'b, Self>) -> Result<SevenWords<'ctx>, Error> {
        let a = self.pop_word(bctx)?;
        let b = self.pop_word(bctx)?;
        let c = self.pop_word(bctx)?;
        let d = self.pop_word(bctx)?;
        let e = self.pop_word(bctx)?;
        let f = self.pop_word(bctx)?;
        let g = self.pop_word(bctx)?;
        Ok((a, b, c, d, e, f, g))
    }
}

// ---------------------------------------------------------------------------
// RuntimeStackBackend
// ---------------------------------------------------------------------------

/// Stack backend that emits `jet_runtime` C-ABI builtin calls.
///
/// Stack state lives in the `Context` struct at runtime.  `pop_word` emits a
/// null-pointer check and branches to a `StackUnderflow` return block on
/// failure, preserving the existing IR contract from `call_stack_pop` in
/// `ops.rs`.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RuntimeStackBackend;

impl RuntimeStackBackend {
    /// Zero-extend a value narrower than 256 bits to i256.
    fn normalize<'ctx, 'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
        value: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, Error> {
        let bit_width = value.get_type().get_bit_width();
        match bit_width {
            1 | 8 | 32 | 64 | 160 => {
                Ok(bctx
                    .builder
                    .build_int_z_extend(value, bctx.env.types().i256, "int_to_word")?)
            }
            256 => Ok(value),
            _ => Err(Error::InvalidBitWidth(bit_width)),
        }
    }
}

impl<'ctx> StackBackend<'ctx> for RuntimeStackBackend {
    fn push_word<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
        value: IntValue<'ctx>,
    ) -> Result<(), Error> {
        let value = self.normalize(bctx, value)?;
        let ret = bctx.builder.build_call(
            bctx.env.symbols().stack_push_word(),
            &[bctx.registers.exec_ctx.into(), value.into()],
            "stack_push_i256",
        )?;
        let success = ret.try_as_basic_value().unwrap_basic().into_int_value();
        branch_on_stack_failure(
            bctx,
            bctx.builder.build_not(success, "stack_push_failed")?,
            ReturnCode::StackOverflow,
            "stack_overflow",
        )
    }

    fn pop_word<'b>(&self, bctx: &BuildCtx<'ctx, 'b, Self>) -> Result<IntValue<'ctx>, Error> {
        let ret = bctx.builder.build_call(
            bctx.env.symbols().stack_pop(),
            &[bctx.registers.exec_ctx.into()],
            "word_ptr",
        )?;
        let ptr = ret.try_as_basic_value().unwrap_basic().into_pointer_value();

        let is_null = bctx.builder.build_is_null(ptr, "is_stack_underflow")?;
        let underflow_block = bctx
            .env
            .context()
            .append_basic_block(bctx.func, "stack_underflow");
        let valid_block = bctx
            .env
            .context()
            .append_basic_block(bctx.func, "stack_valid");
        bctx.builder
            .build_conditional_branch(is_null, underflow_block, valid_block)?;

        bctx.builder.position_at_end(underflow_block);
        bctx.builder
            .build_store(bctx.registers.gas_remaining, bctx.gas_remaining()?)?;
        let underflow_code = bctx
            .env
            .types()
            .i8
            .const_int(ReturnCode::StackUnderflow as u64, false);
        bctx.builder.build_return(Some(&underflow_code))?;

        bctx.builder.position_at_end(valid_block);
        let loaded = bctx
            .builder
            .build_load(bctx.env.types().i256, ptr, "load_int")?;
        let word = loaded.into_int_value();
        Ok(word)
    }

    fn peek_word<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
        depth_from_top: u8,
    ) -> Result<IntValue<'ctx>, Error> {
        let index = bctx.env.types().i8.const_int(depth_from_top as u64, false);
        let ret = bctx.builder.build_call(
            bctx.env.symbols().stack_peek(),
            &[bctx.registers.exec_ctx.into(), index.into()],
            "stack_peek_gas",
        )?;
        let ptr = ret.try_as_basic_value().unwrap_basic().into_pointer_value();
        branch_on_stack_failure(
            bctx,
            bctx.builder.build_is_null(ptr, "stack_peek_gas_failed")?,
            ReturnCode::StackUnderflow,
            "stack_underflow",
        )?;
        Ok(bctx
            .builder
            .build_load(bctx.env.types().i256, ptr, "stack_peek_gas_word")?
            .into_int_value())
    }

    fn has_words<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
        count: u32,
    ) -> Result<IntValue<'ctx>, Error> {
        let stack_ptr = bctx
            .builder
            .build_load(
                bctx.env.types().i32,
                bctx.builder.build_struct_gep(
                    bctx.env.types().exec_ctx,
                    bctx.registers.exec_ctx,
                    0,
                    "gas.stack_ptr",
                )?,
                "gas.stack_depth",
            )?
            .into_int_value();
        Ok(bctx.builder.build_int_compare(
            inkwell::IntPredicate::UGE,
            stack_ptr,
            bctx.env.types().i32.const_int(count as u64, false),
            "gas.stack_available",
        )?)
    }

    fn dup<'b>(&self, bctx: &BuildCtx<'ctx, 'b, Self>, index: u8) -> Result<(), Error> {
        let runtime_index = index
            .checked_sub(1)
            .ok_or_else(|| Error::invariant_violation("dup index must be >= 1"))?;
        let index_value = bctx.env.types().i8.const_int(runtime_index as u64, false);
        let ret = bctx.builder.build_call(
            bctx.env.symbols().stack_peek(),
            &[bctx.registers.exec_ctx.into(), index_value.into()],
            "stack_peek_word_result",
        )?;
        let ptr = ret.try_as_basic_value().unwrap_basic().into_pointer_value();
        branch_on_stack_failure(
            bctx,
            bctx.builder.build_is_null(ptr, "stack_peek_failed")?,
            ReturnCode::StackUnderflow,
            "stack_underflow",
        )?;
        let ret = bctx.builder.build_call(
            bctx.env.symbols().stack_push_ptr(),
            &[bctx.registers.exec_ctx.into(), ptr.into()],
            "stack_push_ptr",
        )?;
        let success = ret.try_as_basic_value().unwrap_basic().into_int_value();
        branch_on_stack_failure(
            bctx,
            bctx.builder.build_not(success, "stack_push_failed")?,
            ReturnCode::StackOverflow,
            "stack_overflow",
        )
    }

    fn swap<'b>(&self, bctx: &BuildCtx<'ctx, 'b, Self>, index: u8) -> Result<(), Error> {
        let runtime_index = index
            .checked_sub(1)
            .ok_or_else(|| Error::invariant_violation("swap index must be >= 1"))?;
        let index_value = bctx.env.types().i8.const_int(runtime_index as u64, false);
        let ret = bctx.builder.build_call(
            bctx.env.symbols().stack_swap(),
            &[bctx.registers.exec_ctx.into(), index_value.into()],
            "stack_swap_ret",
        )?;
        let success = ret.try_as_basic_value().unwrap_basic().into_int_value();
        branch_on_stack_failure(
            bctx,
            bctx.builder.build_not(success, "stack_swap_failed")?,
            ReturnCode::StackUnderflow,
            "stack_underflow",
        )
    }
}

fn branch_on_stack_failure<'ctx>(
    bctx: &BuildCtx<'ctx, '_, RuntimeStackBackend>,
    failed: IntValue<'ctx>,
    return_code: ReturnCode,
    name: &str,
) -> Result<(), Error> {
    let failure_block = bctx.env.context().append_basic_block(bctx.func, name);
    let valid_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, &format!("{name}_valid"));
    bctx.builder
        .build_conditional_branch(failed, failure_block, valid_block)?;
    bctx.builder.position_at_end(failure_block);
    bctx.builder
        .build_store(bctx.registers.gas_remaining, bctx.gas_remaining()?)?;
    let code = bctx.env.types().i8.const_int(return_code as u64, false);
    bctx.builder.build_return(Some(&code))?;
    bctx.builder.position_at_end(valid_block);
    Ok(())
}

// ---------------------------------------------------------------------------
// SymbolicStackBackend
// ---------------------------------------------------------------------------

/// Stack backend that tracks values at compile time as LLVM `IntValue`s.
///
/// No runtime stack traffic is emitted for push/pop/dup/swap.  All operations
/// manipulate an in-memory [`SymbolicStack`] that exists only during
/// compilation.  Values narrower than 256 bits are zero-extended on push,
/// exactly matching the [`RuntimeStackBackend`] contract.
pub(crate) struct SymbolicStackBackend<'ctx> {
    stack: RefCell<SymbolicStack<'ctx>>,
}

impl<'ctx> SymbolicStackBackend<'ctx> {
    pub(crate) fn new() -> Self {
        Self {
            stack: RefCell::new(SymbolicStack::new()),
        }
    }

    pub(crate) fn snapshot(&self) -> SymbolicStack<'ctx> {
        self.stack.borrow().clone()
    }

    pub(crate) fn len(&self) -> usize {
        self.stack.borrow().len()
    }

    pub(crate) fn restore(&self, stack: SymbolicStack<'ctx>) {
        *self.stack.borrow_mut() = stack;
    }

    pub(crate) fn peek_word_known_u64(&self, depth_from_top: usize) -> Result<Option<u64>, Error> {
        self.stack.borrow().peek_word_known_u64(depth_from_top)
    }

    /// Zero-extend a value narrower than 256 bits to i256.
    fn normalize<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
        value: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, Error> {
        let bit_width = value.get_type().get_bit_width();
        match bit_width {
            1 | 8 | 32 | 64 | 160 => {
                Ok(bctx
                    .builder
                    .build_int_z_extend(value, bctx.env.types().i256, "int_to_word")?)
            }
            256 => Ok(value),
            _ => Err(Error::InvalidBitWidth(bit_width)),
        }
    }
}

impl<'ctx> StackBackend<'ctx> for SymbolicStackBackend<'ctx> {
    fn push_word<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
        value: IntValue<'ctx>,
    ) -> Result<(), Error> {
        let value = self.normalize(bctx, value)?;
        self.stack.borrow_mut().push_word(value);
        Ok(())
    }

    fn push_word_with_known_u64<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
        value: IntValue<'ctx>,
        known_u64: Option<u64>,
    ) -> Result<(), Error> {
        let value = self.normalize(bctx, value)?;
        self.stack
            .borrow_mut()
            .push_word_with_known_u64(value, known_u64);
        Ok(())
    }

    fn pop_word<'b>(&self, _bctx: &BuildCtx<'ctx, 'b, Self>) -> Result<IntValue<'ctx>, Error> {
        self.stack.borrow_mut().pop_word()
    }

    fn peek_word<'b>(
        &self,
        _bctx: &BuildCtx<'ctx, 'b, Self>,
        depth_from_top: u8,
    ) -> Result<IntValue<'ctx>, Error> {
        self.stack.borrow().peek_word(depth_from_top as usize)
    }

    fn has_words<'b>(
        &self,
        bctx: &BuildCtx<'ctx, 'b, Self>,
        count: u32,
    ) -> Result<IntValue<'ctx>, Error> {
        Ok(bctx.env.context().bool_type().const_int(
            u64::from(self.stack.borrow().len() >= count as usize),
            false,
        ))
    }

    fn dup<'b>(&self, _bctx: &BuildCtx<'ctx, 'b, Self>, index: u8) -> Result<(), Error> {
        self.stack.borrow_mut().dup(index)
    }

    fn swap<'b>(&self, _bctx: &BuildCtx<'ctx, 'b, Self>, index: u8) -> Result<(), Error> {
        self.stack.borrow_mut().swap(index)
    }

    fn materialize_for_return<'b>(&self, bctx: &BuildCtx<'ctx, 'b, Self>) -> Result<(), Error> {
        let slots = self.stack.borrow().clone_slots();
        for slot in slots {
            let value = match slot {
                crate::builder::symbolic::StackValue::Word { value, .. } => value,
            };
            bctx.builder.build_call(
                bctx.env.symbols().stack_push_word(),
                &[bctx.registers.exec_ctx.into(), value.into()],
                "stack_push_i256",
            )?;
        }
        Ok(())
    }
}
