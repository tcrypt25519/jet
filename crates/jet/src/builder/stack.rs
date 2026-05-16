//! Stack backend trait and concrete implementations.
//!
//! [`StackBackend`] is the only interface between EVM opcode semantics and the
//! underlying stack representation.  There are two concrete implementations:
//!
//! * [`RuntimeStackBackend`] – emits LLVM `call` instructions to the
//!   `jet_runtime` C-ABI builtins, keeping the EVM stack in the `Context`
//!   struct at runtime.  This is a direct replacement for the `call_stack_*`
//!   helpers currently in `ops.rs`.
//!
//! * [`SymbolicStackBackend`] – tracks values as LLVM [`IntValue`]s at
//!   compile time, eliminating all runtime stack traffic for supported ops.
//!
//! [`BuildCtx`]: crate::builder::contract::BuildCtx

use std::cell::RefCell;

use inkwell::values::{AsValueRef, IntValue, PointerValue};
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
            1 | 8 | 32 => {
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
        bctx.builder.build_call(
            bctx.env.symbols().stack_push_word(),
            &[bctx.registers.exec_ctx.into(), value.into()],
            "stack_push_i256",
        )?;
        Ok(())
    }

    fn pop_word<'b>(&self, bctx: &BuildCtx<'ctx, 'b, Self>) -> Result<IntValue<'ctx>, Error> {
        let ret = bctx.builder.build_call(
            bctx.env.symbols().stack_pop(),
            &[bctx.registers.exec_ctx.into()],
            "word_ptr",
        )?;
        let ptr = unsafe { PointerValue::new(ret.as_value_ref()) };

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
        let word = unsafe { IntValue::new(loaded.as_value_ref()) };
        Ok(word)
    }

    fn dup<'b>(&self, bctx: &BuildCtx<'ctx, 'b, Self>, index: u8) -> Result<(), Error> {
        let index_value = bctx.env.types().i8.const_int(index as u64, false);
        let ret = bctx.builder.build_call(
            bctx.env.symbols().stack_peek(),
            &[bctx.registers.exec_ctx.into(), index_value.into()],
            "stack_peek_word_result",
        )?;
        let ptr = unsafe { PointerValue::new(ret.as_value_ref()) };
        bctx.builder.build_call(
            bctx.env.symbols().stack_push_ptr(),
            &[bctx.registers.exec_ctx.into(), ptr.into()],
            "stack_push_ptr",
        )?;
        Ok(())
    }

    fn swap<'b>(&self, bctx: &BuildCtx<'ctx, 'b, Self>, index: u8) -> Result<(), Error> {
        let index_value = bctx.env.types().i8.const_int(index as u64, false);
        bctx.builder.build_call(
            bctx.env.symbols().stack_swap(),
            &[bctx.registers.exec_ctx.into(), index_value.into()],
            "stack_swap_ret",
        )?;
        Ok(())
    }
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
            1 | 8 | 32 => {
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
